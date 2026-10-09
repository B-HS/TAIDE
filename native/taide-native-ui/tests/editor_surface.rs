use egui::epaint::{ClippedShape, ColorMode, PathShape, Shape};
use egui::os::OperatingSystem;
use egui::text::{CCursor, LayoutJob};
use egui::{
    Color32, Context, Event, FontDefinitions, FontFamily, FontId, ImeEvent, Key, Modifiers,
    MouseWheelUnit, OutputCommand, PointerButton, Pos2, RawInput, Rect, Response, Stroke,
    TouchPhase, Ui, Vec2, pos2, vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_model::settings::Settings;
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, LaneMark, Stickiness, Underline,
    UnderlineKind,
};
use taide_native_editor::document::{Edit, EditorError, UndoGroup};
use taide_native_editor::folding::FoldCommand;
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::language_configuration::{
    AutoClosingPair, BracketPair, CharacterPairs, EnterAction, FoldMarker, IndentAction,
    IndentMetadata, Language, LanguageRules, UntokenizedLines,
};
use taide_native_editor::line_breaks::{WrapSettings, WrappingIndent, create_line_breaks};
use taide_native_editor::line_tokens::{LineTokens, TokenStyle, TokenStyleTable};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::css_motion::ease;
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_native_ui::editor_overlay::{
    OverlayBounds, OverlayPlacement, OverlayPreference, place_overlay,
};
use taide_native_ui::editor_row_text::{
    RowColumns, RowFontStyle, RowInlineStyle, RowSection, RowText, RowTokens,
};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorDisplayOptions, EditorPresentation, EditorRequest, EditorTokens,
    FoldControl, NativeEditor,
};
use taide_native_ui::presentation::{editor_folding, editor_presentation, update_editor_font_size};

const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 16;
const BYTE_LIMIT: usize = 2 * 1024 * 1024;
const ROWS: usize = 50_000;
const SCROLL_LINE: usize = 40_000;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const PADDING: f32 = 8.0;
const LINE_NUMBERS_MIN_CHARS: usize = 3;
const LINE_DECORATIONS_WIDTH: f32 = 10.0;
const FOLDING_CONTROLS_WIDTH: f32 = 16.0;
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
const TOKEN_DEFAULT: [u8; 4] = [200, 200, 200, 255];
const TOKEN_KEYWORD: [u8; 4] = [86, 156, 214, 255];
const TOKEN_STRING: [u8; 4] = [206, 145, 120, 255];
const TOKEN_COMMENT: [u8; 4] = [106, 153, 85, 255];
const DEFAULT_STYLE_ID: u32 = 0;
const KEYWORD_STYLE_ID: u32 = 1;
const STRING_STYLE_ID: u32 = 2;
const COMMENT_STYLE_ID: u32 = 3;
const DOC_COMMENT_STYLE_ID: u32 = 4;
const TOKEN_DOCUMENT: &str = "let s = \"값\";\n// done\ntail\n";
const TOKEN_DOCUMENT_ROWS: usize = 4;
const KEYWORD_END: usize = 3;
const DOC_COMMENT_START: usize = 2;
const TEXT_DECORATION_STROKE: f32 = 1.0;
const CARET_STROKE: f32 = 1.0;
const BOLD_FAMILY: &str = "synthetic-editor-bold";
const MISSING_FAMILY: &str = "synthetic-missing";
const TOKEN_CROSSING_BYTES: usize = 7;
const MIXED_LINE_PREFIX: &str = "\t값😀 = \"한글 𐐀\" ";
const TOKEN_ROW_SOURCE: &str = "\t값x";
const TOKEN_ROW_SPLIT: usize = 2;
const CONTINUED_ROW_START_BYTE: usize = 10;
const OVERLAY_VIEWPORT: Rect = Rect {
    min: pos2(100.0, 50.0),
    max: pos2(500.0, 350.0),
};
const OVERLAY_EDITOR: Rect = Rect {
    min: pos2(300.0, 100.0),
    max: pos2(700.0, 590.0),
};
const OVERLAY_WINDOW: Rect = Rect {
    min: pos2(0.0, 0.0),
    max: pos2(1000.0, 600.0),
};
const OVERLAY_SIZE: Vec2 = vec2(120.0, 80.0);
const OVERLAY_TALL_SIZE: Vec2 = vec2(120.0, 200.0);
const OVERLAY_WIDE_SIZE: Vec2 = vec2(450.0, 80.0);
const OVERLAY_ANCHOR_LEFT: f32 = 200.0;
const OVERLAY_ANCHOR_HEIGHT: f32 = 20.0;
const OVERLAY_SHORTFALL: f32 = 1.0;
const OVERLAY_PAGE_VERTICAL_PADDING: f32 = 22.0;
const OVERLAY_PAGE_HORIZONTAL_PADDING: f32 = 15.0;
const DECORATED_DOCUMENT: &str = "\tab = c;\n값 x\nend";
const DECORATED_FIRST_ROW: &str = "    ab = c;";
const DECORATED_ROWS: usize = 3;
const DECORATION_ADDED: [u8; 4] = [46, 160, 67, 255];
const DECORATION_DELETED: [u8; 4] = [248, 81, 73, 255];
const DECORATION_CONFLICT: [u8; 4] = [64, 200, 174, 51];
const DECORATION_FIND: [u8; 4] = [234, 92, 0, 85];
const DECORATION_SELECTION_MATCH: [u8; 4] = [173, 214, 255, 38];
const DECORATION_WARNING: [u8; 4] = [204, 167, 0, 255];
const DECORATION_BRACKET: [u8; 4] = [255, 215, 0, 255];
const DECORATION_LINK: [u8; 4] = [79, 193, 255, 255];
const GUTTER_Z_ORDER: u8 = 0;
const CONFLICT_Z_ORDER: u8 = 1;
const FIND_Z_ORDER: u8 = 2;
const DIAGNOSTICS_Z_ORDER: u8 = 3;
const LANE_BAR_WIDTH: f32 = 3.0;
const DELETED_TRIANGLE_WIDTH: f32 = 6.0;
const DELETED_TRIANGLE_HALF_HEIGHT: f32 = 4.0;
const SQUIGGLE_PERIOD: f32 = 6.0;
const SQUIGGLE_HEIGHT: f32 = 3.0;
const SQUIGGLE_TROUGH_OFFSET: f32 = 1.75;
const SQUIGGLE_STROKE: f32 = 1.0;
const SQUIGGLE_TOLERANCE: f32 = 0.001;
const WRAPPED_DECORATION_TAIL: usize = 5;
const WRAPPED_DECORATION_ROWS: usize = 2;
const MANY_LINES: usize = 1200;
const MANY_LINES_DIGITS: usize = 4;
const UNTRACKED_REVISION: u64 = u64::MAX;
const PLAIN_FRAME: &[&str] = &[
    "rect 0.00,0.00..800.00,200.00 fill 000000ff clip 0.00,0.00..800.00,200.00",
    "rect 0.00,0.00..800.00,20.00 fill 606060ff clip 0.00,0.00..800.00,200.00",
    "line 35.00,0.00 to 35.00,20.00 stroke 1.00 ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"fn main() {\" at 35.00,2.00 size 92.72,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"1\" at 16.56,2.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"    let 값 = \\\"漢字\\\";\" at 35.00,22.00 size 143.28,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"2\" at 16.56,22.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"\" at 35.00,42.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"3\" at 16.56,42.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"    // e\\u{301} 𐐀\" at 35.00,62.00 size 84.28,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"4\" at 16.56,62.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"}\" at 35.00,82.00 size 8.44,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"5\" at 16.56,82.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"\" at 35.00,102.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"6\" at 16.56,102.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "ime 0.00,0.00..800.00,200.00 cursor 35.00,2.00..35.00,18.00",
    "scroll 0.00,0.00 selection 0..0 preedit None",
];
const SELECTION_FRAME: &[&str] = &[
    "rect 0.00,0.00..800.00,200.00 fill 000000ff clip 0.00,0.00..800.00,200.00",
    "text \"fn main() {\" at 35.00,2.00 size 92.72,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"1\" at 16.56,2.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 77.00,20.00..800.00,40.00 fill 0000ffff clip 35.00,0.00..800.00,200.00",
    "text \"    let 값 = \\\"漢字\\\";\" at 35.00,22.00 size 143.28,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"2\" at 16.56,22.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 35.00,40.00..800.00,60.00 fill 0000ffff clip 35.00,0.00..800.00,200.00",
    "text \"\" at 35.00,42.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"3\" at 16.56,42.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 0.00,60.00..800.00,80.00 fill 606060ff clip 0.00,0.00..800.00,200.00",
    "rect 35.00,60.00..119.28,80.00 fill 0000ffff clip 35.00,0.00..800.00,200.00",
    "text \"    // e\\u{301} 𐐀\" at 35.00,62.00 size 84.28,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"4\" at 16.56,62.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"}\" at 35.00,82.00 size 8.44,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"5\" at 16.56,82.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"\" at 35.00,102.00 size 0.00,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"6\" at 16.56,102.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "ime 0.00,0.00..800.00,200.00 cursor 119.28,62.00..119.28,78.00",
    "scroll 0.00,0.00 selection 14..50 preedit None",
];
const REVEALED_VIEW: &str = "scroll 78.88,70.00 selection 148..148 preedit None";
const SCROLLED_COMPOSITION_TRACE: &[&str] = &[
    "scroll 24.00,130.00 selection 0..0 preedit None",
    "scroll 24.00,130.00 selection 70..70 preedit None",
    "scroll 78.88,130.00 selection 148..148 preedit None",
    "rect 0.00,0.00..800.00,200.00 fill 000000ff clip 0.00,0.00..800.00,200.00",
    "text \"row 6\" at -43.88,-8.00 size 42.16,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"7\" at 16.56,-8.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 7\" at -43.88,12.00 size 42.16,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"8\" at 16.56,12.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 0.00,30.00..800.00,50.00 fill 606060ff clip 0.00,0.00..800.00,200.00",
    "line 799.00,30.00 to 799.00,50.00 stroke 1.00 ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\" at -43.88,32.00 size 842.88,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"9\" at 16.56,32.00 size 8.44,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 9\" at -43.88,52.00 size 42.16,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"10\" at 8.16,52.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 10\" at -43.88,72.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"11\" at 8.16,72.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 11\" at -43.88,92.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"12\" at 8.16,92.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 12\" at -43.88,112.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"13\" at 8.16,112.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 13\" at -43.88,132.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"14\" at 8.16,132.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 14\" at -43.88,152.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"15\" at 8.16,152.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 15\" at -43.88,172.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"16\" at 8.16,172.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "text \"row 16\" at -43.88,192.00 size 50.56,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "text \"17\" at 8.16,192.00 size 16.84,16.00 color a0a0a0ff job simple a0a0a0ff clip 0.00,0.00..800.00,200.00",
    "rect 799.00,32.00..815.84,48.00 fill 000000ff clip 35.00,0.00..800.00,200.00",
    "text \"한글\" at 799.00,32.00 size 16.84,16.00 color ffffffff job simple ffffffff clip 35.00,0.00..800.00,200.00",
    "line 799.00,48.00 to 815.84,48.00 stroke 1.00 ffffffff clip 35.00,0.00..800.00,200.00",
    "rect 786.00,22.00..800.00,55.00 fill 35353555 clip 0.00,0.00..800.00,200.00",
    "rect 102.00,188.00..747.00,200.00 fill 35353555 clip 0.00,0.00..800.00,200.00",
    "ime 0.00,0.00..800.00,200.00 cursor 799.00,32.00..799.00,48.00",
    "scroll 78.88,130.00 selection 148..148 preedit Some(\"한글\")",
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

fn line_numbers_width(context: &Context, font: FontId, digits: usize) -> f32 {
    let digit = measure_with(context, &"0".repeat(ADVANCE_SAMPLE), font) / ADVANCE_SAMPLE as f32;
    (digits as f32 * digit).round()
}

fn gutter_width(context: &Context, font: FontId) -> f32 {
    line_numbers_width(context, font, LINE_NUMBERS_MIN_CHARS) + LINE_DECORATIONS_WIDTH
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
    let gutter = gutter_width(&context, FontId::monospace(FONT_SIZE));
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
fn 다중_커서의_ime_확정은_모든_선택을_교체하고_escape는_주커서만_남긴다() {
    let (mut store, view) = fixture("ab\ncd", false);
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![
                    Selection { anchor: 0, head: 1 },
                    Selection { anchor: 3, head: 4 },
                ],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    let context = Context::default();
    frame(&context, &mut store, view, Vec::new(), true);
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "한글".into(),
            active_range_chars: None,
        })],
        false,
    );
    frame(
        &context,
        &mut store,
        view,
        vec![Event::Ime(ImeEvent::Commit("한글".into()))],
        false,
    );
    assert_eq!(text(&store, view), "한글b\n한글d");
    assert_eq!(
        store.views().get(view).unwrap().selection.selections.len(),
        2
    );
    frame(
        &context,
        &mut store,
        view,
        vec![key(Key::Escape, false)],
        false,
    );
    assert_eq!(
        store.views().get(view).unwrap().selection.selections.len(),
        1
    );
}

#[test]
fn 선택_앵커는_원본의_색과_폭으로_표시된다() {
    use taide_native_editor::cursor_commands::{CursorCommand, run_cursor_command};
    use taide_native_editor::line_commands::LineCommandContext;
    let (mut store, view) = fixture("abc", false);
    run_cursor_command(
        &mut store,
        view,
        CursorCommand::SetAnchor,
        LineCommandContext {
            indent: IndentOptions {
                tab_size: TAB_SIZE,
                insert_spaces: true,
            },
            language: None,
            syntax: &UntokenizedLines,
            compare: None,
            transforms: None,
            word_rules: None,
        },
    )
    .unwrap();
    let shown = show_wrapped(
        &Context::default(),
        &editor(),
        &mut store,
        view,
        SCREEN,
        Vec::new(),
    );
    let anchors = filled(&shown.shapes, Color32::from_rgb(0, 122, 204));
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].width(), 2.0);
    assert_eq!(anchors[0].height(), LINE_HEIGHT);
}

#[test]
fn 프레임_밖에서_추가한_아래_커서도_원본처럼_스크롤로_드러낸다() {
    use taide_native_editor::cursor_commands::{CursorCommand, run_cursor_command};
    use taide_native_editor::line_commands::LineCommandContext;
    const CURSOR_LINE: usize = 19;
    let source = "a\n".repeat(SHORT_ROWS);
    let (mut store, view) = fixture(&source, false);
    let context = Context::default();
    show_wrapped(&context, &editor(), &mut store, view, SCREEN, Vec::new());
    place_caret(&mut store, view, CURSOR_LINE * "a\n".len());
    run_cursor_command(
        &mut store,
        view,
        CursorCommand::AddBelow,
        LineCommandContext {
            indent: IndentOptions {
                tab_size: TAB_SIZE,
                insert_spaces: true,
            },
            language: None,
            syntax: &UntokenizedLines,
            compare: None,
            transforms: None,
            word_rules: None,
        },
    )
    .unwrap();
    let shown = show_wrapped(&context, &editor(), &mut store, view, SCREEN, Vec::new());
    assert!(shown.visible_rows.contains(&(CURSOR_LINE + 1)));
}

#[test]
fn 다중_커서의_preedit는_각_캐럿에_그려지고_붙여넣기와_삭제도_같은_선택을_사용한다() {
    let (mut store, view) = fixture("a\nb", false);
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![
                    Selection { anchor: 1, head: 1 },
                    Selection { anchor: 3, head: 3 },
                ],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    let context = Context::default();
    let shown = show_wrapped(
        &context,
        &editor(),
        &mut store,
        view,
        SCREEN,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "한".into(),
            active_range_chars: None,
        })],
    );
    assert_eq!(
        shown
            .shapes
            .iter()
            .filter(
                |shape| matches!(&shape.shape, Shape::Text(text) if text.galley.job.text == "한")
            )
            .count(),
        2
    );
    frame(
        &context,
        &mut store,
        view,
        vec![
            Event::Ime(ImeEvent::Preedit {
                text: String::new(),
                active_range_chars: None,
            }),
            Event::Paste("X\nY".into()),
        ],
        false,
    );
    assert_eq!(text(&store, view), "aX\nbY");
    frame(
        &context,
        &mut store,
        view,
        vec![key(Key::Backspace, false)],
        false,
    );
    assert_eq!(text(&store, view), "a\nb");
    assert_eq!(
        store.views().get(view).unwrap().selection.selections.len(),
        2
    );
}

const POINTER_CLICK_STEP: f64 = 0.05;
const POINTER_RELEASE_STEP: f64 = 0.01;

fn pointer_frame(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    time: f64,
    modifiers: Modifiers,
    events: Vec<Event>,
) -> EditorGeometry {
    let mut geometry = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(SCREEN[0], SCREEN[1]))),
            time: Some(time),
            events: std::iter::once(Event::ModifiersChanged(modifiers))
                .chain(events)
                .collect(),
            ..Default::default()
        },
        |ui| {
            let shown = editor().show(ui, store, view, true).unwrap();
            assert!(shown.errors.is_empty());
            geometry = Some(shown.geometry);
        },
    );
    output.textures_delta.clear();
    geometry.unwrap()
}

fn pointer_click(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    time: f64,
    position: Pos2,
    modifiers: Modifiers,
) {
    pointer_frame(
        context,
        store,
        view,
        time,
        modifiers,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers,
            },
        ],
    );
    pointer_frame(
        context,
        store,
        view,
        time + POINTER_RELEASE_STEP,
        modifiers,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers,
        }],
    );
}

fn pointer_ranges(store: &EditorStore, view: ViewId) -> Vec<(usize, usize)> {
    store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections
        .iter()
        .map(|s| (s.anchor, s.head))
        .collect()
}

