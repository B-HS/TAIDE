use egui::os::OperatingSystem;
use egui::{
    Color32, Context, Event, FontId, ImeEvent, Key, Modifiers, OutputCommand, PointerButton, Pos2,
    RawInput, Rect, pos2, vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, EditorError, UndoGroup};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 16;
const BYTE_LIMIT: usize = 2 * 1024 * 1024;
const ROWS: usize = 50_000;
const SCROLL_LINE: usize = 40_000;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 8.0;
const SCREEN: [f32; 2] = [800.0, 200.0];
const REVEAL_PREFIX: usize = 1000;
const CENTER_DIVISOR: f32 = 2.0;
const FILE_INDENT_SIZE: u32 = 2;
const VERTICAL_SCROLLBAR_SIZE: f32 = 14.0;
const HORIZONTAL_SCROLLBAR_SIZE: f32 = 12.0;
const SCROLLBAR_MIN_SLIDER: f32 = 20.0;
const SCROLL_BEYOND_LAST_COLUMN: usize = 4;
const PAGE_OVERLAP_LINES: f32 = 2.0;
const SHORT_ROWS: usize = 200;
const LONG_LINE: usize = 300;
const SLIDER_DRAG: f32 = 20.0;
const FAR_SCROLL: f32 = 1_000_000.0;
const WIDTH_TOLERANCE: f32 = 0.5;
const TEXT_CLICK_Y: f32 = 30.0;

struct Drawn {
    copied: Vec<String>,
    viewport: Rect,
    rects: Vec<Rect>,
}

fn draw(context: &Context, store: &mut EditorStore, view: ViewId, events: Vec<Event>) -> Drawn {
    let mut viewport = Rect::NOTHING;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let shown = editor().show(ui, store, view, true).unwrap();
            assert!(shown.errors.is_empty());
        },
    );
    output.textures_delta.clear();
    Drawn {
        copied: output
            .platform_output
            .commands
            .iter()
            .filter_map(|command| match command {
                OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        viewport,
        rects: output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Rect(rect) => Some(rect.rect),
                _ => None,
            })
            .collect(),
    }
}

fn measure(context: &Context, text: &str) -> f32 {
    let mut width = 0.0;
    let mut output = context.run_ui(RawInput::default(), |ui| {
        width = ui
            .painter()
            .layout_no_wrap(text.into(), FontId::monospace(FONT_SIZE), Color32::WHITE)
            .size()
            .x;
    });
    output.textures_delta.clear();
    width
}

