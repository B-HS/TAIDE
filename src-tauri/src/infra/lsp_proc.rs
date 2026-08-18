use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::error::{AppError, AppResult};

const HEADER_BODY_SEPARATOR: &[u8] = b"\r\n\r\n";
const CONTENT_LENGTH_HEADER: &str = "content-length";
const LSP_READ_BUFFER_BYTES: usize = 64 * 1024;
const LSP_WAIT_POLL_MS: u64 = 50;

pub struct LspProcConfig {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

pub struct LspProcHandle {
    stdin: tokio::sync::Mutex<tokio::process::ChildStdin>,
    kill_requested: Arc<AtomicBool>,
    /// Flipped to `true` by the wait loop below the instant `child.try_wait()`/`wait()`
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
    /// (`self.killer.lock().kill()`, no `.await`). Setting `kill_requested` alone used to be the
    /// entire implementation — the actual OS-level kill only happened later, inside [`spawn`]'s
    /// background wait-loop task, the next time its up-to-50ms poll noticed the flag. That
    /// asynchronous handoff is fine for the normal `lsp_stop`/`lsp_restart` shutdown path (which
    /// awaits `wait_for_process_exit` afterward), but at app exit `domain::lsp::commands::LspStore::
    /// kill_all` calls this synchronously from the `RunEvent::Exit` handler and the process then
    /// terminates via `std::process::exit` immediately after — which drops the tokio runtime without
    /// running its pending tasks, so the wait-loop task backing this handle's kill might never get
    /// scheduled at all, leaving the language server orphaned. Killing by PID here through `sysinfo`
    /// (already a workspace dependency — see `domain::ide::lockfile::is_pid_alive` for the same
    /// refresh-then-lookup idiom) closes that window: the OS-level kill happens on this call's own
    /// stack, before `kill_all`'s caller ever returns. `kill_requested` is still set so the wait loop
    /// (which usually *does* get to run, on the ordinary `lsp_stop`/`lsp_restart` paths) skips
    /// redundantly calling `child.start_kill()` itself once it next polls.
    ///
    /// Guarded by [`is_exited`](Self::is_exited): once the wait loop has observed this process exit,
    /// the OS has already reaped it and is free to hand this same numeric `pid` to an unrelated
    /// process. A handle can reach this state long before `kill()` is ever called on it — e.g. a
    /// crash-looped session `domain::lsp::commands::handle_process_exit` gives up on (leaving a
    /// reaped, never-cleared `entry.proc`) then sits in `LspStore` for the rest of a long-running
    /// session until app exit's `kill_all` sweeps every entry. Without this guard, that sweep would
    /// blindly `sysinfo`-kill whatever process the OS had since reassigned that `pid` to — silent
    /// collateral damage with no relation to this language server. A still-running process can never
    /// have `exited == true` (only this handle's own wait loop, which requires `try_wait`/`wait` to
    /// have first observed real process death, sets it), so live servers are still killed exactly as
    /// before.
    pub fn kill(&self) {
        self.kill_requested.store(true, Ordering::SeqCst);

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

    /// Whether the wait loop spawned in [`spawn`] has observed this process exit. Lets a shutdown
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

pub fn try_take_message(buffer: &mut Vec<u8>) -> Option<String> {
    let header_end = find_header_end(buffer)?;
    let header_block = std::str::from_utf8(&buffer[..header_end]).ok()?;
    let content_length = parse_content_length(header_block)?;

    let body_start = header_end + HEADER_BODY_SEPARATOR.len();
    let body_end = body_start + content_length;
    if buffer.len() < body_end {
        return None;
    }

    let body = buffer[body_start..body_end].to_vec();
    buffer.drain(0..body_end);
    String::from_utf8(body).ok()
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

    let mut child = command
        .spawn()
        .map_err(|error| AppError::Internal(format!("언어 서버를 시작하지 못했습니다 ({}): {error}", config.command)))?;
    let pid = child.id();

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Internal("언어 서버 stdin을 열지 못했습니다".to_string()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Internal("언어 서버 stdout을 열지 못했습니다".to_string()))?;

    tokio::spawn(async move {
        let mut buffer = Vec::new();
        let mut read_buf = [0u8; LSP_READ_BUFFER_BYTES];

        loop {
            match stdout.read(&mut read_buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&read_buf[..n]);
                    while let Some(message) = try_take_message(&mut buffer) {
                        on_message(message);
                    }
                }
            }
        }
    });

