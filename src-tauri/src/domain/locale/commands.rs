use taide_runtime::locale_actions;

use super::types::{LocaleSummary, ResolvedLocale};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn locale_list(state: tauri::State<'_, AppState>) -> AppResult<Vec<LocaleSummary>> {
    locale_actions::locale_list(&state)
}

#[tauri::command]
#[specta::specta]
pub async fn locale_get(state: tauri::State<'_, AppState>, locale_id: String) -> AppResult<ResolvedLocale> {
    locale_actions::locale_get(&state, locale_id)
}

#[tauri::command]
#[specta::specta]
pub async fn locale_get_current(state: tauri::State<'_, AppState>, system_language: String) -> AppResult<ResolvedLocale> {
    locale_actions::locale_get_current(&state, &system_language)
}
