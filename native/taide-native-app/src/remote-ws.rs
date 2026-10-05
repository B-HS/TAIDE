use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use futures_util::future::BoxFuture;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use taide_model::error::{AppError, AppResult};
use taide_model::remote::RemoteRequest;
use taide_remote::protocol::{
    channel_binary_frame, channel_end_frame, channel_json_frame, response_binary_frame,
    response_frame,
};
use taide_remote::store::RemoteStore;
use taide_remote::types::REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED;
use taide_runtime::AppServices;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

const OUTBOUND_QUEUE_CAPACITY: usize = 256;
const WRITER_SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(3_000);
const SESSION_EXPIRED_REASON: &str = "session_expired";

pub enum ResponseBody {
    Json(String),
    Raw(Vec<u8>),
}

pub type ChannelSink = Box<dyn Fn(ResponseBody) -> AppResult<()> + Send + Sync>;
pub type ChannelFactory = Arc<dyn Fn(String) -> ChannelSink + Send + Sync>;
pub type JsonDispatch = Arc<
    dyn Fn(
            Arc<AppServices>,
            String,
            Value,
            ChannelFactory,
        ) -> BoxFuture<'static, Result<String, Value>>
        + Send
        + Sync,
>;
pub type RawDispatch = Arc<
    dyn Fn(Arc<AppServices>, String, Value) -> BoxFuture<'static, Result<Vec<u8>, Value>>
        + Send
        + Sync,
>;

pub struct Dispatch {
    pub json: JsonDispatch,
    pub raw: RawDispatch,
}

pub fn socket_action(dispatch: Arc<Dispatch>) -> crate::remote_http::SocketAction {
    Arc::new(move |socket, services, digest| {
        Box::pin(handle_socket(socket, services, digest, dispatch.clone()))
    })
}

enum WsOut {
    Text(String),
    Binary(Vec<u8>),
    Close(u16, &'static str),
}

impl WsOut {
    fn into_message(self) -> Message {
        match self {
            Self::Text(text) => Message::Text(text.into()),
            Self::Binary(bytes) => Message::Binary(bytes.into()),
            Self::Close(code, reason) => Message::Close(Some(CloseFrame {
                code,
                reason: reason.into(),
            })),
        }
    }
}

#[derive(Clone)]
struct OutboundQueue {
    sender: mpsc::Sender<WsOut>,
    saturated: watch::Sender<bool>,
}

impl OutboundQueue {
    fn new() -> (Self, mpsc::Receiver<WsOut>, watch::Receiver<bool>) {
        let (sender, receiver) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
        let (saturated, signal) = watch::channel(false);
        (Self { sender, saturated }, receiver, signal)
    }

    fn send(&self, frame: WsOut) -> Result<(), ()> {
        if *self.saturated.borrow() {
            return Err(());
        }
        match self.sender.try_send(frame) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.saturated.send_replace(true);
                Err(())
            }
            Err(TrySendError::Closed(_)) => Err(()),
        }
    }
}

struct ChannelEnd {
    out: OutboundQueue,
    id: u32,
    counter: Arc<AtomicU32>,
}

impl Drop for ChannelEnd {
    fn drop(&mut self) {
        let index = self.counter.load(Ordering::Relaxed);
        let _ = self
            .out
            .send(WsOut::Text(channel_end_frame(self.id, index)));
    }
}

fn channel_factory(out: OutboundQueue) -> ChannelFactory {
    Arc::new(move |id| {
        let id = id.parse::<u32>().unwrap_or(0);
        let counter = Arc::new(AtomicU32::new(0));
        let end = ChannelEnd {
            out: out.clone(),
            id,
            counter: counter.clone(),
        };
        let channel_out = out.clone();
        Box::new(move |body| {
            let _ = &end;
            let index = counter.fetch_add(1, Ordering::Relaxed);
            let delivery = match body {
                ResponseBody::Json(text) => {
                    let message = serde_json::from_str(&text).unwrap_or(Value::Null);
                    channel_out.send(WsOut::Text(channel_json_frame(id, index, message)))
                }
                ResponseBody::Raw(bytes) => {
                    channel_out.send(WsOut::Binary(channel_binary_frame(id, index, &bytes)))
                }
            };
            delivery.map_err(|_| {
                AppError::Io("원격 웹소켓 연결이 종료되어 채널로 전달할 수 없습니다".into())
            })
        })
    })
}

async fn handle_request(
    services: Arc<AppServices>,
    dispatch: &Dispatch,
    request: RemoteRequest,
    factory: ChannelFactory,
    out: OutboundQueue,
) {
    let RemoteRequest { seq, command, args } = request;
    if command == "file_read_raw" {
        let frame = match (dispatch.raw)(services, command, args).await {
            Ok(bytes) => WsOut::Binary(response_binary_frame(seq, &bytes)),
            Err(payload) => WsOut::Text(response_frame(seq, false, payload)),
        };
        let _ = out.send(frame);
        return;
    }
    let (ok, payload) = match (dispatch.json)(services, command, args, factory).await {
        Ok(json) => (true, serde_json::from_str(&json).unwrap_or(Value::Null)),
        Err(payload) => (false, payload),
    };
    let _ = out.send(WsOut::Text(response_frame(seq, ok, payload)));
}

