use std::future::Future;

use taide_app::service;
use taide_model::app::AppFileTarget;
use taide_model::error::AppResult;
use taide_model::settings::Settings;

use crate::AppState;

pub async fn app_file_read(state: &AppState, target: AppFileTarget) -> AppResult<String> {
    let current_settings = state.settings.read().clone();
    service::read_app_file(&state.paths, target, &current_settings)
}

pub async fn app_file_write<F, Fut>(state: &AppState, target: AppFileTarget, content: String, apply_settings: F) -> AppResult<()>
where
    F: FnOnce(Settings) -> Fut,
    Fut: Future<Output = AppResult<Settings>>,
{
    let _guard = state.begin_mutation().await;
    app_file_write_admitted(state, target, content, apply_settings).await
}

pub async fn app_file_write_admitted<F, Fut>(state: &AppState, target: AppFileTarget, content: String, apply_settings: F) -> AppResult<()>
where
    F: FnOnce(Settings) -> Fut,
    Fut: Future<Output = AppResult<Settings>>,
{
    match target {
        AppFileTarget::Settings => {
            let parsed = taide_settings::service::parse_settings_json(&content)?;
            apply_settings(parsed).await?;
        }
        AppFileTarget::Prompt { id } => {
            service::write_prompt_file(&state.paths, id, &content)?;
        }
    }
    Ok(())
}

pub async fn apply_settings_file<F, Fut>(state: &AppState, settings: Settings, apply_settings: F) -> AppResult<()>
where
    F: FnOnce(Settings) -> Fut,
    Fut: Future<Output = AppResult<Settings>>,
{
    let _guard = state.begin_mutation().await;
    apply_settings(settings).await?;
    Ok(())
}
