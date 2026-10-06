use egui::epaint::{ClippedShape, Shape};
use egui::os::OperatingSystem;
use egui::text::{CCursor, LayoutJob};
use egui::{
    Color32, Context, Event, FontId, ImeEvent, Key, Modifiers, MouseWheelUnit, OutputCommand,
    PointerButton, Pos2, RawInput, Rect, TouchPhase, Vec2, pos2, vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_model::settings::Settings;
use taide_native_editor::document::{Edit, EditorError, UndoGroup};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::line_breaks::{WrapSettings, WrappingIndent, create_line_breaks};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_row_text::{RowColumns, RowSection, RowText};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorDisplayOptions, EditorPresentation, NativeEditor,
};
use taide_native_ui::presentation::{editor_presentation, update_editor_font_size};

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
const PLAIN_DOCUMENT: &str = "fn main() {\n\tlet 값 = \"漢字\";\r\n\n    // e\u{301} 𐐀\n}\n";
const SELECTION_PRESS: Pos2 = pos2(80.0, 30.0);
const SELECTION_RELEASE: Pos2 = pos2(140.0, 70.0);
const SCROLLED_ROWS: usize = 60;
const SCROLLED_LONG_ROW: usize = 8;
const SCROLLED_LONG_ROW_COLUMNS: usize = 100;
const WHEEL_POINTER: Pos2 = pos2(400.0, 100.0);
const WHEEL_DELTA: Vec2 = vec2(-24.0, -130.0);
const SCROLLED_CLICK: Pos2 = pos2(200.0, 40.0);
const FRAME_STATE_LINES: usize = 2;
const PLAIN_ROWS: usize = 6;
const SCROLLED_FIRST_ROW: usize = 6;
const ROW_SOURCE: &str = "\t값 = \"𐐀\";";
const ROW_DISPLAY: &str = "    값 = \"𐐀\";";
const ROW_SPLIT: usize = 2;
const TAB_SIZE: u32 = 4;
const WIDE_TAB_SIZE: u32 = 8;
const CONTINUED_START_COLUMN: f64 = 9.0;
const CONTINUED_INDENT: u32 = 2;
const FRACTIONAL_START_COLUMN: f64 = 2.5;
const TALL_LINE_HEIGHT: f32 = 27.0;
const TIGHT_LINE_HEIGHT: f32 = 12.0;
const CENTERED_DOCUMENT: &str = "ab\ncd";
const ROUNDED_HALF_LEADING_SLACK: f32 = 1.0;
const TEXTS_PER_ROW: usize = 2;
const LARGE_FONT_SIZE: u32 = 48;
const LARGE_DOCUMENT: &str = "abcdef\nabcdef";
const LARGE_CLICK_PREFIX: &str = "abc";
const LARGE_CLICK_ROW: usize = 1;
const LARGE_CLICK_EDGE_INSET: f32 = 1.0;
const LARGE_CLICK_PAST_BOUNDARY: f32 = 1.0;
const MONACO_LINE_HEIGHTS: [(u32, f32); 5] =
    [(13, 20.0), (14, 21.0), (15, 23.0), (12, 18.0), (4, 8.0)];
const WRAP_CURSOR_ROOM: f32 = 2.0;
const ADVANCE_SAMPLE: usize = 256;
const WRAPPED_LINE: usize = 200;
const WRAPPED_LINE_ROWS: usize = 3;
const WRAPPED_LINES: usize = 30;
const NARROW_SCREEN: [f32; 2] = [400.0, 200.0];
const WIDE_FONT_SIZE: u32 = 28;
const WRAP_CLICK_COLUMN: usize = 10;
const WRAP_CLICK_PAST_BOUNDARY: f32 = 1.0;
const WRAP_DRAG_FROM_COLUMN: usize = 5;
const WRAP_DRAG_TO_COLUMN: usize = 3;
const REVEAL_WRAPPED_LINE: usize = 5;
const INDENTED_WORDS: usize = 40;
const FULL_WIDTH_COLUMNS: f64 = 2.0;
const STABLE_LINE: usize = 12;
const STABLE_DELTA: f32 = 7.0;
const PLAIN_FRAME: &[&str] = &[
    "rect 0.00,0.00..800.00,200.00 fill 000000ff clip 0.00,0.00..800.00,200.00",
    "rect 0.00,0.00..800.00,20.00 fill 606060ff clip 0.00,0.00..800.00,200.00",
    "line 41.28,0.00 to 41.28,20.00 stroke 1.00 ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"fn main() {\" at 41.28,2.00 size 92.72,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"1\" at 24.84,2.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"    let 값 = \\\"漢字\\\";\" at 41.28,22.00 size 143.28,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"2\" at 24.84,22.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"\" at 41.28,42.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"3\" at 24.84,42.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"    // e\\u{301} 𐐀\" at 41.28,62.00 size 84.28,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"4\" at 24.84,62.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"}\" at 41.28,82.00 size 8.44,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"5\" at 24.84,82.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"\" at 41.28,102.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"6\" at 24.84,102.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "ime 0.00,0.00..800.00,200.00 cursor 41.28,2.00..41.28,18.00",
    "scroll 0.00,0.00 selection 0..0 preedit None",
];
const SELECTION_FRAME: &[&str] = &[
    "rect 0.00,0.00..800.00,200.00 fill 000000ff clip 0.00,0.00..800.00,200.00",
    "text \"fn main() {\" at 41.28,2.00 size 92.72,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"1\" at 24.84,2.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 83.28,20.00..800.00,40.00 fill 0000ffff clip 41.28,0.00..800.00,200.00",
    "text \"    let 값 = \\\"漢字\\\";\" at 41.28,22.00 size 143.28,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"2\" at 24.84,22.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 41.28,40.00..800.00,60.00 fill 0000ffff clip 41.28,0.00..800.00,200.00",
    "text \"\" at 41.28,42.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"3\" at 24.84,42.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 0.00,60.00..800.00,80.00 fill 606060ff clip 0.00,0.00..800.00,200.00",
    "rect 41.28,60.00..125.56,80.00 fill 0000ffff clip 41.28,0.00..800.00,200.00",
    "text \"    // e\\u{301} 𐐀\" at 41.28,62.00 size 84.28,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"4\" at 24.84,62.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"}\" at 41.28,82.00 size 8.44,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"5\" at 24.84,82.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"\" at 41.28,102.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"6\" at 24.84,102.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "ime 0.00,0.00..800.00,200.00 cursor 125.56,62.00..125.56,78.00",
    "scroll 0.00,0.00 selection 14..50 preedit None",
];
const REVEALED_VIEW: &str = "scroll 85.16,70.00 selection 148..148 preedit None";
const SCROLLED_COMPOSITION_TRACE: &[&str] = &[
    "scroll 24.00,130.00 selection 0..0 preedit None",
    "scroll 24.00,130.00 selection 70..70 preedit None",
    "scroll 85.16,130.00 selection 148..148 preedit None",
    "rect 0.00,0.00..800.00,200.00 fill 000000ff clip 0.00,0.00..800.00,200.00",
    "text \"row 6\" at -43.88,-8.00 size 42.16,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"7\" at 24.84,-8.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 7\" at -43.88,12.00 size 42.16,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"8\" at 24.84,12.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 0.00,30.00..800.00,50.00 fill 606060ff clip 0.00,0.00..800.00,200.00",
    "line 799.00,30.00 to 799.00,50.00 stroke 1.00 ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\" at -43.88,32.00 size 842.88,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"9\" at 24.84,32.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 9\" at -43.88,52.00 size 42.16,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"10\" at 16.44,52.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 10\" at -43.88,72.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"11\" at 16.44,72.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 11\" at -43.88,92.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"12\" at 16.44,92.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 12\" at -43.88,112.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"13\" at 16.44,112.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 13\" at -43.88,132.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"14\" at 16.44,132.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 14\" at -43.88,152.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"15\" at 16.44,152.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 15\" at -43.88,172.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"16\" at 16.44,172.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 16\" at -43.88,192.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "text \"17\" at 16.44,192.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 799.00,32.00..815.84,48.00 fill 000000ff clip 41.28,0.00..800.00,200.00",
    "text \"한글\" at 799.00,32.00 size 16.84,16.00 color ffffffff job simple ffffffff clip 41.28,0.00..800.00,200.00",
    "line 799.00,48.00 to 815.84,48.00 stroke 1.00 ffffffff clip 41.28,0.00..800.00,200.00",
    "rect 786.00,22.00..800.00,55.00 fill 35353555 clip 0.00,0.00..800.00,200.00",
    "rect 112.28,188.00..746.28,200.00 fill 35353555 clip 0.00,0.00..800.00,200.00",
    "ime 0.00,0.00..800.00,200.00 cursor 799.00,32.00..799.00,48.00",
    "scroll 85.16,130.00 selection 148..148 preedit Some(\"한글\")",
];

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
    measure_with(context, text, FontId::monospace(FONT_SIZE))
}

