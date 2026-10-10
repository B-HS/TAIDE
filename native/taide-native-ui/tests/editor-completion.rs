#![cfg(all(feature = "native-host", feature = "inspection"))]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use egui::{
    Color32, Context, Event, FontId, Id, Key, Modifiers, RawInput, Rect, Ui, Vec2, pos2, vec2,
};
use serde_json::json;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::completion::{Candidate, Command, PreparationOptions};
use taide_native_editor::completion_model::Model;
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::documentation::{Block, Inline, RichDocument, Span};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::lsp::{LspRange, Position};
use taide_native_editor::snippet_syntax::ParseLimits;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_completion::{Colors, Geometry, Provider, Trigger, Widget};
use taide_native_ui::editor_markup;
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorPresentation, EditorRequest, NativeEditor,
};

const SCREEN: Vec2 = vec2(800.0, 600.0);
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const BYTE_LIMIT: usize = 4096;
const NESTING_LIMIT: usize = 32;
const MARKER_LIMIT: usize = 128;
const TAB_SIZE: u32 = 4;
const LARGE_LIST: usize = 5000;
const PREVIEW_OPACITY: f32 = 0.7;
const RESIZE_DISTANCE: f32 = 100.0;
const FRAME_STEP: f64 = 0.1;
const TOP_PADDING: f32 = 450.0;
const MINIMUM_SUGGEST_ROWS: f32 = 4.3;
const SMALL_RESIZE_DISTANCE: f32 = 6.0;
const DETAILS_RESIZE_DISTANCE: f32 = 50.0;
const HORIZONTAL_SCREEN_MARGIN: f32 = 14.0;
const SWATCH_CONTENT_EM: f32 = 0.7;
const SWATCH_BORDER_EM: f32 = 0.1;
const GEOMETRY_EPSILON: f32 = 0.01;

fn limits() -> ParseLimits {
    ParseLimits {
        max_bytes: BYTE_LIMIT,
        max_nesting: NESTING_LIMIT,
        max_markers: MARKER_LIMIT,
    }
}

struct Completions {
    widget: Option<Widget>,
    labels: Vec<String>,
    preselected: Vec<String>,
    swatch: Option<Color32>,
    requests: Vec<Trigger>,
    accepted: Vec<(String, String, bool)>,
    pending: bool,
    automatic: bool,
    allow_accept: bool,
    deprecated: bool,
    preview_text: Option<String>,
    preview_calls: usize,
    preview_tokens: Option<Arc<taide_native_editor::line_tokens::PreviewTokens>>,
    embedded: bool,
    has_documentation: bool,
}

impl Default for Completions {
    fn default() -> Self {
        Self {
            widget: None,
            labels: vec!["foo".into(), "format".into(), "forEach".into()],
            preselected: Vec::new(),
            swatch: None,
            requests: Vec::new(),
            accepted: Vec::new(),
            pending: false,
            automatic: true,
            allow_accept: true,
            deprecated: false,
            preview_text: None,
            preview_calls: 0,
            preview_tokens: None,
            embedded: false,
            has_documentation: true,
        }
    }
}

impl Completions {
    fn build(&mut self, store: &EditorStore, view: ViewId) {
        let current = store.views().get(view).unwrap();
        let document = store.documents().snapshot(current.document).unwrap();
        let byte = current.selection.selections[current.selection.primary].head;
        let leading = document.rope.byte_slice(..byte).to_string();
        let position = Position::new(0, leading.encode_utf16().count().try_into().unwrap());
        let range = LspRange::new(Position::new(0, 0), position);
        let candidates = self
            .labels
            .iter()
            .filter_map(|label| {
                Candidate::new(
                    &document,
                    position,
                    range,
                    serde_json::from_value(
                        json!({ "label": label, "insertText": self.preview_text.as_ref().unwrap_or(label), "kind": if self.swatch.is_some() { 16 } else { 3 }, "detail": "function\r\ndetail", "preselect": self.preselected.contains(label), "tags": if self.deprecated { Some(vec![1]) } else { None } }),
                    )
                    .unwrap(),
                )
            })
            .collect();
        let model = Rc::new(RefCell::new(Model::new(&document, candidates).unwrap()));
        let token = self.requests.len().to_string();
        self.widget = Some(Widget {
            token,
            byte,
            automatic: self.requests.last() != Some(&Trigger::Manual),
            pending: self.pending,
            model: (!self.pending).then_some(model),
            leading,
            delta: 0,
        });
    }
}

impl editor_markup::Provider for Completions {
    fn open_link(&mut self, _: &Ui, _: &str) -> bool {
        true
    }
}

