use std::future::Future;

use taide_agent::constants::AGENT_NAME_CLAUDE;
use taide_agent::store::HooksServerInfo;
use taide_agent::{hook_files, service};
use taide_model::agent::{AgentHooksStatus, HookInstallScope};
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;

use crate::AppState;

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<String> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| project.root.clone())
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

/// Supplies lazy host capabilities for the existing project and user hook strategies.
pub struct AgentHookInstallPorts<'a, H, E, C, S> {
    resolve_home: H,
    resolve_project_emitter: E,
    is_cli_installed: C,
    start_server: S,
    cli_target_path: &'a str,
}

impl<'a, H, E, C, S> AgentHookInstallPorts<'a, H, E, C, S> {
    /// Defers native work until the matching scope, settings and file gates have passed.
    pub fn new(resolve_home: H, resolve_project_emitter: E, is_cli_installed: C, start_server: S, cli_target_path: &'a str) -> Self {
        Self {
            resolve_home,
            resolve_project_emitter,
            is_cli_installed,
            start_server,
            cli_target_path,
        }
    }
}

/// Reports installed hooks using the agent's existing scope and ownership rules.
pub async fn agent_hooks_status(
    state: &AppState,
    project_id: ProjectId,
    agent_name: String,
    resolve_home: impl FnOnce() -> Option<String>,
) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    let installed = match scope {
        HookInstallScope::Project => {
            let root = project_root(state, &project_id)?;
            let value = hook_files::read_settings_local(&root)?;
            service::has_taide_agent_hook_entries(AGENT_NAME_CLAUDE, &value)
        }
        HookInstallScope::User => {
            let home = resolve_home();
            let path = service::user_level_hooks_path(&agent_name, home.as_deref())?;
            if service::hook_install_shape(&agent_name)? == service::HookInstallShape::OwnedFile {
                hook_files::read_owned_hook_file(&path)?.is_some_and(|source| service::is_owned_hook_file(&source))
            } else {
                let value = hook_files::read_user_level_hooks(&path)?;
                service::has_taide_marker_anywhere(&value)
            }
        }
    };
    Ok(AgentHooksStatus {
        requires_taide_cli: service::requires_taide_cli(&agent_name),
        agent_name,
        scope,
        installed,
    })
}

/// Installs only the selected scope and shape while preserving unrelated rows and files.
pub async fn agent_hooks_install<H, E, C, S, EF, SF>(
    state: &AppState,
    project_id: ProjectId,
    agent_name: String,
    ports: AgentHookInstallPorts<'_, H, E, C, S>,
) -> AppResult<AgentHooksStatus>
where
    H: FnOnce() -> Option<String>,
    E: FnOnce() -> EF,
    C: FnOnce() -> bool,
    S: FnOnce() -> SF,
    EF: Future<Output = service::HookEmitter>,
    SF: Future<Output = AppResult<HooksServerInfo>>,
{
    let scope = service::hook_scope_for_agent(&agent_name)?;
    if !state.settings.read().agent_hooks_enabled {
        return Err(AppError::InvalidArgument("agent hooks are disabled in settings".to_string()));
    }
    match scope {
        HookInstallScope::Project => {
            let root = project_root(state, &project_id)?;
            let emitter = (ports.resolve_project_emitter)().await;
            let value = hook_files::read_settings_local(&root)?;
            let value = service::inject_taide_agent_hook_entries(&agent_name, value, emitter);
            hook_files::write_settings_local(&root, &value)?;
        }
        HookInstallScope::User => install_user_level_hooks(&agent_name, ports).await?,
    }
    Ok(AgentHooksStatus {
        requires_taide_cli: service::requires_taide_cli(&agent_name),
        agent_name,
        scope,
        installed: true,
    })
}

async fn install_user_level_hooks<H, E, C, S, SF>(agent_name: &str, ports: AgentHookInstallPorts<'_, H, E, C, S>) -> AppResult<()>
where
    H: FnOnce() -> Option<String>,
    C: FnOnce() -> bool,
    S: FnOnce() -> SF,
    SF: Future<Output = AppResult<HooksServerInfo>>,
{
    let home = (ports.resolve_home)();
    let path = service::user_level_hooks_path(agent_name, home.as_deref())?;
    if service::hook_install_shape(agent_name)? == service::HookInstallShape::OwnedFile {
        let source = service::build_owned_hook_file_source(agent_name)
            .ok_or_else(|| AppError::Internal(format!("agent has no plugin source: {agent_name}")))?;
        if hook_files::read_owned_hook_file(&path)?.is_some_and(|existing| !service::is_owned_hook_file(&existing)) {
            return Err(AppError::InvalidArgument(format!("{} is not a TAIDE-managed file", path.display())));
        }
        return hook_files::write_owned_hook_file(&path, &source);
    }
    let value = hook_files::read_user_level_hooks(&path)?;
    let value = if service::uses_project_hook_override(agent_name) {
        if !(ports.is_cli_installed)() {
            return Err(AppError::InvalidArgument("taide CLI is not installed yet".to_string()));
        }
        let server = (ports.start_server)().await?;
        let hook_url = service::build_hook_url(&server, agent_name);
        let command = service::build_command_hook_shell_command(ports.cli_target_path, &hook_url);
        let events = service::managed_hook_events_for(agent_name);
        let timeout = service::user_level_hook_command_timeout(agent_name);
        service::inject_taide_command_hook_entries(value, events, &command, timeout)
    } else {
        service::inject_taide_agent_hook_entries(agent_name, value, service::USER_LEVEL_IN_BAND_EMITTER)
    };
    hook_files::write_user_level_hooks(&path, &value)
}

/// Removes only TAIDE-owned rows or files using the existing scope policy.
pub async fn agent_hooks_uninstall(
    state: &AppState,
    project_id: ProjectId,
    agent_name: String,
    resolve_home: impl FnOnce() -> Option<String>,
) -> AppResult<AgentHooksStatus> {
    let scope = service::hook_scope_for_agent(&agent_name)?;
    match scope {
        HookInstallScope::Project => {
            let root = project_root(state, &project_id)?;
            let value = hook_files::read_settings_local(&root)?;
            if service::has_taide_agent_hook_entries(&agent_name, &value) {
                let value = service::remove_taide_agent_hook_entries(&agent_name, value);
                hook_files::write_settings_local(&root, &value)?;
            }
        }
        HookInstallScope::User => {
            let home = resolve_home();
            let path = service::user_level_hooks_path(&agent_name, home.as_deref())?;
            if service::hook_install_shape(&agent_name)? == service::HookInstallShape::OwnedFile {
                hook_files::remove_owned_hook_file(&path)?;
            } else {
                let value = hook_files::read_user_level_hooks(&path)?;
                if service::has_taide_marker_anywhere(&value) {
                    let value = service::remove_taide_agent_hook_entries(&agent_name, value);
                    hook_files::write_user_level_hooks(&path, &value)?;
                }
            }
        }
    }
    Ok(AgentHooksStatus {
        requires_taide_cli: service::requires_taide_cli(&agent_name),
        agent_name,
        scope,
        installed: false,
    })
}
