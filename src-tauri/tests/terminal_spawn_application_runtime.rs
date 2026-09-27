use std::cell::Cell;
use std::sync::{Arc, Mutex};

use futures_util::FutureExt;
use taide_infra::pty::PtySpawnConfig;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_model::terminal::PtySpawnOptions;
use taide_runtime::terminal_actions::TerminalSpawnPorts;
use taide_runtime::{terminal_actions, AppState, EventSink, TaskSupervisor};
use taide_terminal::metadata::TerminalSessionMetadata;
use taide_terminal::session::TerminalSessionOutput;
use taide_terminal::store::TerminalStore;
use uuid::Uuid;

const TEST_COLS: u16 = 80;
const TEST_ROWS: u16 = 24;
#[cfg(unix)]
const FIXTURE_TIMEOUT_MS: u64 = 2_000;
#[cfg(unix)]
const ROOT_OBSERVATION_MS: u64 = 20;

#[derive(Default)]
struct RecordingSink(Mutex<Vec<AppEvent>>);

impl EventSink for RecordingSink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[test]
fn native_factory는_기존_출력_counter와_scan_exit_callback을_유지한다() {
    let source = include_str!("../src/domain/terminal/commands.rs");
    let native = source.split_once("fn spawn_session_for_application(").unwrap().1;
    let native = native.split_once("fn new_session_id(").unwrap().0;
    assert!(native.contains("spawn_terminal_session("));
    assert!(native.contains("perf::add(CounterSlot::PtyOutputBytes, bytes.len() as u64)"));
    assert!(native.contains("perf::add(CounterSlot::PtyOutputChunks, 1)"));
    assert!(native.contains("perf::add(CounterSlot::PtyScanEvents, outcome.events.len() as u64)"));
    assert!(native.contains("dispatch_scan_outcome(&scan_app, &session_id, &command_clock, outcome)"));
    let exit = native.split_once("move |code|").unwrap().1;
    assert!(exit.find("metadata.mark_exited()").unwrap() < exit.find("publish(AppEvent::TerminalExited").unwrap());
    assert!(exit.contains("session_id: exit_session_id"));
}

fn fixture() -> (AppState, TerminalStore, TaskSupervisor, PtySpawnOptions) {
    let dir = std::env::temp_dir().join(format!("taide-terminal-spawn-action-{}", Uuid::new_v4()));
    let state = AppState::new(AppPaths::new(dir));
    let opts = PtySpawnOptions {
        project_id: ProjectId::new(),
        cwd: state.paths.data_dir.join("root").to_string_lossy().into_owned(),
        shell: None,
        cols: TEST_COLS,
        rows: TEST_ROWS,
        scrollback_bytes: None,
    };
    (
        state,
        TerminalStore::new(),
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        opts,
    )
}

fn open_fixture_project(state: &AppState, opts: &PtySpawnOptions) {
    state.projects.write().insert(
        opts.project_id.clone(),
        Project {
            id: opts.project_id.clone(),
            root: opts.cwd.clone(),
            name: "fixture".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        },
    );
}

#[cfg(unix)]
struct DirectoryOwner(std::path::PathBuf);

#[cfg(unix)]
impl Drop for DirectoryOwner {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("자기 UUID fixture만 정리");
    }
}

#[cfg(unix)]
struct PublicationRelease(std::sync::mpsc::Sender<()>);

