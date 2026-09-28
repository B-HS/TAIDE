use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt;
use parking_lot::Mutex;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use taide_model::paths::AppPaths;
use taide_model::project::{CapabilityKind, OpenProjectInSlotRequest, Project, ProjectDisplay, ProjectGroup, ShellSlotEdge};
use taide_project::{service, shell_slots};
use taide_runtime::project_actions::{self, ProjectLifecyclePort};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::oneshot;
use uuid::Uuid;

struct NoLifecycle;

impl ProjectLifecyclePort for NoLifecycle {
    fn detected_kinds(&self, _: &std::path::Path) -> Vec<CapabilityKind> {
        panic!("이 gate는 detect를 호출하지 않는다")
    }
    async fn attach_project_capabilities(&self, _: &Project) -> AppResult<()> {
        panic!("이 gate는 attach를 호출하지 않는다")
    }
    async fn await_project_flush(&self, _: &ProjectId) {
        panic!("이 gate는 flush를 호출하지 않는다")
    }
    fn detach_all(&self, _: &ProjectId) {
        panic!("이 gate는 detach를 호출하지 않는다")
    }
}

struct NoEvents;

const ACTION_TIMEOUT: Duration = Duration::from_secs(5);
const ROOT_WAIT_PROBE: Duration = Duration::from_millis(20);

struct HeldLifecycle {
    started: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<Option<oneshot::Receiver<()>>>,
    should_fail: bool,
}

impl ProjectLifecyclePort for HeldLifecycle {
    fn detected_kinds(&self, _: &Path) -> Vec<CapabilityKind> {
        Vec::new()
    }

    async fn attach_project_capabilities(&self, _: &Project) -> AppResult<()> {
        self.started.lock().take().unwrap().send(()).ok();
        let release = self.release.lock().take().unwrap();
        release.await.unwrap();
        if self.should_fail {
            return Err(AppError::Internal("fixture attach failure".to_string()));
        }
        Ok(())
    }

    async fn await_project_flush(&self, _: &ProjectId) {}

    fn detach_all(&self, _: &ProjectId) {}
}

#[test]
fn native_프로젝트_열기_세_경로는_요청_취소와_독립된_완료_owner를_사용한다() {
    let commands = include_str!("../src/domain/project/commands.rs");
    assert_eq!(commands.matches(".run_nonabortable_result(\"project-").count(), 3);
}

struct Fixture {
    base: PathBuf,
    state: AppState,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!("taide-project-lifecycle-{}", Uuid::new_v4()));
        Self {
            state: AppState::new(AppPaths::new(base.join("data"))),
            base,
        }
    }

    fn root(&self, name: &str) -> String {
        let root = self.base.join(name);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::canonicalize(root).unwrap().to_string_lossy().into_owned()
    }

    fn seed(&self, name: &str) -> Project {
        let mut session = self.state.session.read().clone();
        let mut projects = self.state.projects.read().clone();
        let result = service::open_project(
            &self.state.paths,
            &mut session,
            &mut projects,
            Path::new(&self.root(name)),
            true,
            |_| Vec::new(),
        )
        .unwrap();
        *self.state.session.write() = session;
        *self.state.projects.write() = projects;
        result.project
    }

    fn history(&self, name: &str) -> Project {
        let project = Project {
            id: ProjectId::new(),
            root: self.root(name),
            name: name.to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        service::save_project(&self.state.paths, &project).unwrap();
        project
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.base.exists() {
            std::fs::remove_dir_all(&self.base).expect("자기 UUID fixture만 정리");
        }
    }
}

#[derive(Default)]
struct Events {
    recorded: Mutex<Vec<AppEvent>>,
    is_guard_held: Mutex<Vec<bool>>,
}

impl Events {
    fn names(&self) -> Vec<&'static str> {
        self.recorded
            .lock()
            .iter()
            .map(|event| match event {
                AppEvent::ProjectOpened { .. } => "opened",
                AppEvent::ProjectListChanged { .. } => "list",
                AppEvent::ProjectActivated { .. } => "activated",
                AppEvent::SessionShellSlotsChanged { .. } => "slots",
                AppEvent::ProjectClosed { .. } => "closed",
                _ => panic!("fixture event"),
            })
            .collect()
    }
}

