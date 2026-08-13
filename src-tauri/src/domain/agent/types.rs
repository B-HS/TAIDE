use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const AGENT_POLL_UNIX_MS: u64 = 500;
pub const AGENT_POLL_WINDOWS_MS: u64 = 2_000;
pub const KNOWN_AGENT_NAMES: &[&str] = &["claude", "codex", "gemini"];
pub const WAIT_MARKER_PREFIX: &str = "taide-wait-";

pub const ACTIVITY_WORKING_HOLD_MS: u64 = 2_000;
pub const ACTIVITY_IDLE_QUIET_MS: u64 = 6_000;
pub const HOOK_OVERRIDE_STALE_MS: u64 = 900_000;

pub const HOOKS_HTTP_PATH: &str = "/claude/hook";
pub const HOOKS_TOKEN_QUERY_KEY: &str = "token";
pub const HOOKS_AGENT_QUERY_KEY: &str = "agent";
pub const HOOKS_URL_MARKER: &str = "taide=1";
pub const HOOKS_HTTP_TIMEOUT_SECONDS: u64 = 5;
pub const HOOKS_READ_TIMEOUT_MS: u64 = 5_000;
pub const MAX_HOOKS_REQUEST_BYTES: usize = 65_536;

pub const HOOK_HANDLER_TYPE_HTTP: &str = "http";
pub const HOOK_HANDLER_TYPE_COMMAND: &str = "command";
pub const CODEX_HOOK_COMMAND_TIMEOUT_SECONDS: u64 = HOOKS_HTTP_TIMEOUT_SECONDS;
pub const GEMINI_HOOK_COMMAND_TIMEOUT_MS: u64 = HOOKS_HTTP_TIMEOUT_SECONDS * 1_000;

pub const AGENT_NAME_CLAUDE: &str = "claude";
pub const AGENT_NAME_CODEX: &str = "codex";
pub const AGENT_NAME_GEMINI: &str = "gemini";

pub const HOOK_EVENT_USER_PROMPT_SUBMIT: &str = "UserPromptSubmit";
pub const HOOK_EVENT_NOTIFICATION: &str = "Notification";
pub const HOOK_EVENT_STOP: &str = "Stop";
pub const MANAGED_HOOK_EVENTS: &[&str] = &[HOOK_EVENT_USER_PROMPT_SUBMIT, HOOK_EVENT_NOTIFICATION, HOOK_EVENT_STOP];

pub const HOOK_EVENT_PERMISSION_REQUEST: &str = "PermissionRequest";
pub const HOOK_EVENT_POST_TOOL_USE: &str = "PostToolUse";
pub const HOOK_EVENT_BEFORE_AGENT: &str = "BeforeAgent";
pub const HOOK_EVENT_AFTER_AGENT: &str = "AfterAgent";

pub const CODEX_MANAGED_HOOK_EVENTS: &[&str] = &[
    HOOK_EVENT_USER_PROMPT_SUBMIT,
    HOOK_EVENT_PERMISSION_REQUEST,
    HOOK_EVENT_POST_TOOL_USE,
    HOOK_EVENT_STOP,
];
pub const GEMINI_MANAGED_HOOK_EVENTS: &[&str] = &[HOOK_EVENT_BEFORE_AGENT, HOOK_EVENT_NOTIFICATION, HOOK_EVENT_AFTER_AGENT];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AgentActivity {
    Idle,
    Working,
    AwaitingInput,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetectedAgent {
    pub session_id: String,
    pub name: String,
    pub pid: u32,
    pub activity: AgentActivity,
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
