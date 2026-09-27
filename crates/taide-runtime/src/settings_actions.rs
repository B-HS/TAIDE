use std::future::Future;

use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::settings::{Settings, SettingsPatch};
use taide_settings::service;

use crate::{AppState, EventSink};

pub async fn settings_get(state: &AppState) -> AppResult<Settings> {
    Ok(state.settings.read().clone())
}

pub async fn apply_and_broadcast<F, Fut>(
    state: &AppState,
    next: Settings,
    reconcile_integrations: F,
    events: &dyn EventSink,
) -> AppResult<Settings>
where
    F: FnOnce(Settings, Settings) -> Fut,
    Fut: Future<Output = ()>,
{
    let current = state.settings.read().clone();
    let updated = service::sanitize(next);
    service::save_settings(&state.paths, &updated)?;
    *state.settings.write() = updated.clone();
    reconcile_integrations(current, updated.clone()).await;
    events.publish(AppEvent::SettingsChanged {
        settings: Box::new(updated.clone()),
    });
    Ok(updated)
}

pub async fn settings_update<F, Fut>(
    state: &AppState,
    patch: SettingsPatch,
    reconcile_integrations: F,
    events: &dyn EventSink,
) -> AppResult<Settings>
where
    F: FnOnce(Settings, Settings) -> Fut,
    Fut: Future<Output = ()>,
{
    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    let updated = service::apply_patch(&current, &patch);
    apply_and_broadcast(state, updated, reconcile_integrations, events).await
}

pub async fn settings_set_theme<F, Fut>(
    state: &AppState,
    theme_id: String,
    reconcile_integrations: F,
    events: &dyn EventSink,
) -> AppResult<Settings>
where
    F: FnOnce(Settings, Settings) -> Fut,
    Fut: Future<Output = ()>,
{
    let _guard = state.begin_mutation().await;
    let current = state.settings.read().clone();
    let updated = service::set_theme(&state.paths, &current, &theme_id)?;
    let broadcasted = apply_and_broadcast(state, updated, reconcile_integrations, events).await?;
    events.publish(AppEvent::ThemeChanged {
        theme_id: broadcasted.theme_id.clone(),
    });
    Ok(broadcasted)
}