fn chord(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn press(position: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn selection(store: &EditorStore, view: ViewId) -> (usize, usize) {
    let state = store.views().get(view).unwrap();
    let primary = state.selection.selections[state.selection.primary];
    (primary.anchor, primary.head)
}

#[test]
fn 빈_선택_복사와_잘라내기는_현재_줄을_클립보드에_담고_같은_줄_붙여넣기는_위에_삽입한다() {
    let (mut store, view) = fixture("abc\ndef", false);
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    let copied = draw(
        &context,
        &mut store,
        view,
        vec![key(Key::ArrowDown, false), Event::Copy],
    )
    .copied;
    assert_eq!(copied, ["def\n"]);
    draw(
        &context,
        &mut store,
        view,
        vec![Event::Paste("def\n".into())],
    );
    assert_eq!(text(&store, view), "abc\ndef\ndef");
    assert_eq!(selection(&store, view), (8, 8));
    let cut = draw(&context, &mut store, view, vec![Event::Cut]).copied;
    assert_eq!(cut, ["def\n"]);
    assert_eq!(text(&store, view), "abc\ndef");
    draw(&context, &mut store, view, vec![Event::Paste("zz".into())]);
    assert_eq!(text(&store, view), "abc\ndefzz");
    for expected in ["abc\ndef", "abc\ndef\ndef", "abc\ndef"] {
        draw(&context, &mut store, view, vec![key(Key::Z, true)]);
        assert_eq!(text(&store, view), expected);
    }
    let (mut readonly, readonly_view) = fixture("locked\nline", true);
    let locked = Context::default();
    draw(&locked, &mut readonly, readonly_view, Vec::new());
    assert_eq!(
        draw(&locked, &mut readonly, readonly_view, vec![Event::Cut]).copied,
        ["locked\n"]
    );
    assert_eq!(text(&readonly, readonly_view), "locked\nline");
}

#[test]
fn 연속_입력은_한_번의_undo로_되돌리고_커서_이동은_undo_단위를_나눈다() {
    let (mut store, view) = fixture("", false);
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    for character in ["a", "b", "c"] {
        draw(
            &context,
            &mut store,
            view,
            vec![Event::Text(character.into())],
        );
    }
    assert_eq!(text(&store, view), "abc");
    draw(&context, &mut store, view, vec![key(Key::Z, true)]);
    assert_eq!(text(&store, view), "");
    for events in [
        vec![Event::Text("x".into())],
        vec![key(Key::ArrowLeft, false), key(Key::ArrowRight, false)],
        vec![Event::Text("y".into())],
        vec![key(Key::Backspace, false), key(Key::Backspace, false)],
    ] {
        draw(&context, &mut store, view, events);
    }
    for expected in ["xy", "x", ""] {
        draw(&context, &mut store, view, vec![key(Key::Z, true)]);
        assert_eq!(text(&store, view), expected);
    }
}

#[test]
fn 단어_줄_문서_이동과_삭제_단축키는_host_os의_monaco_keybinding을_따른다() {
    let content = "foo bar\n  baz qux";
    let command = Modifiers::MAC_CMD | Modifiers::COMMAND;
    let (mut store, view) = fixture(content, false);
    let context = Context::default();
    context.set_os(OperatingSystem::Mac);
    draw(&context, &mut store, view, Vec::new());
    for (event, expected) in [
        (key(Key::End, false), (7, 7)),
        (chord(Key::ArrowLeft, Modifiers::ALT), (4, 4)),
        (
            chord(Key::ArrowLeft, Modifiers::ALT | Modifiers::SHIFT),
            (4, 0),
        ),
        (chord(Key::ArrowRight, command), (7, 7)),
        (chord(Key::ArrowLeft, command), (0, 0)),
        (chord(Key::ArrowDown, command), (17, 17)),
        (chord(Key::ArrowLeft, command), (10, 10)),
        (chord(Key::ArrowLeft, command), (8, 8)),
        (chord(Key::ArrowUp, command | Modifiers::SHIFT), (8, 0)),
        (key(Key::ArrowLeft, false), (0, 0)),
        (chord(Key::ArrowRight, Modifiers::ALT), (3, 3)),
        (chord(Key::ArrowRight, Modifiers::CTRL), (3, 3)),
        (chord(Key::ArrowLeft, command | Modifiers::ALT), (3, 3)),
    ] {
        draw(&context, &mut store, view, vec![event]);
        assert_eq!(selection(&store, view), expected);
    }
    for (event, expected) in [
        (chord(Key::Delete, Modifiers::ALT), "foo\n  baz qux"),
        (chord(Key::Backspace, command), "\n  baz qux"),
        (chord(Key::Backspace, Modifiers::ALT), "\n  baz qux"),
        (key(Key::Z, true), "foo\n  baz qux"),
        (key(Key::Z, true), content),
    ] {
        draw(&context, &mut store, view, vec![event]);
        assert_eq!(text(&store, view), expected);
    }
    let control = Modifiers::CTRL | Modifiers::COMMAND;
    let (mut store, view) = fixture(content, false);
    let context = Context::default();
    context.set_os(OperatingSystem::Windows);
    draw(&context, &mut store, view, Vec::new());
    for (event, expected) in [
        (chord(Key::ArrowRight, control), (3, 3)),
        (chord(Key::ArrowRight, control), (7, 7)),
        (chord(Key::ArrowLeft, Modifiers::ALT), (7, 7)),
        (chord(Key::ArrowLeft, control | Modifiers::SHIFT), (7, 4)),
        (chord(Key::End, control), (17, 17)),
        (chord(Key::Home, control), (0, 0)),
        (chord(Key::ArrowRight, control), (3, 3)),
    ] {
        draw(&context, &mut store, view, vec![event]);
        assert_eq!(selection(&store, view), expected);
    }
    for (event, expected) in [
        (chord(Key::Delete, control), "foo\n  baz qux"),
        (chord(Key::Backspace, control), "\n  baz qux"),
        (chord(Key::Backspace, Modifiers::ALT), "\n  baz qux"),
    ] {
        draw(&context, &mut store, view, vec![event]);
        assert_eq!(text(&store, view), expected);
    }
    let (mut store, view) = fixture(content, false);
    let context = Context::default();
    context.set_os(OperatingSystem::Mac);
    for events in [
        Vec::new(),
        vec![
            key(Key::End, false),
            chord(Key::ArrowLeft, Modifiers::ALT),
            chord(Key::Backspace, command),
        ],
    ] {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                editor()
                    .show_with_keymap(ui, &mut store, view, true, |_, event, _| {
                        matches!(
                            event,
                            Event::Key { modifiers, .. } if modifiers.alt || modifiers.mac_cmd
                        )
                    })
                    .unwrap();
            },
        );
        output.textures_delta.clear();
    }
    assert_eq!(selection(&store, view), (7, 7));
    assert_eq!(text(&store, view), content);
}

