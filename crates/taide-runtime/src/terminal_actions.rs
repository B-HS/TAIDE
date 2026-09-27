use std::future::Future;
use std::io::Write;
use std::sync::Arc;

use parking_lot::Mutex;
use taide_infra::pty::{PtySession, PtySpawnConfig};
use taide_infra::root_guard::ensure_within_root;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::terminal::{resolve_scrollback_bytes, PtyAttachResult, PtySpawnOptions, ShellProfile, TerminalSession};
use taide_terminal::metadata::TerminalSessionMetadata;
use taide_terminal::service;
use taide_terminal::session::TerminalSessionOutput;
use taide_terminal::store::{TerminalSessionEntry, TerminalSpawnLease, TerminalStore};
use tokio::sync::OwnedMutexGuard;

use crate::{AppState, EventSink, TaskSupervisor};

const DEFAULT_TERMINAL_COLS: u16 = 80;
const DEFAULT_TERMINAL_ROWS: u16 = 24;

/// Supplies toolkit-owned environment, channel, identity, and native session factories.
pub struct TerminalSpawnPorts<E, D, I, F> {
    pub extra_env: E,
    pub discard_initial_sink: D,
    pub create_session_id: I,
    pub create_session: F,
}

/// Orders spawn preparation, admission, metadata registration, and the spawned event.
pub async fn pty_spawn<E, D, I, F>(
    events: &dyn EventSink,
    state: &AppState,
    store: &TerminalStore,
    tasks: &TaskSupervisor,
    opts: PtySpawnOptions,
    ports: TerminalSpawnPorts<E, D, I, F>,
) -> AppResult<String>
where
    E: Future<Output = Vec<(String, String)>>,
    D: FnOnce(),
    I: FnOnce() -> String,
    F: FnOnce(PtySpawnConfig, String, Arc<TerminalSessionMetadata>, Arc<TerminalSessionOutput>) -> AppResult<PtySession> + Send + 'static,
{
    let extra_env = ports.extra_env.await;
    let guard = state.begin_owned_mutation().await;
    if state.is_shutting_down() {
        return Err(terminal_shutdown_error());
    }
    if !state.projects.read().contains_key(&opts.project_id) {
        return Err(AppError::NotFound(format!("project not open: {}", opts.project_id)));
    }
    (ports.discard_initial_sink)();
    let session_id = (ports.create_session_id)();
    let output = Arc::new(TerminalSessionOutput::new(resolve_scrollback_bytes(opts.scrollback_bytes)));
    let metadata = Arc::new(TerminalSessionMetadata::new(
        opts.project_id.clone(),
        opts.cwd.clone(),
        opts.shell.clone().unwrap_or_else(|| "default".to_string()),
    ));
    let config = PtySpawnConfig {
        shell: opts.shell,
        cwd: opts.cwd,
        cols: opts.cols,
        rows: opts.rows,
        extra_env,
    };
    let _operation = tasks.begin_operation("terminal-spawn-action").ok_or_else(terminal_shutdown_error)?;
    let worker_id = session_id.clone();
    let worker_output = output.clone();
    let worker_metadata = metadata.clone();
    let (handle, _guard) = run_terminal_spawn(tasks, store, guard, move || {
        (ports.create_session)(config, worker_id, worker_metadata, worker_output)
    })
    .await?;
    let spawned = AppEvent::TerminalSpawned {
        session_id: session_id.clone(),
        project_id: metadata.project_id().clone(),
        cwd: metadata.cwd(),
        shell: metadata.shell().to_string(),
    };
    store.insert(session_id.clone(), TerminalSessionEntry::new(handle, metadata, output))?;
    events.publish(spawned);
    Ok(session_id)
}

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

async fn run_terminal_write(tasks: &TaskSupervisor, writer: Arc<Mutex<Box<dyn Write + Send>>>, data: String) -> AppResult<()> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let worker = tasks
        .spawn_blocking_transient_handle("terminal-write", move || {
            let result = (|| {
                let mut writer = writer.lock();
                writer.write_all(data.as_bytes())?;
                writer.flush()?;
                Ok(())
            })();
            drop(sender.send(result));
        })
        .ok_or_else(terminal_shutdown_error)?;
    worker.await.map_err(|error| {
        if error.is_cancelled() {
            return terminal_shutdown_error();
        }
        AppError::Internal(error.to_string())
    })?;
    receiver.await.map_err(|_| terminal_shutdown_error())?
}

