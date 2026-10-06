use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::ids::{ProjectGroupId, ProjectId};
use taide_model::paths::AppPaths;
use taide_model::project::{
    CapabilityKind, Project, SessionShellState, SessionState, ShellSlotTree,
};
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

use super::*;
use crate::projects::NativeProjects;
use crate::remote_gateway;
use crate::remote_ws::{ChannelFactory, ResponseBody};

const DEADLINE: Duration = Duration::from_secs(5);
const FIRST_SIZE: f32 = 25.0;
const SECOND_SIZE: f32 = 75.0;

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[derive(Default)]
struct Control {
    calls: AtomicUsize,
    trace: Mutex<Vec<(&'static str, ProjectId)>>,
    pause_attach: AtomicBool,
    pause_flush: AtomicBool,
    entered: Notify,
    release: Notify,
}

struct Lifecycle {
    native: NativeProjects,
    services: Arc<AppServices>,
    control: Arc<Control>,
}

impl ProjectLifecyclePort for Lifecycle {
    fn detected_kinds(&self, root: &Path) -> Vec<CapabilityKind> {
        self.native.detected_kinds(root)
    }

    async fn attach_project_capabilities(&self, project: &Project) -> AppResult<()> {
        self.native.attach_project_capabilities(project).await?;
        self.control
            .trace
            .lock()
            .unwrap()
            .push(("attach", project.id.clone()));
        if self.control.pause_attach.load(Ordering::Acquire) {
            self.control.entered.notify_one();
            self.control.release.notified().await;
        }
        Ok(())
    }

    async fn await_project_flush(&self, project: &ProjectId) {
        assert!(self.services.state.projects.read().contains_key(project));
        self.control
            .trace
            .lock()
            .unwrap()
            .push(("flush", project.clone()));
        if self.control.pause_flush.load(Ordering::Acquire) {
            self.control.entered.notify_one();
            self.control.release.notified().await;
        }
        self.native.await_project_flush(project).await;
    }

    fn detach_all(&self, project: &ProjectId) {
        assert!(!self.services.state.projects.read().contains_key(project));
        self.control
            .trace
            .lock()
            .unwrap()
            .push(("detach", project.clone()));
        self.native.detach_all(project);
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
    control: Arc<Control>,
    backend: Dispatch,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-projects-{}", ProjectId::new()));
        for name in ["first", "second", "third"] {
            std::fs::create_dir_all(directory.join(name)).unwrap();
        }
        let directory = directory.canonicalize().unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.settings.write().agent_hooks_enabled = false;
        state.settings.write().ide_integration_enabled = false;
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        let control = Arc::new(Control::default());
        let recorded = control.clone();
        let remaining = Dispatch {
            json: Arc::new(|_, name, args, channels| {
                Box::pin(async move {
                    assert_eq!(name, "layout_get");
                    channels("1".into())(ResponseBody::Json("null".into())).unwrap();
                    Ok(args.to_string())
                })
            }),
            raw: Arc::new(|_, name, args| {
                Box::pin(async move {
                    assert_eq!(name, "file_read_raw");
                    assert_eq!(args["owner"], "remote");
                    Ok(vec![0])
                })
            }),
        };
        let backend = remote_gateway::with_policy(extend_backend(
            move |services: Arc<AppServices>| {
                recorded.calls.fetch_add(1, Ordering::AcqRel);
                Lifecycle {
                    native: NativeProjects::new(services.clone()),
                    services,
                    control: recorded.clone(),
                }
            },
            remaining,
        ));
        Self {
            directory,
            services,
            sink,
            control,
            backend,
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("project commands have no channels")));
        let text = (self.backend.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }

    async fn open(&self, name: &str) -> Project {
        let result = self
            .ok("project_open", json!({"path":self.directory.join(name)}))
            .await;
        assert_eq!(result["alreadyOpen"], false);
        serde_json::from_value(result["project"].clone()).unwrap()
    }

    async fn shell(&self) -> SessionShellState {
        serde_json::from_value(self.ok("session_get_shell_state", Value::Null).await).unwrap()
    }

    fn disk(&self) -> SessionState {
        serde_json::from_slice(&std::fs::read(self.services.state.paths.session_file()).unwrap())
            .unwrap()
    }

