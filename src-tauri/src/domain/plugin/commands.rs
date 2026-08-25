use std::path::PathBuf;

use tauri::State;

use crate::error::{AppError, AppErrorKind, AppResult};
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
    let plugins_dir = state.paths.plugins_dir();
    let source = PathBuf::from(&source_path);
    let (temp_dir, staged_plugin_id) = tauri::async_runtime::spawn_blocking(move || {
        if source.is_dir() {
            service::stage_from_directory(&plugins_dir, &source)
        } else {
            service::stage_from_archive(&plugins_dir, &source)
        }
    })
    .await
    .map_err(|error| AppError::Internal(error.to_string()))??;

    let _guard = state.begin_mutation().await;
    let plugin_id = service::commit_staged_install(&state.paths.plugins_dir(), &temp_dir, &staged_plugin_id)?;

    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    loaded.into_iter().find(|plugin| plugin.manifest.id == plugin_id).ok_or_else(|| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.plugin.reloadAfterInstallFailed",
            "failed to reload the installed plugin",
        )
    })
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
