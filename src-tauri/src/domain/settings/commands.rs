use std::future::Future;
use std::pin::Pin;

use tauri::Manager;
use tauri_specta::Event;

use super::service;
use super::types::{Settings, SettingsPatch};
use crate::error::AppResult;
use crate::events::{SettingsChanged, ThemeChanged};
use crate::state::AppState;

pub type SettingsToggleFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
pub type SettingsToggleObserver =
    Box<dyn for<'a> Fn(&'a tauri::AppHandle, &'a Settings, &'a Settings) -> SettingsToggleFuture<'a> + Send + Sync>;

/// The integration reactions [`apply_and_broadcast`] runs between applying a new `Settings` value
/// and emitting `SettingsChanged` — each observer compares the previous and next value and
/// reconciles its own domain's resources (IDE server, agent hooks, remote-access server).
/// `lib.rs`'s assembly registers the concrete observers (`settings_toggle_observers`) so this
/// domain never calls into ide/agent/remote directly (audit R5#6, T1-I §1.4 — same assembly-owned
/// wiring as `ProjectCapabilities`). Observers run **in registration order, awaited inline**,
/// preserving the exact timing the old hand-coded `apply_integration_toggles` had: every reaction
/// completes before `SettingsChanged` is emitted and before the settings command returns.
pub struct SettingsToggleObservers(Vec<SettingsToggleObserver>);

impl SettingsToggleObservers {
    pub fn new(observers: Vec<SettingsToggleObserver>) -> Self {
        Self(observers)
    }

    pub async fn apply(&self, app: &tauri::AppHandle, current: &Settings, updated: &Settings) {
        for observer in &self.0 {
            observer(app, current, updated).await;
        }
    }
}

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
    app.state::<SettingsToggleObservers>().apply(app, &current, &updated).await;

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

/// Unlike `settings_update`, this also emits `ThemeChanged` — the narrower event
/// `ipc-sync-provider.tsx` listens for to invalidate `THEME.ALL` and actually refetch the newly
/// picked theme's colors, which a `SettingsChanged` alone (below, via `apply_and_broadcast`)
/// doesn't trigger.
#[tauri::command]
#[specta::specta]
pub async fn settings_set_theme(app: tauri::AppHandle, state: tauri::State<'_, AppState>, theme_id: String) -> AppResult<Settings> {
    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    let updated = service::set_theme(&state.paths, &current, &theme_id)?;
    let broadcasted = apply_and_broadcast(&app, &state, updated).await?;

    let _ = ThemeChanged {
        theme_id: broadcasted.theme_id.clone(),
    }
    .emit(&app);

    Ok(broadcasted)
}
