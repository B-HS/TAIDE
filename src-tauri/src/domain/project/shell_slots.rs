use crate::domain::layout::types::SplitDir;
use crate::error::{AppError, AppResult};
use crate::ids::{ProjectId, ShellSlotId};

use super::types::{ShellSlotEdge, ShellSlotTree};

/// Percentages a fresh split hands its two children, mirroring
/// `layout::service::SPLIT_TOTAL_PERCENT` — the frontend feeds these straight to
/// `react-resizable-panels`, which expects percentages summing to 100.
const SPLIT_TOTAL_PERCENT: f32 = 100.0;
/// A shell slot split is always binary — see [`ShellSlotTree`]'s own doc for why this tree does not
/// flatten same-direction splits the way `PaneNode` does.
const SPLIT_CHILD_COUNT: usize = 2;

pub fn leaf(project_id: ProjectId) -> ShellSlotTree {
    ShellSlotTree::Leaf {
        slot_id: ShellSlotId::new(),
        project_id,
    }
}

/// Every `(slot, project)` pair in tree order (left/top first), which is also the order the
/// frontend renders them in.
pub fn leaves(tree: &ShellSlotTree) -> Vec<(ShellSlotId, ProjectId)> {
    match tree {
        ShellSlotTree::Leaf { slot_id, project_id } => vec![(slot_id.clone(), project_id.clone())],
        ShellSlotTree::Split { children, .. } => children.iter().flat_map(leaves).collect(),
    }
}

pub fn leaf_count(tree: &ShellSlotTree) -> usize {
    match tree {
        ShellSlotTree::Leaf { .. } => 1,
        ShellSlotTree::Split { children, .. } => children.iter().map(leaf_count).sum(),
    }
}

pub fn first_slot(tree: &ShellSlotTree) -> Option<ShellSlotId> {
    leaves(tree).into_iter().next().map(|(slot_id, _)| slot_id)
}

pub fn contains_slot(tree: &ShellSlotTree, slot_id: &ShellSlotId) -> bool {
    project_of_slot(tree, slot_id).is_some()
}

pub fn project_of_slot(tree: &ShellSlotTree, slot_id: &ShellSlotId) -> Option<ProjectId> {
    match tree {
        ShellSlotTree::Leaf {
            slot_id: candidate,
            project_id,
        } => (candidate == slot_id).then(|| project_id.clone()),
        ShellSlotTree::Split { children, .. } => children.iter().find_map(|child| project_of_slot(child, slot_id)),
    }
}

pub fn slot_of_project(tree: &ShellSlotTree, project_id: &ProjectId) -> Option<ShellSlotId> {
    match tree {
        ShellSlotTree::Leaf {
            slot_id,
            project_id: candidate,
        } => (candidate == project_id).then(|| slot_id.clone()),
        ShellSlotTree::Split { children, .. } => children.iter().find_map(|child| slot_of_project(child, project_id)),
    }
}

/// Points an existing slot at another project, leaving the tree's shape untouched — what
/// [`ShellSlotEdge::Replace`] and a plain sidebar activation both do.
pub fn set_slot_project(tree: &mut ShellSlotTree, slot_id: &ShellSlotId, project_id: ProjectId) -> bool {
    match tree {
        ShellSlotTree::Leaf {
            slot_id: candidate,
            project_id: existing,
        } => {
            if candidate != slot_id {
                return false;
            }
            *existing = project_id;
            true
        }
        ShellSlotTree::Split { children, .. } => children
            .iter_mut()
            .any(|child| set_slot_project(child, slot_id, project_id.clone())),
    }
}

/// The split axis a directional edge creates. `Replace` has none — callers resolve it before ever
/// building a split, the same shape as `layout::service::split_dir_of_edge`.
pub fn split_dir_of_edge(edge: ShellSlotEdge) -> Option<SplitDir> {
    match edge {
        ShellSlotEdge::Left | ShellSlotEdge::Right => Some(SplitDir::Horizontal),
        ShellSlotEdge::Top | ShellSlotEdge::Bottom => Some(SplitDir::Vertical),
        ShellSlotEdge::Replace => None,
    }
}

fn is_leading(edge: ShellSlotEdge) -> bool {
    matches!(edge, ShellSlotEdge::Left | ShellSlotEdge::Top)
}

