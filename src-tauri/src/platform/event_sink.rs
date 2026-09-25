use taide_model::app_event::AppEvent;
use taide_runtime::EventSink;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::events::LayoutChanged;

pub struct TauriEventSink<'a>(pub &'a AppHandle);

impl EventSink for TauriEventSink<'_> {
    fn publish(&self, event: AppEvent) {
        match event {
            AppEvent::LayoutChanged { project_id, revision } => {
                let _ = LayoutChanged { project_id, revision }.emit(self.0);
            }
        }
    }
}
