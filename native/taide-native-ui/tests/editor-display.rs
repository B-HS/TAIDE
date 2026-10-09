#![cfg(feature = "native-host")]

use egui::epaint::{ClippedShape, Shape};
use egui::{
    Color32, Context, Event, FontId, ImeEvent, Modifiers, MouseWheelUnit, RawInput, Rect,
    TouchPhase, Vec2, pos2, vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_model::settings::{
    EditorCursorBlinking, EditorCursorStyle, EditorRenderWhitespace, Settings,
};
use taide_native_editor::language_configuration::{
    BracketPair, CharacterPairs, EnterAction, FoldMarker, IndentMetadata, Language, LanguageRules,
    UntokenizedLines,
};
use taide_native_editor::line_tokens::{LineTokens, TokenStyle, TokenStyleTable};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_brackets::EditorBracketColors;
use taide_native_ui::editor_display::EditorDisplayColors;
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_native_ui::editor_surface::{
    CursorBlinking, CursorStyle, EditorAppearance, EditorDisplayOptions, EditorPresentation,
    EditorRequest, EditorTokens, NativeEditor, RenderWhitespace,
};
use taide_native_ui::presentation::editor_presentation;

const SCREEN: Vec2 = vec2(400.0, 120.0);
const LINE_HEIGHT: f32 = 20.0;
const FONT_SIZE: f32 = 14.0;
const BYTE_LIMIT: usize = 1024 * 1024;
const HISTORY_LIMIT: usize = 8;
const WHITESPACE: Color32 = Color32::from_rgb(23, 44, 66);
const RULER: Color32 = Color32::from_rgb(67, 88, 99);
const CURSOR: Color32 = Color32::from_rgb(99, 11, 55);
const WHEEL: Vec2 = vec2(0.0, -80.0);
const EPSILON: f32 = 0.001;

fn fixture(text: &str, read_only: bool) -> (Context, NativeEditor, EditorStore, ViewId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 2,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            "/synthetic/display.txt".into(),
            OpenedFile {
                path: "/synthetic/display.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
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
                window: "display-test".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let editor = NativeEditor {
        appearance: EditorAppearance {
            font: FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: 8.0,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            muted: Color32::GRAY,
            selection: Color32::BLUE,
            cursor: CURSOR,
            current_line: Color32::DARK_GRAY,
            line_numbers: false,
            indent: "    ".into(),
        },
    };
    (Context::default(), editor, store, view)
}

fn options() -> EditorPresentation {
    EditorPresentation {
        options: EditorDisplayOptions {
            cursor_style: CursorStyle::Line,
            cursor_blinking: CursorBlinking::Solid,
            colors: Some(EditorDisplayColors {
                whitespace: WHITESPACE,
                ruler: RULER,
                scrollbar: Color32::GRAY,
                scrollbar_hover: Color32::LIGHT_GRAY,
            }),
            ..Default::default()
        },
    }
}

struct Frame {
    time: f64,
    wheel: Vec2,
    focus: bool,
    events: Vec<Event>,
    reveal: Option<(f64, f64)>,
}

impl Frame {
    fn at(time: f64) -> Self {
        Self {
            time,
            wheel: Vec2::ZERO,
            focus: true,
            events: Vec::new(),
            reveal: None,
        }
    }
}

struct Shown {
    shapes: Vec<ClippedShape>,
    geometry: EditorGeometry,
}

fn show(
    context: &Context,
    editor: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    frame: Frame,
) -> Shown {
    let mut geometry = None;
    let mut events = vec![Event::PointerMoved(pos2(100.0, 50.0))];
    events.extend(frame.events);
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
            time: Some(frame.time),
            events,
            ..Default::default()
        },
        |ui| {
            if !frame.focus {
                ui.memory_mut(|memory| {
                    memory.surrender_focus(ui.make_persistent_id(("native-code-editor", view)))
                });
            }
            if frame.wheel != Vec2::ZERO {
                ui.input_mut(|input| input.smooth_scroll_delta = frame.wheel);
            }
            if let Some((line, column)) = frame.reveal {
                editor
                    .reveal_presented(ui, store, view, line, column, presentation)
                    .unwrap();
            }
            let shown = editor
                .show_presented(
                    ui,
                    store,
                    view,
                    frame.focus,
                    |_, _, _| false,
                    |_| None,
                    presentation,
                )
                .unwrap();
            assert!(shown.errors.is_empty());
            geometry = Some(shown.geometry);
        },
    );
    output.textures_delta.clear();
    Shown {
        shapes: output.shapes,
        geometry: geometry.unwrap(),
    }
}

fn select(store: &mut EditorStore, view: ViewId, selections: Vec<Selection>) {
    let state = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections,
            },
            state.scroll,
            state.folds,
        )
        .unwrap();
}

fn cursor(shown: &Shown) -> Vec<Rect> {
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Rect(rect)
                if rect.fill.to_srgba_unmultiplied()[..3]
                    == CURSOR.to_srgba_unmultiplied()[..3] =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect()
}

fn markers(shown: &Shown) -> Vec<(egui::Pos2, f32, Rect)> {
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Circle(circle) if circle.fill == WHITESPACE => {
                Some((circle.center, circle.radius, shape.clip_rect))
            }
            _ => None,
        })
        .collect()
}

