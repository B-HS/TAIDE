use std::path::Path;

use taide_runtime::PlatformServices;

use crate::error::{AppError, AppResult};

pub struct TauriPlatformServices;

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
}
