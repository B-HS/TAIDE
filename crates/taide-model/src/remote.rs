use serde::{Deserialize, Serialize};
use specta::Type;

pub const ALLOWED_HOST_WILDCARD_PREFIX: &str = "*.";

pub use taide_remote_wire::protocol::RemoteRequest;
pub use taide_remote_wire::REMOTE_OWNER_LABEL;

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
