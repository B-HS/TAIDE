use serde_json::{json, Value};

use crate::protocol::{
    channel_binary_frame, channel_end_frame, channel_json_frame, decode_binary,
    response_binary_frame, response_frame, BinaryFrame, RemoteRequest, TextFrame,
};

const SEQUENCE: u32 = 0x1234_5678;
const CHANNEL: u32 = 0x90ab_cdef;
const INDEX: u32 = 0xfedc_ba98;
const PAYLOAD: &[u8] = &[0, 255, 12];

#[test]
fn 서버_wire는_동일한_json과_big_endian_binary를_브라우저에서_읽는다() {
    let response = response_binary_frame(SEQUENCE, PAYLOAD);
    assert_eq!(response, [2, 0x12, 0x34, 0x56, 0x78, 0, 255, 12]);
    assert_eq!(
        decode_binary(&response),
        Some(BinaryFrame::Response {
            seq: SEQUENCE,
            bytes: PAYLOAD
        })
    );
    let channel = channel_binary_frame(CHANNEL, INDEX, PAYLOAD);
    assert_eq!(
        channel,
        [1, 0x90, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0, 255, 12]
    );
    assert_eq!(
        decode_binary(&channel),
        Some(BinaryFrame::Channel {
            channel_id: CHANNEL,
            index: INDEX,
            bytes: PAYLOAD
        })
    );
    for length in 0..5 {
        assert_eq!(decode_binary(&response[..length]), None);
    }
    for length in 0..9 {
        assert_eq!(decode_binary(&channel[..length]), None);
    }
    assert_eq!(decode_binary(&[255, 0, 0, 0, 1]), None);
    assert_eq!(
        decode_binary(&response[..5]),
        Some(BinaryFrame::Response {
            seq: SEQUENCE,
            bytes: &[]
        })
    );
    let body = json!({"name":"합성", "value":null});
    assert_eq!(
        serde_json::from_str::<TextFrame>(&response_frame(SEQUENCE, true, body.clone())).unwrap(),
        TextFrame::Response {
            seq: SEQUENCE,
            ok: true,
            payload: body.clone()
        }
    );
    assert_eq!(
        serde_json::from_str::<TextFrame>(&channel_json_frame(CHANNEL, INDEX, body.clone()))
            .unwrap(),
        TextFrame::Channel {
            channel_id: CHANNEL,
            index: INDEX,
            message: body
        }
    );
    assert_eq!(
        serde_json::from_str::<TextFrame>(&channel_end_frame(CHANNEL, INDEX)).unwrap(),
        TextFrame::ChannelEnd {
            channel_id: CHANNEL,
            index: INDEX
        }
    );
    let request: RemoteRequest =
        serde_json::from_value(json!({"seq":SEQUENCE, "command":"settings_get"})).unwrap();
    assert_eq!(request.args, Value::Null);
    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({"seq":SEQUENCE, "command":"settings_get", "args":null})
    );
    for invalid in [
        json!({"t":"chan", "channelId":-1, "index":0, "message":null}),
        json!({"t":"resp", "seq":4294967296u64, "ok":true, "payload":null}),
        json!({"t":"resp", "seq":1, "ok":"true", "payload":null}),
        json!({"t":"event", "event":"settings-changed", "payload":{}}),
        json!({"t":"chanEnd", "index":0}),
    ] {
        assert!(serde_json::from_value::<TextFrame>(invalid).is_err());
    }
}
