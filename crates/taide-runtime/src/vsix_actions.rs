use std::path::{Path, PathBuf};

use taide_model::error::{AppError, AppResult};
use taide_model::plugin::LoadedPlugin;
use taide_model::vsix::VsixThemeExtractionResult;
use taide_vsix::service;

use crate::AppState;

/// Applies the shared vsix extract themes policy.
pub async fn vsix_extract_themes(vsix_path: String) -> AppResult<VsixThemeExtractionResult> {
    service::extract_themes(Path::new(&vsix_path))
}

/// Applies the shared vsix import plugin policy.
pub async fn vsix_import_plugin(
    state: &AppState,
    commit_staged_import: impl FnOnce(&Path, &str) -> AppResult<LoadedPlugin>,
    vsix_path: String,
) -> AppResult<LoadedPlugin> {
    let plugins_dir = state.paths.plugins_dir();
    let vsix_path = PathBuf::from(&vsix_path);
    let (temp_dir, staged_plugin_id) = tokio::task::spawn_blocking(move || service::stage_vsix_import(&plugins_dir, &vsix_path))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))??;

    let _guard = state.begin_mutation().await;
    commit_staged_import(&temp_dir, &staged_plugin_id)
}
