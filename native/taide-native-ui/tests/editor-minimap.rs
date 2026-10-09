#![cfg(feature = "native-host")]

use std::sync::Arc;

use egui::{
    Color32, ColorImage, Context, Event, FontId, Modifiers, PointerButton, RawInput, Rect, Vec2,
    pos2, vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::line_tokens::{LineTokens, TokenStyle, TokenStyleTable};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_native_ui::editor_minimap::EditorMinimapColors;
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorDisplayOptions, EditorPresentation, EditorRequest, EditorTokens,
    NativeEditor,
};

const SCREEN: Vec2 = vec2(400.0, 240.0);
const LINE_HEIGHT: f32 = 20.0;
const FONT_SIZE: f32 = 14.0;
const BYTE_LIMIT: usize = 1024 * 1024;
const HISTORY_LIMIT: usize = 8;
const DOCUMENT_LINES: usize = 400;
const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 30);
const FOREGROUND: Color32 = Color32::from_rgb(212, 212, 212);
const SLIDER: Color32 = Color32::from_rgb(40, 50, 60);
const SLIDER_HOVER: Color32 = Color32::from_rgb(70, 80, 90);
const SLIDER_ACTIVE: Color32 = Color32::from_rgb(100, 110, 120);
const MINIMAP_GUTTER: usize = 8;
const EPSILON: f32 = 0.001;

struct Fixture {
    context: Context,
    editor: NativeEditor,
    store: EditorStore,
    view: ViewId,
    presentation: EditorPresentation,
    tokens: Option<(u64, LineTokens, TokenStyleTable)>,
    screen: Vec2,
    max_texture_side: Option<usize>,
    foreign: Option<String>,
    decorations: Vec<taide_native_editor::decoration::DecorationLayer>,
}

impl Fixture {
    fn new(text: &str) -> Self {
        Self::admitted(text, FileSizeTier::Normal, false)
    }

    fn admitted(text: &str, tier: FileSizeTier, read_only: bool) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 2,
            max_views: 2,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = store
            .open_file(
                "/synthetic/minimap.txt".into(),
                OpenedFile {
                    path: "/synthetic/minimap.txt".into(),
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
                    window: "minimap-test".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
            )
            .unwrap();
        Self {
            context: Context::default(),
            editor: NativeEditor {
                appearance: EditorAppearance {
                    font: FontId::monospace(FONT_SIZE),
                    line_height: LINE_HEIGHT,
                    horizontal_padding: 8.0,
                    background: BACKGROUND,
                    foreground: FOREGROUND,
                    muted: Color32::GRAY,
                    selection: Color32::BLUE,
                    cursor: Color32::WHITE,
                    current_line: Color32::DARK_GRAY,
                    line_numbers: false,
                    indent: "    ".into(),
                },
            },
            store,
            view,
            tokens: None,
            screen: SCREEN,
            max_texture_side: None,
            foreign: None,
            decorations: Vec::new(),
            presentation: EditorPresentation {
                options: EditorDisplayOptions {
                    minimap: true,
                    scroll_beyond_last_line: true,
                    minimap_colors: Some(EditorMinimapColors {
                        background: BACKGROUND,
                        selection: Color32::BLUE,
                        slider: SLIDER,
                        slider_hover: SLIDER_HOVER,
                        slider_active: SLIDER_ACTIVE,
                        shadow: Color32::TRANSPARENT,
                    }),
                    ..Default::default()
                },
            },
        }
    }

