use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

pub const SESSION_SCHEMA_VERSION: u32 = 1;
pub const PROJECT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityKind {
    Git,
    Lsp,
    Terminal,
    AgentWatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRef {
    pub id: ProjectId,
    pub root: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    pub root: String,
    pub name: String,
    #[serde(default)]
    pub capabilities: Vec<CapabilityKind>,
    #[serde(default)]
    pub root_missing: bool,
    /// Epoch milliseconds this project was last opened or activated (IPC time-field convention —
    /// see `docs/data-model.md` §6's `f64` epoch-ms fields). `#[serde(default)]` reads a pre-d-27
    /// project record (no such field on disk) as `0.0`, so it sorts last in
    /// `service::list_recent_projects` rather than failing to parse — a decorative field earning
    /// its default rather than a migration (contract §1.3).
    #[serde(default)]
    pub last_opened_at: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub version: u32,
    #[serde(default)]
    pub projects: Vec<ProjectRef>,
    #[serde(default)]
    pub active_project: Option<ProjectId>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            version: SESSION_SCHEMA_VERSION,
            projects: Vec::new(),
            active_project: None,
        }
    }
}
