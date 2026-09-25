use taide_model::app_event::AppEvent;
use taide_runtime::EventSink;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::events::{GitRefsChanged, GitStatusChanged, LayoutChanged};

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
        }
    }
}
