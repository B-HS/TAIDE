use std::path::Path;

use taide_infra::language::LanguageOverlay;
use taide_model::plugin::LoadedPlugin;
use tauri::AppHandle;

use crate::error::AppResult;

pub struct PluginRuntimePort {
    pub language_overlays: fn(&AppHandle) -> Vec<LanguageOverlay>,
    pub commit_staged_import: fn(&AppHandle, &Path, &str) -> AppResult<LoadedPlugin>,
}
