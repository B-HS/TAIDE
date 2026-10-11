#![cfg(all(feature = "native-host", feature = "inspection"))]

use std::sync::Arc;

use egui::{Color32, Context, Event, FontId, FullOutput, Modifiers, RawInput, Rect, Ui};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_code_lenses::{Colors, Item, Presentation};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorOutput, EditorPresentation, EditorRequest, NativeEditor,
};

const TEXT: &str = "  call(\u{1f600}value)\nnext\nlast";
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const FRAME_STEP: f64 = 0.1;
const BYTE_LIMIT: usize = 4096;
const SCREEN: egui::Vec2 = egui::vec2(800.0, 600.0);
const LENS_HEIGHT: f32 = 17.0;
const REVEAL_LINE: f64 = 1.0;
const REVEAL_COLUMN: f64 = 8.0;

struct Fixture {
    context: Context,
    store: EditorStore,
    document: DocumentId,
    view: ViewId,
    editor: NativeEditor,
    lenses: Presentation,
    presentation: EditorPresentation,
    time: f64,
    screen: egui::Vec2,
    reveal_position: (f64, f64),
}

fn painted_text(output: &FullOutput) -> Vec<(String, f32, Color32)> {
    output
        .shapes
        .iter()
        .filter_map(|shape| {
            if let egui::Shape::Text(text) = &shape.shape {
                Some((
                    text.galley.text().to_owned(),
                    text.galley.job.sections.first()?.format.font_id.size,
                    text.fallback_color,
                ))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn 실제_lens는_줄_위_그룹과_들여쓰기_글꼴_아이콘_구분자를_표시하고_원문을_보존한다() {
    let mut fixture = Fixture::new();
    let before = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    let (shown, painted) = fixture.frame(Vec::new(), false);
    assert_eq!(shown.code_lenses.geometry.zones.len(), 3);
    assert_eq!(shown.code_lenses.geometry.commands.len(), 3);
    assert_eq!(shown.code_lenses.geometry.zones[0].height(), LENS_HEIGHT);
    assert_eq!(
        shown.geometry.caret_rect(0).unwrap().top(),
        shown.geometry.rect.top() + LENS_HEIGHT
    );
    let first = shown.code_lenses.geometry.commands[0].1;
    assert_eq!(
        first.left(),
        shown
            .geometry
            .caret_rect(TEXT.find("call").unwrap())
            .unwrap()
            .left()
    );
    for zone in &shown.code_lenses.geometry.zones {
        assert_eq!(shown.geometry.byte_at(zone.center()), None);
        assert_eq!(shown.geometry.content_byte_at(zone.center()), None);
    }
    let texts = painted_text(&painted);
    for text in [
        "\u{eb2c}",
        " Run",
        "3 refs",
        "\u{a0}|\u{a0}",
        "$(run)",
        " literal",
    ] {
        assert!(
            texts
                .iter()
                .any(|(current, size, _)| current == text && *size == 12.0),
            "{text}: {texts:?}"
        );
    }
    let after = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    assert_eq!(
        (after.rope, after.revision, after.dirty),
        (before.rope, before.revision, before.dirty)
    );
}

#[test]
fn lens_클릭과_빈_zone은_본문_선택을_바꾸지_않고_hover_색과_명령_인덱스를_반환한다() {
    let mut fixture = Fixture::new();
    let (shown, _) = fixture.frame(Vec::new(), false);
    let before = fixture
        .store
        .views()
        .get(fixture.view)
        .unwrap()
        .selection
        .clone();
    let command = shown.code_lenses.geometry.commands[0].1;
    let mut clicked = None;
    for pressed in [true, false] {
        let (shown, painted) = fixture.frame(
            vec![
                Event::PointerMoved(command.center()),
                Event::PointerButton {
                    pos: command.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            false,
        );
        clicked = clicked.or(shown.code_lenses.clicked);
        assert!(
            painted_text(&painted)
                .iter()
                .any(|(text, _, color)| text == " Run" && *color == Color32::RED)
        );
    }
    assert_eq!(clicked, Some(0));
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().selection,
        before
    );
    let (shown, _) = fixture.frame(Vec::new(), false);
    let blank = shown.code_lenses.geometry.zones[1].center();
    for pressed in [true, false] {
        let (shown, _) = fixture.frame(
            vec![
                Event::PointerMoved(blank),
                Event::PointerButton {
                    pos: blank,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            false,
        );
        assert_eq!(shown.code_lenses.clicked, None);
    }
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().selection,
        before
    );
}

#[test]
fn 접기와_wrap_reveal은_lens_공간을_보존하고_같은_프레임_편집은_오래된_표시를_제거한다() {
    let mut fixture = Fixture::new();
    fixture.presentation.options.folding = true;
    fixture.presentation.options.word_wrap = true;
    let start = TEXT.find("next").unwrap();
    let end = TEXT.find("last").unwrap() - 1;
    fixture.screen.x = 120.0;
    let current = fixture.store.views().get(fixture.view).unwrap().clone();
    fixture
        .store
        .set_view_state(
            fixture.view,
            current.selection,
            current.scroll,
            vec![start..end],
        )
        .unwrap();
    let (shown, _) = fixture.frame(Vec::new(), true);
    assert_eq!(shown.code_lenses.geometry.zones.len(), 2);
    assert!(shown.geometry.visible_rows.len() > 2);
    assert_eq!(shown.code_lenses.geometry.commands.len(), 3);
    fixture.frame(Vec::new(), false);
    let (shown, painted) = fixture.frame(vec![Event::Text("X".into())], false);
    assert!(shown.changed);
    assert!(shown.code_lenses.geometry.zones.is_empty());
    assert!(shown.code_lenses.geometry.commands.is_empty());
    assert!(
        !painted_text(&painted)
            .iter()
            .any(|(text, _, _)| text == "3 refs")
    );
    let snapshot = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    assert!(snapshot.rope.to_string().contains('X'));
    assert!(!snapshot.rope.to_string().contains("Run"));
}

impl Fixture {
    fn new() -> Self {
        let context = Context::default();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 4,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), TEXT, "rust".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "code-lens-fixture".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        let lenses = Presentation {
            document,
            revision: store.documents().snapshot(document).unwrap().revision,
            items: Arc::from([
                Item {
                    anchor: 0,
                    title: Some("$(run) Run".into()),
                },
                Item {
                    anchor: TEXT.find("call").unwrap(),
                    title: Some("3   refs".into()),
                },
                Item {
                    anchor: TEXT.find("next").unwrap(),
                    title: None,
                },
                Item {
                    anchor: TEXT.find("last").unwrap(),
                    title: Some("\\$(run) literal".into()),
                },
            ]),
            colors: Colors {
                foreground: Color32::GRAY,
                hover: Color32::RED,
            },
        };
        Self {
            context,
            store,
            document,
            view,
            lenses,
            presentation: EditorPresentation::default(),
            time: 0.0,
            screen: SCREEN,
            reveal_position: (REVEAL_LINE, REVEAL_COLUMN),
            editor: NativeEditor {
                appearance: EditorAppearance {
                    font: FontId::monospace(FONT_SIZE),
                    line_height: LINE_HEIGHT,
                    horizontal_padding: 0.0,
                    background: Color32::BLACK,
                    foreground: Color32::WHITE,
                    muted: Color32::GRAY,
                    selection: Color32::BLUE,
                    cursor: Color32::WHITE,
                    current_line: Color32::TRANSPARENT,
                    line_numbers: false,
                    indent: "    ".into(),
                },
            },
        }
    }

    fn frame(&mut self, events: Vec<Event>, reveal: bool) -> (EditorOutput, FullOutput) {
        self.time += FRAME_STEP;
        let mut shown = None;
        let presentation = &self.presentation;
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, self.screen)),
                time: Some(self.time),
                events,
                focused: true,
                ..Default::default()
            },
            |ui| {
                if reveal {
                    self.editor
                        .reveal_presented(
                            ui,
                            &mut self.store,
                            self.view,
                            self.reveal_position.0,
                            self.reveal_position.1,
                            presentation,
                        )
                        .unwrap();
                }
                shown = Some(
                    self.editor
                        .show_request_with_code_lenses(
                            ui,
                            &mut self.store,
                            self.view,
                            EditorRequest {
                                request_focus: self.time == FRAME_STEP,
                                keymap: |_: &Ui, _: &Event, _: bool| false,
                                route: |response: &egui::Response| {
                                    response.ctx.keyboard_input_route(response.id)
                                },
                                presentation,
                                tokens: |_: &EditorStore| None,
                                language: None,
                                decorations: &[],
                                fold_commands: &[],
                                fold_controls: None,
                                problems: None,
                                locations: None,
                                documentation: None,
                                documentation_commands: &[],
                                completion: None,
                                completion_commands: &[],
                                syntax_folds: None,
                            },
                            |_, _, _, _, _| false,
                            None,
                            None,
                            None,
                            Some(self.lenses.clone()),
                        )
                        .unwrap(),
                );
            },
        );
        output.textures_delta.clear();
        let shown = shown.unwrap();
        assert!(shown.errors.is_empty());
        (shown, output)
    }
}

#[test]
fn 기존_lens를_보관한_reveal은_스크롤_계산에_모든_줄_위_공간을_포함한다() {
    const VIEWPORT_HEIGHT: f32 = 80.0;
    const VIEWPORT_WIDTH: f32 = 120.0;
    let mut fixture = Fixture::new();
    fixture.screen = egui::vec2(VIEWPORT_WIDTH, VIEWPORT_HEIGHT);
    fixture.presentation.options.word_wrap = true;
    fixture.presentation.options.smooth_scrolling = false;
    fixture.frame(Vec::new(), false);
    fixture.reveal_position = (3.0, 1.0);
    let (shown, _) = fixture.frame(Vec::new(), true);
    let caret = shown
        .geometry
        .caret_rect(TEXT.find("last").unwrap())
        .unwrap();
    assert!(caret.top() >= shown.geometry.rect.top());
    assert!(caret.bottom() <= shown.geometry.rect.bottom());
    assert!(shown.geometry.scroll.y > 0.0);
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
}
