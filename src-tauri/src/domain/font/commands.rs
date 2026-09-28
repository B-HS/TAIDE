use taide_runtime::TaskSupervisor;
use tauri::State;

use super::service;
use super::types::FontFamily;
use crate::error::AppResult;

/// Never takes `AppState::begin_mutation` (it reads no app state); the one blocking cost left is
/// the first call's system font scan, which runs on a blocking thread so it doesn't pin an async
/// worker — every later call clones `service`'s process-lifetime cache (audit R8#11).
#[tauri::command]
#[specta::specta]
pub async fn font_list(tasks: State<'_, TaskSupervisor>) -> AppResult<Vec<FontFamily>> {
    tasks.run_blocking_result("font-list", || Ok(service::list_families())).await
}
