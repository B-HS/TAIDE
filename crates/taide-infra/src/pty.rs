use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtyPair, PtySize};

use taide_model::error::{AppError, AppResult};

use crate::shell_integration;

/// Output-batching knobs for the reader thread below — owned here (their only consumer) rather
/// than by `domain::terminal::types`, so this infra module carries no domain reference
/// (layer direction, T1-I §1.3).
const READ_BUFFER_BYTES: usize = 64 * 1024;
const OUTPUT_BATCH_MS: u64 = 4;
const OUTPUT_FLUSH_TICK_MS: u64 = 5;

pub struct PtySpawnConfig {
    pub shell: Option<String>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
    pub extra_env: Vec<(String, String)>,
}

struct PauseGate {
    state: Mutex<PauseState>,
    condvar: Condvar,
}

#[derive(Default)]
struct PauseState {
    is_paused: bool,
    is_stopped: bool,
}

type SharedChildKiller = Arc<Mutex<Option<Box<dyn ChildKiller + Send + Sync>>>>;

struct PtyChildWaitOwner {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    killer: SharedChildKiller,
}

impl PtyChildWaitOwner {
    fn new(child: Box<dyn portable_pty::Child + Send + Sync>) -> Self {
        let killer = Arc::new(Mutex::new(Some(child.clone_killer())));
        Self { child, killer }
    }

    fn finish(mut self) -> std::io::Result<portable_pty::ExitStatus> {
        #[cfg(unix)]
        {
            let observed = self
                .child
                .as_mut()
                .as_any_mut()
                .downcast_mut::<std::process::Child>()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Unsupported, "native Unix PTY child is not std::process::Child"))
                .and_then(crate::owned_child::wait_for_exit_unreaped);
            self.killer.lock().take();
            observed?;
        }
        let status = self.child.wait();
        #[cfg(not(unix))]
        self.killer.lock().take();
        status
    }
}

impl Drop for PtyChildWaitOwner {
    fn drop(&mut self) {
        let mut killer = self.killer.lock();
        if let Some(killer) = killer.as_mut() {
            killer.kill().ok();
        }
        killer.take();
    }
}

type PtyWorker = std::thread::JoinHandle<std::io::Result<()>>;

#[derive(Default)]
struct PtyWorkerWaitState {
    workers: Vec<PtyWorker>,
    joining: Option<tokio::task::JoinHandle<Result<(), String>>>,
    result: Option<Result<(), String>>,
}

struct PtyCompletionInner {
    state: tokio::sync::Mutex<PtyWorkerWaitState>,
    killer: SharedChildKiller,
    pause: Arc<PauseGate>,
}

/// Owns PTY worker completion independently of the session's master and writer resources.
#[derive(Clone)]
pub struct PtyCompletionHandle(Arc<PtyCompletionInner>);

impl PtyCompletionHandle {
    fn new(workers: Vec<PtyWorker>, killer: SharedChildKiller, pause: Arc<PauseGate>) -> Self {
        Self(Arc::new(PtyCompletionInner {
            state: tokio::sync::Mutex::new(PtyWorkerWaitState {
                workers,
                ..PtyWorkerWaitState::default()
            }),
            killer,
            pause,
        }))
    }

    /// Checks whether two handles own the same worker set, regardless of their session metadata.
    pub fn is_same_worker(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// Requests termination using the same revocable permission and permanent pause gate as the session.
    /// This does not cancel a blocking read or callback and does not join workers.
    pub fn kill(&self) -> AppResult<()> {
        self.0.pause.stop();
        let mut killer = self.0.killer.lock();
        match killer.as_mut() {
            Some(killer) => killer.kill().map_err(AppError::from),
            None => Ok(()),
        }
    }

    /// Reports successful joining of every worker, not merely child exit or callback entry.
    /// Wait, worker panic, or join-task errors remain unfinished rather than proving child reaping.
    pub fn is_finished(&self) -> bool {
        self.0.state.try_lock().is_ok_and(|state| matches!(state.result, Some(Ok(()))))
    }

    /// Joins every worker on a blocking pool thread without blocking the async or native event loop.
    /// Dropping this future retains the join task for a later wait; callbacks and OS reads are not aborted.
    /// Call inside a live Tokio runtime. Errors are cached and do not prove successful child reaping.
    pub async fn wait_for_completion(&self) -> AppResult<()> {
        let mut state = self.0.state.lock().await;
        if let Some(result) = &state.result {
            return result.as_ref().map(|_| ()).map_err(|error| AppError::Internal(error.clone()));
        }
        if state.joining.is_none() {
            let workers = std::mem::take(&mut state.workers);
            state.joining = Some(tokio::task::spawn_blocking(move || {
                let mut failure = None;
                for worker in workers {
                    match worker.join() {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            failure.get_or_insert_with(|| format!("PTY worker failed: {error}"));
                        }
                        Err(_) => {
                            failure.get_or_insert_with(|| "PTY worker panicked".to_string());
                        }
                    }
                }
                failure.map_or(Ok(()), Err)
            }));
        }
        let result = match state.joining.as_mut() {
            Some(task) => match task.await {
                Ok(result) => result,
                Err(error) => Err(format!("PTY join task failed: {error}")),
            },
            None => Err("PTY join task missing".to_string()),
        };
        state.joining.take();
        let output = result.as_ref().map(|_| ()).map_err(|error| AppError::Internal(error.clone()));
        state.result = Some(result);
        output
    }
}

