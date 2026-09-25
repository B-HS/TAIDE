use crate::ids::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    LayoutChanged { project_id: ProjectId, revision: u32 },
}
