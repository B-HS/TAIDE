use std::path::Path;

use super::service;
use super::types::VsixThemeExtractionResult;
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub async fn vsix_extract_themes(vsix_path: String) -> AppResult<VsixThemeExtractionResult> {
    service::extract_themes(Path::new(&vsix_path))
}