fn close(left: f32, right: f32) {
    assert!((left - right).abs() < EPSILON, "{left} != {right}");
}

fn font_width(context: &Context, text: &str) -> f32 {
    context.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap(text.into(), FontId::monospace(FONT_SIZE), Color32::WHITE)
            .size()
            .x
    })
}

#[test]
fn 표시_설정은_기본값과_모든_선택지를_native_presentation에_공급한다() {
    let mut settings = Settings::default();
    let defaults = editor_presentation(&settings).options;
    assert_eq!(defaults.render_whitespace, RenderWhitespace::Selection);
    assert_eq!(defaults.cursor_style, CursorStyle::Line);
    assert_eq!(defaults.cursor_blinking, CursorBlinking::Blink);
    assert!(defaults.scroll_beyond_last_line);
    assert!(!defaults.smooth_scrolling && !defaults.smooth_caret);
    assert!(defaults.bracket_pair_colorization);
    assert!(!defaults.bracket_pair_guides);
    for (source, target) in [
        (EditorRenderWhitespace::None, RenderWhitespace::None),
        (EditorRenderWhitespace::Boundary, RenderWhitespace::Boundary),
        (EditorRenderWhitespace::All, RenderWhitespace::All),
    ] {
        settings.editor_render_whitespace = source;
        assert_eq!(
            editor_presentation(&settings).options.render_whitespace,
            target
        );
    }
    for (source, target) in [
        (EditorCursorStyle::Block, CursorStyle::Block),
        (EditorCursorStyle::Underline, CursorStyle::Underline),
    ] {
        settings.editor_cursor_style = source;
        assert_eq!(editor_presentation(&settings).options.cursor_style, target);
    }
    for (source, target) in [
        (EditorCursorBlinking::Smooth, CursorBlinking::Smooth),
        (EditorCursorBlinking::Phase, CursorBlinking::Phase),
        (EditorCursorBlinking::Expand, CursorBlinking::Expand),
        (EditorCursorBlinking::Solid, CursorBlinking::Solid),
    ] {
        settings.editor_cursor_blinking = source;
        assert_eq!(
            editor_presentation(&settings).options.cursor_blinking,
            target
        );
    }
    settings.editor_rulers = vec![80, 120];
    settings.editor_smooth_scrolling = true;
    settings.editor_cursor_smooth_caret_animation = true;
    settings.editor_scroll_beyond_last_line = false;
    let changed = editor_presentation(&settings).options;
    assert_eq!(changed.rulers, settings.editor_rulers);
    assert!(changed.smooth_scrolling && changed.smooth_caret);
    assert!(!changed.scroll_beyond_last_line);
}

#[test]
fn 공백_모드는_탭과_unicode의_실제_좌표와_끝_제외_선택을_보존한다() {
    let source = " a b  한\t😀 ";
    let (context, editor, mut store, view) = fixture(source, false);
    let mut presentation = options();
    presentation.options.render_whitespace = RenderWhitespace::All;
    let all = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let marks = markers(&all);
    assert_eq!(marks.len(), 5);
    let space_width = font_width(&context, " ");
    for ((center, radius, clip), byte) in marks.iter().zip([0, 2, 4, 5, source.len() - 1]) {
        close(
            center.x,
            all.geometry.caret_rect(byte).unwrap().left() + space_width / 2.0,
        );
        close(*radius, space_width / 7.0);
        assert_eq!(*clip, all.geometry.content_rect);
    }
    assert_eq!(all.shapes.iter().filter(|shape| matches!(&shape.shape, Shape::Mesh(mesh) if mesh.vertices.iter().all(|vertex| vertex.color == WHITESPACE))).count(), 1);
    presentation.options.render_whitespace = RenderWhitespace::Boundary;
    assert_eq!(
        markers(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.1)
        ))
        .len(),
        4
    );
    presentation.options.render_whitespace = RenderWhitespace::Selection;
    select(&mut store, view, vec![Selection { anchor: 6, head: 4 }]);
    assert_eq!(
        markers(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.2)
        ))
        .len(),
        2
    );
    presentation.options.render_whitespace = RenderWhitespace::None;
    assert!(
        markers(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.3)
        ))
        .is_empty()
    );
}

#[test]
fn rulers는_문자_열과_스크롤에_맞춰_1px로_그려지고_옵션_변경으로_회수된다() {
    let (context, editor, mut store, view) = fixture(&"x".repeat(160), false);
    let mut presentation = options();
    presentation.options.rulers = vec![20, 80];
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let width = font_width(&context, "n");
    let mut frame = Frame::at(0.1);
    frame.wheel = vec2(-40.0, 0.0);
    let scrolled = show(&context, &editor, &mut store, view, &presentation, frame);
    let lines: Vec<_> = scrolled
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Rect(rect) if rect.fill == RULER => Some((rect.rect, shape.clip_rect)),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 2);
    for ((rect, clip), column) in lines.iter().zip([20.0, 80.0]) {
        close(rect.width(), 1.0);
        close(
            rect.left(),
            scrolled.geometry.content_rect.left() + column * width - scrolled.geometry.scroll.x,
        );
        assert_eq!(*clip, scrolled.geometry.content_rect);
    }
    presentation.options.rulers.clear();
    assert!(
        !show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.2)
        )
        .shapes
        .iter()
        .any(|shape| matches!(&shape.shape, Shape::Rect(rect) if rect.fill == RULER))
    );
}

