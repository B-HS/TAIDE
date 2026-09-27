use taide_infra::pty::PtySession;
use taide_model::error::{AppError, AppResult};
use taide_terminal::store::{TerminalSpawnLease, TerminalStore};
use tokio::sync::OwnedMutexGuard;

use crate::TaskSupervisor;

struct TerminalSpawnWork<F> {
    lease: TerminalSpawnLease,
    guard: OwnedMutexGuard<()>,
    create: F,
}

struct TerminalSpawnResult {
    session: Option<PtySession>,
    guard: Option<OwnedMutexGuard<()>>,
    store: TerminalStore,
}

impl TerminalSpawnResult {
    fn into_parts(mut self) -> (PtySession, OwnedMutexGuard<()>) {
        (
            self.session.take().expect("terminal result owns its session"),
            self.guard.take().expect("terminal result owns its mutation guard"),
        )
    }
}

impl Drop for TerminalSpawnResult {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            self.store.retire_session(session);
        }
    }
}

/// Supervises actual blocking spawn work independently of its request future.
/// The worker owns admission and the mutation guard; successful delivery returns the guard for registration.
pub async fn run_terminal_spawn(
    tasks: &TaskSupervisor,
    store: &TerminalStore,
    guard: OwnedMutexGuard<()>,
    create: impl FnOnce() -> AppResult<PtySession> + Send + 'static,
) -> AppResult<(PtySession, OwnedMutexGuard<()>)> {
    let owner = TerminalSpawnWork {
        lease: store.begin_spawn()?,
        guard,
        create,
    };
    let worker_store = store.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let worker = tasks
        .spawn_blocking_transient_handle("terminal-spawn", move || {
            let owner = owner;
            let result = owner.lease.spawn(owner.create).map(|session| TerminalSpawnResult {
                session: Some(session),
                guard: Some(owner.guard),
                store: worker_store,
            });
            drop(sender.send(result));
        })
        .ok_or_else(terminal_shutdown_error)?;
    worker.await.map_err(|error| {
        if error.is_cancelled() {
            return terminal_shutdown_error();
        }
        AppError::Internal(format!("terminal spawn task failed: {error}"))
    })?;
    Ok(receiver.await.map_err(|_| terminal_shutdown_error())??.into_parts())
}

