use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const DEFAULT_SCROLLBACK_BYTES: usize = 2 * 1024 * 1024;
pub const READ_BUFFER_BYTES: usize = 64 * 1024;
pub const OUTPUT_BATCH_MS: u64 = 4;
pub const OUTPUT_FLUSH_TICK_MS: u64 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShellProfile {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PtySpawnOptions {
    pub project_id: ProjectId,
    pub cwd: String,
    #[serde(default)]
    pub shell: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    pub id: String,
    pub project_id: ProjectId,
    pub cwd: String,
    pub shell: String,
    pub running: bool,
}