async fn session_expiration_close(remote: RemoteStore, digest: String) -> Option<WsOut> {
    let deadline = remote
        .session_expires_at(&digest)
        .unwrap_or_else(tokio::time::Instant::now);
    tokio::time::sleep_until(deadline).await;
    if remote.has_active_session_digest(&digest) {
        return None;
    }
    Some(WsOut::Close(
        REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED,
        SESSION_EXPIRED_REASON,
    ))
}

struct ConnectionOwner {
    remote: RemoteStore,
    generation: u64,
}

impl ConnectionOwner {
    fn new(remote: RemoteStore, digest: &str) -> Option<Self> {
        let generation = remote.connection_generation()?;
        if !remote.has_active_session_digest(digest) {
            return None;
        }
        remote.client_connected_for_generation(generation)?;
        Some(Self { remote, generation })
    }
}

impl Drop for ConnectionOwner {
    fn drop(&mut self) {
        self.remote
            .client_disconnected_for_generation(self.generation);
    }
}

struct AbortOnDrop(JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub async fn handle_socket(
    socket: WebSocket,
    services: Arc<AppServices>,
    digest: String,
    dispatch: Arc<Dispatch>,
) {
    let Some(_operation) = services.tasks.begin_operation("native-remote-ws") else {
        return;
    };
    let remote = services.remote.clone();
    let mut events = remote.subscribe_events();
    let mut session_epoch = remote.subscribe_session_epoch();
    remote.sweep_expired_sessions();
    let Some(_owner) = ConnectionOwner::new(remote.clone(), &digest) else {
        return;
    };
    let expiry = session_expiration_close(remote, digest);
    tokio::pin!(expiry);
    let (mut sink, mut stream) = socket.split();
    let (out, mut receiver, mut saturation_signal) = OutboundQueue::new();
    let Some(writer) =
        services
            .tasks
            .spawn_transient_handle("native-remote-ws-writer", async move {
                while let Some(frame) = receiver.recv().await {
                    if sink.send(frame.into_message()).await.is_err() {
                        break;
                    }
                }
            })
    else {
        return;
    };
    let mut writer = AbortOnDrop(writer);
    let event_out = out.clone();
    let Some(event_task) =
        services
            .tasks
            .spawn_transient_handle("native-remote-ws-events", async move {
                loop {
                    match events.recv().await {
                        Ok(frame) => {
                            if event_out.send(WsOut::Text(frame)).is_err() {
                                break;
                            }
                        }
                        Err(RecvError::Lagged(count)) => {
                            log::warn!("원격 이벤트 {count}건이 유실되어 재동기화가 필요합니다");
                        }
                        Err(RecvError::Closed) => break,
                    }
                }
            })
    else {
        return;
    };
    let event_task = AbortOnDrop(event_task);
    let factory = channel_factory(out.clone());
    loop {
        tokio::select! {
            frame = stream.next() => {
                let Some(Ok(message)) = frame else { break };
                match message {
                    Message::Text(text) => {
                        let Ok(request) = serde_json::from_str::<RemoteRequest>(text.as_str()) else {
                            continue;
                        };
                        let request_services = services.clone();
                        let request_dispatch = dispatch.clone();
                        let request_factory = factory.clone();
                        let request_out = out.clone();
                        if !services.tasks.spawn_transient("native-remote-ws-request", async move {
                            let Some(_permit) = request_services.remote_dispatch_limiter.acquire().await else {
                                log::warn!("원격 dispatch 세마포어를 획득하지 못해 요청을 처리하지 못했습니다");
                                return;
                            };
                            handle_request(request_services.clone(), &request_dispatch, request, request_factory, request_out).await;
                        }) {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            _ = session_epoch.changed() => break,
            close = &mut expiry => {
                if let Some(close) = close {
                    let _ = out.send(close);
                }
                break;
            }
            _ = saturation_signal.changed() => break,
        }
    }
    let is_saturated = *saturation_signal.borrow();
    event_task.0.abort();
    drop(factory);
    drop(out);
    if is_saturated {
        writer.0.abort();
        drop((&mut writer.0).await);
        return;
    }
    if tokio::time::timeout(WRITER_SHUTDOWN_TIMEOUT, &mut writer.0)
        .await
        .is_err()
    {
        log::warn!("원격 웹소켓 writer 종료 제한 시간 초과로 정리합니다");
        writer.0.abort();
        drop((&mut writer.0).await);
    }
}

#[cfg(test)]
#[path = "remote-ws-tests.rs"]
mod tests;