impl EventSink for Events {
    fn publish(&self, event: AppEvent) {
        self.recorded.lock().push(event);
    }
}

struct InspectingEvents<'a> {
    state: &'a AppState,
    events: &'a Events,
}

impl EventSink for InspectingEvents<'_> {
    fn publish(&self, event: AppEvent) {
        assert!(self.state.session.try_write().is_some());
        assert_eq!(service::load_session(&self.state.paths).unwrap().0, *self.state.session.read());
        self.events
            .is_guard_held
            .lock()
            .push(self.state.begin_mutation().now_or_never().is_none());
        self.events.recorded.lock().push(event);
    }
}

type OnAttach<'a> = Box<dyn Fn(&Project) + Send + Sync + 'a>;

struct Lifecycle<'a> {
    state: &'a AppState,
    calls: &'a Mutex<Vec<String>>,
    failing_name: Option<&'a str>,
    on_attach: OnAttach<'a>,
}

impl<'a> Lifecycle<'a> {
    fn new(state: &'a AppState, calls: &'a Mutex<Vec<String>>) -> Self {
        Self {
            state,
            calls,
            failing_name: None,
            on_attach: Box::new(|_| {}),
        }
    }
}

impl ProjectLifecyclePort for Lifecycle<'_> {
    fn detected_kinds(&self, root: &Path) -> Vec<CapabilityKind> {
        assert!(root.is_absolute());
        assert!(self.state.begin_mutation().now_or_never().is_none());
        self.calls.lock().push("detect".to_string());
        vec![CapabilityKind::Git, CapabilityKind::Terminal]
    }

    async fn attach_project_capabilities(&self, project: &Project) -> AppResult<()> {
        assert!(self.state.begin_mutation().now_or_never().is_some());
        assert!(self.state.projects.read().contains_key(&project.id));
        assert_eq!(service::load_session(&self.state.paths).unwrap().0, *self.state.session.read());
        self.calls.lock().push(format!("attach:{}", project.name));
        (self.on_attach)(project);
        tokio::task::yield_now().await;
        if self.failing_name == Some(project.name.as_str()) {
            return Err(AppError::Internal("fixture attach failure".to_string()));
        }
        Ok(())
    }

    async fn await_project_flush(&self, _: &ProjectId) {
        assert!(self.state.begin_mutation().now_or_never().is_some());
        self.calls.lock().push("flush".to_string());
        tokio::task::yield_now().await;
    }

    fn detach_all(&self, project_id: &ProjectId) {
        assert!(self.state.begin_mutation().now_or_never().is_none());
        assert!(!self.state.projects.read().contains_key(project_id));
        assert!(self.state.session.read().projects.iter().all(|project| &project.id != project_id));
        self.calls.lock().push("detach".to_string());
    }
}

impl EventSink for NoEvents {
    fn publish(&self, _: taide_model::app_event::AppEvent) {
        panic!("이 gate는 이벤트를 발행하지 않는다")
    }
}