    fn show(&mut self, time: f64, events: Vec<Event>) -> Shown {
        let mut geometry = None;
        let mut id = None;
        let mut rendered = None;
        let mut foreign_rect = None;
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), self.screen)),
                max_texture_side: self.max_texture_side,
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                let mut editor_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("minimap-editor")
                        .max_rect(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
                );
                let shown = self
                    .editor
                    .show_request(
                        &mut editor_ui,
                        &mut self.store,
                        self.view,
                        EditorRequest {
                            request_focus: time == 0.0,
                            keymap: |_: &egui::Ui, _: &Event, _: bool| false,
                            route: |response: &egui::Response| {
                                response.ctx.keyboard_input_route(response.id)
                            },
                            presentation: &self.presentation,
                            tokens: |_: &EditorStore| {
                                self.tokens
                                    .as_ref()
                                    .map(|(revision, lines, styles)| EditorTokens {
                                        revision: *revision,
                                        lines,
                                        styles,
                                    })
                            },
                            language: None,
                            decorations: &self.decorations.iter().collect::<Vec<_>>(),
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
                id = Some(shown.response.id);
                rendered = Some(shown.rendered_lines);
                if let Some(text) = self.foreign.as_mut() {
                    let mut foreign_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("minimap-foreign-pane")
                            .max_rect(Rect::from_min_max(
                                pos2(SCREEN.x, 0.0),
                                pos2(self.screen.x, self.screen.y),
                            )),
                    );
                    let shown = egui::TextEdit::singleline(text)
                        .id(egui::Id::new("minimap-foreign-input"))
                        .show(&mut foreign_ui);
                    ui.ctx().register_pointer_keyboard_focus(shown.response.id);
                    foreign_rect = Some(shown.response.rect);
                }
            },
        );
        let manager = self.context.tex_manager();
        let images = output
            .textures_delta
            .set
            .iter()
            .flat_map(|(id, deltas)| {
                if !manager
                    .read()
                    .meta(*id)
                    .is_some_and(|metadata| metadata.name.starts_with("native-minimap-"))
                {
                    return Vec::new();
                }
                deltas
                    .iter()
                    .map(|delta| {
                        let egui::ImageData::Color(image) = &delta.image;
                        Arc::clone(image)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        let freed = output.textures_delta.free.len();
        output.textures_delta.clear();
        Shown {
            geometry: geometry.unwrap(),
            id: id.unwrap(),
            rendered: rendered.unwrap(),
            images,
            shapes: output.shapes,
            foreign_rect,
            freed,
        }
    }

    fn warm(&mut self) -> Shown {
        self.show(0.0, Vec::new());
        self.show(0.1, Vec::new())
    }

    fn selection(&self) -> SelectionSet {
        self.store.views().get(self.view).unwrap().selection.clone()
    }

    fn select(&mut self, anchor: usize, head: usize) {
        let state = self.store.views().get(self.view).unwrap().clone();
        self.store
            .set_view_state(
                self.view,
                SelectionSet {
                    selections: vec![Selection { anchor, head }],
                    primary: 0,
                },
                state.scroll,
                state.folds,
            )
            .unwrap();
    }
}

struct Shown {
    geometry: EditorGeometry,
    id: egui::Id,
    rendered: std::ops::Range<usize>,
    images: Vec<Arc<ColorImage>>,
    shapes: Vec<egui::epaint::ClippedShape>,
    foreign_rect: Option<Rect>,
    freed: usize,
}

fn text() -> String {
    (0..DOCUMENT_LINES)
        .map(|_| "abc")
        .collect::<Vec<_>>()
        .join("\n")
}

fn pointer(position: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn click(position: egui::Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(position),
        pointer(position, true),
        pointer(position, false),
    ]
}

#[test]
fn 미니맵은_본문_폭을_예약하고_원본_축소문자와_추가_토큰_요청_줄을_표시한다() {
    let mut fixture = Fixture::new(&text());
    let shown = fixture.show(0.0, Vec::new());
    let rect = shown.geometry.minimap_rect.unwrap();
    assert!(rect.width() > 0.0);
    assert_eq!(shown.geometry.content_rect.right(), rect.left());
    assert!(shown.geometry.byte_at(rect.center()).is_none());
    assert_eq!(shown.images.len(), 1);
    assert_eq!(shown.images[0].size[1], SCREEN.y as usize);
    assert!(shown.rendered.end > shown.geometry.visible_rows.end);
    let source: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/minimap-reference.txt")).unwrap();
    let glyph = source["glyphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|glyph| {
            glyph["scale"] == 1
                && glyph["code"] == u16::from(b'a')
                && glyph["alpha"] == 255
                && glyph["light"] == false
        })
        .unwrap();
    for row in 0..2 {
        let rgba = glyph["rgba"].as_array().unwrap();
        let start = row * 4;
        let expected = [
            rgba[start].as_u64().unwrap() as u8,
            rgba[start + 1].as_u64().unwrap() as u8,
            rgba[start + 2].as_u64().unwrap() as u8,
            255,
        ];
        assert_eq!(
            shown.images[0][(MINIMAP_GUTTER, row)].to_srgba_unmultiplied(),
            expected
        );
    }
}

#[test]
fn 같은_줄창과_선택_변경은_축소_텍스처를_다시_올리지_않고_실제_선택은_그린다() {
    let mut fixture = Fixture::new(&text());
    fixture.show(0.0, Vec::new());
    assert!(fixture.show(0.1, Vec::new()).images.is_empty());
    fixture.select(0, 2);
    let selected = fixture.show(0.2, Vec::new());
    assert!(selected.images.is_empty());
    assert!(selected.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Rect(rect) if rect.rect.intersects(selected.geometry.minimap_rect.unwrap()) && rect.fill == Color32::BLUE.gamma_multiply(0.9))));
}

