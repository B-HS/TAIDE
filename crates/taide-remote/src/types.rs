pub use taide_model::remote::ALLOWED_HOST_WILDCARD_PREFIX;

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
