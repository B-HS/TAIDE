use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, fixture_with_view_state_and_setup, key, paint, wait,
};
use super::*;
use taide_native_editor::view::Selection;

const ANCHOR_COLUMN: u32 = 11;
const STORED_LINE: u32 = 2;
const ANCHOR_BYTE: usize = 18;
const HEAD_COLUMN: u32 = 3;
const HEAD_BYTE: usize = 8;
const SHIFT_HEAD_BYTE: usize = 12;
const STABLE_FRAMES: usize = 16;
const LONG_LINE_COUNT: usize = 144;
const WIDE_RUNS: usize = 40;
const FIRST_VISIBLE_LINE: usize = 45;
const SCROLL_LEFT: f32 = 72.0;
const SCROLL_DELTA_TOP: f32 = -7.0;
const LEGACY_SCROLL_TOP: f32 = 320.0;
const NAVIGATION_LINE: u32 = 100;
const INVALID_UTF8: u8 = 0xff;

#[test]
fn 실제_본문은_저장된_utf16_역방향_선택을_복원하고_후속_layout에서_재복원하지_않는다() {
    let saved = serde_json::json!({
        "cursorState": [{
            "inSelectionMode": true,
            "selectionStart": {"lineNumber": STORED_LINE, "column": ANCHOR_COLUMN},
            "position": {"lineNumber": STORED_LINE, "column": HEAD_COLUMN}
        }],
        "viewState": {
            "scrollLeft": 0,
            "firstPosition": {"lineNumber": 1, "column": 1},
            "firstPositionDeltaTop": 0
        },
        "contributionsState": {}
    })
    .to_string();
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        Some(saved),
        "class\n  \u{1f600}method\nend",
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    let original = application.store.documents().snapshot(document).unwrap();
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: HEAD_BYTE,
        }],
        "persisted Unicode selection must reach the live model"
    );
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    for _ in 0..STABLE_FRAMES {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
    }
    let changed = Selection {
        anchor: ANCHOR_BYTE,
        head: SHIFT_HEAD_BYTE,
    };
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![changed]
    );
    let reset = serde_json::json!({
        "cursorState": [{
            "selectionStart": {"lineNumber": 1, "column": 1},
            "position": {"lineNumber": 1, "column": 1}
        }],
        "viewState": {"scrollLeft": 0, "scrollTopWithoutViewZones": 0}
    })
    .to_string();
    application
        .runtime
        .handle()
        .clone()
        .block_on(taide_runtime::layout_actions::layout_set_view_state(
            application.services.events.as_ref(),
            &application.services.state,
            fixture.tab.clone(),
            Some(reset.clone()),
        ))
        .unwrap();
    wait(application, context, "layout metadata", |application| {
        crate::tabs::tabs_in(&application.controller.snapshot().layouts[&fixture.project].root)
            .iter()
            .any(|tab| tab.id == fixture.tab && tab.view_state.as_ref() == Some(&reset))
    });
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![changed]
    );
    let current = application.store.documents().snapshot(document).unwrap();
    assert_eq!(current.rope.to_string(), original.rope.to_string());
    assert_eq!(current.revision, original.revision);
    assert_eq!(current.dirty, original.dirty);
    assert_eq!(
        std::fs::read_to_string(&fixture.path).unwrap(),
        original.rope.to_string()
    );
}

#[test]
fn 실제_초기_명시적_줄_이동은_저장된_scroll보다_우선한다() {
    let text = (0..LONG_LINE_COUNT)
        .map(|index| format!("row {index}\n"))
        .collect::<String>();
    let saved = serde_json::json!({
        "cursorState": [{"position": {"lineNumber": 1, "column": 1}}],
        "viewState": {
            "firstPosition": {"lineNumber": FIRST_VISIBLE_LINE, "column": 1},
            "firstPositionDeltaTop": SCROLL_DELTA_TOP
        }
    })
    .to_string();
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        Some(saved),
        &text,
        |application, path, project, tab| {
            assert!(application.reveals.queue_position(
                crate::editor_reveal::Target {
                    project,
                    tab,
                    path,
                    viewport: egui::ViewportId::ROOT,
                },
                crate::editor_reveal::Position {
                    line: f64::from(NAVIGATION_LINE),
                    column: 1.0
                },
                &application.services.state.layouts.read(),
                Instant::now(),
            ));
        },
    );
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &fixture.path, &fixture.tab);
    let state = application.store.views().get(view).unwrap();
    let document = application
        .store
        .documents()
        .snapshot(state.document)
        .unwrap();
    let byte = document
        .rope
        .line_to_byte(usize::try_from(NAVIGATION_LINE).unwrap() - 1);
    assert_eq!(
        state.selection.selections,
        vec![Selection {
            anchor: byte,
            head: byte
        }]
    );
    let display = state.display.as_ref().unwrap();
    let saved_byte = document.rope.line_to_byte(FIRST_VISIBLE_LINE - 1);
    let layout = taide_native_editor::display_layout::VerticalLayout::new(
        application.editor.appearance.line_height,
        display.row_count(),
    );
    let saved_top = layout.row_top(display.row_of_byte(&document, saved_byte)) - SCROLL_DELTA_TOP;
    assert!(
        state.scroll.y > saved_top,
        "explicit navigation was replaced by persisted scroll: {:?}",
        state.scroll
    );
    assert_eq!(document.rope.to_string(), text);
    assert!(!document.dirty);
}

