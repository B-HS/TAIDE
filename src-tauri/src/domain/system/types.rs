use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemUsage {
    pub cpu_percent: Option<f64>,
    pub memory_bytes: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AppDataPathKind {
    Plugins,
    Themes,
    Locales,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SystemUsageProcessKind {
    App,
    Terminal,
    Lsp,
    Agent,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemUsageProcess {
    pub pid: u32,
    pub kind: SystemUsageProcessKind,
    pub label: String,
    /// `null` until this pid has survived one prior breakdown refresh — sysinfo reports
    /// `cpu_usage() == 0.0` for a pid it has never refreshed before, which is indistinguishable
    /// from a genuinely idle process (ipc-contract.md §기능 확장 2차 계약 확정 추가).
    pub cpu_percent: Option<f64>,
    pub memory_bytes: f64,
}
