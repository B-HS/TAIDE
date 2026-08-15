use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::search::types::SearchQuery;
use crate::ids::{PaneId, TabId};

pub const LAYOUT_SCHEMA_VERSION: u32 = 1;
pub const CLOSED_TAB_STACK_LIMIT: usize = 20;
pub const FIRST_UNTITLED_INDEX: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SplitDir {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DropEdge {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TabKind {
    #[serde(rename_all = "camelCase")]
    File {
        path: String,
    },
    #[serde(rename_all = "camelCase")]
    Terminal {
        session_id: String,
        #[serde(default)]
        cwd: Option<String>,
    },
    Settings,
    #[serde(rename_all = "camelCase")]
    Diff {
        path: String,
        staged: bool,
        #[serde(default)]
        compare_with: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    ClaudeDiff {
        request_id: String,
        path: String,
    },
    Welcome,
    #[serde(rename_all = "camelCase")]
    Untitled {
        index: u32,
    },
    /// A persistent, editable search results surface (VS Code calls this a "Search Editor").
    /// Only the query is kept — results are cheap to re-run (`search::commands::search_run`)
    /// and streaming a potentially large `SearchMatch` list through hot-exit/session-restore
    /// JSON would bloat the layout file for no benefit, so the tab restores to the query and
    /// re-searches rather than replaying stale results. See
    /// `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4.
    #[serde(rename_all = "camelCase")]
    SearchEditor {
        query: SearchQuery,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: TabId,
    pub kind: TabKind,
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub preview: bool,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    pub view_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "node", rename_all = "camelCase")]
pub enum PaneNode {
    #[serde(rename_all = "camelCase")]
    Split {
        id: PaneId,
        dir: SplitDir,
        children: Vec<PaneNode>,
        sizes: Vec<f32>,
    },
    #[serde(rename_all = "camelCase")]
    Leaf { id: PaneId, tabs: Vec<Tab>, active: Option<TabId> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLayout {
    pub version: u32,
    pub root: PaneNode,
    pub focused_pane: PaneId,
    #[serde(default)]
    pub revision: u32,
    #[serde(default)]
    pub closed_tabs: Vec<ClosedTab>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClosedTab {
    pub tab: Tab,
    pub pane_id: PaneId,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FocusKind {
    File,
    Terminal,
    Settings,
    Diff,
    ClaudeDiff,
    Welcome,
    Untitled,
    SearchEditor,
}

impl From<&TabKind> for FocusKind {
    fn from(value: &TabKind) -> Self {
        match value {
            TabKind::File { .. } => FocusKind::File,
            TabKind::Terminal { .. } => FocusKind::Terminal,
            TabKind::Settings => FocusKind::Settings,
            TabKind::Diff { .. } => FocusKind::Diff,
            TabKind::ClaudeDiff { .. } => FocusKind::ClaudeDiff,
            TabKind::Welcome => FocusKind::Welcome,
            TabKind::Untitled { .. } => FocusKind::Untitled,
            TabKind::SearchEditor { .. } => FocusKind::SearchEditor,
        }
    }
}
