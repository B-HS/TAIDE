use super::{service, types::AppInfo};
use crate::error::AppResult;

#[tauri::command]
#[specta::specta]
pub async fn app_get_info() -> AppResult<AppInfo> {
    Ok(service::app_info())
}