impl Provider for Completions {
    fn swatch(&mut self, _: &EditorStore, _: ViewId, _: &str, _: usize) -> Option<Color32> {
        self.swatch
    }
    fn is_embedded(&self, _: ViewId) -> bool {
        self.embedded
    }
    fn preview_tokens(
        &self,
        _view: ViewId,
        _line: usize,
    ) -> Option<Arc<taide_native_editor::line_tokens::PreviewTokens>> {
        self.preview_tokens.clone()
    }
    fn available(&self, _: &EditorStore, _: ViewId) -> bool {
        true
    }
    fn request(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        trigger: Trigger,
    ) -> Result<bool, EditorError> {
        self.requests.push(trigger);
        self.build(store, view);
        Ok(true)
    }
    fn current(&self, _: &EditorStore, _: ViewId) -> Option<Widget> {
        self.widget.clone()
    }
    fn preview(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        token: &str,
        candidate: usize,
        alternate: bool,
    ) -> Vec<taide_native_editor::completion_preview::GhostText> {
        self.preview_calls += 1;
        let widget = self.widget.as_ref().unwrap();
        assert_eq!(widget.token, token);
        let model = widget.model.as_ref().unwrap().borrow();
        let candidate = model.candidate(candidate).unwrap();
        let source = store.views().get(view).unwrap();
        let snapshot = store.documents().snapshot(source.document).unwrap();
        let primary = source.selection.selections[source.selection.primary].head;
        let range = if alternate {
            candidate.replace.clone()
        } else {
            candidate.insert.clone()
        };
        taide_native_editor::completion_preview::compute(
            &snapshot,
            range,
            candidate.text(),
            primary,
            taide_native_editor::completion_preview::Options {
                mode: taide_native_editor::completion_preview::Mode::SubwordSmart,
                preview_suffix_utf16: 0,
            },
        )
        .unwrap()
        .into_iter()
        .collect()
    }
    fn close(&mut self, _: ViewId) {
        self.widget = None;
    }
    fn triggers(&self, _: &EditorStore, _: ViewId) -> Vec<String> {
        vec![".".into()]
    }
    fn should_auto_trigger(&self, _: &EditorStore, _: ViewId) -> bool {
        self.automatic
    }
    fn after_event(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        _: &Event,
    ) -> Result<(), EditorError> {
        let should_rebuild = self.widget.as_ref().is_some_and(|widget| {
            let source = store.views().get(view).unwrap();
            let document = store.documents().snapshot(source.document).unwrap();
            source.selection.selections[source.selection.primary].head != widget.byte
                || widget.model.as_ref().is_some_and(|model| {
                    model
                        .borrow()
                        .candidate(0)
                        .is_some_and(|candidate| candidate.revision != document.revision)
                })
        });
        if should_rebuild {
            self.build(store, view);
        }
        Ok(())
    }
    fn accept(
        &mut self,
        store: &mut EditorStore,
        view: ViewId,
        token: &str,
        index: usize,
        alternate: bool,
    ) -> Result<bool, EditorError> {
        if !self.allow_accept {
            return Ok(false);
        }
        let widget = self.widget.as_ref().ok_or(EditorError::NotFound)?;
        if widget.token != token {
            return Err(EditorError::StaleRevision);
        }
        let model = widget.model.as_ref().ok_or(EditorError::NotFound)?.borrow();
        let candidate = model.candidate(index).ok_or(EditorError::NotFound)?;
        let owner = store.views().get(view).unwrap().clone();
        let document = store.documents().snapshot(owner.document)?;
        let prepared = candidate.prepare(
            &document,
            &owner.selection,
            PreparationOptions {
                alternate,
                indent: IndentOptions {
                    tab_size: TAB_SIZE,
                    insert_spaces: true,
                },
                limits: limits(),
            },
            |_, _| None,
            |_, _| Ok(None),
            |_, _, _| Ok(String::new()),
        )?;
        self.accepted.push((
            document.rope.to_string(),
            candidate.item.label.clone(),
            alternate,
        ));
        taide_native_editor::snippet_insertion::insert(
            store,
            &owner,
            document.revision,
            prepared,
            limits(),
        )?;
        Ok(true)
    }
    fn documentation(
        &mut self,
        _: &EditorStore,
        _: ViewId,
        _: &str,
        index: usize,
    ) -> Option<Arc<RichDocument>> {
        if !self.has_documentation {
            return None;
        }
        Some(Arc::new(RichDocument {
            blocks: vec![Block::Paragraph(vec![Inline::Text(Span {
                text: format!("documentation for {}", self.labels[index]),
                ..Default::default()
            })])],
        }))
    }
    fn detail(&self, _: &EditorStore, _: ViewId, _: &str, _: usize) -> Option<String> {
        Some("function\ndetail".into())
    }
}

struct Fixture {
    context: Context,
    store: EditorStore,
    document: DocumentId,
    view: ViewId,
    editor: NativeEditor,
    completion: Completions,
    body: Option<Id>,
    geometry: Geometry,
    painted: Vec<String>,
    intercepted: Option<Key>,
    other: String,
    other_id: Option<Id>,
    jobs: Vec<Arc<egui::text::LayoutJob>>,
    rectangles: Vec<egui::epaint::RectShape>,
    preview: bool,
    editor_geometry: Option<taide_native_ui::editor_geometry::EditorGeometry>,
    preview_colors: taide_native_ui::editor_completion::PreviewColors,
    top_padding: f32,
}

