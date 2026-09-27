use taide_runtime::plugin_actions;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;

use super::service::PluginStore;
use super::types::LoadedPlugin;

#[tauri::command]
#[specta::specta]
pub async fn plugin_list(state: State<'_, AppState>, store: State<'_, PluginStore>) -> AppResult<Vec<LoadedPlugin>> {
    plugin_actions::plugin_list(&state, &store).await
}

#[tauri::command]
#[specta::specta]
pub async fn plugin_reload(state: State<'_, AppState>, store: State<'_, PluginStore>) -> AppResult<Vec<LoadedPlugin>> {
    plugin_actions::plugin_reload(&state, &store).await
}

/// The heavy half — up-to-128MB archive extraction or a recursive directory copy into a unique
/// `.tmp` staging dir — runs on a blocking thread **before** `AppState::begin_mutation` is taken
/// (audit R7#10, C11 axis A: the old body held the guard for the whole install, freezing every
/// other mutation for the extraction's duration). What the guard actually protected is preserved
/// in the second half, which still runs under it: the authoritative already-installed check plus
/// the atomic rename into `plugins_dir/{id}` (`service::commit_staged_install`) and the store
/// reload stay serialized with `plugin_uninstall`/`plugin_reload`/other installs, so a duplicate
/// id deterministically fails exactly as before and the store snapshot always reflects a settled
/// plugins dir. Staging itself needs no serialization — every invocation writes only its own
/// uuid-suffixed temp dir.
#[tauri::command]
#[specta::specta]
pub async fn plugin_install(state: State<'_, AppState>, store: State<'_, PluginStore>, source_path: String) -> AppResult<LoadedPlugin> {
    plugin_actions::plugin_install(&state, &store, source_path).await
}

/// No built-in-plugin protection — every entry in `plugins_dir` is a user-installed directory
/// (contract §3.4: "빌트인 보호 없음 — 사용자 디렉토리만"), since TAIDE ships no bundled plugins.
#[tauri::command]
#[specta::specta]
pub async fn plugin_uninstall(
    state: State<'_, AppState>,
    store: State<'_, PluginStore>,
    plugin_id: String,
) -> AppResult<Vec<LoadedPlugin>> {
    plugin_actions::plugin_uninstall(&state, &store, plugin_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn plugin_read_grammar(
    state: State<'_, AppState>,
    store: State<'_, PluginStore>,
    plugin_id: String,
    language_id: String,
) -> AppResult<String> {
    plugin_actions::plugin_read_grammar(&state, &store, plugin_id, language_id).await
}
