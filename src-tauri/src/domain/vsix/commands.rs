use std::path::Path;

use tauri::State;

use super::service;
use super::types::VsixThemeExtractionResult;
use crate::domain::plugin::commands::PluginStore;
use crate::domain::plugin::service as plugin_service;
use crate::domain::plugin::types::LoadedPlugin;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn vsix_extract_themes(vsix_path: String) -> AppResult<VsixThemeExtractionResult> {
    service::extract_themes(Path::new(&vsix_path))
}

/// Imports a real VS Code `.vsix`'s language/grammar contributions as a new TAIDE plugin
/// (`service::import_vsix_as_plugin`), then reloads the plugin list the same way `plugin_install`
/// does so the frontend gets the freshly-installed plugin's enabled/error state immediately.
#[tauri::command]
#[specta::specta]
pub async fn vsix_import_plugin(state: State<'_, AppState>, store: State<'_, PluginStore>, vsix_path: String) -> AppResult<LoadedPlugin> {
    let _guard = state.begin_mutation().await;
    let plugin_id = service::import_vsix_as_plugin(Path::new(&vsix_path), &state.paths.plugins_dir())?;

    let loaded = plugin_service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    loaded
        .into_iter()
        .find(|plugin| plugin.manifest.id == plugin_id)
        .ok_or_else(|| AppError::Internal("가져온 플러그인을 다시 불러오지 못했습니다".to_string()))
}
