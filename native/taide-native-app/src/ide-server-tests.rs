use std::sync::Mutex;

use serde_json::{Value, json};
use taide_ide::protocol::{diff_outcome_text, text_content};
use taide_model::ide::IdeLockfileContent;
use taide_model::ids::ProjectId;
use taide_model::layout::TabKind;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_runtime::{AppState, EventSink, TaskSupervisor, layout_actions};
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const VERSION: &str = "synthetic-native-version";
const TOOL_COUNT: usize = 12;
const SERVER_TASKS: usize = 1;
const CONNECTION_TASKS: usize = 3;
const CLIENT_COUNT: u32 = 2;
const FIRST_PORT: u32 = 12345;
const SECOND_PORT: u32 = 12346;

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Default)]
struct Sink(
    Mutex<Vec<AppEvent>>,
    Mutex<Option<taide_ide::store::IdeStore>>,
);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        if matches!(&event, AppEvent::IdeStatusChanged { status } if status.connected)
            && let Some(store) = self.1.lock().unwrap().as_ref()
        {
            store.broadcast(
                json!({"jsonrpc": "2.0", "method": "synthetic-connected-notification"}).to_string(),
            );
        }
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    project: Project,
    sink: Arc<Sink>,
    ports: Arc<Ports>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-ide-server-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = Project {
            id: ProjectId::new(),
            root: directory.join("project").to_str().unwrap().into(),
            name: "synthetic IDE server project".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        };
        state
            .projects
            .write()
            .insert(project.id.clone(), project.clone());
        state
            .layouts
            .write()
            .insert(project.id.clone(), taide_layout::service::default_layout());
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        let actions = LayoutActions {
            open_file_tab: |services, project, path, title, preview| {
                Box::pin(async move {
                    layout_actions::layout_open_tab(
                        services.events.as_ref(),
                        &services.state,
                        project,
                        TabKind::File { path },
                        title,
                        None,
                        preview,
                    )
                    .await
                    .map(|_| ())
                })
            },
            close_tab: Arc::new(|services, tab| {
                Box::pin(async move {
                    layout_actions::close_tab_and_finish(
                        services.events.as_ref(),
                        &services.state,
                        &tab,
                        |tab| services.ide.reconcile_closed_tab(tab),
                    )
                    .await
                    .map(|(_, closed, _)| closed.tab)
                })
            }),
        };
        let mut ports = Ports::new(actions, VERSION.into());
        ports.resolve_directory =
            |services| Ok(services.state.paths.data_dir.join("synthetic-ide"));
        Self {
            directory,
            services,
            project,
            sink,
            ports: Arc::new(ports),
        }
    }

    fn lock_directory(&self) -> PathBuf {
        self.services.state.paths.data_dir.join("synthetic-ide")
    }

    fn lock_content(&self, port: u32) -> IdeLockfileContent {
        serde_json::from_str(
            &std::fs::read_to_string(lockfile::lockfile_path(&self.lock_directory(), port))
                .unwrap(),
        )
        .unwrap()
    }

    async fn next_diff(&self) -> String {
        loop {
            {
                let mut events = self.sink.0.lock().unwrap();
                if let Some(index) = events
                    .iter()
                    .position(|event| matches!(event, AppEvent::IdeDiffRequested { .. }))
                {
                    let AppEvent::IdeDiffRequested { request_id, .. } = events.remove(index) else {
                        panic!("diff request expected");
                    };
                    return request_id;
                }
            }
            tokio::task::yield_now().await;
        }
    }

    async fn finish(&self) {
        stop(&self.services);
        self.services.tasks.shutdown().await;
        assert_eq!(self.services.ide.status(), IdeStatus::default());
        assert!(self.services.ide.lockfile_context().is_none());
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        stop(&self.services);
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn client_request(port: u32, token: Option<&str>, offer_mcp: bool) -> Request {
    let mut request = format!("ws://127.0.0.1:{port}/")
        .into_client_request()
        .unwrap();
    if let Some(token) = token {
        request
            .headers_mut()
            .insert(AUTH_HEADER, HeaderValue::from_str(token).unwrap());
    }
    if offer_mcp {
        request.headers_mut().insert(
            SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("other, another, mcp"),
        );
    }
    request.map(|_| ())
}

async fn wait_until(mut condition: impl FnMut() -> bool) {
    while !condition() {
        tokio::task::yield_now().await;
    }
}

async fn next_json(client: &mut Client) -> Value {
    loop {
        let message = client.next().await.unwrap().unwrap();
        if let Message::Text(text) = message {
            return serde_json::from_str(text.as_str()).unwrap();
        }
    }
}

async fn rpc(client: &mut Client, id: u32, method: &str, params: Value) -> Value {
    client
        .send(Message::text(
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string(),
        ))
        .await
        .unwrap();
    loop {
        let message = next_json(client).await;
        if message.get("id") == Some(&json!(id)) {
            return message;
        }
    }
}

#[tokio::test]
async fn native_ide_server는_실제_인증_mcp_rpc_notify와_pending_종료_lockfile을_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let services = &fixture.services;
        let status = start(services.clone(), fixture.ports.clone()).await.unwrap();
        assert!(status.running);
        assert_eq!(status.client_count, 0);
        let content = fixture.lock_content(status.port);
        assert_eq!(content.pid, std::process::id());
        assert_eq!(content.workspace_folders, vec![fixture.project.root.clone()]);
        let (_, token, _) = services.ide.lockfile_context().unwrap();
        assert!(content.auth_token == token);
        assert_eq!(content.transport, "ws");
        assert_eq!(content.ide_name, "TAIDE");
        let cached = start(services.clone(), fixture.ports.clone()).await.unwrap();
        assert_eq!(cached.port, status.port);

        for token in [None, Some("synthetic-wrong-token")] {
            let result = tokio_tungstenite::connect_async(client_request(status.port, token, false)).await;
            let Err(WebSocketError::Http(response)) = result else {
                panic!("authentication must reject handshake");
            };
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
        let mut repeated = client_request(status.port, Some(&token), false);
        repeated.headers_mut().append(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static("other"));
        repeated.headers_mut().append(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static("another, mcp"));
        let response = Authentication(token.clone()).on_request(&repeated, Response::new(())).unwrap();
        assert_eq!(response.headers().get(SEC_WEBSOCKET_PROTOCOL).unwrap(), MCP_SUBPROTOCOL);
        let (mut first, response) = tokio_tungstenite::connect_async(client_request(status.port, Some(&token), true)).await.unwrap();
        assert_eq!(response.headers().get(SEC_WEBSOCKET_PROTOCOL).unwrap(), MCP_SUBPROTOCOL);
        let (mut second, response) = tokio_tungstenite::connect_async(client_request(status.port, Some(&token), false)).await.unwrap();
        assert!(response.headers().get(SEC_WEBSOCKET_PROTOCOL).is_none());
        wait_until(|| services.ide.status().client_count == CLIENT_COUNT).await;
        assert!(services.ide.status().connected);

        let initialized = rpc(&mut first, 1, "initialize", json!({"protocolVersion": "synthetic-protocol"})).await;
        assert_eq!(initialized["result"]["serverInfo"]["version"], VERSION);
        assert_eq!(initialized["result"]["protocolVersion"], "synthetic-protocol");
        let listed = rpc(&mut first, 1, "tools/list", json!({})).await;
        assert_eq!(listed["result"]["tools"].as_array().unwrap().len(), TOOL_COUNT);
        first.send(Message::text("synthetic invalid json")).await.unwrap();
        assert_eq!(rpc(&mut first, 1, "ping", json!({})).await["result"], json!({}));
        services.ide.broadcast(json!({"jsonrpc": "2.0", "method": "synthetic-notification", "params": {}}).to_string());
        assert_eq!(next_json(&mut first).await["method"], "synthetic-notification");
        assert_eq!(next_json(&mut second).await["method"], "synthetic-notification");
        first.send(Message::Ping(vec![1].into())).await.unwrap();
        loop {
            let message = first.next().await.unwrap().unwrap();
            if let Message::Pong(payload) = message {
                assert_eq!(payload.as_ref(), &[1]);
                break;
            }
        }
        let target = PathBuf::from(&fixture.project.root).join("synthetic-proposed.cs");
        let diff_params = json!({"name": "openDiff", "arguments": {"new_file_path": target, "new_file_contents": "synthetic proposed"}});
        first.send(Message::text(json!({"id": 1, "method": "tools/call", "params": diff_params}).to_string())).await.unwrap();
        let cancelled_id = fixture.next_diff().await;
        drop(first);
        wait_until(|| services.ide.status().client_count == 1 && services.tasks.tracked_count() <= SERVER_TASKS + CONNECTION_TASKS).await;
        assert!(services.ide.take_pending_diff(&cancelled_id).is_none());

        second.send(Message::text(json!({"id": 1, "method": "tools/call", "params": diff_params}).to_string())).await.unwrap();
        let stale_id = fixture.next_diff().await;
        services.state.projects.write().remove(&fixture.project.id);
        reconcile_stale_pending(services);
        let result = next_json(&mut second).await;
        assert_eq!(result["result"], text_content(diff_outcome_text(IdeDiffOutcome::TabClosed)));
        assert!(services.ide.take_pending_diff(&stale_id).is_none());
        refresh_lockfile(services);
        assert!(fixture.lock_content(status.port).workspace_folders.is_empty());
        stop(services);
        assert_eq!(services.ide.status(), IdeStatus::default());
        assert!(!lockfile::lockfile_path(&fixture.lock_directory(), status.port).exists());
        assert!(matches!(second.next().await, None | Some(Err(_)) | Some(Ok(Message::Close(_)))));
        drop(second);

        services.state.projects.write().insert(fixture.project.id.clone(), fixture.project.clone());
        let restarted = start(services.clone(), fixture.ports.clone()).await.unwrap();
        let (_, new_token, _) = services.ide.lockfile_context().unwrap();
        assert!(new_token != token);
        let old_auth = tokio_tungstenite::connect_async(client_request(restarted.port, Some(&token), false)).await;
        assert!(matches!(old_auth, Err(WebSocketError::Http(response)) if response.status() == StatusCode::UNAUTHORIZED));
        let (third, _) = tokio_tungstenite::connect_async(client_request(restarted.port, Some(&new_token), false)).await.unwrap();
        wait_until(|| services.ide.status().client_count == 1).await;
        fixture.finish().await;
        assert!(!lockfile::lockfile_path(&fixture.lock_directory(), restarted.port).exists());
        drop(third);
    }).await.unwrap();
}

