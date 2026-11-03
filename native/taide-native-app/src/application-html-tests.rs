use super::super::inlay_hints_tests::{body_view, fixture_with_language, paint, wait};
use super::super::selection_ranges_tests::run_palette;
use super::*;
use taide_native_editor::folding::FoldRegion;
use taide_native_editor::html_format;
use taide_native_editor::view::{Selection, SelectionSet};

const HTML: &str = "<div><p>a</p>\n<ul><li>x</li></ul></div>\n";
const PASTED: &str = "<div><p>q</p></div>";
const SAVED: &str = "<div><p>s</p></div>";
const ROWS: usize = 60;
const SCROLL_LINE: usize = 20;
const MAX_SAVE_UNDOS: usize = 4;
const LIMITS: html_format::Limits = html_format::Limits {
    max_bytes: 1 << 20,
    max_output_bytes: 1 << 20,
    max_indent_width: 1024,
    max_tokens: 1 << 20,
    max_depth: 64,
};

fn text(application: &NativeApplication, document: DocumentId) -> String {
    application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string()
}

fn expected(
    application: &mut NativeApplication,
    document: DocumentId,
    input: &str,
    range: Option<std::ops::Range<usize>>,
) -> String {
    let options = application.save_formatting_options(document).unwrap();
    let edits = html_format::format(
        input,
        range,
        html_format::Options {
            indentation: taide_native_editor::indent::IndentOptions {
                tab_size: options.tab_size,
                insert_spaces: options.insert_spaces,
            },
        },
        LIMITS,
    )
    .unwrap();
    let mut output = input.to_owned();
    for edit in edits.into_iter().rev() {
        output.replace_range(edit.bytes, &edit.text);
    }
    output
}

fn select(application: &mut NativeApplication, view: ViewId, bytes: std::ops::Range<usize>) {
    let state = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: bytes.start,
                    head: bytes.end,
                }],
            },
            state.scroll,
            state.folds,
        )
        .unwrap();
}

#[test]
fn actual_app의_html은_스마트선택으로_속성값에서_속성까지_확장하고_되돌린다() {
    let source = "<div class=\"foo\">bar</div>\n";
    let mut fixture = fixture_with_language(source, "html");
    let context = fixture.context.clone();
    let path = fixture.path.clone();
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &path, &fixture.tab);
    let start = source.find("class").unwrap();
    let end = source.find('>').unwrap();
    let caret = source.find("oo").unwrap();
    select(application, view, caret..caret);
    paint(application, &context, Vec::new());
    let body = crate::editor_context_menu::body(&context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    let mut expansions = 0;
    for _ in 0..MAX_SELECTION_EXPANSIONS {
        let selection = application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .clone();
        paint(
            application,
            &context,
            super::super::inlay_hints_tests::key(egui::Key::ArrowRight, SMART_SELECTION_MODIFIERS),
        );
        wait(application, &context, "html smart select", |application| {
            application.store.views().get(view).unwrap().selection != selection
        });
        expansions += 1;
        let selected = application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections[0];
        if (selected.anchor, selected.head) == (start, end) {
            break;
        }
    }
    let selected = application
        .store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections[0];
    assert_eq!((selected.anchor, selected.head), (start, end));
    for _ in 0..expansions {
        let selection = application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .clone();
        paint(
            application,
            &context,
            super::super::inlay_hints_tests::key(egui::Key::ArrowLeft, SMART_SELECTION_MODIFIERS),
        );
        wait(application, &context, "html smart shrink", |application| {
            application.store.views().get(view).unwrap().selection != selection
        });
    }
    let selected = application
        .store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections[0];
    assert_eq!((selected.anchor, selected.head), (caret, caret));
}

#[test]
fn actual_app의_html은_요소_심볼로_고정줄과_접기를_공급한다() {
    let source = format!(
        "<div id=\"main\" class=\"box wide\">\n{}</div>\n",
        "  <p>x</p>\n".repeat(ROWS)
    );
    let mut fixture = fixture_with_language(&source, "html");
    let context = fixture.context.clone();
    let path = fixture.path.clone();
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    wait(
        application,
        &context,
        "built-in html structure",
        |application| {
            let snapshot = application.store.documents().snapshot(document).unwrap();
            application
                .editor_folding
                .builtin_model(&snapshot)
                .is_some_and(|model| {
                    model.regions()[..]
                        == [FoldRegion {
                            start_line: 0,
                            end_line: ROWS,
                        }]
                })
                && application
                    .editor_symbols
                    .builtin_model(&snapshot)
                    .is_some()
        },
    );
    let snapshot = application.store.documents().snapshot(document).unwrap();
    let model = application.editor_symbols.builtin_model(&snapshot).unwrap();
    let symbols = model.symbols();
    assert_eq!(symbols.len(), ROWS + 1);
    assert_eq!(symbols[0].name, "div#main.box.wide");
    assert_eq!(symbols[1].name, "p");
    assert_eq!(symbols[1].container_label, "div#main.box.wide");
    assert!(model.sticky_model().is_some());
    application
        .services
        .state
        .settings
        .write()
        .editor_sticky_scroll_enabled = true;
    paint(application, &context, Vec::new());
    let previous = application.store.views().get(view).unwrap().clone();
    let byte = snapshot.rope.line_to_byte(SCROLL_LINE);
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
            taide_native_editor::view::ScrollPosition {
                x: 0.0,
                y: application.editor.appearance.line_height * SCROLL_LINE as f32,
            },
            previous.folds,
        )
        .unwrap();
    paint(application, &context, Vec::new());
    let body = crate::editor_context_menu::body(&context, view).unwrap();
    assert!(
        context
            .read_response(body.with(("sticky-line", 0usize)))
            .is_some(),
        "HTML element scope must stay pinned while its body is scrolled"
    );
}

