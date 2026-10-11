use super::inlay_hints_tests::{Fixture, body_view, fixture_with_provider, key, paint, wait};
use super::*;
use std::path::Path;
use taide_native_editor::cursor_commands::CursorCommand;
use taide_native_editor::view::{Selection, SelectionSet};

const TEXT: &str = "class\n  \u{1f600}method\nend";
const WORD: &str = "\u{1f600}method";
const STABLE_FRAMES: usize = 16;
const SMART_KEYS: egui::Modifiers = egui::Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: true,
    command: true,
};

fn fixture(mode: &str) -> Fixture {
    fixture_with_provider(mode, false, |application, project, document| {
        !application
            .lsp
            .as_ref()
            .unwrap()
            .selection_range_providers(
                project,
                &application.store.documents().snapshot(document).unwrap(),
            )
            .is_empty()
    })
}

fn point(application: &mut NativeApplication, view: ViewId, byte: usize) {
    let current = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: byte,
                    head: byte,
                }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
}

fn selection(application: &NativeApplication, view: ViewId) -> Selection {
    application
        .store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections[0]
}

fn focus(application: &mut NativeApplication, context: &egui::Context, view: ViewId) {
    paint(application, context, Vec::new());
    let id = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (source, _))| {
            (*viewport == context.viewport_id() && *source == view).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(id));
    paint(application, context, Vec::new());
}

pub(super) fn run_palette(
    application: &mut NativeApplication,
    context: &egui::Context,
    command: &str,
) {
    application.palette.open(
        context,
        taide_native_ui::command_registry::PaletteEntry::Commands,
    );
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    assert!(application.palette.inspection().has_input_focus);
    let label = crate::command_registry::registry()
        .unwrap()
        .command(command)
        .unwrap()
        .label(&application.locale);
    paint(application, context, vec![egui::Event::Text(label)]);
    assert!(
        application
            .palette
            .inspection()
            .rows
            .iter()
            .any(|row| row.key == command && row.is_enabled && row.is_selected),
        "{:?}",
        application.palette.inspection()
    );
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
}

fn diagnostic(application: &NativeApplication, label: &str) -> bool {
    application
        .lsp_diagnostics
        .batches()
        .any(|batch| batch.diagnostics.iter().any(|item| item.message == label))
}

fn exit(application: &mut NativeApplication) {
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_app의_같은프레임_줄선택과_스마트확장은_명령_순서를_유지한다() {
    let mut fixture = fixture("--native-selection-ranges");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = body_view(application, &fixture.path, &fixture.tab);
    point(application, view, TEXT.find("method").unwrap() + 2);
    focus(application, context, view);
    application.document_edits.extend([
        (
            fixture.tab.clone(),
            DocumentEdit::Cursor(CursorCommand::ExpandLine),
        ),
        (
            fixture.tab.clone(),
            DocumentEdit::Cursor(CursorCommand::Expand),
        ),
    ]);
    paint(application, context, Vec::new());
    wait(
        application,
        context,
        "ordered smart selection",
        |application| {
            taide_native_editor::cursor_commands::has_smart_selection(&application.store, view)
        },
    );
    assert_eq!(
        selection(application, view),
        Selection {
            anchor: 0,
            head: TEXT.len()
        }
    );
    let snapshot = application.store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.revision, 0);
    assert!(!snapshot.dirty);
    assert_eq!(snapshot.rope.to_string(), TEXT);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    exit(application);
}