#[test]
fn 포인터_더블_트리플_클릭과_단어_단위_드래그는_선택_단위를_유지한다() {
    let (mut store, view) = fixture("foo bar baz\nother", false);
    let context = Context::default();
    let shown = pointer_frame(&context, &mut store, view, 0.0, Modifiers::NONE, vec![]);
    let from = shown.caret_rect(1).unwrap().center();
    let to = shown.caret_rect(6).unwrap().center();
    pointer_click(
        &context,
        &mut store,
        view,
        POINTER_CLICK_STEP,
        from,
        Modifiers::NONE,
    );
    pointer_frame(
        &context,
        &mut store,
        view,
        POINTER_CLICK_STEP * 2.0,
        Modifiers::NONE,
        vec![press(from, true)],
    );
    assert_eq!(pointer_ranges(&store, view), [(0, 3)]);
    pointer_frame(
        &context,
        &mut store,
        view,
        POINTER_CLICK_STEP * 2.0 + POINTER_RELEASE_STEP,
        Modifiers::NONE,
        vec![Event::PointerMoved(to)],
    );
    assert_eq!(pointer_ranges(&store, view), [(0, 7)]);
    pointer_frame(
        &context,
        &mut store,
        view,
        POINTER_CLICK_STEP * 2.0 + POINTER_RELEASE_STEP * 2.0,
        Modifiers::NONE,
        vec![press(to, false)],
    );
    pointer_click(&context, &mut store, view, 1.0, from, Modifiers::NONE);
    pointer_click(
        &context,
        &mut store,
        view,
        1.0 + POINTER_CLICK_STEP,
        from,
        Modifiers::NONE,
    );
    pointer_click(
        &context,
        &mut store,
        view,
        1.0 + POINTER_CLICK_STEP * 2.0,
        from,
        Modifiers::NONE,
    );
    assert_eq!(pointer_ranges(&store, view), [(0, 12)]);
}

#[test]
fn 포인터_alt_클릭은_커서를_추가하고_같은_선택을_클릭하면_제거한다() {
    let (mut store, view) = fixture("abcd\nefgh", false);
    let context = Context::default();
    let shown = pointer_frame(&context, &mut store, view, 0.0, Modifiers::NONE, vec![]);
    let position = shown.caret_rect(6).unwrap().center();
    let modifiers = Modifiers {
        alt: true,
        ..Modifiers::NONE
    };
    pointer_click(&context, &mut store, view, 1.0, position, modifiers);
    assert_eq!(pointer_ranges(&store, view), [(0, 0), (6, 6)]);
    pointer_click(&context, &mut store, view, 2.0, position, modifiers);
    assert_eq!(pointer_ranges(&store, view), [(0, 0)]);
}

#[test]
fn 포인터_shift_alt_드래그는_주선택_시작부터_각_표시줄의_컬럼을_선택한다() {
    let (mut store, view) = fixture("abcd\nefgh\nijkl", false);
    let context = Context::default();
    let shown = pointer_frame(&context, &mut store, view, 0.0, Modifiers::NONE, vec![]);
    let from = shown.caret_rect(1).unwrap().center();
    let to = shown.caret_rect(13).unwrap().center();
    pointer_click(&context, &mut store, view, 1.0, from, Modifiers::NONE);
    let modifiers = Modifiers {
        alt: true,
        shift: true,
        ..Modifiers::NONE
    };
    pointer_frame(
        &context,
        &mut store,
        view,
        2.0,
        modifiers,
        vec![Event::PointerButton {
            pos: from,
            button: PointerButton::Primary,
            pressed: true,
            modifiers,
        }],
    );
    pointer_frame(
        &context,
        &mut store,
        view,
        2.0 + POINTER_CLICK_STEP,
        modifiers,
        vec![Event::PointerMoved(to)],
    );
    assert_eq!(pointer_ranges(&store, view), [(1, 3), (6, 8), (11, 13)]);
    pointer_frame(
        &context,
        &mut store,
        view,
        2.0 + POINTER_CLICK_STEP + POINTER_RELEASE_STEP,
        modifiers,
        vec![Event::PointerButton {
            pos: to,
            button: PointerButton::Primary,
            pressed: false,
            modifiers,
        }],
    );
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
    for (index, events) in [
        vec![
            Event::PointerMoved(SELECTION_PRESS),
            press(SELECTION_PRESS, true),
        ],
        vec![press(SELECTION_PRESS, false)],
        vec![press(SELECTION_PRESS, true)],
        vec![Event::PointerMoved(SELECTION_RELEASE)],
    ]
    .into_iter()
    .enumerate()
    {
        pointer_frame(
            &context,
            &mut store,
            view,
            index as f64 + 1.0,
            Modifiers::NONE,
            events,
        );
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
    let gutter = gutter_width(&context, FontId::monospace(FONT_SIZE));
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
    let gutter = gutter_width(&context, font.clone());
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
    show_wrapped_at(context, surface, store, view, screen, events, None)
}

fn show_wrapped_at(
    context: &Context,
    surface: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    screen: [f32; 2],
    events: Vec<Event>,
    time: Option<f64>,
) -> Wrapped {
    let mut shown = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(screen[0], screen[1]),
            )),
            events,
            time,
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
    let gutter = gutter_width(context, font);
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
    for (index, events) in [
        vec![Event::PointerMoved(from), press(from, true)],
        vec![press(from, false)],
        vec![press(from, true)],
        vec![Event::PointerMoved(to)],
    ]
    .into_iter()
    .enumerate()
    {
        show_wrapped_at(
            &context,
            &surface,
            &mut store,
            view,
            SCREEN,
            events,
            Some(index as f64 + 1.0),
        );
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

fn token_style(foreground: [u8; 4]) -> TokenStyle {
    TokenStyle {
        foreground,
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: TokenKind::Other,
    }
}

fn token_styles() -> TokenStyleTable {
    let comment = TokenStyle {
        is_struck_through: true,
        kind: TokenKind::Comment,
        ..token_style(TOKEN_COMMENT)
    };
    TokenStyleTable::new(
        token_style(TOKEN_DEFAULT),
        vec![
            token_style(TOKEN_DEFAULT),
            TokenStyle {
                is_bold: true,
                ..token_style(TOKEN_KEYWORD)
            },
            TokenStyle {
                is_italic: true,
                is_underlined: true,
                kind: TokenKind::String,
                ..token_style(TOKEN_STRING)
            },
            comment,
            comment,
        ],
    )
}

fn token_color(foreground: [u8; 4]) -> Color32 {
    let [red, green, blue, alpha] = foreground;
    Color32::from_rgba_unmultiplied(red, green, blue, alpha)
}

fn decoration(foreground: [u8; 4]) -> Stroke {
    Stroke::new(TEXT_DECORATION_STROKE, token_color(foreground))
}

fn spans(tokens: &[(usize, u32)]) -> Vec<u32> {
    tokens
        .iter()
        .flat_map(|(start, style_id)| [u32::try_from(*start).unwrap(), *style_id])
        .collect()
}

fn line_tokens(lines: &[Vec<u32>]) -> LineTokens {
    let mut tokens = LineTokens::new(lines.len());
    for (line, spans) in lines.iter().enumerate() {
        if !spans.is_empty() {
            tokens.set_line(line, spans.clone(), false);
        }
    }
    tokens
}

fn revision(store: &EditorStore, view: ViewId) -> u64 {
    store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap()
        .revision
}

fn bold_context(family: &FontFamily, faces: FontFamily) -> Context {
    let context = Context::default();
    let mut definitions = FontDefinitions::default();
    let chain = definitions.families[&faces].clone();
    definitions.families.insert(family.clone(), chain);
    context.set_fonts(definitions);
    context
}

fn bold_presentation(bold_family: Option<FontFamily>) -> EditorPresentation {
    EditorPresentation {
        options: EditorDisplayOptions {
            bold_family,
            ..Default::default()
        },
    }
}

struct Highlight<'a> {
    surface: NativeEditor,
    presentation: EditorPresentation,
    tokens: Option<EditorTokens<'a>>,
}

fn show_highlighted(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    highlight: &Highlight<'_>,
    events: Vec<Event>,
) -> Wrapped {
    let mut shown = None;
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
            let viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let output = highlight
                .surface
                .show_tokenized(
                    ui,
                    store,
                    view,
                    true,
                    |_, _, _| false,
                    |_| None,
                    &highlight.presentation,
                    |_| highlight.tokens,
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

#[derive(Debug, Clone, PartialEq)]
struct Run {
    text: String,
    color: Color32,
    family: FontFamily,
    is_italic: bool,
    underline: Stroke,
    strikethrough: Stroke,
}

fn run(text: &str, foreground: [u8; 4]) -> Run {
    Run {
        text: text.into(),
        color: token_color(foreground),
        family: FontFamily::Monospace,
        is_italic: false,
        underline: Stroke::NONE,
        strikethrough: Stroke::NONE,
    }
}

fn body_runs(shown: &Wrapped) -> Vec<Vec<Run>> {
    shown
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Text(text) if clipped.clip_rect.left() > shown.viewport.left() => {
                let job = &text.galley.job;
                Some(
                    job.sections
                        .iter()
                        .map(|section| Run {
                            text: job.text[section.byte_range.start.0..section.byte_range.end.0]
                                .to_owned(),
                            color: section.format.color,
                            family: section.format.font_id.family.clone(),
                            is_italic: section.format.italics,
                            underline: section.format.underline,
                            strikethrough: section.format.strikethrough,
                        })
                        .collect(),
                )
            }
            _ => None,
        })
        .collect()
}

fn token_document_lines() -> LineTokens {
    let string = TOKEN_DOCUMENT.find('"').unwrap();
    let string_end = TOKEN_DOCUMENT.rfind('"').unwrap() + 1;
    line_tokens(&[
        spans(&[
            (0, KEYWORD_STYLE_ID),
            (KEYWORD_END, DEFAULT_STYLE_ID),
            (string, STRING_STYLE_ID),
            (string_end, DEFAULT_STYLE_ID),
        ]),
        spans(&[
            (0, COMMENT_STYLE_ID),
            (DOC_COMMENT_START, DOC_COMMENT_STYLE_ID),
        ]),
        Vec::new(),
        Vec::new(),
    ])
}

fn token_document_runs(keyword_family: FontFamily) -> Vec<Vec<Run>> {
    vec![
        vec![
            Run {
                family: keyword_family,
                ..run("let", TOKEN_KEYWORD)
            },
            run(" s = ", TOKEN_DEFAULT),
            Run {
                is_italic: true,
                underline: decoration(TOKEN_STRING),
                ..run("\"값\"", TOKEN_STRING)
            },
            run(";", TOKEN_DEFAULT),
        ],
        vec![Run {
            strikethrough: decoration(TOKEN_COMMENT),
            ..run("// done", TOKEN_COMMENT)
        }],
        vec![run("tail", TOKEN_DEFAULT)],
        vec![run("", TOKEN_DEFAULT)],
    ]
}

#[test]
fn 토큰_구간의_색과_글꼴_스타일은_스타일_표와_같고_토큰_없는_줄은_표의_기본_스타일이다() {
    let (mut store, view) = fixture(TOKEN_DOCUMENT, false);
    let styles = token_styles();
    let lines = token_document_lines();
    let bold_family = FontFamily::Name(BOLD_FAMILY.into());
    let highlight = |bold_family: Option<FontFamily>| Highlight {
        surface: editor(),
        presentation: bold_presentation(bold_family),
        tokens: Some(EditorTokens {
            revision: revision(&store, view),
            lines: &lines,
            styles: &styles,
        }),
    };
    let unregistered = [
        None,
        Some(FontFamily::Name(MISSING_FAMILY.into())),
        Some(bold_family.clone()),
    ]
    .map(highlight);
    let registered = highlight(Some(bold_family.clone()));
    let context = Context::default();
    show_highlighted(&context, &mut store, view, &unregistered[0], Vec::new());
    for highlight in &unregistered {
        let shown = show_highlighted(&context, &mut store, view, highlight, Vec::new());
        assert_eq!(shown.rendered_lines, 0..TOKEN_DOCUMENT_ROWS);
        assert_eq!(
            body_runs(&shown),
            token_document_runs(FontFamily::Monospace)
        );
    }
    let context = bold_context(&bold_family, FontFamily::Monospace);
    show_highlighted(&context, &mut store, view, &registered, Vec::new());
    let shown = show_highlighted(&context, &mut store, view, &registered, Vec::new());
    assert_eq!(body_runs(&shown), token_document_runs(bold_family));
}

fn placed(shown: &Wrapped) -> Vec<String> {
    let mut lines: Vec<String> = shown
        .shapes
        .iter()
        .map(|clipped| match &clipped.shape {
            Shape::Text(text) => format!(
                "text {:?} at {} size {}",
                text.galley.job.text,
                point(text.pos),
                point(text.galley.size().to_pos2())
            ),
            _ => painted(clipped),
        })
        .collect();
    lines.push(format!("ime {:?}", shown.ime_cursor.map(bounds)));
    lines
}

#[test]
fn 토큰은_본문_글자의_구간만_바꾸고_선택_캐럿_현재_줄_ime_좌표는_평문_화면과_같다() {
    let styles = token_styles();
    let lines = token_document_lines();
    let frames = |tokens: Option<EditorTokens<'_>>| {
        let (mut store, view) = fixture(TOKEN_DOCUMENT, false);
        let context = Context::default();
        let highlight = Highlight {
            surface: editor(),
            presentation: EditorPresentation::default(),
            tokens,
        };
        [
            Vec::new(),
            vec![
                Event::PointerMoved(SELECTION_PRESS),
                press(SELECTION_PRESS, true),
            ],
            vec![Event::PointerMoved(SELECTION_RELEASE)],
            vec![press(SELECTION_RELEASE, false)],
            vec![Event::Ime(ImeEvent::Preedit {
                text: "한".into(),
                active_range_chars: None,
            })],
        ]
        .map(|events| show_highlighted(&context, &mut store, view, &highlight, events))
    };
    let plain = frames(None);
    let highlighted = frames(Some(EditorTokens {
        revision: 0,
        lines: &lines,
        styles: &styles,
    }));
    let [.., selected, composing] = &plain;
    assert!(!filled(&selected.shapes, Color32::BLUE).is_empty());
    assert!(!filled(&selected.shapes, Color32::DARK_GRAY).is_empty());
    assert!(composing.ime_cursor.is_some());
    for (plain, highlighted) in plain.iter().zip(&highlighted) {
        assert_eq!(
            body_runs(highlighted)[..TOKEN_DOCUMENT_ROWS],
            token_document_runs(FontFamily::Monospace)
        );
        assert_eq!(placed(highlighted), placed(plain));
    }
}

fn expected_runs(
    line: &str,
    tokens: &[(usize, u32)],
    row: std::ops::Range<usize>,
    indent: &str,
) -> Vec<(String, Color32)> {
    let styles = token_styles();
    let color = |style_id: u32| token_color(styles.style(style_id).foreground);
    let indented = (row.start > 0).then(|| (indent.to_owned(), color(DEFAULT_STYLE_ID)));
    let pieces = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, (start, style_id))| {
            let end = tokens.get(index + 1).map_or(line.len(), |(next, _)| *next);
            let piece = (*start).max(row.start)..end.min(row.end);
            (piece.start < piece.end).then(|| (line[piece].replace('\t', indent), color(*style_id)))
        });
    let mut runs: Vec<(String, Color32)> = Vec::new();
    for (text, color) in indented.into_iter().chain(pieces) {
        match runs.last_mut() {
            Some((merged, last)) if *last == color => merged.push_str(&text),
            _ => runs.push((text, color)),
        }
    }
    runs
}

fn colored_rows(shown: &Wrapped) -> Vec<Vec<(String, Color32)>> {
    body_runs(shown)
        .into_iter()
        .map(|row| row.into_iter().map(|run| (run.text, run.color)).collect())
        .collect()
}

#[test]
fn 탭_wrap_cjk_emoji가_섞인_줄에서_토큰_경계는_문서_바이트에_맞는_표시_문자에_놓인다() {
    let line = format!("{MIXED_LINE_PREFIX}{}", "word ".repeat(INDENTED_WORDS));
    let (mut store, view) = fixture(&line, false);
    let context = Context::default();
    let styles = token_styles();
    let wrapped = Highlight {
        surface: editor(),
        presentation: wrapping(),
        tokens: None,
    };
    show_highlighted(&context, &mut store, view, &wrapped, Vec::new());
    let breaks = create_line_breaks(
        store
            .views()
            .get(view)
            .unwrap()
            .display
            .as_ref()
            .and_then(|display| display.wrap_settings())
            .unwrap(),
        &line,
    )
    .unwrap();
    let crossing = breaks.break_offsets[0];
    let tokens = [
        (0, KEYWORD_STYLE_ID),
        (line.find(" = ").unwrap(), DEFAULT_STYLE_ID),
        (line.find('"').unwrap(), STRING_STYLE_ID),
        (line.rfind('"').unwrap() + 1, DEFAULT_STYLE_ID),
        (crossing - TOKEN_CROSSING_BYTES, COMMENT_STYLE_ID),
        (crossing + TOKEN_CROSSING_BYTES, DEFAULT_STYLE_ID),
    ];
    let lines = line_tokens(&[spans(&tokens)]);
    let tokenized = Some(EditorTokens {
        revision: revision(&store, view),
        lines: &lines,
        styles: &styles,
    });
    let indent = " ".repeat(TAB_SIZE as usize);
    assert!(breaks.break_offsets.len() >= WRAPPED_LINE_ROWS);
    assert_eq!(breaks.wrapped_text_indent_length, TAB_SIZE);
    let mut start = 0;
    let expected: Vec<_> = breaks
        .break_offsets
        .iter()
        .map(|end| {
            let row = expected_runs(&line, &tokens, start..*end, &indent);
            start = *end;
            row
        })
        .collect();
    assert_eq!(
        expected[0][..3],
        [
            (format!("{indent}값😀"), token_color(TOKEN_KEYWORD)),
            (" = ".to_owned(), token_color(TOKEN_DEFAULT)),
            ("\"한글 𐐀\"".to_owned(), token_color(TOKEN_STRING)),
        ]
    );
    assert_eq!(
        expected[1][..2],
        [
            (indent.clone(), token_color(TOKEN_DEFAULT)),
            (
                line[crossing..crossing + TOKEN_CROSSING_BYTES].to_owned(),
                token_color(TOKEN_COMMENT)
            ),
        ]
    );
    let shown = show_highlighted(
        &context,
        &mut store,
        view,
        &Highlight {
            tokens: tokenized,
            ..wrapped
        },
        Vec::new(),
    );
    assert_eq!(colored_rows(&shown), expected);
    let shown = show_highlighted(
        &context,
        &mut store,
        view,
        &Highlight {
            surface: editor(),
            presentation: EditorPresentation::default(),
            tokens: tokenized,
        },
        Vec::new(),
    );
    assert_eq!(
        colored_rows(&shown),
        [expected_runs(&line, &tokens, 0..line.len(), &indent)]
    );
}