#[test]
fn enter는_문서_개행과_들여쓰기를_잇고_tab과_shift_tab은_선택_줄을_들여쓰고_내어쓴다() {
    let (mut store, view) = fixture("    one\r\ntwo", false);
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    for (events, expected) in [
        (
            vec![key(Key::End, false), key(Key::Enter, false)],
            "    one\r\n    \r\ntwo",
        ),
        (vec![Event::Text("x".into())], "    one\r\n    x\r\ntwo"),
        (
            vec![chord(Key::Tab, Modifiers::SHIFT)],
            "    one\r\nx\r\ntwo",
        ),
        (
            vec![key(Key::A, true), key(Key::Tab, false)],
            "        one\r\n    x\r\n    two",
        ),
        (
            vec![chord(Key::Tab, Modifiers::SHIFT)],
            "    one\r\nx\r\ntwo",
        ),
        (vec![key(Key::Z, true)], "        one\r\n    x\r\n    two"),
    ] {
        draw(&context, &mut store, view, events);
        assert_eq!(text(&store, view), expected);
    }
    let (mut mixed, mixed_view) = fixture("one\ntwo\r\nthree\r\nfour", false);
    let context = Context::default();
    draw(&context, &mut mixed, mixed_view, Vec::new());
    draw(
        &context,
        &mut mixed,
        mixed_view,
        vec![key(Key::End, false), key(Key::Enter, false)],
    );
    assert_eq!(text(&mixed, mixed_view), "one\r\n\ntwo\r\nthree\r\nfour");
}

#[test]
fn 세로_이동은_goal_column을_보존하고_page_이동은_화면_줄_수를_따른다() {
    let (mut store, view) = fixture("abcdef\nab\nabcdef", false);
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    for (event, expected) in [
        (key(Key::End, false), (6, 6)),
        (key(Key::ArrowDown, false), (9, 9)),
        (key(Key::ArrowDown, false), (16, 16)),
        (key(Key::ArrowUp, false), (9, 9)),
        (chord(Key::ArrowUp, Modifiers::SHIFT), (9, 6)),
    ] {
        draw(&context, &mut store, view, vec![event]);
        assert_eq!(selection(&store, view), expected);
    }
    let (mut paged, paged_view) = fixture(&"row\n".repeat(SHORT_ROWS), false);
    let context = Context::default();
    let viewport = draw(&context, &mut paged, paged_view, Vec::new()).viewport;
    let page = ((viewport.height() / LINE_HEIGHT).floor() - PAGE_OVERLAP_LINES).max(1.0) as usize;
    let row = "row\n".len();
    for (event, expected) in [
        (key(Key::PageDown, false), (page * row, page * row)),
        (
            chord(Key::PageDown, Modifiers::SHIFT),
            (page * row, page * row * 2),
        ),
        (key(Key::PageUp, false), (0, 0)),
    ] {
        draw(&context, &mut paged, paged_view, vec![event]);
        assert_eq!(selection(&paged, paged_view), expected);
    }
}

