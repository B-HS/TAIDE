use std::future::{poll_fn, Future};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::Poll;

use parking_lot::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::Notify;

use taide_model::error::{AppError, AppErrorKind, AppResult};

const HEADER_BODY_SEPARATOR: &[u8] = b"\r\n\r\n";
const CONTENT_LENGTH_HEADER: &str = "content-length";
const LSP_READ_BUFFER_BYTES: usize = 64 * 1024;
/// Upper bound on the stderr this module keeps per language server: only the newest
/// [`LSP_STDERR_TAIL_BYTES`] bytes survive, so a server that logs for hours costs a fixed amount of
/// memory instead of growing without bound. The tail exists to name *why* a server died in
/// `domain::lsp::commands::handle_process_exit`'s exit log, and the newest bytes are where that
/// answer lives (the panic, the missing runtime, the rejected flag).
const LSP_STDERR_TAIL_BYTES: usize = 4 * 1024;
/// Read chunk for the stderr reader task, smaller than [`LSP_READ_BUFFER_BYTES`] because stderr
/// carries occasional diagnostics rather than the protocol stream.
const LSP_STDERR_READ_BUFFER_BYTES: usize = 8 * 1024;
const LSP_READER_DRAIN_TIMEOUT_MS: u64 = 500;

struct ReaderTask(tokio::task::JoinHandle<()>);

impl ReaderTask {
    fn new(task: tokio::task::JoinHandle<()>) -> Self {
        Self(task)
    }

    async fn finish(mut self) {
        if tokio::time::timeout(tokio::time::Duration::from_millis(LSP_READER_DRAIN_TIMEOUT_MS), &mut self.0)
            .await
            .is_err()
        {
            self.0.abort();
            let _ = (&mut self.0).await;
        }
    }
}

impl Drop for ReaderTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub struct LspProcConfig {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

/// The newest [`LSP_STDERR_TAIL_BYTES`] bytes a language server wrote to stderr, shared between
/// [`spawn`]'s reader task (which appends) and the exit report (which snapshots). Before this
/// existed stderr was `Stdio::null()`, so a server that failed to start — a missing `node` for a
/// `#!/usr/bin/env node` shim, a rejected flag, a panic — said exactly why on a stream nothing
/// read, and the app could only report the exit code (d-64 §0.1).
#[derive(Default)]
struct StderrTail {
    bytes: Vec<u8>,
}

impl StderrTail {
    fn push(&mut self, chunk: &[u8]) {
        if chunk.len() >= LSP_STDERR_TAIL_BYTES {
            self.bytes.clear();
            self.bytes.extend_from_slice(&chunk[chunk.len() - LSP_STDERR_TAIL_BYTES..]);
            return;
        }

        self.bytes.extend_from_slice(chunk);
        let overflow = self.bytes.len().saturating_sub(LSP_STDERR_TAIL_BYTES);
        if overflow > 0 {
            self.bytes.drain(0..overflow);
        }
    }

