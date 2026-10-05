use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;
use taide_model::agent::HookInstallScope;
use taide_model::app::AppFileTarget;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::settings::SettingsPatch;
use taide_remote::command_policy::{self, RemoteDenialPolicy};
use taide_remote::policy::{
    enforce_remote_owner_label, strip_remote_gated_settings, strip_remote_gated_settings_patch,
};
use taide_runtime::AppServices;

use crate::remote_ws::Dispatch;

pub fn with_policy(backend: Dispatch) -> Dispatch {
    let backend = Arc::new(backend);
    let json_backend = backend.clone();
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let backend = json_backend.clone();
            Box::pin(async move {
                let args = prepare_json(&services, &name, args).map_err(error_value)?;
                (backend.json)(services, name, args, channels).await
            })
        }),
        raw: Arc::new(move |services, name, args| {
            let backend = backend.clone();
            Box::pin(async move {
                command_policy::admit(&name).map_err(error_value)?;
                if name != "file_read_raw" {
                    return Err(error_value(
                        RemoteDenialPolicy::Unclassified.denial_error(&name),
                    ));
                }
                (backend.raw)(services, name, enforce_remote_owner_label(args)).await
            })
        }),
    }
}

pub(crate) fn error_value(error: AppError) -> Value {
    serde_json::to_value(error)
        .unwrap_or_else(|_| serde_json::json!({"code":"Internal","message":"직렬화 실패"}))
}

pub(crate) fn respond<T: serde::Serialize>(result: AppResult<T>) -> Result<String, Value> {
    let value = result.map_err(error_value)?;
    serde_json::to_string(&value)
        .map_err(|error| error_value(AppError::Internal(error.to_string())))
}

pub(crate) fn argument<T: DeserializeOwned>(args: &Value, key: &str) -> AppResult<T> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|error| AppError::InvalidArgument(format!("{key}: {error}")))
}

fn set_argument<T: serde::Serialize>(args: &mut Value, key: &str, value: T) -> AppResult<()> {
    let value =
        serde_json::to_value(value).map_err(|error| AppError::Internal(error.to_string()))?;
    let Some(fields) = args.as_object_mut() else {
        return Err(AppError::InvalidArgument(
            "remote args must be an object".into(),
        ));
    };
    fields.insert(key.into(), value);
    Ok(())
}

fn prepare_json(services: &AppServices, name: &str, args: Value) -> AppResult<Value> {
    command_policy::admit(name)?;
    if name == "file_read_raw" {
        return Err(RemoteDenialPolicy::Unclassified.denial_error(name));
    }
    let mut args = enforce_remote_owner_label(args);
    match name {
        "agent_hooks_install" => {
            let _: ProjectId = argument(&args, "projectId")?;
            let agent_name: String = argument(&args, "agentName")?;
            if matches!(
                taide_agent::service::hook_scope_for_agent(&agent_name),
                Ok(HookInstallScope::User)
            ) {
                return Err(RemoteDenialPolicy::DesktopCliInterception.denial_error(name));
            }
        }
        "settings_update" => {
            let patch: SettingsPatch = argument(&args, "patch")?;
            set_argument(&mut args, "patch", strip_remote_gated_settings_patch(patch))?;
        }
        "app_file_write" => {
            let target: AppFileTarget = argument(&args, "target")?;
            let content: String = argument(&args, "content")?;
            if target == AppFileTarget::Settings {
                let parsed = taide_settings::service::parse_settings_json(&content)?;
                let current = services.state.settings.read().clone();
                let sanitized = strip_remote_gated_settings(parsed, &current);
                let content = serde_json::to_string(&sanitized)
                    .map_err(|error| AppError::Internal(error.to_string()))?;
                set_argument(&mut args, "content", content)?;
            }
        }
        _ => {}
    }
    Ok(args)
}

#[cfg(test)]
#[path = "remote-gateway-tests.rs"]
mod tests;