#[test]
fn 미니맵_클릭은_해당_줄을_가운데로_보이고_선택과_본문_포커스를_보존한다() {
    let mut fixture = Fixture::new(&text());
    let shown = fixture.warm();
    let selection = fixture.selection();
    let rect = shown.geometry.minimap_rect.unwrap();
    let position = pos2(rect.center().x, rect.top() + 200.0);
    let shown = fixture.show(0.2, click(position));
    assert!(
        (shown.geometry.scroll.y - 1890.0).abs() < EPSILON,
        "scroll {:?}",
        shown.geometry.scroll
    );
    assert_eq!(fixture.selection(), selection);
    assert!(fixture.context.memory(|memory| memory.has_focus(shown.id)));
}

#[test]
fn 미니맵_slider_drag는_최초_ratio로_즉시_스크롤하고_본문_선택을_바꾸지_않는다() {
    let mut fixture = Fixture::new(&text());
    let shown = fixture.warm();
    let selection = fixture.selection();
    let rect = shown.geometry.minimap_rect.unwrap();
    let start = pos2(rect.center().x, rect.top() + 10.0);
    fixture.show(0.2, vec![Event::PointerMoved(start), pointer(start, true)]);
    let dragged = fixture.show(0.3, vec![Event::PointerMoved(start + vec2(0.0, 50.0))]);
    assert!(
        (dragged.geometry.scroll.y - 1847.0).abs() < EPSILON,
        "scroll {:?}",
        dragged.geometry.scroll
    );
    assert_eq!(fixture.selection(), selection);
    fixture.show(0.4, vec![pointer(start + vec2(0.0, 50.0), false)]);
}

#[test]
fn 토큰과_테마_변경은_원본_밝은_축소문자_색으로_텍스처를_갱신한다() {
    let mut fixture = Fixture::new(&text());
    fixture.warm();
    let document = fixture.store.views().get(fixture.view).unwrap().document;
    let snapshot = fixture.store.documents().snapshot(document).unwrap();
    let style = TokenStyle {
        foreground: [0, 128, 0, 255],
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: TokenKind::Other,
    };
    let mut lines = LineTokens::new(snapshot.rope.len_lines());
    lines.set_line(0, vec![0, 0], false);
    fixture.tokens = Some((
        snapshot.revision,
        lines,
        TokenStyleTable::new(style, vec![style]),
    ));
    fixture
        .presentation
        .options
        .minimap_colors
        .as_mut()
        .unwrap()
        .background = Color32::WHITE;
    let shown = fixture.show(0.2, Vec::new());
    assert_eq!(shown.images.len(), 1);
    let source: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/minimap-reference.txt")).unwrap();
    let glyph = source["glyphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|glyph| {
            glyph["scale"] == 1
                && glyph["code"] == u16::from(b'a')
                && glyph["alpha"] == 255
                && glyph["light"] == true
        })
        .unwrap();
    for row in 0..2 {
        let expected = glyph["rgba"].as_array().unwrap()[row * 4..row * 4 + 4]
            .iter()
            .map(|value| value.as_u64().unwrap() as u8)
            .collect::<Vec<_>>();
        assert_eq!(
            shown.images[0][(MINIMAP_GUTTER, row)]
                .to_srgba_unmultiplied()
                .as_slice(),
            expected
        );
    }
}

