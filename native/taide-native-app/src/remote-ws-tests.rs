use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;

use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_remote::service;
use taide_remote::types::REMOTE_SESSION_TTL_MS;
use taide_runtime::{AppState, EventSink, RemoteDispatchLimiter, TaskSupervisor};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};
use tokio_tungstenite::tungstenite::{Error, Message as ClientMessage};

use super::*;
use crate::remote_http;

const DEADLINE: Duration = Duration::from_secs(5);
const FILE_BYTES: &[u8] = &[0, 255, 12];
const CHANNEL_ID: u32 = 17;

struct MemorySecret;

impl SecretStore for MemorySecret {
    fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
        panic!("WebSocket fixture must not write secrets")
    }

    fn get(&self, _: SecretAccount) -> AppResult<Option<String>> {
        Ok(None)
    }

    fn delete(&self, _: SecretAccount) -> AppResult<()> {
        panic!("WebSocket fixture must not delete secrets")
    }
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    ports: Arc<remote_http::Ports>,
    calls: Arc<AtomicUsize>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-ws-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        std::fs::write(directory.join("project/fixture.bin"), FILE_BYTES).unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = Project {
            id: ProjectId::new(),
            root: directory.join("project").to_str().unwrap().into(),
            name: "synthetic WebSocket project".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        };
        state.projects.write().insert(project.id.clone(), project);
        let mut services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        );
        let mutable = Arc::get_mut(&mut services).unwrap();
        mutable.secrets = SecretStoreState(Arc::new(MemorySecret));
        mutable.remote_dispatch_limiter = RemoteDispatchLimiter::new(1);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let dispatch = Dispatch {
            json: Arc::new(move |services, command, _, channels| {
                let calls = observed.clone();
                Box::pin(async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    match command.as_str() {
                        "settings_get" => {
                            let settings =
                                taide_runtime::settings_actions::settings_get(&services.state)
                                    .await
                                    .unwrap();
                            Ok(serde_json::to_string(&settings).unwrap())
                        }
                        "synthetic_channels" => {
                            let sink = channels(CHANNEL_ID.to_string());
                            sink(ResponseBody::Json("{\"synthetic\":true}".into())).unwrap();
                            sink(ResponseBody::Raw(FILE_BYTES.to_vec())).unwrap();
                            drop(sink);
                            Ok("null".into())
                        }
                        "synthetic_bad_json" => Ok("invalid JSON".into()),
                        _ => Err(serde_json::json!({"code":"SYNTHETIC_DENIED"})),
                    }
                })
            }),
            raw: Arc::new(|services, command, args| {
                Box::pin(async move {
                    assert_eq!(command, "file_read_raw");
                    let path = args
                        .get("path")
                        .and_then(Value::as_str)
                        .ok_or_else(|| serde_json::json!({"code":"InvalidArgument"}))?;
                    taide_runtime::file_actions::file_read_raw(&services.state, path.into())
                        .await
                        .map_err(|error| serde_json::to_value(error).unwrap())
                })
            }),
        };
        let ports = Arc::new(remote_http::Ports {
            socket: socket_action(Arc::new(dispatch)),
            assets: Arc::new(|path| {
                (path == "index.html").then(|| crate::remote_serving::Asset {
                    mime: "text/html".into(),
                    bytes: b"<!doctype html><title>synthetic</title>".to_vec(),
                })
            }),
        });
        Self {
            directory,
            services,
            ports,
            calls,
        }
    }

    async fn start(&self) {
        remote_http::start(self.services.clone(), self.ports.clone())
            .await
            .unwrap();
    }

    fn session(&self) -> String {
        self.services.remote.issue_session_without_nonce()
    }

    async fn connect(&self, session: Option<&str>) -> Result<WebSocketStream<TcpStream>, Error> {
        let url = format!("ws://127.0.0.1:{}/__taide/ws", self.services.remote.port());
        let mut request = url.into_client_request().unwrap();
        if let Some(session) = session {
            request.headers_mut().insert(
                "Cookie",
                HeaderValue::from_str(&format!("taide_remote_session={session}")).unwrap(),
            );
        }
        let socket = TcpStream::connect((
            "127.0.0.1",
            u16::try_from(self.services.remote.port()).unwrap(),
        ))
        .await
        .unwrap();
        let (socket, _) = tokio_tungstenite::client_async(request, socket).await?;
        Ok(socket)
    }

    async fn http_status(&self, session: &str) -> String {
        let port = u16::try_from(self.services.remote.port()).unwrap();
        let mut socket = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let request = format!(
            "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nCookie: taide_remote_session={session}\r\nConnection: close\r\n\r\n"
        );
        socket.write_all(request.as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).await.unwrap();
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .to_string()
    }

    async fn finish(&self) {
        remote_http::stop(&self.services);
        self.services.tasks.shutdown().await;
        assert_eq!(self.services.tasks.tracked_count(), 0);
        assert_eq!(self.services.remote.status().client_count, 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        remote_http::stop(&self.services);
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

async fn wait_until(predicate: impl Fn() -> bool) {
    tokio::time::timeout(DEADLINE, async {
        while !predicate() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("state transition within deadline");
}

async fn receive(socket: &mut WebSocketStream<TcpStream>) -> ClientMessage {
    tokio::time::timeout(DEADLINE, socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}

async fn send_request(
    socket: &mut WebSocketStream<TcpStream>,
    seq: u32,
    command: &str,
    args: Value,
) {
    socket
        .send(ClientMessage::text(
            serde_json::json!({"seq":seq,"command":command,"args":args}).to_string(),
        ))
        .await
        .unwrap();
}

fn json_frame(message: ClientMessage) -> Value {
    let ClientMessage::Text(text) = message else {
        panic!("JSON frame expected")
    };
    serde_json::from_str(&text).unwrap()
}

#[tokio::test]
async fn 실제_ws_인증과_json_raw_channel_event_왕복_후_정리된다() {
    let fixture = Fixture::new();
    fixture.start().await;
    let Err(Error::Http(rejection)) = fixture.connect(None).await else {
        panic!("unauthenticated upgrade must fail")
    };
    assert_eq!(rejection.status(), StatusCode::UNAUTHORIZED);
    let session = fixture.session();
    let mut socket = fixture.connect(Some(&session)).await.unwrap();
    wait_until(|| fixture.services.remote.status().client_count == 1).await;
    socket
        .send(ClientMessage::text("invalid request"))
        .await
        .unwrap();
    send_request(&mut socket, 1, "settings_get", Value::Null).await;
    let reply = json_frame(receive(&mut socket).await);
    assert_eq!(reply["seq"], 1);
    assert_eq!(reply["ok"], true);
    assert_eq!(
        reply["payload"]["language"],
        fixture.services.state.settings.read().language
    );
    let path = fixture.directory.join("project/fixture.bin");
    send_request(
        &mut socket,
        2,
        "file_read_raw",
        serde_json::json!({"path":path}),
    )
    .await;
    let ClientMessage::Binary(bytes) = receive(&mut socket).await else {
        panic!("binary response expected")
    };
    assert_eq!(bytes.as_ref(), response_binary_frame(2, FILE_BYTES));
    send_request(
        &mut socket,
        3,
        "file_read_raw",
        serde_json::json!({"path":fixture.directory.join("outside.bin")}),
    )
    .await;
    let denied = json_frame(receive(&mut socket).await);
    assert_eq!(denied["ok"], false);
    assert_eq!(denied["payload"]["code"], "Localized");
    assert_eq!(denied["payload"]["message"]["kind"], "Forbidden");
    send_request(&mut socket, 4, "synthetic_channels", Value::Null).await;
    assert_eq!(
        json_frame(receive(&mut socket).await),
        serde_json::json!({"t":"chan","channelId":CHANNEL_ID,"index":0,"message":{"synthetic":true}})
    );
    let ClientMessage::Binary(bytes) = receive(&mut socket).await else {
        panic!("binary channel expected")
    };
    assert_eq!(
        bytes.as_ref(),
        channel_binary_frame(CHANNEL_ID, 1, FILE_BYTES)
    );
    assert_eq!(
        json_frame(receive(&mut socket).await),
        serde_json::json!({"t":"chanEnd","channelId":CHANNEL_ID,"index":2})
    );
    assert_eq!(
        json_frame(receive(&mut socket).await),
        serde_json::json!({"t":"resp","seq":4,"ok":true,"payload":null})
    );
    send_request(&mut socket, 5, "synthetic_bad_json", Value::Null).await;
    assert_eq!(
        json_frame(receive(&mut socket).await)["payload"],
        Value::Null
    );
    send_request(&mut socket, 6, "synthetic_denied", Value::Null).await;
    let denied = json_frame(receive(&mut socket).await);
    assert_eq!(denied["ok"], false);
    assert_eq!(denied["payload"]["code"], "SYNTHETIC_DENIED");
    fixture
        .services
        .remote
        .broadcast_event("{\"t\":\"synthetic_event\"}".into());
    assert_eq!(
        json_frame(receive(&mut socket).await),
        serde_json::json!({"t":"synthetic_event"})
    );
    socket.close(None).await.unwrap();
    wait_until(|| fixture.services.remote.status().client_count == 0).await;
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_7일_시계_경과는_http_401과_ws_4001을_보낸다() {
    let fixture = Fixture::new();
    fixture.start().await;
    let session = fixture.session();
    let mut socket = fixture.connect(Some(&session)).await.unwrap();
    wait_until(|| fixture.services.remote.status().client_count == 1).await;
    send_request(&mut socket, 1, "settings_get", Value::Null).await;
    assert_eq!(json_frame(receive(&mut socket).await)["ok"], true);
    tokio::time::pause();
    tokio::time::advance(Duration::from_millis(REMOTE_SESSION_TTL_MS)).await;
    tokio::time::resume();
    let ClientMessage::Close(Some(frame)) = receive(&mut socket).await else {
        panic!("expiration close expected")
    };
    assert_eq!(u16::from(frame.code), REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED);
    assert_eq!(frame.reason.as_str(), SESSION_EXPIRED_REASON);
    assert_eq!(
        fixture.http_status(&session).await,
        "HTTP/1.1 401 Unauthorized"
    );
    assert!(!fixture.services.remote.has_active_session(&session));
    wait_until(|| fixture.services.remote.status().client_count == 0).await;
    fixture.finish().await;
}

#[tokio::test]
async fn permit_대기_요청은_폐기_뒤_실행하며_옛_connection은_새_count를_변경하지_않는다() {
    let fixture = Fixture::new();
    fixture.start().await;
    let session = fixture.session();
    let mut old = fixture.connect(Some(&session)).await.unwrap();
    send_request(&mut old, 1, "settings_get", Value::Null).await;
    assert_eq!(json_frame(receive(&mut old).await)["ok"], true);
    let held = fixture
        .services
        .remote_dispatch_limiter
        .acquire()
        .await
        .unwrap();
    let baseline = fixture.services.tasks.tracked_count();
    send_request(&mut old, 2, "settings_get", Value::Null).await;
    wait_until(|| fixture.services.tasks.tracked_count() > baseline).await;
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 1);
    let generation = fixture.services.remote.connection_generation().unwrap();
    fixture.services.remote.revoke_all_sessions();
    remote_http::stop(&fixture.services);
    fixture.start().await;
    assert_ne!(
        fixture.services.remote.connection_generation().unwrap(),
        generation
    );
    let next_session = fixture.session();
    let mut next = fixture.connect(Some(&next_session)).await.unwrap();
    wait_until(|| fixture.services.remote.status().client_count == 1).await;
    let old_end = tokio::time::timeout(DEADLINE, old.next()).await.unwrap();
    assert!(!matches!(old_end, Some(Ok(ClientMessage::Text(_)))));
    wait_until(|| fixture.services.tasks.tracked_count() <= baseline + 1).await;
    assert_eq!(fixture.services.remote.status().client_count, 1);
    assert_eq!(
        fixture
            .services
            .remote
            .client_disconnected_for_generation(generation),
        None
    );
    drop(held);
    wait_until(|| fixture.calls.load(Ordering::SeqCst) == 2).await;
    send_request(&mut next, 3, "settings_get", Value::Null).await;
    assert_eq!(json_frame(receive(&mut next).await)["ok"], true);
    fixture.finish().await;
    assert!(
        !fixture
            .services
            .remote
            .has_active_session_digest(&service::digest_hex(&next_session))
    );
}

#[test]
fn 채널_wire_순서와_닫힌_수신자_포화_거부가_보존된다() {
    let (out, mut receiver, _) = OutboundQueue::new();
    let sink = channel_factory(out)("bad-id".into());
    sink(ResponseBody::Json("invalid".into())).unwrap();
    sink(ResponseBody::Raw(FILE_BYTES.to_vec())).unwrap();
    drop(sink);
    let Ok(WsOut::Text(json)) = receiver.try_recv() else {
        panic!("JSON expected")
    };
    assert_eq!(
        serde_json::from_str::<Value>(&json).unwrap(),
        serde_json::json!({"t":"chan","channelId":0,"index":0,"message":null})
    );
    let Ok(WsOut::Binary(bytes)) = receiver.try_recv() else {
        panic!("binary expected")
    };
    assert_eq!(bytes, channel_binary_frame(0, 1, FILE_BYTES));
    let Ok(WsOut::Text(end)) = receiver.try_recv() else {
        panic!("channel end expected")
    };
    assert_eq!(end, channel_end_frame(0, 2));
    let (out, receiver, _) = OutboundQueue::new();
    let closed = channel_factory(out)("1".into());
    drop(receiver);
    assert!(closed(ResponseBody::Json("null".into())).is_err());
    let (out, mut receiver, signal) = OutboundQueue::new();
    let saturated = channel_factory(out)("1".into());
    for _ in 0..OUTBOUND_QUEUE_CAPACITY {
        saturated(ResponseBody::Json("null".into())).unwrap();
    }
    assert!(saturated(ResponseBody::Raw(FILE_BYTES.to_vec())).is_err());
    assert!(*signal.borrow());
    assert_eq!(receiver.len(), OUTBOUND_QUEUE_CAPACITY);
    receiver
        .try_recv()
        .unwrap_or_else(|_| panic!("queued frame expected"));
    assert!(saturated(ResponseBody::Json("null".into())).is_err());
    drop(saturated);
    assert_eq!(receiver.len(), OUTBOUND_QUEUE_CAPACITY - 1);
}

#[tokio::test]
async fn 실제_활성_ws의_감독자_직접_종료는_upgrade_owner까지_회수한다() {
    let fixture = Fixture::new();
    fixture.start().await;
    let session = fixture.session();
    let mut socket = fixture.connect(Some(&session)).await.unwrap();
    send_request(&mut socket, 1, "settings_get", Value::Null).await;
    assert_eq!(json_frame(receive(&mut socket).await)["ok"], true);
    tokio::time::timeout(DEADLINE, fixture.services.tasks.shutdown())
        .await
        .expect("direct shutdown must reclaim the live upgrade callback");
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    assert_eq!(fixture.services.remote.status().client_count, 0);
    let ended = tokio::time::timeout(DEADLINE, socket.next()).await.unwrap();
    assert!(!matches!(ended, Some(Ok(ClientMessage::Text(_)))));
}
