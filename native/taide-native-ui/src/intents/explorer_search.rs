use taide_model::ids::{ProjectId, ShellSlotId};

#[derive(Debug, Clone)]
pub enum Intent {
    RevealInExplorer {
        project: ProjectId,
        slot: ShellSlotId,
        path: String,
        rename: bool,
    },
    RevealInAuxiliaryExplorer {
        project: ProjectId,
        slot: u32,
        path: String,
        rename: bool,
    },
    ToggleAuxiliaryExplorer {
        project: ProjectId,
        slot: u32,
    },
    ShowExplorer {
        project: ProjectId,
        slot: ShellSlotId,
    },
    ShowAuxiliaryExplorer {
        project: ProjectId,
        slot: u32,
    },
}
