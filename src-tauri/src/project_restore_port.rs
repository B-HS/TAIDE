use tauri::AppHandle;

use crate::ids::ProjectId;
use crate::infra::watcher::WatcherHandle;
use crate::state::AppState;

pub struct ProjectRestoreWatchers {
    pub build_file: fn(&AppHandle, &ProjectId, &str) -> Option<WatcherHandle>,
    pub build_git: fn(&AppHandle, &ProjectId, &str) -> Option<WatcherHandle>,
    pub register_file: fn(&AppState, &ProjectId, WatcherHandle),
    pub register_git: fn(&AppState, &ProjectId, WatcherHandle),
}