impl PauseGate {
    fn new() -> Self {
        Self {
            state: Mutex::new(PauseState::default()),
            condvar: Condvar::new(),
        }
    }

    fn set_paused(&self, paused: bool) {
        let mut state = self.state.lock();
        if state.is_stopped {
            return;
        }
        state.is_paused = paused;
        if !paused {
            self.condvar.notify_all();
        }
    }

    fn stop(&self) {
        let mut state = self.state.lock();
        state.is_stopped = true;
        state.is_paused = false;
        self.condvar.notify_all();
    }

    fn wait_while_paused(&self) {
        let mut guard = self.state.lock();
        while guard.is_paused {
            self.condvar.wait(&mut guard);
        }
    }
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pause: Arc<PauseGate>,
    completion: PtyCompletionHandle,
    #[cfg_attr(not(windows), allow(dead_code))]
    shell_pid: Option<u32>,
    /// The temp directory `build_command`'s `shell_integration::prepare` call created for this
    /// session's OSC 133 injection (`None` for an unsupported/opted-out/fish shell — nothing was
    /// ever created). Removed by this session's `Drop` impl rather than left to the injected
    /// script's own best-effort `rm -rf` — see the `temp_dir` field doc on
    /// `shell_integration::ShellIntegrationPlan` for why that alone isn't reliable.
    shell_integration_temp_dir: Option<PathBuf>,
}

impl PtySession {
    pub fn write(&self, data: &[u8]) -> AppResult<()> {
        let mut writer = self.writer.lock();
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    /// Hands out a clone of the same `Arc<Mutex<..>>` [`PtySession::write`] itself locks — not a
    /// second, independent writer. Lets `terminal::commands::pty_write` release `TerminalStore`'s
    /// lock before the (potentially blocking, if the child isn't draining its stdin) write, while
    /// still writing through the one real writer every other path uses.
    pub fn writer_handle(&self) -> Arc<Mutex<Box<dyn Write + Send>>> {
        self.writer.clone()
    }

    pub fn resize(&self, cols: u16, rows: u16) -> AppResult<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    /// Permanently opens the pause gate before requesting termination, even when signaling fails.
    /// Unix child wait revokes signal permission before reaping; later requests do not signal a cached PID.
    /// This requests shutdown but does not join the child wait, reader, or flusher threads.
    pub fn kill(&self) -> AppResult<()> {
        self.completion.kill()
    }

    /// Clones worker ownership without retaining the master or writer; session Drop remains a shutdown request.
    pub fn completion_handle(&self) -> PtyCompletionHandle {
        self.completion.clone()
    }

    pub fn set_paused(&self, paused: bool) {
        self.pause.set_paused(paused);
    }

    #[cfg(unix)]
    pub fn foreground_pid(&self) -> Option<u32> {
        self.master.process_group_leader().map(|pid| pid as u32)
    }

    #[cfg(windows)]
    pub fn foreground_pid(&self) -> Option<u32> {
        self.shell_pid
    }

    #[cfg(not(any(unix, windows)))]
    pub fn foreground_pid(&self) -> Option<u32> {
        None
    }
}

/// Opens the pause gate, requests child termination, and removes the owned integration directory.
/// Shutdown requests do not prove child reaping or reader/flusher/callback completion.
impl Drop for PtySession {
    fn drop(&mut self) {
        self.kill().ok();
        if let Some(temp_dir) = &self.shell_integration_temp_dir {
            std::fs::remove_dir_all(temp_dir).ok();
        }
    }
}

fn should_flush(batch_len: usize, elapsed: Duration) -> bool {
    batch_len >= READ_BUFFER_BYTES || elapsed >= Duration::from_millis(OUTPUT_BATCH_MS)
}

#[derive(Default)]
struct FlushState {
    pending: bool,
    stopped: bool,
}

/// Wakes the flusher thread only when the reader thread has actually left bytes sitting in the
/// batch, instead of the unconditional `sleep(OUTPUT_FLUSH_TICK_MS)` loop this replaces. That loop
/// woke, took the batch lock and called a no-op `flush` 200 times a second **per session** for the
/// entire life of every terminal — including the overwhelmingly common case of an idle shell with
/// nothing to flush (§2 L-2). Now an idle session parks on the condvar and costs nothing; a session
/// producing output pays one wakeup per burst, which is what the timer was ever for.
struct FlushSignal {
    state: Mutex<FlushState>,
    condvar: Condvar,
}

struct ReaderFlushStop(Arc<FlushSignal>);

impl Drop for ReaderFlushStop {
    fn drop(&mut self) {
        self.0.stop();
    }
}

impl FlushSignal {
    fn new() -> Self {
        Self {
            state: Mutex::new(FlushState::default()),
            condvar: Condvar::new(),
        }
    }

