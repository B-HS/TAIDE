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
    admitted_fixture(text, FileSizeTier::Normal, read_only)
}

fn admitted_fixture(
    text: &str,
    tier: FileSizeTier,
    read_only: bool,
) -> (Context, NativeEditor, EditorStore, ViewId) {
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
                tier,
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
    focus_ids: Vec<egui::Id>,
}

fn show(
    context: &Context,
    editor: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    frame: Frame,
) -> Shown {
    show_problems(context, editor, store, view, presentation, frame, None)
}

fn show_problems(
    context: &Context,
    editor: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    frame: Frame,
    mut problems: Option<&mut ProblemProvider>,
) -> Shown {
    let mut geometry = None;
    let mut focus_ids = Vec::new();
    let mut events = if context.input(|input| input.pointer.hover_pos().is_none())
        && !frame
            .events
            .iter()
            .any(|event| matches!(event, Event::PointerMoved(_)))
    {
        vec![Event::PointerMoved(pos2(100.0, 50.0))]
    } else {
        Vec::new()
    };
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
                .show_request(
                    ui,
                    store,
                    view,
                    EditorRequest {
                        request_focus: frame.focus,
                        keymap: |_: &egui::Ui, _: &Event, _: bool| false,
                        route: |response: &egui::Response| {
                            response.ctx.keyboard_input_route(response.id)
                        },
                        presentation,
                        tokens: |_: &EditorStore| None,
                        language: None,
                        decorations: &[],
                        fold_commands: &[],
                        #[cfg(feature = "native-host")]
                        syntax_folds: None,
                        #[cfg(feature = "native-host")]
                        documentation: None,
                        #[cfg(feature = "native-host")]
                        documentation_commands: &[],
                        fold_controls: None,
                        problems: if let Some(provider) = problems.as_mut() {
                            Some(&mut **provider)
                        } else {
                            None
                        },
                        #[cfg(feature = "native-host")]
                        locations: None,
                    },
                )
                .unwrap();
            assert!(shown.errors.is_empty());
            focus_ids = shown.focus_ids;
            geometry = Some(shown.geometry);
        },
    );
    output.textures_delta.clear();
    Shown {
        shapes: output.shapes,
        geometry: geometry.unwrap(),
        focus_ids,
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
    assert!(defaults.sticky_scroll);
    assert!(defaults.minimap);
    settings.editor_minimap = false;
    assert!(!editor_presentation(&settings).options.minimap);
    settings.editor_sticky_scroll_enabled = false;
    assert!(!editor_presentation(&settings).options.sticky_scroll);
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
const MATCH_BORDER: Color32 = Color32::from_rgb(91, 182, 73);
const MATCH_BACKGROUND: [u8; 4] = [0, 100, 0, 26];
const MATCH_SELECTION_LIMIT: usize = 100;

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
        match_background: Color32::TRANSPARENT,
        match_border: Color32::TRANSPARENT,
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
    let mut frame = Frame::at(0.0);
    frame.focus = false;
    show_brackets_frame(context, editor, store, view, presentation, tokens, frame)
}

fn show_brackets_frame(
    context: &Context,
    editor: &NativeEditor,
    store: &mut EditorStore,
    view: ViewId,
    presentation: &EditorPresentation,
    tokens: Option<EditorTokens<'_>>,
    frame: Frame,
) -> Shown {
    let rules = display_bracket_rules();
    let mut geometry = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
            time: Some(frame.time),
            events: frame.events,
            ..Default::default()
        },
        |ui| {
            if frame.wheel != Vec2::ZERO {
                ui.input_mut(|input| input.smooth_scroll_delta = frame.wheel);
            }
            if let Some((line, column)) = frame.reveal {
                editor
                    .reveal_presented(ui, store, view, line, column, presentation)
                    .unwrap();
            }
            let shown = editor
                .show_request(
                    ui,
                    store,
                    view,
                    EditorRequest {
                        request_focus: frame.focus,
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
                        #[cfg(feature = "native-host")]
                        syntax_folds: None,
                        #[cfg(feature = "native-host")]
                        documentation: None,
                        #[cfg(feature = "native-host")]
                        documentation_commands: &[],
                        fold_controls: None,
                        #[cfg(feature = "native-host")]
                        problems: None,
                        #[cfg(feature = "native-host")]
                        locations: None,
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
        focus_ids: Vec::new(),
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

fn matching_options() -> EditorPresentation {
    let mut presentation = bracket_options();
    presentation.options.bracket_pair_guides = false;
    let colors = presentation.options.bracket_colors.as_mut().unwrap();
    colors.match_background = Color32::from_rgba_unmultiplied(
        MATCH_BACKGROUND[0],
        MATCH_BACKGROUND[1],
        MATCH_BACKGROUND[2],
        MATCH_BACKGROUND[3],
    );
    colors.match_border = MATCH_BORDER;
    presentation
}

fn matched_rects(shown: &Shown, border: Color32) -> Vec<Rect> {
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Rect(rect) if rect.stroke.color == border => {
                close(rect.stroke.width, 1.0);
                assert_eq!(rect.stroke_kind, egui::StrokeKind::Inside);
                Some(rect.rect)
            }
            _ => None,
        })
        .collect()
}

fn assert_matched_ranges(shown: &Shown, ranges: &[std::ops::Range<usize>]) {
    let expected = ranges
        .iter()
        .flat_map(|range| shown.geometry.range_rects(range.clone()))
        .collect::<Vec<_>>();
    let actual = matched_rects(shown, MATCH_BORDER);
    assert_eq!(actual.len(), expected.len(), "{actual:?} != {expected:?}");
    for (actual, expected) in actual.into_iter().zip(expected) {
        close(actual.left(), expected.left());
        close(actual.right(), expected.right());
        close(actual.top(), expected.top());
        close(actual.bottom(), expected.bottom());
    }
}

#[test]
fn 괄호_일치_강조는_near_경계와_enclosing의_실제_글자_좌표를_따르고_색상을_보존한다() {
    let (context, editor, mut store, view) = fixture("{[a]}", false);
    let presentation = matching_options();
    for (byte, ranges) in [
        (0, [0..1, 4..5]),
        (1, [1..2, 3..4]),
        (2, [1..2, 3..4]),
        (3, [1..2, 3..4]),
        (4, [0..1, 4..5]),
        (5, [0..1, 4..5]),
    ] {
        select(&mut store, view, vec![display_caret(byte)]);
        let shown = show_brackets_frame(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            None,
            Frame::at(0.0),
        );
        assert_matched_ranges(&shown, &ranges);
        assert_eq!(
            colored_brackets(&shown),
            [
                ('{', BRACKET_PALETTE[0]),
                ('[', BRACKET_PALETTE[1]),
                (']', BRACKET_PALETTE[1]),
                ('}', BRACKET_PALETTE[0])
            ]
        );
        assert!(
            shown
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    Shape::Rect(rect) if rect.stroke.color == MATCH_BORDER =>
                        Some((rect, shape.clip_rect)),
                    _ => None,
                })
                .all(|(rect, clip)| rect.fill
                    == presentation
                        .options
                        .bracket_colors
                        .unwrap()
                        .match_background
                    && clip == shown.geometry.content_rect)
        );
    }
    let text = "{ [ a ] }";
    let (context, editor, mut store, view) = fixture(text, false);
    select(
        &mut store,
        view,
        vec![display_caret(text.find('a').unwrap())],
    );
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_matched_ranges(&shown, &[2..3, 6..7]);
}

