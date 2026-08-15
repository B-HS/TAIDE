use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

pub const REMOTE_SESSION_COOKIE_NAME: &str = "taide_remote_session";
pub const REMOTE_LOGIN_NONCE_COOKIE_NAME: &str = "taide_remote_login_nonce";
pub const REMOTE_LINK_TOKEN_QUERY_KEY: &str = "t";
pub const REMOTE_LOGIN_PATH: &str = "/__taide/login";
pub const REMOTE_BROADCAST_CHANNEL_CAPACITY: usize = 256;
pub const REMOTE_SHUTDOWN_GRACE_MS: u64 = 2_000;
pub const REMOTE_HANDSHAKE_TIMEOUT_MS: u64 = 10_000;

/// How long an established remote session cookie stays valid without the
/// device reconnecting (7 days).
pub const REMOTE_SESSION_TTL_MS: u64 = 7 * 24 * 60 * 60 * 1_000;

/// How long a login nonce (the "form pass" minted when a one-time link token
/// is consumed while a password is configured) stays valid. Long enough for
/// a few password retries, short enough that a stale tab can't be replayed.
pub const REMOTE_LOGIN_NONCE_TTL_MS: u64 = 5 * 60 * 1_000;

/// Consecutive login failures allowed before the exponential-backoff lockout
/// engages.
pub const REMOTE_LOGIN_MAX_ATTEMPTS: u32 = 5;
/// Lockout duration for the first failure past `REMOTE_LOGIN_MAX_ATTEMPTS`,
/// doubling with every further failure up to `REMOTE_LOGIN_LOCKOUT_MAX_MS`.
pub const REMOTE_LOGIN_LOCKOUT_BASE_MS: u64 = 1_000;
/// Upper bound for the exponential-backoff lockout duration.
pub const REMOTE_LOGIN_LOCKOUT_MAX_MS: u64 = 60_000;

/// Minimum accepted length (in `chars`, after trimming) for a newly set
/// remote-access password. Only enforced on write (`remote_set_password`) —
/// a password already stored below this length keeps working until the user
/// changes it.
pub const REMOTE_PASSWORD_MIN_LEN: usize = 8;

/// Hostnames that always resolve to this loopback-only server regardless of
/// `Settings::remote_allowed_hosts` — the server only ever binds `127.0.0.1`
/// (see `commands.rs::bind_and_start`), so these three aliases for it are
/// permanently allowed and are never exposed to sync/dispatch stripping the
/// way the user-registered tunnel hosts are.
pub const REMOTE_LOOPBACK_HOSTNAMES: &[&str] = &["127.0.0.1", "localhost", "::1"];

/// WebSocket close code sent when an individual session's `REMOTE_SESSION_TTL_MS`
/// expires while a socket authenticated under it is still open. A private-use
/// code (RFC 6455 reserves 4000-4999 for applications) so the frontend can
/// distinguish this from an ordinary network drop and redirect to the login
/// page instead of silently retrying — see `ws.rs::handle_socket`.
pub const REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED: u16 = 4001;
pub const REMOTE_WS_CLOSE_REASON_SESSION_EXPIRED: &str = "session_expired";

pub const REMOTE_CHANNEL_PREFIX: &str = "__CHANNEL__:";
pub const REMOTE_BINARY_TAG_RESPONSE: u8 = 0x02;
pub const REMOTE_BINARY_TAG_CHANNEL: u8 = 0x01;

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