impl Fixture {
    fn new() -> Self {
        let context = Context::default();
        context.set_os(egui::os::OperatingSystem::Mac);
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(
            egui::FontFamily::Name(taide_native_ui::font_families::EDITOR_BOLD_FAMILY.into()),
            fonts.families[&egui::FontFamily::Monospace].clone(),
        );
        context.set_fonts(fonts);
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 4,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = store
            .open_untitled(TabId::new(), "fo".into(), "rust".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "completion".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
            )
            .unwrap();
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection { anchor: 2, head: 2 }],
                },
                ScrollPosition::default(),
                Vec::new(),
            )
            .unwrap();
        Self {
            context,
            store,
            document,
            view,
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
            completion: Completions::default(),
            body: None,
            geometry: Geometry::default(),
            painted: Vec::new(),
            intercepted: None,
            other: String::new(),
            other_id: None,
            jobs: Vec::new(),
            rectangles: Vec::new(),
            preview: false,
            preview_colors: taide_native_ui::editor_completion::PreviewColors::for_dark_mode(true),
            editor_geometry: None,
            top_padding: 0.0,
        }
    }

    fn text(&self) -> String {
        self.store
            .documents()
            .snapshot(self.document)
            .unwrap()
            .rope
            .to_string()
    }

    fn selected_label(&self) -> Option<String> {
        let widget = self.completion.widget.as_ref()?;
        let model = widget.model.as_ref()?.borrow();
        Some(model.candidate(self.geometry.selected?)?.item.label.clone())
    }

    fn frame(&mut self, time: f64, events: Vec<Event>, commands: &[Command]) -> bool {
        let mut presentation = EditorPresentation::default();
        let markup = editor_markup::Colors {
            background: Color32::DARK_GRAY,
            foreground: Color32::WHITE,
            border: Color32::GRAY,
            highlight: Color32::YELLOW,
            link: Color32::LIGHT_BLUE,
            code_background: Color32::BLACK,
            shadow: Color32::BLACK,
        };
        presentation.options.suggest_preview = self.preview;
        presentation.options.completion_colors = Some(Colors {
            background: Color32::DARK_GRAY,
            foreground: Color32::WHITE,
            border: Color32::GRAY,
            selected_background: Color32::BLUE,
            selected_foreground: Color32::WHITE,
            selected_icon: Color32::LIGHT_BLUE,
            highlight: Color32::YELLOW,
            selected_highlight: Color32::YELLOW,
            resize: Color32::LIGHT_BLUE,
            documentation: markup,
            preview: self.preview_colors,
        });
        let mut rendered = None;
        let mut output = self.context.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)), time: Some(time), events, ..Default::default() }, |ui| {
            self.other_id = Some(ui.text_edit_singleline(&mut self.other).id);
            ui.add_space(self.top_padding);
            rendered = Some(self.editor.show_request(ui, &mut self.store, self.view, EditorRequest { request_focus: self.body.is_none(), keymap: |_: &Ui, event: &Event, _: bool| matches!(event, Event::Key { key, pressed: true, .. } if Some(*key) == self.intercepted), route: |response: &egui::Response| response.ctx.keyboard_input_route(response.id), presentation: &presentation, tokens: |_: &EditorStore| None, language: None, decorations: &[], fold_commands: &[], fold_controls: None, problems: None, locations: None, documentation: None, documentation_commands: &[], completion: Some(&mut self.completion), completion_commands: commands, syntax_folds: None }).unwrap());
        });
        self.rectangles = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rectangle) => Some(rectangle.clone()),
                _ => None,
            })
            .collect();
        self.jobs = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some(text.galley.job.clone())
                } else {
                    None
                }
            })
            .collect();
        self.painted = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape {
                    Some(text.galley.job.text.clone())
                } else {
                    None
                }
            })
            .collect();
        output.textures_delta.clear();
        let rendered = rendered.unwrap();
        assert!(rendered.errors.is_empty(), "{:?}", rendered.errors);
        self.body = Some(rendered.response.id);
        self.editor_geometry = Some(rendered.geometry.clone());
        self.geometry = rendered.completion_geometry;
        rendered.changed
    }

    fn open(&mut self) {
        self.frame(0.0, Vec::new(), &[]);
        self.frame(0.1, vec![key(Key::Space, Modifiers::CTRL)], &[]);
        self.frame(0.2, Vec::new(), &[]);
    }

    fn drag(&mut self, time: f64, start: egui::Pos2, delta: Vec2) {
        self.frame(
            time,
            vec![
                Event::PointerMoved(start),
                Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            &[],
        );
        let end = start + delta;
        self.frame(time + FRAME_STEP, vec![Event::PointerMoved(end)], &[]);
        self.frame(
            time + FRAME_STEP * 2.0,
            vec![Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
            &[],
        );
        self.frame(time + FRAME_STEP * 3.0, Vec::new(), &[]);
    }
}

#[test]
fn 색_후보는_원본_크기의_검정_테두리와_반투명_견본으로_일반_아이콘을_대체한다() {
    let mut fixture = Fixture::new();
    let color = Color32::from_rgba_unmultiplied(255, 0, 0, 128);
    fixture.completion.swatch = Some(color);
    fixture.open();
    fixture.frame(0.3, Vec::new(), &[]);
    let content = fixture
        .rectangles
        .iter()
        .find(|shape| shape.fill == color)
        .expect("visible color swatch after popup animation");
    assert!((content.rect.width() - FONT_SIZE * SWATCH_CONTENT_EM).abs() < GEOMETRY_EPSILON);
    assert!((content.rect.height() - FONT_SIZE * SWATCH_CONTENT_EM).abs() < GEOMETRY_EPSILON);
    let outer = content.rect.expand(FONT_SIZE * SWATCH_BORDER_EM);
    assert!(fixture.rectangles.iter().any(|shape| {
        shape.fill == Color32::TRANSPARENT
            && shape.stroke.color == Color32::BLACK
            && (shape.stroke.width - FONT_SIZE * SWATCH_BORDER_EM).abs() < GEOMETRY_EPSILON
            && shape.rect.min.distance(outer.min) < GEOMETRY_EPSILON
            && shape.rect.max.distance(outer.max) < GEOMETRY_EPSILON
    }));
    assert!(!fixture.painted.iter().any(|text| text == "\u{eb5c}"));
    assert_eq!(fixture.selected_label().as_deref(), Some("foo"));
    fixture.completion.swatch = None;
    fixture.frame(0.4, Vec::new(), &[]);
    assert!(!fixture.rectangles.iter().any(|shape| shape.fill == color));
    assert!(fixture.painted.iter().any(|text| text == "\u{eb5c}"));
    assert_eq!(fixture.selected_label().as_deref(), Some("foo"));
}

#[test]
fn 동일_요청의_모델_교체는_이전_인덱스의_다른_후보를_유지하지_않고_우선_후보를_재선택한다() {
    let mut fixture = Fixture::new();
    fixture.open();
    fixture.frame(0.3, Vec::new(), &[Command::Next]);
    fixture.completion.labels = vec!["foAlpha".into(), "foBeta".into(), "foGamma".into()];
    fixture.completion.preselected = vec!["foGamma".into()];
    fixture.completion.build(&fixture.store, fixture.view);
    fixture.frame(0.4, Vec::new(), &[]);
    assert_eq!(fixture.selected_label().as_deref(), Some("foGamma"));
}

#[test]
fn suggest_최고_점수의_첫_preselect만_선택하고_더_낮은_점수는_우선하지_않는다() {
    let mut fixture = Fixture::new();
    fixture.completion.preselected = vec!["format".into()];
    fixture.open();
    assert_eq!(fixture.selected_label().as_deref(), Some("format"));
    fixture.frame(0.3, vec![key(Key::Enter, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(), "format");

    let mut fixture = Fixture::new();
    fixture.completion.labels = vec!["fo".into(), "foo".into()];
    fixture.completion.preselected = vec!["foo".into()];
    fixture.open();
    assert_eq!(fixture.selected_label().as_deref(), Some("fo"));
}

#[test]
fn suggest_재입력은_first로_재선택하고_방향키와_새_요청의_선택을_구분한다() {
    let mut fixture = Fixture::new();
    fixture.completion.preselected = vec!["format".into(), "forEach".into()];
    fixture.open();
    assert_eq!(fixture.selected_label().as_deref(), Some("forEach"));
    fixture.frame(0.3, vec![key(Key::ArrowDown, Modifiers::NONE)], &[]);
    assert_eq!(fixture.selected_label().as_deref(), Some("format"));
    fixture.frame(0.4, Vec::new(), &[]);
    assert_eq!(fixture.selected_label().as_deref(), Some("format"));
    fixture.frame(0.5, vec![key(Key::Escape, Modifiers::NONE)], &[]);
    fixture.completion.preselected.clear();
    fixture.frame(0.6, vec![key(Key::Space, Modifiers::CTRL)], &[]);
    fixture.frame(0.7, Vec::new(), &[]);
    assert_eq!(fixture.selected_label().as_deref(), Some("foo"));
    fixture.frame(0.8, Vec::new(), &[Command::Next, Command::Next]);
    assert_eq!(fixture.selected_label().as_deref(), Some("format"));
    fixture.frame(0.9, vec![Event::Text("r".into())], &[]);
    fixture.frame(1.0, Vec::new(), &[]);
    assert_eq!(fixture.selected_label().as_deref(), Some("forEach"));
}

#[test]
fn suggest_preview는_설정에_따라_실제_본문과_추가줄을_표시하고_취소하면_문서를_보존하며_회수한다() {
    let mut fixture = Fixture::new();
    fixture.completion.preview_text = Some("foo(\n\targument\n)".into());
    fixture.open();
    assert_eq!(fixture.completion.preview_calls, 0);
    assert!(!fixture.painted.iter().any(|text| text == "foo("));
    fixture.preview = true;
    fixture.frame(0.3, Vec::new(), &[]);
    assert!(fixture.painted.iter().any(|text| text == "foo("));
    assert!(fixture.painted.iter().any(|text| text == "    argument"));
    assert!(fixture.painted.iter().any(|text| text == ")"));
    assert_eq!(fixture.text(), "fo");
    let caret = fixture
        .editor_geometry
        .as_ref()
        .unwrap()
        .caret_rect(2)
        .unwrap();
    fixture.frame(0.4, vec![key(Key::Escape, Modifiers::NONE)], &[]);
    assert_eq!(fixture.text(), "fo");
    assert!(
        !fixture
            .painted
            .iter()
            .any(|text| text == "foo(" || text == "    argument")
    );
    assert_eq!(
        fixture
            .editor_geometry
            .as_ref()
            .unwrap()
            .caret_rect(2)
            .unwrap(),
        caret
    );
}

#[test]
fn suggest_preview는_가상줄_토큰과_테마를_주입문자에만_적용하고_짧은후보의_전경색을_보존한다() {
    use taide_native_editor::line_tokens::{
        LineTokens, PreviewTokens, TokenStyle, TokenStyleTable,
    };
    let mut fixture = Fixture::new();
    fixture.preview = true;
    fixture.preview_colors.background = Some(Color32::GREEN);
    fixture.preview_colors.border = Some(Color32::YELLOW);
    fixture.completion.preview_text = Some("foo(\nargument\n)".into());
    let style = |color: Color32| TokenStyle {
        foreground: color.to_srgba_unmultiplied(),
        is_italic: false,
        is_bold: false,
        is_underlined: false,
        is_struck_through: false,
        kind: taide_native_editor::syntax::TokenKind::Other,
    };
    let mut lines = LineTokens::new(3);
    for line in 0..3 {
        lines.set_line(line, vec![0, line as u32], false);
    }
    fixture.completion.preview_tokens = Some(Arc::new(PreviewTokens {
        lines,
        styles: TokenStyleTable::new(
            style(Color32::WHITE),
            vec![
                style(Color32::RED),
                style(Color32::BLUE),
                style(Color32::YELLOW),
            ],
        ),
    }));
    fixture.open();
    let first = fixture.jobs.iter().find(|job| job.text == "foo(").unwrap();
    assert_eq!(
        first.sections[0].format.color,
        fixture.editor.appearance.foreground
    );
    assert_eq!(first.sections[0].format.background, Color32::TRANSPARENT);
    assert!(first.sections.iter().any(|section| section.format.color
        == Color32::RED.gamma_multiply(PREVIEW_OPACITY)
        && section.format.background == Color32::GREEN.gamma_multiply(PREVIEW_OPACITY)
        && section.format.italics));
    let additional = fixture
        .jobs
        .iter()
        .find(|job| job.text == "argument")
        .unwrap();
    assert!(
        additional
            .sections
            .iter()
            .all(
                |section| section.format.color == Color32::BLUE.gamma_multiply(PREVIEW_OPACITY)
                    && section.format.italics
            )
    );
    fixture.completion.preview_text = Some("foo".into());
    fixture.completion.build(&fixture.store, fixture.view);
    fixture.frame(0.3, Vec::new(), &[]);
    let short = fixture.jobs.iter().find(|job| job.text == "foo").unwrap();
    assert!(
        short
            .sections
            .iter()
            .any(|section| section.format.color == fixture.preview_colors.foreground)
    );
    assert!(
        !short
            .sections
            .iter()
            .any(|section| section.format.color == Color32::RED.gamma_multiply(PREVIEW_OPACITY))
    );
    assert_eq!(fixture.text(), "fo");
}

#[test]
fn suggest_preview의_숨긴접미사는_추가줄로_옮겨지고_다음_문서줄은_그_아래에_표시된다() {
    let mut fixture = Fixture::new();
    fixture
        .store
        .apply(
            fixture.document,
            taide_native_editor::store::Transaction {
                revision: 0,
                group: taide_native_editor::document::UndoGroup(0),
                origin: None,
                selection_after: None,
                edits: vec![taide_native_editor::document::Edit {
                    bytes: 2..2,
                    text: " tail\nnext".into(),
                }],
            },
        )
        .unwrap();
    fixture
        .store
        .set_view_state(
            fixture.view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 2, head: 2 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    fixture.preview = true;
    fixture.completion.preview_text = Some("foo(\n\targument\n)".into());
    fixture.open();
    assert!(fixture.painted.iter().any(|text| text == "foo("));
    assert!(fixture.painted.iter().any(|text| text == ") tail"));
    assert!(fixture.painted.iter().any(|text| text == "next"));
    assert!(!fixture.painted.iter().any(|text| text == "foo( tail"));
    let geometry = fixture.editor_geometry.as_ref().unwrap();
    let head = geometry.caret_rect(2).unwrap();
    let next = geometry.caret_rect(8).unwrap();
    assert_eq!(next.top() - head.top(), LINE_HEIGHT * 3.0);
    assert_eq!(fixture.text(), "fo tail\nnext");
    fixture.frame(0.3, Vec::new(), &[Command::Accept { alternate: false }]);
    assert_eq!(fixture.text(), "foo(\n    argument\n) tail\nnext");
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn manual_completion_navigation_keeps_document_and_original_dimensions() {
    let mut fixture = Fixture::new();
    fixture.open();
    assert_eq!(fixture.completion.requests, [Trigger::Manual]);
    let rect = fixture.geometry.list.unwrap();
    assert!((rect.width() - 430.0).abs() < 1.0);
    assert_eq!(fixture.text(), "fo");
    let before = fixture.geometry.selected;
    fixture.frame(0.3, vec![key(Key::ArrowDown, Modifiers::NONE)], &[]);
    assert_ne!(fixture.geometry.selected, before);
    assert_eq!(fixture.text(), "fo");
    assert_eq!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        2
    );
    fixture.frame(0.4, vec![key(Key::Enter, Modifiers::SHIFT)], &[]);
    assert_eq!(fixture.completion.accepted.len(), 1);
    assert!(fixture.completion.accepted[0].2);
    assert_eq!(fixture.text(), fixture.completion.accepted[0].1);
    fixture.store.undo(fixture.document).unwrap();
    assert_eq!(fixture.text(), "fo");
}

#[test]
fn completion_오른쪽_크기조절은_문서와_본문포커스를_보존하고_재개해도_폭을_유지한다() {
    let mut fixture = Fixture::new();
    fixture.open();
    let initial = fixture.geometry.list.unwrap();
    let start = initial.right_center();
    fixture.frame(
        0.3,
        vec![
            Event::PointerMoved(start),
            Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &[],
    );
    let end = start + vec2(RESIZE_DISTANCE, 0.0);
    fixture.frame(0.4, vec![Event::PointerMoved(end)], &[]);
    fixture.frame(
        0.5,
        vec![Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    fixture.frame(0.6, Vec::new(), &[]);
    assert!(
        (fixture.geometry.list.unwrap().width() - initial.width() - RESIZE_DISTANCE).abs() < 1.0
    );
    assert_eq!(fixture.text(), "fo");
    assert!(fixture.completion.accepted.is_empty());
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        fixture.body
    );
    fixture.frame(0.7, vec![key(Key::Escape, Modifiers::NONE)], &[]);
    fixture.frame(0.8, Vec::new(), &[Command::Trigger]);
    fixture.frame(0.9, Vec::new(), &[]);
    assert!(
        (fixture.geometry.list.unwrap().width() - initial.width() - RESIZE_DISTANCE).abs() < 1.0
    );
}

#[test]
fn completion_세로크기를_줄이면_페이지키는_현재_보이는_행부터_이동한다() {
    let mut fixture = Fixture::new();
    fixture.completion.labels = (0..LARGE_LIST)
        .map(|index| format!("foo{index:04}"))
        .collect();
    fixture.open();
    let initial = fixture.geometry.list.unwrap();
    fixture.drag(0.3, initial.center_bottom(), vec2(0.0, -RESIZE_DISTANCE));
    assert!(
        (fixture.geometry.list.unwrap().height() - initial.height() + RESIZE_DISTANCE).abs() < 1.0
    );
    fixture.frame(0.7, vec![key(Key::PageDown, Modifiers::NONE)], &[]);
    assert_eq!(fixture.geometry.selected, Some(6));
    fixture.frame(0.8, vec![key(Key::PageDown, Modifiers::NONE)], &[]);
    assert_eq!(fixture.geometry.selected, Some(13));
    fixture.frame(0.9, vec![key(Key::PageUp, Modifiers::NONE)], &[]);
    assert_eq!(fixture.geometry.selected, Some(7));
    fixture.frame(1.0, vec![key(Key::PageUp, Modifiers::NONE)], &[]);
    assert_eq!(fixture.geometry.selected, Some(0));
    assert_eq!(fixture.text(), "fo");
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        fixture.body
    );
}

#[test]
fn completion_위쪽_크기조절과_화면상한은_커서쪽_테두리를_보존한다() {
    let mut fixture = Fixture::new();
    fixture.top_padding = TOP_PADDING;
    fixture.completion.labels = (0..LARGE_LIST)
        .map(|index| format!("foo{index:04}"))
        .collect();
    fixture.open();
    let initial = fixture.geometry.list.unwrap();
    let caret = fixture
        .editor_geometry
        .as_ref()
        .unwrap()
        .caret_rect(2)
        .unwrap();
    assert!((initial.bottom() - caret.top()).abs() < 1.0);
    fixture.drag(0.3, initial.center_top(), vec2(0.0, -RESIZE_DISTANCE));
    let resized = fixture.geometry.list.unwrap();
    assert!(
        (resized.height() - initial.height() - RESIZE_DISTANCE).abs() < 1.0,
        "initial={initial:?}, resized={resized:?}, caret={caret:?}"
    );
    assert!((resized.bottom() - caret.top()).abs() < 1.0);
    fixture.drag(
        0.7,
        resized.left_top() + vec2(resized.width() / 2.0, 0.0),
        vec2(0.0, -SCREEN.y),
    );
    let clamped = fixture.geometry.list.unwrap();
    assert!(clamped.top() >= 0.0);
    assert!((clamped.bottom() - caret.top()).abs() < 1.0);
    assert_eq!(fixture.text(), "fo");
}

#[test]
fn completion_작은드래그와_축별저장_내장뷰_초기화는_원하는크기를_보존한다() {
    let mut fixture = Fixture::new();
    fixture.completion.labels = (0..LARGE_LIST)
        .map(|index| format!("foo{index:04}"))
        .collect();
    fixture.open();
    let initial = fixture.geometry.list.unwrap();
    let start = initial.right_center();
    let end = start + vec2(SMALL_RESIZE_DISTANCE, 0.0);
    fixture.frame(
        0.3,
        vec![
            Event::PointerMoved(start),
            Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &[],
    );
    assert!(
        fixture.geometry.list.is_some(),
        "press: accepted={:?}, focus={:?}",
        fixture.completion.accepted,
        fixture.context.memory(|memory| memory.focused())
    );
    fixture.frame(0.4, vec![Event::PointerMoved(end)], &[]);
    assert!(
        fixture.geometry.list.is_some(),
        "move: accepted={:?}, focus={:?}",
        fixture.completion.accepted,
        fixture.context.memory(|memory| memory.focused())
    );
    fixture.frame(
        0.5,
        vec![Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    assert!(
        fixture.geometry.list.is_some(),
        "release: accepted={:?}, focus={:?}",
        fixture.completion.accepted,
        fixture.context.memory(|memory| memory.focused())
    );
    fixture.frame(0.6, Vec::new(), &[]);
    assert!((fixture.geometry.list.unwrap().width() - initial.width()).abs() < 1.0);
    fixture.drag(
        0.9,
        fixture.geometry.list.unwrap().right_center(),
        vec2(RESIZE_DISTANCE, 0.0),
    );
    fixture.frame(1.3, Vec::new(), &[Command::Hide]);
    fixture.completion.embedded = true;
    fixture.frame(1.4, Vec::new(), &[Command::Trigger]);
    fixture.frame(1.5, Vec::new(), &[]);
    assert!((fixture.geometry.list.unwrap().width() - initial.width()).abs() < 1.0);
    fixture.drag(
        1.6,
        fixture.geometry.list.unwrap().right_center(),
        vec2(-RESIZE_DISTANCE, 0.0),
    );
    fixture.frame(2.0, Vec::new(), &[Command::Hide]);
    fixture.frame(2.1, Vec::new(), &[Command::ResetSize]);
    fixture.frame(2.2, Vec::new(), &[Command::Trigger]);
    fixture.frame(2.3, Vec::new(), &[]);
    assert!((fixture.geometry.list.unwrap().width() - initial.width()).abs() < 1.0);
    fixture.frame(2.4, Vec::new(), &[Command::Hide]);
    fixture.completion.embedded = false;
    fixture.frame(2.5, Vec::new(), &[Command::Trigger]);
    fixture.frame(2.6, Vec::new(), &[]);
    assert!(
        (fixture.geometry.list.unwrap().width() - initial.width() - RESIZE_DISTANCE).abs() < 1.0
    );
    fixture.frame(2.7, Vec::new(), &[Command::ResetSize]);
    fixture.frame(2.8, Vec::new(), &[]);
    assert!((fixture.geometry.list.unwrap().width() - initial.width()).abs() < 1.0);
}

#[test]
fn completion_후보필터는_저장한높이를_보존하고_닫을때_최소높이를_회복한다() {
    let mut fixture = Fixture::new();
    fixture.completion.labels = (0..LARGE_LIST)
        .map(|index| format!("foo{index:04}"))
        .collect();
    fixture.open();
    let initial = fixture.geometry.list.unwrap();
    fixture.drag(0.3, initial.center_bottom(), vec2(0.0, -RESIZE_DISTANCE));
    let wanted = fixture.geometry.list.unwrap().height();
    fixture.frame(0.7, Vec::new(), &[Command::Hide]);
    let labels = fixture.completion.labels.clone();
    fixture.completion.labels = vec!["foo".into()];
    fixture.frame(0.8, Vec::new(), &[Command::Trigger]);
    fixture.frame(0.9, Vec::new(), &[]);
    assert!(fixture.geometry.list.unwrap().height() < wanted);
    fixture.frame(1.0, Vec::new(), &[Command::Hide]);
    fixture.completion.labels = labels;
    fixture.frame(1.1, Vec::new(), &[Command::Trigger]);
    fixture.frame(1.2, Vec::new(), &[]);
    assert!(
        (fixture.geometry.list.unwrap().height() - wanted).abs() < 1.0,
        "wanted={wanted}, restored={:?}",
        fixture.geometry.list
    );
    fixture.drag(
        1.3,
        fixture.geometry.list.unwrap().center_bottom(),
        vec2(0.0, -SCREEN.y),
    );
    assert!(fixture.geometry.list.unwrap().height() < LINE_HEIGHT * MINIMUM_SUGGEST_ROWS);
    fixture.frame(1.7, Vec::new(), &[Command::Hide]);
    fixture.frame(1.8, Vec::new(), &[Command::Trigger]);
    fixture.frame(1.9, Vec::new(), &[]);
    assert!(
        (fixture.geometry.list.unwrap().height() - (LINE_HEIGHT * MINIMUM_SUGGEST_ROWS).ceil())
            .abs()
            < 1.0
    );
}

#[test]
fn completion_상세창은_타입만_있는_후보도_열고_크기와_포커스를_보존한다() {
    let mut fixture = Fixture::new();
    fixture.completion.has_documentation = false;
    fixture.open();
    fixture.frame(0.3, Vec::new(), &[Command::ToggleDetails]);
    fixture.frame(0.4, Vec::new(), &[]);
    let initial = fixture.geometry.details.expect("type-only details");
    assert!(
        fixture
            .painted
            .iter()
            .any(|text| text.contains("function\ndetail"))
    );
    fixture.drag(
        0.5,
        initial.left_center(),
        vec2(-DETAILS_RESIZE_DISTANCE, 0.0),
    );
    let resized = fixture.geometry.details.unwrap();
    let expected =
        (initial.width() + DETAILS_RESIZE_DISTANCE).min(initial.right() - HORIZONTAL_SCREEN_MARGIN);
    assert!(
        (resized.width() - expected).abs() < 1.0,
        "initial={initial:?}, resized={resized:?}"
    );
    assert!((resized.right() - initial.right()).abs() < 1.0);
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        fixture.body
    );
    assert_eq!(fixture.text(), "fo");
    fixture.frame(0.9, Vec::new(), &[Command::Next]);
    fixture.frame(1.0, Vec::new(), &[]);
    assert!((fixture.geometry.details.unwrap().width() - resized.width()).abs() < 1.0);
    fixture.frame(1.1, Vec::new(), &[Command::ToggleDetails]);
    assert!(fixture.geometry.details.is_none());
    fixture.frame(1.2, Vec::new(), &[Command::ToggleDetails]);
    fixture.frame(1.3, Vec::new(), &[]);
    assert!((fixture.geometry.details.unwrap().width() - resized.width()).abs() < 1.0);
}

#[test]
fn completion_위쪽_상세창은_북쪽테두리로_늘리고_닫기버튼은_후보와_본문을_보존한다() {
    let mut fixture = Fixture::new();
    fixture.top_padding = TOP_PADDING;
    fixture.completion.labels = (0..LARGE_LIST)
        .map(|index| format!("foo{index:04}"))
        .collect();
    fixture.open();
    fixture.frame(0.3, Vec::new(), &[Command::ToggleDetails]);
    fixture.frame(0.4, Vec::new(), &[]);
    let initial = fixture.geometry.details.unwrap();
    assert!(initial.bottom() <= fixture.geometry.list.unwrap().top() + 1.0);
    fixture.drag(
        0.5,
        initial.center_top(),
        vec2(0.0, -DETAILS_RESIZE_DISTANCE),
    );
    let resized = fixture.geometry.details.unwrap();
    assert!(
        (resized.height() - initial.height() - DETAILS_RESIZE_DISTANCE).abs() < 1.0,
        "initial={initial:?}, resized={resized:?}"
    );
    assert!((resized.bottom() - initial.bottom()).abs() < 1.0);
    let close = fixture.geometry.details_close.unwrap().center();
    fixture.frame(
        0.9,
        vec![
            Event::PointerMoved(close),
            Event::PointerButton {
                pos: close,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &[],
    );
    fixture.frame(
        1.0,
        vec![Event::PointerButton {
            pos: close,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    fixture.frame(1.1, Vec::new(), &[]);
    assert!(fixture.geometry.details.is_none());
    assert!(fixture.geometry.list.is_some());
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        fixture.body
    );
    assert_eq!(fixture.text(), "fo");
    assert!(fixture.completion.accepted.is_empty());
}

#[test]
fn completion_모서리크기조절은_화면에_제한되고_테두리두번클릭은_내용폭으로_초기화한다() {
    let mut fixture = Fixture::new();
    fixture.completion.labels = (0..LARGE_LIST)
        .map(|index| format!("foo{index:04}"))
        .collect();
    fixture.open();
    let initial = fixture.geometry.list.unwrap();
    fixture.drag(0.3, initial.right_bottom(), SCREEN);
    let expanded = fixture.geometry.list.unwrap();
    assert!(expanded.right() <= SCREEN.x);
    assert!(expanded.bottom() <= SCREEN.y);
    assert!(expanded.width() > initial.width());
    assert!(expanded.height() > initial.height());
    let right = expanded.right_center();
    for (time, pressed) in [(0.9, true), (1.0, false), (1.1, true), (1.2, false)] {
        fixture.frame(
            time,
            vec![
                Event::PointerMoved(right),
                Event::PointerButton {
                    pos: right,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ],
            &[],
        );
    }
    fixture.frame(1.3, Vec::new(), &[]);
    let reset = fixture.geometry.list.unwrap();
    assert!(reset.width() < initial.width());
    assert!((reset.height() - expanded.height()).abs() < 1.0);
    assert_eq!(fixture.text(), "fo");
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        fixture.body
    );
}

#[test]
fn completion_accept_runs_after_prior_text_in_same_frame() {
    let mut fixture = Fixture::new();
    fixture.open();
    let changed = fixture.frame(
        0.3,
        vec![Event::Text("r".into()), key(Key::Tab, Modifiers::NONE)],
        &[],
    );
    assert!(changed);
    assert_eq!(fixture.completion.accepted[0].0, "for");
    assert!(fixture.text().starts_with("for"));
    assert!(fixture.geometry.list.is_none());
    fixture.frame(0.4, vec![Event::Text("!".into())], &[]);
    assert!(fixture.text().ends_with('!'));
}

#[test]
fn quick_completion_waits_ten_ms_and_trigger_character_is_immediate() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, vec![Event::Text("o".into())], &[]);
    fixture.frame(0.109, Vec::new(), &[]);
    assert!(fixture.completion.requests.is_empty());
    fixture.frame(0.111, Vec::new(), &[]);
    assert_eq!(fixture.completion.requests, [Trigger::Automatic]);
    fixture.frame(0.2, vec![Event::Text(".".into())], &[]);
    assert_eq!(
        fixture.completion.requests.last(),
        Some(&Trigger::Character(".".into()))
    );
}

#[test]
fn manual_loading_and_empty_state_follow_original_delay() {
    let mut fixture = Fixture::new();
    fixture.completion.pending = true;
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, vec![key(Key::Space, Modifiers::CTRL)], &[]);
    fixture.frame(0.149, Vec::new(), &[]);
    assert!(fixture.geometry.list.is_none());
    fixture.frame(0.151, Vec::new(), &[]);
    fixture.frame(0.16, Vec::new(), &[]);
    assert!(fixture.painted.iter().any(|text| text == "Loading..."));
    fixture.completion.pending = false;
    fixture.completion.labels.clear();
    fixture.completion.build(&fixture.store, fixture.view);
    fixture.frame(0.2, Vec::new(), &[]);
    assert!(fixture.painted.iter().any(|text| text == "No suggestions."));
    fixture.frame(
        0.3,
        vec![key(Key::Escape, Modifiers::SHIFT), Event::Text("!".into())],
        &[],
    );
    assert_eq!(fixture.text(), "fo!");
}

#[test]
fn completion_details_focus_escape_restores_same_frame_body_text() {
    let mut fixture = Fixture::new();
    fixture.open();
    fixture.frame(
        0.3,
        vec![key(Key::Space, Modifiers::CTRL | Modifiers::ALT)],
        &[],
    );
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(fixture.body.unwrap().with("completion-details")),
        "request pass"
    );
    fixture.frame(0.4, Vec::new(), &[]);
    assert!(fixture.geometry.details.is_some());
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(fixture.body.unwrap().with("completion-details")),
        "response={:?}",
        fixture
            .context
            .read_response(fixture.body.unwrap().with("completion-details"))
            .map(|response| (response.sense, response.enabled(), response.rect))
    );
    let selected = fixture.geometry.selected;
    fixture.frame(0.5, vec![key(Key::ArrowDown, Modifiers::NONE)], &[]);
    assert_eq!(fixture.geometry.selected, selected);
    fixture.frame(
        0.6,
        vec![key(Key::Escape, Modifiers::NONE), Event::Text("!".into())],
        &[],
    );
    assert_eq!(
        fixture.text(),
        "fo!",
        "focused={:?} list={:?} requests={:?}",
        fixture.context.memory(|memory| memory.focused()),
        fixture.geometry.list,
        fixture.completion.requests
    );
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        fixture.body
    );
}

#[test]
fn completion_large_list_virtualizes_and_reveals_last_candidate() {
    let mut fixture = Fixture::new();
    fixture.completion.labels = (0..LARGE_LIST)
        .map(|index| format!("foo{index:04}"))
        .collect();
    fixture.open();
    assert!(fixture.geometry.rows.len() < LARGE_LIST);
    fixture.frame(0.3, Vec::new(), &[Command::Last]);
    fixture.frame(0.4, Vec::new(), &[]);
    assert_eq!(fixture.geometry.selected, Some(LARGE_LIST - 1));
    assert!(
        fixture
            .geometry
            .rows
            .iter()
            .any(|(index, _)| *index == LARGE_LIST - 1)
    );
    fixture.frame(0.5, Vec::new(), &[Command::Accept { alternate: false }]);
    assert_eq!(fixture.text(), format!("foo{:04}", LARGE_LIST - 1));
}

#[test]
fn completion_respects_caller_shortcuts_before_default_navigation() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.intercepted = Some(Key::Space);
    fixture.frame(0.1, vec![key(Key::Space, Modifiers::CTRL)], &[]);
    assert!(fixture.completion.requests.is_empty());
    fixture.intercepted = None;
    fixture.frame(0.2, Vec::new(), &[Command::Trigger]);
    let selected = fixture.geometry.selected;
    fixture.intercepted = Some(Key::ArrowDown);
    fixture.frame(0.3, vec![key(Key::ArrowDown, Modifiers::NONE)], &[]);
    assert_eq!(fixture.geometry.selected, selected);
    assert_eq!(fixture.text(), "fo");
}

#[test]
fn completion_ime_keeps_preedit_and_defers_quick_request_until_commit() {
    let mut fixture = Fixture::new();
    fixture.open();
    fixture.frame(
        0.3,
        vec![Event::Ime(egui::ImeEvent::Preedit {
            text: "한".into(),
            active_range_chars: None,
        })],
        &[],
    );
    assert!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .composition
            .is_some()
    );
    assert_eq!(fixture.text(), "fo");
    assert!(fixture.geometry.list.is_none());
    fixture.frame(0.4, vec![key(Key::Space, Modifiers::CTRL)], &[]);
    assert_eq!(fixture.completion.requests, [Trigger::Manual]);
    fixture.frame(
        0.5,
        vec![Event::Ime(egui::ImeEvent::Commit("한".into()))],
        &[],
    );
    assert_eq!(fixture.text(), "fo한");
    assert!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .composition
            .is_none()
    );
    fixture.frame(0.511, Vec::new(), &[]);
    assert_eq!(
        fixture.completion.requests.last(),
        Some(&Trigger::Automatic)
    );
}

#[test]
fn completion_pointer_accept_preserves_body_focus_and_reports_change() {
    let mut fixture = Fixture::new();
    fixture.open();
    let position = fixture.geometry.rows[1].1.center();
    fixture.frame(
        0.3,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &[],
    );
    let changed = fixture.frame(
        0.4,
        vec![Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    assert!(changed);
    assert_eq!(fixture.completion.accepted.len(), 1);
    fixture.frame(0.5, vec![Event::Text("!".into())], &[]);
    assert!(fixture.text().ends_with('!'));
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        fixture.body
    );
}

#[test]
fn completion_external_focus_cancels_without_stealing_sibling_input() {
    let mut fixture = Fixture::new();
    fixture.open();
    fixture.frame(
        0.3,
        vec![key(Key::Space, Modifiers::CTRL | Modifiers::ALT)],
        &[],
    );
    fixture.frame(0.4, Vec::new(), &[]);
    let other = fixture.other_id.unwrap();
    fixture
        .context
        .memory_mut(|memory| memory.request_focus(other));
    fixture.frame(0.5, vec![Event::Text("other".into())], &[]);
    assert_eq!(fixture.text(), "fo");
    assert_eq!(fixture.other, "other");
    assert!(fixture.geometry.list.is_none());
    assert_eq!(
        fixture.context.memory(|memory| memory.focused()),
        Some(other)
    );
}

#[test]
fn completion_label_escapes_crlf_and_preserves_unicode_highlight_and_deprecation() {
    let mut fixture = Fixture::new();
    fixture.completion.labels = vec!["fo\u{1f642}\r\nx".into()];
    fixture.open();
    let label = "fo\u{1f642}\u{23ce}x";
    let job = fixture.jobs.iter().find(|job| job.text == label).unwrap();
    assert!(
        job.sections
            .iter()
            .any(|section| section.format.color == Color32::YELLOW)
    );
    assert!(
        job.sections
            .iter()
            .any(|section| section.format.font_id.family
                == egui::FontFamily::Name(
                    taide_native_ui::font_families::EDITOR_BOLD_FAMILY.into()
                ))
    );
    assert!(fixture.painted.iter().any(|text| text == "functiondetail"));
    fixture.completion.deprecated = true;
    fixture.completion.build(&fixture.store, fixture.view);
    fixture.frame(0.3, Vec::new(), &[]);
    let job = fixture.jobs.iter().find(|job| job.text == label).unwrap();
    assert!(
        job.sections
            .iter()
            .all(|section| section.format.color != Color32::YELLOW)
    );
    assert!(
        job.sections
            .iter()
            .all(|section| section.format.strikethrough.width > 0.0)
    );
}
