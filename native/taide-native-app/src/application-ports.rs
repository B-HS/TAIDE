use std::sync::Arc;

use taide_model::app::AppInfo;
use taide_runtime::AppServices;
use taide_runtime::sync_gist_http::SyncGistHttpPort;

use crate::event_relay::GitEvents;
use crate::remote_preferences::Reconcile;
use crate::remote_serving::AssetResolver;
use crate::remote_ws::Dispatch;

pub struct Ports {
    pub ide: Arc<crate::ide_server::Ports>,
    pub remote: Arc<crate::remote_http::Ports>,
    pub dispatch: Arc<Dispatch>,
    pub reconcile: Reconcile,
}

impl Ports {
    pub fn new(
        services: &AppServices,
        git_events: Arc<GitEvents>,
        terminal: crate::remote_terminal::Ports,
        assets: AssetResolver,
        info: AppInfo,
    ) -> Self {
        Self::build(services, git_events, terminal, assets, info, None)
    }

    pub fn with_loading_assets(
        services: &AppServices,
        git_events: Arc<GitEvents>,
        terminal: crate::remote_terminal::Ports,
        assets: Arc<crate::remote_assets::Catalog>,
        info: AppInfo,
    ) -> Self {
        Self::build(
            services,
            git_events,
            terminal,
            assets.resolver(),
            info,
            Some(assets),
        )
    }

    fn build(
        services: &AppServices,
        git_events: Arc<GitEvents>,
        terminal: crate::remote_terminal::Ports,
        assets: AssetResolver,
        info: AppInfo,
        loading_assets: Option<Arc<crate::remote_assets::Catalog>>,
    ) -> Self {
        let ide = Arc::new(crate::ide_server::Ports::new(
            crate::ide_tools::LayoutActions::new(terminal.terminals.clone()),
            info.version.clone(),
        ));
        let mut connections = None;
        let remote = Arc::new_cyclic(|weak_remote| {
            let reconcile = Arc::new(crate::settings_integrations::Integrations::with_assets(
                ide.clone(),
                weak_remote.clone(),
                loading_assets,
            ))
            .reconcile();
            let dispatch = Arc::new(crate::remote_dispatch::create_dispatch(
                crate::remote_dispatch::Ports {
                    preferences: crate::remote_preferences::Ports {
                        info,
                        reconcile: reconcile.clone(),
                    },
                    agents: crate::remote_agents::Ports::new(),
                    git: crate::remote_git::Ports::new(git_events),
                    lsp: crate::remote_lsp::Ports::new(),
                    terminal,
                    utilities: crate::remote_utilities::Ports::new(
                        services,
                        crate::remote_utilities::domain_label_providers(),
                    ),
                    create_gist_client: Arc::new(SyncGistHttpPort::default),
                },
                crate::projects::NativeProjects::new,
            ));
            let socket = crate::remote_ws::socket_action(dispatch.clone());
            connections = Some((reconcile, dispatch));
            crate::remote_http::Ports { socket, assets }
        });
        let (reconcile, dispatch) = connections.expect("remote ports constructor initialized");
        Self {
            ide,
            remote,
            dispatch,
            reconcile,
        }
    }

    pub async fn start(&self, services: Arc<AppServices>) {
        let settings = services.state.settings.read().clone();
        let mut current = settings.clone();
        current.ide_integration_enabled = false;
        current.agent_hooks_enabled = false;
        current.remote_access_enabled = false;
        (self.reconcile)(services, current, settings).await;
    }

    pub fn stop(&self, services: &AppServices) {
        Self::stop_services(services);
    }

    pub(crate) fn stop_services(services: &AppServices) {
        crate::remote_http::stop(services);
        crate::agent_hooks::stop(services);
        crate::ide_server::stop(services);
    }
}

#[cfg(test)]
#[path = "application-ports-tests.rs"]
mod tests;