/// Notifies input observers before resolving the shared writer and supervising its write and flush.
/// Request cancellation detaches the waiter but leaves already-started work tracked for normal exit drainage.
pub async fn pty_write(
    tasks: &TaskSupervisor,
    store: &TerminalStore,
    session_id: String,
    data: String,
    notify_input: impl FnOnce(&str),
) -> AppResult<()> {
    notify_input(&session_id);
    let writer = store.writer_handle(&session_id)?;
    run_terminal_write(tasks, writer, data).await
}

/// Applies the shared terminal pty resize policy.
pub async fn pty_resize(store: &TerminalStore, session_id: String, cols: u16, rows: u16) -> AppResult<()> {
    store.resize(&session_id, cols, rows)
}

/// Applies the shared terminal pty kill policy.
pub async fn pty_kill(state: &AppState, store: &TerminalStore, session_id: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    store.kill(&session_id)
}

/// Applies the shared terminal pty set paused policy.
pub async fn pty_set_paused(store: &TerminalStore, session_id: String, paused: bool) -> AppResult<()> {
    store.set_paused(&session_id, paused)
}

/// Applies the shared terminal pty attach policy.
pub async fn pty_attach<F, S>(state: &AppState, store: &TerminalStore, session_id: String, create_sink: F) -> AppResult<PtyAttachResult>
where
    F: FnOnce() -> S,
    S: Fn(&[u8]) -> bool + Send + Sync + 'static,
{
    let _guard = state.begin_mutation().await;
    store.attach(&session_id, create_sink())
}

/// Applies the shared terminal pty detach policy.
pub async fn pty_detach(state: &AppState, store: &TerminalStore, session_id: String, subscription_id: u32) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    store.detach(&session_id, subscription_id)
}

/// Applies the shared terminal terminal sessions policy.
pub async fn terminal_sessions(store: &TerminalStore, project_id: ProjectId) -> AppResult<Vec<TerminalSession>> {
    Ok(store.sessions_for_project(&project_id))
}

/// Applies the shared terminal shell profiles policy.
pub async fn shell_profiles() -> AppResult<Vec<ShellProfile>> {
    Ok(service::list_shell_profiles())
}

/// Applies the shared terminal resolve terminal path policy.
pub async fn resolve_terminal_path(state: &AppState, path: String, cwd: String) -> AppResult<String> {
    let projects = state.projects.read().clone();
    service::guard_terminal_path(&projects, &path, &cwd)
}

/// Applies the shared terminal terminal resolve link candidates policy.
pub async fn terminal_resolve_link_candidates(state: &AppState, cwd: String, candidates: Vec<String>) -> AppResult<Vec<Option<String>>> {
    let projects = state.projects.read().clone();
    Ok(service::resolve_link_candidates(&projects, &cwd, &candidates))
}

