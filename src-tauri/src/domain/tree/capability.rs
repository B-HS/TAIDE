use tauri::{AppHandle, Manager};

use crate::domain::project::capability::ProjectCapability;
use crate::ids::ProjectId;
use crate::state::AppState;

use super::commands::TreeStore;

/// Evicts the closing project's cached tree ([`TreeStore`]). Plain cache eviction — no correctness
/// risk either way, since `ensure_entry` transparently rebuilds a missing entry on next access —
/// but still matters: without it, reopening the same folder resurrects a possibly-stale directory
/// listing instead of scanning fresh, and the entry otherwise sits in memory for the rest of the
/// app's lifetime regardless of whether the project ever reopens.
pub struct TreeCacheCapability;

impl ProjectCapability for TreeCacheCapability {
    fn detach(&self, app: &AppHandle, _state: &AppState, project_id: &ProjectId) {
        app.state::<TreeStore>().remove(project_id);
    }
}
