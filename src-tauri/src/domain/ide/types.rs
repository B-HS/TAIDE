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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IdeSelectionInput {
    pub project_id: ProjectId,
    pub path: String,
    pub text: String,
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
    pub is_empty: bool,
}