#[tokio::test]
async fn native_ide_server는_connected_이벤트의_동기_notify를_구독후_발행한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        *fixture.sink.1.lock().unwrap() = Some(fixture.services.ide.clone());
        let status = start(fixture.services.clone(), fixture.ports.clone())
            .await
            .unwrap();
        let (_, token, _) = fixture.services.ide.lockfile_context().unwrap();
        let (mut client, _) =
            tokio_tungstenite::connect_async(client_request(status.port, Some(&token), false))
                .await
                .unwrap();
        assert_eq!(
            next_json(&mut client).await["method"],
            "synthetic-connected-notification"
        );
        fixture.finish().await;
        drop(client);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_ide_server는_disabled_shutdown_등록거절과_lockfile_실패를_실제_시작으로_오인하지않는다()
 {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let ports = Arc::new(Ports {
            actions: fixture.ports.actions.clone(),
            version: VERSION.into(),
            resolve_directory: |_| panic!("denied startup must not resolve directory"),
        });
        fixture
            .services
            .state
            .settings
            .write()
            .ide_integration_enabled = false;
        let error = start(fixture.services.clone(), ports.clone())
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        apply_toggle(fixture.services.clone(), ports.clone(), false, false).await;
        fixture
            .services
            .state
            .settings
            .write()
            .ide_integration_enabled = true;
        fixture.services.state.begin_shutdown();
        let error = start(fixture.services.clone(), ports).await.err().unwrap();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        assert!(!fixture.lock_directory().exists());
        fixture.finish().await;

        let stopped = Fixture::new();
        stopped.services.tasks.stop_all();
        let error = start(stopped.services.clone(), stopped.ports.clone())
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        assert!(!stopped.lock_directory().exists());
        stopped.finish().await;

        let failure = Fixture::new();
        std::fs::write(
            &failure.services.state.paths.data_dir,
            "synthetic data parent obstruction",
        )
        .unwrap();
        let error = start(failure.services.clone(), failure.ports.clone())
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind(), AppErrorKind::Io);
        assert!(!failure.services.ide.is_running());
        assert!(failure.sink.0.lock().unwrap().is_empty());
        assert_eq!(
            std::fs::read_to_string(&failure.services.state.paths.data_dir).unwrap(),
            "synthetic data parent obstruction"
        );
        failure.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_ide_server의_늦은_connection_owner는_새_token의_연결수를_감소시키지않는다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let services = &fixture.services;
        let server = services
            .tasks
            .spawn_transient_handle("synthetic-ide-first", std::future::pending())
            .unwrap();
        services
            .ide
            .mark_started(
                FIRST_PORT,
                "synthetic-first-token".into(),
                fixture.lock_directory(),
                server,
            )
            .unwrap();
        let old = ConnectionOwner::new(services.clone(), "synthetic-first-token".into()).unwrap();
        assert_eq!(services.ide.status().client_count, 1);
        stop(services);
        let server = services
            .tasks
            .spawn_transient_handle("synthetic-ide-second", std::future::pending())
            .unwrap();
        services
            .ide
            .mark_started(
                SECOND_PORT,
                "synthetic-second-token".into(),
                fixture.lock_directory(),
                server,
            )
            .unwrap();
        let current =
            ConnectionOwner::new(services.clone(), "synthetic-second-token".into()).unwrap();
        assert!(ConnectionOwner::new(services.clone(), "synthetic-first-token".into()).is_none());
        drop(old);
        assert_eq!(services.ide.status().client_count, 1);
        drop(current);
        assert_eq!(services.ide.status().client_count, 0);
        assert!(!services.ide.status().connected);
        fixture.finish().await;
    })
    .await
    .unwrap();
}
