use std::sync::Arc;

use parking_lot::Mutex;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::State;

use super::service::normalize_cpu_percent;
use super::types::SystemUsage;
use crate::error::{AppError, AppResult};

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
