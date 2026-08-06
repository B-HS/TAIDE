use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const RESTART_BACKOFF_LIMIT: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LspServerId {
    Vtsls,
    RustAnalyzer,
    BasedPyright,
    Ruff,
    Marksman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LspSessionStatus {
    Starting,
    Running,
    Crashed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LanguageServerSpec {
    pub id: LspServerId,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub language_ids: Vec<String>,
    #[serde(default)]
    pub shares_sessions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LspServerDetection {
    pub id: LspServerId,
    pub name: String,
    #[serde(default)]
    pub resolved_path: Option<String>,
    pub available: bool,
    #[serde(default)]
    pub install_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LspSessionInfo {
    pub session_id: String,
    pub project_id: ProjectId,
    pub server_id: LspServerId,
    pub root: String,
    pub status: LspSessionStatus,
    #[serde(default)]
    pub last_error: Option<String>,
}
