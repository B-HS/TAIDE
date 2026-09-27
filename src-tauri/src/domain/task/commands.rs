use taide_runtime::{task_actions, TaskSupervisor};
use tauri::State;

use super::types::Task;
use crate::error::AppResult;
use crate::ids::ProjectId;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn detect_tasks(state: State<'_, AppState>, tasks: State<'_, TaskSupervisor>, project_id: ProjectId) -> AppResult<Vec<Task>> {
    task_actions::detect_tasks(&state, &tasks, project_id).await
}
