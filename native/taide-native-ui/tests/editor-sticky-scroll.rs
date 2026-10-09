#![cfg(feature = "native-host")]

use std::sync::Arc;

use egui::epaint::{ClippedShape, Shape};
use egui::{
    Color32, Context, Event, FontId, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2,
    pos2, vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness,
};
use taide_native_editor::line_tokens::{LineTokens, TokenStyle, TokenStyleTable};
use taide_native_editor::sticky_model::{StickyModel, StickyScope};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::syntax::TokenKind;
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_native_ui::editor_sticky_scroll::EditorStickyColors;
use taide_native_ui::editor_surface::{
    CursorBlinking, EditorAppearance, EditorDisplayOptions, EditorPresentation, EditorRequest,
    EditorTokens, NativeEditor,
};

const SCREEN: Vec2 = vec2(400.0, 240.0);
const LINE_HEIGHT: f32 = 20.0;
const FONT_SIZE: f32 = 14.0;
const BYTE_LIMIT: usize = 1024 * 1024;
const HISTORY_LIMIT: usize = 8;
const BACKGROUND: Color32 = Color32::from_rgb(21, 31, 41);
const BORDER: Color32 = Color32::from_rgb(51, 61, 71);
const HOVER: Color32 = Color32::from_rgb(81, 91, 101);
const SCROLL_TOP: f32 = 100.0;
const CLICK_COLUMN: usize = 2;
const POINTER_LINE_OFFSET: f32 = 5.0;

struct Fixture {
    context: Context,
    editor: NativeEditor,
    store: EditorStore,
    view: ViewId,
    presentation: EditorPresentation,
    screen: Vec2,
    foreign: Option<String>,
    tokens: Option<(u64, LineTokens, TokenStyleTable)>,
    layers: Vec<DecorationLayer>,
    syntax_folds: Option<Arc<taide_native_editor::syntax_folding::SyntaxFolds>>,
}

impl Fixture {
    fn new(text: &str, scopes: &[StickyScope]) -> Self {
        Self::admitted(text, scopes, FileSizeTier::Normal, false)
    }

    fn admitted(text: &str, scopes: &[StickyScope], tier: FileSizeTier, read_only: bool) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 2,
            max_views: 2,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = store
            .open_file(
                "/synthetic/sticky.txt".into(),
                OpenedFile {
                    path: "/synthetic/sticky.txt".into(),
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
                    window: "sticky-test".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
            )
            .unwrap();
        let snapshot = store.documents().snapshot(document).unwrap();
        let model = StickyModel::new(&snapshot, scopes);
        let mut state = store.views().get(view).unwrap().clone();
        state.scroll.y = SCROLL_TOP;
        let byte = snapshot.rope.line_to_byte(10);
        state.selection = SelectionSet {
            selections: vec![Selection {
                anchor: byte,
                head: byte,
            }],
            primary: 0,
        };
        store
            .set_view_state(view, state.selection, state.scroll, state.folds)
            .unwrap();
        Self {
            context: Context::default(),
            editor: NativeEditor {
                appearance: EditorAppearance {
                    font: FontId::monospace(FONT_SIZE),
                    line_height: LINE_HEIGHT,
                    horizontal_padding: 8.0,
                    background: Color32::BLACK,
                    foreground: Color32::WHITE,
                    muted: Color32::GRAY,
                    selection: Color32::BLUE,
                    cursor: Color32::YELLOW,
                    current_line: Color32::DARK_GRAY,
                    line_numbers: false,
                    indent: "    ".into(),
                },
            },
            store,
            view,
            screen: SCREEN,
            foreign: None,
            tokens: None,
            layers: Vec::new(),
            syntax_folds: None,
            presentation: EditorPresentation {
                options: EditorDisplayOptions {
                    sticky_scroll: true,
                    sticky_colors: Some(EditorStickyColors {
                        background: BACKGROUND,
                        border: BORDER,
                        hover: HOVER,
                        shadow: Color32::TRANSPARENT,
                    }),
                    sticky_model: Some(Arc::new(model)),
                    cursor_blinking: CursorBlinking::Solid,
                    ..Default::default()
                },
            },
        }
    }

