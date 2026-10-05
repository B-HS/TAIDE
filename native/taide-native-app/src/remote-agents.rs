use std::sync::Arc;

use futures_util::future::BoxFuture;
use serde_json::Value;
use taide_agent::service::{DetectedAgentProbe, HookEmitter};
use taide_model::agent::CliInstallStatus;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_runtime::{AppServices, agent_actions, agent_hook_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &[
    "agent_list",
    "agent_release_marker",
    "agent_cli_status",
    "agent_hooks_status",
    "agent_hooks_install",
    "agent_hooks_uninstall",
];

pub type ForegroundPids = Arc<dyn Fn(&AppServices, &ProjectId) -> Vec<(String, u32)> + Send + Sync>;
pub type Probe = Arc<
    dyn Fn(
            Arc<AppServices>,
            Vec<(String, u32)>,
        ) -> BoxFuture<'static, AppResult<Vec<DetectedAgentProbe>>>
        + Send
        + Sync,
>;
pub type ProjectEmitter =
    Arc<dyn Fn(Arc<AppServices>) -> BoxFuture<'static, HookEmitter> + Send + Sync>;

pub struct Ports {
    pub foreground_pids: ForegroundPids,
    pub probe: Probe,
    pub cli_status: Arc<dyn Fn() -> CliInstallStatus + Send + Sync>,
    pub resolve_home: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    pub project_emitter: ProjectEmitter,
    pub cli_target: String,
}

impl Ports {
    pub fn new() -> Self {
        Self {
            foreground_pids: Arc::new(|services, project| {
                services.terminal.foreground_pids(project)
            }),
            probe: Arc::new(|services, pids| {
                Box::pin(async move {
                    taide_runtime::agent_host::detect_agents_for_pids_blocking(
                        &services.tasks,
                        &services.agents,
                        pids,
                    )
                    .await
                })
            }),
            cli_status: Arc::new(|| {
                taide_runtime::agent_host::cli_install_status(std::path::Path::new(
                    crate::agent_hooks::TAIDE_CLI_TARGET_PATH,
                ))
            }),
            resolve_home: Arc::new(taide_infra::home::home_dir_env),
            project_emitter: Arc::new(|services| {
                Box::pin(async move {
                    crate::agent_hooks::resolve_claude_hook_emitter(&services.tasks).await
                })
            }),
            cli_target: crate::agent_hooks::TAIDE_CLI_TARGET_PATH.into(),
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
                    return dispatch(services, &ports, &name, &args).await;
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
) -> Result<String, Value> {
    macro_rules! arg {
        ($key:literal) => {
            argument(args, $key).map_err(error_value)?
        };
    }
    match name {
        "agent_list" => {
            let project: ProjectId = arg!("projectId");
            respond(
                agent_actions::agent_list(
                    &services.state,
                    &services.agents,
                    &services.agent_hooks,
                    &services.tasks,
                    || (ports.foreground_pids)(&services, &project),
                    |pids| (ports.probe)(services.clone(), pids),
                    project.clone(),
                )
                .await,
            )
        }
        "agent_release_marker" => respond(
            agent_actions::agent_release_marker(
                &services.state,
                &services.agents,
                &services.tasks,
                arg!("marker"),
            )
            .await,
        ),
        "agent_cli_status" => respond(Ok((ports.cli_status)())),
        "agent_hooks_status" => respond(
            agent_hook_actions::agent_hooks_status(
                &services.state,
                &services.tasks,
                arg!("projectId"),
                arg!("agentName"),
                || (ports.resolve_home)(),
            )
            .await,
        ),
        "agent_hooks_install" => respond(
            agent_hook_actions::agent_hooks_install(
                &services.state,
                &services.tasks,
                arg!("projectId"),
                arg!("agentName"),
                agent_hook_actions::AgentHookInstallPorts::new(
                    || (ports.resolve_home)(),
                    || (ports.project_emitter)(services.clone()),
                    || (ports.cli_status)().installed,
                    || crate::agent_hooks::ensure_started(services.clone()),
                    &ports.cli_target,
                ),
            )
            .await,
        ),
        "agent_hooks_uninstall" => respond(
            agent_hook_actions::agent_hooks_uninstall(
                &services.state,
                &services.tasks,
                arg!("projectId"),
                arg!("agentName"),
                || (ports.resolve_home)(),
            )
            .await,
        ),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote agent routing mismatch: {name}"
        )))),
    }
}

#[cfg(test)]
#[path = "remote-agents-tests.rs"]
mod tests;
