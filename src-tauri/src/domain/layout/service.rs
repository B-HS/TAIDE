use std::collections::{HashMap, HashSet};
use std::path::Path;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use super::types::{
    AuxWindowLayout, ClosedTab, DropEdge, PaneNode, ProjectLayout, ShellViewPatch, ShellViewState, SplitDir, Tab, TabKind, TabPathChange,
    TabPathMove, CLOSED_TAB_STACK_LIMIT, FIRST_UNTITLED_INDEX, LAYOUT_SCHEMA_VERSION,
};
use crate::error::{AppError, AppResult};
use crate::events::LayoutChanged;
use crate::ids::{PaneId, ProjectId, TabId};
use crate::infra::persist;
use crate::paths::AppPaths;
use crate::state::AppState;

const SPLIT_TOTAL_PERCENT: f32 = 100.0;
const FIRST_WINDOW_SLOT: u32 = 1;

/// Sentinel `index` for [`move_tab`] meaning "append at the end of the target pane's tabs" —
/// `insert_tab`'s `index.unwrap_or(tabs.len()).min(tabs.len())` clamp turns any out-of-range index
/// into the tab count, so `usize::MAX` always lands there regardless of how many tabs the target
/// pane holds.
const APPEND_AT_END: usize = usize::MAX;

/// Identifies which of a `ProjectLayout`'s pane trees a pane/tab lives in — the main tree, or one
/// auxiliary window's own tree (indexed into `ProjectLayout::auxiliary_windows`). Every pane/tab
/// mutation locates this first so it can operate on (and, for focus bookkeeping, update) the right
/// tree regardless of which OS window the caller is in. See
/// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.1/§3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneTreeRef {
    Main,
    Auxiliary(usize),
}

/// Every pane tree a project's layout owns — the main tree followed by each auxiliary window's own
/// tree, in `auxiliary_windows` order. Callers that only care "does this project have the tab/pane
/// anywhere" ([`locate_project_with_tab`]/[`locate_project_with_pane`] below and the
/// `ide::service`/`ide::server` snapshots) iterate
/// this instead of hardcoding `&layout.root`, so auxiliary-window content isn't invisible to them.
pub fn all_roots(layout: &ProjectLayout) -> impl Iterator<Item = &PaneNode> {
    std::iter::once(&layout.root).chain(layout.auxiliary_windows.iter().map(|window| &window.root))
}

fn tree_root(layout: &ProjectLayout, tree: PaneTreeRef) -> &PaneNode {
    match tree {
        PaneTreeRef::Main => &layout.root,
        PaneTreeRef::Auxiliary(index) => &layout.auxiliary_windows[index].root,
    }
}

fn tree_root_mut(layout: &mut ProjectLayout, tree: PaneTreeRef) -> &mut PaneNode {
    match tree {
        PaneTreeRef::Main => &mut layout.root,
        PaneTreeRef::Auxiliary(index) => &mut layout.auxiliary_windows[index].root,
    }
}

fn set_tree_focused_pane(layout: &mut ProjectLayout, tree: PaneTreeRef, pane_id: PaneId) {
    match tree {
        PaneTreeRef::Main => layout.focused_pane = pane_id,
        PaneTreeRef::Auxiliary(index) => layout.auxiliary_windows[index].focused_pane = pane_id,
    }
}

fn locate_tree_of_pane(layout: &ProjectLayout, pane_id: &PaneId) -> Option<PaneTreeRef> {
    if contains_pane(&layout.root, pane_id) {
        return Some(PaneTreeRef::Main);
    }
    layout
        .auxiliary_windows
        .iter()
        .position(|window| contains_pane(&window.root, pane_id))
        .map(PaneTreeRef::Auxiliary)
}

fn locate_tree_of_tab(layout: &ProjectLayout, tab_id: &TabId) -> Option<PaneTreeRef> {
    if find_tab(&layout.root, tab_id).is_some() {
        return Some(PaneTreeRef::Main);
    }
    layout
        .auxiliary_windows
        .iter()
        .position(|window| find_tab(&window.root, tab_id).is_some())
        .map(PaneTreeRef::Auxiliary)
}

/// Finds a tab regardless of which tree it lives in — used by mutations that only need the `Tab`
/// itself (`set_preview`/`set_dirty`/`set_view_state`/`set_terminal_session`), not the pane/focus
/// bookkeeping `PaneTreeRef`-aware operations need.
fn find_tab_mut_in_layout<'a>(layout: &'a mut ProjectLayout, tab_id: &TabId) -> Option<&'a mut Tab> {
    if let Some(tab) = find_tab_mut(&mut layout.root, tab_id) {
        return Some(tab);
    }
    layout
        .auxiliary_windows
        .iter_mut()
        .find_map(|window| find_tab_mut(&mut window.root, tab_id))
}

/// A pane tree with no tabs anywhere in it — always a single empty `Leaf` once `normalize` has run,
/// since `normalize_owned` collapses every empty-leaf-only split down to one. Used to decide when an
/// auxiliary window's entry (and its now-pointless OS window) should be cleaned up.
pub fn is_layout_tree_empty(root: &PaneNode) -> bool {
    matches!(root, PaneNode::Leaf { tabs, .. } if tabs.is_empty())
}

pub fn default_layout() -> ProjectLayout {
    let welcome = Tab {
        id: TabId::new(),
        kind: super::types::TabKind::Welcome,
        title: "Welcome".to_string(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    let terminal = Tab {
        id: TabId::new(),
        kind: super::types::TabKind::Terminal {
            session_id: String::new(),
            cwd: None,
        },
        title: "Terminal".to_string(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    let active = welcome.id.clone();
    let leaf_id = PaneId::new();
    let root = PaneNode::Leaf {
        id: leaf_id.clone(),
        tabs: vec![welcome, terminal],
        active: Some(active),
    };

    ProjectLayout {
        version: LAYOUT_SCHEMA_VERSION,
        root,
        focused_pane: leaf_id,
        revision: 0,
        closed_tabs: Vec::new(),
        auxiliary_windows: Vec::new(),
        shell_view: ShellViewState::default(),
    }
}

pub fn pane_id_of(node: &PaneNode) -> &PaneId {
    match node {
        PaneNode::Split { id, .. } => id,
        PaneNode::Leaf { id, .. } => id,
    }
}

pub fn contains_pane(node: &PaneNode, pane_id: &PaneId) -> bool {
    if pane_id_of(node) == pane_id {
        return true;
    }
    match node {
        PaneNode::Split { children, .. } => children.iter().any(|child| contains_pane(child, pane_id)),
        PaneNode::Leaf { .. } => false,
    }
}

pub fn find_leaf<'a>(node: &'a PaneNode, pane_id: &PaneId) -> Option<&'a PaneNode> {
    match node {
        PaneNode::Leaf { id, .. } if id == pane_id => Some(node),
        PaneNode::Leaf { .. } => None,
        PaneNode::Split { children, .. } => children.iter().find_map(|child| find_leaf(child, pane_id)),
    }
}

fn find_leaf_mut<'a>(node: &'a mut PaneNode, pane_id: &PaneId) -> Option<&'a mut PaneNode> {
    let is_match = matches!(node, PaneNode::Leaf { id, .. } if id == pane_id);
    if is_match {
        return Some(node);
    }
    match node {
        PaneNode::Leaf { .. } => None,
        PaneNode::Split { children, .. } => children.iter_mut().find_map(|child| find_leaf_mut(child, pane_id)),
    }
}

fn find_split_mut<'a>(node: &'a mut PaneNode, pane_id: &PaneId) -> Option<&'a mut PaneNode> {
    let is_match = matches!(node, PaneNode::Split { id, .. } if id == pane_id);
    if is_match {
        return Some(node);
    }
    match node {
        PaneNode::Leaf { .. } => None,
        PaneNode::Split { children, .. } => children.iter_mut().find_map(|child| find_split_mut(child, pane_id)),
    }
}

pub fn find_tab<'a>(node: &'a PaneNode, tab_id: &TabId) -> Option<(&'a PaneId, usize)> {
    match node {
        PaneNode::Leaf { id, tabs, .. } => tabs.iter().position(|tab| &tab.id == tab_id).map(|index| (id, index)),
        PaneNode::Split { children, .. } => children.iter().find_map(|child| find_tab(child, tab_id)),
    }
}

fn find_tab_mut<'a>(node: &'a mut PaneNode, tab_id: &TabId) -> Option<&'a mut Tab> {
    match node {
        PaneNode::Leaf { tabs, .. } => tabs.iter_mut().find(|tab| &tab.id == tab_id),
        PaneNode::Split { children, .. } => children.iter_mut().find_map(|child| find_tab_mut(child, tab_id)),
    }
}

pub fn collect_leaves(node: &PaneNode) -> Vec<&PaneNode> {
    match node {
        PaneNode::Leaf { .. } => vec![node],
        PaneNode::Split { children, .. } => children.iter().flat_map(collect_leaves).collect(),
    }
}

/// Every distinct `File` tab path currently open across `layout`'s main tree and its auxiliary
/// windows — the same `all_roots` + `collect_leaves` walk `domain::ide::service::
/// open_editors_snapshot` uses, without that function's IDE-specific fields (`is_active`/`label`/
/// `language_id`). Used by `domain::project::commands::restore_project_watchers` to synthesize one `FsChanged`
/// refresh for a project's already-open tabs right after its watcher goes live — see that
/// function's doc (boot-watcher-defer contract §3.1's `FILE.CONTENT`/`FILE.RAW` gap).
pub fn open_file_paths(layout: &ProjectLayout) -> Vec<String> {
    let mut paths = HashSet::new();
    for root in all_roots(layout) {
        for node in collect_leaves(root) {
            let PaneNode::Leaf { tabs, .. } = node else { continue };
            for tab in tabs {
                if let TabKind::File { path } = &tab.kind {
                    paths.insert(path.clone());
                }
            }
        }
    }
    paths.into_iter().collect()
}

/// title 이 일치하는 첫 탭을 찾는다(Claude Code 의 `close_tab(tab_name)` 처럼 안정적인 id 대신
/// 표시 이름으로 탭을 지칭하는 외부 프로토콜을 위한 헬퍼).
pub fn find_tab_by_title(node: &PaneNode, title: &str) -> Option<TabId> {
    match node {
        PaneNode::Leaf { tabs, .. } => tabs.iter().find(|tab| tab.title == title).map(|tab| tab.id.clone()),
        PaneNode::Split { children, .. } => children.iter().find_map(|child| find_tab_by_title(child, title)),
    }
}

/// 열려 있는 모든 ClaudeDiff 탭의 id 를 모은다(`closeAllDiffTabs` 용).
pub fn collect_claude_diff_tab_ids(node: &PaneNode) -> Vec<TabId> {
    match node {
        PaneNode::Leaf { tabs, .. } => tabs
            .iter()
            .filter(|tab| matches!(tab.kind, TabKind::ClaudeDiff { .. }))
            .map(|tab| tab.id.clone())
            .collect(),
        PaneNode::Split { children, .. } => children.iter().flat_map(collect_claude_diff_tab_ids).collect(),
    }
}

/// ClaudeDiff 탭이면 그 탭이 붙잡고 있던 IDE diff 요청 id 를 돌려준다.
pub fn claude_diff_request_id(tab: &Tab) -> Option<String> {
    match &tab.kind {
        TabKind::ClaudeDiff { request_id, .. } => Some(request_id.clone()),
        _ => None,
    }
}

fn is_empty_leaf(node: &PaneNode) -> bool {
    matches!(node, PaneNode::Leaf { tabs, .. } if tabs.is_empty())
}

fn extract_tab(node: &mut PaneNode, tab_id: &TabId) -> Option<Tab> {
    match node {
        PaneNode::Leaf { tabs, active, .. } => {
            let index = tabs.iter().position(|tab| &tab.id == tab_id)?;
            let tab = tabs.remove(index);
            if active.as_ref() == Some(&tab.id) {
                *active = if tabs.is_empty() {
                    None
                } else {
                    Some(tabs[index.min(tabs.len() - 1)].id.clone())
                };
            }
            Some(tab)
        }
        PaneNode::Split { children, .. } => children.iter_mut().find_map(|child| extract_tab(child, tab_id)),
    }
}

fn insert_tab(leaf: &mut PaneNode, tab: Tab, index: Option<usize>) {
    if let PaneNode::Leaf { tabs, active, .. } = leaf {
        let position = index.unwrap_or(tabs.len()).min(tabs.len());
        let id = tab.id.clone();
        tabs.insert(position, tab);
        *active = Some(id);
    }
}

fn wrap_leaf_in_split(existing: PaneNode, new_leaf: PaneNode, dir: SplitDir, edge: DropEdge) -> PaneNode {
    let (first, second) = match edge {
        DropEdge::Left | DropEdge::Top => (new_leaf, existing),
        _ => (existing, new_leaf),
    };
    PaneNode::Split {
        id: PaneId::new(),
        dir,
        children: vec![first, second],
        sizes: vec![SPLIT_TOTAL_PERCENT / 2.0, SPLIT_TOTAL_PERCENT / 2.0],
    }
}

fn insert_split_at(node: &mut PaneNode, target: &PaneId, dir: SplitDir, edge: DropEdge, pending: &mut Option<PaneNode>) {
    if pending.is_none() {
        return;
    }
    if let PaneNode::Split {
        dir: node_dir,
        children,
        sizes,
        ..
    } = node
    {
        if let Some(pos) = children.iter().position(|child| pane_id_of(child) == target) {
            let Some(new_leaf) = pending.take() else { return };
            if *node_dir == dir {
                let insert_at = match edge {
                    DropEdge::Left | DropEdge::Top => pos,
                    _ => pos + 1,
                };
                let old_size = sizes[pos];
                let half = old_size / 2.0;
                sizes[pos] = half;
                sizes.insert(insert_at, half);
                children.insert(insert_at, new_leaf);
            } else {
                let existing = children.remove(pos);
                let old_size = sizes.remove(pos);
                let wrapped = wrap_leaf_in_split(existing, new_leaf, dir, edge);
                children.insert(pos, wrapped);
                sizes.insert(pos, old_size);
            }
            return;
        }
        for child in children.iter_mut() {
            insert_split_at(child, target, dir, edge, pending);
            if pending.is_none() {
                return;
            }
        }
    }
}

