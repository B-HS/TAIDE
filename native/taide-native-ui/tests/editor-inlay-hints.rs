#![cfg(all(feature = "native-host", feature = "inspection"))]

use std::sync::Arc;

use egui::{Color32, Context, Event, FontId, FullOutput, Modifiers, RawInput, Rect, Ui};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::DocumentId;
use taide_native_editor::inlay_hints::{Hint, Kind};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_inlay_hints::{self, Colors, Presentation, Style};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorOutput, EditorPresentation, EditorRequest, NativeEditor,
};

const TEXT: &str = "call(\u{1f600}value)\nnext";
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const FRAME_STEP: f64 = 0.1;
const BYTE_LIMIT: usize = 4096;
const SCREEN: egui::Vec2 = egui::vec2(800.0, 600.0);
const TOOLTIP: &str = "parameter tooltip";
const TOOLTIP_FRAMES: usize = 20;

struct Fixture {
    context: Context,
    store: EditorStore,
    document: DocumentId,
    view: ViewId,
    editor: NativeEditor,
    hints: Presentation,
    time: f64,
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
                    window: "inlay-fixture".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        let hints = Presentation {
            document,
            revision: store.documents().snapshot(document).unwrap().revision,
            items: Arc::from([
                Hint {
                    byte: 0,
                    label: "D".into(),
                    kind: None,
                    tooltip: None,
                    padding_left: false,
                    padding_right: false,
                },
                Hint {
                    byte: TEXT.find("value").unwrap(),
                    label: "param: ".into(),
                    kind: Some(Kind::Parameter),
                    tooltip: Some(TOOLTIP.into()),
                    padding_left: true,
                    padding_right: true,
                },
                Hint {
                    byte: TEXT.find("\n").unwrap(),
                    label: ": Type".into(),
                    kind: Some(Kind::Type),
                    tooltip: None,
                    padding_left: false,
                    padding_right: false,
                },
            ]),
            colors: Colors {
                default: Style {
                    foreground: Color32::RED,
                    background: Color32::DARK_RED,
                },
                type_hint: Style {
                    foreground: Color32::GREEN,
                    background: Color32::DARK_GREEN,
                },
                parameter: Style {
                    foreground: Color32::BLUE,
                    background: Color32::DARK_BLUE,
                },
            },
        };
        Self {
            context,
            store,
            document,
            view,
            hints,
            time: 0.0,
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
        let presentation = EditorPresentation::default();
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(self.time),
                events,
                focused: true,
                ..Default::default()
            },
            |ui| {
                if reveal {
                    self.editor
                        .reveal(ui, &mut self.store, self.view, 1.0, 8.0)
                        .unwrap();
                }
                shown = Some(
                    self.editor
                        .show_request_with_inlay_hints(
                            ui,
                            &mut self.store,
                            self.view,
                            EditorRequest {
                                request_focus: self.time == FRAME_STEP,
                                keymap: |_: &Ui, _: &Event, _: bool| false,
                                route: |response: &egui::Response| {
                                    response.ctx.keyboard_input_route(response.id)
                                },
                                presentation: &presentation,
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
                            Some(self.hints.clone()),
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

fn formats(output: &FullOutput) -> Vec<(String, Color32, Color32)> {
    output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) => Some(text),
            _ => None,
        })
        .flat_map(|text| {
            text.galley.job.sections.iter().map(|section| {
                (
                    text.galley.job.text[section.byte_range.start.0..section.byte_range.end.0]
                        .into(),
                    section.format.color,
                    section.format.background,
                )
            })
        })
        .collect()
}

#[test]
fn 종류별_표시와_여백은_문서와_revision을_바꾸지_않는다() {
    let mut fixture = Fixture::new();
    let before = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    let (shown, painted) = fixture.frame(Vec::new(), false);
    let sections = formats(&painted);
    for (label, foreground, background) in [
        ("D", Color32::RED, Color32::DARK_RED),
        ("param:\u{a0}", Color32::BLUE, Color32::DARK_BLUE),
        (":\u{a0}Type", Color32::GREEN, Color32::DARK_GREEN),
    ] {
        assert!(
            sections
                .iter()
                .any(|section| section == &(label.into(), foreground, background)),
            "{label}: {sections:?}"
        );
    }
    assert!(
        sections
            .iter()
            .filter(|(text, _, _)| text == "\u{200a}")
            .all(|(_, _, background)| *background == Color32::TRANSPARENT)
    );
    let rectangles = editor_inlay_hints::hint_rects(&shown.geometry);
    assert_eq!(rectangles.len(), fixture.hints.items.len());
    for (_, rect) in rectangles {
        assert!(rect.width() > 0.0);
        assert_eq!(shown.geometry.byte_at(rect.center()), None);
        assert_eq!(shown.geometry.content_byte_at(rect.center()), None);
    }
    let after = fixture
        .store
        .documents()
        .snapshot(fixture.document)
        .unwrap();
    assert_eq!(after.rope.to_string(), TEXT);
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.dirty, before.dirty);
}

#[test]
fn 힌트_위_클릭은_원문_커서만_옮기고_툴팁과_reveal은_표시를_보존한다() {
    let mut fixture = Fixture::new();
    let (shown, _) = fixture.frame(Vec::new(), false);
    let rect = editor_inlay_hints::hint_rects(&shown.geometry)
        .into_iter()
        .find(|(index, _)| *index == 1)
        .unwrap()
        .1;
    let byte = fixture.hints.items[1].byte;
    assert!(shown.geometry.caret_rect(byte).unwrap().left() >= rect.right());
    for pressed in [true, false] {
        fixture.frame(
            vec![
                Event::PointerMoved(rect.center()),
                Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            false,
        );
    }
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        byte
    );
    let mut tooltip_shown = false;
    fixture.frame(
        vec![Event::PointerMoved(
            shown.geometry.content_rect.right_bottom(),
        )],
        false,
    );
    fixture.frame(vec![Event::PointerMoved(rect.center())], false);
    for _ in 0..TOOLTIP_FRAMES {
        let (_, output) = fixture.frame(Vec::new(), false);
        tooltip_shown |= formats(&output)
            .iter()
            .any(|(text, _, _)| text.contains(TOOLTIP));
    }
    assert!(tooltip_shown);
    let (revealed, _) = fixture.frame(Vec::new(), true);
    assert_eq!(
        editor_inlay_hints::hint_rects(&revealed.geometry).len(),
        fixture.hints.items.len()
    );
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

#[test]
fn 편집_프레임부터_오래된_힌트가_사라지고_원문에_삽입되지_않는다() {
    let mut fixture = Fixture::new();
    fixture.frame(Vec::new(), false);
    let (changed, _) = fixture.frame(vec![Event::Text("x".into())], false);
    assert!(changed.changed);
    assert!(editor_inlay_hints::hint_rects(&changed.geometry).is_empty());
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(fixture.document)
            .unwrap()
            .rope
            .to_string(),
        format!("x{TEXT}")
    );
    let (next, _) = fixture.frame(Vec::new(), false);
    assert!(editor_inlay_hints::hint_rects(&next.geometry).is_empty());
}
