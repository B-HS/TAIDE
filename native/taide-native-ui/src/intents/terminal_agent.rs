use taide_model::ids::{PaneId, ProjectId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliAction {
    ConnectExternalEditor,
    Install,
    Uninstall,
}

#[derive(Debug, Clone)]
pub enum Intent {
    NewTerminal {
        project: ProjectId,
        pane: PaneId,
    },
    #[cfg(feature = "native-host")]
    Cli(CliAction),
}
