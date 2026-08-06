use super::service;
use super::types::{LocaleSummary, ResolvedLocale};
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn locale_list(state: tauri::State<'_, AppState>) -> AppResult<Vec<LocaleSummary>> {
    Ok(service::list_locales(&state.paths))
}

#[tauri::command]
#[specta::specta]
pub async fn locale_get(state: tauri::State<'_, AppState>, locale_id: String) -> AppResult<ResolvedLocale> {
    service::load_locale(&state.paths, &locale_id)
}

#[tauri::command]
#[specta::specta]
pub async fn locale_get_current(state: tauri::State<'_, AppState>, system_language: String) -> AppResult<ResolvedLocale> {
    let language = state.settings.read().language.clone();
    let locale_id = service::resolve_language(&state.paths, &language, &system_language);
    service::load_locale(&state.paths, &locale_id)
}