#[test]
fn 괄호_일치_강조는_빈_다중_선택만_합치고_개수_상한과_실제_포커스를_따른다() {
    let text = "(a) [b]";
    let (context, editor, mut store, view) = fixture(text, false);
    let presentation = matching_options();
    select(
        &mut store,
        view,
        vec![
            display_caret(0),
            display_caret(1),
            Selection { anchor: 4, head: 7 },
        ],
    );
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_matched_ranges(&shown, &[0..1, 2..3]);
    let owner = context.memory(|memory| memory.focused()).unwrap();
    context.memory_mut(|memory| memory.surrender_focus(owner));
    let unfocused = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    assert!(matched_rects(&unfocused, MATCH_BORDER).is_empty());
    select(&mut store, view, vec![Selection { anchor: 0, head: 3 }]);
    let selected = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert!(matched_rects(&selected, MATCH_BORDER).is_empty());

    let text = "() ".repeat(MATCH_SELECTION_LIMIT + 1);
    let (context, editor, mut store, view) = fixture(&text, false);
    select(
        &mut store,
        view,
        (0..MATCH_SELECTION_LIMIT)
            .map(|index| display_caret(index * "() ".len()))
            .collect(),
    );
    let admitted = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert!(!matched_rects(&admitted, MATCH_BORDER).is_empty());
    select(
        &mut store,
        view,
        (0..MATCH_SELECTION_LIMIT + 1)
            .map(|index| display_caret(index * "() ".len()))
            .collect(),
    );
    let limited = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert!(matched_rects(&limited, MATCH_BORDER).is_empty());
}

#[test]
fn 괄호_일치_위젯_포커스는_본문_강조를_유지하면서_외부_입력의_문자를_보존한다() {
    let (context, editor, mut store, view) = fixture("(a)", false);
    let rules = display_bracket_rules();
    let mut presentation = matching_options();
    let mut query = "a".to_owned();
    let input_id = egui::Id::new("matching-find-input");
    for related in [true, false] {
        let mut matched = None;
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                events: vec![Event::Text("Z".into())],
                ..Default::default()
            },
            |ui| {
                ui.memory_mut(|memory| memory.request_focus(input_id));
                egui::TextEdit::singleline(&mut query).id(input_id).show(ui);
                presentation.options.bracket_widget_focus =
                    related && ui.memory(|memory| memory.has_focus(input_id));
                let shown = editor
                    .show_request(
                        ui,
                        &mut store,
                        view,
                        EditorRequest {
                            request_focus: false,
                            keymap: |_: &egui::Ui, _: &Event, _| false,
                            route: |response: &egui::Response| {
                                response.ctx.keyboard_input_route(response.id)
                            },
                            presentation: &presentation,
                            tokens: |_: &EditorStore| None,
                            language: Some(Language {
                                rules: &rules,
                                syntax: &UntokenizedLines,
                            }),
                            decorations: &[],
                            fold_commands: &[],
                            #[cfg(feature = "native-host")]
                            syntax_folds: None,
                            #[cfg(feature = "native-host")]
                            documentation: None,
                            #[cfg(feature = "native-host")]
                            documentation_commands: &[],
                            fold_controls: None,
                            #[cfg(feature = "native-host")]
                            problems: None,
                            #[cfg(feature = "native-host")]
                            locations: None,
                        },
                    )
                    .unwrap();
                assert!(shown.errors.is_empty());
                matched = Some(shown.geometry);
                assert!(ui.memory(|memory| memory.has_focus(input_id)));
            },
        );
        output.textures_delta.clear();
        let shown = Shown {
            shapes: output.shapes,
            geometry: matched.unwrap(),
            focus_ids: Vec::new(),
        };
        assert_eq!(
            matched_rects(&shown, MATCH_BORDER).len(),
            if related { 2 } else { 0 }
        );
        let document = store.views().get(view).unwrap().document;
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "(a)"
        );
    }
    assert_eq!(query.matches('Z').count(), 2);
}