fn measure_with(context: &Context, text: &str, font: FontId) -> f32 {
    let mut width = 0.0;
    let mut output = context.run_ui(RawInput::default(), |ui| {
        width = ui
            .painter()
            .layout_no_wrap(text.into(), font.clone(), Color32::WHITE)
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

fn bounds(rect: Rect) -> String {
    format!(
        "{:.2},{:.2}..{:.2},{:.2}",
        rect.left(),
        rect.top(),
        rect.right(),
        rect.bottom()
    )
}

fn point(position: Pos2) -> String {
    format!("{:.2},{:.2}", position.x, position.y)
}

fn hex(color: Color32) -> String {
    let [red, green, blue, alpha] = color.to_array();
    format!("{red:02x}{green:02x}{blue:02x}{alpha:02x}")
}

fn painted(clipped: &ClippedShape) -> String {
    let clip = bounds(clipped.clip_rect);
    match &clipped.shape {
        Shape::Rect(shape) => format!(
            "rect {} fill {} clip {clip}",
            bounds(shape.rect),
            hex(shape.fill)
        ),
        Shape::LineSegment { points, stroke } => format!(
            "line {} to {} stroke {:.2} {} clip {clip}",
            point(points[0]),
            point(points[1]),
            stroke.width,
            hex(stroke.color)
        ),
        Shape::Text(shape) => {
            let job = &shape.galley.job;
            let layout = job
                .sections
                .first()
                .filter(|section| {
                    **job
                        == LayoutJob::simple(
                            job.text.clone(),
                            FontId::monospace(FONT_SIZE),
                            section.format.color,
                            f32::INFINITY,
                        )
                })
                .map_or("custom".into(), |section| {
                    format!("simple {}", hex(section.format.color))
                });
            format!(
                "text {:?} at {} size {} color {} job {layout} clip {clip}",
                job.text,
                point(shape.pos),
                point(shape.galley.size().to_pos2()),
                hex(shape.fallback_color)
            )
        }
        _ => "other".into(),
    }
}

fn view_line(store: &EditorStore, view: ViewId) -> String {
    let state = store.views().get(view).unwrap();
    let (anchor, head) = selection(store, view);
    format!(
        "scroll {:.2},{:.2} selection {anchor}..{head} preedit {:?}",
        state.scroll.x,
        state.scroll.y,
        state
            .composition
            .as_ref()
            .map(|composition| composition.preedit.as_str())
    )
}

fn snapshot(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    events: Vec<Event>,
) -> Vec<String> {
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
            let shown = editor().show(ui, store, view, true).unwrap();
            assert!(shown.errors.is_empty());
        },
    );
    output.textures_delta.clear();
    let mut lines: Vec<String> = output.shapes.iter().map(painted).collect();
    lines.push(output.platform_output.ime.map_or("ime none".into(), |ime| {
        format!(
            "ime {} cursor {}",
            bounds(ime.rect),
            bounds(ime.cursor_rect)
        )
    }));
    lines.push(view_line(store, view));
    lines
}

fn assert_frame(actual: &[String], expected: &[&str]) {
    assert!(actual == expected, "{actual:#?}");
}

#[test]
fn 평문_화면은_배경_현재_줄_캐럿_본문_줄_번호를_고정된_순서와_좌표로_그린다() {
    let (mut store, view) = fixture(PLAIN_DOCUMENT, false);
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    assert_frame(
        &snapshot(&context, &mut store, view, Vec::new()),
        PLAIN_FRAME,
    );
}

#[test]
fn 포인터_드래그로_만든_여러_줄_선택은_줄별_선택_영역을_고정된_좌표로_그린다() {
    let (mut store, view) = fixture(PLAIN_DOCUMENT, false);
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    for events in [
        vec![
            Event::PointerMoved(SELECTION_PRESS),
            press(SELECTION_PRESS, true),
        ],
        vec![press(SELECTION_PRESS, false)],
        vec![press(SELECTION_PRESS, true)],
        vec![Event::PointerMoved(SELECTION_RELEASE)],
    ] {
        draw(&context, &mut store, view, events);
    }
    assert_frame(
        &snapshot(
            &context,
            &mut store,
            view,
            vec![press(SELECTION_RELEASE, false)],
        ),
        SELECTION_FRAME,
    );
}

fn scrolled_document() -> String {
    (0..SCROLLED_ROWS)
        .map(|row| {
            if row == SCROLLED_LONG_ROW {
                "x".repeat(SCROLLED_LONG_ROW_COLUMNS)
            } else {
                format!("row {row}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn reveal은_긴_줄의_끝_열을_고정된_가로_세로_스크롤로_맞춘다() {
    let (mut store, view) = fixture(&scrolled_document(), false);
    let context = Context::default();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| {
            editor()
                .reveal(
                    ui,
                    &mut store,
                    view,
                    (SCROLLED_LONG_ROW + 1) as f64,
                    (SCROLLED_LONG_ROW_COLUMNS + 1) as f64,
                )
                .unwrap();
        },
    );
    output.textures_delta.clear();
    assert_eq!(view_line(&store, view), REVEALED_VIEW);
}

#[test]
fn 스크롤_뒤_ime_조합은_preedit와_후보_창_좌표를_스크롤된_캐럿에_맞춘다() {
    let (mut store, view) = fixture(&scrolled_document(), false);
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    let wheel = |delta: Vec2, phase: TouchPhase| Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta,
        phase,
        modifiers: Modifiers::NONE,
    };
    draw(
        &context,
        &mut store,
        view,
        vec![
            Event::PointerMoved(WHEEL_POINTER),
            wheel(Vec2::ZERO, TouchPhase::Start),
            wheel(WHEEL_DELTA, TouchPhase::Move),
        ],
    );
    let mut trace = vec![view_line(&store, view)];
    draw(
        &context,
        &mut store,
        view,
        vec![
            Event::PointerMoved(SCROLLED_CLICK),
            press(SCROLLED_CLICK, true),
        ],
    );
    draw(
        &context,
        &mut store,
        view,
        vec![press(SCROLLED_CLICK, false)],
    );
    trace.push(view_line(&store, view));
    draw(&context, &mut store, view, vec![key(Key::End, false)]);
    trace.push(view_line(&store, view));
    trace.extend(snapshot(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "한글".into(),
            active_range_chars: None,
        })],
    ));
    assert_frame(&trace, SCROLLED_COMPOSITION_TRACE);
}

