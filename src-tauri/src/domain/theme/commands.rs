use taide_runtime::theme_actions;

use super::types::{ResolvedTheme, Theme, ThemeSummary};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn theme_list(state: tauri::State<'_, AppState>) -> AppResult<Vec<ThemeSummary>> {
    theme_actions::theme_list(&state)
}

#[tauri::command]
#[specta::specta]
pub async fn theme_get(state: tauri::State<'_, AppState>, theme_id: String) -> AppResult<ResolvedTheme> {
    theme_actions::theme_get(&state, theme_id)
}

#[tauri::command]
#[specta::specta]
pub async fn theme_get_current(state: tauri::State<'_, AppState>, system_theme: String) -> AppResult<ResolvedTheme> {
    theme_actions::theme_get_current(&state, &system_theme)
}

#[tauri::command]
#[specta::specta]
pub async fn theme_save(state: tauri::State<'_, AppState>, theme: Theme) -> AppResult<ThemeSummary> {
    theme_actions::theme_save(&state, theme)
}

#[tauri::command]
#[specta::specta]
pub async fn theme_delete(state: tauri::State<'_, AppState>, theme_id: String) -> AppResult<()> {
    theme_actions::theme_delete(&state, theme_id)
}
