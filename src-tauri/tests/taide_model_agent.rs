use taide_lib::domain::agent::types::{AgentActivity, AgentHooksStatus, BlockedReason, DetectedAgent, ExternalOpenRequest, ProjectAgents};
use taide_model::agent::{
    AgentHooksStatus as ModelAgentHooksStatus, DetectedAgent as ModelDetectedAgent, ExternalOpenRequest as ModelExternalOpenRequest,
    ProjectAgents as ModelProjectAgents,
};

#[test]
fn 감지된_에이전트의_기존_상태_wire를_유지한다() {
    let model: ModelDetectedAgent = serde_json::from_value(serde_json::json!({
        "sessionId": "pty-1",
        "name": "codex",
        "pid": 42,
        "activity": "awaitingInput",
        "blockedReason": "permission"
    }))
    .expect("기존 에이전트 상태");
    let facade: DetectedAgent = model;
    assert_eq!(facade.activity, AgentActivity::AwaitingInput);
    assert_eq!(facade.blocked_reason, Some(BlockedReason::Permission));

    let agents: ModelProjectAgents = serde_json::from_value(serde_json::json!({
        "projectId": "prj-1",
        "agents": []
    }))
    .expect("프로젝트 에이전트 목록");
    let facade: ProjectAgents = agents;
    assert!(facade.agents.is_empty());
}

#[test]
fn hook과_외부_열기_요청의_기존_wire를_유지한다() {
    let status: ModelAgentHooksStatus = serde_json::from_value(serde_json::json!({
        "agentName": "claude",
        "scope": "project",
        "installed": true,
        "requiresTaideCli": false
    }))
    .expect("기존 hook 상태");
    let facade: AgentHooksStatus = status;
    assert!(facade.installed);
    assert!(!facade.requires_taide_cli);

    let request: ModelExternalOpenRequest = serde_json::from_value(serde_json::json!({
        "path": "/repo/main.rs"
    }))
    .expect("기존 외부 열기 요청");
    let facade: ExternalOpenRequest = request;
    assert_eq!(facade.wait_marker, None);
}