    /// Records whether the batch currently holds bytes the reader thread did *not* flush itself,
    /// waking the flusher when it does. Called after every read, so a push that already flushed
    /// (batch full, or `OUTPUT_BATCH_MS` elapsed) clears the flag instead of scheduling a wakeup
    /// that would find nothing to do.
    fn set_pending(&self, pending: bool) {
        let mut state = self.state.lock();
        if state.pending == pending {
            return;
        }
        state.pending = pending;
        if pending {
            self.condvar.notify_one();
        }
    }

    /// Marks the reader thread finished so the flusher's next wait returns `false` and its loop
    /// ends. The reader thread flushes the final batch itself, so nothing is left behind here.
    fn stop(&self) {
        self.state.lock().stopped = true;
        self.condvar.notify_all();
    }

    /// Blocks until the batch has bytes waiting, consuming the flag; returns `false` once the
    /// reader thread has stopped, which is the flusher loop's only exit.
    fn wait_for_pending(&self) -> bool {
        let mut state = self.state.lock();
        while !state.pending && !state.stopped {
            self.condvar.wait(&mut state);
        }
        if state.stopped {
            return false;
        }
        state.pending = false;
        true
    }
}

/// Flushes bytes the reader thread batched but could not send itself — the last, small chunk of a
/// burst, which `should_flush` holds back and which would otherwise sit in memory until the *next*
/// read (the trailing escape sequence that left autocomplete ghosts on screen — `terminal.md`
/// §12.2-A). Waits `OUTPUT_FLUSH_TICK_MS` after being woken so a burst still coalesces into one
/// send rather than one per read.
fn run_flusher<D: Fn(&[u8])>(signal: &FlushSignal, batch: &Mutex<OutputBatch<D>>) {
    while signal.wait_for_pending() {
        std::thread::sleep(Duration::from_millis(OUTPUT_FLUSH_TICK_MS));
        batch.lock().flush();
    }
}

struct OutputBatch<D> {
    buf: Vec<u8>,
    last_flush: Instant,
    on_data: D,
}

impl<D: Fn(&[u8])> OutputBatch<D> {
    fn new(on_data: D) -> Self {
        Self {
            buf: Vec::with_capacity(READ_BUFFER_BYTES),
            last_flush: Instant::now(),
            on_data,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
        if should_flush(self.buf.len(), self.last_flush.elapsed()) {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        (self.on_data)(&self.buf);
        self.buf.clear();
        self.last_flush = Instant::now();
    }

    /// Whether [`OutputBatch::push`] left bytes behind — what the reader thread reports to
    /// [`FlushSignal::set_pending`] so the flusher thread is woken only when there is work.
    fn has_pending(&self) -> bool {
        !self.buf.is_empty()
    }
}

const FALLBACK_LOCALE: &str = "en_US.UTF-8";

fn utf8_locale() -> String {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if value.to_ascii_uppercase().contains("UTF-8") || value.to_ascii_uppercase().contains("UTF8") {
                return value;
            }
        }
    }
    FALLBACK_LOCALE.to_string()
}

/// Assembles the `CommandBuilder` for a pty spawn. **Not pure**: it delegates
/// to [`shell_integration::prepare`], which — for zsh/bash — creates a temp
/// directory and writes the OSC 133 integration script(s) into it as a side
/// effect (`std::fs::create_dir_all`/`std::fs::write`), swallowing any I/O
/// failure into a `log::warn!` + no-injection fallback rather than surfacing
/// it through this function's `CommandBuilder` return type. Every call to
/// [`spawn`] therefore performs a filesystem write before the child process
/// exists. Also hands back that temp directory (`None` when no injection
/// happened) so [`spawn`] can stash it on the resulting `PtySession` for
/// deterministic Drop-time cleanup — see the `temp_dir` field doc on
/// `shell_integration::ShellIntegrationPlan`.
fn build_command(config: &PtySpawnConfig) -> (CommandBuilder, Option<PathBuf>) {
    let integration = shell_integration::prepare(config.shell.as_deref());

    let mut cmd = match integration.as_ref().and_then(|plan| plan.override_program.as_ref()) {
        Some((program, _)) => CommandBuilder::new(program),
        None => match config.shell.as_deref() {
            Some(shell) => CommandBuilder::new(shell),
            None => CommandBuilder::new_default_prog(),
        },
    };
    if let Some((_, args)) = integration.as_ref().and_then(|plan| plan.override_program.as_ref()) {
        cmd.args(args);
    }

    cmd.cwd(&config.cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "TAIDE");
    let locale = utf8_locale();
    cmd.env("LANG", &locale);
    cmd.env("LC_CTYPE", &locale);
    if let Some(plan) = &integration {
        for (key, value) in &plan.extra_env {
            cmd.env(key, value);
        }
    }
    for (key, value) in &config.extra_env {
        cmd.env(key, value);
    }
    (cmd, integration.map(|plan| plan.temp_dir))
}

pub fn spawn<D, X>(config: PtySpawnConfig, on_data: D, on_exit: X) -> AppResult<PtySession>
where
    D: Fn(&[u8]) + Send + 'static,
    X: FnOnce(Option<i32>) + Send + 'static,
{
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| AppError::Internal(error.to_string()))?;