fn row_text(source: &str, tab_size: u32, start_column: f64, indent_columns: u32) -> RowText {
    RowText::expanded(
        source,
        RowColumns {
            tab_size,
            start_column,
            indent_columns,
        },
        Color32::WHITE,
    )
}

#[test]
fn 표시_줄_텍스트는_탭을_탭_정지까지의_공백으로_펼치고_단일_구간_layout_job을_만든다() {
    let font = FontId::monospace(FONT_SIZE);
    let row = row_text(ROW_SOURCE, TAB_SIZE, 0.0, 0);
    let tab_spaces = TAB_SIZE as usize;
    let offsets: Vec<usize> = ROW_SOURCE
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([ROW_SOURCE.len()])
        .collect();
    assert_eq!(row.text, ROW_DISPLAY);
    assert_eq!(row.indent_chars, 0);
    assert_eq!(
        row.sections,
        [RowSection {
            chars: 0..ROW_DISPLAY.chars().count(),
            foreground: Color32::WHITE,
        }]
    );
    assert_eq!(
        row.layout_job(&font),
        LayoutJob::simple(
            ROW_DISPLAY.into(),
            font.clone(),
            Color32::WHITE,
            f32::INFINITY
        )
    );
    for (character, offset) in offsets.iter().enumerate() {
        let display_char = if character == 0 {
            0
        } else {
            character + tab_spaces - 1
        };
        assert_eq!(row.model_byte(display_char), *offset);
        assert_eq!(row.display_char(*offset), display_char);
    }
    assert_eq!([1, 2, 3].map(|inside| row.model_byte(inside)), [0, 0, 1]);
    assert_eq!(
        row.model_byte(ROW_DISPLAY.chars().count() + 1),
        ROW_SOURCE.len()
    );
    assert_eq!(row.display_char(offsets[1] + 1), tab_spaces);
    let empty = row_text("", TAB_SIZE, 0.0, 0);
    assert_eq!(empty.model_bytes, [0]);
    assert_eq!(
        empty.layout_job(&font),
        LayoutJob::simple(String::new(), font, Color32::WHITE, f32::INFINITY)
    );
}

#[test]
fn 표시_줄_탭은_다음_탭_정지까지만_펼쳐지고_이어지는_줄은_시작_열과_들여쓰기를_따른다() {
    for (source, tab_size, display) in [
        ("a\tb", TAB_SIZE, "a   b"),
        ("abcd\te", TAB_SIZE, "abcd    e"),
        ("한\tb", TAB_SIZE, "한  b"),
        ("\u{10400}\tb", TAB_SIZE, "\u{10400}  b"),
        ("ab\tc", FILE_INDENT_SIZE, "ab  c"),
        ("a\t\tb", WIDE_TAB_SIZE, "a               b"),
    ] {
        assert_eq!(
            row_text(source, tab_size, 0.0, 0).text,
            display,
            "{source:?}"
        );
    }
    let continued = row_text("\tb", TAB_SIZE, CONTINUED_START_COLUMN, CONTINUED_INDENT);
    assert_eq!(continued.text, "     b");
    assert_eq!(continued.indent_chars, CONTINUED_INDENT as usize);
    assert_eq!(continued.model_bytes, [0, 0, 0, 0, 0, 1, 2]);
    assert_eq!(
        [0, 1, 2].map(|byte| continued.display_char(byte)),
        [2, 5, 6]
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5, 6].map(|display_char| continued.model_byte(display_char)),
        [0, 0, 0, 0, 1, 1, 2]
    );
    let narrow = row_text("a\tb", TAB_SIZE, FRACTIONAL_START_COLUMN, 0);
    assert_eq!(narrow.text, "ab");
    assert_eq!(narrow.model_bytes, [0, 2, 3]);
    assert_eq!(
        [0, 1, 2, 3].map(|byte| narrow.display_char(byte)),
        [0, 1, 1, 2]
    );
}

