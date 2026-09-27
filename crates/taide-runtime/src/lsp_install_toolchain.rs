use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::{Condvar, Mutex};
#[cfg(unix)]
use taide_infra::owned_child::{has_exited_unreaped, kill_unreaped_child_group};
use taide_infra::redact::mask_known_secrets;
use taide_lsp::install::{install_cancelled_error, InstallCancellationResource, LspInstallLease};
use taide_lsp::service;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::lsp::{LanguageServerSpec, LspInstallPhase};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader, Lines};
use tokio::task::JoinHandle;

use crate::lsp_install_actions::{emit_install_progress, run_install_blocking_step};
use crate::{EventSink, TaskSupervisor};

const TOOLCHAIN_POLL_INTERVAL_MS: u64 = 100;
const TOOLCHAIN_OUTPUT_TAIL_LINES: usize = 20;
const TOOLCHAIN_READER_DRAIN_TIMEOUT_MS: u64 = 500;
#[cfg(unix)]
const MIN_SIGNALABLE_PGID: u32 = 2;

#[cfg(unix)]
fn should_signal_process_group(pid: u32) -> bool {
    pid >= MIN_SIGNALABLE_PGID
}

fn cancel_child(child: &mut Child) {
    #[cfg(unix)]
    {
        if !should_signal_process_group(child.id()) || has_exited_unreaped(child).is_err() {
            return;
        }
        kill_unreaped_child_group(child).ok();
        child.kill().ok();
    }
    #[cfg(not(unix))]
    {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        child.kill().ok();
    }
}

struct InstallChild {
    child: Mutex<Child>,
    is_reaped: std::sync::atomic::AtomicBool,
    changed: Condvar,
    _lease: LspInstallLease,
}

impl InstallChild {
    fn wait_for_exit(&self) -> AppResult<ExitStatus> {
        let mut child = self.child.lock();
        loop {
            #[cfg(unix)]
            if has_exited_unreaped(&mut child)? {
                kill_unreaped_child_group(&mut child)?;
                let status = child.wait()?;
                self.is_reaped.store(true, std::sync::atomic::Ordering::Release);
                return Ok(status);
            }
            #[cfg(not(unix))]
            match child.try_wait()? {
                Some(status) => {
                    self.is_reaped.store(true, std::sync::atomic::Ordering::Release);
                    return Ok(status);
                }
                None => {}
            }
            self.changed.wait_for(&mut child, Duration::from_millis(TOOLCHAIN_POLL_INTERVAL_MS));
        }
    }
}

impl InstallCancellationResource for InstallChild {
    fn cancel(&self) {
        let mut child = self.child.lock();
        if !self.is_reaped.load(std::sync::atomic::Ordering::Acquire) {
            cancel_child(&mut child);
        }
        self.changed.notify_all();
    }
}

impl Drop for InstallChild {
    fn drop(&mut self) {
        let child = self.child.get_mut();
        if !*self.is_reaped.get_mut() {
            cancel_child(child);
        }
        child.wait().ok();
    }
}

struct InstallReaderWork<R: AsyncRead + Unpin> {
    lines: Lines<BufReader<R>>,
    tail: Arc<Mutex<Vec<String>>>,
    _lease: LspInstallLease,
}

struct InstallOutputReader {
    task: JoinHandle<()>,
    tail: Arc<Mutex<Vec<String>>>,
}

impl InstallOutputReader {
    fn spawn(reader: impl AsyncRead + Unpin + Send + 'static, lease: LspInstallLease, tasks: &TaskSupervisor) -> AppResult<Self> {
        let tail = Arc::new(Mutex::new(Vec::new()));
        let owner = InstallReaderWork {
            lines: BufReader::new(reader).lines(),
            tail: tail.clone(),
            _lease: lease,
        };
        let task = tasks
            .spawn_transient_handle("lsp-install-output", async move {
                let mut owner = owner;
                while let Ok(Some(line)) = owner.lines.next_line().await {
                    let mut tail = owner.tail.lock();
                    tail.push(line);
                    if tail.len() > TOOLCHAIN_OUTPUT_TAIL_LINES {
                        tail.remove(0);
                    }
                }
            })
            .ok_or_else(install_cancelled_error)?;
        Ok(Self { task, tail })
    }