    async fn finish(&self) {
        self.services.state.stop_watchers();
        tokio::time::timeout(DEADLINE, self.services.state.watcher_stops.wait_for_idle())
            .await
            .unwrap();
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.state.stop_watchers();
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn catalog_실제_arm_입력_policy_remaining과_종료_후_거절을_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMANDS.len());
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    for previous in [
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_layout::COMMANDS,
    ] {
        assert!(declared.iter().all(|name| !previous.contains(name)));
    }
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let source = include_str!("remote-projects.rs");
    let arms: BTreeSet<_> = pattern
        .captures_iter(source)
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new();
    assert_eq!(fixture.ok("project_list", Value::Null).await, json!([]));
    assert_eq!(
        fixture.ok("project_get_active", Value::Null).await,
        Value::Null
    );
    assert_eq!(
        fixture
            .call("project_open", json!({"path":0}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(fixture.control.calls.load(Ordering::Acquire), 0);
    assert!(
        fixture
            .call("project_forget_recent", Value::Null)
            .await
            .is_err()
    );
    assert!(
        fixture
            .call("synthetic_unknown", Value::Null)
            .await
            .is_err()
    );
    let channels: ChannelFactory = Arc::new(|id| {
        assert_eq!(id, "1");
        Box::new(|value| {
            assert!(matches!(value, ResponseBody::Json(text) if text == "null"));
            Ok(())
        })
    });
    let forwarded = (fixture.backend.json)(
        fixture.services.clone(),
        "layout_get".into(),
        json!({"owner":"main"}),
        channels,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&forwarded).unwrap()["owner"],
        "remote"
    );
    assert_eq!(
        (fixture.backend.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            json!({"owner":"main"})
        )
        .await
        .unwrap(),
        vec![0]
    );
    fixture.services.tasks.stop_all();
    for (name, args) in [
        (
            "project_open",
            json!({"path":fixture.directory.join("first")}),
        ),
        (
            "project_open_in_slot",
            json!({"request":{"path":fixture.directory.join("second"),"targetSlot":"synthetic-slot","edge":"right"}}),
        ),
        (
            "project_group_open",
            json!({"groupId":ProjectGroupId::new()}),
        ),
    ] {
        assert_eq!(
            fixture.call(name, args).await.unwrap_err()["code"],
            "Forbidden"
        );
    }
    assert_eq!(fixture.control.calls.load(Ordering::Acquire), 0);
    assert!(fixture.services.state.projects.read().is_empty());
    assert!(fixture.sink.0.lock().unwrap().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn 실제23명령의_프로젝트_그룹_슬롯_파일저장과_native_수명을_보존한다() {
    let _os_watch_registration = NativeProjects::exclusive_os_watch_registration().await;
    let fixture = Fixture::new();
    let first = fixture.open("first").await;
    assert_eq!(first.capabilities, vec![CapabilityKind::Terminal]);
    assert!(
        fixture
            .services
            .state
            .layouts
            .read()
            .contains_key(&first.id)
    );
    assert!(
        fixture
            .services
            .state
            .watchers
            .read()
            .contains_key(&first.id)
    );
    {
        let events = fixture.sink.0.lock().unwrap();
        assert!(
            matches!(&events[..], [AppEvent::ProjectOpened { project }, AppEvent::ProjectListChanged { .. },
            AppEvent::ProjectActivated { project_id: Some(active) }, AppEvent::SessionShellSlotsChanged { .. }]
            if project.id == first.id && active == &first.id)
        );
    }
    assert_eq!(
        fixture
            .ok("project_get", json!({"projectId":first.id}))
            .await,
        serde_json::to_value(&first).unwrap()
    );
    assert_eq!(
        fixture.ok("project_get_active", Value::Null).await,
        json!(first.id)
    );
    let duplicate = fixture.ok("project_open", json!({"path":first.root})).await;
    assert_eq!(duplicate["alreadyOpen"], true);
    assert_eq!(fixture.control.trace.lock().unwrap().len(), 1);
    fixture
        .ok(
            "project_set_display",
            json!({"projectId":first.id,"patch":{"label":"A","color":"lane1"}}),
        )
        .await;
    assert_eq!(
        fixture.disk().projects[0].display.label.as_deref(),
        Some("A")
    );
    let slot_first = fixture.shell().await.focused.unwrap();
    let split: SessionShellState = serde_json::from_value(fixture.ok("project_open_in_slot",
        json!({"request":{"path":fixture.directory.join("second"),"targetSlot":slot_first,"edge":"right"}})).await).unwrap();
    let slot_second = split.focused.unwrap();
    let second = fixture
        .services
        .state
        .session
        .read()
        .active_project
        .clone()
        .unwrap();
    fixture
        .ok(
            "session_set_shell_slot_sizes",
            json!({"path":[],"sizes":[FIRST_SIZE,SECOND_SIZE]}),
        )
        .await;
    assert!(
        matches!(fixture.disk().shell_slots, Some(ShellSlotTree::Split { sizes, .. }) if sizes == vec![FIRST_SIZE,SECOND_SIZE])
    );
    fixture
        .ok("session_focus_shell_slot", json!({"slotId":slot_first}))
        .await;
    assert_eq!(
        fixture.ok("project_get_active", Value::Null).await,
        json!(first.id)
    );
    assert_eq!(
        fixture
            .ok(
                "session_set_window_chrome",
                json!({"patch":{"zen":true,"sidebarRailCollapsed":true}})
            )
            .await,
        json!({"zen":true,"sidebarRailCollapsed":true})
    );
    assert!(fixture.disk().window_chrome.zen);
    fixture
        .ok("project_reorder", json!({"ids":[second,first.id]}))
        .await;
    assert_eq!(
        fixture.ok("project_list", Value::Null).await[0]["id"],
        json!(second)
    );
    fixture
        .ok("shell_slot_close", json!({"slotId":slot_second}))
        .await;
    assert!(fixture.services.state.projects.read().contains_key(&second));
    assert!(fixture.services.state.watchers.read().contains_key(&second));
    assert!(
        fixture
            .call("shell_slot_close", json!({"slotId":slot_first}))
            .await
            .is_err()
    );
    fixture
        .ok("project_activate", json!({"projectId":second}))
        .await;
    assert_eq!(
        fixture.ok("project_get_active", Value::Null).await,
        json!(second)
    );
    let group = fixture
        .ok(
            "project_group_create",
            json!({"name":"synthetic","color":"lane1","members":[first.id,second]}),
        )
        .await;
    let group_id = group["id"].clone();
    let other = fixture
        .ok(
            "project_group_create",
            json!({"name":"other","color":null,"members":null}),
        )
        .await;
    fixture
        .ok(
            "project_group_rename",
            json!({"groupId":group_id,"name":"renamed"}),
        )
        .await;
    fixture
        .ok(
            "project_group_set_color",
            json!({"groupId":group_id,"color":"lane2"}),
        )
        .await;
    fixture
        .ok(
            "project_group_set_collapsed",
            json!({"groupId":group_id,"collapsed":true}),
        )
        .await;
    fixture
        .ok(
            "project_group_set_members",
            json!({"groupId":group_id,"members":[second]}),
        )
        .await;
    fixture
        .ok(
            "project_group_reorder",
            json!({"ids":[other["id"],group_id]}),
        )
        .await;
    let groups = fixture.ok("project_group_list", Value::Null).await;
    assert_eq!(groups[1]["name"], "renamed");
    assert_eq!(groups[1]["color"], "lane2");
    assert_eq!(groups[1]["collapsed"], true);
    assert_eq!(groups[1]["members"], json!([second]));
    assert_eq!(serde_json::to_value(fixture.disk().groups).unwrap(), groups);
    let before_close = fixture.sink.0.lock().unwrap().len();
    fixture
        .ok("project_close", json!({"projectId":second}))
        .await;
    assert!(!fixture.services.state.projects.read().contains_key(&second));
    assert!(!fixture.services.state.layouts.read().contains_key(&second));
    assert!(!fixture.services.state.watchers.read().contains_key(&second));
    assert_eq!(fixture.disk().groups[1].members, vec![second.clone()]);
    {
        let events = fixture.sink.0.lock().unwrap();
        assert!(
            matches!(&events[before_close..], [AppEvent::ProjectClosed { project_id }, AppEvent::ProjectActivated { .. },
            AppEvent::SessionShellSlotsChanged { .. }, AppEvent::ProjectListChanged { .. }] if project_id == &second)
        );
        let trace = fixture.control.trace.lock().unwrap();
        assert_eq!(
            &trace[trace.len() - 2..],
            &[("flush", second.clone()), ("detach", second.clone())]
        );
    }
    let reopened = fixture
        .ok("project_group_open", json!({"groupId":group_id}))
        .await;
    assert_eq!(reopened, json!({"opened":[second],"skipped":[]}));
    assert!(fixture.services.state.watchers.read().contains_key(&second));
    let skipped = fixture
        .ok("project_group_open", json!({"groupId":group_id}))
        .await;
    assert_eq!(skipped, json!({"opened":[],"skipped":[second]}));
    fixture
        .ok("project_group_delete", json!({"groupId":other["id"]}))
        .await;
    assert_eq!(fixture.disk().groups.len(), 1);
    let record: Project = serde_json::from_slice(
        &std::fs::read(fixture.services.state.paths.project_file(&first.id)).unwrap(),
    )
    .unwrap();
    assert_eq!(record.display.label.as_deref(), Some("A"));
    fixture.finish().await;
}

#[tokio::test]
async fn 열기3진입점의_취소와_감독종료는_승인된_작업을_남기고_닫기_flush는_취소된다() {
    let _os_watch_registration = NativeProjects::exclusive_os_watch_registration().await;
    for name in ["project_open", "project_open_in_slot", "project_group_open"] {
        let fixture = Fixture::new();
        let args = match name {
            "project_open_in_slot" => {
                fixture.open("first").await;
                let slot = fixture.shell().await.focused.unwrap();
                json!({"request":{"path":fixture.directory.join("second"),"targetSlot":slot,"edge":"right"}})
            }
            "project_group_open" => {
                let project = fixture.open("second").await;
                let group = fixture
                    .ok(
                        "project_group_create",
                        json!({"name":"queued","members":[project.id]}),
                    )
                    .await;
                fixture
                    .ok("project_close", json!({"projectId":project.id}))
                    .await;
                json!({"groupId":group["id"]})
            }
            _ => json!({"path":fixture.directory.join("second")}),
        };
        fixture.control.pause_attach.store(true, Ordering::Release);
        let services = fixture.services.clone();
        let backend = fixture.backend.json.clone();
        let caller = tokio::spawn(async move {
            let channels: ChannelFactory = Arc::new(|_| Box::new(|_| panic!("no channels")));
            backend(services, name.into(), args, channels).await
        });
        tokio::time::timeout(DEADLINE, fixture.control.entered.notified())
            .await
            .unwrap();
        let opening = fixture
            .control
            .trace
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .1
            .clone();
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        fixture.services.tasks.stop_all();
        let tasks = fixture.services.tasks.clone();
        let drained = tokio::spawn(async move { tasks.shutdown().await });
        tokio::task::yield_now().await;
        assert!(!drained.is_finished());
        assert!(fixture.services.tasks.tracked_count() > 0);
        fixture.control.release.notify_one();
        tokio::time::timeout(DEADLINE, drained)
            .await
            .unwrap()
            .unwrap();
        assert!(
            fixture
                .services
                .state
                .projects
                .read()
                .contains_key(&opening)
        );
        assert!(
            fixture
                .services
                .state
                .watchers
                .read()
                .contains_key(&opening)
        );
        assert!(fixture.sink.0.lock().unwrap().iter().any(
            |event| matches!(event, AppEvent::ProjectOpened { project } if project.id == opening)
        ));
        fixture.finish().await;
    }

    let fixture = Fixture::new();
    let project = fixture.open("first").await;
    fixture.control.pause_flush.store(true, Ordering::Release);
    let services = fixture.services.clone();
    let backend = fixture.backend.json.clone();
    let id = project.id.clone();
    let caller = tokio::spawn(async move {
        let channels: ChannelFactory = Arc::new(|_| Box::new(|_| panic!("no channels")));
        backend(
            services,
            "project_close".into(),
            json!({"projectId":id}),
            channels,
        )
        .await
    });
    tokio::time::timeout(DEADLINE, fixture.control.entered.notified())
        .await
        .unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    assert!(
        fixture
            .services
            .state
            .projects
            .read()
            .contains_key(&project.id)
    );
    assert!(
        fixture
            .services
            .state
            .watchers
            .read()
            .contains_key(&project.id)
    );
    assert!(
        !fixture
            .control
            .trace
            .lock()
            .unwrap()
            .iter()
            .any(|(step, _)| *step == "detach")
    );
    fixture.control.pause_flush.store(false, Ordering::Release);
    fixture
        .ok("project_close", json!({"projectId":project.id}))
        .await;
    assert!(fixture.services.state.projects.read().is_empty());
    fixture.finish().await;
}