    let (cmd, shell_integration_temp_dir) = build_command(&config);

    let PtyPair { slave, master } = pair;
    let child = slave.spawn_command(cmd).map_err(|error| AppError::Internal(error.to_string()))?;
    drop(slave);

    let shell_pid = child.process_id();
    let child = PtyChildWaitOwner::new(child);
    let killer = child.killer.clone();
    let mut reader = master.try_clone_reader().map_err(|error| AppError::Internal(error.to_string()))?;
    let writer = master.take_writer().map_err(|error| AppError::Internal(error.to_string()))?;

    let pause = Arc::new(PauseGate::new());
    let reader_pause = pause.clone();
    let child_exit_pause = pause.clone();

    let batch = Arc::new(Mutex::new(OutputBatch::new(on_data)));
    let flusher_batch = batch.clone();
    let flush_signal = Arc::new(FlushSignal::new());
    let flusher_signal = flush_signal.clone();

    let flusher = std::thread::spawn(move || {
        run_flusher(&flusher_signal, &flusher_batch);
        Ok(())
    });

    let reader = std::thread::spawn(move || {
        let _stop_flusher = ReaderFlushStop(flush_signal.clone());
        let mut buf = [0u8; READ_BUFFER_BYTES];

        loop {
            reader_pause.wait_while_paused();

            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let mut batch = batch.lock();
                    batch.push(&buf[..n]);
                    flush_signal.set_pending(batch.has_pending());
                }
            }
        }

