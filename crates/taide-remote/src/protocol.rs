use serde_json::Value;

use crate::types::{REMOTE_BINARY_TAG_CHANNEL, REMOTE_BINARY_TAG_RESPONSE};

const CHANNEL_FRAME_HEADER_BYTES: usize = 1 + std::mem::size_of::<u32>() * 2;
const RESPONSE_FRAME_HEADER_BYTES: usize = 1 + std::mem::size_of::<u32>();

pub fn channel_binary_frame(channel_id: u32, index: u32, bytes: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(CHANNEL_FRAME_HEADER_BYTES + bytes.len());
    frame.push(REMOTE_BINARY_TAG_CHANNEL);
    frame.extend_from_slice(&channel_id.to_be_bytes());
    frame.extend_from_slice(&index.to_be_bytes());
    frame.extend_from_slice(bytes);
    frame
}

pub fn response_binary_frame(seq: u32, bytes: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(RESPONSE_FRAME_HEADER_BYTES + bytes.len());
    frame.push(REMOTE_BINARY_TAG_RESPONSE);
    frame.extend_from_slice(&seq.to_be_bytes());
    frame.extend_from_slice(bytes);
    frame
}

pub fn response_frame(seq: u32, ok: bool, payload: Value) -> String {
    serde_json::json!({ "t": "resp", "seq": seq, "ok": ok, "payload": payload }).to_string()
}

pub fn channel_json_frame(channel_id: u32, index: u32, message: Value) -> String {
    serde_json::json!({ "t": "chan", "channelId": channel_id, "index": index, "message": message })
        .to_string()
}

pub fn channel_end_frame(channel_id: u32, index: u32) -> String {
    serde_json::json!({ "t": "chanEnd", "channelId": channel_id, "index": index }).to_string()
}
