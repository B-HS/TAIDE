use super::inlay_hints_tests::{
    Fixture, body_view, fixture_with_provider, key, paint, text_contains, wait,
};
use super::selection_ranges_tests::run_palette;
use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};
use taide_native_editor::view::{Selection, SelectionSet};
use taide_native_ui::editor_context_menu::{Action, Invocation, Session};

const TEXT: &str = "class\n  \u{1f600}method\nend";
const EMPTY: egui::Modifiers = egui::Modifiers::NONE;
const SHIFT: egui::Modifiers = egui::Modifiers {
    shift: true,
    ..EMPTY
};

fn fixture() -> Fixture {
    fixture_with_provider(
        "--native-inlay-hints",
        false,
        |application, project, document| {
            !application
                .lsp
                .as_ref()
                .unwrap()
                .location_providers(
                    project,
                    &application.store.documents().snapshot(document).unwrap(),
                    taide_native_editor::symbol_locations::Kind::Definition,
                )
                .is_empty()
        },
    )
}

fn target(
    application: &NativeApplication,
    context: &egui::Context,
    project: &ProjectId,
    source: ViewId,
    owner: ViewId,
    action: Action,
) -> crate::editor_context_menu::Request {
    crate::editor_context_menu::Request {
        invocation: Invocation {
            session: Session::open(
                &application.store,
                source,
                owner,
                context.viewport_id(),
                egui::Pos2::ZERO,
                Vec::new(),
            )
            .unwrap(),
            action,
        },
        context: context.clone(),
        project: Some(project.clone()),
        scope: WindowScope::Main,
        owner_key: application.store.views().get(owner).unwrap().key.clone(),
    }
}

fn select(application: &mut NativeApplication, view: ViewId, anchor: usize, head: usize) {
    let current = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor, head }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
}

fn focus(application: &mut NativeApplication, context: &egui::Context, view: ViewId) {
    paint(application, context, Vec::new());
    let id = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(id));
    paint(application, context, Vec::new());
}

#[derive(Default)]
struct Clipboard {
    text: RwLock<String>,
    writes: Mutex<Vec<String>>,
    fail: AtomicBool,
    attempts: AtomicUsize,
}

fn clipboard(application: &mut NativeApplication, context: &egui::Context) -> Arc<Clipboard> {
    let clipboard = Arc::new(Clipboard::default());
    let writer = clipboard.clone();
    let reader = clipboard.clone();
    let repaint = context.clone();
    let runtime = application.runtime.handle().clone();
    runtime.block_on(async {
        let bridge = HostBridge::connect_with_clipboard_ports(
            application.services.clone(),
            Arc::new(move || repaint.request_repaint()),
            Arc::new(move |text| {
                writer.attempts.fetch_add(1, Ordering::SeqCst);
                if writer.fail.load(Ordering::SeqCst) {
                    return Err(AppError::Internal("fixture clipboard refused".into()));
                }
                writer.writes.lock().unwrap().push(text.into());
                *writer.text.write().unwrap() = text.into();
                Ok(())
            }),
            Arc::new(move || Ok(reader.text.read().unwrap().clone())),
            None,
        )
        .unwrap();
        let old = application.bridge.replace(bridge).unwrap();
        old.disconnect().await.unwrap();
    });
    clipboard
}

fn menu_action(
    application: &mut NativeApplication,
    context: &egui::Context,
    view: ViewId,
    label: &str,
) -> egui::FullOutput {
    focus(application, context, view);
    let opened = paint(application, context, key(egui::Key::F10, SHIFT));
    assert!(
        text_contains(&opened, "Copy"),
        "menu did not open: action={label}, view={view:?}, status={:?}, focus={:?}",
        application.status,
        context.memory(|memory| memory.focused())
    );
    let mut events = vec![egui::Event::Text(label.into())];
    events.extend(key(egui::Key::Enter, EMPTY));
    paint(application, context, events)
}

#[test]
fn 실제_본문_메뉴의_직렬_키와_클립보드는_문서_undo와_dirty를_보존한다() {
    let mut fixture = fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    let clipboard = clipboard(application, context);
    select(application, view, 0, 0);
    menu_action(application, context, view, "Copy");
    wait(application, context, "menu copy", |_| {
        clipboard.writes.lock().unwrap().len() == 1
    });
    assert_eq!(*clipboard.text.read().unwrap(), "class\n");
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
    menu_action(application, context, view, "Cut");
    wait(application, context, "menu cut", |application| {
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
            == "  \u{1f600}method\nend"
    });
    menu_action(application, context, view, "Paste");
    wait(application, context, "menu paste", |application| {
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
            == TEXT
    });
    assert!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
    assert!(application.store.undo(document).unwrap());
    paint(application, context, Vec::new());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "  \u{1f600}method\nend"
    );
    assert!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
    assert!(application.store.undo(document).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
    assert!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
}

