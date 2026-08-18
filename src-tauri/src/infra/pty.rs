use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtyPair, PtySize};

use crate::domain::terminal::types::{OUTPUT_BATCH_MS, OUTPUT_FLUSH_TICK_MS, READ_BUFFER_BYTES};
use crate::error::{AppError, AppResult};
use crate::infra::shell_integration;

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
/// close) previously left both the reader thread and the flusher thread it gates (`draining` only
/// flips once the reader thread's read loop exits) parked forever — two threads leaked per
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
    let draining = Arc::new(AtomicBool::new(true));
    let flusher_draining = draining.clone();

    std::thread::spawn(move || {
        while flusher_draining.load(AtomicOrdering::SeqCst) {
            std::thread::sleep(Duration::from_millis(OUTPUT_FLUSH_TICK_MS));
            flusher_batch.lock().flush();
        }
    });

    std::thread::spawn(move || {
        let mut buf = [0u8; READ_BUFFER_BYTES];

        loop {
            reader_pause.wait_while_paused();

            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => batch.lock().push(&buf[..n]),
            }
        }

        draining.store(false, AtomicOrdering::SeqCst);
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
