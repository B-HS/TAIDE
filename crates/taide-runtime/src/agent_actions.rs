use std::collections::HashSet;
use std::future::Future;

use taide_agent::service;
use taide_agent::store::{AgentHooksStore, AgentStore};
use taide_model::agent::{AgentActivity, DetectedAgent, ExternalOpenRequest, ProjectAgents};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;

use crate::{AppState, EventSink, TaskSupervisor};

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
    tasks: &TaskSupervisor,
    foreground_pids: impl FnOnce() -> Vec<(String, u32)>,
    probe_agents: F,
    project_id: ProjectId,
) -> AppResult<ProjectAgents>
where
    F: FnOnce(Vec<(String, u32)>) -> Fut,
    Fut: Future<Output = AppResult<Vec<service::DetectedAgentProbe>>>,
{
    ensure_project_open(state, &project_id)?;
    let _operation = tasks
        .begin_operation("agent-list")
        .ok_or_else(|| AppError::Forbidden("agent runtime is shutting down".to_string()))?;

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

/// Owns one poll through probe, state publication and pruning using the host's registered supervisor.
pub async fn poll_agents<F, Fut>(
    state: &AppState,
    agents: &AgentStore,
    agent_hooks: &AgentHooksStore,
    events: &impl EventSink,
    tasks: &TaskSupervisor,
    foreground_pids: impl Fn(&ProjectId) -> Vec<(String, u32)>,
    probe_agents: F,
) where
    F: Fn(Vec<(String, u32)>) -> Fut,
    Fut: Future<Output = AppResult<Vec<service::DetectedAgentProbe>>>,
{
    let Some(_operation) = tasks.begin_operation("agent-poll") else {
        return;
    };
    let project_ids: Vec<_> = state.projects.read().keys().cloned().collect();
    let mut valid_session_ids = HashSet::new();
    let mut live_pids = HashSet::new();

    for project_id in project_ids {
        let pids = foreground_pids(&project_id);
        live_pids.extend(pids.iter().map(|(_, pid)| *pid));

        let Ok(probes) = probe_agents(pids).await else {
            continue;
        };
        let detected = build_detected_agents(agents, agent_hooks, &project_id, probes);
        valid_session_ids.extend(detected.iter().map(|agent| agent.session_id.clone()));

        if let Some(changed) = agents.diff(&project_id, &detected) {
            events.publish(AppEvent::AgentStateChanged {
                project_id: project_id.clone(),
                agents: changed,
            });
        }
    }

    agents.prune_signals(&valid_session_ids);
    agents.retain_process_names(&live_pids);
}

/// Owns a decoded hook payload through project override, cached activity and event publication.
pub fn apply_hook_payload(
    state: &AppState,
    agents: &AgentStore,
    agent_hooks: &AgentHooksStore,
    events: &impl EventSink,
    tasks: &TaskSupervisor,
    agent_name: &str,
    payload: &service::HookPayload,
) {
    let Some(_operation) = tasks.begin_operation("agent-hook-payload") else {
        return;
    };
    if !service::is_hook_managed_agent(agent_name) {
        return;
    }
    let Some(activity) = service::map_hook_event_to_activity(agent_name, &payload.hook_event_name) else {
        return;
    };

    let projects: Vec<_> = {
        let guard = state.projects.read();
        guard.iter().map(|(id, project)| (id.clone(), project.root.clone())).collect()
    };

    let Some(project_id) = service::match_project_by_cwd(&payload.cwd, &projects).cloned() else {
        return;
    };

    agent_hooks.set_project_override(project_id.clone(), agent_name.to_string(), activity);

    let mut updated = agents.agents_for(&project_id);
    if updated.is_empty() {
        return;
    }
    service::apply_hook_activity(&mut updated, agent_name, activity);

    if let Some(changed) = agents.diff(&project_id, &updated) {
        events.publish(AppEvent::AgentStateChanged {
            project_id,
            agents: changed,
        });
    }
}
