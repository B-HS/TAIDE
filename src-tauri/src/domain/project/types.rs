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

/// Per-project sidebar presentation overrides: a curated lucide icon name, a 1–4 codepoint text
/// label, and a `graph.laneN` color token, each independently optional and each `None` by default
/// (the plain folder icon). Written only through `service::set_project_display`, which sanitizes
/// every axis — nothing else in this domain may widen these shapes. `#[serde(default)]` (here and
/// on both owners' fields) reads a record written before this field existed as the all-`None`
/// default, so this is a decorative field earning its default rather than a schema migration
/// (`docs/data-model.md` §2's field-addition rule, the same route `last_opened_at` took in d-27).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDisplay {
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

/// A partial [`ProjectDisplay`] update, one axis per field, following the settings domain's
/// clearable-string convention (`domain::settings::service::merge_clearable_string`): `None`
/// leaves that axis untouched, `Some("")` clears it back to the default, and any other `Some`
/// replaces it. A plain `Option<String>` per axis could otherwise only express "touch" and
/// "don't touch", collapsing "reset this axis" and "leave it alone" into the same `None` — so the
/// display dialog can set an icon, clear a label, and leave the color alone in one call.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDisplayPatch {
    pub icon: Option<String>,
    pub label: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRef {
    pub id: ProjectId,
    pub root: String,
    pub name: String,
    /// Mirror of `Project.display`, kept in sync by `service::upsert_project_ref` exactly like
    /// `root`/`name` — the sidebar renders from `project_list`'s `ProjectRef[]` alone, so without
    /// this mirror it would need one `project_get` per project just to draw an icon.
    #[serde(default)]
    pub display: ProjectDisplay,
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
    /// Sidebar presentation overrides — see [`ProjectDisplay`]. Persisted here (in
    /// `projects/<id>/project.json`) as the source of truth and mirrored into `ProjectRef` so a
    /// project keeps its icon/label/color across close and re-open.
    #[serde(default)]
    pub display: ProjectDisplay,
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