#[test]
fn 캐럿은_스타일과_grapheme_탭_줄끝_선택_머리를_반영한다() {
    let (context, editor, mut store, view) = fixture("e\u{301}\t😀", false);
    let mut presentation = options();
    let line = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    close(cursor(&line)[0].width(), 2.0);
    close(cursor(&line)[0].height(), LINE_HEIGHT);
    presentation.options.cursor_style = CursorStyle::Block;
    let block = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.1),
    );
    close(
        cursor(&block)[0].width(),
        block.geometry.caret_rect(3).unwrap().left() - block.geometry.caret_rect(0).unwrap().left(),
    );
    select(&mut store, view, vec![Selection { anchor: 0, head: 3 }]);
    let tab = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.2),
    );
    close(cursor(&tab)[0].width(), font_width(&context, "n"));
    select(&mut store, view, vec![Selection { anchor: 4, head: 4 }]);
    let emoji = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.3),
    );
    close(
        cursor(&emoji)[0].width(),
        emoji.geometry.caret_rect(8).unwrap().left() - emoji.geometry.caret_rect(4).unwrap().left(),
    );
    presentation.options.cursor_style = CursorStyle::Underline;
    let underline = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.4),
    );
    close(cursor(&underline)[0].height(), 2.0);
    close(cursor(&underline)[0].bottom(), LINE_HEIGHT);
    select(&mut store, view, vec![Selection { anchor: 8, head: 8 }]);
    let end = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.5),
    );
    close(cursor(&end)[0].width(), font_width(&context, "n"));
}

#[test]
fn 깜빡임은_이동으로_재시작하고_읽기_전용은_고정하며_포커스와_ime에서_숨긴다() {
    let (context, editor, mut store, view) = fixture("abc", false);
    let mut presentation = options();
    presentation.options.cursor_blinking = CursorBlinking::Blink;
    assert_eq!(
        cursor(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.0)
        ))
        .len(),
        1
    );
    assert!(
        cursor(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.6)
        ))
        .is_empty()
    );
    select(&mut store, view, vec![Selection { anchor: 1, head: 1 }]);
    assert_eq!(
        cursor(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.7)
        ))
        .len(),
        1
    );
    let mut blurred = Frame::at(0.8);
    blurred.focus = false;
    assert!(
        cursor(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            blurred
        ))
        .is_empty()
    );
    let mut ime = Frame::at(0.9);
    ime.events = vec![Event::Ime(ImeEvent::Preedit {
        text: "한".into(),
        active_range_chars: None,
    })];
    assert!(
        cursor(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            ime
        ))
        .is_empty()
    );
    let (context, editor, mut store, view) = fixture("abc", true);
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    assert_eq!(
        cursor(&show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.6)
        ))
        .len(),
        1
    );
}

#[test]
fn smooth_caret는_80ms에_도착하고_다중_커서_개수_변경을_즉시_반영한다() {
    let (context, editor, mut store, view) = fixture("abcdef", false);
    let mut presentation = options();
    presentation.options.smooth_caret = true;
    let first = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    select(&mut store, view, vec![Selection { anchor: 4, head: 4 }]);
    let started = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.1),
    );
    close(cursor(&started)[0].left(), cursor(&first)[0].left());
    let middle = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.14),
    );
    let ended = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.19),
    );
    assert!(
        cursor(&middle)[0].left() > cursor(&first)[0].left()
            && cursor(&middle)[0].left() < cursor(&ended)[0].left()
    );
    close(
        cursor(&ended)[0].left(),
        ended.geometry.caret_rect(4).unwrap().left() - 1.0,
    );
    select(
        &mut store,
        view,
        vec![
            Selection { anchor: 1, head: 1 },
            Selection { anchor: 5, head: 5 },
        ],
    );
    let multiple = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.2),
    );
    assert_eq!(cursor(&multiple).len(), 2);
    close(
        cursor(&multiple)[0].left(),
        multiple.geometry.caret_rect(1).unwrap().left() - 1.0,
    );
}

#[test]
fn 마지막_줄_이후_여백과_옵션_변경은_저장한_스크롤과_표시_좌표에_반영된다() {
    let source = (0..10).map(|_| "row").collect::<Vec<_>>().join("\n");
    let (context, editor, mut store, view) = fixture(&source, false);
    let mut presentation = options();
    presentation.options.scroll_beyond_last_line = true;
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let mut wheel = Frame::at(0.1);
    wheel.wheel = vec2(0.0, -1000.0);
    let bottom = show(&context, &editor, &mut store, view, &presentation, wheel);
    close(bottom.geometry.scroll.y, 9.0 * LINE_HEIGHT);
    close(
        bottom.geometry.caret_rect(source.len()).unwrap().top(),
        bottom.geometry.rect.top(),
    );
    presentation.options.scroll_beyond_last_line = false;
    let clamped = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.2),
    );
    close(
        clamped.geometry.scroll.y,
        10.0 * LINE_HEIGHT - clamped.geometry.rect.height(),
    );
    close(
        store.views().get(view).unwrap().scroll.y,
        clamped.geometry.scroll.y,
    );
}

