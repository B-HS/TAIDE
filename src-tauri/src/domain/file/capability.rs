use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::domain::project::capability::{ProjectAttachment, ProjectCapability};
use crate::domain::project::types::Project;
use crate::events::FsChanged;
use crate::ids::ProjectId;
use crate::infra::self_write::resolve_from_app;
use crate::infra::watcher;
use crate::state::AppState;

use super::types::FsChange;

/// The expensive half of the project-root file watcher attach — the file-ID index walk
/// `watcher::start_watch` performs, which stats every entry below the root that the watcher speaks
/// for (`infra::watcher::ScopedIdCache`, which prunes `IGNORED_DIR_NAMES` subtrees; the crate's own
/// `FileIdMap` did not, and took seconds on a large working tree). Deliberately touches **no**
/// `AppState`: both callers
/// (`project_open`'s `FileWatcherCapability::build_attachment` and the boot restore path
/// `domain::project::commands::restore_project_watchers`) run it with no `AppState::begin_mutation`
/// held, and reach for the guard only around [`register_watcher_handle`].
///
/// The returned handle is already **live** — `start_watch` has subscribed by the time this returns
/// — so fs changes arriving between the build and its registration are still delivered; the
/// registration is only what gives `project_close` a handle to drop. Returns `None` (after logging
/// the same warning the caller would have) when the watcher fails to start.
pub fn build_watcher_handle(app: &AppHandle, project_id: &ProjectId, root: &str) -> Option<watcher::WatcherHandle> {
    let emit_handle = app.clone();
    let emit_project = project_id.clone();

    match watcher::start_watch(
        std::path::PathBuf::from(root),
        watcher::WatchScope::Project,
        move |changes: Vec<FsChange>| {
            let changes = resolve_from_app(&emit_handle.state::<AppState>().self_writes, changes);
            for change in changes {
                let _ = FsChanged {
                    project_id: emit_project.clone(),
                    change,
                }
                .emit(&emit_handle);
            }
        },
    ) {
        Ok(handle) => Some(handle),
        Err(error) => {
            log::warn!("파일 감시를 시작하지 못했습니다 ({root}): {error}");
            None
        }
    }
}

/// Registers a handle [`build_watcher_handle`] already built — the one `AppState` write the split
/// needs `AppState::begin_mutation` for, held only across this call (plus the caller's own
/// re-checks), never across the handle's own walk.
pub fn register_watcher_handle(state: &AppState, project_id: &ProjectId, handle: watcher::WatcherHandle) {
    state.watchers.write().insert(project_id.clone(), handle);
}

pub struct FileWatcherCapability;

impl ProjectCapability for FileWatcherCapability {
    fn build_attachment(&self, app: &AppHandle, _state: &AppState, project: &Project) -> ProjectAttachment {
        let Some(handle) = build_watcher_handle(app, &project.id, &project.root) else {
            return ProjectAttachment::none();
        };

        ProjectAttachment::new(move |state: &AppState, project: &Project| {
            register_watcher_handle(state, &project.id, handle);
        })
    }

    fn detach(&self, _app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        state.watchers.write().remove(project_id);
    }
}
