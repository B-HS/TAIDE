use tauri_specta::Event;

use super::service::{self, SettingsPatch};
use super::types::Settings;
use crate::error::AppResult;
use crate::events::ThemeChanged;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn settings_get(state: tauri::State<'_, AppState>) -> AppResult<Settings> {
    Ok(state.settings.read().clone())
}

#[tauri::command]
#[specta::specta]
pub async fn settings_update(state: tauri::State<'_, AppState>, patch: SettingsPatch) -> AppResult<Settings> {
    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    let updated = service::apply_patch(&current, &patch);
    service::save_settings(&state.paths, &updated)?;
    *state.settings.write() = updated.clone();
    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn settings_set_theme(app: tauri::AppHandle, state: tauri::State<'_, AppState>, theme_id: String) -> AppResult<Settings> {
    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    let updated = service::set_theme(&state.paths, &current, &theme_id)?;
    *state.settings.write() = updated.clone();

    let _ = ThemeChanged {
        theme_id: updated.theme_id.clone(),
    }
    .emit(&app);

    Ok(updated)
}
