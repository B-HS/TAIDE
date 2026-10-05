use std::sync::Arc;

use serde_json::Value;
use taide_model::error::AppError;
use taide_runtime::{AppServices, ide_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &[
    "ide_get_status",
    "ide_set_selection",
    "ide_clear_selection",
    "ide_publish_diagnostics",
    "ide_resolve_diff",
    "ide_resolve_save",
    "ide_notify_at_mention",
];

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
    macro_rules! arg {
        ($key:literal) => {
            argument(args, $key).map_err(error_value)?
        };
    }
    match name {
        "ide_get_status" => respond(ide_actions::ide_get_status(&services.ide).await),
        "ide_set_selection" => {
            respond(ide_actions::ide_set_selection(&services.ide, arg!("input")).await)
        }
        "ide_clear_selection" => {
            respond(ide_actions::ide_clear_selection(&services.ide, arg!("owner")).await)
        }
        "ide_publish_diagnostics" => respond(
            ide_actions::ide_publish_diagnostics(&services.ide, arg!("projectId"), arg!("items"))
                .await,
        ),
        "ide_resolve_diff" => respond(
            ide_actions::ide_resolve_diff(
                &services.state,
                &services.ide_save_file,
                &services.ide,
                arg!("requestId"),
                arg!("outcome"),
                arg!("content"),
            )
            .await,
        ),
        "ide_resolve_save" => respond(
            ide_actions::ide_resolve_save(&services.ide, arg!("requestId"), arg!("saved")).await,
        ),
        "ide_notify_at_mention" => respond(
            ide_actions::ide_notify_at_mention(
                &services.ide,
                arg!("path"),
                arg!("lineStart"),
                arg!("lineEnd"),
            )
            .await,
        ),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote IDE routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-ide-tests.rs"]
mod tests;