#[test]
fn 괄호_일치_강조는_wrap_접기_탭_유니코드와_양방향_스크롤_clip을_따른다() {
    let text = format!("\t한({})", "x".repeat(100));
    let (context, editor, mut store, view) = fixture(&text, false);
    let mut presentation = matching_options();
    presentation.options.word_wrap = true;
    select(
        &mut store,
        view,
        vec![display_caret(text.find('(').unwrap())],
    );
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_matched_ranges(&shown, &[4..5, text.len() - 1..text.len()]);
    assert!(
        matched_rects(&shown, MATCH_BORDER)[1].top() > matched_rects(&shown, MATCH_BORDER)[0].top()
    );

    let text = "{\n    [\n        x\n    ]\n}\nlast";
    let (context, editor, mut store, view) = fixture(text, false);
    presentation.options.word_wrap = false;
    presentation.options.folding = true;
    select(&mut store, view, vec![display_caret(0)]);
    let state = store.views().get(view).unwrap().clone();
    let snapshot = store.documents().snapshot(state.document).unwrap();
    let fold = snapshot.rope.line_to_byte(1)
        ..taide_native_editor::editing::line_content_range(&snapshot, 3).end;
    store
        .set_view_state(view, state.selection, state.scroll, vec![fold])
        .unwrap();
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_matched_ranges(
        &shown,
        &[0..1, text.find('}').unwrap()..text.find('}').unwrap() + 1],
    );
    assert_eq!(
        matched_rects(&shown, MATCH_BORDER)[1].top(),
        shown.geometry.rect.top() + LINE_HEIGHT
    );

    let text = format!(
        "prefix ({})\n{}\nlast",
        "x".repeat(100),
        "tail\n".repeat(10)
    );
    let (context, editor, mut store, view) = fixture(&text, false);
    select(
        &mut store,
        view,
        vec![display_caret(text.find('(').unwrap())],
    );
    let mut state = store.views().get(view).unwrap().clone();
    state.scroll.x = 40.0;
    state.scroll.y = 4.0;
    store
        .set_view_state(view, state.selection, state.scroll, state.folds)
        .unwrap();
    presentation.options.folding = false;
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_matched_ranges(&shown, &[7..8, 108..109]);
    close(shown.geometry.scroll.x, 40.0);
    close(shown.geometry.scroll.y, 4.0);
    assert!(shown.shapes.iter().filter(|shape| matches!(&shape.shape, Shape::Rect(rect) if rect.stroke.color == MATCH_BORDER)).all(|shape| shape.clip_rect == shown.geometry.content_rect));
}

#[test]
fn 괄호_일치_강조는_dpr_테마_옵션과_읽기전용_대형_설정을_따른다() {
    for tier in [
        FileSizeTier::Normal,
        FileSizeTier::Large,
        FileSizeTier::ReadOnly,
    ] {
        let (context, editor, mut store, view) = admitted_fixture("(한)", tier, true);
        let mut presentation = matching_options();
        presentation.options.bracket_pair_colorization = false;
        for scale in [1.0, 1.25, 2.0, 3.0] {
            context.set_pixels_per_point(scale);
            let shown = show_brackets_frame(
                &context,
                &editor,
                &mut store,
                view,
                &presentation,
                None,
                Frame::at(0.0),
            );
            assert_matched_ranges(&shown, &[0..1, 4..5]);
            assert_eq!(
                colored_brackets(&shown),
                [('(', Color32::WHITE), (')', Color32::WHITE)]
            );
        }
        presentation
            .options
            .bracket_colors
            .as_mut()
            .unwrap()
            .match_border = Color32::RED;
        let themed = show_brackets(&context, &editor, &mut store, view, &presentation, None);
        assert!(matched_rects(&themed, MATCH_BORDER).is_empty());
        assert_eq!(matched_rects(&themed, Color32::RED).len(), 2);
        presentation.options.bracket_colors = None;
        let cleared = show_brackets(&context, &editor, &mut store, view, &presentation, None);
        assert!(matched_rects(&cleared, Color32::RED).is_empty());
    }
}

#[test]
fn 괄호_일치_강조는_문자_편집과_문서_뷰_수명에서_이전_짝을_회수한다() {
    let (context, editor, mut store, view) = fixture("(한)", false);
    let presentation = matching_options();
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_matched_ranges(&shown, &[0..1, 4..5]);
    select(&mut store, view, vec![Selection { anchor: 4, head: 5 }]);
    let mut frame = Frame::at(0.1);
    frame.events = vec![Event::Text("]".into())];
    let invalid = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        frame,
    );
    assert!(matched_rects(&invalid, MATCH_BORDER).is_empty());
    let document = store.views().get(view).unwrap().document;
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "(한]"
    );
    select(&mut store, view, vec![Selection { anchor: 4, head: 5 }]);
    let mut frame = Frame::at(0.2);
    frame.events = vec![Event::Text(")".into())];
    let restored = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        frame,
    );
    assert_matched_ranges(&restored, &[0..1, 4..5]);
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
    let shared = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        second,
        &presentation,
        None,
        Frame::at(0.3),
    );
    assert_matched_ranges(&shared, &[0..1, 4..5]);
    store.detach_view(view).unwrap();
    let current = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        second,
        &presentation,
        None,
        Frame::at(0.4),
    );
    assert_matched_ranges(&current, &[0..1, 4..5]);
}