pub fn normalize(root: &mut PaneNode) {
    let placeholder = PaneNode::Leaf {
        id: PaneId::new(),
        tabs: Vec::new(),
        active: None,
    };
    let taken = std::mem::replace(root, placeholder);
    *root = normalize_owned(taken);
}

fn normalize_owned(node: PaneNode) -> PaneNode {
    match node {
        PaneNode::Leaf { .. } => node,
        PaneNode::Split { id, dir, children, sizes } => {
            let mut new_children: Vec<PaneNode> = Vec::new();
            let mut new_sizes: Vec<f32> = Vec::new();

            for (child, size) in children.into_iter().zip(sizes) {
                let normalized_child = normalize_owned(child);
                if is_empty_leaf(&normalized_child) {
                    continue;
                }

                match normalized_child {
                    PaneNode::Split {
                        dir: child_dir,
                        children: grandchildren,
                        sizes: grandchild_sizes,
                        ..
                    } if child_dir == dir => {
                        let total: f32 = grandchild_sizes.iter().sum();
                        let count = grandchild_sizes.len().max(1) as f32;
                        for (grandchild, grandchild_size) in grandchildren.into_iter().zip(grandchild_sizes) {
                            let portion = if total > 0.0 {
                                size * (grandchild_size / total)
                            } else {
                                size / count
                            };
                            new_children.push(grandchild);
                            new_sizes.push(portion);
                        }
                    }
                    other => {
                        new_children.push(other);
                        new_sizes.push(size);
                    }
                }
            }

            if new_children.is_empty() {
                return PaneNode::Leaf {
                    id: PaneId::new(),
                    tabs: Vec::new(),
                    active: None,
                };
            }
            if new_children.len() == 1 {
                return new_children.into_iter().next().unwrap_or_else(|| PaneNode::Leaf {
                    id,
                    tabs: Vec::new(),
                    active: None,
                });
            }

            PaneNode::Split {
                id,
                dir,
                children: new_children,
                sizes: new_sizes,
            }
        }
    }
}