/// Wraps the leaf named by `target` in a fresh binary split and puts `project_id` in the new
/// sibling, answering with the new slot's id. Returns `None` when `target` names no leaf, which the
/// caller turns into `NotFound` — the tree is untouched in that case, because the rewrite only
/// happens on the node that matched.
pub fn split_slot(
    tree: &mut ShellSlotTree,
    target: &ShellSlotId,
    edge: ShellSlotEdge,
    project_id: &ProjectId,
) -> AppResult<Option<ShellSlotId>> {
    let Some(dir) = split_dir_of_edge(edge) else {
        return Err(AppError::InvalidArgument("replace edge has no split axis".to_string()));
    };
    Ok(split_at(tree, target, dir, is_leading(edge), project_id))
}

fn split_at(tree: &mut ShellSlotTree, target: &ShellSlotId, dir: SplitDir, leading: bool, project_id: &ProjectId) -> Option<ShellSlotId> {
    match tree {
        ShellSlotTree::Leaf { slot_id, .. } => {
            if slot_id != target {
                return None;
            }
            let new_slot = ShellSlotId::new();
            let new_leaf = ShellSlotTree::Leaf {
                slot_id: new_slot.clone(),
                project_id: project_id.clone(),
            };
            let placeholder = ShellSlotTree::Leaf {
                slot_id: new_slot.clone(),
                project_id: project_id.clone(),
            };
            let existing = std::mem::replace(tree, placeholder);
            let children = if leading {
                vec![new_leaf, existing]
            } else {
                vec![existing, new_leaf]
            };
            *tree = ShellSlotTree::Split {
                dir,
                children,
                sizes: vec![SPLIT_TOTAL_PERCENT / SPLIT_CHILD_COUNT as f32; SPLIT_CHILD_COUNT],
            };
            Some(new_slot)
        }
        ShellSlotTree::Split { children, .. } => children
            .iter_mut()
            .find_map(|child| split_at(child, target, dir, leading, project_id)),
    }
}

/// The one collapse rule every removal shares: a split that loses one child is replaced by the
/// surviving child (so no single-child split can ever exist), and a split that loses both
/// disappears with them. `None` means the whole tree is gone.
fn prune(tree: ShellSlotTree, keep: &dyn Fn(&ShellSlotId, &ProjectId) -> bool) -> Option<ShellSlotTree> {
    match tree {
        ShellSlotTree::Leaf { slot_id, project_id } => keep(&slot_id, &project_id).then_some(ShellSlotTree::Leaf { slot_id, project_id }),
        ShellSlotTree::Split { dir, children, sizes } => {
            let mut kept_children = Vec::new();
            let mut kept_sizes = Vec::new();
            for (child, size) in children.into_iter().zip(sizes) {
                if let Some(pruned) = prune(child, keep) {
                    kept_children.push(pruned);
                    kept_sizes.push(size);
                }
            }
            match kept_children.len() {
                0 => None,
                1 => kept_children.pop(),
                _ => Some(ShellSlotTree::Split {
                    dir,
                    children: kept_children,
                    sizes: kept_sizes,
                }),
            }
        }
    }
}

pub fn remove_slot(tree: ShellSlotTree, slot_id: &ShellSlotId) -> Option<ShellSlotTree> {
    prune(tree, &|candidate, _| candidate != slot_id)
}

fn is_slot_leaf(tree: &ShellSlotTree, slot_id: &ShellSlotId) -> bool {
    matches!(tree, ShellSlotTree::Leaf { slot_id: candidate, .. } if candidate == slot_id)
}

/// Which slot should inherit focus when `slot_id` is pruned: the **previous sibling's last slot**
/// inside the split holding it, or, when `slot_id` is that split's first child, the **next sibling's
/// first slot**. The shell-slot twin of `layout::service::successor_leaf_after_prune`, and read
/// *before* the prune for the same reason — [`prune`] collapses the split around the removed slot,
/// taking with it the only record of which slot sat next to it.
///
/// Splits here are always binary ([`SPLIT_CHILD_COUNT`]), so "the previous or next sibling" is just
/// the other child; the descent into it keeps the on-screen adjacency when that child is itself a
/// split. `None` when `slot_id` is the tree's root leaf (nothing outlives it to inherit) or is not in
/// the tree at all — callers then fall back to the rest of the focus preference order.
pub fn successor_slot_after_prune(tree: &ShellSlotTree, slot_id: &ShellSlotId) -> Option<ShellSlotId> {
    let ShellSlotTree::Split { children, .. } = tree else {
        return None;
    };

    let Some(position) = children.iter().position(|child| is_slot_leaf(child, slot_id)) else {
        return children.iter().find_map(|child| successor_slot_after_prune(child, slot_id));
    };

    if position > 0 {
        return leaves(&children[position - 1]).pop().map(|(slot, _)| slot);
    }
    leaves(children.get(1)?).into_iter().next().map(|(slot, _)| slot)
}

