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

    pub fn kill(&self) {
        self.kill_requested.store(true, Ordering::SeqCst);
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
}
