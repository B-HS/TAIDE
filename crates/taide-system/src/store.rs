use std::collections::HashSet;
use std::sync::Arc;

use parking_lot::Mutex;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
use taide_model::error::{AppError, AppResult};
use taide_model::system::SystemUsage;

use crate::service::{normalize_cpu_percent, ProcessRecord};

const FALLBACK_CPU_COUNT: usize = 1;

struct AppUsageInner {
    system: System,
    has_previous_sample: bool,
}

struct BreakdownUsageInner {
    system: System,
    known_pids: HashSet<u32>,
}

#[derive(Clone)]
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

    pub fn collect_app_usage(&self) -> AppResult<SystemUsage> {
        let pid = sysinfo::get_current_pid().map_err(|error| AppError::Internal(error.to_string()))?;
        let mut guard = self.app.lock();

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

    pub fn refresh_process_records(&self) -> Vec<ProcessRecord> {
        let mut guard = self.breakdown.lock();
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
}

impl Default for SystemUsageStore {
    fn default() -> Self {
        Self::new()
    }
}
