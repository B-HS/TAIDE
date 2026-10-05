use std::ffi::OsString;
use std::sync::Arc;

use serde_json::Value;
use taide_infra::lsp_proc::LspProcHandle;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::lsp::LanguageServerSpec;
use taide_runtime::AppServices;
use taide_runtime::lsp_actions::{self, LspActionContext, LspSpawnPorts};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::{ChannelFactory, Dispatch, ResponseBody};

const REMOTE_CHANNEL_PREFIX: &str = "__CHANNEL__:";

pub const COMMANDS: &[&str] = &[
    "lsp_spawn",
    "lsp_send",
    "lsp_stop",
    "lsp_restart",
    "lsp_confirm_reinitialize",
    "lsp_report_reinitialize_failure",
    "lsp_sessions",
    "lsp_detect_servers",
    "lsp_resolve_root",
    "lsp_install_cancel",
];

pub type CreateProcess = Arc<
    dyn Fn(
            Arc<AppServices>,
            String,
            u64,
            LanguageServerSpec,
            String,
        ) -> AppResult<Arc<LspProcHandle>>
        + Send
        + Sync,
>;

pub struct Ports {
    pub create_process: CreateProcess,
    pub path_var: Arc<dyn Fn() -> OsString + Send + Sync>,
}

impl Ports {
    pub fn new() -> Self {
        Self {
            create_process: Arc::new(crate::lsp_process::create_process),
            path_var: Arc::new(|| std::env::var_os("PATH").unwrap_or_default()),
        }
    }
}

impl Default for Ports {
    fn default() -> Self {
        Self::new()
    }
}

pub fn extend_backend(ports: Ports, remaining: Dispatch) -> Dispatch {
    let ports = Arc::new(ports);
    let remaining_json = remaining.json;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let ports = ports.clone();
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if COMMANDS.contains(&name.as_str()) {
                    return dispatch(services, &ports, &name, &args, channels).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn dispatch(
    services: Arc<AppServices>,
    ports: &Ports,
    name: &str,
    args: &Value,
    channels: ChannelFactory,
) -> Result<String, Value> {
    macro_rules! arg {
        ($key:literal) => {
            argument(args, $key).map_err(error_value)?
        };
    }
    let context = LspActionContext::new(&services.state, &services.lsp, &services.tasks);
    let events = services.events.as_ref();
    match name {
        "lsp_spawn" => {
            let request = arg!("request");
            let raw = args
                .get("onMessage")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    error_value(
                        AppError::localized(
                            AppErrorKind::InvalidArgument,
                            "error.remote.channelArgRequired",
                            "onMessage: a channel argument is required",
                        )
                        .with_arg("arg", "onMessage"),
                    )
                })?;
            let channel = channels(
                raw.strip_prefix(REMOTE_CHANNEL_PREFIX)
                    .unwrap_or(raw)
                    .into(),
            );
            respond(
                lsp_actions::lsp_spawn(
                    events,
                    context,
                    request,
                    LspSpawnPorts::new(
                        || {
                            move |message: &str| {
                                let Ok(body) = serde_json::to_string(message) else {
                                    return false;
                                };
                                channel(ResponseBody::Json(body)).is_ok()
                            }
                        },
                        || format!("lsp-{}", uuid::Uuid::new_v4()),
                        |id, epoch, spec, root| {
                            (ports.create_process)(services.clone(), id, epoch, spec, root)
                        },
                    ),
                )
                .await,
            )
        }
        "lsp_send" => {
            respond(lsp_actions::lsp_send(&services.lsp, arg!("sessionId"), arg!("message")).await)
        }
        "lsp_stop" => respond(
            lsp_actions::lsp_stop(
                events,
                context,
                arg!("sessionId"),
                arg!("root"),
                arg!("owner"),
            )
            .await,
        ),
        "lsp_restart" => respond(
            lsp_actions::lsp_restart(
                events,
                context,
                arg!("sessionId"),
                |id, epoch, spec, root| {
                    (ports.create_process)(services.clone(), id, epoch, spec, root)
                },
            )
            .await,
        ),
        "lsp_confirm_reinitialize" => respond(lsp_actions::lsp_confirm_reinitialize(
            events,
            &services.lsp,
            arg!("sessionId"),
            arg!("generation"),
        )),
        "lsp_report_reinitialize_failure" => respond(lsp_actions::lsp_report_reinitialize_failure(
            events,
            &services.lsp,
            arg!("sessionId"),
            arg!("generation"),
        )),
        "lsp_sessions" => respond(lsp_actions::lsp_sessions(&services.lsp, arg!("projectId"))),
        "lsp_detect_servers" => respond(Ok(taide_lsp::service::detect_servers(
            &services.state.paths.lsp_dir(),
            &(ports.path_var)(),
        ))),
        "lsp_resolve_root" => respond(lsp_actions::lsp_resolve_root(
            arg!("serverId"),
            arg!("filePath"),
        )),
        "lsp_install_cancel" => respond(lsp_actions::lsp_install_cancel(
            &services.lsp_install,
            arg!("serverId"),
        )),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote LSP routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-lsp-tests.rs"]
mod tests;
