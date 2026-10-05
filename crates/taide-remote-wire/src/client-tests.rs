use serde_json::{json, Value};

use super::*;
use crate::protocol::{
    channel_binary_frame, channel_end_frame, channel_json_frame, response_binary_frame,
    response_frame,
};

const NORMAL_CLOSE: u16 = 1000;
const RESTART_CLOSE: u16 = 1012;
const CHANNEL_ID: u32 = 17;

fn queued_commands(opened: &Opened) -> Vec<String> {
    opened
        .queued
        .iter()
        .map(|text| serde_json::from_str::<RemoteRequest>(text).unwrap().command)
        .collect()
}

#[test]
fn 초기_실패와_재연결은_기존_mutation을_폐기하고_새_요청만_fifo로_보낸다() {
    let mut client = Client::new();
    let old = client.connection();
    let mutation = client
        .invoke("file_rename", json!({"path":"synthetic.txt"}))
        .unwrap();
    assert!(mutation.send.is_none());
    assert_eq!(
        client.closed(old, NORMAL_CLOSE),
        Some(Closed {
            rejected: vec![mutation.seq],
            action: AfterClose::ReconnectAfter(REMOTE_RECONNECT_DELAY_MS),
        })
    );
    let first = client.invoke("file_open", Value::Null).unwrap();
    let second = client.invoke("settings_get", Value::Null).unwrap();
    assert!(first.seq > mutation.seq && second.seq > first.seq);
    let current = client.reconnect().unwrap();
    assert!(client.opened(old).is_none());
    assert!(client.closed(old, NORMAL_CLOSE).is_none());
    assert!(client
        .text(old, &response_frame(first.seq, true, Value::Null))
        .is_none());
    let opened = client.opened(current).unwrap();
    assert_eq!(queued_commands(&opened), ["file_open", "settings_get"]);
    assert!(opened.recovered);
    assert!(client.opened(current).is_none());
    let attach = client
        .invoke("pty_attach", json!({"channel":"__CHANNEL__:17"}))
        .unwrap();
    assert!(attach.send.is_some());
    assert_eq!(
        client.closed(current, RESTART_CLOSE).unwrap().rejected,
        [first.seq, second.seq, attach.seq]
    );
    let next = client.reconnect().unwrap();
    let reopened = client.opened(next).unwrap();
    assert!(reopened.queued.is_empty());
    assert!(reopened.recovered);
    let reattach = client.invoke("pty_attach", Value::Null).unwrap();
    assert!(reattach.seq > attach.seq);
    assert!(reattach.send.is_some());
    assert_eq!(client.dispose(), [reattach.seq]);
    assert!(client.reconnect().is_none());
    assert!(client.dispose().is_empty());
}

#[test]
fn 응답은_한번만_완료하고_불완전_frame과_옛_connection은_pending을_소비하지_않는다() {
    let mut client = Client::new();
    let connection = client.connection();
    assert!(!client.opened(connection).unwrap().recovered);
    let request = client.invoke("file_read_raw", Value::Null).unwrap();
    assert!(client.binary(connection, &[2, 0]).is_none());
    assert!(client.text(connection, "not-json").is_none());
    assert!(client
        .text(
            connection,
            &format!(
                "{{\"t\":\"resp\",\"seq\":{},\"ok\":\"true\",\"payload\":null}}",
                request.seq
            )
        )
        .is_none());
    let bytes = response_binary_frame(request.seq, &[0, 255]);
    assert_eq!(
        client.binary(connection, &bytes),
        Some(Delivery::Response {
            seq: request.seq,
            result: Ok(ResponsePayload::Binary(vec![0, 255])),
        })
    );
    assert!(client.binary(connection, &bytes).is_none());
    assert!(client
        .text(connection, &response_frame(request.seq, true, Value::Null))
        .is_none());
    let request = client.invoke("settings_get", Value::Null).unwrap();
    let error = json!({"code":"Forbidden"});
    assert_eq!(
        client.text(
            connection,
            &response_frame(request.seq, false, error.clone())
        ),
        Some(Delivery::Response {
            seq: request.seq,
            result: Err(error),
        })
    );
    let request = client.invoke("settings_get", Value::Null).unwrap();
    assert_eq!(
        client.text(connection, &response_frame(request.seq, true, Value::Null)),
        Some(Delivery::Response {
            seq: request.seq,
            result: Ok(ResponsePayload::Json(Value::Null)),
        })
    );
    assert_eq!(
        client.text(
            connection,
            &channel_json_frame(CHANNEL_ID, 0, json!({"title":"synthetic"}))
        ),
        Some(Delivery::ChannelJson {
            channel_id: CHANNEL_ID,
            index: 0,
            message: json!({"title":"synthetic"}),
        })
    );
    assert_eq!(
        client.binary(connection, &channel_binary_frame(CHANNEL_ID, 1, b"pty")),
        Some(Delivery::ChannelBinary {
            channel_id: CHANNEL_ID,
            index: 1,
            bytes: b"pty".to_vec(),
        })
    );
    assert_eq!(
        client.text(connection, &channel_end_frame(CHANNEL_ID, 2)),
        Some(Delivery::ChannelEnd {
            channel_id: CHANNEL_ID,
            index: 2
        })
    );
    let payload = json!({"active":true}).to_string();
    let event = json!({"t":"event","event":"settings-changed","payload":payload}).to_string();
    assert_eq!(
        client.text(connection, &event),
        Some(Delivery::Event {
            event: "settings-changed".into(),
            payload
        })
    );
    assert!(client
        .closed(connection, NORMAL_CLOSE)
        .unwrap()
        .rejected
        .is_empty());
    assert!(client
        .binary(connection, &channel_binary_frame(CHANNEL_ID, 3, b"late"))
        .is_none());
}

#[test]
fn 세션_만료와_dispose는_로그인_또는_종료로_수렴하고_seq를_재사용하지_않는다() {
    let mut client = Client::new();
    let connection = client.connection();
    let request = client.invoke("file_rename", Value::Null).unwrap();
    assert_eq!(
        client.closed(connection, REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED),
        Some(Closed {
            rejected: vec![request.seq],
            action: AfterClose::Authenticate
        })
    );
    assert!(client.reconnect().is_none());
    assert!(client.opened(connection).is_none());
    assert!(matches!(
        client.invoke("file_open", Value::Null),
        Err(InvokeError::SessionExpired)
    ));
    assert!(client.dispose().is_empty());
    assert!(matches!(
        client.invoke("file_open", Value::Null),
        Err(InvokeError::Closed)
    ));
    let mut client = Client::new();
    client.next_seq = Some(u32::MAX);
    let request = client.invoke("settings_get", Value::Null).unwrap();
    assert_eq!(request.seq, u32::MAX);
    assert!(matches!(
        client.invoke("file_rename", Value::Null),
        Err(InvokeError::SequenceExhausted)
    ));
    assert_eq!(client.dispose(), [u32::MAX]);
    assert!(client.outbox.is_empty() && client.pending.is_empty());
}
