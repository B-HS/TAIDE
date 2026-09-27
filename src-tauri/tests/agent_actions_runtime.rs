use std::cell::Cell;
use std::path::PathBuf;

use futures_util::FutureExt;
use taide_agent::constants::{AGENT_NAME_CLAUDE, AGENT_NAME_CODEX, AGENT_NAME_GEMINI, WAIT_MARKER_PREFIX};
use taide_agent::service::DetectedAgentProbe;
use taide_agent::store::{AgentHooksStore, AgentStore};
use taide_infra::terminal_scan::ScanOutcome;
use taide_model::agent::{AgentActivity, BlockedReason, ExternalOpenRequest};
use taide_model::error::{AppError, AppErrorKind};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay};
use taide_runtime::{agent_actions, AppState};
use uuid::Uuid;

const FIXTURE_PID: u32 = 42;
const SESSION_ID: &str = "fixture-session";

fn state() -> AppState {
    AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-agent-actions-{}", Uuid::new_v4())),
    ))
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

fn probe(name: &'static str) -> DetectedAgentProbe {
    DetectedAgentProbe {
        session_id: SESSION_ID.to_string(),
        name,
        pid: FIXTURE_PID,
    }
}

struct Marker(PathBuf);

impl Marker {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("{WAIT_MARKER_PREFIX}{}", Uuid::new_v4())))
    }

    fn name(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for Marker {
    fn drop(&mut self) {
        if self.0.is_dir() {
            std::fs::remove_dir(&self.0).expect("자기 UUID 빈 marker 디렉터리 정리");
            return;
        }
        if self.0.exists() {
            std::fs::remove_file(&self.0).expect("자기 UUID marker 파일 정리");
        }
    }
}

#[tokio::test]
async fn 닫힌_프로젝트는_pid와_probe_port를_호출하지_않는다() {
    let state = state();
    let error = agent_actions::agent_list(
        &state,
        &AgentStore::new(),
        &AgentHooksStore::new(),
        || panic!("project gate 전 PID 조회 금지"),
        |_| async { panic!("project gate 전 probe 금지") },
        ProjectId::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn list는_프로젝트_검증_뒤_pid_probe_state_조립_순서를_유지한다() {
    let state = state();
    let id = open_project(&state);
    let agents = AgentStore::new();
    let hooks = AgentHooksStore::new();
    hooks.set_project_override(id.clone(), AGENT_NAME_GEMINI.to_string(), AgentActivity::Working);
    let did_read_pids = Cell::new(false);
    let result = agent_actions::agent_list(
        &state,
        &agents,
        &hooks,
        || {
            assert!(state.projects.try_write().is_some());
            assert!(state.begin_mutation().now_or_never().is_some());
            did_read_pids.set(true);
            vec![(SESSION_ID.to_string(), FIXTURE_PID)]
        },
        |pids| {
            assert!(did_read_pids.get());
            assert_eq!(pids, [(SESSION_ID.to_string(), FIXTURE_PID)]);
            async { Ok(vec![probe(AGENT_NAME_GEMINI)]) }
        },
        id.clone(),
    )
    .await
    .unwrap();
    assert_eq!(result.project_id, id);
    assert_eq!(result.agents.len(), 1);
    assert_eq!(result.agents[0].activity, AgentActivity::Working);
    assert_eq!(result.agents[0].blocked_reason, None);
    assert_eq!(result.agents[0].name, AGENT_NAME_GEMINI);
    assert_eq!(result.agents[0].pid, FIXTURE_PID);
    assert!(agents.agents_for(&id).is_empty(), "list는 poll의 diff cache를 변경하지 않는다");
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn probe_오류는_state_조립_전에_그대로_반환된다() {
    let state = state();
    let id = open_project(&state);
    let result = agent_actions::agent_list(
        &state,
        &AgentStore::new(),
        &AgentHooksStore::new(),
        Vec::new,
        |_| async { Err(AppError::Internal("fixture probe error".to_string())) },
        id,
    )
    .await;
    assert!(matches!(result, Err(AppError::Internal(message)) if message == "fixture probe error"));
}

#[test]
fn override는_http_agent의_무신호에만_적용되고_세션_신호를_덮지_않는다() {
    let agents = AgentStore::new();
    let hooks = AgentHooksStore::new();
    let id = ProjectId::new();
    for name in [AGENT_NAME_GEMINI, AGENT_NAME_CODEX, AGENT_NAME_CLAUDE] {
        hooks.set_project_override(id.clone(), name.to_string(), AgentActivity::Working);
    }
    let unknown = agent_actions::resolve_state(&agents, &hooks, &id, &probe(AGENT_NAME_CODEX));
    assert_eq!(unknown.activity, AgentActivity::Unknown);
    agents.classify_session_state(SESSION_ID, AGENT_NAME_CLAUDE);
    agents.record_scan(
        SESSION_ID,
        &ScanOutcome {
            events: Vec::new(),
            text: "Do you want to proceed?".to_string(),
            overlap: String::new(),
        },
    );
    let dialog = agent_actions::resolve_state(&agents, &hooks, &id, &probe(AGENT_NAME_CLAUDE));
    assert_eq!(dialog.activity, AgentActivity::AwaitingInput);
    assert_eq!(dialog.blocked_reason, Some(BlockedReason::Dialog));
    let changed = agent_actions::resolve_state(&agents, &hooks, &id, &probe(AGENT_NAME_GEMINI));
    assert_eq!(changed.activity, AgentActivity::Working);
    assert_eq!(changed.blocked_reason, None);
    assert!(agent_actions::build_detected_agents(&agents, &hooks, &id, Vec::new()).is_empty());
}

#[tokio::test]
async fn marker는_검증_뒤_삭제하고_missing도_성공해_추적을_해제한다() {
    let state = state();
    let agents = AgentStore::new();
    let marker = Marker::new();
    std::fs::File::create_new(&marker.0).unwrap();
    agents.register_wait_marker(marker.name());
    agent_actions::agent_release_marker(&state, &agents, marker.name()).await.unwrap();
    assert!(!marker.0.exists());
    assert!(agents.take_all_markers().is_empty());
    agents.register_wait_marker(marker.name());
    agent_actions::agent_release_marker(&state, &agents, marker.name()).await.unwrap();
    assert!(agents.take_all_markers().is_empty());
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn marker_검증_실패는_추적을_유지하고_삭제_실패는_추적을_해제한다() {
    let state = state();
    let agents = AgentStore::new();
    let invalid = "relative-marker".to_string();
    agents.register_wait_marker(invalid.clone());
    let error = agent_actions::agent_release_marker(&state, &agents, invalid.clone())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
    assert_eq!(agents.take_all_markers(), [invalid]);
    let directory = Marker::new();
    std::fs::create_dir(&directory.0).unwrap();
    agents.register_wait_marker(directory.name());
    assert!(agent_actions::agent_release_marker(&state, &agents, directory.name())
        .await
        .is_err());
    assert!(directory.0.is_dir());
    assert!(agents.take_all_markers().is_empty());
}

#[test]
fn cleanup은_검증된_marker만_삭제하고_전체_추적을_소비한다() {
    let agents = AgentStore::new();
    let valid = Marker::new();
    std::fs::File::create_new(&valid.0).unwrap();
    let invalid = Marker(std::env::temp_dir().join(format!("taide-agent-invalid-{}", Uuid::new_v4())));
    std::fs::File::create_new(&invalid.0).unwrap();
    agents.register_wait_marker(valid.name());
    agents.register_wait_marker(invalid.name());
    agent_actions::cleanup_all_wait_markers(&agents);
    assert!(!valid.0.exists());
    assert!(invalid.0.exists());
    assert!(agents.take_all_markers().is_empty());
}

#[tokio::test]
async fn pending_open은_순서를_유지해_한_번만_꺼낸다() {
    let agents = AgentStore::new();
    let first = ExternalOpenRequest {
        path: "fixture-first".to_string(),
        wait_marker: Some("fixture-marker".to_string()),
    };
    let second = ExternalOpenRequest {
        path: "fixture-second".to_string(),
        wait_marker: None,
    };
    agents.push_pending_external_open(first.clone());
    agents.push_pending_external_open(second.clone());
    assert_eq!(agent_actions::agent_pending_external_opens(&agents).await.unwrap(), [first, second]);
    assert!(agent_actions::agent_pending_external_opens(&agents).await.unwrap().is_empty());
}

#[test]
fn 세_command와_공유_helper는_runtime으로_위임한다() {
    let source = include_str!("../src/domain/agent/commands.rs");
    for name in ["agent_list", "agent_release_marker", "agent_pending_external_opens"] {
        let body = source
            .split_once(&format!("pub async fn {name}("))
            .unwrap()
            .1
            .split_once("\n}")
            .unwrap()
            .0;
        assert!(body.contains(&format!("agent_actions::{name}(")));
    }
    assert!(source.contains("pub use taide_runtime::agent_actions::{build_detected_agents, cleanup_all_wait_markers, resolve_state}"));
}
