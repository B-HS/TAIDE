use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::TabKind;

/// A collapsible section of the source control panel, in render order (`git-section.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Merge,
    Staged,
    Changes,
    Stashes,
    Graph,
}

/// One change row of the source control list: its group and repository-relative path
/// (`change-row-navigation.ts` row ids carry the group because one path can be in two groups).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RowKey {
    pub section: Section,
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum Intent {
    ShowView {
        project: ProjectId,
        slot: ShellSlotId,
    },
    ShowAuxiliaryView {
        project: ProjectId,
        slot: u32,
    },
    Shown {
        project: ProjectId,
        slot: ShellSlotId,
    },
    ToggleSection {
        project: ProjectId,
        slot: ShellSlotId,
        section: Section,
    },
    Init(ProjectId),
    Stage {
        project: ProjectId,
        paths: Vec<String>,
    },
    Unstage {
        project: ProjectId,
        paths: Vec<String>,
    },
    RequestDiscard {
        project: ProjectId,
        slot: ShellSlotId,
        paths: Vec<String>,
    },
    ResolveDiscard {
        project: ProjectId,
        slot: ShellSlotId,
        confirmed: bool,
    },
    OpenFile {
        project: ProjectId,
        path: String,
    },
    OpenChanges {
        project: ProjectId,
        path: String,
        staged: bool,
        before_path: Option<String>,
    },
    Select {
        project: ProjectId,
        slot: ShellSlotId,
        selected: Vec<RowKey>,
        anchor: Option<RowKey>,
    },
    RowMenu {
        project: ProjectId,
        slot: ShellSlotId,
        target: RowKey,
    },
    CheckoutBranch {
        project: ProjectId,
        name: String,
    },
    CheckoutRemoteBranch {
        project: ProjectId,
        remote_ref: String,
    },
    CreateBranch {
        project: ProjectId,
        name: String,
    },
    Sync(ProjectId),
    StashPush(ProjectId),
    StashApply {
        project: ProjectId,
        index: u32,
    },
    StashDrop {
        project: ProjectId,
        index: u32,
    },
    SetCommitMessage {
        project: ProjectId,
        slot: ShellSlotId,
        message: String,
    },
    RequestCommit {
        project: ProjectId,
        slot: ShellSlotId,
    },
    ResolveStageAll {
        project: ProjectId,
        slot: ShellSlotId,
        confirmed: bool,
    },
    GenerateCommitMessage {
        project: ProjectId,
        slot: ShellSlotId,
    },
    /// A diff tab drew this frame (its `DiffPane` is mounted), keeping its reads observed.
    DiffShown {
        project: ProjectId,
        tab: TabId,
        path: String,
        staged: bool,
        compare_with: Option<String>,
        before_path: Option<String>,
        rev: Option<String>,
        parent_rev: Option<String>,
    },
    ToggleDiffSideBySide(TabId),
    /// A diff tab's gutter lane press: stages the working tree hunk, or unstages it on a staged tab.
    StageHunk {
        project: ProjectId,
        path: String,
        start: u32,
        end: u32,
        staged: bool,
    },
    /// The file tab menu's "Open Changes" (`pane-tab-bar.tsx`): a preview diff tab in the
    /// invoking window's focused pane.
    OpenTabChanges {
        project: ProjectId,
        path: String,
        title: String,
        window: Option<u32>,
    },
    /// A graph row click or Enter/Space selects its commit, or clears the selection when that commit
    /// is already selected; `None` (the detail panel's close button) clears it.
    SelectCommit {
        project: ProjectId,
        slot: ShellSlotId,
        commit: Option<String>,
    },
    GraphMenu {
        project: ProjectId,
        slot: ShellSlotId,
        commit: String,
    },
    /// The graph menu's Revert: a conflicted outcome opens the first conflicted file.
    Revert {
        project: ProjectId,
        rev: String,
    },
    /// `git.revertHead`: reports the outcome without opening a conflicted file.
    RevertHead(ProjectId),
    RequestCreateTag {
        project: ProjectId,
        slot: ShellSlotId,
        target: String,
    },
    /// `git.createTagOnHead` in the main window: the focused slot shows the Git view and the tag
    /// dialog for `HEAD`, whether or not the graph is expanded.
    CreateTagOnHead {
        project: ProjectId,
        slot: ShellSlotId,
    },
    CreateAuxiliaryTagOnHead {
        project: ProjectId,
        slot: u32,
    },
    CloseTagDialog {
        project: ProjectId,
        slot: ShellSlotId,
    },
    CreateTag {
        project: ProjectId,
        slot: ShellSlotId,
        name: String,
        message: String,
    },
    DeleteTag {
        project: ProjectId,
        name: String,
    },
    /// A finished graph separator drag: the graph pane's new height, or `None` when it was dragged
    /// below the collapse threshold.
    ResizeGraph {
        project: ProjectId,
        slot: ShellSlotId,
        height: Option<f32>,
    },
    /// A commit detail file: a preview commit diff tab.
    OpenCommitFile {
        project: ProjectId,
        kind: TabKind,
        title: String,
    },
    /// A Claude diff tab drew this frame with documents at these (original, modified) revisions.
    ClaudeDiffShown {
        project: ProjectId,
        tab: TabId,
        request_id: String,
        path: String,
        revisions: (u64, u64),
    },
    /// Accept (with the modified side's text) or Reject on a Claude diff tab.
    ResolveClaudeDiff {
        tab: TabId,
        request_id: String,
        content: Option<String>,
    },
    /// Opens the invoking window's file history dialog for `path`.
    OpenFileHistory {
        project: ProjectId,
        path: String,
    },
    /// A file editor drew this frame with its 1-based cursor line, keeping its gutter, blame and
    /// conflict reads observed.
    EditorShown {
        project: ProjectId,
        pane: PaneId,
        tab: TabId,
        path: String,
        cursor_line: u32,
    },
    /// A press on a file editor's gutter lane at a 1-based line: a conflict start opens the
    /// resolution dialog, a hunk line the discard dialog.
    EditorLanePress {
        pane: PaneId,
        line: u32,
    },
    ResolveHunkDiscard {
        pane: PaneId,
        confirmed: bool,
    },
    ResolveConflict {
        pane: PaneId,
        action: ConflictAction,
    },
    CloseConflictCompare {
        pane: PaneId,
    },
    /// `git.toggleBlame` on the pane's file editor.
    ToggleBlame {
        pane: PaneId,
        tab: TabId,
    },
    /// `taide.gitStageSelection` on the pane's file editor.
    StageSelection {
        pane: PaneId,
        tab: TabId,
    },
}

/// A choice in the conflict resolution dialog (`ConflictResolutionDialog`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictAction {
    Cancel,
    Compare,
    AcceptBoth,
    AcceptIncoming,
    AcceptCurrent,
}