#[test]
fn 괄호_일치_강조는_고정줄_포커스에서_본문만_표시한다() {
    let text = "{\n  a\n  b\n  c\n  d\n  e\n}\ntail\nlast";
    let (context, editor, mut store, view) = fixture(text, false);
    let mut presentation = matching_options();
    presentation.options.sticky_scroll = true;
    presentation.options.sticky_colors =
        Some(taide_native_ui::editor_sticky_scroll::EditorStickyColors {
            background: Color32::BLACK,
            border: Color32::GRAY,
            hover: Color32::DARK_GRAY,
            shadow: Color32::TRANSPARENT,
        });
    let mut state = store.views().get(view).unwrap().clone();
    let snapshot = store.documents().snapshot(state.document).unwrap();
    presentation.options.sticky_model = Some(std::sync::Arc::new(
        taide_native_editor::sticky_model::StickyModel::new(
            &snapshot,
            &[taide_native_editor::sticky_model::StickyScope {
                start_line: 0,
                end_line: 6,
            }],
        ),
    ));
    state.scroll.y = LINE_HEIGHT * 2.0;
    state.selection = SelectionSet {
        primary: 0,
        selections: vec![display_caret(text.find('c').unwrap())],
    };
    store
        .set_view_state(view, state.selection, state.scroll, state.folds)
        .unwrap();
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_matched_ranges(
        &shown,
        &[0..1, text.find('}').unwrap()..text.find('}').unwrap() + 1],
    );
    let owner = context.memory(|memory| memory.focused()).unwrap();
    context.memory_mut(|memory| memory.request_focus(owner.with(("sticky-line", 0usize))));
    let sticky = show_brackets(&context, &editor, &mut store, view, &presentation, None);
    assert_matched_ranges(
        &sticky,
        &[0..1, text.find('}').unwrap()..text.find('}').unwrap() + 1],
    );
    assert!(
        matched_rects(&sticky, MATCH_BORDER)
            .iter()
            .all(|rect| rect.top() >= sticky.geometry.rect.top() + LINE_HEIGHT)
    );
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

const DIAGNOSTIC_ERROR: Color32 = Color32::from_rgb(213, 19, 113);
const DIAGNOSTIC_WARNING: Color32 = Color32::from_rgb(19, 213, 113);
const DIAGNOSTIC_HINT: Color32 = Color32::from_rgb(113, 19, 213);

fn diagnostic_options(
    store: &EditorStore,
    view: ViewId,
    bytes: std::ops::Range<usize>,
    severity: taide_native_editor::diagnostics::Severity,
) -> EditorPresentation {
    use std::sync::Arc;
    use taide_native_editor::diagnostics::{Marker, MarkerSet, Message};
    let snapshot = store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap();
    let mut presentation = options();
    presentation.options.diagnostics = Some(Arc::new(
        MarkerSet::new(
            &snapshot,
            vec![Marker {
                bytes,
                message: Arc::new(Message {
                    severity,
                    text: "synthetic diagnostic".into(),
                    source: Some("synthetic server".into()),
                    code: Some("42".into()),
                }),
            }],
        )
        .unwrap(),
    ));
    presentation.options.diagnostic_colors =
        Some(taide_native_ui::editor_diagnostics::DiagnosticColors {
            error: DIAGNOSTIC_ERROR,
            warning: DIAGNOSTIC_WARNING,
            information: Color32::GREEN,
            hint: DIAGNOSTIC_HINT,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            border: Color32::GRAY,
        });
    presentation
}

fn diagnostic_paths(shown: &Shown, color: Color32) -> Vec<Rect> {
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Path(path) if path.stroke.color == egui::epaint::ColorMode::Solid(color) => {
                Some(shape.clip_rect)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn 같은_범위의_여러_진단_hover는_각_메시지를_한번씩_표시한다() {
    use std::sync::Arc;
    use taide_native_editor::diagnostics::{Marker, MarkerSet, Message, Severity};
    let (context, editor, mut store, view) = fixture("abc def", false);
    let mut presentation = diagnostic_options(&store, view, 0..7, Severity::Error);
    let document = store
        .documents()
        .snapshot(store.views().get(view).unwrap().document)
        .unwrap();
    presentation.options.diagnostics = Some(Arc::new(
        MarkerSet::new(
            &document,
            [
                ("owner one", Severity::Error),
                ("owner two", Severity::Warning),
            ]
            .into_iter()
            .map(|(text, severity)| Marker {
                bytes: 0..7,
                message: Arc::new(Message {
                    severity,
                    text: text.into(),
                    source: None,
                    code: None,
                }),
            })
            .collect(),
        )
        .unwrap(),
    ));
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let pointer = shown.geometry.range_rects(0..7)[0].center();
    let mut frame = Frame::at(0.1);
    frame.events.push(Event::PointerMoved(pointer));
    show(&context, &editor, &mut store, view, &presentation, frame);
    show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.7),
    );
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(1.0),
    );
    let text = text_shapes(&shown);
    assert_eq!(
        text.iter()
            .filter(|text| text.as_str() == "owner one")
            .count(),
        1,
        "{text:?}"
    );
    assert_eq!(
        text.iter()
            .filter(|text| text.as_str() == "owner two")
            .count(),
        1,
        "{text:?}"
    );
}

#[test]
fn 진단_밑줄은_실제_탭_unicode_글자와_편집_anchor_clip을_따른다() {
    use taide_native_editor::diagnostics::Severity;
    let (context, editor, mut store, view) = fixture("a\t\u{1f600}한 bad\nnext", false);
    let presentation = diagnostic_options(&store, view, 2..9, Severity::Error);
    let before = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let rectangles = before.geometry.range_rects(2..9);
    let paths = diagnostic_paths(&before, DIAGNOSTIC_ERROR);
    assert_eq!(paths.len(), 1);
    close(paths[0].left(), rectangles[0].left());
    close(paths[0].right(), rectangles[0].right());
    assert!(before.geometry.content_rect.contains_rect(paths[0]));
    select(&mut store, view, vec![display_caret(0)]);
    let mut frame = Frame::at(0.1);
    frame.events.push(Event::Text("X".into()));
    let after = show(&context, &editor, &mut store, view, &presentation, frame);
    let range = after.geometry.range_rects(3..10);
    let paths = diagnostic_paths(&after, DIAGNOSTIC_ERROR);
    assert_eq!(paths.len(), 1);
    close(paths[0].left(), range[0].left());
    close(paths[0].right(), range[0].right());
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0],
        display_caret(1)
    );
}

#[test]
fn 진단_힌트는_점으로_표시하고_빈_오류는_한글자_폭이며_읽기전용은_밑줄을_숨긴다() {
    use taide_native_editor::diagnostics::Severity;
    let (context, editor, mut store, view) = fixture("abc\n", false);
    let hint = diagnostic_options(&store, view, 0..2, Severity::Hint);
    let shown = show(&context, &editor, &mut store, view, &hint, Frame::at(0.0));
    assert!(diagnostic_paths(&shown, DIAGNOSTIC_HINT).is_empty());
    assert_eq!(shown.shapes.iter().filter(|shape| matches!(&shape.shape, Shape::Circle(circle) if circle.fill == DIAGNOSTIC_HINT)).count(), 3);
    let empty = diagnostic_options(&store, view, 4..4, Severity::Error);
    let shown = show(&context, &editor, &mut store, view, &empty, Frame::at(0.1));
    let paths = diagnostic_paths(&shown, DIAGNOSTIC_ERROR);
    assert_eq!(paths.len(), 1);
    close(paths[0].width(), font_width(&context, "n"));
    let (context, editor, mut store, view) = fixture("abc", true);
    let readonly = diagnostic_options(&store, view, 0..2, Severity::Warning);
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &readonly,
        Frame::at(0.0),
    );
    assert!(diagnostic_paths(&shown, DIAGNOSTIC_WARNING).is_empty());
}

