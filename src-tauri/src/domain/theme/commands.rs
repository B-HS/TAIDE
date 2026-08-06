use super::service;
use super::types::{ResolvedTheme, ThemeSummary};
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
pub async fn theme_get_current(state: tauri::State<'_, AppState>) -> AppResult<ResolvedTheme> {
    let theme_id = state.settings.read().theme_id.clone();
    service::load_theme(&state.paths, &theme_id)
}
