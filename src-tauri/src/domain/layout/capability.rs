use tauri::AppHandle;

use crate::domain::project::capability::ProjectCapability;
use crate::domain::project::types::Project;
use crate::ids::ProjectId;
use crate::state::AppState;

use super::service;

/// Owns the in-memory layout entry for a project: loads it from disk on attach and, on detach,
/// flushes a still-dirty layout to disk *before* dropping the in-memory copy.
pub struct LayoutCapability;

impl ProjectCapability for LayoutCapability {
    fn attach(&self, _app: &AppHandle, state: &AppState, project: &Project) {
        let layout = service::load_layout(&state.paths, &project.id);
        state.layouts.write().insert(project.id.clone(), layout);
    }

    /// The flush-then-remove order here is correctness-sensitive, not just cleanup: a layout marked
    /// dirty but not yet caught by `flush_dirty_layouts`'s periodic 2-second timer would otherwise
    /// be discarded unsaved — the `state.layouts.write().remove` below drops the in-memory copy,
    /// and the periodic flusher's next pass over `dirty_layouts` would then find nothing left in
    /// `state.layouts` for this `project_id` and silently skip it (see that function's
    /// `filter_map`, which also `log::warn!`s on exactly this miss as a safety net for any other
    /// path that removes a layout without flushing first). Flushing synchronously here, before the
    /// removal, closes that window.
    fn detach(&self, _app: &AppHandle, state: &AppState, project_id: &ProjectId) {
        if state.dirty_layouts.write().remove(project_id) {
            if let Some(layout) = state.layouts.read().get(project_id).cloned() {
                if let Err(error) = service::save_layout(&state.paths, project_id, &layout) {
                    log::warn!("프로젝트 종료 시 레이아웃 저장 실패 ({project_id}): {error}");
                }
            }
        }

        state.layouts.write().remove(project_id);
    }
}
