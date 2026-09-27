use std::future::Future;
use std::path::PathBuf;

use taide_model::error::{AppError, AppResult};
use taide_model::plugin::LoadedPlugin;
use tokio::task::{AbortHandle, JoinError};

use crate::TaskSupervisor;

pub(crate) struct StagedPlugin {
    pub path: PathBuf,
    pub id: String,
}

impl Drop for StagedPlugin {
    fn drop(&mut self) {
        if self.path.exists() {
            std::fs::remove_dir_all(&self.path).ok();
        }
    }
}

struct CancelOnDrop(AbortHandle);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn shutdown_error() -> AppError {
    AppError::Forbidden("plugin runtime is shutting down".to_string())
}

fn join_error(error: JoinError) -> AppError {
    if error.is_cancelled() {
        return shutdown_error();
    }
    AppError::Internal(error.to_string())
}

pub(crate) async fn run_install(
    tasks: &TaskSupervisor,
    name: &'static str,
    operation: impl Future<Output = AppResult<LoadedPlugin>> + Send + 'static,
) -> AppResult<LoadedPlugin> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let operation = tasks
        .spawn_transient_handle(name, async move {
            drop(sender.send(operation.await));
        })
        .ok_or_else(shutdown_error)?;
    let _cancellation = CancelOnDrop(operation.abort_handle());
    operation.await.map_err(join_error)?;
    receiver.await.map_err(|_| shutdown_error())?
}

pub(crate) async fn stage_plugin(
    tasks: &TaskSupervisor,
    stage: impl FnOnce() -> AppResult<(PathBuf, String)> + Send + 'static,
) -> AppResult<StagedPlugin> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let worker = tasks
        .spawn_blocking_transient_handle("plugin-stage", move || {
            let result = stage().map(|(path, id)| StagedPlugin { path, id });
            drop(sender.send(result));
        })
        .ok_or_else(shutdown_error)?;
    worker.await.map_err(join_error)?;
    receiver.await.map_err(|_| shutdown_error())?
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_lsp::store::LspStore;
    use taide_model::error::AppErrorKind;
    use taide_terminal::store::TerminalStore;

    use super::{run_install, stage_plugin};
    use crate::{ExitDrain, TaskSupervisor};

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const PENDING_PROBE_MS: u64 = 20;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("taide-plugin-worker-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).expect("자기 UUID staging fixture만 정리");
        }
    }

    struct Release(Option<std::sync::mpsc::Sender<()>>);

    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                sender.send(()).ok();
            }
        }
    }

    #[tokio::test]
    async fn 종료_입장은_설치_future와_stage_factory를_실행하지_않는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        tasks.stop_all();
        let called = Arc::new(AtomicBool::new(false));
        let marker = called.clone();
        let error = run_install(&tasks, "fixture-install", async move {
            marker.store(true, Ordering::SeqCst);
            panic!("종료 뒤 설치 실행 금지")
        })
        .await
        .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        let error = stage_plugin(&tasks, || panic!("종료 뒤 stage 실행 금지")).await.err().unwrap();
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn stage_반환_소유자의_drop은_반환된_임시_경로만_정리한다() {
        let fixture = Fixture::new();
        let staged_path = fixture.0.join("staged");
        std::fs::create_dir_all(&staged_path).unwrap();
        let source = fixture.0.join("source");
        std::fs::write(&source, "fixture").unwrap();
        let worker_path = staged_path.clone();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let staged = stage_plugin(&tasks, move || Ok((worker_path, "fixture".to_string())))
            .await
            .unwrap();
        assert!(staged_path.exists());
        drop(staged);
        assert!(!staged_path.exists());
        assert!(source.exists());
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn stage_panic은_internal_오류로_반환하며_감독에서_회수된다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let error = stage_plugin(&tasks, || panic!("synthetic stage panic")).await.err().unwrap();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn 요청_abort_뒤_root는_시작한_stage의_완료와_임시_정리를_기다린다() {
        let fixture = Fixture::new();
        let staged_path = fixture.0.join("staged");
        let worker_path = staged_path.clone();
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let operation_tasks = tasks.clone();
        let worker_tasks = tasks.clone();
        let (started, started_rx) = tokio::sync::oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let release = Release(Some(release));
        let request = tokio::spawn(async move {
            run_install(&operation_tasks, "fixture-install", async move {
                let _staged = stage_plugin(&worker_tasks, move || {
                    std::fs::create_dir_all(&worker_path).unwrap();
                    started.send(()).ok();
                    release_rx.recv().unwrap();
                    Ok((worker_path, "fixture".to_string()))
                })
                .await?;
                panic!("취소 뒤 commit 진입 금지")
            })
            .await
        });
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
            .await
            .unwrap()
            .unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        let mut drain = ExitDrain::default();
        let (ready, mut ready_rx) = tokio::sync::oneshot::channel();
        assert!(drain.begin(
            &runtime,
            tasks.clone(),
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                ready.send(()).ok();
            }
        ));
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut ready_rx)
            .await
            .is_err());
        assert!(!drain.is_ready());
        assert!(staged_path.exists());
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert!(!staged_path.exists());
        assert_eq!(tasks.tracked_count(), 0);
    }
}