#[test]
fn scrollbar는_가로_scroll을_가장_긴_표시_줄로_제한하고_track_클릭과_slider_드래그로_이동한다() {
    let content = format!("{}\n{}", "x".repeat(LONG_LINE), "row\n".repeat(SHORT_ROWS));
    let (mut store, view) = fixture(&content, false);
    let context = Context::default();
    let widest = measure(&context, &"x".repeat(LONG_LINE)).ceil();
    let beyond = measure(&context, &" ".repeat(SCROLL_BEYOND_LAST_COLUMN));
    let gutter = measure(&context, "000") + PADDING * 2.0;
    let viewport = draw(&context, &mut store, view, Vec::new()).viewport;
    let text_width = viewport.width() - gutter;
    let content_width = widest + beyond + VERTICAL_SCROLLBAR_SIZE;
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            current.selection,
            ScrollPosition {
                x: FAR_SCROLL,
                y: 0.0,
            },
            current.folds,
        )
        .unwrap();
    draw(&context, &mut store, view, Vec::new());
    let scroll = store.views().get(view).unwrap().scroll.clone();
    assert!((scroll.x - (content_width - text_width)).abs() < WIDTH_TOLERANCE);
    let visible = viewport.height();
    let content_height = (SHORT_ROWS + 2) as f32 * LINE_HEIGHT;
    let slider = (visible * visible / content_height)
        .floor()
        .max(SCROLLBAR_MIN_SLIDER);
    let ratio = (visible - slider) / (content_height - visible);
    let track = pos2(
        viewport.right() - VERTICAL_SCROLLBAR_SIZE / CENTER_DIVISOR,
        viewport.top() + visible / CENTER_DIVISOR,
    );
    let pressed = draw(
        &context,
        &mut store,
        view,
        vec![Event::PointerMoved(track), press(track, true)],
    );
    let jumped = ((visible / CENTER_DIVISOR - slider / CENTER_DIVISOR) / ratio).round();
    assert_eq!(store.views().get(view).unwrap().scroll.y, jumped);
    assert!(pressed.rects.iter().any(|rect| {
        rect.width() == VERTICAL_SCROLLBAR_SIZE
            && rect.right() == viewport.right()
            && rect.height() == slider
    }));
    let dragged = pos2(track.x, track.y + SLIDER_DRAG);
    draw(
        &context,
        &mut store,
        view,
        vec![Event::PointerMoved(dragged)],
    );
    let moved = (((jumped * ratio).round() + SLIDER_DRAG) / ratio).round();
    assert_eq!(store.views().get(view).unwrap().scroll.y, moved);
    draw(&context, &mut store, view, vec![press(dragged, false)]);
    assert_eq!(store.views().get(view).unwrap().scroll.y, moved);
    assert_eq!(selection(&store, view), (0, 0));
    let track_length = text_width - VERTICAL_SCROLLBAR_SIZE;
    let horizontal_slider = (text_width * track_length / content_width)
        .floor()
        .max(SCROLLBAR_MIN_SLIDER);
    let horizontal_ratio = (track_length - horizontal_slider) / (content_width - text_width);
    let horizontal = pos2(
        viewport.left() + gutter + track_length / CENTER_DIVISOR,
        viewport.bottom() - HORIZONTAL_SCROLLBAR_SIZE / CENTER_DIVISOR,
    );
    draw(
        &context,
        &mut store,
        view,
        vec![Event::PointerMoved(horizontal), press(horizontal, true)],
    );
    let centered = ((track_length / CENTER_DIVISOR - horizontal_slider / CENTER_DIVISOR)
        / horizontal_ratio)
        .round();
    assert!((store.views().get(view).unwrap().scroll.x - centered).abs() < WIDTH_TOLERANCE);
    draw(&context, &mut store, view, vec![press(horizontal, false)]);
    assert_eq!(selection(&store, view), (0, 0));
    let body = pos2(viewport.left() + gutter, viewport.top() + TEXT_CLICK_Y);
    draw(
        &context,
        &mut store,
        view,
        vec![Event::PointerMoved(body), press(body, true)],
    );
    draw(&context, &mut store, view, vec![press(body, false)]);
    assert!(selection(&store, view).1 > LONG_LINE);
    let (mut short, short_view) = fixture("fits", false);
    let context = Context::default();
    draw(&context, &mut short, short_view, Vec::new());
    let current = short.views().get(short_view).unwrap().clone();
    short
        .set_view_state(
            short_view,
            current.selection,
            ScrollPosition {
                x: FAR_SCROLL,
                y: 0.0,
            },
            current.folds,
        )
        .unwrap();
    let drawn = draw(&context, &mut short, short_view, Vec::new());
    assert_eq!(short.views().get(short_view).unwrap().scroll.x, 0.0);
    assert!(
        drawn
            .rects
            .iter()
            .all(|rect| rect.width() != VERTICAL_SCROLLBAR_SIZE)
    );
}

