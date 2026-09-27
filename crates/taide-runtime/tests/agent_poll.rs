#[cfg(unix)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use taide_agent::constants::AGENT_NAME_CLAUDE;
use taide_agent::service::DetectedAgentProbe;
use taide_agent::store::{AgentHooksStore, AgentStore};
use taide_infra::terminal_scan::ScanOutcome;
use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::agent::{AgentActivity, BlockedReason};
use taide_model::app_event::AppEvent;
use taide_model::error::AppError;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::agent_actions::{build_detected_agents, poll_agents};
use taide_runtime::agent_probe::probe_process_tree;
use taide_runtime::{AppState, EventSink, ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;
use uuid::Uuid;

const FIXTURE_PID: u32 = 41;
const SECOND_PID: u32 = 42;
#[cfg(unix)]
const STALE_PID: u32 = 43;
const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const PENDING_PROBE_MS: u64 = 20;
const SESSION: &str = "fixture-session";
const SECOND_SESSION: &str = "second-session";
const STALE_SESSION: &str = "stale-session";

#[derive(Clone)]
struct Sink {
    agents: AgentStore,
    events: Arc<Mutex<Vec<AppEvent>>>,
}

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        let AppEvent::AgentStateChanged { project_id, agents } = &event else {
            panic!("poll은 agent state만 발행한다");
        };
        assert_eq!(
            serde_json::to_value(self.agents.agents_for(project_id)).unwrap(),
            serde_json::to_value(agents).unwrap()
        );
        self.events.lock().unwrap().push(event);
    }
}

struct Fixture {
    state: AppState,
    agents: AgentStore,
    hooks: AgentHooksStore,
    sink: Sink,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-agent-poll-{}", Uuid::new_v4())),
        ));
        let project_id = open_project(&state);
        let agents = AgentStore::new();
        Self {
            state,
            hooks: AgentHooksStore::new(),
            sink: Sink {
                agents: agents.clone(),
                events: Arc::default(),
            },
            agents,
            project_id,
        }
    }
}

fn open_project(state: &AppState) -> ProjectId {
    let project = Project {
        id: ProjectId::new(),
        root: state.paths.data_dir.join("uncreated-root").to_string_lossy().into_owned(),
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

fn probes(pids: Vec<(String, u32)>) -> Vec<DetectedAgentProbe> {
    pids.into_iter()
        .map(|(session_id, pid)| DetectedAgentProbe {
            session_id,
            pid,
            name: AGENT_NAME_CLAUDE,
        })
        .collect()
}

fn dialog(agents: &AgentStore, session: &str) {
    agents.classify_session_state(session, AGENT_NAME_CLAUDE);
    agents.record_scan(
        session,
        &ScanOutcome {
            text: "Do you want to proceed?".to_string(),
            ..ScanOutcome::default()
        },
    );
}

fn deadline() -> Duration {
    Duration::from_millis(FIXTURE_TIMEOUT_MS)
}

#[tokio::test]
async fn 닫힌_입장은_port_event_signal_cache를_변경하지_않는다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    dialog(&fixture.agents, STALE_SESSION);
    #[cfg(unix)]
    fixture.agents.remember_process_names(HashMap::from([(STALE_PID, None)]));
    tasks.stop_all();
    poll_agents(
        &fixture.state,
        &fixture.agents,
        &fixture.hooks,
        &fixture.sink,
        &tasks,
        |_| panic!("closed foreground"),
        |_| async { panic!("closed probe") },
    )
    .await;
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    assert_eq!(
        fixture
            .agents
            .classify_session_state(STALE_SESSION, AGENT_NAME_CLAUDE)
            .blocked_reason,
        Some(BlockedReason::Dialog)
    );
    #[cfg(unix)]
    assert!(fixture.agents.unresolved_pids(&[(STALE_SESSION.to_string(), STALE_PID)]).is_empty());
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!fixture.state.paths.data_dir.exists());
}

