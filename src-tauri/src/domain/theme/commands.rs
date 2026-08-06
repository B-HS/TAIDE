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
    let theme_id = {
        let settings = state.settings.read();
        if settings.follow_system_theme {
            service::builtin_id_for_system(&system_theme).to_string()
        } else {
            settings.theme_id.clone()
        }
    };
    service::load_theme(&state.paths, &theme_id)
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
