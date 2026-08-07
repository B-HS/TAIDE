use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const AGENT_POLL_UNIX_MS: u64 = 1_000;
pub const AGENT_POLL_WINDOWS_MS: u64 = 2_000;
pub const KNOWN_AGENT_NAMES: &[&str] = &["claude", "codex", "gemini"];
pub const WAIT_MARKER_PREFIX: &str = "taide-wait-";

/// 프로세스 상태가 "실행 중"으로 관측된 뒤에도 이 시간 동안은 Working 을 유지한다(폴링 사이 관측 누락 보정).
pub const ACTIVITY_WORKING_HOLD_MS: u64 = 2_000;
/// 마지막 활동 관측 이후 이 시간이 지나면 Idle 로 전이한다. 그 사이 구간은 히스테리시스로 직전 상태를 유지한다.
pub const ACTIVITY_IDLE_QUIET_MS: u64 = 6_000;
/// hooks 로 설정된 프로젝트 단위 활동 override 가 이 시간보다 오래되면 무시하고 휴리스틱으로 복귀한다.
pub const HOOK_OVERRIDE_STALE_MS: u64 = 900_000;

pub const HOOKS_HTTP_PATH: &str = "/claude/hook";
pub const HOOKS_TOKEN_QUERY_KEY: &str = "token";
pub const HOOKS_URL_MARKER: &str = "taide=1";
pub const HOOKS_HTTP_TIMEOUT_SECONDS: u64 = 5;
pub const MAX_HOOKS_REQUEST_BYTES: usize = 65_536;

pub const HOOK_EVENT_USER_PROMPT_SUBMIT: &str = "UserPromptSubmit";
pub const HOOK_EVENT_NOTIFICATION: &str = "Notification";
pub const HOOK_EVENT_STOP: &str = "Stop";
pub const MANAGED_HOOK_EVENTS: &[&str] = &[HOOK_EVENT_USER_PROMPT_SUBMIT, HOOK_EVENT_NOTIFICATION, HOOK_EVENT_STOP];

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
pub struct AgentHooksStatus {
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
    pub target_path: String,
    pub editor_env_hint: String,
}
