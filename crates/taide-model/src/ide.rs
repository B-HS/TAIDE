use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

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
/// fixed `domain::remote::types::REMOTE_OWNER_LABEL`) lets `ide_set_selection`
/// tell a real desktop window's selection apart from a remote session's — see the doc comment there
/// for why a remote-sourced selection must never reach `IdeStore`'s
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
