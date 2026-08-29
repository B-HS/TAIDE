use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::app::types::AppFileTarget;
use crate::domain::search::types::SearchQuery;
use crate::ids::{PaneId, TabId};

/// v2 adds the auxiliary-window axis (`ProjectLayout::auxiliary_windows`) and per-project shell
/// chrome state (`ProjectLayout::shell_view`) — both purely additive over v1, since every new field
/// already deserializes via `#[serde(default)]`. See `service::migrate_layout` and
/// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.2.
pub const LAYOUT_SCHEMA_VERSION: u32 = 2;
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
    /// `rev`/`parent_rev`/`before_path` are optional so a tab can additionally represent a
    /// *commit* diff (`git_show_file(rev, path)` vs `git_show_file(parent_rev, before_path ??
    /// path)`), reusing the same `TabKind::Diff` the working-tree/staged diff already uses instead
    /// of a dedicated variant — see
    /// `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §2.6. `#[serde(default)]` on all
    /// three keeps a pre-existing `layout.json` (written before this field set existed)
    /// deserializing unchanged, the same backward-compatibility `compare_with` already relies on.
    #[serde(rename_all = "camelCase")]
    Diff {
        path: String,
        staged: bool,
        #[serde(default)]
        compare_with: Option<String>,
        #[serde(default)]
        rev: Option<String>,
        #[serde(default)]
        parent_rev: Option<String>,
        #[serde(default)]
        before_path: Option<String>,
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
    /// and streaming a potentially large `SearchFileMatches` list through hot-exit/session-restore
    /// JSON would bloat the layout file for no benefit, so the tab restores to the query and
    /// re-searches rather than replaying stale results. See
    /// `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4.
    #[serde(rename_all = "camelCase")]
    SearchEditor {
        query: SearchQuery,
    },
    /// An app-owned config file (`settings.json` or a prompt template override) opened as a tab —
    /// unlike `File`, its path is never carried in the tab itself: `target` is a closed enum Rust
    /// resolves to an `AppPaths`-derived location (`app::service::app_file_path`), so neither the
    /// layout JSON nor the frontend ever sees an absolute path for it. See
    /// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.3.
    #[serde(rename_all = "camelCase")]
    AppFile {
        target: AppFileTarget,
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

/// One auxiliary editor window's own pane tree, keyed by `slot` — a project-scoped semantic id
/// (allocated by `service::next_window_slot`) distinct from the OS-level Tauri window label
/// (`editor-<n>`, allocated globally by `domain::window::service::next_auxiliary_label`). A window
/// close returns its tabs to the main tree's tail and drops this entry
/// (`service::return_auxiliary_window_tabs`) — TAIDE's 0-loss philosophy, unlike VS Code discarding
/// an auxiliary window's content on close.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AuxWindowLayout {
    pub slot: u32,
    pub root: PaneNode,
    pub focused_pane: PaneId,
}

/// Per-project display-chrome state for the **main** window only — auxiliary windows are
/// sidebar/status-bar-less editor chrome by design (contract §3.2), so this isn't per-window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShellViewState {
    #[serde(default)]
    pub zen: bool,
    #[serde(default)]
    pub sidebar_collapsed: bool,
}

/// Partial update for [`ShellViewState`] — `None` fields are left at their current value, same
/// merge convention as `settings::types::SettingsPatch`/`settings::service::apply_patch`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShellViewPatch {
    pub zen: Option<bool>,
    pub sidebar_collapsed: Option<bool>,
}

/// Destination for `layout_move_tab_to_window`. `Existing` addresses an auxiliary window already
/// recorded in `ProjectLayout::auxiliary_windows` by its project-scoped `slot`; `NewAuxiliary`
/// allocates a fresh slot (`service::next_window_slot`) and asks `domain::window` to open a real OS
/// window for it before the tab lands there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TabWindowTarget {
    Main,
    NewAuxiliary,
    #[serde(rename_all = "camelCase")]
    Existing {
        slot: u32,
    },
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
    #[serde(default)]
    pub auxiliary_windows: Vec<AuxWindowLayout>,
    #[serde(default)]
    pub shell_view: ShellViewState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClosedTab {
    pub tab: Tab,
    pub pane_id: PaneId,
    pub index: u32,
}

/// What a completed `file_rename`/`file_delete` means for the tabs that address the changed path —
/// the input of `layout_apply_path_change`. `from`/`to`/`path` are the very strings the frontend
/// handed the file command, matched against `TabKind::File`'s stored path by path components
/// (`service::retargeted_path`), never canonicalized again here: after the rename `from` no longer
/// exists on disk, so there is nothing left to canonicalize it against. Both variants address a
/// whole subtree when they name a directory. See
/// `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §3 S8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TabPathChange {
    #[serde(rename_all = "camelCase")]
    Renamed { from: String, to: String },
    #[serde(rename_all = "camelCase")]
    Deleted { path: String },
}

/// One distinct open file path a [`TabPathChange::Renamed`] moved. Reported back so the frontend can
/// carry that path's own per-path state (monaco model, hot-exit mirror, `FILE.CONTENT` cache,
/// "reopen with" override) to the new path instead of leaving it stranded under a path that no
/// longer exists. `dirty` is true when any moved tab for `from` had unsaved edits — the frontend
/// needs that to decide whether a draft has to be re-mirrored under `to` before the pane, which
/// resets to clean on a path switch, syncs the model back to the new path's disk content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TabPathMove {
    pub from: String,
    pub to: String,
    pub dirty: bool,
}

/// `layout_apply_path_change`'s response: the post-change layout (same value every other layout
/// mutation returns, so the frontend's `applyFreshLayout` revision gate still applies) plus what
/// the change did to file tabs — `moved` for a rename, `closed_paths` for a delete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TabPathChangeResult {
    pub layout: ProjectLayout,
    pub moved: Vec<TabPathMove>,
    pub closed_paths: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 구버전_diff_tabkind_json은_새_필드가_없어도_역직렬화된다() {
        let legacy_json = serde_json::json!({
            "kind": "diff",
            "path": "src/main.rs",
            "staged": false,
            "compareWith": null,
        });

        let kind: TabKind = serde_json::from_value(legacy_json).expect("구버전 Diff JSON 역직렬화 성공");

        assert_eq!(
            kind,
            TabKind::Diff {
                path: "src/main.rs".to_string(),
                staged: false,
                compare_with: None,
                rev: None,
                parent_rev: None,
                before_path: None,
            }
        );
    }

    #[test]
    fn 커밋_diff용_필드가_포함된_json도_역직렬화된다() {
        let commit_diff_json = serde_json::json!({
            "kind": "diff",
            "path": "src/main.rs",
            "staged": false,
            "rev": "abc1234",
            "parentRev": "def5678",
            "beforePath": "src/old-main.rs",
        });

        let kind: TabKind = serde_json::from_value(commit_diff_json).expect("커밋 diff JSON 역직렬화 성공");

        assert_eq!(
            kind,
            TabKind::Diff {
                path: "src/main.rs".to_string(),
                staged: false,
                compare_with: None,
                rev: Some("abc1234".to_string()),
                parent_rev: Some("def5678".to_string()),
                before_path: Some("src/old-main.rs".to_string()),
            }
        );
    }
}
