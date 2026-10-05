use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use taide_ide::protocol::{encode, parse_incoming};
use taide_ide::{lockfile, service};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ide::{IdeDiffOutcome, IdeStatus};
use taide_runtime::AppServices;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{
    Callback, ErrorResponse, Request, Response,
};
use tokio_tungstenite::tungstenite::http::header::SEC_WEBSOCKET_PROTOCOL;
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};

use crate::ide_tools::{self, LayoutActions};

const PORT_BIND_MAX_ATTEMPTS: u32 = 20;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_millis(10_000);
const ACCEPT_RETRY_DELAY: Duration = Duration::from_millis(100);
const AUTH_HEADER: &str = "X-Claude-Code-Ide-Authorization";
const MCP_SUBPROTOCOL: &str = "mcp";

pub struct Ports {
    actions: Arc<LayoutActions>,
    version: String,
    resolve_directory: fn(&AppServices) -> AppResult<PathBuf>,
}

impl Ports {
    pub fn new(actions: LayoutActions, version: String) -> Self {
        Self {
            actions: Arc::new(actions),
            version,
            resolve_directory: |_| lockfile::lockfile_dir(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_test_directory(actions: LayoutActions) -> Self {
        Self {
            actions: Arc::new(actions),
            version: "synthetic-exit".into(),
            resolve_directory: |services| Ok(services.state.paths.data_dir.join("ide")),
        }
    }
}

pub async fn apply_toggle(
    services: Arc<AppServices>,
    ports: Arc<Ports>,
    was_enabled: bool,
    enabled: bool,
) {
    if was_enabled == enabled {
        return;
    }
    if enabled {
        if let Err(error) = start(services, ports).await {
            log::warn!("IDE 연동 시작 실패: {error}");
        }
        return;
    }
    stop(&services);
}

pub async fn start(services: Arc<AppServices>, ports: Arc<Ports>) -> AppResult<IdeStatus> {
    if services.ide.is_running() {
        return Ok(services.ide.status());
    }
    if !services.state.settings.read().ide_integration_enabled {
        return Err(AppError::InvalidArgument(
            "IDE integration is disabled in settings".into(),
        ));
    }
    let _operation = services
        .tasks
        .begin_operation("native-ide-server-start")
        .ok_or_else(|| AppError::Internal("IDE server task supervisor stopped".into()))?;
    if services.state.is_shutting_down() {
        return Err(AppError::Internal(
            "IDE server unavailable during shutdown".into(),
        ));
    }
    let workspace_folders = service::workspace_folders(&services.state.projects.read());
    let token = service::generate_auth_token();
    let directory = (ports.resolve_directory)(&services)?;
    let current_pid = std::process::id();
    match lockfile::cleanup_stale_lockfiles(&directory, current_pid) {
        Ok(0) => {}
        Ok(removed) => log::info!("정지된 IDE lockfile {removed}개 정리"),
        Err(error) => log::warn!("정지된 IDE lockfile 정리 실패: {error}"),
    }
    let mut last_error = None;
    for _ in 0..PORT_BIND_MAX_ATTEMPTS {
        let candidate_port = service::random_port();
        let bind_port = u16::try_from(candidate_port).map_err(|_| {
            AppError::Internal("IDE candidate port is outside the TCP range".into())
        })?;
        match TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, bind_port)).await {
            Ok(listener) => {
                let content =
                    lockfile::build_lockfile_content(current_pid, workspace_folders, token.clone());
                if let Err(error) =
                    lockfile::write_lockfile_atomic(&directory, candidate_port, &content)
                {
                    remove_candidate(&directory, candidate_port);
                    return Err(error);
                }
                let server_services = services.clone();
                let server_token = token.clone();
                let server_ports = ports.clone();
                let (ready_sender, ready_receiver) = oneshot::channel();
                let server =
                    services
                        .tasks
                        .spawn_transient_handle("native-ide-server", async move {
                            if ready_receiver.await.is_ok() {
                                accept_loop(server_services, server_ports, listener, server_token)
                                    .await;
                            }
                        });
                let Some(server) = server else {
                    remove_candidate(&directory, candidate_port);
                    return Err(AppError::Internal(
                        "IDE server task supervisor stopped".into(),
                    ));
                };
                if services.state.is_shutting_down() {
                    server.abort();
                    remove_candidate(&directory, candidate_port);
                    return Err(AppError::Internal(
                        "IDE server unavailable during shutdown".into(),
                    ));
                }
                let Some(status) =
                    services
                        .ide
                        .mark_started(candidate_port, token, directory.clone(), server)
                else {
                    remove_candidate(&directory, candidate_port);
                    return Ok(services.ide.status());
                };
                if services.state.is_shutting_down() {
                    stop(&services);
                    return Err(AppError::Internal(
                        "IDE server unavailable during shutdown".into(),
                    ));
                }
                services
                    .events
                    .publish(AppEvent::IdeStatusChanged { status });
                if ready_sender.send(()).is_err() {
                    stop(&services);
                    return Err(AppError::Internal(
                        "IDE server task stopped before accepting connections".into(),
                    ));
                }
                return Ok(status);
            }
            Err(error) => last_error = Some(error),
        }
    }
    let detail = last_error
        .map(|error| error.to_string())
        .unwrap_or_default();
    Err(AppError::localized(
        AppErrorKind::Internal,
        "error.ide.serverPortUnavailable",
        format!("could not find an IDE server port: {detail}"),
    )
    .with_arg("detail", &detail))
}

pub fn stop(services: &AppServices) {
    let Some(shutdown) = services.ide.take_shutdown_state() else {
        return;
    };
    if let Some(handle) = shutdown.server_handle {
        handle.abort();
    }
    for handle in shutdown.connection_handles {
        handle.abort();
    }
    if let Some(directory) = shutdown.dir {
        remove_candidate(&directory, shutdown.port);
    }
    for pending in shutdown.pending_diffs {
        drop(pending.responder.send((IdeDiffOutcome::Rejected, None)));
    }
    for pending in shutdown.pending_saves {
        let _ = pending.responder.send(false);
    }
    services.events.publish(AppEvent::IdeStatusChanged {
        status: IdeStatus::default(),
    });
}

fn remove_candidate(directory: &Path, port: u32) {
    if let Err(error) = lockfile::remove_lockfile(directory, port) {
        log::warn!("IDE lockfile 삭제 실패: {error}");
    }
}

pub fn refresh_lockfile(services: &AppServices) {
    let Some((port, token, directory)) = services.ide.lockfile_context() else {
        return;
    };
    let roots = service::workspace_folders(&services.state.projects.read());
    let content = lockfile::build_lockfile_content(std::process::id(), roots, token);
    if let Err(error) = lockfile::write_lockfile_atomic(&directory, port, &content) {
        log::warn!("IDE lockfile 갱신 실패: {error}");
    }
}

pub fn reconcile_stale_pending(services: &AppServices) {
    if !services.ide.is_running() {
        return;
    }
    let projects: HashSet<_> = services.state.projects.read().keys().cloned().collect();
    services.ide.resolve_pending_for_missing_projects(&projects);
}

struct Authentication(String);

impl Callback for Authentication {
    fn on_request(
        self,
        request: &Request,
        mut response: Response,
    ) -> Result<Response, ErrorResponse> {
        let provided = request
            .headers()
            .get(AUTH_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        if !service::constant_time_eq(provided.as_bytes(), self.0.as_bytes()) {
            let mut rejection = ErrorResponse::new(None);
            *rejection.status_mut() = StatusCode::UNAUTHORIZED;
            return Err(rejection);
        }
        let offers_mcp = request
            .headers()
            .get_all(SEC_WEBSOCKET_PROTOCOL)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .any(|value| {
                value
                    .split(',')
                    .any(|protocol| protocol.trim() == MCP_SUBPROTOCOL)
            });
        if offers_mcp {
            response.headers_mut().insert(
                SEC_WEBSOCKET_PROTOCOL,
                HeaderValue::from_static(MCP_SUBPROTOCOL),
            );
        }
        Ok(response)
    }
}

struct ConnectionOwner {
    services: Arc<AppServices>,
    token: String,
}

impl ConnectionOwner {
    fn new(services: Arc<AppServices>, token: String) -> Option<Self> {
        services.ide.client_connected_for_token(&token)?;
        services.events.publish(AppEvent::IdeStatusChanged {
            status: services.ide.status(),
        });
        Some(Self { services, token })
    }
}

impl Drop for ConnectionOwner {
    fn drop(&mut self) {
        if self
            .services
            .ide
            .client_disconnected_for_token(&self.token)
            .is_some()
        {
            self.services.events.publish(AppEvent::IdeStatusChanged {
                status: self.services.ide.status(),
            });
        }
    }
}

async fn handle_connection(
    services: Arc<AppServices>,
    ports: Arc<Ports>,
    stream: TcpStream,
    token: String,
) {
    let handshake = tokio::time::timeout(
        HANDSHAKE_TIMEOUT,
        tokio_tungstenite::accept_hdr_async(stream, Authentication(token.clone())),
    )
    .await;
    let stream = match handshake {
        Ok(Ok(stream)) => stream,
        Ok(Err(_)) => return,
        Err(_) => return,
    };
    let mut notifications = services.ide.subscribe();
    let Some(_owner) = ConnectionOwner::new(services.clone(), token) else {
        return;
    };
    let (mut sink, mut source) = stream.split();
    let (out_sender, mut out_receiver) = mpsc::unbounded_channel::<Message>();
    let mut children = JoinSet::new();
    let Some(writer_owner) = services.tasks.begin_operation("native-ide-writer") else {
        return;
    };
    children.spawn(async move {
        let _operation = writer_owner;
        while let Some(message) = out_receiver.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });
    let notification_sender = out_sender.clone();
    let Some(notify_owner) = services.tasks.begin_operation("native-ide-notify") else {
        return;
    };
    children.spawn(async move {
        let _operation = notify_owner;
        loop {
            match notifications.recv().await {
                Ok(message) => {
                    if notification_sender.send(Message::text(message)).is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
    while let Some(message) = source.next().await {
        let Ok(message) = message else { break };
        match message {
            Message::Text(text) => {
                let Ok(incoming) = parse_incoming(text.as_str()) else {
                    continue;
                };
                let request_services = services.clone();
                let request_ports = ports.clone();
                let response_sender = out_sender.clone();
                let Some(request_owner) = services.tasks.begin_operation("native-ide-request")
                else {
                    break;
                };
                while children.try_join_next().is_some() {}
                children.spawn(async move {
                    let _operation = request_owner;
                    if let Some(response) = ide_tools::handle_incoming(
                        request_services,
                        &request_ports.actions,
                        incoming,
                        &request_ports.version,
                    )
                    .await
                    {
                        drop(response_sender.send(Message::text(encode(&response))));
                    }
                });
            }
            Message::Ping(payload) => {
                drop(out_sender.send(Message::Pong(payload)));
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    children.shutdown().await;
}

async fn accept_loop(
    services: Arc<AppServices>,
    ports: Arc<Ports>,
    listener: TcpListener,
    token: String,
) {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let connection_services = services.clone();
                let connection_ports = ports.clone();
                let connection_token = token.clone();
                let handle =
                    services
                        .tasks
                        .spawn_transient_handle("native-ide-connection", async move {
                            handle_connection(
                                connection_services,
                                connection_ports,
                                stream,
                                connection_token,
                            )
                            .await;
                        });
                let Some(handle) = handle else { break };
                if !services.ide.register_connection(handle) {
                    break;
                }
            }
            Err(error) => {
                log::warn!("IDE accept 실패(계속): {error}");
                tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
            }
        }
    }
}

#[cfg(test)]
#[path = "ide-server-tests.rs"]
mod tests;
