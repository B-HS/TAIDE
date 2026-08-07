use super::types::{
    ClosedTab, DropEdge, FocusKind, PaneNode, ProjectLayout, SplitDir, Tab, TabKind, CLOSED_TAB_STACK_LIMIT, LAYOUT_SCHEMA_VERSION,
};
use crate::error::{AppError, AppResult};
use crate::ids::{PaneId, ProjectId, TabId};
use crate::infra::persist;
use crate::paths::AppPaths;

const SPLIT_TOTAL_PERCENT: f32 = 100.0;

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
    let leaf = find_leaf_mut(&mut layout.root, pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
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

pub fn close_tab(layout: &mut ProjectLayout, tab_id: &TabId) -> AppResult<ClosedTab> {
    let (pane_id, index) = find_tab(&layout.root, tab_id)
        .map(|(pane_id, index)| (pane_id.clone(), index))
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let tab = extract_tab(&mut layout.root, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    normalize(&mut layout.root);

    let closed = ClosedTab {
        tab,
        pane_id,
        index: index as u32,
    };
    push_closed(layout, closed.clone());
    layout.revision += 1;
    Ok(closed)
}

/// ClaudeDiff 탭은 닫히는 순간 pending 요청이 해소되므로(`reconcile_closed_tab`),
/// 되살려도 수락/거부가 실패하는 좀비 탭이 된다. 스택에 넣지 않는다.
fn push_closed(layout: &mut ProjectLayout, closed: ClosedTab) {
    if matches!(closed.tab.kind, TabKind::ClaudeDiff { .. }) {
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

    let target_pane = if find_leaf(&layout.root, &pane_id).is_some() {
        pane_id
    } else if find_leaf(&layout.root, &layout.focused_pane).is_some() {
        layout.focused_pane.clone()
    } else {
        collect_leaves(&layout.root).first().map(|leaf| pane_id_of(leaf).clone())?
    };

    if let Some(leaf) = find_leaf_mut(&mut layout.root, &target_pane) {
        insert_tab(leaf, tab, Some(index as usize));
    }
    layout.focused_pane = target_pane;
    layout.revision += 1;
    Some(tab_id)
}

pub fn activate_tab(layout: &mut ProjectLayout, tab_id: &TabId) -> AppResult<()> {
    let pane_id = find_tab(&layout.root, tab_id)
        .map(|(pane_id, _)| pane_id.clone())
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let leaf = find_leaf_mut(&mut layout.root, &pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
    if let PaneNode::Leaf { active, .. } = leaf {
        *active = Some(tab_id.clone());
    }
    layout.focused_pane = pane_id;
    layout.revision += 1;
    Ok(())
}

pub fn pin_tab(layout: &mut ProjectLayout, tab_id: &TabId, pinned: bool) -> AppResult<()> {
    let pane_id = find_tab(&layout.root, tab_id)
        .map(|(pane_id, _)| pane_id.clone())
        .ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let leaf = find_leaf_mut(&mut layout.root, &pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
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
    let tab = find_tab_mut(&mut layout.root, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    tab.preview = preview;
    layout.revision += 1;
    Ok(())
}

pub fn move_tab(layout: &mut ProjectLayout, tab_id: &TabId, target_pane: &PaneId, index: usize) -> AppResult<()> {
    if find_leaf(&layout.root, target_pane).is_none() {
        return Err(AppError::NotFound(format!("pane not found: {target_pane}")));
    }
    let tab = extract_tab(&mut layout.root, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;

    let target = find_leaf_mut(&mut layout.root, target_pane).ok_or_else(|| AppError::Internal("pane vanished during move".to_string()))?;
    insert_tab(target, tab, Some(index));
    layout.focused_pane = target_pane.clone();
    normalize(&mut layout.root);
    layout.revision += 1;
    Ok(())
}

pub fn split(layout: &mut ProjectLayout, target_pane: &PaneId, edge: DropEdge, tab_id: &TabId) -> AppResult<()> {
    if find_leaf(&layout.root, target_pane).is_none() {
        return Err(AppError::NotFound(format!("pane not found: {target_pane}")));
    }

    if edge == DropEdge::Center {
        return move_tab(layout, tab_id, target_pane, usize::MAX);
    }

    let tab = extract_tab(&mut layout.root, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let dir = match edge {
        DropEdge::Left | DropEdge::Right => SplitDir::Horizontal,
        DropEdge::Top | DropEdge::Bottom => SplitDir::Vertical,
        DropEdge::Center => return Err(AppError::Internal("center edge handled above".to_string())),
    };

    let new_leaf_id = PaneId::new();
    let new_tab_id = tab.id.clone();
    let new_leaf = PaneNode::Leaf {
        id: new_leaf_id.clone(),
        tabs: vec![tab],
        active: Some(new_tab_id),
    };

    let is_root_target = matches!(&layout.root, PaneNode::Leaf { id, .. } if id == target_pane);
    if is_root_target {
        let placeholder = PaneNode::Leaf {
            id: PaneId::new(),
            tabs: Vec::new(),
            active: None,
        };
        let existing = std::mem::replace(&mut layout.root, placeholder);
        layout.root = wrap_leaf_in_split(existing, new_leaf, dir, edge);
    } else {
        let mut pending = Some(new_leaf);
        insert_split_at(&mut layout.root, target_pane, dir, edge, &mut pending);
        if pending.is_some() {
            return Err(AppError::NotFound(format!("pane not found: {target_pane}")));
        }
    }

    layout.focused_pane = new_leaf_id;
    normalize(&mut layout.root);
    layout.revision += 1;
    Ok(())
}

pub fn resize(layout: &mut ProjectLayout, pane_id: &PaneId, sizes: Vec<f32>) -> AppResult<()> {
    let node = find_split_mut(&mut layout.root, pane_id).ok_or_else(|| AppError::NotFound(format!("pane not found: {pane_id}")))?;
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
    if find_leaf(&layout.root, pane_id).is_none() {
        return Err(AppError::NotFound(format!("pane not found: {pane_id}")));
    }
    layout.focused_pane = pane_id.clone();
    layout.revision += 1;
    Ok(())
}

pub fn set_view_state(layout: &mut ProjectLayout, tab_id: &TabId, view_state: Option<String>) -> AppResult<()> {
    let tab = find_tab_mut(&mut layout.root, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    tab.view_state = view_state;
    layout.revision += 1;
    Ok(())
}

pub fn set_dirty(layout: &mut ProjectLayout, tab_id: &TabId, dirty: bool) -> AppResult<()> {
    let tab = find_tab_mut(&mut layout.root, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    tab.dirty = dirty;
    layout.revision += 1;
    Ok(())
}

pub fn set_terminal_session(layout: &mut ProjectLayout, tab_id: &TabId, session_id: String) -> AppResult<()> {
    let tab = find_tab_mut(&mut layout.root, tab_id).ok_or_else(|| AppError::NotFound(format!("tab not found: {tab_id}")))?;
    let TabKind::Terminal { session_id: existing, .. } = &mut tab.kind else {
        return Err(AppError::InvalidArgument(format!("tab is not a terminal: {tab_id}")));
    };
    *existing = session_id;
    layout.revision += 1;
    Ok(())
}

pub fn focus_kind(layout: &ProjectLayout) -> Option<FocusKind> {
    let leaf = find_leaf(&layout.root, &layout.focused_pane)?;
    let PaneNode::Leaf { tabs, active, .. } = leaf else { return None };
    let active_id = active.as_ref()?;
    let tab = tabs.iter().find(|tab| &tab.id == active_id)?;
    Some(FocusKind::from(&tab.kind))
}

fn strip_claude_diff_node(node: &mut PaneNode) {
    match node {
        PaneNode::Leaf { tabs, active, .. } => {
            let removed_index = active
                .as_ref()
                .and_then(|active_id| tabs.iter().position(|tab| &tab.id == active_id));
            let active_was_claude_diff = removed_index
                .map(|index| matches!(tabs[index].kind, TabKind::ClaudeDiff { .. }))
                .unwrap_or(false);
            tabs.retain(|tab| !matches!(tab.kind, TabKind::ClaudeDiff { .. }));
            if active_was_claude_diff {
                *active = if tabs.is_empty() {
                    None
                } else {
                    Some(tabs[removed_index.unwrap_or(0).min(tabs.len() - 1)].id.clone())
                };
            }
        }
        PaneNode::Split { children, .. } => children.iter_mut().for_each(strip_claude_diff_node),
    }
}

/// `normalize` 로 패널이 사라지면 `focused_pane` 이 존재하지 않는 pane 을 가리킬 수 있다.
/// 그 상태로 저장/복원되면 이후 탭 열기가 계속 실패하므로 첫 리프로 보정한다.
pub fn ensure_focused_pane_valid(layout: &mut ProjectLayout) {
    if find_leaf(&layout.root, &layout.focused_pane).is_some() {
        return;
    }
    let fallback = collect_leaves(&layout.root).first().map(|leaf| pane_id_of(leaf).clone());
    if let Some(pane_id) = fallback {
        layout.focused_pane = pane_id;
    }
}

pub fn strip_claude_diff_tabs(layout: &ProjectLayout) -> ProjectLayout {
    let mut persisted = layout.clone();
    strip_claude_diff_node(&mut persisted.root);
    normalize(&mut persisted.root);
    ensure_focused_pane_valid(&mut persisted);
    persisted
        .closed_tabs
        .retain(|closed| !matches!(closed.tab.kind, TabKind::ClaudeDiff { .. }));
    persisted
}

pub fn save_layout(paths: &AppPaths, project_id: &ProjectId, layout: &ProjectLayout) -> AppResult<()> {
    let persisted = strip_claude_diff_tabs(layout);
    persist::write_json(&paths.layout_file(project_id), &persisted)
}

pub fn load_layout(paths: &AppPaths, project_id: &ProjectId) -> ProjectLayout {
    match persist::read_json::<ProjectLayout>(&paths.layout_file(project_id)) {
        Ok(Some(layout)) if layout.version == LAYOUT_SCHEMA_VERSION => {
            let mut layout = layout;
            ensure_focused_pane_valid(&mut layout);
            layout
        }
        _ => default_layout(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::TabKind;
    use super::*;

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
    fn 포커스_종류는_활성_탭의_종류를_따른다() {
        let mut layout = default_layout();
        assert_eq!(focus_kind(&layout), Some(FocusKind::Welcome));

        let PaneNode::Leaf { id: leaf_id, .. } = &layout.root else {
            panic!("expected leaf")
        };
        let leaf_id = leaf_id.clone();
        let tab = 파일_탭("a.rs");
        let tab_id = tab.id.clone();
        open_tab(&mut layout, &leaf_id, tab, false).expect("open");
        activate_tab(&mut layout, &tab_id).expect("activate");

        assert_eq!(focus_kind(&layout), Some(FocusKind::File));
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
        };

        let persisted = strip_claude_diff_tabs(&layout);

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

        let persisted = strip_claude_diff_tabs(&layout);

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
        };

        let persisted = strip_claude_diff_tabs(&layout);

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
        };

        let persisted = strip_claude_diff_tabs(&layout);

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
        };

        let persisted = strip_claude_diff_tabs(&layout);

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
}