#[test]
fn smooth_scrolling은_연속_휠의_목표를_누적하고_끄면_즉시_도착한다() {
    let source = "row\n".repeat(100);
    let (context, editor, mut store, view) = fixture(&source, false);
    let mut presentation = options();
    presentation.options.smooth_scrolling = true;
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let mut frame = Frame::at(0.1);
    frame.wheel = WHEEL;
    let first = show(&context, &editor, &mut store, view, &presentation, frame);
    assert!(first.geometry.scroll.y > 0.0 && first.geometry.scroll.y < -WHEEL.y);
    let mut frame = Frame::at(0.15);
    frame.wheel = WHEEL;
    let second = show(&context, &editor, &mut store, view, &presentation, frame);
    assert!(
        second.geometry.scroll.y > first.geometry.scroll.y
            && second.geometry.scroll.y < -WHEEL.y * 2.0
    );
    let ended = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.3),
    );
    close(ended.geometry.scroll.y, -WHEEL.y * 2.0);
    let mut frame = Frame::at(0.4);
    frame.wheel = WHEEL;
    show(&context, &editor, &mut store, view, &presentation, frame);
    presentation.options.smooth_scrolling = false;
    let instant = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.41),
    );
    close(instant.geometry.scroll.y, -WHEEL.y * 3.0);
}

#[test]
fn 실제_휠은_보간을_끄면_즉시_소비되고_후속_프레임에_잔류하지_않는다() {
    const LINE_WHEEL_DISTANCE: f32 = 40.0;
    let (context, editor, mut store, view) = fixture(&"row\n".repeat(100), false);
    let presentation = options();
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let mut wheel = Frame::at(0.016);
    wheel.events = vec![Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, -1.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::default(),
    }];
    let immediate = show(&context, &editor, &mut store, view, &presentation, wheel);
    close(immediate.geometry.scroll.y, LINE_WHEEL_DISTANCE);
    let stable = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.032),
    );
    close(stable.geometry.scroll.y, LINE_WHEEL_DISTANCE);
}

#[test]
fn 부드러운_휠은_한_번만_보간하고_터치패드_입력은_지연_없이_반영한다() {
    const LINE_WHEEL_DISTANCE: f32 = 40.0;
    let (context, editor, mut store, view) = fixture(&"row\n".repeat(100), false);
    let mut presentation = options();
    presentation.options.smooth_scrolling = true;
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let mut wheel = Frame::at(0.016);
    wheel.events = vec![Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, -1.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::default(),
    }];
    let animated = show(&context, &editor, &mut store, view, &presentation, wheel);
    assert!(animated.geometry.scroll.y > 0.0 && animated.geometry.scroll.y < LINE_WHEEL_DISTANCE);
    close(
        show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.16),
        )
        .geometry
        .scroll
        .y,
        LINE_WHEEL_DISTANCE,
    );
    let mut touch = Frame::at(0.18);
    touch.events = vec![Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -20.0),
        phase: TouchPhase::Start,
        modifiers: Modifiers::default(),
    }];
    close(
        show(&context, &editor, &mut store, view, &presentation, touch)
            .geometry
            .scroll
            .y,
        LINE_WHEEL_DISTANCE + 20.0,
    );
    let mut touch = Frame::at(0.19);
    touch.events = vec![Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, -30.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::default(),
    }];
    close(
        show(&context, &editor, &mut store, view, &presentation, touch)
            .geometry
            .scroll
            .y,
        LINE_WHEEL_DISTANCE + 50.0,
    );
}

#[test]
fn css_캐럿은_지연과_왕복_반복_종료를_보존하고_line_폭을_dpr에_맞춘다() {
    for blinking in [
        CursorBlinking::Smooth,
        CursorBlinking::Phase,
        CursorBlinking::Expand,
    ] {
        let (context, editor, mut store, view) = fixture("abc", false);
        let mut presentation = options();
        presentation.options.cursor_blinking = blinking;
        show(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.0),
        );
        assert_eq!(
            cursor(&show(
                &context,
                &editor,
                &mut store,
                view,
                &presentation,
                Frame::at(0.49)
            ))
            .len(),
            1
        );
        assert!(
            cursor(&show(
                &context,
                &editor,
                &mut store,
                view,
                &presentation,
                Frame::at(0.99)
            ))
            .is_empty()
        );
        assert_eq!(
            cursor(&show(
                &context,
                &editor,
                &mut store,
                view,
                &presentation,
                Frame::at(1.49)
            ))
            .len(),
            1
        );
        assert_eq!(
            cursor(&show(
                &context,
                &editor,
                &mut store,
                view,
                &presentation,
                Frame::at(10.6)
            ))
            .len(),
            1
        );
    }
    let (context, editor, mut store, view) = fixture("abc", false);
    context.set_pixels_per_point(1.25);
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &options(),
        Frame::at(0.0),
    );
    close(cursor(&shown)[0].width(), 1.6);
}

