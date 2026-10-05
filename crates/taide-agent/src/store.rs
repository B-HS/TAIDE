use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use taide_infra::terminal_scan::{ScanEvent, ScanOutcome};
use taide_model::agent::{AgentActivity, DetectedAgent, ExternalOpenRequest};
use taide_model::ids::ProjectId;
use tokio::task::JoinHandle;

use crate::constants::HOOK_OVERRIDE_STALE_MS;
use crate::service;

#[derive(Default)]
struct AgentStoreInner {
    agents: HashMap<ProjectId, Vec<DetectedAgent>>,
    wait_markers: HashSet<String>,
    signals: HashMap<String, service::AgentSessionSignals>,
    #[cfg(unix)]
    process_names: HashMap<u32, Option<&'static str>>,
    pending_external_opens: Vec<ExternalOpenRequest>,
}

#[derive(Clone, Default)]
pub struct AgentStore(Arc<Mutex<AgentStoreInner>>);

fn last_known_activity(inner: &AgentStoreInner, session_id: &str) -> AgentActivity {
    inner
        .agents
        .values()
        .flatten()
        .find(|agent| agent.session_id == session_id)
        .map(|agent| agent.activity)
        .unwrap_or(AgentActivity::Unknown)
}

impl AgentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn diff(&self, project_id: &ProjectId, current: &[DetectedAgent]) -> Option<Vec<DetectedAgent>> {
        let mut guard = self.0.lock();
        let previous = guard.agents.entry(project_id.clone()).or_default();
        if service::agents_changed(previous, current) {
            *previous = current.to_vec();
            Some(current.to_vec())
        } else {
            None
        }
    }

    pub fn agents_for(&self, project_id: &ProjectId) -> Vec<DetectedAgent> {
        self.0.lock().agents.get(project_id).cloned().unwrap_or_default()
    }

    /// Classifies one detected agent session from the signals collected since the last tick,
    /// starting a signal record for a session seen for the first time.
    ///
    /// The record restarts when the same session's foreground agent changed, which `prune_signals`
    /// cannot catch on its own: it only drops sessions that ran *no* agent on a tick, so a handoff
    /// with no shell in between would leave the old agent's dialog latch and signature table
    /// speaking for the new one.
    pub fn classify_session_state(&self, session_id: &str, agent_name: &'static str) -> service::SessionState {
        let mut guard = self.0.lock();
        let previous = last_known_activity(&guard, session_id);
        let signals = guard
            .signals
            .entry(session_id.to_string())
            .or_insert_with(|| service::AgentSessionSignals::new(agent_name));
        if signals.agent_name != agent_name {
            *signals = service::AgentSessionSignals::new(agent_name);
        }
        service::classify_session_state(signals, previous, Instant::now())
    }

    /// Folds one scanned pty chunk into the session's signals. Runs on the pty reader thread for
    /// every chunk of every session, so a session without a detected agent costs one lock and one
    /// failed lookup and nothing else.
    pub fn record_scan(&self, session_id: &str, outcome: &ScanOutcome) {
        let mut guard = self.0.lock();
        let Some(signals) = guard.signals.get_mut(session_id) else {
            return;
        };
        let agent_name = signals.agent_name;
        service::apply_scan_to_signals(signals, outcome, agent_name, Instant::now());
    }

    pub fn record_input(&self, session_id: &str) {
        let mut guard = self.0.lock();
        let Some(signals) = guard.signals.get_mut(session_id) else {
            return;
        };
        service::note_input(signals, Instant::now());
    }

    pub fn record_scan_parts_at<'event>(
        &self,
        session_id: &str,
        events: impl IntoIterator<Item = &'event ScanEvent>,
        text: &str,
        overlap: &str,
        now: Instant,
    ) {
        let mut guard = self.0.lock();
        let Some(signals) = guard.signals.get_mut(session_id) else { return };
        let agent_name = signals.agent_name;
        service::apply_scan_parts_to_signals(signals, events, text, overlap, agent_name, now);
    }

    pub fn prune_signals(&self, valid_session_ids: &HashSet<String>) {
        self.0.lock().signals.retain(|session_id, _| valid_session_ids.contains(session_id));
    }

    #[cfg(unix)]
    pub fn unresolved_pids(&self, pids: &[(String, u32)]) -> Vec<u32> {
        let guard = self.0.lock();
        let mut unresolved: Vec<u32> = pids
            .iter()
            .map(|(_, pid)| *pid)
            .filter(|pid| !guard.process_names.contains_key(pid))
            .collect();
        unresolved.sort_unstable();
        unresolved.dedup();
        unresolved
    }

    #[cfg(unix)]
    pub fn remember_process_names(&self, resolved: HashMap<u32, Option<&'static str>>) {
        self.0.lock().process_names.extend(resolved);
    }

    #[cfg(unix)]
    pub fn probes_for(&self, pids: Vec<(String, u32)>) -> Vec<service::DetectedAgentProbe> {
        let guard = self.0.lock();
        pids.into_iter()
            .filter_map(|(session_id, pid)| {
                let name = (*guard.process_names.get(&pid)?)?;
                Some(service::DetectedAgentProbe { session_id, name, pid })
            })
            .collect()
    }

    /// Drops name-cache entries for pids that are no longer any session's foreground process, so a
    /// long-running app does not accumulate one entry per command the user ever ran.
    pub fn retain_process_names(&self, live_pids: &HashSet<u32>) {
        #[cfg(unix)]
        self.0.lock().process_names.retain(|pid, _| live_pids.contains(pid));
        #[cfg(not(unix))]
        let _ = live_pids;
    }

    pub fn register_wait_marker(&self, marker: String) {
        self.0.lock().wait_markers.insert(marker);
    }

    pub fn forget_wait_marker(&self, marker: &str) {
        self.0.lock().wait_markers.remove(marker);
    }

    pub fn take_all_markers(&self) -> Vec<String> {
        std::mem::take(&mut self.0.lock().wait_markers).into_iter().collect()
    }

    /// Queues an external-open request (from a cold-start or single-instance `taide <file>`
    /// invocation) so the frontend can drain it once it has mounted and subscribed, even if it
    /// missed the corresponding `AgentExternalOpen` event.
    pub fn push_pending_external_open(&self, request: ExternalOpenRequest) {
        self.0.lock().pending_external_opens.push(request);
    }

    pub fn drain_pending_external_opens(&self) -> Vec<ExternalOpenRequest> {
        std::mem::take(&mut self.0.lock().pending_external_opens)
    }
}

