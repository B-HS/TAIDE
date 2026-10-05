use std::sync::Arc;

use serde_json::Value;
use taide_runtime::sync_actions::SyncGistPort;
use taide_runtime::{AppServices, settings_actions, sync_actions};

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_preferences::Reconcile;
use crate::remote_ws::Dispatch;

pub const COMMANDS: &[&str] = &["sync_status", "sync_upload", "sync_download"];

pub struct Ports<G> {
    pub create_client: Arc<dyn Fn() -> G + Send + Sync>,
    pub reconcile: Reconcile,
}

pub fn extend_backend<G: SyncGistPort + 'static>(ports: Ports<G>, remaining: Dispatch) -> Dispatch {
    let ports = Arc::new(ports);
    let remaining_json = remaining.json;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let ports = ports.clone();
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if COMMANDS.contains(&name.as_str()) {
                    return dispatch(services, ports, &name, &args).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn dispatch<G: SyncGistPort + 'static>(
    services: Arc<AppServices>,
    ports: Arc<Ports<G>>,
    name: &str,
    args: &Value,
) -> Result<String, Value> {
    match name {
        "sync_status" => respond(
            sync_actions::sync_status(&services.state, services.secrets.0.as_ref(), || {
                (ports.create_client)()
            })
            .await,
        ),
        "sync_upload" => {
            let worker = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-sync-upload", async move {
                        sync_actions::sync_upload(
                            &worker.state,
                            worker.secrets.0.as_ref(),
                            || (ports.create_client)(),
                            worker.events.as_ref(),
                        )
                        .await
                    })
                    .await,
            )
        }
        "sync_download" => {
            let force = argument(args, "force").map_err(error_value)?;
            let prepared = sync_actions::prepare_sync_download(
                &services.state,
                services.secrets.0.as_ref(),
                || (ports.create_client)(),
            )
            .await
            .map_err(error_value)?;
            let worker = services.clone();
            respond(
                services
                    .tasks
                    .run_nonabortable_result("native-remote-sync-download-apply", async move {
                        sync_actions::apply_sync_download(
                            &worker.state,
                            prepared,
                            |next| {
                                settings_actions::apply_and_broadcast(
                                    &worker.state,
                                    next,
                                    |current, updated| {
                                        (ports.reconcile)(worker.clone(), current, updated)
                                    },
                                    worker.events.as_ref(),
                                )
                            },
                            force,
                            worker.events.as_ref(),
                        )
                        .await
                    })
                    .await,
            )
        }
        _ => unreachable!("sync command catalog and dispatch arms must agree"),
    }
}

#[cfg(test)]
#[path = "remote-sync-tests.rs"]
mod tests;