#[test]
fn 토큰_세대가_같아도_같은_주소의_스타일_교체는_축소_텍스처를_갱신한다() {
    let mut fixture = Fixture::new(&text());
    let document = fixture.store.views().get(fixture.view).unwrap().document;
    let snapshot = fixture.store.documents().snapshot(document).unwrap();
    let style = TokenStyle {
        foreground: [255, 0, 0, 255],
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: TokenKind::Other,
    };
    let mut lines = LineTokens::new(snapshot.rope.len_lines());
    lines.set_line(0, vec![0, 0], false);
    fixture.tokens = Some((
        snapshot.revision,
        lines,
        TokenStyleTable::new(style, vec![style]),
    ));
    let initial = fixture.show(0.0, Vec::new());
    assert!(fixture.show(0.1, Vec::new()).images.is_empty());
    let replacement = TokenStyle {
        foreground: [0, 255, 0, 255],
        ..style
    };
    fixture.tokens.as_mut().unwrap().2 = TokenStyleTable::new(replacement, vec![replacement]);
    let replaced = fixture.show(0.2, Vec::new());
    assert_eq!(replaced.images.len(), 1);
    assert_ne!(
        initial.images[0][(MINIMAP_GUTTER, 0)],
        replaced.images[0][(MINIMAP_GUTTER, 0)]
    );
}

#[test]
fn 미니맵_클릭_뒤의_문자_입력은_본문에_입력하고_최종_캐럿을_보인다() {
    let mut fixture = Fixture::new(&text());
    let shown = fixture.warm();
    let rect = shown.geometry.minimap_rect.unwrap();
    let mut events = click(pos2(rect.center().x, rect.top() + 200.0));
    events.push(Event::Text("x".into()));
    let shown = fixture.show(0.2, events);
    assert_eq!(fixture.selection().selections[0].head, 1);
    assert_eq!(shown.geometry.scroll.y, 0.0);
}

#[test]
fn 문자_입력_뒤의_미니맵_클릭은_마지막_클릭_위치를_보인다() {
    let mut fixture = Fixture::new(&text());
    let shown = fixture.warm();
    let rect = shown.geometry.minimap_rect.unwrap();
    let mut events = vec![Event::Text("x".into())];
    events.extend(click(pos2(rect.center().x, rect.top() + 200.0)));
    let shown = fixture.show(0.2, events);
    assert_eq!(fixture.selection().selections[0].head, 1);
    assert!((shown.geometry.scroll.y - 1890.0).abs() < EPSILON);
}

#[test]
fn 미니맵과_다른_입력창의_클릭_순서와_무관하게_그_입력창의_문자와_포커스를_보존한다() {
    const FOREIGN_WIDTH: f32 = 240.0;
    for map_first in [true, false] {
        let mut fixture = Fixture::new(&text());
        fixture.foreign = Some(String::new());
        fixture.screen.x += FOREIGN_WIDTH;
        let shown = fixture.warm();
        let rect = shown.geometry.minimap_rect.unwrap();
        let map = pos2(rect.center().x, rect.top() + 200.0);
        let foreign = shown.foreign_rect.unwrap().center();
        let mut events = if map_first {
            click(map)
        } else {
            click(foreign)
        };
        events.extend(if map_first {
            click(foreign)
        } else {
            click(map)
        });
        events.push(Event::Text("x".into()));
        let selection = fixture.selection();
        let shown = fixture.show(0.2, events);
        assert_eq!(fixture.foreign.as_deref(), Some("x"));
        assert_eq!(fixture.selection(), selection);
        assert_eq!(
            fixture.context.memory(|memory| memory.focused()),
            Some(egui::Id::new("minimap-foreign-input"))
        );
        assert!((shown.geometry.scroll.y - 1890.0).abs() < EPSILON);
    }
}

