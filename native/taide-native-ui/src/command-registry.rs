use std::collections::HashSet;
use std::sync::OnceLock;

use serde_json::Value;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::locale::ResolvedLocale;
use taide_native_editor::cursor_commands::CursorCommand;
pub use taide_native_editor::folding::FoldCommand;
use taide_native_editor::line_commands::{LineCommand, TextCase};

const CATALOG: &str = include_str!("keybinding-commands.json");
const EDITOR_ACTION_PREFIX: &str = "monaco.";
const MAC_PLATFORM: &str = "mac";
pub const EDITOR_FIND_AVAILABLE: bool = cfg!(feature = "native-host");
const EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE: [&str; 21] = [
    "editor.action.goToImplementation",
    "editor.action.goToLocation",
    "editor.action.goToReferences",
    "editor.action.goToTypeDefinition",
    "editor.action.inlineSuggest.toggleAlwaysShowToolbar",
    "editor.action.peekDeclaration",
    "editor.action.peekDefinition",
    "editor.action.peekImplementation",
    "editor.action.peekTypeDefinition",
    "editor.action.quickFix",
    "editor.action.referenceSearch.trigger",
    "editor.action.revealDeclaration",
    "editor.action.revealDefinition",
    "editor.action.revealDefinitionAside",
    "editor.action.showOrFocusStandaloneColorPicker",
    "editor.action.toggleStickyScroll",
    "editor.action.toggleTabFocusMode",
    "editor.action.unicodeHighlight.disableHighlightingOfAmbiguousCharacters",
    "editor.action.unicodeHighlight.disableHighlightingOfInvisibleCharacters",
    "editor.action.unicodeHighlight.disableHighlightingOfNonBasicAsciiCharacters",
    "editor.action.unicodeHighlight.showExcludeOptions",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupTarget {
    Direction(Direction),
    Position(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabCycle {
    Next,
    Previous,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentEdit {
    DeleteAllLeft,
    OutdentLines,
    Line(LineCommand),
    Cursor(CursorCommand),
    #[cfg(feature = "native-host")]
    Find(crate::editor_find::FindCommand),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteEntry {
    Files,
    Commands,
    WorkspaceSymbols,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Run {
    OpenPalette(PaletteEntry),
    OpenSettingsTab,
    OpenSettingsFile,
    OpenKeybindingsEditor,
    OpenTerminalTab,
    ReopenClosedTab,
    CloseTab,
    ToggleSidebar,
    Split,
    CycleTab(TabCycle),
    SaveActiveTab,
    ToggleTerminal,
    ToggleZenMode,
    FocusGroup(GroupTarget),
    MoveTabToGroup(Direction),
    CloseAllTabs,
    ChangeEditorFontSize {
        increase: bool,
    },
    #[cfg(feature = "native-host")]
    ToggleEditorStickyScroll,
    EditDocument(DocumentEdit),
    FoldDocument(FoldCommand),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActiveEditor {
    pub is_read_only: bool,
    pub has_folding: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Execution {
    Native(Run),
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Enablement {
    Always,
    Never,
    WebviewDiagnostics,
    MainWindow,
    AuxiliaryWindow,
    ActiveProject,
    ActiveEditor,
    SupportedEditorAction,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WindowKind {
    #[default]
    Main,
    Auxiliary,
}

#[derive(Clone, Debug, Default)]
pub struct CommandContext {
    pub active_project: Option<ProjectId>,
    pub focused_shell_slot: Option<ShellSlotId>,
    pub active_editor_actions: Option<HashSet<String>>,
    pub window: WindowKind,
}

#[derive(Clone, Debug)]
pub struct Command {
    pub id: String,
    pub title_key: String,
    pub category_key: Option<String>,
    pub keymap_id: Option<String>,
    pub title_default_value: Option<String>,
    pub default_binding_label: Option<String>,
    pub is_mac_only: bool,
    pub execution: Execution,
    pub enablement: Enablement,
}

impl Command {
    fn parse(value: &Value) -> AppResult<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let required = |key: &str| {
            text(key)
                .ok_or_else(|| AppError::Internal(format!("native command metadata missing {key}")))
        };
        let id = required("id")?;
        let keymap_id = text("keymapId");
        Ok(Self {
            title_key: required("titleKey")?,
            category_key: text("categoryKey"),
            title_default_value: text("titleDefaultValue"),
            default_binding_label: text("defaultBindingLabel"),
            is_mac_only: value.get("platform").and_then(Value::as_str) == Some(MAC_PLATFORM),
            execution: execution(&id, keymap_id.as_deref()),
            enablement: enablement(&id),
            keymap_id,
            id,
        })
    }

    pub fn editor_action_id(&self) -> Option<&str> {
        self.id.strip_prefix(EDITOR_ACTION_PREFIX)
    }

    pub fn runs_via_command(&self) -> bool {
        self.keymap_id.is_none() && self.editor_action_id().is_none()
    }

    pub fn is_registered(&self, is_mac: bool) -> bool {
        is_mac || !self.is_mac_only
    }

    pub fn runnable(&self, context: &CommandContext) -> Option<Run> {
        match self.execution {
            Execution::Native(run) if self.enablement.allows(self, context) => Some(run),
            _ => None,
        }
    }

    pub fn is_runnable(&self, context: &CommandContext) -> bool {
        self.runnable(context).is_some()
    }

    pub fn label(&self, locale: &ResolvedLocale) -> String {
        format_categorized_label(
            locale,
            self.category_key.as_deref(),
            &self.title_key,
            self.title_default_value.as_deref(),
        )
    }
}

impl Enablement {
    fn allows(self, command: &Command, context: &CommandContext) -> bool {
        match self {
            Self::Always => true,
            Self::Never | Self::WebviewDiagnostics => false,
            Self::MainWindow => context.window != WindowKind::Auxiliary,
            Self::AuxiliaryWindow => context.window == WindowKind::Auxiliary,
            Self::ActiveProject => context.active_project.is_some(),
            Self::ActiveEditor => context.active_editor_actions.is_some(),
            Self::SupportedEditorAction => command
                .editor_action_id()
                .zip(context.active_editor_actions.as_ref())
                .is_some_and(|(action, supported)| supported.contains(action)),
        }
    }
}

fn enablement(id: &str) -> Enablement {
    #[cfg(feature = "native-host")]
    if id == "editor.find" {
        return Enablement::ActiveEditor;
    }
    match id {
        "tab.close" | "editor.find" => Enablement::Never,
        "terminal.copyImeDebug" | "app.showPerfSnapshot" => Enablement::WebviewDiagnostics,
        "tab.moveToMainWindow" => Enablement::AuxiliaryWindow,
        "view.toggleZenMode" => Enablement::MainWindow,
        "ai.inlineEdit" => Enablement::ActiveEditor,
        "git.revertHead" | "git.createTagOnHead" | "task.runTask" => Enablement::ActiveProject,
        _ => match id.strip_prefix(EDITOR_ACTION_PREFIX) {
            Some(action) if !EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE.contains(&action) => {
                Enablement::SupportedEditorAction
            }
            _ => Enablement::Always,
        },
    }
}

fn execution(id: &str, keymap_id: Option<&str>) -> Execution {
    #[cfg(feature = "native-host")]
    if let Some(command) = if id == "editor.find" {
        Some(crate::editor_find::FindCommand::Open)
    } else {
        id.strip_prefix(EDITOR_ACTION_PREFIX)
            .and_then(crate::editor_find::FindCommand::from_action)
    } {
        return Execution::Native(Run::EditDocument(DocumentEdit::Find(command)));
    }
    let run = match id {
        #[cfg(feature = "native-host")]
        "monaco.editor.action.toggleStickyScroll" => Some(Run::ToggleEditorStickyScroll),
        "settings.open" => Some(Run::OpenSettingsTab),
        "app.openSettingsFile" => Some(Run::OpenSettingsFile),
        "monaco.deleteAllLeft" => Some(Run::EditDocument(DocumentEdit::DeleteAllLeft)),
        "monaco.editor.action.outdentLines" => Some(Run::EditDocument(DocumentEdit::OutdentLines)),
        "monaco.taide.saveFile" => Some(Run::SaveActiveTab),
        _ => id
            .strip_prefix(EDITOR_ACTION_PREFIX)
            .and_then(fold_command)
            .map(Run::FoldDocument)
            .or_else(|| {
                id.strip_prefix(EDITOR_ACTION_PREFIX)
                    .and_then(line_command)
                    .map(|command| Run::EditDocument(DocumentEdit::Line(command)))
            })
            .or_else(|| {
                id.strip_prefix(EDITOR_ACTION_PREFIX)
                    .and_then(cursor_command)
                    .map(|command| Run::EditDocument(DocumentEdit::Cursor(command)))
            })
            .or_else(|| keymap_id.and_then(keymap_run)),
    };
    run.map_or(Execution::Unavailable, Execution::Native)
}

pub fn line_command(action: &str) -> Option<LineCommand> {
    Some(match action {
        "editor.action.moveLinesUpAction" => LineCommand::MoveLinesUp,
        "editor.action.moveLinesDownAction" => LineCommand::MoveLinesDown,
        "editor.action.copyLinesUpAction" => LineCommand::CopyLinesUp,
        "editor.action.copyLinesDownAction" => LineCommand::CopyLinesDown,
        "editor.action.duplicateSelection" => LineCommand::DuplicateSelection,
        "editor.action.deleteLines" => LineCommand::DeleteLines,
        "editor.action.insertLineBefore" => LineCommand::InsertLineBefore,
        "editor.action.insertLineAfter" => LineCommand::InsertLineAfter,
        "editor.action.joinLines" => LineCommand::JoinLines,
        "deleteAllLeft" => LineCommand::DeleteAllLeft,
        "deleteAllRight" => LineCommand::DeleteAllRight,
        "deleteInsideWord" => LineCommand::DeleteInsideWord,
        "editor.action.indentLines" => LineCommand::IndentLines,
        "editor.action.outdentLines" => LineCommand::OutdentLines,
        "editor.action.trimTrailingWhitespace" => LineCommand::TrimTrailingWhitespace,
        "editor.action.insertFinalNewLine" => LineCommand::InsertFinalNewLine,
        "editor.action.sortLinesAscending" => LineCommand::SortLinesAscending,
        "editor.action.sortLinesDescending" => LineCommand::SortLinesDescending,
        "editor.action.removeDuplicateLines" => LineCommand::RemoveDuplicateLines,
        "editor.action.reverseLines" => LineCommand::ReverseLines,
        "editor.action.transformToUppercase" => LineCommand::Transform(TextCase::Upper),
        "editor.action.transformToLowercase" => LineCommand::Transform(TextCase::Lower),
        "editor.action.transformToTitlecase" => LineCommand::Transform(TextCase::Title),
        "editor.action.transformToSnakecase" => LineCommand::Transform(TextCase::Snake),
        "editor.action.transformToCamelcase" => LineCommand::Transform(TextCase::Camel),
        "editor.action.transformToPascalcase" => LineCommand::Transform(TextCase::Pascal),
        "editor.action.transformToKebabcase" => LineCommand::Transform(TextCase::Kebab),
        "editor.action.commentLine" => LineCommand::ToggleLineComment,
        "editor.action.addCommentLine" => LineCommand::AddLineComment,
        "editor.action.removeCommentLine" => LineCommand::RemoveLineComment,
        "editor.action.blockComment" => LineCommand::ToggleBlockComment,
        "editor.action.removeBrackets" => LineCommand::RemoveBrackets,
        "editor.action.transpose" => LineCommand::Transpose,
        "editor.action.transposeLetters" => LineCommand::TransposeLetters,
        _ => return None,
    })
}

pub fn cursor_command(action: &str) -> Option<CursorCommand> {
    Some(match action {
        "editor.action.insertCursorAbove" => CursorCommand::AddAbove,
        "editor.action.insertCursorBelow" => CursorCommand::AddBelow,
        "editor.action.insertCursorAtEndOfEachLineSelected" => CursorCommand::LineEnds,
        "editor.action.addCursorsToTop" => CursorCommand::ToTop,
        "editor.action.addCursorsToBottom" => CursorCommand::ToBottom,
        "editor.action.focusNextCursor" => CursorCommand::FocusNext,
        "editor.action.focusPreviousCursor" => CursorCommand::FocusPrevious,
        "editor.action.addSelectionToNextFindMatch" => CursorCommand::AddNextMatch,
        "editor.action.addSelectionToPreviousFindMatch" => CursorCommand::AddPreviousMatch,
        "editor.action.moveSelectionToNextFindMatch" => CursorCommand::MoveNextMatch,
        "editor.action.moveSelectionToPreviousFindMatch" => CursorCommand::MovePreviousMatch,
        "editor.action.selectHighlights" => CursorCommand::SelectMatches,
        "editor.action.changeAll" => CursorCommand::ChangeAll,
        "expandLineSelection" => CursorCommand::ExpandLine,
        "editor.action.smartSelect.expand" => CursorCommand::Expand,
        "editor.action.smartSelect.shrink" => CursorCommand::Shrink,
        "editor.action.jumpToBracket" => CursorCommand::JumpToBracket,
        "editor.action.selectToBracket" => CursorCommand::SelectToBracket,
        "cursorUndo" => CursorCommand::Undo,
        "cursorRedo" => CursorCommand::Redo,
        "editor.action.setSelectionAnchor" => CursorCommand::SetAnchor,
        "editor.action.moveCarretLeftAction" => CursorCommand::MoveCaretLeft,
        "editor.action.moveCarretRightAction" => CursorCommand::MoveCaretRight,
        _ => return None,
    })
}

fn fold_command(action: &str) -> Option<FoldCommand> {
    Some(match action {
        "editor.fold" => FoldCommand::Fold,
        "editor.unfold" => FoldCommand::Unfold,
        "editor.toggleFold" => FoldCommand::ToggleFold,
        "editor.foldRecursively" => FoldCommand::FoldRecursively,
        "editor.unfoldRecursively" => FoldCommand::UnfoldRecursively,
        "editor.toggleFoldRecursively" => FoldCommand::ToggleFoldRecursively,
        "editor.foldAll" => FoldCommand::FoldAll,
        "editor.unfoldAll" => FoldCommand::UnfoldAll,
        "editor.foldAllExcept" => FoldCommand::FoldAllExcept,
        "editor.unfoldAllExcept" => FoldCommand::UnfoldAllExcept,
        "editor.gotoParentFold" => FoldCommand::GotoParentFold,
        "editor.gotoPreviousFold" => FoldCommand::GotoPreviousFold,
        "editor.gotoNextFold" => FoldCommand::GotoNextFold,
        "editor.foldAllBlockComments" => FoldCommand::FoldAllBlockComments,
        "editor.foldAllMarkerRegions" => FoldCommand::FoldAllMarkerRegions,
        "editor.unfoldAllMarkerRegions" => FoldCommand::UnfoldAllMarkerRegions,
        _ => return None,
    })
}

pub fn keymap_run(keymap_id: &str) -> Option<Run> {
    #[cfg(feature = "native-host")]
    if keymap_id == "find" {
        return Some(Run::EditDocument(DocumentEdit::Find(
            crate::editor_find::FindCommand::Open,
        )));
    }
    let focus_direction = |direction| Run::FocusGroup(GroupTarget::Direction(direction));
    let focus_position = |position| Run::FocusGroup(GroupTarget::Position(position));
    Some(match keymap_id {
        "quick-open" => Run::OpenPalette(PaletteEntry::Files),
        "command-palette" => Run::OpenPalette(PaletteEntry::Commands),
        "workspace-symbol" => Run::OpenPalette(PaletteEntry::WorkspaceSymbols),
        "open-keybindings-editor" => Run::OpenKeybindingsEditor,
        "close-tab" => Run::CloseTab,
        "toggle-sidebar" => Run::ToggleSidebar,
        "split" => Run::Split,
        "tab-cycle-next" | "editor-next" => Run::CycleTab(TabCycle::Next),
        "tab-cycle-prev" | "editor-previous" => Run::CycleTab(TabCycle::Previous),
        "save" => Run::SaveActiveTab,
        "close-all-tabs" => Run::CloseAllTabs,
        "toggle-terminal" => Run::ToggleTerminal,
        "new-terminal" => Run::OpenTerminalTab,
        "reopen-closed-tab" => Run::ReopenClosedTab,
        "font-size-up" => Run::ChangeEditorFontSize { increase: true },
        "font-size-down" => Run::ChangeEditorFontSize { increase: false },
        "toggle-zen-mode" => Run::ToggleZenMode,
        "focus-group-left" => focus_direction(Direction::Left),
        "focus-group-right" => focus_direction(Direction::Right),
        "focus-group-up" => focus_direction(Direction::Up),
        "focus-group-down" => focus_direction(Direction::Down),
        "focus-group-1" => focus_position(1),
        "focus-group-2" => focus_position(2),
        "focus-group-3" => focus_position(3),
        "focus-group-4" => focus_position(4),
        "focus-group-5" => focus_position(5),
        "focus-group-6" => focus_position(6),
        "focus-group-7" => focus_position(7),
        "focus-group-8" => focus_position(8),
        "focus-group-9" => focus_position(9),
        "move-tab-to-group-left" => Run::MoveTabToGroup(Direction::Left),
        "move-tab-to-group-right" => Run::MoveTabToGroup(Direction::Right),
        _ => return None,
    })
}

pub fn format_categorized_label(
    locale: &ResolvedLocale,
    category_key: Option<&str>,
    title_key: &str,
    title_default_value: Option<&str>,
) -> String {
    let title = locale
        .messages
        .get(title_key)
        .map(String::as_str)
        .or_else(|| title_default_value.filter(|value| !value.is_empty()))
        .unwrap_or(title_key);
    let Some(category) = category_key.filter(|value| !value.is_empty()) else {
        return title.to_owned();
    };
    let category = locale
        .messages
        .get(category)
        .map(String::as_str)
        .unwrap_or(category);
    format!("{category}: {title}")
}

pub struct Registry {
    commands: Vec<Command>,
}

impl Registry {
    fn load() -> AppResult<Self> {
        let entries: Vec<Value> =
            serde_json::from_str(CATALOG).map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(Self {
            commands: entries
                .iter()
                .map(Command::parse)
                .collect::<AppResult<_>>()?,
        })
    }

    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    pub fn command(&self, id: &str) -> Option<&Command> {
        self.commands.iter().find(|command| command.id == id)
    }

    pub fn editor_action_ids(&self, editor: ActiveEditor) -> HashSet<String> {
        self.commands
            .iter()
            .filter_map(|command| {
                let action = command.editor_action_id()?;
                match command.execution {
                    #[cfg(feature = "native-host")]
                    Execution::Native(Run::EditDocument(DocumentEdit::Find(command)))
                        if !editor.is_read_only || !command.requires_write() =>
                    {
                        Some(action.to_owned())
                    }
                    Execution::Native(Run::EditDocument(DocumentEdit::Cursor(command)))
                        if !editor.is_read_only
                            || !matches!(
                                command,
                                CursorCommand::ChangeAll
                                    | CursorCommand::MoveCaretLeft
                                    | CursorCommand::MoveCaretRight
                            ) =>
                    {
                        Some(action.to_owned())
                    }
                    Execution::Native(Run::EditDocument(_)) if editor.is_read_only => None,
                    Execution::Native(Run::FoldDocument(_)) if !editor.has_folding => None,
                    Execution::Native(_) => Some(action.to_owned()),
                    Execution::Unavailable => None,
                }
            })
            .collect()
    }
}

pub fn registry() -> AppResult<&'static Registry> {
    static REGISTRY: OnceLock<AppResult<Registry>> = OnceLock::new();
    REGISTRY
        .get_or_init(Registry::load)
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FIXTURE: &str =
        include_str!("../../taide-native-app/tests/fixtures/keybinding-catalog.json");
    const KEYMAP_DEFAULTS: &str = include_str!("keymap-defaults.json");
    const COMMAND_COUNT: usize = 212;

    #[cfg(feature = "native-host")]
    #[test]
    fn 고정_줄_토글은_기존_원본_명령만_연결하고_읽기전용에서도_활성화한다() {
        let registry = registry().unwrap();
        let command = registry
            .command("monaco.editor.action.toggleStickyScroll")
            .unwrap();
        assert_eq!(
            command.runnable(&CommandContext::default()),
            Some(Run::ToggleEditorStickyScroll)
        );
        let actions = registry.editor_action_ids(ActiveEditor {
            is_read_only: true,
            has_folding: false,
        });
        assert!(actions.contains("editor.action.toggleStickyScroll"));
        assert!(
            registry
                .command("monaco.editor.action.focusStickyScroll")
                .is_none()
        );
    }
    const CURSOR_ACTIONS: [&str; 23] = [
        "editor.action.insertCursorAbove",
        "editor.action.insertCursorBelow",
        "editor.action.insertCursorAtEndOfEachLineSelected",
        "editor.action.addCursorsToTop",
        "editor.action.addCursorsToBottom",
        "editor.action.focusNextCursor",
        "editor.action.focusPreviousCursor",
        "editor.action.addSelectionToNextFindMatch",
        "editor.action.addSelectionToPreviousFindMatch",
        "editor.action.moveSelectionToNextFindMatch",
        "editor.action.moveSelectionToPreviousFindMatch",
        "editor.action.selectHighlights",
        "editor.action.changeAll",
        "expandLineSelection",
        "editor.action.smartSelect.expand",
        "editor.action.smartSelect.shrink",
        "editor.action.jumpToBracket",
        "editor.action.selectToBracket",
        "cursorUndo",
        "cursorRedo",
        "editor.action.setSelectionAnchor",
        "editor.action.moveCarretLeftAction",
        "editor.action.moveCarretRightAction",
    ];
    const KEYMAP_COUNT: usize = 41;
    const NATIVE_KEYMAP_COUNT: usize = 34 + EDITOR_FIND_AVAILABLE as usize;
    const FOLD_COMMANDS: [(&str, FoldCommand); 16] = [
        ("monaco.editor.fold", FoldCommand::Fold),
        ("monaco.editor.foldAll", FoldCommand::FoldAll),
        (
            "monaco.editor.foldAllBlockComments",
            FoldCommand::FoldAllBlockComments,
        ),
        ("monaco.editor.foldAllExcept", FoldCommand::FoldAllExcept),
        (
            "monaco.editor.foldAllMarkerRegions",
            FoldCommand::FoldAllMarkerRegions,
        ),
        (
            "monaco.editor.foldRecursively",
            FoldCommand::FoldRecursively,
        ),
        ("monaco.editor.gotoNextFold", FoldCommand::GotoNextFold),
        ("monaco.editor.gotoParentFold", FoldCommand::GotoParentFold),
        (
            "monaco.editor.gotoPreviousFold",
            FoldCommand::GotoPreviousFold,
        ),
        ("monaco.editor.toggleFold", FoldCommand::ToggleFold),
        (
            "monaco.editor.toggleFoldRecursively",
            FoldCommand::ToggleFoldRecursively,
        ),
        ("monaco.editor.unfold", FoldCommand::Unfold),
        ("monaco.editor.unfoldAll", FoldCommand::UnfoldAll),
        (
            "monaco.editor.unfoldAllExcept",
            FoldCommand::UnfoldAllExcept,
        ),
        (
            "monaco.editor.unfoldAllMarkerRegions",
            FoldCommand::UnfoldAllMarkerRegions,
        ),
        (
            "monaco.editor.unfoldRecursively",
            FoldCommand::UnfoldRecursively,
        ),
    ];
    const UNSUPPORTED_FOLD_COMMANDS: [&str; 3] = [
        "monaco.editor.createFoldingRangeFromSelection",
        "monaco.editor.removeManualFoldingRanges",
        "monaco.editor.toggleImportFold",
    ];
    const DOCUMENT_ACTIONS: [&str; 34] = [
        "deleteAllLeft",
        "deleteAllRight",
        "deleteInsideWord",
        "editor.action.moveLinesUpAction",
        "editor.action.moveLinesDownAction",
        "editor.action.copyLinesUpAction",
        "editor.action.copyLinesDownAction",
        "editor.action.duplicateSelection",
        "editor.action.deleteLines",
        "editor.action.insertLineBefore",
        "editor.action.insertLineAfter",
        "editor.action.joinLines",
        "editor.action.indentLines",
        "editor.action.outdentLines",
        "editor.action.trimTrailingWhitespace",
        "editor.action.insertFinalNewLine",
        "editor.action.sortLinesAscending",
        "editor.action.sortLinesDescending",
        "editor.action.removeDuplicateLines",
        "editor.action.reverseLines",
        "editor.action.transformToUppercase",
        "editor.action.transformToLowercase",
        "editor.action.transformToTitlecase",
        "editor.action.transformToSnakecase",
        "editor.action.transformToCamelcase",
        "editor.action.transformToPascalcase",
        "editor.action.transformToKebabcase",
        "editor.action.commentLine",
        "editor.action.addCommentLine",
        "editor.action.removeCommentLine",
        "editor.action.blockComment",
        "editor.action.removeBrackets",
        "editor.action.transpose",
        "editor.action.transposeLetters",
    ];
    const SAVE_ACTION: &str = "taide.saveFile";
    const FOLDING_CATEGORY: &str = "keymap.category.editorFolding";
    const NATIVE_COMMANDS: [(&str, Run); 35] = [
        ("settings.open", Run::OpenSettingsTab),
        ("app.openSettingsFile", Run::OpenSettingsFile),
        ("keybindings.open", Run::OpenKeybindingsEditor),
        ("terminal.new", Run::OpenTerminalTab),
        ("tab.reopenClosed", Run::ReopenClosedTab),
        ("file.quickOpen", Run::OpenPalette(PaletteEntry::Files)),
        ("tab.close", Run::CloseTab),
        ("view.toggleSidebar", Run::ToggleSidebar),
        ("editor.split", Run::Split),
        ("tab.cycleNext", Run::CycleTab(TabCycle::Next)),
        ("tab.cyclePrev", Run::CycleTab(TabCycle::Previous)),
        ("editor.save", Run::SaveActiveTab),
        ("view.toggleTerminal", Run::ToggleTerminal),
        ("view.toggleZenMode", Run::ToggleZenMode),
        ("tab.previousEditor", Run::CycleTab(TabCycle::Previous)),
        ("tab.nextEditor", Run::CycleTab(TabCycle::Next)),
        (
            "editor.focusGroupLeft",
            Run::FocusGroup(GroupTarget::Direction(Direction::Left)),
        ),
        (
            "editor.focusGroupRight",
            Run::FocusGroup(GroupTarget::Direction(Direction::Right)),
        ),
        (
            "editor.focusGroupUp",
            Run::FocusGroup(GroupTarget::Direction(Direction::Up)),
        ),
        (
            "editor.focusGroupDown",
            Run::FocusGroup(GroupTarget::Direction(Direction::Down)),
        ),
        ("tab.moveToGroupLeft", Run::MoveTabToGroup(Direction::Left)),
        (
            "tab.moveToGroupRight",
            Run::MoveTabToGroup(Direction::Right),
        ),
        ("tab.closeAllInGroup", Run::CloseAllTabs),
        (
            "editor.focusGroup1",
            Run::FocusGroup(GroupTarget::Position(1)),
        ),
        (
            "editor.focusGroup2",
            Run::FocusGroup(GroupTarget::Position(2)),
        ),
        (
            "editor.focusGroup3",
            Run::FocusGroup(GroupTarget::Position(3)),
        ),
        (
            "editor.focusGroup4",
            Run::FocusGroup(GroupTarget::Position(4)),
        ),
        (
            "editor.focusGroup5",
            Run::FocusGroup(GroupTarget::Position(5)),
        ),
        (
            "editor.focusGroup6",
            Run::FocusGroup(GroupTarget::Position(6)),
        ),
        (
            "editor.focusGroup7",
            Run::FocusGroup(GroupTarget::Position(7)),
        ),
        (
            "editor.focusGroup8",
            Run::FocusGroup(GroupTarget::Position(8)),
        ),
        (
            "editor.focusGroup9",
            Run::FocusGroup(GroupTarget::Position(9)),
        ),
        (
            "monaco.deleteAllLeft",
            Run::EditDocument(DocumentEdit::DeleteAllLeft),
        ),
        (
            "monaco.editor.action.outdentLines",
            Run::EditDocument(DocumentEdit::OutdentLines),
        ),
        ("monaco.taide.saveFile", Run::SaveActiveTab),
    ];

    fn locale(messages: &[(&str, &str)]) -> ResolvedLocale {
        ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: messages
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    fn editor_context(registry: &Registry, is_read_only: bool) -> CommandContext {
        CommandContext {
            active_editor_actions: Some(registry.editor_action_ids(ActiveEditor {
                is_read_only,
                has_folding: false,
            })),
            ..Default::default()
        }
    }

    #[test]
    fn command_registry는_원본_전체명령의_등록순서_메타데이터와_플랫폼등록을_보존한다() {
        let fixtures: Vec<Value> = serde_json::from_str(FIXTURE).unwrap();
        let registry = registry().unwrap();
        assert_eq!(registry.commands().len(), COMMAND_COUNT);
        assert_eq!(
            registry
                .commands()
                .iter()
                .map(|command| command.id.as_str())
                .collect::<HashSet<_>>()
                .len(),
            COMMAND_COUNT
        );
        for name in ["default-mac", "default-non-mac"] {
            let fixture = fixtures
                .iter()
                .find(|fixture| fixture["name"] == name)
                .unwrap();
            let is_mac = fixture["mac"].as_bool().unwrap();
            let expected = fixture["rows"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| !row["commandId"].is_null())
                .map(|row| {
                    json!({
                        "id": row["commandId"],
                        "titleKey": row["titleKey"],
                        "titleDefaultValue": row["titleDefaultValue"],
                        "categoryKey": row["categoryKey"],
                        "keymapId": row["keymapId"],
                        "defaultBindingLabel": row["defaultBindingLabel"],
                        "runsViaCommand": row["runsViaCommand"],
                        "source": row["source"],
                    })
                })
                .collect::<Vec<_>>();
            let actual = registry
                .commands()
                .iter()
                .filter(|command| command.is_registered(is_mac))
                .map(|command| {
                    json!({
                        "id": command.id,
                        "titleKey": command.title_key,
                        "titleDefaultValue": command.title_default_value,
                        "categoryKey": command.category_key,
                        "keymapId": command.keymap_id,
                        "defaultBindingLabel": command.default_binding_label,
                        "runsViaCommand": command.runs_via_command(),
                        "source": if command.editor_action_id().is_some() { "monaco" } else { "app" },
                    })
                })
                .collect::<Vec<_>>();
            assert_eq!(json!(actual), json!(expected), "{name}");
        }
        assert!(
            registry
                .command("cli.installShellCommand")
                .unwrap()
                .is_mac_only
        );
        assert!(registry.command("unknown.future").is_none());
    }

    #[test]
    fn 실행경로는_native동작이_있는_명령과_keymap액션만_typed요청으로_구분한다() {
        let registry = registry().unwrap();
        let native = registry
            .commands()
            .iter()
            .filter_map(|command| match command.execution {
                #[cfg(feature = "native-host")]
                Execution::Native(Run::ToggleEditorStickyScroll) => None,
                #[cfg(feature = "native-host")]
                Execution::Native(Run::EditDocument(DocumentEdit::Find(_))) => None,
                Execution::Native(Run::EditDocument(
                    DocumentEdit::Line(_) | DocumentEdit::Cursor(_),
                )) => None,
                Execution::Native(run) => Some((command.id.as_str(), run)),
                Execution::Unavailable => None,
            })
            .collect::<Vec<_>>();
        let fold_commands = FOLD_COMMANDS.map(|(id, command)| (id, Run::FoldDocument(command)));
        let (before_folds, after_folds) = NATIVE_COMMANDS.split_at(NATIVE_COMMANDS.len() - 1);
        assert_eq!(native, [before_folds, &fold_commands, after_folds].concat());
        let defaults: Vec<Value> = serde_json::from_str(KEYMAP_DEFAULTS).unwrap();
        assert_eq!(defaults.len(), KEYMAP_COUNT);
        let keymap_ids = defaults
            .iter()
            .map(|entry| entry["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            keymap_ids
                .iter()
                .filter(|id| keymap_run(id).is_some())
                .count(),
            NATIVE_KEYMAP_COUNT
        );
        for (id, entry) in [
            ("quick-open", PaletteEntry::Files),
            ("command-palette", PaletteEntry::Commands),
            ("workspace-symbol", PaletteEntry::WorkspaceSymbols),
        ] {
            assert_eq!(keymap_run(id), Some(Run::OpenPalette(entry)), "{id}");
        }
        for pending in [
            "find",
            "search",
            "search-replace",
            "explorer",
            "git",
            "terminal-jump-to-previous-command",
            "terminal-jump-to-next-command",
            "focus-group-10",
            "settings.open",
        ] {
            if pending == "find" && EDITOR_FIND_AVAILABLE {
                continue;
            }
            assert_eq!(keymap_run(pending), None, "{pending}");
        }
        for command in registry.commands() {
            if let Some(keymap_id) = &command.keymap_id {
                assert!(keymap_ids.contains(&keymap_id.as_str()), "{}", command.id);
                assert_eq!(
                    command.execution,
                    keymap_run(keymap_id).map_or(Execution::Unavailable, Execution::Native),
                    "{}",
                    command.id
                );
            }
        }
        let fold_actions =
            FOLD_COMMANDS.map(|(id, _)| id.strip_prefix(EDITOR_ACTION_PREFIX).unwrap());
        for (is_read_only, has_folding) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let expected = [SAVE_ACTION]
                .into_iter()
                .chain(DOCUMENT_ACTIONS.into_iter().filter(|_| !is_read_only))
                .chain(CURSOR_ACTIONS.into_iter().filter(|action| {
                    !is_read_only
                        || !matches!(
                            *action,
                            "editor.action.changeAll"
                                | "editor.action.moveCarretLeftAction"
                                | "editor.action.moveCarretRightAction"
                        )
                }))
                .chain(fold_actions.into_iter().filter(|_| has_folding))
                .chain(
                    ["editor.action.toggleStickyScroll"]
                        .into_iter()
                        .filter(|_| cfg!(feature = "native-host")),
                )
                .chain(
                    [
                        "actions.find",
                        "actions.findWithSelection",
                        "editor.actions.findWithArgs",
                        "editor.action.nextMatchFindAction",
                        "editor.action.previousMatchFindAction",
                        "editor.action.nextSelectionMatchFindAction",
                        "editor.action.previousSelectionMatchFindAction",
                        "editor.action.startFindReplaceAction",
                    ]
                    .into_iter()
                    .filter(|action| {
                        EDITOR_FIND_AVAILABLE
                            && (!is_read_only || *action != "editor.action.startFindReplaceAction")
                    }),
                )
                .map(str::to_owned)
                .collect::<HashSet<_>>();
            assert_eq!(
                registry.editor_action_ids(ActiveEditor {
                    is_read_only,
                    has_folding,
                }),
                expected,
                "read only {is_read_only} folding {has_folding}"
            );
        }
    }

    #[test]
    fn 접기_명령은_접기가_켜진_활성_editor에서만_실행되고_수동_범위나_provider가_필요한_명령은_실행경로가_없다()
     {
        let registry = registry().unwrap();
        let foldable = CommandContext {
            active_editor_actions: Some(registry.editor_action_ids(ActiveEditor {
                is_read_only: true,
                has_folding: true,
            })),
            ..Default::default()
        };
        let unfoldable = editor_context(registry, false);
        for (id, fold) in FOLD_COMMANDS {
            let command = registry.command(id).unwrap();
            assert_eq!(
                command.enablement,
                Enablement::SupportedEditorAction,
                "{id}"
            );
            assert_eq!(
                command.runnable(&foldable),
                Some(Run::FoldDocument(fold)),
                "{id}"
            );
            assert!(!command.is_runnable(&unfoldable), "{id}");
            assert!(!command.is_runnable(&CommandContext::default()), "{id}");
        }
        for id in UNSUPPORTED_FOLD_COMMANDS {
            let command = registry.command(id).unwrap();
            assert_eq!(command.execution, Execution::Unavailable, "{id}");
            assert!(!command.is_runnable(&foldable), "{id}");
        }
        assert_eq!(
            registry
                .commands()
                .iter()
                .filter(|command| command.category_key.as_deref() == Some(FOLDING_CATEGORY))
                .count(),
            FOLD_COMMANDS.len() + UNSUPPORTED_FOLD_COMMANDS.len()
        );
    }

    #[test]
    fn enabled_판정은_원본_조건과_실행경로_유무를_함께_적용한다() {
        let registry = registry().unwrap();
        let command = |id: &str| registry.command(id).unwrap();
        let main = CommandContext::default();
        let auxiliary = CommandContext {
            window: WindowKind::Auxiliary,
            ..Default::default()
        };
        let project = CommandContext {
            active_project: Some(ProjectId::new()),
            ..Default::default()
        };
        let writable = editor_context(registry, false);
        let read_only = editor_context(registry, true);
        let contexts = [&main, &auxiliary, &project, &writable, &read_only];

        for always in ["settings.open", "app.openSettingsFile", "editor.save"] {
            assert_eq!(command(always).enablement, Enablement::Always);
            assert!(
                contexts
                    .iter()
                    .all(|context| command(always).is_runnable(context))
            );
        }
        assert_eq!(
            command("settings.open").runnable(&main),
            Some(Run::OpenSettingsTab)
        );

        for (id, enablement) in [
            ("tab.close", Enablement::Never),
            ("terminal.copyImeDebug", Enablement::WebviewDiagnostics),
            ("app.showPerfSnapshot", Enablement::WebviewDiagnostics),
        ] {
            assert_eq!(command(id).enablement, enablement, "{id}");
            assert!(
                contexts
                    .iter()
                    .all(|context| !command(id).is_runnable(context))
            );
        }
        let find = command("editor.find");
        assert_eq!(
            find.enablement,
            if EDITOR_FIND_AVAILABLE {
                Enablement::ActiveEditor
            } else {
                Enablement::Never
            }
        );
        assert!(!find.is_runnable(&main));
        assert_eq!(find.is_runnable(&writable), EDITOR_FIND_AVAILABLE);
        assert_eq!(find.is_runnable(&read_only), EDITOR_FIND_AVAILABLE);
        assert_eq!(
            command("tab.close").execution,
            Execution::Native(Run::CloseTab)
        );

        assert_eq!(
            command("view.toggleZenMode").enablement,
            Enablement::MainWindow
        );
        assert!(command("view.toggleZenMode").is_runnable(&main));
        assert!(!command("view.toggleZenMode").is_runnable(&auxiliary));

        for (id, enablement, allowed) in [
            (
                "tab.moveToMainWindow",
                Enablement::AuxiliaryWindow,
                [false, true, false, false, false],
            ),
            (
                "git.revertHead",
                Enablement::ActiveProject,
                [false, false, true, false, false],
            ),
            (
                "git.createTagOnHead",
                Enablement::ActiveProject,
                [false, false, true, false, false],
            ),
            (
                "task.runTask",
                Enablement::ActiveProject,
                [false, false, true, false, false],
            ),
            (
                "ai.inlineEdit",
                Enablement::ActiveEditor,
                [false, false, false, true, true],
            ),
            ("window.reload", Enablement::Always, [true; 5]),
            ("sync.uploadNow", Enablement::Always, [true; 5]),
            ("git.toggleBlame", Enablement::Always, [true; 5]),
            (
                "monaco.editor.action.quickFix",
                Enablement::Always,
                [true; 5],
            ),
        ] {
            let command = command(id);
            assert_eq!(command.enablement, enablement, "{id}");
            assert_eq!(
                contexts.map(|context| command.enablement.allows(command, context)),
                allowed,
                "{id}"
            );
            assert_eq!(command.execution, Execution::Unavailable, "{id}");
            assert!(
                contexts.iter().all(|context| !command.is_runnable(context)),
                "{id}"
            );
        }

        for edit in [
            "monaco.deleteAllLeft",
            "monaco.editor.action.outdentLines",
            "monaco.editor.action.commentLine",
        ] {
            assert_eq!(command(edit).enablement, Enablement::SupportedEditorAction);
            assert!(command(edit).is_runnable(&writable));
            for context in [&main, &auxiliary, &project, &read_only] {
                assert!(!command(edit).is_runnable(context), "{edit}");
            }
        }
        assert!(command("monaco.taide.saveFile").is_runnable(&writable));
        assert!(command("monaco.taide.saveFile").is_runnable(&read_only));
        assert!(!command("monaco.taide.saveFile").is_runnable(&main));
        assert_eq!(
            registry
                .commands()
                .iter()
                .filter(|command| command.editor_action_id().is_some()
                    && command.enablement == Enablement::Always)
                .count(),
            EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE.len()
        );
    }

    #[test]
    fn 제목은_카테고리_콜론_제목_형식과_번역_기본값_키_순서를_따른다() {
        let registry = registry().unwrap();
        let locale = locale(&[
            ("keymap.category.app", "App"),
            ("settings.title", "Settings"),
            ("app.welcome", "Welcome"),
        ]);
        assert_eq!(
            registry.command("settings.open").unwrap().label(&locale),
            "App: Settings"
        );
        assert_eq!(
            registry
                .command("app.showPerfSnapshot")
                .unwrap()
                .label(&locale),
            "App: Show Performance Snapshot"
        );
        assert_eq!(
            registry.command("view.welcome").unwrap().label(&locale),
            "keymap.category.view: Welcome"
        );
        assert_eq!(
            format_categorized_label(&locale, None, "app.welcome", None),
            "Welcome"
        );
        assert_eq!(
            format_categorized_label(&locale, Some(""), "missing.title", Some("")),
            "missing.title"
        );
        assert_eq!(
            format_categorized_label(&locale, None, "settings.title", Some("Fallback")),
            "Settings"
        );
    }
}
