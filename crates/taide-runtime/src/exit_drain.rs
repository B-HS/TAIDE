use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::error::AppResult;
use taide_terminal::store::TerminalStore;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

use crate::{AiRequestStore, AppState, TaskSupervisor};

/// Owns exit drainage outside the supervisor it stops, keeping the native event loop available.
#[derive(Default)]
pub struct ExitDrain {
    task: Option<JoinHandle<()>>,
    ready: Arc<AtomicBool>,
    ai_requests: AiRequestStore,
    state: Option<AppState>,
}

impl ExitDrain {
    /// Uses the same AI request store registered by the host's application services.
    pub fn new(ai_requests: AiRequestStore) -> Self {
        Self {
            task: None,
            ready: Arc::new(AtomicBool::new(false)),
            ai_requests,
            state: None,
        }
    }

    /// Includes the host application state and its watcher completion tracker in exit drainage.
    pub fn with_state(mut self, state: AppState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    async fn wait_for_owned_resources(
        tasks: TaskSupervisor,
        state: Option<AppState>,
        installs: LspInstallStore,
        processes: LspStore,
        terminals: TerminalStore,
        ai_requests: AiRequestStore,
    ) -> AppResult<()> {
        tasks.shutdown().await;
        if let Some(state) = state {
            state.stop_watchers();
            state.watcher_stops.wait_for_idle().await;
        }
        installs.wait_for_idle().await;
        processes.wait_for_idle().await;
        ai_requests.wait_for_idle().await;
        terminals.wait_for_idle().await
    }

    /// Closes admission and waits for all owned resources when the event loop cannot defer exit.
    pub async fn wait_for_direct_exit(
        &self,
        tasks: TaskSupervisor,
        installs: LspInstallStore,
        processes: LspStore,
        terminals: TerminalStore,
    ) -> AppResult<()> {
        if let Some(state) = &self.state {
            state.begin_shutdown();
        }
        installs.shutdown();
        processes.shutdown();
        terminals.shutdown();
        self.ai_requests.shutdown();
        tasks.stop_all();
        Self::wait_for_owned_resources(tasks, self.state.clone(), installs, processes, terminals, self.ai_requests.clone()).await
    }

    /// Calls on_ready after tracked resources finish; terminal join errors keep readiness closed.
    pub fn begin(
        &mut self,
        runtime: &Handle,
        tasks: TaskSupervisor,
        installs: LspInstallStore,
        processes: LspStore,
        terminals: TerminalStore,
        on_ready: impl FnOnce() + Send + 'static,
    ) -> bool {
        if self.is_ready() || self.task.as_ref().is_some_and(|task| !task.is_finished()) {
            return false;
        }
        if let Some(state) = &self.state {
            state.begin_shutdown();
        }
        installs.shutdown();
        processes.shutdown();
        terminals.shutdown();
        self.ai_requests.shutdown();
        tasks.stop_all();
        let ready = self.ready.clone();
        let ai_requests = self.ai_requests.clone();
        let state = self.state.clone();
        self.task = Some(runtime.spawn(async move {
            if let Err(error) = Self::wait_for_owned_resources(tasks, state, installs, processes, terminals, ai_requests).await {
                log::error!("terminal runtime drain failed: {error}");
                return;
            }
            ready.store(true, Ordering::Release);
            on_ready();
        }));
        true
    }
}

impl Drop for ExitDrain {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_lsp::store::LspStore;
    use taide_model::lsp::LspServerId;
    use taide_model::paths::AppPaths;
    use taide_terminal::store::TerminalStore;
    use tokio::sync::oneshot;

    use super::ExitDrain;
    use crate::{AiRequestStore, AppState, TaskSupervisor};

    const FIXTURE_TIMEOUT_MS: u64 = 2_000;
    const AI_PENDING_PROBE_MS: u64 = 20;