#[tokio::test]
async fn diff_저장_뒤에만_event를_발행하고_동일_tick은_조용하며_빈_성공은_제거를_발행한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    dialog(&fixture.agents, SESSION);
    dialog(&fixture.agents, STALE_SESSION);
    for _ in 0..2 {
        poll_agents(
            &fixture.state,
            &fixture.agents,
            &fixture.hooks,
            &fixture.sink,
            &tasks,
            |_| vec![(SESSION.to_string(), FIXTURE_PID)],
            |pids| async move { Ok(probes(pids)) },
        )
        .await;
    }
    assert_eq!(fixture.sink.events.lock().unwrap().len(), 1);
    let current = fixture.agents.agents_for(&fixture.project_id);
    assert_eq!(current[0].activity, AgentActivity::AwaitingInput);
    assert_eq!(current[0].blocked_reason, Some(BlockedReason::Dialog));
    assert_eq!(
        fixture.agents.classify_session_state(STALE_SESSION, AGENT_NAME_CLAUDE).activity,
        AgentActivity::Unknown
    );
    poll_agents(
        &fixture.state,
        &fixture.agents,
        &fixture.hooks,
        &fixture.sink,
        &tasks,
        |_| Vec::new(),
        |pids| async move { Ok(probes(pids)) },
    )
    .await;
    let events = fixture.sink.events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert!(
        matches!(&events[1], AppEvent::AgentStateChanged { project_id, agents } if project_id == &fixture.project_id && agents.is_empty())
    );
    assert!(fixture.agents.agents_for(&fixture.project_id).is_empty());
    assert_eq!(
        fixture.agents.classify_session_state(SESSION, AGENT_NAME_CLAUDE).activity,
        AgentActivity::Unknown
    );
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn probe_실패는_diff를_유지하고_다른_프로젝트와_원래_prune_정책은_계속된다() {
    let fixture = Fixture::new();
    let second = open_project(&fixture.state);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    dialog(&fixture.agents, SESSION);
    dialog(&fixture.agents, SECOND_SESSION);
    let previous = build_detected_agents(
        &fixture.agents,
        &fixture.hooks,
        &fixture.project_id,
        probes(vec![(SESSION.to_string(), FIXTURE_PID)]),
    );
    fixture.agents.diff(&fixture.project_id, &previous);
    #[cfg(unix)]
    fixture.agents.remember_process_names(HashMap::from([
        (FIXTURE_PID, None),
        (SECOND_PID, Some(AGENT_NAME_CLAUDE)),
        (STALE_PID, None),
    ]));
    let calls = AtomicUsize::new(0);
    poll_agents(
        &fixture.state,
        &fixture.agents,
        &fixture.hooks,
        &fixture.sink,
        &tasks,
        |id| {
            assert!(fixture.state.projects.try_write().is_some());
            if id == &fixture.project_id {
                return vec![(SESSION.to_string(), FIXTURE_PID)];
            }
            assert_eq!(id, &second);
            vec![(SECOND_SESSION.to_string(), SECOND_PID)]
        },
        |pids| {
            calls.fetch_add(1, Ordering::SeqCst);
            async move {
                if pids[0].1 == FIXTURE_PID {
                    return Err(AppError::Internal("fixture probe error".to_string()));
                }
                Ok(probes(pids))
            }
        },
    )
    .await;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.sink.events.lock().unwrap().len(), 1);
    assert_eq!(
        serde_json::to_value(fixture.agents.agents_for(&fixture.project_id)).unwrap(),
        serde_json::to_value(previous).unwrap()
    );
    assert_eq!(
        fixture.agents.classify_session_state(SESSION, AGENT_NAME_CLAUDE).activity,
        AgentActivity::Unknown
    );
    assert_eq!(
        fixture
            .agents
            .classify_session_state(SECOND_SESSION, AGENT_NAME_CLAUDE)
            .blocked_reason,
        Some(BlockedReason::Dialog)
    );
    #[cfg(unix)]
    {
        assert!(fixture
            .agents
            .unresolved_pids(&[(SESSION.to_string(), FIXTURE_PID), (SECOND_SESSION.to_string(), SECOND_PID)])
            .is_empty());
        assert_eq!(
            fixture.agents.unresolved_pids(&[(STALE_SESSION.to_string(), STALE_PID)]),
            vec![STALE_PID]
        );
    }
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 프로젝트_snapshot은_조회_중_추가를_제외하고_제거된_id도_원래대로_처리한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let called = AtomicBool::new(false);
    poll_agents(
        &fixture.state,
        &fixture.agents,
        &fixture.hooks,
        &fixture.sink,
        &tasks,
        |id| {
            assert_eq!(id, &fixture.project_id);
            assert!(!called.swap(true, Ordering::SeqCst));
            fixture.state.projects.write().remove(id);
            open_project(&fixture.state);
            vec![(SESSION.to_string(), FIXTURE_PID)]
        },
        |pids| async move { Ok(probes(pids)) },
    )
    .await;
    assert_eq!(fixture.sink.events.lock().unwrap().len(), 1);
    assert!(!fixture.state.projects.read().contains_key(&fixture.project_id));
    assert_eq!(fixture.agents.agents_for(&fixture.project_id).len(), 1);
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 열린_프로젝트가_없으면_port와_diff는_건드리지_않고_신호와_이름을_정리한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    fixture.state.projects.write().clear();
    dialog(&fixture.agents, STALE_SESSION);
    #[cfg(unix)]
    fixture.agents.remember_process_names(HashMap::from([(STALE_PID, None)]));
    poll_agents(
        &fixture.state,
        &fixture.agents,
        &fixture.hooks,
        &fixture.sink,
        &tasks,
        |_| panic!("empty projects"),
        |_| async { panic!("empty probes") },
    )
    .await;
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    assert_eq!(
        fixture.agents.classify_session_state(STALE_SESSION, AGENT_NAME_CLAUDE).activity,
        AgentActivity::Unknown
    );
    #[cfg(unix)]
    assert_eq!(
        fixture.agents.unresolved_pids(&[(STALE_SESSION.to_string(), STALE_PID)]),
        vec![STALE_PID]
    );
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn probe_대기_중_정상_root는_poll_owner를_기다리고_취소는_event와_prune을_생략한다() {
    let fixture = Fixture::new();
    dialog(&fixture.agents, STALE_SESSION);
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let state = fixture.state.clone();
    let agents = fixture.agents.clone();
    let hooks = fixture.hooks.clone();
    let sink = fixture.sink.clone();
    let request_tasks = tasks.clone();
    let (started, started_rx) = oneshot::channel();
    let started = Mutex::new(Some(started));
    let request = tokio::spawn(async move {
        poll_agents(
            &state,
            &agents,
            &hooks,
            &sink,
            &request_tasks,
            |_| vec![(SESSION.to_string(), FIXTURE_PID)],
            |_| async {
                started.lock().unwrap().take().unwrap().send(()).ok();
                std::future::pending().await
            },
        )
        .await;
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
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
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    assert_eq!(
        fixture
            .agents
            .classify_session_state(STALE_SESSION, AGENT_NAME_CLAUDE)
            .blocked_reason,
        Some(BlockedReason::Dialog)
    );
    assert_eq!(tasks.tracked_count(), 0);
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
async fn probe_완료_뒤에도_root는_event_callback과_마지막_prune까지_기다린다() {
    let fixture = Fixture::new();
    dialog(&fixture.agents, STALE_SESSION);
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let state = fixture.state.clone();
    let agents = fixture.agents.clone();
    let hooks = fixture.hooks.clone();
    let request_tasks = tasks.clone();
    let request_runtime = runtime.clone();
    let (entered, entered_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let sink = GateSink {
        sink: fixture.sink.clone(),
        entered: Mutex::new(Some(entered)),
        release: Mutex::new(release_rx),
    };
    let request = tokio::task::spawn_blocking(move || {
        request_runtime.block_on(poll_agents(
            &state,
            &agents,
            &hooks,
            &sink,
            &request_tasks,
            |_| vec![(SESSION.to_string(), FIXTURE_PID)],
            |pids| async move { Ok(probes(pids)) },
        ))
    });
    tokio::time::timeout(deadline(), entered_rx).await.unwrap().unwrap();
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
    assert_eq!(fixture.sink.events.lock().unwrap().len(), 1);
    assert_eq!(
        fixture
            .agents
            .classify_session_state(STALE_SESSION, AGENT_NAME_CLAUDE)
            .blocked_reason,
        Some(BlockedReason::Dialog)
    );
    drop(release);
    tokio::time::timeout(deadline(), request).await.unwrap().unwrap();
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert_eq!(
        fixture.agents.classify_session_state(STALE_SESSION, AGENT_NAME_CLAUDE).activity,
        AgentActivity::Unknown
    );
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn native는_동일_state_ports_supervisor를_위임하고_setup_tick은_유지한다() {
    let native = include_str!("../../../src-tauri/src/domain/agent/commands.rs");
    let assembly = include_str!("../../../src-tauri/src/lib.rs");
    let runtime = include_str!("../src/agent_actions.rs");
    let poll = native
        .split("pub(crate) async fn poll_agents(")
        .nth(1)
        .unwrap()
        .split("\n}")
        .next()
        .unwrap();
    assert!(poll.contains("agent_actions::poll_agents("));
    assert!(poll.contains("&TauriEventSink(app)"));
    assert!(poll.contains("|project_id| (foreground_pids.0)(app, project_id)"));
    assert!(poll.contains("|pids| detect_agents_for_pids_blocking(&tasks, &agents, pids)"));
    assert!(!poll.contains("agents.diff("));
    assert!(runtime.contains("tasks.begin_operation(\"agent-poll\")"));
    assert!(assembly.contains("spawn(\"agent-poll\", async move"));
    assert!(assembly.contains("domain::agent::commands::poll_agents(&agent_handle).await"));
    assert!(assembly.contains("domain::agent::types::AGENT_POLL_WINDOWS_MS"));
    assert!(assembly.contains("domain::agent::types::AGENT_POLL_UNIX_MS"));
}

#[tokio::test]
async fn poll_취소_뒤에도_결합한_probe의_실제_worker가_root를_소유한다() {
    let fixture = Fixture::new();
    dialog(&fixture.agents, STALE_SESSION);
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let state = fixture.state.clone();
    let agents = fixture.agents.clone();
    let hooks = fixture.hooks.clone();
    let sink = fixture.sink.clone();
    let request_tasks = tasks.clone();
    let (started, started_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let gate = Mutex::new(Some((started, release_rx)));
    let request = tokio::spawn(async move {
        poll_agents(
            &state,
            &agents,
            &hooks,
            &sink,
            &request_tasks,
            |_| vec![(SESSION.to_string(), FIXTURE_PID)],
            |pids| {
                let (started, release_rx) = gate.lock().unwrap().take().unwrap();
                probe_process_tree(&request_tasks, pids, move |pids| {
                    started.send(()).ok();
                    release_rx.recv().unwrap();
                    probes(pids)
                })
            },
        )
        .await;
    });
    tokio::time::timeout(deadline(), started_rx).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 3);
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
    assert_eq!(tasks.tracked_count(), 2);
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
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    assert_eq!(
        fixture
            .agents
            .classify_session_state(STALE_SESSION, AGENT_NAME_CLAUDE)
            .blocked_reason,
        Some(BlockedReason::Dialog)
    );
    drop(release);
    tokio::time::timeout(deadline(), ready_rx).await.unwrap().unwrap();
    assert!(drain.is_ready());
    assert!(fixture.agents.agents_for(&fixture.project_id).is_empty());
    assert_eq!(tasks.tracked_count(), 0);
}