        flush_signal.stop();
        batch.lock().flush();
        Ok(())
    });

    let waiter = std::thread::spawn(move || {
        let status = child.finish();
        let code = status.as_ref().ok().map(|status| status.exit_code() as i32);
        child_exit_pause.stop();
        on_exit(code);
        status.map(|_| ())
    });

    let completion = PtyCompletionHandle::new(vec![flusher, reader, waiter], killer, pause.clone());

    Ok(PtySession {
        master,
        writer: Arc::new(Mutex::new(writer)),
        pause,
        completion,
        shell_pid,
        shell_integration_temp_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    #[cfg(unix)]
    #[derive(Debug, Clone)]
    struct RecordingKiller {
        called: Arc<AtomicBool>,
        should_fail: bool,
    }

    #[cfg(unix)]
    impl ChildKiller for RecordingKiller {
        fn kill(&mut self) -> std::io::Result<()> {
            self.called.store(true, AtomicOrdering::SeqCst);
            if self.should_fail {
                return Err(std::io::Error::other("synthetic kill failure"));
            }
            Ok(())
        }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(self.clone())
        }
    }

    #[cfg(unix)]
    impl portable_pty::Child for RecordingKiller {
        fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
            Ok(None)
        }

        fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
            Err(std::io::Error::other("synthetic child must not be reaped"))
        }

        fn process_id(&self) -> Option<u32> {
            None
        }
    }

    #[cfg(unix)]
    #[test]
    fn wait_owner_drop은_회수_전_권한만_사용하고_공유_killer를_반납한다() {
        let called = Arc::new(AtomicBool::new(false));
        let owner = PtyChildWaitOwner::new(Box::new(RecordingKiller {
            called: called.clone(),
            should_fail: false,
        }));
        let killer = owner.killer.clone();
        drop(owner);
        assert!(called.load(AtomicOrdering::SeqCst));
        assert!(killer.lock().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn 종료_관찰_오류는_권한을_닫고_늦은_시그널을_보내지_않는다() {
        let called = Arc::new(AtomicBool::new(false));
        let owner = PtyChildWaitOwner::new(Box::new(RecordingKiller {
            called: called.clone(),
            should_fail: false,
        }));
        let killer = owner.killer.clone();
        let result = owner.finish();
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
        assert!(killer.lock().is_none());
        assert!(!called.load(AtomicOrdering::SeqCst));
    }

    #[cfg(unix)]
    fn recording_session(should_fail: bool) -> (PtySession, Arc<AtomicBool>) {
        let pair = native_pty_system().openpty(PtySize::default()).unwrap();
        let killed = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(PauseGate::new());
        let killer: SharedChildKiller = Arc::new(Mutex::new(Some(Box::new(RecordingKiller {
            called: killed.clone(),
            should_fail,
        }))));
        let session = PtySession {
            master: pair.master,
            writer: Arc::new(Mutex::new(Box::new(std::io::sink()))),
            pause: pause.clone(),
            completion: PtyCompletionHandle::new(Vec::new(), killer, pause),
            shell_pid: None,
            shell_integration_temp_dir: None,
        };
        drop(pair.slave);
        (session, killed)
    }

    #[cfg(unix)]
    #[test]
    fn 명시적_kill은_drop_전에도_paused_reader를_깨운다() {
        let (session, killed) = recording_session(false);
        session.set_paused(true);
        let reader_pause = session.pause.clone();
        let (ready, ready_rx) = std::sync::mpsc::channel();
        let (done, done_rx) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || {
            ready.send(()).unwrap();
            reader_pause.wait_while_paused();
            done.send(()).unwrap();
        });
        ready_rx.recv_timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS)).unwrap();
        let result = session.kill();
        let woke_before_drop = done_rx.recv_timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS)).is_ok();
        drop(session);
        reader.join().unwrap();
        assert!(result.is_ok());
        assert!(killed.load(AtomicOrdering::SeqCst));
        assert!(
            woke_before_drop,
            "kill_all retains the session, so Drop cannot wake its paused reader"
        );
    }

    #[cfg(unix)]
    #[test]
    fn 종료_요청_뒤의_pause는_reader를_다시_가두지_않는다() {
        let (session, _) = recording_session(false);
        session.kill().unwrap();
        session.set_paused(true);
        let was_paused = session.pause.state.lock().is_paused;
        drop(session);
        assert!(!was_paused);
    }

    #[cfg(unix)]
    #[test]
    fn kill_오류가_있어도_pause_gate는_해제된다() {
        let (session, killed) = recording_session(true);
        session.set_paused(true);
        let result = session.kill();
        let was_paused = session.pause.state.lock().is_paused;
        drop(session);
        assert!(result.is_err());
        assert!(killed.load(AtomicOrdering::SeqCst));
        assert!(!was_paused);
    }

    #[test]
    fn child_wait도_exit_callback_전에_같은_pause_gate를_닫는다() {
        let source = include_str!("pty.rs");
        let spawn = source
            .split_once("pub fn spawn<D, X>(")
            .unwrap()
            .1
            .split_once("#[cfg(test)]")
            .unwrap()
            .0;
        assert!(spawn.contains("let child_exit_pause = pause.clone();"));
        let wait = spawn.find("let status = child.finish();").unwrap();
        let stop = spawn.find("child_exit_pause.stop();").unwrap();
        let exit = spawn.find("on_exit(code);").unwrap();
        assert!(wait < stop && stop < exit);
    }

    #[test]
    fn spawn은_reader_flusher_wait의_완료_핸들을_버리지_않는다() {
        let source = include_str!("pty.rs");
        let spawn = source
            .split_once("pub fn spawn<D, X>(")
            .unwrap()
            .1
            .split_once("#[cfg(test)]")
            .unwrap()
            .0;
        assert!(!spawn.contains("\n    std::thread::spawn(move"), "worker handles must remain owned");
        assert!(spawn.contains("PtyCompletionHandle::new(vec![flusher, reader, waiter]"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn child_종료_뒤에도_callback_반환을_기다리고_대기_drop_후_재대기한다() {
        const EXIT_CODE: i32 = 7;
        let (started, started_rx) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel::<()>();
        let session = spawn(
            controlled_shell_config(),
            |_| {},
            move |code| {
                started.send(code).ok();
                held.recv().ok();
            },
        )
        .unwrap();
        let completion = session.completion_handle();
        let written = session.write(format!("exit {EXIT_CODE}\n").as_bytes());
        let started = tokio::time::timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS), started_rx).await;
        let mut waiting = Box::pin(completion.wait_for_completion());
        let was_pending = tokio::time::timeout(Duration::from_millis(SIGNAL_PROBE_MS), &mut waiting)
            .await
            .is_err();
        drop(waiting);
        let was_finished = completion.is_finished();
        drop(session);
        let mut waiting = Box::pin(completion.wait_for_completion());
        let remained_pending = tokio::time::timeout(Duration::from_millis(SIGNAL_PROBE_MS), &mut waiting)
            .await
            .is_err();
        drop(release);
        let joined = tokio::time::timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS), &mut waiting).await;
        drop(waiting);
        assert!(written.is_ok());
        assert_eq!(started.unwrap().unwrap(), Some(EXIT_CODE));
        assert!(was_pending);
        assert!(!was_finished);
        assert!(remained_pending);
        assert!(joined.unwrap().is_ok());
        assert!(completion.is_finished());
        assert!(completion.wait_for_completion().await.is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn worker_panic과_오류도_다른_worker를_join한_뒤_실패로_기록한다() {
        let (release, held) = std::sync::mpsc::channel::<()>();
        let ended = Arc::new(AtomicBool::new(false));
        let ended_worker = ended.clone();
        let workers = vec![
            std::thread::spawn(|| panic!("synthetic worker panic")),
            std::thread::spawn(|| Err(std::io::Error::other("synthetic worker error"))),
            std::thread::spawn(move || {
                held.recv().ok();
                ended_worker.store(true, AtomicOrdering::SeqCst);
                Ok(())
            }),
        ];
        let completion = PtyCompletionHandle::new(workers, Arc::new(Mutex::new(None)), Arc::new(PauseGate::new()));
        let mut waiting = Box::pin(completion.wait_for_completion());
        let was_pending = tokio::time::timeout(Duration::from_millis(SIGNAL_PROBE_MS), &mut waiting)
            .await
            .is_err();
        drop(release);
        let result = tokio::time::timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS), &mut waiting).await;
        drop(waiting);
        assert!(was_pending);
        assert!(result.unwrap().is_err());
        assert!(ended.load(AtomicOrdering::SeqCst));
        assert!(!completion.is_finished());
        assert!(completion.wait_for_completion().await.is_err());
    }

    #[test]
    fn reader_panic에도_flusher는_stop을_받아_실제로_종료된다() {
        let (batch, _) = collecting_batch();
        let signal = Arc::new(FlushSignal::new());
        let reader_signal = signal.clone();
        let flusher_signal = signal.clone();
        let (done, done_rx) = std::sync::mpsc::channel();
        let flusher = std::thread::spawn(move || {
            run_flusher(&flusher_signal, &Mutex::new(batch));
            done.send(()).ok();
        });
        let reader = std::thread::spawn(move || {
            let _stop_flusher = ReaderFlushStop(reader_signal);
            panic!("synthetic reader panic");
        });
        let panicked = reader.join().is_err();
        let finished = done_rx.recv_timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS)).is_ok();
        signal.stop();
        flusher.join().unwrap();
        assert!(panicked);
        assert!(finished);
    }

    #[test]
    fn 종료_전의_pause와_unpause는_기존_상태를_유지한다() {
        let pause = PauseGate::new();
        pause.set_paused(true);
        assert!(pause.state.lock().is_paused);
        pause.set_paused(false);
        assert!(!pause.state.lock().is_paused);
        pause.wait_while_paused();
    }

    #[cfg(unix)]
    #[test]
    fn child_wait_회수_뒤의_kill은_숫자_pid_권한을_재사용하지_않는다() {
        const EXIT_CODE: i32 = 7;
        let (exit, exited) = std::sync::mpsc::channel();
        let session = spawn(
            controlled_shell_config(),
            |_| {},
            move |code| {
                exit.send(code).ok();
            },
        )
        .unwrap();
        let signaled = Arc::new(AtomicBool::new(false));
        *session.completion.0.killer.lock() = Some(Box::new(RecordingKiller {
            called: signaled.clone(),
            should_fail: false,
        }));
        let written = session.write(format!("exit {EXIT_CODE}\n").as_bytes());
        let code = exited.recv_timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS));
        let result = session.kill();
        let signaled_after_reaping = signaled.load(AtomicOrdering::SeqCst);
        drop(session);
        assert!(written.is_ok());
        assert_eq!(code, Ok(Some(EXIT_CODE)));
        assert!(result.is_ok());
        assert!(!signaled_after_reaping, "reaped child PID must not remain signalable");
    }

    #[cfg(unix)]
    #[test]
    fn 종료_관찰이_대기중이어도_살아있는_자기_pty는_kill할_수_있다() {
        let (exit, exited) = std::sync::mpsc::channel();
        let (output, output_rx) = std::sync::mpsc::channel();
        let session = spawn(
            controlled_shell_config(),
            move |_| {
                output.send(()).ok();
            },
            move |code| {
                exit.send(code).ok();
            },
        )
        .unwrap();
        let written = session.write(b"printf ready\\n\n");
        let alive = output_rx.recv_timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS));
        let was_waiting = matches!(exited.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));
        let killed = session.kill();
        let code = exited.recv_timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS));
        let is_revoked = session.completion.0.killer.lock().is_none();
        drop(session);
        assert!(written.is_ok());
        assert!(alive.is_ok());
        assert!(was_waiting);
        assert!(killed.is_ok());
        assert!(code.is_ok());
        assert!(is_revoked);
    }

    #[cfg(unix)]
    fn controlled_shell_config() -> PtySpawnConfig {
        let mut config = base_config(Some("/bin/sh"));
        config.extra_env = vec![("ENV".to_string(), String::new()), ("BASH_ENV".to_string(), String::new())];
        config
    }

    #[test]
    fn 배치_크기가_임계값을_넘으면_플러시한다() {
        assert!(should_flush(READ_BUFFER_BYTES, Duration::from_millis(0)));
        assert!(should_flush(READ_BUFFER_BYTES + 1, Duration::from_millis(0)));
        assert!(!should_flush(1, Duration::from_millis(0)));
    }

    #[test]
    fn 배치_경과시간이_임계값을_넘으면_플러시한다() {
        assert!(should_flush(1, Duration::from_millis(OUTPUT_BATCH_MS)));
        assert!(!should_flush(1, Duration::from_millis(OUTPUT_BATCH_MS - 1)));
    }

    type SentChunks = Arc<Mutex<Vec<Vec<u8>>>>;
    type DataSink = Box<dyn Fn(&[u8]) + Send>;
    type TestBatch = OutputBatch<DataSink>;

    fn collecting_batch() -> (TestBatch, SentChunks) {
        let sent: SentChunks = Arc::new(Mutex::new(Vec::new()));
        let recorder = sent.clone();
        let on_data: DataSink = Box::new(move |bytes: &[u8]| recorder.lock().push(bytes.to_vec()));
        (OutputBatch::new(on_data), sent)
    }

    #[test]
    fn 로케일이_없으면_utf8_기본값을_쓴다() {
        let locale = utf8_locale();
        assert!(locale.to_ascii_uppercase().contains("UTF-8") || locale.to_ascii_uppercase().contains("UTF8"));
    }

    #[test]
    fn 소량_청크는_즉시_전송되지_않고_배치에_남는다() {
        let (mut batch, sent) = collecting_batch();
        batch.push(b"hello");
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn 타이머_플러시가_갇힌_청크를_내보낸다() {
        let (mut batch, sent) = collecting_batch();
        batch.push(b"hello");
        batch.flush();
        assert_eq!(sent.lock().as_slice(), [b"hello".to_vec()]);
    }

    #[test]
    fn 빈_배치_플러시는_전송하지_않는다() {
        let (mut batch, sent) = collecting_batch();
        batch.flush();
        batch.flush();
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn 플러시_후_배치는_비워진다() {
        let (mut batch, sent) = collecting_batch();
        batch.push(b"a");
        batch.flush();
        batch.push(b"b");
        batch.flush();
        assert_eq!(sent.lock().as_slice(), [b"a".to_vec(), b"b".to_vec()]);
    }

    const SIGNAL_PROBE_MS: u64 = 60;
    const SIGNAL_WAKE_TIMEOUT_MS: u64 = 3_000;

    /// §2 L-2 의 요점 자체를 고정한다: 보낼 것이 없으면 플러셔는 **깨어나지 않는다**. 5ms 틱
    /// 루프였을 때는 유휴 세션도 초당 200회 깨어나 빈 배치에 락을 걸었다.
    #[test]
    fn 대기중인_데이터가_없으면_플러셔는_깨어나지_않는다() {
        let signal = Arc::new(FlushSignal::new());
        let (woke_tx, woke_rx) = std::sync::mpsc::channel();
        let waiter = signal.clone();
        std::thread::spawn(move || woke_tx.send(waiter.wait_for_pending()).ok());

        assert!(
            woke_rx.recv_timeout(Duration::from_millis(SIGNAL_PROBE_MS)).is_err(),
            "보낼 데이터가 없으면 대기에서 깨어나면 안 된다"
        );

        signal.set_pending(true);
        assert_eq!(
            woke_rx.recv_timeout(Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS)).ok(),
            Some(true),
            "데이터가 생기면 즉시 깨어나야 한다"
        );
    }

    #[test]
    fn 대기는_pending_플래그를_소비한다() {
        let signal = FlushSignal::new();
        signal.set_pending(true);
        assert!(signal.wait_for_pending());

        signal.stop();
        assert!(!signal.wait_for_pending(), "소비된 뒤에는 stop 으로만 대기가 끝나야 한다");
    }

    #[test]
    fn reader_가_끝나면_플러셔_루프도_끝난다() {
        let (batch, _sent) = collecting_batch();
        let batch = Arc::new(Mutex::new(batch));
        let signal = Arc::new(FlushSignal::new());
        let flusher_signal = signal.clone();
        let flusher_batch = batch.clone();
        let flusher = std::thread::spawn(move || run_flusher(&flusher_signal, &flusher_batch));

        signal.stop();
        flusher.join().expect("stop 이후 플러셔 스레드는 종료되어야 한다");
    }

    /// 배치에 갇힌 소량 청크(버스트 마지막 조각)를 플러셔가 실제로 내보내는지 — condvar 전환
    /// 이후에도 `terminal.md` §12.2-A 가 요구한 타이머 flush 의 역할이 유지되는지 확인한다.
    #[test]
    fn 배치에_남은_청크는_플러셔가_내보낸다() {
        let (mut batch, sent) = collecting_batch();
        batch.buf.extend_from_slice(b"trailing");
        assert!(batch.has_pending());

        let batch = Arc::new(Mutex::new(batch));
        let signal = Arc::new(FlushSignal::new());
        let flusher_signal = signal.clone();
        let flusher_batch = batch.clone();
        let flusher = std::thread::spawn(move || run_flusher(&flusher_signal, &flusher_batch));

        signal.set_pending(true);

        let deadline = Instant::now() + Duration::from_millis(SIGNAL_WAKE_TIMEOUT_MS);
        while sent.lock().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(OUTPUT_FLUSH_TICK_MS));
        }

        signal.stop();
        flusher.join().expect("플러셔 스레드 종료");

        assert_eq!(sent.lock().as_slice(), [b"trailing".to_vec()]);
    }

    fn base_config(shell: Option<&str>) -> PtySpawnConfig {
        PtySpawnConfig {
            shell: shell.map(str::to_string),
            cwd: std::env::temp_dir().to_string_lossy().to_string(),
            cols: 80,
            rows: 24,
            extra_env: Vec::new(),
        }
    }

    #[test]
    fn 셸_통합이_비활성이면_기존_default_prog_빌더_그대로다() {
        let _environment = shell_integration::test_support::ShellIntegrationTestEnvironment::new(true);

        let (cmd, temp_dir) = build_command(&base_config(None));
        let has_temp_dir = temp_dir.is_some();
        if let Some(temp_dir) = temp_dir {
            std::fs::remove_dir_all(temp_dir).unwrap();
        }

        assert!(cmd.is_default_prog(), "주입이 없으면 프로그램 선택을 바꾸지 않아야 한다");
        assert!(!has_temp_dir, "주입이 없으면 임시 디렉터리도 생성되지 않아야 한다");
    }

    #[test]
    fn zsh_주입은_프로그램은_바꾸지_않고_zdotdir만_추가한다() {
        let _environment = shell_integration::test_support::ShellIntegrationTestEnvironment::new(false);

        let (cmd, temp_dir) = build_command(&base_config(Some("/bin/zsh")));

        assert!(!cmd.is_default_prog());
        assert_eq!(cmd.get_argv(), &vec![std::ffi::OsString::from("/bin/zsh")]);
        assert!(cmd.get_env("ZDOTDIR").is_some());
        let temp_dir = temp_dir.expect("zsh 주입은 임시 디렉터리를 만들어야 한다");
        assert!(temp_dir.join(".zshrc").is_file());
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn bash_주입은_init_file_인자를_추가한다() {
        let _environment = shell_integration::test_support::ShellIntegrationTestEnvironment::new(false);

        let (cmd, temp_dir) = build_command(&base_config(Some("/bin/bash")));

        let argv = cmd.get_argv();
        assert_eq!(argv[0], std::ffi::OsString::from("/bin/bash"));
        assert_eq!(argv[1], std::ffi::OsString::from("--init-file"));
        std::fs::remove_dir_all(temp_dir.expect("bash 주입은 임시 디렉터리를 만들어야 한다")).ok();
    }

    #[cfg(unix)]
    #[test]
    fn drop은_일시정지된_세션도_자식_프로세스를_종료시킨다() {
        let config = base_config(Some("/bin/sh"));
        let exited = Arc::new(AtomicBool::new(false));
        let exited_for_exit = exited.clone();

        let session = spawn(
            config,
            |_bytes| {},
            move |_code| exited_for_exit.store(true, AtomicOrdering::SeqCst),
        )
        .expect("스폰 성공");

        session.set_paused(true);
        drop(session);

        let deadline = Instant::now() + Duration::from_secs(3);
        while !exited.load(AtomicOrdering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }

        assert!(
            exited.load(AtomicOrdering::SeqCst),
            "reader 스레드가 paused 상태여도 drop 이 자식 프로세스를 종료시켜야 한다"
        );
    }

    #[cfg(unix)]
    #[test]
    fn drop은_셸_통합_임시_디렉터리도_정리한다() {
        let _environment = shell_integration::test_support::ShellIntegrationTestEnvironment::new(false);

        let session = spawn(base_config(Some("/bin/zsh")), |_bytes| {}, |_code| {}).expect("스폰 성공");

        let temp_dir = session
            .shell_integration_temp_dir
            .clone()
            .expect("zsh 주입 세션은 셸 통합 임시 디렉터리를 가지고 있어야 한다");
        assert!(temp_dir.is_dir(), "세션이 살아있는 동안에는 임시 디렉터리가 존재해야 한다");

        drop(session);

        assert!(
            !temp_dir.exists(),
            "세션이 drop되면 셸 통합 임시 디렉터리도 삭제되어야 한다 — 주입된 스크립트가 self-rm 라인에 도달하지 못한 경우를 대비한 결정적 정리 경로다"
        );
    }
}