#[test]
fn 토큰이_없거나_현재_문서와_맞지_않으면_기존_평문_화면과_같다() {
    let styles = token_styles();
    let keyword_lines = |count: usize| line_tokens(&vec![spans(&[(0, KEYWORD_STYLE_ID)]); count]);
    let matching = keyword_lines(PLAIN_ROWS);
    let shorter = keyword_lines(PLAIN_ROWS - 1);
    let frame = |tokens: Option<EditorTokens<'_>>| {
        let (mut store, view) = fixture(PLAIN_DOCUMENT, false);
        let context = Context::default();
        draw(&context, &mut store, view, Vec::new());
        let highlight = Highlight {
            surface: editor(),
            presentation: EditorPresentation::default(),
            tokens,
        };
        show_highlighted(&context, &mut store, view, &highlight, Vec::new())
            .shapes
            .iter()
            .map(painted)
            .collect::<Vec<_>>()
    };
    let plain = &PLAIN_FRAME[..PLAIN_FRAME.len() - FRAME_STATE_LINES];
    for tokens in [
        None,
        Some(EditorTokens {
            revision: 1,
            lines: &matching,
            styles: &styles,
        }),
        Some(EditorTokens {
            revision: 0,
            lines: &shorter,
            styles: &styles,
        }),
    ] {
        assert_frame(&frame(tokens), plain);
    }
    let highlighted = frame(Some(EditorTokens {
        revision: 0,
        lines: &matching,
        styles: &styles,
    }));
    assert!(highlighted.as_slice() != plain, "{highlighted:#?}");
}

#[test]
fn 토큰은_그_프레임의_입력을_반영한_문서에_대해_요청된다() {
    let (mut store, view) = fixture("ab", false);
    let document = store.views().get(view).unwrap().document;
    let context = Context::default();
    draw(&context, &mut store, view, Vec::new());
    let before = revision(&store, view);
    let mut requested = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: vec![Event::Text("x".into())],
            ..Default::default()
        },
        |ui| {
            editor()
                .show_tokenized(
                    ui,
                    &mut store,
                    view,
                    true,
                    |_, _, _| false,
                    |_| None,
                    &EditorPresentation::default(),
                    |store| {
                        let snapshot = store.documents().snapshot(document).unwrap();
                        requested = Some((snapshot.revision, snapshot.rope.to_string()));
                        None
                    },
                )
                .unwrap();
        },
    );
    output.textures_delta.clear();
    assert_eq!(requested, Some((before + 1, "xab".to_owned())));
}

fn caret_offset_with(context: &Context, text: &str, chars: usize, font: FontId) -> f32 {
    let mut offset = 0.0;
    let mut output = context.run_ui(RawInput::default(), |ui| {
        offset = ui
            .painter()
            .layout_no_wrap(text.into(), font.clone(), Color32::WHITE)
            .pos_from_cursor(CCursor::new(chars))
            .left();
    });
    output.textures_delta.clear();
    offset
}

#[test]
fn reveal은_토큰의_굵은_글꼴로_그린_줄의_끝에_가로_스크롤을_맞춘다() {
    let bold_family = FontFamily::Name(BOLD_FAMILY.into());
    let presentation = bold_presentation(Some(bold_family.clone()));
    let styles = token_styles();
    let lines = line_tokens(
        &(0..SCROLLED_ROWS)
            .map(|row| {
                spans(&[(
                    0,
                    if row == SCROLLED_LONG_ROW {
                        KEYWORD_STYLE_ID
                    } else {
                        DEFAULT_STYLE_ID
                    },
                )])
            })
            .collect::<Vec<_>>(),
    );
    let reveal = |tokens: Option<EditorTokens<'_>>| {
        let (mut store, view) = fixture(&scrolled_document(), false);
        let context = bold_context(&bold_family, FontFamily::Proportional);
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
                    .reveal_tokenized(
                        ui,
                        &mut store,
                        view,
                        (SCROLLED_LONG_ROW + 1) as f64,
                        (SCROLLED_LONG_ROW_COLUMNS + 1) as f64,
                        &presentation,
                        tokens,
                    )
                    .unwrap();
            },
        );
        output.textures_delta.clear();
        (
            context,
            view_line(&store, view),
            store.views().get(view).unwrap().scroll.x,
        )
    };
    let (_, plain, _) = reveal(None);
    assert_eq!(plain, REVEALED_VIEW);
    let (context, _, scrolled) = reveal(Some(EditorTokens {
        revision: 0,
        lines: &lines,
        styles: &styles,
    }));
    let long = "x".repeat(SCROLLED_LONG_ROW_COLUMNS);
    let regular_end = caret_offset(&context, &long, SCROLLED_LONG_ROW_COLUMNS);
    let bold_end = caret_offset_with(
        &context,
        &long,
        SCROLLED_LONG_ROW_COLUMNS,
        FontId::new(FONT_SIZE, bold_family.clone()),
    );
    let text_width = SCREEN[0] - gutter_width(&context, FontId::monospace(FONT_SIZE));
    assert!((bold_end - regular_end).abs() > WIDTH_TOLERANCE);
    assert!(
        (scrolled - (bold_end + CARET_STROKE - text_width).max(0.0)).abs() <= WIDTH_TOLERANCE,
        "{scrolled}"
    );
}

fn row_section(foreground: [u8; 4], chars: std::ops::Range<usize>) -> RowSection {
    RowSection {
        chars,
        foreground: token_color(foreground),
    }
}

#[test]
fn 줄_토큰은_탭과_들여쓰기를_건너_표시_문자_구간이_되고_어긋난_경계는_문자_시작으로_내린다() {
    let styles = token_styles();
    let font = FontId::monospace(FONT_SIZE);
    let bold_family = FontFamily::Name(BOLD_FAMILY.into());
    let highlighted = |mut row: RowText, tokens: &[(usize, u32)], row_start_byte: usize| {
        row.highlight(RowTokens {
            spans: &spans(tokens),
            styles: &styles,
            row_start_byte,
        });
        row
    };
    let plain = RowFontStyle::default();
    let bold = RowFontStyle {
        is_bold: true,
        ..plain
    };
    let slanted = RowFontStyle {
        is_italic: true,
        is_underlined: true,
        ..plain
    };
    let struck = RowFontStyle {
        is_struck_through: true,
        ..plain
    };
    let tab_spaces = TAB_SIZE as usize;
    let value_end = TOKEN_ROW_SOURCE.find('x').unwrap();
    let [aligned, inside_character] = [1, TOKEN_ROW_SPLIT].map(|string_start| {
        highlighted(
            row_text(TOKEN_ROW_SOURCE, TAB_SIZE, 0.0, 0),
            &[
                (0, KEYWORD_STYLE_ID),
                (string_start, STRING_STYLE_ID),
                (value_end, DEFAULT_STYLE_ID),
            ],
            0,
        )
    });
    assert_eq!(
        aligned.sections,
        [
            row_section(TOKEN_KEYWORD, 0..tab_spaces),
            row_section(TOKEN_STRING, tab_spaces..tab_spaces + 1),
            row_section(TOKEN_DEFAULT, tab_spaces + 1..tab_spaces + 2),
        ]
    );
    assert_eq!(aligned.font_styles, [bold, slanted, plain]);
    assert_eq!(inside_character, aligned);
    let formats = |job: LayoutJob| {
        job.sections
            .into_iter()
            .map(|section| {
                (
                    section.format.font_id,
                    section.format.italics,
                    section.format.underline,
                    section.format.strikethrough,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        formats(aligned.styled_layout_job(&font, Some(&bold_family))),
        [
            (
                FontId::new(FONT_SIZE, bold_family),
                false,
                Stroke::NONE,
                Stroke::NONE
            ),
            (font.clone(), true, decoration(TOKEN_STRING), Stroke::NONE),
            (font.clone(), false, Stroke::NONE, Stroke::NONE),
        ]
    );
    assert_eq!(
        aligned.layout_job(&font),
        aligned.styled_layout_job(&font, None)
    );
    assert_eq!(aligned.layout_job(&font).sections[0].format.font_id, font);
    let continued = highlighted(
        row_text("\tb", TAB_SIZE, CONTINUED_START_COLUMN, CONTINUED_INDENT),
        &[
            (0, KEYWORD_STYLE_ID),
            (CONTINUED_ROW_START_BYTE - 1, STRING_STYLE_ID),
            (CONTINUED_ROW_START_BYTE + 1, COMMENT_STYLE_ID),
        ],
        CONTINUED_ROW_START_BYTE,
    );
    let indent = CONTINUED_INDENT as usize;
    let tab_end = continued.text.chars().count() - 1;
    assert_eq!(
        continued.sections,
        [
            row_section(TOKEN_DEFAULT, 0..indent),
            row_section(TOKEN_STRING, indent..tab_end),
            row_section(TOKEN_COMMENT, tab_end..tab_end + 1),
        ]
    );
    assert_eq!(continued.font_styles, [plain, slanted, struck]);
    let short = || row_text("abcd", TAB_SIZE, 0.0, 0);
    let untokenized = highlighted(short(), &[], 0);
    assert_eq!(untokenized.sections, [row_section(TOKEN_DEFAULT, 0..4)]);
    assert_eq!(untokenized.font_styles, [plain]);
    let late = highlighted(short(), &[(TOKEN_ROW_SPLIT, KEYWORD_STYLE_ID)], 0);
    assert_eq!(
        late.sections,
        [
            row_section(TOKEN_DEFAULT, 0..TOKEN_ROW_SPLIT),
            row_section(TOKEN_KEYWORD, TOKEN_ROW_SPLIT..4),
        ]
    );
    assert_eq!(late.font_styles, [plain, bold]);
    let merged = highlighted(
        short(),
        &[
            (0, COMMENT_STYLE_ID),
            (TOKEN_ROW_SPLIT, DOC_COMMENT_STYLE_ID),
        ],
        0,
    );
    assert_eq!(merged.sections, [row_section(TOKEN_COMMENT, 0..4)]);
    assert_eq!(merged.font_styles, [struck]);
    let beyond = highlighted(
        short(),
        &[
            (0, KEYWORD_STYLE_ID),
            (4, STRING_STYLE_ID),
            (9, COMMENT_STYLE_ID),
        ],
        0,
    );
    assert_eq!(beyond.sections, [row_section(TOKEN_KEYWORD, 0..4)]);
    let empty = highlighted(row_text("", TAB_SIZE, 0.0, 0), &[(0, KEYWORD_STYLE_ID)], 0);
    assert_eq!(empty.sections, [row_section(TOKEN_DEFAULT, 0..0)]);
    assert_eq!(
        empty.layout_job(&font),
        LayoutJob::simple(
            String::new(),
            font,
            token_color(TOKEN_DEFAULT),
            f32::INFINITY
        )
    );
}

fn overlay_anchor(left: f32, top: f32) -> Rect {
    Rect::from_min_size(pos2(left, top), vec2(0.0, OVERLAY_ANCHOR_HEIGHT))
}

fn overlay_at(left: f32, top: f32, preference: OverlayPreference) -> Option<OverlayPlacement> {
    Some(OverlayPlacement {
        position: pos2(left, top),
        preference,
    })
}

#[test]
fn 오버레이는_선호_순서에서_처음_맞는_쪽에_놓이고_어느_쪽도_맞지_않으면_첫_선호에_놓인다() {
    let viewport = OverlayBounds::Viewport(OVERLAY_VIEWPORT);
    let above_first = [OverlayPreference::Above, OverlayPreference::Below];
    let below_first = [OverlayPreference::Below, OverlayPreference::Above];
    let placed = |top: f32, size: Vec2, preferences: &[OverlayPreference]| {
        place_overlay(
            overlay_anchor(OVERLAY_ANCHOR_LEFT, top),
            size,
            preferences,
            viewport,
        )
    };
    let above = |top: f32, size: Vec2| {
        overlay_at(OVERLAY_ANCHOR_LEFT, top - size.y, OverlayPreference::Above)
    };
    let below = |top: f32| {
        overlay_at(
            OVERLAY_ANCHOR_LEFT,
            top + OVERLAY_ANCHOR_HEIGHT,
            OverlayPreference::Below,
        )
    };
    let middle = OVERLAY_VIEWPORT.center().y;
    assert_eq!(
        placed(middle, OVERLAY_SIZE, &above_first),
        above(middle, OVERLAY_SIZE)
    );
    assert_eq!(placed(middle, OVERLAY_SIZE, &below_first), below(middle));
    let barely_above = OVERLAY_VIEWPORT.top() + OVERLAY_SIZE.y;
    let near_top = barely_above - OVERLAY_SHORTFALL;
    assert_eq!(
        placed(barely_above, OVERLAY_SIZE, &above_first),
        above(barely_above, OVERLAY_SIZE)
    );
    assert_eq!(
        placed(near_top, OVERLAY_SIZE, &above_first),
        below(near_top)
    );
    let barely_below = OVERLAY_VIEWPORT.bottom() - OVERLAY_SIZE.y - OVERLAY_ANCHOR_HEIGHT;
    let near_bottom = barely_below + OVERLAY_SHORTFALL;
    assert_eq!(
        placed(barely_below, OVERLAY_SIZE, &below_first),
        below(barely_below)
    );
    assert_eq!(
        placed(near_bottom, OVERLAY_SIZE, &below_first),
        above(near_bottom, OVERLAY_SIZE)
    );
    assert!(OVERLAY_TALL_SIZE.y > OVERLAY_VIEWPORT.height() / CENTER_DIVISOR);
    assert_eq!(
        placed(middle, OVERLAY_TALL_SIZE, &above_first),
        above(middle, OVERLAY_TALL_SIZE)
    );
    assert_eq!(
        placed(middle, OVERLAY_TALL_SIZE, &below_first),
        below(middle)
    );
    assert_eq!(
        placed(near_top, OVERLAY_SIZE, &[OverlayPreference::Above]),
        above(near_top, OVERLAY_SIZE)
    );
    assert_eq!(
        placed(near_bottom, OVERLAY_SIZE, &[OverlayPreference::Below]),
        below(near_bottom)
    );
    assert_eq!(placed(middle, OVERLAY_SIZE, &[]), None);
}

#[test]
fn 오버레이의_가로_위치는_경계_안으로_당기고_exact_선호는_앵커_위치를_그대로_쓴다() {
    let viewport = OverlayBounds::Viewport(OVERLAY_VIEWPORT);
    let top = OVERLAY_VIEWPORT.center().y;
    let left_of = |anchor_left: f32, size: Vec2, preference: OverlayPreference| {
        place_overlay(
            overlay_anchor(anchor_left, top),
            size,
            &[preference],
            viewport,
        )
        .map(|placement| placement.position.x)
    };
    let inside = OVERLAY_VIEWPORT.right() - OVERLAY_SIZE.x;
    let past_right = inside + OVERLAY_SHORTFALL;
    let past_left = OVERLAY_VIEWPORT.left() - OVERLAY_SHORTFALL;
    for preference in [OverlayPreference::Above, OverlayPreference::Below] {
        assert_eq!(left_of(inside, OVERLAY_SIZE, preference), Some(inside));
        assert_eq!(left_of(past_right, OVERLAY_SIZE, preference), Some(inside));
        assert_eq!(
            left_of(past_left, OVERLAY_SIZE, preference),
            Some(OVERLAY_VIEWPORT.left())
        );
        assert!(OVERLAY_WIDE_SIZE.x > OVERLAY_VIEWPORT.width());
        assert_eq!(
            left_of(OVERLAY_ANCHOR_LEFT, OVERLAY_WIDE_SIZE, preference),
            Some(OVERLAY_VIEWPORT.left())
        );
    }
    for anchor_left in [past_left, past_right] {
        assert_eq!(
            place_overlay(
                overlay_anchor(anchor_left, top),
                OVERLAY_WIDE_SIZE,
                &[OverlayPreference::Exact, OverlayPreference::Below],
                viewport,
            ),
            overlay_at(anchor_left, top, OverlayPreference::Exact)
        );
    }
}

#[test]
fn 편집기_밖으로_넘칠_수_있는_오버레이는_창의_여백을_기준으로_뒤집고_당긴다() {
    let page = OverlayBounds::Page {
        editor: OVERLAY_EDITOR,
        window: OVERLAY_WINDOW,
    };
    let above_first = [OverlayPreference::Above, OverlayPreference::Below];
    let below_first = [OverlayPreference::Below, OverlayPreference::Above];
    let placed = |left: f32, top: f32, preferences: &[OverlayPreference], bounds| {
        place_overlay(overlay_anchor(left, top), OVERLAY_SIZE, preferences, bounds)
    };
    let left = OVERLAY_EDITOR.center().x;
    let barely_above = OVERLAY_WINDOW.top() + OVERLAY_PAGE_VERTICAL_PADDING + OVERLAY_SIZE.y;
    assert!(barely_above - OVERLAY_SIZE.y < OVERLAY_EDITOR.top());
    assert_eq!(
        placed(left, barely_above, &above_first, page),
        overlay_at(
            left,
            barely_above - OVERLAY_SIZE.y,
            OverlayPreference::Above
        )
    );
    let near_top = barely_above - OVERLAY_SHORTFALL;
    assert_eq!(
        placed(left, near_top, &above_first, page),
        overlay_at(
            left,
            near_top + OVERLAY_ANCHOR_HEIGHT,
            OverlayPreference::Below
        )
    );
    let barely_below = OVERLAY_WINDOW.bottom()
        - OVERLAY_PAGE_VERTICAL_PADDING
        - OVERLAY_SIZE.y
        - OVERLAY_ANCHOR_HEIGHT;
    assert_eq!(
        placed(left, barely_below, &below_first, page),
        overlay_at(
            left,
            barely_below + OVERLAY_ANCHOR_HEIGHT,
            OverlayPreference::Below
        )
    );
    let near_bottom = barely_below + OVERLAY_SHORTFALL;
    assert_eq!(
        placed(left, near_bottom, &below_first, page),
        overlay_at(left, near_bottom - OVERLAY_SIZE.y, OverlayPreference::Above)
    );
    let top = OVERLAY_EDITOR.center().y;
    let x = |left: f32, bounds| {
        placed(left, top, &below_first, bounds).map(|placement| placement.position.x)
    };
    let overflowing = OVERLAY_EDITOR.right() - OVERLAY_SHORTFALL;
    assert_eq!(x(overflowing, page), Some(overflowing));
    assert_eq!(
        x(OVERLAY_EDITOR.right() + OVERLAY_SIZE.x, page),
        Some(OVERLAY_EDITOR.right())
    );
    assert_eq!(
        x(
            OVERLAY_EDITOR.left() - OVERLAY_SIZE.x - OVERLAY_SHORTFALL,
            page
        ),
        Some(OVERLAY_EDITOR.left() - OVERLAY_SIZE.x)
    );
    let narrow = OverlayBounds::Page {
        editor: OVERLAY_EDITOR,
        window: Rect::from_min_max(
            OVERLAY_WINDOW.min,
            pos2(OVERLAY_EDITOR.right(), OVERLAY_WINDOW.bottom()),
        ),
    };
    assert_eq!(
        x(overflowing, narrow),
        Some(OVERLAY_EDITOR.right() - OVERLAY_PAGE_HORIZONTAL_PADDING - OVERLAY_SIZE.x)
    );
    let flush = OverlayBounds::Page {
        editor: Rect::from_min_max(OVERLAY_WINDOW.min, OVERLAY_EDITOR.max),
        window: OVERLAY_WINDOW,
    };
    assert_eq!(
        x(OVERLAY_WINDOW.left(), flush),
        Some(OVERLAY_WINDOW.left() + OVERLAY_PAGE_HORIZONTAL_PADDING)
    );
}

fn layer(
    revision: u64,
    z_order: u8,
    stickiness: Stickiness,
    items: Vec<(std::ops::Range<usize>, DecorationKind)>,
) -> DecorationLayer {
    DecorationLayer::new(
        revision,
        z_order,
        items
            .into_iter()
            .map(|(bytes, kind)| Decoration {
                bytes,
                kind,
                stickiness,
            })
            .collect(),
    )
}

fn lane(mark: LaneMark, color: [u8; 4]) -> DecorationKind {
    DecorationKind::Lane { mark, color }
}

fn range_background(color: [u8; 4]) -> DecorationKind {
    DecorationKind::Inline(InlineStyle {
        background: Some(color),
        ..InlineStyle::default()
    })
}

fn squiggle(color: [u8; 4]) -> DecorationKind {
    DecorationKind::Inline(InlineStyle {
        underline: Some(Underline {
            kind: UnderlineKind::Squiggly,
            color,
        }),
        ..InlineStyle::default()
    })
}

fn show_decorated(
    context: &Context,
    surface: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    layers: &[&DecorationLayer],
    events: Vec<Event>,
) -> (Wrapped, EditorGeometry) {
    let mut shown = None;
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
            let viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let output = surface
                .show_request(
                    ui,
                    store,
                    view,
                    EditorRequest {
                        request_focus: true,
                        keymap: |_: &Ui, _: &Event, _: bool| false,
                        route: |_: &Response| None,
                        presentation,
                        tokens: |_: &EditorStore| None,
                        language: None,
                        decorations: layers,
                        fold_commands: &[],
                        fold_controls: None,
                    },
                )
                .unwrap();
            assert!(output.errors.is_empty());
            shown = Some((viewport, output.rendered_lines, output.geometry));
        },
    );
    output.textures_delta.clear();
    let (viewport, rendered_lines, geometry) = shown.unwrap();
    (
        Wrapped {
            shapes: output.shapes,
            viewport,
            rendered_lines,
            visible_rows: geometry.visible_rows.clone(),
            ime_cursor: output.platform_output.ime.map(|ime| ime.cursor_rect),
        },
        geometry,
    )
}

fn stroke_color(path: &PathShape) -> Color32 {
    match &path.stroke.color {
        ColorMode::Solid(color) => *color,
        ColorMode::UV(_) => Color32::PLACEHOLDER,
    }
}

fn outline(clipped: &ClippedShape) -> String {
    match &clipped.shape {
        Shape::Rect(rect) => format!("rect {}", hex(rect.fill)),
        Shape::LineSegment { stroke, .. } => format!("line {}", hex(stroke.color)),
        Shape::Text(text) => format!("text {:?}", text.galley.job.text),
        Shape::Path(path) if path.closed => format!("polygon {}", hex(path.fill)),
        Shape::Path(path) => format!("stroke {}", hex(stroke_color(path))),
        _ => "other".into(),
    }
}

fn paths(shapes: &[ClippedShape], is_closed: bool) -> Vec<(&PathShape, Rect)> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Path(path) if path.closed == is_closed => Some((path, clipped.clip_rect)),
            _ => None,
        })
        .collect()
}

