use super::super::inlay_hints_tests::{body_view, fixture_with_language, paint, wait};
use super::super::selection_ranges_tests::run_palette;
use super::*;
use taide_native_editor::css_format;
use taide_native_editor::folding::FoldRegion;
use taide_native_editor::view::{Selection, SelectionSet};

const CSS: &str =
    ".a {\n  color:red;\n  margin:0;\n}\n/* #region x */\n.b{top:0}\n/* #endregion */\n";
const SCSS: &str = "// #region\n.a {\n  &:hover { b: c; }\n}\n// #endregion\n";
const PASTED: &str = ".x{y:z}";
const SAVED: &str = ".s{t:u}";
const MAX_SAVE_UNDOS: usize = 4;
const LIMITS: css_format::Limits = css_format::Limits {
    max_bytes: 1 << 20,
    max_output_bytes: 1 << 20,
    max_indent_width: 1024,
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
    let edits = css_format::format(
        input,
        range,
        css_format::Options {
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

fn wait_folds(
    application: &mut NativeApplication,
    context: &egui::Context,
    document: DocumentId,
    regions: &[(usize, usize, Option<&str>)],
) {
    wait(application, context, "built-in css folds", |application| {
        let snapshot = application.store.documents().snapshot(document).unwrap();
        application
            .editor_folding
            .builtin_model(&snapshot)
            .is_some_and(|model| {
                model.regions().len() == regions.len()
                    && regions.iter().all(|&(start_line, end_line, kind)| {
                        model.regions().contains(&FoldRegion {
                            start_line,
                            end_line,
                        }) && model.kind(start_line) == kind
                    })
            })
    });
    let snapshot = application.store.documents().snapshot(document).unwrap();
    assert!(
        application
            .editor_symbols
            .builtin_model(&snapshot)
            .is_none()
    );
}

#[test]
fn actual_app의_css는_외부서버없이_접기와_문서_선택_붙여넣기_저장_포맷을_공급한다() {
    let mut fixture = fixture_with_language(CSS, "css");
    let context = fixture.context.clone();
    let path = fixture.path.clone();
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .metadata
            .language_id,
        "css"
    );
    wait_folds(
        application,
        &context,
        document,
        &[(0, 2, None), (4, 6, Some("region"))],
    );
    let body = crate::editor_context_menu::body(&context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    run_palette(application, &context, "monaco.editor.action.formatDocument");
    let formatted = expected(application, document, CSS, None);
    assert!(formatted.contains("color: red;") && formatted.contains("}\n\n/* #region x */"));
    wait(application, &context, "document format", |application| {
        text(application, document) == formatted
    });
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), CSS);
    let start = CSS.find("color").unwrap();
    let end = start + "color:red;".len();
    select(application, view, start..end);
    paint(application, &context, Vec::new());
    context.memory_mut(|memory| memory.request_focus(body));
    run_palette(
        application,
        &context,
        "monaco.editor.action.formatSelection",
    );
    let selection = expected(application, document, CSS, Some(start..end));
    assert!(selection.contains("color: red;\n  margin:0;"));
    wait(application, &context, "selection format", |application| {
        text(application, document) == selection
    });
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), CSS);
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
    select(application, view, 0..CSS.len());
    paint(application, &context, Vec::new());
    context.memory_mut(|memory| memory.request_focus(body));
    paint(
        application,
        &context,
        vec![egui::Event::Paste(PASTED.into())],
    );
    let pasted = expected(application, document, PASTED, Some(0..PASTED.len()));
    assert!(pasted.starts_with(".x {\n") && pasted.ends_with("y: z\n}"));
    wait(application, &context, "paste format", |application| {
        text(application, document) == pasted
    });
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), PASTED);
    assert!(application.store.undo(document).unwrap());
    assert_eq!(text(application, document), CSS);
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
    let snapshot = application.store.save_snapshot(document).unwrap();
    assert!(
        application.submit(HostCommand::Editor(crate::domains::editor::Command::Save {
            path: path.clone(),
            snapshot,
        }))
    );
    wait(application, &context, "save format", |application| {
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

#[test]
fn actual_app의_scss는_한줄주석_region과_중첩규칙_접기를_공급하고_문서포맷을_적용한다() {
    let mut fixture = fixture_with_language(SCSS, "scss");
    let context = fixture.context.clone();
    let path = fixture.path.clone();
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    wait_folds(
        application,
        &context,
        document,
        &[(0, 4, Some("region")), (1, 2, None)],
    );
    let body = crate::editor_context_menu::body(&context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    run_palette(application, &context, "monaco.editor.action.formatDocument");
    let formatted = expected(application, document, SCSS, None);
    assert!(formatted.contains("&:hover {\n"));
    wait(
        application,
        &context,
        "scss document format",
        |application| text(application, document) == formatted,
    );
}
