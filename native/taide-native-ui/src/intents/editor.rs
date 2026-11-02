use taide_model::ids::{PaneId, ProjectId, TabId};

use crate::command_registry::{DocumentEdit, FoldCommand};

#[derive(Debug, Clone)]
pub enum Intent {
    NewUntitled {
        project: ProjectId,
        pane: PaneId,
    },
    #[cfg(feature = "native-host")]
    ChooseIndentation {
        tab: TabId,
        command: crate::command_registry::IndentationCommand,
    },
    #[cfg(feature = "native-host")]
    FormatEditor {
        tab: TabId,
        command: taide_native_editor::formatting::Command,
    },
    #[cfg(feature = "native-host")]
    RenameEditor {
        tab: TabId,
    },
    #[cfg(feature = "native-host")]
    CodeActions {
        tab: TabId,
        command: crate::editor_code_actions::Command,
    },
    #[cfg(feature = "native-host")]
    ToggleEditorStickyScroll,
    RequestSaveTab(TabId),
    EditDocument {
        tab: TabId,
        edit: DocumentEdit,
    },
    FoldDocument {
        tab: TabId,
        command: FoldCommand,
    },
}
