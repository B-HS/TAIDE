use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tauri::ipc::InvokeResponseBody;
use tauri::{AppHandle, Manager};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::{self, UnboundedSender};

use super::commands::RemoteStore;
use super::dispatch::{self, ChannelFactory, ChannelSink};
use super::types::{RemoteRequest, REMOTE_BINARY_TAG_CHANNEL, REMOTE_BINARY_TAG_RESPONSE};

enum WsOut {
    Text(String),
    Binary(Vec<u8>),
}

impl WsOut {
    fn into_message(self) -> Message {
        match self {
            WsOut::Text(text) => Message::Text(text.into()),
            WsOut::Binary(bytes) => Message::Binary(bytes.into()),
        }
    }
}

struct ChannelEndGuard {
    ws_out: UnboundedSender<WsOut>,
    channel_id: u32,
    counter: Arc<AtomicU32>,
}

impl Drop for ChannelEndGuard {
    fn drop(&mut self) {
        let index = self.counter.load(Ordering::Relaxed);
        let frame = serde_json::json!({ "t": "chanEnd", "channelId": self.channel_id, "index": index }).to_string();
        let _ = self.ws_out.send(WsOut::Text(frame));
    }
}

fn channel_binary_frame(channel_id: u32, index: u32, bytes: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(9 + bytes.len());
    frame.push(REMOTE_BINARY_TAG_CHANNEL);
    frame.extend_from_slice(&channel_id.to_be_bytes());
    frame.extend_from_slice(&index.to_be_bytes());
    frame.extend_from_slice(bytes);
    frame
}

fn response_binary_frame(seq: u32, bytes: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(5 + bytes.len());
    frame.push(REMOTE_BINARY_TAG_RESPONSE);
    frame.extend_from_slice(&seq.to_be_bytes());
    frame.extend_from_slice(bytes);
    frame
}

fn response_frame(seq: u32, ok: bool, payload: Value) -> String {
    serde_json::json!({ "t": "resp", "seq": seq, "ok": ok, "payload": payload }).to_string()
}

fn make_channel_factory(ws_out: UnboundedSender<WsOut>) -> ChannelFactory {
    Arc::new(move |id: String| -> ChannelSink {
        let channel_id = id.parse::<u32>().unwrap_or(0);
        let counter = Arc::new(AtomicU32::new(0));
        let guard = ChannelEndGuard {
            ws_out: ws_out.clone(),
            channel_id,
            counter: counter.clone(),
        };
        let sink_out = ws_out.clone();
        Box::new(move |body: InvokeResponseBody| {
            let index = counter.fetch_add(1, Ordering::Relaxed);
            let _ = &guard;
            match body {
                InvokeResponseBody::Json(text) => {
                    let message = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
                    let frame = serde_json::json!({ "t": "chan", "channelId": channel_id, "index": index, "message": message }).to_string();
                    let _ = sink_out.send(WsOut::Text(frame));
                }
                InvokeResponseBody::Raw(bytes) => {
                    let _ = sink_out.send(WsOut::Binary(channel_binary_frame(channel_id, index, &bytes)));
                }
            }
            Ok(())
        })
    })
}

async fn handle_request(app: &AppHandle, request: RemoteRequest, factory: ChannelFactory, ws_out: &UnboundedSender<WsOut>) {
    let RemoteRequest { seq, command, args } = request;

    if command == "file_read_raw" {
        match dispatch::dispatch_raw(app, &command, args).await {
            Ok(bytes) => {
                let _ = ws_out.send(WsOut::Binary(response_binary_frame(seq, &bytes)));
            }
            Err(payload) => {
                let _ = ws_out.send(WsOut::Text(response_frame(seq, false, payload)));
            }
        }
        return;
    }

    match dispatch::dispatch(app, &command, args, factory).await {
        Ok(json) => {
            let payload = serde_json::from_str::<Value>(&json).unwrap_or(Value::Null);
            let _ = ws_out.send(WsOut::Text(response_frame(seq, true, payload)));
        }
        Err(payload) => {
            let _ = ws_out.send(WsOut::Text(response_frame(seq, false, payload)));
        }
    }
}

/// Drives one upgraded remote WebSocket connection. Besides the request/
/// response and event-fanout loops, this races the connection's own read
/// loop against [`RemoteStore::subscribe_session_epoch`] so a bulk session
/// revocation (revoke-all, a password change, or server stop) that happens
/// *after* the upgrade closes the socket immediately — see that method's
/// doc comment for what this does and does not cover.
pub async fn handle_socket(socket: WebSocket, app: AppHandle) {
    let remote = app.state::<RemoteStore>();
    remote.client_connected();
    let mut events = remote.subscribe_events();
    let mut session_epoch = remote.subscribe_session_epoch();

    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsOut>();

    let writer = tauri::async_runtime::spawn(async move {
        while let Some(out) = rx.recv().await {
            if sink.send(out.into_message()).await.is_err() {
                break;
            }
        }
    });

    let event_out = tx.clone();
    let event_task = tauri::async_runtime::spawn(async move {
        loop {
            match events.recv().await {
                Ok(frame) => {
                    if event_out.send(WsOut::Text(frame)).is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(count)) => {
                    log::warn!("원격 이벤트 {count}건이 유실되어 재동기화가 필요합니다");
                    continue;
                }
                Err(RecvError::Closed) => break,
            }
        }
    });

    let factory = make_channel_factory(tx.clone());

    loop {
        tokio::select! {
            frame = stream.next() => {
                let Some(Ok(message)) = frame else { break };
                match message {
                    Message::Text(text) => {
                        let Ok(request) = serde_json::from_str::<RemoteRequest>(text.as_str()) else {
                            continue;
                        };
                        let request_app = app.clone();
                        let request_factory = factory.clone();
                        let request_out = tx.clone();
                        tauri::async_runtime::spawn(async move {
                            handle_request(&request_app, request, request_factory, &request_out).await;
                        });
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            _ = session_epoch.changed() => {
                log::info!("원격 세션이 무효화되어 연결을 종료합니다");
                break;
            }
        }
    }

    event_task.abort();
    drop(factory);
    drop(tx);
    let _ = writer.await;
    remote.client_disconnected();
}
