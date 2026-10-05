pub mod client;
pub mod protocol;

pub const REMOTE_LOGIN_PATH: &str = "/__taide/login";
pub const REMOTE_CHANNEL_PREFIX: &str = "__CHANNEL__:";
pub const REMOTE_OWNER_LABEL: &str = "remote";
pub const REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED: u16 = 4001;
pub const REMOTE_RECONNECT_DELAY_MS: u32 = 1_000;
pub const REMOTE_BINARY_TAG_RESPONSE: u8 = 0x02;
pub const REMOTE_BINARY_TAG_CHANNEL: u8 = 0x01;

#[cfg(test)]
mod tests;
