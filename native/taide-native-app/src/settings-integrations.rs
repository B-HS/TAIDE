use std::sync::{Arc, Weak};

use futures_util::future::BoxFuture;
use taide_runtime::AppServices;

use crate::remote_preferences::Reconcile;

type Toggle = Arc<dyn Fn(Arc<AppServices>, bool, bool) -> BoxFuture<'static, ()> + Send + Sync>;

pub struct Integrations {
    ide: Toggle,
    hooks: Toggle,
    remote: Toggle,
}

impl Integrations {
    pub fn new(
        ide: Arc<crate::ide_server::Ports>,
        remote: Weak<crate::remote_http::Ports>,
    ) -> Self {
        Self::with_assets(ide, remote, None)
    }

    pub(crate) fn with_assets(
        ide: Arc<crate::ide_server::Ports>,
        remote: Weak<crate::remote_http::Ports>,
        assets: Option<Arc<crate::remote_assets::Catalog>>,
    ) -> Self {
        Self {
            ide: Arc::new(move |services, current, updated| {
                Box::pin(crate::ide_server::apply_toggle(
                    services,
                    ide.clone(),
                    current,
                    updated,
                ))
            }),
            hooks: Arc::new(|services, current, updated| {
                Box::pin(crate::agent_hooks::apply_toggle(services, current, updated))
            }),
            remote: Arc::new(move |services, current, updated| {
                let remote = remote.clone();
                let assets = assets.clone();
                Box::pin(async move {
                    if current == updated {
                        return;
                    }
                    if !updated {
                        crate::remote_http::stop(&services);
                        return;
                    }
                    let Some(ports) = remote.upgrade() else {
                        log::warn!("원격 접속 서버 시작 실패: native remote ports owner expired");
                        return;
                    };
                    if let Some(assets) = assets
                        && let Err(error) = assets.prepare(services.tasks.clone()).await
                    {
                        log::warn!(
                            "native remote assets preparation failed: {:?}",
                            error.kind()
                        );
                        return;
                    }
                    crate::remote_http::apply_toggle(services, ports, current, updated).await;
                })
            }),
        }
    }

    pub fn reconcile(self: Arc<Self>) -> Reconcile {
        Arc::new(move |services, current, updated| {
            let integrations = self.clone();
            Box::pin(async move {
                (integrations.ide)(
                    services.clone(),
                    current.ide_integration_enabled,
                    updated.ide_integration_enabled,
                )
                .await;
                (integrations.hooks)(
                    services.clone(),
                    current.agent_hooks_enabled,
                    updated.agent_hooks_enabled,
                )
                .await;
                (integrations.remote)(
                    services,
                    current.remote_access_enabled,
                    updated.remote_access_enabled,
                )
                .await;
            })
        })
    }
}

#[cfg(test)]
#[path = "settings-integrations-tests.rs"]
mod tests;
