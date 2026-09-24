use std::collections::HashSet;

use taide_model::ids::{ProjectGroupId, ProjectId};
use taide_model::project::ProjectGroup;

/// One member `project_group_open` decided to open, with the root its record named and whether it
/// is the member that takes focus. Not an IPC type — the command turns the plan into
/// `ProjectGroupOpenResult` once each step has actually run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupOpenStep {
    pub project_id: ProjectId,
    pub root: String,
    pub activate: bool,
}

/// Why a member was left out of the queue. The two are logged differently: an already-open member
/// is the ordinary case (the user asked for the group and part of it was already up), while an
/// unavailable one means the record or its root vanished and the user's group now names something
/// that is not there — the case contract §0.1 S-7 asks to `log::warn` about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSkipReason {
    AlreadyOpen,
    Unavailable,
}

/// The whole decision `project_group_open` makes before it opens anything: which members to open in
/// which order, which one activates, and which members are passed over and why. Separated from the
/// command so the rule is unit-testable without an `AppHandle` — the same split
/// `commands::projects_pending_watcher_restore` uses for the boot watcher queue.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GroupOpenPlan {
    pub steps: Vec<GroupOpenStep>,
    pub skipped: Vec<(ProjectId, GroupSkipReason)>,
}

pub fn find<'a>(groups: &'a [ProjectGroup], group_id: &ProjectGroupId) -> Option<&'a ProjectGroup> {
    groups.iter().find(|group| &group.id == group_id)
}

pub fn find_mut<'a>(groups: &'a mut [ProjectGroup], group_id: &ProjectGroupId) -> Option<&'a mut ProjectGroup> {
    groups.iter_mut().find(|group| &group.id == group_id)
}

/// Drops repeats while keeping the caller's order — the order is the user's own arrangement (it
/// decides the sidebar listing and the open queue), so this cannot sort or use a set.
pub fn dedupe_members(members: Vec<ProjectId>) -> Vec<ProjectId> {
    let mut seen = HashSet::with_capacity(members.len());
    members.into_iter().filter(|project_id| seen.insert(project_id.clone())).collect()
}

/// Enforces the one-group-per-project rule ([`ProjectGroup`]): every group other than `owner` gives
/// up the listed members. Answers whether anything actually moved, so the caller can tell a real
/// change from a no-op re-assignment.
pub fn claim_members(groups: &mut [ProjectGroup], owner: &ProjectGroupId, members: &[ProjectId]) -> bool {
    let claimed: HashSet<&ProjectId> = members.iter().collect();
    let mut moved = false;
    for group in groups.iter_mut() {
        if &group.id == owner {
            continue;
        }
        let before = group.members.len();
        group.members.retain(|project_id| !claimed.contains(project_id));
        moved |= group.members.len() != before;
    }
    moved
}

/// Removes every forgotten project from every group — the membership half of
/// `service::forget_recent_projects` (contract §0.1 S-7). Answers whether any group changed so the
/// caller only rewrites `session.json` when it must.
pub fn forget_members(groups: &mut [ProjectGroup], forgotten: &HashSet<ProjectId>) -> bool {
    let mut changed = false;
    for group in groups.iter_mut() {
        let before = group.members.len();
        group.members.retain(|project_id| !forgotten.contains(project_id));
        changed |= group.members.len() != before;
    }
    changed
}

/// Reorders `groups` to follow `ids`, appending anything `ids` did not mention in its existing
/// relative order — the same tolerant shape `service::reorder_projects` uses, so a stale client
/// list can never drop a group.
pub fn reorder(groups: Vec<ProjectGroup>, ids: &[ProjectGroupId]) -> Vec<ProjectGroup> {
    let mut remaining = groups;
    let mut reordered = Vec::with_capacity(remaining.len());

    for id in ids {
        if let Some(position) = remaining.iter().position(|group| &group.id == id) {
            reordered.push(remaining.remove(position));
        }
    }
    reordered.extend(remaining);
    reordered
}

