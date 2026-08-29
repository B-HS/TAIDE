use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Notify;

use crate::error::{AppError, AppErrorKind, AppResult};

const HEADER_BODY_SEPARATOR: &[u8] = b"\r\n\r\n";
const CONTENT_LENGTH_HEADER: &str = "content-length";
const LSP_READ_BUFFER_BYTES: usize = 64 * 1024;

pub struct LspProcConfig {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

pub struct LspProcHandle {
    stdin: tokio::sync::Mutex<tokio::process::ChildStdin>,
    /// Wakes [`spawn`]'s wait task so it force-kills the child, replacing the `AtomicBool` that task
    /// used to notice only on its next 50ms `try_wait` poll (§2 L-3). The task now parks on
    /// `child.wait()` and this signal at once, so both real process death and a [`kill`](Self::kill)
    /// request wake it immediately, and an idle language server costs no wakeups at all.
    kill_signal: Arc<Notify>,
    /// Flipped to `true` by the wait task below the instant `child.wait()`
    /// observes the process has actually exited — *before* the `on_exit` callback runs, so a
    /// caller polling [`LspProcHandle::is_exited`] (see `domain::lsp::commands::shutdown_entry`)
    /// can detect real process death directly instead of guessing from a fixed sleep.
    exited: Arc<AtomicBool>,
    pid: Option<u32>,
}

impl LspProcHandle {
    pub async fn write_message(&self, payload: &str) -> AppResult<()> {
        let mut stdin = self.stdin.lock().await;
        let framed = encode_message(payload);
        stdin.write_all(&framed).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Kills the language server process **synchronously**, mirroring `infra::pty::PtySession::kill`
    /// (`self.killer.lock().kill()`, no `.await`). Signaling the wait task alone used to be the
    /// entire implementation — the actual OS-level kill only happened later, inside [`spawn`]'s
    /// background wait task, once it observed the request. That
    /// asynchronous handoff is fine for the normal `lsp_stop`/`lsp_restart` shutdown path (which
    /// awaits `wait_for_process_exit` afterward), but at app exit `domain::lsp::commands::LspStore::
    /// kill_all` calls this synchronously from the `RunEvent::Exit` handler and the process then
    /// terminates via `std::process::exit` immediately after — which drops the tokio runtime without
    /// running its pending tasks, so the wait task backing this handle's kill might never get
    /// scheduled at all, leaving the language server orphaned. Killing by PID here through `sysinfo`
    /// (already a workspace dependency — see `domain::ide::lockfile::is_pid_alive` for the same
    /// refresh-then-lookup idiom) closes that window: the OS-level kill happens on this call's own
    /// stack, before `kill_all`'s caller ever returns. `kill_signal` is still notified so the wait
    /// task (which usually *does* get to run, on the ordinary `lsp_stop`/`lsp_restart` paths) still
    /// reaps the child through its own `child.start_kill()` + `wait()` path — the notification is
    /// never lost even when it arrives before that task parks, since `Notify` stores the permit.
    ///
    /// Guarded by [`is_exited`](Self::is_exited): once the wait task has observed this process exit,
    /// the OS has already reaped it and is free to hand this same numeric `pid` to an unrelated
    /// process. A handle can reach this state long before `kill()` is ever called on it — e.g. a
    /// crash-looped session `domain::lsp::commands::handle_process_exit` gives up on (leaving a
    /// reaped, never-cleared `entry.proc`) then sits in `LspStore` for the rest of a long-running
    /// session until app exit's `kill_all` sweeps every entry. Without this guard, that sweep would
    /// blindly `sysinfo`-kill whatever process the OS had since reassigned that `pid` to — silent
    /// collateral damage with no relation to this language server. A still-running process can never
    /// have `exited == true` (only this handle's own wait task, which requires `child.wait()` to
    /// have first observed real process death, sets it), so live servers are still killed exactly as
    /// before.
    pub fn kill(&self) {
        self.kill_signal.notify_one();

        if self.exited.load(Ordering::SeqCst) {
            return;
        }

        let Some(pid) = self.pid else {
            return;
        };
        if let Some(process) = refreshed_process_snapshot(pid).process(sysinfo::Pid::from_u32(pid)) {
            process.kill();
        }
    }

    /// PID of the spawned language server process, for system-usage attribution
    /// (`domain::system::commands::system_usage_breakdown`).
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Whether the wait task spawned in [`spawn`] has observed this process exit. Lets a shutdown
    /// sequence poll for real process death (bounded by its own timeout) instead of blindly
    /// sleeping for a fixed duration regardless of how quickly the server actually exits.
    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }
}

/// Refreshes a single process's `sysinfo` snapshot in isolation, so callers can look `pid` up on
/// the returned `System` right afterward. Shared by [`LspProcHandle::kill`] (to find the process to
/// kill) and this module's tests (to confirm it's actually gone) — mirrors
/// `domain::ide::lockfile::is_pid_alive`'s refresh-then-lookup idiom, kept local here rather than
/// imported since that function lives in a `domain` module `infra` must not depend on.
fn refreshed_process_snapshot(pid: u32) -> sysinfo::System {
    let mut system = sysinfo::System::new();
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::Pid::from_u32(pid)]),
        true,
        sysinfo::ProcessRefreshKind::nothing(),
    );
    system
}

