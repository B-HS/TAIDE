use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IdeStatus {
    pub running: bool,
    pub port: u32,
    pub connected: bool,
    pub client_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IdeDiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IdeDiagnostic {
    pub path: String,
    pub severity: IdeDiagnosticSeverity,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub message: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum IdeDiffOutcome {
    Saved,
    Rejected,
    TabClosed,
}

/// `owner` (`getCurrentWindow().label` on the frontend — `main`/`editor-<n>`, or the remote client's
/// fixed `domain::remote::types::REMOTE_OWNER_LABEL`) lets [`ide_set_selection`](super::commands::ide_set_selection)
/// tell a real desktop window's selection apart from a remote session's — see the doc comment there
/// for why a remote-sourced selection must never reach [`IdeStore`](super::commands::IdeStore)'s
/// desktop-facing slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IdeSelectionInput {
    pub owner: String,
    pub project_id: ProjectId,
    pub path: String,
    pub text: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub is_empty: bool,
}