#[test]
fn touch는_중심_스크롤을_즉시_적용하고_다른_손가락과_취소_뒤_입력을_무시한다() {
    let mut fixture = Fixture::new(&text());
    let shown = fixture.warm();
    let rect = shown.geometry.minimap_rect.unwrap();
    let position = pos2(rect.center().x, rect.top() + 200.0);
    let touch = |id, phase, pos| Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(id),
        phase,
        pos,
        force: None,
    };
    let selection = fixture.selection();
    let shown = fixture.show(
        0.2,
        vec![
            Event::PointerMoved(position),
            pointer(position, true),
            touch(1, egui::TouchPhase::Start, position),
        ],
    );
    assert!((shown.geometry.scroll.y - 6946.0).abs() < EPSILON);
    assert_eq!(fixture.selection(), selection);
    assert!(fixture.context.memory(|memory| memory.has_focus(shown.id)));
    let shown = fixture.show(0.3, vec![touch(2, egui::TouchPhase::Move, rect.min)]);
    assert!((shown.geometry.scroll.y - 6946.0).abs() < EPSILON);
    assert!(shown.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Rect(slider) if slider.fill == SLIDER_ACTIVE && slider.rect.intersects(rect))));
    let shown = fixture.show(
        0.4,
        vec![touch(
            1,
            egui::TouchPhase::Move,
            pos2(position.x, rect.top() + 100.0),
        )],
    );
    assert!((shown.geometry.scroll.y - 3251.0).abs() < EPSILON);
    let shown = fixture.show(
        0.5,
        vec![
            pointer(position, false),
            touch(1, egui::TouchPhase::Cancel, position),
            touch(1, egui::TouchPhase::Move, rect.min),
        ],
    );
    assert!((shown.geometry.scroll.y - 3251.0).abs() < EPSILON);
    assert_eq!(fixture.selection(), selection);
}

#[test]
fn 제한된_gpu_텍스처_크기에서도_미니맵은_표시_줄을_잃지_않는다() {
    const TEXTURE_SIDE: usize = 1024;
    const PIXEL_RATIO: f32 = 8.0;
    let mut fixture = Fixture::new(&text());
    fixture.context.set_pixels_per_point(PIXEL_RATIO);
    fixture.max_texture_side = Some(TEXTURE_SIDE);
    let shown = fixture.show(0.0, Vec::new());
    assert!(shown.geometry.minimap_rect.is_some());
    assert!(!shown.images.is_empty());
    assert!(
        shown
            .images
            .iter()
            .all(|image| image.size.iter().all(|side| *side <= TEXTURE_SIDE))
    );
    assert_eq!(
        shown
            .images
            .iter()
            .map(|image| image.size[1])
            .sum::<usize>(),
        (SCREEN.y * PIXEL_RATIO) as usize
    );
}

#[test]
fn 반투명_토큰_색은_원본_alpha_혼합으로_축소_문자에_반영된다() {
    const ALPHA: u8 = 127;
    let mut fixture = Fixture::new(&text());
    let document = fixture.store.views().get(fixture.view).unwrap().document;
    let snapshot = fixture.store.documents().snapshot(document).unwrap();
    let style = TokenStyle {
        foreground: [212, 212, 212, ALPHA],
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: TokenKind::Other,
    };
    let mut lines = LineTokens::new(snapshot.rope.len_lines());
    lines.set_line(0, vec![0, 0], false);
    fixture.tokens = Some((
        snapshot.revision,
        lines,
        TokenStyleTable::new(style, vec![style]),
    ));
    let shown = fixture.show(0.0, Vec::new());
    let source: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/minimap-reference.txt")).unwrap();
    let glyph = source["glyphs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|glyph| {
            glyph["scale"] == 1
                && glyph["code"] == u16::from(b'a')
                && glyph["alpha"] == ALPHA
                && glyph["light"] == false
        })
        .unwrap();
    for row in 0..2 {
        let expected = glyph["rgba"].as_array().unwrap()[row * 4..row * 4 + 4]
            .iter()
            .map(|value| value.as_u64().unwrap() as u8)
            .collect::<Vec<_>>();
        assert_eq!(
            shown.images[0][(MINIMAP_GUTTER, row)]
                .to_srgba_unmultiplied()
                .as_slice(),
            expected
        );
    }
}