pub fn encode_message(payload: &str) -> Vec<u8> {
    let body = payload.as_bytes();
    let mut framed = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    framed.extend_from_slice(body);
    framed
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(HEADER_BODY_SEPARATOR.len())
        .position(|window| window == HEADER_BODY_SEPARATOR)
}

fn parse_content_length(header_block: &str) -> Option<usize> {
    header_block.split("\r\n").find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().to_ascii_lowercase() == CONTENT_LENGTH_HEADER {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

/// Receive-side framing buffer for one language server's stdout: bytes read so far plus a cursor
/// marking how much of them earlier [`take_message`](Self::take_message) calls already handed out.
///
/// The cursor is what keeps framing linear. The previous shape (a free function over `&mut Vec<u8>`)
/// called `Vec::drain(0..body_end)` once per message, memmoving everything still buffered down by the
/// size of the message just taken — so one 64KB read carrying *k* messages moved its tail *k* times
/// (§2 L-3). Here taking a message only advances an integer, and bytes move only in
/// [`compact`](Self::compact), under a condition that makes the amortized cost per byte constant. The
/// ordinary case never moves anything: draining a read to completion leaves the cursor at the end,
/// which resets the buffer to empty outright.
#[derive(Default)]
pub struct MessageBuffer {
    data: Vec<u8>,
    consumed: usize,
}

impl MessageBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extend(&mut self, chunk: &[u8]) {
        self.data.extend_from_slice(chunk);
    }

    /// Takes the next complete message, or `None` while the buffer holds no whole frame yet.
    ///
    /// A header block carrying no parseable `Content-Length` (or one that isn't UTF-8) cannot be
    /// framed, so it is **dropped** and the scan continues at the bytes after it. Leaving it in place
    /// and returning `None` — the previous behavior — wedged the session permanently (§4-A-11): every
    /// later read re-found that same malformed block at the front of the buffer, so no message ever
    /// came out again while the buffer grew for as long as the server kept talking. Non-protocol noise
    /// on stdout (a panic report, a stray log line followed by a blank line) is the realistic source,
    /// and dropping it costs at most the noise itself. A body that isn't valid UTF-8 is skipped the
    /// same way — that frame was already consumed by length, so the scan may as well continue to the
    /// next message instead of stalling until more bytes happen to arrive.
    pub fn take_message(&mut self) -> Option<String> {
        loop {
            let header_end = find_header_end(&self.data[self.consumed..])?;
            let header_start = self.consumed;
            let body_start = header_start + header_end + HEADER_BODY_SEPARATOR.len();
            let content_length = std::str::from_utf8(&self.data[header_start..header_start + header_end])
                .ok()
                .and_then(parse_content_length);

            let Some(content_length) = content_length else {
                self.consumed = body_start;
                self.compact();
                continue;
            };

            let body_end = body_start + content_length;
            if self.data.len() < body_end {
                return None;
            }

            let body = self.data[body_start..body_end].to_vec();
            self.consumed = body_end;
            self.compact();

            if let Ok(message) = String::from_utf8(body) {
                return Some(message);
            }
        }
    }

    /// Drops the consumed prefix once it is at least as large as the bytes that would have to move,
    /// so a byte is copied at most once per its own length of consumed data — amortized O(1) per
    /// byte, against the unconditional per-message move the old `drain` did.
    fn compact(&mut self) {
        if self.consumed == 0 {
            return;
        }
        if self.consumed == self.data.len() {
            self.data.clear();
            self.consumed = 0;
            return;
        }
        if self.consumed >= self.data.len() - self.consumed {
            self.data.drain(0..self.consumed);
            self.consumed = 0;
        }
    }
}

pub fn spawn<D, X>(config: LspProcConfig, on_message: D, on_exit: X) -> AppResult<LspProcHandle>
where
    D: Fn(String) + Send + 'static,
    X: FnOnce(Option<i32>) + Send + 'static,
{
    let mut command = Command::new(&config.command);
    command
        .args(&config.args)
        .current_dir(&config.cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let mut child = command.spawn().map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.serverSpawnFailed",
            format!("failed to start language server ({}): {error}", config.command),
        )
        .with_arg("command", &config.command)
        .with_arg("detail", &error)
    })?;
    let pid = child.id();

    let stdin = child.stdin.take().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.stdinUnavailable",
            "could not open the language server's stdin",
        )
    })?;
    let mut stdout = child.stdout.take().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.lsp.stdoutUnavailable",
            "could not open the language server's stdout",
        )
    })?;

    tokio::spawn(async move {
        let mut buffer = MessageBuffer::new();
        let mut read_buf = [0u8; LSP_READ_BUFFER_BYTES];

        loop {
            match stdout.read(&mut read_buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buffer.extend(&read_buf[..n]);
                    while let Some(message) = buffer.take_message() {
                        on_message(message);
                    }
                }
            }
        }
    });

    let kill_signal = Arc::new(Notify::new());
    let kill_for_wait = kill_signal.clone();
    let exited = Arc::new(AtomicBool::new(false));
    let exited_for_wait = exited.clone();

    // Parks on process death and a kill request at once instead of waking every
    // `LSP_WAIT_POLL_MS` to `try_wait` (§2 L-3): an idle language server costs this task nothing,
    // and a real exit is observed the moment it happens rather than up to a poll later. `wait` is
    // cancel-safe, so dropping it when the kill branch wins loses nothing — the second `wait` below
    // still reaps the child.
    tokio::spawn(async move {
        let waited = tokio::select! {
            status = child.wait() => Some(status),
            _ = kill_for_wait.notified() => None,
        };

        let status = match waited {
            Some(status) => status,
            None => {
                let _ = child.start_kill();
                child.wait().await
            }
        };

        exited_for_wait.store(true, Ordering::SeqCst);
        on_exit(status.ok().and_then(|status| status.code()));
    });

    Ok(LspProcHandle {
        stdin: tokio::sync::Mutex::new(stdin),
        kill_signal,
        exited,
        pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn buffer_of(bytes: &[u8]) -> MessageBuffer {
        let mut buffer = MessageBuffer::new();
        buffer.extend(bytes);
        buffer
    }

    #[test]
    fn 헤더가_덜_도착하면_메시지를_만들지_않는다() {
        let mut buffer = buffer_of(b"Content-Length: 11\r\n");

        assert_eq!(buffer.take_message(), None);

        buffer.extend(b"\r\nhello world");
        assert_eq!(buffer.take_message(), Some("hello world".to_string()));
    }

    #[test]
    fn 바디가_덜_도착하면_메시지를_만들지_않는다() {
        let mut buffer = buffer_of(b"Content-Length: 11\r\n\r\nhello");

        assert_eq!(buffer.take_message(), None);

        buffer.extend(b" world");
        assert_eq!(buffer.take_message(), Some("hello world".to_string()));
    }

    #[test]
    fn 한_버퍼에_여러_메시지가_있으면_하나씩_꺼낸다() {
        let mut buffer = buffer_of(b"Content-Length: 5\r\n\r\nhello");
        buffer.extend(b"Content-Length: 5\r\n\r\nworld");

        assert_eq!(buffer.take_message(), Some("hello".to_string()));
        assert_eq!(buffer.take_message(), Some("world".to_string()));
        assert_eq!(buffer.take_message(), None);
    }

    #[test]
    fn 헤더_이름_대소문자를_구분하지_않는다() {
        let mut buffer = buffer_of(b"content-length: 5\r\n\r\nhello");
        assert_eq!(buffer.take_message(), Some("hello".to_string()));

        let mut mixed = buffer_of(b"CoNtEnT-LeNgTh: 5\r\n\r\nhello");
        assert_eq!(mixed.take_message(), Some("hello".to_string()));
    }

    #[test]
    fn 인코딩한_메시지는_다시_디코딩된다() {
        let payload = "{\"jsonrpc\":\"2.0\",\"method\":\"initialized\"}";
        let mut buffer = buffer_of(&encode_message(payload));

        assert_eq!(buffer.take_message(), Some(payload.to_string()));
    }

    #[test]
    fn content_type_헤더가_섞여도_content_length만_읽는다() {
        let mut buffer = buffer_of(b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: 5\r\n\r\nhello");
        assert_eq!(buffer.take_message(), Some("hello".to_string()));
    }

    /// §4-A-11 regression: a header block with no `Content-Length` used to stay at the front of the
    /// buffer forever, so every message that arrived after it — for the rest of the session — was
    /// never framed. The assertion after the second `extend` is the one that fails without the drain:
    /// the scan keeps re-finding the same malformed block and never reaches `hello`.
    #[test]
    fn content_length_없는_헤더_블록은_버리고_다음_프레임을_계속_찾는다() {
        let mut buffer = buffer_of(b"X-Server-Note: starting up\r\n\r\n");

        assert_eq!(buffer.take_message(), None);

        buffer.extend(b"Content-Length: 5\r\n\r\nhello");
        assert_eq!(buffer.take_message(), Some("hello".to_string()));
    }

    #[test]
    fn 불량_헤더가_같은_읽기에_섞여_와도_뒤따르는_메시지를_꺼낸다() {
        let mut buffer = buffer_of(b"panic: server exploded\r\n\r\nContent-Length: 5\r\n\r\nhello");

        assert_eq!(buffer.take_message(), Some("hello".to_string()));
        assert_eq!(buffer.take_message(), None);
    }

    /// The malformed block is dropped, never the frames around it: a valid message before the noise
    /// still comes out first, and one after it still comes out in order.
    #[test]
    fn 불량_헤더는_앞뒤의_정상_메시지를_잡아먹지_않는다() {
        let mut buffer = buffer_of(b"Content-Length: 5\r\n\r\nfirst");
        buffer.extend(b"garbage-without-length\r\n\r\n");
        buffer.extend(b"Content-Length: 6\r\n\r\nsecond");

        assert_eq!(buffer.take_message(), Some("first".to_string()));
        assert_eq!(buffer.take_message(), Some("second".to_string()));
        assert_eq!(buffer.take_message(), None);
    }

    #[test]
    fn utf8가_아닌_바디는_건너뛰고_다음_메시지를_꺼낸다() {
        let mut buffer = MessageBuffer::new();
        buffer.extend(b"Content-Length: 2\r\n\r\n\xff\xfe");
        buffer.extend(b"Content-Length: 5\r\n\r\nhello");

        assert_eq!(buffer.take_message(), Some("hello".to_string()));
        assert_eq!(buffer.take_message(), None);
    }

    /// Exercises the consumed-cursor path across compaction: messages are taken while the buffer
    /// still holds a partial frame, then the rest arrives. A cursor that compacted without rebasing
    /// (or one that never advanced) would slice the wrong bytes here.
    #[test]
    fn 부분_프레임이_남은_채로_메시지를_꺼내도_이어지는_바이트를_잃지_않는다() {
        let mut buffer = MessageBuffer::new();
        buffer.extend(b"Content-Length: 5\r\n\r\nhello");
        buffer.extend(b"Content-Length: 5\r\n\r\nwor");

        assert_eq!(buffer.take_message(), Some("hello".to_string()));
        assert_eq!(buffer.take_message(), None);

        buffer.extend(b"ld");
        assert_eq!(buffer.take_message(), Some("world".to_string()));

        for index in 0..64 {
            buffer.extend(&encode_message(&format!("message-{index}")));
            assert_eq!(buffer.take_message(), Some(format!("message-{index}")));
        }
        assert_eq!(buffer.take_message(), None);
    }

    /// Real-process exercise of the `exited` flag `domain::lsp::commands::shutdown_entry`'s
    /// polling loop reads via [`LspProcHandle::is_exited`]. `sh -c "exit 0"` is a real child
    /// process, not a mock, so this genuinely proves the wait task's `child.wait()` branch flips the
    /// flag on real process death rather than only on the `kill_requested` path.
    #[cfg(unix)]
    #[tokio::test]
    async fn 프로세스가_스스로_종료하면_is_exited가_true로_바뀐다() {
        let config = LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "exit 0".to_string()],
            cwd: std::env::temp_dir(),
        };

        let handle = spawn(config, |_message| {}, |_code| {}).expect("프로세스 spawn 성공");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while !handle.is_exited() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(handle.is_exited(), "짧게 실행되고 종료하는 프로세스는 곧 감지되어야 한다");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn kill을_호출하지_않으면_is_exited는_계속_실행중인_프로세스에서_false를_유지한다() {
        let config = LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: std::env::temp_dir(),
        };

        let handle = spawn(config, |_message| {}, |_code| {}).expect("프로세스 spawn 성공");
        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(!handle.is_exited(), "아직 실행 중인 프로세스는 종료로 감지되면 안 된다");
        handle.kill();
    }

    /// Proves `kill()` sends the OS-level kill on its own call stack rather than only signaling
    /// `kill_signal` and relying on `spawn`'s background wait task to notice it later. There is no `.await` between `spawn` and the assertion below, so — on the
    /// single-threaded test runtime `#[tokio::test]` defaults to — that background task has had zero
    /// opportunity to run by the time `kill()` returns: nothing here has yielded to the scheduler.
    /// Under the old "set a flag and hope the poll loop gets to it" implementation the process would
    /// therefore still be observed `Run`/`Sleep`; under this fix it must already be signaled (at
    /// minimum `Zombie`, pending this same background task's own `wait()` reaping it) purely from
    /// `kill()`'s own synchronous `sysinfo` call.
    #[cfg(unix)]
    #[tokio::test]
    async fn kill은_os_시그널을_보내_프로세스를_종료시킨다() {
        let config = LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: std::env::temp_dir(),
        };
        let handle = spawn(config, |_message| {}, |_code| {}).expect("프로세스 spawn 성공");
        let pid = handle.pid().expect("pid 확보");

        handle.kill();

        const KILL_OBSERVE_TIMEOUT_MS: u64 = 2_000;
        const KILL_OBSERVE_POLL_MS: u64 = 20;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(KILL_OBSERVE_TIMEOUT_MS);
        let mut still_running = true;
        while tokio::time::Instant::now() < deadline {
            still_running = refreshed_process_snapshot(pid)
                .process(sysinfo::Pid::from_u32(pid))
                .is_some_and(|process| matches!(process.status(), sysinfo::ProcessStatus::Run | sysinfo::ProcessStatus::Sleep));
            if !still_running {
                break;
            }
            tokio::time::sleep(Duration::from_millis(KILL_OBSERVE_POLL_MS)).await;
        }

        assert!(
            !still_running,
            "kill() 이 OS 시그널을 보내 프로세스를 종료시켜야 한다 (시그널을 안 보내면 sleep 5 가 이 관찰 창을 넘겨 살아남는다)"
        );
    }

    /// Covers the wait task's kill branch end to end: `kill()`'s notification must wake it out of
    /// `child.wait()`, and it must still reap the child and flip `exited` afterward — the guarantee
    /// `domain::lsp::commands::shutdown_entry` polls for and `confirms_healthy_restart` reads. A
    /// select branch that forgot the second `wait()`/`store` would leave this handle reporting a
    /// killed process as still running forever.
    #[cfg(unix)]
    #[tokio::test]
    async fn kill_이후에는_wait_태스크가_프로세스를_거두고_is_exited를_세운다() {
        let config = LspProcConfig {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 5".to_string()],
            cwd: std::env::temp_dir(),
        };
        let handle = spawn(config, |_message| {}, |_code| {}).expect("프로세스 spawn 성공");

        handle.kill();

        const EXIT_OBSERVE_TIMEOUT_MS: u64 = 2_000;
        const EXIT_OBSERVE_POLL_MS: u64 = 10;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(EXIT_OBSERVE_TIMEOUT_MS);
        while !handle.is_exited() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(EXIT_OBSERVE_POLL_MS)).await;
        }

        assert!(handle.is_exited(), "kill 된 프로세스는 wait 태스크가 거두어 종료로 표시해야 한다");
    }

    /// Reproduces the PID-reuse hazard directly: a handle whose `exited` flag is already `true`
    /// (as if the wait task had long since reaped it — the state a crash-abandoned or already
    /// shut-down session sits in indefinitely inside `LspStore`) must not touch the OS process
    /// currently living at its stale `pid`, even though that PID number resolves to a real,
    /// unrelated, still-running process (`victim` below stands in for whatever process the OS
    /// reassigned the freed PID to). Constructs `LspProcHandle` directly (private-field struct
    /// literal, valid because this `tests` module is a descendant of the defining module) instead
    /// of going through [`spawn`], since the whole point is a handle whose `pid` and `exited` are
    /// inconsistent with each other in exactly the way a real reap-then-reuse race produces.
    #[cfg(unix)]
    #[tokio::test]
    async fn 이미_종료로_표시된_핸들의_kill은_같은_pid를_점유한_무관한_프로세스를_건드리지_않는다() {
        let mut victim = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("sleep 5")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("victim 프로세스 spawn 성공");
        let victim_pid = victim.id().expect("victim pid 확보");
        let victim_stdin = victim.stdin.take().expect("victim stdin 확보");

        let stale_handle = LspProcHandle {
            stdin: tokio::sync::Mutex::new(victim_stdin),
            kill_signal: Arc::new(Notify::new()),
            exited: Arc::new(AtomicBool::new(true)),
            pid: Some(victim_pid),
        };

        stale_handle.kill();

        let victim_still_running = refreshed_process_snapshot(victim_pid)
            .process(sysinfo::Pid::from_u32(victim_pid))
            .is_some_and(|process| matches!(process.status(), sysinfo::ProcessStatus::Run | sysinfo::ProcessStatus::Sleep));
        assert!(
            victim_still_running,
            "exited==true 인 핸들의 kill()은 재사용된 pid의 무관한 프로세스를 죽이면 안 된다"
        );

        let _ = victim.start_kill();
        let _ = victim.wait().await;
    }
}
