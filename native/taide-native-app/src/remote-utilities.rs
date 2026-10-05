use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use taide_model::error::{AppError, AppResult};
use taide_model::font::FontFamily;
use taide_model::system::{SystemUsage, SystemUsageProcess, SystemUsageProcessKind};
use taide_runtime::{AppServices, task_actions};
use taide_system::service::{self, ProcessRecord};
use taide_system::store::SystemUsageStore;

use crate::remote_gateway::{argument, error_value, respond};
use crate::remote_ws::Dispatch;

const FALLBACK_CPU_COUNT: usize = 1;
const APP_PROCESS_LABEL: &str = "TAIDE";

pub const COMMANDS: &[&str] = &[
    "detect_tasks",
    "font_list",
    "system_usage_get",
    "system_usage_breakdown",
];

pub type UsageLabels = HashMap<u32, (SystemUsageProcessKind, String)>;
pub type LabelProvider = Arc<dyn Fn(&AppServices) -> UsageLabels + Send + Sync>;

pub trait UsageProvider: Send + Sync {
    fn collect_app_usage(&self) -> AppResult<SystemUsage>;
    fn refresh_process_records(&self) -> Vec<ProcessRecord>;
}

impl UsageProvider for SystemUsageStore {
    fn collect_app_usage(&self) -> AppResult<SystemUsage> {
        SystemUsageStore::collect_app_usage(self)
    }

    fn refresh_process_records(&self) -> Vec<ProcessRecord> {
        SystemUsageStore::refresh_process_records(self)
    }
}

pub struct Ports {
    pub fonts: Arc<dyn Fn() -> Vec<FontFamily> + Send + Sync>,
    pub usage: Arc<dyn UsageProvider>,
    pub root_pid: Arc<dyn Fn() -> AppResult<u32> + Send + Sync>,
    pub cpu_count: Arc<dyn Fn() -> usize + Send + Sync>,
    pub label_providers: Vec<LabelProvider>,
}

impl Ports {
    pub fn new(services: &AppServices, label_providers: Vec<LabelProvider>) -> Self {
        Self {
            fonts: Arc::new(taide_font::service::list_families),
            usage: Arc::new(services.system_usage.clone()),
            root_pid: Arc::new(taide_system::store::current_pid),
            cpu_count: Arc::new(|| {
                std::thread::available_parallelism()
                    .map(|count| count.get())
                    .unwrap_or(FALLBACK_CPU_COUNT)
            }),
            label_providers,
        }
    }
}

pub fn domain_label_providers() -> Vec<LabelProvider> {
    vec![
        Arc::new(|services| {
            let projects = services.state.projects.read();
            let mut labels = UsageLabels::new();
            for (id, project) in projects.iter() {
                for (_, pid) in services.terminal.foreground_pids(id) {
                    labels.insert(
                        pid,
                        (SystemUsageProcessKind::Terminal, project.name.clone()),
                    );
                }
            }
            labels
        }),
        Arc::new(|services| {
            let projects = services.state.projects.read();
            let mut labels = UsageLabels::new();
            for (id, project) in projects.iter() {
                for agent in services.agents.agents_for(id) {
                    labels.insert(
                        agent.pid,
                        (
                            SystemUsageProcessKind::Agent,
                            format!("{} · {}", agent.name, project.name),
                        ),
                    );
                }
            }
            labels
        }),
        Arc::new(|services| {
            let server_pids = services.lsp.server_pids();
            let projects = services.state.projects.read();
            let mut labels = UsageLabels::new();
            for (project_id, server_name, pid) in server_pids {
                let label = projects
                    .get(&project_id)
                    .map(|project| format!("{server_name} · {}", project.name))
                    .unwrap_or(server_name);
                labels.insert(pid, (SystemUsageProcessKind::Lsp, label));
            }
            labels
        }),
    ]
}

pub fn extend_backend(ports: Ports, remaining: Dispatch) -> Dispatch {
    let ports = Arc::new(ports);
    let remaining_json = remaining.json;
    Dispatch {
        json: Arc::new(move |services, name, args, channels| {
            let ports = ports.clone();
            let remaining = remaining_json.clone();
            Box::pin(async move {
                if COMMANDS.contains(&name.as_str()) {
                    return dispatch(&services, ports, &name, &args).await;
                }
                remaining(services, name, args, channels).await
            })
        }),
        raw: remaining.raw,
    }
}

async fn dispatch(
    services: &AppServices,
    ports: Arc<Ports>,
    name: &str,
    args: &Value,
) -> Result<String, Value> {
    match name {
        "detect_tasks" => respond(
            task_actions::detect_tasks(
                &services.state,
                &services.tasks,
                argument(args, "projectId").map_err(error_value)?,
            )
            .await,
        ),
        "font_list" => respond(
            services
                .tasks
                .run_blocking_result("font-list", move || Ok((ports.fonts)()))
                .await,
        ),
        "system_usage_get" => respond(collect_usage(services, ports).await),
        "system_usage_breakdown" => respond(collect_breakdown(services, ports).await),
        _ => Err(error_value(AppError::Internal(format!(
            "native remote utility routing mismatch: {name}"
        )))),
    }
}

pub(crate) async fn collect_usage(
    services: &AppServices,
    ports: Arc<Ports>,
) -> AppResult<SystemUsage> {
    services
        .tasks
        .run_blocking_result("system-usage-get", move || ports.usage.collect_app_usage())
        .await
}

pub(crate) async fn collect_breakdown(
    services: &AppServices,
    ports: Arc<Ports>,
) -> AppResult<Vec<SystemUsageProcess>> {
    let _operation = services
        .tasks
        .begin_operation("system-usage-breakdown")
        .ok_or_else(|| AppError::Forbidden("task supervisor is shutting down".into()))?;
    let root_pid = (ports.root_pid)()?;
    let mut labels = UsageLabels::new();
    for provider in &ports.label_providers {
        labels.extend(provider(services));
    }
    let cpu_count = (ports.cpu_count)();
    let records = services
        .tasks
        .run_blocking_result("system-usage-breakdown", move || {
            Ok(ports.usage.refresh_process_records())
        })
        .await?;
    Ok(service::build_usage_processes(
        &records,
        root_pid,
        APP_PROCESS_LABEL,
        &labels,
        cpu_count,
    ))
}

#[cfg(test)]
#[path = "remote-utilities-tests.rs"]
mod tests;
