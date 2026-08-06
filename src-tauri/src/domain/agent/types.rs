use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const AGENT_POLL_UNIX_MS: u64 = 1_000;
pub const AGENT_POLL_WINDOWS_MS: u64 = 2_000;
pub const KNOWN_AGENT_NAMES: &[&str] = &["claude", "codex", "gemini"];
pub const WAIT_MARKER_PREFIX: &str = "taide-wait-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectedAgent {
    pub session_id: String,
    pub name: String,
    pub pid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgents {
    pub project_id: ProjectId,
    pub agents: Vec<DetectedAgent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExternalOpenRequest {
    pub path: String,
    #[serde(default)]
    pub wait_marker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallStatus {
    pub installed: bool,
    #[serde(default)]
    pub resolved_path: Option<String>,
    pub target_path: String,
    pub editor_env_hint: String,
}
