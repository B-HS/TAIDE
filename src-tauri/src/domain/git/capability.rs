use std::path::Path;

use tauri::{AppHandle, Manager};

use crate::domain::project::capability::ProjectCapability;
use crate::domain::project::types::{CapabilityKind, Project};
use crate::ids::ProjectId;
use crate::state::AppState;

use super::commands::GitStore;
use super::watch;

/// Owns the `.git`-directory watcher for a project. Split from [`GitCacheCapability`] (rather than
/// one combined git capability) so a single forward walk of the registration list reproduces
/// `project_close`'s historical reap order exactly — the repo-root cache eviction sits *after* the
/// terminal reap in that order, while the watcher removal sits before it.
///
/// [`Self::detected_kind`] is the single source of the `Git` entry in `Project.capabilities`
/// (`project_open` injects the registry's `detected_kinds` into `open_project`), and
/// [`Self::attach`]'s `contains(Git)` gate is therefore the one git decision on the open path —
/// `watch::register_git_watcher` performs no second filesystem probe. Only the boot restore path
/// re-probes the live filesystem, via `watch::attach_git_watcher`.
pub struct GitWatcherCapability;

impl ProjectCapability for GitWatcherCapability {
    fn detected_kind(&self, root: &Path) -> Option<CapabilityKind> {
        root.join(watch::GIT_DIR_NAME).is_dir().then_some(CapabilityKind::Git)
    }

    fn attach(&self, app: &AppHandle, state: &AppState, project: &Project) {
        if !project.capabilities.contains(&CapabilityKind::Git) {
            return;
        }
        watch::register_git_watcher(app, state, &project.id, &project.root);
    }

    fn detach(&self, _app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        state.git_watchers.write().remove(project_id);
    }
}

/// Evicts the closing project's cached repo root ([`GitStore`]). Plain cache eviction — no
/// correctness risk either way, since `resolve_repo_root` transparently rebuilds a missing entry
/// on next access — but still matters: without it, reopening the same folder resurrects a
/// possibly-stale repo root instead of resolving fresh, and the entry otherwise sits in memory for
/// the rest of the app's lifetime regardless of whether the project ever reopens.
pub struct GitCacheCapability;

impl ProjectCapability for GitCacheCapability {
    fn detach(&self, app: &AppHandle, _state: &AppState, project_id: &ProjectId) {
        app.state::<GitStore>().remove(project_id);
    }
}