fn terminal_shutdown_error() -> AppError {
    AppError::Forbidden("terminal runtime is shutting down".to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use taide_model::paths::AppPaths;

    use super::*;
    use crate::AppState;

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const PENDING_PROBE_MS: u64 = 60;
    #[cfg(unix)]
    const TEST_COLS: u16 = 80;
    #[cfg(unix)]
    const TEST_ROWS: u16 = 24;

    struct Release(Option<std::sync::mpsc::Sender<()>>);

    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                sender.send(()).ok();
            }
        }
    }

    fn fixture_state() -> AppState {
        AppState::new(AppPaths::new(std::env::temp_dir()))
    }

    #[cfg(unix)]
    fn controlled_shell_config() -> taide_infra::pty::PtySpawnConfig {
        taide_infra::pty::PtySpawnConfig {
            shell: Some("/bin/sh".to_string()),
            cwd: std::env::temp_dir().to_string_lossy().to_string(),
            cols: TEST_COLS,
            rows: TEST_ROWS,
            extra_env: vec![("ENV".to_string(), String::new()), ("BASH_ENV".to_string(), String::new())],
        }
    }

    #[tokio::test]
    async fn 종료한_감독자는_factory를_실행하지_않고_lease와_guard를_반납한다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let store = TerminalStore::new();
        let state = fixture_state();
        let called = Arc::new(AtomicBool::new(false));
        let marker = called.clone();
        tasks.stop_all();
        let result = run_terminal_spawn(&tasks, &store, state.begin_owned_mutation().await, move || {
            marker.store(true, Ordering::SeqCst);
            Err(AppError::Internal("unreachable synthetic factory".to_string()))
        })
        .await;
        assert!(matches!(result, Err(AppError::Forbidden(_))));
        assert!(!called.load(Ordering::SeqCst));
        let guard = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), state.begin_mutation())
            .await
            .unwrap();
        drop(guard);
        store.shutdown();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), store.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn factory_panic도_감독_오류로_반환하고_guard와_lease를_반납한다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let store = TerminalStore::new();
        let state = fixture_state();
        let result = run_terminal_spawn(&tasks, &store, state.begin_owned_mutation().await, || {
            panic!("synthetic terminal factory panic")
        })
        .await;
        assert!(matches!(result, Err(AppError::Internal(_))));
        drop(state.begin_mutation().await);
        store.shutdown();
        store.wait_for_idle().await.unwrap();
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn 요청_drop과_shutdown은_시작한_factory의_guard와_lease를_먼저_회수하지_않는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let store = TerminalStore::new();
        let state = fixture_state();
        let guard = state.begin_owned_mutation().await;
        let worker_tasks = tasks.clone();
        let worker_store = store.clone();
        let (started, started_rx) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let release = Release(Some(release));
        let request = tokio::spawn(async move {
            run_terminal_spawn(&worker_tasks, &worker_store, guard, move || {
                started.send(()).ok();
                held.recv().ok();
                Err(AppError::Internal("synthetic spawn failure".to_string()))
            })
            .await
        });
        started_rx.await.unwrap();
        request.abort();
        assert!(request.await.is_err());
        tasks.stop_all();
        store.shutdown();
        assert_eq!(tasks.tracked_count(), 1);
        assert!(
            tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), state.begin_mutation())
                .await
                .is_err()
        );
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), store.wait_for_idle())
            .await
            .is_err());
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), tasks.shutdown())
            .await
            .is_err());
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), store.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
        drop(state.begin_mutation().await);
    }

    #[test]
    fn shutdown은_대기열_factory를_실행하지_않고_소유한_guard와_lease를_반납한다() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        runtime.block_on(async {
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let store = TerminalStore::new();
            let state = fixture_state();
            let (started, started_rx) = tokio::sync::oneshot::channel();
            let (release, held) = std::sync::mpsc::channel();
            let release = Release(Some(release));
            let blocker = tokio::task::spawn_blocking(move || {
                started.send(()).ok();
                held.recv().ok();
            });
            started_rx.await.unwrap();
            let called = Arc::new(AtomicBool::new(false));
            let marker = called.clone();
            let mut request = Box::pin(run_terminal_spawn(&tasks, &store, state.begin_owned_mutation().await, move || {
                marker.store(true, Ordering::SeqCst);
                Err(AppError::Internal("unreachable queued factory".to_string()))
            }));
            assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut request)
                .await
                .is_err());
            tasks.stop_all();
            store.shutdown();
            drop(release);
            blocker.await.unwrap();
            let result = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), request)
                .await
                .unwrap();
            assert!(matches!(result, Err(AppError::Forbidden(_))));
            assert!(!called.load(Ordering::SeqCst));
            drop(state.begin_mutation().await);
            store.wait_for_idle().await.unwrap();
            tasks.shutdown().await;
        });
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 정상_전달은_등록까지_동일한_mutation_guard를_보유한다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let store = TerminalStore::new();
        let state = fixture_state();
        let (session, guard) = run_terminal_spawn(&tasks, &store, state.begin_owned_mutation().await, || {
            taide_infra::pty::spawn(controlled_shell_config(), |_| {}, |_| {})
        })
        .await
        .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), state.begin_mutation())
                .await
                .is_err()
        );
        store.retire_session(session);
        drop(guard);
        drop(state.begin_mutation().await);
        store.shutdown();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), store.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
        tasks.shutdown().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 취소한_요청의_늦은_생성_결과도_child_callback_완료까지_root에_남는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let store = TerminalStore::new();
        let state = fixture_state();
        let guard = state.begin_owned_mutation().await;
        let worker_tasks = tasks.clone();
        let worker_store = store.clone();
        let (started, started_rx) = tokio::sync::oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let release = Release(Some(release));
        let (callback, callback_rx) = tokio::sync::oneshot::channel();
        let (finish, finish_rx) = std::sync::mpsc::channel();
        let finish = Release(Some(finish));
        let request = tokio::spawn(async move {
            run_terminal_spawn(&worker_tasks, &worker_store, guard, move || {
                started.send(()).ok();
                held.recv().ok();
                taide_infra::pty::spawn(
                    controlled_shell_config(),
                    |_| {},
                    move |_| {
                        callback.send(()).ok();
                        finish_rx.recv().ok();
                    },
                )
            })
            .await
        });
        started_rx.await.unwrap();
        request.abort();
        assert!(request.await.is_err());
        tasks.stop_all();
        store.shutdown();
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), callback_rx)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
            .await
            .unwrap();
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), store.wait_for_idle())
            .await
            .is_err());
        drop(finish);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), store.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
        drop(state.begin_mutation().await);
    }
}
