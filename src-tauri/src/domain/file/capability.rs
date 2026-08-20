use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::domain::project::capability::ProjectCapability;
use crate::domain::project::types::Project;
use crate::events::FsChanged;
use crate::ids::ProjectId;
use crate::infra::self_write::resolve_from_app;
use crate::infra::watcher;
use crate::state::AppState;

use super::types::FsChange;

/// Starts the project-root file watcher that fans debounced fs changes out as [`FsChanged`]
/// events, registering its handle in `state.watchers`. `project_open`'s
/// `FileWatcherCapability::attach` is the only caller that still builds and registers in one
/// call — it already runs its entire `ProjectCapabilities::attach_all` walk under one
/// `AppState::begin_mutation` acquisition, so splitting internally here changes nothing about when
/// that guard is held. The boot restore path (`lib.rs::restore_project_watchers`) calls
/// [`build_watcher_handle`] and [`register_watcher_handle`] separately instead, so its own guard
/// only has to cover the register half — see that function's doc.
pub fn attach_watcher(app: &AppHandle, state: &AppState, project_id: &ProjectId, root: &str) {
    if let Some(handle) = build_watcher_handle(app, project_id, root) {
        register_watcher_handle(state, project_id, handle);
    }
}

/// The expensive half of [`attach_watcher`] — the `notify-debouncer-full` `FileIdMap` walk
/// `watcher::start_watch` performs — split out so it can run without touching `AppState` at all.
/// Returns `None` (after logging the same warning `attach_watcher` always has on failure) when the
/// watcher fails to start. [`register_watcher_handle`] is the only `AppState` write this handle
/// still needs.
pub fn build_watcher_handle(app: &AppHandle, project_id: &ProjectId, root: &str) -> Option<watcher::WatcherHandle> {
    let emit_handle = app.clone();
    let emit_project = project_id.clone();

    match watcher::start_watch(std::path::PathBuf::from(root), move |changes: Vec<FsChange>| {
        let changes = resolve_from_app(&emit_handle.state::<AppState>().self_writes, changes);
        for change in changes {
            let _ = FsChanged {
                project_id: emit_project.clone(),
                change,
            }
            .emit(&emit_handle);
        }
    }) {
        Ok(handle) => Some(handle),
        Err(error) => {
            log::warn!("파일 감시를 시작하지 못했습니다 ({root}): {error}");
            None
        }
    }
}

/// Registers a handle [`build_watcher_handle`] already built — the one `AppState` write the boot
/// restore split needs `AppState::begin_mutation` for (`lib.rs::restore_project_watchers`), held
/// only across this call and its own `state.projects`/`state.watchers` re-checks, not across the
/// handle's own walk.
pub fn register_watcher_handle(state: &AppState, project_id: &ProjectId, handle: watcher::WatcherHandle) {
    state.watchers.write().insert(project_id.clone(), handle);
}

pub struct FileWatcherCapability;

impl ProjectCapability for FileWatcherCapability {
    fn attach(&self, app: &AppHandle, state: &AppState, project: &Project) {
        attach_watcher(app, state, &project.id, &project.root);
    }

    fn detach(&self, _app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        state.watchers.write().remove(project_id);
    }
}