#[tokio::test]
async fn close의_닫힌_project는_flush_detach와_저장_이벤트에_도달하지_않는다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-project-lifecycle-{}", Uuid::new_v4())),
    ));
    let error = project_actions::project_close(&NoEvents, &state, &NoLifecycle, ProjectId::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn open의_missing과_non_directory는_detect_attach_저장_이벤트보다_먼저_거절된다() {
    let fixture = Fixture::new();
    let missing = fixture.base.join("missing");
    let error = project_actions::project_open(&NoEvents, &fixture.state, &NoLifecycle, missing.to_string_lossy().into_owned())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(!fixture.state.paths.data_dir.exists());
    std::fs::create_dir_all(&fixture.base).unwrap();
    let file = fixture.base.join("file");
    std::fs::write(&file, "fixture file").unwrap();
    let error = project_actions::project_open(&NoEvents, &fixture.state, &NoLifecycle, file.to_string_lossy().into_owned())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert!(!fixture.state.paths.data_dir.exists());
}

#[tokio::test]
async fn open은_저장_state_뒤_guard_밖_attach를_기다리고_재열기는_활성화만_발행한다() {
    let fixture = Fixture::new();
    let calls = Mutex::new(Vec::new());
    let lifecycle = Lifecycle::new(&fixture.state, &calls);
    let events = Events::default();
    let sink = InspectingEvents {
        state: &fixture.state,
        events: &events,
    };
    let root = fixture.root("first");
    let opening = project_actions::project_open(&sink, &fixture.state, &lifecycle, root.clone());
    tokio::pin!(opening);
    assert!(opening.as_mut().now_or_never().is_none());
    assert_eq!(fixture.state.projects.read().len(), 1);
    assert!(events.recorded.lock().is_empty());
    assert_eq!(*calls.lock(), ["detect", "attach:first"]);
    let opened = opening.await.unwrap();
    assert!(!opened.already_open);
    assert_eq!(opened.project.capabilities, [CapabilityKind::Git, CapabilityKind::Terminal]);
    assert_eq!(events.names(), ["opened", "list", "activated", "slots"]);
    assert!(events.is_guard_held.lock().iter().all(|held| !held));
    events.recorded.lock().clear();
    let reopened = project_actions::project_open(&sink, &fixture.state, &NoLifecycle, root)
        .await
        .unwrap();
    assert!(reopened.already_open);
    assert_eq!(reopened.project.id, opened.project.id);
    assert_eq!(events.names(), ["activated", "slots"]);
    assert_eq!(*calls.lock(), ["detect", "attach:first"]);
}

#[tokio::test]
async fn 요청_취소_뒤에도_project_attach_성공과_실패_rollback을_root가_기다린다() {
    for should_fail in [false, true] {
        let fixture = Fixture::new();
        let root = fixture.root("held");
        let state = fixture.state.clone();
        let events = Arc::new(Events::default());
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let request_tasks = tasks.clone();
        let (started, started_rx) = oneshot::channel();
        let (release, held) = oneshot::channel();
        let lifecycle = HeldLifecycle {
            started: Mutex::new(Some(started)),
            release: Mutex::new(Some(held)),
            should_fail,
        };
        let request_events = events.clone();
        let request = tokio::spawn(async move {
            request_tasks
                .run_nonabortable_result("project-open", async move {
                    project_actions::project_open(request_events.as_ref(), &state, &lifecycle, root).await
                })
                .await
        });
        tokio::time::timeout(ACTION_TIMEOUT, started_rx).await.unwrap().unwrap();
        assert_eq!(fixture.state.projects.read().len(), 1);
        assert!(events.recorded.lock().is_empty());
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        tasks.stop_all();
        let shutdown = tasks.shutdown();
        tokio::pin!(shutdown);
        assert!(tokio::time::timeout(ROOT_WAIT_PROBE, &mut shutdown).await.is_err());
        release.send(()).unwrap();
        tokio::time::timeout(ACTION_TIMEOUT, shutdown).await.unwrap();
        if should_fail {
            assert!(fixture.state.projects.read().is_empty());
            assert_eq!(events.names(), ["closed", "activated", "slots", "list"]);
        } else {
            assert_eq!(fixture.state.projects.read().len(), 1);
            assert_eq!(events.names(), ["opened", "list", "activated", "slots"]);
        }
    }
}

#[tokio::test]
async fn open의_attach_실패는_close_flush_detach로_되돌리고_original_오류를_반환한다() {
    let fixture = Fixture::new();
    let calls = Mutex::new(Vec::new());
    let mut lifecycle = Lifecycle::new(&fixture.state, &calls);
    lifecycle.failing_name = Some("failed");
    let events = Events::default();
    let sink = InspectingEvents {
        state: &fixture.state,
        events: &events,
    };
    let error = project_actions::project_open(&sink, &fixture.state, &lifecycle, fixture.root("failed"))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Internal(message) if message == "fixture attach failure"));
    assert!(fixture.state.projects.read().is_empty());
    assert!(fixture.state.session.read().projects.is_empty());
    assert_eq!(*calls.lock(), ["detect", "attach:failed", "flush", "detach"]);
    assert_eq!(events.names(), ["closed", "activated", "slots", "list"]);
    assert!(events.is_guard_held.lock().iter().all(|held| *held));
}