#[test]
fn wrap의_가짜_들여쓰기와_접힌_줄은_공백_표식에서_제외한다() {
    let source = format!("\tlet message = {}", "x ".repeat(120));
    let (context, editor, mut store, view) = fixture(&source, false);
    let mut presentation = options();
    presentation.options.word_wrap = true;
    presentation.options.render_whitespace = RenderWhitespace::All;
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    assert!(shown.geometry.visible_rows.end > 1);
    let width = font_width(&context, " ");
    for (center, _, _) in markers(&shown) {
        if let Some(byte) = shown
            .geometry
            .byte_at(pos2(center.x - width / 2.0, center.y))
        {
            assert_eq!(source.as_bytes()[byte], b' ');
        }
    }
    let source = "head\n  hide \n  hide \nend";
    let (context, editor, mut store, view) = fixture(source, false);
    let state = store.views().get(view).unwrap().clone();
    store
        .set_view_state(
            view,
            state.selection,
            state.scroll,
            vec![5..source.find("end").unwrap()],
        )
        .unwrap();
    let mut presentation = options();
    presentation.options.folding = true;
    presentation.options.render_whitespace = RenderWhitespace::All;
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    assert!(markers(&shown).is_empty());
    assert_eq!(shown.geometry.visible_rows.end, 2);
}

#[test]
fn 드러내기는_장거리_스크롤을_보간하고_다른_뷰의_복원_좌표는_보존한다() {
    let (context, editor, mut store, view) = fixture(&"row\n".repeat(100), false);
    let document = store.views().get(view).unwrap().document;
    let other = store
        .attach_view(
            ViewKey {
                window: "other".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let mut presentation = options();
    presentation.options.smooth_scrolling = true;
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let mut frame = Frame::at(0.1);
    frame.reveal = Some((80.0, 1.0));
    let moving = show(&context, &editor, &mut store, view, &presentation, frame);
    let ended = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.3),
    );
    assert!(moving.geometry.scroll.y > 0.0 && moving.geometry.scroll.y < ended.geometry.scroll.y);
    close(
        ended.geometry.scroll.y,
        79.0 * LINE_HEIGHT + LINE_HEIGHT / 2.0 - ended.geometry.rect.height() / 2.0,
    );
    let other_shown = show(
        &context,
        &editor,
        &mut store,
        other,
        &presentation,
        Frame::at(0.4),
    );
    close(other_shown.geometry.scroll.y, 0.0);
    close(
        store.views().get(view).unwrap().scroll.y,
        ended.geometry.scroll.y,
    );
}

#[test]
fn 가로_스크롤바가_있고_아래_여백을_끄면_마지막_줄을_가리지_않는다() {
    const HORIZONTAL_BAR_HEIGHT: f32 = 12.0;
    let source = format!(
        "{}\n{}",
        "x".repeat(160),
        (0..10).map(|_| "row").collect::<Vec<_>>().join("\n")
    );
    let (context, editor, mut store, view) = fixture(&source, false);
    let presentation = options();
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let mut frame = Frame::at(0.1);
    frame.wheel = vec2(0.0, -1000.0);
    let shown = show(&context, &editor, &mut store, view, &presentation, frame);
    close(
        shown.geometry.scroll.y,
        11.0 * LINE_HEIGHT - shown.geometry.rect.height() + HORIZONTAL_BAR_HEIGHT,
    );
    close(
        shown.geometry.caret_rect(source.len()).unwrap().bottom(),
        shown.geometry.rect.bottom() - HORIZONTAL_BAR_HEIGHT,
    );
}

#[test]
fn 대형_문서를_스크롤해도_ruler는_현재_화면_높이를_계속_덮는다() {
    const LONG_DOCUMENT_LINES: usize = 100_000;
    let (context, editor, mut store, view) = fixture(&"row\n".repeat(LONG_DOCUMENT_LINES), false);
    let mut presentation = options();
    presentation.options.rulers = vec![20];
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let mut frame = Frame::at(0.1);
    frame.wheel = vec2(0.0, -1_600_000.0);
    let shown = show(&context, &editor, &mut store, view, &presentation, frame);
    let line = shown
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Rect(rect) if rect.fill == RULER => Some(rect.rect),
            _ => None,
        })
        .unwrap();
    assert!(
        line.top() <= shown.geometry.content_rect.top()
            && line.bottom() >= shown.geometry.content_rect.bottom()
    );
}

const BRACKET_PALETTE: [Color32; 3] = [
    Color32::from_rgb(183, 37, 46),
    Color32::from_rgb(24, 178, 77),
    Color32::from_rgb(149, 45, 189),
];
const UNEXPECTED_BRACKET: Color32 = Color32::from_rgb(246, 94, 12);
const INDENT_GUIDE: Color32 = Color32::from_rgb(44, 55, 66);
const ACTIVE_INDENT_GUIDE: Color32 = Color32::from_rgb(77, 88, 99);
const BRACKET_CHARACTERS: [char; 6] = ['{', '}', '[', ']', '(', ')'];
const INACTIVE_GUIDE_OPACITY: f32 = 0.3;

struct DisplayBracketRules {
    pairs: CharacterPairs,
}

