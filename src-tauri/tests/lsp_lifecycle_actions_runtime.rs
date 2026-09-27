use std::cell::Cell;
use std::sync::{Arc, Mutex};

use futures_util::FutureExt;
use taide_infra::lsp_proc::LspProcHandle;
use taide_lsp::manifest;
use taide_lsp::session::LspMessageSubscribers;
use taide_lsp::store::{LspSessionEntry, LspStore};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::ProjectId;
use taide_model::lsp::{LanguageServerSpec, LspSessionStatus, LspSpawnRequest};
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::lsp_actions::{LspActionContext, LspSpawnPorts};
use taide_runtime::{lsp_actions, AppState, EventSink, TaskSupervisor};
use uuid::Uuid;

#[cfg(unix)]
const PROCESS_FIXTURE_TIMEOUT_MS: u64 = 8_000;
#[cfg(unix)]
const ROOT_OBSERVATION_MS: u64 = 20;

#[derive(Default)]
struct Events(Mutex<Vec<AppEvent>>);

impl EventSink for Events {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct ObservedEvents<F> {
    events: Events,
    observe: F,
}

impl<F: Fn(&AppEvent) + Send + Sync> EventSink for ObservedEvents<F> {
    fn publish(&self, event: AppEvent) {
        (self.observe)(&event);
        self.events.publish(event);
    }
}

fn unused_sink() -> fn(&str) -> bool {
    panic!("입장 거절은 channel을 소비하지 않는다")
}

fn unused_id() -> String {
    panic!("입장 거절은 ID를 생성하지 않는다")
}

fn unused_process(_: String, _: u64, _: LanguageServerSpec, _: String) -> AppResult<Arc<LspProcHandle>> {
    panic!("입장 거절은 프로세스를 생성하지 않는다")
}

fn fixture() -> (AppState, LspStore, TaskSupervisor, LspSpawnRequest) {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-lsp-lifecycle-{}", Uuid::new_v4())),
    ));
    let request = LspSpawnRequest {
        project_id: ProjectId::new(),
        server_id: "gopls".into(),
        root: "fixture-root".to_string(),
        owner: "fixture-owner".to_string(),
    };
    (
        state,
        LspStore::new(),
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        request,
    )
}

fn open_project(state: &AppState, request: &LspSpawnRequest) {
    state.projects.write().insert(
        request.project_id.clone(),
        Project {
            id: request.project_id.clone(),
            root: request.root.clone(),
            name: "fixture".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        },
    );
}

fn seed_session(store: &LspStore, request: &LspSpawnRequest) -> Arc<LspSessionEntry> {
    let subscribers = LspMessageSubscribers::new();
    subscribers.insert(request.owner.clone(), |_| true);
    let entry = Arc::new(LspSessionEntry::new(
        request.project_id.clone(),
        manifest::find_spec(request.server_id.as_str()).unwrap(),
        request.root.clone(),
        subscribers,
    ));
    entry.lifecycle.set_status(LspSessionStatus::Running, None);
    store.insert("fixture-session".to_string(), entry.clone());
    entry
}