#[test]
fn 실제_app의_기본_키는_utf16_범위를_단어와_합치고_readonly_확장축소를_재사용한다() {
    let mut fixture = fixture("--native-selection-ranges");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let start = TEXT.find(WORD).unwrap();
    let caret = TEXT.find("method").unwrap() + 2;
    point(application, view, caret);
    focus(application, context, view);
    let before = application.store.documents().snapshot(document).unwrap();
    paint(application, context, key(egui::Key::ArrowRight, SMART_KEYS));
    wait(application, context, "body smart word", |application| {
        taide_native_editor::cursor_commands::has_smart_selection(&application.store, view)
            && diagnostic(application, "synthetic selection requested 1")
    });
    assert_eq!(
        selection(application, view),
        Selection {
            anchor: start,
            head: start + WORD.len()
        }
    );
    assert!(diagnostic(application, "synthetic selection requested 1"));
    let mut file = taide_file::service::open_file(Path::new(&fixture.path), &[], false).unwrap();
    file.read_only = true;
    application
        .store
        .observe_file(document, Path::new(&fixture.path), file)
        .unwrap();
    paint(application, context, key(egui::Key::ArrowRight, SMART_KEYS));
    paint(application, context, Vec::new());
    assert_eq!(
        selection(application, view),
        Selection {
            anchor: TEXT.find('\n').unwrap() + 1,
            head: start + WORD.len()
        }
    );
    for _ in 0..STABLE_FRAMES {
        paint(application, context, Vec::new());
    }
    assert!(!diagnostic(application, "synthetic selection requested 2"));
    for expected in [
        Selection {
            anchor: start,
            head: start + WORD.len(),
        },
        Selection {
            anchor: caret,
            head: caret,
        },
    ] {
        paint(application, context, key(egui::Key::ArrowLeft, SMART_KEYS));
        paint(application, context, Vec::new());
        assert_eq!(selection(application, view), expected);
    }
    let after = application.store.documents().snapshot(document).unwrap();
    assert!(after.metadata.read_only);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.dirty, before.dirty);
    assert_eq!(after.rope.to_string(), TEXT);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    exit(application);
    assert!(!taide_native_editor::cursor_commands::has_smart_selection(
        &application.store,
        view
    ));
}