#[test]
fn 실제_메뉴의_잘라내기_실패와_늦은_붙여넣기는_내용을_바꾸지_않는다() {
    let mut fixture = fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    let clipboard = clipboard(application, context);
    clipboard.fail.store(true, Ordering::SeqCst);
    select(application, view, 0, 5);
    menu_action(application, context, view, "Cut");
    wait(application, context, "failed clipboard", |application| {
        clipboard.attempts.load(Ordering::SeqCst) == 1
            && !application.editor_clipboard.has_pending(view)
    });
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
    *clipboard.text.write().unwrap() = "late".into();
    let request = target(
        application,
        context,
        &fixture.project,
        view,
        view,
        Action::Paste,
    );
    assert!(application.run_editor_context_menu(request).unwrap());
    select(application, view, 5, 5);
    paint(application, context, Vec::new());
    wait(
        application,
        context,
        "stale paste discarded",
        |application| !application.editor_clipboard.has_pending(view),
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
    assert!(
        !application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
}

#[test]
fn 실제_본문과_peek_메뉴는_원본_레이블과_공급자_범위를_유지한다() {
    let mut fixture = fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let clipboard = clipboard(application, context);
    application
        .locale
        .messages
        .insert("editor.toggleMinimap".into(), "미니맵 전환".into());
    application.services.state.settings.write().keymap_overrides = Some(
        r#"[{"actionId":"monaco.editor.action.revealDefinition","key":"d","mods":["mod"]}]"#.into(),
    );
    focus(application, context, view);
    let output = paint(application, context, key(egui::Key::F10, SHIFT));
    assert!(text_contains(&output, "Go to Definition"));
    assert!(text_contains(&output, "미니맵 전환"));
    assert!(text_contains(&output, "⌘D"));
    paint(application, context, key(egui::Key::Escape, EMPTY));
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Request {
            kind: taide_native_editor::symbol_locations::Kind::Definition,
            mode: taide_native_editor::symbol_locations::Mode::Peek,
        }),
    ));
    wait(application, context, "menu peek", |application| {
        application
            .editor_locations
            .current(view)
            .is_some_and(|session| session.preview.is_some())
    });
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    focus(application, context, preview);
    let output = paint(application, context, key(egui::Key::F10, SHIFT));
    assert!(text_contains(&output, "Copy"));
    assert!(text_contains(&output, "Go to Definition"));
    assert!(!text_contains(&output, "미니맵 전환"));
    let document = application.store.views().get(preview).unwrap().document;
    let before = application.store.documents().snapshot(document).unwrap();
    paint(application, context, key(egui::Key::Escape, EMPTY));
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        before.revision
    );
    select(application, preview, 8, 18);
    menu_action(application, context, preview, "Copy");
    wait(application, context, "peek copy", |_| {
        clipboard.writes.lock().unwrap().len() == 1
    });
    assert_eq!(*clipboard.text.read().unwrap(), "\u{1f600}method");
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        before.revision
    );
    application.editor_locations.close(view);
    select(application, view, 0, 0);
    menu_action(application, context, view, "Go to Definition");
    let trace: Option<(String, bool, bool)> =
        context.data(|data| data.get_temp(egui::Id::new("native-editor-context-menu-invocation")));
    assert_eq!(
        trace,
        Some(("editor.action.revealDefinition".into(), true, true)),
        "menu invocation trace: {trace:?}"
    );
    let handled: Option<String> =
        context.data(|data| data.get_temp(egui::Id::new("native-editor-context-menu-result")));
    assert_eq!(handled.as_deref(), Some("Ok(true)"));
    wait(
        application,
        context,
        "menu definition navigation",
        |application| {
            application.store.views().get(view).is_some_and(|view| {
                view.selection.selections
                    == vec![Selection {
                        anchor: 12,
                        head: 12,
                    }]
            }) || application
                .editor_locations
                .current(view)
                .is_none_or(|session| session.model.is_some())
        },
    );
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: 12,
            head: 12
        }],
        "location={:?}",
        application.editor_locations.current(view).map(|session| (
            &session.request.kind,
            &session.request.position,
            session.model.as_ref().map(|model| model.targets())
        ))
    );
}

