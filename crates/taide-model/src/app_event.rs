use crate::ids::{ProjectId, ShellSlotId};
use crate::project::{ProjectGroup, ProjectRef, ShellSlotTree, WindowChrome};
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
}