#[cfg(unix)]
impl Drop for PublicationRelease {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[cfg(unix)]
struct BlockingSpawnSink {
    state: AppState,
    store: TerminalStore,
    tasks: TaskSupervisor,
    published: Mutex<Option<tokio::sync::oneshot::Sender<AppEvent>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

#[cfg(unix)]
impl EventSink for BlockingSpawnSink {
    fn publish(&self, event: AppEvent) {
        let AppEvent::TerminalSpawned {
            session_id,
            project_id,
            cwd,
            shell,
        } = &event
        else {
            panic!("fixture는 spawn publication만 처리한다");
        };
        let sessions = self.store.sessions_for_project(project_id);
        assert_eq!(sessions.len(), 1);
        assert_eq!((&sessions[0].id, &sessions[0].cwd, &sessions[0].shell), (session_id, cwd, shell));
        assert!(self.state.begin_mutation().now_or_never().is_none());
        assert_eq!(self.tasks.tracked_count(), 1);
        self.published.lock().unwrap().take().unwrap().send(event).unwrap();
        self.release.lock().unwrap().recv().unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn 자기_shell의_spawn_이벤트는_삽입_뒤_guard_안에서_발행하고_정상_root는_반환을_기다린다() {
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_lsp::store::LspStore;
    use taide_runtime::ExitDrain;

    let (state, store, tasks, mut opts) = fixture();
    open_fixture_project(&state, &opts);
    std::fs::create_dir_all(&opts.cwd).unwrap();
    let _directory = DirectoryOwner(state.paths.data_dir.clone());
    opts.shell = Some("/bin/sh".to_string());
    let expected_project = opts.project_id.clone();
    let expected_cwd = opts.cwd.clone();
    let (published, published_rx) = tokio::sync::oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = PublicationRelease(release);
    let sink = BlockingSpawnSink {
        state: state.clone(),
        store: store.clone(),
        tasks: tasks.clone(),
        published: Mutex::new(Some(published)),
        release: Mutex::new(release_rx),
    };
    let request_state = state.clone();
    let request_store = store.clone();
    let request_tasks = tasks.clone();
    let runtime = tokio::runtime::Handle::current();
    let request_runtime = runtime.clone();
    let request = std::thread::spawn(move || {
        request_runtime.block_on(terminal_actions::pty_spawn(
            &sink,
            &request_state,
            &request_store,
            &request_tasks,
            opts,
            TerminalSpawnPorts {
                extra_env: async { vec![("ENV".to_string(), String::new()), ("BASH_ENV".to_string(), String::new())] },
                discard_initial_sink: || {},
                create_session_id: || "fixture-session".to_string(),
                create_session: |config, _, metadata: Arc<TerminalSessionMetadata>, output| {
                    taide_terminal::runtime::spawn_terminal_session(
                        config,
                        output,
                        |_| {},
                        |_| {},
                        move |_| {
                            metadata.mark_exited();
                        },
                    )
                },
            },
        ))
    });
    let event = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), published_rx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        event,
        AppEvent::TerminalSpawned {
            session_id: "fixture-session".to_string(),
            project_id: expected_project,
            cwd: expected_cwd,
            shell: "/bin/sh".to_string(),
        }
    );
    let mut drain = ExitDrain::default();
    let (finished, mut finished_rx) = tokio::sync::oneshot::channel();
    assert!(drain.begin(
        &runtime,
        tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        store.clone(),
        move || {
            let _ = finished.send(());
        }
    ));
    assert!(tokio::time::timeout(Duration::from_millis(ROOT_OBSERVATION_MS), &mut finished_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    drop(release);
    assert_eq!(request.join().unwrap().unwrap(), "fixture-session");
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), finished_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(drain.is_ready());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[tokio::test]
async fn 닫힌_worker와_store는_env_sink_id_뒤_factory를_실행하지_않는다() {
    for is_worker_stopped in [false, true] {
        let (state, store, tasks, opts) = fixture();
        open_fixture_project(&state, &opts);
        if is_worker_stopped {
            tasks.stop_all();
        } else {
            store.shutdown();
        }
        let stages = Mutex::new(Vec::new());
        let sink = RecordingSink::default();
        let error = terminal_actions::pty_spawn(
            &sink,
            &state,
            &store,
            &tasks,
            opts,
            TerminalSpawnPorts {
                extra_env: async {
                    stages.lock().unwrap().push("env");
                    Vec::new()
                },
                discard_initial_sink: || {
                    stages.lock().unwrap().push("sink");
                },
                create_session_id: || {
                    stages.lock().unwrap().push("id");
                    "fixture-session".to_string()
                },
                create_session: |_, _, _, _| panic!("closed worker/store must not create a session"),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(error, AppError::Forbidden(message) if message == "terminal runtime is shutting down"));
        assert_eq!(*stages.lock().unwrap(), ["env", "sink", "id"]);
        assert!(sink.0.lock().unwrap().is_empty());
        assert!(state.begin_mutation().now_or_never().is_some());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(!state.paths.data_dir.exists());
    }
}

#[tokio::test]
async fn env는_guard보다_먼저_완료하고_없는_project는_native_port를_실행하지_않는다() {
    let (state, store, tasks, opts) = fixture();
    let called = Cell::new(false);
    let sink = RecordingSink::default();
    let error = terminal_actions::pty_spawn(
        &sink,
        &state,
        &store,
        &tasks,
        opts,
        TerminalSpawnPorts {
            extra_env: async {
                assert!(state.begin_mutation().now_or_never().is_some());
                called.set(true);
                Vec::new()
            },
            discard_initial_sink: || panic!("missing project must not discard the sink explicitly"),
            create_session_id: || panic!("missing project must not allocate a session ID"),
            create_session: |_, _, _, _| panic!("missing project must not create a native session"),
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(called.get());
    assert!(state.begin_mutation().now_or_never().is_some());
    assert!(sink.0.lock().unwrap().is_empty());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn 지연_env와_guard_대기는_후속_native_port를_선실행하지_않는다() {
    let (state, store, tasks, opts) = fixture();
    let sink = RecordingSink::default();
    let called = Cell::new(false);
    let guard = state.begin_mutation().await;
    assert!(terminal_actions::pty_spawn(
        &sink,
        &state,
        &store,
        &tasks,
        opts.clone(),
        TerminalSpawnPorts {
            extra_env: async {
                called.set(true);
                Vec::new()
            },
            discard_initial_sink: || panic!("guard is held"),
            create_session_id: || panic!("guard is held"),
            create_session: |_, _, _, _| panic!("guard is held"),
        },
    )
    .now_or_never()
    .is_none());
    assert!(called.get());
    assert_eq!(tasks.tracked_count(), 0);
    drop(guard);
    assert!(terminal_actions::pty_spawn(
        &sink,
        &state,
        &store,
        &tasks,
        opts,
        TerminalSpawnPorts {
            extra_env: std::future::pending(),
            discard_initial_sink: || panic!("env is pending"),
            create_session_id: || panic!("env is pending"),
            create_session: |_, _, _, _| panic!("env is pending"),
        },
    )
    .now_or_never()
    .is_none());
    assert!(state.begin_mutation().now_or_never().is_some());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn shutdown은_env_뒤_project_오류보다_먼저_거절한다() {
    let (state, store, tasks, opts) = fixture();
    state.begin_shutdown();
    let called = Cell::new(false);
    let sink = RecordingSink::default();
    let error = terminal_actions::pty_spawn(
        &sink,
        &state,
        &store,
        &tasks,
        opts,
        TerminalSpawnPorts {
            extra_env: async {
                called.set(true);
                Vec::new()
            },
            discard_initial_sink: || panic!("app is shutting down"),
            create_session_id: || panic!("app is shutting down"),
            create_session: |_, _, _, _| panic!("app is shutting down"),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(message) if message == "terminal runtime is shutting down"));
    assert!(called.get());
    assert!(sink.0.lock().unwrap().is_empty());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn 서비스_실패는_원래_config_metadata와_port_순서를_유지하고_삽입과_이벤트를_생략한다() {
    let (state, store, tasks, opts) = fixture();
    open_fixture_project(&state, &opts);
    let stages = Arc::new(Mutex::new(Vec::new()));
    let sink_stages = stages.clone();
    let id_stages = stages.clone();
    let create_stages = stages.clone();
    let expected = opts.clone();
    let project_id = opts.project_id.clone();
    let worker_state = state.clone();
    let sink = RecordingSink::default();
    let error = terminal_actions::pty_spawn(
        &sink,
        &state,
        &store,
        &tasks,
        opts,
        TerminalSpawnPorts {
            extra_env: async {
                stages.lock().unwrap().push("env");
                vec![("FIXTURE_ENV".to_string(), "fixture".to_string())]
            },
            discard_initial_sink: move || {
                sink_stages.lock().unwrap().push("sink");
            },
            create_session_id: move || {
                id_stages.lock().unwrap().push("id");
                "fixture-session".to_string()
            },
            create_session: move |config: PtySpawnConfig,
                                  session_id,
                                  metadata: Arc<TerminalSessionMetadata>,
                                  output: Arc<TerminalSessionOutput>| {
                create_stages.lock().unwrap().push("factory");
                assert_eq!(config.cwd, expected.cwd);
                assert_eq!(config.shell, expected.shell);
                assert_eq!((config.cols, config.rows), (TEST_COLS, TEST_ROWS));
                assert_eq!(config.extra_env, [("FIXTURE_ENV".to_string(), "fixture".to_string())]);
                assert_eq!(session_id, "fixture-session");
                assert_eq!(metadata.project_id(), &expected.project_id);
                assert_eq!(metadata.cwd(), expected.cwd);
                assert_eq!(metadata.shell(), "default");
                assert!(worker_state.begin_mutation().now_or_never().is_none());
                let replay = output.attach(|_| true);
                output.detach(replay.subscription_id);
                Err(AppError::InvalidArgument("fixture-spawn-error".to_string()))
            },
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AppError::InvalidArgument(message) if message == "fixture-spawn-error"));
    assert_eq!(*stages.lock().unwrap(), ["env", "sink", "id", "factory"]);
    assert!(sink.0.lock().unwrap().is_empty());
    assert!(store.sessions_for_project(&project_id).is_empty());
    assert!(state.begin_mutation().now_or_never().is_some());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!state.paths.data_dir.exists());
}