#[test]
fn 축척과_탭_전각_utf16_문자는_원본_물리_픽셀과_열을_사용한다() {
    const CJK: u16 = 0x4e00;
    const HIGH: u16 = 0xd83d;
    const LOW: u16 = 0xde00;
    let source: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/minimap-reference.txt")).unwrap();
    for ratio in [1.0, 1.25, 2.0, 3.0] {
        let mut fixture = Fixture::new("a一\tb\n😀");
        fixture.context.set_pixels_per_point(ratio);
        let shown = fixture.show(0.0, Vec::new());
        let scale = if ratio >= 2.0 { 2 } else { 1 };
        let image = &shown.images[0];
        assert_eq!(image.size[1], (SCREEN.y * ratio).floor() as usize);
        for (code, column, line) in [
            (u16::from(b'a'), 0, 0),
            (CJK, 1, 0),
            (CJK, 2, 0),
            (u16::from(b'b'), 5, 0),
            (HIGH, 0, 1),
            (LOW, 1, 1),
        ] {
            let glyph = source["glyphs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|glyph| {
                    glyph["scale"] == scale
                        && glyph["code"] == code
                        && glyph["alpha"] == 255
                        && glyph["light"] == false
                })
                .unwrap();
            let expected = glyph["rgba"].as_array().unwrap();
            for dy in 0..scale * 2 {
                for dx in 0..scale {
                    let offset = (dy * scale + dx) * 4;
                    let rgba = expected[offset..offset + 4]
                        .iter()
                        .map(|value| value.as_u64().unwrap() as u8)
                        .collect::<Vec<_>>();
                    assert_eq!(
                        image[(MINIMAP_GUTTER + column * scale + dx, line * scale * 2 + dy)]
                            .to_srgba_unmultiplied()
                            .as_slice(),
                        rgba,
                        "ratio {ratio} code {code} column {column}"
                    );
                }
            }
        }
    }
}

#[test]
fn 옵션을_끄면_폭과_텍스처를_반환하고_다시_켜면_최신_문서를_그린다() {
    let mut fixture = Fixture::new(&text());
    let initial = fixture.warm();
    let reserved = initial.geometry.content_rect.width();
    fixture.presentation.options.minimap = false;
    let disabled = fixture.show(0.2, vec![Event::Text("Z".into())]);
    assert!(disabled.geometry.minimap_rect.is_none());
    assert!(disabled.images.is_empty());
    assert!(disabled.freed > 0);
    assert!(disabled.geometry.content_rect.width() > reserved);
    fixture.presentation.options.minimap = true;
    let enabled = fixture.show(0.3, Vec::new());
    assert_eq!(enabled.images.len(), 1);
    assert_eq!(fixture.selection().selections[0].head, 1);
    assert!(fixture.show(0.4, Vec::new()).images.is_empty());
}

#[test]
fn 미니맵_휠과_읽기_전용_클릭은_스크롤하며_대형_제한은_폭을_예약하지_않는다() {
    const WHEEL: f32 = -LINE_HEIGHT * 3.0;
    for tier in [FileSizeTier::Large, FileSizeTier::ReadOnly] {
        let mut fixture = Fixture::admitted(&text(), tier, true);
        let shown = fixture.show(0.0, Vec::new());
        assert!(shown.geometry.minimap_rect.is_none());
        assert!(shown.images.is_empty());
    }
    let mut fixture = Fixture::admitted(&text(), FileSizeTier::Normal, true);
    let shown = fixture.warm();
    let rect = shown.geometry.minimap_rect.unwrap();
    let selection = fixture.selection();
    let wheel = fixture.show(
        0.2,
        vec![
            Event::PointerMoved(rect.center()),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, WHEEL),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ],
    );
    assert!(wheel.geometry.scroll.y > 0.0);
    let clicked = fixture.show(0.3, click(pos2(rect.center().x, rect.top() + 200.0)));
    assert!(clicked.geometry.scroll.y > wheel.geometry.scroll.y);
    assert_eq!(fixture.selection(), selection);
}