#[test]
fn 파일별_들여쓰기는_실제_tab_입력에_적용되고_다른_editor의_기본값을_바꾸지_않는다() {
    let base = editor();
    for (spaces, expected) in [(true, "  draft"), (false, "\tdraft")] {
        let (mut store, view) = fixture("draft", false);
        let context = Context::default();
        let configured = base.with_indent(IndentOptions {
            tab_size: FILE_INDENT_SIZE,
            insert_spaces: spaces,
        });
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events: vec![key(Key::Tab, false)],
                ..Default::default()
            },
            |ui| {
                let shown = configured.show(ui, &mut store, view, true).unwrap();
                assert!(shown.changed);
                assert!(shown.errors.is_empty());
            },
        );
        output.textures_delta.clear();
        assert_eq!(text(&store, view), expected);
    }
    assert_eq!(base.appearance.indent, "    ");
    assert_eq!(base.appearance.font.size, FONT_SIZE);
}

#[test]
fn external_keymap은_재지정한_저장의_예전_cmd_s를_실행하지_않는다() {
    let (mut store, view) = fixture("draft", false);
    let context = Context::default();
    let editor = editor();
    let mut save_requested = false;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: vec![key(Key::S, true)],
            ..Default::default()
        },
        |ui| {
            save_requested = editor
                .show_with_keymap(ui, &mut store, view, true, |_, _, _| false)
                .unwrap()
                .save_requested;
        },
    );
    output.textures_delta.clear();
    assert!(!save_requested);
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: vec![key(Key::S, true)],
            ..Default::default()
        },
        |ui| {
            save_requested = editor
                .show(ui, &mut store, view, true)
                .unwrap()
                .save_requested;
        },
    );
    output.textures_delta.clear();
    assert!(save_requested);
    assert_eq!(text(&store, view), "draft");
}