    fn show(&mut self, time: f64, events: Vec<Event>, modifiers: Modifiers) -> Shown {
        let mut geometry = None;
        let mut id = None;
        let mut foreign_rect = None;
        let mut toggle = false;
        let mut controls = Vec::new();
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.screen)),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                ui.input_mut(|input| input.modifiers = modifiers);
                let layers = self.layers.iter().collect::<Vec<_>>();
                let mut paint =
                    |_: &egui::Ui, control: taide_native_ui::editor_surface::FoldControl| {
                        controls.push((control.rect, control.color))
                    };
                let mut editor_ui =
                    ui.new_child(egui::UiBuilder::new().id_salt("sticky-editor").max_rect(
                        Rect::from_min_size(Pos2::ZERO, vec2(SCREEN.x, self.screen.y)),
                    ));
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
                            decorations: &layers,
                            fold_commands: &[],
                            #[cfg(feature = "native-host")]
                            syntax_folds: self.syntax_folds.clone(),
                            #[cfg(feature = "native-host")]
                            documentation: None,
                            #[cfg(feature = "native-host")]
                            documentation_commands: &[],
                            fold_controls: Some(&mut paint),
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
                toggle = shown.toggle_sticky_scroll;
                if let Some(text) = self.foreign.as_mut() {
                    let mut foreign_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("sticky-foreign-pane")
                            .max_rect(Rect::from_min_max(
                                pos2(SCREEN.x, 0.0),
                                pos2(self.screen.x, self.screen.y),
                            )),
                    );
                    let shown = egui::TextEdit::singleline(text)
                        .id(Id::new("sticky-foreign-input"))
                        .show(&mut foreign_ui);
                    ui.ctx().register_pointer_keyboard_focus(shown.response.id);
                    foreign_rect = Some(shown.response.rect);
                }
            },
        );
        output.textures_delta.clear();
        Shown {
            shapes: output.shapes,
            geometry: geometry.unwrap(),
            id: id.unwrap(),
            foreign_rect,
            toggle,
            controls,
        }
    }

    fn text(&self) -> String {
        let document = self.store.views().get(self.view).unwrap().document;
        self.store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
    }

    fn head(&self) -> usize {
        let selection = &self.store.views().get(self.view).unwrap().selection;
        selection.selections[selection.primary].head
    }
}

struct Shown {
    shapes: Vec<ClippedShape>,
    geometry: EditorGeometry,
    id: Id,
    foreign_rect: Option<Rect>,
    toggle: bool,
    controls: Vec<(Rect, Color32)>,
}

fn document() -> String {
    format!(
        "outer\n{}\nlast",
        (1..30)
            .map(|line| format!("    body{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn click(position: Pos2, modifiers: Modifiers) -> Vec<Event> {
    vec![
        Event::PointerMoved(position),
        Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: true,
            modifiers,
        },
        Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers,
        },
    ]
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn sticky_background(shown: &Shown) -> Option<Rect> {
    shown.shapes.iter().find_map(|shape| match &shape.shape {
        Shape::Rect(rect) if rect.fill == BACKGROUND => Some(rect.rect),
        _ => None,
    })
}

fn sticky_text(shown: &Shown) -> Vec<String> {
    let Some(rect) = sticky_background(shown) else {
        return Vec::new();
    };
    shown
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Text(text)
                if rect.y_range().contains(text.pos.y)
                    && shape.clip_rect.top() >= rect.top()
                    && shape.clip_rect.bottom() <= rect.bottom() =>
            {
                Some(text.galley.job.text.clone())
            }
            _ => None,
        })
        .collect()
}

fn scroll(fixture: &mut Fixture, top: f32) {
    let mut state = fixture.store.views().get(fixture.view).unwrap().clone();
    state.scroll.y = top;
    fixture
        .store
        .set_view_state(fixture.view, state.selection, state.scroll, state.folds)
        .unwrap();
}

#[test]
fn 고정_줄_클릭과_같은_입력_묶음의_문자는_클릭한_열에_입력된다() {
    let mut fixture = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 30,
        }],
    );
    let initial = fixture.show(0.0, vec![], Modifiers::NONE);
    assert!(
        initial
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape, Shape::Rect(rect) if rect.fill == BACKGROUND))
    );
    fixture.show(0.1, vec![], Modifiers::NONE);
    let width = fixture.context.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap("ou".into(), FontId::monospace(FONT_SIZE), Color32::WHITE)
            .size()
            .x
    });
    let position = pos2(
        initial.geometry.content_rect.left() + width,
        initial.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    let mut events = click(position, Modifiers::NONE);
    events.push(Event::Text("X".into()));
    fixture.show(0.2, events, Modifiers::NONE);
    assert!(fixture.text().starts_with("ouXter\n"), "{}", fixture.text());
    assert_eq!(fixture.head(), CLICK_COLUMN + 1);
}