#[test]
fn 실제_터미널_대상은_같은_pane의_기존_탭을_우선하고_새_탭은_준비까지_보존한다() {
    let mut fixture = fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let source = target(
        application,
        context,
        &fixture.project,
        view,
        view,
        Action::RunSelectedText,
    );
    let pane = source.owner_key.pane.clone();
    let runtime = application.runtime.handle().clone();
    let first = runtime
        .block_on(crate::editor_context_menu::terminal_target(
            &application.services,
            crate::editor_context_menu::TerminalRequest {
                target: source,
                payload: "class\n".into(),
                title: "Terminal".into(),
            },
        ))
        .unwrap();
    assert_eq!(
        first.session(&application.services.state).as_deref(),
        Some("")
    );
    let terminal = first.tab.clone();
    assert_eq!(first.payload, "class\n");
    runtime
        .block_on(taide_runtime::layout_actions::layout_activate_tab(
            application.services.events.as_ref(),
            &application.services.state,
            fixture.tab.clone(),
        ))
        .unwrap();
    let second = runtime
        .block_on(crate::editor_context_menu::terminal_target(
            &application.services,
            crate::editor_context_menu::TerminalRequest {
                target: target(
                    application,
                    context,
                    &fixture.project,
                    view,
                    view,
                    Action::RunSelectedText,
                ),
                payload: "second\n".into(),
                title: "Terminal".into(),
            },
        ))
        .unwrap();
    assert_eq!(second.tab, terminal);
    assert_eq!(second.payload, "second\n");
    let layouts = application.services.state.layouts.read();
    let taide_model::layout::PaneNode::Leaf { tabs, active, .. } =
        taide_layout::service::find_leaf(&layouts[&fixture.project].root, &pane).unwrap()
    else {
        panic!();
    };
    assert_eq!(
        tabs.iter()
            .filter(|tab| matches!(tab.kind, TabKind::Terminal { .. }))
            .count(),
        1
    );
    assert_eq!(active.as_ref(), Some(&terminal));
    drop(layouts);
    application.editor_terminal_writes.push(first);
    application.editor_terminal_writes.push(second);
    assert!(application.flush_dirty());
    assert_eq!(application.editor_terminal_writes.len(), 2);
    application
        .services
        .state
        .layouts
        .write()
        .remove(&fixture.project);
    assert!(application.flush_dirty());
    assert!(application.editor_terminal_writes.is_empty());
}

fn terminal_text(application: &NativeApplication, project: &ProjectId) -> Option<String> {
    let layouts = application.services.state.layouts.read();
    let layout = layouts.get(project)?;
    let session = crate::tabs::tabs_in(&layout.root)
        .into_iter()
        .find_map(|tab| match &tab.kind {
            TabKind::Terminal { session_id, .. } => application.terminals.hub().get(session_id),
            _ => None,
        })?;
    session
        .snapshot(|snapshot| {
            snapshot
                .core
                .content()
                .map(|content| content.display_iter.map(|cell| cell.c).collect())
        })
        .ok()?
        .ok()
}

#[test]
fn 실제_메뉴의_현재줄과_선택은_준비된_동일_터미널에_raw로_전달된다() {
    let mut fixture = fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    application.services.state.settings.write().shell_override = Some("/bin/cat".into());
    let label = presentation::message(&application.locale, "terminal.runSelectedText", &[]);
    let view = body_view(application, &fixture.path, &fixture.tab);
    select(application, view, 0, 0);
    menu_action(application, context, view, &label);
    wait(
        application,
        context,
        "terminal current line",
        |application| {
            terminal_text(application, &fixture.project).is_some_and(|text| text.contains("class"))
        },
    );
    let sessions = application
        .services
        .terminal
        .sessions_for_project(&fixture.project);
    assert_eq!(sessions.len(), 1);
    let session = sessions[0].id.clone();
    assert!(application.editor_terminal_writes.is_empty());
    let runtime = application.runtime.handle().clone();
    runtime
        .block_on(taide_runtime::layout_actions::layout_activate_tab(
            application.services.events.as_ref(),
            &application.services.state,
            fixture.tab.clone(),
        ))
        .unwrap();
    paint(application, context, Vec::new());
    let view = body_view(application, &fixture.path, &fixture.tab);
    select(application, view, 8, 18);
    menu_action(application, context, view, &label);
    wait(application, context, "terminal selection", |application| {
        terminal_text(application, &fixture.project)
            .is_some_and(|text| text.contains("method") && text.contains('\u{1f600}'))
    });
    let sessions = application
        .services
        .terminal
        .sessions_for_project(&fixture.project);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, session);
    assert!(application.editor_terminal_writes.is_empty());
    assert!(terminal_text(application, &fixture.project).is_some_and(|text| !text.contains("end")));
}