#[derive(Clone)]
pub struct HooksServerInfo {
    pub port: u16,
    pub token: String,
}

#[derive(Default)]
struct AgentHooksStoreInner {
    server: Option<HooksServerInfo>,
    accept_handle: Option<JoinHandle<()>>,
    project_overrides: HashMap<(ProjectId, String), (AgentActivity, Instant)>,
}

#[derive(Clone, Default)]
pub struct AgentHooksStore(Arc<Mutex<AgentHooksStoreInner>>);

impl AgentHooksStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn server_info(&self) -> Option<HooksServerInfo> {
        self.0.lock().server.clone()
    }

    pub fn set_server(&self, info: HooksServerInfo, accept_handle: JoinHandle<()>) -> HooksServerInfo {
        let mut guard = self.0.lock();
        if let Some(existing) = guard.server.clone() {
            accept_handle.abort();
            return existing;
        }
        guard.server = Some(info.clone());
        guard.accept_handle = Some(accept_handle);
        info
    }

    pub fn take_server(&self) -> Option<JoinHandle<()>> {
        let mut guard = self.0.lock();
        guard.server = None;
        guard.project_overrides.clear();
        guard.accept_handle.take()
    }

    pub fn set_project_override(&self, project_id: ProjectId, agent_name: String, activity: AgentActivity) {
        self.0
            .lock()
            .project_overrides
            .insert((project_id, agent_name), (activity, Instant::now()));
    }

    pub fn fresh_project_override(&self, project_id: &ProjectId, agent_name: &str) -> Option<AgentActivity> {
        let guard = self.0.lock();
        let key = (project_id.clone(), agent_name.to_string());
        let (activity, set_at) = guard.project_overrides.get(&key)?;
        if set_at.elapsed() >= Duration::from_millis(HOOK_OVERRIDE_STALE_MS) {
            return None;
        }
        Some(*activity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 세션의_에이전트가_바뀌면_이전_에이전트의_신호를_버린다() {
        use taide_model::agent::BlockedReason;

        use crate::constants::{AGENT_NAME_CLAUDE, AGENT_NAME_CODEX};

        let store = AgentStore::new();
        let dialog = ScanOutcome {
            events: Vec::new(),
            text: "Do you want to proceed?".to_string(),
            overlap: String::new(),
        };

        store.classify_session_state("session-1", AGENT_NAME_CLAUDE);
        store.record_scan("session-1", &dialog);
        assert_eq!(
            store.classify_session_state("session-1", AGENT_NAME_CLAUDE),
            service::SessionState {
                activity: AgentActivity::AwaitingInput,
                blocked_reason: Some(BlockedReason::Dialog),
            }
        );

        assert_eq!(
            store.classify_session_state("session-1", AGENT_NAME_CODEX),
            service::SessionState {
                activity: AgentActivity::Unknown,
                blocked_reason: None,
            },
            "에이전트가 바뀐 세션에 이전 에이전트의 다이얼로그 래치가 남으면 안 된다"
        );
    }

    #[test]
    fn 오래된_프로젝트_hook_override는_반환하지_않는다() {
        let store = AgentHooksStore::new();
        let project_id = ProjectId::from("project".to_string());
        store.0.lock().project_overrides.insert(
            (project_id.clone(), "codex".to_string()),
            (
                AgentActivity::AwaitingInput,
                Instant::now() - Duration::from_millis(HOOK_OVERRIDE_STALE_MS),
            ),
        );

        assert_eq!(store.fresh_project_override(&project_id, "codex"), None);
    }
}
