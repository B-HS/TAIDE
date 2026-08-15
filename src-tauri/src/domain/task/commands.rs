use tauri::State;

use super::service;
use super::types::Task;
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::infra::root_guard;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn detect_tasks(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Vec<Task>> {
    let root = root_guard::project_root(&state.projects.read(), &project_id)?;

    tauri::async_runtime::spawn_blocking(move || service::detect_tasks(&root))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}
