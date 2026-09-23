use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use crate::layout::SplitDir;

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
    /// Mirror of `Project.root_missing`, recomputed against the live filesystem wherever that field
    /// is (`service::restore_session`, `service::open_project`) and written through by
    /// `service::upsert_project_ref` — the same reason `display` is mirrored.
    ///
    /// Without it the one surface that shows a restored project — the sidebar rail and its slot
    /// header, both of which render from `project_list` alone — had no way to know the folder is
    /// gone, so a project whose drive was unplugged came back looking perfectly healthy with an
    /// empty file tree, while `Open Recent` and the Welcome list (which read `Project`) disabled
    /// the very same entry.
    ///
    /// `#[serde(default)]` reads a pre-d-67 `session.json` (no such field) as `false`; the next
    /// `restore_session` recomputes it from disk anyway.
    #[serde(default)]
    pub root_missing: bool,
}

/// What one `service::forget_recent_projects` call did. The three answers are deliberately
/// separate: most calls delete records that no group ever listed, so `removed` being non-zero says
/// nothing about whether the sidebar's groups moved, and `skipped_with_drafts` counts records the
/// call refused to touch rather than ones it failed to.
///
/// `project_forget_recent` emits `ProjectGroupsChanged` only when `groups_changed` is true — every
/// window re-reads its group query on that event, and a clear-recent that touched no membership has
/// nothing for them to re-read. `skipped_with_drafts` is what the frontend turns into the "초안이
/// 있는 프로젝트는 남겨 두었습니다" notice, so the user learns why the list did not empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ForgetRecentOutcome {
    pub removed: u32,
    pub skipped_with_drafts: u32,
    pub groups_changed: bool,
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

/// How the main window is divided between whole project shells — the d-62 "shell slot" tree. Each
/// `Leaf` holds one open project (identified for IPC by its own [`ShellSlotId`]), each `Split` two
/// children side by side, so the shape mirrors `layout::types::PaneNode` without being it: a pane
/// leaf's payload is a tab list, a shell slot leaf's payload is a project, and the two trees nest
/// (every slot renders its project's whole pane tree inside itself). Deliberately **binary** —
/// unlike `PaneNode`, which flattens same-direction splits into n-ary nodes — because a slot split
/// is created one drop at a time and a fixed arity keeps `sizes` addressable by child index
/// (`service::set_shell_slot_sizes`, which is how the resize commit names a node: a `Split` carries
/// no id of its own).
///
/// `SplitDir` is reused from the layout domain rather than re-declared: it is the same geometry the
/// frontend hands `react-resizable-panels`, and one shared enum keeps the two trees' direction
/// values from drifting apart.
///
/// Naming: "shell slot", never "window slot" — `layout::types::AuxWindowLayout::slot` is a
/// *different* concept (an auxiliary OS window), and contract §0.1 S-8 keeps the two apart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "node", rename_all = "camelCase")]
pub enum ShellSlotTree {
    #[serde(rename_all = "camelCase")]
    Split {
        dir: SplitDir,
        children: Vec<ShellSlotTree>,
        sizes: Vec<f32>,
    },
    #[serde(rename_all = "camelCase")]
    Leaf { slot_id: ShellSlotId, project_id: ProjectId },
}

/// Where `project_open_in_slot` puts a project relative to the slot it was dropped on. The four
/// directional edges create a split; `Replace` swaps the target slot's project in place, which is
/// what the sidebar's plain click has always done to the single slot. Distinct from
/// `layout::types::DropEdge` on purpose: that enum's `Center` means "into this pane's tab strip",
/// which has no meaning for a slot (a slot holds one project, not a list).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ShellSlotEdge {
    Left,
    Right,
    Top,
    Bottom,
    Replace,
}

/// Window-level chrome state, moved off `layout::types::ShellViewState` (which is per *project*) by
/// contract §0.1 S-6: the sidebar icon rail and Zen mode are properties of the one window, so
/// keeping them per project made them flicker as the focused slot changed. The per-project struct
/// keeps its fields for backward compatibility — `service::promote_legacy_window_chrome` reads them
/// once at boot and clears them — and keeps owning the slot-local axes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowChrome {
    #[serde(default)]
    pub zen: bool,
    #[serde(default)]
    pub sidebar_rail_collapsed: bool,
}