fn fill(color: [u8; 4]) -> String {
    hex(token_color(color))
}

fn assert_squiggle(wave: &PathShape, clip: Rect, band: Rect, color: [u8; 4]) {
    assert_eq!(clip, band);
    assert_eq!(band.height(), SQUIGGLE_HEIGHT);
    assert_eq!(wave.fill, Color32::TRANSPARENT);
    assert_eq!(
        (wave.stroke.width, stroke_color(wave)),
        (SQUIGGLE_STROKE, token_color(color))
    );
    let points = &wave.points;
    assert!(points[0].x <= band.left());
    assert!(points[points.len() - 1].x >= band.right());
    let trough = points
        .iter()
        .find(|point| point.y == band.bottom())
        .expect("squiggle reaches the bottom of its band");
    assert!((trough.x - band.left() - SQUIGGLE_TROUGH_OFFSET).abs() < SQUIGGLE_TOLERANCE);
    for pair in points.windows(2) {
        let half_period = pair[1].x - pair[0].x;
        assert!((half_period - SQUIGGLE_PERIOD / CENTER_DIVISOR).abs() < SQUIGGLE_TOLERANCE);
        let mut heights = [pair[0].y, pair[1].y];
        heights.sort_by(f32::total_cmp);
        assert_eq!(heights, [band.top(), band.bottom()]);
    }
}

#[test]
fn 장식_층은_선택과_본문_사이에_그리고_gutter의_lane_표식은_줄_번호_앞에_그린다() {
    let (mut store, view) = fixture(DECORATED_DOCUMENT, false);
    let context = Context::default();
    let surface = editor();
    let presentation = EditorPresentation::default();
    let marked = DECORATED_DOCUMENT.find("ab").unwrap();
    let second = DECORATED_DOCUMENT.find('값').unwrap();
    let third = DECORATED_DOCUMENT.find("end").unwrap();
    let current = revision(&store, view);
    let sticky = Stickiness::default();
    let lanes = layer(
        current,
        GUTTER_Z_ORDER,
        sticky,
        vec![
            (0..second, lane(LaneMark::Bar, DECORATION_ADDED)),
            (
                third..third,
                lane(LaneMark::DeletedTriangle, DECORATION_DELETED),
            ),
        ],
    );
    let conflict = layer(
        current,
        CONFLICT_Z_ORDER,
        sticky,
        vec![(
            second..second,
            DecorationKind::LineBackground(DECORATION_CONFLICT),
        )],
    );
    let find = layer(
        current,
        FIND_Z_ORDER,
        sticky,
        vec![(marked..marked + 2, range_background(DECORATION_FIND))],
    );
    let diagnostics = layer(
        current,
        DIAGNOSTICS_Z_ORDER,
        sticky,
        vec![
            (0..marked + 2, squiggle(DECORATION_WARNING)),
            (
                second..second + '값'.len_utf8(),
                DecorationKind::Inline(InlineStyle {
                    foreground: Some(DECORATION_BRACKET),
                    underline: Some(Underline {
                        kind: UnderlineKind::Straight,
                        color: DECORATION_LINK,
                    }),
                    ..InlineStyle::default()
                }),
            ),
        ],
    );
    let layers = [&diagnostics, &lanes, &find, &conflict];
    let mut draw = |layers: &[&DecorationLayer]| {
        show_decorated(
            &context,
            &surface,
            &mut store,
            view,
            &presentation,
            layers,
            Vec::new(),
        )
        .0
    };
    draw(&layers);
    let shown = draw(&layers);
    assert_eq!(shown.rendered_lines, 0..DECORATED_ROWS);
    assert_eq!(
        shown.shapes.iter().map(outline).collect::<Vec<_>>(),
        [
            format!("rect {}", hex(Color32::BLACK)),
            format!("rect {}", hex(Color32::DARK_GRAY)),
            format!("rect {}", fill(DECORATION_FIND)),
            format!("stroke {}", fill(DECORATION_WARNING)),
            format!("line {}", hex(Color32::WHITE)),
            format!("text {DECORATED_FIRST_ROW:?}"),
            format!("rect {}", fill(DECORATION_ADDED)),
            format!("text {:?}", "1"),
            format!("rect {}", fill(DECORATION_CONFLICT)),
            format!("text {:?}", "값 x"),
            format!("rect {}", fill(DECORATION_ADDED)),
            format!("text {:?}", "2"),
            format!("text {:?}", "end"),
            format!("polygon {}", fill(DECORATION_DELETED)),
            format!("text {:?}", "3"),
        ]
    );
    let font = FontId::monospace(FONT_SIZE);
    let lane_left =
        shown.viewport.left() + line_numbers_width(&context, font, LINE_NUMBERS_MIN_CHARS);
    let left = lane_left + LINE_DECORATIONS_WIDTH;
    let row = |index: usize| shown.viewport.top() + index as f32 * LINE_HEIGHT;
    let x = |chars: usize| left + caret_offset(&context, DECORATED_FIRST_ROW, chars);
    let tab_chars = TAB_SIZE as usize;
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_FIND)),
        [Rect::from_min_max(
            pos2(x(tab_chars), row(0)),
            pos2(x(tab_chars + 2), row(1))
        )]
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_CONFLICT)),
        [Rect::from_min_max(
            pos2(left, row(1)),
            pos2(shown.viewport.right(), row(2))
        )]
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_ADDED)),
        [0, 1].map(|index| Rect::from_min_size(
            pos2(lane_left, row(index)),
            vec2(LANE_BAR_WIDTH, LINE_HEIGHT)
        ))
    );
    let waves = paths(&shown.shapes, false);
    assert_eq!(waves.len(), 1);
    assert_squiggle(
        waves[0].0,
        waves[0].1,
        Rect::from_min_max(
            pos2(x(0), row(1) - SQUIGGLE_HEIGHT),
            pos2(x(tab_chars + 2), row(1)),
        ),
        DECORATION_WARNING,
    );
    let polygons = paths(&shown.shapes, true);
    assert_eq!(polygons.len(), 1);
    let (triangle, clip) = polygons[0];
    assert_eq!(clip, shown.viewport);
    assert_eq!(triangle.fill, token_color(DECORATION_DELETED));
    assert_eq!(
        triangle.points,
        [
            pos2(lane_left, row(2) - DELETED_TRIANGLE_HALF_HEIGHT),
            pos2(lane_left + DELETED_TRIANGLE_WIDTH, row(2)),
            pos2(lane_left, row(2) + DELETED_TRIANGLE_HALF_HEIGHT),
        ]
    );
    let plain = |text: &str| Run {
        color: Color32::WHITE,
        ..run(text, DECORATION_BRACKET)
    };
    assert_eq!(
        body_runs(&shown),
        [
            vec![plain(DECORATED_FIRST_ROW)],
            vec![
                Run {
                    underline: decoration(DECORATION_LINK),
                    ..run("값", DECORATION_BRACKET)
                },
                plain(" x"),
            ],
            vec![plain("end")],
        ]
    );
    let undecorated = draw(&[]);
    for body in [true, false] {
        assert_eq!(
            texts(&shown.shapes, shown.viewport, body),
            texts(&undecorated.shapes, undecorated.viewport, body)
        );
    }
}

#[test]
fn 겹치거나_맞닿은_같은_모양의_범위_장식은_합치고_같은_lane_표식은_줄마다_한_번_그린다() {
    let content = "abcdefghij";
    let (mut store, view) = fixture(content, false);
    let context = Context::default();
    let surface = editor();
    let presentation = EditorPresentation::default();
    let current = revision(&store, view);
    let sticky = Stickiness::default();
    let find = layer(
        current,
        FIND_Z_ORDER,
        sticky,
        vec![
            (1..3, range_background(DECORATION_FIND)),
            (2..5, range_background(DECORATION_FIND)),
            (5..6, range_background(DECORATION_FIND)),
            (6..7, range_background(DECORATION_SELECTION_MATCH)),
            (8..9, range_background(DECORATION_FIND)),
            (1..2, squiggle(DECORATION_WARNING)),
            (2..4, squiggle(DECORATION_WARNING)),
        ],
    );
    let lanes = [(); 2].map(|()| {
        layer(
            current,
            GUTTER_Z_ORDER,
            sticky,
            vec![(0..0, lane(LaneMark::Bar, DECORATION_ADDED))],
        )
    });
    let layers = [&find, &lanes[0], &lanes[1]];
    let mut draw = || {
        show_decorated(
            &context,
            &surface,
            &mut store,
            view,
            &presentation,
            &layers,
            Vec::new(),
        )
        .0
    };
    draw();
    let shown = draw();
    let left = shown.viewport.left() + gutter_width(&context, FontId::monospace(FONT_SIZE));
    let top = shown.viewport.top();
    let span = |chars: std::ops::Range<usize>| {
        Rect::from_min_max(
            pos2(left + caret_offset(&context, content, chars.start), top),
            pos2(
                left + caret_offset(&context, content, chars.end),
                top + LINE_HEIGHT,
            ),
        )
    };
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_FIND)),
        [span(1..6), span(8..9)]
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_SELECTION_MATCH)),
        [span(6..7)]
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_ADDED)).len(),
        1
    );
    let waves = paths(&shown.shapes, false);
    assert_eq!(waves.len(), 1);
    let underlined = span(1..4);
    assert_squiggle(
        waves[0].0,
        waves[0].1,
        Rect::from_min_max(
            pos2(underlined.left(), underlined.bottom() - SQUIGGLE_HEIGHT),
            underlined.max,
        ),
        DECORATION_WARNING,
    );
}

#[test]
fn 장식_범위는_탭_전각_문자_wrap_줄에서_글자의_x_범위에_놓이고_줄_장식은_모든_표시_줄에_놓인다() {
    let context = Context::default();
    let (columns, gutter) = wrap_columns(&context, FontId::monospace(FONT_SIZE), SCREEN[0]);
    let prefix = "\tab\t값 ";
    let line = format!("{prefix}{}", "word ".repeat(INDENTED_WORDS));
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
    let wrapped_rows = breaks.break_offsets.len();
    assert!(wrapped_rows >= WRAPPED_DECORATION_ROWS);
    assert_eq!(breaks.wrapped_text_indent_length, TAB_SIZE);
    let first_row_end = breaks.break_offsets[0];
    let (mut store, view) = fixture(&format!("{line}\nnext"), false);
    let surface = editor();
    let presentation = wrapping();
    let current = revision(&store, view);
    let sticky = Stickiness::default();
    let inner_tab = prefix.rfind('\t').unwrap();
    let wide = prefix.find('값').unwrap();
    let range = inner_tab..first_row_end + WRAPPED_DECORATION_TAIL;
    let marks = layer(
        current,
        GUTTER_Z_ORDER,
        sticky,
        vec![
            (
                line.len()..line.len(),
                DecorationKind::LineBackground(DECORATION_CONFLICT),
            ),
            (
                line.len()..line.len(),
                lane(LaneMark::Bar, DECORATION_ADDED),
            ),
        ],
    );
    let find = layer(
        current,
        FIND_Z_ORDER,
        sticky,
        vec![(range.clone(), range_background(DECORATION_FIND))],
    );
    let diagnostics = layer(
        current,
        DIAGNOSTICS_Z_ORDER,
        sticky,
        vec![(wide..wide + '값'.len_utf8(), squiggle(DECORATION_WARNING))],
    );
    let layers = [&marks, &find, &diagnostics];
    let mut draw = || {
        show_decorated(
            &context,
            &surface,
            &mut store,
            view,
            &presentation,
            &layers,
            Vec::new(),
        )
    };
    draw();
    let (shown, geometry) = draw();
    let rows = body_rows(&shown);
    assert_eq!(rows.len(), wrapped_rows + 1);
    let indent = TAB_SIZE as usize;
    let inner_tab_chars = rows[0].find('b').unwrap() + 1;
    let wide_chars = rows[0]
        .chars()
        .position(|character| character == '값')
        .unwrap();
    assert!(inner_tab_chars < wide_chars - 1);
    assert!(rows[1].starts_with(&format!("{}word ", " ".repeat(indent))));
    let lane_left = shown.viewport.left() + gutter - LINE_DECORATIONS_WIDTH;
    let left = shown.viewport.left() + gutter;
    let row = |index: usize| shown.viewport.top() + index as f32 * LINE_HEIGHT;
    let x = |index: usize, chars: usize| left + caret_offset(&context, &rows[index], chars);
    let highlighted = [
        Rect::from_min_max(
            pos2(x(0, inner_tab_chars), row(0)),
            pos2(x(0, rows[0].chars().count()), row(1)),
        ),
        Rect::from_min_max(
            pos2(x(1, indent), row(1)),
            pos2(x(1, indent + WRAPPED_DECORATION_TAIL), row(2)),
        ),
    ];
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_FIND)),
        highlighted
    );
    assert_eq!(geometry.range_rects(range), highlighted);
    assert_eq!(
        geometry.caret_rect(first_row_end),
        Some(Rect::from_min_max(
            pos2(x(1, indent), row(1)),
            pos2(x(1, indent), row(2))
        ))
    );
    let waves = paths(&shown.shapes, false);
    assert_eq!(waves.len(), 1);
    assert_squiggle(
        waves[0].0,
        waves[0].1,
        Rect::from_min_max(
            pos2(x(0, wide_chars), row(1) - SQUIGGLE_HEIGHT),
            pos2(x(0, wide_chars + 1), row(1)),
        ),
        DECORATION_WARNING,
    );
    let wrapped_rows = 0..wrapped_rows;
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_CONFLICT)),
        wrapped_rows
            .clone()
            .map(|index| Rect::from_min_max(
                pos2(left, row(index)),
                pos2(shown.viewport.right(), row(index + 1))
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_ADDED)),
        wrapped_rows
            .map(|index| Rect::from_min_size(
                pos2(lane_left, row(index)),
                vec2(LANE_BAR_WIDTH, LINE_HEIGHT)
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn 스크롤된_화면의_범위_장식은_글자와_함께_움직이고_줄_장식은_본문과_gutter_폭에_남는다() {
    let content = scrolled_document();
    let (mut store, view) = fixture(&content, false);
    let context = Context::default();
    let surface = editor();
    let presentation = EditorPresentation::default();
    let long_row = content
        .split_inclusive('\n')
        .take(SCROLLED_LONG_ROW)
        .map(str::len)
        .sum::<usize>();
    let marked_chars = WRAP_CLICK_COLUMN..WRAP_CLICK_COLUMN + WRAPPED_DECORATION_TAIL;
    let marked = long_row + marked_chars.start..long_row + marked_chars.end;
    let marks = layer(
        revision(&store, view),
        FIND_Z_ORDER,
        Stickiness::default(),
        vec![
            (marked.clone(), range_background(DECORATION_FIND)),
            (
                long_row..long_row,
                DecorationKind::LineBackground(DECORATION_CONFLICT),
            ),
            (long_row..long_row, lane(LaneMark::Bar, DECORATION_ADDED)),
        ],
    );
    let draw = |store: &mut EditorStore| {
        show_decorated(
            &context,
            &surface,
            store,
            view,
            &presentation,
            &[&marks],
            Vec::new(),
        )
    };
    draw(&mut store);
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            current.selection,
            ScrollPosition {
                x: -WHEEL_DELTA.x,
                y: -WHEEL_DELTA.y,
            },
            current.folds,
        )
        .unwrap();
    let (shown, geometry) = draw(&mut store);
    assert_eq!(geometry.scroll, -WHEEL_DELTA);
    let lane_left = shown.viewport.left()
        + line_numbers_width(
            &context,
            FontId::monospace(FONT_SIZE),
            LINE_NUMBERS_MIN_CHARS,
        );
    let left = lane_left + LINE_DECORATIONS_WIDTH;
    let top = shown.viewport.top() + SCROLLED_LONG_ROW as f32 * LINE_HEIGHT + WHEEL_DELTA.y;
    let long = "x".repeat(SCROLLED_LONG_ROW_COLUMNS);
    let x = |chars: usize| left + WHEEL_DELTA.x + caret_offset(&context, &long, chars);
    let highlighted = filled(&shown.shapes, token_color(DECORATION_FIND));
    assert_eq!(highlighted.len(), 1);
    assert!((highlighted[0].left() - x(marked_chars.start)).abs() < SQUIGGLE_TOLERANCE);
    assert!((highlighted[0].right() - x(marked_chars.end)).abs() < SQUIGGLE_TOLERANCE);
    assert_eq!(
        (highlighted[0].top(), highlighted[0].bottom()),
        (top, top + LINE_HEIGHT)
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_CONFLICT)),
        [Rect::from_min_max(
            pos2(left, top),
            pos2(shown.viewport.right(), top + LINE_HEIGHT)
        )]
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_ADDED)),
        [Rect::from_min_size(
            pos2(lane_left, top),
            vec2(LANE_BAR_WIDTH, LINE_HEIGHT)
        )]
    );
    assert_eq!(geometry.range_rects(marked.clone()), highlighted);
    let caret = geometry
        .caret_rect(marked.start)
        .expect("the long row is on screen");
    assert_eq!(
        (caret.left(), caret.right(), caret.top(), caret.bottom()),
        (
            highlighted[0].left(),
            highlighted[0].left(),
            top,
            top + LINE_HEIGHT
        )
    );
    assert_eq!(geometry.byte_at(caret.center()), Some(marked.start));
}

