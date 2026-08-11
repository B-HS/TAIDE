use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

pub const REMOTE_SESSION_COOKIE_NAME: &str = "taide_remote_session";
pub const REMOTE_LINK_TOKEN_QUERY_KEY: &str = "t";
pub const REMOTE_BROADCAST_CHANNEL_CAPACITY: usize = 256;
pub const REMOTE_SHUTDOWN_GRACE_MS: u64 = 2_000;
pub const REMOTE_HANDSHAKE_TIMEOUT_MS: u64 = 10_000;

pub const REMOTE_CHANNEL_PREFIX: &str = "__CHANNEL__:";
pub const REMOTE_BINARY_TAG_RESPONSE: u8 = 0x02;
pub const REMOTE_BINARY_TAG_CHANNEL: u8 = 0x01;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    pub running: bool,
    pub port: u32,
    pub client_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLinkInfo {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteRequest {
    pub seq: u32,
    pub command: String,
    #[serde(default)]
    pub args: Value,
}
