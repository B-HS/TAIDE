use serde::{Deserialize, Serialize};
use specta::Type;

pub const SYSTEM_USAGE_POLL_INTERVAL_MS: u32 = 3_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemUsage {
    pub cpu_percent: Option<f64>,
    pub memory_bytes: f64,
}
