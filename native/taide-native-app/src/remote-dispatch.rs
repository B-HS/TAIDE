use std::sync::Arc;

use taide_remote::command_policy::RemoteDenialPolicy;
use taide_runtime::AppServices;
use taide_runtime::project_actions::ProjectLifecyclePort;
use taide_runtime::sync_actions::SyncGistPort;

use crate::remote_gateway::{error_value, with_policy};
use crate::remote_ws::Dispatch;
use crate::{
    remote_agents, remote_ai, remote_files, remote_git, remote_ide, remote_layout, remote_lsp,
    remote_plugins, remote_preferences, remote_projects, remote_search, remote_sync,
    remote_terminal, remote_utilities,
};

pub struct Ports<G> {
    pub preferences: remote_preferences::Ports,
    pub agents: remote_agents::Ports,
    pub git: remote_git::Ports,
    pub lsp: remote_lsp::Ports,
    pub terminal: remote_terminal::Ports,
    pub utilities: remote_utilities::Ports,
    pub create_gist_client: Arc<dyn Fn() -> G + Send + Sync>,
}

pub fn create_dispatch<P, F, G>(ports: Ports<G>, create_lifecycle: F) -> Dispatch
where
    P: ProjectLifecyclePort + 'static,
    F: Fn(Arc<AppServices>) -> P + Send + Sync + 'static,
    G: SyncGistPort + 'static,
{
    let layout = remote_layout::Ports::new(ports.terminal.terminals.clone());
    let sync = remote_sync::Ports {
        create_client: ports.create_gist_client,
        reconcile: ports.preferences.reconcile.clone(),
    };
    let remaining = Dispatch {
        json: Arc::new(|_, name, _, _| {
            Box::pin(async move {
                Err(error_value(
                    RemoteDenialPolicy::Unclassified.denial_error(&name),
                ))
            })
        }),
        raw: Arc::new(|_, name, _| {
            Box::pin(async move {
                Err(error_value(
                    RemoteDenialPolicy::Unclassified.denial_error(&name),
                ))
            })
        }),
    };
    let backend = remote_ai::extend_backend(remaining);
    let backend = remote_sync::extend_backend(sync, backend);
    let backend = remote_utilities::extend_backend(ports.utilities, backend);
    let backend = remote_terminal::extend_backend(ports.terminal, backend);
    let backend = remote_lsp::extend_backend(ports.lsp, backend);
    let backend = remote_agents::extend_backend(ports.agents, backend);
    let backend = remote_git::extend_backend(ports.git, backend);
    let backend = remote_plugins::extend_backend(backend);
    let backend = remote_search::extend_backend(backend);
    let backend = remote_projects::extend_backend(create_lifecycle, backend);
    let backend = remote_layout::extend_backend(layout, backend);
    let backend = remote_files::extend_backend(backend);
    let backend = remote_ide::extend_backend(backend);
    with_policy(remote_preferences::extend_backend(
        ports.preferences,
        backend,
    ))
}

#[cfg(test)]
#[path = "remote-dispatch-tests.rs"]
mod tests;