#[test]
fn 고정_줄_키_이동과_enter_escape는_본문_선택과_뒤따르는_문자를_보존한다() {
    let scopes = [
        StickyScope {
            start_line: 0,
            end_line: 30,
        },
        StickyScope {
            start_line: 1,
            end_line: 25,
        },
    ];
    let mut fixture = Fixture::new(&document(), &scopes);
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    let first = shown.id.with(("sticky-line", 0usize));
    let last = shown.id.with(("sticky-line", 1usize));
    let original = fixture.head();
    fixture
        .context
        .memory_mut(|memory| memory.request_focus(last));
    fixture.show(0.1, vec![], Modifiers::NONE);
    fixture.show(0.2, vec![key(Key::ArrowUp)], Modifiers::NONE);
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(first)
    );
    assert_eq!(fixture.head(), original);
    fixture.show(
        0.3,
        vec![
            key(Key::ArrowDown),
            key(Key::Enter),
            Event::Text("X".into()),
        ],
        Modifiers::NONE,
    );
    assert!(fixture.text().starts_with("outer\nX    body1\n"));
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(shown.id)
    );
    scroll(&mut fixture, SCROLL_TOP);
    fixture.show(0.4, vec![], Modifiers::NONE);
    fixture
        .context
        .memory_mut(|memory| memory.request_focus(first));
    fixture.show(0.5, vec![], Modifiers::NONE);
    let before = fixture.head();
    fixture.show(
        0.6,
        vec![key(Key::Escape), Event::Text("Y".into())],
        Modifiers::NONE,
    );
    assert_eq!(fixture.head(), before + 1);
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(shown.id)
    );
}

#[test]
fn shift_hover와_클릭은_종료_줄을_표시하고_그_줄의_처음으로_이동한다() {
    let mut fixture = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 30,
        }],
    );
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    let position = pos2(
        shown.geometry.content_rect.left(),
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    let preview = fixture.show(0.1, vec![Event::PointerMoved(position)], Modifiers::SHIFT);
    assert_eq!(sticky_text(&preview), vec!["last"]);
    fixture.show(0.2, click(position, Modifiers::SHIFT), Modifiers::SHIFT);
    let document = fixture.store.views().get(fixture.view).unwrap().document;
    let snapshot = fixture.store.documents().snapshot(document).unwrap();
    assert_eq!(fixture.head(), snapshot.rope.line_to_byte(30));
    assert!(fixture.store.views().get(fixture.view).unwrap().scroll.y > SCROLL_TOP);
}

#[test]
fn 고정_줄의_최대_높이와_마지막_밀림은_실제_화면과_clip에_적용된다() {
    let scopes = (0..7)
        .map(|start_line| StickyScope {
            start_line,
            end_line: 30 - start_line,
        })
        .collect::<Vec<_>>();
    let mut fixture = Fixture::new(&document(), &scopes);
    scroll(&mut fixture, LINE_HEIGHT * 10.0);
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    assert_eq!(
        sticky_background(&shown).unwrap().height(),
        LINE_HEIGHT * 3.0
    );
    fixture.screen.y = LINE_HEIGHT * 25.0;
    let taller = fixture.show(0.1, vec![], Modifiers::NONE);
    assert_eq!(
        sticky_background(&taller).unwrap().height(),
        LINE_HEIGHT * 5.0
    );
    fixture.screen.y = LINE_HEIGHT;
    assert!(sticky_background(&fixture.show(0.2, vec![], Modifiers::NONE)).is_none());
    let mut pushed = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 15,
        }],
    );
    scroll(&mut pushed, LINE_HEIGHT * 15.0 - LINE_HEIGHT / 2.0);
    let shown = pushed.show(0.0, vec![], Modifiers::NONE);
    let rect = sticky_background(&shown).unwrap();
    assert_eq!(rect.height(), LINE_HEIGHT / 2.0);
    assert!(
        shown
            .shapes
            .iter()
            .filter(
                |shape| matches!(&shape.shape, Shape::Text(text) if text.galley.job.text == "outer")
            )
            .all(|shape| shape.clip_rect.top() == rect.top()
                && shape.clip_rect.bottom() == rect.bottom())
    );
}