    #[tokio::test]
    async fn 정상_root_종료는_폐기된_watcher의_실제_중지_완료를_기다린다() {
        let state = AppState::new(AppPaths::new(std::env::temp_dir().join("taide-watcher-stop-synthetic")));
        let (started, started_rx) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        state.watcher_stops.schedule(Box::new(move || {
            started.send(()).ok();
            release_rx.recv().unwrap();
        }));
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
            .await
            .unwrap()
            .unwrap();

        let runtime = tokio::runtime::Handle::current();
        let mut drain = ExitDrain::new(AiRequestStore::new()).with_state(state);
        let (finished, mut finished_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            TaskSupervisor::new(runtime.clone()),
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                finished.send(()).ok();
            },
        ));
        assert!(tokio::time::timeout(Duration::from_millis(AI_PENDING_PROBE_MS), &mut finished_rx)
            .await
            .is_err());
        assert!(!drain.is_ready());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), finished_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
    }

    #[tokio::test]
    async fn 직접_종료는_폐기된_watcher의_실제_중지_완료를_기다린다() {
        let state = AppState::new(AppPaths::new(std::env::temp_dir().join("taide-watcher-stop-synthetic")));
        let (started, started_rx) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        state.watcher_stops.schedule(Box::new(move || {
            started.send(()).ok();
            release_rx.recv().unwrap();
        }));
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
            .await
            .unwrap()
            .unwrap();

        let drain = ExitDrain::new(AiRequestStore::new()).with_state(state);
        let direct_exit = drain.wait_for_direct_exit(
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
        );
        tokio::pin!(direct_exit);
        assert!(tokio::time::timeout(Duration::from_millis(AI_PENDING_PROBE_MS), &mut direct_exit)
            .await
            .is_err());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), direct_exit)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn 직접_종료는_감독_작업_설치_ai_owner의_완료를_모두_기다린다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime);
        let task_owner = tasks.begin_operation("synthetic-direct-exit").unwrap();
        let installs = LspInstallStore::new();
        let install_owner = installs.begin(&LspServerId::from("synthetic-direct-exit")).unwrap();
        let requests = AiRequestStore::new();
        let (request_owner, _) = requests.begin("main", "synthetic-direct-exit").unwrap();
        let drain = ExitDrain::new(requests.clone());
        let direct_exit = drain.wait_for_direct_exit(tasks.clone(), installs.clone(), LspStore::new(), TerminalStore::new());
        tokio::pin!(direct_exit);

        assert!(tokio::time::timeout(Duration::from_millis(AI_PENDING_PROBE_MS), &mut direct_exit)
            .await
            .is_err());
        assert!(tasks.begin_operation("late").is_none());
        assert!(installs.is_stopped());
        assert!(requests.begin("late", "late").is_none());

        drop(task_owner);
        assert!(tokio::time::timeout(Duration::from_millis(AI_PENDING_PROBE_MS), &mut direct_exit)
            .await
            .is_err());
        drop(install_owner);
        assert!(tokio::time::timeout(Duration::from_millis(AI_PENDING_PROBE_MS), &mut direct_exit)
            .await
            .is_err());
        drop(request_owner);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), direct_exit)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn 정상_root_종료는_취소된_ai_owner의_실제_drop까지_기다린다() {
        let requests = AiRequestStore::new();
        let (owner, cancelled) = requests.begin("main", "fixture").unwrap();
        let runtime = tokio::runtime::Handle::current();
        let mut drain = ExitDrain::new(requests.clone());
        let (finished, mut finished_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            TaskSupervisor::new(runtime.clone()),
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                finished.send(()).ok();
            },
        ));
        assert!(cancelled.await.is_ok());
        assert!(requests.begin("other", "new").is_none());
        assert!(tokio::time::timeout(Duration::from_millis(AI_PENDING_PROBE_MS), &mut finished_rx)
            .await
            .is_err());
        assert!(!drain.is_ready());
        drop(owner);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), finished_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
    }

    #[tokio::test]
    async fn root_drain을_drop해도_ai_입장이_닫히고_새_drain이_같은_owner를_기다린다() {
        let requests = AiRequestStore::new();
        let (owner, _) = requests.begin("main", "fixture").unwrap();
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let mut drain = ExitDrain::new(requests.clone());
        assert!(drain.begin(
            &runtime,
            tasks.clone(),
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
            || panic!("AI owner가 살아있다")
        ));
        drop(drain);
        assert!(requests.begin("other", "new").is_none());
        let mut restarted = ExitDrain::new(requests.clone());
        let (finished, finished_rx) = oneshot::channel();
        assert!(restarted.begin(
            &runtime,
            tasks,
            LspInstallStore::new(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                finished.send(()).ok();
            }
        ));
        drop(owner);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), finished_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(restarted.is_ready());
    }
    #[cfg(unix)]
    const PENDING_PROBE_MS: u64 = 60;
    #[cfg(unix)]
    const TEST_COLS: u16 = 80;
    #[cfg(unix)]
    const TEST_ROWS: u16 = 24;

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

    #[cfg(unix)]
    struct ExitRelease(Option<std::sync::mpsc::Sender<()>>);

    #[cfg(unix)]
    impl Drop for ExitRelease {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                sender.send(()).ok();
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn 직접_종료는_lsp와_pty_callback_반환까지_기다린다() {
        let processes = LspStore::new();
        let (lsp_started, lsp_started_rx) = oneshot::channel();
        let (release_lsp, held_lsp) = std::sync::mpsc::channel();
        let release_lsp = ExitRelease(Some(release_lsp));
        let process = processes
            .spawn_process(|| {
                Ok(Arc::new(taide_infra::lsp_proc::spawn(
                    taide_infra::lsp_proc::LspProcConfig {
                        command: "/bin/sh".to_string(),
                        args: vec!["-c".to_string(), "exit 0".to_string()],
                        cwd: std::env::temp_dir(),
                    },
                    |_| {},
                    move |_, _| {
                        lsp_started.send(()).ok();
                        held_lsp.recv().ok();
                    },
                )?))
            })
            .unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), lsp_started_rx)
            .await
            .unwrap()
            .unwrap();

        let terminals = TerminalStore::new();
        let (pty_started, pty_started_rx) = oneshot::channel();
        let (release_pty, held_pty) = std::sync::mpsc::channel();
        let release_pty = ExitRelease(Some(release_pty));
        let session = terminals
            .begin_spawn()
            .unwrap()
            .spawn(|| {
                taide_infra::pty::spawn(
                    controlled_shell_config(),
                    |_| {},
                    move |_| {
                        pty_started.send(()).ok();
                        held_pty.recv().ok();
                    },
                )
            })
            .unwrap();
        let completion = session.completion_handle();
        session.write(b"exit 0\n").unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), pty_started_rx)
            .await
            .unwrap()
            .unwrap();
        terminals.retire_session(session);

        let runtime = tokio::runtime::Handle::current();
        let drain = ExitDrain::default();
        let direct_exit = drain.wait_for_direct_exit(TaskSupervisor::new(runtime), LspInstallStore::new(), processes, terminals);
        tokio::pin!(direct_exit);
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut direct_exit)
            .await
            .is_err());
        drop(release_lsp);
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut direct_exit)
            .await
            .is_err());
        drop(release_pty);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), direct_exit)
            .await
            .unwrap()
            .unwrap();
        assert!(process.is_finished() && completion.is_finished());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn 정상_root_종료는_제거한_pty의_callback_반환까지_기다린다() {
        const EXIT_CODE: i32 = 7;
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let terminals = TerminalStore::new();
        let (started, started_rx) = oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let release = ExitRelease(Some(release));
        let session = terminals
            .begin_spawn()
            .unwrap()
            .spawn(|| {
                taide_infra::pty::spawn(
                    controlled_shell_config(),
                    |_| {},
                    move |code| {
                        started.send(code).ok();
                        held.recv().ok();
                    },
                )
            })
            .unwrap();
        let completion = session.completion_handle();
        session.write(format!("exit {EXIT_CODE}\n").as_bytes()).unwrap();
        let code = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(code, Some(EXIT_CODE));
        terminals.retire_session(session);
        let mut drain = ExitDrain::default();
        let (exit, mut exit_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            tasks,
            LspInstallStore::new(),
            LspStore::new(),
            terminals.clone(),
            move || {
                exit.send(()).ok();
            }
        ));
        assert!(terminals.begin_spawn().is_err());
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut exit_rx)
            .await
            .is_err());
        assert!(!drain.is_ready() && !completion.is_finished());
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), exit_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready() && completion.is_finished());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pty_callback_panic은_정상_root_종료_준비로_오인하지_않는다() {
        let runtime = tokio::runtime::Handle::current();
        let terminals = TerminalStore::new();
        let session = terminals
            .begin_spawn()
            .unwrap()
            .spawn(|| taide_infra::pty::spawn(controlled_shell_config(), |_| {}, |_| panic!("synthetic PTY callback panic")))
            .unwrap();
        let completion = session.completion_handle();
        terminals.retire_session(session);
        let mut drain = ExitDrain::default();
        let (exit, exit_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            TaskSupervisor::new(runtime.clone()),
            LspInstallStore::new(),
            LspStore::new(),
            terminals.clone(),
            move || {
                exit.send(()).ok();
            }
        ));
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), drain.task.as_mut().unwrap())
            .await
            .unwrap()
            .unwrap();
        assert!(!drain.is_ready() && !completion.is_finished());
        assert!(exit_rx.await.is_err());
        assert!(terminals.wait_for_idle().await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn 종료_드레인은_프로세스_종료와_reader_callback_완료를_구분해_기다린다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let processes = LspStore::new();
        let (started, ready) = oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let release = ExitRelease(Some(release));
        let process = processes
            .spawn_process(|| {
                Ok(Arc::new(taide_infra::lsp_proc::spawn(
                    taide_infra::lsp_proc::LspProcConfig {
                        command: "sh".to_string(),
                        args: vec!["-c".to_string(), "exit 0".to_string()],
                        cwd: std::env::temp_dir(),
                    },
                    |_| {},
                    move |_, _| {
                        started.send(()).ok();
                        held.recv().ok();
                    },
                )?))
            })
            .unwrap();
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready)
            .await
            .unwrap()
            .unwrap();
        assert!(process.is_exited());
        assert!(!process.is_finished());
        let mut drain = ExitDrain::default();
        let (exit, mut exit_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            tasks,
            LspInstallStore::new(),
            processes,
            TerminalStore::new(),
            move || {
                exit.send(()).ok();
            }
        ));
        tokio::task::yield_now().await;
        assert!(!drain.is_ready());
        assert!(matches!(exit_rx.try_recv(), Err(tokio::sync::oneshot::error::TryRecvError::Empty)));
        drop(release);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), exit_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert!(process.is_finished());
    }

    #[tokio::test]
    async fn 종료_드레인은_메인_응답을_막지_않고_worker와_lease_종료_뒤_한번만_종료를_요청한다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let installs = LspInstallStore::new();
        let guard = installs.begin(&LspServerId::from("synthetic-exit")).unwrap();
        let (started, started_rx) = oneshot::channel();
        let (reply, reply_rx) = std::sync::mpsc::channel();
        let worker = tasks
            .spawn_blocking_transient_handle("synthetic-main-reply", move || {
                started.send(()).unwrap();
                reply_rx.recv().unwrap();
            })
            .unwrap();
        started_rx.await.unwrap();
        let mut drain = ExitDrain::default();
        let (exit, exit_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            tasks.clone(),
            installs.clone(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                exit.send(()).unwrap();
            }
        ));
        let duplicate_exit = Arc::new(AtomicBool::new(false));
        let duplicate_signal = duplicate_exit.clone();
        assert!(!drain.begin(
            &runtime,
            tasks.clone(),
            installs.clone(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                duplicate_signal.store(true, Ordering::SeqCst);
            }
        ));
        assert!(!drain.is_ready());
        assert!(!worker.is_finished());
        reply.send(()).unwrap();
        worker.await.unwrap();
        tokio::task::yield_now().await;
        assert!(!drain.is_ready());
        drop(guard);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), exit_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(drain.is_ready());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(!duplicate_exit.load(Ordering::SeqCst));
        assert!(installs.is_stopped());
        assert!(!drain.begin(&runtime, tasks, installs, LspStore::new(), TerminalStore::new(), || {}));
    }

    #[tokio::test]
    async fn 드레인_owner_drop은_완료_전_종료_callback을_실행하지_않는다() {
        let runtime = tokio::runtime::Handle::current();
        let tasks = TaskSupervisor::new(runtime.clone());
        let installs = LspInstallStore::new();
        let guard = installs.begin(&LspServerId::from("synthetic-exit")).unwrap();
        let mut drain = ExitDrain::default();
        let (exit, exit_rx) = oneshot::channel();
        assert!(drain.begin(
            &runtime,
            tasks,
            installs.clone(),
            LspStore::new(),
            TerminalStore::new(),
            move || {
                exit.send(()).unwrap();
            }
        ));
        drop(drain);
        assert!(exit_rx.await.is_err());
        drop(guard);
        installs.wait_for_idle().await;
    }
}
