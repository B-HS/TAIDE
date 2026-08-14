use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;

use crate::domain::layout::types::ProjectLayout;
use crate::domain::project::types::{Project, SessionState};
use crate::domain::settings::types::Settings;
use crate::ids::ProjectId;
use crate::infra::watcher::WatcherHandle;
use crate::paths::AppPaths;

/// Tracks the hot-exit close-intercept handshake between the `CloseRequested`
/// window event and the frontend's flush completion. `Idle` -> `Pending` on
/// the first close attempt (guards re-entrant close requests from re-emitting
/// the flush event), `Pending` -> `Ready` once the frontend confirms the
/// flush or the timeout fallback fires (guards a double `AppHandle::exit`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotExitFlushPhase {
    Idle,
    Pending,
    Ready,
}

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
    hot_exit_flush: parking_lot::Mutex<HotExitFlushPhase>,
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
            hot_exit_flush: parking_lot::Mutex::new(HotExitFlushPhase::Idle),
        }
    }

    pub async fn begin_mutation(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.mutation_guard.lock().await
    }

    /// Returns `true` only for the first `CloseRequested` after boot; later
    /// re-entrant close attempts return `false` so the caller can skip
    /// re-emitting the flush-requested event while still blocking the close.
    pub fn begin_hot_exit_flush(&self) -> bool {
        let mut phase = self.hot_exit_flush.lock();
        if *phase != HotExitFlushPhase::Idle {
            return false;
        }
        *phase = HotExitFlushPhase::Pending;
        true
    }

    /// Returns `true` only for whichever of {frontend completion, timeout
    /// fallback} observes the flush as still pending, so exactly one of them
    /// proceeds to actually exit the app.
    pub fn complete_hot_exit_flush(&self) -> bool {
        let mut phase = self.hot_exit_flush.lock();
        if *phase != HotExitFlushPhase::Pending {
            return false;
        }
        *phase = HotExitFlushPhase::Ready;
        true
    }
}