#[test]
fn 뒤처진_장식_층은_그_프레임의_편집까지_저널로_옮겨_그리고_따라갈_수_없는_층은_그리지_않는다() {
    let (mut store, view) = fixture("abcdef\nsecond", false);
    let context = Context::default();
    let surface = editor();
    let presentation = EditorPresentation::default();
    let items = || {
        vec![
            (2..4, range_background(DECORATION_FIND)),
            (0..0, lane(LaneMark::Bar, DECORATION_ADDED)),
        ]
    };
    let stale = layer(
        revision(&store, view),
        FIND_Z_ORDER,
        Stickiness::default(),
        items(),
    );
    let draw = |store: &mut EditorStore, layers: &[&DecorationLayer], events: Vec<Event>| {
        show_decorated(
            &context,
            &surface,
            store,
            view,
            &presentation,
            layers,
            events,
        )
        .0
    };
    let shown = draw(&mut store, &[&stale], Vec::new());
    let lane_left = shown.viewport.left()
        + line_numbers_width(
            &context,
            FontId::monospace(FONT_SIZE),
            LINE_NUMBERS_MIN_CHARS,
        );
    let left = lane_left + LINE_DECORATIONS_WIDTH;
    let row = |index: usize| shown.viewport.top() + index as f32 * LINE_HEIGHT;
    let span = |text: &str, chars: std::ops::Range<usize>, index: usize| {
        Rect::from_min_max(
            pos2(left + caret_offset(&context, text, chars.start), row(index)),
            pos2(
                left + caret_offset(&context, text, chars.end),
                row(index + 1),
            ),
        )
    };
    let bar = |index: usize| {
        Rect::from_min_size(
            pos2(lane_left, row(index)),
            vec2(LANE_BAR_WIDTH, LINE_HEIGHT),
        )
    };
    let typed = draw(&mut store, &[&stale], vec![Event::Text("XY".into())]);
    assert_eq!(text(&store, view), "XYabcdef\nsecond");
    assert_eq!(stale.revision() + 1, revision(&store, view));
    assert_eq!(
        filled(&typed.shapes, token_color(DECORATION_FIND)),
        [span("XYabcdef", 4..6, 0)]
    );
    assert_eq!(
        filled(&typed.shapes, token_color(DECORATION_ADDED)),
        [bar(0)]
    );
    let broken = draw(&mut store, &[&stale], vec![key(Key::Enter, false)]);
    assert_eq!(text(&store, view), "XY\nabcdef\nsecond");
    assert_eq!(
        filled(&broken.shapes, token_color(DECORATION_FIND)),
        [span("abcdef", 2..4, 1)]
    );
    assert_eq!(
        filled(&broken.shapes, token_color(DECORATION_ADDED)),
        [bar(0), bar(1)]
    );
    let untracked = layer(
        UNTRACKED_REVISION,
        FIND_Z_ORDER,
        Stickiness::default(),
        items(),
    );
    let fresh = layer(
        revision(&store, view),
        DIAGNOSTICS_Z_ORDER,
        Stickiness::default(),
        vec![(0..2, range_background(DECORATION_SELECTION_MATCH))],
    );
    let shown = draw(&mut store, &[&untracked, &fresh], Vec::new());
    assert!(filled(&shown.shapes, token_color(DECORATION_FIND)).is_empty());
    assert!(filled(&shown.shapes, token_color(DECORATION_ADDED)).is_empty());
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_SELECTION_MATCH)),
        [span("XY", 0..2, 0)]
    );
}

#[test]
fn 문자_중간의_장식_경계는_문자_시작으로_내리고_문서_끝을_넘으면_문서_끝에서_자른다() {
    let content = "값x\n끝";
    let (mut store, view) = fixture(content, false);
    let context = Context::default();
    let surface = editor();
    let presentation = EditorPresentation::default();
    let last = content.find('끝').unwrap();
    let marks = layer(
        revision(&store, view),
        FIND_Z_ORDER,
        Stickiness::default(),
        vec![
            (1..last - 1, range_background(DECORATION_FIND)),
            (
                last + 1..content.len() + last,
                range_background(DECORATION_SELECTION_MATCH),
            ),
            (
                content.len() + 1..content.len() + last,
                DecorationKind::LineBackground(DECORATION_CONFLICT),
            ),
        ],
    );
    let mut draw = || {
        show_decorated(
            &context,
            &surface,
            &mut store,
            view,
            &presentation,
            &[&marks],
            Vec::new(),
        )
        .0
    };
    draw();
    let shown = draw();
    let left = shown.viewport.left() + gutter_width(&context, FontId::monospace(FONT_SIZE));
    let row = |index: usize| shown.viewport.top() + index as f32 * LINE_HEIGHT;
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_FIND)),
        [Rect::from_min_max(
            pos2(left, row(0)),
            pos2(left + caret_offset(&context, "값x", 2), row(1))
        )]
    );
    assert_eq!(
        filled(&shown.shapes, token_color(DECORATION_SELECTION_MATCH)),
        [Rect::from_min_max(
            pos2(left, row(1)),
            pos2(left + caret_offset(&context, "끝", 1), row(2))
        )]
    );
    assert!(filled(&shown.shapes, token_color(DECORATION_CONFLICT)).is_empty());
}

#[test]
fn gutter_폭은_줄_번호와_줄_장식과_접기_폭의_합이고_줄_번호는_그_폭의_오른쪽에_맞춘다() {
    let context = Context::default();
    let font = FontId::monospace(FONT_SIZE);
    let numbers = |digits: usize| line_numbers_width(&context, font.clone(), digits);
    let folding = |word_wrap: bool| EditorPresentation {
        options: EditorDisplayOptions {
            word_wrap,
            folding: true,
            ..Default::default()
        },
    };
    let unnumbered = || NativeEditor {
        appearance: EditorAppearance {
            line_numbers: false,
            ..editor().appearance
        },
    };
    let many_lines = "x\n".repeat(MANY_LINES);
    assert_eq!(
        (many_lines.lines().count() + 1).to_string().len(),
        MANY_LINES_DIGITS
    );
    let shown = |surface: &NativeEditor, content: &str, presentation: &EditorPresentation| {
        let (mut store, view) = fixture(content, false);
        let context = Context::default();
        show_decorated(
            &context,
            surface,
            &mut store,
            view,
            presentation,
            &[],
            Vec::new(),
        )
    };
    let plain = EditorPresentation::default();
    for (surface, content, presentation, line_numbers, decorations) in [
        (
            editor(),
            PLAIN_DOCUMENT,
            &plain,
            numbers(LINE_NUMBERS_MIN_CHARS),
            LINE_DECORATIONS_WIDTH,
        ),
        (
            editor(),
            PLAIN_DOCUMENT,
            &folding(false),
            numbers(LINE_NUMBERS_MIN_CHARS),
            LINE_DECORATIONS_WIDTH + FOLDING_CONTROLS_WIDTH,
        ),
        (
            editor(),
            many_lines.as_str(),
            &plain,
            numbers(MANY_LINES_DIGITS),
            LINE_DECORATIONS_WIDTH,
        ),
        (
            unnumbered(),
            PLAIN_DOCUMENT,
            &plain,
            0.0,
            LINE_DECORATIONS_WIDTH,
        ),
        (
            unnumbered(),
            PLAIN_DOCUMENT,
            &folding(false),
            0.0,
            LINE_DECORATIONS_WIDTH + FOLDING_CONTROLS_WIDTH,
        ),
    ] {
        let (frame, geometry) = shown(&surface, content, presentation);
        let viewport = frame.viewport;
        let content_left = viewport.left() + line_numbers + decorations;
        assert_eq!(
            geometry.gutter_rect,
            Rect::from_min_max(viewport.min, pos2(content_left, viewport.bottom()))
        );
        assert_eq!(
            geometry.content_rect,
            Rect::from_min_max(pos2(content_left, viewport.top()), viewport.max)
        );
        for (_, position, _) in texts(&frame.shapes, viewport, true) {
            assert_eq!(position.x, content_left);
        }
        let numbered = texts(&frame.shapes, viewport, false);
        assert_eq!(numbered.is_empty(), !surface.appearance.line_numbers);
        for (_, position, width) in numbered {
            assert_eq!(position.x + width, viewport.left() + line_numbers);
        }
    }
    assert_eq!(numbers(LINE_NUMBERS_MIN_CHARS).fract(), 0.0);
    assert!(numbers(MANY_LINES_DIGITS) > numbers(LINE_NUMBERS_MIN_CHARS));
    let (mut store, view) = fixture(&"x".repeat(WRAPPED_LINE), false);
    let wrapped_context = Context::default();
    show_decorated(
        &wrapped_context,
        &editor(),
        &mut store,
        view,
        &folding(true),
        &[],
        Vec::new(),
    );
    let advance = measure(&context, &"x".repeat(ADVANCE_SAMPLE)) / ADVANCE_SAMPLE as f32;
    let text_width = SCREEN[0]
        - numbers(LINE_NUMBERS_MIN_CHARS)
        - LINE_DECORATIONS_WIDTH
        - FOLDING_CONTROLS_WIDTH;
    assert_eq!(
        store
            .views()
            .get(view)
            .unwrap()
            .display
            .as_ref()
            .and_then(|display| display.wrap_settings())
            .map(|settings| settings.wrap_column),
        Some(((text_width - VERTICAL_SCROLLBAR_SIZE - WRAP_CURSOR_ROOM) / advance).floor() as u32)
    );
    let (mut scrolled, scrolled_view) = fixture(&scrolled_document(), false);
    let reveal_context = Context::default();
    let mut output = reveal_context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| {
            editor()
                .reveal_presented(
                    ui,
                    &mut scrolled,
                    scrolled_view,
                    (SCROLLED_LONG_ROW + 1) as f64,
                    (SCROLLED_LONG_ROW_COLUMNS + 1) as f64,
                    &folding(false),
                )
                .unwrap();
        },
    );
    output.textures_delta.clear();
    let long_row_end = caret_offset(
        &context,
        &"x".repeat(SCROLLED_LONG_ROW_COLUMNS),
        SCROLLED_LONG_ROW_COLUMNS,
    );
    let revealed = scrolled.views().get(scrolled_view).unwrap().scroll.x;
    assert!((revealed - (long_row_end + CARET_STROKE - text_width)).abs() <= WIDTH_TOLERANCE);
}

#[test]
fn 좌표_질의는_보이는_표시_줄에서_문서_위치와_화면_rect를_서로_바꾼다() {
    let (mut store, view) = fixture(DECORATED_DOCUMENT, false);
    let context = Context::default();
    let surface = editor();
    let presentation = EditorPresentation::default();
    let draw = |store: &mut EditorStore, view: ViewId| {
        show_decorated(
            &context,
            &surface,
            store,
            view,
            &presentation,
            &[],
            Vec::new(),
        )
    };
    draw(&mut store, view);
    let (shown, geometry) = draw(&mut store, view);
    let viewport = shown.viewport;
    let left = viewport.left() + gutter_width(&context, FontId::monospace(FONT_SIZE));
    let row = |index: usize| viewport.top() + index as f32 * LINE_HEIGHT;
    let x = |chars: usize| left + caret_offset(&context, DECORATED_FIRST_ROW, chars);
    let caret = |x: f32, index: usize| {
        Some(Rect::from_min_max(
            pos2(x, row(index)),
            pos2(x, row(index + 1)),
        ))
    };
    let marked = DECORATED_DOCUMENT.find("ab").unwrap();
    let second = DECORATED_DOCUMENT.find('값').unwrap();
    let third = DECORATED_DOCUMENT.find("end").unwrap();
    let tab_chars = TAB_SIZE as usize;
    let first_row_chars = DECORATED_FIRST_ROW.chars().count();
    assert_eq!(geometry.caret_rect(0), caret(left, 0));
    assert_eq!(geometry.caret_rect(marked), caret(x(tab_chars), 0));
    assert_eq!(
        geometry.caret_rect(second - 1),
        caret(x(first_row_chars), 0)
    );
    assert_eq!(geometry.caret_rect(second), caret(left, 1));
    assert_eq!(
        geometry.caret_rect(DECORATED_DOCUMENT.len()),
        caret(left + caret_offset(&context, "end", 3), 2)
    );
    assert_eq!(geometry.caret_rect(DECORATED_DOCUMENT.len() + 1), None);
    assert_eq!(
        geometry.range_rects(marked..second + '값'.len_utf8()),
        [
            Rect::from_min_max(pos2(x(tab_chars), row(0)), pos2(x(first_row_chars), row(1))),
            Rect::from_min_max(
                pos2(left, row(1)),
                pos2(left + caret_offset(&context, "값 x", 1), row(2))
            ),
        ]
    );
    assert!(geometry.range_rects(marked..marked).is_empty());
    let inside = |x: f32, index: usize| pos2(x, row(index) + LINE_HEIGHT / CENTER_DIVISOR);
    let nudge = LARGE_CLICK_PAST_BOUNDARY;
    assert_eq!(
        geometry.byte_at(inside(x(tab_chars) + nudge, 0)),
        Some(marked)
    );
    assert_eq!(geometry.byte_at(inside(left + nudge, 0)), Some(0));
    assert_eq!(
        geometry.byte_at(inside(x(tab_chars) - nudge, 0)),
        Some(marked)
    );
    assert_eq!(
        geometry.byte_at(inside(viewport.right() - nudge, 1)),
        Some(third - 1)
    );
    assert_eq!(geometry.byte_at(inside(left - nudge, 0)), None);
    assert_eq!(geometry.byte_at(inside(left + nudge, DECORATED_ROWS)), None);
    assert_eq!(
        geometry.byte_at(pos2(left + nudge, viewport.bottom() + nudge)),
        None
    );
    let boundaries = DECORATED_DOCUMENT
        .char_indices()
        .map(|(byte, _)| byte)
        .chain([DECORATED_DOCUMENT.len()]);
    for byte in boundaries {
        let rect = geometry
            .caret_rect(byte)
            .expect("every boundary is on a shown row");
        assert_eq!(geometry.byte_at(rect.center()), Some(byte));
    }
    let content = scrolled_document();
    let (mut scrolled, scrolled_view) = fixture(&content, false);
    let context = Context::default();
    let draw = |store: &mut EditorStore, view: ViewId| {
        show_decorated(
            &context,
            &surface,
            store,
            view,
            &presentation,
            &[],
            Vec::new(),
        )
    };
    draw(&mut scrolled, scrolled_view);
    let (_, geometry) = draw(&mut scrolled, scrolled_view);
    let shown_rows = (SCREEN[1] / LINE_HEIGHT) as usize;
    assert_eq!(geometry.visible_rows, 0..shown_rows + 1);
    let row_start = |index: usize| {
        content
            .split_inclusive('\n')
            .take(index)
            .map(str::len)
            .sum::<usize>()
    };
    assert!(geometry.caret_rect(row_start(shown_rows - 1)).is_some());
    assert_eq!(geometry.caret_rect(row_start(shown_rows)), None);
    assert!(
        geometry
            .range_rects(row_start(shown_rows)..row_start(shown_rows + 1))
            .is_empty()
    );
    assert_eq!(geometry.caret_rect(content.len()), None);
}

