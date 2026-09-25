use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use taide_infra::lsp_proc::LspProcHandle;
use taide_model::ids::ProjectId;
use taide_model::lsp::{LanguageServerSpec, LspServerId, LspSessionInfo};

use crate::session::{LspMessageSubscribers, LspSessionLifecycle, LspSessionRoots};

/// Stores the process slot and shared state of one language-server session.
pub struct LspSessionEntry {
    pub project_id: ProjectId,
    pub server_id: LspServerId,
    pub root: String,
    pub spec: LanguageServerSpec,
    pub proc: Mutex<Option<Arc<LspProcHandle>>>,
    pub subscribers: LspMessageSubscribers,
    pub lifecycle: LspSessionLifecycle,
    pub roots: LspSessionRoots,
}

impl LspSessionEntry {
    pub fn new(
        project_id: ProjectId,
        spec: LanguageServerSpec,
        root: String,
        subscribers: LspMessageSubscribers,
    ) -> Self {
        Self {
            project_id,
            server_id: spec.id.clone(),
            roots: LspSessionRoots::new(root.clone()),
            root,
            spec,
            proc: Mutex::new(None),
            subscribers,
            lifecycle: LspSessionLifecycle::new(),
        }
    }
}

/// Tracks language-server sessions without depending on a UI or IPC runtime.
#[derive(Default)]
pub struct LspStore(Mutex<HashMap<String, Arc<LspSessionEntry>>>);

impl LspStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, session_id: &str) -> Option<Arc<LspSessionEntry>> {
        self.0.lock().get(session_id).cloned()
    }

    pub fn insert(&self, session_id: String, entry: Arc<LspSessionEntry>) {
        self.0.lock().insert(session_id, entry);
    }

    pub fn remove(&self, session_id: &str) -> Option<Arc<LspSessionEntry>> {
        self.0.lock().remove(session_id)
    }

    pub fn contains(&self, session_id: &str) -> bool {
        self.0.lock().contains_key(session_id)
    }

    pub fn find_reusable(
        &self,
        project_id: &ProjectId,
        server_id: &LspServerId,
        owner: &str,
    ) -> Option<(String, Arc<LspSessionEntry>)> {
        self.0
            .lock()
            .iter()
            .find(|(_, entry)| {
                &entry.project_id == project_id
                    && &entry.server_id == server_id
                    && entry.spec.shares_sessions
                    && !entry.lifecycle.is_stopping()
                    && entry.subscribers.contains(owner)
            })
            .map(|(id, entry)| (id.clone(), entry.clone()))
    }

    pub fn sessions_for_project(&self, project_id: &ProjectId) -> Vec<LspSessionInfo> {
        self.0
            .lock()
            .iter()
            .filter(|(_, entry)| &entry.project_id == project_id)
            .map(|(id, entry)| {
                let snapshot = entry.lifecycle.snapshot();
                LspSessionInfo {
                    session_id: id.clone(),
                    project_id: entry.project_id.clone(),
                    server_id: entry.server_id.clone(),
                    root: entry.root.clone(),
                    status: snapshot.status,
                    last_error: snapshot.last_error,
                    generation: snapshot.generation,
                }
            })
            .collect()
    }

    pub fn kill_all(&self) {
        for entry in self.0.lock().values() {
            entry.lifecycle.mark_stopping();
            if let Some(proc) = entry.proc.lock().as_ref() {
                proc.kill();
            }
        }
    }

    pub fn server_pids(&self) -> Vec<(ProjectId, String, u32)> {
        self.0
            .lock()
            .values()
            .filter_map(|entry| {
                let pid = entry.proc.lock().as_ref().and_then(|proc| proc.pid())?;
                Some((entry.project_id.clone(), entry.spec.name.clone(), pid))
            })
            .collect()
    }
}
