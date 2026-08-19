use std::path::Path;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::service::{self, PluginStore};
use super::types::LoadedPlugin;

#[tauri::command]
#[specta::specta]
pub async fn plugin_list(state: State<'_, AppState>, store: State<'_, PluginStore>) -> AppResult<Vec<LoadedPlugin>> {
    Ok(service::ensure_loaded(&store, &state.paths.plugins_dir()))
}

#[tauri::command]
#[specta::specta]
pub async fn plugin_reload(state: State<'_, AppState>, store: State<'_, PluginStore>) -> AppResult<Vec<LoadedPlugin>> {
    let _guard = state.begin_mutation().await;
    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    Ok(loaded)
}

#[tauri::command]
#[specta::specta]
pub async fn plugin_install(state: State<'_, AppState>, store: State<'_, PluginStore>, source_path: String) -> AppResult<LoadedPlugin> {
    let _guard = state.begin_mutation().await;
    let source = Path::new(&source_path);
    let plugin_id = if source.is_dir() {
        service::install_from_directory(&state.paths.plugins_dir(), source)?
    } else {
        service::install_from_archive(&state.paths.plugins_dir(), source)?
    };

    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    loaded
        .into_iter()
        .find(|plugin| plugin.manifest.id == plugin_id)
        .ok_or_else(|| AppError::Internal("설치한 플러그인을 다시 불러오지 못했습니다".to_string()))
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
    let _guard = state.begin_mutation().await;
    service::uninstall(&state.paths.plugins_dir(), &plugin_id)?;

    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    Ok(loaded)
}

#[tauri::command]
#[specta::specta]
pub async fn plugin_read_grammar(
    state: State<'_, AppState>,
    store: State<'_, PluginStore>,
    plugin_id: String,
    language_id: String,
) -> AppResult<String> {
    let loaded = service::ensure_loaded(&store, &state.paths.plugins_dir());
    service::read_grammar(&loaded, &plugin_id, &language_id)
}