#[test]
fn 장식이_없는_요청은_기존_평문_화면과_같고_보이지_않는_줄의_장식은_도형을_더하지_않는다() {
    let (mut store, view) = fixture(PLAIN_DOCUMENT, false);
    let context = Context::default();
    let surface = editor();
    let presentation = EditorPresentation::default();
    let draw =
        |context: &Context, store: &mut EditorStore, view: ViewId, layers: &[&DecorationLayer]| {
            show_decorated(
                context,
                &surface,
                store,
                view,
                &presentation,
                layers,
                Vec::new(),
            )
            .0
            .shapes
            .iter()
            .map(painted)
            .collect::<Vec<_>>()
        };
    draw(&context, &mut store, view, &[]);
    assert_frame(
        &draw(&context, &mut store, view, &[]),
        &PLAIN_FRAME[..PLAIN_FRAME.len() - FRAME_STATE_LINES],
    );
    let content = scrolled_document();
    let (mut scrolled, scrolled_view) = fixture(&content, false);
    let end = content.len();
    let hidden = layer(
        revision(&scrolled, scrolled_view),
        FIND_Z_ORDER,
        Stickiness::default(),
        vec![
            (end - 2..end, range_background(DECORATION_FIND)),
            (end - 2..end, squiggle(DECORATION_WARNING)),
            (
                end..end,
                DecorationKind::LineBackground(DECORATION_CONFLICT),
            ),
            (end..end, lane(LaneMark::Bar, DECORATION_ADDED)),
            (
                end..end,
                lane(LaneMark::DeletedTriangle, DECORATION_DELETED),
            ),
        ],
    );
    let context = Context::default();
    draw(&context, &mut scrolled, scrolled_view, &[]);
    let undecorated = draw(&context, &mut scrolled, scrolled_view, &[]);
    assert_eq!(
        draw(&context, &mut scrolled, scrolled_view, &[&hidden]),
        undecorated
    );
}

#[test]
fn 표시_줄_장식은_구간을_경계에서_나누어_전경색과_밑줄색만_바꾸고_글꼴_스타일은_남긴다() {
    let styles = token_styles();
    let font = FontId::monospace(FONT_SIZE);
    let source = "\tabcd";
    let string_start = source.find('c').unwrap();
    let mut row = row_text(source, TAB_SIZE, 0.0, 0);
    row.highlight(RowTokens {
        spans: &spans(&[(0, KEYWORD_STYLE_ID), (string_start, STRING_STYLE_ID)]),
        styles: &styles,
        row_start_byte: 0,
    });
    let tab_chars = TAB_SIZE as usize;
    let string_chars = tab_chars + 2;
    let end = string_chars + 2;
    let link = token_color(DECORATION_LINK);
    let style = RowInlineStyle {
        foreground: Some(token_color(DECORATION_BRACKET)),
        underline: Some(link),
    };
    let undecorated = row.clone();
    row.decorate(string_chars..string_chars, style);
    assert_eq!(row, undecorated);
    row.decorate(string_chars - 1..string_chars + 1, style);
    let bold = RowFontStyle {
        is_bold: true,
        ..RowFontStyle::default()
    };
    let slanted = RowFontStyle {
        is_italic: true,
        is_underlined: true,
        ..RowFontStyle::default()
    };
    assert_eq!(
        row.sections,
        [
            row_section(TOKEN_KEYWORD, 0..string_chars - 1),
            row_section(DECORATION_BRACKET, string_chars - 1..string_chars),
            row_section(DECORATION_BRACKET, string_chars..string_chars + 1),
            row_section(TOKEN_STRING, string_chars + 1..end),
        ]
    );
    assert_eq!(row.font_styles, [bold, bold, slanted, slanted]);
    assert_eq!(row.underline_colors, [None, Some(link), Some(link), None]);
    assert_eq!(row.text, undecorated.text);
    assert_eq!(row.model_bytes, undecorated.model_bytes);
    let job = row.styled_layout_job(&font, None);
    assert_eq!(
        job.sections
            .iter()
            .map(|section| (
                section.format.color,
                section.format.italics,
                section.format.underline
            ))
            .collect::<Vec<_>>(),
        [
            (token_color(TOKEN_KEYWORD), false, Stroke::NONE),
            (
                token_color(DECORATION_BRACKET),
                false,
                decoration(DECORATION_LINK)
            ),
            (
                token_color(DECORATION_BRACKET),
                true,
                decoration(DECORATION_LINK)
            ),
            (token_color(TOKEN_STRING), true, decoration(TOKEN_STRING)),
        ]
    );
    row.decorate(
        0..end,
        RowInlineStyle {
            foreground: None,
            underline: Some(link),
        },
    );
    assert_eq!(row.sections.len(), 4);
    assert_eq!(
        row.sections[0],
        row_section(TOKEN_KEYWORD, 0..string_chars - 1)
    );
    assert_eq!(row.underline_colors, [Some(link); 4]);
}

const FOLD_CONTROL_MARGIN: f32 = 2.0;
const FOLD_CONTROL_CLICK_INSET: f32 = 4.0;
const FOLD_CONTROL_FONT_SCALE: f32 = 1.4;
const FOLD_CONTROL_FADE_SECONDS: f64 = 0.5;
const FOLD_BACKGROUND_OPACITY: f32 = 0.3;
const FOLD_PLACEHOLDER: &str = "\u{22EF}";
const FOLD_PLACEHOLDER_MARGIN_EM: f32 = 0.2;
const FOLD_PLACEHOLDER_GRAY: u8 = 128;
const FOLD_LINES: [&str; 7] = [
    "fn outer() {",
    "    if a {",
    "        one();",
    "    }",
    "    tail();",
    "}",
    "end",
];
const INNER_HEADER: usize = 1;
const INNER_HIDDEN: std::ops::RangeInclusive<usize> = 2..=2;
const OUTER_HIDDEN: std::ops::RangeInclusive<usize> = 1..=4;
const FOLD_CLICK_INSIDE: f32 = 1.0;
const WRAPPED_HEADER_CHARS: usize = 100;
const TRAILING_FOLD_DOCUMENT: &str = "a\n  b\n  c";
const FOLD_BLOCKS: usize = 40;
const FOLD_LONG_LINES: usize = 200;
const FOLD_LONG_HEADER: usize = 150;
const FOLD_ABOVE_HEADER: usize = 10;
const FOLD_ABOVE_LAST: usize = 19;
const FOLD_SCROLLED_LINE: usize = 100;
const FOLD_SCROLLED_HIDDEN_LINE: usize = 15;
const FOLD_EDGE_CLICK_INSET: f32 = 3.0;
const FOLD_SHOWN_LINE: usize = 4;
const FOLD_SHOWN_ROW: usize = 3;
const FOLD_SHOWN_MARK: std::ops::Range<usize> = 4..8;
const FOLD_HIDDEN_MARK_BYTES: usize = 2;
const FOLD_DRAG_FROM_COLUMN: usize = 3;
const FOLD_DRAG_TO_LINE: usize = 3;
const FOLD_DRAG_TO_ROW: usize = 2;
const FOLD_DRAG_TO_COLUMN: usize = 5;
const HOVER_START: f64 = 2.0;
const HOVER_END: f64 = 3.0;
const GUTTER_POINT: Pos2 = pos2(5.0, 30.0);
const TEXT_POINT: Pos2 = pos2(400.0, 130.0);

fn fold_document() -> String {
    FOLD_LINES.join("\n")
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(index, _)| index + 1))
        .collect()
}

fn lines_fold(text: &str, hidden: std::ops::RangeInclusive<usize>) -> std::ops::Range<usize> {
    let starts = line_starts(text);
    let end = starts
        .get(hidden.end() + 1)
        .map_or(text.len(), |next| next - 1);
    starts[*hidden.start()]..end
}

fn fold_line_start(line: usize) -> usize {
    line_starts(&fold_document())[line]
}

fn fold_line_end(line: usize) -> usize {
    fold_line_start(line) + FOLD_LINES[line].len()
}

fn hidden_fold(hidden: std::ops::RangeInclusive<usize>) -> std::ops::Range<usize> {
    lines_fold(&fold_document(), hidden)
}

fn folds(store: &EditorStore, view: ViewId) -> Vec<std::ops::Range<usize>> {
    store.views().get(view).unwrap().folds.clone()
}

fn only_fold(store: &EditorStore, view: ViewId) -> std::ops::Range<usize> {
    let folds = folds(store, view);
    assert_eq!(folds.len(), 1);
    folds[0].clone()
}

fn hide(store: &mut EditorStore, view: ViewId, folds: Vec<std::ops::Range<usize>>) {
    let current = store.views().get(view).unwrap().clone();
    store
        .set_view_state(view, current.selection, current.scroll, folds)
        .unwrap();
}

fn place_caret(store: &mut EditorStore, view: ViewId, byte: usize) {
    let current = store.views().get(view).unwrap().clone();
    store
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

fn folding_presentation(word_wrap: bool) -> EditorPresentation {
    EditorPresentation {
        options: EditorDisplayOptions {
            word_wrap,
            folding: true,
            ..Default::default()
        },
    }
}

struct FoldFrame<'a> {
    presentation: &'a EditorPresentation,
    commands: &'a [FoldCommand],
    events: Vec<Event>,
    modifiers: Modifiers,
    time: Option<f64>,
    tokens: Option<EditorTokens<'a>>,
    language: Option<Language<'a>>,
    decorations: &'a [&'a DecorationLayer],
}

impl<'a> FoldFrame<'a> {
    fn idle(presentation: &'a EditorPresentation) -> Self {
        Self {
            presentation,
            commands: &[],
            events: Vec::new(),
            modifiers: Modifiers::NONE,
            time: None,
            tokens: None,
            language: None,
            decorations: &[],
        }
    }
}

struct Folded {
    shown: Wrapped,
    controls: Vec<FoldControl>,
    control_color: Color32,
}

fn show_folding(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    frame: FoldFrame<'_>,
) -> Folded {
    let mut shown = None;
    let mut controls = Vec::new();
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events: std::iter::once(Event::ModifiersChanged(frame.modifiers))
                .chain(frame.events)
                .collect(),
            time: frame.time,
            ..Default::default()
        },
        |ui| {
            let viewport = ui.available_rect_before_wrap().intersect(ui.clip_rect());
            let control_color = ui.visuals().weak_text_color();
            let mut record = |_: &Ui, control: FoldControl| controls.push(control);
            let output = editor()
                .show_request(
                    ui,
                    store,
                    view,
                    EditorRequest {
                        request_focus: true,
                        keymap: |_: &Ui, _: &Event, _: bool| false,
                        route: |_: &Response| None,
                        presentation: frame.presentation,
                        tokens: |_: &EditorStore| frame.tokens,
                        language: frame.language,
                        decorations: frame.decorations,
                        fold_commands: frame.commands,
                        fold_controls: Some(&mut record),
                    },
                )
                .unwrap();
            assert!(output.errors.is_empty());
            shown = Some((
                viewport,
                output.rendered_lines,
                output.geometry.visible_rows,
                control_color,
            ));
        },
    );
    output.textures_delta.clear();
    let (viewport, rendered_lines, visible_rows, control_color) = shown.unwrap();
    Folded {
        shown: Wrapped {
            shapes: output.shapes,
            viewport,
            rendered_lines,
            visible_rows,
            ime_cursor: output.platform_output.ime.map(|ime| ime.cursor_rect),
        },
        controls,
        control_color,
    }
}

fn run_folds(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    commands: &[FoldCommand],
) -> Folded {
    show_folding(
        context,
        store,
        view,
        FoldFrame {
            commands,
            ..FoldFrame::idle(presentation)
        },
    )
}

fn press_keys(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    events: Vec<Event>,
) -> Folded {
    show_folding(
        context,
        store,
        view,
        FoldFrame {
            events,
            ..FoldFrame::idle(presentation)
        },
    )
}

fn fold_click(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    position: Pos2,
    (button, modifiers): (PointerButton, Modifiers),
) -> Folded {
    let presentation = folding_presentation(false);
    let pressed = |pressed: bool| Event::PointerButton {
        pos: position,
        button,
        pressed,
        modifiers,
    };
    for events in [
        vec![Event::PointerMoved(position), pressed(true)],
        vec![pressed(false)],
    ] {
        show_folding(
            context,
            store,
            view,
            FoldFrame {
                events,
                modifiers,
                ..FoldFrame::idle(&presentation)
            },
        );
    }
    show_folding(context, store, view, FoldFrame::idle(&presentation))
}

fn fold_lane_left(context: &Context) -> f32 {
    line_numbers_width(
        context,
        FontId::monospace(FONT_SIZE),
        LINE_NUMBERS_MIN_CHARS,
    )
}

fn fold_text_left(context: &Context) -> f32 {
    fold_lane_left(context) + LINE_DECORATIONS_WIDTH + FOLDING_CONTROLS_WIDTH
}

fn row_middle(row: usize) -> f32 {
    (row as f32 + 1.0 / CENTER_DIVISOR) * LINE_HEIGHT
}

fn fold_control_rect(context: &Context, row: usize) -> Rect {
    Rect::from_center_size(
        pos2(
            fold_lane_left(context)
                + FOLD_CONTROL_MARGIN
                + (LINE_DECORATIONS_WIDTH + FOLDING_CONTROLS_WIDTH) / CENTER_DIVISOR,
            row_middle(row),
        ),
        Vec2::splat(FONT_SIZE * FOLD_CONTROL_FONT_SCALE),
    )
}

fn fold_control_point(context: &Context, row: usize) -> Pos2 {
    pos2(
        fold_lane_left(context)
            + FOLD_CONTROL_MARGIN
            + FOLD_CONTROL_CLICK_INSET
            + FOLD_CLICK_INSIDE,
        row_middle(row),
    )
}

fn gutter_numbers(folded: &Folded) -> Vec<String> {
    texts(&folded.shown.shapes, folded.shown.viewport, false)
        .into_iter()
        .map(|(text, _, _)| text)
        .collect()
}

fn fold_background() -> Color32 {
    Color32::BLUE.gamma_multiply(FOLD_BACKGROUND_OPACITY)
}

fn collapsed_control(context: &Context, folded: &Folded, row: usize) -> FoldControl {
    FoldControl {
        rect: fold_control_rect(context, row),
        chevron_rotation: 0.0,
        color: folded.control_color,
    }
}

#[test]
fn 접힌_머리_줄은_배경과_생략_표식과_펼침_컨트롤을_가지고_숨김_줄은_본문과_줄_번호에서_빠진다() {
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    let unfolded = show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    assert_eq!(body_rows(&unfolded.shown), FOLD_LINES);
    assert!(unfolded.controls.is_empty());
    assert!(filled(&unfolded.shown.shapes, fold_background()).is_empty());
    hide(&mut store, view, vec![hidden_fold(INNER_HIDDEN)]);
    let folded = show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let viewport = folded.shown.viewport;
    let header = FOLD_LINES[INNER_HEADER];
    assert_eq!(
        body_rows(&folded.shown),
        [
            FOLD_LINES[0],
            header,
            FOLD_PLACEHOLDER,
            FOLD_LINES[3],
            FOLD_LINES[4],
            FOLD_LINES[5],
            FOLD_LINES[6]
        ]
    );
    assert_eq!(gutter_numbers(&folded), ["1", "2", "4", "5", "6", "7"]);
    assert_eq!(folded.shown.rendered_lines, 0..FOLD_LINES.len());
    assert_eq!(folded.shown.visible_rows, 0..FOLD_LINES.len() - 1);
    let left = viewport.left() + fold_text_left(&context);
    let row_top = |row: usize| viewport.top() + row as f32 * LINE_HEIGHT;
    assert_eq!(
        filled(&folded.shown.shapes, fold_background()),
        [Rect::from_min_max(
            pos2(left, row_top(INNER_HEADER)),
            pos2(viewport.right(), row_top(INNER_HEADER + 1))
        )]
    );
    let body = texts(&folded.shown.shapes, viewport, true);
    assert_eq!(
        body[INNER_HEADER + 1].1,
        pos2(
            left + caret_offset(&context, header, header.len())
                + FOLD_PLACEHOLDER_MARGIN_EM * FONT_SIZE,
            body[INNER_HEADER].1.y
        )
    );
    assert_eq!(
        colored_rows(&folded.shown)[INNER_HEADER + 1],
        [(
            FOLD_PLACEHOLDER.to_owned(),
            Color32::from_gray(FOLD_PLACEHOLDER_GRAY)
        )]
    );
    assert_eq!(
        folded.controls,
        [collapsed_control(&context, &folded, INNER_HEADER)]
    );
    hide(
        &mut store,
        view,
        vec![hidden_fold(OUTER_HIDDEN), hidden_fold(INNER_HIDDEN)],
    );
    let nested = show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    assert_eq!(
        body_rows(&nested.shown),
        [
            FOLD_LINES[0],
            FOLD_PLACEHOLDER,
            FOLD_LINES[5],
            FOLD_LINES[6]
        ]
    );
    assert_eq!(gutter_numbers(&nested), ["1", "6", "7"]);
    assert_eq!(nested.controls, [collapsed_control(&context, &nested, 0)]);
}

#[test]
fn 줄바꿈된_접힌_머리_줄은_모든_표시_줄에_배경을_마지막_표시_줄에_생략_표식을_첫_표시_줄에_컨트롤을_둔다()
 {
    let long = "x".repeat(WRAPPED_HEADER_CHARS);
    let document = format!("{long}\n    body\nend");
    let (mut store, view) = fixture(&document, false);
    let context = Context::default();
    let presentation = folding_presentation(true);
    hide(&mut store, view, vec![lines_fold(&document, 1..=1)]);
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let folded = show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let viewport = folded.shown.viewport;
    let left = viewport.left() + fold_text_left(&context);
    let advance = measure(&context, &"x".repeat(ADVANCE_SAMPLE)) / ADVANCE_SAMPLE as f32;
    let columns =
        ((viewport.width() - fold_text_left(&context) - VERTICAL_SCROLLBAR_SIZE - WRAP_CURSOR_ROOM)
            / advance)
            .floor() as usize;
    assert!(columns < WRAPPED_HEADER_CHARS && WRAPPED_HEADER_CHARS < columns * 2);
    assert_eq!(
        body_rows(&folded.shown),
        [&long[..columns], &long[columns..], FOLD_PLACEHOLDER, "end"]
    );
    assert_eq!(gutter_numbers(&folded), ["1", "3"]);
    let row_top = |row: usize| viewport.top() + row as f32 * LINE_HEIGHT;
    assert_eq!(
        filled(&folded.shown.shapes, fold_background()),
        [0, 1].map(|row| Rect::from_min_max(
            pos2(left, row_top(row)),
            pos2(viewport.right(), row_top(row + 1))
        ))
    );
    let body = texts(&folded.shown.shapes, viewport, true);
    assert_eq!(
        body[2].1,
        pos2(
            left + caret_offset(&context, &long[columns..], WRAPPED_HEADER_CHARS - columns)
                + FOLD_PLACEHOLDER_MARGIN_EM * FONT_SIZE,
            body[1].1.y
        )
    );
    assert_eq!(folded.controls, [collapsed_control(&context, &folded, 0)]);
}