#[test]
fn 진단_밑줄은_wrap_접기_다중뷰_테마변경과_잘못된_문서공급을_따른다() {
    use taide_native_editor::diagnostics::Severity;
    let text = "abcdefghij ".repeat(8) + "\nhidden\nend";
    let (context, editor, mut store, view) = fixture(&text, false);
    let mut presentation = diagnostic_options(&store, view, 0..88, Severity::Warning);
    presentation.options.word_wrap = true;
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    assert_eq!(
        diagnostic_paths(&shown, DIAGNOSTIC_WARNING).len(),
        shown.geometry.range_rects(0..88).len()
    );
    assert!(diagnostic_paths(&shown, DIAGNOSTIC_WARNING).len() > 1);
    presentation
        .options
        .diagnostic_colors
        .as_mut()
        .unwrap()
        .warning = DIAGNOSTIC_ERROR;
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.1),
    );
    assert!(diagnostic_paths(&shown, DIAGNOSTIC_WARNING).is_empty());
    assert!(!diagnostic_paths(&shown, DIAGNOSTIC_ERROR).is_empty());
    let other = store
        .attach_view(
            ViewKey {
                window: "another-display".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            store.views().get(view).unwrap().document,
        )
        .unwrap();
    let shown = show(
        &context,
        &editor,
        &mut store,
        other,
        &presentation,
        Frame::at(0.2),
    );
    assert!(!diagnostic_paths(&shown, DIAGNOSTIC_ERROR).is_empty());
    let (context, editor, mut different, view) = fixture("different", false);
    let mut foreign = different
        .documents()
        .snapshot(different.views().get(view).unwrap().document)
        .unwrap();
    foreign.key = taide_native_editor::document::DocumentKey::File("/synthetic/foreign.txt".into());
    presentation.options.diagnostics = Some(std::sync::Arc::new(
        taide_native_editor::diagnostics::MarkerSet::new(
            &foreign,
            vec![taide_native_editor::diagnostics::Marker {
                bytes: 0..2,
                message: presentation.options.diagnostics.as_ref().unwrap().markers()[0]
                    .message
                    .clone(),
            }],
        )
        .unwrap(),
    ));
    let shown = show(
        &context,
        &editor,
        &mut different,
        view,
        &presentation,
        Frame::at(0.3),
    );
    assert!(diagnostic_paths(&shown, DIAGNOSTIC_ERROR).is_empty());
}

fn text_shapes(shown: &Shown) -> Vec<String> {
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn 진단_hover는_메시지와_source_code를_표시하고_본문_선택을_바꾸지_않는다() {
    use taide_native_editor::diagnostics::Severity;
    let (context, editor, mut store, view) = fixture("message target", false);
    let presentation = diagnostic_options(&store, view, 0..7, Severity::Error);
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let pointer = shown.geometry.range_rects(0..7)[0].center();
    let mut popup_point = None;
    for time in [0.1, 0.7, 1.0] {
        let mut frame = Frame::at(time);
        if time == 0.1 {
            frame.events.push(Event::PointerMoved(pointer));
        }
        let shown = show(&context, &editor, &mut store, view, &presentation, frame);
        if time == 1.0 {
            let text = text_shapes(&shown).join("\n");
            assert!(text.contains("synthetic diagnostic"), "{text}");
            assert!(text.contains("synthetic server(42)"), "{text}");
            popup_point = shown.shapes.iter().find_map(|shape| match &shape.shape {
                Shape::Text(text) if text.galley.job.text == "synthetic diagnostic" => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            });
        }
    }
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0],
        display_caret(0)
    );
    let mut frame = Frame::at(1.1);
    frame.events.push(Event::PointerMoved(popup_point.unwrap()));
    let shown = show(&context, &editor, &mut store, view, &presentation, frame);
    assert!(
        text_shapes(&shown)
            .iter()
            .any(|text| text.contains("synthetic diagnostic"))
    );
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0],
        display_caret(0)
    );
    let mut frame = Frame::at(1.2);
    frame.events.push(Event::PointerGone);
    let shown = show(&context, &editor, &mut store, view, &presentation, frame);
    assert!(
        !text_shapes(&shown)
            .iter()
            .any(|text| text.contains("synthetic diagnostic"))
    );
}

const OVERVIEW_ERROR: Color32 = Color32::from_rgb(177, 15, 107);
const OVERVIEW_FIND: Color32 = Color32::from_rgb(107, 177, 15);
const OVERVIEW_BRACKET: Color32 = Color32::from_rgb(15, 107, 177);

fn overview_colors() -> taide_native_ui::editor_overview::OverviewColors {
    taide_native_ui::editor_overview::OverviewColors {
        error: OVERVIEW_ERROR,
        warning: DIAGNOSTIC_WARNING,
        information: Color32::GREEN,
        find: OVERVIEW_FIND,
        minimap_find: OVERVIEW_FIND,
        bracket: OVERVIEW_BRACKET,
        border: Color32::GRAY,
    }
}

#[test]
fn 진단_overview는_오른쪽_lane_실제_표시줄과_최소높이_읽기전용을_따른다() {
    use taide_native_editor::diagnostics::Severity;
    let text = "line\n".repeat(100);
    let (context, editor, mut store, view) = fixture(&text, false);
    let mut presentation = diagnostic_options(&store, view, 250..254, Severity::Error);
    presentation.options.overview_colors = Some(overview_colors());
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    let marks = filled_guides(&shown, OVERVIEW_ERROR);
    assert_eq!(marks.len(), 1);
    let (mark, clip) = marks[0];
    assert_eq!(clip.right(), shown.geometry.rect.right());
    close(mark.right(), clip.right());
    close(mark.height(), 6.0);
    let expected_center = shown.geometry.rect.top() + shown.geometry.rect.height() * 50.5 / 101.0;
    close(mark.center().y, expected_center);
    let (context, editor, mut store, view) = fixture(&text, true);
    let mut presentation = diagnostic_options(&store, view, 250..254, Severity::Error);
    presentation.options.overview_colors = Some(overview_colors());
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    assert!(filled_guides(&shown, OVERVIEW_ERROR).is_empty());
}