#[tokio::test]
async fn 같은_owner의_공유_세션은_id와_native_factory_없이_새_root를_등록한다() {
    let (state, store, tasks, mut request) = fixture();
    open_project(&state, &request);
    let entry = seed_session(&store, &request);
    let old_root = request.root.clone();
    request.root = "fixture-second-root".to_string();
    let received = Arc::new(Mutex::new(Vec::new()));
    let recorded = received.clone();
    let events = Events::default();
    let id = lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        request,
        LspSpawnPorts::new(
            move || {
                move |message: &str| {
                    recorded.lock().unwrap().push(message.to_string());
                    true
                }
            },
            || panic!("재사용은 ID를 생성하지 않는다"),
            |_: String, _: u64, _: LanguageServerSpec, _: String| -> AppResult<Arc<LspProcHandle>> {
                panic!("재사용은 프로세스를 생성하지 않는다")
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(id, "fixture-session");
    assert_eq!(entry.roots.paths(), vec![old_root, "fixture-second-root".to_string()]);
    entry.subscribers.broadcast("fixture-message");
    assert_eq!(*received.lock().unwrap(), vec!["fixture-message"]);
    assert!(events.0.lock().unwrap().is_empty());
    assert_eq!(entry.lifecycle.snapshot().generation, 0);
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[tokio::test]
async fn mutation_대기와_project_server_오류는_native_port_호출보다_앞선다() {
    let (state, store, tasks, mut request) = fixture();
    let events = Events::default();
    request.server_id = "fixture-unknown".into();
    let guard = state.begin_mutation().await;
    let mut action = Box::pin(lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        request.clone(),
        LspSpawnPorts::new(unused_sink, unused_id, unused_process),
    ));
    assert!(action.as_mut().now_or_never().is_none());
    assert_eq!(tasks.tracked_count(), 0);
    drop(guard);
    let error = action.await.unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(
        error.to_string(),
        format!("path not found: project not open: {}", request.project_id)
    );
    open_project(&state, &request);
    tasks.stop_all();
    let error = lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        request.clone(),
        LspSpawnPorts::new(unused_sink, unused_id, unused_process),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert_eq!(error.to_string(), "invalid argument: unknown language server: fixture-unknown");
    request.server_id = "gopls".into();
    let error = lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        request,
        LspSpawnPorts::new(unused_sink, unused_id, unused_process),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    assert_eq!(error.to_string(), "forbidden: language server runtime is shutting down");
    assert!(events.0.lock().unwrap().is_empty());
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[tokio::test]
async fn spawn_실패는_먼저_등록한_entry를_반납하고_guard와_operation을_해제한다() {
    let (state, store, tasks, request) = fixture();
    open_project(&state, &request);
    let events = Events::default();
    let expected = request.clone();
    let channel_called = Cell::new(false);
    let error = lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        request,
        LspSpawnPorts::new(
            || {
                channel_called.set(true);
                |_: &str| true
            },
            || "fixture-session".to_string(),
            |id: String, epoch, spec: LanguageServerSpec, root| {
                assert_eq!(id, "fixture-session");
                assert_eq!(epoch, 1);
                assert_eq!(spec.id, expected.server_id);
                assert_eq!(root, expected.root);
                let entry = store.get(&id).unwrap();
                assert!(entry.lifecycle.is_active_process_epoch(epoch));
                assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Starting);
                assert!(entry.subscribers.contains(&expected.owner));
                assert_eq!(tasks.tracked_count(), 1);
                assert!(state.begin_mutation().now_or_never().is_none());
                Err(AppError::Internal("fixture spawn failed".to_string()))
            },
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "operation failed: fixture spawn failed");
    assert!(channel_called.get());
    assert!(!store.contains("fixture-session"));
    assert!(events.0.lock().unwrap().is_empty());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[tokio::test]
async fn stopping_다른_owner_빈_root는_기존_세션을_재사용하지_않는다() {
    for case in ["stopping", "other-owner", "empty-root"] {
        let (state, store, tasks, mut request) = fixture();
        open_project(&state, &request);
        let entry = seed_session(&store, &request);
        match case {
            "stopping" => entry.lifecycle.mark_stopping(),
            "other-owner" => request.owner = "other-owner".to_string(),
            "empty-root" => request.root.clear(),
            _ => unreachable!(),
        }
        let called = Cell::new(false);
        let events = Events::default();
        let error = lsp_actions::lsp_spawn(
            &events,
            LspActionContext::new(&state, &store, &tasks),
            request,
            LspSpawnPorts::new(
                || |_: &str| true,
                || "fixture-replacement".to_string(),
                |_, _, _, _| {
                    called.set(true);
                    Err(AppError::Internal("fixture replacement failure".to_string()))
                },
            ),
        )
        .await
        .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        assert!(called.get());
        assert!(Arc::ptr_eq(&entry, &store.get("fixture-session").unwrap()));
        assert!(!store.contains("fixture-replacement"));
        assert!(events.0.lock().unwrap().is_empty());
        assert_eq!(tasks.tracked_count(), 0);
    }
}

#[tokio::test]
async fn closed_process_store는_factory를_거절하고_새_entry만_반납한다() {
    let (state, store, tasks, request) = fixture();
    open_project(&state, &request);
    store.shutdown();
    let events = Events::default();
    let error = lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        request,
        LspSpawnPorts::new(|| |_: &str| true, || "fixture-session".to_string(), unused_process),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    assert!(!store.contains("fixture-session"));
    assert!(events.0.lock().unwrap().is_empty());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 부분_root_해제와_중복_참조는_구독을_유지하고_최종_해제만_정지한다() {
    let (state, store, tasks, request) = fixture();
    let entry = seed_session(&store, &request);
    entry.roots.acquire(request.root.clone());
    entry.roots.acquire("fixture-second-root".to_string());
    let events = ObservedEvents {
        events: Events::default(),
        observe: |event: &AppEvent| {
            assert!(matches!(
                event,
                AppEvent::LspSessionStatusChanged {
                    status: LspSessionStatus::Stopped,
                    generation: 0,
                    ..
                }
            ));
            assert!(!store.contains("fixture-session"));
            assert!(!entry.subscribers.contains(&request.owner));
            assert!(entry.lifecycle.is_stopping());
            assert!(state.begin_mutation().now_or_never().is_some());
            assert_eq!(tasks.tracked_count(), 1);
        },
    };
    for root in ["fixture-root", "fixture-root", "fixture-unknown-root", "fixture-second-root"] {
        lsp_actions::lsp_stop(
            &events,
            LspActionContext::new(&state, &store, &tasks),
            "fixture-session".to_string(),
            Some(root.to_string()),
            request.owner.clone(),
        )
        .await
        .unwrap();
        if root != "fixture-second-root" {
            assert!(store.contains("fixture-session"));
            assert!(entry.subscribers.contains(&request.owner));
            assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Running);
            assert!(events.events.0.lock().unwrap().is_empty());
        }
    }
    assert_eq!(events.events.0.lock().unwrap().len(), 1);
    assert_eq!(tasks.tracked_count(), 0);
    assert!(entry.roots.paths().is_empty());
    let error = lsp_actions::lsp_stop(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        "fixture-session".to_string(),
        None,
        request.owner.clone(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(events.events.0.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn rootless_stop은_전체_해제하며_closed_tasks의_기존_session은_변경하지_않는다() {
    let (state, store, tasks, request) = fixture();
    let entry = seed_session(&store, &request);
    entry.roots.acquire("fixture-second-root".to_string());
    let events = Events::default();
    lsp_actions::lsp_stop(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        "fixture-session".to_string(),
        None,
        request.owner.clone(),
    )
    .await
    .unwrap();
    assert!(!store.contains("fixture-session"));
    assert!(!entry.subscribers.contains(&request.owner));
    assert_eq!(entry.roots.paths(), vec![request.root.clone(), "fixture-second-root".to_string()]);
    let entry = seed_session(&store, &request);
    tasks.stop_all();
    for id in ["fixture-missing", "fixture-session"] {
        let expected = if id == "fixture-missing" {
            AppErrorKind::NotFound
        } else {
            AppErrorKind::Forbidden
        };
        let error = lsp_actions::lsp_stop(
            &events,
            LspActionContext::new(&state, &store, &tasks),
            id.to_string(),
            None,
            request.owner.clone(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.kind(), expected);
        let error = lsp_actions::lsp_restart(
            &events,
            LspActionContext::new(&state, &store, &tasks),
            id.to_string(),
            unused_process,
        )
        .await
        .unwrap_err();
        assert_eq!(error.kind(), expected);
    }
    assert!(!entry.lifecycle.is_stopping());
    assert!(entry.subscribers.contains(&request.owner));
    assert_eq!(events.0.lock().unwrap().len(), 1);
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[tokio::test]
async fn restart는_비가드_정지_중_삭제된_session을_다시_생성하지_않는다() {
    let (state, store, tasks, request) = fixture();
    let entry = seed_session(&store, &request);
    let events = ObservedEvents {
        events: Events::default(),
        observe: |event: &AppEvent| {
            assert!(matches!(
                event,
                AppEvent::LspSessionStatusChanged {
                    status: LspSessionStatus::Stopped,
                    ..
                }
            ));
            assert!(entry.lifecycle.is_stopping());
            assert!(state.begin_mutation().now_or_never().is_some());
            assert!(store.remove("fixture-session").is_some());
            assert_eq!(tasks.tracked_count(), 1);
        },
    };
    let error = lsp_actions::lsp_restart(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        "fixture-session".to_string(),
        unused_process,
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(events.events.0.lock().unwrap().len(), 1);
    assert!(entry.lifecycle.is_stopping());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[tokio::test]
async fn restart_factory_실패는_기존_generation과_starting_상태를_유지한다() {
    let (state, store, tasks, request) = fixture();
    let entry = seed_session(&store, &request);
    let old_epoch = entry.lifecycle.advance_process_epoch();
    entry.lifecycle.auto_respawned("fixture old error".to_string());
    let events = ObservedEvents {
        events: Events::default(),
        observe: |event: &AppEvent| {
            let AppEvent::LspSessionStatusChanged {
                status,
                generation,
                last_error,
                ..
            } = event
            else {
                panic!("LSP status만 발행한다")
            };
            assert_eq!(*generation, 1);
            assert!(last_error.is_none());
            assert!(store.contains("fixture-session"));
            assert_eq!(tasks.tracked_count(), 1);
            match status {
                LspSessionStatus::Stopped => {
                    assert!(entry.lifecycle.is_stopping());
                    assert!(state.begin_mutation().now_or_never().is_some());
                }
                LspSessionStatus::Starting => {
                    assert!(!entry.lifecycle.is_stopping());
                    assert!(state.begin_mutation().now_or_never().is_none());
                }
                _ => panic!("실패한 respawn은 running을 발행하지 않는다"),
            }
        },
    };
    let error = lsp_actions::lsp_restart(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        "fixture-session".to_string(),
        |id, epoch, spec, root| {
            assert_eq!(id, "fixture-session");
            assert_eq!(epoch, old_epoch + 1);
            assert_eq!(spec, entry.spec);
            assert_eq!(root, entry.root);
            assert!(entry.lifecycle.is_active_process_epoch(epoch));
            Err(AppError::Internal("fixture restart failed".to_string()))
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert_eq!(entry.lifecycle.snapshot().status, LspSessionStatus::Starting);
    assert_eq!(entry.lifecycle.snapshot().generation, 1);
    assert_eq!(events.events.0.lock().unwrap().len(), 2);
    assert!(Arc::ptr_eq(&entry, &store.get("fixture-session").unwrap()));
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[cfg(unix)]
struct FixtureOwner {
    directory: std::path::PathBuf,
    processes: LspStore,
}

#[cfg(unix)]
impl Drop for FixtureOwner {
    fn drop(&mut self) {
        self.processes.shutdown();
        std::fs::remove_dir_all(&self.directory).expect("자기 UUID fixture만 정리");
    }
}

#[cfg(unix)]
fn fixture_process(store: &LspStore, id: String, epoch: u64, root: String) -> AppResult<Arc<LspProcHandle>> {
    let messages = store.clone();
    Ok(Arc::new(taide_infra::lsp_proc::spawn(
        taide_infra::lsp_proc::LspProcConfig {
            command: "/bin/cat".to_string(),
            args: Vec::new(),
            cwd: root.into(),
        },
        move |message| {
            let Some(entry) = messages.get(&id) else { return };
            if entry.lifecycle.is_active_process_epoch(epoch) {
                entry.subscribers.broadcast(&message);
            }
        },
        |_, _| {},
    )?))
}

#[cfg(unix)]
#[tokio::test]
async fn 통제된_echo_process는_workspace_추가_해제와_restart의_같은_id를_유지한다() {
    use std::time::Duration;

    use taide_lsp::protocol::workspace_folders_notification;

    let (state, store, tasks, mut request) = fixture();
    std::fs::create_dir_all(&state.paths.data_dir).unwrap();
    let _fixture = FixtureOwner {
        directory: state.paths.data_dir.clone(),
        processes: store.clone(),
    };
    request.root = state.paths.data_dir.to_string_lossy().into_owned();
    open_project(&state, &request);
    let events = ObservedEvents {
        events: Events::default(),
        observe: |event: &AppEvent| {
            let AppEvent::LspSessionStatusChanged {
                session_id,
                status,
                generation,
                last_error,
            } = event
            else {
                panic!("LSP status만 발행한다")
            };
            assert_eq!(session_id, "fixture-session");
            assert_eq!(*generation, 0);
            assert!(last_error.is_none());
            assert_eq!(tasks.tracked_count(), 1);
            if *status == LspSessionStatus::Stopped {
                assert!(state.begin_mutation().now_or_never().is_some());
            } else {
                assert!(state.begin_mutation().now_or_never().is_none());
            }
        },
    };
    let id = lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        request.clone(),
        LspSpawnPorts::new(
            || |_: &str| true,
            || "fixture-session".to_string(),
            |id, epoch, _, root| fixture_process(&store, id, epoch, root),
        ),
    )
    .await
    .unwrap();
    let entry = store.get(&id).unwrap();
    let first = entry.proc.lock().clone().unwrap();
    assert!(!first.is_exited());
    let (messages, mut messages_rx) = tokio::sync::mpsc::unbounded_channel();
    let second_root = state.paths.data_dir.join("한글 second root").to_string_lossy().into_owned();
    let mut second_request = request.clone();
    second_request.root = second_root.clone();
    let reused = lsp_actions::lsp_spawn(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        second_request,
        LspSpawnPorts::new(
            move || move |message: &str| messages.send(message.to_string()).is_ok(),
            unused_id,
            unused_process,
        ),
    )
    .await
    .unwrap();
    assert_eq!(reused, id);
    let added = tokio::time::timeout(Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS), messages_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(added, workspace_folders_notification(std::slice::from_ref(&second_root), &[]));
    lsp_actions::lsp_stop(
        &events,
        LspActionContext::new(&state, &store, &tasks),
        id.clone(),
        Some(second_root.clone()),
        request.owner.clone(),
    )
    .await
    .unwrap();
    let removed = tokio::time::timeout(Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS), messages_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(removed, workspace_folders_notification(&[], std::slice::from_ref(&second_root)));
    assert!(entry.subscribers.contains(&request.owner));
    assert_eq!(events.events.0.lock().unwrap().len(), 1);
    tokio::time::timeout(
        Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS),
        lsp_actions::lsp_restart(
            &events,
            LspActionContext::new(&state, &store, &tasks),
            id.clone(),
            |id, epoch, _, root| {
                assert_eq!(epoch, 2);
                fixture_process(&store, id, epoch, root)
            },
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let second = entry.proc.lock().clone().unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    assert!(entry.lifecycle.is_active_process_epoch(2));
    assert!(!entry.lifecycle.is_active_process_epoch(1));
    assert_eq!(entry.roots.paths(), vec![request.root]);
    assert!(Arc::ptr_eq(&entry, &store.get(&id).unwrap()));
    let statuses = events
        .events
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|event| {
            let AppEvent::LspSessionStatusChanged { status, .. } = event else {
                unreachable!()
            };
            *status
        })
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        [
            LspSessionStatus::Running,
            LspSessionStatus::Stopped,
            LspSessionStatus::Starting,
            LspSessionStatus::Running
        ]
    );
    store.remove(&id);
    store.shutdown();
    tokio::time::timeout(Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS), store.wait_for_idle())
        .await
        .unwrap();
    assert!(first.is_finished());
    assert!(second.is_finished());
    assert_eq!(tasks.tracked_count(), 0);
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
    store: LspStore,
    tasks: TaskSupervisor,
    published: Mutex<Option<tokio::sync::oneshot::Sender<Arc<LspProcHandle>>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

#[cfg(unix)]
impl EventSink for BlockingSpawnSink {
    fn publish(&self, event: AppEvent) {
        let AppEvent::LspSessionStatusChanged {
            session_id,
            status: LspSessionStatus::Running,
            generation: 0,
            last_error: None,
        } = event
        else {
            panic!("fixture는 최초 running publication만 처리한다")
        };
        let entry = self.store.get(&session_id).unwrap();
        let process = entry.proc.lock().clone().unwrap();
        assert!(entry.lifecycle.is_active_process_epoch(1));
        assert!(self.state.begin_mutation().now_or_never().is_none());
        assert_eq!(self.tasks.tracked_count(), 1);
        assert!(self.published.lock().unwrap().take().unwrap().send(process).is_ok());
        self.release.lock().unwrap().recv().unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn 정상_root는_running_발행의_action_owner와_등록된_process의_실제_종료를_기다린다() {
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_runtime::ExitDrain;
    use taide_terminal::store::TerminalStore;

    let (state, store, tasks, mut request) = fixture();
    std::fs::create_dir_all(&state.paths.data_dir).unwrap();
    let _fixture = FixtureOwner {
        directory: state.paths.data_dir.clone(),
        processes: store.clone(),
    };
    request.root = state.paths.data_dir.to_string_lossy().into_owned();
    open_project(&state, &request);
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
    let action_runtime = runtime.clone();
    let action = std::thread::spawn(move || {
        action_runtime.block_on(lsp_actions::lsp_spawn(
            &sink,
            LspActionContext::new(&request_state, &request_store, &request_tasks),
            request,
            LspSpawnPorts::new(
                || |_: &str| true,
                || "fixture-session".to_string(),
                |id, epoch, _, root| fixture_process(&request_store, id, epoch, root),
            ),
        ))
    });
    let process = tokio::time::timeout(Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS), published_rx)
        .await
        .unwrap()
        .unwrap();
    let mut drain = ExitDrain::default();
    let (finished, mut finished_rx) = tokio::sync::oneshot::channel();
    assert!(drain.begin(
        &runtime,
        tasks.clone(),
        LspInstallStore::new(),
        store.clone(),
        TerminalStore::new(),
        move || {
            let _ = finished.send(());
        }
    ));
    assert!(tokio::time::timeout(Duration::from_millis(ROOT_OBSERVATION_MS), &mut finished_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    assert!(state.begin_mutation().now_or_never().is_none());
    drop(release);
    tokio::time::timeout(Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS), finished_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(drain.is_ready());
    assert_eq!(action.join().unwrap().unwrap(), "fixture-session");
    assert!(process.is_exited());
    assert!(process.is_finished());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(state.begin_mutation().now_or_never().is_some());
}

#[test]
fn native_callback은_epoch_확인과_exit_owner를_유지하며_application은_한번만_등록한다() {
    let source = include_str!("../src/domain/lsp/commands.rs");
    let native = source
        .split_once("fn create_process(")
        .unwrap()
        .1
        .split_once("fn channel_sink(")
        .unwrap()
        .0;
    assert!(native.contains("spawn_language_server("));
    assert!(native.contains("entry.lifecycle.is_active_process_epoch(process_epoch)"));
    assert!(native.contains("entry.subscribers.broadcast(&message)"));
    assert!(native.contains("spawn_transient(\"lsp-process-exit\""));
    assert!(native.contains("handle_process_exit(&task_app, exit_session_id, process_epoch, code, stderr_tail)"));
    assert!(!native.contains(".spawn_process("));
    for name in ["lsp_spawn", "lsp_stop", "lsp_restart"] {
        let action = source.split_once(&format!("pub async fn {name}(")).unwrap().1;
        let action = action.split_once("\n}").unwrap().0;
        assert!(action.contains(&format!("lsp_actions::{name}(")));
        assert!(!action.contains("store.spawn_process("));
        assert!(!action.contains("spawn_process(&app"));
    }
    let runtime = include_str!("../../crates/taide-runtime/src/lsp_actions.rs");
    assert_eq!(runtime.matches("store.spawn_process(||").count(), 2);
    assert!(runtime.contains("guard: Option<OwnedMutexGuard<()>>,"));
    assert!(runtime.contains("_lease: TaskOperationLease,"));
}

#[cfg(unix)]
#[tokio::test]
async fn stop_요청_abort로_삭제된_session의_process도_정상_root가_드레인한다() {
    use std::time::Duration;

    use taide_lsp::install::LspInstallStore;
    use taide_runtime::ExitDrain;
    use taide_terminal::store::TerminalStore;

    let (state, store, tasks, mut request) = fixture();
    std::fs::create_dir_all(&state.paths.data_dir).unwrap();
    let _fixture = FixtureOwner {
        directory: state.paths.data_dir.clone(),
        processes: store.clone(),
    };
    request.root = state.paths.data_dir.to_string_lossy().into_owned();
    let entry = seed_session(&store, &request);
    let epoch = entry.lifecycle.advance_process_epoch();
    let process = store
        .spawn_process(|| fixture_process(&store, "fixture-session".to_string(), epoch, request.root.clone()))
        .unwrap();
    *entry.proc.lock() = Some(process.clone());
    let events = Arc::new(Events::default());
    let action_events = events.clone();
    let action_state = state.clone();
    let action_store = store.clone();
    let action_tasks = tasks.clone();
    let action = tokio::spawn(async move {
        lsp_actions::lsp_stop(
            action_events.as_ref(),
            LspActionContext::new(&action_state, &action_store, &action_tasks),
            "fixture-session".to_string(),
            None,
            request.owner,
        )
        .await
    });
    tokio::time::timeout(Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS), async {
        while store.contains("fixture-session") || !entry.lifecycle.is_stopping() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(state.begin_mutation().now_or_never().is_some());
    assert_eq!(tasks.tracked_count(), 1);
    assert!(!process.is_finished());
    action.abort();
    assert!(action.await.unwrap_err().is_cancelled());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(events.0.lock().unwrap().is_empty());
    let runtime = tokio::runtime::Handle::current();
    let mut drain = ExitDrain::default();
    let (finished, finished_rx) = tokio::sync::oneshot::channel();
    assert!(
        drain.begin(&runtime, tasks, LspInstallStore::new(), store, TerminalStore::new(), move || {
            let _ = finished.send(());
        })
    );
    tokio::time::timeout(Duration::from_millis(PROCESS_FIXTURE_TIMEOUT_MS), finished_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(drain.is_ready());
    assert!(process.is_exited());
    assert!(process.is_finished());
    assert!(state.begin_mutation().now_or_never().is_some());
}
