use std::sync::{Arc, Mutex};
use std::time::Duration;

use taide_agent::constants::{AGENT_NAME_CLAUDE, AGENT_NAME_GEMINI};
use taide_agent::store::{AgentHooksStore, AgentStore};
use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::agent::{AgentActivity, BlockedReason, DetectedAgent, HookPayload};
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::agent_actions::apply_hook_payload;
use taide_runtime::{AppState, EventSink, ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;
use uuid::Uuid;

const FIXTURE_PID: u32 = 42;
const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const PENDING_CALLBACK_MS: u64 = 20;

#[derive(Clone)]
struct Sink {
    state: AppState,
    agents: AgentStore,
    hooks: AgentHooksStore,
    tasks: TaskSupervisor,
    events: Arc<Mutex<Vec<AppEvent>>>,
}

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        let AppEvent::AgentStateChanged { project_id, agents } = &event else {
            panic!("hook payload은 agent state만 발행한다");
        };
        assert_eq!(self.tasks.tracked_count(), 1);
        assert!(self.state.projects.try_write().is_some());
        assert_eq!(self.agents.agents_for(project_id), *agents);
        assert_eq!(
            self.hooks.fresh_project_override(project_id, AGENT_NAME_GEMINI),
            Some(AgentActivity::Working)
        );
        self.events.lock().unwrap().push(event);
    }
}

struct Fixture {
    state: AppState,
    agents: AgentStore,
    hooks: AgentHooksStore,
    tasks: TaskSupervisor,
    sink: Sink,
    project_id: ProjectId,
    root: String,
}

impl Fixture {
    fn new() -> Self {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-hook-payload-{}", Uuid::new_v4())),
        ));
        let root = state.paths.data_dir.join("uncreated-project").to_string_lossy().into_owned();
        let project_id = open_project(&state, root.clone());
        let agents = AgentStore::new();
        let hooks = AgentHooksStore::new();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let sink = Sink {
            state: state.clone(),
            agents: agents.clone(),
            hooks: hooks.clone(),
            tasks: tasks.clone(),
            events: Arc::default(),
        };
        Self {
            state,
            agents,
            hooks,
            tasks,
            sink,
            project_id,
            root,
        }
    }

    fn apply(&self, agent: &str, event: &str, cwd: &str) {
        apply_hook_payload(
            &self.state,
            &self.agents,
            &self.hooks,
            &self.sink,
            &self.tasks,
            agent,
            &payload(event, cwd),
        );
        assert_eq!(self.tasks.tracked_count(), 0);
        assert!(!self.state.paths.data_dir.exists());
    }
}

fn open_project(state: &AppState, root: String) -> ProjectId {
    let project = Project {
        id: ProjectId::new(),
        root,
        name: "fixture".to_string(),
        capabilities: Vec::new(),
        root_missing: false,
        last_opened_at: 0.0,
        display: ProjectDisplay::default(),
    };
    let id = project.id.clone();
    state.projects.write().insert(id.clone(), project);
    id
}

fn payload(event: &str, cwd: &str) -> HookPayload {
    HookPayload {
        hook_event_name: event.to_string(),
        cwd: cwd.to_string(),
    }
}

fn detected(name: &str, session: &str) -> DetectedAgent {
    DetectedAgent {
        session_id: session.to_string(),
        name: name.to_string(),
        pid: FIXTURE_PID,
        activity: AgentActivity::AwaitingInput,
        blocked_reason: Some(BlockedReason::Dialog),
    }
}