#[test]
fn reveal은_단일_readonly_editor를_중앙표시하고_수평_cursor와_focus를_보존한다() {
    let target = format!("{}漢𐐀e\u{301}", "x".repeat(REVEAL_PREFIX));
    let text = (0..ROWS)
        .map(|line| {
            if line == SCROLL_LINE {
                format!("{target}\n")
            } else {
                format!("line {line}\n")
            }
        })
        .collect::<String>();
    let (mut store, view) = fixture(&text, true);
    let document = store.views().get(view).unwrap().document;
    let other = store
        .attach_view(
            ViewKey {
                window: "auxiliary".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let initial = store.documents().snapshot(document).unwrap();
    let context = Context::default();
    let mut viewport = Rect::NOTHING;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| {
            viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let editor = editor();
            editor
                .reveal(
                    ui,
                    &mut store,
                    view,
                    (SCROLL_LINE + 1) as f64,
                    (target.encode_utf16().count() + 1) as f64,
                )
                .unwrap();
            let shown = editor.show(ui, &mut store, view, true).unwrap();
            assert!(shown.rendered_lines.contains(&SCROLL_LINE));
            assert!(shown.response.has_focus());
            assert!(!shown.changed);
            assert!(shown.errors.is_empty());
            let scroll = &store.views().get(view).unwrap().scroll;
            assert!(scroll.x > 0.0);
            let expected = ((SCROLL_LINE as f32 + 1.0 / CENTER_DIVISOR) * LINE_HEIGHT
                - viewport.height() / CENTER_DIVISOR)
                .max(0.0);
            assert_eq!(scroll.y, expected);
        },
    );
    output.textures_delta.clear();
    assert!(output.platform_output.ime.is_none());
    let cursor = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::epaint::Shape::LineSegment { points, stroke }
                if stroke.color == Color32::WHITE =>
            {
                Some(Rect::from_two_pos(points[0], points[1]))
            }
            _ => None,
        })
        .expect("focused readonly editor paints its caret");
    assert!(cursor.left() >= viewport.left());
    assert!(cursor.right() <= viewport.right());
    assert!((cursor.center().y - viewport.center().y).abs() <= LINE_HEIGHT);
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, initial.revision);
    assert_eq!(after.dirty, initial.dirty);
    assert_eq!(after.rope, initial.rope);
    assert_eq!(
        store.views().get(other).unwrap().selection,
        SelectionSet::default()
    );
    assert_eq!(
        store.views().get(other).unwrap().scroll,
        ScrollPosition::default()
    );
}

fn fixture(text: &str, read_only: bool) -> (EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let tab = TabId::new();
    let document = store
        .open_file(
            "/synthetic/editor.rs".into(),
            OpenedFile {
                path: "/synthetic/editor.rs".into(),
                content: text.into(),
                language_id: "rust".into(),
                byte_size: u32::try_from(text.len()).unwrap(),
                line_count: 1,
                tier: FileSizeTier::Normal,
                read_only,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab,
            },
            document,
        )
        .unwrap();
    (store, view)
}

fn editor() -> NativeEditor {
    NativeEditor {
        appearance: EditorAppearance {
            font: FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: PADDING,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            muted: Color32::GRAY,
            selection: Color32::BLUE,
            cursor: Color32::WHITE,
            current_line: Color32::DARK_GRAY,
            line_numbers: true,
            indent: "    ".into(),
        },
    }
}

fn frame(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    events: Vec<Event>,
    focus: bool,
) -> (std::ops::Range<usize>, Vec<EditorError>, bool) {
    let mut result = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            let output = editor().show(ui, store, view, focus).unwrap();
            result = Some((output.rendered_lines, output.errors, output.save_requested));
        },
    );
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
    result.unwrap()
}

fn key(key: Key, command: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            command,
            ..Modifiers::default()
        },
    }
}

#[test]
fn 비활성_editor는_확인창_뒤의_text_ime와_focus를_소비하지_않는다() {
    let (mut store, view) = fixture("abc", false);
    let document = store.views().get(view).unwrap().document;
    let context = Context::default();
    frame(&context, &mut store, view, Vec::new(), true);
    let before = store.documents().snapshot(document).unwrap();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: vec![
                Event::Text("must not enter the disabled document".into()),
                Event::Ime(egui::ImeEvent::Commit("確認".into())),
            ],
            ..Default::default()
        },
        |ui| {
            ui.add_enabled_ui(false, |ui| {
                let output = editor().show(ui, &mut store, view, true).unwrap();
                assert!(!output.changed);
            });
            assert_eq!(ui.input(|input| input.events.len()), 2);
        },
    );
    output.textures_delta.clear();
    let after = store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.rope, before.rope);
}