    /// Lossy on purpose: the bound cuts at a byte offset, which can land mid-character, and a
    /// replacement character inside one log line beats dropping the whole tail.
    fn snapshot(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
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
    stderr_tail: Arc<Mutex<StderrTail>>,
    wait_gate: Arc<Mutex<bool>>,
    wait_task: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl LspProcHandle {
    pub async fn write_message(&self, payload: &str) -> AppResult<()> {
        let mut stdin = self.stdin.lock().await;
        let framed = encode_message(payload);
        stdin.write_all(&framed).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Requests termination synchronously while serializing numeric PID use with child wait polling.
    /// The gate closes before a reaped PID can be reused or the child wait owner is dropped.
    /// Actual reader and worker completion must be awaited separately with wait_for_completion.
    pub fn kill(&self) {
        let can_signal = self.wait_gate.lock();
        self.kill_signal.notify_one();

        if !*can_signal || self.exited.load(Ordering::SeqCst) {
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

    /// The newest bytes this server wrote to stderr, at most `LSP_STDERR_TAIL_BYTES` of them.
    /// Process output, so a caller that logs or shows it must mask it first
    /// (`domain::lsp::commands::masked_stderr_tail`) — the same policy the toolchain-install tail
    /// follows (d-57 §1.D).
    pub fn stderr_tail(&self) -> String {
        self.stderr_tail.lock().snapshot()
    }

    /// Whether the wait task spawned in [`spawn`] has observed this process exit. Lets a shutdown
    /// sequence poll for real process death (bounded by its own timeout) instead of blindly
    /// sleeping for a fixed duration regardless of how quickly the server actually exits.
    pub fn is_exited(&self) -> bool {
        self.exited.load(Ordering::SeqCst)
    }

    /// Reports whether the owned wait worker has ended, including cancellation or panic.
    /// Successful completion includes child reaping, both readers, and the exit callback.
    pub fn is_finished(&self) -> bool {
        match self.wait_task.try_lock() {
            Ok(task) => task.as_ref().is_none_or(|task| task.is_finished()),
            Err(_) => false,
        }
    }

    /// Waits for the owned worker without losing its handle when this waiting future is dropped.
    /// Worker failure is not surfaced here and does not prove successful child reaping.
    pub async fn wait_for_completion(&self) {
        let mut task = self.wait_task.lock().await;
        if let Some(task) = task.as_mut() {
            task.await.ok();
        }
        task.take();
    }
}

impl Drop for LspProcHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

struct ChildWaitOwner {
    child: tokio::process::Child,
    can_signal: Arc<Mutex<bool>>,
}

impl Drop for ChildWaitOwner {
    fn drop(&mut self) {
        let mut can_signal = self.can_signal.lock();
        if *can_signal {
            self.child.start_kill().ok();
        }
        *can_signal = false;
    }
}

async fn wait_for_child(
    child: &mut tokio::process::Child,
    wait_gate: &Mutex<bool>,
    exited: &AtomicBool,
) -> std::io::Result<std::process::ExitStatus> {
    let mut waiting = std::pin::pin!(child.wait());
    poll_fn(|context| {
        let mut can_signal = wait_gate.lock();
        let status = waiting.as_mut().poll(context);
        if status.is_ready() {
            *can_signal = false;
        }
        if matches!(&status, Poll::Ready(Ok(_))) {
            exited.store(true, Ordering::SeqCst);
        }
        status
    })
    .await
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
/// `compact`, under a condition that makes the amortized cost per byte constant. The
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

/// Starts one language server process and wires its three streams: framed messages out of stdout
/// to `on_message`, writes in through [`LspProcHandle::write_message`], and stderr into a bounded
/// tail.
///
/// `on_exit` receives the exit code and that tail. The tail is passed *by value* rather than read
/// back off the handle at exit time because the handle is installed on the session
/// by its caller only after this function returns: a server that
/// dies immediately — the very case the tail is for — would otherwise have its exit observed while
/// the session still holds the previous process's handle, or none at all.
pub fn spawn<D, X>(config: LspProcConfig, on_message: D, on_exit: X) -> AppResult<LspProcHandle>
where
    D: Fn(String) + Send + 'static,
    X: FnOnce(Option<i32>, String) + Send + 'static,
{
    let mut command = Command::new(&config.command);
    command
        .args(&config.args)
        .current_dir(&config.cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

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

    let stderr_tail = Arc::new(Mutex::new(StderrTail::default()));
    let stderr_reader = child.stderr.take().map(|mut stderr| {
        let tail = stderr_tail.clone();
        ReaderTask::new(tokio::spawn(async move {
            let mut read_buf = [0u8; LSP_STDERR_READ_BUFFER_BYTES];

            loop {
                match stderr.read(&mut read_buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => tail.lock().push(&read_buf[..n]),
                }
            }
        }))
    });

    let stdout_reader = ReaderTask::new(tokio::spawn(async move {
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
    }));

    let kill_signal = Arc::new(Notify::new());
    let kill_for_wait = kill_signal.clone();
    let exited = Arc::new(AtomicBool::new(false));
    let exited_for_wait = exited.clone();
    let stderr_tail_for_exit = stderr_tail.clone();
    let wait_gate = Arc::new(Mutex::new(true));
    let gate_for_wait = wait_gate.clone();
    let owner = ChildWaitOwner {
        child,
        can_signal: gate_for_wait.clone(),
    };
    let wait_task = tokio::spawn(async move {
        let mut owner = owner;
        let waited = tokio::select! {
            status = wait_for_child(&mut owner.child, &gate_for_wait, &exited_for_wait) => Some(status),
            _ = kill_for_wait.notified() => None,
        };

        let status = match waited {
            Some(status) => status,
            None => {
                let _ = owner.child.start_kill();
                wait_for_child(&mut owner.child, &gate_for_wait, &exited_for_wait).await
            }
        };

        let stderr_finish = async move {
            if let Some(reader) = stderr_reader {
                reader.finish().await;
            }
        };
        tokio::join!(stdout_reader.finish(), stderr_finish);

        let tail = stderr_tail_for_exit.lock().snapshot();
        on_exit(status.ok().and_then(|status| status.code()), tail);
    });

    Ok(LspProcHandle {
        stdin: tokio::sync::Mutex::new(stdin),
        kill_signal,
        exited,
        pid,
        stderr_tail,
        wait_gate,
        wait_task: tokio::sync::Mutex::new(Some(wait_task)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const READER_TEST_TIMEOUT_SECS: u64 = 2;
    #[cfg(unix)]
    const OWNED_CHILD_DURATION_SECONDS: u64 = 30;

    #[cfg(unix)]
    #[tokio::test]
    async fn 마지막_핸들_drop은_직접_소유한_child의_종료를_요청한다() {
        let mut child = tokio::process::Command::new("sleep")
            .arg(OWNED_CHILD_DURATION_SECONDS.to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let handle = LspProcHandle {
            stdin: tokio::sync::Mutex::new(child.stdin.take().unwrap()),
            kill_signal: Arc::new(Notify::new()),
            exited: Arc::new(AtomicBool::new(false)),
            pid: child.id(),
            stderr_tail: Arc::new(Mutex::new(StderrTail::default())),
            wait_gate: Arc::new(Mutex::new(true)),
            wait_task: tokio::sync::Mutex::new(None),
        };
        drop(handle);
        let exited = tokio::time::timeout(Duration::from_secs(READER_TEST_TIMEOUT_SECS), child.wait()).await;
        if exited.is_err() {
            child.start_kill().ok();
            child.wait().await.ok();
        }
        assert!(exited.is_ok(), "dropping the owner left its direct child alive");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 완료_대기_future_drop은_작업_핸들을_보유해_재대기할_수_있다() {
        let handle = spawn(
            LspProcConfig {
                command: "sleep".to_string(),
                args: vec![OWNED_CHILD_DURATION_SECONDS.to_string()],
                cwd: std::env::temp_dir(),
            },
            |_| {},
            |_, _| {},
        )
        .unwrap();
        {
            let mut waiting = std::pin::pin!(handle.wait_for_completion());
            poll_fn(|context| {
                assert!(waiting.as_mut().poll(context).is_pending());
                Poll::Ready(())
            })
            .await;
            assert!(handle.wait_task.try_lock().is_err());
        }
        assert!(handle.wait_task.try_lock().unwrap().is_some());
        assert!(!handle.is_finished());
        handle.kill();
        tokio::time::timeout(Duration::from_secs(READER_TEST_TIMEOUT_SECS), handle.wait_for_completion())
            .await
            .unwrap();
        assert!(handle.is_exited());
        assert!(handle.is_finished());
        assert!(handle.wait_task.try_lock().unwrap().is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 미poll_wait_owner_취소는_pid_시그널_권한을_반납한다() {
        let child = Command::new("sleep")
            .arg(OWNED_CHILD_DURATION_SECONDS.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let can_signal = Arc::new(Mutex::new(true));
        let owner = ChildWaitOwner {
            child,
            can_signal: can_signal.clone(),
        };
        let task = tokio::spawn(async move {
            let _owner = owner;
            std::future::pending::<()>().await;
        });
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(!*can_signal.lock());
    }

    async fn await_reader_drop(receiver: tokio::sync::oneshot::Receiver<()>) {
        assert!(tokio::time::timeout(Duration::from_secs(READER_TEST_TIMEOUT_SECS), receiver)
            .await
            .expect("reader 캡처를 제한 시간 안에 회수해야 합니다")
            .is_err());
    }

    #[tokio::test]
    async fn reader_정상_완료는_결과를_기다리고_캡처를_회수한다() {
        let (sent, received) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            sent.send("drained").unwrap();
        });
        ReaderTask::new(task).finish().await;
        assert_eq!(received.await.unwrap(), "drained");
    }

    #[tokio::test]
    async fn reader_eof_지연은_deadline_뒤_abort와_join으로_회수한다() {
        let (started, ready) = tokio::sync::oneshot::channel();
        let (retained, dropped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let _retained = retained;
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        let reader = ReaderTask::new(task);
        ready.await.unwrap();
        tokio::time::timeout(Duration::from_secs(READER_TEST_TIMEOUT_SECS), reader.finish())
            .await
            .expect("EOF 지연에도 reader 회수는 끝나야 합니다");
        await_reader_drop(dropped).await;
    }

    #[tokio::test]
    async fn reader_owner를_poll_전에_버려도_작업을_detach하지_않는다() {
        let (retained, dropped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let _retained = retained;
            std::future::pending::<()>().await;
        });
        drop(ReaderTask::new(task));
        await_reader_drop(dropped).await;
    }

    #[tokio::test]
    async fn 부모_worker가_취소되면_소유한_reader를_같이_회수한다() {
        let (started, ready) = tokio::sync::oneshot::channel();
        let (retained, dropped) = tokio::sync::oneshot::channel::<()>();
        let reader = ReaderTask::new(tokio::spawn(async move {
            let _retained = retained;
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        }));
        let (parent_started, parent_ready) = tokio::sync::oneshot::channel();
        let parent = tokio::spawn(async move {
            parent_started.send(()).unwrap();
            reader.finish().await;
        });
        ready.await.unwrap();
        parent_ready.await.unwrap();
        parent.abort();
        assert!(parent.await.unwrap_err().is_cancelled());
        await_reader_drop(dropped).await;
    }

    #[test]
    fn 실제_reader_소유권과_회수는_child_exit와_콜백_사이에_배치된다() {
        let source = include_str!("lsp_proc.rs");
        let spawn = source
            .split_once("pub fn spawn<D, X>(")
            .unwrap()
            .1
            .split_once("#[cfg(test)]")
            .unwrap()
            .0;
        assert_eq!(spawn.matches("ReaderTask::new(tokio::spawn(").count(), 2);
        assert!(spawn.contains("stdout_reader.finish()"));
        assert!(spawn.contains("reader.finish().await"));
        let exited = spawn.find("status = wait_for_child(&mut owner.child").unwrap();
        let drain = spawn.find("tokio::join!(").unwrap();
        let report = spawn.find("on_exit(status.ok()").unwrap();
        assert!(exited < drain && drain < report);
        let wait = source.split_once("async fn wait_for_child(").unwrap().1;
        let gate = wait.find("let mut can_signal = wait_gate.lock()").unwrap();
        let poll = wait.find("waiting.as_mut().poll(context)").unwrap();
        let exited = wait.find("exited.store(true, Ordering::SeqCst)").unwrap();
        assert!(gate < poll && poll < exited);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stdout_프레임과_stderr_tail은_자식_종료_콜백_전에_드레인된다() {
        let saw_message = Arc::new(AtomicBool::new(false));
        let message_flag = saw_message.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let handle = spawn(
            LspProcConfig {
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    "printf 'Content-Length: 5\\r\\n\\r\\nhello'; printf 'fixture stderr' >&2; exit 0".to_string(),
                ],
                cwd: std::env::temp_dir(),
            },
            move |message| {
                assert_eq!(message, "hello");
                message_flag.store(true, Ordering::SeqCst);
            },
            move |code, tail| {
                sender.send((saw_message.load(Ordering::SeqCst), code, tail)).unwrap();
            },
        )
        .unwrap();
        let (was_message_drained, code, tail) = tokio::time::timeout(Duration::from_secs(READER_TEST_TIMEOUT_SECS), receiver)
            .await
            .expect("reader를 정리한 뒤 자식 종료를 보고해야 합니다")
            .unwrap();
        assert!(was_message_drained);
        assert_eq!(code, Some(0));
        assert_eq!(tail, "fixture stderr");
        assert!(handle.is_exited());
    }

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

        let handle = spawn(config, |_message| {}, |_code, _tail| {}).expect("프로세스 spawn 성공");

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

        let handle = spawn(config, |_message| {}, |_code, _tail| {}).expect("프로세스 spawn 성공");
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
        let handle = spawn(config, |_message| {}, |_code, _tail| {}).expect("프로세스 spawn 성공");
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
        let handle = spawn(config, |_message| {}, |_code, _tail| {}).expect("프로세스 spawn 성공");

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
            stderr_tail: Arc::new(Mutex::new(StderrTail::default())),
            wait_gate: Arc::new(Mutex::new(true)),
            wait_task: tokio::sync::Mutex::new(None),
        };

        stale_handle.kill();

        let victim_still_running = refreshed_process_snapshot(victim_pid)
            .process(sysinfo::Pid::from_u32(victim_pid))
            .is_some_and(|process| matches!(process.status(), sysinfo::ProcessStatus::Run | sysinfo::ProcessStatus::Sleep));
        assert!(
            victim_still_running,
            "exited==true 인 핸들의 kill()은 재사용된 pid의 무관한 프로세스를 죽이면 안 된다"
        );

        *stale_handle.wait_gate.lock() = false;
        stale_handle.exited.store(false, Ordering::SeqCst);
        stale_handle.kill();
        assert!(refreshed_process_snapshot(victim_pid)
            .process(sysinfo::Pid::from_u32(victim_pid))
            .is_some_and(|process| matches!(process.status(), sysinfo::ProcessStatus::Run | sysinfo::ProcessStatus::Sleep)));

        let _ = victim.start_kill();
        let _ = victim.wait().await;
    }

    /// The `sh` victim pattern above pointed at stderr: a server that says why it is dying must have
    /// that text reach the exit report, which is the whole point of piping stderr instead of
    /// `Stdio::null()` (d-64 §0.1 — the app could previously only report an exit code). Asserts the
    /// code alongside it so a callback that lost one of the two arguments cannot pass.
    #[cfg(unix)]
    #[tokio::test]
    async fn stderr에_쓴_뒤_종료한_프로세스의_tail이_종료_콜백에_실린다() {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let config = LspProcConfig {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "echo 'env: node: No such file or directory' 1>&2; exit 127".to_string(),
            ],
            cwd: std::env::temp_dir(),
        };

        let handle = spawn(
            config,
            |_message| {},
            move |code, tail| {
                let _ = sender.send((code, tail));
            },
        )
        .expect("프로세스 spawn 성공");

        const EXIT_CALLBACK_TIMEOUT_MS: u64 = 5_000;
        let (code, tail) = tokio::time::timeout(Duration::from_millis(EXIT_CALLBACK_TIMEOUT_MS), receiver)
            .await
            .expect("종료 콜백이 제때 호출되어야 한다")
            .expect("종료 콜백 결과 수신");

        assert_eq!(code, Some(127), "종료 코드는 그대로 전달되어야 한다");
        assert!(
            tail.contains("No such file or directory"),
            "stderr 내용이 tail 에 남아야 한다 (실제: {tail})"
        );
        assert!(
            handle.stderr_tail().contains("No such file or directory"),
            "핸들 접근자도 같은 tail 을 돌려줘야 한다"
        );
    }

    /// The bound is what keeps a chatty server from growing this buffer for the life of the session.
    /// Covers both ways it can be crossed: many small writes accumulating past it, and a single
    /// write larger than it.
    #[test]
    fn stderr_tail은_상한을_넘으면_오래된_앞부분부터_잘린다() {
        const CHUNK_BYTES: usize = 512;
        let mut tail = StderrTail::default();

        tail.push(b"oldest-line\n");
        for _ in 0..(LSP_STDERR_TAIL_BYTES / CHUNK_BYTES) {
            tail.push(&[b'x'; CHUNK_BYTES]);
        }

        let snapshot = tail.snapshot();
        assert_eq!(snapshot.len(), LSP_STDERR_TAIL_BYTES, "보관량이 상한을 넘으면 안 된다");
        assert!(
            !snapshot.contains("oldest-line"),
            "상한을 넘으면 가장 오래된 바이트부터 버려야 한다"
        );

        tail.push(&vec![b'y'; LSP_STDERR_TAIL_BYTES * 2]);

        let oversized = tail.snapshot();
        assert_eq!(
            oversized.len(),
            LSP_STDERR_TAIL_BYTES,
            "한 번에 상한보다 큰 청크가 와도 상한을 지켜야 한다"
        );
        assert!(oversized.chars().all(|char| char == 'y'), "상한보다 큰 청크는 그 꼬리만 남는다");
    }
}
