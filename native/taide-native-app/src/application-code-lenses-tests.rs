use super::inlay_hints_tests::{
    Fixture, body_view, fixture_with_provider, key, paint, text_contains, wait,
};
use super::*;
use taide_native_editor::view::{Selection, SelectionSet};

const TEXT: &str = "class\n  \u{1f600}method\nend";
const STABLE_FRAMES: usize = 16;

fn fixture(mode: &str) -> Fixture {
    fixture_with_provider(mode, false, |application, project, document| {
        !application
            .lsp
            .as_ref()
            .unwrap()
            .code_lens_providers(
                project,
                &application.store.documents().snapshot(document).unwrap(),
            )
            .is_empty()
    })
}

fn lenses(
    application: &NativeApplication,
    context: &egui::Context,
    view: ViewId,
) -> Option<taide_native_ui::editor_code_lenses::Presentation> {
    application.editor_code_lenses.display(
        &application.store,
        context.viewport_id(),
        view,
        application.editor_code_lens_colors,
    )
}

fn diagnostic(application: &NativeApplication, message: &str) -> bool {
    application
        .lsp_diagnostics
        .batches()
        .any(|batch| batch.diagnostics.iter().any(|item| item.message == message))
}

fn point(output: &egui::FullOutput, title: &str) -> egui::Pos2 {
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == title => {
                Some(text.pos + text.galley.size() / 2.0)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("label missing: {title}"))
}

fn click(application: &mut NativeApplication, context: &egui::Context, position: egui::Pos2) {
    paint(
        application,
        context,
        vec![egui::Event::PointerMoved(position)],
    );
    for pressed in [true, false] {
        paint(
            application,
            context,
            vec![egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

#[test]
fn 실제_app의_클릭_서버명령_갱신과_client_참조명령은_본문과_peek를_보존한다() {
    let mut fixture = fixture("--native-code-lenses-refresh");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = body_view(application, &fixture.path, &fixture.tab);
    wait(
        application,
        context,
        "resolved body lenses",
        |application| {
            lenses(application, context, view).is_some_and(|value| {
                value.items.len() == 3 && value.items.iter().all(|item| item.title.is_some())
            })
        },
    );
    let before = application.store.documents().snapshot(document).unwrap();
    let original = application
        .editor_code_lenses
        .request(context.viewport_id(), view)
        .unwrap()
        .clone();
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, "Resolved v0"));
    assert_eq!(
        lenses(application, context, view).unwrap().items[1].anchor,
        TEXT.find("method").unwrap()
    );
    for _ in 0..STABLE_FRAMES {
        paint(application, context, Vec::new());
    }
    assert_eq!(
        application
            .editor_code_lenses
            .request(context.viewport_id(), view)
            .unwrap()
            .token,
        original.token
    );
    assert!(diagnostic(application, "synthetic lens provided 1"));
    click(application, context, point(&output, "Run v0"));
    wait(
        application,
        context,
        "scoped refresh and fresh lens",
        |application| {
            application
                .editor_code_lenses
                .request(context.viewport_id(), view)
                .is_some_and(|request| request.token != original.token)
                && diagnostic(application, "synthetic lens provided 2")
                && lenses(application, context, view)
                    .is_some_and(|value| value.items.iter().all(|item| item.title.is_some()))
        },
    );
    assert!(original.is_cancelled());
    let output = paint(application, context, Vec::new());
    click(application, context, point(&output, "Show references"));
    wait(
        application,
        context,
        "client references peek",
        |application| {
            application
                .editor_locations
                .current(view)
                .is_some_and(|session| session.preview.is_some())
        },
    );
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    wait(application, context, "peek lenses", |application| {
        lenses(application, context, preview).is_some_and(|value| {
            value.items.len() == 3 && value.items.iter().all(|item| item.title.is_some())
        })
    });
    assert_eq!(
        application.store.views().get(preview).unwrap().document,
        document
    );
    assert!(lenses(application, context, view).is_some());
    let head = TEXT.find("method").unwrap();
    application
        .store
        .set_view_state(
            preview,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 0, head: 0 }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    let peek_id = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == context.viewport_id() && *target == preview).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(peek_id));
    application.palette.open(
        context,
        taide_native_ui::command_registry::PaletteEntry::Commands,
    );
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    assert!(application.palette.inspection().has_input_focus);
    assert_eq!(
        application.focused_editor_source(context, &fixture.tab, Some(document)),
        Some((preview, view))
    );
    let command = "monaco.codelens.showLensesInCurrentLine";
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
    wait(
        application,
        context,
        "peek command picker preserves source",
        |application| application.palette.is_picking_code_lenses(),
    );
    paint(application, context, Vec::new());
    assert_eq!(
        application.palette.inspection().rows[0].label,
        "Resolved v0"
    );
    paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    let peek = application
        .editor_code_lenses
        .request(context.viewport_id(), preview)
        .unwrap()
        .clone();
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Close),
    ));
    wait(
        application,
        context,
        "peek close and lens cancellation",
        |application| {
            application.editor_locations.current(view).is_none()
                && application
                    .editor_code_lenses
                    .request(context.viewport_id(), preview)
                    .is_none()
        },
    );
    assert!(peek.is_cancelled());
    let after = application.store.documents().snapshot(document).unwrap();
    assert_eq!(
        (after.rope, after.revision, after.dirty),
        (before.rope, before.revision, before.dirty)
    );
    let current = application
        .editor_code_lenses
        .request(context.viewport_id(), view)
        .unwrap()
        .clone();
    eframe::App::on_exit(application);
    assert!(current.is_cancelled());
    assert_eq!(application.services.tasks.tracked_count(), 0);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    assert_eq!(std::fs::read_to_string(&fixture.peek_path).unwrap(), TEXT);
}