#[test]
fn near_괄호만_overview_가운데_lane에_표시하고_enclosing은_제외한다() {
    let (context, editor, mut store, view) = fixture("(abc)\nend", false);
    let mut presentation = matching_options();
    presentation.options.overview_colors = Some(overview_colors());
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.0),
    );
    assert_eq!(filled_guides(&shown, OVERVIEW_BRACKET).len(), 1);
    select(&mut store, view, vec![display_caret(2)]);
    let shown = show_brackets_frame(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        None,
        Frame::at(0.1),
    );
    assert!(filled_guides(&shown, OVERVIEW_BRACKET).is_empty());
    assert_eq!(matched_rects(&shown, MATCH_BORDER).len(), 2);
}

struct ProblemProvider {
    markers: taide_native_editor::diagnostics::MarkerSet,
    navigation: taide_native_editor::problem_navigation::Navigation,
    shown: Option<taide_native_editor::problem_navigation::Coordinate>,
    calls: Vec<(
        taide_native_editor::problem_navigation::Command,
        usize,
        String,
    )>,
}

impl ProblemProvider {
    fn new(store: &EditorStore, view: ViewId) -> Self {
        use taide_native_editor::diagnostics::{Marker, MarkerSet, Message, Severity};
        let document = store
            .documents()
            .snapshot(store.views().get(view).unwrap().document)
            .unwrap();
        Self {
            markers: MarkerSet::new(
                &document,
                vec![Marker {
                    bytes: 4..7,
                    message: std::sync::Arc::new(Message {
                        severity: Severity::Error,
                        text: "synthetic problem".into(),
                        source: Some("server".into()),
                        code: Some("42".into()),
                    }),
                }],
            )
            .unwrap(),
            navigation: Default::default(),
            shown: None,
            calls: Vec::new(),
        }
    }
}

impl taide_native_ui::editor_problems::Provider for ProblemProvider {
    fn current(
        &mut self,
        store: &EditorStore,
        view: ViewId,
    ) -> Option<taide_native_ui::editor_problems::Widget> {
        let current = store.views().get(view)?;
        let document = store.documents().snapshot(current.document).ok()?;
        let markers = self.markers.tracked(
            &document,
            store
                .changes_since(document.id, self.markers.revision())
                .ok()?,
        )?;
        let mut coordinate = self.shown.clone()?;
        coordinate.problem.marker = markers.markers()[0].clone();
        let head = current.selection.selections[current.selection.primary].head;
        let bytes = &coordinate.problem.marker.bytes;
        let position = if bytes.start <= head && head <= bytes.end {
            head
        } else {
            bytes.start
        };
        Some(taide_native_ui::editor_problems::Widget {
            coordinate,
            position,
            title: "display.txt".into(),
        })
    }

    fn execute(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        command: taide_native_editor::problem_navigation::Command,
    ) -> Result<bool, taide_native_editor::document::EditorError> {
        use taide_native_editor::problem_navigation::{Command, Problem};
        let current = store.views().get(view).unwrap().clone();
        let document = store.documents().snapshot(current.document)?;
        let head = current.selection.selections[current.selection.primary].head;
        self.calls.push((command, head, document.rope.to_string()));
        if command == Command::Close {
            self.shown = None;
            self.navigation.reset();
            return Ok(true);
        }
        let markers = self
            .markers
            .tracked(
                &document,
                store.changes_since(document.id, self.markers.revision())?,
            )
            .unwrap();
        let resource = "file:///synthetic/display.txt";
        self.navigation.update(
            markers
                .markers()
                .iter()
                .map(|marker| Problem {
                    resource: resource.into(),
                    document: document.id,
                    marker: marker.clone(),
                    initial_range: marker.bytes.clone(),
                })
                .collect(),
        );
        self.navigation.follow_cursor(document.id, head);
        let coordinate = self
            .navigation
            .navigate(resource, head, command.forward())
            .unwrap();
        let position = coordinate.problem.marker.bytes.start;
        store.set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![display_caret(position)],
            },
            current.scroll,
            current.folds,
        )?;
        store.request_selection_reveal(view, position..position, true)?;
        self.shown = Some(coordinate);
        Ok(true)
    }
}

fn problem_options() -> EditorPresentation {
    let mut presentation = options();
    presentation.options.problem_colors = Some(taide_native_ui::editor_problems::Colors {
        diagnostics: taide_native_ui::editor_diagnostics::DiagnosticColors {
            error: DIAGNOSTIC_ERROR,
            warning: DIAGNOSTIC_WARNING,
            information: Color32::GREEN,
            hint: DIAGNOSTIC_HINT,
            background: Color32::BLACK,
            foreground: Color32::WHITE,
            border: Color32::GRAY,
        },
        background: Color32::BLACK,
        heading: Color32::WHITE,
        detail: Color32::GRAY,
        hover: Color32::DARK_GRAY,
        focus: Color32::BLUE,
    });
    presentation
}

fn problem_key(alt: bool, shift: bool) -> Event {
    Event::Key {
        key: egui::Key::F8,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            alt,
            shift,
            ..Modifiers::NONE
        },
    }
}