#[test]
fn 실제_app의_peek_팔레트와_기본_키는_원래_입력_대상을_보존하고_닫힘을_회수한다() {
    let mut fixture = fixture("--native-selection-ranges");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let start = TEXT.find(WORD).unwrap();
    let caret = TEXT.find("method").unwrap() + 2;
    point(application, view, caret);
    focus(application, context, view);
    run_palette(application, context, "monaco.editor.action.peekDefinition");
    wait(application, context, "selection preview", |application| {
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
    let body_selection = selection(application, view);
    point(application, preview, caret);
    focus(application, context, preview);
    assert_eq!(
        application.focused_editor_source(context, &fixture.tab, Some(document)),
        Some((preview, view))
    );
    run_palette(
        application,
        context,
        "monaco.editor.action.smartSelect.expand",
    );
    wait(application, context, "preview smart word", |application| {
        taide_native_editor::cursor_commands::has_smart_selection(&application.store, preview)
            && diagnostic(application, "synthetic selection requested 1")
    });
    assert_eq!(
        selection(application, preview),
        Selection {
            anchor: start,
            head: start + WORD.len()
        }
    );
    assert_eq!(selection(application, view), body_selection);
    focus(application, context, preview);
    paint(application, context, key(egui::Key::ArrowRight, SMART_KEYS));
    wait(application, context, "preview smart line", |application| {
        selection(application, preview)
            == Selection {
                anchor: TEXT.find('\n').unwrap() + 1,
                head: start + WORD.len(),
            }
    });
    assert_eq!(selection(application, view), body_selection);
    run_palette(
        application,
        context,
        "monaco.editor.action.smartSelect.shrink",
    );
    wait(
        application,
        context,
        "preview smart shrink",
        |application| {
            selection(application, preview)
                == Selection {
                    anchor: start,
                    head: start + WORD.len(),
                }
        },
    );
    assert_eq!(selection(application, view), body_selection);
    focus(application, context, preview);
    paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    assert!(application.editor_locations.current(view).is_none());
    assert!(application.store.views().get(preview).is_none());
    let snapshot = application.store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.revision, 0);
    assert!(!snapshot.dirty);
    assert_eq!(snapshot.rope.to_string(), TEXT);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    exit(application);
}

#[test]
fn 실제_app의_재지정_키와_다중커서와_ime와_대기취소는_문서를_편집하지_않는다() {
    {
        let mut fixture = fixture("--native-selection-ranges");
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let document = application.files[&fixture.path].id;
        let view = body_view(application, &fixture.path, &fixture.tab);
        let caret = TEXT.find("method").unwrap() + 2;
        let start = TEXT.find(WORD).unwrap();
        let current = application.store.views().get(view).unwrap().clone();
        application
            .store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![
                        Selection {
                            anchor: caret,
                            head: caret,
                        },
                        Selection { anchor: 2, head: 2 },
                    ],
                },
                current.scroll,
                current.folds,
            )
            .unwrap();
        application.services.state.settings.write().keymap_overrides = Some(r#"[{"actionId":"monaco.editor.action.smartSelect.expand","key":"F9","mods":[]},{"actionId":"monaco.editor.action.smartSelect.shrink","key":"F10","mods":[]}]"#.into());
        focus(application, context, view);
        paint(
            application,
            context,
            key(egui::Key::F9, egui::Modifiers::NONE),
        );
        wait(
            application,
            context,
            "multicursor smart range",
            |application| {
                taide_native_editor::cursor_commands::has_smart_selection(&application.store, view)
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
            vec![
                Selection {
                    anchor: start,
                    head: start + WORD.len()
                },
                Selection {
                    anchor: 0,
                    head: "class".len()
                }
            ]
        );
        assert!(diagnostic(application, "synthetic selection requested 1"));
        paint(
            application,
            context,
            key(egui::Key::F10, egui::Modifiers::NONE),
        );
        paint(application, context, Vec::new());
        assert_eq!(
            application
                .store
                .views()
                .get(view)
                .unwrap()
                .selection
                .selections,
            vec![
                Selection {
                    anchor: caret,
                    head: caret
                },
                Selection { anchor: 2, head: 2 }
            ]
        );
        point(application, view, caret);
        paint(
            application,
            context,
            vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                text: "pending".into(),
                active_range_chars: None,
            })],
        );
        assert!(
            application
                .store
                .views()
                .get(view)
                .unwrap()
                .composition
                .is_some()
        );
        paint(
            application,
            context,
            key(egui::Key::F9, egui::Modifiers::NONE),
        );
        assert!(!taide_native_editor::cursor_commands::has_smart_selection(
            &application.store,
            view
        ));
        assert!(!diagnostic(application, "synthetic selection requested 2"));
        paint(
            application,
            context,
            vec![egui::Event::Ime(egui::ImeEvent::Preedit {
                text: String::new(),
                active_range_chars: None,
            })],
        );
        assert!(
            application
                .store
                .views()
                .get(view)
                .unwrap()
                .composition
                .is_none()
        );
        let snapshot = application.store.documents().snapshot(document).unwrap();
        assert_eq!(snapshot.revision, 0);
        assert!(!snapshot.dirty);
        assert_eq!(snapshot.rope.to_string(), TEXT);
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
        exit(application);
    }
    {
        let mut fixture = fixture("--native-selection-ranges-wait");
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let document = application.files[&fixture.path].id;
        let view = body_view(application, &fixture.path, &fixture.tab);
        point(application, view, TEXT.find("method").unwrap() + 2);
        focus(application, context, view);
        paint(application, context, key(egui::Key::ArrowRight, SMART_KEYS));
        wait(application, context, "held smart range", |application| {
            diagnostic(application, "synthetic selection waiting")
        });
        point(application, view, 2);
        wait(
            application,
            context,
            "cancelled smart range",
            |application| diagnostic(application, "synthetic selection cancelled"),
        );
        assert_eq!(
            selection(application, view),
            Selection { anchor: 2, head: 2 }
        );
        assert!(!taide_native_editor::cursor_commands::has_smart_selection(
            &application.store,
            view
        ));
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
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
        exit(application);
    }
}
