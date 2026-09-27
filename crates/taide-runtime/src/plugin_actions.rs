use std::path::{Path, PathBuf};

use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::plugin::LoadedPlugin;
use taide_plugin::service::{self, PluginStore};

use crate::AppState;

/// Applies the shared plugin list policy.
pub async fn plugin_list(state: &AppState, store: &PluginStore) -> AppResult<Vec<LoadedPlugin>> {
    Ok(service::ensure_loaded(store, &state.paths.plugins_dir()))
}

/// Applies the shared plugin reload policy.
pub async fn plugin_reload(state: &AppState, store: &PluginStore) -> AppResult<Vec<LoadedPlugin>> {
    let _guard = state.begin_mutation().await;
    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    Ok(loaded)
}

/// Applies the shared plugin install policy.
pub async fn plugin_install(state: &AppState, store: &PluginStore, source_path: String) -> AppResult<LoadedPlugin> {
    let plugins_dir = state.paths.plugins_dir();
    let source = PathBuf::from(&source_path);
    let (temp_dir, staged_plugin_id) = tokio::task::spawn_blocking(move || {
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

/// Applies the shared plugin uninstall policy.
pub async fn plugin_uninstall(state: &AppState, store: &PluginStore, plugin_id: String) -> AppResult<Vec<LoadedPlugin>> {
    let _guard = state.begin_mutation().await;
    service::uninstall(&state.paths.plugins_dir(), &plugin_id)?;

    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    Ok(loaded)
}

/// Applies the shared plugin read grammar policy.
pub async fn plugin_read_grammar(state: &AppState, store: &PluginStore, plugin_id: String, language_id: String) -> AppResult<String> {
    let loaded = service::ensure_loaded(store, &state.paths.plugins_dir());
    service::read_grammar(&loaded, &plugin_id, &language_id)
}

/// Commits a staged VSIX plugin and refreshes the shared cache under the caller's mutation guard.
pub fn commit_staged_vsix_plugin(
    state: &AppState,
    store: &PluginStore,
    temp_dir: &Path,
    staged_plugin_id: &str,
) -> AppResult<LoadedPlugin> {
    let plugin_id = service::commit_staged_install(&state.paths.plugins_dir(), temp_dir, staged_plugin_id)?;
    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    loaded.into_iter().find(|plugin| plugin.manifest.id == plugin_id).ok_or_else(|| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.vsix.reloadAfterImportFailed",
            "failed to reload the imported plugin",
        )
    })
}
