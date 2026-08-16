use tauri::Manager;
use tauri_specta::Event;

use super::service::{self, SettingsPatch};
use super::types::Settings;
use crate::domain::agent::hooks;
use crate::domain::ide::commands as ide_commands;
use crate::domain::ide::commands::IdeStore;
use crate::domain::remote::commands as remote_commands;
use crate::domain::remote::commands::RemoteStore;
use crate::error::AppResult;
use crate::events::{SettingsChanged, ThemeChanged};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn settings_get(state: tauri::State<'_, AppState>) -> AppResult<Settings> {
    Ok(state.settings.read().clone())
}

/// Sanitizes, persists, applies to live `AppState`, reconciles integration toggles (IDE/agent
/// hooks/remote access) against the previous value, and broadcasts `SettingsChanged` — the single
/// reapply path every settings-writing entry point funnels through
/// (`settings_update`, `sync_download`, `app_file_write`'s `Settings` target), so a hand-edited
/// `settings.json` or a synced gist download reaches every window/remote session exactly the same
/// way a `settings_update` patch does. Callers already inside `state.begin_mutation()` must call
/// this directly rather than through a command — it does **not** take the mutation guard itself, to
/// avoid a re-entrant deadlock on `AppState`'s single global `tokio::sync::Mutex`.
pub async fn apply_and_broadcast(app: &tauri::AppHandle, state: &AppState, next: Settings) -> AppResult<Settings> {
    let current = state.settings.read().clone();
    let updated = service::sanitize(next);
    service::save_settings(&state.paths, &updated)?;
    *state.settings.write() = updated.clone();
    apply_integration_toggles(app, &current, &updated).await;

    let _ = SettingsChanged { settings: updated.clone() }.emit(app);

    Ok(updated)
}

#[tauri::command]
#[specta::specta]
pub async fn settings_update(app: tauri::AppHandle, state: tauri::State<'_, AppState>, patch: SettingsPatch) -> AppResult<Settings> {
    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    let updated = service::apply_patch(&current, &patch);
    apply_and_broadcast(&app, &state, updated).await
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

    if current.remote_access_enabled != updated.remote_access_enabled {
        let remote = app.state::<RemoteStore>();
        if updated.remote_access_enabled {
            if let Err(error) = remote_commands::remote_start(app.clone(), remote).await {
                log::warn!("원격 접속 서버 시작 실패: {error}");
            }
        } else {
            remote_commands::stop_server(app, &remote);
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