#[test]
fn 실제_lossy_readonly_본문은_새_live_view의_위치를_복원하고_선택만_이동한다() {
    let saved = serde_json::json!({
        "cursorState": [{
            "selectionStart": {"lineNumber": STORED_LINE, "column": ANCHOR_COLUMN},
            "position": {"lineNumber": STORED_LINE, "column": HEAD_COLUMN}
        }],
        "viewState": {"scrollLeft": 0, "scrollTopWithoutViewZones": 0}
    })
    .to_string();
    let text = "class\n  \u{1f600}method\nend";
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        Some(saved),
        text,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let previous_view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    let bytes = [text.as_bytes(), &[INVALID_UTF8]].concat();
    std::fs::write(&fixture.path, &bytes).unwrap();
    let opened = application
        .runtime
        .handle()
        .clone()
        .block_on(taide_runtime::native_file_actions::open_document_file(
            &application.services.state,
            &application.services.tasks,
            fixture.path.clone(),
            Vec::new,
        ))
        .unwrap();
    assert!(opened.file.read_only && opened.file.encoding_lossy);
    application
        .store
        .observe_file(document, &opened.canonical_path, opened.file)
        .unwrap();
    application.store.detach_view(previous_view).unwrap();
    paint(application, context, Vec::new());
    let view = body_view(application, &fixture.path, &fixture.tab);
    assert_ne!(view, previous_view);
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: HEAD_BYTE,
        }]
    );
    let original = application.store.documents().snapshot(document).unwrap();
    assert!(original.metadata.read_only && original.metadata.lossy);
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        vec![egui::Event::Text("blocked".into())],
    );
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    for _ in 0..STABLE_FRAMES {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
    }
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: SHIFT_HEAD_BYTE,
        }]
    );
    let current = application.store.documents().snapshot(document).unwrap();
    assert_eq!(current.rope.to_string(), original.rope.to_string());
    assert_eq!(current.revision, original.revision);
    assert_eq!(current.dirty, original.dirty);
    assert!(current.metadata.read_only);
    assert_eq!(std::fs::read(&fixture.path).unwrap(), bytes);
}

#[test]
fn 실제_본문은_커서와_독립된_첫_모델_위치와_legacy_scroll을_즉시_복원한다() {
    let text = (0..LONG_LINE_COUNT)
        .map(|index| format!("row {index} {}\n", "wide text ".repeat(WIDE_RUNS)))
        .collect::<String>();
    for (scroll, is_anchored) in [
        (
            serde_json::json!({
                "scrollLeft": SCROLL_LEFT,
                "firstPosition": {"lineNumber": FIRST_VISIBLE_LINE, "column": 1},
                "firstPositionDeltaTop": SCROLL_DELTA_TOP,
            }),
            true,
        ),
        (
            serde_json::json!({
                "scrollLeft": SCROLL_LEFT,
                "scrollTopWithoutViewZones": LEGACY_SCROLL_TOP,
            }),
            false,
        ),
    ] {
        let saved = serde_json::json!({
            "cursorState": [{"position": {"lineNumber": 1, "column": 1}}],
            "viewState": scroll,
        })
        .to_string();
        let mut fixture = fixture_with_view_state(
            "--native-inlay-hints",
            false,
            |_, _, _| true,
            Some(saved),
            &text,
        );
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let view = body_view(application, &fixture.path, &fixture.tab);
        let state = application.store.views().get(view).unwrap();
        let document = application
            .store
            .documents()
            .snapshot(state.document)
            .unwrap();
        let expected_top = if is_anchored {
            let display = state.display.as_ref().unwrap();
            let byte = document.rope.line_to_byte(FIRST_VISIBLE_LINE - 1);
            let layout = taide_native_editor::display_layout::VerticalLayout::new(
                application.editor.appearance.line_height,
                display.row_count(),
            );
            layout.row_top(display.row_of_byte(&document, byte)) - SCROLL_DELTA_TOP
        } else {
            LEGACY_SCROLL_TOP
        };
        assert_eq!(
            state.selection.selections,
            vec![Selection { anchor: 0, head: 0 }]
        );
        assert_eq!(state.scroll.x, SCROLL_LEFT);
        assert_eq!(state.scroll.y, expected_top);
        for _ in 0..STABLE_FRAMES {
            eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
            paint(application, context, Vec::new());
        }
        let state = application.store.views().get(view).unwrap();
        assert_eq!(state.scroll.x, SCROLL_LEFT);
        assert_eq!(state.scroll.y, expected_top);
        let current = application
            .store
            .documents()
            .snapshot(state.document)
            .unwrap();
        assert_eq!(current.revision, document.revision);
        assert!(!current.dirty);
        assert_eq!(current.rope.to_string(), text);
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), text);
    }
}
