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

/// Evicts the closing project's cached repo root ([`GitStore`]). `resolve_repo_root` transparently
/// rebuilds a missing `repo_roots` entry on next access, so on that axis alone eviction carries no
/// correctness risk — but `GitStore::remove` does more than that single-map eviction: it also
/// conditionally evicts the repo's `push_fetch_lock` entry (contract 2026-08-25 §1-b), and that half
/// *is* correctness-sensitive — an unconditional removal there would defeat same-repo push/fetch
/// serialization for a project reopened mid-push/fetch at the same root (see `GitStore::remove`'s
/// own doc for the `Arc::strong_count` guard that avoids exactly that). Skipping this call
/// altogether would also resurrect a possibly-stale repo root on reopen instead of resolving fresh,
/// and leave the entry sitting in memory for the rest of the app's lifetime regardless of whether
/// the project ever reopens.
pub struct GitCacheCapability;

impl ProjectCapability for GitCacheCapability {
    fn detach(&self, app: &AppHandle, _state: &AppState, project_id: &ProjectId) {
        app.state::<GitStore>().remove(project_id);
    }
}
