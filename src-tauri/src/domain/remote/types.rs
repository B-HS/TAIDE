use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

/// Single-owner wildcard-prefix syntax for a `remote_allowed_hosts` entry (RFC 6125 single-label
/// wildcard, matched by `service::host_matches_allowed_entry`) — `settings::service::
/// is_valid_allowed_host` (sanitizing user input) and `service`'s matcher/link-formatting callers
/// all read the same constant instead of each hardcoding `"*."`. Lives in `types` (not `service`)
/// so the settings domain's reference stays a data-shape reference rather than a cross-domain
/// `service::` path (T1-I §1.0).
pub const ALLOWED_HOST_WILDCARD_PREFIX: &str = "*.";

pub const REMOTE_SESSION_COOKIE_NAME: &str = "taide_remote_session";
pub const REMOTE_LOGIN_NONCE_COOKIE_NAME: &str = "taide_remote_login_nonce";
pub const REMOTE_LINK_TOKEN_QUERY_KEY: &str = "t";
pub const REMOTE_LOGIN_PATH: &str = "/__taide/login";
pub const REMOTE_BROADCAST_CHANNEL_CAPACITY: usize = 256;
pub const REMOTE_SHUTDOWN_GRACE_MS: u64 = 2_000;
pub const REMOTE_HANDSHAKE_TIMEOUT_MS: u64 = 10_000;

/// Upper bound `ws.rs::handle_socket` waits for its own writer task to drain and exit after the
/// connection's main loop breaks, before aborting it outright. The writer task only exits once every
/// clone of its `UnboundedSender<WsOut>` is dropped — besides `handle_socket`'s own `tx` (dropped
/// right before this wait), a domain store (`LspStore`/`SearchStore`/`AiRequestStore`/pty subscribers)
/// can still be holding a `ChannelSink` closure that captured its own clone via
/// `make_channel_factory`, for a session this connection spawned that never gets pruned (nothing ever
/// attempts a `send` to it again — a traffic-idle remote session's own channel). Without this bound,
/// that leaked clone keeps the writer's `rx.recv().await` parked forever, which in turn keeps
/// `handle_socket` from ever reaching `RemoteStore::client_disconnected()` — the client count and any
/// pruning gated on this connection actually ending stay stuck (§1.3(3),
/// `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md`).
pub const REMOTE_WS_WRITER_SHUTDOWN_TIMEOUT_MS: u64 = 3_000;

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

/// Fixed window-label the remote (browser) client always reports as its caller-supplied `owner` on
/// window-scoped IPC calls — `getCurrentWindow().label` returns this constant in the remote-mirror
/// shim (`src/shared/lib/remote/tauri-internals-shim.ts::REMOTE_WINDOW_LABEL`), the same label
/// `domain::lsp::commands::SessionEntry.channels` already keys reuse by. Domains that must keep a
/// remote session's writes from being mistaken for a real desktop window's — e.g.
/// `domain::ide::store::IdeStore`'s selection state, which must reflect only the local desktop
/// editor for the local IDE MCP protocol (`ide::server`) — compare their caller-supplied `owner`
/// against this constant rather than duplicating the literal.
///
/// That comparison alone is not the trust boundary: a value read straight off a remote request is
/// client-controlled, so a domain that merely checks `owner != REMOTE_OWNER_LABEL` can be defeated by
/// a client that simply never sends the real label (`owner: "main"`). The actual enforcement point is
/// `remote::dispatch::enforce_remote_owner_label`, which force-overwrites every `"owner"` key in a
/// remote request's `args` (top-level or nested) with this constant before any handler — including the
/// comparisons above — ever sees the value. The per-domain comparisons stay in place as defense in
/// depth (and as the actual behavioral no-op for a legitimate remote call), but a remote caller can no
/// longer make `owner` say anything else in the first place.
pub const REMOTE_OWNER_LABEL: &str = "remote";

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
