use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;

use crate::domain::layout::types::ProjectLayout;
use crate::domain::project::types::{Project, SessionState};
use crate::domain::settings::types::Settings;
use crate::ids::ProjectId;
use crate::infra::watcher::WatcherHandle;
use crate::paths::AppPaths;

pub struct AppState {
    pub paths: AppPaths,
    pub session: RwLock<SessionState>,
    pub projects: RwLock<HashMap<ProjectId, Project>>,
    pub layouts: RwLock<HashMap<ProjectId, ProjectLayout>>,
    pub settings: RwLock<Settings>,
    pub dirty_layouts: RwLock<HashSet<ProjectId>>,
    pub watchers: RwLock<HashMap<ProjectId, WatcherHandle>>,
    pub git_watchers: RwLock<HashMap<ProjectId, WatcherHandle>>,
    mutation_guard: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            session: RwLock::new(SessionState::default()),
            projects: RwLock::new(HashMap::new()),
            layouts: RwLock::new(HashMap::new()),
            settings: RwLock::new(Settings::default()),
            dirty_layouts: RwLock::new(HashSet::new()),
            watchers: RwLock::new(HashMap::new()),
            git_watchers: RwLock::new(HashMap::new()),
            mutation_guard: tokio::sync::Mutex::new(()),
        }
    }

    pub async fn begin_mutation(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.mutation_guard.lock().await
    }
}