#[tokio::test]
async fn rollback_저장도_실패하면_attach_오류를_덮지_않고_detach와_event는_실행하지_않는다() {
    let fixture = Fixture::new();
    let calls = Mutex::new(Vec::new());
    let mut lifecycle = Lifecycle::new(&fixture.state, &calls);
    lifecycle.failing_name = Some("failed");
    lifecycle.on_attach = Box::new(|_| {
        let file = fixture.state.paths.session_file();
        std::fs::remove_file(&file).unwrap();
        std::fs::create_dir(&file).unwrap();
    });
    let error = project_actions::project_open(&NoEvents, &fixture.state, &lifecycle, fixture.root("failed"))
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Internal(message) if message == "fixture attach failure"));
    assert_eq!(fixture.state.projects.read().len(), 1);
    assert_eq!(*calls.lock(), ["detect", "attach:failed", "flush"]);
    assert!(fixture.state.paths.session_file().is_dir());
}

#[tokio::test]
async fn close의_두_flush_경합은_guard_재검사로_한_번만_저장_reap_event를_실행한다() {
    let fixture = Fixture::new();
    let project = fixture.seed("first");
    let calls = Mutex::new(Vec::new());
    let lifecycle = Lifecycle::new(&fixture.state, &calls);
    let events = Events::default();
    let sink = InspectingEvents {
        state: &fixture.state,
        events: &events,
    };
    let first = project_actions::project_close(&sink, &fixture.state, &lifecycle, project.id.clone());
    let second = project_actions::project_close(&sink, &fixture.state, &lifecycle, project.id.clone());
    tokio::pin!(first, second);
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    assert!(fixture.state.projects.read().contains_key(&project.id));
    assert!(events.recorded.lock().is_empty());
    first.await.unwrap();
    second.await.unwrap();
    assert_eq!(*calls.lock(), ["flush", "flush", "detach"]);
    assert_eq!(events.names(), ["closed", "activated", "slots", "list"]);
    assert!(events.is_guard_held.lock().iter().all(|held| *held));
    assert!(fixture.state.projects.read().is_empty());
    assert!(fixture.state.paths.project_file(&project.id).is_file());
}

#[tokio::test]
async fn close의_저장_실패는_live_state_detach_event를_변경하지_않는다() {
    let fixture = Fixture::new();
    let project = fixture.seed("first");
    let session = fixture.state.session.read().clone();
    std::fs::remove_file(fixture.state.paths.session_file()).unwrap();
    std::fs::create_dir(fixture.state.paths.session_file()).unwrap();
    let calls = Mutex::new(Vec::new());
    let lifecycle = Lifecycle::new(&fixture.state, &calls);
    assert!(
        project_actions::project_close(&NoEvents, &fixture.state, &lifecycle, project.id.clone())
            .await
            .is_err()
    );
    assert_eq!(*fixture.state.session.read(), session);
    assert!(fixture.state.projects.read().contains_key(&project.id));
    assert_eq!(*calls.lock(), ["flush"]);
}

#[tokio::test]
async fn slot의_선택자_target_duplicate_검증은_새_project_열기_전에_거절한다() {
    let fixture = Fixture::new();
    let project = fixture.seed("first");
    let target = fixture.state.session.read().focused_shell_slot.clone().unwrap();
    let original = std::fs::read(fixture.state.paths.session_file()).unwrap();
    let requests = [
        OpenProjectInSlotRequest {
            path: Some(fixture.base.join("missing").to_string_lossy().into_owned()),
            project_id: Some(project.id.clone()),
            target_slot: target.clone(),
            edge: ShellSlotEdge::Right,
        },
        OpenProjectInSlotRequest {
            path: Some(fixture.base.join("missing").to_string_lossy().into_owned()),
            project_id: None,
            target_slot: ShellSlotId::new(),
            edge: ShellSlotEdge::Right,
        },
        OpenProjectInSlotRequest {
            path: None,
            project_id: Some(project.id),
            target_slot: target,
            edge: ShellSlotEdge::Right,
        },
    ];
    for request in requests {
        assert!(
            project_actions::project_open_in_slot(&NoEvents, &fixture.state, &NoLifecycle, request)
                .await
                .is_err()
        );
        assert_eq!(fixture.state.projects.read().len(), 1);
        assert_eq!(std::fs::read(fixture.state.paths.session_file()).unwrap(), original);
    }
}

