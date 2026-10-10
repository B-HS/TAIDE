use std::collections::HashSet;

use taide_model::ids::TabId;
use taide_native_editor::document::EditorError;
use taide_native_editor::line_commands::{LineCommand, LineCommandContext, run_line_command};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::shell::WindowScope;
use taide_native_ui::snapshot::ShellSnapshot;

use crate::command_registry::{
    ActiveEditor, CommandContext, DocumentEdit, FoldCommand, Run, WindowKind, registry,
};

pub(crate) fn active_editor_actions(
    store: &EditorStore,
    focused_view: Option<&ViewKey>,
) -> Option<HashSet<String>> {
    let view = store.views().get(store.views().find(focused_view?)?)?;
    let document = store.documents().snapshot(view.document).ok()?;
    Some(registry().ok()?.editor_action_ids(ActiveEditor {
        is_read_only: document.metadata.read_only,
        has_folding: taide_native_ui::presentation::editor_folding(document.metadata.tier),
    }))
}

pub(crate) fn context(
    snapshot: &ShellSnapshot,
    scope: &WindowScope,
    active_editor_actions: Option<HashSet<String>>,
) -> CommandContext {
    match scope {
        WindowScope::Main => CommandContext {
            active_project: snapshot.focused_project().cloned(),
            focused_shell_slot: snapshot.shell.focused.clone(),
            active_editor_actions,
            window: WindowKind::Main,
        },
        WindowScope::Auxiliary { project, .. } => CommandContext {
            active_project: Some(project.clone()),
            focused_shell_slot: None,
            active_editor_actions,
            window: WindowKind::Auxiliary,
        },
    }
}

fn command_run(id: &str, context: &CommandContext) -> Option<Run> {
    registry().ok()?.command(id)?.runnable(context)
}

pub(crate) fn accepts(id: &str, has_focused_shell: bool, context: &CommandContext) -> bool {
    if id == "find" {
        return command_run("editor.find", context).is_some();
    }
    if crate::shell_keymap::supports(id) {
        return has_focused_shell || crate::shell_keymap::runs_without_focused_project(id);
    }
    command_run(id, context).is_some()
}

pub(crate) fn intent(
    id: &str,
    context: &CommandContext,
    snapshot: &ShellSnapshot,
) -> Option<ShellIntent> {
    if id == "find" {
        return crate::shell_keymap::intent(command_run("editor.find", context)?, snapshot);
    }
    if crate::shell_keymap::supports(id) {
        return crate::shell_keymap::action(id, snapshot);
    }
    crate::shell_keymap::intent(command_run(id, context)?, snapshot)
}

pub(crate) fn take_fold_commands(
    tab: &TabId,
    pending: &mut Vec<(TabId, FoldCommand)>,
) -> Vec<FoldCommand> {
    pending
        .extract_if(.., |(owner, _)| owner == tab)
        .map(|(_, command)| command)
        .collect()
}

