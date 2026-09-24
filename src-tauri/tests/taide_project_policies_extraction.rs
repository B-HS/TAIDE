use std::any::TypeId;
use std::collections::HashSet;

use taide_lib::domain::project::{groups as legacy_groups, shell_slots as legacy_shell_slots};
use taide_model::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use taide_model::project::{ProjectGroup, ShellSlotTree};

#[test]
fn 프로젝트_그룹과_슬롯_정책은_독립_crate와_같다() {
    assert_eq!(
        TypeId::of::<taide_project::groups::GroupOpenStep>(),
        TypeId::of::<legacy_groups::GroupOpenStep>()
    );

    let extracted_find: for<'a> fn(&'a [ProjectGroup], &ProjectGroupId) -> Option<&'a ProjectGroup> = taide_project::groups::find;
    let legacy_find: for<'a> fn(&'a [ProjectGroup], &ProjectGroupId) -> Option<&'a ProjectGroup> = legacy_groups::find;
    assert!(std::ptr::fn_addr_eq(extracted_find, legacy_find));

    let extracted_leaf: fn(ProjectId) -> ShellSlotTree = taide_project::shell_slots::leaf;
    let legacy_leaf: fn(ProjectId) -> ShellSlotTree = legacy_shell_slots::leaf;
    assert!(std::ptr::fn_addr_eq(extracted_leaf, legacy_leaf));

    let members = vec![ProjectId::from("project-a".to_string())];
    let plan = legacy_groups::plan_open(&members, &HashSet::new(), |_| Some("/repo".to_string()));
    assert_eq!(plan.steps.len(), 1);
    assert!(plan.steps[0].activate);

    let tree = legacy_leaf(members[0].clone());
    let slots: Vec<(ShellSlotId, ProjectId)> = legacy_shell_slots::leaves(&tree);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].1, members[0]);
}
