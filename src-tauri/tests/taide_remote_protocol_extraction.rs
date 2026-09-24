#[test]
fn remote_바이너리와_json_프레임은_기존_wire_형식을_유지한다() {
    assert_eq!(
        taide_remote::protocol::channel_binary_frame(0x01020304, 0x05060708, &[0xaa, 0xbb]),
        [0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0xaa, 0xbb]
    );
    assert_eq!(
        taide_remote::protocol::response_binary_frame(0x01020304, &[0xaa, 0xbb]),
        [0x02, 0x01, 0x02, 0x03, 0x04, 0xaa, 0xbb]
    );

    let payload = serde_json::json!({ "message": "ok" });
    let response = taide_remote::protocol::response_frame(7, true, payload.clone());
    let parsed: serde_json::Value = serde_json::from_str(&response).expect("유효한 JSON 응답");
    assert_eq!(parsed, serde_json::json!({ "t": "resp", "seq": 7, "ok": true, "payload": payload }));

    assert_eq!(
        taide_remote::types::REMOTE_CHANNEL_PREFIX,
        taide_lib::domain::remote::types::REMOTE_CHANNEL_PREFIX
    );
    assert_eq!(
        taide_remote::types::REMOTE_BINARY_TAG_CHANNEL,
        taide_lib::domain::remote::types::REMOTE_BINARY_TAG_CHANNEL
    );
    assert_eq!(
        taide_remote::types::REMOTE_BINARY_TAG_RESPONSE,
        taide_lib::domain::remote::types::REMOTE_BINARY_TAG_RESPONSE
    );
}

#[test]
fn remote_채널_json과_종료_프레임은_기존_wire_형식을_유지한다() {
    let message = serde_json::json!({ "text": "escaped \" value" });
    let channel = taide_remote::protocol::channel_json_frame(7, 2, message.clone());
    let parsed: serde_json::Value = serde_json::from_str(&channel).expect("유효한 채널 JSON");
    assert_eq!(
        parsed,
        serde_json::json!({ "t": "chan", "channelId": 7, "index": 2, "message": message })
    );

    let end = taide_remote::protocol::channel_end_frame(7, 3);
    let parsed: serde_json::Value = serde_json::from_str(&end).expect("유효한 채널 종료 JSON");
    assert_eq!(parsed, serde_json::json!({ "t": "chanEnd", "channelId": 7, "index": 3 }));
}