#[test]
fn 실제_추가_메뉴는_readonly_tier와_lossy의_공급_조건을_보존한다() {
    let mut fixture = fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    let clipboard = clipboard(application, context);
    use taide_model::file::FileSizeTier;
    for (tier, read_only, lossy, definition) in [
        (FileSizeTier::Normal, false, false, true),
        (FileSizeTier::Normal, true, false, true),
        (FileSizeTier::Large, true, false, false),
        (FileSizeTier::ReadOnly, true, false, false),
        (FileSizeTier::Normal, true, true, false),
        (FileSizeTier::Normal, false, false, true),
    ] {
        let mut file =
            taide_file::service::open_file(std::path::Path::new(&fixture.path), &[], false)
                .unwrap();
        file.tier = tier;
        file.read_only = read_only;
        file.encoding_lossy = lossy;
        application
            .store
            .observe_file(document, std::path::Path::new(&fixture.path), file)
            .unwrap();
        focus(application, context, view);
        let output = paint(application, context, key(egui::Key::F10, SHIFT));
        assert!(text_contains(&output, "Copy"));
        assert_eq!(text_contains(&output, "Cut"), !read_only);
        assert_eq!(text_contains(&output, "Paste"), !read_only);
        assert_eq!(text_contains(&output, "Change All Occurrences"), !read_only);
        assert_eq!(text_contains(&output, "Go to Definition"), definition);
        paint(application, context, key(egui::Key::Escape, EMPTY));
        assert_eq!(
            application
                .store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            TEXT
        );
    }
    let mut file =
        taide_file::service::open_file(std::path::Path::new(&fixture.path), &[], false).unwrap();
    file.read_only = true;
    application
        .store
        .observe_file(document, std::path::Path::new(&fixture.path), file)
        .unwrap();
    select(application, view, 8, 18);
    menu_action(application, context, view, "Copy");
    wait(application, context, "readonly menu copy", |_| {
        clipboard.writes.lock().unwrap().len() == 1
    });
    assert_eq!(*clipboard.text.read().unwrap(), "\u{1f600}method");
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
}

#[test]
fn 실제_추가_팔레트_메뉴는_본문과_peek의_원래_입력_대상을_보존한다() {
    let mut fixture = fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let clipboard = clipboard(application, context);
    focus(application, context, view);
    run_palette(application, context, "monaco.editor.action.showContextMenu");
    wait(
        application,
        context,
        "body palette menu command",
        |application| application.document_edits.is_empty(),
    );
    let output = paint(application, context, Vec::new());
    assert!(
        text_contains(&output, "Copy"),
        "body palette menu missing; palette={:?}; commands={:?}; edits={:?}; menu={:?}; selection={:?}; focus={:?}",
        application.palette.inspection(),
        application.palette_commands,
        application.document_edits,
        context.data(
            |data| data.get_temp::<String>(egui::Id::new("native-editor-context-menu-result"))
        ),
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        context.memory(|memory| memory.focused())
    );
    assert!(
        context.data(|data| data.get_temp::<String>(egui::Id::new("native-editor-context-menu-result"))).is_none(),
        "palette confirmation must not execute a menu item"
    );
    let mut events = vec![egui::Event::Text("Copy".into())];
    events.extend(key(egui::Key::Enter, EMPTY));
    paint(application, context, events);
    wait(application, context, "palette body copy", |_| {
        clipboard.writes.lock().unwrap().len() == 1
    });
    assert_eq!(*clipboard.text.read().unwrap(), "class\n");
    select(application, view, 0, 0);
    focus(application, context, view);
    run_palette(application, context, "monaco.editor.action.peekDefinition");
    wait(application, context, "palette menu peek", |application| {
        application
            .editor_locations
            .current(view)
            .is_some_and(|session| session.preview.is_some())
    });
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    select(application, preview, 8, 18);
    focus(application, context, preview);
    run_palette(application, context, "monaco.editor.action.showContextMenu");
    wait(
        application,
        context,
        "peek palette menu command",
        |application| application.document_edits.is_empty(),
    );
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, "Copy"), "peek palette menu missing");
    let mut events = vec![egui::Event::Text("Copy".into())];
    events.extend(key(egui::Key::Enter, EMPTY));
    paint(application, context, events);
    wait(application, context, "palette peek copy", |_| {
        clipboard.writes.lock().unwrap().len() == 2
    });
    assert_eq!(*clipboard.text.read().unwrap(), "\u{1f600}method");
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection { anchor: 0, head: 0 }]
    );
    focus(application, context, preview);
    run_palette(application, context, "monaco.editor.action.showContextMenu");
    application.editor_locations.close(view);
    wait(
        application,
        context,
        "closed peek menu command",
        |application| application.document_edits.is_empty(),
    );
    let output = paint(application, context, Vec::new());
    assert!(!text_contains(&output, "Copy"));
    assert_eq!(clipboard.writes.lock().unwrap().len(), 2);
    let document = application.store.views().get(view).unwrap().document;
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
}
