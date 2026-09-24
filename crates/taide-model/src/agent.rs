use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AgentActivity {
    Idle,
    Working,
    AwaitingInput,
    Unknown,
}

/// What an `AwaitingInput` session is waiting on, so the badge tooltip and the OS notification can
/// name one of the two instead of listing both. Derived from the latch that produced the state
/// (`service::blocked_reason`): an in-band event the agent sent names itself, while a
/// phrase read off the screen only proves some dialog is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum BlockedReason {
    Permission,
    Question,
    Dialog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectedAgent {
    pub session_id: String,
    pub name: String,
    pub pid: u32,
    pub activity: AgentActivity,
    /// Set only while the latch behind `AwaitingInput` is still held, so a reason can never outlive
    /// the block it explains. `None` on every other activity, and on a session whose
    /// `AwaitingInput` came from the project-scoped HTTP override instead of its own signals —
    /// that bridge carries an activity and nothing else.
    #[serde(default)]
    pub blocked_reason: Option<BlockedReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HookInstallScope {
    Project,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentHooksStatus {
    pub agent_name: String,
    pub scope: HookInstallScope,
    pub installed: bool,
    /// Whether this agent's install needs TAIDE's `taide` CLI symlink to exist first
    /// (`service::requires_taide_cli`). Read by the settings UI so the CLI warning and the
    /// disabled toggle follow the agent spec table instead of a second copy of it in the frontend.
    pub requires_taide_cli: bool,
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
    /// True when the symlink exists but its target can no longer be resolved
    /// (e.g. the app bundle moved or was removed) — a reinstall is needed.
    #[serde(default)]
    pub dangling: bool,
    pub target_path: String,
    pub editor_env_hint: String,
}