#[test]
fn 접기와_줄바꿈은_실제_표시_줄과_함께_축소_이미지를_갱신한다() {
    const LONG_COLUMNS: usize = 80;
    let content = format!("{}\n    Z\n    B\nC", "a".repeat(LONG_COLUMNS));
    let mut fixture = Fixture::new(&content);
    fixture.presentation.options.folding = true;
    let plain = fixture.show(0.0, Vec::new());
    fixture.presentation.options.word_wrap = true;
    let wrapped = fixture.show(0.2, Vec::new());
    assert_eq!(wrapped.images.len(), 1);
    assert!(wrapped.geometry.visible_rows.end > plain.geometry.visible_rows.end);
    assert_ne!(plain.images.first(), wrapped.images.first());
    let state = fixture.store.views().get(fixture.view).unwrap().clone();
    let snapshot = fixture.store.documents().snapshot(state.document).unwrap();
    fixture
        .store
        .set_view_state(
            fixture.view,
            state.selection,
            state.scroll,
            vec![snapshot.rope.line_to_byte(1)..snapshot.rope.line_to_byte(3) - 1],
        )
        .unwrap();
    let folded = fixture.show(0.3, Vec::new());
    assert_eq!(folded.images.len(), 1);
    assert!(folded.geometry.visible_rows.end < wrapped.geometry.visible_rows.end);
    assert_ne!(folded.images[0], wrapped.images[0]);
    assert!(fixture.show(0.4, Vec::new()).images.is_empty());
}

#[test]
fn 빈줄_진단과_찾기는_미니맵_행_배경을_표시하고_hint와_readonly는_진단을_제외한다() {
    use taide_native_editor::decoration::{
        Decoration, DecorationKind, DecorationLayer, OverviewLane, Stickiness,
    };
    use taide_native_editor::diagnostics::{Marker, MarkerSet, Message, Severity};
    const ERROR: Color32 = Color32::from_rgb(189, 17, 141);
    const FIND: Color32 = Color32::from_rgb(17, 189, 141);
    for (read_only, severity, expected) in [
        (false, Severity::Error, 1),
        (true, Severity::Error, 0),
        (false, Severity::Hint, 0),
    ] {
        let mut fixture = Fixture::admitted("a\n\nend", FileSizeTier::Normal, read_only);
        let document = fixture
            .store
            .documents()
            .snapshot(fixture.store.views().get(fixture.view).unwrap().document)
            .unwrap();
        fixture.presentation.options.overview_colors =
            Some(taide_native_ui::editor_overview::OverviewColors {
                error: ERROR,
                warning: Color32::YELLOW,
                information: Color32::BLUE,
                find: FIND,
                minimap_find: FIND,
                bracket: Color32::GRAY,
                border: Color32::TRANSPARENT,
            });
        fixture.presentation.options.diagnostics = Some(Arc::new(
            MarkerSet::new(
                &document,
                vec![Marker {
                    bytes: 2..2,
                    message: Arc::new(Message {
                        severity,
                        text: "empty line".into(),
                        source: None,
                        code: None,
                    }),
                }],
            )
            .unwrap(),
        ));
        fixture.decorations.push(DecorationLayer::new(
            document.revision,
            0,
            vec![Decoration {
                bytes: 0..1,
                kind: DecorationKind::Overview {
                    lane: OverviewLane::Center,
                    color: FIND.to_srgba_unmultiplied(),
                    minimap: Some(FIND.to_srgba_unmultiplied()),
                },
                stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
            }],
        ));
        let shown = fixture.warm();
        let minimap = shown.geometry.minimap_rect.unwrap();
        let background_rects = |color: Color32| {
            shown
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Rect(rect)
                        if rect.fill == color.gamma_multiply(0.45)
                            && shape.clip_rect == minimap =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let errors = background_rects(ERROR);
        assert_eq!(errors.len(), expected);
        if let Some(rect) = errors.first() {
            assert!((rect.top() - minimap.top() - 2.0).abs() < EPSILON);
            assert!((rect.left() - minimap.left() - MINIMAP_GUTTER as f32).abs() < EPSILON);
            assert_eq!(rect.right(), minimap.right());
        }
        assert_eq!(background_rects(FIND).len(), 1);
    }
}