fn display_bracket_rules() -> DisplayBracketRules {
    DisplayBracketRules {
        pairs: CharacterPairs {
            brackets: [("{", "}"), ("[", "]"), ("(", ")")]
                .into_iter()
                .map(|(open, close)| BracketPair {
                    open: open.into(),
                    close: close.into(),
                })
                .collect(),
            ..Default::default()
        },
    }
}

impl LanguageRules for DisplayBracketRules {
    fn pairs(&self) -> &CharacterPairs {
        &self.pairs
    }
    fn enter_action(&self, _: &str, _: &str, _: &str) -> Option<EnterAction> {
        None
    }
    fn indent_metadata(&self, _: &str) -> Option<IndentMetadata> {
        None
    }
    fn without_brackets(&self, text: &str) -> String {
        text.replace(BRACKET_CHARACTERS, "")
    }
    fn bracket_ranges(&self, text: &str) -> Vec<std::ops::Range<usize>> {
        text.match_indices(BRACKET_CHARACTERS)
            .map(|(start, value)| start..start + value.len())
            .collect()
    }
    fn last_bracket(&self, text: &str) -> Option<std::ops::Range<usize>> {
        self.bracket_ranges(text).into_iter().next_back()
    }
    fn is_off_side(&self) -> bool {
        false
    }
    fn fold_marker(&self, _: &str) -> Option<FoldMarker> {
        None
    }
    fn starts_marker_region(&self, _: &str) -> bool {
        false
    }
}

fn bracket_options() -> EditorPresentation {
    let mut presentation = options();
    presentation.options.bracket_pair_colorization = true;
    presentation.options.bracket_pair_guides = true;
    presentation.options.bracket_colors = Some(EditorBracketColors {
        palette: BRACKET_PALETTE,
        unexpected: UNEXPECTED_BRACKET,
        indent: INDENT_GUIDE,
        active_indent: ACTIVE_INDENT_GUIDE,
    });
    presentation
}

fn show_brackets(
    context: &Context,
    editor: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    tokens: Option<EditorTokens<'_>>,
) -> Shown {
    let rules = display_bracket_rules();
    let mut geometry = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
            time: Some(0.0),
            ..Default::default()
        },
        |ui| {
            let shown = editor
                .show_request(
                    ui,
                    store,
                    view,
                    EditorRequest {
                        request_focus: false,
                        keymap: |_: &egui::Ui, _: &Event, _| false,
                        route: |_: &egui::Response| None,
                        presentation,
                        tokens: |_: &EditorStore| tokens,
                        language: Some(Language {
                            rules: &rules,
                            syntax: &UntokenizedLines,
                        }),
                        decorations: &[],
                        fold_commands: &[],
                        fold_controls: None,
                    },
                )
                .unwrap();
            assert!(shown.errors.is_empty());
            geometry = Some(shown.geometry);
        },
    );
    output.textures_delta.clear();
    Shown {
        shapes: output.shapes,
        geometry: geometry.unwrap(),
    }
}

fn colored_brackets(shown: &Shown) -> Vec<(char, Color32)> {
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Text(text) => Some(text),
            _ => None,
        })
        .flat_map(|text| {
            text.galley
                .job
                .sections
                .iter()
                .flat_map(|section| {
                    text.galley.job.text[section.byte_range.start.0..section.byte_range.end.0]
                        .chars()
                        .filter(|character| BRACKET_CHARACTERS.contains(character))
                        .map(|character| (character, section.format.color))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn filled_guides(shown: &Shown, color: Color32) -> Vec<(Rect, Rect)> {
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Rect(rect) if rect.fill == color => Some((rect.rect, shape.clip_rect)),
            _ => None,
        })
        .collect()
}

fn display_caret(byte: usize) -> Selection {
    Selection {
        anchor: byte,
        head: byte,
    }
}

#[test]
fn 괄호_색상은_실제_글자에_중첩을_적용하고_문자열과_설정_테마_변경을_따른다() {
    let text = "{[()]} \"([])\"";
    let (context, editor, mut store, view) = fixture(text, false);
    let mut presentation = bracket_options();
    let default_style = TokenStyle {
        foreground: [255; 4],
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: TokenKind::Other,
    };
    let styles = TokenStyleTable::new(
        default_style,
        vec![
            default_style,
            TokenStyle {
                kind: TokenKind::String,
                ..default_style
            },
        ],
    );
    let mut lines = LineTokens::new(1);
    lines.set_line(0, vec![0, 0, 7, 1], false);
    let tokens = Some(EditorTokens {
        revision: 0,
        lines: &lines,
        styles: &styles,
    });
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, tokens);
    let colors: Vec<_> = colored_brackets(&shown)
        .into_iter()
        .map(|(_, color)| color)
        .collect();
    assert_eq!(
        colors,
        [
            BRACKET_PALETTE[0],
            BRACKET_PALETTE[1],
            BRACKET_PALETTE[2],
            BRACKET_PALETTE[2],
            BRACKET_PALETTE[1],
            BRACKET_PALETTE[0],
            Color32::WHITE,
            Color32::WHITE,
            Color32::WHITE,
            Color32::WHITE
        ]
    );
    presentation.options.bracket_pair_colorization = false;
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, tokens);
    assert!(
        colored_brackets(&shown)
            .iter()
            .all(|(_, color)| *color == Color32::WHITE)
    );
    presentation.options.bracket_pair_colorization = true;
    presentation
        .options
        .bracket_colors
        .as_mut()
        .unwrap()
        .palette[0] = Color32::YELLOW;
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, tokens);
    assert_eq!(colored_brackets(&shown)[0].1, Color32::YELLOW);
    assert_eq!(colored_brackets(&shown)[1].1, BRACKET_PALETTE[1]);
}

