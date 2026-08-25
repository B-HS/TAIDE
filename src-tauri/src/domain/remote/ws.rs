use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use axum::extract::ws::{CloseFrame, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tauri::ipc::InvokeResponseBody;
use tauri::{AppHandle, Manager};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::{self, UnboundedSender};

use super::commands::{RemoteDispatchLimiter, RemoteStore};
use super::dispatch::{self, ChannelFactory, ChannelSink};
use super::types::{
    RemoteRequest, REMOTE_BINARY_TAG_CHANNEL, REMOTE_BINARY_TAG_RESPONSE, REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED,
    REMOTE_WS_CLOSE_REASON_SESSION_EXPIRED, REMOTE_WS_WRITER_SHUTDOWN_TIMEOUT_MS,
};

enum WsOut {
    Text(String),
    Binary(Vec<u8>),
    Close(u16, &'static str),
}

impl WsOut {
    fn into_message(self) -> Message {
        match self {
            WsOut::Text(text) => Message::Text(text.into()),
            WsOut::Binary(bytes) => Message::Binary(bytes.into()),
            WsOut::Close(code, reason) => Message::Close(Some(CloseFrame {
                code,
                reason: reason.into(),
            })),
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
            let delivery = match body {
                InvokeResponseBody::Json(text) => {
                    let message = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
                    let frame = serde_json::json!({ "t": "chan", "channelId": channel_id, "index": index, "message": message }).to_string();
                    sink_out.send(WsOut::Text(frame))
                }
                InvokeResponseBody::Raw(bytes) => sink_out.send(WsOut::Binary(channel_binary_frame(channel_id, index, &bytes))),
            };
            delivery.map_err(|_| {
                tauri::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "원격 웹소켓 연결이 종료되어 채널로 전달할 수 없습니다",
                ))
            })
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
/// loop against two independent session-invalidation signals:
///
/// - [`RemoteStore::subscribe_session_epoch`] — a *bulk* revocation
///   (revoke-all, a password change, or server stop) closes every open
///   socket immediately, regardless of which session each one authenticated
///   under.
/// - a `sleep_until` deadline at `session_digest`'s own
///   `REMOTE_SESSION_TTL_MS` expiry (`session_digest` is what
///   `ws_upgrade_route` extracted from the upgrade request's session
///   cookie) — closes *this* socket specifically the moment its own session
///   ages out, even though nothing else changed. Without this, a socket that
///   was live before its session's 7-day mark would otherwise stay connected
///   indefinitely (the epoch above only fires on bulk revocation, not
///   natural per-session expiry) — see
///   `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §3.1. The
///   deadline-fired branch re-checks [`RemoteStore::has_active_session_digest`]
///   rather than closing unconditionally, both to log accurately and — as a
///   side effect of that check — to prune the now-stale entry out of
///   `RemoteStore`'s session map immediately instead of waiting for the next
///   [`RemoteStore::sweep_expired_sessions`] pass. The close carries
///   `REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED`/`REMOTE_WS_CLOSE_REASON_SESSION_EXPIRED`
///   so the frontend can tell this apart from an ordinary network drop and
///   redirect to the login page instead of silently retrying forever.
///
/// Once the main loop above breaks, the writer task's own shutdown is bounded by
/// [`REMOTE_WS_WRITER_SHUTDOWN_TIMEOUT_MS`] rather than awaited unconditionally — see that
/// constant's doc comment for why an unbounded wait can park [`RemoteStore::client_disconnected`]
/// forever. A normal shutdown (including the session-expiry `Close` frame queued onto `tx` right
/// before it's dropped) still flushes well within the timeout in the common case — but see that
/// constant's doc for a second path (a permit-queued request's own `tx` clone, under a saturated
/// dispatch limiter) that can also ride the full timeout, not just the traffic-idle leaked-sender
/// case it was originally written for.
///
/// Each inbound `Message::Text` spawns its own `handle_request` task, and that task is where it
/// waits for a [`RemoteDispatchLimiter`] permit (contract 2026-08-25 §1-c) — not here, before the
/// spawn. Gating the spawn itself instead would block this function's own `tokio::select!` loop
/// from ever reaching its next `stream.next()` poll while every permit is checked out, which would
/// also stall the two session-invalidation arms documented above — a saturated dispatch limiter
/// must never delay noticing a bulk revocation or this socket's own TTL expiry.
///
/// That permit wait has costs this design accepts rather than works around (2026-08-25 d-35 review
/// findings C1/C2/C4, all downgraded to minor on adversarial re-verification — see that review for
/// the full analysis):
///
/// - The semaphore does not distinguish request kinds. When it saturates, every remote command —
///   including the cancel-class ones (`*_cancel`, `pty_kill`, `remote_revoke_sessions`) that could
///   otherwise relieve the saturation — waits in the same queue behind it. Recovery from a
///   saturated queue therefore has to come from the desktop side instead: stopping the remote
///   server or revoking sessions there does not go through this semaphore at all.
/// - A request already queued for a permit is not cancelled when either session-invalidation arm
///   above fires; it executes as soon as a permit frees up even if the session that sent it was
///   revoked or expired in the meantime. This is not new exposure: the set of requests that end up
///   executing after invalidation is the same set the pre-limiter code would have executed (the
///   read loop above `break`s on invalidation, so no *further* frames get read either way) — what
///   changed is only how long that already-fixed set takes to drain. `dispatch` itself has never
///   re-validated the session after the WebSocket upgrade; reflecting revocation into an
///   already-queued request is deferred follow-up work, not a regression this change introduced.
/// - [`RemoteDispatchLimiter::acquire`] returning `None` — a path nothing in this codebase
///   currently reaches, see that method's doc — makes the spawned task below `return` with no
///   response frame sent for that request's `seq` at all, rather than an error response. The
///   client-side promise for that `seq` stays pending until the socket itself closes, at which
///   point `remote-ws-client.ts`'s `rejectAll` clears it.
pub async fn handle_socket(socket: WebSocket, app: AppHandle, session_digest: String) {
    let remote = app.state::<RemoteStore>();
    remote.sweep_expired_sessions();
    remote.client_connected();
    let mut events = remote.subscribe_events();
    let mut session_epoch = remote.subscribe_session_epoch();
    let deadline = remote
        .session_expires_at(&session_digest)
        .map(tokio::time::Instant::from_std)
        .unwrap_or_else(tokio::time::Instant::now);

    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsOut>();

    let mut writer = tauri::async_runtime::spawn(async move {
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
                            let limiter = request_app.state::<RemoteDispatchLimiter>();
                            let Some(_permit) = limiter.acquire().await else {
                                log::warn!("원격 dispatch 세마포어를 획득하지 못해 요청을 처리하지 못했습니다");
                                return;
                            };
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
            _ = tokio::time::sleep_until(deadline) => {
                if !remote.has_active_session_digest(&session_digest) {
                    log::info!("원격 세션이 만료되어 연결을 종료합니다");
                    let _ = tx.send(WsOut::Close(REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED, REMOTE_WS_CLOSE_REASON_SESSION_EXPIRED));
                }
                break;
            }
        }
    }

    event_task.abort();
    drop(factory);
    drop(tx);
    tokio::select! {
        _ = &mut writer => {}
        _ = tokio::time::sleep(std::time::Duration::from_millis(REMOTE_WS_WRITER_SHUTDOWN_TIMEOUT_MS)) => {
            log::warn!("원격 웹소켓 writer 태스크가 제한 시간 내에 스스로 끝나지 않아 강제로 정리합니다 (도메인 스토어가 이 연결의 채널 송신자를 계속 쥐고 있을 수 있음)");
            writer.abort();
        }
    }
    remote.client_disconnected();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_변형은_코드와_사유를_담은_close_프레임으로_변환된다() {
        let message = WsOut::Close(REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED, REMOTE_WS_CLOSE_REASON_SESSION_EXPIRED).into_message();

        let Message::Close(Some(frame)) = message else {
            panic!("Close 변형은 CloseFrame을 담은 Message::Close로 변환되어야 한다");
        };
        assert_eq!(frame.code, REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED);
        assert_eq!(frame.reason.as_str(), REMOTE_WS_CLOSE_REASON_SESSION_EXPIRED);
    }

    /// R8#3≡R3#1 회귀: 웹소켓 쓰기 루프(수신자)가 사라진 뒤에도 채널 전송이
    /// `Ok(())`로 위장되면 pty/lsp 도메인의 `retain(|_, ch| ch.send(...).is_ok())`
    /// 프루닝 불변식이 깨져 끊어진 원격 채널이 스토어에 계속 남는다.
    #[test]
    fn 수신자가_사라진_채널은_송신_실패를_ok으로_위장하지_않는다() {
        let (tx, rx) = mpsc::unbounded_channel::<WsOut>();
        let sink = make_channel_factory(tx)("1".to_string());
        drop(rx);

        let result = sink(InvokeResponseBody::Json("{}".to_string()));

        assert!(
            result.is_err(),
            "수신자가 사라지면 채널 전송은 Err 를 반환해 프루닝 대상임을 알려야 한다"
        );
    }

    #[test]
    fn 수신자가_살아있는_채널은_송신_성공을_ok으로_반환한다() {
        let (tx, mut rx) = mpsc::unbounded_channel::<WsOut>();
        let sink = make_channel_factory(tx)("1".to_string());

        let result = sink(InvokeResponseBody::Json("{}".to_string()));

        assert!(result.is_ok());
        assert!(matches!(rx.try_recv(), Ok(WsOut::Text(_))));
    }

    /// §1.3(3) 회귀: `make_channel_factory` 가 만든 싱크는 `ws_out`(`handle_socket` 의 `tx`) 의
    /// clone 을 내부에 캡처한다 — 도메인 스토어(LSP/검색/AI/pty 세션 등)가 이 싱크를 계속 쥐고
    /// 있으면, `handle_socket` 이 자신의 로컬 `tx` 를 명시적으로 drop 해도 writer 태스크의
    /// `rx.recv()` 는 그 살아있는 clone 때문에 영원히 완료되지 않는다. 이 테스트는 그 메커니즘
    /// 자체가 실재함을 최소 재현해, `handle_socket` 의 유한 대기(+강제 종료) 방어가 왜
    /// 필요한지 고정한다.
    #[tokio::test]
    async fn 채널_싱크가_송신자를_쥐고_있으면_writer는_스스로_끝나지_않는다() {
        let (tx, mut rx) = mpsc::unbounded_channel::<WsOut>();
        let leaked_sink = make_channel_factory(tx.clone())("1".to_string());
        drop(tx);

        let mut writer = tauri::async_runtime::spawn(async move { while rx.recv().await.is_some() {} });

        let finished = tokio::select! {
            _ = &mut writer => true,
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => false,
        };

        assert!(
            !finished,
            "도메인 스토어가 쥔 채널 싱크(tx clone 내장)가 살아있는 한 writer 는 스스로 끝나면 안 된다 — 버그 상황 재현"
        );

        drop(leaked_sink);
        writer.abort();
    }
}
