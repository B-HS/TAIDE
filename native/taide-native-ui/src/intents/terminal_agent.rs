use taide_model::ids::{PaneId, ProjectId};

#[derive(Debug, Clone)]
pub enum Intent {
    NewTerminal { project: ProjectId, pane: PaneId },
}
