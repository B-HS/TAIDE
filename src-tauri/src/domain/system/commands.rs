use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::State;

use super::service::{file_url, normalize_cpu_percent};
use super::types::{AppDataPathKind, SystemUsage};
use crate::error::{AppError, AppResult};
use crate::infra::root_guard;
use crate::state::AppState;

const FALLBACK_CPU_COUNT: usize = 1;

struct SystemUsageInner {
    system: System,
    has_previous_sample: bool,
}

pub struct SystemUsageStore(Arc<Mutex<SystemUsageInner>>);

impl SystemUsageStore {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(SystemUsageInner {
            system: System::new(),
            has_previous_sample: false,
        })))
    }
}

impl Default for SystemUsageStore {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_system_usage(inner: &Mutex<SystemUsageInner>) -> AppResult<SystemUsage> {
    let pid = sysinfo::get_current_pid().map_err(|error| AppError::Internal(error.to_string()))?;
    let mut guard = inner.lock();

    guard.system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );

    let (memory_bytes, cpu_usage) = {
        let process = guard
            .system
            .process(pid)
            .ok_or_else(|| AppError::Internal("failed to read the TAIDE process info".to_string()))?;
        (process.memory() as f64, process.cpu_usage())
    };

    let cpu_percent = guard.has_previous_sample.then(|| {
        let cpu_count = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(FALLBACK_CPU_COUNT);
        normalize_cpu_percent(cpu_usage, cpu_count)
    });
    guard.has_previous_sample = true;

    Ok(SystemUsage { cpu_percent, memory_bytes })
}

#[tauri::command]
#[specta::specta]
pub async fn system_usage_get(store: State<'_, SystemUsageStore>) -> AppResult<SystemUsage> {
    let inner = store.0.clone();
    tauri::async_runtime::spawn_blocking(move || collect_system_usage(&inner))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// 열린 프로젝트 루트 안의 경로만 OS 셸로 넘긴다 — opener 플러그인 권한을 열지 않고
/// 이 커맨드를 유일한 통로로 두기 위한 게이트다(ipc-contract §4).
fn resolve_within_open_project(state: &AppState, path: &str) -> AppResult<PathBuf> {
    let projects = state.projects.read().clone();
    let (_, resolved) = root_guard::resolve_owning_project(&projects, Path::new(path))?;
    Ok(resolved)
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_path(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let resolved = resolve_within_open_project(&state, &path)?;
    tauri_plugin_opener::open_path(resolved, None::<&str>).map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn system_reveal_path(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let resolved = resolve_within_open_project(&state, &path)?;
    tauri_plugin_opener::reveal_item_in_dir(resolved).map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_in_browser(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let resolved = resolve_within_open_project(&state, &path)?;
    tauri_plugin_opener::open_url(file_url(&resolved), None::<&str>).map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn system_open_app_data_path(state: State<'_, AppState>, kind: AppDataPathKind) -> AppResult<()> {
    let dir = match kind {
        AppDataPathKind::Plugins => state.paths.plugins_dir(),
        AppDataPathKind::Themes => state.paths.themes_dir(),
        AppDataPathKind::Locales => state.paths.locales_dir(),
    };
    std::fs::create_dir_all(&dir)?;
    tauri_plugin_opener::reveal_item_in_dir(dir).map_err(|error| AppError::Internal(error.to_string()))
}
