use std::path::{Path, PathBuf};

use taide_infra::external_url::validate_external_url;
use taide_infra::root_guard;
use taide_model::error::AppResult;
use taide_model::system::AppDataPathKind;
use taide_system::service::file_url;

use crate::{AppState, PlatformServices};

fn resolve_within_open_project(state: &AppState, path: &str) -> AppResult<PathBuf> {
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(path))?;
    Ok(resolved)
}

pub fn system_open_path(state: &AppState, platform: &dyn PlatformServices, path: &str) -> AppResult<()> {
    let resolved = resolve_within_open_project(state, path)?;
    platform.open_path(&resolved)
}

pub fn system_reveal_path(state: &AppState, platform: &dyn PlatformServices, path: &str) -> AppResult<()> {
    let resolved = resolve_within_open_project(state, path)?;
    platform.reveal_item_in_dir(&resolved)
}

pub fn system_open_in_browser(state: &AppState, platform: &dyn PlatformServices, path: &str) -> AppResult<()> {
    let resolved = resolve_within_open_project(state, path)?;
    platform.open_url(&file_url(&resolved))
}

pub fn system_open_external_url(platform: &dyn PlatformServices, url: &str) -> AppResult<()> {
    let validated = validate_external_url(url)?;
    platform.open_url(&validated)
}

pub fn system_open_app_data_path(state: &AppState, platform: &dyn PlatformServices, kind: AppDataPathKind) -> AppResult<()> {
    let dir = match kind {
        AppDataPathKind::Plugins => state.paths.plugins_dir(),
        AppDataPathKind::Themes => state.paths.themes_dir(),
        AppDataPathKind::Locales => state.paths.locales_dir(),
        AppDataPathKind::Snippets => state.paths.snippets_dir(),
    };
    std::fs::create_dir_all(&dir)?;
    platform.reveal_item_in_dir(&dir)
}