#[test]
fn 표시_줄_구간은_문자_범위를_layout_job의_바이트_구간과_전경색으로_옮긴다() {
    let mut row = row_text(ROW_SOURCE, TAB_SIZE, 0.0, 0);
    row.sections = vec![
        RowSection {
            chars: 0..ROW_SPLIT,
            foreground: Color32::RED,
        },
        RowSection {
            chars: ROW_SPLIT..ROW_DISPLAY.chars().count(),
            foreground: Color32::GREEN,
        },
    ];
    let job = row.layout_job(&FontId::monospace(FONT_SIZE));
    let split = ROW_DISPLAY.char_indices().nth(ROW_SPLIT).unwrap().0;
    assert_eq!(job.text, ROW_DISPLAY);
    assert_eq!(
        job.sections
            .iter()
            .map(|section| (
                section.byte_range.start.0,
                section.byte_range.end.0,
                section.format.color
            ))
            .collect::<Vec<_>>(),
        [
            (0, split, Color32::RED),
            (split, ROW_DISPLAY.len(), Color32::GREEN)
        ]
    );
    assert_eq!(job.wrap.max_width, f32::INFINITY);
    assert!(job.break_on_newline);
}

#[test]
fn 기본_presentation의_show_presented는_기존_show와_같은_화면과_좌표_정보를_돌려준다() {
    let (mut store, view) = fixture(PLAIN_DOCUMENT, false);
    let context = Context::default();
    let viewport = draw(&context, &mut store, view, Vec::new()).viewport;
    let mut geometry = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| {
            let shown = editor()
                .show_presented(
                    ui,
                    &mut store,
                    view,
                    true,
                    |_, _, _| false,
                    |_| None,
                    &EditorPresentation::default(),
                )
                .unwrap();
            assert_eq!(shown.rendered_lines, 0..PLAIN_ROWS);
            geometry = Some(shown.geometry);
        },
    );
    output.textures_delta.clear();
    let shapes: Vec<String> = output.shapes.iter().map(painted).collect();
    assert_frame(
        &shapes,
        &PLAIN_FRAME[..PLAIN_FRAME.len() - FRAME_STATE_LINES],
    );
    let gutter = measure(&context, "000") + PADDING * 2.0;
    let geometry = geometry.unwrap();
    assert_eq!(geometry.rect, viewport);
    assert_eq!(
        geometry.gutter_rect,
        Rect::from_min_max(
            viewport.min,
            pos2(viewport.left() + gutter, viewport.bottom())
        )
    );
    assert_eq!(
        geometry.content_rect,
        Rect::from_min_max(pos2(viewport.left() + gutter, viewport.top()), viewport.max)
    );
    assert_eq!(geometry.line_height, LINE_HEIGHT);
    assert_eq!(geometry.visible_rows, 0..PLAIN_ROWS);
    assert_eq!(geometry.scroll, Vec2::ZERO);
    let (mut scrolled, scrolled_view) = fixture(&scrolled_document(), false);
    let context = Context::default();
    draw(&context, &mut scrolled, scrolled_view, Vec::new());
    let current = scrolled.views().get(scrolled_view).unwrap().clone();
    scrolled
        .set_view_state(
            scrolled_view,
            current.selection,
            ScrollPosition {
                x: -WHEEL_DELTA.x,
                y: -WHEEL_DELTA.y,
            },
            current.folds,
        )
        .unwrap();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| {
            let shown = editor()
                .show(ui, &mut scrolled, scrolled_view, true)
                .unwrap();
            let rows = (SCREEN[1] / LINE_HEIGHT).ceil() as usize + 1;
            assert_eq!(
                shown.geometry.visible_rows,
                SCROLLED_FIRST_ROW..SCROLLED_FIRST_ROW + rows
            );
            assert_eq!(shown.geometry.visible_rows, shown.rendered_lines);
            assert_eq!(shown.geometry.scroll, -WHEEL_DELTA);
        },
    );
    output.textures_delta.clear();
}

struct Presented {
    shapes: Vec<ClippedShape>,
    viewport: Rect,
    font_height: f32,
}

fn present(
    context: &Context,
    surface: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    events: Vec<Event>,
) -> Presented {
    let mut viewport = Rect::NOTHING;
    let mut font_height = 0.0;
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
            font_height = ui.fonts_mut(|fonts| fonts.row_height(&surface.appearance.font));
            let shown = surface.show(ui, store, view, true).unwrap();
            assert!(shown.errors.is_empty());
        },
    );
    output.textures_delta.clear();
    Presented {
        shapes: output.shapes,
        viewport,
        font_height,
    }
}

#[test]
fn 본문과_줄_번호의_글리프는_줄_상자의_세로_가운데에_놓이고_캐럿은_줄_상자_전체를_덮는다() {
    for line_height in [LINE_HEIGHT, TALL_LINE_HEIGHT, TIGHT_LINE_HEIGHT] {
        let (mut store, view) = fixture(CENTERED_DOCUMENT, false);
        let context = Context::default();
        let mut surface = editor();
        surface.appearance.line_height = line_height;
        present(&context, &surface, &mut store, view, Vec::new());
        let presented = present(&context, &surface, &mut store, view, Vec::new());
        let top = presented.viewport.top();
        let half_leading = ((line_height - presented.font_height) / CENTER_DIVISOR).round();
        let glyphs: Vec<(String, f32)> = presented
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Text(text) => Some((text.galley.job.text.clone(), text.pos.y)),
                _ => None,
            })
            .collect();
        assert_eq!(
            glyphs
                .iter()
                .map(|(text, _)| text.as_str())
                .collect::<Vec<_>>(),
            ["ab", "1", "cd", "2"]
        );
        for (row, texts) in glyphs.chunks(TEXTS_PER_ROW).enumerate() {
            let row_top = top + row as f32 * line_height;
            for (_, glyph_top) in texts {
                let above = glyph_top - row_top;
                let below = row_top + line_height - (glyph_top + presented.font_height);
                assert_eq!(above, half_leading);
                assert!((above - below).abs() <= ROUNDED_HALF_LEADING_SLACK);
            }
        }
        let caret = presented
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::LineSegment { points, .. } => Some(*points),
                _ => None,
            })
            .expect("focused editor paints its caret");
        assert_eq!([caret[0].y, caret[1].y], [top, top + line_height]);
    }
}

