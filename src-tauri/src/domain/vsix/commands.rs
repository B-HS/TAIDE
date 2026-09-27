use taide_runtime::vsix_actions;
use tauri::{AppHandle, State};

use super::types::VsixThemeExtractionResult;
use crate::domain::plugin::types::LoadedPlugin;
use crate::error::AppResult;
use crate::plugin_port::PluginRuntimePort;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn vsix_extract_themes(vsix_path: String) -> AppResult<VsixThemeExtractionResult> {
    vsix_actions::vsix_extract_themes(vsix_path).await
}

/// Imports a real VS Code `.vsix`'s language/grammar contributions as a new TAIDE plugin. The
/// heavy half — reading the archive and writing the staged plugin into a unique `.tmp` dir
/// (`service::stage_vsix_import`) — runs on a blocking thread **before**
/// `AppState::begin_mutation` is taken (audit R7#10, C11 axis A: the old body held the guard for
/// the whole import). What the guard actually protected is preserved in the second half, still
/// under it: the authoritative already-installed check plus the atomic rename
/// (`PluginRuntimePort::commit_staged_import`) and the plugin-list reload stay serialized with every
/// other guarded plugin mutation, the same shape `plugin_install` uses, so the frontend gets the
/// freshly-installed plugin's enabled/error state immediately.
#[tauri::command]
#[specta::specta]
pub async fn vsix_import_plugin(
    app: AppHandle,
    state: State<'_, AppState>,
    plugins: State<'_, PluginRuntimePort>,
    vsix_path: String,
) -> AppResult<LoadedPlugin> {
    vsix_actions::vsix_import_plugin(
        &state,
        |temp_dir, staged_plugin_id| (plugins.commit_staged_import)(&app, temp_dir, staged_plugin_id),
        vsix_path,
    )
    .await
}
