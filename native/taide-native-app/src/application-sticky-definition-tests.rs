use super::inlay_hints_tests::{body_view, fixture_with_view_state, paint, wait};
use super::*;

const HEADER: &str = "fn source() {";
const INDENTED_LINE: &str = "    let value = 1;";
const BODY_LINE_COUNT: usize = 80;
const FIRST_STICKY_ROW: usize = 0;
const SCROLL_LINE_COUNT: usize = 10;
const CLICKED_CHARACTER: usize = 5;
const DESTINATION_LINE: usize = 1;
const DEADLINE: Duration = Duration::from_secs(10);
const FRAME_SIZE: egui::Vec2 = egui::vec2(1000.0, 700.0);

fn wait_modifiers(
    application: &mut NativeApplication,
    context: &egui::Context,
    modifiers: egui::Modifiers,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let runtime = application.runtime.handle().clone();
    let result = runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
                paint_modifiers(application, context, Vec::new(), modifiers);
                if ready(application) {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
    });
    assert!(
        result.is_ok(),
        "sticky modifier phase; status={:?}",
        application.status
    );
}

#[test]
fn 실제_고정줄_정의클릭은_표시글자의_utf16위치를_서버와_최종선택에_전달한다() {
    sticky_definition(false, HEADER, false);
}

#[test]
fn 실제_고정줄_정의클릭의_옆열기는_같은문서의_독립뷰와_원본을_보존한다() {
    sticky_definition(true, HEADER, false);
}

#[test]
fn 실제_읽기전용_고정줄은_비bmp문자_뒤의_utf16열로_정의를_이동하고_원문을_보존한다() {
    sticky_definition(false, "fn \u{10400}source() {", true);
}

fn pointer(point: egui::Pos2, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    }
}

fn paint_modifiers(
    application: &mut NativeApplication,
    context: &egui::Context,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
) -> egui::FullOutput {
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
            events: std::iter::once(egui::Event::ModifiersChanged(modifiers))
                .chain(events)
                .collect(),
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    output.textures_delta.clear();
    output
}

fn sticky_definition(side: bool, header: &str, read_only: bool) {
    let original = format!(
        "{header}\n{}\n}}",
        std::iter::repeat_n(INDENTED_LINE, BODY_LINE_COUNT)
            .collect::<Vec<_>>()
            .join("\n")
    );
    let mut fixture = fixture_with_view_state(
        "--native-locations-sticky",
        false,
        |application, project, document| {
            let snapshot = application.store.documents().snapshot(document).unwrap();
            application.lsp.as_ref().is_some_and(|lsp| {
                !lsp.location_providers(
                    project,
                    &snapshot,
                    taide_native_editor::symbol_locations::Kind::Definition,
                )
                .is_empty()
            })
        },
        None,
        &original,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    application
        .services
        .state
        .settings
        .write()
        .editor_sticky_scroll_enabled = true;
    let source = body_view(application, &fixture.path, &fixture.tab);
    let document = application.store.views().get(source).unwrap().document;
    if read_only {
        let mut opened =
            taide_file::service::open_file(std::path::Path::new(&fixture.path), &[], false)
                .unwrap();
        opened.read_only = true;
        application
            .store
            .observe_file(document, std::path::Path::new(&fixture.path), opened)
            .unwrap();
    }
    let snapshot = application.store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.metadata.read_only, read_only);
    let prefix = header.chars().take(CLICKED_CHARACTER).collect::<String>();
    let clicked_byte = prefix.len();
    let clicked_utf16 = prefix.encode_utf16().count();
    let destination = snapshot.rope.line_to_byte(DESTINATION_LINE) + clicked_utf16;
    let before = application.store.views().get(source).unwrap().clone();
    let byte = snapshot.rope.line_to_byte(SCROLL_LINE_COUNT);
    let line_height = application.editor.appearance.line_height;
    application
        .store
        .set_view_state(
            source,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: byte,
                    head: byte,
                }],
            },
            taide_native_editor::view::ScrollPosition {
                x: 0.0,
                y: line_height * SCROLL_LINE_COUNT as f32,
            },
            before.folds,
        )
        .unwrap();
    let body = crate::editor_context_menu::body(context, source).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    let output = paint(application, context, Vec::new());
    let band = context
        .read_response(body.with(("sticky-line", FIRST_STICKY_ROW)))
        .expect("actual sticky row")
        .rect;
    let shape = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == header && band.contains(text.pos) =>
            {
                Some(text)
            }
            _ => None,
        })
        .expect("actual sticky header shape");
    let cursor = shape
        .galley
        .pos_from_cursor(egui::text::CCursor::new(CLICKED_CHARACTER));
    let point = shape.pos + cursor.center().to_vec2();
    assert!(band.contains(point));
    let modifiers = egui::Modifiers {
        mac_cmd: true,
        command: true,
        alt: side,
        ..egui::Modifiers::NONE
    };
    paint_modifiers(
        application,
        context,
        vec![egui::Event::PointerMoved(point)],
        modifiers,
    );
    wait_modifiers(application, context, modifiers, |application| {
        application
            .editor_locations
            .hover(source)
            .is_some_and(|session| session.model.is_some())
    });
    let hover = application.editor_locations.hover(source).unwrap();
    assert_eq!(
        hover.request.position,
        taide_native_editor::lsp::Position {
            line: 0,
            character: clicked_utf16.try_into().unwrap()
        }
    );
    assert_eq!(hover.request.source, source);
    assert_eq!(hover.request.snapshot.id, document);
    paint_modifiers(
        application,
        context,
        vec![
            egui::Event::PointerMoved(point),
            pointer(point, true, modifiers),
        ],
        modifiers,
    );
    paint_modifiers(
        application,
        context,
        vec![pointer(point, false, modifiers)],
        modifiers,
    );
    wait(
        application,
        context,
        "sticky definition destination",
        |application| {
            application
                .store
                .views()
                .for_document(document)
                .any(|view| {
                    view.selection.selections.iter().any(|selection| {
                        selection.anchor == destination && selection.head == destination
                    })
                })
        },
    );
    let views = application
        .store
        .views()
        .for_document(document)
        .collect::<Vec<_>>();
    if side {
        assert!(views.iter().any(|view| view.id != source));
        assert_eq!(
            application
                .store
                .views()
                .get(source)
                .unwrap()
                .selection
                .selections[0]
                .head,
            clicked_byte
        );
    } else {
        assert_eq!(
            application
                .store
                .views()
                .get(source)
                .unwrap()
                .selection
                .selections[0]
                .anchor,
            destination
        );
    }
    let current = application.store.documents().snapshot(document).unwrap();
    assert_eq!(current.metadata.read_only, read_only);
    assert_eq!(current.revision, snapshot.revision);
    assert_eq!(current.rope.to_string(), original);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), original);
    eframe::App::on_exit(application);
    application.runtime.block_on(async {
        application.services.tasks.shutdown().await;
    });
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
