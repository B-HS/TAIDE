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
///
/// A second, unrelated way to hit this same timeout was added by [`crate::domain::remote::commands::
/// RemoteDispatchLimiter`] (contract 2026-08-25 §1-c): `ws.rs::handle_socket` clones `tx` into each
/// spawned per-request task *before* that task waits on the limiter's permit, so a request still
/// queued for a permit when the connection closes keeps its own `tx` clone alive for as long as it
/// stays queued. Under a saturated dispatch limiter, an otherwise-ordinary disconnect can ride this
/// timeout too — this is no longer exclusively the traffic-idle leaked-sender case above.
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

/// Upper bound on concurrently in-flight remote-dispatch requests (`ws.rs::handle_socket`'s
/// per-message `tauri::async_runtime::spawn`), enforced by [`crate::domain::remote::commands::
/// RemoteDispatchLimiter`] (contract 2026-08-25 §1-c). Before this cap, that spawn was unbounded:
/// a client sending requests faster than they complete (a retry storm, a runaway frontend loop, or
/// simply several remote tabs open at once) could drive concurrent in-flight tasks arbitrarily
/// high, and `docs/acknowledge/2026-08-19-audit-t1h-lock-io-contract.md` §5's `git_pull` deadlock
/// finding established that 512 concurrent calls are analytically reachable that way — derived from
/// tokio's default `max_blocking_threads` (see below), not measured in a live stress run.
///
/// This app never overrides Tauri's default async runtime (`tauri::async_runtime::default_runtime`
/// builds it via plain `tokio::runtime::Runtime::new()` — no `Builder::max_blocking_threads` call
/// anywhere in this codebase), and tokio's own documented default for `Builder::max_blocking_threads`
/// is 512. That blocking-thread pool is a single process-wide resource shared by *every*
/// `spawn_blocking` call regardless of origin — a desktop-invoked command through Tauri's own IPC
/// and a remote-invoked one through this dispatch table draw from the exact same 512 slots. 128 is
/// simply a quarter of that one number; the T1-H citation above is not an independent confirmation
/// of it — T1-H's 512 is itself derived from the same tokio default, not a second, separate
/// measurement.
///
/// This cap keeps concurrent remote-dispatch tasks well under that shared ceiling, intended to leave
/// headroom for the desktop window's own concurrent blocking work (terminal PTYs, LSP installs, git
/// operations, ...) that can be in flight at the same time. That headroom is not a hard reservation,
/// though: nothing in this codebase bounds how many blocking threads the desktop side can occupy at
/// once (every desktop-originated `spawn_blocking` call is itself uncapped), so a large enough
/// desktop-side burst can still exhaust the pool regardless of what the remote side is doing — this
/// cap only bounds the remote side's own contribution. In ordinary use it's generous enough for
/// legitimate remote traffic: a single UI interaction (opening a project, switching a tab) fans out
/// to at most a handful of concurrent commands, so even several simultaneously connected remote
/// sessions stay far below this number. A request that arrives once the cap is saturated **waits**
/// for a permit rather than being rejected — no behavior change for the caller, only bounded queuing
/// instead of unbounded concurrency (contract §1-c: "초과 시 대기 — 거부 금지").
///
/// This cap is also the only thing keeping [`crate::state::AppState::begin_mutation_blocking`]'s
/// GIT-1 deadlock class (Phase E) unreachable through the remote path: its sole remaining caller,
/// `search_replace`, is remote-exposed, so the number of `begin_mutation_blocking` parks a remote
/// client can create at once is bounded by how many concurrent `search_replace` calls it can have
/// in flight — which this semaphore caps at 128, well under the 512-thread pool. Raising this
/// constant toward 512 (or removing it) would need re-evaluating that margin alongside
/// `begin_mutation_blocking`'s guard-holder count — see that method's doc.
pub const REMOTE_DISPATCH_MAX_CONCURRENT: usize = 128;

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
