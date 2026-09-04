use std::path::Path;

use tauri::{AppHandle, Manager};

use crate::domain::project::capability::{ProjectAttachment, ProjectCapability};
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
/// [`Self::build_attachment`]'s `contains(Git)` gate is therefore the git decision on the open
/// path. Both paths now share `watch::build_git_watcher_handle`, so both also carry its `.git`
/// `is_dir` probe: on the boot restore path that probe is the whole decision (a project's persisted
/// `capabilities` predates whatever happened to `root` while the app was closed), while here it is
/// a re-check of what `detected_kind` decided moments earlier in the same command — one `stat` that
/// closes the window where the directory was removed in between, rather than starting a watch on a
/// path that no longer exists.
pub struct GitWatcherCapability;

impl ProjectCapability for GitWatcherCapability {
    fn detected_kind(&self, root: &Path) -> Option<CapabilityKind> {
        root.join(watch::GIT_DIR_NAME).is_dir().then_some(CapabilityKind::Git)
    }

    fn build_attachment(&self, app: &AppHandle, _state: &AppState, project: &Project) -> ProjectAttachment {
        if !project.capabilities.contains(&CapabilityKind::Git) {
            return ProjectAttachment::none();
        }
        let Some(handle) = watch::build_git_watcher_handle(app, &project.id, &project.root) else {
            return ProjectAttachment::none();
        };

        ProjectAttachment::new(move |state: &AppState, project: &Project| {
            watch::register_git_watcher_handle(state, &project.id, handle);
        })
    }

    fn detach(&self, _app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        state.git_watchers.write().remove(project_id);
    }
}

/// Evicts the closing project's cached repo root **and cached `git_status` result** ([`GitStore`]).
/// `resolve_repo_root` transparently rebuilds a missing `repo_roots` entry on next access, so on
/// that axis alone eviction carries no correctness risk — but `GitStore::remove` does more than that
/// single-map eviction: it also drops the project's status-cache slot, and conditionally evicts the
/// repo's `push_fetch_lock` entry (contract 2026-08-25 §1-b). The lock half *is*
/// correctness-sensitive — an unconditional removal there would defeat same-repo push/fetch
/// serialization for a project reopened mid-push/fetch at the same root (see `GitStore::remove`'s
/// own doc for the `Arc::strong_count` guard that avoids exactly that). Skipping this call
/// altogether would also resurrect a possibly-stale repo root and a stale status on reopen instead
/// of resolving both fresh, and leave the entries sitting in memory for the rest of the app's
/// lifetime regardless of whether the project ever reopens.
///
/// The status cache needs no `attach` half: its slots are created on demand by `git_status` and its
/// invalidation subscriptions are process-wide, so this `detach` is the whole of its
/// `architecture.md` §6.3 obligation.
pub struct GitCacheCapability;

impl ProjectCapability for GitCacheCapability {
    fn detach(&self, app: &AppHandle, _state: &AppState, project_id: &ProjectId) {
        app.state::<GitStore>().remove(project_id);
    }
}
