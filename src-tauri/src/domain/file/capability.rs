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
/// events, registering its handle in `state.watchers`. Also called directly by the boot restore
/// path in `lib.rs`, which re-attaches watchers for restored projects without running the full
/// capability attach (layout/ide/agent have their own dedicated boot paths).
pub fn attach_watcher(app: &AppHandle, state: &AppState, project_id: &ProjectId, root: &str) {
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
        Ok(handle) => {
            state.watchers.write().insert(project_id.clone(), handle);
        }
        Err(error) => log::warn!("파일 감시를 시작하지 못했습니다 ({root}): {error}"),
    }
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