#[tokio::test]
async fn unknown_event_agent_빈_cwd와_형제_prefix는_override_diff_event를_건드리지_않는다() {
    let fixture = Fixture::new();
    let original = vec![detected(AGENT_NAME_GEMINI, "gemini")];
    fixture.agents.diff(&fixture.project_id, &original);
    for (agent, event, cwd) in [
        ("unknown", "BeforeAgent", fixture.root.clone()),
        (AGENT_NAME_GEMINI, "unknown", fixture.root.clone()),
        (AGENT_NAME_GEMINI, "BeforeAgent", String::new()),
        (AGENT_NAME_GEMINI, "BeforeAgent", format!("{}-sibling", fixture.root)),
    ] {
        fixture.apply(agent, event, &cwd);
    }
    assert_eq!(fixture.agents.agents_for(&fixture.project_id), original);
    assert_eq!(fixture.hooks.fresh_project_override(&fixture.project_id, AGENT_NAME_GEMINI), None);
    assert!(fixture.sink.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn 닫힌_감독자는_기존_override와_diff를_보존한다() {
    let fixture = Fixture::new();
    let original = vec![detected(AGENT_NAME_GEMINI, "gemini")];
    fixture.agents.diff(&fixture.project_id, &original);
    fixture
        .hooks
        .set_project_override(fixture.project_id.clone(), AGENT_NAME_GEMINI.to_string(), AgentActivity::Idle);
    fixture.tasks.stop_all();
    fixture.apply(AGENT_NAME_GEMINI, "BeforeAgent", &fixture.root);
    assert_eq!(fixture.agents.agents_for(&fixture.project_id), original);
    assert_eq!(
        fixture.hooks.fresh_project_override(&fixture.project_id, AGENT_NAME_GEMINI),
        Some(AgentActivity::Idle)
    );
    assert!(fixture.sink.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn 빈_agent_snapshot도_override를_먼저_저장하지만_event는_발행하지_않는다() {
    let fixture = Fixture::new();
    fixture.apply(AGENT_NAME_GEMINI, "BeforeAgent", &fixture.root);
    assert_eq!(
        fixture.hooks.fresh_project_override(&fixture.project_id, AGENT_NAME_GEMINI),
        Some(AgentActivity::Working)
    );
    assert!(fixture.agents.agents_for(&fixture.project_id).is_empty());
    assert!(fixture.sink.events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn 가장_긴_root의_같은_agent_세션만_변경하고_reason은_지우며_동일_payload는_조용하다() {
    let fixture = Fixture::new();
    let child_root = format!("{}/nested", fixture.root);
    let child = open_project(&fixture.state, child_root.clone());
    let parent_rows = vec![detected(AGENT_NAME_GEMINI, "parent")];
    fixture.agents.diff(&fixture.project_id, &parent_rows);
    let other = detected(AGENT_NAME_CLAUDE, "claude");
    fixture.agents.diff(
        &child,
        &[
            detected(AGENT_NAME_GEMINI, "first"),
            other.clone(),
            detected(AGENT_NAME_GEMINI, "second"),
        ],
    );
    fixture.state.settings.write().agent_hooks_enabled = false;
    let cwd = format!("{child_root}/src/");
    fixture.apply(AGENT_NAME_GEMINI, "BeforeAgent", &cwd);
    fixture.apply(AGENT_NAME_GEMINI, "BeforeAgent", &cwd);
    let rows = fixture.agents.agents_for(&child);
    for row in rows.iter().filter(|row| row.name == AGENT_NAME_GEMINI) {
        assert_eq!(row.activity, AgentActivity::Working);
        assert_eq!(row.blocked_reason, None);
    }
    assert_eq!(rows[1], other);
    assert_eq!(fixture.agents.agents_for(&fixture.project_id), parent_rows);
    assert_eq!(fixture.hooks.fresh_project_override(&fixture.project_id, AGENT_NAME_GEMINI), None);
    let events = fixture.sink.events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], AppEvent::AgentStateChanged { project_id, agents } if project_id == &child && agents == &rows));
}

#[tokio::test]
async fn 다른_agent만_캐시되어_있으면_override만_갱신하고_diff_event는_없다() {
    let fixture = Fixture::new();
    let original = vec![detected(AGENT_NAME_CLAUDE, "claude")];
    fixture.agents.diff(&fixture.project_id, &original);
    fixture.apply(AGENT_NAME_GEMINI, "BeforeAgent", &fixture.root);
    assert_eq!(
        fixture.hooks.fresh_project_override(&fixture.project_id, AGENT_NAME_GEMINI),
        Some(AgentActivity::Working)
    );
    assert_eq!(fixture.agents.agents_for(&fixture.project_id), original);
    assert!(fixture.sink.events.lock().unwrap().is_empty());
}

struct Release(Option<std::sync::mpsc::Sender<()>>);

impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            sender.send(()).ok();
        }
    }
}

