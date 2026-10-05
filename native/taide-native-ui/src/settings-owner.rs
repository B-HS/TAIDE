use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{PaneNode, ProjectLayout, TabKind, find_leaf};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Owner {
    pub project: ProjectId,
    pub pane: PaneId,
    pub tab: TabId,
}

impl Owner {
    pub fn is_active_in(&self, layout: &ProjectLayout) -> bool {
        std::iter::once(&layout.root)
            .chain(layout.auxiliary_windows.iter().map(|window| &window.root))
            .any(|root| {
                matches!(find_leaf(root, &self.pane),
                    Some(PaneNode::Leaf { tabs, active, .. })
                    if active.as_ref() == Some(&self.tab)
                    && tabs.iter().any(|tab| tab.id == self.tab && matches!(tab.kind, TabKind::Settings)))
            })
    }

    #[cfg(feature = "native-host")]
    pub fn is_active(&self, state: &taide_runtime::AppState) -> bool {
        state
            .layouts
            .read()
            .get(&self.project)
            .is_some_and(|layout| self.is_active_in(layout))
    }
}