/// Applies the shared terminal pty default options policy.
pub async fn pty_default_options(state: &AppState, project_id: ProjectId, cwd: Option<String>) -> AppResult<PtySpawnOptions> {
    let root = state
        .projects
        .read()
        .get(&project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let resolved_cwd = match cwd {
        Some(requested) => ensure_within_root(std::path::Path::new(&root), std::path::Path::new(&requested))?
            .to_string_lossy()
            .to_string(),
        None => root,
    };

    Ok(PtySpawnOptions {
        project_id,
        cwd: resolved_cwd,
        shell: state.settings.read().shell_override.clone(),
        cols: DEFAULT_TERMINAL_COLS,
        rows: DEFAULT_TERMINAL_ROWS,
        scrollback_bytes: None,
    })
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

#[cfg(test)]
mod write_tests {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use parking_lot::Mutex as WriterMutex;
    use taide_lsp::install::LspInstallStore;
    use taide_lsp::store::LspStore;
    use taide_model::error::AppErrorKind;
    use taide_terminal::store::TerminalStore;
    use tokio::sync::oneshot;

    use super::run_terminal_write;
    use crate::{ExitDrain, TaskSupervisor};

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const PENDING_PROBE_MS: u64 = 20;

    struct RecordingWriter {
        calls: Arc<Mutex<Vec<String>>>,
        should_fail_write: bool,
        should_fail_flush: bool,
        should_panic: bool,
        started: Option<oneshot::Sender<()>>,
        release: Option<std::sync::mpsc::Receiver<()>>,
    }

    impl Write for RecordingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if let Some(started) = self.started.take() {
                started.send(()).ok();
            }
            if let Some(release) = self.release.take() {
                release.recv().unwrap();
            }
            assert!(!self.should_panic, "synthetic writer panic");
            self.calls.lock().unwrap().push(String::from_utf8(bytes.to_vec()).unwrap());
            if self.should_fail_write {
                return Err(io::Error::other("synthetic write error"));
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.calls.lock().unwrap().push("flush".to_string());
            if self.should_fail_flush {
                return Err(io::Error::other("synthetic flush error"));
            }
            Ok(())
        }
    }

    struct Release(Option<std::sync::mpsc::Sender<()>>);

    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(release) = self.0.take() {
                release.send(()).ok();
            }
        }
    }

    #[tokio::test]
    async fn write와_flush_순서_오류와_panic을_유지하고_감독에서_회수한다() {
        for (fail_write, fail_flush, panic) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let calls = Arc::new(Mutex::new(Vec::new()));
            let writer: Arc<WriterMutex<Box<dyn Write + Send>>> = Arc::new(WriterMutex::new(Box::new(RecordingWriter {
                calls: calls.clone(),
                should_fail_write: fail_write,
                should_fail_flush: fail_flush,
                should_panic: panic,
                started: None,
                release: None,
            })));
            let result = run_terminal_write(&tasks, writer, "fixture".to_string()).await;
            if panic {
                assert_eq!(result.unwrap_err().kind(), AppErrorKind::Internal);
            } else if fail_write || fail_flush {
                assert_eq!(result.unwrap_err().kind(), AppErrorKind::Io);
            } else {
                result.unwrap();
            }
            let recorded = calls.lock().unwrap().clone();
            if panic {
                assert!(recorded.is_empty());
            } else if fail_write {
                assert_eq!(recorded, ["fixture"]);
            } else {
                assert_eq!(recorded, ["fixture", "flush"]);
            }
            assert_eq!(tasks.tracked_count(), 0);
        }
    }

    #[tokio::test]
    async fn 요청_abort_뒤_정상_root는_시작한_writer의_실제_완료까지_기다린다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (started, started_rx) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let release = Release(Some(release));
        let writer: Arc<WriterMutex<Box<dyn Write + Send>>> = Arc::new(WriterMutex::new(Box::new(RecordingWriter {
            calls: calls.clone(),
            should_fail_write: false,
            should_fail_flush: false,
            should_panic: false,
            started: Some(started),
            release: Some(release_rx),
        })));
        let worker_tasks = tasks.clone();
        let request = tokio::spawn(async move { run_terminal_write(&worker_tasks, writer, "fixture".to_string()).await });
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
            .await
            .unwrap()
            .unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert_eq!(tasks.tracked_count(), 1);
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
        assert_eq!(tasks.tracked_count(), 1);
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert_eq!(tasks.tracked_count(), 0);
        assert_eq!(*calls.lock().unwrap(), ["fixture", "flush"]);
    }

    #[tokio::test]
    async fn 종료한_감독자는_writer를_실행하지_않는다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        tasks.stop_all();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let writer: Arc<WriterMutex<Box<dyn Write + Send>>> = Arc::new(WriterMutex::new(Box::new(RecordingWriter {
            calls: calls.clone(),
            should_fail_write: false,
            should_fail_flush: false,
            should_panic: false,
            started: None,
            release: None,
        })));
        let error = run_terminal_write(&tasks, writer, "fixture".to_string()).await.unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
        assert!(calls.lock().unwrap().is_empty());
        assert_eq!(tasks.tracked_count(), 0);
    }
}