#[test]
fn 접기_설정과_독립된_fallback과_옵션_종료는_이전_고정_포커스를_회수한다() {
    let mut fixture = Fixture::new(&document(), &[]);
    fixture.presentation.options.sticky_model = None;
    fixture.presentation.options.folding = false;
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    assert_eq!(sticky_text(&shown), vec!["outer"]);
    fixture
        .context
        .memory_mut(|memory| memory.request_focus(shown.id.with(("sticky-line", 0usize))));
    fixture.show(0.1, vec![], Modifiers::NONE);
    let before = fixture.head();
    fixture.presentation.options.sticky_scroll = false;
    let disabled = fixture.show(0.2, vec![Event::Text("X".into())], Modifiers::NONE);
    assert!(sticky_background(&disabled).is_none());
    assert_eq!(fixture.head(), before + 1);
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(shown.id)
    );
    assert!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .folds
            .is_empty()
    );
}

#[test]
fn 실제_문서_심볼은_선택_시작줄을_고정하고_빈응답과_편집은_접기_fallback을_사용한다() {
    use taide_native_editor::document_symbols::DocumentSymbols;
    let mut fixture = Fixture::new(&document(), &[]);
    let id = fixture.store.views().get(fixture.view).unwrap().document;
    let snapshot = fixture.store.documents().snapshot(id).unwrap();
    let uri = "file:///synthetic/sticky.txt".parse().unwrap();
    let response = serde_json::from_value(serde_json::json!([{"name":"scope", "kind":5, "range":{"start":{"line":0,"character":0},"end":{"line":25,"character":9}}, "selectionRange":{"start":{"line":2,"character":4},"end":{"line":2,"character":9}}}])).unwrap();
    let symbols = DocumentSymbols::new(&snapshot, &uri, Some(response)).unwrap();
    fixture.presentation.options.sticky_model = symbols.sticky_model().cloned();
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    assert_eq!(sticky_text(&shown), ["    body2"]);
    let empty = DocumentSymbols::new(&snapshot, &uri, None).unwrap();
    fixture.presentation.options.sticky_model = empty.sticky_model().cloned();
    let fallback = fixture.show(0.1, vec![], Modifiers::NONE);
    assert_eq!(sticky_text(&fallback), ["outer"]);
    fixture.presentation.options.sticky_model = symbols.sticky_model().cloned();
    let view = fixture.store.views().get(fixture.view).unwrap().clone();
    taide_native_editor::editing::type_text(&mut fixture.store, view.id, "X").unwrap();
    let current = fixture.store.documents().snapshot(id).unwrap();
    assert!(!symbols.describes(&current));
    let invalidated = fixture.show(0.2, vec![], Modifiers::NONE);
    assert_eq!(sticky_text(&invalidated), ["outer"]);
}

#[test]
fn 고정_줄은_아웃라인_구문_들여쓰기_순서와_공급_cache_교체를_따른다() {
    use taide_native_editor::folding::FoldRegion;
    use taide_native_editor::syntax_folding::{SyntaxFoldRange, SyntaxFolds};
    let mut fixture = Fixture::new(&document(), &[]);
    let id = fixture.store.views().get(fixture.view).unwrap().document;
    let snapshot = fixture.store.documents().snapshot(id).unwrap();
    let syntax = Arc::new(SyntaxFolds::new(
        &snapshot,
        vec![vec![SyntaxFoldRange {
            region: FoldRegion {
                start_line: 2,
                end_line: 25,
            },
            kind: None,
        }]],
    ));
    fixture.presentation.options.sticky_model = None;
    fixture.syntax_folds = Some(syntax);
    let supplied = fixture.show(0.0, vec![], Modifiers::NONE);
    assert_eq!(sticky_text(&supplied), ["    body2"]);
    fixture.presentation.options.sticky_model = Some(Arc::new(StickyModel::new(
        &snapshot,
        &[StickyScope {
            start_line: 1,
            end_line: 26,
        }],
    )));
    let outline = fixture.show(0.1, vec![], Modifiers::NONE);
    assert_eq!(sticky_text(&outline), ["    body1"]);
    fixture.presentation.options.sticky_model = None;
    fixture.syntax_folds = Some(Arc::new(SyntaxFolds::new(&snapshot, vec![vec![]])));
    assert!(sticky_text(&fixture.show(0.2, vec![], Modifiers::NONE)).is_empty());
    fixture.syntax_folds = None;
    assert_eq!(
        sticky_text(&fixture.show(0.3, vec![], Modifiers::NONE)),
        ["outer"]
    );
}

