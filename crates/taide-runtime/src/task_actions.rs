use taide_infra::root_guard;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::task::Task;
use taide_task::service;

use crate::{AppState, TaskSupervisor};

fn task_shutdown_error() -> AppError {
    AppError::Forbidden("task runtime is shutting down".to_string())
}

async fn run_task_detection(tasks: &TaskSupervisor, detect: impl FnOnce() -> Vec<Task> + Send + 'static) -> AppResult<Vec<Task>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let worker = tasks
        .spawn_blocking_transient_handle("task-detect", move || {
            drop(sender.send(detect()));
        })
        .ok_or_else(task_shutdown_error)?;
    worker.await.map_err(|error| {
        if error.is_cancelled() {
            return task_shutdown_error();
        }
        AppError::Internal(error.to_string())
    })?;
    receiver.await.map_err(|_| task_shutdown_error())
}

/// Resolves an open project before supervising its existing task scan without executing detected commands.
/// Already-started scan work stays tracked when its request waiter is dropped.
pub async fn detect_tasks(state: &AppState, tasks: &TaskSupervisor, project_id: ProjectId) -> AppResult<Vec<Task>> {
    let root = root_guard::project_root(&state.projects.read(), &project_id)?;
    run_task_detection(tasks, move || service::detect_tasks(&root)).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_lsp::store::LspStore;
    use taide_model::error::AppErrorKind;
    use taide_terminal::store::TerminalStore;
    use tokio::sync::oneshot;

    use super::run_task_detection;
    use crate::{ExitDrain, TaskSupervisor};

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const PENDING_PROBE_MS: u64 = 20;

    struct Release(Option<std::sync::mpsc::Sender<()>>);

    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                sender.send(()).ok();
            }
        }
    }

    #[tokio::test]
    async fn 종료한_감독자는_scan_factory를_실행하지_않는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let called = Arc::new(AtomicBool::new(false));
        let marker = called.clone();
        tasks.stop_all();
        let error = run_task_detection(&tasks, move || {
            marker.store(true, Ordering::SeqCst);
            Vec::new()
        })
        .await
        .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn 정상_결과와_panic은_기존_join_오류로_반환하고_감독에서_회수된다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        assert!(run_task_detection(&tasks, Vec::new).await.unwrap().is_empty());
        let error = run_task_detection(&tasks, || panic!("synthetic task scan panic"))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn 요청_abort_뒤에도_정상_root는_시작한_scan의_실제_완료를_기다린다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let (started, started_rx) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let release = Release(Some(release));
        let worker_tasks = tasks.clone();
        let completed = Arc::new(AtomicBool::new(false));
        let marker = completed.clone();
        let request = tokio::spawn(async move {
            run_task_detection(&worker_tasks, move || {
                started.send(()).ok();
                release_rx.recv().unwrap();
                marker.store(true, Ordering::SeqCst);
                Vec::new()
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
        let (ready, mut ready_rx) = oneshot::channel();
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
        assert!(!completed.load(Ordering::SeqCst));
        assert_eq!(tasks.tracked_count(), 1);
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert!(completed.load(Ordering::SeqCst));
        assert_eq!(tasks.tracked_count(), 0);
    }
}