pub fn open_tab(layout: &mut ProjectLayout, pane_id: &PaneId, mut tab: Tab, preview: bool) -> AppResult<TabId> {
    let tree = locate_tree_of_pane(layout, pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    let leaf =
        find_leaf_mut(tree_root_mut(layout, tree), pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    let PaneNode::Leaf { tabs, active, .. } = leaf else {
        return Err(AppError::Internal("expected leaf pane".to_string()));
    };

    if let Some(existing) = tabs.iter_mut().find(|existing| existing.kind == tab.kind) {
        let id = existing.id.clone();
        if !preview && existing.preview {
            existing.preview = false;
        }
        *active = Some(id.clone());
        layout.revision += 1;
        return Ok(id);
    }

    tab.preview = preview;
    let id = tab.id.clone();
    if preview {
        if let Some(pos) = tabs.iter().position(|existing| existing.preview) {
            tabs[pos] = tab;
        } else {
            tabs.push(tab);
        }
    } else {
        tabs.push(tab);
    }
    *active = Some(id.clone());
    layout.revision += 1;
    Ok(id)
}

/// 현재 열려 있는 Untitled 탭들이 쓰지 않는 최소 번호를 돌려준다(1부터, VSCode 와 동일한 번호 재사용).
/// Untitled 탭은 `closed_tabs` 에 남지 않으므로(`push_closed`) 열린 탭만 훑으면 된다.
pub fn next_untitled_index(layout: &ProjectLayout) -> u32 {
    let used: HashSet<u32> = all_roots(layout)
        .flat_map(collect_leaves)
        .filter_map(|leaf| match leaf {
            PaneNode::Leaf { tabs, .. } => Some(tabs),
            PaneNode::Split { .. } => None,
        })
        .flatten()
        .filter_map(|tab| match &tab.kind {
            TabKind::Untitled { index } => Some(*index),
            _ => None,
        })
        .collect();

    let mut candidate = FIRST_UNTITLED_INDEX;
    while used.contains(&candidate) {
        candidate += 1;
    }
    candidate
}

/// Untitled 탭을 저장된 파일 탭으로 in-place 치환한다. 같은 leaf 에 동일 경로의 File 탭이
/// 이미 있으면 그 탭을 재사용하고(중복 탭 방지 — `open_tab` 의 중복 정책과 동일 철학) untitled 탭은 제거한다.
pub fn convert_untitled_to_file(layout: &mut ProjectLayout, tab_id: &TabId, path: String, title: String) -> AppResult<TabId> {
    let tree = locate_tree_of_tab(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let (pane_id, _) = find_tab(tree_root(layout, tree), tab_id)
        .map(|(pane_id, index)| (pane_id.clone(), index))
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;

    let leaf = find_leaf(tree_root(layout, tree), &pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    let PaneNode::Leaf { tabs, .. } = leaf else {
        return Err(AppError::Internal("expected leaf pane".to_string()));
    };
    if !tabs
        .iter()
        .any(|tab| &tab.id == tab_id && matches!(tab.kind, TabKind::Untitled { .. }))
    {
        return Err(AppError::InvalidArgument(format!("tab is not untitled: {tab_id}")));
    }
    let existing_id = tabs
        .iter()
        .find(|existing| matches!(&existing.kind, TabKind::File { path: existing_path } if existing_path == &path))
        .map(|existing| existing.id.clone());

    if let Some(existing_id) = existing_id {
        extract_tab(tree_root_mut(layout, tree), tab_id);
        normalize(tree_root_mut(layout, tree));
        activate_tab(layout, &existing_id)?;
        return Ok(existing_id);
    }

    let tab = find_tab_mut(tree_root_mut(layout, tree), tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    tab.kind = TabKind::File { path };
    tab.title = title;
    tab.dirty = false;
    tab.preview = false;
    layout.revision += 1;
    Ok(tab_id.clone())
}

pub fn close_tab(layout: &mut ProjectLayout, tab_id: &TabId) -> AppResult<ClosedTab> {
    let tree = locate_tree_of_tab(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let (pane_id, index) = find_tab(tree_root(layout, tree), tab_id)
        .map(|(pane_id, index)| (pane_id.clone(), index))
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let tab = extract_tab(tree_root_mut(layout, tree), tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    normalize(tree_root_mut(layout, tree));

    let closed = ClosedTab {
        tab,
        pane_id,
        index: index as u32,
    };
    push_closed(layout, closed.clone());
    layout.revision += 1;
    Ok(closed)
}

/// ClaudeDiff tabs resolve their pending IDE request the moment they close
/// (`reconcile_closed_tab`), so reviving one from the undo stack would
/// restore a zombie tab whose accept/reject can never succeed again.
fn push_closed(layout: &mut ProjectLayout, closed: ClosedTab) {
    if is_volatile(&closed.tab.kind) {
        return;
    }
    layout.closed_tabs.push(closed);
    if layout.closed_tabs.len() > CLOSED_TAB_STACK_LIMIT {
        layout.closed_tabs.remove(0);
    }
}

pub fn reopen_closed(layout: &mut ProjectLayout) -> Option<TabId> {
    let ClosedTab { tab, pane_id, index } = layout.closed_tabs.pop()?;
    let tab_id = tab.id.clone();

    let (target_tree, target_pane) = if let Some(tree) = locate_tree_of_pane(layout, &pane_id) {
        (tree, pane_id)
    } else if find_leaf(&layout.root, &layout.focused_pane).is_some() {
        (PaneTreeRef::Main, layout.focused_pane.clone())
    } else {
        let fallback = collect_leaves(&layout.root).first().map(|leaf| pane_id_of(leaf).clone())?;
        (PaneTreeRef::Main, fallback)
    };

    let root = tree_root_mut(layout, target_tree);
    if let Some(leaf) = find_leaf_mut(root, &target_pane) {
        insert_tab(leaf, tab, Some(index as usize));
    }
    set_tree_focused_pane(layout, target_tree, target_pane);
    layout.revision += 1;
    Some(tab_id)
}

/// What [`apply_tab_path_change`] did — the same two lists `TabPathChangeResult` carries to the
/// frontend, without the layout snapshot the command adds around them, plus whether the layout was
/// mutated at all.
///
/// `layout_changed` is deliberately *not* the same question as [`Self::is_empty`]. The closed-tab
/// stack follows a rename silently — it has no `TabPathMove` to report, because the frontend owns no
/// per-path state for a tab nobody has open — so a rename that matched only closed tabs produces two
/// empty lists and a changed layout. Collapsing the two questions is what made
/// `layout_apply_path_change` throw that rewrite away on its "nothing to report" early return,
/// leaving "Reopen Closed Tab" pointing at a path that no longer exists (contract §3 S8's own
/// requirement).
#[derive(Debug)]
pub struct TabPathChangeOutcome {
    pub moved: Vec<TabPathMove>,
    pub closed_paths: Vec<String>,
    pub layout_changed: bool,
}

impl TabPathChangeOutcome {
    /// Whether there is nothing for the *frontend* to act on — no per-path state to migrate and no
    /// closed path to release. Says nothing about whether the layout itself changed
    /// (`layout_changed`).
    pub fn is_empty(&self) -> bool {
        self.moved.is_empty() && self.closed_paths.is_empty()
    }
}

/// `path` rewritten so a `from` prefix becomes `to`, or `None` when `path` is neither `from` itself
/// nor anything under it.
///
/// Matched by path *components* (`Path::strip_prefix`), never by string prefix: renaming `src` must
/// not drag a sibling `src-old` along with it, which a `starts_with(from)` test would (the same
/// component-wise rule `tree::service`'s descendant pruning settled on for the identical reason).
fn retargeted_path(path: &str, from: &str, to: &str) -> Option<String> {
    let relative = Path::new(path).strip_prefix(Path::new(from)).ok()?;
    if relative.as_os_str().is_empty() {
        return Some(to.to_string());
    }
    Some(Path::new(to).join(relative).to_string_lossy().into_owned())
}

/// Whether `path` is `ancestor` itself or lives under it — the same component-wise rule
/// [`retargeted_path`] matches with.
fn is_same_or_under(path: &str, ancestor: &str) -> bool {
    Path::new(path).strip_prefix(Path::new(ancestor)).is_ok()
}

/// The tab title a file tab carries for `path` — its file name, matching what every frontend
/// producer of a `TabKind::File` tab passes as `title` (`fileNameOf(path)`). Falls back to the whole
/// path for a path with no final component, which a real project file never has.
fn file_tab_title(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn visit_tabs_mut<F: FnMut(&mut Tab)>(node: &mut PaneNode, visit: &mut F) {
    match node {
        PaneNode::Leaf { tabs, .. } => {
            for tab in tabs.iter_mut() {
                visit(tab);
            }
        }
        PaneNode::Split { children, .. } => {
            for child in children.iter_mut() {
                visit_tabs_mut(child, visit);
            }
        }
    }
}

/// Every *open* tab of every tree the project owns — the main tree plus each auxiliary window's —
/// as `&mut`. The mutable counterpart of [`all_roots`] + [`collect_leaves`]; the closed-tab stack is
/// deliberately not part of it, so callers decide separately whether the undo stack is in scope.
fn visit_open_tabs_mut<F: FnMut(&mut Tab)>(layout: &mut ProjectLayout, visit: &mut F) {
    visit_tabs_mut(&mut layout.root, visit);
    for window in layout.auxiliary_windows.iter_mut() {
        visit_tabs_mut(&mut window.root, visit);
    }
}

/// Repoints every open `TabKind::File` tab under `from` at `to` (title included), and does the same
/// silently for the closed-tab stack so "Reopen Closed Tab" doesn't resurrect a path that no longer
/// exists. Returns one [`TabPathMove`] per distinct moved path — a file open in two panes reports
/// once, with `dirty` true if *either* tab held unsaved edits.
fn retarget_file_tabs(layout: &mut ProjectLayout, from: &str, to: &str) -> TabPathChangeOutcome {
    let mut moved: Vec<TabPathMove> = Vec::new();

    visit_open_tabs_mut(layout, &mut |tab| {
        let TabKind::File { path } = &tab.kind else { return };
        let Some(next) = retargeted_path(path, from, to) else { return };

        let previous = path.clone();
        tab.kind = TabKind::File { path: next.clone() };
        tab.title = file_tab_title(&next);

        match moved.iter_mut().find(|entry| entry.from == previous) {
            Some(entry) => entry.dirty = entry.dirty || tab.dirty,
            None => moved.push(TabPathMove {
                from: previous,
                to: next,
                dirty: tab.dirty,
            }),
        }
    });

    let mut closed_tabs_retargeted = false;
    for closed in layout.closed_tabs.iter_mut() {
        let TabKind::File { path } = &closed.tab.kind else { continue };
        let Some(next) = retargeted_path(path, from, to) else { continue };
        closed.tab.kind = TabKind::File { path: next.clone() };
        closed.tab.title = file_tab_title(&next);
        closed_tabs_retargeted = true;
    }

    if !moved.is_empty() {
        layout.revision += 1;
    }
    let layout_changed = !moved.is_empty() || closed_tabs_retargeted;
    TabPathChangeOutcome {
        moved,
        closed_paths: Vec::new(),
        layout_changed,
    }
}

/// Closes every open `TabKind::File` tab whose path is `path` or lives under it, returning each
/// distinct closed path so the frontend can release that path's own per-path state. Each close goes
/// through [`close_tab`], so the closed-tab stack, active-tab handoff and pane normalization behave
/// exactly as if the user had closed those tabs by hand.
fn close_file_tabs_under(layout: &mut ProjectLayout, path: &str) -> TabPathChangeOutcome {
    let targets: Vec<(TabId, String)> = all_roots(layout)
        .flat_map(collect_leaves)
        .filter_map(|leaf| match leaf {
            PaneNode::Leaf { tabs, .. } => Some(tabs),
            PaneNode::Split { .. } => None,
        })
        .flatten()
        .filter_map(|tab| match &tab.kind {
            TabKind::File { path: tab_path } if is_same_or_under(tab_path, path) => Some((tab.id.clone(), tab_path.clone())),
            _ => None,
        })
        .collect();

    let mut closed_paths: Vec<String> = Vec::new();
    for (tab_id, tab_path) in targets {
        if close_tab(layout, &tab_id).is_err() {
            continue;
        }
        if !closed_paths.contains(&tab_path) {
            closed_paths.push(tab_path);
        }
    }

    let layout_changed = !closed_paths.is_empty();
    TabPathChangeOutcome {
        moved: Vec::new(),
        closed_paths,
        layout_changed,
    }
}

/// Makes the tab bar follow a path change the file domain already performed on disk — the layout
/// half of audit §4-B A3 ("rename/delete 후 열린 탭 미추종"). Only `TabKind::File` tabs take part:
/// their identity *is* the path (their title is that path's file name at every producer), whereas
/// `Diff`/`ClaudeDiff` tabs are derived comparison views whose labels and paired revisions a rename
/// does not transfer. Renaming retargets, deleting closes — see the contract §3 S8 for both
/// decisions.
pub fn apply_tab_path_change(layout: &mut ProjectLayout, change: &TabPathChange) -> TabPathChangeOutcome {
    match change {
        TabPathChange::Renamed { from, to } => retarget_file_tabs(layout, from, to),
        TabPathChange::Deleted { path } => close_file_tabs_under(layout, path),
    }
}

pub fn activate_tab(layout: &mut ProjectLayout, tab_id: &TabId) -> AppResult<()> {
    let tree = locate_tree_of_tab(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let pane_id = find_tab(tree_root(layout, tree), tab_id)
        .map(|(pane_id, _)| pane_id.clone())
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let leaf =
        find_leaf_mut(tree_root_mut(layout, tree), &pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    if let PaneNode::Leaf { active, .. } = leaf {
        *active = Some(tab_id.clone());
    }
    set_tree_focused_pane(layout, tree, pane_id);
    layout.revision += 1;
    Ok(())
}

pub fn pin_tab(layout: &mut ProjectLayout, tab_id: &TabId, pinned: bool) -> AppResult<()> {
    let tree = locate_tree_of_tab(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let pane_id = find_tab(tree_root(layout, tree), tab_id)
        .map(|(pane_id, _)| pane_id.clone())
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let leaf =
        find_leaf_mut(tree_root_mut(layout, tree), &pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    if let PaneNode::Leaf { tabs, .. } = leaf {
        if let Some(tab) = tabs.iter_mut().find(|tab| &tab.id == tab_id) {
            tab.pinned = pinned;
        }
        tabs.sort_by_key(|tab| !tab.pinned);
    }
    layout.revision += 1;
    Ok(())
}

pub fn set_preview(layout: &mut ProjectLayout, tab_id: &TabId, preview: bool) -> AppResult<()> {
    let tab = find_tab_mut_in_layout(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    tab.preview = preview;
    layout.revision += 1;
    Ok(())
}

/// Moves `tab_id` to `index` within whichever leaf `target_pane` resolves to — same-tree
/// (reordering, or a drop into a different pane of the same window) or, since the source and
/// target pane are each located independently by id, across trees too (the mechanism
/// `move_tab_to_main`/`move_tab_to_existing_window`/`move_tab_to_new_window` below reuse). When
/// source and target share a tree, extraction and insertion happen back-to-back with a single
/// `normalize` at the end — matching the pre-multi-window behavior exactly, including the
/// same-leaf-reorder edge case (extracting a leaf's only tab must not let it get pruned as "empty"
/// before the very next line reinserts it). Only when the trees differ is the source tree
/// normalized separately, right after extraction, since at that point it can no longer affect
/// `target_pane`'s lookup.
pub fn move_tab(layout: &mut ProjectLayout, tab_id: &TabId, target_pane: &PaneId, index: usize) -> AppResult<()> {
    let target_tree =
        locate_tree_of_pane(layout, target_pane).ok_or_else(|| AppError::NotFound(format!("pane not found: {target_pane}")))?;
    let source_tree = locate_tree_of_tab(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;

    let tab =
        extract_tab(tree_root_mut(layout, source_tree), tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    if source_tree != target_tree {
        normalize(tree_root_mut(layout, source_tree));
    }

    let target_root = tree_root_mut(layout, target_tree);
    let leaf = find_leaf_mut(target_root, target_pane).ok_or_else(|| AppError::Internal("pane vanished during move".to_string()))?;
    insert_tab(leaf, tab, Some(index));

    set_tree_focused_pane(layout, target_tree, target_pane.clone());
    normalize(tree_root_mut(layout, target_tree));
    ensure_focused_pane_valid(layout);
    layout.revision += 1;
    Ok(())
}

/// Maps a directional drop edge onto the split axis it creates. `Center` has no axis — it means
/// "drop into the target pane itself", which every caller resolves before a new leaf is ever built.
fn split_dir_of_edge(edge: DropEdge) -> Option<SplitDir> {
    match edge {
        DropEdge::Left | DropEdge::Right => Some(SplitDir::Horizontal),
        DropEdge::Top | DropEdge::Bottom => Some(SplitDir::Vertical),
        DropEdge::Center => None,
    }
}

/// Wraps one tab in a fresh `Leaf` that already has it active — the node shape both [`split`]
/// (which moves an existing tab out into its own pane) and [`open_tab_in_split`] (which creates a
/// brand-new tab already in its own pane) hand to [`insert_new_leaf`].
fn leaf_with_tab(tab: Tab) -> PaneNode {
    let active = tab.id.clone();
    PaneNode::Leaf {
        id: PaneId::new(),
        tabs: vec![tab],
        active: Some(active),
    }
}

/// Grafts an already-built leaf next to `target_pane` along `edge` and focuses it — the half of a
/// split that does not care where the tab inside that leaf came from. [`split`] arrives here with a
/// tab extracted out of the tree, [`open_tab_in_split`] with a tab that did not exist a moment ago.
/// Sharing this is what lets "open a new terminal to the right" be a single mutation (one
/// `layout:changed`, zero remounts of the pane that was already there) instead of the
/// `open_tab` + `split` pair, which activated the new tab in the source pane first and so
/// unmounted the live terminal and spawned its shell a second time on remount — see
/// `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §F.2.
fn insert_new_leaf(layout: &mut ProjectLayout, target_pane: &PaneId, edge: DropEdge, new_leaf: PaneNode) -> AppResult<()> {
    let target_tree =
        locate_tree_of_pane(layout, target_pane).ok_or_else(|| AppError::NotFound(format!("pane not found: {target_pane}")))?;
    let dir = split_dir_of_edge(edge).ok_or_else(|| AppError::Internal("center edge has no split axis".to_string()))?;
    let new_leaf_id = pane_id_of(&new_leaf).clone();

    let target_root = tree_root_mut(layout, target_tree);
    let is_root_target = matches!(target_root, PaneNode::Leaf { id, .. } if id == target_pane);
    if is_root_target {
        let placeholder = PaneNode::Leaf {
            id: PaneId::new(),
            tabs: Vec::new(),
            active: None,
        };
        let existing = std::mem::replace(target_root, placeholder);
        *tree_root_mut(layout, target_tree) = wrap_leaf_in_split(existing, new_leaf, dir, edge);
    } else {
        let mut pending = Some(new_leaf);
        insert_split_at(tree_root_mut(layout, target_tree), target_pane, dir, edge, &mut pending);
        if pending.is_some() {
            return Err(AppError::NotFound(format!("pane not found: {target_pane}")));
        }
    }

    set_tree_focused_pane(layout, target_tree, new_leaf_id);
    normalize(tree_root_mut(layout, target_tree));
    ensure_focused_pane_valid(layout);
    layout.revision += 1;
    Ok(())
}

pub fn split(layout: &mut ProjectLayout, target_pane: &PaneId, edge: DropEdge, tab_id: &TabId) -> AppResult<()> {
    let target_tree =
        locate_tree_of_pane(layout, target_pane).ok_or_else(|| AppError::NotFound(format!("pane not found: {target_pane}")))?;

    if edge == DropEdge::Center {
        return move_tab(layout, tab_id, target_pane, APPEND_AT_END);
    }

    let source_tree = locate_tree_of_tab(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let tab =
        extract_tab(tree_root_mut(layout, source_tree), tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    if source_tree != target_tree {
        normalize(tree_root_mut(layout, source_tree));
    }

    insert_new_leaf(layout, target_pane, edge, leaf_with_tab(tab))
}

/// Creates `tab` in a brand-new pane beside `target_pane` — the "open a new terminal to the right"
/// primitive, as opposed to [`split`]'s "move this existing tab to the right". `Center` is rejected
/// instead of being folded into `move_tab` the way [`split`] folds it: there is no existing tab to
/// move, and "open a new tab inside this very pane" is already [`open_tab`]'s job. Unlike
/// [`open_tab`] there is no kind-equality dedupe — the caller asked for a second pane, and two
/// terminal tabs that have not been given their session id yet compare equal.
pub fn open_tab_in_split(layout: &mut ProjectLayout, target_pane: &PaneId, edge: DropEdge, tab: Tab) -> AppResult<TabId> {
    if edge == DropEdge::Center {
        return Err(AppError::InvalidArgument("center edge cannot open a tab in a split".to_string()));
    }

    let tab_id = tab.id.clone();
    insert_new_leaf(layout, target_pane, edge, leaf_with_tab(tab))?;
    Ok(tab_id)
}

pub fn resize(layout: &mut ProjectLayout, pane_id: &PaneId, sizes: Vec<f32>) -> AppResult<()> {
    let tree = locate_tree_of_pane(layout, pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    let node =
        find_split_mut(tree_root_mut(layout, tree), pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    let PaneNode::Split {
        children, sizes: existing, ..
    } = node
    else {
        return Err(AppError::Internal("expected split pane".to_string()));
    };
    if sizes.len() != children.len() {
        return Err(AppError::InvalidArgument(format!(
            "sizes length {} does not match children length {}",
            sizes.len(),
            children.len()
        )));
    }
    *existing = sizes;
    layout.revision += 1;
    Ok(())
}

pub fn focus_pane(layout: &mut ProjectLayout, pane_id: &PaneId) -> AppResult<()> {
    let tree = locate_tree_of_pane(layout, pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    set_tree_focused_pane(layout, tree, pane_id.clone());
    layout.revision += 1;
    Ok(())
}

pub fn set_view_state(layout: &mut ProjectLayout, tab_id: &TabId, view_state: Option<String>) -> AppResult<()> {
    let tab = find_tab_mut_in_layout(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    tab.view_state = view_state;
    layout.revision += 1;
    Ok(())
}

pub fn set_dirty(layout: &mut ProjectLayout, tab_id: &TabId, dirty: bool) -> AppResult<()> {
    let tab = find_tab_mut_in_layout(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    tab.dirty = dirty;
    layout.revision += 1;
    Ok(())
}

pub fn set_terminal_session(layout: &mut ProjectLayout, tab_id: &TabId, session_id: String) -> AppResult<()> {
    let tab = find_tab_mut_in_layout(layout, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let TabKind::Terminal { session_id: existing, .. } = &mut tab.kind else {
        return Err(AppError::InvalidArgument(format!("tab is not a terminal: {tab_id}")));
    };
    *existing = session_id;
    layout.revision += 1;
    Ok(())
}

/// Lowest unused auxiliary-window `slot` for this project, starting at 1 — a project-scoped
/// semantic key distinct from the global OS-level `editor-<n>` window label
/// (`domain::window::service::next_auxiliary_label`), mirroring that function's lowest-unused
/// allocation strategy so closed/returned slots get reused instead of counting up forever.
pub fn next_window_slot(layout: &ProjectLayout) -> u32 {
    let used: HashSet<u32> = layout.auxiliary_windows.iter().map(|window| window.slot).collect();
    let mut candidate = FIRST_WINDOW_SLOT;
    while used.contains(&candidate) {
        candidate += 1;
    }
    candidate
}

/// `TabWindowTarget::Main` — moves `tab_id` to the end of the main tree's currently focused pane
/// (or the first available leaf, via [`ensure_focused_pane_valid`], if that pane no longer exists).
pub fn move_tab_to_main(layout: &mut ProjectLayout, tab_id: &TabId) -> AppResult<()> {
    ensure_focused_pane_valid(layout);
    let target = layout.focused_pane.clone();
    move_tab(layout, tab_id, &target, APPEND_AT_END)
}

/// `TabWindowTarget::Existing { slot }` — moves `tab_id` into an auxiliary window already recorded
/// in `auxiliary_windows`, targeting that window's own focused pane.
pub fn move_tab_to_existing_window(layout: &mut ProjectLayout, tab_id: &TabId, slot: u32) -> AppResult<()> {
    let index = layout
        .auxiliary_windows
        .iter()
        .position(|window| window.slot == slot)
        .ok_or_else(|| AppError::NotFound(format!("auxiliary window not found: slot {slot}")))?;
    let tree = PaneTreeRef::Auxiliary(index);
    let target = {
        let root = tree_root(layout, tree);
        let focused = &layout.auxiliary_windows[index].focused_pane;
        if find_leaf(root, focused).is_some() {
            focused.clone()
        } else {
            collect_leaves(root)
                .first()
                .map(|leaf| pane_id_of(leaf).clone())
                .ok_or_else(|| AppError::Internal(format!("auxiliary window has no panes: slot {slot}")))?
        }
    };
    move_tab(layout, tab_id, &target, APPEND_AT_END)
}

/// `TabWindowTarget::NewAuxiliary` — records a brand-new `slot` (already reserved by the caller via
/// [`next_window_slot`] before it asked `domain::window` to open the OS window, so the two stay in
/// sync even if this call fails) as a single fresh leaf, then moves `tab_id` into it. On failure
/// (`tab_id` vanished between the caller's lookup and this call — not reachable through the
/// `layout_move_tab_to_window` command, which locates the tab first, but kept defensive) the
/// just-inserted empty window entry is rolled back so a failed move never leaves a phantom
/// auxiliary window in the layout.
pub fn move_tab_to_new_window(layout: &mut ProjectLayout, tab_id: &TabId, slot: u32) -> AppResult<()> {
    let leaf_id = PaneId::new();
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot,
        root: PaneNode::Leaf {
            id: leaf_id.clone(),
            tabs: Vec::new(),
            active: None,
        },
        focused_pane: leaf_id.clone(),
    });

    if let Err(error) = move_tab(layout, tab_id, &leaf_id, 0) {
        layout.auxiliary_windows.retain(|window| window.slot != slot);
        return Err(error);
    }
    Ok(())
}

/// Merges an auxiliary window's tabs back into the main tree's tail before dropping that window's
/// layout entry — TAIDE's 0-loss philosophy on aux-window close (contract §3.1), unlike VS Code
/// discarding an auxiliary window's content when it closes. Tabs are appended in the window's
/// leaf-traversal order without stealing the main tree's current focus (only an empty target leaf's
/// `active` gets set, so a user actively working in main isn't yanked away when a background
/// window closes). Returns `false` (a no-op) if `window_slot` doesn't name a currently-recorded
/// auxiliary window — already-processed idempotent closes (`CloseRequested` then `Destroyed` for
/// the same window) hit this path harmlessly.
pub fn return_auxiliary_window_tabs(layout: &mut ProjectLayout, window_slot: u32) -> bool {
    let Some(position) = layout.auxiliary_windows.iter().position(|window| window.slot == window_slot) else {
        return false;
    };
    let removed = layout.auxiliary_windows.remove(position);
    let tabs: Vec<Tab> = collect_leaves(&removed.root)
        .into_iter()
        .filter_map(|leaf| match leaf {
            PaneNode::Leaf { tabs, .. } => Some(tabs.clone()),
            PaneNode::Split { .. } => None,
        })
        .flatten()
        .collect();

    if tabs.is_empty() {
        layout.revision += 1;
        return true;
    }

    ensure_focused_pane_valid(layout);
    let target = layout.focused_pane.clone();
    if let Some(PaneNode::Leaf {
        tabs: leaf_tabs, active, ..
    }) = find_leaf_mut(&mut layout.root, &target)
    {
        let had_active = active.is_some();
        for tab in tabs {
            let id = tab.id.clone();
            leaf_tabs.push(tab);
            if !had_active {
                *active = Some(id);
            }
        }
    }
    layout.revision += 1;
    true
}

pub fn apply_shell_view_patch(layout: &mut ProjectLayout, patch: &ShellViewPatch) {
    if let Some(zen) = patch.zen {
        layout.shell_view.zen = zen;
    }
    if let Some(sidebar_collapsed) = patch.sidebar_collapsed {
        layout.shell_view.sidebar_collapsed = sidebar_collapsed;
    }
    layout.revision += 1;
}

/// Tab kinds excluded from persisted storage. ClaudeDiff tabs are tied to a
/// pending IDE request that only exists for the current session, so nothing
/// else qualifies — Untitled tabs are hot-exit mirrored by `TabId` and are
/// safe to persist and restore.
fn is_volatile(kind: &TabKind) -> bool {
    matches!(kind, TabKind::ClaudeDiff { .. })
}

fn strip_volatile_node(node: &mut PaneNode) {
    match node {
        PaneNode::Leaf { tabs, active, .. } => {
            let removed_index = active
                .as_ref()
                .and_then(|active_id| tabs.iter().position(|tab| &tab.id == active_id));
            let active_was_volatile = removed_index.map(|index| is_volatile(&tabs[index].kind)).unwrap_or(false);
            tabs.retain(|tab| !is_volatile(&tab.kind));
            if active_was_volatile {
                *active = if tabs.is_empty() {
                    None
                } else {
                    Some(tabs[removed_index.unwrap_or(0).min(tabs.len() - 1)].id.clone())
                };
            }
        }
        PaneNode::Split { children, .. } => children.iter_mut().for_each(strip_volatile_node),
    }
}

/// `normalize` 로 패널이 사라지면 `focused_pane` 이 존재하지 않는 pane 을 가리킬 수 있다.
/// 그 상태로 저장/복원되면 이후 탭 열기가 계속 실패하므로 첫 리프로 보정한다. 보조 창 각각의
/// `focused_pane` 도 동일하게 보정한다.
pub fn ensure_focused_pane_valid(layout: &mut ProjectLayout) {
    if find_leaf(&layout.root, &layout.focused_pane).is_none() {
        if let Some(pane_id) = collect_leaves(&layout.root).first().map(|leaf| pane_id_of(leaf).clone()) {
            layout.focused_pane = pane_id;
        }
    }
    for window in layout.auxiliary_windows.iter_mut() {
        if find_leaf(&window.root, &window.focused_pane).is_none() {
            if let Some(pane_id) = collect_leaves(&window.root).first().map(|leaf| pane_id_of(leaf).clone()) {
                window.focused_pane = pane_id;
            }
        }
    }
}

/// Strips volatile tabs from every tree (main and each auxiliary window), then drops any auxiliary
/// window whose tree is left fully empty — an aux window that would restore with zero tabs (every
/// tab in it was volatile) isn't worth recreating as an OS window on next launch.
pub fn strip_volatile_tabs(layout: &ProjectLayout) -> ProjectLayout {
    let mut persisted = layout.clone();
    strip_volatile_node(&mut persisted.root);
    normalize(&mut persisted.root);
    for window in persisted.auxiliary_windows.iter_mut() {
        strip_volatile_node(&mut window.root);
        normalize(&mut window.root);
    }
    persisted.auxiliary_windows.retain(|window| !is_layout_tree_empty(&window.root));
    ensure_focused_pane_valid(&mut persisted);
    persisted.closed_tabs.retain(|closed| !is_volatile(&closed.tab.kind));
    persisted
}

pub fn save_layout(paths: &AppPaths, project_id: &ProjectId, layout: &ProjectLayout) -> AppResult<()> {
    let persisted = strip_volatile_tabs(layout);
    persist::write_json(&paths.layout_file(project_id), &persisted)
}

pub(crate) const LAYOUT_FLUSH_INTERVAL_MS: u64 = 2_000;

pub(crate) fn flush_dirty_layouts(state: &AppState) {
    let dirty: Vec<_> = state.dirty_layouts.write().drain().collect();
    if dirty.is_empty() {
        return;
    }

    let snapshots: Vec<_> = {
        let layouts = state.layouts.read();
        dirty
            .into_iter()
            .filter_map(|project_id| match layouts.get(&project_id) {
                Some(layout) => Some((project_id, layout.clone())),
                None => {
                    log::warn!("dirty_layouts 에 등록된 프로젝트의 레이아웃을 찾을 수 없어 저장을 건너뜁니다: {project_id}");
                    None
                }
            })
            .collect()
    };

    for (project_id, layout) in snapshots {
        if let Err(error) = save_layout(&state.paths, &project_id, &layout) {
            log::warn!("레이아웃 저장 실패 ({project_id}): {error}");
        }
    }
}

/// v1 → v2: adds the auxiliary-window axis and per-project shell chrome state. Both are purely
/// additive — a v1 payload already deserializes cleanly into the current `ProjectLayout` shape via
/// `#[serde(default)]` on every new field — so migrating is just stamping the current version onto
/// the same tabs/tree v1 already had, not a structural rewrite. Existing tabs are preserved
/// untouched. See `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.2.
fn migrate_layout(mut layout: ProjectLayout) -> ProjectLayout {
    if layout.version < LAYOUT_SCHEMA_VERSION {
        layout.version = LAYOUT_SCHEMA_VERSION;
    }
    layout
}

/// `AppFile` tabs (`settings.json`/prompt overrides) have no hot-exit mirror (contract §3.3: 1차
/// 제외) — their `dirty` flag is the only persisted signal of an in-progress edit, and it survives
/// a save/restore round trip with nothing behind it to actually restore. Restoring a layout with
/// `dirty: true` on one of these tabs would therefore show the clean synced-from-disk content
/// (there is nothing else to show) marked as if it had unsaved changes — a permanent, un-clearable
/// ghost, since the only way to flip it back is a save that would just re-persist what's already on
/// disk. Called once at load time (not on every mutation) so a *live* session's dirty flag — set by
/// `layout_set_dirty` while the app is running, when the frontend's local `draftRef` genuinely does
/// hold unsaved text — is left alone.
fn clear_app_file_dirty(node: &mut PaneNode) {
    match node {
        PaneNode::Leaf { tabs, .. } => {
            for tab in tabs.iter_mut() {
                if matches!(tab.kind, TabKind::AppFile { .. }) {
                    tab.dirty = false;
                }
            }
        }
        PaneNode::Split { children, .. } => children.iter_mut().for_each(clear_app_file_dirty),
    }
}

/// Loads a project's layout, migrating an older-but-parseable schema version forward
/// ([`migrate_layout`]) instead of discarding it — only a genuinely unparseable file (corrupt JSON)
/// or one from a *newer* schema version than this build understands falls back to
/// [`default_layout`], since downgrading a future schema safely isn't possible.
pub fn load_layout(paths: &AppPaths, project_id: &ProjectId) -> ProjectLayout {
    match persist::read_json::<ProjectLayout>(&paths.layout_file(project_id)) {
        Ok(Some(layout)) if layout.version <= LAYOUT_SCHEMA_VERSION => {
            let mut layout = migrate_layout(layout);
            ensure_focused_pane_valid(&mut layout);
            clear_app_file_dirty(&mut layout.root);
            for window in layout.auxiliary_windows.iter_mut() {
                clear_app_file_dirty(&mut window.root);
            }
            layout
        }
        _ => default_layout(),
    }
}

/// Resolves which project owns `tab_id`, searching every pane tree that project's layout owns —
/// the main tree *and* every auxiliary window's tree ([`all_roots`]) — not just `layout.root`.
/// A tab moved into an auxiliary window (`layout_move_tab_to_window`) no longer lives in the main
/// tree at all, so scoping this to `layout.root` would make every layout command targeting it
/// (close/activate/move/pin/set_dirty/move back to main, ...) fail with a spurious NotFound.
pub fn locate_project_with_tab(layouts: &HashMap<ProjectId, ProjectLayout>, tab_id: &TabId) -> AppResult<ProjectId> {
    layouts
        .iter()
        .find_map(|(project_id, layout)| {
            all_roots(layout)
                .any(|root| find_tab(root, tab_id).is_some())
                .then(|| project_id.clone())
        })
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))
}

/// Same rationale as [`locate_project_with_tab`], for pane ids — a pane inside an auxiliary window's
/// tree must resolve too (split/resize/focus_pane target panes there once a tab has moved there).
pub fn locate_project_with_pane(layouts: &HashMap<ProjectId, ProjectLayout>, pane_id: &PaneId) -> AppResult<ProjectId> {
    layouts
        .iter()
        .find_map(|(project_id, layout)| {
            all_roots(layout)
                .any(|root| contains_pane(root, pane_id))
                .then(|| project_id.clone())
        })
        .ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))
}

pub fn get_layout_mut<'a>(layouts: &'a mut HashMap<ProjectId, ProjectLayout>, project_id: &ProjectId) -> AppResult<&'a mut ProjectLayout> {
    layouts
        .get_mut(project_id)
        .ok_or_else(|| AppError::NotFound(format!("layout not found: {project_id}")))
}

pub fn finish_mutation(app: &AppHandle, state: &AppState, project_id: &ProjectId, layout: &mut ProjectLayout) -> ProjectLayout {
    let snapshot = layout.clone();
    state.dirty_layouts.write().insert(project_id.clone());

    let _ = LayoutChanged {
        project_id: project_id.clone(),
        revision: snapshot.revision,
    }
    .emit(app);

    snapshot
}

/// Opens a tab and completes the post-processing (dirty marking + layout-changed event emission).
/// Shared so the Tauri command (`layout_open_tab`) and the IDE domain's `openFile` tool handler
/// run the same path — the service-level entry point that keeps the IDE from reusing the command
/// surface as a second entry point (R6#3). Takes the mutation guard itself to prevent layout
/// read-clone-write races.
pub async fn open_tab_and_finish(
    app: &AppHandle,
    state: &AppState,
    project_id: ProjectId,
    kind: TabKind,
    title: String,
    target: Option<PaneId>,
    preview: bool,
) -> AppResult<ProjectLayout> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let preview = preview && state.settings.read().enable_preview_tabs;
    let pane_id = target.unwrap_or_else(|| layout.focused_pane.clone());
    let tab = Tab {
        id: TabId::new(),
        kind,
        title,
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    open_tab(layout, &pane_id, tab, preview)?;

    let updated = finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;
    Ok(updated)
}

/// 탭을 닫고 후처리(레이아웃 갱신 이벤트 발신 + IDE 도메인의 pending diff 해소 + 터미널 탭이면
/// pty 세션 회수)까지 마친다. Tauri 커맨드(`layout_close_tab`)와 IDE 도메인의
/// `close_tab`/`closeAllDiffTabs` 도구 핸들러가 동일한 경로를 타도록 공유한다 — ClaudeDiff 탭이
/// 어떤 경로로 닫히든 pending 요청이 반드시 해소되고, 터미널 탭이 어떤 경로로 닫히든 그 pty 가
/// 반드시 죽는다. 레이아웃 read-clone-write 경합을 막기 위해 뮤테이션 가드는 이 함수가 직접 잡는다.
///
/// The pty reap is the "탭 닫기 시 pty_kill" half of the T0 #21 fix (`docs/acknowledge/
/// 2026-08-18-audit-t0-fix-contract.md` §2.3) — `project_close`'s `TerminalStore::kill_project`
/// only reaps sessions when the *project* closes, and before this fix nothing called `pty_kill` when
/// an individual terminal tab closed; the session lingered, attached to nothing, until the owning
/// project or the whole app closed.
pub async fn close_tab_and_finish(app: &AppHandle, state: &AppState, tab_id: &TabId) -> AppResult<(ProjectId, ClosedTab, ProjectLayout)> {
    let _guard = state.begin_mutation().await;
    let mut layouts = state.layouts.read().clone();
    let project_id = locate_project_with_tab(&layouts, tab_id)?;
    let layout = get_layout_mut(&mut layouts, &project_id)?;

    let closed = close_tab(layout, tab_id)?;

    let updated = finish_mutation(app, state, &project_id, layout);
    *state.layouts.write() = layouts;

    crate::domain::ide::store::reconcile_closed_tab(app, &closed.tab);
    if let TabKind::Terminal { session_id, .. } = &closed.tab.kind {
        app.state::<crate::domain::terminal::commands::TerminalStore>()
            .kill_session(session_id);
    }

    Ok((project_id, closed, updated))
}

#[cfg(test)]
mod tests {
    use super::super::types::TabKind;
    use super::*;
    use crate::domain::search::types::SearchQuery;

    fn 검색_쿼리(text: &str) -> SearchQuery {
        SearchQuery {
            text: text.to_string(),
            case_sensitive: false,
            whole_word: false,
            regex: false,
            include_glob: None,
            exclude_glob: None,
            context_lines: 0,
            respect_gitignore: true,
        }
    }

    fn 검색_에디터_탭(text: &str) -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::SearchEditor {
                query: 검색_쿼리(text)
            },
            title: format!("Search: {text}"),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }
    }

    fn 파일_탭(path: &str) -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::File { path: path.to_string() },
            title: path.to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }
    }

    fn 클로드_diff_탭(path: &str) -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::ClaudeDiff {
                request_id: "req-1".to_string(),
                path: path.to_string(),
            },
            title: path.to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }
    }

    fn 언타이틀드_탭(index: u32) -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::Untitled { index },
            title: format!("Untitled-{index}"),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }
    }

    fn 리프(tabs: Vec<Tab>) -> PaneNode {
        let active = tabs.first().map(|tab| tab.id.clone());
        PaneNode::Leaf {
            id: PaneId::new(),
            tabs,
            active,
        }
    }

    #[test]
    fn 기본_레이아웃은_웰컴과_터미널_탭을_가진다() {
        let layout = default_layout();
        let PaneNode::Leaf { tabs, active, .. } = &layout.root else {
            panic!("root must be a leaf");
        };

        assert_eq!(tabs.len(), 2);
        assert!(matches!(tabs[0].kind, TabKind::Welcome));
        assert!(matches!(tabs[1].kind, TabKind::Terminal { .. }));
        assert_eq!(active, &Some(tabs[0].id.clone()));
    }

    #[test]
    fn 빈_리프는_정규화에서_제거된다() {
        let empty_leaf = 리프(vec![]);
        let full_leaf = 리프(vec![파일_탭("a.rs")]);
        let mut root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![empty_leaf, full_leaf],
            sizes: vec![0.5, 0.5],
        };

        normalize(&mut root);

        assert!(matches!(root, PaneNode::Leaf { .. }));
    }

    #[test]
    fn 마지막_남은_빈_리프는_유지된다() {
        let mut root = 리프(vec![]);
        normalize(&mut root);
        assert!(matches!(root, PaneNode::Leaf { .. }));
    }

    #[test]
    fn 같은_방향_스플릿에서는_형제로_삽입된다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();
        let tab_to_split = 파일_탭("a.rs");
        let tab_id = tab_to_split.id.clone();
        open_tab(&mut layout, &leaf_id, tab_to_split, false).expect("open");

        split(&mut layout, &leaf_id, DropEdge::Right, &tab_id).expect("first split");

        let PaneNode::Split { dir, children, .. } = &layout.root else {
            panic!("expected split")
        };
        assert_eq!(*dir, SplitDir::Horizontal);
        assert_eq!(children.len(), 2);

        let second_tab = 파일_탭("b.rs");
        let second_tab_id = second_tab.id.clone();
        open_tab(&mut layout, &leaf_id, second_tab, false).expect("open second");
        split(&mut layout, &leaf_id, DropEdge::Right, &second_tab_id).expect("second split");

        let PaneNode::Split { dir, children, sizes, .. } = &layout.root else {
            panic!("expected split")
        };
        assert_eq!(*dir, SplitDir::Horizontal);
        assert_eq!(children.len(), 3);
        assert_eq!(sizes.len(), 3);
    }

    #[test]
    fn 다른_방향으로_스플릿하면_리프를_감싼다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();
        let tab = 파일_탭("a.rs");
        let tab_id = tab.id.clone();
        open_tab(&mut layout, &leaf_id, tab, false).expect("open");

        split(&mut layout, &leaf_id, DropEdge::Bottom, &tab_id).expect("split");

        let PaneNode::Split { dir, children, .. } = &layout.root else {
            panic!("expected split")
        };
        assert_eq!(*dir, SplitDir::Vertical);
        assert_eq!(children.len(), 2);
    }

    fn 루트_리프_id(layout: &ProjectLayout) -> PaneId {
        let PaneNode::Leaf { id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        id.clone()
    }

    fn 터미널_탭() -> Tab {
        Tab {
            id: TabId::new(),
            kind: TabKind::Terminal {
                session_id: String::new(),
                cwd: None,
            },
            title: "Terminal".to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }
    }

    #[test]
    fn 새_탭_분할은_루트_리프를_감싸고_새_페인을_포커스한다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);
        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let 원래_탭_수 = tabs.len();

        let tab = 파일_탭("new.rs");
        let tab_id = tab.id.clone();
        let opened = open_tab_in_split(&mut layout, &leaf_id, DropEdge::Right, tab).expect("open in split");

        assert_eq!(opened, tab_id);
        let PaneNode::Split { dir, children, .. } = &layout.root else {
            panic!("expected split")
        };
        assert_eq!(*dir, SplitDir::Horizontal);
        assert_eq!(children.len(), 2);

        let PaneNode::Leaf { id, tabs, .. } = &children[0] else {
            panic!("expected existing leaf")
        };
        assert_eq!(id, &leaf_id);
        assert_eq!(tabs.len(), 원래_탭_수, "기존 페인의 탭은 옮겨지지 않는다");

        let PaneNode::Leaf {
            id: new_leaf_id,
            tabs: new_tabs,
            active,
        } = &children[1]
        else {
            panic!("expected new leaf")
        };
        assert_eq!(new_tabs.len(), 1);
        assert_eq!(new_tabs[0].id, tab_id);
        assert_eq!(active.as_ref(), Some(&tab_id));
        assert_eq!(&layout.focused_pane, new_leaf_id);
        assert_eq!(layout.revision, 1);
    }

    #[test]
    fn 새_탭_분할의_left_top_엣지는_새_페인을_기존_페인_앞에_둔다() {
        for (edge, expected_dir) in [(DropEdge::Left, SplitDir::Horizontal), (DropEdge::Top, SplitDir::Vertical)] {
            let mut layout = default_layout();
            let leaf_id = 루트_리프_id(&layout);
            let tab = 파일_탭("a.rs");
            let tab_id = tab.id.clone();

            open_tab_in_split(&mut layout, &leaf_id, edge, tab).expect("open in split");

            let PaneNode::Split { dir, children, .. } = &layout.root else {
                panic!("expected split")
            };
            assert_eq!(*dir, expected_dir, "{edge:?}");
            let PaneNode::Leaf { tabs, .. } = &children[0] else {
                panic!("expected new leaf first")
            };
            assert_eq!(tabs[0].id, tab_id, "{edge:?} 는 새 페인이 앞에 와야 한다");
            assert_eq!(pane_id_of(&children[1]), &leaf_id, "{edge:?} 는 기존 페인이 뒤에 와야 한다");
        }
    }

    #[test]
    fn 새_탭_분할의_right_bottom_엣지는_새_페인을_기존_페인_뒤에_둔다() {
        for (edge, expected_dir) in [(DropEdge::Right, SplitDir::Horizontal), (DropEdge::Bottom, SplitDir::Vertical)] {
            let mut layout = default_layout();
            let leaf_id = 루트_리프_id(&layout);

            open_tab_in_split(&mut layout, &leaf_id, edge, 파일_탭("a.rs")).expect("open in split");

            let PaneNode::Split { dir, children, .. } = &layout.root else {
                panic!("expected split")
            };
            assert_eq!(*dir, expected_dir, "{edge:?}");
            assert_eq!(pane_id_of(&children[0]), &leaf_id, "{edge:?} 는 기존 페인이 앞에 와야 한다");
        }
    }

    #[test]
    fn 새_탭_분할은_중첩_대상에서도_새_페인을_포커스하고_revision을_한_번만_올린다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);
        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Right, 파일_탭("a.rs")).expect("first split");
        let revision_before = layout.revision;

        let tab = 파일_탭("b.rs");
        let tab_id = tab.id.clone();
        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Bottom, tab).expect("nested split");

        assert_eq!(layout.revision, revision_before + 1);
        let PaneNode::Split { children, .. } = &layout.root else {
            panic!("expected split")
        };
        let PaneNode::Split { children: inner, .. } = &children[0] else {
            panic!("expected wrapped split")
        };
        let PaneNode::Leaf { id, tabs, active, .. } = &inner[1] else {
            panic!("expected new leaf")
        };
        assert_eq!(tabs[0].id, tab_id);
        assert_eq!(active.as_ref(), Some(&tab_id));
        assert_eq!(&layout.focused_pane, id, "새 페인이 포커스를 받아야 한다");
    }

    #[test]
    fn 새_탭_분할은_기존_페인의_활성_탭을_바꾸지_않는다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);
        let PaneNode::Leaf { active, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let active_before = active.clone();

        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Right, 파일_탭("a.rs")).expect("open in split");

        let PaneNode::Split { children, .. } = &layout.root else {
            panic!("expected split")
        };
        let PaneNode::Leaf { active, .. } = &children[0] else {
            panic!("expected existing leaf")
        };
        assert_eq!(active, &active_before, "기존 페인이 새 탭을 활성화하면 터미널이 언마운트된다");
    }

    #[test]
    fn 새_탭_분할은_같은_방향_부모에_형제로_삽입된다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);

        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Right, 파일_탭("a.rs")).expect("first split");
        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Right, 파일_탭("b.rs")).expect("second split");

        let PaneNode::Split { dir, children, sizes, .. } = &layout.root else {
            panic!("expected split")
        };
        assert_eq!(*dir, SplitDir::Horizontal);
        assert_eq!(children.len(), 3);
        assert_eq!(sizes.len(), 3);
    }

    #[test]
    fn 새_탭_분할은_다른_방향_부모에서는_대상_리프만_감싼다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);

        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Right, 파일_탭("a.rs")).expect("first split");
        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Bottom, 파일_탭("b.rs")).expect("second split");

        let PaneNode::Split { dir, children, .. } = &layout.root else {
            panic!("expected split")
        };
        assert_eq!(*dir, SplitDir::Horizontal);
        assert_eq!(children.len(), 2);

        let PaneNode::Split {
            dir: inner_dir,
            children: inner,
            ..
        } = &children[0]
        else {
            panic!("expected wrapped split")
        };
        assert_eq!(*inner_dir, SplitDir::Vertical);
        assert_eq!(inner.len(), 2);
        assert_eq!(pane_id_of(&inner[0]), &leaf_id);
    }

    #[test]
    fn 없는_페인에_새_탭_분할은_notfound_이고_레이아웃을_바꾸지_않는다() {
        let mut layout = default_layout();
        let missing = PaneId::new();

        let error = open_tab_in_split(&mut layout, &missing, DropEdge::Right, 파일_탭("a.rs")).expect_err("pane not found");

        assert!(matches!(error, AppError::NotFound(_)));
        assert!(matches!(layout.root, PaneNode::Leaf { .. }));
        assert_eq!(layout.revision, 0);
    }

    #[test]
    fn 새_탭_분할은_center_엣지를_거부한다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);

        let error = open_tab_in_split(&mut layout, &leaf_id, DropEdge::Center, 파일_탭("a.rs")).expect_err("center rejected");

        assert!(matches!(error, AppError::InvalidArgument(_)));
        assert!(matches!(layout.root, PaneNode::Leaf { .. }));
        assert_eq!(layout.revision, 0);
    }

    /// `open_tab` 은 kind 가 같은 탭을 재사용하므로 session_id 가 아직 빈 터미널 탭이 있으면
    /// "새 터미널"이 기존 탭 활성화로 끝난다. 분할 생성 경로는 그 중복 제거를 타지 않는다.
    #[test]
    fn 새_탭_분할은_같은_kind_터미널_탭을_중복_제거하지_않는다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);
        let terminal = 터미널_탭();
        let terminal_id = terminal.id.clone();

        open_tab_in_split(&mut layout, &leaf_id, DropEdge::Bottom, terminal).expect("open in split");

        let PaneNode::Split { children, .. } = &layout.root else {
            panic!("expected split")
        };
        let PaneNode::Leaf { tabs, .. } = &children[1] else {
            panic!("expected new leaf")
        };
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].id, terminal_id);
    }

    /// `insert_new_leaf` 추출 후에도 `split` 은 "이동" 의미를 유지한다 — 원래 페인에서 탭이 사라진다.
    #[test]
    fn 기존_split_은_추출_이후에도_탭을_원래_페인에서_옮긴다() {
        let mut layout = default_layout();
        let leaf_id = 루트_리프_id(&layout);
        let tab = 파일_탭("a.rs");
        let tab_id = tab.id.clone();
        open_tab(&mut layout, &leaf_id, tab, false).expect("open");
        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let 원래_탭_수 = tabs.len();

        split(&mut layout, &leaf_id, DropEdge::Right, &tab_id).expect("split");

        let PaneNode::Split { children, .. } = &layout.root else {
            panic!("expected split")
        };
        let PaneNode::Leaf { tabs: 남은_탭, .. } = &children[0] else {
            panic!("expected source leaf")
        };
        assert_eq!(남은_탭.len(), 원래_탭_수 - 1);
        assert!(남은_탭.iter().all(|tab| tab.id != tab_id));

        let PaneNode::Leaf { tabs: 옮겨진_탭, .. } = &children[1] else {
            panic!("expected new leaf")
        };
        assert_eq!(옮겨진_탭.len(), 1);
        assert_eq!(옮겨진_탭[0].id, tab_id);
    }

    #[test]
    fn 중첩된_같은_방향_스플릿은_평탄화된다() {
        let a = 리프(vec![파일_탭("a.rs")]);
        let b = 리프(vec![파일_탭("b.rs")]);
        let c = 리프(vec![파일_탭("c.rs")]);
        let inner = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![b, c],
            sizes: vec![0.5, 0.5],
        };
        let mut root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![a, inner],
            sizes: vec![0.5, 0.5],
        };

        normalize(&mut root);

        let PaneNode::Split { children, sizes, .. } = &root else {
            panic!("expected split")
        };
        assert_eq!(children.len(), 3);
        assert_eq!(sizes.len(), 3);
    }

    #[test]
    fn 프리뷰_탭은_기존_프리뷰_탭을_대체한다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();

        open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), true).expect("open preview a");
        open_tab(&mut layout, &leaf_id, 파일_탭("b.rs"), true).expect("open preview b");

        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let preview_tabs: Vec<_> = tabs.iter().filter(|tab| tab.preview).collect();
        assert_eq!(preview_tabs.len(), 1);
        assert!(matches!(&preview_tabs[0].kind, TabKind::File { path } if path == "b.rs"));
    }

    #[test]
    fn 프리뷰가_꺼진_채_열린_탭은_서로를_대체하지_않는다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();

        open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open a");
        open_tab(&mut layout, &leaf_id, 파일_탭("b.rs"), false).expect("open b");

        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let file_tabs: Vec<_> = tabs.iter().filter(|tab| matches!(&tab.kind, TabKind::File { .. })).collect();
        assert_eq!(file_tabs.len(), 2);
        assert!(file_tabs.iter().all(|tab| !tab.preview));
    }

    #[test]
    fn 핀_탭은_좌측_정렬을_유지한다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();

        let a = 파일_탭("a.rs");
        let a_id = a.id.clone();
        open_tab(&mut layout, &leaf_id, a, false).expect("open a");
        let b = 파일_탭("b.rs");
        open_tab(&mut layout, &leaf_id, b, false).expect("open b");

        pin_tab(&mut layout, &a_id, true).expect("pin a");

        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert!(tabs[0].pinned);
        assert_eq!(tabs[0].id, a_id);
    }

    #[test]
    fn 같은_파일_재열기는_활성화만_한다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();
        let before_len = tabs.len();

        let first_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open a");
        let second_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("reopen a");

        let PaneNode::Leaf { tabs, active, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert_eq!(first_id, second_id);
        assert_eq!(tabs.len(), before_len + 1);
        assert_eq!(active, &Some(first_id));
    }

    #[test]
    fn 닫은_탭_스택은_상한을_넘지_않는다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();

        for index in 0..(CLOSED_TAB_STACK_LIMIT + 5) {
            let tab = 파일_탭(&format!("file-{index}.rs"));
            let tab_id = tab.id.clone();
            open_tab(&mut layout, &leaf_id, tab, false).expect("open");
            close_tab(&mut layout, &tab_id).expect("close");
        }

        assert_eq!(layout.closed_tabs.len(), CLOSED_TAB_STACK_LIMIT);
    }

    #[test]
    fn 재열기는_후입선출_순서를_따른다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();

        let a = 파일_탭("a.rs");
        let a_id = a.id.clone();
        open_tab(&mut layout, &leaf_id, a, false).expect("open a");
        close_tab(&mut layout, &a_id).expect("close a");

        let b = 파일_탭("b.rs");
        let b_id = b.id.clone();
        open_tab(&mut layout, &leaf_id, b, false).expect("open b");
        close_tab(&mut layout, &b_id).expect("close b");

        let reopened_first = reopen_closed(&mut layout).expect("reopen first");
        let reopened_second = reopen_closed(&mut layout).expect("reopen second");

        assert_eq!(reopened_first, b_id);
        assert_eq!(reopened_second, a_id);
    }

    #[test]
    fn 스플릿_후_sizes_길이는_children_길이와_일치한다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();
        let tab = 파일_탭("a.rs");
        let tab_id = tab.id.clone();
        open_tab(&mut layout, &leaf_id, tab, false).expect("open");

        split(&mut layout, &leaf_id, DropEdge::Right, &tab_id).expect("split");

        let PaneNode::Split { children, sizes, .. } = &layout.root else {
            panic!("expected split")
        };
        assert_eq!(children.len(), sizes.len());
    }

    #[test]
    fn 탭을_닫으면_비는_리프가_제거되고_트리가_정규화된다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();
        let tab = 파일_탭("a.rs");
        let tab_id = tab.id.clone();
        open_tab(&mut layout, &leaf_id, tab, false).expect("open");
        split(&mut layout, &leaf_id, DropEdge::Right, &tab_id).expect("split");

        let PaneNode::Split { children, .. } = &layout.root else {
            panic!("expected split")
        };
        let new_leaf_id = pane_id_of(&children[1]).clone();

        close_tab(&mut layout, &tab_id).expect("close moved tab");

        assert!(matches!(&layout.root, PaneNode::Leaf { .. }));
        assert!(find_leaf(&layout.root, &new_leaf_id).is_none());
    }

    #[test]
    fn 리사이즈는_sizes_길이가_다르면_실패한다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();
        let tab = 파일_탭("a.rs");
        let tab_id = tab.id.clone();
        open_tab(&mut layout, &leaf_id, tab, false).expect("open");
        split(&mut layout, &leaf_id, DropEdge::Right, &tab_id).expect("split");

        let PaneNode::Split { id: split_id, .. } = &layout.root else {
            panic!("expected split")
        };
        let split_id = split_id.clone();

        let result = resize(&mut layout, &split_id, vec![1.0]);
        assert!(result.is_err());
    }

    #[test]
    fn 터미널_세션_설정은_탭의_세션_id_를_갱신한다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let terminal_tab_id = tabs[1].id.clone();
        let revision_before = layout.revision;

        set_terminal_session(&mut layout, &terminal_tab_id, "session-1".to_string()).expect("set session");

        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert!(matches!(&tabs[1].kind, TabKind::Terminal { session_id, .. } if session_id == "session-1"));
        assert_eq!(layout.revision, revision_before + 1);
    }

    #[test]
    fn 터미널이_아닌_탭에_세션을_설정하면_실패한다() {
        let mut layout = default_layout();
        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let welcome_tab_id = tabs[0].id.clone();

        let result = set_terminal_session(&mut layout, &welcome_tab_id, "session-1".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn 저장시_클로드_diff_탭은_필터링되고_활성탭은_인접탭으로_넘어간다() {
        let file_tab = 파일_탭("a.rs");
        let file_tab_id = file_tab.id.clone();
        let claude_diff_tab = 클로드_diff_탭("b.rs");
        let claude_diff_tab_id = claude_diff_tab.id.clone();
        let root = PaneNode::Leaf {
            id: PaneId::new(),
            tabs: vec![file_tab, claude_diff_tab],
            active: Some(claude_diff_tab_id),
        };
        let layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root,
            focused_pane: PaneId::new(),
            revision: 0,
            closed_tabs: Vec::new(),
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        let persisted = strip_volatile_tabs(&layout);

        let PaneNode::Leaf { tabs, active, .. } = &persisted.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].id, file_tab_id);
        assert_eq!(active, &Some(file_tab_id));
    }

    #[test]
    fn 클로드_diff_탭이_없으면_저장_대상이_그대로_유지된다() {
        let layout = default_layout();

        let persisted = strip_volatile_tabs(&layout);

        let PaneNode::Leaf { tabs, .. } = &persisted.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 2);
    }

    #[test]
    fn 클로드_diff_탭은_닫아도_닫힌_탭_스택에_쌓이지_않는다() {
        let file_tab = 파일_탭("a.rs");
        let file_tab_id = file_tab.id.clone();
        let claude_diff_tab = 클로드_diff_탭("b.rs");
        let claude_diff_tab_id = claude_diff_tab.id.clone();
        let mut layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root: 리프(vec![file_tab, claude_diff_tab]),
            focused_pane: PaneId::new(),
            revision: 0,
            closed_tabs: Vec::new(),
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        close_tab(&mut layout, &claude_diff_tab_id).expect("close claude diff");
        assert!(layout.closed_tabs.is_empty());

        close_tab(&mut layout, &file_tab_id).expect("close file");
        assert_eq!(layout.closed_tabs.len(), 1);
        assert_eq!(layout.closed_tabs[0].tab.id, file_tab_id);
    }

    #[test]
    fn 저장시_닫힌_탭_스택에서도_클로드_diff_탭이_제거된다() {
        let file_closed = ClosedTab {
            tab: 파일_탭("a.rs"),
            pane_id: PaneId::new(),
            index: 0,
        };
        let claude_diff_closed = ClosedTab {
            tab: 클로드_diff_탭("b.rs"),
            pane_id: PaneId::new(),
            index: 1,
        };
        let layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root: 리프(vec![파일_탭("root.rs")]),
            focused_pane: PaneId::new(),
            revision: 0,
            closed_tabs: vec![file_closed, claude_diff_closed],
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        let persisted = strip_volatile_tabs(&layout);

        assert_eq!(persisted.closed_tabs.len(), 1);
        assert!(!matches!(persisted.closed_tabs[0].tab.kind, TabKind::ClaudeDiff { .. }));
    }

    #[test]
    fn 클로드_diff_탭만_있던_리프는_저장시_정규화로_제거된다() {
        let claude_only_leaf = 리프(vec![클로드_diff_탭("b.rs")]);
        let file_leaf = 리프(vec![파일_탭("a.rs")]);
        let root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![claude_only_leaf, file_leaf],
            sizes: vec![0.5, 0.5],
        };
        let layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root,
            focused_pane: PaneId::new(),
            revision: 0,
            closed_tabs: Vec::new(),
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        let persisted = strip_volatile_tabs(&layout);

        assert!(matches!(&persisted.root, PaneNode::Leaf { .. }));
    }

    #[test]
    fn 제거된_패널을_가리키던_focused_pane은_남은_리프로_보정된다() {
        let claude_only_leaf = 리프(vec![클로드_diff_탭("b.rs")]);
        let removed_pane_id = pane_id_of(&claude_only_leaf).clone();
        let file_leaf = 리프(vec![파일_탭("a.rs")]);
        let kept_pane_id = pane_id_of(&file_leaf).clone();
        let root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![claude_only_leaf, file_leaf],
            sizes: vec![0.5, 0.5],
        };
        let layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root,
            focused_pane: removed_pane_id,
            revision: 0,
            closed_tabs: Vec::new(),
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        let persisted = strip_volatile_tabs(&layout);

        assert_eq!(persisted.focused_pane, kept_pane_id);
        assert!(find_leaf(&persisted.root, &persisted.focused_pane).is_some());
    }

    #[test]
    fn 살아있는_focused_pane은_그대로_유지된다() {
        let file_leaf = 리프(vec![파일_탭("a.rs")]);
        let focused = pane_id_of(&file_leaf).clone();
        let mut layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root: file_leaf,
            focused_pane: focused.clone(),
            revision: 0,
            closed_tabs: Vec::new(),
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        ensure_focused_pane_valid(&mut layout);

        assert_eq!(layout.focused_pane, focused);
    }

    #[test]
    fn title로_탭을_찾는다() {
        let tab = 클로드_diff_탭("b.rs");
        let tab_id = tab.id.clone();
        let root = 리프(vec![파일_탭("a.rs"), tab]);

        assert_eq!(find_tab_by_title(&root, "b.rs"), Some(tab_id));
    }

    #[test]
    fn 일치하는_title이_없으면_none이다() {
        let root = 리프(vec![파일_탭("a.rs")]);
        assert_eq!(find_tab_by_title(&root, "없는파일.rs"), None);
    }

    #[test]
    fn split_안쪽_패널의_탭도_title로_찾는다() {
        let tab = 클로드_diff_탭("nested.rs");
        let tab_id = tab.id.clone();
        let root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![리프(vec![파일_탭("a.rs")]), 리프(vec![tab])],
            sizes: vec![0.5, 0.5],
        };

        assert_eq!(find_tab_by_title(&root, "nested.rs"), Some(tab_id));
    }

    #[test]
    fn claude_diff_탭_id를_전부_모은다() {
        let diff_a = 클로드_diff_탭("a.rs");
        let diff_b = 클로드_diff_탭("b.rs");
        let ids = [diff_a.id.clone(), diff_b.id.clone()];
        let root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![리프(vec![파일_탭("plain.rs"), diff_a]), 리프(vec![diff_b])],
            sizes: vec![0.5, 0.5],
        };

        let mut collected = collect_claude_diff_tab_ids(&root);
        collected.sort_by_key(|id| id.as_str().to_string());
        let mut expected = ids.to_vec();
        expected.sort_by_key(|id| id.as_str().to_string());
        assert_eq!(collected, expected);
    }

    #[test]
    fn claude_diff_탭이_없으면_빈_목록이다() {
        let root = 리프(vec![파일_탭("a.rs")]);
        assert!(collect_claude_diff_tab_ids(&root).is_empty());
    }

    #[test]
    fn untitled_탭은_인덱스가_1부터_증가한다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();

        assert_eq!(next_untitled_index(&layout), 1);

        let first_index = next_untitled_index(&layout);
        open_tab(&mut layout, &leaf_id, 언타이틀드_탭(first_index), false).expect("open first untitled");
        assert_eq!(next_untitled_index(&layout), 2);

        let second_index = next_untitled_index(&layout);
        open_tab(&mut layout, &leaf_id, 언타이틀드_탭(second_index), false).expect("open second untitled");
        assert_eq!(next_untitled_index(&layout), 3);
    }

    #[test]
    fn 닫은_untitled_인덱스는_바로_재사용된다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();

        let first_index = next_untitled_index(&layout);
        let first_id = open_tab(&mut layout, &leaf_id, 언타이틀드_탭(first_index), false).expect("open first untitled");
        assert_eq!(next_untitled_index(&layout), 2);

        close_tab(&mut layout, &first_id).expect("close untitled");

        assert_eq!(next_untitled_index(&layout), 1);
    }

    #[test]
    fn untitled_두개를_열면_각각_별도_탭이_된다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();

        let first_id = open_tab(&mut layout, &leaf_id, 언타이틀드_탭(1), false).expect("open first");
        let second_id = open_tab(&mut layout, &leaf_id, 언타이틀드_탭(2), false).expect("open second");

        assert_ne!(first_id, second_id);
        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.iter().filter(|tab| matches!(tab.kind, TabKind::Untitled { .. })).count(), 2);
    }

    #[test]
    fn untitled_탭은_영속_저장에_포함된다() {
        let file_tab = 파일_탭("a.rs");
        let file_tab_id = file_tab.id.clone();
        let untitled_tab = 언타이틀드_탭(1);
        let untitled_id = untitled_tab.id.clone();
        let layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root: 리프(vec![file_tab, untitled_tab]),
            focused_pane: PaneId::new(),
            revision: 0,
            closed_tabs: Vec::new(),
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        let persisted = strip_volatile_tabs(&layout);

        let PaneNode::Leaf { tabs, .. } = &persisted.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].id, file_tab_id);
        assert!(tabs.iter().any(|tab| tab.id == untitled_id));
    }

    #[test]
    fn untitled_탭은_닫으면_닫힌_탭_스택에_쌓인다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let untitled_id = open_tab(&mut layout, &leaf_id, 언타이틀드_탭(1), false).expect("open untitled");

        close_tab(&mut layout, &untitled_id).expect("close untitled");

        assert_eq!(layout.closed_tabs.len(), 1);
        assert_eq!(layout.closed_tabs[0].tab.id, untitled_id);
    }

    #[test]
    fn 저장하면_같은_자리에서_file_탭이_된다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let untitled_id = open_tab(&mut layout, &leaf_id, 언타이틀드_탭(1), false).expect("open untitled");

        let converted_id =
            convert_untitled_to_file(&mut layout, &untitled_id, "notes.md".to_string(), "notes.md".to_string()).expect("convert");

        assert_eq!(converted_id, untitled_id);
        let (pane_id, _) = find_tab(&layout.root, &converted_id).expect("tab exists");
        assert_eq!(pane_id, &leaf_id);
        let tab = find_tab_mut(&mut layout.root, &converted_id).expect("tab exists");
        assert_eq!(
            tab.kind,
            TabKind::File {
                path: "notes.md".to_string()
            }
        );
        assert_eq!(tab.title, "notes.md");
        assert!(!tab.dirty);
    }

    #[test]
    fn 같은_leaf에_동일_경로_파일_탭이_있으면_기존_탭이_활성화된다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let existing_id = open_tab(&mut layout, &leaf_id, 파일_탭("notes.md"), false).expect("open existing file");
        let untitled_id = open_tab(&mut layout, &leaf_id, 언타이틀드_탭(1), false).expect("open untitled");

        let converted_id =
            convert_untitled_to_file(&mut layout, &untitled_id, "notes.md".to_string(), "notes.md".to_string()).expect("convert");

        assert_eq!(converted_id, existing_id);
        assert!(find_tab(&layout.root, &untitled_id).is_none());
        let PaneNode::Leaf { tabs, active, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.iter().filter(|tab| tab.id == existing_id).count(), 1);
        assert_eq!(active, &Some(existing_id));
    }

    #[test]
    fn untitled이_아닌_탭을_변환하려_하면_에러() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let file_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open file");

        let result = convert_untitled_to_file(&mut layout, &file_id, "b.rs".to_string(), "b.rs".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn 검색_에디터_탭은_영속_저장에_포함된다() {
        let file_tab = 파일_탭("a.rs");
        let file_tab_id = file_tab.id.clone();
        let search_tab = 검색_에디터_탭("needle");
        let search_tab_id = search_tab.id.clone();
        let layout = ProjectLayout {
            version: LAYOUT_SCHEMA_VERSION,
            root: 리프(vec![file_tab, search_tab]),
            focused_pane: PaneId::new(),
            revision: 0,
            closed_tabs: Vec::new(),
            auxiliary_windows: Vec::new(),
            shell_view: ShellViewState::default(),
        };

        let persisted = strip_volatile_tabs(&layout);

        let PaneNode::Leaf { tabs, .. } = &persisted.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].id, file_tab_id);
        assert!(tabs.iter().any(|tab| tab.id == search_tab_id));
    }

    #[test]
    fn 같은_쿼리의_검색_에디터_재열기는_활성화만_한다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();

        let first_id = open_tab(&mut layout, &leaf_id, 검색_에디터_탭("needle"), false).expect("open search editor");
        let second_id = open_tab(&mut layout, &leaf_id, 검색_에디터_탭("needle"), false).expect("reopen same query");

        let PaneNode::Leaf { tabs, active, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert_eq!(first_id, second_id);
        assert_eq!(
            tabs.iter().filter(|tab| matches!(tab.kind, TabKind::SearchEditor { .. })).count(),
            1
        );
        assert_eq!(active, &Some(first_id));
    }

    #[test]
    fn 다른_쿼리의_검색_에디터는_별도_탭으로_열린다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();

        open_tab(&mut layout, &leaf_id, 검색_에디터_탭("needle"), false).expect("open first search editor");
        open_tab(&mut layout, &leaf_id, 검색_에디터_탭("haystack"), false).expect("open second search editor");

        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert_eq!(
            tabs.iter().filter(|tab| matches!(tab.kind, TabKind::SearchEditor { .. })).count(),
            2
        );
    }

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-layout-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn v1_레이아웃_파일은_기존_탭을_보존한채_v2로_마이그레이션된다() {
        let paths = AppPaths::new(temp_data_dir("migrate-v1"));
        let project_id = ProjectId::new();

        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        open_tab(&mut layout, &leaf_id, 파일_탭("kept.rs"), false).expect("open");
        layout.version = 1;

        let mut raw = serde_json::to_value(&layout).expect("serialize");
        let object = raw.as_object_mut().expect("object");
        object.remove("auxiliaryWindows");
        object.remove("shellView");
        std::fs::create_dir_all(paths.project_dir(&project_id)).expect("create project dir");
        std::fs::write(
            paths.layout_file(&project_id),
            serde_json::to_vec_pretty(&raw).expect("serialize v1 json"),
        )
        .expect("write v1 layout file");

        let loaded = load_layout(&paths, &project_id);

        assert_eq!(loaded.version, LAYOUT_SCHEMA_VERSION);
        assert!(loaded.auxiliary_windows.is_empty());
        assert_eq!(loaded.shell_view, ShellViewState::default());
        let PaneNode::Leaf { tabs, .. } = &loaded.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 3, "welcome + terminal + kept.rs 가 모두 보존되어야 한다");
        assert!(tabs
            .iter()
            .any(|tab| matches!(&tab.kind, TabKind::File { path } if path == "kept.rs")));

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 미래_버전의_레이아웃_파일은_기본값으로_폴백한다() {
        let paths = AppPaths::new(temp_data_dir("migrate-future"));
        let project_id = ProjectId::new();
        let mut layout = default_layout();
        layout.version = LAYOUT_SCHEMA_VERSION + 1;
        std::fs::create_dir_all(paths.project_dir(&project_id)).expect("create project dir");
        persist::write_json(&paths.layout_file(&project_id), &layout).expect("write");

        let loaded = load_layout(&paths, &project_id);

        assert_eq!(loaded.version, LAYOUT_SCHEMA_VERSION);
        let PaneNode::Leaf { tabs, .. } = &loaded.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 2, "미래 버전은 마이그레이션하지 않고 기본 레이아웃으로 폴백해야 한다");

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 손상된_레이아웃_파일은_기본값으로_폴백한다() {
        let paths = AppPaths::new(temp_data_dir("migrate-corrupt"));
        let project_id = ProjectId::new();
        std::fs::create_dir_all(paths.project_dir(&project_id)).expect("create project dir");
        std::fs::write(paths.layout_file(&project_id), b"{not json").expect("write corrupt");

        let loaded = load_layout(&paths, &project_id);

        assert_eq!(loaded.version, LAYOUT_SCHEMA_VERSION);

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn appfile_탭의_dirty는_불러올때_초기화된다() {
        use crate::domain::app::types::AppFileTarget;

        let paths = AppPaths::new(temp_data_dir("appfile-dirty"));
        let project_id = ProjectId::new();

        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let mut app_file_tab = 파일_탭("unused.rs");
        app_file_tab.kind = TabKind::AppFile {
            target: AppFileTarget::Settings,
        };
        app_file_tab.dirty = true;
        open_tab(&mut layout, &leaf_id, app_file_tab, false).expect("open");
        std::fs::create_dir_all(paths.project_dir(&project_id)).expect("create project dir");
        persist::write_json(&paths.layout_file(&project_id), &layout).expect("write");

        let loaded = load_layout(&paths, &project_id);

        let PaneNode::Leaf { tabs, .. } = &loaded.root else {
            panic!("expected leaf")
        };
        let app_file_tab = tabs
            .iter()
            .find(|tab| matches!(tab.kind, TabKind::AppFile { .. }))
            .expect("appFile 탭이 보존되어야 한다");
        assert!(
            !app_file_tab.dirty,
            "미러가 없는 AppFile 탭의 dirty 는 재시작 후 유령으로 남으면 안 된다"
        );

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 새_보조_창으로_탭을_이동하면_dirty_pinned_view_state가_보존된다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let mut tab = 파일_탭("a.rs");
        tab.dirty = true;
        tab.pinned = true;
        tab.view_state = Some("scroll:10".to_string());
        let tab_id = tab.id.clone();
        open_tab(&mut layout, &leaf_id, tab, false).expect("open");

        let slot = next_window_slot(&layout);
        move_tab_to_new_window(&mut layout, &tab_id, slot).expect("move to new window");

        assert_eq!(layout.auxiliary_windows.len(), 1);
        let window = &layout.auxiliary_windows[0];
        assert_eq!(window.slot, slot);
        let PaneNode::Leaf { tabs, .. } = &window.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 1);
        assert!(tabs[0].dirty);
        assert!(tabs[0].pinned);
        assert_eq!(tabs[0].view_state, Some("scroll:10".to_string()));
        assert!(find_tab(&layout.root, &tab_id).is_none(), "탭은 main 트리에서 사라져야 한다");
    }

    #[test]
    fn 기존_보조_창으로_탭을_이동할_수_있다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let first_tab_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open a");
        let slot = next_window_slot(&layout);
        move_tab_to_new_window(&mut layout, &first_tab_id, slot).expect("move a to new window");

        let second_tab_id = open_tab(&mut layout, &leaf_id, 파일_탭("b.rs"), false).expect("open b");
        move_tab_to_existing_window(&mut layout, &second_tab_id, slot).expect("move b to existing window");

        let window = layout
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == slot)
            .expect("window exists");
        let PaneNode::Leaf { tabs, .. } = &window.root else {
            panic!("expected leaf")
        };
        assert_eq!(tabs.len(), 2);
    }

    #[test]
    fn 존재하지_않는_슬롯으로_이동하면_에러() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open");

        let result = move_tab_to_existing_window(&mut layout, &tab_id, 999);

        assert!(result.is_err());
    }

    #[test]
    fn 보조_창의_마지막_탭을_main으로_되돌리면_창이_비워진다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open");
        let slot = next_window_slot(&layout);
        move_tab_to_new_window(&mut layout, &tab_id, slot).expect("move to new window");

        move_tab_to_main(&mut layout, &tab_id).expect("move back to main");

        let window = layout
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == slot)
            .expect("window still recorded");
        assert!(is_layout_tree_empty(&window.root));
        assert!(find_tab(&layout.root, &tab_id).is_some(), "탭은 main으로 돌아와야 한다");
    }

    #[test]
    fn 보조_창_닫기는_탭을_main_말미로_복귀시키고_창_항목을_제거한다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open");
        let slot = next_window_slot(&layout);
        move_tab_to_new_window(&mut layout, &tab_id, slot).expect("move to new window");

        let active_before = {
            let PaneNode::Leaf { active, .. } = &layout.root else {
                panic!("expected leaf")
            };
            active.clone()
        };

        let returned = return_auxiliary_window_tabs(&mut layout, slot);

        assert!(returned);
        assert!(layout.auxiliary_windows.is_empty());
        let PaneNode::Leaf { tabs, active, .. } = &layout.root else {
            panic!("expected leaf")
        };
        assert!(tabs.iter().any(|tab| tab.id == tab_id));
        assert_eq!(active, &active_before, "복귀는 main의 현재 포커스를 빼앗지 않는다");
    }

    #[test]
    fn 존재하지_않는_슬롯의_탭_복귀는_아무_일도_하지_않는다() {
        let mut layout = default_layout();
        assert!(!return_auxiliary_window_tabs(&mut layout, 999));
    }

    #[test]
    fn 닫힌_보조_창의_슬롯_번호는_재사용된다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open");
        let first_slot = next_window_slot(&layout);
        move_tab_to_new_window(&mut layout, &tab_id, first_slot).expect("move");
        assert_eq!(first_slot, 1);

        return_auxiliary_window_tabs(&mut layout, first_slot);

        assert_eq!(next_window_slot(&layout), 1, "닫힌 슬롯 번호는 재사용되어야 한다");
    }

    #[test]
    fn 보조_창_내부에서도_스플릿과_탭_열기가_그_창의_트리에만_반영된다() {
        let mut layout = default_layout();
        let leaf_id = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &leaf_id, 파일_탭("a.rs"), false).expect("open");
        let slot = next_window_slot(&layout);
        move_tab_to_new_window(&mut layout, &tab_id, slot).expect("move to new window");

        let aux_leaf_id = layout.auxiliary_windows[0].focused_pane.clone();
        let second_tab = 파일_탭("b.rs");
        let second_tab_id = second_tab.id.clone();
        open_tab(&mut layout, &aux_leaf_id, second_tab, false).expect("open second tab in aux window");
        split(&mut layout, &aux_leaf_id, DropEdge::Right, &second_tab_id).expect("split within aux window");

        let window = &layout.auxiliary_windows[0];
        assert!(
            matches!(&window.root, PaneNode::Split { .. }),
            "보조 창 내부 스플릿이 그 창의 트리에 반영되어야 한다"
        );
        assert!(matches!(&layout.root, PaneNode::Leaf { .. }), "main 트리는 영향을 받지 않아야 한다");
    }

    #[test]
    fn shell_view_패치는_none_필드를_보존한다() {
        let mut layout = default_layout();
        apply_shell_view_patch(
            &mut layout,
            &ShellViewPatch {
                zen: Some(true),
                sidebar_collapsed: None,
            },
        );
        assert!(layout.shell_view.zen);
        assert!(!layout.shell_view.sidebar_collapsed);

        apply_shell_view_patch(
            &mut layout,
            &ShellViewPatch {
                zen: None,
                sidebar_collapsed: Some(true),
            },
        );
        assert!(layout.shell_view.zen, "zen 값은 유지되어야 한다");
        assert!(layout.shell_view.sidebar_collapsed);
    }

    /// Regression coverage for the project-resolution locator helpers themselves — every layout
    /// `#[tauri::command]` (`layout_close_tab`, `layout_activate_tab`, `layout_move_tab_to_window`
    /// back to `Main`, ...) funnels through `locate_project_with_tab`/`locate_project_with_pane`
    /// first, so a regression here would silently make every one of those commands return
    /// `NotFound` for anything living in an auxiliary window — exactly the class of bug the pure
    /// tree tests above (`move_tab_to_new_window`/`move_tab_to_main`) can't catch, since they call
    /// tree functions directly and never exercise this `HashMap<ProjectId, ProjectLayout>`
    /// project-resolution step at all.
    #[test]
    fn locate_project_with_tab_과_pane_은_보조_창_트리도_찾는다() {
        let mut layout = default_layout();
        let source_pane = layout.focused_pane.clone();

        let PaneNode::Leaf { tabs, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let tab_id = tabs[0].id.clone();

        move_tab_to_new_window(&mut layout, &tab_id, 1).expect("move into new window");
        let aux_pane_id = layout.auxiliary_windows[0].focused_pane.clone();

        let mut layouts = HashMap::new();
        let project_id = ProjectId::new();
        layouts.insert(project_id.clone(), layout);

        assert_eq!(
            locate_project_with_tab(&layouts, &tab_id).expect("보조 창으로 옮긴 탭도 찾아야 한다"),
            project_id,
            "탭이 보조 창 트리에만 있어도 프로젝트를 찾아야 한다"
        );
        assert_eq!(
            locate_project_with_pane(&layouts, &aux_pane_id).expect("보조 창의 pane 도 찾아야 한다"),
            project_id,
            "pane 이 보조 창 트리에만 있어도 프로젝트를 찾아야 한다"
        );
        assert_eq!(
            locate_project_with_pane(&layouts, &source_pane).expect("main 트리의 pane 은 여전히 찾아야 한다"),
            project_id
        );

        assert!(
            locate_project_with_tab(&layouts, &TabId::new()).is_err(),
            "존재하지 않는 탭은 NotFound 여야 한다"
        );
    }

    #[test]
    fn open_file_paths는_메인과_보조창의_파일_탭_경로를_중복없이_수집한다() {
        let mut layout = default_layout();
        let main_pane = layout.focused_pane.clone();

        open_tab(&mut layout, &main_pane, 파일_탭("a.rs"), false).expect("open a.rs in main");
        let move_target = open_tab(&mut layout, &main_pane, 파일_탭("b.rs"), false).expect("open b.rs in main");

        move_tab_to_new_window(&mut layout, &move_target, 1).expect("move b.rs into new window");
        let aux_pane = layout.auxiliary_windows[0].focused_pane.clone();
        open_tab(&mut layout, &aux_pane, 파일_탭("a.rs"), false).expect("open a.rs again in aux window");

        let mut paths = open_file_paths(&layout);
        paths.sort();

        assert_eq!(
            paths,
            vec!["a.rs".to_string(), "b.rs".to_string()],
            "메인·보조 창에 걸쳐 열린 파일 탭 경로가 중복 없이 모여야 한다"
        );
    }

    #[test]
    fn open_file_paths는_파일_탭이_없으면_비어있다() {
        let layout = default_layout();
        assert!(
            open_file_paths(&layout).is_empty(),
            "기본 레이아웃(웰컴·터미널 탭만)에는 파일 탭이 없어야 한다"
        );
    }

    fn 개명(from: &str, to: &str) -> TabPathChange {
        TabPathChange::Renamed {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    fn 탭_경로들(layout: &ProjectLayout) -> Vec<String> {
        all_roots(layout)
            .flat_map(collect_leaves)
            .filter_map(|leaf| match leaf {
                PaneNode::Leaf { tabs, .. } => Some(tabs),
                PaneNode::Split { .. } => None,
            })
            .flatten()
            .filter_map(|tab| match &tab.kind {
                TabKind::File { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn 파일_개명은_열린_탭의_경로와_제목을_따라간다() {
        let mut layout = default_layout();
        let pane = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &pane, 파일_탭("/repo/src/old.rs"), false).expect("open");

        let outcome = apply_tab_path_change(&mut layout, &개명("/repo/src/old.rs", "/repo/src/new.rs"));

        let tab = find_tab_mut_in_layout(&mut layout, &tab_id).expect("tab exists");
        assert_eq!(
            tab.kind,
            TabKind::File {
                path: "/repo/src/new.rs".to_string()
            }
        );
        assert_eq!(tab.title, "new.rs", "탭 제목도 새 파일 이름을 따라야 한다");
        assert_eq!(
            outcome.moved,
            vec![TabPathMove {
                from: "/repo/src/old.rs".to_string(),
                to: "/repo/src/new.rs".to_string(),
                dirty: false,
            }]
        );
        assert!(outcome.closed_paths.is_empty());
    }

    #[test]
    fn 폴더_개명은_하위_열린_탭_전체의_경로를_치환한다() {
        let mut layout = default_layout();
        let pane = layout.focused_pane.clone();
        open_tab(&mut layout, &pane, 파일_탭("/repo/src/a.rs"), false).expect("open a");
        let moved_out = open_tab(&mut layout, &pane, 파일_탭("/repo/src/nested/b.rs"), false).expect("open b");
        open_tab(&mut layout, &pane, 파일_탭("/repo/other/c.rs"), false).expect("open c");
        move_tab_to_new_window(&mut layout, &moved_out, 1).expect("move b into new window");

        let outcome = apply_tab_path_change(&mut layout, &개명("/repo/src", "/repo/lib"));

        let mut paths = 탭_경로들(&layout);
        paths.sort();
        assert_eq!(
            paths,
            vec![
                "/repo/lib/a.rs".to_string(),
                "/repo/lib/nested/b.rs".to_string(),
                "/repo/other/c.rs".to_string(),
            ],
            "보조 창의 하위 탭까지 prefix 가 치환되고, 무관한 탭은 그대로여야 한다"
        );
        assert_eq!(outcome.moved.len(), 2);
    }

    #[test]
    fn 이름이_접두사인_형제는_개명_대상이_아니다() {
        let mut layout = default_layout();
        let pane = layout.focused_pane.clone();
        open_tab(&mut layout, &pane, 파일_탭("/repo/src-old/a.rs"), false).expect("open sibling");

        let outcome = apply_tab_path_change(&mut layout, &개명("/repo/src", "/repo/lib"));

        assert_eq!(탭_경로들(&layout), vec!["/repo/src-old/a.rs".to_string()]);
        assert!(outcome.is_empty(), "문자열 접두사만 겹치는 형제는 움직이면 안 된다");
    }

    #[test]
    fn 같은_파일이_두_페인에_열려_있으면_한_번만_보고하고_dirty_를_합친다() {
        let mut layout = default_layout();
        let pane = layout.focused_pane.clone();
        let clean_tab = open_tab(&mut layout, &pane, 파일_탭("/repo/a.rs"), false).expect("open clean");
        let mut dirty_tab = 파일_탭("/repo/a.rs");
        dirty_tab.dirty = true;
        let dirty_tab_id = dirty_tab.id.clone();
        move_tab_to_new_window(&mut layout, &clean_tab, 1).expect("move clean tab into new window");
        open_tab(&mut layout, &pane, dirty_tab, false).expect("open dirty");

        let outcome = apply_tab_path_change(&mut layout, &개명("/repo/a.rs", "/repo/b.rs"));

        assert_eq!(
            outcome.moved,
            vec![TabPathMove {
                from: "/repo/a.rs".to_string(),
                to: "/repo/b.rs".to_string(),
                dirty: true,
            }],
            "경로 하나당 한 건만 보고하되 dirty 는 어느 한쪽이라도 참이면 참이어야 한다"
        );
        assert!(find_tab_mut_in_layout(&mut layout, &dirty_tab_id).is_some());
        assert_eq!(탭_경로들(&layout).len(), 2, "두 탭 모두 새 경로로 이동해야 한다");
    }

    #[test]
    fn 개명은_닫은_탭_스택의_경로도_따라간다() {
        let mut layout = default_layout();
        let pane = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &pane, 파일_탭("/repo/src/old.rs"), false).expect("open");
        close_tab(&mut layout, &tab_id).expect("close");

        apply_tab_path_change(&mut layout, &개명("/repo/src", "/repo/lib"));

        let closed = layout.closed_tabs.last().expect("closed tab exists");
        assert_eq!(
            closed.tab.kind,
            TabKind::File {
                path: "/repo/lib/old.rs".to_string()
            }
        );
        assert_eq!(closed.tab.title, "old.rs");
    }

    #[test]
    fn 삭제는_해당_경로와_하위_파일_탭만_닫는다() {
        let mut layout = default_layout();
        let pane = layout.focused_pane.clone();
        open_tab(&mut layout, &pane, 파일_탭("/repo/src/a.rs"), false).expect("open a");
        open_tab(&mut layout, &pane, 파일_탭("/repo/src/nested/b.rs"), false).expect("open b");
        open_tab(&mut layout, &pane, 파일_탭("/repo/src-old/c.rs"), false).expect("open c");
        open_tab(&mut layout, &pane, 검색_에디터_탭("keep"), false).expect("open search editor");

        let outcome = apply_tab_path_change(
            &mut layout,
            &TabPathChange::Deleted {
                path: "/repo/src".to_string(),
            },
        );

        assert_eq!(탭_경로들(&layout), vec!["/repo/src-old/c.rs".to_string()]);
        let mut closed = outcome.closed_paths.clone();
        closed.sort();
        assert_eq!(closed, vec!["/repo/src/a.rs".to_string(), "/repo/src/nested/b.rs".to_string()]);
        assert!(outcome.moved.is_empty());
    }

    #[test]
    fn 대상_탭이_없으면_revision_을_올리지_않는다() {
        let mut layout = default_layout();
        let before = layout.revision;

        let renamed = apply_tab_path_change(&mut layout, &개명("/repo/gone.rs", "/repo/other.rs"));
        let deleted = apply_tab_path_change(
            &mut layout,
            &TabPathChange::Deleted {
                path: "/repo/gone.rs".to_string(),
            },
        );

        assert!(renamed.is_empty() && deleted.is_empty());
        assert!(
            !renamed.layout_changed && !deleted.layout_changed,
            "매칭된 것이 하나도 없으면 레이아웃 자체가 바뀌지 않았다"
        );
        assert_eq!(layout.revision, before, "아무 탭도 바뀌지 않으면 revision 은 그대로여야 한다");
    }

    #[test]
    fn 열린_탭이_없어도_닫은_탭_스택의_경로는_따라가고_레이아웃_변경으로_보고된다() {
        let mut layout = default_layout();
        let pane = layout.focused_pane.clone();
        let tab_id = open_tab(&mut layout, &pane, 파일_탭("/repo/a.txt"), false).expect("open");
        close_tab(&mut layout, &tab_id).expect("close");
        let before = layout.revision;

        let outcome = apply_tab_path_change(&mut layout, &개명("/repo/a.txt", "/repo/b.txt"));

        assert!(outcome.is_empty(), "열린 탭이 없으므로 프론트가 이관할 상태는 없다");
        assert!(
            outcome.layout_changed,
            "닫은 탭 스택이 새 경로로 바뀌었으므로 이 레이아웃은 저장돼야 한다 (커맨드의 조기 반환이 버리면 안 된다)"
        );
        assert_eq!(
            layout.closed_tabs.last().expect("closed tab exists").tab.kind,
            TabKind::File {
                path: "/repo/b.txt".to_string()
            }
        );
        assert_eq!(layout.revision, before, "화면에 보이는 것은 없으므로 revision 은 그대로다");
    }
}