#[tokio::test]
async fn slot의_기존_replace는_attach_없이_활성화하고_새_split은_attach_뒤_event를_발행한다() {
    let fixture = Fixture::new();
    let first = fixture.seed("first");
    let target = fixture.state.session.read().focused_shell_slot.clone().unwrap();
    let events = Events::default();
    let sink = InspectingEvents {
        state: &fixture.state,
        events: &events,
    };
    project_actions::project_open_in_slot(
        &sink,
        &fixture.state,
        &NoLifecycle,
        OpenProjectInSlotRequest {
            path: None,
            project_id: Some(first.id),
            target_slot: target.clone(),
            edge: ShellSlotEdge::Replace,
        },
    )
    .await
    .unwrap();
    assert_eq!(events.names(), ["activated", "slots"]);
    events.recorded.lock().clear();
    let calls = Mutex::new(Vec::new());
    let lifecycle = Lifecycle::new(&fixture.state, &calls);
    let result = project_actions::project_open_in_slot(
        &sink,
        &fixture.state,
        &lifecycle,
        OpenProjectInSlotRequest {
            path: Some(fixture.root("second")),
            project_id: None,
            target_slot: target,
            edge: ShellSlotEdge::Right,
        },
    )
    .await
    .unwrap();
    assert_eq!(shell_slots::leaves(result.tree.as_ref().unwrap()).len(), 2);
    assert_eq!(events.names(), ["opened", "list", "activated", "slots"]);
    assert_eq!(*calls.lock(), ["detect", "attach:second"]);
    assert!(events.is_guard_held.lock().iter().all(|held| !held));
}

#[tokio::test]
async fn slot_attach_실패의_rollback은_새_slot만_지우고_기존_project와_focus를_보존한다() {
    let fixture = Fixture::new();
    let first = fixture.seed("first");
    let target = fixture.state.session.read().focused_shell_slot.clone().unwrap();
    let calls = Mutex::new(Vec::new());
    let mut lifecycle = Lifecycle::new(&fixture.state, &calls);
    lifecycle.failing_name = Some("failed");
    let events = Events::default();
    let sink = InspectingEvents {
        state: &fixture.state,
        events: &events,
    };
    let result = project_actions::project_open_in_slot(
        &sink,
        &fixture.state,
        &lifecycle,
        OpenProjectInSlotRequest {
            path: Some(fixture.root("failed")),
            project_id: None,
            target_slot: target,
            edge: ShellSlotEdge::Right,
        },
    )
    .await;
    assert!(matches!(result, Err(AppError::Internal(message)) if message == "fixture attach failure"));
    assert_eq!(fixture.state.projects.read().len(), 1);
    assert_eq!(fixture.state.session.read().active_project.as_ref(), Some(&first.id));
    assert_eq!(
        shell_slots::leaves(fixture.state.session.read().shell_slots.as_ref().unwrap()).len(),
        1
    );
    assert_eq!(events.names(), ["closed", "activated", "slots", "list"]);
    assert_eq!(*calls.lock(), ["detect", "attach:failed", "flush", "detach"]);
}

