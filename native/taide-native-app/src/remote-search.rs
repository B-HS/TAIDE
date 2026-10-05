use std::sync::Arc;

use serde_json::Value;
use taide_infra::perf::{self, SpanSlot};
use taide_model::error::{AppError, AppErrorKind};
use taide_remote::types::REMOTE_CHANNEL_PREFIX;
use taide_runtime::{AppServices, search_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::{ChannelFactory, Dispatch, ResponseBody};

pub const COMMANDS: &[&str] = &[
    "search_run",
    "search_replace",
    "search_cancel",
    "search_list_files",
];

pub fn extend_backend(remaining: Dispatch) -> Dispatch {
    let remaining_json = remaining.json;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if COMMANDS.contains(&name.as_str()) {
                    return dispatch(&services, &name, &args, channels).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn dispatch(
    services: &AppServices,
    name: &str,
    args: &Value,
    channels: ChannelFactory,
) -> Result<String, Value> {
    macro_rules! arg {
        ($key:literal) => {
            argument(args, $key).map_err(error_value)?
        };
    }
    let state = &services.state;
    match name {
        "search_run" => {
            let _span = perf::span(SpanSlot::SearchRun);
            let project = arg!("projectId");
            let owner = arg!("owner");
            let session = arg!("sessionId");
            let query = arg!("query");
            let raw = args.get("onMatch").and_then(Value::as_str).ok_or_else(|| {
                error_value(
                    AppError::localized(
                        AppErrorKind::InvalidArgument,
                        "error.remote.channelArgRequired",
                        "onMatch: a channel argument is required",
                    )
                    .with_arg("arg", "onMatch"),
                )
            })?;
            let on_match = channels(
                raw.strip_prefix(REMOTE_CHANNEL_PREFIX)
                    .unwrap_or(raw)
                    .into(),
            );
            respond(
                search_actions::search_run(
                    search_actions::SearchRunContext {
                        state,
                        store: &services.search,
                        tasks: &services.tasks,
                    },
                    project,
                    owner,
                    session,
                    query,
                    move |batch| {
                        if let Ok(text) = serde_json::to_string(&batch) {
                            let _ = on_match(ResponseBody::Json(text));
                        }
                    },
                )
                .await,
            )
        }
        "search_replace" => respond(
            search_actions::search_replace(
                state,
                &services.tasks,
                arg!("projectId"),
                arg!("query"),
                arg!("replacement"),
                arg!("paths"),
            )
            .await,
        ),
        "search_cancel" => respond(
            search_actions::search_cancel(
                state,
                &services.search,
                arg!("owner"),
                arg!("sessionId"),
            )
            .await,
        ),
        "search_list_files" => {
            let _span = perf::span(SpanSlot::SearchListFiles);
            respond(
                search_actions::search_list_files(state, &services.tasks, arg!("projectId")).await,
            )
        }
        _ => Err(error_value(AppError::Internal(format!(
            "native remote search routing mismatch: {name}"
        )))),
    }
}
