use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use taide_model::agent::AgentActivity;
use taide_model::ids::ProjectId;
use tokio::task::JoinHandle;

use crate::constants::HOOK_OVERRIDE_STALE_MS;

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