pub fn remove_project(tree: ShellSlotTree, project_id: &ProjectId) -> Option<ShellSlotTree> {
    prune(tree, &|_, candidate| candidate != project_id)
}

/// Boot-time normalization half of contract §0.1 S-1: a leaf naming a project the session no longer
/// lists (a record forgotten while the app was closed, or a session file edited by hand) is dropped
/// and the tree collapsed around it, rather than rendering a slot whose project can never load.
pub fn retain_projects(tree: ShellSlotTree, open: &std::collections::HashSet<ProjectId>) -> Option<ShellSlotTree> {
    prune(tree, &|_, project_id| open.contains(project_id))
}

fn sizes_path_invalid(path: &[u32]) -> AppError {
    AppError::NotFound(format!("shell slot split not found at path: {path:?}"))
}

/// Writes one split node's child percentages. The node is addressed by its **child-index path from
/// the root** (`[]` is the root itself, `[0, 1]` the second child of the first child) because a
/// `Split` carries no id — see [`ShellSlotTree`]. `sizes` must have exactly as many entries as that
/// node has children, the same arity check `layout::service::resize` makes.
pub fn set_sizes(tree: &mut ShellSlotTree, path: &[u32], sizes: Vec<f32>) -> AppResult<()> {
    let mut node = tree;
    for index in path {
        let ShellSlotTree::Split { children, .. } = node else {
            return Err(sizes_path_invalid(path));
        };
        node = children.get_mut(*index as usize).ok_or_else(|| sizes_path_invalid(path))?;
    }

    let ShellSlotTree::Split {
        children, sizes: existing, ..
    } = node
    else {
        return Err(sizes_path_invalid(path));
    };
    if sizes.len() != children.len() {
        return Err(AppError::InvalidArgument(format!(
            "sizes length {} does not match children length {}",
            sizes.len(),
            children.len()
        )));
    }
    *existing = sizes;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn 프로젝트(label: &str) -> ProjectId {
        ProjectId(format!("prj-{label}"))
    }

    fn 슬롯_아이디(tree: &ShellSlotTree, project: &ProjectId) -> ShellSlotId {
        slot_of_project(tree, project).expect("슬롯이 있어야 한다")
    }

    /// 화면상 `[A | B | C]` — 슬롯 분할은 항상 이진이라 트리는 `Split(A, Split(B, C))` 가 된다.
    fn 삼분할_트리() -> (ShellSlotTree, Vec<ShellSlotId>) {
        let (a, b, c) = (프로젝트("a"), 프로젝트("b"), 프로젝트("c"));
        let mut tree = leaf(a.clone());
        let slot_a = 슬롯_아이디(&tree, &a);
        let slot_b = split_slot(&mut tree, &slot_a, ShellSlotEdge::Right, &b)
            .expect("첫 분할")
            .expect("b 슬롯");
        let slot_c = split_slot(&mut tree, &slot_b, ShellSlotEdge::Right, &c)
            .expect("둘째 분할")
            .expect("c 슬롯");
        (tree, vec![slot_a, slot_b, slot_c])
    }

    #[test]
    fn 네_방향_분할은_축과_순서가_각각_다르다() {
        for (edge, expected_dir, new_is_first) in [
            (ShellSlotEdge::Left, SplitDir::Horizontal, true),
            (ShellSlotEdge::Right, SplitDir::Horizontal, false),
            (ShellSlotEdge::Top, SplitDir::Vertical, true),
            (ShellSlotEdge::Bottom, SplitDir::Vertical, false),
        ] {
            let base = 프로젝트("base");
            let added = 프로젝트("added");
            let mut tree = leaf(base.clone());
            let target = 슬롯_아이디(&tree, &base);

            let new_slot = split_slot(&mut tree, &target, edge, &added)
                .expect("분할")
                .expect("대상 슬롯을 찾아야 한다");

            let ShellSlotTree::Split { dir, children, sizes } = &tree else {
                panic!("분할 결과는 split 이어야 한다");
            };
            assert_eq!(*dir, expected_dir);
            assert_eq!(children.len(), SPLIT_CHILD_COUNT);
            assert_eq!(sizes, &vec![50.0, 50.0]);

            let ordered = leaves(&tree);
            let first_is_new = ordered[0].0 == new_slot;
            assert_eq!(first_is_new, new_is_first, "{edge:?} 의 새 슬롯 위치가 다르다");
            assert_eq!(ordered.len(), 2);
        }
    }

    #[test]
    fn replace_는_트리_모양을_바꾸지_않고_프로젝트만_교체한다() {
        let base = 프로젝트("base");
        let other = 프로젝트("other");
        let mut tree = leaf(base.clone());
        let target = 슬롯_아이디(&tree, &base);

        assert!(set_slot_project(&mut tree, &target, other.clone()));

        assert_eq!(leaves(&tree), vec![(target.clone(), other)]);
        assert!(split_slot(&mut tree, &target, ShellSlotEdge::Replace, &base).is_err());
    }

    #[test]
    fn 없는_슬롯을_분할하면_none이고_트리는_그대로다() {
        let base = 프로젝트("base");
        let mut tree = leaf(base.clone());
        let before = tree.clone();

        let result = split_slot(
            &mut tree,
            &ShellSlotId("shellslot-missing".to_string()),
            ShellSlotEdge::Right,
            &base,
        )
        .expect("분할 호출");

        assert!(result.is_none());
        assert_eq!(tree, before);
    }

    #[test]
    fn 슬롯을_닫으면_형제가_부모_자리로_올라온다() {
        let base = 프로젝트("base");
        let added = 프로젝트("added");
        let mut tree = leaf(base.clone());
        let target = 슬롯_아이디(&tree, &base);
        let new_slot = split_slot(&mut tree, &target, ShellSlotEdge::Right, &added)
            .expect("분할")
            .expect("새 슬롯");

        let collapsed = remove_slot(tree, &new_slot).expect("마지막 슬롯이 남는다");

        assert_eq!(
            collapsed,
            ShellSlotTree::Leaf {
                slot_id: target,
                project_id: base
            }
        );
    }

    #[test]
    fn 중첩_분할에서_한_슬롯을_닫아도_나머지_구조는_유지된다() {
        let a = 프로젝트("a");
        let b = 프로젝트("b");
        let c = 프로젝트("c");
        let mut tree = leaf(a.clone());
        let slot_a = 슬롯_아이디(&tree, &a);
        let slot_b = split_slot(&mut tree, &slot_a, ShellSlotEdge::Right, &b)
            .expect("첫 분할")
            .expect("b 슬롯");
        let slot_c = split_slot(&mut tree, &slot_b, ShellSlotEdge::Bottom, &c)
            .expect("둘째 분할")
            .expect("c 슬롯");

        let pruned = remove_slot(tree, &slot_c).expect("두 슬롯이 남는다");

        assert_eq!(leaf_count(&pruned), 2);
        assert_eq!(
            leaves(&pruned),
            vec![(slot_a, a), (slot_b, b)],
            "c 가 빠지면 세로 분할이 사라지고 원래 가로 분할만 남아야 한다"
        );
    }

    /// `[A | B | C]` 는 `Split(A, Split(B, C))` 이므로, 세 번째 슬롯의 유일한 형제는 B 다.
    #[test]
    fn 삼분할의_마지막_슬롯은_바로_이웃을_승계자로_지목한다() {
        let (tree, slots) = 삼분할_트리();

        assert_eq!(successor_slot_after_prune(&tree, &slots[2]), Some(slots[1].clone()));
    }

    #[test]
    fn 첫_슬롯의_승계자는_다음_형제의_첫_슬롯이다() {
        let (tree, slots) = 삼분할_트리();

        assert_eq!(successor_slot_after_prune(&tree, &slots[0]), Some(slots[1].clone()));
    }

    /// 형제가 리프가 아니라 서브트리면 그 안의 마지막 슬롯까지 내려가야 화면상 이웃이 된다.
    /// `split_slot` 은 리프만 감싸므로 `Split(Split(A, B), C)` 형태는 직접 조립한다.
    #[test]
    fn 승계자는_이전_형제_서브트리의_마지막_슬롯이다() {
        let (a, b, c) = (프로젝트("a"), 프로젝트("b"), 프로젝트("c"));
        let mut left = leaf(a.clone());
        let slot_a = 슬롯_아이디(&left, &a);
        let slot_b = split_slot(&mut left, &slot_a, ShellSlotEdge::Right, &b)
            .expect("분할")
            .expect("b 슬롯");
        let right = leaf(c.clone());
        let slot_c = 슬롯_아이디(&right, &c);
        let tree = ShellSlotTree::Split {
            dir: SplitDir::Horizontal,
            children: vec![left, right],
            sizes: vec![50.0, 50.0],
        };

        assert_eq!(successor_slot_after_prune(&tree, &slot_c), Some(slot_b));
    }

    #[test]
    fn 루트_리프와_없는_슬롯에는_승계자가_없다() {
        let a = 프로젝트("a");
        let single = leaf(a.clone());
        let slot_a = 슬롯_아이디(&single, &a);
        let (tree, _) = 삼분할_트리();

        assert_eq!(successor_slot_after_prune(&single, &slot_a), None);
        assert_eq!(
            successor_slot_after_prune(&tree, &ShellSlotId("shellslot-missing".to_string())),
            None
        );
    }

    #[test]
    fn 마지막_슬롯을_제거하면_트리가_사라진다() {
        let base = 프로젝트("base");
        let tree = leaf(base.clone());
        let slot = 슬롯_아이디(&tree, &base);

        assert!(remove_slot(tree, &slot).is_none());
    }

    #[test]
    fn 프로젝트_제거는_그_프로젝트의_모든_리프를_지운다() {
        let a = 프로젝트("a");
        let b = 프로젝트("b");
        let mut tree = leaf(a.clone());
        let slot_a = 슬롯_아이디(&tree, &a);
        let slot_b = split_slot(&mut tree, &slot_a, ShellSlotEdge::Right, &b)
            .expect("분할")
            .expect("b 슬롯");

        let pruned = remove_project(tree, &a).expect("b 가 남는다");

        assert_eq!(leaves(&pruned), vec![(slot_b, b)]);
    }

    #[test]
    fn 세션에_없는_프로젝트_리프는_복원시_제거된다() {
        let a = 프로젝트("a");
        let gone = 프로젝트("gone");
        let mut tree = leaf(a.clone());
        let slot_a = 슬롯_아이디(&tree, &a);
        split_slot(&mut tree, &slot_a, ShellSlotEdge::Right, &gone)
            .expect("분할")
            .expect("사라질 슬롯");

        let open: std::collections::HashSet<ProjectId> = [a.clone()].into_iter().collect();
        let pruned = retain_projects(tree, &open).expect("a 가 남는다");

        assert_eq!(leaves(&pruned), vec![(slot_a, a)]);
    }

    #[test]
    fn 열린_프로젝트가_하나도_없으면_트리가_통째로_사라진다() {
        let a = 프로젝트("a");
        let tree = leaf(a);

        assert!(retain_projects(tree, &std::collections::HashSet::new()).is_none());
    }

    #[test]
    fn 크기는_자식_인덱스_경로로_지정한다() {
        let a = 프로젝트("a");
        let b = 프로젝트("b");
        let c = 프로젝트("c");
        let mut tree = leaf(a.clone());
        let slot_a = 슬롯_아이디(&tree, &a);
        let slot_b = split_slot(&mut tree, &slot_a, ShellSlotEdge::Right, &b)
            .expect("분할")
            .expect("b 슬롯");
        split_slot(&mut tree, &slot_b, ShellSlotEdge::Bottom, &c)
            .expect("둘째 분할")
            .expect("c 슬롯");

        set_sizes(&mut tree, &[], vec![30.0, 70.0]).expect("루트 크기");
        set_sizes(&mut tree, &[1], vec![20.0, 80.0]).expect("중첩 크기");

        let ShellSlotTree::Split { sizes, children, .. } = &tree else {
            panic!("루트는 split 이다");
        };
        assert_eq!(sizes, &vec![30.0, 70.0]);
        let ShellSlotTree::Split { sizes: nested, .. } = &children[1] else {
            panic!("두 번째 자식은 split 이다");
        };
        assert_eq!(nested, &vec![20.0, 80.0]);
    }

    #[test]
    fn 잘못된_크기_경로와_개수는_거부된다() {
        let a = 프로젝트("a");
        let b = 프로젝트("b");
        let mut tree = leaf(a.clone());
        let slot_a = 슬롯_아이디(&tree, &a);
        split_slot(&mut tree, &slot_a, ShellSlotEdge::Right, &b)
            .expect("분할")
            .expect("b 슬롯");

        assert!(set_sizes(&mut tree, &[5], vec![50.0, 50.0]).is_err(), "없는 자식 인덱스");
        assert!(set_sizes(&mut tree, &[0], vec![50.0, 50.0]).is_err(), "리프에는 크기가 없다");
        assert!(set_sizes(&mut tree, &[], vec![100.0]).is_err(), "자식 수와 개수가 달라야 거부된다");
    }
}
