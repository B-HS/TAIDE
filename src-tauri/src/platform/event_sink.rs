use taide_model::app_event::AppEvent;
use taide_runtime::EventSink;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::events::{
    AgentExternalOpen, AgentStateChanged, FsChanged, FsRescanRequired, GitRefsChanged, GitStatusChanged, IdeCloseTabRequested,
    IdeDiffRequested, IdeSaveRequested, IdeStatusChanged, LayoutChanged, LspInstallProgress, LspSessionStatusChanged, ProjectActivated,
    ProjectClosed, ProjectGroupsChanged, ProjectListChanged, ProjectOpened, ProjectRecentCleared, RemoteStateChanged,
    SessionShellSlotsChanged, SettingsChanged, SyncStateChanged, TerminalCommandFinished, TerminalCwdChanged, TerminalExited,
    TerminalSpawned, ThemeChanged, WindowChromeChanged,
};

pub struct TauriEventSink<'a>(pub &'a AppHandle);

impl EventSink for TauriEventSink<'_> {
    fn publish(&self, event: AppEvent) {
        match event {
            AppEvent::LayoutChanged { project_id, revision } => {
                let _ = LayoutChanged { project_id, revision }.emit(self.0);
            }
            AppEvent::GitStatusChanged { project_id } => {
                let _ = GitStatusChanged { project_id }.emit(self.0);
            }
            AppEvent::GitRefsChanged { project_id } => {
                let _ = GitRefsChanged { project_id }.emit(self.0);
            }
            AppEvent::TerminalSpawned {
                session_id,
                project_id,
                cwd,
                shell,
            } => {
                let _ = TerminalSpawned {
                    session_id,
                    project_id,
                    cwd,
                    shell,
                }
                .emit(self.0);
            }
            AppEvent::TerminalExited { session_id, code } => {
                let _ = TerminalExited { session_id, code }.emit(self.0);
            }
            AppEvent::TerminalCwdChanged { session_id, cwd } => {
                let _ = TerminalCwdChanged { session_id, cwd }.emit(self.0);
            }
            AppEvent::TerminalCommandFinished {
                session_id,
                cwd,
                exit_code,
                duration_ms,
            } => {
                let _ = TerminalCommandFinished {
                    session_id,
                    cwd,
                    exit_code,
                    duration_ms,
                }
                .emit(self.0);
            }
            AppEvent::SettingsChanged { settings } => {
                let _ = SettingsChanged { settings: *settings }.emit(self.0);
            }
            AppEvent::ThemeChanged { theme_id } => {
                let _ = ThemeChanged { theme_id }.emit(self.0);
            }
            AppEvent::SyncStateChanged { status } => {
                let _ = SyncStateChanged { status }.emit(self.0);
            }
            AppEvent::RemoteStateChanged { status } => {
                let _ = RemoteStateChanged { status }.emit(self.0);
            }
            AppEvent::WindowChromeChanged { chrome } => {
                let _ = WindowChromeChanged { chrome }.emit(self.0);
            }
            AppEvent::ProjectListChanged { projects } => {
                let _ = ProjectListChanged { projects }.emit(self.0);
            }
            AppEvent::ProjectGroupsChanged { groups } => {
                let _ = ProjectGroupsChanged { groups }.emit(self.0);
            }
            AppEvent::SessionShellSlotsChanged { tree, focused } => {
                let _ = SessionShellSlotsChanged { tree, focused }.emit(self.0);
            }
            AppEvent::ProjectOpened { project } => {
                let _ = ProjectOpened { project: *project }.emit(self.0);
            }
            AppEvent::ProjectClosed { project_id } => {
                let _ = ProjectClosed { project_id }.emit(self.0);
            }
            AppEvent::ProjectActivated { project_id } => {
                let _ = ProjectActivated { project_id }.emit(self.0);
            }
            AppEvent::ProjectRecentCleared {
                removed,
                skipped_with_drafts,
            } => {
                let _ = ProjectRecentCleared {
                    removed,
                    skipped_with_drafts,
                }
                .emit(self.0);
            }
            AppEvent::FsChanged { project_id, change } => {
                let _ = FsChanged { project_id, change }.emit(self.0);
            }
            AppEvent::FsRescanRequired { project_id } => {
                let _ = FsRescanRequired { project_id }.emit(self.0);
            }
            AppEvent::LspSessionStatusChanged {
                session_id,
                status,
                last_error,
                generation,
            } => {
                let _ = LspSessionStatusChanged {
                    session_id,
                    status,
                    last_error,
                    generation,
                }
                .emit(self.0);
            }
            AppEvent::LspInstallProgress {
                server_id,
                phase,
                received_bytes,
                total_bytes,
                message,
            } => {
                let _ = LspInstallProgress {
                    server_id,
                    phase,
                    received_bytes,
                    total_bytes,
                    message,
                }
                .emit(self.0);
            }
            AppEvent::IdeStatusChanged { status } => {
                let _ = IdeStatusChanged { status }.emit(self.0);
            }
            AppEvent::IdeDiffRequested {
                request_id,
                project_id,
                old_path,
                new_path,
                new_contents,
                tab_name,
            } => {
                let _ = IdeDiffRequested {
                    request_id,
                    project_id,
                    old_path,
                    new_path,
                    new_contents,
                    tab_name,
                }
                .emit(self.0);
            }
            AppEvent::IdeSaveRequested {
                request_id,
                project_id,
                path,
            } => {
                let _ = IdeSaveRequested {
                    request_id,
                    project_id,
                    path,
                }
                .emit(self.0);
            }
            AppEvent::IdeCloseTabRequested { tab_name, request_id } => {
                let _ = IdeCloseTabRequested { tab_name, request_id }.emit(self.0);
            }
            AppEvent::AgentStateChanged { project_id, agents } => {
                let _ = AgentStateChanged { project_id, agents }.emit(self.0);
            }
            AppEvent::AgentExternalOpen { request } => {
                let _ = AgentExternalOpen { request }.emit(self.0);
            }
        }
    }
}
