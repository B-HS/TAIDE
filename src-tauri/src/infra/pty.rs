use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtyPair, PtySize};

use crate::error::{AppError, AppResult};
use crate::infra::shell_integration;

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
    paused: Mutex<bool>,
    condvar: Condvar,
}

impl PauseGate {
    fn new() -> Self {
        Self {
            paused: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    fn set_paused(&self, paused: bool) {
        *self.paused.lock() = paused;
        if !paused {
            self.condvar.notify_all();
        }
    }

    fn wait_while_paused(&self) {
        let mut guard = self.paused.lock();
        while *guard {
            self.condvar.wait(&mut guard);
        }
    }
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    pause: Arc<PauseGate>,
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

    pub fn kill(&self) -> AppResult<()> {
        self.killer.lock().kill().map_err(AppError::from)
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

/// Guarantees a `PtySession` never leaks its reader/flusher threads, its child process, or its
/// shell-integration temp directory, no matter how it stops being reachable — an explicit
/// `pty_kill`, `TerminalStore::kill_project`/`kill_all` dropping the map entry, or a panic unwinding
/// through the store's lock. `pause.set_paused(false)` must run *before* `kill`: the reader thread
/// parked in `wait_while_paused` only wakes on the pause condvar's `notify_all`, never on the child
/// dying, so a session killed while paused (`pty_set_paused(true)` with no matching `false` before
/// close) previously left both the reader thread and the flusher thread it gates (only the reader
/// thread's read loop exiting calls [`FlushSignal::stop`]) parked forever — two threads leaked per
/// paused-then-killed session. Killing first would still leave the reader blocked on the condvar
/// since the child's death doesn't touch the pause gate at all. The temp-dir removal runs
/// unconditionally alongside the two, independent of whether the shell ever reached its injected
/// script's own self-`rm -rf` line.
impl Drop for PtySession {
    fn drop(&mut self) {
        self.pause.set_paused(false);
        let _ = self.killer.lock().kill();
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
    let mut child = slave.spawn_command(cmd).map_err(|error| AppError::Internal(error.to_string()))?;
    drop(slave);

    let shell_pid = child.process_id();
    let killer = child.clone_killer();
    let mut reader = master.try_clone_reader().map_err(|error| AppError::Internal(error.to_string()))?;
    let writer = master.take_writer().map_err(|error| AppError::Internal(error.to_string()))?;

    let pause = Arc::new(PauseGate::new());
    let reader_pause = pause.clone();

    let batch = Arc::new(Mutex::new(OutputBatch::new(on_data)));
    let flusher_batch = batch.clone();
    let flush_signal = Arc::new(FlushSignal::new());
    let flusher_signal = flush_signal.clone();

    std::thread::spawn(move || run_flusher(&flusher_signal, &flusher_batch));

    std::thread::spawn(move || {
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
    });

    std::thread::spawn(move || {
        let code = child.wait().ok().map(|status| status.exit_code() as i32);
        on_exit(code);
    });

    Ok(PtySession {
        master,
        writer: Arc::new(Mutex::new(writer)),
        killer: Mutex::new(killer),
        pause,
        shell_pid,
        shell_integration_temp_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

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
        let original = std::env::var(shell_integration::SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::set_var(shell_integration::SHELL_INTEGRATION_ENV_VAR, "1");

        let (cmd, temp_dir) = build_command(&base_config(None));

        match original {
            Some(value) => std::env::set_var(shell_integration::SHELL_INTEGRATION_ENV_VAR, value),
            None => std::env::remove_var(shell_integration::SHELL_INTEGRATION_ENV_VAR),
        }

        assert!(cmd.is_default_prog(), "주입이 없으면 프로그램 선택을 바꾸지 않아야 한다");
        assert!(temp_dir.is_none(), "주입이 없으면 임시 디렉터리도 생성되지 않아야 한다");
    }

    #[test]
    fn zsh_주입은_프로그램은_바꾸지_않고_zdotdir만_추가한다() {
        let original = std::env::var(shell_integration::SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::remove_var(shell_integration::SHELL_INTEGRATION_ENV_VAR);

        let (cmd, temp_dir) = build_command(&base_config(Some("/bin/zsh")));

        if let Some(value) = original {
            std::env::set_var(shell_integration::SHELL_INTEGRATION_ENV_VAR, value);
        }

        assert!(!cmd.is_default_prog());
        assert_eq!(cmd.get_argv(), &vec![std::ffi::OsString::from("/bin/zsh")]);
        assert!(cmd.get_env("ZDOTDIR").is_some());
        let temp_dir = temp_dir.expect("zsh 주입은 임시 디렉터리를 만들어야 한다");
        assert!(temp_dir.join(".zshrc").is_file());
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn bash_주입은_init_file_인자를_추가한다() {
        let original = std::env::var(shell_integration::SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::remove_var(shell_integration::SHELL_INTEGRATION_ENV_VAR);

        let (cmd, temp_dir) = build_command(&base_config(Some("/bin/bash")));

        if let Some(value) = original {
            std::env::set_var(shell_integration::SHELL_INTEGRATION_ENV_VAR, value);
        }

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
        let original = std::env::var(shell_integration::SHELL_INTEGRATION_ENV_VAR).ok();
        std::env::remove_var(shell_integration::SHELL_INTEGRATION_ENV_VAR);

        let session = spawn(base_config(Some("/bin/zsh")), |_bytes| {}, |_code| {}).expect("스폰 성공");

        if let Some(value) = original {
            std::env::set_var(shell_integration::SHELL_INTEGRATION_ENV_VAR, value);
        }

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
