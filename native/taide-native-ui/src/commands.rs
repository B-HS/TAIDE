use taide_model::error::{AppError, AppResult};
use taide_model::ids::{PaneId, ProjectGroupId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::DropEdge;
#[cfg(feature = "native-host")]
use taide_model::layout::ShellViewPatch;
use taide_model::project::WindowChromePatch;
#[cfg(feature = "native-host")]
use taide_runtime::{AppState, EventSink, layout_actions, project_actions};

use crate::command_registry::{DocumentEdit, PaletteEntry};

#[derive(Debug, Clone)]
pub enum ShellMutation {
    ActivateProject(ProjectId),
    FocusSlot(ShellSlotId),
    CloseSlot(ShellSlotId),
    SetGroupCollapsed {
        group: ProjectGroupId,
        collapsed: bool,
    },
    ResizeSlots {
        path: Vec<u32>,
        sizes: Vec<f32>,
    },
    SetWindowChrome(WindowChromePatch),
    SetSidebarCollapsed {
        project: ProjectId,
        collapsed: bool,
    },
    ActivateTab(TabId),
    ReopenClosed(ProjectId),
    FocusPane(PaneId),
    MoveTab {
        tab: TabId,
        pane: PaneId,
        index: u32,
    },
    PinTab {
        tab: TabId,
        pinned: bool,
    },
    KeepTab(TabId),
    SplitTab {
        pane: PaneId,
        edge: DropEdge,
        tab: TabId,
    },
    ResizePane {
        pane: PaneId,
        sizes: Vec<f32>,
    },
}

#[derive(Debug, Clone)]
pub enum ShellIntent {
    Mutate(ShellMutation),
    OpenFolder,
    OpenFile { project: ProjectId, pane: PaneId },
    NewUntitled { project: ProjectId, pane: PaneId },
    NewTerminal { project: ProjectId, pane: PaneId },
    OpenSettings,
    OpenSettingsFile,
    OpenKeybindings,
    OpenPalette(PaletteEntry),
    ShowOpenProjectNotice,
    ChangeEditorFontSize { increase: bool },
    RequestCloseTab(TabId),
    RequestCloseTabs(Vec<TabId>),
    RequestSaveTab(TabId),
    EditDocument { tab: TabId, edit: DocumentEdit },
}

#[cfg(feature = "native-host")]
pub async fn dispatch(
    sink: &dyn EventSink,
    state: &AppState,
    command: ShellMutation,
) -> AppResult<()> {
    match command {
        ShellMutation::ActivateProject(project) => {
            project_actions::project_activate(sink, state, project).await?
        }
        ShellMutation::FocusSlot(slot) => {
            project_actions::session_focus_shell_slot(sink, state, slot).await?
        }
        ShellMutation::CloseSlot(slot) => {
            project_actions::shell_slot_close(sink, state, slot).await?
        }
        ShellMutation::SetGroupCollapsed { group, collapsed } => {
            project_actions::project_group_set_collapsed(sink, state, group, collapsed).await?
        }
        ShellMutation::ResizeSlots { path, sizes } => {
            project_actions::session_set_shell_slot_sizes(sink, state, path, sizes).await?
        }
        ShellMutation::SetWindowChrome(patch) => {
            project_actions::session_set_window_chrome(sink, state, patch).await?;
        }
        ShellMutation::SetSidebarCollapsed { project, collapsed } => {
            layout_actions::layout_set_shell_view(
                sink,
                state,
                project,
                ShellViewPatch {
                    zen: None,
                    sidebar_collapsed: Some(collapsed),
                },
            )
            .await?;
        }
        ShellMutation::ActivateTab(tab) => {
            layout_actions::layout_activate_tab(sink, state, tab).await?;
        }
        ShellMutation::ReopenClosed(project) => {
            layout_actions::layout_reopen_closed(sink, state, project).await?;
        }
        ShellMutation::FocusPane(pane) => {
            layout_actions::layout_focus_pane(sink, state, pane).await?;
        }
        ShellMutation::MoveTab { tab, pane, index } => {
            layout_actions::layout_move_tab(sink, state, tab, pane, index).await?;
        }
        ShellMutation::PinTab { tab, pinned } => {
            layout_actions::layout_pin_tab(sink, state, tab, pinned).await?;
        }
        ShellMutation::KeepTab(tab) => {
            layout_actions::layout_set_preview(sink, state, tab, false).await?;
        }
        ShellMutation::SplitTab { pane, edge, tab } => {
            layout_actions::layout_split(sink, state, pane, edge, tab).await?;
        }
        ShellMutation::ResizePane { pane, sizes } => {
            layout_actions::layout_resize(sink, state, pane, sizes).await?;
        }
    }
    Ok(())
}

pub fn request_close_tab(tab: &taide_model::layout::Tab) -> AppResult<ShellIntent> {
    if tab.pinned {
        return Err(AppError::Forbidden("tab is pinned".into()));
    }
    Ok(ShellIntent::RequestCloseTab(tab.id.clone()))
}
