use taide_model::app_event::AppEvent;
use taide_runtime::EventSink;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::events::{
    GitRefsChanged, GitStatusChanged, LayoutChanged, SettingsChanged, SyncStateChanged, TerminalCommandFinished, TerminalCwdChanged,
    TerminalExited, TerminalSpawned, ThemeChanged,
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
        }
    }
}
