use tauri::State;

pub use taide_runtime::TreeStore;
use taide_runtime::{tree_actions, TaskSupervisor};

use super::types::TreeRowPage;
use crate::error::AppResult;
use crate::ids::ProjectId;
use crate::infra::perf::{self, SpanSlot};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn tree_rows(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    tasks: State<'_, TaskSupervisor>,
    project_id: ProjectId,
    offset: u32,
    limit: Option<u32>,
) -> AppResult<TreeRowPage> {
    tree_actions::tree_rows(&state, &tree_store, &tasks, project_id, offset, limit).await
}

#[tauri::command]
#[specta::specta]
pub async fn tree_toggle(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    tasks: State<'_, TaskSupervisor>,
    project_id: ProjectId,
    path: String,
) -> AppResult<TreeRowPage> {
    let _span = perf::span(SpanSlot::TreeToggle);
    tree_actions::tree_toggle(&state, &tree_store, &tasks, project_id, path).await
}

/// "모두 접기" — clears this project's entire expanded set and returns the collapsed page.
///
/// One mutation instead of the frontend's former loop of a `tree_toggle` per *visible* expanded
/// row. That loop could only name rows it could see, so every descendant hidden under an
/// already-collapsed parent survived it and sprang back the next time its parent opened
/// (`service::collapse_all`), and each iteration paid the app-wide mutation guard plus a full page
/// re-serialization. Collapsing reads no directory, so the prefetch here only covers the cold-start
/// case where this is the first call for the project and `ensure_entry` has to load the root.
#[tauri::command]
#[specta::specta]
pub async fn tree_collapse_all(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    tasks: State<'_, TaskSupervisor>,
    project_id: ProjectId,
) -> AppResult<TreeRowPage> {
    tree_actions::tree_collapse_all(&state, &tree_store, &tasks, project_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn tree_reveal(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    tasks: State<'_, TaskSupervisor>,
    project_id: ProjectId,
    path: String,
) -> AppResult<TreeRowPage> {
    let _span = perf::span(SpanSlot::TreeReveal);
    tree_actions::tree_reveal(&state, &tree_store, &tasks, project_id, path).await
}

#[tauri::command]
#[specta::specta]
pub async fn tree_refresh(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    tasks: State<'_, TaskSupervisor>,
    project_id: ProjectId,
    dir: String,
) -> AppResult<TreeRowPage> {
    tree_actions::tree_refresh(&state, &tree_store, &tasks, project_id, dir).await
}