#[test]
fn actual_app의_html은_외부서버없이_문서_선택_붙여넣기_저장_포맷을_공급한다() {
    let mut fixture = fixture_with_language(HTML, "html");
    let context = fixture.context.clone();
    let path = fixture.path.clone();
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    let body = crate::editor_context_menu::body(&context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    run_palette(application, &context, "monaco.editor.action.formatDocument");
    let formatted = expected(application, document, HTML, None);
    assert!(formatted.contains("<p>a</p>\n") && formatted.contains("<li>x</li>"));
    assert_ne!(formatted, HTML);
    wait(
        application,
        &context,
        "html document format",
        |application| text(application, document) == formatted,
    );
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), HTML);
    let start = HTML.find("<ul>").unwrap();
    let end = HTML.find("</div>").unwrap();
    select(application, view, start..end);
    paint(application, &context, Vec::new());
    context.memory_mut(|memory| memory.request_focus(body));
    run_palette(
        application,
        &context,
        "monaco.editor.action.formatSelection",
    );
    let selection = expected(application, document, HTML, Some(start..end));
    assert_ne!(selection, HTML);
    wait(
        application,
        &context,
        "html selection format",
        |application| text(application, document) == selection,
    );
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), HTML);
    let paste = application
        .services
        .state
        .settings
        .read()
        .editor_format_on_paste;
    application
        .services
        .state
        .settings
        .write()
        .editor_format_on_paste = true;
    select(application, view, 0..HTML.len());
    paint(application, &context, Vec::new());
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        &context,
        vec![egui::Event::Paste(PASTED.into())],
    );
    let pasted = expected(application, document, PASTED, Some(0..PASTED.len()));
    assert!(pasted.starts_with("<div>\n") && pasted.ends_with("</div>"));
    wait(application, &context, "html paste format", |application| {
        text(application, document) == pasted
    });
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), PASTED);
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), HTML);
    application
        .services
        .state
        .settings
        .write()
        .editor_format_on_paste = paste;
    let flags = {
        let mut settings = application.services.state.settings.write();
        let flags = (
            settings.format_on_save,
            settings.trim_trailing_whitespace_on_save,
            settings.insert_final_newline_on_save,
        );
        settings.format_on_save = true;
        settings.trim_trailing_whitespace_on_save = false;
        settings.insert_final_newline_on_save = false;
        flags
    };
    let current = application.store.documents().snapshot(document).unwrap();
    application
        .store
        .apply(
            document,
            taide_native_editor::store::Transaction {
                revision: current.revision,
                edits: vec![taide_native_editor::document::Edit {
                    bytes: 0..current.rope.len_bytes(),
                    text: SAVED.into(),
                }],
                group: taide_native_editor::document::UndoGroup(1),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let saved = expected(application, document, SAVED, None);
    assert_ne!(saved, SAVED);
    let snapshot = application.store.save_snapshot(document).unwrap();
    assert!(
        application.submit(HostCommand::Editor(crate::domains::editor::Command::Save {
            path: path.clone(),
            snapshot,
        }))
    );
    wait(application, &context, "html save format", |application| {
        !application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
            && std::fs::read_to_string(&path).is_ok_and(|text| text == saved)
    });
    assert_eq!(text(application, document), saved);
    for _ in 0..MAX_SAVE_UNDOS {
        if text(application, document) == SAVED {
            break;
        }
        assert!(application.store.undo(document).unwrap());
    }
    assert_eq!(text(application, document), SAVED);
    {
        let mut settings = application.services.state.settings.write();
        settings.format_on_save = flags.0;
        settings.trim_trailing_whitespace_on_save = flags.1;
        settings.insert_final_newline_on_save = flags.2;
    }
}