#[test]
fn 큰_글꼴에서_줄_상자의_위아래_가장자리_클릭은_클릭한_열에_캐럿을_둔다() {
    let (mut store, view) = fixture(LARGE_DOCUMENT, false);
    let context = Context::default();
    let mut surface = editor();
    assert!(update_editor_font_size(
        &mut surface.appearance,
        LARGE_FONT_SIZE
    ));
    let font = surface.appearance.font.clone();
    let line_height = surface.appearance.line_height;
    let gutter = measure_with(&context, "000", font.clone()) + PADDING * 2.0;
    let prefix = measure_with(&context, LARGE_CLICK_PREFIX, font);
    let viewport = present(&context, &surface, &mut store, view, Vec::new()).viewport;
    let x = viewport.left() + gutter + prefix + LARGE_CLICK_PAST_BOUNDARY;
    let row_top = viewport.top() + LARGE_CLICK_ROW as f32 * line_height;
    let expected = LARGE_DOCUMENT.find('\n').unwrap() + 1 + LARGE_CLICK_PREFIX.len();
    for y in [
        row_top + LARGE_CLICK_EDGE_INSET,
        row_top + line_height - LARGE_CLICK_EDGE_INSET,
    ] {
        let position = pos2(x, y);
        present(
            &context,
            &surface,
            &mut store,
            view,
            vec![Event::PointerMoved(position), press(position, true)],
        );
        present(
            &context,
            &surface,
            &mut store,
            view,
            vec![press(position, false)],
        );
        assert_eq!(selection(&store, view), (expected, expected));
    }
}

#[test]
fn 편집기_줄_높이는_글꼴_크기의_1_5배를_정수로_반올림하고_최소값을_지킨다() {
    let mut surface = editor();
    for (font_size, line_height) in MONACO_LINE_HEIGHTS {
        update_editor_font_size(&mut surface.appearance, font_size);
        assert_eq!(surface.appearance.font.size, font_size as f32);
        assert_eq!(surface.appearance.line_height, line_height);
    }
}

#[test]
fn 설정의_word_wrap은_표시_옵션으로_전달되고_설정_기본값은_현재_동작과_같은_꺼짐이다() {
    let mut settings = Settings::default();
    assert_eq!(
        editor_presentation(&settings),
        EditorPresentation::default()
    );
    settings.editor_word_wrap = true;
    assert_eq!(editor_presentation(&settings), wrapping());
}

struct Wrapped {
    shapes: Vec<ClippedShape>,
    viewport: Rect,
    rendered_lines: std::ops::Range<usize>,
    visible_rows: std::ops::Range<usize>,
    ime_cursor: Option<Rect>,
}

fn wrapping() -> EditorPresentation {
    EditorPresentation {
        options: EditorDisplayOptions {
            word_wrap: true,
            ..Default::default()
        },
    }
}

fn show_wrapped(
    context: &Context,
    surface: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    screen: [f32; 2],
    events: Vec<Event>,
) -> Wrapped {
    let mut shown = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(screen[0], screen[1]),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            let viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let output = surface
                .show_presented(
                    ui,
                    store,
                    view,
                    true,
                    |_, _, _| false,
                    |_| None,
                    &wrapping(),
                )
                .unwrap();
            assert!(output.errors.is_empty());
            shown = Some((
                viewport,
                output.rendered_lines,
                output.geometry.visible_rows,
            ));
        },
    );
    output.textures_delta.clear();
    let (viewport, rendered_lines, visible_rows) = shown.unwrap();
    Wrapped {
        shapes: output.shapes,
        viewport,
        rendered_lines,
        visible_rows,
        ime_cursor: output.platform_output.ime.map(|ime| ime.cursor_rect),
    }
}

fn click_wrapped(
    context: &Context,
    surface: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    position: Pos2,
) -> Wrapped {
    show_wrapped(
        context,
        surface,
        store,
        view,
        SCREEN,
        vec![Event::PointerMoved(position), press(position, true)],
    );
    show_wrapped(
        context,
        surface,
        store,
        view,
        SCREEN,
        vec![press(position, false)],
    )
}

fn texts(shapes: &[ClippedShape], viewport: Rect, body: bool) -> Vec<(String, Pos2, f32)> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(text) if (clipped.clip_rect.left() > viewport.left()) == body => {
                Some((text.galley.job.text.clone(), text.pos, text.galley.size().x))
            }
            _ => None,
        })
        .collect()
}

fn body_rows(shown: &Wrapped) -> Vec<String> {
    texts(&shown.shapes, shown.viewport, true)
        .into_iter()
        .map(|(text, _, _)| text)
        .collect()
}

fn caret_line(shapes: &[ClippedShape]) -> [Pos2; 2] {
    shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            Shape::LineSegment { points, stroke } if stroke.color == Color32::WHITE => {
                Some(*points)
            }
            _ => None,
        })
        .expect("focused editor paints its caret")
}

fn assert_caret(shown: &Wrapped, x: f32, top: f32) {
    let caret = caret_line(&shown.shapes);
    assert_eq!(
        (caret[0].x, caret[0].y, caret[1].y),
        (x, top, top + LINE_HEIGHT)
    );
}

fn filled(shapes: &[ClippedShape], fill: Color32) -> Vec<Rect> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) if rect.fill == fill => Some(rect.rect),
            _ => None,
        })
        .collect()
}

fn caret_offset(context: &Context, text: &str, chars: usize) -> f32 {
    let mut offset = 0.0;
    let mut output = context.run_ui(RawInput::default(), |ui| {
        offset = ui
            .painter()
            .layout_no_wrap(text.into(), FontId::monospace(FONT_SIZE), Color32::WHITE)
            .pos_from_cursor(CCursor::new(chars))
            .left();
    });
    output.textures_delta.clear();
    offset
}

fn wrap_columns(context: &Context, font: FontId, width: f32) -> (usize, f32) {
    let advance =
        measure_with(context, &"x".repeat(ADVANCE_SAMPLE), font.clone()) / ADVANCE_SAMPLE as f32;
    let gutter = measure_with(context, "000", font) + PADDING * 2.0;
    (
        ((width - gutter - VERTICAL_SCROLLBAR_SIZE - WRAP_CURSOR_ROOM) / advance).floor() as usize,
        gutter,
    )
}

fn wrapped_lines() -> String {
    vec!["x".repeat(WRAPPED_LINE); WRAPPED_LINES].join("\n")
}

