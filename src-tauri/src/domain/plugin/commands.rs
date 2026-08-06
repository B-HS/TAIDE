use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;

use super::service;
use super::types::LoadedPlugin;

pub struct PluginStore(pub parking_lot::RwLock<Option<Vec<LoadedPlugin>>>);

impl PluginStore {
    pub fn new() -> Self {
        Self(parking_lot::RwLock::new(None))
    }
}

impl Default for PluginStore {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[specta::specta]
pub async fn plugin_list(state: State<'_, AppState>, store: State<'_, PluginStore>) -> AppResult<Vec<LoadedPlugin>> {
    if let Some(cached) = store.0.read().clone() {
        return Ok(cached);
    }

    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    Ok(loaded)
}

#[tauri::command]
#[specta::specta]
pub async fn plugin_reload(state: State<'_, AppState>, store: State<'_, PluginStore>) -> AppResult<Vec<LoadedPlugin>> {
    let _guard = state.begin_mutation().await;
    let loaded = service::load_plugins(&state.paths.plugins_dir());
    *store.0.write() = Some(loaded.clone());
    Ok(loaded)
}
