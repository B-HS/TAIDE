use std::path::{Path, PathBuf};

use taide_model::error::AppResult;
use taide_model::plugin::LoadedPlugin;
use taide_model::vsix::VsixThemeExtractionResult;
use taide_vsix::service;

use crate::plugin_install_worker::{run_install, stage_plugin};
use crate::{AppState, TaskSupervisor};

/// Applies the shared vsix extract themes policy.
pub async fn vsix_extract_themes(vsix_path: String) -> AppResult<VsixThemeExtractionResult> {
    service::extract_themes(Path::new(&vsix_path))
}

/// Applies the shared vsix import plugin policy.
pub async fn vsix_import_plugin(
    state: &AppState,
    tasks: &TaskSupervisor,
    commit_staged_import: impl FnOnce(&Path, &str) -> AppResult<LoadedPlugin> + Send + 'static,
    vsix_path: String,
) -> AppResult<LoadedPlugin> {
    let state = state.clone();
    let worker_tasks = tasks.clone();
    run_install(tasks, "vsix-import", async move {
        let plugins_dir = state.paths.plugins_dir();
        let vsix_path = PathBuf::from(&vsix_path);
        let staged = stage_plugin(&worker_tasks, move || service::stage_vsix_import(&plugins_dir, &vsix_path)).await?;
        let _guard = state.begin_mutation().await;
        commit_staged_import(&staged.path, &staged.id)
    })
    .await
}