fn scroll_to(store: &mut EditorStore, view: ViewId, y: f32) {
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            current.selection,
            ScrollPosition { x: 0.0, y },
            current.folds,
        )
        .unwrap();
}

#[test]
fn wrap_표시_줄은_본문_폭의_wrap_열에서_나뉘고_줄_번호는_문서_줄의_첫_표시_줄에만_있다() {
    let long = "x".repeat(WRAPPED_LINE);
    let (mut store, view) = fixture(&format!("{long}\ntail"), false);
    let context = Context::default();
    let (columns, gutter) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let surface = editor();
    show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    let shown = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    assert_eq!(shown.viewport.width(), SCREEN[0]);
    assert!(columns * (WRAPPED_LINE_ROWS - 1) < WRAPPED_LINE);
    assert!(WRAPPED_LINE <= columns * WRAPPED_LINE_ROWS);
    let body = texts(&shown.shapes, shown.viewport, true);
    assert_eq!(
        body.iter()
            .map(|(text, _, _)| text.as_str())
            .collect::<Vec<_>>(),
        [
            &long[..columns],
            &long[..columns],
            &long[columns * 2..],
            "tail"
        ]
    );
    let widest = shown.viewport.width() - gutter - VERTICAL_SCROLLBAR_SIZE - WRAP_CURSOR_ROOM;
    for (row, (_, position, width)) in body.iter().enumerate() {
        assert_eq!(position.x, shown.viewport.left() + gutter);
        assert_eq!(position.y - body[0].1.y, row as f32 * LINE_HEIGHT);
        assert!(*width <= widest);
    }
    assert_eq!(
        texts(&shown.shapes, shown.viewport, false)
            .iter()
            .map(|(text, position, _)| (text.as_str(), position.y))
            .collect::<Vec<_>>(),
        [("1", body[0].1.y), ("2", body[WRAPPED_LINE_ROWS].1.y)]
    );
    assert_eq!(shown.rendered_lines, 0..2);
    assert_eq!(shown.visible_rows, 0..WRAPPED_LINE_ROWS + 1);
    let current = store.views().get(view).unwrap().clone();
    assert_eq!(
        current
            .display
            .as_ref()
            .and_then(|display| display.wrap_settings())
            .map(|settings| (settings.wrap_column as usize, settings.tab_size)),
        Some((columns, TAB_SIZE))
    );
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
    show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    assert_eq!(
        store.views().get(view).unwrap().scroll,
        ScrollPosition::default()
    );
}

#[test]
fn wrap_클릭과_드래그는_표시_줄의_열을_고르고_줄_끝_너머의_클릭은_윗_줄_끝에_캐럿을_둔다() {
    let text = "x".repeat(WRAPPED_LINE);
    let (mut store, view) = fixture(&text, false);
    let context = Context::default();
    let (columns, gutter) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let [clicked, row_end, second, drag_from, drag_to] = [
        WRAP_CLICK_COLUMN,
        columns,
        1,
        WRAP_DRAG_FROM_COLUMN,
        WRAP_DRAG_TO_COLUMN,
    ]
    .map(|chars| caret_offset(&context, &text[..columns], chars));
    let surface = editor();
    let viewport = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new()).viewport;
    let left = viewport.left() + gutter;
    let row_top = |row: usize| viewport.top() + row as f32 * LINE_HEIGHT;
    let row_middle = |row: usize| row_top(row) + LINE_HEIGHT / CENTER_DIVISOR;
    let shown = click_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        pos2(left + clicked + WRAP_CLICK_PAST_BOUNDARY, row_middle(1)),
    );
    let inside = columns + WRAP_CLICK_COLUMN;
    assert_eq!(selection(&store, view), (inside, inside));
    assert_caret(&shown, left + clicked, row_top(1));
    let shown = click_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        pos2(viewport.right() - WRAP_CLICK_PAST_BOUNDARY, row_middle(0)),
    );
    let state = store.views().get(view).unwrap();
    let revision = store.documents().snapshot(state.document).unwrap().revision;
    assert_eq!(selection(&store, view), (columns, columns));
    assert!(state.head_at_row_end(0, revision));
    assert_caret(&shown, left + row_end, row_top(0));
    for (pressed, head, x) in [
        (Key::ArrowRight, columns + 1, left + second),
        (Key::ArrowLeft, columns, left),
    ] {
        let shown = show_wrapped(
            &context,
            &surface,
            &mut store,
            view,
            SCREEN,
            vec![key(pressed, false)],
        );
        assert_eq!(selection(&store, view), (head, head));
        assert_caret(&shown, x, row_top(1));
    }
    let from = pos2(left + drag_from + WRAP_CLICK_PAST_BOUNDARY, row_middle(0));
    let to = pos2(left + drag_to + WRAP_CLICK_PAST_BOUNDARY, row_middle(2));
    for events in [
        vec![Event::PointerMoved(from), press(from, true)],
        vec![press(from, false)],
        vec![press(from, true)],
        vec![Event::PointerMoved(to)],
    ] {
        show_wrapped(&context, &surface, &mut store, view, SCREEN, events);
    }
    let shown = show_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        SCREEN,
        vec![press(to, false)],
    );
    assert_eq!(
        selection(&store, view),
        (WRAP_DRAG_FROM_COLUMN, columns * 2 + WRAP_DRAG_TO_COLUMN)
    );
    assert_eq!(
        filled(&shown.shapes, Color32::BLUE),
        [
            Rect::from_min_max(
                pos2(left + drag_from, row_top(0)),
                pos2(left + row_end, row_top(1))
            ),
            Rect::from_min_max(pos2(left, row_top(1)), pos2(left + row_end, row_top(2))),
            Rect::from_min_max(pos2(left, row_top(2)), pos2(left + drag_to, row_top(3))),
        ]
    );
}

