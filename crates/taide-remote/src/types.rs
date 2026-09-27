pub use taide_model::remote::{ALLOWED_HOST_WILDCARD_PREFIX, REMOTE_OWNER_LABEL};

pub const REMOTE_LINK_TOKEN_QUERY_KEY: &str = "t";
pub const REMOTE_LOGIN_PATH: &str = "/__taide/login";
pub const REMOTE_CHANNEL_PREFIX: &str = "__CHANNEL__:";
pub const REMOTE_BINARY_TAG_RESPONSE: u8 = 0x02;
pub const REMOTE_BINARY_TAG_CHANNEL: u8 = 0x01;

/// Minimum accepted length (in `chars`, after trimming) for a newly set
/// remote-access password. Only enforced on write (Tauri's `remote_set_password`) —
/// a password already stored below this length keeps working until the user
/// changes it.
pub const REMOTE_PASSWORD_MIN_LEN: usize = 8;

/// Hostnames that always resolve to this loopback-only server regardless of
/// `Settings::remote_allowed_hosts` — the server only ever binds `127.0.0.1`
/// (see Tauri's `domain::remote::commands::bind_and_start`), so these three aliases for it are
/// permanently allowed and are never exposed to sync/dispatch stripping the
/// way the user-registered tunnel hosts are.
pub const REMOTE_LOOPBACK_HOSTNAMES: &[&str] = &["127.0.0.1", "localhost", "::1"];

pub const REMOTE_BROADCAST_CHANNEL_CAPACITY: usize = 256;

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
