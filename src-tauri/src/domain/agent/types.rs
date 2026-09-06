use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const AGENT_POLL_UNIX_MS: u64 = 500;
pub const AGENT_POLL_WINDOWS_MS: u64 = 2_000;
pub const KNOWN_AGENT_NAMES: &[&str] = &["claude", "codex", "gemini"];
pub const WAIT_MARKER_PREFIX: &str = "taide-wait-";

pub const ACTIVITY_WORKING_HOLD_MS: u64 = 2_000;

/// How long a session must produce nothing at all before it is called idle. Sized against the two
/// things a busy Claude Code session keeps writing even when it looks stuck — the `⏺` blink every
/// ~600ms and the spinner/elapsed-counter frames at ≤1s — so a quiet window this long means the
/// agent really stopped, while still entering idle noticeably sooner than the 6s this used to be
/// (contract 2026-09-06 §0.2·§1.2).
pub const ACTIVITY_IDLE_QUIET_MS: u64 = 4_000;

/// How long a `◐`/`◑` title glyph counts as evidence the agent is working. Claude Code alternates
/// the two roughly every 1~2s while a turn runs and freezes the last one while a permission dialog
/// is up, so anything past this window is a frozen title rather than a live one.
pub const TITLE_WORKING_FRESH_MS: u64 = 3_000;

/// How long after a keystroke reaches the pty its echo is expected back. Output inside this window
/// is the terminal drawing what the user just typed, not the agent saying something.
pub const ECHO_SUPPRESS_MS: u64 = 300;

/// How many non-whitespace, non-spinner characters a chunk must carry to count as the agent
/// actually printing something (rather than a blink/spinner frame repainting).
pub const SUBSTANTIVE_OUTPUT_MIN_CHARS: usize = 2;

pub const HOOK_OVERRIDE_STALE_MS: u64 = 900_000;

/// Version of the in-band agent event envelope (`{"v":1,…}`), also the value of the
/// `TAIDE_AGENT_PROTOCOL_VERSION` variable every pty inherits — the hook commands TAIDE installs
/// emit nothing when it is absent, so a Claude session started outside TAIDE stays silent.
pub const AGENT_PROTOCOL_VERSION: u32 = 1;

pub const AGENT_PROTOCOL_VERSION_ENV_NAME: &str = "TAIDE_AGENT_PROTOCOL_VERSION";
pub const APP_VERSION_ENV_NAME: &str = "TAIDE_APP_VERSION";

/// First Claude Code release whose hooks honor the `terminalSequence` field of a hook's JSON
/// output; older ones need the `/dev/tty` emitter instead (contract 2026-09-06 §1.4).
pub const CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION: (u32, u32, u32) = (2, 1, 141);

/// How long the one-shot `claude --version` probe may take before the emitter falls back to
/// `/dev/tty`.
pub const CLAUDE_VERSION_TIMEOUT_SECONDS: u64 = 3;

pub const HOOKS_HTTP_PATH: &str = "/claude/hook";
pub const HOOKS_TOKEN_QUERY_KEY: &str = "token";
pub const HOOKS_AGENT_QUERY_KEY: &str = "agent";
pub const HOOKS_URL_MARKER: &str = "taide=1";
pub const HOOKS_HTTP_TIMEOUT_SECONDS: u64 = 5;
pub const HOOKS_READ_TIMEOUT_MS: u64 = 5_000;
pub const MAX_HOOKS_REQUEST_BYTES: usize = 65_536;

pub const HOOK_HANDLER_TYPE_COMMAND: &str = "command";
pub const CODEX_HOOK_COMMAND_TIMEOUT_SECONDS: u64 = HOOKS_HTTP_TIMEOUT_SECONDS;
pub const GEMINI_HOOK_COMMAND_TIMEOUT_MS: u64 = HOOKS_HTTP_TIMEOUT_SECONDS * 1_000;

/// The OSC 777 `notify` prefix TAIDE's in-band agent events carry, and the size of the normalized
/// text tail the scanner hands back with every chunk. Owned by the scanner that has to recognize
/// them (infra may not reference domain — architecture.md §2) and re-exported here so the hook
/// builders that emit the sequence and the scanner that reads it share one definition.
pub use crate::infra::terminal_scan::{AGENT_OSC_MARKER, AGENT_OSC_SENTINEL, OSC_NOTIFY_IDENT, OSC_NOTIFY_SUBCOMMAND, TEXT_OVERLAP_BYTES};

pub const AGENT_NAME_CLAUDE: &str = "claude";
pub const AGENT_NAME_CODEX: &str = "codex";
pub const AGENT_NAME_GEMINI: &str = "gemini";

pub const HOOK_EVENT_USER_PROMPT_SUBMIT: &str = "UserPromptSubmit";
pub const HOOK_EVENT_NOTIFICATION: &str = "Notification";
pub const HOOK_EVENT_STOP: &str = "Stop";

pub const HOOK_EVENT_PERMISSION_REQUEST: &str = "PermissionRequest";
pub const HOOK_EVENT_POST_TOOL_USE: &str = "PostToolUse";
pub const HOOK_EVENT_STOP_FAILURE: &str = "StopFailure";
pub const HOOK_EVENT_BEFORE_AGENT: &str = "BeforeAgent";
pub const HOOK_EVENT_AFTER_AGENT: &str = "AfterAgent";

/// The `Notification` sub-kinds Claude Code dispatches its notification hooks on, used as the
/// entry's `matcher` so one notification kind cannot stand in for another (contract §1.4).
pub const NOTIFICATION_MATCHER_PERMISSION_PROMPT: &str = "permission_prompt";
pub const NOTIFICATION_MATCHER_ELICITATION_DIALOG: &str = "elicitation_dialog";
pub const NOTIFICATION_MATCHER_IDLE_PROMPT: &str = "idle_prompt";

/// Every hook event TAIDE has ever written a Claude entry under — the current in-band command set
/// plus `UserPromptSubmit`, which only the retired HTTP install used. Removal and "is TAIDE
/// installed here" both walk this union so an older install is still recognized and cleaned up.
pub const CLAUDE_MANAGED_HOOK_EVENTS: &[&str] = &[
    HOOK_EVENT_USER_PROMPT_SUBMIT,
    HOOK_EVENT_PERMISSION_REQUEST,
    HOOK_EVENT_NOTIFICATION,
    HOOK_EVENT_POST_TOOL_USE,
    HOOK_EVENT_STOP,
    HOOK_EVENT_STOP_FAILURE,
];

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