#[test]
fn wrap_end_home_page와_세로_이동은_표시_줄을_기준으로_하고_스크롤은_표시_줄_수를_따른다() {
    let (mut store, view) = fixture(&wrapped_lines(), false);
    let context = Context::default();
    let (columns, gutter) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let surface = editor();
    let viewport = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new()).viewport;
    let page = ((viewport.height() / LINE_HEIGHT).floor() - PAGE_OVERLAP_LINES).max(1.0) as usize;
    let row_start = |row: usize| {
        row / WRAPPED_LINE_ROWS * (WRAPPED_LINE + 1) + row % WRAPPED_LINE_ROWS * columns
    };
    for (event, expected) in [
        (key(Key::End, false), (columns, columns)),
        (key(Key::ArrowDown, false), (columns * 2, columns * 2)),
        (key(Key::End, false), (WRAPPED_LINE, WRAPPED_LINE)),
        (key(Key::Home, false), (columns * 2, columns * 2)),
        (key(Key::Home, false), (0, 0)),
        (chord(Key::End, Modifiers::SHIFT), (0, columns)),
        (key(Key::ArrowLeft, false), (0, 0)),
        (
            key(Key::PageDown, false),
            (row_start(page), row_start(page)),
        ),
        (
            key(Key::PageDown, false),
            (row_start(page * 2), row_start(page * 2)),
        ),
        (key(Key::PageUp, false), (row_start(page), row_start(page))),
    ] {
        show_wrapped(&context, &surface, &mut store, view, SCREEN, vec![event]);
        assert_eq!(selection(&store, view), expected);
    }
    let followed = (page * 2 + 1) as f32 * LINE_HEIGHT - viewport.height();
    assert_eq!(store.views().get(view).unwrap().scroll.y, followed);
    let shown = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    assert_caret(
        &shown,
        viewport.left() + gutter,
        viewport.top() + page as f32 * LINE_HEIGHT - followed,
    );
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            current.selection,
            ScrollPosition {
                x: 0.0,
                y: FAR_SCROLL,
            },
            current.folds,
        )
        .unwrap();
    show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    assert_eq!(
        store.views().get(view).unwrap().scroll.y,
        (WRAPPED_LINES * WRAPPED_LINE_ROWS) as f32 * LINE_HEIGHT - viewport.height()
    );
}

#[test]
fn wrap_상태의_reveal과_ime는_캐럿이_놓인_표시_줄의_좌표를_쓴다() {
    let (mut store, view) = fixture(&wrapped_lines(), false);
    let context = Context::default();
    let (columns, gutter) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let caret_x = caret_offset(&context, &"x".repeat(columns), WRAP_CLICK_COLUMN);
    let surface = editor();
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
            surface
                .reveal_presented(
                    ui,
                    &mut store,
                    view,
                    REVEAL_WRAPPED_LINE as f64,
                    (columns + WRAP_CLICK_COLUMN + 1) as f64,
                    &wrapping(),
                )
                .unwrap();
            surface
                .show_presented(
                    ui,
                    &mut store,
                    view,
                    true,
                    |_, _, _| false,
                    |_| None,
                    &wrapping(),
                )
                .unwrap();
        },
    );
    output.textures_delta.clear();
    let row = (REVEAL_WRAPPED_LINE - 1) * WRAPPED_LINE_ROWS + 1;
    let head = (REVEAL_WRAPPED_LINE - 1) * (WRAPPED_LINE + 1) + columns + WRAP_CLICK_COLUMN;
    let centered =
        (row as f32 + 1.0 / CENTER_DIVISOR) * LINE_HEIGHT - viewport.height() / CENTER_DIVISOR;
    assert_eq!(selection(&store, view), (head, head));
    assert_eq!(
        store.views().get(view).unwrap().scroll,
        ScrollPosition {
            x: 0.0,
            y: centered
        }
    );
    let shown = show_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        SCREEN,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "한".into(),
            active_range_chars: None,
        })],
    );
    let row_top = viewport.top() + row as f32 * LINE_HEIGHT - centered;
    let cursor = shown.ime_cursor.expect("focused editor reports its caret");
    assert_eq!(cursor.left(), viewport.left() + gutter + caret_x);
    assert_eq!(
        cursor.top() - row_top,
        row_top + LINE_HEIGHT - cursor.bottom()
    );
    assert!(
        texts(&shown.shapes, shown.viewport, true)
            .iter()
            .any(|(text, position, _)| text == "한" && *position == cursor.min)
    );
}

#[test]
fn wrap_표시_줄은_편집_undo_폭과_글꼴_변경_뒤에_다시_나뉘고_wrap을_끄면_한_줄로_돌아간다() {
    let text = "x".repeat(WRAPPED_LINE);
    let (mut store, view) = fixture(&text, false);
    let context = Context::default();
    let mut large = editor();
    assert!(update_editor_font_size(
        &mut large.appearance,
        WIDE_FONT_SIZE
    ));
    let (columns, _) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let (narrow_columns, _) =
        wrap_columns(&context, FontId::monospace(FONT_SIZE), NARROW_SCREEN[0]);
    let (large_columns, _) = wrap_columns(&context, large.appearance.font.clone(), SCREEN[0]);
    let surface = editor();
    let split = |text: &str, columns: usize| {
        text.as_bytes()
            .chunks(columns)
            .map(|row| String::from_utf8(row.to_vec()).unwrap())
            .collect::<Vec<_>>()
    };
    show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    let shown = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    assert_eq!(body_rows(&shown), split(&text, columns));
    let shown = show_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        SCREEN,
        vec![Event::Text("y".into())],
    );
    assert_eq!(body_rows(&shown), split(&format!("y{text}"), columns));
    let shown = show_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        SCREEN,
        vec![key(Key::Z, true)],
    );
    assert_eq!(body_rows(&shown), split(&text, columns));
    let shown = show_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        NARROW_SCREEN,
        Vec::new(),
    );
    assert!(narrow_columns < columns);
    assert_eq!(body_rows(&shown), split(&text, narrow_columns));
    let shown = show_wrapped(&context, &large, &mut store, view, SCREEN, Vec::new());
    assert!(large_columns < columns && large_columns != narrow_columns);
    assert_eq!(body_rows(&shown), split(&text, large_columns));
    let unwrapped = present(&context, &surface, &mut store, view, Vec::new());
    assert_eq!(
        texts(&unwrapped.shapes, unwrapped.viewport, true)
            .into_iter()
            .map(|(text, _, _)| text)
            .collect::<Vec<_>>(),
        [text]
    );
}