fn text(store: &EditorStore, view: ViewId) -> String {
    store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap()
        .rope
        .to_string()
}

#[test]
fn 실제_editor_surface는_입력_선택_ime_commit과_stale_거절을_연결한다() {
    let (mut store, view) = fixture("abc", false);
    let context = Context::default();
    frame(&context, &mut store, view, Vec::new(), true);
    let (_, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![key(Key::End, false), Event::Text("한".into())],
        false,
    );
    assert!(errors.is_empty());
    assert_eq!(text(&store, view), "abc한");
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "글".into(),
            active_range_chars: None,
        })],
        false,
    );
    assert_eq!(text(&store, view), "abc한");
    assert!(store.views().get(view).unwrap().composition.is_some());
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Commit("글".into()))],
        false,
    );
    assert_eq!(text(&store, view), "abc한글");
    frame(&context, &mut store, view, vec![key(Key::Z, true)], false);
    assert_eq!(text(&store, view), "abc");
    let (_, errors, save) = frame(&context, &mut store, view, vec![key(Key::S, true)], false);
    assert!(errors.is_empty());
    assert!(save);
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "旧".into(),
            active_range_chars: None,
        })],
        false,
    );
    let document = store.views().get(view).unwrap().document;
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "external".into(),
                }],
                group: UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    let before = text(&store, view);
    let (_, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Commit("旧".into()))],
        false,
    );
    assert_eq!(errors, vec![EditorError::StaleRevision]);
    assert_eq!(text(&store, view), before);
}

#[test]
fn 큰_document의_화면은_보이는_줄만_렌더하고_readonly_입력을_거절한다() {
    let text = "let x = 1;\n".repeat(ROWS);
    let (mut store, view) = fixture(&text, true);
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 0, head: 0 }],
            },
            ScrollPosition {
                x: 0.0,
                y: SCROLL_LINE as f32 * LINE_HEIGHT,
            },
            current.folds,
        )
        .unwrap();
    let context = Context::default();
    let (lines, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![Event::Paste("must-not-write".into())],
        true,
    );
    assert_eq!(lines.start, SCROLL_LINE);
    assert!(lines.len() <= (SCREEN[1] / LINE_HEIGHT).ceil() as usize + 1);
    assert_eq!(errors, vec![EditorError::ReadOnly]);
    assert_eq!(
        store
            .documents()
            .snapshot(current.document)
            .unwrap()
            .revision,
        0
    );
    let second = store
        .attach_view(
            ViewKey {
                window: "aux".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            current.document,
        )
        .unwrap();
    let auxiliary = Context::default();
    let (lines, _, _) = frame(&auxiliary, &mut store, second, Vec::new(), true);
    assert_eq!(lines.start, 0);
    assert_eq!(
        store.views().get(view).unwrap().scroll.y,
        SCROLL_LINE as f32 * LINE_HEIGHT
    );
}

#[test]
fn 커서_이동과_큰_선택_삭제는_스크롤을_현재_문서로_복원한다() {
    let (mut store, view) = fixture(&"row\n".repeat(ROWS), false);
    let context = Context::default();
    frame(&context, &mut store, view, Vec::new(), true);
    let (lines, errors, _) = frame(&context, &mut store, view, vec![key(Key::End, true)], false);
    assert!(errors.is_empty());
    assert!(lines.start > SCROLL_LINE);
    frame(&context, &mut store, view, vec![key(Key::A, true)], false);
    let (lines, errors, _) = frame(
        &context,
        &mut store,
        view,
        vec![Event::Text("small".into())],
        false,
    );
    assert!(errors.is_empty());
    assert_eq!(lines, 0..1);
    assert_eq!(store.views().get(view).unwrap().scroll.y, 0.0);
    assert_eq!(text(&store, view), "small");
}
