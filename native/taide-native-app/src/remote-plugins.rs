use std::sync::Arc;

use serde_json::Value;
use taide_model::error::AppError;
use taide_runtime::{AppServices, plugin_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &["plugin_list", "plugin_reload", "plugin_read_grammar"];

pub fn extend_backend(remaining: Dispatch) -> Dispatch {
    let remaining_json = remaining.json;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if COMMANDS.contains(&name.as_str()) {
                    return dispatch(&services, &name, &args).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn dispatch(services: &AppServices, name: &str, args: &Value) -> Result<String, Value> {
    let state = &services.state;
    let store = &services.plugin;
    match name {
        "plugin_list" => respond(plugin_actions::plugin_list(state, store).await),
        "plugin_reload" => respond(plugin_actions::plugin_reload(state, store).await),
        "plugin_read_grammar" => respond(
            plugin_actions::plugin_read_grammar(
                state,
                store,
                argument(args, "pluginId").map_err(error_value)?,
                argument(args, "languageId").map_err(error_value)?,
            )
            .await,
        ),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote plugin routing mismatch: {name}"
        )))),
    }
}