    async fn finish(mut self) -> Vec<String> {
        if tokio::time::timeout(Duration::from_millis(TOOLCHAIN_READER_DRAIN_TIMEOUT_MS), &mut self.task)
            .await
            .is_err()
        {
            self.task.abort();
            (&mut self.task).await.ok();
        }
        self.tail.lock().clone()
    }
}

impl Drop for InstallOutputReader {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn configure_toolchain_command(command: &mut Command) {
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        command.process_group(0);
    }
}

struct ToolchainOutcome {
    status: ExitStatus,
    stderr: Vec<String>,
    stdout: Vec<String>,
}

async fn run_toolchain_process(
    command: Command,
    binary: String,
    lease: &LspInstallLease,
    tasks: &TaskSupervisor,
) -> AppResult<ToolchainOutcome> {
    let runtime = tokio::runtime::Handle::current();
    let worker_lease = lease.clone();
    let worker_tasks = tasks.clone();
    run_install_blocking_step(tasks, lease, move || {
        let mut command = command;
        let child = worker_lease.register_resource(|| {
            let child = command.spawn().map_err(|error| {
                AppError::localized(
                    AppErrorKind::Internal,
                    "error.lsp.toolchainRunFailed",
                    format!("{binary} failed to run: {error}"),
                )
                .with_arg("binary", &binary)
                .with_arg("detail", &error)
            })?;
            Ok(Arc::new(InstallChild {
                child: Mutex::new(child),
                is_reaped: std::sync::atomic::AtomicBool::new(false),
                changed: Condvar::new(),
                _lease: worker_lease.clone(),
            }))
        })?;
        let (stdout, stderr) = {
            let _entered = runtime.enter();
            let mut process = child.child.lock();
            let stdout = process.stdout.take().map(tokio::process::ChildStdout::from_std).transpose()?;
            let stderr = process.stderr.take().map(tokio::process::ChildStderr::from_std).transpose()?;
            (
                stdout
                    .map(|reader| InstallOutputReader::spawn(reader, worker_lease.clone(), &worker_tasks))
                    .transpose()?,
                stderr
                    .map(|reader| InstallOutputReader::spawn(reader, worker_lease.clone(), &worker_tasks))
                    .transpose()?,
            )
        };
        let status = child.wait_for_exit()?;
        let (stderr, stdout) = runtime.block_on(async move {
            tokio::join!(
                async {
                    match stderr {
                        Some(reader) => reader.finish().await,
                        None => Vec::new(),
                    }
                },
                async {
                    match stdout {
                        Some(reader) => reader.finish().await,
                        None => Vec::new(),
                    }
                },
            )
        });
        Ok(ToolchainOutcome { status, stderr, stdout })
    })
    .await
}

fn toolchain_install_failure_message(binary: &str, exit_code: Option<i32>, tail: &str) -> String {
    let masked_tail = mask_known_secrets(tail);
    if masked_tail.is_empty() {
        return format!("{binary} 설치 명령이 실패했습니다 (종료 코드: {exit_code:?})");
    }
    format!("{binary} 설치 명령이 실패했습니다 (종료 코드: {exit_code:?}): {masked_tail}")
}