#[test]
fn 대형_문서는_고정_줄을_끄고_정상_읽기전용은_문자_변경없이_탐색한다() {
    let scopes = [StickyScope {
        start_line: 0,
        end_line: 30,
    }];
    for tier in [FileSizeTier::Large, FileSizeTier::ReadOnly] {
        let mut fixture = Fixture::admitted(&document(), &scopes, tier, true);
        assert!(sticky_background(&fixture.show(0.0, vec![], Modifiers::NONE)).is_none());
    }
    let mut fixture = Fixture::admitted(&document(), &scopes, FileSizeTier::Normal, true);
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    fixture.show(0.1, vec![], Modifiers::NONE);
    let position = pos2(
        shown.geometry.content_rect.left(),
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    fixture.show(0.2, click(position, Modifiers::NONE), Modifiers::NONE);
    assert_eq!(fixture.head(), 0);
    assert_eq!(fixture.text(), document());
}

#[test]
fn 고정_줄_위의_휠은_본문_선택을_바꾸지_않고_스크롤한다() {
    let mut fixture = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 30,
        }],
    );
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    let position = pos2(
        shown.geometry.content_rect.left(),
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    fixture.show(0.1, vec![Event::PointerMoved(position)], Modifiers::NONE);
    let before = fixture.head();
    let shown = fixture.show(
        0.2,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: vec2(0.0, -1.0),
            phase: egui::TouchPhase::Move,
            modifiers: Modifiers::NONE,
        }],
        Modifiers::NONE,
    );
    assert!(shown.geometry.scroll.y > SCROLL_TOP);
    assert_eq!(fixture.head(), before);
}

#[test]
fn 고정_줄_클릭_다음_다른_입력창을_클릭하면_문자와_최종_포커스는_그_창에_남는다() {
    const FOREIGN_WIDTH: f32 = 160.0;
    let mut fixture = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 30,
        }],
    );
    fixture.foreign = Some(String::new());
    fixture.screen.x += FOREIGN_WIDTH;
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    fixture.show(0.1, vec![], Modifiers::NONE);
    let sticky = pos2(
        shown.geometry.content_rect.left(),
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    let foreign = shown.foreign_rect.unwrap().center();
    let mut events = click(sticky, Modifiers::NONE);
    events.extend(click(foreign, Modifiers::NONE));
    events.push(Event::Text("X".into()));
    fixture.show(0.2, events, Modifiers::NONE);
    assert_eq!(fixture.text(), document());
    assert_eq!(fixture.foreign.as_deref(), Some("X"));
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(Id::new("sticky-foreign-input"))
    );
}

#[test]
fn 고정_줄은_실제_토큰과_인라인_장식_색을_쓰고_줄_배경과_테마_변경을_구별한다() {
    const TOKEN: Color32 = Color32::from_rgb(121, 131, 141);
    const INLINE: Color32 = Color32::from_rgb(151, 161, 171);
    const LINE_BACKGROUND: Color32 = Color32::from_rgb(181, 191, 201);
    let mut fixture = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 30,
        }],
    );
    let style = TokenStyle {
        foreground: TOKEN.to_srgba_unmultiplied(),
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: TokenKind::Other,
    };
    let mut lines = LineTokens::new(31);
    lines.set_line(0, vec![0, 0], false);
    fixture.tokens = Some((0, lines, TokenStyleTable::new(style, vec![style])));
    fixture.layers = vec![DecorationLayer::new(
        0,
        1,
        vec![
            Decoration {
                bytes: 1..3,
                kind: DecorationKind::Inline(InlineStyle {
                    foreground: Some(INLINE.to_srgba_unmultiplied()),
                    ..Default::default()
                }),
                stickiness: Stickiness::default(),
            },
            Decoration {
                bytes: 0..5,
                kind: DecorationKind::LineBackground(LINE_BACKGROUND.to_srgba_unmultiplied()),
                stickiness: Stickiness::default(),
            },
        ],
    )];
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    let header = shown
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.job.text == "outer" => Some(text),
            _ => None,
        })
        .unwrap();
    let colored = header
        .galley
        .job
        .sections
        .iter()
        .flat_map(|section| {
            header.galley.job.text[section.byte_range.start.0..section.byte_range.end.0]
                .chars()
                .map(|character| (character, section.format.color))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        colored,
        vec![
            ('o', TOKEN),
            ('u', INLINE),
            ('t', INLINE),
            ('e', TOKEN),
            ('r', TOKEN)
        ]
    );
    assert!(
        !shown
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape, Shape::Rect(rect) if rect.fill == LINE_BACKGROUND))
    );
    fixture
        .presentation
        .options
        .sticky_colors
        .as_mut()
        .unwrap()
        .background = Color32::DARK_RED;
    let changed = fixture.show(0.1, vec![], Modifiers::NONE);
    assert!(sticky_background(&changed).is_none());
    assert!(
        changed.shapes.iter().any(
            |shape| matches!(&shape.shape, Shape::Rect(rect) if rect.fill == Color32::DARK_RED)
        )
    );
}