/// Turns a group's member list into the open queue. `root_of` answers a member's persisted root, or
/// `None` when the record is gone or its root is no longer a directory — those members are skipped
/// with a warning by the caller rather than failing the whole group (contract §0.1 S-7), as are
/// members that are already open. Exactly one step carries `activate`: the first member this call
/// will actually open, so the user lands on something they were not already looking at, and the
/// remaining members join the session without stealing focus (contract §0.1 S-2). Whether that
/// member really gets the focus is not settled here — an open can still fail, and
/// `commands::run_group_open_plan` hands the mark to the next step that succeeds (contract §3 S-9).
pub fn plan_open(
    members: &[ProjectId],
    open_project_ids: &HashSet<ProjectId>,
    root_of: impl Fn(&ProjectId) -> Option<String>,
) -> GroupOpenPlan {
    let mut plan = GroupOpenPlan::default();
    for project_id in members {
        if open_project_ids.contains(project_id) {
            plan.skipped.push((project_id.clone(), GroupSkipReason::AlreadyOpen));
            continue;
        }
        let Some(root) = root_of(project_id) else {
            plan.skipped.push((project_id.clone(), GroupSkipReason::Unavailable));
            continue;
        };
        plan.steps.push(GroupOpenStep {
            project_id: project_id.clone(),
            root,
            activate: plan.steps.is_empty(),
        });
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(id: &str) -> ProjectId {
        ProjectId::from(id.to_string())
    }

    fn group(id: &str, members: &[&str]) -> ProjectGroup {
        ProjectGroup {
            id: ProjectGroupId::from(id.to_string()),
            name: id.to_string(),
            color: None,
            members: members.iter().map(|member| project(member)).collect(),
            collapsed: false,
        }
    }

    #[test]
    fn 멤버_중복은_앞_순서를_남기고_제거된다() {
        let deduped = dedupe_members(vec![project("prj-b"), project("prj-a"), project("prj-b")]);

        assert_eq!(deduped, vec![project("prj-b"), project("prj-a")]);
    }

    #[test]
    fn 멤버를_차지하면_다른_그룹에서_빠진다() {
        let mut groups = vec![group("group-old", &["prj-a", "prj-b"]), group("group-new", &["prj-a"])];

        let moved = claim_members(&mut groups, &ProjectGroupId::from("group-new".to_string()), &[project("prj-a")]);

        assert!(moved, "다른 그룹에서 멤버가 빠졌으면 변경으로 보고해야 합니다");
        assert_eq!(groups[0].members, vec![project("prj-b")]);
        assert_eq!(groups[1].members, vec![project("prj-a")]);
    }

    #[test]
    fn 소유_그룹_자신의_멤버는_차지에서_건드리지_않는다() {
        let mut groups = vec![group("group-one", &["prj-a"])];

        let moved = claim_members(&mut groups, &ProjectGroupId::from("group-one".to_string()), &[project("prj-a")]);

        assert!(!moved);
        assert_eq!(groups[0].members, vec![project("prj-a")]);
    }

    #[test]
    fn forget_된_프로젝트는_모든_그룹의_멤버에서_사라진다() {
        let mut groups = vec![group("group-one", &["prj-a", "prj-b"]), group("group-two", &["prj-b"])];
        let forgotten: HashSet<ProjectId> = [project("prj-b")].into_iter().collect();

        let changed = forget_members(&mut groups, &forgotten);

        assert!(changed);
        assert_eq!(groups[0].members, vec![project("prj-a")]);
        assert!(groups[1].members.is_empty());
        assert!(
            !forget_members(&mut groups, &forgotten),
            "지울 것이 없으면 변경 없음으로 보고해 세션 재저장을 막아야 합니다"
        );
    }

    #[test]
    fn 재정렬에서_빠진_그룹은_뒤에_원래_순서로_붙는다() {
        let groups = vec![group("group-a", &[]), group("group-b", &[]), group("group-c", &[])];

        let reordered = reorder(
            groups,
            &[
                ProjectGroupId::from("group-c".to_string()),
                ProjectGroupId::from("group-a".to_string()),
            ],
        );

        assert_eq!(
            reordered.iter().map(|group| group.id.to_string()).collect::<Vec<_>>(),
            vec!["group-c", "group-a", "group-b"]
        );
    }

    #[test]
    fn 열기_계획은_멤버_순서를_지키고_첫_열기만_활성화한다() {
        let members = vec![project("prj-a"), project("prj-b"), project("prj-c")];

        let plan = plan_open(&members, &HashSet::new(), |project_id| Some(format!("/repo/{project_id}")));

        assert_eq!(
            plan.steps.iter().map(|step| step.project_id.clone()).collect::<Vec<_>>(),
            members,
            "멤버 순서 그대로 열어야 합니다"
        );
        assert_eq!(
            plan.steps.iter().map(|step| step.activate).collect::<Vec<_>>(),
            vec![true, false, false],
            "첫 멤버만 활성화하고 나머지는 포커스를 가로채지 않아야 합니다"
        );
        assert!(plan.skipped.is_empty());
    }

    #[test]
    fn 이미_열린_멤버와_결손_멤버는_건너뛰고_그다음이_활성화된다() {
        let members = vec![project("prj-open"), project("prj-gone"), project("prj-next")];
        let open: HashSet<ProjectId> = [project("prj-open")].into_iter().collect();

        let plan = plan_open(&members, &open, |project_id| {
            (project_id != &project("prj-gone")).then(|| format!("/repo/{project_id}"))
        });

        assert_eq!(
            plan.steps,
            vec![GroupOpenStep {
                project_id: project("prj-next"),
                root: "/repo/prj-next".to_string(),
                activate: true,
            }],
            "건너뛴 멤버 뒤의 첫 열기가 활성화를 이어받아야 합니다"
        );
        assert_eq!(
            plan.skipped,
            vec![
                (project("prj-open"), GroupSkipReason::AlreadyOpen),
                (project("prj-gone"), GroupSkipReason::Unavailable),
            ],
            "건너뛴 이유가 로그 등급을 가르므로 함께 보고돼야 합니다"
        );
    }

    #[test]
    fn 열_수_있는_멤버가_없으면_계획은_비고_전부_스킵된다() {
        let members = vec![project("prj-a")];
        let open: HashSet<ProjectId> = [project("prj-a")].into_iter().collect();

        let plan = plan_open(&members, &open, |_| None);

        assert!(plan.steps.is_empty());
        assert_eq!(plan.skipped, vec![(project("prj-a"), GroupSkipReason::AlreadyOpen)]);
    }

    #[test]
    fn 그룹_조회는_아이디로_찾고_없으면_none_이다() {
        let mut groups = vec![group("group-one", &[])];

        assert!(find(&groups, &ProjectGroupId::from("group-one".to_string())).is_some());
        assert!(find(&groups, &ProjectGroupId::from("group-missing".to_string())).is_none());
        assert!(find_mut(&mut groups, &ProjectGroupId::from("group-missing".to_string())).is_none());
    }
}