struct GateSink {
    sink: Sink,
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

impl EventSink for GateSink {
    fn publish(&self, event: AppEvent) {
        self.sink.publish(event);
        self.entered.lock().unwrap().take().unwrap().send(()).ok();
        self.release.lock().unwrap().recv().unwrap();
    }
}

#[tokio::test]
async fn 정상_root는_payload의_동기_event_callback_완료까지_기다린다() {
    let fixture = Fixture::new();
    fixture.agents.diff(&fixture.project_id, &[detected(AGENT_NAME_GEMINI, "gemini")]);
    let runtime = tokio::runtime::Handle::current();
    let state = fixture.state.clone();
    let agents = fixture.agents.clone();
    let hooks = fixture.hooks.clone();
    let tasks = fixture.tasks.clone();
    let payload = payload("BeforeAgent", &fixture.root);
    let (entered, entered_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let sink = GateSink {
        sink: fixture.sink.clone(),
        entered: Mutex::new(Some(entered)),
        release: Mutex::new(release_rx),
    };
    let request =
        tokio::task::spawn_blocking(move || apply_hook_payload(&state, &agents, &hooks, &sink, &tasks, AGENT_NAME_GEMINI, &payload));
    let deadline = Duration::from_millis(FIXTURE_TIMEOUT_MS);
    tokio::time::timeout(deadline, entered_rx).await.unwrap().unwrap();
    assert_eq!(fixture.tasks.tracked_count(), 1);
    let mut drain = ExitDrain::default();
    let (ready, mut ready_rx) = oneshot::channel();
    assert!(drain.begin(
        &runtime,
        fixture.tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        TerminalStore::new(),
        move || {
            ready.send(()).ok();
        }
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_CALLBACK_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    assert_eq!(fixture.sink.events.lock().unwrap().len(), 1);
    drop(release);
    tokio::time::timeout(deadline, request).await.unwrap().unwrap();
    tokio::time::timeout(deadline, ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert_eq!(fixture.tasks.tracked_count(), 0);
    assert!(!fixture.state.paths.data_dir.exists());
}

#[test]
fn native는_같은_state_stores_event_sink_supervisor를_주입하고_인증_뒤에만_payload를_적용한다() {
    let native = include_str!("../../../src-tauri/src/domain/agent/hooks.rs");
    let wrapper = native.split("fn apply_hook_payload(").nth(1).unwrap();
    for port in [
        "app.state::<AppState>()",
        "app.state::<AgentStore>()",
        "app.state::<AgentHooksStore>()",
        "app.state::<TaskSupervisor>()",
        "&TauriEventSink(app)",
        "agent_actions::apply_hook_payload(",
    ] {
        assert!(wrapper.contains(port));
    }
    let connection = native
        .split("async fn handle_connection(")
        .nth(1)
        .unwrap()
        .split("fn apply_hook_payload(")
        .next()
        .unwrap();
    assert!(connection.find("constant_time_eq").unwrap() < connection.find("serde_json::from_slice").unwrap());
    assert!(connection.find("serde_json::from_slice").unwrap() < connection.find("apply_hook_payload(&app").unwrap());
    assert!(connection.contains("HTTP/1.1 403 Forbidden"));
    assert!(connection.contains("HTTP/1.1 400 Bad Request"));
    assert!(connection.contains("HTTP/1.1 200 OK"));
}