    let kill_requested = Arc::new(AtomicBool::new(false));
    let kill_for_wait = kill_requested.clone();
    let exited = Arc::new(AtomicBool::new(false));
    let exited_for_wait = exited.clone();

    tokio::spawn(async move {
        loop {
            if kill_for_wait.load(Ordering::SeqCst) {
                let _ = child.start_kill();
            }

            match child.try_wait() {
                Ok(Some(status)) => {
                    exited_for_wait.store(true, Ordering::SeqCst);
                    on_exit(status.code());
                    break;
                }
                Ok(None) => {
                    tokio::time::sleep(Duration::from_millis(LSP_WAIT_POLL_MS)).await;
                }
                Err(_) => {
                    exited_for_wait.store(true, Ordering::SeqCst);
                    on_exit(None);
                    break;
                }
            }
        }
    });

    Ok(LspProcHandle {
        stdin: tokio::sync::Mutex::new(stdin),
        kill_requested,
        exited,
        pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 헤더가_덜_도착하면_메시지를_만들지_않는다() {
        let mut buffer = b"Content-Length: 11\r\n".to_vec();
        let before = buffer.clone();

        assert_eq!(try_take_message(&mut buffer), None);
        assert_eq!(buffer, before);
    }

    #[test]
    fn 바디가_덜_도착하면_메시지를_만들지_않는다() {
        let mut buffer = b"Content-Length: 11\r\n\r\nhello".to_vec();
        let before = buffer.clone();

        assert_eq!(try_take_message(&mut buffer), None);
        assert_eq!(buffer, before);
    }

    #[test]
    fn 한_버퍼에_여러_메시지가_있으면_하나씩_꺼낸다() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"Content-Length: 5\r\n\r\nhello");
        buffer.extend_from_slice(b"Content-Length: 5\r\n\r\nworld");

        assert_eq!(try_take_message(&mut buffer), Some("hello".to_string()));
        assert_eq!(try_take_message(&mut buffer), Some("world".to_string()));
        assert_eq!(try_take_message(&mut buffer), None);
        assert!(buffer.is_empty());
    }

    #[test]
    fn 헤더_이름_대소문자를_구분하지_않는다() {
        let mut buffer = b"content-length: 5\r\n\r\nhello".to_vec();
        assert_eq!(try_take_message(&mut buffer), Some("hello".to_string()));

        let mut mixed = b"CoNtEnT-LeNgTh: 5\r\n\r\nhello".to_vec();
        assert_eq!(try_take_message(&mut mixed), Some("hello".to_string()));
    }

    #[test]
    fn 인코딩한_메시지는_다시_디코딩된다() {
        let payload = "{\"jsonrpc\":\"2.0\",\"method\":\"initialized\"}";
        let mut buffer = encode_message(payload);

        assert_eq!(try_take_message(&mut buffer), Some(payload.to_string()));
    }

    #[test]
    fn content_type_헤더가_섞여도_content_length만_읽는다() {
        let mut buffer = b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: 5\r\n\r\nhello".to_vec();
        assert_eq!(try_take_message(&mut buffer), Some("hello".to_string()));
    }

    /// Real-process exercise of the `exited` flag `domain::lsp::commands::shutdown_entry`'s
    /// polling loop reads via [`LspProcHandle::is_exited`]. `sh -c "exit 0"` is a real child
    /// process, not a mock, so this genuinely proves the wait loop's `try_wait` branch flips the
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

    /// Proves `kill()` sends the OS-level kill on its own call stack rather than only flipping
    /// `kill_requested` and relying on `spawn`'s background wait-loop task (up to 50ms polling) to
    /// notice it later. There is no `.await` between `spawn` and the assertion below, so — on the
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

    /// Reproduces the PID-reuse hazard directly: a handle whose `exited` flag is already `true`
    /// (as if the wait loop had long since reaped it — the state a crash-abandoned or already
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
            kill_requested: Arc::new(AtomicBool::new(false)),
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