#[test]
fn 줄_나눔은_첫_표시_줄만_고정하고_탭_unicode와_가로_스크롤_clip을_보존한다() {
    let header = "\touter 한글 ".repeat(20);
    let text = document().replacen("outer", &header, 1);
    let scopes = [StickyScope {
        start_line: 0,
        end_line: 30,
    }];
    let mut fixture = Fixture::new(&text, &scopes);
    fixture.presentation.options.word_wrap = true;
    let wrapped = fixture.show(0.0, vec![], Modifiers::NONE);
    let texts = sticky_text(&wrapped);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].starts_with("    outer 한글"));
    assert!(texts[0].len() < header.len());
    fixture.presentation.options.word_wrap = false;
    fixture.show(0.1, vec![], Modifiers::NONE);
    let mut state = fixture.store.views().get(fixture.view).unwrap().clone();
    const HORIZONTAL_SCROLL: f32 = 40.0;
    state.scroll.x = HORIZONTAL_SCROLL;
    state.scroll.y = SCROLL_TOP;
    fixture
        .store
        .set_view_state(fixture.view, state.selection, state.scroll, state.folds)
        .unwrap();
    let horizontal = fixture.show(0.2, vec![], Modifiers::NONE);
    let text = horizontal
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text)
                if text.galley.job.text.starts_with("    outer 한글")
                    && shape.clip_rect.bottom() <= horizontal.geometry.rect.top() + LINE_HEIGHT =>
            {
                Some((text, shape.clip_rect))
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        text.0.pos.x,
        horizontal.geometry.content_rect.left() - HORIZONTAL_SCROLL
    );
    assert_eq!(text.1.left(), horizontal.geometry.content_rect.left());
    fixture.show(0.3, vec![], Modifiers::NONE);
    let click_at = pos2(
        horizontal.geometry.content_rect.left() + POINTER_LINE_OFFSET,
        horizontal.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    fixture.show(0.4, click(click_at, Modifiers::NONE), Modifiers::NONE);
    assert!(fixture.head() > 1);
    assert!(fixture.text().is_char_boundary(fixture.head()));
}

#[test]
fn 고정_줄_접기_버튼은_기존_접기를_바꾸고_헤더_높이에_맞춰_스크롤한다() {
    let mut fixture = Fixture::new(&document(), &[]);
    fixture.presentation.options.sticky_model = None;
    fixture.presentation.options.folding = true;
    fixture.presentation.options.scroll_beyond_last_line = true;
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    fixture.show(0.1, vec![], Modifiers::NONE);
    let position = pos2(
        shown.geometry.content_rect.left() - POINTER_LINE_OFFSET,
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    let before = fixture.head();
    fixture.show(0.2, click(position, Modifiers::NONE), Modifiers::NONE);
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().folds.len(),
        1
    );
    assert_ne!(fixture.head(), before);
    assert_eq!(fixture.head(), "outer".len());
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(shown.id)
    );
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().scroll.y,
        1.0
    );
    fixture.show(0.3, vec![], Modifiers::NONE);
    fixture.show(0.4, click(position, Modifiers::NONE), Modifiers::NONE);
    assert!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .folds
            .is_empty()
    );
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().scroll.y,
        1.0
    );
}