#[test]
fn 펼쳐진_영역의_접기_컨트롤은_gutter에_포인터가_있는_동안만_반_초에_걸쳐_나타나고_사라진다() {
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    let mut at = |time: f64, events: Vec<Event>| {
        show_folding(
            &context,
            &mut store,
            view,
            FoldFrame {
                events,
                time: Some(time),
                ..FoldFrame::idle(&presentation)
            },
        )
    };
    let half = FOLD_CONTROL_FADE_SECONDS / f64::from(CENTER_DIVISOR);
    let measuring = Context::default();
    let rects = [0, INNER_HEADER].map(|row| fold_control_rect(&measuring, row));
    let expanded = |folded: &Folded, opacity: f32| {
        rects.map(|rect| FoldControl {
            rect,
            chevron_rotation: std::f32::consts::FRAC_PI_2,
            color: folded.control_color.gamma_multiply(opacity),
        })
    };
    let half_eased = ease(1.0 / CENTER_DIVISOR);
    assert!(at(HOVER_START - half, Vec::new()).controls.is_empty());
    assert!(
        at(HOVER_START, vec![Event::PointerMoved(GUTTER_POINT)])
            .controls
            .is_empty()
    );
    let appearing = at(HOVER_START + half, Vec::new());
    assert_eq!(appearing.controls, expanded(&appearing, half_eased));
    let appeared = at(HOVER_START + FOLD_CONTROL_FADE_SECONDS, Vec::new());
    assert_eq!(appeared.controls, expanded(&appeared, 1.0));
    let leaving = at(HOVER_END, vec![Event::PointerMoved(TEXT_POINT)]);
    assert_eq!(leaving.controls, expanded(&leaving, 1.0));
    let disappearing = at(HOVER_END + half, Vec::new());
    assert_eq!(
        disappearing.controls,
        expanded(&disappearing, 1.0 - half_eased)
    );
    assert!(
        at(HOVER_END + FOLD_CONTROL_FADE_SECONDS, Vec::new())
            .controls
            .is_empty()
    );
}

#[test]
fn gutter의_접기_컨트롤_클릭은_캐럿을_옮기지_않고_그_줄의_영역을_전환하며_수정_키와_가운데_버튼은_재귀와_주변_전환이다()
 {
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let plain = (PointerButton::Primary, Modifiers::NONE);
    let shifted = (PointerButton::Primary, Modifiers::SHIFT);
    let inner = hidden_fold(INNER_HIDDEN);
    let outer = hidden_fold(OUTER_HIDDEN);
    show_folding(
        &context,
        &mut store,
        view,
        FoldFrame::idle(&folding_presentation(false)),
    );
    let measuring = Context::default();
    let control = |row: usize| fold_control_point(&measuring, row);
    let folded = fold_click(&context, &mut store, view, control(INNER_HEADER), plain);
    assert_eq!(only_fold(&store, view), inner);
    assert_eq!(selection(&store, view), (0, 0));
    assert_eq!(body_rows(&folded.shown).len(), FOLD_LINES.len());
    assert_eq!(gutter_numbers(&folded), ["1", "2", "4", "5", "6", "7"]);
    fold_click(&context, &mut store, view, control(INNER_HEADER), plain);
    assert!(folds(&store, view).is_empty());
    fold_click(&context, &mut store, view, control(4), plain);
    assert!(folds(&store, view).is_empty());
    assert_eq!(selection(&store, view), (0, 0));
    let outside = pos2(
        control(INNER_HEADER).x - FOLD_CLICK_INSIDE * CENTER_DIVISOR,
        row_middle(INNER_HEADER),
    );
    fold_click(&context, &mut store, view, outside, plain);
    assert!(folds(&store, view).is_empty());
    assert_eq!(selection(&store, view), (0, 0));
    fold_click(&context, &mut store, view, control(0), shifted);
    assert_eq!(only_fold(&store, view), inner);
    assert_eq!(selection(&store, view), (0, 0));
    let folded = fold_click(&context, &mut store, view, control(0), shifted);
    assert_eq!(folds(&store, view), [outer.clone(), inner.clone()]);
    assert_eq!(selection(&store, view), (0, 0));
    assert_eq!(gutter_numbers(&folded), ["1", "6", "7"]);
    fold_click(
        &context,
        &mut store,
        view,
        control(0),
        (PointerButton::Primary, Modifiers::ALT),
    );
    assert_eq!(folds(&store, view), [outer, inner]);
    fold_click(
        &context,
        &mut store,
        view,
        control(0),
        (PointerButton::Middle, Modifiers::NONE),
    );
    assert!(folds(&store, view).is_empty());
    assert_eq!(selection(&store, view), (0, 0));
}

#[test]
fn 접힌_줄_끝의_생략_표식_클릭은_캐럿을_줄_끝에_두고_펼치며_그_뒤의_빈_곳과_줄_안의_클릭은_펼치지_않는다()
 {
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let plain = (PointerButton::Primary, Modifiers::NONE);
    let inner = hidden_fold(INNER_HIDDEN);
    let header = FOLD_LINES[INNER_HEADER];
    let text_end = fold_text_left(&context) + caret_offset(&context, header, header.len());
    let margin = FOLD_PLACEHOLDER_MARGIN_EM * FONT_SIZE;
    let placeholder_width = measure(&context, FOLD_PLACEHOLDER);
    let at = |x: f32| pos2(x, row_middle(INNER_HEADER));
    let header_end = fold_line_end(INNER_HEADER);
    for (x, unfolds, head) in [
        (
            text_end + margin + placeholder_width / CENTER_DIVISOR,
            true,
            header_end,
        ),
        (
            text_end + margin * CENTER_DIVISOR + placeholder_width + LINE_HEIGHT,
            false,
            header_end,
        ),
        (text_end + margin / CENTER_DIVISOR, true, header_end),
        (
            fold_text_left(&context) + FOLD_CLICK_INSIDE,
            false,
            fold_line_start(INNER_HEADER),
        ),
    ] {
        hide(&mut store, view, vec![inner.clone()]);
        place_caret(&mut store, view, 0);
        show_folding(
            &context,
            &mut store,
            view,
            FoldFrame::idle(&folding_presentation(false)),
        );
        fold_click(&context, &mut store, view, at(x), plain);
        assert_eq!(folds(&store, view).is_empty(), unfolds, "{x}");
        assert_eq!(selection(&store, view), (head, head), "{x}");
    }
}

#[test]
fn 접기_명령은_그_프레임에_적용되어_캐럿을_머리_줄_끝으로_옮기고_접기가_꺼진_표시에서는_접힘을_지운다()
 {
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    let inner = hidden_fold(INNER_HIDDEN);
    let outer = hidden_fold(OUTER_HIDDEN);
    place_caret(&mut store, view, fold_line_start(2));
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let folded = run_folds(
        &context,
        &mut store,
        view,
        &presentation,
        &[FoldCommand::Fold],
    );
    assert_eq!(only_fold(&store, view), inner);
    let header = FOLD_LINES[INNER_HEADER];
    let header_end = fold_line_end(INNER_HEADER);
    assert_eq!(selection(&store, view), (header_end, header_end));
    assert_eq!(gutter_numbers(&folded), ["1", "2", "4", "5", "6", "7"]);
    assert_caret(
        &folded.shown,
        fold_text_left(&context) + caret_offset(&context, header, header.len()),
        INNER_HEADER as f32 * LINE_HEIGHT,
    );
    let all = run_folds(
        &context,
        &mut store,
        view,
        &presentation,
        &[FoldCommand::FoldAll],
    );
    assert_eq!(folds(&store, view), [outer, inner.clone()]);
    assert_eq!(
        selection(&store, view),
        (fold_line_end(0), fold_line_end(0))
    );
    assert_eq!(
        body_rows(&all.shown),
        [
            FOLD_LINES[0],
            FOLD_PLACEHOLDER,
            FOLD_LINES[5],
            FOLD_LINES[6]
        ]
    );
    run_folds(
        &context,
        &mut store,
        view,
        &presentation,
        &[FoldCommand::UnfoldAll],
    );
    assert!(folds(&store, view).is_empty());
    run_folds(
        &context,
        &mut store,
        view,
        &presentation,
        &[FoldCommand::FoldAll, FoldCommand::UnfoldAll],
    );
    assert!(folds(&store, view).is_empty());
    let unfoldable = EditorPresentation::default();
    run_folds(
        &context,
        &mut store,
        view,
        &unfoldable,
        &[FoldCommand::FoldAll],
    );
    assert!(folds(&store, view).is_empty());
    hide(&mut store, view, vec![inner]);
    let shown = run_folds(&context, &mut store, view, &unfoldable, &[]);
    assert!(folds(&store, view).is_empty());
    assert_eq!(body_rows(&shown.shown), FOLD_LINES);
    assert!(shown.controls.is_empty());
}

fn long_fold_document(header: usize, last: usize) -> String {
    (0..FOLD_LONG_LINES)
        .map(|line| {
            if line == header {
                "head {"
            } else if line > header && line <= last {
                "    body"
            } else {
                "x"
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn 접기_명령_뒤에_선택의_시작이_화면_밖이면_가운데로_보이게_하고_화면_안이면_스크롤을_그대로_둔다()
{
    let document = long_fold_document(FOLD_LONG_HEADER, FOLD_LONG_HEADER + 1);
    let starts = line_starts(&document);
    let (mut store, view) = fixture(&document, false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    place_caret(&mut store, view, starts[FOLD_LONG_HEADER + 1]);
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    assert_eq!(
        store.views().get(view).unwrap().scroll,
        ScrollPosition::default()
    );
    run_folds(
        &context,
        &mut store,
        view,
        &presentation,
        &[FoldCommand::Fold],
    );
    let hidden = lines_fold(&document, FOLD_LONG_HEADER + 1..=FOLD_LONG_HEADER + 1);
    assert_eq!(folds(&store, view), [hidden]);
    let header_end = starts[FOLD_LONG_HEADER + 1] - 1;
    assert_eq!(selection(&store, view), (header_end, header_end));
    let centered = row_middle(FOLD_LONG_HEADER) - SCREEN[1] / CENTER_DIVISOR;
    assert_eq!(store.views().get(view).unwrap().scroll.y, centered);
    run_folds(
        &context,
        &mut store,
        view,
        &presentation,
        &[FoldCommand::Unfold],
    );
    assert!(folds(&store, view).is_empty());
    assert_eq!(store.views().get(view).unwrap().scroll.y, centered);
}

#[test]
fn 접기_컨트롤_클릭_뒤에_머리_줄이_화면에_다_보이지_않으면_가운데로_보이게_하고_다_보이면_스크롤을_그대로_둔다()
 {
    let document = long_fold_document(FOLD_ABOVE_HEADER, FOLD_ABOVE_LAST);
    let hidden = lines_fold(&document, FOLD_ABOVE_HEADER + 1..=FOLD_ABOVE_LAST);
    let (mut store, view) = fixture(&document, false);
    let context = Context::default();
    let measuring = Context::default();
    let presentation = folding_presentation(false);
    let plain = (PointerButton::Primary, Modifiers::NONE);
    let scroll_y = |store: &EditorStore| store.views().get(view).unwrap().scroll.y;
    let header_top = FOLD_ABOVE_HEADER as f32 * LINE_HEIGHT;
    let clipped = header_top + STABLE_DELTA - SCREEN[1];
    scroll_to(&mut store, view, clipped);
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    assert_eq!(scroll_y(&store), clipped);
    let control_x = fold_control_point(&measuring, FOLD_ABOVE_HEADER).x;
    fold_click(
        &context,
        &mut store,
        view,
        pos2(control_x, header_top - clipped + FOLD_EDGE_CLICK_INSET),
        plain,
    );
    assert_eq!(folds(&store, view), [hidden]);
    let centered = row_middle(FOLD_ABOVE_HEADER) - SCREEN[1] / CENTER_DIVISOR;
    assert_eq!(scroll_y(&store), centered);
    fold_click(
        &context,
        &mut store,
        view,
        pos2(control_x, row_middle(FOLD_ABOVE_HEADER) - centered),
        plain,
    );
    assert!(folds(&store, view).is_empty());
    assert_eq!(scroll_y(&store), centered);
}

#[test]
fn 접힌_상태의_좌우_이동은_숨김_줄을_건너뛰고_캐럿이_숨김_줄에_들어가면_그_프레임에_펼친다() {
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    let inner = hidden_fold(INNER_HIDDEN);
    hide(&mut store, view, vec![inner.clone()]);
    place_caret(&mut store, view, fold_line_start(3));
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    for (pressed, head) in [
        (Key::ArrowLeft, fold_line_end(INNER_HEADER)),
        (Key::ArrowRight, fold_line_start(3)),
        (Key::ArrowUp, fold_line_start(INNER_HEADER)),
        (Key::ArrowDown, fold_line_start(3)),
    ] {
        press_keys(
            &context,
            &mut store,
            view,
            &presentation,
            vec![key(pressed, false)],
        );
        assert_eq!(selection(&store, view), (head, head), "{pressed:?}");
        assert_eq!(only_fold(&store, view), inner, "{pressed:?}");
    }

    let (mut store, view) = fixture(TRAILING_FOLD_DOCUMENT, false);
    let context = Context::default();
    let trailing = lines_fold(TRAILING_FOLD_DOCUMENT, 1..=2);
    let end = TRAILING_FOLD_DOCUMENT.len();
    hide(&mut store, view, vec![trailing.clone()]);
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let extended = press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![chord(
            Key::End,
            Modifiers {
                command: true,
                shift: true,
                ..Modifiers::default()
            },
        )],
    );
    assert_eq!(selection(&store, view), (0, end));
    assert_eq!(only_fold(&store, view), trailing);
    assert_eq!(body_rows(&extended.shown), ["a", FOLD_PLACEHOLDER]);
    place_caret(&mut store, view, 0);
    let revealed = press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![key(Key::End, true)],
    );
    assert_eq!(selection(&store, view), (end, end));
    assert!(folds(&store, view).is_empty());
    assert_eq!(body_rows(&revealed.shown), ["a", "  b", "  c"]);

    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    hide(&mut store, view, vec![inner]);
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
                .reveal_presented(ui, &mut store, view, 3.0, 1.0, &presentation)
                .unwrap();
        },
    );
    output.textures_delta.clear();
    assert!(folds(&store, view).is_empty());
    assert_eq!(
        selection(&store, view),
        (fold_line_start(2), fold_line_start(2))
    );
}

#[test]
fn 접힌_상태의_page_이동과_세로_이동은_보이는_표시_줄_수를_따른다() {
    let document = (0..FOLD_BLOCKS)
        .map(|block| format!("h{block}\n  b{block}"))
        .chain(["end".to_owned()])
        .collect::<Vec<_>>()
        .join("\n");
    let starts = line_starts(&document);
    let (mut store, view) = fixture(&document, false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let folded = run_folds(
        &context,
        &mut store,
        view,
        &presentation,
        &[FoldCommand::FoldAll],
    );
    assert_eq!(folds(&store, view).len(), FOLD_BLOCKS);
    assert_eq!(selection(&store, view), (0, 0));
    let page_rows = (folded.shown.viewport.height() / LINE_HEIGHT - PAGE_OVERLAP_LINES) as usize;
    let header = |block: usize| starts[block * 2];
    for (pressed, block) in [
        (Key::PageDown, page_rows),
        (Key::ArrowDown, page_rows + 1),
        (Key::ArrowUp, page_rows),
        (Key::ArrowUp, page_rows - 1),
        (Key::PageDown, page_rows * 2 - 1),
    ] {
        press_keys(
            &context,
            &mut store,
            view,
            &presentation,
            vec![key(pressed, false)],
        );
        assert_eq!(
            selection(&store, view),
            (header(block), header(block)),
            "{pressed:?}"
        );
    }
    assert_eq!(folds(&store, view).len(), FOLD_BLOCKS);
    let shown = show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    assert_eq!(
        store.views().get(view).unwrap().scroll.y,
        (page_rows * 2) as f32 * LINE_HEIGHT - shown.shown.viewport.height()
    );
}

#[test]
fn 접힌_머리_줄_끝의_입력은_접힘을_유지하고_줄바꿈은_펼치며_undo는_접힘을_지우지_않는다() {
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    let inner = hidden_fold(INNER_HIDDEN);
    hide(&mut store, view, vec![inner.clone()]);
    place_caret(&mut store, view, fold_line_end(INNER_HEADER));
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let typed = press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![Event::Text("x".into())],
    );
    assert_eq!(only_fold(&store, view), inner.start + 1..inner.end + 1);
    assert_eq!(
        body_rows(&typed.shown)[INNER_HEADER..=INNER_HEADER + 1],
        [
            format!("{}x", FOLD_LINES[INNER_HEADER]),
            FOLD_PLACEHOLDER.to_owned()
        ]
    );
    press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![key(Key::Z, true)],
    );
    assert_eq!(text(&store, view), fold_document());
    assert_eq!(only_fold(&store, view), inner);
    let broken = press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![key(Key::Enter, false)],
    );
    assert!(folds(&store, view).is_empty());
    assert_eq!(body_rows(&broken.shown).len(), FOLD_LINES.len() + 1);
    press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![key(Key::Z, true)],
    );
    assert_eq!(text(&store, view), fold_document());
    assert!(folds(&store, view).is_empty());

    hide(&mut store, view, vec![inner.clone()]);
    place_caret(&mut store, view, fold_document().len());
    press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![Event::Text("y".into())],
    );
    assert_eq!(only_fold(&store, view), inner);
    press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![key(Key::Z, true)],
    );
    assert_eq!(folds(&store, view), [inner]);
}