#[test]
fn 괄호_안내선은_안쪽_활성_짝을_표시하고_들여쓰기와_중복되지_않는다() {
    let text = "outer {\n    [\n        x\n    ]\n}";
    let (context, editor, mut store, view) = fixture(text, false);
    let presentation = bracket_options();
    let primary = text.find('x').unwrap();
    select(&mut store, view, vec![display_caret(primary)]);
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    let inner = filled_guides(&shown, BRACKET_PALETTE[1]);
    assert_eq!(inner.len(), 1);
    close(inner[0].0.width(), 1.0);
    close(inner[0].0.height(), LINE_HEIGHT);
    close(
        inner[0].0.left(),
        shown.geometry.content_rect.left() + font_width(&context, " ") * 4.0,
    );
    assert!(
        filled_guides(
            &shown,
            BRACKET_PALETTE[0].gamma_multiply(INACTIVE_GUIDE_OPACITY)
        )
        .len()
            >= 2
    );
    assert!(filled_guides(&shown, ACTIVE_INDENT_GUIDE).is_empty());
    assert!(
        filled_guides(&shown, INDENT_GUIDE)
            .iter()
            .all(|(rect, _)| (rect.left() - inner[0].0.left()).abs() >= EPSILON)
    );
    select(&mut store, view, vec![display_caret(0)]);
    let outside = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    assert!(filled_guides(&outside, BRACKET_PALETTE[1]).is_empty());
    assert!(
        filled_guides(
            &outside,
            BRACKET_PALETTE[1].gamma_multiply(INACTIVE_GUIDE_OPACITY)
        )
        .len()
            >= 1
    );
}

#[test]
fn 괄호_가로선은_실제_괄호_좌표와_닫는_줄의_내용에_맞고_텍스트_clip을_쓴다() {
    let text = "call {\n  x\n   y }";
    let (context, editor, mut store, view) = fixture(text, false);
    let presentation = bracket_options();
    select(
        &mut store,
        view,
        vec![display_caret(text.find('x').unwrap())],
    );
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    let horizontal: Vec<_> = filled_guides(&shown, BRACKET_PALETTE[0])
        .into_iter()
        .filter(|(rect, _)| rect.height() == 1.0)
        .collect();
    assert_eq!(horizontal.len(), 2);
    let start = shown.geometry.content_rect.left() + font_width(&context, " ") * 2.0;
    close(horizontal[0].0.left(), start);
    close(
        horizontal[0].0.right(),
        shown
            .geometry
            .caret_rect(text.find('{').unwrap())
            .unwrap()
            .left(),
    );
    close(
        horizontal[0].0.top(),
        shown.geometry.rect.top() + LINE_HEIGHT - 1.0,
    );
    close(
        horizontal[1].0.right(),
        shown
            .geometry
            .caret_rect(text.find('}').unwrap())
            .unwrap()
            .left(),
    );
    close(
        horizontal[1].0.top(),
        shown.geometry.rect.top() + LINE_HEIGHT * 3.0 - 1.0,
    );
    assert!(
        horizontal
            .iter()
            .all(|(_, clip)| *clip == shown.geometry.content_rect)
    );
    let vertical: Vec<_> = filled_guides(&shown, BRACKET_PALETTE[0])
        .into_iter()
        .filter(|(rect, _)| rect.width() == 1.0)
        .collect();
    assert_eq!(vertical.len(), 2);
}

#[test]
fn 괄호_한줄_가로선은_wrap의_실제_표시줄을_따르고_접힌_짝은_그리지_않는다() {
    let text = format!("({})", "x".repeat(100));
    let (context, editor, mut store, view) = fixture(&text, false);
    let mut presentation = bracket_options();
    presentation.options.word_wrap = true;
    select(&mut store, view, vec![display_caret(2)]);
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    let guides = filled_guides(&shown, BRACKET_PALETTE[0]);
    assert!(guides.len() >= 2);
    assert!(guides.iter().all(|(rect, clip)| rect.height() == 1.0
        && rect.width() > 1.0
        && *clip == shown.geometry.content_rect));
    assert_eq!(
        colored_brackets(&shown),
        [('(', BRACKET_PALETTE[0]), (')', BRACKET_PALETTE[0])]
    );

    let (context, editor, mut store, view) = fixture("{\n    [\n        x\n    ]\n}", false);
    let state = store.views().get(view).unwrap().clone();
    let document = store.documents().snapshot(state.document).unwrap();
    let folded = document.rope.line_to_byte(1)
        ..taide_native_editor::editing::line_content_range(&document, 3).end;
    store
        .set_view_state(view, state.selection, state.scroll, vec![folded])
        .unwrap();
    let mut presentation = bracket_options();
    presentation.options.folding = true;
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    assert_eq!(
        colored_brackets(&shown),
        [('{', BRACKET_PALETTE[0]), ('}', BRACKET_PALETTE[0])]
    );
    assert!(filled_guides(&shown, BRACKET_PALETTE[1]).is_empty());
    assert!(
        filled_guides(
            &shown,
            BRACKET_PALETTE[1].gamma_multiply(INACTIVE_GUIDE_OPACITY)
        )
        .is_empty()
    );
}

