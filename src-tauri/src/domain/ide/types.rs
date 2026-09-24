pub const IDE_NAME: &str = "TAIDE";
pub const IDE_TRANSPORT: &str = "ws";
pub const IDE_AUTH_HEADER_NAME: &str = "X-Claude-Code-Ide-Authorization";
pub const IDE_PORT_RANGE_START: u32 = 10_000;
pub const IDE_PORT_RANGE_END: u32 = 65_535;
pub const IDE_PORT_BIND_MAX_ATTEMPTS: u32 = 20;
pub const IDE_SAVE_TIMEOUT_MS: u64 = 5_000;
pub const IDE_DIFF_TIMEOUT_MS: u64 = 600_000;
pub const IDE_HANDSHAKE_TIMEOUT_MS: u64 = 10_000;
pub const IDE_RECONCILE_INTERVAL_MS: u64 = 1_000;
pub const CLAUDE_CODE_SSE_PORT_ENV: &str = "CLAUDE_CODE_SSE_PORT";
pub const IDE_ACCEPT_RETRY_DELAY_MS: u64 = 100;
pub const MCP_SUBPROTOCOL: &str = "mcp";
pub const IDE_READY_WAIT_MS: u64 = 2_000;
pub const IDE_READY_POLL_INTERVAL_MS: u64 = 50;

pub use taide_model::ide::*;
