use std::sync::Arc;

use serde_json::Value;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::terminal::PtySpawnOptions;
use taide_native_terminal::Size;
use taide_runtime::{AppServices, terminal_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::{ChannelFactory, ChannelSink, Dispatch, ResponseBody};
use crate::terminal_dispatch::ObservePorts;
use crate::terminal_environment::Environment;
use crate::terminal_host::{Hub, Session};

const REMOTE_CHANNEL_PREFIX: &str = "__CHANNEL__:";

pub const COMMANDS: &[&str] = &[
    "pty_spawn",
    "pty_attach",
    "pty_write",
    "pty_resize",
    "pty_kill",
    "pty_set_paused",
    "pty_detach",
    "terminal_sessions",
    "shell_profiles",
    "resolve_terminal_path",
    "terminal_resolve_link_candidates",
    "pty_default_options",
];

pub type History = Arc<dyn Fn(&AppServices, &PtySpawnOptions) -> usize + Send + Sync>;
pub type Effects =
    Arc<dyn Fn(&AppServices, &PtySpawnOptions) -> AppResult<ObservePorts> + Send + Sync>;

pub struct Ports {
    pub terminals: Arc<Hub>,
    pub environment: Environment,
    pub history: History,
    pub effects: Effects,
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

fn channel(args: &Value, channels: &ChannelFactory) -> Result<ChannelSink, Value> {
    let raw = args.get("onData").and_then(Value::as_str).ok_or_else(|| {
        error_value(
            AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.remote.channelArgRequired",
                "onData: a channel argument is required",
            )
            .with_arg("arg", "onData"),
        )
    })?;
    Ok(channels(
        raw.strip_prefix(REMOTE_CHANNEL_PREFIX)
            .unwrap_or(raw)
            .into(),
    ))
}

fn session(ports: &Ports, id: &str) -> AppResult<Arc<Session>> {
    ports
        .terminals
        .get(id)
        .ok_or_else(|| AppError::NotFound(format!("terminal session not found: {id}")))
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
    let state = &services.state;
    let store = &services.terminal;
    match name {
        "pty_spawn" => {
            let opts: PtySpawnOptions = arg!("opts");
            let sink = channel(args, &channels)?;
            let history = (ports.history)(&services, &opts);
            let effects = (ports.effects)(&services, &opts).map_err(error_value)?;
            respond(
                ports
                    .terminals
                    .spawn_observed_with_initial_sink(
                        opts,
                        history,
                        (ports.environment)(services.clone()),
                        effects,
                        move || drop(sink),
                    )
                    .await,
            )
        }
        "pty_attach" => {
            let id = arg!("sessionId");
            let sink = channel(args, &channels)?;
            respond(
                terminal_actions::pty_attach(state, store, id, || {
                    move |bytes: &[u8]| sink(ResponseBody::Raw(bytes.to_vec())).is_ok()
                })
                .await,
            )
        }
        "pty_write" => {
            let id: String = arg!("sessionId");
            let data = arg!("data");
            services.agents.record_input(&id);
            let result = match session(ports, &id) {
                Ok(session) => session.write_raw(&services, data).await,
                Err(error) => Err(error),
            };
            respond(result)
        }
        "pty_resize" => {
            let id: String = arg!("sessionId");
            let size = Size {
                columns: arg!("cols"),
                rows: arg!("rows"),
            };
            let result = match session(ports, &id) {
                Ok(session) => session.resize(&services, size).await.map(|_| ()),
                Err(error) => Err(error),
            };
            respond(result)
        }
        "pty_kill" => {
            let id: String = arg!("sessionId");
            let result = terminal_actions::pty_kill(state, store, id.clone()).await;
            ports.terminals.discard(&id);
            respond(result)
        }
        "pty_set_paused" => respond(
            terminal_actions::pty_set_paused(store, arg!("sessionId"), arg!("paused")).await,
        ),
        "pty_detach" => respond(
            terminal_actions::pty_detach(state, store, arg!("sessionId"), arg!("subscriptionId"))
                .await,
        ),
        "terminal_sessions" => {
            respond(terminal_actions::terminal_sessions(store, arg!("projectId")).await)
        }
        "shell_profiles" => respond(terminal_actions::shell_profiles().await),
        "resolve_terminal_path" => {
            respond(terminal_actions::resolve_terminal_path(state, arg!("path"), arg!("cwd")).await)
        }
        "terminal_resolve_link_candidates" => respond(
            terminal_actions::terminal_resolve_link_candidates(
                state,
                arg!("cwd"),
                arg!("candidates"),
            )
            .await,
        ),
        "pty_default_options" => respond(
            terminal_actions::pty_default_options(state, arg!("projectId"), arg!("cwd")).await,
        ),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote terminal routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-terminal-tests.rs"]
mod tests;