pub(crate) fn apply_document_edits(
    store: &mut EditorStore,
    view: ViewId,
    tab: &TabId,
    mut context: LineCommandContext<'_>,
    indentation: taide_native_editor::indent::IndentConfiguration,
    pending: &mut Vec<(TabId, DocumentEdit)>,
    errors: &mut Vec<EditorError>,
) -> bool {
    if let Some(document) = store.views().get(view).map(|view| view.document)
        && let Ok(snapshot) = store.documents().snapshot(document)
        && let Some(options) = snapshot.indent_options
    {
        context.indent = options;
    }
    let mut changed = false;
    for (_, edit) in pending.extract_if(.., |(owner, edit)| {
        owner == tab
            && !matches!(
                edit,
                DocumentEdit::Find(_)
                    | DocumentEdit::Problem(_)
                    | DocumentEdit::Location(_)
                    | DocumentEdit::Documentation(_)
                    | DocumentEdit::Completion(_)
                    | DocumentEdit::Highlight(_)
            )
    }) {
        let result = match edit {
            DocumentEdit::Indentation(command) => {
                let result =
                    taide_native_editor::indent::run_command(store, view, command, indentation);
                if result.is_ok() {
                    context.syntax.follow_edits(store);
                    if let Some(document) = store.views().get(view).map(|view| view.document)
                        && let Ok(snapshot) = store.documents().snapshot(document)
                        && let Some(options) = snapshot.indent_options
                    {
                        context.indent = options;
                    }
                }
                result
            }
            DocumentEdit::DeleteAllLeft => {
                run_line_command(store, view, LineCommand::DeleteAllLeft, context)
            }
            DocumentEdit::OutdentLines => {
                run_line_command(store, view, LineCommand::OutdentLines, context)
            }
            DocumentEdit::Line(command) => run_line_command(store, view, command, context),
            DocumentEdit::Cursor(command) => {
                taide_native_editor::cursor_commands::run_cursor_command(
                    store, view, command, context,
                )
            }
            DocumentEdit::Find(_)
            | DocumentEdit::Problem(_)
            | DocumentEdit::Location(_)
            | DocumentEdit::Documentation(_) => {
                continue;
            }
            DocumentEdit::Completion(_) => {
                continue;
            }
            DocumentEdit::Highlight(_) => {
                continue;
            }
        };
        match result {
            Ok(applied) => changed |= applied,
            Err(error) => errors.push(error),
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_registry::PaletteEntry;
    use taide_model::{
        ids::{PaneId, ProjectId, ShellSlotId},
        layout::{PaneNode, Tab, TabKind},
        paths::AppPaths,
        project::ShellSlotTree,
    };
    use taide_native_editor::editing::{Motion, move_selection};
    use taide_native_editor::indent::IndentOptions;
    use taide_native_editor::language_configuration::UntokenizedLines;
    use taide_native_editor::store::EditorLimits;
    use taide_native_ui::commands::ShellMutation;
    use taide_runtime::AppState;

    const DOCUMENT_BYTES: usize = 1024;
    const INDENT_WIDTH: u32 = 4;
    const UNDO_GROUPS: usize = 8;
    const AUXILIARY_SLOT: u32 = 1;

    fn indentation_configuration() -> taide_native_editor::indent::IndentConfiguration {
        taide_native_editor::indent::IndentConfiguration {
            defaults: IndentOptions {
                tab_size: INDENT_WIDTH,
                insert_spaces: true,
            },
            detect_indentation: false,
        }
    }

    #[test]
    fn 변환_뒤의_편집_큐는_같은_프레임과_다음_요청에서_최신_옵션을_사용한다() {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: UNDO_GROUPS,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), "    x", "plaintext".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "synthetic".into(),
                    pane: PaneId::new(),
                    tab: tab.clone(),
                },
                document,
            )
            .unwrap();
        let context = LineCommandContext {
            indent: indentation_configuration().defaults,
            language: None,
            syntax: &UntokenizedLines,
            compare: None,
            transforms: None,
            word_rules: None,
        };
        let mut pending = vec![
            (
                tab.clone(),
                DocumentEdit::Indentation(taide_native_editor::indent::Command::ToTabs),
            ),
            (tab.clone(), DocumentEdit::Line(LineCommand::IndentLines)),
        ];
        let mut errors = Vec::new();
        assert!(apply_document_edits(
            &mut store,
            view,
            &tab,
            context,
            indentation_configuration(),
            &mut pending,
            &mut errors
        ));
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "\t\tx"
        );
        pending.push((tab.clone(), DocumentEdit::Line(LineCommand::IndentLines)));
        assert!(apply_document_edits(
            &mut store,
            view,
            &tab,
            context,
            indentation_configuration(),
            &mut pending,
            &mut errors
        ));
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "\t\t\tx"
        );
        assert!(errors.is_empty());
        assert!(store.undo(document).unwrap());
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "\t\tx"
        );
        assert!(store.undo(document).unwrap());
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "\tx"
        );
        assert!(store.undo(document).unwrap());
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "    x"
        );
    }

    #[test]
    fn 등록된_줄_명령은_앱_편집_큐에서_언어_규칙과_icu_비교를_적용한다() {
        for (id, before, expected) in [
            (
                "monaco.editor.action.sortLinesAscending",
                "file2\nfile10\nfile1",
                "file1\nfile10\nfile2",
            ),
            (
                "monaco.editor.action.sortLinesAscending",
                "ä\na\nA\ná\nae",
                "a\nA\ná\nä\nae",
            ),
            ("monaco.editor.action.commentLine", "fooBar", "// fooBar"),
            (
                "monaco.editor.action.transformToUppercase",
                "fooBar",
                "FOOBAR",
            ),
            (
                "monaco.editor.action.copyLinesDownAction",
                "fooBar",
                "fooBar\nfooBar",
            ),
            (
                "monaco.editor.action.transpose",
                "\u{1f600}x",
                "\u{fffd}\u{fffd}x",
            ),
        ] {
            let mut store = EditorStore::new(EditorLimits {
                max_documents: 1,
                max_views: 1,
                max_undo_groups: UNDO_GROUPS,
                max_document_bytes: DOCUMENT_BYTES,
            })
            .unwrap();
            let tab = TabId::new();
            let document = store
                .open_untitled(tab.clone(), before, "typescript".into())
                .unwrap();
            let view_key = ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: tab.clone(),
            };
            let view = store.attach_view(view_key.clone(), document).unwrap();
            let actions = active_editor_actions(&store, Some(&view_key));
            let context = CommandContext {
                active_editor_actions: actions,
                ..Default::default()
            };
            assert!(accepts(id, false, &context));
            let Run::EditDocument(edit) = command_run(id, &context).unwrap() else {
                panic!("{id}");
            };
            let mut pending = vec![(tab.clone(), edit)];
            let mut errors = Vec::new();
            let resources = crate::editor_command_text::resources().unwrap();
            let compare = |left: &str, right: &str| resources.compare(left, right);
            let language = crate::editor_syntax::language_rules("typescript").map(|rules| {
                taide_native_editor::language_configuration::Language {
                    rules,
                    syntax: &UntokenizedLines,
                }
            });
            assert!(
                apply_document_edits(
                    &mut store,
                    view,
                    &tab,
                    LineCommandContext {
                        indent: IndentOptions {
                            tab_size: INDENT_WIDTH,
                            insert_spaces: true
                        },
                        language,
                        syntax: &UntokenizedLines,
                        compare: Some(&compare),
                        transforms: Some(&resources.transforms),
                        word_rules: language.map(|language| language.rules),
                    },
                    indentation_configuration(),
                    &mut pending,
                    &mut errors
                ),
                "{id}"
            );
            assert!(pending.is_empty());
            assert!(errors.is_empty(), "{id}: {errors:?}");
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                expected,
                "{id}"
            );
            assert!(store.undo(document).unwrap());
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                before
            );
        }
    }
    #[test]
    fn 등록된_커서_명령은_앱_큐에서_선택만_바꾸고_다른_탭의_명령을_보존한다() {
        for (id, expected) in [
            (
                "monaco.editor.action.insertCursorBelow",
                vec![(0, 0), (4, 4)],
            ),
            (
                "monaco.editor.action.addSelectionToNextFindMatch",
                vec![(0, 3)],
            ),
            (
                "monaco.editor.action.selectHighlights",
                vec![(0, 3), (4, 7)],
            ),
            ("monaco.expandLineSelection", vec![(0, 4)]),
        ] {
            let mut store = EditorStore::new(EditorLimits {
                max_documents: 1,
                max_views: 1,
                max_undo_groups: UNDO_GROUPS,
                max_document_bytes: DOCUMENT_BYTES,
            })
            .unwrap();
            let tab = TabId::new();
            let document = store
                .open_untitled(tab.clone(), "cat\ncat", "typescript".into())
                .unwrap();
            let view_key = ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: tab.clone(),
            };
            let view = store.attach_view(view_key.clone(), document).unwrap();
            let context = CommandContext {
                active_editor_actions: active_editor_actions(&store, Some(&view_key)),
                ..Default::default()
            };
            assert!(accepts(id, false, &context));
            let Run::EditDocument(edit) = command_run(id, &context).unwrap() else {
                panic!("{id}");
            };
            let other = TabId::new();
            let mut pending = vec![
                (tab.clone(), edit),
                (other.clone(), DocumentEdit::DeleteAllLeft),
            ];
            let mut errors = Vec::new();
            let rules = crate::editor_syntax::language_rules("typescript").unwrap();
            assert!(
                !apply_document_edits(
                    &mut store,
                    view,
                    &tab,
                    LineCommandContext {
                        indent: IndentOptions {
                            tab_size: INDENT_WIDTH,
                            insert_spaces: true
                        },
                        language: Some(taide_native_editor::language_configuration::Language {
                            rules,
                            syntax: &UntokenizedLines
                        }),
                        syntax: &UntokenizedLines,
                        compare: None,
                        transforms: None,
                        word_rules: Some(rules),
                    },
                    indentation_configuration(),
                    &mut pending,
                    &mut errors
                ),
                "{id}"
            );
            assert_eq!(pending.len(), 1);
            assert_eq!(pending[0].0, other);
            assert!(errors.is_empty(), "{id}: {errors:?}");
            assert_eq!(
                store
                    .views()
                    .get(view)
                    .unwrap()
                    .selection
                    .selections
                    .iter()
                    .map(|selection| (selection.anchor, selection.head))
                    .collect::<Vec<_>>(),
                expected,
                "{id}"
            );
            let snapshot = store.documents().snapshot(document).unwrap();
            assert_eq!(snapshot.rope.to_string(), "cat\ncat");
            assert_eq!(snapshot.revision, 0);
        }
    }

    const PALETTE_ENTRIES: [(&str, PaletteEntry); 3] = [
        ("quick-open", PaletteEntry::Files),
        ("command-palette", PaletteEntry::Commands),
        ("workspace-symbol", PaletteEntry::WorkspaceSymbols),
    ];
    const KEYMAP_ACTIONS: [&str; 34] = [
        "quick-open",
        "command-palette",
        "workspace-symbol",
        "open-keybindings-editor",
        "close-tab",
        "toggle-sidebar",
        "split",
        "tab-cycle-next",
        "tab-cycle-prev",
        "editor-next",
        "editor-previous",
        "save",
        "close-all-tabs",
        "toggle-terminal",
        "new-terminal",
        "reopen-closed-tab",
        "font-size-up",
        "font-size-down",
        "toggle-zen-mode",
        "focus-group-left",
        "focus-group-right",
        "focus-group-up",
        "focus-group-down",
        "focus-group-1",
        "focus-group-2",
        "focus-group-3",
        "focus-group-4",
        "focus-group-5",
        "focus-group-6",
        "focus-group-7",
        "focus-group-8",
        "focus-group-9",
        "move-tab-to-group-left",
        "move-tab-to-group-right",
    ];
    const PROJECTLESS_KEYMAP_ACTIONS: [&str; 10] = [
        "quick-open",
        "command-palette",
        "workspace-symbol",
        "toggle-sidebar",
        "open-keybindings-editor",
        "new-terminal",
        "reopen-closed-tab",
        "font-size-up",
        "font-size-down",
        "toggle-zen-mode",
    ];

    const FOLD_ACTIONS: [(&str, FoldCommand); 19] = [
        ("editor.fold", FoldCommand::Fold),
        ("editor.unfold", FoldCommand::Unfold),
        ("editor.toggleFold", FoldCommand::ToggleFold),
        ("editor.foldRecursively", FoldCommand::FoldRecursively),
        ("editor.unfoldRecursively", FoldCommand::UnfoldRecursively),
        (
            "editor.toggleFoldRecursively",
            FoldCommand::ToggleFoldRecursively,
        ),
        ("editor.foldAll", FoldCommand::FoldAll),
        ("editor.unfoldAll", FoldCommand::UnfoldAll),
        ("editor.foldAllExcept", FoldCommand::FoldAllExcept),
        ("editor.unfoldAllExcept", FoldCommand::UnfoldAllExcept),
        ("editor.gotoParentFold", FoldCommand::GotoParentFold),
        ("editor.gotoPreviousFold", FoldCommand::GotoPreviousFold),
        ("editor.gotoNextFold", FoldCommand::GotoNextFold),
        (
            "editor.foldAllBlockComments",
            FoldCommand::FoldAllBlockComments,
        ),
        (
            "editor.foldAllMarkerRegions",
            FoldCommand::FoldAllMarkerRegions,
        ),
        (
            "editor.unfoldAllMarkerRegions",
            FoldCommand::UnfoldAllMarkerRegions,
        ),
        (
            "editor.createFoldingRangeFromSelection",
            FoldCommand::CreateFromSelection,
        ),
        (
            "editor.removeManualFoldingRanges",
            FoldCommand::RemoveManualRanges,
        ),
        ("editor.toggleImportFold", FoldCommand::ToggleImports),
    ];

    fn editor_actions(is_read_only: bool, has_folding: bool) -> CommandContext {
        CommandContext {
            active_editor_actions: Some(registry().unwrap().editor_action_ids(ActiveEditor {
                is_read_only,
                has_folding,
            })),
            ..Default::default()
        }
    }

    fn editor_context(is_read_only: bool) -> CommandContext {
        editor_actions(is_read_only, true)
    }

    #[test]
    fn 명령_gate는_keymap액션의_무프로젝트_허용목록과_명령의_enabled판정을_보존한다() {
        let context = CommandContext::default();
        for action in KEYMAP_ACTIONS {
            assert!(accepts(action, true, &context), "{action}");
            assert_eq!(
                accepts(action, false, &context),
                PROJECTLESS_KEYMAP_ACTIONS.contains(&action),
                "{action}"
            );
        }
        let project = CommandContext {
            active_project: Some(ProjectId::new()),
            ..Default::default()
        };
        for unavailable in [
            "find",
            "search",
            "search-replace",
            "explorer",
            "git",
            "terminal-jump-to-previous-command",
            "terminal-jump-to-next-command",
            "unknown.future",
            "window.reload",
            "view.welcome",
            "tab.close",
            "tab.moveToNewWindow",
            "sync.uploadNow",
            "git.revertHead",
            "task.runTask",
            "monaco.editor.action.commentLine",
        ] {
            assert!(!accepts(unavailable, true, &project), "{unavailable}");
        }
        for command in ["settings.open", "app.openSettingsFile", "file.quickOpen"] {
            assert!(accepts(command, false, &context), "{command}");
        }
        for edit in ["monaco.deleteAllLeft", "monaco.editor.action.outdentLines"] {
            assert!(!accepts(edit, true, &context), "{edit}");
            assert!(accepts(edit, true, &editor_context(false)), "{edit}");
            assert!(!accepts(edit, true, &editor_context(true)), "{edit}");
        }
        assert!(accepts(
            "monaco.taide.saveFile",
            true,
            &editor_context(true)
        ));
        for (action, _) in FOLD_ACTIONS {
            let fold = format!("monaco.{action}");
            assert!(!accepts(&fold, true, &context), "{fold}");
            assert!(accepts(&fold, true, &editor_context(true)), "{fold}");
            assert!(
                !accepts(&fold, true, &editor_actions(false, false)),
                "{fold}"
            );
        }
        assert!(!accepts(
            "view.toggleZenMode",
            true,
            &CommandContext {
                window: WindowKind::Auxiliary,
                ..Default::default()
            }
        ));
    }

    #[tokio::test]
    async fn 명령_실행은_keymap액션과_명령을_같은_typed_intent로_해석한다() {
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-command-dispatch-{project}")),
        ));
        let mut layout = taide_layout::service::default_layout();
        let pane = layout.focused_pane.clone();
        let make_tab = |kind| Tab {
            id: TabId::new(),
            kind,
            title: "synthetic command".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        };
        let editor = make_tab(TabKind::Untitled { index: 1 });
        let terminal = make_tab(TabKind::Terminal {
            session_id: String::new(),
            cwd: None,
        });
        layout.root = PaneNode::Leaf {
            id: pane.clone(),
            active: Some(editor.id.clone()),
            tabs: vec![editor.clone(), terminal.clone()],
        };
        state.layouts.write().insert(project.clone(), layout);
        let unfocused = ShellSnapshot::read(&state).await;
        let slot = ShellSlotId::new();
        {
            let mut session = state.session.write();
            session.shell_slots = Some(ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            });
            session.focused_shell_slot = Some(slot.clone());
        }
        let snapshot = ShellSnapshot::read(&state).await;
        let main = context(&snapshot, &WindowScope::Main, None);
        assert_eq!(main.active_project.as_ref(), Some(&project));
        assert_eq!(main.focused_shell_slot.as_ref(), Some(&slot));
        assert_eq!(main.window, WindowKind::Main);
        assert!(main.active_editor_actions.is_none());
        let other = ProjectId::new();
        let auxiliary = context(
            &snapshot,
            &WindowScope::Auxiliary {
                project: other.clone(),
                slot: AUXILIARY_SLOT,
            },
            None,
        );
        assert_eq!(auxiliary.active_project.as_ref(), Some(&other));
        assert!(auxiliary.focused_shell_slot.is_none());
        assert_eq!(auxiliary.window, WindowKind::Auxiliary);
        assert!(
            context(&unfocused, &WindowScope::Main, None)
                .active_project
                .is_none()
        );

        assert!(matches!(
            intent("toggle-sidebar", &main, &snapshot),
            Some(ShellIntent::Mutate(ShellMutation::SetSidebarCollapsed { project: target, .. })) if target == project
        ));
        assert!(matches!(
            intent("view.toggleSidebar", &main, &snapshot),
            Some(ShellIntent::Mutate(ShellMutation::SetSidebarCollapsed { project: target, .. })) if target == project
        ));
        assert!(intent("toggle-sidebar", &main, &unfocused).is_none());
        for scope in [&snapshot, &unfocused] {
            assert!(matches!(
                intent("settings.open", &main, scope),
                Some(ShellIntent::OpenSettings)
            ));
            assert!(matches!(
                intent("app.openSettingsFile", &main, scope),
                Some(ShellIntent::OpenSettingsFile)
            ));
        }
        for unavailable in ["sync.uploadNow", "tab.close", "unknown.future"] {
            assert!(
                intent(unavailable, &main, &snapshot).is_none(),
                "{unavailable}"
            );
        }
        for (id, entry) in PALETTE_ENTRIES
            .into_iter()
            .chain([("file.quickOpen", PaletteEntry::Files)])
        {
            for scope in [&snapshot, &unfocused] {
                assert!(
                    matches!(
                        intent(id, &main, scope),
                        Some(ShellIntent::OpenPalette(opened)) if opened == entry
                    ),
                    "{id}"
                );
            }
        }
        assert!(matches!(
            intent("view.toggleZenMode", &main, &snapshot),
            Some(ShellIntent::Mutate(ShellMutation::SetWindowChrome(patch))) if patch.zen == Some(true)
        ));
        assert!(intent("view.toggleZenMode", &auxiliary, &snapshot).is_none());

        let writable = CommandContext {
            active_editor_actions: editor_context(false).active_editor_actions,
            ..main.clone()
        };
        for (id, expected) in [
            ("monaco.deleteAllLeft", DocumentEdit::DeleteAllLeft),
            (
                "monaco.editor.action.outdentLines",
                DocumentEdit::OutdentLines,
            ),
        ] {
            assert!(matches!(
                intent(id, &writable, &snapshot),
                Some(ShellIntent::EditDocument { tab, edit }) if tab == editor.id && edit == expected
            ));
            assert!(intent(id, &main, &snapshot).is_none(), "{id}");
        }
        for save in ["monaco.taide.saveFile", "editor.save", "save"] {
            assert!(matches!(
                intent(save, &writable, &snapshot),
                Some(ShellIntent::RequestSaveTab(tab)) if tab == editor.id
            ));
        }
        for (action, expected) in FOLD_ACTIONS {
            let fold = format!("monaco.{action}");
            assert!(
                matches!(
                    intent(&fold, &writable, &snapshot),
                    Some(ShellIntent::FoldDocument { tab, command })
                        if tab == editor.id && command == expected
                ),
                "{fold}"
            );
            assert!(intent(&fold, &main, &snapshot).is_none(), "{fold}");
        }
        let mut terminal_active = snapshot.clone();
        let PaneNode::Leaf { active, .. } =
            &mut terminal_active.layouts.get_mut(&project).unwrap().root
        else {
            panic!("expected leaf")
        };
        *active = Some(terminal.id.clone());
        for document_command in [
            "monaco.deleteAllLeft",
            "monaco.editor.action.outdentLines",
            "monaco.taide.saveFile",
            "monaco.editor.toggleFold",
            "save",
        ] {
            assert!(
                intent(document_command, &writable, &terminal_active).is_none(),
                "{document_command}"
            );
        }
        if state.paths.data_dir.exists() {
            std::fs::remove_dir_all(&state.paths.data_dir).unwrap();
        }
    }

    #[test]
    fn 활성_editor_액션은_연결된_view가_있는_탭에서만_native_실행가능_집합을_돌려준다() {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: UNDO_GROUPS,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), "", "plaintext".into())
            .unwrap();
        let key = ViewKey {
            window: "main".into(),
            pane: PaneId::new(),
            tab,
        };
        store.attach_view(key.clone(), document).unwrap();
        let actions = active_editor_actions(&store, Some(&key)).unwrap();
        for action in [
            "deleteAllLeft",
            "editor.action.outdentLines",
            "taide.saveFile",
            "editor.action.moveLinesUpAction",
            "editor.action.transpose",
            "editor.action.insertCursorBelow",
            "cursorUndo",
            "editor.action.smartSelect.expand",
            "editor.action.triggerSuggest",
            "editor.action.resetSuggestSize",
        ]
        .into_iter()
        .chain(FOLD_ACTIONS.map(|(action, _)| action))
        {
            assert!(actions.contains(action), "{action}");
        }
        for action in ["editor.action.formatDocument"] {
            assert!(!actions.contains(action), "{action}");
        }
        let detached = ViewKey {
            tab: TabId::new(),
            ..key
        };
        assert!(active_editor_actions(&store, Some(&detached)).is_none());
        assert!(active_editor_actions(&store, None).is_none());
    }

    #[test]
    fn 접기_요청은_대상_탭의_것만_들어온_순서로_꺼내고_다른_탭_요청을_남긴다() {
        let tab = TabId::new();
        let other = TabId::new();
        let mut pending = vec![
            (tab.clone(), FoldCommand::FoldAll),
            (other.clone(), FoldCommand::Fold),
            (tab.clone(), FoldCommand::Unfold),
        ];
        assert_eq!(
            take_fold_commands(&tab, &mut pending),
            [FoldCommand::FoldAll, FoldCommand::Unfold]
        );
        assert_eq!(pending, [(other, FoldCommand::Fold)]);
        assert!(take_fold_commands(&tab, &mut pending).is_empty());
    }

    #[test]
    fn 문서_편집_요청은_대상_탭의_view에만_개별_undo단계로_적용되고_다른_탭_요청을_남긴다() {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: UNDO_GROUPS,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), "    alpha beta", "plaintext".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "main".into(),
                    pane: PaneId::new(),
                    tab: tab.clone(),
                },
                document,
            )
            .unwrap();
        move_selection(&mut store, view, Motion::DocumentEnd, false).unwrap();
        let text = |store: &EditorStore| {
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string()
        };
        let indent = IndentOptions {
            tab_size: INDENT_WIDTH,
            insert_spaces: true,
        };
        let indent = LineCommandContext {
            indent,
            language: None,
            syntax: &UntokenizedLines,
            compare: None,
            transforms: None,
            word_rules: None,
        };
        let other = TabId::new();
        let mut pending = vec![
            (tab.clone(), DocumentEdit::OutdentLines),
            (other.clone(), DocumentEdit::DeleteAllLeft),
            (
                tab.clone(),
                DocumentEdit::Find(taide_native_ui::editor_find::FindCommand::Open),
            ),
            (
                tab.clone(),
                DocumentEdit::Problem(
                    taide_native_editor::problem_navigation::Command::NextInFiles,
                ),
            ),
        ];
        let mut errors = Vec::new();
        assert!(apply_document_edits(
            &mut store,
            view,
            &tab,
            indent,
            indentation_configuration(),
            &mut pending,
            &mut errors
        ));
        assert_eq!(text(&store), "alpha beta");
        assert_eq!(
            pending,
            [
                (other, DocumentEdit::DeleteAllLeft),
                (
                    tab.clone(),
                    DocumentEdit::Find(taide_native_ui::editor_find::FindCommand::Open),
                ),
                (
                    tab.clone(),
                    DocumentEdit::Problem(
                        taide_native_editor::problem_navigation::Command::NextInFiles
                    )
                ),
            ]
        );
        assert!(!apply_document_edits(
            &mut store,
            view,
            &tab,
            indent,
            indentation_configuration(),
            &mut pending,
            &mut errors
        ));
        assert_eq!(text(&store), "alpha beta");
        pending.push((tab.clone(), DocumentEdit::DeleteAllLeft));
        assert!(apply_document_edits(
            &mut store,
            view,
            &tab,
            indent,
            indentation_configuration(),
            &mut pending,
            &mut errors
        ));
        assert_eq!(text(&store), "");
        assert_eq!(pending.len(), 3);
        assert!(errors.is_empty());
        assert!(store.undo(document).unwrap());
        assert_eq!(text(&store), "alpha beta");
        assert!(store.undo(document).unwrap());
        assert_eq!(text(&store), "    alpha beta");
    }
}