#[test]
fn 문제_기본키는_본문_문자와_순서대로_실행하고_메시지_zone은_다음_줄을_아래로_옮긴다() {
    use taide_native_editor::problem_navigation::Command;
    let (context, editor, mut store, view) = fixture("abc def\nnext", false);
    let presentation = problem_options();
    let mut provider = ProblemProvider::new(&store, view);
    let before = show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
        Some(&mut provider),
    );
    let mut frame = Frame::at(0.1);
    frame.events = vec![
        Event::Text("X".into()),
        problem_key(false, false),
        Event::Text("Y".into()),
    ];
    let shown = show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    assert_eq!(
        provider.calls[0],
        (Command::NextInFiles, 1, "Xabc def\nnext".into())
    );
    assert_eq!(
        store
            .documents()
            .snapshot(store.views().get(view).unwrap().document)
            .unwrap()
            .rope
            .to_string(),
        "Xabc Ydef\nnext"
    );
    assert!(
        text_shapes(&shown)
            .join("\n")
            .contains("synthetic problemserver(42)")
    );
    assert!(text_shapes(&shown).join("\n").contains("1 of 1 problem"));
    close(
        shown.geometry.caret_rect(10).unwrap().top() - before.geometry.caret_rect(8).unwrap().top(),
        80.0,
    );
    assert_eq!(
        shown
            .geometry
            .byte_at(pos2(50.0, shown.geometry.rect.top() + 50.0)),
        None
    );
}

#[test]
fn 문제_close_클릭_뒤_f8은_다시_메시지를_열고_늦은_클릭_처리로_닫히지_않는다() {
    use taide_native_editor::problem_navigation::Command;
    let (context, editor, mut store, view) = fixture("abc def\nnext", false);
    let presentation = problem_options();
    let mut provider = ProblemProvider::new(&store, view);
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
        Some(&mut provider),
    );
    let mut frame = Frame::at(0.1);
    frame.events.push(problem_key(true, false));
    let shown = show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    let close_button = shown
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.job.text == "\u{ea76}" => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap();
    let mut frame = Frame::at(0.2);
    frame.events.push(Event::PointerMoved(close_button));
    for pressed in [true, false] {
        frame.events.push(Event::PointerButton {
            pos: close_button,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    frame.events.push(problem_key(false, false));
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    assert_eq!(
        provider.calls.iter().map(|call| call.0).collect::<Vec<_>>(),
        vec![Command::Next, Command::Close, Command::NextInFiles]
    );
    assert!(provider.shown.is_some());
}

#[test]
fn 문제_버튼_space는_해제에_실행하고_뒤의_본문_문자를_보존한다() {
    use taide_native_editor::problem_navigation::Command;
    let (context, editor, mut store, view) = fixture("abc def\nnext", false);
    let presentation = problem_options();
    let mut provider = ProblemProvider::new(&store, view);
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
        Some(&mut provider),
    );
    let mut frame = Frame::at(0.1);
    frame.events.push(problem_key(true, false));
    let shown = show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    let close = *shown.focus_ids.last().unwrap();
    context.memory_mut(|memory| memory.request_focus(close));
    let mut frame = Frame::at(0.2);
    frame.focus = false;
    frame.events = vec![
        Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
        Event::Text(" ".into()),
    ];
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    assert_eq!(provider.calls.len(), 1);
    assert!(provider.shown.is_some());
    let mut frame = Frame::at(0.3);
    frame.focus = false;
    frame.events = vec![
        Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
        Event::Text("X".into()),
    ];
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    assert_eq!(provider.calls[1].0, Command::Close);
    assert!(provider.shown.is_none());
    assert_eq!(
        store
            .documents()
            .snapshot(store.views().get(view).unwrap().document)
            .unwrap()
            .rope
            .to_string(),
        "abc Xdef\nnext"
    );
}

#[test]
fn 문제_기본키는_조합중에_보류하고_commit_뒤_실행하며_읽기전용에서도_이동한다() {
    use taide_native_editor::problem_navigation::Command;
    let (context, editor, mut store, view) = fixture("abc def\nnext", false);
    let presentation = problem_options();
    let mut provider = ProblemProvider::new(&store, view);
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
        Some(&mut provider),
    );
    let mut frame = Frame::at(0.1);
    frame.events = vec![
        Event::Ime(ImeEvent::Preedit {
            text: "한".into(),
            active_range_chars: None,
        }),
        problem_key(false, false),
    ];
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    assert!(provider.calls.is_empty());
    assert!(store.views().get(view).unwrap().composition.is_some());
    let mut frame = Frame::at(0.2);
    frame.events = vec![
        Event::Ime(ImeEvent::Commit("한".into())),
        problem_key(false, true),
    ];
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    assert_eq!(
        provider.calls[0],
        (Command::PreviousInFiles, 3, "한abc def\nnext".into())
    );
    let (context, editor, mut store, view) = fixture("abc def\nnext", true);
    let mut provider = ProblemProvider::new(&store, view);
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
        Some(&mut provider),
    );
    let mut frame = Frame::at(0.1);
    frame.events.push(problem_key(true, true));
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    assert_eq!(provider.calls[0].0, Command::Previous);
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0],
        display_caret(4)
    );
    assert_eq!(
        store
            .documents()
            .snapshot(store.views().get(view).unwrap().document)
            .unwrap()
            .revision,
        0
    );
}

#[test]
fn 문제_위젯이_열린_편집기의_외부_드러내기는_예약한_높이를_포함한다() {
    let text = "abc def\n".to_owned() + &"next\n".repeat(30);
    let (context, editor, mut store, view) = fixture(&text, false);
    let presentation = problem_options();
    let mut provider = ProblemProvider::new(&store, view);
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
        Some(&mut provider),
    );
    let mut frame = Frame::at(0.1);
    frame.events.push(problem_key(true, false));
    show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    let mut frame = Frame::at(0.2);
    frame.reveal = Some((10.0, 1.0));
    let shown = show_problems(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        frame,
        Some(&mut provider),
    );
    let byte = store.views().get(view).unwrap().selection.selections[0].head;
    close(
        shown.geometry.caret_rect(byte).unwrap().center().y,
        shown.geometry.rect.center().y,
    );
}

