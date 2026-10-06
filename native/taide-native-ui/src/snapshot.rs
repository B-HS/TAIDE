use std::collections::HashMap;

use taide_model::ids::{PaneId, ProjectGroupId, ProjectId, ShellSlotId};
use taide_model::layout::{PaneNode, ProjectLayout, Tab, find_leaf};
use taide_model::project::{ProjectGroup, ProjectRef, SessionShellState, ShellSlotTree};
#[cfg(feature = "native-host")]
use taide_runtime::AppState;

#[derive(Clone)]
pub struct ShellSnapshot {
    pub projects: Vec<ProjectRef>,
    pub groups: Vec<ProjectGroup>,
    pub shell: SessionShellState,
    pub layouts: HashMap<ProjectId, ProjectLayout>,
    pub hide_status_in_zen: bool,
    pub resizer_thickness: f32,
    pub welcome_on_empty_editor: bool,
}

impl ShellSnapshot {
    #[cfg(feature = "native-host")]
    pub async fn read(state: &AppState) -> Self {
        let _guard = state.begin_mutation().await;
        let session = state.session.read();
        let settings = state.settings.read();
        Self {
            projects: session.projects.clone(),
            groups: session.groups.clone(),
            shell: taide_project::service::shell_state(&session),
            layouts: state.layouts.read().clone(),
            hide_status_in_zen: settings.zen_hide_status_bar,
            resizer_thickness: settings.resizer_thickness as f32,
            welcome_on_empty_editor: settings.welcome_on_empty_editor,
        }
    }

    pub fn project(&self, id: &ProjectId) -> Option<&ProjectRef> {
        self.projects.iter().find(|project| &project.id == id)
    }

    pub fn focused_project(&self) -> Option<&ProjectId> {
        let tree = self.shell.tree.as_ref()?;
        let focused = self.shell.focused.as_ref()?;
        slot_project(tree, focused)
    }

    pub fn focused_tab(&self) -> Option<&Tab> {
        let layout = self.layouts.get(self.focused_project()?)?;
        active_tab(&layout.root, &layout.focused_pane)
    }
}

pub struct ProjectGroupSections<'a> {
    pub sections: Vec<(&'a ProjectGroup, Vec<&'a ProjectRef>)>,
    pub ungrouped: Vec<&'a ProjectRef>,
}

pub fn project_group_sections<'a>(
    projects: &'a [ProjectRef],
    groups: &'a [ProjectGroup],
) -> ProjectGroupSections<'a> {
    let owners: HashMap<&ProjectId, &ProjectGroupId> = groups
        .iter()
        .flat_map(|group| group.members.iter().map(move |member| (member, &group.id)))
        .collect();
    ProjectGroupSections {
        sections: groups
            .iter()
            .map(|group| {
                let members = projects
                    .iter()
                    .filter(|project| owners.get(&project.id) == Some(&&group.id))
                    .collect();
                (group, members)
            })
            .collect(),
        ungrouped: projects
            .iter()
            .filter(|project| !owners.contains_key(&project.id))
            .collect(),
    }
}

pub fn slot_project<'a>(tree: &'a ShellSlotTree, target: &ShellSlotId) -> Option<&'a ProjectId> {
    match tree {
        ShellSlotTree::Leaf {
            slot_id,
            project_id,
        } => (slot_id == target).then_some(project_id),
        ShellSlotTree::Split { children, .. } => children
            .iter()
            .find_map(|child| slot_project(child, target)),
    }
}

pub fn slot_count(tree: &ShellSlotTree) -> usize {
    match tree {
        ShellSlotTree::Leaf { .. } => 1,
        ShellSlotTree::Split { children, .. } => children.iter().map(slot_count).sum(),
    }
}

pub fn active_tab<'a>(root: &'a PaneNode, focused: &PaneId) -> Option<&'a Tab> {
    let PaneNode::Leaf { tabs, active, .. } = find_leaf(root, focused)? else {
        return None;
    };
    tabs.iter().find(|tab| Some(&tab.id) == active.as_ref())
}
