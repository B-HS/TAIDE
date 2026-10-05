use std::sync::Arc;

use serde_json::Value;
use taide_runtime::{AppServices, ai_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &[
    "ai_token_status",
    "ai_list_models",
    "ai_inline_complete",
    "ai_inline_edit",
    "ai_commit_message",
    "ai_request_cancel",
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
    let state = &services.state;
    let secret = &services.secrets;
    let requests = &services.ai_requests;
    match name {
        "ai_token_status" => respond(ai_actions::ai_token_status(state, secret).await),
        "ai_list_models" => {
            respond(ai_actions::ai_list_models(state, secret, arg!("provider")).await)
        }
        "ai_inline_complete" => {
            respond(ai_actions::ai_inline_complete(state, requests, secret, arg!("request")).await)
        }
        "ai_inline_edit" => {
            respond(ai_actions::ai_inline_edit(state, requests, secret, arg!("request")).await)
        }
        "ai_commit_message" => {
            respond(ai_actions::ai_commit_message(state, requests, secret, arg!("request")).await)
        }
        "ai_request_cancel" => {
            respond(ai_actions::ai_request_cancel(requests, arg!("owner"), arg!("requestId")).await)
        }
        _ => unreachable!("AI command catalog and dispatch arms must agree"),
    }
}

#[cfg(test)]
#[path = "remote-ai-tests.rs"]
mod tests;