#[test]
fn 실제_app의_현재줄_선택_설정_편집_readonly_tier와_종료는_렌즈_수명을_따른다() {
    let mut fixture = fixture("--native-code-lenses");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = body_view(application, &fixture.path, &fixture.tab);
    wait(application, context, "resolved lenses", |application| {
        lenses(application, context, view).is_some_and(|value| {
            value.items.len() == 3 && value.items.iter().all(|item| item.title.is_some())
        })
    });
    let before = application.store.documents().snapshot(document).unwrap();
    let head = TEXT.find("method").unwrap();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    application
        .document_edits
        .push((fixture.tab.clone(), DocumentEdit::CodeLens));
    wait(application, context, "current line picker", |application| {
        application.palette.is_picking_code_lenses()
    });
    paint(application, context, Vec::new());
    assert_eq!(
        application.palette.inspection().placeholder,
        "Select a command"
    );
    assert_eq!(application.palette.inspection().rows.len(), 1);
    assert_eq!(
        application.palette.inspection().rows[0].label,
        "Resolved v0"
    );
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    wait(
        application,
        context,
        "selected server command",
        |application| {
            !application.palette.is_open() && diagnostic(application, "synthetic lens executed")
        },
    );
    let initial = application
        .editor_code_lenses
        .request(context.viewport_id(), view)
        .unwrap()
        .clone();
    application
        .services
        .state
        .settings
        .write()
        .editor_code_lens_enabled = false;
    paint(application, context, Vec::new());
    assert!(initial.is_cancelled());
    assert!(lenses(application, context, view).is_none());
    application
        .services
        .state
        .settings
        .write()
        .editor_code_lens_enabled = true;
    wait(application, context, "reenabled lenses", |application| {
        lenses(application, context, view).is_some_and(|value| value.items.len() == 3)
    });
    let initial = application
        .editor_code_lenses
        .request(context.viewport_id(), view)
        .unwrap()
        .clone();
    let editor_id = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == context.viewport_id() && *target == view).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(editor_id));
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: TEXT.len(),
                    head: TEXT.len(),
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    paint(application, context, vec![egui::Event::Text("x".into())]);
    assert!(initial.is_cancelled());
    assert!(lenses(application, context, view).is_none());
    wait(
        application,
        context,
        "latest document lenses",
        |application| {
            lenses(application, context, view).is_some_and(|value| {
                value
                    .items
                    .first()
                    .is_some_and(|item| item.title.as_deref() == Some("Run v1"))
            })
        },
    );
    let edited = application.store.documents().snapshot(document).unwrap();
    assert_eq!(edited.rope.to_string(), format!("{TEXT}x"));
    assert_eq!(edited.revision, before.revision + 1);
    for tier in [
        taide_model::file::FileSizeTier::Large,
        taide_model::file::FileSizeTier::ReadOnly,
        taide_model::file::FileSizeTier::Normal,
    ] {
        application
            .store
            .observe_file(
                document,
                std::path::Path::new(&fixture.path),
                taide_model::file::OpenedFile {
                    path: fixture.path.clone(),
                    content: edited.rope.to_string(),
                    language_id: "rust".into(),
                    byte_size: edited.rope.len_bytes().try_into().unwrap(),
                    line_count: edited.rope.len_lines().try_into().unwrap(),
                    tier,
                    read_only: true,
                    encoding_lossy: false,
                    modified_ms: 0.0,
                    editor_config: Default::default(),
                },
            )
            .unwrap();
        paint(application, context, Vec::new());
        if tier == taide_model::file::FileSizeTier::Normal {
            wait(
                application,
                context,
                "readonly normal lenses",
                |application| {
                    lenses(application, context, view).is_some_and(|value| !value.items.is_empty())
                },
            );
            application
                .document_edits
                .push((fixture.tab.clone(), DocumentEdit::CodeLens));
            wait(
                application,
                context,
                "readonly current line picker",
                |application| application.palette.is_picking_code_lenses(),
            );
            paint(
                application,
                context,
                key(egui::Key::Escape, egui::Modifiers::NONE),
            );
            paint(
                application,
                context,
                vec![egui::Event::Text("blocked".into())],
            );
            assert_eq!(
                application
                    .store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope,
                edited.rope
            );
        } else {
            assert!(lenses(application, context, view).is_none());
        }
    }
    let current = application
        .editor_code_lenses
        .request(context.viewport_id(), view)
        .unwrap()
        .clone();
    eframe::App::on_exit(application);
    assert!(current.is_cancelled());
    assert_eq!(application.services.tasks.tracked_count(), 0);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
}
