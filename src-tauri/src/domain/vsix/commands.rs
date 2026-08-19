use std::path::{Path, PathBuf};

use tauri::State;

use super::service;
use super::types::VsixThemeExtractionResult;
use crate::domain::plugin::service as plugin_service;
use crate::domain::plugin::service::PluginStore;
use crate::domain::plugin::types::LoadedPlugin;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn vsix_extract_themes(vsix_path: String) -> AppResult<VsixThemeExtractionResult> {
    service::extract_themes(Path::new(&vsix_path))
}

/// Imports a real VS Code `.vsix`'s language/grammar contributions as a new TAIDE plugin. The
/// heavy half — reading the archive and writing the staged plugin into a unique `.tmp` dir
/// (`service::stage_vsix_import`) — runs on a blocking thread **before**
/// `AppState::begin_mutation` is taken (audit R7#10, C11 axis A: the old body held the guard for
/// the whole import). What the guard actually protected is preserved in the second half, still
/// under it: the authoritative already-installed check plus the atomic rename
/// (`plugin_service::commit_staged_install`) and the plugin-list reload stay serialized with every
/// other guarded plugin mutation, the same shape `plugin_install` uses, so the frontend gets the
/// freshly-installed plugin's enabled/error state immediately.
#[tauri::command]
#[specta::specta]
pub async fn vsix_import_plugin(state: State<'_, AppState>, store: State<'_, PluginStore>, vsix_path: String) -> AppResult<LoadedPlugin> {
    let plugins_dir = state.paths.plugins_dir();
    let vsix_path = PathBuf::from(&vsix_path);
    let (temp_dir, staged_plugin_id) = tauri::async_runtime::spawn_blocking(move || service::stage_vsix_import(&vsix_path, &plugins_dir))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;

    let _guard = state.begin_mutation().await;
    let plugin_id = plugin_service::commit_staged_install(&state.paths.plugins_dir(), &temp_dir, &staged_plugin_id)?;

    let loaded = plugin_service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    loaded
        .into_iter()
        .find(|plugin| plugin.manifest.id == plugin_id)
        .ok_or_else(|| AppError::Internal("가져온 플러그인을 다시 불러오지 못했습니다".to_string()))
}
