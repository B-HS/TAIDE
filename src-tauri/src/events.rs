use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::domain::project::types::{Project, ProjectRef};
use crate::domain::settings::types::Settings;
use crate::ids::ProjectId;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:opened")]
pub struct ProjectOpened {
    pub project: Project,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:closed")]
pub struct ProjectClosed {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:activated")]
pub struct ProjectActivated {
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:list-changed")]
pub struct ProjectListChanged {
    pub projects: Vec<ProjectRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "layout:changed")]
pub struct LayoutChanged {
    pub project_id: ProjectId,
    pub revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "theme:changed")]
pub struct ThemeChanged {
    pub theme_id: String,
}

/// Emitted after every settings write reaches the shared reapply path
/// (`settings::commands::apply_and_broadcast`) — a `settings_update` patch, a `sync_download`, or an
/// `app_file_write` on the `Settings` target. Carries the full sanitized `Settings` so a listener
/// can update immediately without a round-trip, though the frontend's own convention is to
/// invalidate its cached `SETTINGS.CURRENT` query and let TanStack Query refetch
/// (`docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.3).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "settings:changed")]
pub struct SettingsChanged {
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "fs:changed")]
pub struct FsChanged {
    pub project_id: ProjectId,
    pub change: crate::domain::file::types::FsChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "terminal:exited")]
pub struct TerminalExited {
    pub session_id: String,
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "terminal:cwd-changed")]
pub struct TerminalCwdChanged {
    pub session_id: String,
    pub cwd: String,
}

/// A shell command that ran in a pty session ended, with how long it ran for.
///
/// Detected on the pty reader thread (`domain::terminal::commands::report_command_marker`) rather
/// than in the frontend's own OSC 133 tracker, because the notification this feeds exists precisely
/// for the case the frontend cannot see: a long command in a terminal tab the user switched away
/// from, whose `TerminalSession` — and with it the xterm instance and its tracker — is unmounted
/// while it runs. `duration_ms` is measured between the shell's `133;C` and `133;D`, so it is real
/// elapsed time no matter who was watching; a command whose start was never seen reports nothing at
/// all rather than a guess.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "terminal:command-finished")]
pub struct TerminalCommandFinished {
    pub session_id: String,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "git:status-changed")]
pub struct GitStatusChanged {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "git:refs-changed")]
pub struct GitRefsChanged {
    pub project_id: ProjectId,
}

/// `generation` (R7#1) increases only when `crate::domain::lsp::commands::handle_process_exit`'s
/// automatic crash-restart path successfully respawns the process — see the `generation` field doc
/// on `domain::lsp::commands::SessionEntry` for the full semantics: a `status: Crashed` event whose
/// `generation` is higher than the last one the renderer saw means "the process behind this
/// `session_id` was silently replaced — discard your old LSP client state, re-run `initialize` over
/// `lsp_send`, then call `domain::lsp::commands::lsp_confirm_reinitialize` with this same
/// `generation`" (only that confirmation flips `status` back to `Running`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "lsp:session-status-changed")]
pub struct LspSessionStatusChanged {
    pub session_id: String,
    pub status: crate::domain::lsp::types::LspSessionStatus,
    pub last_error: Option<String>,
    pub generation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "lsp:install-progress")]
pub struct LspInstallProgress {
    pub server_id: crate::domain::lsp::types::LspServerId,
    pub phase: crate::domain::lsp::types::LspInstallPhase,
    pub received_bytes: f64,
    pub total_bytes: Option<f64>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "agent:state-changed")]
pub struct AgentStateChanged {
    pub project_id: ProjectId,
    pub agents: Vec<crate::domain::agent::types::DetectedAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "agent:external-open")]
pub struct AgentExternalOpen {
    pub request: crate::domain::agent::types::ExternalOpenRequest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:status-changed")]
pub struct IdeStatusChanged {
    pub status: crate::domain::ide::types::IdeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:diff-requested")]
pub struct IdeDiffRequested {
    pub request_id: String,
    pub project_id: ProjectId,
    pub old_path: String,
    pub new_path: String,
    pub new_contents: String,
    pub tab_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:save-requested")]
pub struct IdeSaveRequested {
    pub request_id: String,
    pub project_id: ProjectId,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:close-tab-requested")]
pub struct IdeCloseTabRequested {
    pub tab_name: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "sync:state-changed")]
pub struct SyncStateChanged {
    pub status: crate::domain::sync::types::SyncStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "remote:state-changed")]
pub struct RemoteStateChanged {
    pub status: crate::domain::remote::types::RemoteStatus,
}

/// Emitted once when the OS requests the window to close, asking the
/// frontend to flush every dirty editor model to the hot-exit mirror before
/// the app actually exits. `timeout_ms` mirrors `HOT_EXIT_FLUSH_TIMEOUT_MS`
/// so the frontend never needs its own copy of that constant.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "app:hot-exit-flush-requested")]
pub struct HotExitFlushRequested {
    pub timeout_ms: f64,
}