#[test]
fn 문제_외부_입력창의_기본키와_문자는_보존하고_닫기클릭_뒤_본문_소유권만_전환한다() {
    use taide_native_editor::problem_navigation::Command;
    use taide_native_ui::editor_problems::Provider;
    let (context, editor, mut store, view) = fixture("abc def\nnext", false);
    let presentation = problem_options();
    let mut provider = ProblemProvider::new(&store, view);
    let input_id = egui::Id::new("problem-other-input");
    let mut query = String::new();
    let mut render = |time: f64, events: Vec<Event>, open: bool, focus: bool| {
        if open {
            provider.execute(&mut store, view, Command::Next).unwrap();
        }
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                let shown = editor
                    .show_request(
                        ui,
                        &mut store,
                        view,
                        EditorRequest {
                            request_focus: false,
                            keymap: |_: &egui::Ui, _: &Event, _: bool| false,
                            route: |response: &egui::Response| {
                                response.ctx.keyboard_input_route(response.id)
                            },
                            presentation: &presentation,
                            tokens: |_: &EditorStore| None,
                            language: None,
                            decorations: &[],
                            fold_commands: &[],
                            #[cfg(feature = "native-host")]
                            syntax_folds: None,
                            #[cfg(feature = "native-host")]
                            documentation: None,
                            #[cfg(feature = "native-host")]
                            documentation_commands: &[],
                            fold_controls: None,
                            problems: Some(&mut provider),
                            #[cfg(feature = "native-host")]
                            locations: None,
                        },
                    )
                    .unwrap();
                assert!(shown.errors.is_empty());
                if focus {
                    ui.memory_mut(|memory| memory.request_focus(input_id));
                }
                ui.put(
                    Rect::from_min_size(pos2(10.0, 90.0), vec2(100.0, 22.0)),
                    egui::TextEdit::singleline(&mut query).id(input_id),
                );
            },
        );
        output.textures_delta.clear();
        output.shapes
    };
    render(0.0, Vec::new(), false, true);
    render(
        0.1,
        vec![Event::Text("Z".into()), problem_key(false, false)],
        false,
        false,
    );
    let shapes = render(0.2, Vec::new(), true, false);
    let close_button = shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.job.text == "\u{ea76}" => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap();
    let mut events = vec![Event::PointerMoved(close_button)];
    for pressed in [true, false] {
        events.push(Event::PointerButton {
            pos: close_button,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    events.push(Event::Text("X".into()));
    render(0.3, events, false, false);
    drop(render);
    assert_eq!(query, "Z");
    assert_eq!(
        provider.calls.iter().map(|call| call.0).collect::<Vec<_>>(),
        vec![Command::Next, Command::Close]
    );
    assert_eq!(
        store
            .documents()
            .snapshot(store.views().get(view).unwrap().document)
            .unwrap()
            .rope
            .to_string(),
        "abc Xdef\nnext"
    );
}

#[test]
fn 진단_밑줄은_접힌_본문에서_숨기고_펼친_뒤_원래_위치에_표시한다() {
    use taide_native_editor::diagnostics::Severity;
    let (context, editor, mut store, view) = fixture("header\n  hidden\nend", false);
    let mut presentation = diagnostic_options(&store, view, 9..15, Severity::Error);
    presentation.options.folding = true;
    let before = store.views().get(view).unwrap().clone();
    store
        .set_view_state(view, before.selection, before.scroll, vec![7..15])
        .unwrap();
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.0),
    );
    assert!(diagnostic_paths(&shown, DIAGNOSTIC_ERROR).is_empty());
    let before = store.views().get(view).unwrap().clone();
    store
        .set_view_state(view, before.selection, before.scroll, Vec::new())
        .unwrap();
    let shown = show(
        &context,
        &editor,
        &mut store,
        view,
        &presentation,
        Frame::at(0.1),
    );
    assert_eq!(diagnostic_paths(&shown, DIAGNOSTIC_ERROR).len(), 1);
    close(
        diagnostic_paths(&shown, DIAGNOSTIC_ERROR)[0].left(),
        shown.geometry.range_rects(9..15)[0].left(),
    );
}

#[test]
fn overview는_wrap_접기_문제_zone과_dpr을_같은_표시좌표에_투영한다() {
    use taide_native_editor::diagnostics::Severity;
    let text = "abcdefghij ".repeat(8) + "\n  hidden\nend\n" + &"line\n".repeat(100);
    for ratio in [1.0, 1.5, 2.0] {
        let (context, editor, mut store, view) = fixture(&text, false);
        context.set_pixels_per_point(ratio);
        let mut presentation = diagnostic_options(&store, view, 98..101, Severity::Error);
        presentation.options.word_wrap = true;
        presentation.options.folding = true;
        presentation.options.overview_colors = Some(overview_colors());
        presentation.options.problem_colors = problem_options().options.problem_colors;
        let mut provider = ProblemProvider::new(&store, view);
        show_problems(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.0),
            Some(&mut provider),
        );
        let mut frame = Frame::at(0.1);
        frame.events.push(problem_key(true, false));
        let before = show_problems(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            frame,
            Some(&mut provider),
        );
        let marker = filled_guides(&before, OVERVIEW_ERROR)[0].0;
        close(marker.height(), 6.0);
        close(marker.width() * ratio, ((14.0 * ratio - 1.0) / 3.0).floor());
        let snapshot = store
            .documents()
            .snapshot(store.views().get(view).unwrap().document)
            .unwrap();
        let display = store.take_display(view).unwrap().unwrap();
        let row = display.row_of_byte(&snapshot, 98);
        let total = display.row_count();
        store.set_display(view, Some(display)).unwrap();
        let zone_height = 80.0;
        let top = row as f32 * LINE_HEIGHT + zone_height;
        close(
            marker.center().y,
            before.geometry.rect.top()
                + before.geometry.rect.height() * (top + LINE_HEIGHT / 2.0)
                    / (total as f32 * LINE_HEIGHT + zone_height),
        );
        let current = store.views().get(view).unwrap().clone();
        store
            .set_view_state(view, current.selection, current.scroll, vec![89..97])
            .unwrap();
        let folded = show_problems(
            &context,
            &editor,
            &mut store,
            view,
            &presentation,
            Frame::at(0.2),
            Some(&mut provider),
        );
        assert!(filled_guides(&folded, OVERVIEW_ERROR)[0].0.center().y < marker.center().y);
    }
}
