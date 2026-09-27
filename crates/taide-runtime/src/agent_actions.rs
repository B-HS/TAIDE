use std::future::Future;

use taide_agent::service;
use taide_agent::store::{AgentHooksStore, AgentStore};
use taide_model::agent::{AgentActivity, DetectedAgent, ExternalOpenRequest, ProjectAgents};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;

use crate::AppState;

/// Cleans up tracked wait markers using the existing path validation policy.
pub fn cleanup_all_wait_markers(store: &AgentStore) {
    let temp_dir = std::env::temp_dir();
    for marker in store.take_all_markers() {
        if let Ok(path) = service::validate_wait_marker_path(&marker, &temp_dir) {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// The session's own signals decide. Only a session that has produced no signal at all falls back to
/// the hook bridge's project-scoped override, and only for an agent that actually delivers its
/// events through that bridge (`service::uses_project_hook_override`) — an agent whose events arrive
/// in-band per session must not be spoken for by a stale project-wide answer.
///
/// The fallback carries no `blocked_reason`: the override is one activity per project, recorded
/// from an HTTP hook that has already been mapped to it (`service::map_hook_event_to_activity`), so
/// there is no session latch to read a reason off. Reporting `AwaitingInput` with no reason is the
/// honest answer there.
pub fn resolve_state(
    agents: &AgentStore,
    hooks_store: &AgentHooksStore,
    project_id: &ProjectId,
    probe: &service::DetectedAgentProbe,
) -> service::SessionState {
    let state = agents.classify_session_state(&probe.session_id, probe.name);
    if state.activity != AgentActivity::Unknown || !service::uses_project_hook_override(probe.name) {
        return state;
    }
    match hooks_store.fresh_project_override(project_id, probe.name) {
        Some(activity) => service::SessionState {
            activity,
            blocked_reason: None,
        },
        None => state,
    }
}

/// Builds detected agents with the existing session and hook state policy.
pub fn build_detected_agents(
    agents: &AgentStore,
    hooks_store: &AgentHooksStore,
    project_id: &ProjectId,
    probes: Vec<service::DetectedAgentProbe>,
) -> Vec<DetectedAgent> {
    probes
        .into_iter()
        .map(|probe| {
            let state = resolve_state(agents, hooks_store, project_id, &probe);
            DetectedAgent {
                session_id: probe.session_id,
                name: probe.name.to_string(),
                pid: probe.pid,
                activity: state.activity,
                blocked_reason: state.blocked_reason,
            }
        })
        .collect()
}

fn ensure_project_open(state: &AppState, project_id: &ProjectId) -> AppResult<()> {
    if state.projects.read().contains_key(project_id) {
        return Ok(());
    }
    Err(AppError::NotFound(format!("project not open: {project_id}")))
}

/// Applies the project gate before querying native PID and probe ports.
pub async fn agent_list<F, Fut>(
    state: &AppState,
    agents: &AgentStore,
    agent_hooks: &AgentHooksStore,
    foreground_pids: impl FnOnce() -> Vec<(String, u32)>,
    probe_agents: F,
    project_id: ProjectId,
) -> AppResult<ProjectAgents>
where
    F: FnOnce(Vec<(String, u32)>) -> Fut,
    Fut: Future<Output = AppResult<Vec<service::DetectedAgentProbe>>>,
{
    ensure_project_open(state, &project_id)?;

    let pids = foreground_pids();
    let probes = probe_agents(pids).await?;

    let detected = build_detected_agents(agents, agent_hooks, &project_id, probes);
    Ok(ProjectAgents {
        project_id,
        agents: detected,
    })
}

/// Releases a validated wait marker and updates the tracking set.
pub async fn agent_release_marker(state: &AppState, agents: &AgentStore, marker: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let path = service::validate_wait_marker_path(&marker, &std::env::temp_dir())?;

    let result = match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from(error)),
    };
    agents.forget_wait_marker(&marker);
    result
}

/// Drains pending external-open requests in their existing order.
pub async fn agent_pending_external_opens(agents: &AgentStore) -> AppResult<Vec<ExternalOpenRequest>> {
    Ok(agents.drain_pending_external_opens())
}
