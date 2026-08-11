use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::domain::layout::types::FocusKind;
use crate::domain::project::types::{Project, ProjectRef};
use crate::ids::ProjectId;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[tauri_specta(event_name = "app:ready")]
pub struct AppReady {
    pub version: String,
}

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
#[tauri_specta(event_name = "project:focus-kind-changed")]
pub struct ProjectFocusKindChanged {
    pub project_id: ProjectId,
    pub kind: FocusKind,
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "lsp:session-status-changed")]
pub struct LspSessionStatusChanged {
    pub session_id: String,
    pub status: crate::domain::lsp::types::LspSessionStatus,
    pub last_error: Option<String>,
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
