use tauri::Manager;
use tauri_specta::Event;

use super::service::{self, SettingsPatch};
use super::types::Settings;
use crate::domain::agent::hooks;
use crate::domain::ide::commands as ide_commands;
use crate::domain::ide::commands::IdeStore;
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
pub async fn settings_update(app: tauri::AppHandle, state: tauri::State<'_, AppState>, patch: SettingsPatch) -> AppResult<Settings> {
    let updated = {
        let _guard = state.begin_mutation().await;
        let current = state.settings.read().clone();
        let updated = service::apply_patch(&current, &patch);
        service::save_settings(&state.paths, &updated)?;
        *state.settings.write() = updated.clone();
        apply_integration_toggles(&app, &current, &updated).await;
        updated
    };
    Ok(updated)
}

async fn apply_integration_toggles(app: &tauri::AppHandle, current: &Settings, updated: &Settings) {
    if current.ide_integration_enabled != updated.ide_integration_enabled {
        let ide = app.state::<IdeStore>();
        if updated.ide_integration_enabled {
            if let Err(error) = ide_commands::ide_start(app.clone(), app.state::<AppState>(), ide).await {
                log::warn!("IDE 연동 시작 실패: {error}");
            }
        } else {
            ide_commands::stop_server(app, &ide);
        }
    }

    if current.agent_hooks_enabled != updated.agent_hooks_enabled {
        if updated.agent_hooks_enabled {
            hooks::reconcile_installed_hooks(app).await;
        } else {
            hooks::uninstall_hooks_from_open_projects(app).await;
            hooks::stop_hooks_server(app);
        }
    }
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
