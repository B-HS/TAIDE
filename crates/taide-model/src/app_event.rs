use crate::file::FsChange;
use crate::ids::{ProjectId, ShellSlotId};
use crate::lsp::{LspInstallPhase, LspServerId, LspSessionStatus};
use crate::project::{Project, ProjectGroup, ProjectRef, ShellSlotTree, WindowChrome};
use crate::remote::RemoteStatus;
use crate::settings::Settings;
use crate::sync::SyncStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    LayoutChanged {
        project_id: ProjectId,
        revision: u32,
    },
    GitStatusChanged {
        project_id: ProjectId,
    },
    GitRefsChanged {
        project_id: ProjectId,
    },
    TerminalSpawned {
        session_id: String,
        project_id: ProjectId,
        cwd: String,
        shell: String,
    },
    TerminalExited {
        session_id: String,
        code: Option<i32>,
    },
    TerminalCwdChanged {
        session_id: String,
        cwd: String,
    },
    TerminalCommandFinished {
        session_id: String,
        cwd: Option<String>,
        exit_code: Option<i32>,
        duration_ms: u32,
    },
    SettingsChanged {
        settings: Box<Settings>,
    },
    ThemeChanged {
        theme_id: String,
    },
    SyncStateChanged {
        status: SyncStatus,
    },
    RemoteStateChanged {
        status: RemoteStatus,
    },
    WindowChromeChanged {
        chrome: WindowChrome,
    },
    ProjectListChanged {
        projects: Vec<ProjectRef>,
    },
    ProjectGroupsChanged {
        groups: Vec<ProjectGroup>,
    },
    SessionShellSlotsChanged {
        tree: Option<ShellSlotTree>,
        focused: Option<ShellSlotId>,
    },
    ProjectOpened {
        project: Box<Project>,
    },
    ProjectClosed {
        project_id: ProjectId,
    },
    ProjectActivated {
        project_id: Option<ProjectId>,
    },
    ProjectRecentCleared {
        removed: u32,
        skipped_with_drafts: u32,
    },
    FsChanged {
        project_id: ProjectId,
        change: FsChange,
    },
    FsRescanRequired {
        project_id: ProjectId,
    },
    LspSessionStatusChanged {
        session_id: String,
        status: LspSessionStatus,
        last_error: Option<String>,
        generation: u32,
    },
    LspInstallProgress {
        server_id: LspServerId,
        phase: LspInstallPhase,
        received_bytes: f64,
        total_bytes: Option<f64>,
        message: Option<String>,
    },
}
