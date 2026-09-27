use std::path::Path;

use taide_model::app_event::AppEvent;
use taide_runtime::{EventSink, PlatformServices};
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::error::{AppError, AppResult};

use super::event_sink::TauriEventSink;

pub struct TauriPlatformServices(pub AppHandle);

impl EventSink for TauriPlatformServices {
    fn publish(&self, event: AppEvent) {
        TauriEventSink(&self.0).publish(event);
    }
}

impl PlatformServices for TauriPlatformServices {
    fn open_path(&self, path: &Path) -> AppResult<()> {
        tauri_plugin_opener::open_path(path, None::<&str>).map_err(|error| AppError::Internal(error.to_string()))
    }

    fn reveal_item_in_dir(&self, path: &Path) -> AppResult<()> {
        tauri_plugin_opener::reveal_item_in_dir(path).map_err(|error| AppError::Internal(error.to_string()))
    }

    fn open_url(&self, url: &str) -> AppResult<()> {
        tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| AppError::Internal(error.to_string()))
    }

    fn send_notification(&self, title: &str, body: &str) -> AppResult<()> {
        self.0
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|error| AppError::Internal(error.to_string()))
    }
}