#[test]
fn wrap_이어지는_줄은_들여쓰기를_채우고_들여쓰기와_탭_안의_클릭은_가까운_위치로_간다() {
    let line = format!("\tstart {}", "word ".repeat(INDENTED_WORDS));
    let (mut store, view) = fixture(&line, false);
    let context = Context::default();
    let (columns, gutter) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let breaks = create_line_breaks(
        &WrapSettings {
            wrap_column: u32::try_from(columns).unwrap(),
            tab_size: TAB_SIZE,
            full_width_columns: FULL_WIDTH_COLUMNS,
            wrapping_indent: WrappingIndent::Same,
        },
        &line,
    )
    .unwrap();
    let indent = " ".repeat(TAB_SIZE as usize);
    let indented = format!("{indent}w");
    let [first_space, last_space, indent_end] =
        [1, 3, TAB_SIZE as usize].map(|chars| caret_offset(&context, &indented, chars));
    let surface = editor();
    show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    let shown = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    let mut start = 0;
    let expected: Vec<String> = breaks
        .break_offsets
        .iter()
        .map(|end| {
            let row = format!(
                "{}{}",
                if start == 0 { "" } else { indent.as_str() },
                line[start..*end].replace('\t', &indent)
            );
            start = *end;
            row
        })
        .collect();
    assert_eq!(breaks.wrapped_text_indent_length, TAB_SIZE);
    assert!(expected.len() >= WRAPPED_LINE_ROWS);
    assert_eq!(body_rows(&shown), expected);
    let left = shown.viewport.left() + gutter;
    let row_top = |row: usize| shown.viewport.top() + row as f32 * LINE_HEIGHT;
    let row_middle = |row: usize| row_top(row) + LINE_HEIGHT / CENTER_DIVISOR;
    let continued = breaks.break_offsets[0];
    for (position, head, caret) in [
        (
            pos2(left + first_space + WRAP_CLICK_PAST_BOUNDARY, row_middle(1)),
            continued,
            (left + indent_end, row_top(1)),
        ),
        (
            pos2(left + first_space + WRAP_CLICK_PAST_BOUNDARY, row_middle(0)),
            0,
            (left, row_top(0)),
        ),
        (
            pos2(left + last_space + WRAP_CLICK_PAST_BOUNDARY, row_middle(0)),
            1,
            (left + indent_end, row_top(0)),
        ),
    ] {
        let shown = click_wrapped(&context, &surface, &mut store, view, position);
        assert_eq!(selection(&store, view), (head, head));
        assert_caret(&shown, caret.0, caret.1);
    }
}

#[test]
fn wrap_현재_줄_강조는_캐럿이_놓인_문서_줄의_모든_표시_줄을_칠한다() {
    let (mut store, view) = fixture(&format!("{}\ntail", "x".repeat(WRAPPED_LINE)), false);
    let context = Context::default();
    let (columns, gutter) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let surface = editor();
    let viewport = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new()).viewport;
    let left = viewport.left() + gutter;
    let row_top = |row: usize| viewport.top() + row as f32 * LINE_HEIGHT;
    let row_span = |row: usize| (row_top(row), row_top(row) + LINE_HEIGHT);
    let row_start = |row: usize| {
        pos2(
            left + WRAP_CLICK_PAST_BOUNDARY,
            row_top(row) + LINE_HEIGHT / CENTER_DIVISOR,
        )
    };
    let highlighted = |shown: &Wrapped| {
        filled(&shown.shapes, Color32::DARK_GRAY)
            .iter()
            .map(|band| (band.top(), band.bottom()))
            .collect::<Vec<_>>()
    };
    let shown = click_wrapped(&context, &surface, &mut store, view, row_start(1));
    assert_eq!(selection(&store, view), (columns, columns));
    assert_caret(&shown, left, row_top(1));
    assert_eq!(
        highlighted(&shown),
        (0..WRAPPED_LINE_ROWS).map(&row_span).collect::<Vec<_>>()
    );
    let shown = click_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        row_start(WRAPPED_LINE_ROWS),
    );
    assert_eq!(highlighted(&shown), [row_span(WRAPPED_LINE_ROWS)]);
}

#[test]
fn wrap_설정이_바뀌어도_맨_위에_보이던_문서_위치는_맨_위에_남는다() {
    let (mut store, view) = fixture(&wrapped_lines(), false);
    let context = Context::default();
    let mut large = editor();
    assert!(update_editor_font_size(
        &mut large.appearance,
        WIDE_FONT_SIZE
    ));
    assert_ne!(large.appearance.line_height, LINE_HEIGHT);
    let (columns, _) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let (narrow_columns, _) =
        wrap_columns(&context, FontId::monospace(FONT_SIZE), NARROW_SCREEN[0]);
    let (large_columns, _) = wrap_columns(&context, large.appearance.font.clone(), SCREEN[0]);
    let surface = editor();
    let scroll_top = |store: &EditorStore| store.views().get(view).unwrap().scroll.y;
    let first_row = |columns: usize| STABLE_LINE * WRAPPED_LINE.div_ceil(columns);
    let top = |row: usize, line_height: f32| row as f32 * line_height + STABLE_DELTA;
    present(&context, &surface, &mut store, view, Vec::new());
    scroll_to(&mut store, view, top(STABLE_LINE, LINE_HEIGHT));
    present(&context, &surface, &mut store, view, Vec::new());
    assert_eq!(scroll_top(&store), top(STABLE_LINE, LINE_HEIGHT));
    let shown = show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    assert_eq!(scroll_top(&store), top(first_row(columns), LINE_HEIGHT));
    assert_eq!(shown.visible_rows.start, first_row(columns));
    assert_eq!(shown.rendered_lines.start, STABLE_LINE);
    scroll_to(&mut store, view, top(first_row(columns) + 1, LINE_HEIGHT));
    show_wrapped(&context, &surface, &mut store, view, SCREEN, Vec::new());
    let shown = show_wrapped(
        &context,
        &surface,
        &mut store,
        view,
        NARROW_SCREEN,
        Vec::new(),
    );
    let narrow_row = columns / narrow_columns;
    assert!(narrow_row > 0);
    assert_eq!(
        scroll_top(&store),
        top(first_row(narrow_columns) + narrow_row, LINE_HEIGHT)
    );
    assert_eq!(shown.rendered_lines.start, STABLE_LINE);
    let shown = show_wrapped(&context, &large, &mut store, view, SCREEN, Vec::new());
    let large_row = narrow_row * narrow_columns / large_columns;
    assert_eq!(
        scroll_top(&store),
        top(
            first_row(large_columns) + large_row,
            large.appearance.line_height
        )
    );
    assert_eq!(shown.rendered_lines.start, STABLE_LINE);
    present(&context, &surface, &mut store, view, Vec::new());
    assert_eq!(scroll_top(&store), top(STABLE_LINE, LINE_HEIGHT));
}
