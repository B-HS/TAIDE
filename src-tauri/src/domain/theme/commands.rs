use taide_runtime::theme_actions;

use super::service;
use super::types::{ResolvedTheme, Theme, ThemeSummary};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn theme_list(state: tauri::State<'_, AppState>) -> AppResult<Vec<ThemeSummary>> {
    Ok(service::list_themes(&state.paths))
}

#[tauri::command]
#[specta::specta]
pub async fn theme_get(state: tauri::State<'_, AppState>, theme_id: String) -> AppResult<ResolvedTheme> {
    service::load_theme(&state.paths, &theme_id)
}

#[tauri::command]
#[specta::specta]
pub async fn theme_get_current(state: tauri::State<'_, AppState>, system_theme: String) -> AppResult<ResolvedTheme> {
    theme_actions::theme_get_current(&state, &system_theme)
}

#[tauri::command]
#[specta::specta]
pub async fn theme_save(state: tauri::State<'_, AppState>, theme: Theme) -> AppResult<ThemeSummary> {
    service::save_theme(&state.paths, &theme)
}

#[tauri::command]
#[specta::specta]
pub async fn theme_delete(state: tauri::State<'_, AppState>, theme_id: String) -> AppResult<()> {
    service::delete_theme(&state.paths, &theme_id)
}
