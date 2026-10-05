use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{REMOTE_BINARY_TAG_CHANNEL, REMOTE_BINARY_TAG_RESPONSE};

const SEQUENCE_BYTES: usize = std::mem::size_of::<u32>();
const CHANNEL_FRAME_HEADER_BYTES: usize = 1 + SEQUENCE_BYTES * 2;
const RESPONSE_FRAME_HEADER_BYTES: usize = 1 + SEQUENCE_BYTES;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRequest {
    pub seq: u32,
    pub command: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "t")]
pub enum TextFrame {
    #[serde(rename = "resp")]
    Response { seq: u32, ok: bool, payload: Value },
    #[serde(rename = "chan")]
    Channel {
        #[serde(rename = "channelId")]
        channel_id: u32,
        index: u32,
        message: Value,
    },
    #[serde(rename = "chanEnd")]
    ChannelEnd {
        #[serde(rename = "channelId")]
        channel_id: u32,
        index: u32,
    },
    #[serde(rename = "event")]
    Event { event: String, payload: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFrame<'a> {
    Response {
        seq: u32,
        bytes: &'a [u8],
    },
    Channel {
        channel_id: u32,
        index: u32,
        bytes: &'a [u8],
    },
}

pub fn decode_binary(frame: &[u8]) -> Option<BinaryFrame<'_>> {
    match *frame.first()? {
        REMOTE_BINARY_TAG_RESPONSE => Some(BinaryFrame::Response {
            seq: u32::from_be_bytes(frame.get(1..RESPONSE_FRAME_HEADER_BYTES)?.try_into().ok()?),
            bytes: frame.get(RESPONSE_FRAME_HEADER_BYTES..)?,
        }),
        REMOTE_BINARY_TAG_CHANNEL => Some(BinaryFrame::Channel {
            channel_id: u32::from_be_bytes(
                frame.get(1..RESPONSE_FRAME_HEADER_BYTES)?.try_into().ok()?,
            ),
            index: u32::from_be_bytes(
                frame
                    .get(RESPONSE_FRAME_HEADER_BYTES..CHANNEL_FRAME_HEADER_BYTES)?
                    .try_into()
                    .ok()?,
            ),
            bytes: frame.get(CHANNEL_FRAME_HEADER_BYTES..)?,
        }),
        _ => None,
    }
}

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