/// Partial update for [`WindowChrome`] — `None` leaves that axis alone, the same merge convention as
/// `layout::types::ShellViewPatch` and `settings::types::SettingsPatch`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowChromePatch {
    pub zen: Option<bool>,
    pub sidebar_rail_collapsed: Option<bool>,
}

/// Everything `session_get_shell_state` answers with — the boot-time read of the state
/// `SessionShellSlotsChanged`/`WindowChromeChanged` deliver on change afterwards. It exists for the
/// same reason `project_get_active` does: an event only fires at a transition, so a window that
/// just mounted has no other way to learn the current arrangement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionShellState {
    pub tree: Option<ShellSlotTree>,
    pub focused: Option<ShellSlotId>,
    pub window_chrome: WindowChrome,
}

/// `project_open_in_slot`'s payload. Exactly one of `path`/`project_id` must be present: a drop from
/// the sidebar names an already-open project, while "open a folder to the right" names a path that
/// may not be open yet. Grouped into one struct like `layout::types::OpenTabInSplitRequest` so the
/// command stays under `clippy::too_many_arguments`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectInSlotRequest {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub target_slot: ShellSlotId,
    pub edge: ShellSlotEdge,
}

/// A named, ordered bundle of projects in the sidebar (d-62 §1.C). A group is an organization of
/// things the user *can* open, not a record of what is open: `session.projects` stays the single
/// truth for both open-ness and the global sidebar order, and `members` is only a membership set —
/// closing a project leaves it in its group, and only forgetting its record
/// (`service::forget_recent_projects`, contract §0.1 S-7) takes it out.
///
/// `color` reuses `ProjectDisplay`'s palette (`service::sanitize_display_color`'s
/// `graph.lane1..lane12` allow-list) rather than declaring a second color vocabulary, so a group
/// header and a project icon can be tinted from the same theme tokens.
///
/// A project belongs to **at most one** group: `service::set_group_members` and
/// `service::create_group` take a project away from whatever group held it before. Without that
/// rule the sidebar would have to render the same project under two headers, and the contract's
/// own "그룹에서 제거" context-menu item (§0.1 U-6) would have no single group to name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGroup {
    pub id: ProjectGroupId,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub members: Vec<ProjectId>,
    #[serde(default)]
    pub collapsed: bool,
}

/// What `project_group_open` answers with: `opened` lists the members this call actually opened, in
/// the order it opened them (the first of them is the one that took focus), and `skipped` lists the
/// members it deliberately passed over — already open, record gone from disk, root no longer a
/// directory, or never reached because a shutdown interrupted the queue.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGroupOpenResult {
    pub opened: Vec<ProjectId>,
    pub skipped: Vec<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub version: u32,
    #[serde(default)]
    pub projects: Vec<ProjectRef>,
    /// The project of the **focused shell slot** since d-62 — the field kept its name and its
    /// meaning for the single-slot case, so every existing reader (`project_get_active`, the native
    /// menu, the frontend's `activeProjectQueryOptions`) stays correct without knowing about slots.
    #[serde(default)]
    pub active_project: Option<ProjectId>,
    /// `None` means "no slots materialized" — either nothing is open, or this session was written
    /// before d-62. `service::normalize_shell_slots` resolves the latter into a single leaf holding
    /// `active_project` at boot, which is why no schema migration is needed
    /// (`docs/data-model.md` §2's field-addition rule, the route `ProjectDisplay` took in §20).
    #[serde(default)]
    pub shell_slots: Option<ShellSlotTree>,
    #[serde(default)]
    pub focused_shell_slot: Option<ShellSlotId>,
    #[serde(default)]
    pub window_chrome: WindowChrome,
    /// Sidebar project groups in display order — see [`ProjectGroup`]. `#[serde(default)]` reads a
    /// session written before d-62 2c as "no groups", so this is another field addition rather than
    /// a schema migration (`docs/data-model.md` §5's rule, §23).
    #[serde(default)]
    pub groups: Vec<ProjectGroup>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            version: SESSION_SCHEMA_VERSION,
            projects: Vec::new(),
            active_project: None,
            shell_slots: None,
            focused_shell_slot: None,
            window_chrome: WindowChrome::default(),
            groups: Vec::new(),
        }
    }
}