/// Runs a toolchain installer with supervised child reaping and owned stdout/stderr readers.
pub async fn run_toolchain_install(
    events: &dyn EventSink,
    spec: &LanguageServerSpec,
    lease: &LspInstallLease,
    tasks: &TaskSupervisor,
) -> AppResult<()> {
    lease.ensure_active()?;
    let toolchain = spec.install.toolchain.as_ref().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.lsp.toolchainInfoMissing",
            format!("{}: toolchain install info is not configured yet", spec.id),
        )
        .with_arg("serverId", &spec.id)
    })?;
    let binary = service::toolchain_binary(toolchain.tool);
    if service::find_in_path(binary).is_none() {
        return Err(AppError::localized(
            AppErrorKind::NotFound,
            "error.lsp.toolchainNotFound",
            format!("could not find the {binary} toolchain"),
        )
        .with_arg("binary", binary));
    }
    emit_install_progress(
        events,
        &spec.id,
        LspInstallPhase::Downloading,
        0,
        None,
        Some(format!("{binary} 로 설치 중")),
    );
    let mut command = Command::new(binary);
    command.args(&toolchain.install_args);
    configure_toolchain_command(&mut command);
    let outcome = match run_toolchain_process(command, binary.to_string(), lease, tasks).await {
        Ok(outcome) => outcome,
        Err(error) => {
            if matches!(&error, AppError::Localized(localized) if localized.key == "error.lsp.toolchainRunFailed") {
                return Err(error);
            }
            let mut message = error.to_string();
            if lease.ensure_active().is_err() {
                message = "설치가 취소되었습니다".to_string();
            }
            emit_install_progress(events, &spec.id, LspInstallPhase::Failed, 0, None, Some(message));
            return Err(error);
        }
    };
    if outcome.status.success() {
        if let Err(error) = lease.commit(|| Ok(())) {
            emit_install_progress(
                events,
                &spec.id,
                LspInstallPhase::Failed,
                0,
                None,
                Some("설치가 취소되었습니다".to_string()),
            );
            return Err(error);
        }
        emit_install_progress(events, &spec.id, LspInstallPhase::Done, 0, None, None);
        return Ok(());
    }
    let tail = outcome.stderr.into_iter().chain(outcome.stdout).collect::<Vec<_>>().join("\n");
    let message = toolchain_install_failure_message(binary, outcome.status.code(), &tail);
    emit_install_progress(events, &spec.id, LspInstallPhase::Failed, 0, None, Some(message.clone()));
    Err(AppError::Internal(message))
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use taide_lsp::install::LspInstallStore;
    use taide_model::lsp::LspServerId;

    use super::*;
    use crate::TaskSupervisor;

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const OUTPUT_FIXTURE_BUFFER_BYTES: usize = 1_024;
    #[cfg(unix)]
    const CHILD_FIXTURE_DURATION_SECONDS: u64 = 30;
    #[cfg(unix)]
    const FAILURE_FIXTURE_EXIT_CODE: i32 = 7;
    #[cfg(unix)]
    const PARENT_FIXTURE_POLL_SECONDS: &str = "0.01";

    #[cfg(unix)]
    struct ParentExitFixture {
        child: ChildFixture,
        release: std::path::PathBuf,
        anchor: Option<Child>,
        group_id: Option<u32>,
    }

    #[cfg(unix)]
    impl Drop for ParentExitFixture {
        fn drop(&mut self) {
            self.child.store.shutdown();
            self.child.request.abort();
            if let Some(anchor) = &mut self.anchor {
                if matches!(anchor.try_wait(), Ok(None)) {
                    if let Some(group_id) = self.group_id {
                        Command::new("kill").arg("-KILL").arg(format!("-{group_id}")).status().ok();
                    }
                    anchor.kill().ok();
                }
                anchor.wait().ok();
            }
            std::fs::remove_file(&self.release).ok();
        }
    }

    #[cfg(unix)]
    struct ChildFixture {
        marker: std::path::PathBuf,
        descendant_marker: Option<std::path::PathBuf>,
        store: LspInstallStore,
        request: JoinHandle<AppResult<ToolchainOutcome>>,
    }

    #[cfg(unix)]
    impl Drop for ChildFixture {
        fn drop(&mut self) {
            self.store.shutdown();
            self.request.abort();
            std::fs::remove_file(&self.marker).ok();
            if let Some(marker) = &self.descendant_marker {
                std::fs::remove_file(marker).ok();
            }
        }
    }

    #[cfg(unix)]
    async fn verify_child_cancellation(should_drop_request: bool, should_shutdown: bool) {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-cancel-child");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let worker_tasks = tasks.clone();
        let marker = std::env::temp_dir().join(format!("taide-toolchain-child-{}", uuid::Uuid::new_v4()));
        let mut command = Command::new("sh");
        command.args(["-c", "printf '%s' \"$$\" > \"$1\"; exec sleep \"$2\"", "fixture"]);
        command.arg(&marker).arg(CHILD_FIXTURE_DURATION_SECONDS.to_string());
        configure_toolchain_command(&mut command);
        let request =
            tokio::spawn(async move { run_toolchain_process(command, "synthetic-sh".to_string(), &guard.lease(), &worker_tasks).await });
        let mut fixture = ChildFixture {
            marker,
            descendant_marker: None,
            store: store.clone(),
            request,
        };
        let pid = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            loop {
                if let Ok(text) = std::fs::read_to_string(&fixture.marker) {
                    if let Ok(pid) = text.parse::<u32>() {
                        return pid;
                    }
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        assert!(store.begin(&server_id).is_none());
        if should_drop_request {
            fixture.request.abort();
            assert!(matches!(
                tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), &mut fixture.request).await.unwrap(),
                Err(error) if error.is_cancelled()
            ));
        } else {
            if should_shutdown {
                store.shutdown();
                tasks.shutdown().await;
                store.wait_for_idle().await;
            } else {
                store.cancel(&server_id);
            }
            assert!(
                tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), &mut fixture.request)
                    .await
                    .unwrap()
                    .unwrap()
                    .is_err()
            );
        }
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            while tasks.tracked_count() > 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(!Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        if should_shutdown {
            assert!(store.is_stopped());
            assert!(store.begin(&server_id).is_none());
        } else {
            assert!(store.begin(&server_id).is_some());
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 요청_drop은_자기_child를_kill_reap하고_reader를_회수한다() {
        verify_child_cancellation(true, false).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 명시_취소는_자기_child를_kill_reap하고_reader를_회수한다() {
        verify_child_cancellation(false, false).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 저장소_shutdown은_자기_child를_kill_reap하고_새_설치를_거절한다() {
        verify_child_cancellation(false, true).await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 종료_드레인은_term을_무시하는_자기_그룹의_자손도_종료한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-descendant");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let worker_tasks = tasks.clone();
        let marker = std::env::temp_dir().join(format!("taide-toolchain-group-{}", uuid::Uuid::new_v4()));
        let descendant_marker = marker.with_extension("descendant");
        let mut command = Command::new("sh");
        command.args(["-c", "sh -c 'trap \"\" TERM; printf \"%s\" \"$$\" > \"$1\"; exec sleep \"$2\"' fixture \"$1\" \"$2\" & printf '%s' \"$$\" > \"$3\"; wait", "fixture"]);
        command
            .arg(&descendant_marker)
            .arg(CHILD_FIXTURE_DURATION_SECONDS.to_string())
            .arg(&marker);
        configure_toolchain_command(&mut command);
        let request =
            tokio::spawn(
                async move { run_toolchain_process(command, "synthetic-group-sh".to_string(), &guard.lease(), &worker_tasks).await },
            );
        let mut fixture = ChildFixture {
            marker,
            descendant_marker: Some(descendant_marker.clone()),
            store: store.clone(),
            request,
        };
        let descendant = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            loop {
                if let Ok(text) = std::fs::read_to_string(&descendant_marker) {
                    if let Ok(pid) = text.parse::<u32>() {
                        return pid;
                    }
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(Command::new("kill")
            .args(["-TERM", &descendant.to_string()])
            .status()
            .unwrap()
            .success());
        assert!(Command::new("kill")
            .args(["-0", &descendant.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success());
        store.shutdown();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
            .await
            .unwrap();
        store.wait_for_idle().await;
        assert!((&mut fixture.request).await.unwrap().is_err());
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            loop {
                let is_alive = Command::new("kill")
                    .args(["-0", &descendant.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success();
                if !is_alive {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 부모_선종료도_자기_그룹을_정리하고_원래_종료_코드를_보존한다() {
        use std::os::unix::process::CommandExt;

        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-parent-exit");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let worker_tasks = tasks.clone();
        let marker = std::env::temp_dir().join(format!("taide-toolchain-parent-{}", uuid::Uuid::new_v4()));
        let descendant_marker = marker.with_extension("descendant");
        let release = marker.with_extension("release");
        let mut command = Command::new("sh");
        command.args(["-c", "sh -c 'trap \"\" TERM; printf \"%s\" \"$$\" > \"$1\"; exec sleep \"$2\"' fixture \"$1\" \"$2\" & printf '%s' \"$$\" > \"$3\"; while [ ! -f \"$4\" ]; do sleep \"$5\"; done; exit 0", "fixture"]);
        command
            .arg(&descendant_marker)
            .arg(CHILD_FIXTURE_DURATION_SECONDS.to_string())
            .arg(&marker)
            .arg(&release)
            .arg(PARENT_FIXTURE_POLL_SECONDS);
        configure_toolchain_command(&mut command);
        let request =
            tokio::spawn(
                async move { run_toolchain_process(command, "synthetic-parent-sh".to_string(), &guard.lease(), &worker_tasks).await },
            );
        let mut fixture = ParentExitFixture {
            child: ChildFixture {
                marker,
                descendant_marker: Some(descendant_marker.clone()),
                store: store.clone(),
                request,
            },
            release,
            anchor: None,
            group_id: None,
        };
        let (parent, descendant) = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            loop {
                let parent = std::fs::read_to_string(&fixture.child.marker)
                    .ok()
                    .and_then(|text| text.parse::<u32>().ok());
                let descendant = std::fs::read_to_string(&descendant_marker)
                    .ok()
                    .and_then(|text| text.parse::<u32>().ok());
                if let (Some(parent), Some(descendant)) = (parent, descendant) {
                    return (parent, descendant);
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(should_signal_process_group(parent));
        let mut anchor = Command::new("sleep");
        anchor
            .arg(CHILD_FIXTURE_DURATION_SECONDS.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(i32::try_from(parent).unwrap());
        fixture.anchor = Some(anchor.spawn().unwrap());
        fixture.group_id = Some(parent);
        std::fs::write(&fixture.release, b"release").unwrap();
        let outcome = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), &mut fixture.child.request)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(outcome.status.success());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(store.begin(&server_id).is_some());
        let disappeared = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            loop {
                let is_alive = Command::new("kill")
                    .args(["-0", &descendant.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .unwrap()
                    .success();
                if !is_alive {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(
            disappeared.is_ok(),
            "parent exit left its descendant alive after releasing the install slot"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 정상_child는_출력을_드레인하고_실제_reap_뒤_결과를_반환한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-child");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'stdout-line\\n'; printf 'stderr-line\\n' >&2; exit 0"]);
        configure_toolchain_command(&mut command);
        let result = run_toolchain_process(command, "sh".to_string(), &guard.lease(), &tasks)
            .await
            .unwrap();
        assert!(result.status.success());
        assert_eq!(result.stdout, ["stdout-line"]);
        assert_eq!(result.stderr, ["stderr-line"]);
        assert_eq!(tasks.tracked_count(), 0);
        drop(guard);
        assert!(store.begin(&server_id).is_some());
    }

    #[cfg(unix)]
    #[test]
    fn 회수한_child의_늦은_취소는_그룹_소유권을_재사용하지_않는다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-reaped-child");
        let guard = store.begin(&server_id).unwrap();
        let mut command = Command::new("sh");
        command.args(["-c", "exit 0"]);
        configure_toolchain_command(&mut command);
        let child = InstallChild {
            child: Mutex::new(command.spawn().unwrap()),
            is_reaped: std::sync::atomic::AtomicBool::new(false),
            changed: Condvar::new(),
            _lease: guard.lease(),
        };
        assert!(child.wait_for_exit().unwrap().success());
        assert!(child.is_reaped.load(std::sync::atomic::Ordering::Acquire));
        child.cancel();
        assert!(child.child.lock().wait().unwrap().success());
        drop(child);
        drop(guard);
        assert!(store.begin(&server_id).is_some());
    }

    #[tokio::test]
    async fn eof가_지연돼도_reader를_abort_후_join하고_마지막_tail을_보존한다() {
        use tokio::io::AsyncWriteExt;

        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-reader");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let (reader, mut writer) = tokio::io::duplex(OUTPUT_FIXTURE_BUFFER_BYTES);
        let output = InstallOutputReader::spawn(reader, guard.lease(), &tasks).unwrap();
        writer.write_all(b"last-line\n").await.unwrap();
        let tail = output.tail.clone();
        tokio::time::timeout(std::time::Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            while tail.lock().is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        drop(guard);
        assert!(store.begin(&server_id).is_none());
        let lines = output.finish().await;
        assert_eq!(lines, ["last-line"]);
        assert_eq!(tasks.tracked_count(), 0);
        assert!(store.begin(&server_id).is_some());
        drop(writer);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 실패_child는_실제_reap과_출력_드레인_뒤_마지막_20줄을_보존한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-failed-child");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let mut command = Command::new("sh");
        command.args([
            "-c",
            "i=0; while [ \"$i\" -lt \"$1\" ]; do printf 'out-%s\\n' \"$i\"; i=$((i + 1)); done; printf 'failure-line\\n' >&2; exit \"$2\"",
            "fixture",
        ]);
        command
            .arg((TOOLCHAIN_OUTPUT_TAIL_LINES + 1).to_string())
            .arg(FAILURE_FIXTURE_EXIT_CODE.to_string());
        configure_toolchain_command(&mut command);
        let result = run_toolchain_process(command, "sh".to_string(), &guard.lease(), &tasks)
            .await
            .unwrap();
        assert_eq!(result.status.code(), Some(FAILURE_FIXTURE_EXIT_CODE));
        assert_eq!(result.stderr, ["failure-line"]);
        assert_eq!(result.stdout.len(), TOOLCHAIN_OUTPUT_TAIL_LINES);
        assert_eq!(result.stdout.first().unwrap(), "out-1");
        assert_eq!(result.stdout.last().unwrap(), &format!("out-{TOOLCHAIN_OUTPUT_TAIL_LINES}"));
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn reader_owner_drop은_abort_요청과_실제_task_drop을_구분한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-dropped-reader");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let (reader, _writer) = tokio::io::duplex(OUTPUT_FIXTURE_BUFFER_BYTES);
        let output = InstallOutputReader::spawn(reader, guard.lease(), &tasks).unwrap();
        drop(guard);
        drop(output);
        assert!(store.begin(&server_id).is_none());
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            while tasks.tracked_count() > 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(store.begin(&server_id).is_some());
    }

    #[tokio::test]
    async fn 멈춘_감독자는_새_reader를_등록하지_않고_lease를_반납한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("synthetic-reader");
        let guard = store.begin(&server_id).unwrap();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        tasks.stop_all();
        let (reader, _writer) = tokio::io::duplex(OUTPUT_FIXTURE_BUFFER_BYTES);
        assert!(InstallOutputReader::spawn(reader, guard.lease(), &tasks).is_err());
        drop(guard);
        assert!(store.begin(&server_id).is_some());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 취소_또는_종료된_슬롯은_child를_시작하지_않는다() {
        for should_stop_tasks in [false, true] {
            let store = LspInstallStore::new();
            let server_id = LspServerId::from("synthetic-child");
            let guard = store.begin(&server_id).unwrap();
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            if should_stop_tasks {
                tasks.stop_all();
            } else {
                store.shutdown();
            }
            let mut command = Command::new("does-not-exist-synthetic-toolchain");
            configure_toolchain_command(&mut command);
            assert!(run_toolchain_process(command, "synthetic".to_string(), &guard.lease(), &tasks)
                .await
                .is_err());
            assert_eq!(tasks.tracked_count(), 0);
        }
    }

    #[test]
    fn 툴체인_설치_실패_메시지의_레지스트리_자격증명은_마스킹된다() {
        let tail = "npm ERR! code E401\nnpm ERR! //registry.npmjs.org/:_authToken=abcd-1234-efgh-5678\nnpm ERR! 401 Unauthorized";
        let message = toolchain_install_failure_message("npm", Some(1), tail);
        assert!(!message.contains("abcd-1234-efgh-5678"));
        assert!(message.contains("_authToken=[redacted:key_value]"));
        assert!(message.contains("npm ERR! 401 Unauthorized"));
        assert!(message.starts_with("npm 설치 명령이 실패했습니다 (종료 코드: Some(1))"));
    }

    #[test]
    fn 툴체인_설치_실패_메시지는_출력이_없으면_종료_코드만_남긴다() {
        assert_eq!(
            toolchain_install_failure_message("go", None, ""),
            "go 설치 명령이 실패했습니다 (종료 코드: None)"
        );
    }

    #[cfg(unix)]
    #[test]
    fn 프로세스_그룹_시그널은_자기_그룹과_전체_시그널_pid를_거부한다() {
        assert!(!should_signal_process_group(0));
        assert!(!should_signal_process_group(1));
        assert!(should_signal_process_group(MIN_SIGNALABLE_PGID));
        assert!(should_signal_process_group(48_231));
    }
}
