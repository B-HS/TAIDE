use std::collections::HashMap;

use taide_runtime::{system_actions, PlatformServicesState};
use tauri::State;

pub use taide_system::store::SystemUsageStore;

use super::service;
use super::types::{AppDataPathKind, SystemUsage, SystemUsageProcess, SystemUsageProcessKind};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const FALLBACK_CPU_COUNT: usize = 1;
const APP_PROCESS_LABEL: &str = "TAIDE";

pub type SystemUsageLabels = HashMap<u32, (SystemUsageProcessKind, String)>;
pub type SystemUsageLabelProvider = Box<dyn Fn(&tauri::AppHandle) -> SystemUsageLabels + Send + Sync>;

/// The pid → (kind, label) providers [`system_usage_breakdown`] consults to label terminal, agent,
/// and LSP child processes. `lib.rs`'s assembly registers one closure per owning domain
/// (`system_usage_label_providers`) so this domain never reads another domain's store directly
/// (audit R8#9, T1-I §1.4). Providers run in registration order and later entries overwrite
/// earlier ones for the same pid: with the registered terminal → agent → LSP order, an agent
/// process detected inside a terminal's foreground pid set labels as Agent and LSP labels apply
/// last. Each provider reads its own project snapshot, and registration order — not the old
/// hand-coded collection's per-project `HashMap` iteration order — decides every pid collision;
/// see the assembly doc (`lib.rs`) for why that determinization is intentional.
pub struct SystemUsageLabelProviders(Vec<SystemUsageLabelProvider>);

impl SystemUsageLabelProviders {
    pub fn new(providers: Vec<SystemUsageLabelProvider>) -> Self {
        Self(providers)
    }

    pub fn collect(&self, app: &tauri::AppHandle) -> SystemUsageLabels {
        let mut labels = HashMap::new();
        for provider in &self.0 {
            labels.extend(provider(app));
        }
        labels
    }
}

#[tauri::command]
#[specta::specta]
pub async fn system_usage_get(store: State<'_, SystemUsageStore>) -> AppResult<SystemUsage> {
    let store = (*store).clone();
    tauri::async_runtime::spawn_blocking(move || store.collect_app_usage())
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn system_usage_breakdown(
    app: tauri::AppHandle,
    store: State<'_, SystemUsageStore>,
    providers: State<'_, SystemUsageLabelProviders>,
) -> AppResult<Vec<SystemUsageProcess>> {
    let root_pid = sysinfo::get_current_pid()
        .map_err(|error| AppError::Internal(error.to_string()))?
        .as_u32();

    let domain_labels = providers.collect(&app);

    let cpu_count = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(FALLBACK_CPU_COUNT);

    let store = (*store).clone();
    let records = tauri::async_runtime::spawn_blocking(move || store.refresh_process_records())
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(service::build_usage_processes(
        &records,
        root_pid,
        APP_PROCESS_LABEL,
        &domain_labels,
        cpu_count,
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_path(state: State<'_, AppState>, platform: State<'_, PlatformServicesState>, path: String) -> AppResult<()> {
    system_actions::system_open_path(&state, platform.0.as_ref(), &path)
}

#[tauri::command]
#[specta::specta]
pub async fn system_reveal_path(state: State<'_, AppState>, platform: State<'_, PlatformServicesState>, path: String) -> AppResult<()> {
    system_actions::system_reveal_path(&state, platform.0.as_ref(), &path)
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_in_browser(state: State<'_, AppState>, platform: State<'_, PlatformServicesState>, path: String) -> AppResult<()> {
    system_actions::system_open_in_browser(&state, platform.0.as_ref(), &path)
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_external_url(platform: State<'_, PlatformServicesState>, url: String) -> AppResult<()> {
    system_actions::system_open_external_url(platform.0.as_ref(), &url)
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_app_data_path(
    state: State<'_, AppState>,
    platform: State<'_, PlatformServicesState>,
    kind: AppDataPathKind,
) -> AppResult<()> {
    system_actions::system_open_app_data_path(&state, platform.0.as_ref(), kind)
}
