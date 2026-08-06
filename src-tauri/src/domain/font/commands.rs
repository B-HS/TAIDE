use super::service;
use super::types::FontFamily;
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub async fn font_list() -> AppResult<Vec<FontFamily>> {
    Ok(service::list_families())
}
