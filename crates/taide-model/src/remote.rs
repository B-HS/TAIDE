use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

pub const ALLOWED_HOST_WILDCARD_PREFIX: &str = "*.";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    pub running: bool,
    pub port: u32,
    pub client_count: u32,
    pub password_configured: bool,
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
