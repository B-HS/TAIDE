use super::service;
use super::types::FontFamily;
use crate::error::{AppError, AppResult};

/// Never takes `AppState::begin_mutation` (it reads no app state); the one blocking cost left is
/// the first call's system font scan, which runs on a blocking thread so it doesn't pin an async
/// worker — every later call clones `service`'s process-lifetime cache (audit R8#11).
#[tauri::command]
#[specta::specta]
pub async fn font_list() -> AppResult<Vec<FontFamily>> {
    tauri::async_runtime::spawn_blocking(service::list_families)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}