#[test]
fn 접기_옵션은_대형과_읽기_전용_크기의_파일에서_꺼지고_설정만으로는_켜지지_않는다() {
    for (tier, is_foldable) in [
        (FileSizeTier::Normal, true),
        (FileSizeTier::Large, false),
        (FileSizeTier::ReadOnly, false),
        (FileSizeTier::Refused, true),
    ] {
        assert_eq!(editor_folding(tier), is_foldable, "{tier:?}");
    }
    assert!(!editor_presentation(&Settings::default()).options.folding);
}

#[test]
fn 화면_위쪽의_접힘이_바뀌어도_맨_위에_보이던_줄은_그_자리에_남고_그_줄이_숨겨지면_스크롤을_옮기지_않는다()
 {
    let document = long_fold_document(FOLD_ABOVE_HEADER, FOLD_ABOVE_LAST);
    let hidden = lines_fold(&document, FOLD_ABOVE_HEADER + 1..=FOLD_ABOVE_LAST);
    let hidden_height = (FOLD_ABOVE_LAST - FOLD_ABOVE_HEADER) as f32 * LINE_HEIGHT;
    let (mut store, view) = fixture(&document, false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    let shown = |store: &mut EditorStore| {
        show_folding(&context, store, view, FoldFrame::idle(&presentation));
        store.views().get(view).unwrap().scroll.y
    };
    let scrolled = FOLD_SCROLLED_LINE as f32 * LINE_HEIGHT + STABLE_DELTA;
    scroll_to(&mut store, view, scrolled);
    assert_eq!(shown(&mut store), scrolled);
    hide(&mut store, view, vec![hidden.clone()]);
    assert_eq!(shown(&mut store), scrolled - hidden_height);
    hide(&mut store, view, Vec::new());
    assert_eq!(shown(&mut store), scrolled);
    let inside = FOLD_SCROLLED_HIDDEN_LINE as f32 * LINE_HEIGHT + STABLE_DELTA;
    scroll_to(&mut store, view, inside);
    assert_eq!(shown(&mut store), inside);
    hide(&mut store, view, vec![hidden]);
    assert_eq!(shown(&mut store), inside);
}

#[test]
fn 접힌_상태의_토큰과_장식과_현재_줄_강조와_ime_좌표와_드래그_선택은_숨김_줄을_뺀_표시_줄에_놓인다()
{
    let (mut store, view) = fixture(&fold_document(), false);
    let context = Context::default();
    let presentation = folding_presentation(false);
    let inner = hidden_fold(INNER_HIDDEN);
    hide(&mut store, view, vec![inner.clone()]);
    place_caret(&mut store, view, fold_line_start(FOLD_SHOWN_LINE));
    show_folding(&context, &mut store, view, FoldFrame::idle(&presentation));
    let styles = token_styles();
    let lines = line_tokens(&[
        Vec::new(),
        Vec::new(),
        spans(&[(0, KEYWORD_STYLE_ID)]),
        spans(&[(0, STRING_STYLE_ID)]),
        spans(&[(0, COMMENT_STYLE_ID)]),
        Vec::new(),
        Vec::new(),
    ]);
    let hidden_start = fold_line_start(*INNER_HIDDEN.start());
    let shown_start = fold_line_start(FOLD_SHOWN_LINE);
    let current = revision(&store, view);
    let marks = layer(
        current,
        FIND_Z_ORDER,
        Stickiness::default(),
        vec![
            (
                hidden_start..hidden_start + FOLD_HIDDEN_MARK_BYTES,
                range_background(DECORATION_SELECTION_MATCH),
            ),
            (
                shown_start + FOLD_SHOWN_MARK.start..shown_start + FOLD_SHOWN_MARK.end,
                range_background(DECORATION_FIND),
            ),
        ],
    );
    let folded = show_folding(
        &context,
        &mut store,
        view,
        FoldFrame {
            tokens: Some(EditorTokens {
                revision: current,
                lines: &lines,
                styles: &styles,
            }),
            decorations: &[&marks],
            ..FoldFrame::idle(&presentation)
        },
    );
    assert_eq!(only_fold(&store, view), inner);
    let measuring = Context::default();
    let viewport = folded.shown.viewport;
    let left = viewport.left() + fold_text_left(&measuring);
    let row_top = |row: usize| viewport.top() + row as f32 * LINE_HEIGHT;
    let x = |line: usize, chars: usize| left + caret_offset(&measuring, FOLD_LINES[line], chars);
    let colored = |line: usize, foreground: [u8; 4]| {
        vec![(FOLD_LINES[line].to_owned(), token_color(foreground))]
    };
    assert_eq!(
        colored_rows(&folded.shown),
        [
            colored(0, TOKEN_DEFAULT),
            colored(INNER_HEADER, TOKEN_DEFAULT),
            vec![(
                FOLD_PLACEHOLDER.to_owned(),
                Color32::from_gray(FOLD_PLACEHOLDER_GRAY)
            )],
            colored(3, TOKEN_STRING),
            colored(FOLD_SHOWN_LINE, TOKEN_COMMENT),
            colored(5, TOKEN_DEFAULT),
            colored(6, TOKEN_DEFAULT),
        ]
    );
    assert_eq!(
        filled(&folded.shown.shapes, token_color(DECORATION_FIND)),
        [Rect::from_min_max(
            pos2(
                x(FOLD_SHOWN_LINE, FOLD_SHOWN_MARK.start),
                row_top(FOLD_SHOWN_ROW)
            ),
            pos2(
                x(FOLD_SHOWN_LINE, FOLD_SHOWN_MARK.end),
                row_top(FOLD_SHOWN_ROW + 1)
            )
        )]
    );
    assert!(
        filled(
            &folded.shown.shapes,
            token_color(DECORATION_SELECTION_MATCH)
        )
        .is_empty()
    );
    assert_eq!(
        filled(&folded.shown.shapes, Color32::DARK_GRAY),
        [Rect::from_min_max(
            pos2(viewport.left(), row_top(FOLD_SHOWN_ROW)),
            pos2(viewport.right(), row_top(FOLD_SHOWN_ROW + 1))
        )]
    );
    assert_caret(
        &folded.shown,
        x(FOLD_SHOWN_LINE, 0),
        row_top(FOLD_SHOWN_ROW),
    );
    let cursor = folded
        .shown
        .ime_cursor
        .expect("focused editor reports its caret");
    assert_eq!(cursor.left(), x(FOLD_SHOWN_LINE, 0));
    assert_eq!(
        cursor.top() - row_top(FOLD_SHOWN_ROW),
        row_top(FOLD_SHOWN_ROW + 1) - cursor.bottom()
    );
    let row_center = |row: usize| row_top(row) + LINE_HEIGHT / CENTER_DIVISOR;
    let from = pos2(
        x(0, FOLD_DRAG_FROM_COLUMN) + FOLD_CLICK_INSIDE,
        row_center(0),
    );
    let to = pos2(
        x(FOLD_DRAG_TO_LINE, FOLD_DRAG_TO_COLUMN) + FOLD_CLICK_INSIDE,
        row_center(FOLD_DRAG_TO_ROW),
    );
    for events in [
        vec![Event::PointerMoved(from), press(from, true)],
        vec![press(from, false)],
        vec![press(from, true)],
        vec![Event::PointerMoved(to)],
    ] {
        press_keys(&context, &mut store, view, &presentation, events);
    }
    let dragged = press_keys(
        &context,
        &mut store,
        view,
        &presentation,
        vec![press(to, false)],
    );
    assert_eq!(
        selection(&store, view),
        (
            FOLD_DRAG_FROM_COLUMN,
            fold_line_start(FOLD_DRAG_TO_LINE) + FOLD_DRAG_TO_COLUMN
        )
    );
    assert_eq!(only_fold(&store, view), inner);
    assert_eq!(
        filled(&dragged.shown.shapes, Color32::BLUE),
        [
            Rect::from_min_max(
                pos2(x(0, FOLD_DRAG_FROM_COLUMN), row_top(0)),
                pos2(viewport.right(), row_top(1))
            ),
            Rect::from_min_max(
                pos2(left, row_top(INNER_HEADER)),
                pos2(viewport.right(), row_top(INNER_HEADER + 1))
            ),
            Rect::from_min_max(
                pos2(left, row_top(FOLD_DRAG_TO_ROW)),
                pos2(
                    x(FOLD_DRAG_TO_LINE, FOLD_DRAG_TO_COLUMN),
                    row_top(FOLD_DRAG_TO_ROW + 1)
                )
            ),
        ]
    );
}

const BRACE_BRACKETS: [char; 4] = ['{', '}', '(', ')'];
const BRACE_AUTO_CLOSE_BEFORE: &str = ";:.,=}])> \n\t";
const REGION_START: &str = "#region";
const REGION_END: &str = "#endregion";

struct BraceRules {
    pairs: CharacterPairs,
}

fn brace_rules() -> BraceRules {
    let pair = |open: &str, close: &str| BracketPair {
        open: open.into(),
        close: close.into(),
    };
    BraceRules {
        pairs: CharacterPairs {
            brackets: vec![pair("{", "}"), pair("(", ")")],
            auto_closing_pairs: [("{", "}"), ("(", ")"), ("\"", "\"")]
                .map(|(open, close)| AutoClosingPair {
                    open: open.into(),
                    close: close.into(),
                    excluded_tokens: Vec::new(),
                })
                .into(),
            surrounding_pairs: vec![pair("(", ")")],
            auto_close_before_quotes: BRACE_AUTO_CLOSE_BEFORE.into(),
            auto_close_before_brackets: BRACE_AUTO_CLOSE_BEFORE.into(),
            block_comment_start: None,
        },
    }
}

impl LanguageRules for BraceRules {
    fn pairs(&self) -> &CharacterPairs {
        &self.pairs
    }

    fn enter_action(&self, _: &str, before_enter: &str, after_enter: &str) -> Option<EnterAction> {
        let indent_action = if after_enter.trim_start().starts_with('}') {
            IndentAction::IndentOutdent
        } else {
            IndentAction::Indent
        };
        before_enter
            .trim_end()
            .ends_with('{')
            .then_some(EnterAction {
                indent_action,
                append_text: None,
                remove_text: None,
            })
    }

    fn indent_metadata(&self, _: &str) -> Option<IndentMetadata> {
        None
    }

    fn without_brackets(&self, text: &str) -> String {
        text.replace(BRACE_BRACKETS, "")
    }

    fn bracket_ranges(&self, line: &str) -> Vec<std::ops::Range<usize>> {
        line.match_indices(BRACE_BRACKETS)
            .map(|(start, bracket)| start..start + bracket.len())
            .collect()
    }

    fn last_bracket(&self, text: &str) -> Option<std::ops::Range<usize>> {
        text.rmatch_indices(BRACE_BRACKETS)
            .next()
            .map(|(start, bracket)| start..start + bracket.len())
    }

    fn is_off_side(&self) -> bool {
        false
    }

    fn fold_marker(&self, line: &str) -> Option<FoldMarker> {
        if line.starts_with(REGION_START) {
            Some(FoldMarker::Start)
        } else {
            line.starts_with(REGION_END).then_some(FoldMarker::End)
        }
    }

    fn starts_marker_region(&self, line: &str) -> bool {
        line.starts_with(REGION_START)
    }
}

fn type_in_language(
    context: &Context,
    store: &mut EditorStore,
    view: ViewId,
    rules: &BraceRules,
    events: Vec<Event>,
) -> (String, (usize, usize)) {
    let presentation = folding_presentation(false);
    show_folding(
        context,
        store,
        view,
        FoldFrame {
            events,
            language: Some(Language {
                rules,
                syntax: &UntokenizedLines,
            }),
            ..FoldFrame::idle(&presentation)
        },
    );
    (text(store, view), selection(store, view))
}

#[test]
fn 언어_구성이_있으면_입력이_괄호_쌍과_enter_들여쓰기를_따른다() {
    let rules = brace_rules();
    let context = Context::default();
    let (mut store, view) = fixture("run", false);
    place_caret(&mut store, view, 3);
    let mut press =
        |events: Vec<Event>| type_in_language(&context, &mut store, view, &rules, events);
    press(Vec::new());
    assert_eq!(
        press(vec![Event::Text("{".into())]),
        ("run{}".into(), (4, 4))
    );
    assert_eq!(
        press(vec![key(Key::Enter, false)]),
        ("run{\n    \n}".into(), (9, 9))
    );
    assert_eq!(
        press(vec![Event::Text("(".into()), Event::Text(")".into())]),
        ("run{\n    ()\n}".into(), (11, 11))
    );
    assert_eq!(
        press(vec![Event::Text("(".into())]),
        ("run{\n    ()()\n}".into(), (12, 12))
    );
    assert_eq!(
        press(vec![key(Key::Backspace, false)]),
        ("run{\n    ()\n}".into(), (11, 11))
    );
    assert_eq!(
        press(vec![Event::Text("}".into())]),
        ("run{\n    ()}\n}".into(), (12, 12))
    );
}

#[test]
fn 캐럿이_자동으로_닫은_짝을_벗어나면_다음_프레임에도_덮어쓰지_않는다() {
    let rules = brace_rules();
    let context = Context::default();
    let (mut store, view) = fixture("run", false);
    place_caret(&mut store, view, 3);
    let mut press =
        |events: Vec<Event>| type_in_language(&context, &mut store, view, &rules, events);
    press(Vec::new());
    assert_eq!(
        press(vec![Event::Text("(".into())]),
        ("run()".into(), (4, 4))
    );
    assert_eq!(
        press(vec![
            key(Key::ArrowLeft, false),
            key(Key::ArrowRight, false),
            Event::Text(")".into())
        ]),
        ("run())".into(), (5, 5))
    );
}

#[test]
fn 공백뿐인_줄의_닫는_괄호_입력은_여는_줄의_들여쓰기로_맞춘다() {
    let rules = brace_rules();
    let context = Context::default();
    let (mut store, view) = fixture("run {\n    work\n    ", false);
    place_caret(&mut store, view, 19);
    let mut press =
        |events: Vec<Event>| type_in_language(&context, &mut store, view, &rules, events);
    press(Vec::new());
    assert_eq!(
        press(vec![Event::Text("}".into())]),
        ("run {\n    work\n}".into(), (16, 16))
    );
}

#[test]
fn 조합이_끝난_한_글자도_짝을_닫고_언어_구성이_없으면_그대로_넣는다() {
    let rules = brace_rules();
    let context = Context::default();
    let (mut store, view) = fixture("x = ", false);
    place_caret(&mut store, view, 4);
    let composed = |text: &str| {
        vec![
            Event::Ime(ImeEvent::Preedit {
                text: text.into(),
                active_range_chars: None,
            }),
            Event::Ime(ImeEvent::Commit(text.into())),
        ]
    };
    type_in_language(&context, &mut store, view, &rules, Vec::new());
    assert_eq!(
        type_in_language(&context, &mut store, view, &rules, composed("\"")),
        ("x = \"\"".into(), (5, 5))
    );
    assert_eq!(
        type_in_language(&context, &mut store, view, &rules, composed("한")),
        ("x = \"한\"".into(), (8, 8))
    );

    let (mut plain, plain_view) = fixture("x = ", false);
    place_caret(&mut plain, plain_view, 4);
    let presentation = folding_presentation(false);
    press_keys(&context, &mut plain, plain_view, &presentation, Vec::new());
    press_keys(
        &context,
        &mut plain,
        plain_view,
        &presentation,
        [
            composed("\""),
            vec![Event::Text("{".into()), key(Key::Enter, false)],
        ]
        .concat(),
    );
    assert_eq!(text(&plain, plain_view), "x = \"{\n");
}

#[test]
fn 다중_커서의_자동_닫기와_한글자_조합과_enter는_언어_규칙을_함께_적용한다() {
    let rules = brace_rules();
    for events in [
        vec![Event::Text("{".into())],
        vec![
            Event::Ime(ImeEvent::Preedit {
                text: "{".into(),
                active_range_chars: None,
            }),
            Event::Ime(ImeEvent::Commit("{".into())),
        ],
    ] {
        let (mut store, view) = fixture("a\nb", false);
        let current = store.views().get(view).unwrap().clone();
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![
                        Selection { anchor: 1, head: 1 },
                        Selection { anchor: 3, head: 3 },
                    ],
                },
                current.scroll,
                current.folds,
            )
            .unwrap();
        let context = Context::default();
        type_in_language(&context, &mut store, view, &rules, Vec::new());
        type_in_language(&context, &mut store, view, &rules, events);
        assert_eq!(text(&store, view), "a{}\nb{}");
        type_in_language(
            &context,
            &mut store,
            view,
            &rules,
            vec![key(Key::Enter, false)],
        );
        assert_eq!(text(&store, view), "a{\n    \n}\nb{\n    \n}");
        assert_eq!(
            store.views().get(view).unwrap().selection.selections.len(),
            2
        );
    }
}

#[test]
fn 표식_접기_명령은_언어_구성의_시작_표식_영역을_접는다() {
    let rules = brace_rules();
    let context = Context::default();
    let content = "#region a\nx\n#endregion\nrun {\n    work\n}";
    let (mut store, view) = fixture(content, false);
    place_caret(&mut store, view, 0);
    let presentation = folding_presentation(false);
    let mut run = |commands: &[FoldCommand], language: Option<Language<'_>>| {
        show_folding(
            &context,
            &mut store,
            view,
            FoldFrame {
                commands,
                language,
                ..FoldFrame::idle(&presentation)
            },
        );
        folds(&store, view)
    };
    let language = Language {
        rules: &rules,
        syntax: &UntokenizedLines,
    };
    let unfolded = Vec::<std::ops::Range<usize>>::new();
    assert_eq!(run(&[FoldCommand::FoldAllMarkerRegions], None), unfolded);
    let marker_fold = content.find("x\n").unwrap()..content.find("\nrun").unwrap();
    assert_eq!(
        run(&[FoldCommand::FoldAllMarkerRegions], Some(language)),
        [marker_fold]
    );
    assert_eq!(
        run(&[FoldCommand::UnfoldAllMarkerRegions], Some(language)),
        unfolded
    );
}