#[test]
fn 괄호_안내선을_꺼도_기본_들여쓰기와_활성_가이드는_남고_빈줄과_탭을_따른다() {
    let text = "{\n\tx\n\n\ty\n}";
    let (context, editor, mut store, view) = fixture(text, false);
    let mut presentation = bracket_options();
    presentation.options.bracket_pair_guides = false;
    select(
        &mut store,
        view,
        vec![display_caret(text.find('x').unwrap())],
    );
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    let active = filled_guides(&shown, ACTIVE_INDENT_GUIDE);
    assert_eq!(active.len(), 3);
    assert!(active.iter().all(|(rect, _)| rect.width() == 1.0
        && rect.height() == LINE_HEIGHT
        && rect.left() == shown.geometry.content_rect.left()));
    assert!(filled_guides(&shown, BRACKET_PALETTE[0]).is_empty());
    presentation.options.bracket_colors = None;
    let cleared = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    assert!(filled_guides(&cleared, ACTIVE_INDENT_GUIDE).is_empty());
    assert!(filled_guides(&cleared, INDENT_GUIDE).is_empty());
}

#[test]
fn 괄호_뷰_문서_편집은_불일치_색을_회수하고_다른_뷰도_같은_분석을_재사용한다() {
    let (context, editor, mut store, view) = fixture("{)", false);
    let presentation = bracket_options();
    let invalid = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    assert_eq!(
        colored_brackets(&invalid),
        [('{', UNEXPECTED_BRACKET), (')', UNEXPECTED_BRACKET)]
    );
    let document = store.views().get(view).unwrap().document;
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            taide_native_editor::store::Transaction {
                revision: snapshot.revision,
                edits: vec![taide_native_editor::document::Edit {
                    bytes: 1..2,
                    text: "}".into(),
                }],
                group: taide_native_editor::document::UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    let corrected = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    assert_eq!(
        colored_brackets(&corrected),
        [('{', BRACKET_PALETTE[0]), ('}', BRACKET_PALETTE[0])]
    );
    let rules = display_bracket_rules();
    let model = store
        .bracket_model(document, Some(&rules), 4, None)
        .unwrap();
    assert_eq!(model.refresh_count(), 2);
    let second = store
        .attach_view(
            ViewKey {
                window: "display-test".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let shared = show_brackets(&context, &editor, &mut store, second, &presentation, None);
    assert_eq!(
        colored_brackets(&shared),
        [('{', BRACKET_PALETTE[0]), ('}', BRACKET_PALETTE[0])]
    );
    let repeated = store
        .bracket_model(document, Some(&rules), 4, None)
        .unwrap();
    assert_eq!(repeated.refresh_count(), model.refresh_count());
    assert!(std::sync::Arc::ptr_eq(&model, &repeated));
}

#[test]
fn 괄호_뷰_안내선은_탭_열과_양방향_스크롤_복원_좌표에_맞춰_화면만_그린다() {
    const BODY_LINES: usize = 100;
    const LINE_CHARS: usize = 100;
    const SCROLL_LINE: f32 = 50.0;
    const SCROLL_X: f32 = 40.0;
    const SCROLL_OFFSET: f32 = 4.0;
    const GUIDE_COLUMNS: f32 = 12.0;
    let text = format!(
        "\t\t\t{{\n{}\t\t\t}}",
        format!("\t\t\t\tx{}\n", "x".repeat(LINE_CHARS)).repeat(BODY_LINES)
    );
    let (context, editor, mut store, view) = fixture(&text, false);
    let presentation = bracket_options();
    let mut state = store.views().get(view).unwrap().clone();
    state.scroll.x = SCROLL_X;
    state.scroll.y = LINE_HEIGHT * SCROLL_LINE + SCROLL_OFFSET;
    store
        .set_view_state(view, state.selection, state.scroll, state.folds)
        .unwrap();
    let shown = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    close(shown.geometry.scroll.x, SCROLL_X);
    close(
        shown.geometry.scroll.y,
        LINE_HEIGHT * SCROLL_LINE + SCROLL_OFFSET,
    );
    let guides = filled_guides(
        &shown,
        BRACKET_PALETTE[0].gamma_multiply(INACTIVE_GUIDE_OPACITY),
    );
    assert!(!guides.is_empty());
    let x = shown.geometry.content_rect.left() - shown.geometry.scroll.x
        + font_width(&context, " ") * GUIDE_COLUMNS;
    for (rect, clip) in guides {
        close(rect.left(), x);
        close(rect.width(), 1.0);
        close(rect.height(), LINE_HEIGHT);
        assert_eq!(clip, shown.geometry.content_rect);
        assert!(rect.intersects(clip));
    }
}