#[test]
fn 같은_입력_묶음에서_문서가_변해도_고정_줄의_이전_글자_위치는_유효하게_추적된다() {
    let mut fixture = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 30,
        }],
    );
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    let state = fixture.store.views().get(fixture.view).unwrap().clone();
    fixture
        .store
        .set_view_state(
            fixture.view,
            SelectionSet {
                selections: vec![Selection { anchor: 0, head: 0 }],
                primary: 0,
            },
            state.scroll,
            state.folds,
        )
        .unwrap();
    fixture.show(0.1, vec![], Modifiers::NONE);
    let width = fixture.context.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap("o".into(), FontId::monospace(FONT_SIZE), Color32::WHITE)
            .size()
            .x
    });
    let position = pos2(
        shown.geometry.content_rect.left() + width,
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    let mut events = vec![Event::Text("é".into())];
    events.extend(click(position, Modifiers::NONE));
    events.push(Event::Text("X".into()));
    fixture.show(0.2, events, Modifiers::NONE);
    assert!(fixture.text().starts_with("éoXuter\n"));
    assert_eq!(fixture.head(), "éoX".len());
}

#[test]
fn 고정_줄의_문맥_메뉴는_기존_설정_토글을_한번_요청하고_본문을_바꾸지_않는다() {
    let mut fixture = Fixture::new(
        &document(),
        &[StickyScope {
            start_line: 0,
            end_line: 30,
        }],
    );
    fixture.presentation.options.sticky_toggle_label = Some("Sticky Scroll".into());
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    fixture.show(0.1, vec![], Modifiers::NONE);
    let position = pos2(
        shown.geometry.content_rect.left(),
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    let events = [true, false]
        .into_iter()
        .map(|pressed| Event::PointerButton {
            pos: position,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        })
        .collect();
    fixture.show(0.2, events, Modifiers::NONE);
    let menu = fixture.show(0.3, vec![], Modifiers::NONE);
    let label = menu
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text) if text.galley.job.text == "Sticky Scroll" => {
                Some(Rect::from_min_size(text.pos, text.galley.size()).center())
            }
            _ => None,
        })
        .unwrap();
    assert!(
        fixture
            .show(0.4, click(label, Modifiers::NONE), Modifiers::NONE)
            .toggle
    );
    assert!(!fixture.show(0.5, vec![], Modifiers::NONE).toggle);
    assert_eq!(fixture.text(), document());
}

#[test]
fn 접기_아이콘은_고정_줄_gutter에서_표시되고_마우스가_떠나면_사라진다() {
    const FADE_DONE: f64 = 0.6;
    let mut fixture = Fixture::new(&document(), &[]);
    fixture.presentation.options.sticky_model = None;
    fixture.presentation.options.folding = true;
    let shown = fixture.show(0.0, vec![], Modifiers::NONE);
    let gutter = pos2(
        shown.geometry.content_rect.left() - POINTER_LINE_OFFSET,
        shown.geometry.rect.top() + POINTER_LINE_OFFSET,
    );
    let text = pos2(
        shown.geometry.content_rect.left() + POINTER_LINE_OFFSET,
        gutter.y,
    );
    assert!(
        fixture
            .show(0.1, vec![Event::PointerMoved(text)], Modifiers::NONE)
            .controls
            .is_empty()
    );
    fixture.show(0.2, vec![Event::PointerMoved(gutter)], Modifiers::NONE);
    assert_eq!(
        fixture
            .show(0.2 + FADE_DONE, vec![], Modifiers::NONE)
            .controls
            .len(),
        1
    );
    assert_eq!(
        fixture
            .show(0.9, vec![Event::PointerMoved(text)], Modifiers::NONE)
            .controls
            .len(),
        1
    );
    assert!(
        fixture
            .show(0.9 + FADE_DONE, vec![], Modifiers::NONE)
            .controls
            .is_empty()
    );
}

#[test]
fn 명령_토글은_세션_메모리에만_남고_실제_설정_변경과_새_세션을_따른다() {
    use taide_native_ui::editor_sticky_scroll::StickySetting;
    let configured = true;
    let mut first = StickySetting::new(configured);
    first.toggle(configured);
    assert!(!first.synchronize(configured));
    let mut independent = StickySetting::new(configured);
    assert!(independent.synchronize(configured));
    first.toggle(configured);
    first.toggle(configured);
    assert!(!first.synchronize(configured));
    assert!(!first.synchronize(false));
    assert!(first.synchronize(true));
    assert!(StickySetting::new(configured).synchronize(configured));
}
