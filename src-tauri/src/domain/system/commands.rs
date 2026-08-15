use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::State;

use super::service::{self, file_url, normalize_cpu_percent, ProcessRecord};
use super::types::{AppDataPathKind, SystemUsage, SystemUsageProcess, SystemUsageProcessKind};
use crate::domain::agent::commands::AgentStore;
use crate::domain::lsp::commands::LspStore;
use crate::domain::terminal::commands::TerminalStore;
use crate::error::{AppError, AppResult};
use crate::infra::root_guard;
use crate::state::AppState;

const FALLBACK_CPU_COUNT: usize = 1;
const APP_PROCESS_LABEL: &str = "TAIDE";

struct AppUsageInner {
    system: System,
    has_previous_sample: bool,
}

struct BreakdownUsageInner {
    system: System,
    known_pids: HashSet<u32>,
}

/// Two independent sysinfo `System` instances, one per command. sysinfo derives a process's
/// CPU delta from the elapsed time since *that `System` instance's* last refresh of that pid;
/// sharing one `System` between `system_usage_get` (app pid only, polled continuously by the
/// status bar) and `system_usage_breakdown` (full process table, polled only while the usage
/// modal is open) let the two independently-phased pollers refresh the app pid within
/// sysinfo's 200ms minimum interval of each other, which collapses the app row's CPU% to near
/// zero (ipc-contract.md §기능 확장 2차 계약 확정 추가).
pub struct SystemUsageStore {
    app: Arc<Mutex<AppUsageInner>>,
    breakdown: Arc<Mutex<BreakdownUsageInner>>,
}

impl SystemUsageStore {
    pub fn new() -> Self {
        Self {
            app: Arc::new(Mutex::new(AppUsageInner {
                system: System::new(),
                has_previous_sample: false,
            })),
            breakdown: Arc::new(Mutex::new(BreakdownUsageInner {
                system: System::new(),
                known_pids: HashSet::new(),
            })),
        }
    }
}

impl Default for SystemUsageStore {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_system_usage(inner: &Mutex<AppUsageInner>) -> AppResult<SystemUsage> {
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
    let inner = store.app.clone();
    tauri::async_runtime::spawn_blocking(move || collect_system_usage(&inner))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?
}

/// Full-process-table sysinfo scan, reduced to what `build_usage_processes` needs. Only ever
/// invoked while the usage breakdown modal is open (query `enabled` gate on the frontend) —
/// refreshed against its own `System` instance (see [`SystemUsageStore`]) so it never shares
/// cpu-delta timing state with `system_usage_get`'s app-only refresh. `known_pids` from the
/// previous breakdown call marks which pids already have a valid cpu delta; a pid missing from
/// it is this `System` instance's first refresh of that process, so it is flagged via
/// `has_previous_cpu_sample: false` rather than trusting sysinfo's meaningless first-sample 0.0.
fn refresh_all_process_records(inner: &Mutex<BreakdownUsageInner>) -> Vec<ProcessRecord> {
    let mut guard = inner.lock();
    guard
        .system
        .refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory());

    let records: Vec<ProcessRecord> = guard
        .system
        .processes()
        .values()
        .map(|process| {
            let pid = process.pid().as_u32();
            ProcessRecord {
                pid,
                parent_pid: process.parent().map(|pid| pid.as_u32()),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                has_previous_cpu_sample: guard.known_pids.contains(&pid),
            }
        })
        .collect();

    guard.known_pids = records.iter().map(|record| record.pid).collect();
    records
}

fn collect_terminal_and_agent_labels(
    state: &AppState,
    terminals: &TerminalStore,
    agents: &AgentStore,
) -> HashMap<u32, (SystemUsageProcessKind, String)> {
    let projects = state.projects.read();
    let mut labels = HashMap::new();

    for (project_id, project) in projects.iter() {
        for (_, pid) in terminals.foreground_pids(project_id) {
            labels.insert(pid, (SystemUsageProcessKind::Terminal, project.name.clone()));
        }
        for agent in agents.agents_for(project_id) {
            labels.insert(
                agent.pid,
                (SystemUsageProcessKind::Agent, format!("{} · {}", agent.name, project.name)),
            );
        }
    }

    labels
}

#[tauri::command]
#[specta::specta]
pub async fn system_usage_breakdown(
    state: State<'_, AppState>,
    store: State<'_, SystemUsageStore>,
    terminals: State<'_, TerminalStore>,
    lsp: State<'_, LspStore>,
    agents: State<'_, AgentStore>,
) -> AppResult<Vec<SystemUsageProcess>> {
    let root_pid = sysinfo::get_current_pid()
        .map_err(|error| AppError::Internal(error.to_string()))?
        .as_u32();

    let mut domain_labels = collect_terminal_and_agent_labels(&state, &terminals, &agents);
    let projects = state.projects.read().clone();
    for (project_id, server_name, pid) in lsp.server_pids() {
        let label = projects
            .get(&project_id)
            .map(|project| format!("{server_name} · {}", project.name))
            .unwrap_or(server_name);
        domain_labels.insert(pid, (SystemUsageProcessKind::Lsp, label));
    }

    let cpu_count = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(FALLBACK_CPU_COUNT);

    let inner = store.breakdown.clone();
    let records = tauri::async_runtime::spawn_blocking(move || refresh_all_process_records(&inner))
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
        AppDataPathKind::Snippets => state.paths.snippets_dir(),
    };
    std::fs::create_dir_all(&dir)?;
    tauri_plugin_opener::reveal_item_in_dir(dir).map_err(|error| AppError::Internal(error.to_string()))
}