#[tokio::test]
async fn close_후_history_재열기는_같은_id와_display를_보존하고_capability를_다시_부착한다() {
    let fixture = Fixture::new();
    let mut history = fixture.history("history");
    history.display.label = Some("Fixture".to_string());
    service::save_project(&fixture.state.paths, &history).unwrap();
    let calls = Mutex::new(Vec::new());
    let lifecycle = Lifecycle::new(&fixture.state, &calls);
    let events = Events::default();
    let sink = InspectingEvents {
        state: &fixture.state,
        events: &events,
    };
    let first = project_actions::project_open(&sink, &fixture.state, &lifecycle, history.root.clone())
        .await
        .unwrap();
    project_actions::project_close(&sink, &fixture.state, &lifecycle, first.project.id.clone())
        .await
        .unwrap();
    let reopened = project_actions::project_open(&sink, &fixture.state, &lifecycle, history.root)
        .await
        .unwrap();
    assert!(!reopened.already_open);
    assert_eq!(reopened.project.id, history.id);
    assert_eq!(reopened.project.display, history.display);
    assert_eq!(
        *calls.lock(),
        ["detect", "attach:history", "flush", "detach", "detect", "attach:history"]
    );
}

#[tokio::test]
async fn group은_기존_missing을_넘기고_실패한_첫_멤버의_활성화를_다음_성공에_승계한다() {
    for stop_after_success in [false, true] {
        let fixture = Fixture::new();
        let existing = fixture.seed("existing");
        let failed = fixture.history("failed");
        let first = fixture.history("success");
        let rest = fixture.history("rest");
        let missing = ProjectId::new();
        let group_id = ProjectGroupId::new();
        fixture.state.session.write().groups.push(ProjectGroup {
            id: group_id.clone(),
            name: "fixture".to_string(),
            color: None,
            collapsed: false,
            members: vec![existing.id, missing.clone(), failed.id.clone(), first.id.clone(), rest.id.clone()],
        });
        service::save_session(&fixture.state.paths, &fixture.state.session.read()).unwrap();
        let calls = Mutex::new(Vec::new());
        let mut lifecycle = Lifecycle::new(&fixture.state, &calls);
        lifecycle.failing_name = Some("failed");
        lifecycle.on_attach = Box::new(|project| {
            if stop_after_success && project.name == "success" {
                fixture.state.begin_shutdown();
            }
        });
        let events = Events::default();
        let sink = InspectingEvents {
            state: &fixture.state,
            events: &events,
        };
        let result = project_actions::project_group_open(&sink, &fixture.state, &lifecycle, group_id)
            .await
            .unwrap();
        assert_eq!(result.opened[0], first.id);
        assert_eq!(fixture.state.session.read().active_project.as_ref(), Some(&first.id));
        assert!(result.skipped.contains(&missing));
        assert!(result.skipped.contains(&failed.id));
        let successful_activations = events
            .recorded
            .lock()
            .iter()
            .filter(|event| matches!(event, AppEvent::ProjectActivated { project_id: Some(id) } if id == &first.id))
            .count();
        assert_eq!(successful_activations, 1);
        if stop_after_success {
            assert_eq!(result.opened.len(), 1);
            assert!(result.skipped.contains(&rest.id));
            assert!(!fixture.state.projects.read().contains_key(&rest.id));
        } else {
            assert_eq!(result.opened, [first.id, rest.id]);
        }
    }
}

#[test]
fn native_네_command는_같은_capability_flush_detach_adapter와_event_sink를_주입한다() {
    let source = include_str!("../src/domain/project/commands.rs");
    for name in ["project_open", "project_open_in_slot", "project_close", "project_group_open"] {
        let body = source
            .split_once(&format!("pub async fn {name}("))
            .unwrap()
            .1
            .split_once("\n}\n")
            .unwrap()
            .0;
        assert!(body.contains(&format!("project_actions::{name}(")));
        assert!(body.contains("&NativeProjectLifecycle { app: &app, state: &state }"));
        assert!(body.contains("&TauriEventSink(&app)"));
    }
    assert!(source.contains("self.app.state::<ProjectCapabilities>().detected_kinds(root)"));
    assert!(source.contains("attach_project_capabilities(self.app, project).await"));
    assert!(source.contains("await_project_flush(self.app, self.state, project_id).await"));
    assert!(source.contains("self.app.state::<ProjectCapabilities>().detach_all(self.app, self.state, project_id)"));
}
