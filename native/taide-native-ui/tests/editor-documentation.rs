#![cfg(all(feature = "native-host", feature = "inspection"))]

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use egui::{
    Color32, Context, Event, FontFamily, FontId, Id, Key, Modifiers, RawInput, Rect, Ui, Vec2,
    pos2, vec2,
};
use serde_json::json;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::documentation::{
    Block, Command, Inline, Kind, RichDocument, SignatureCommand, SignatureTriggers, Signatures,
    Span,
};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_documentation::{Colors, Content, Part, Provider, Widget};
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorOutput, EditorPresentation, EditorRequest, NativeEditor,
};

const SCREEN: Vec2 = vec2(800.0, 600.0);
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const BYTE_LIMIT: usize = 4096;
const TEXT: &str = "first\nmethod(x,\nend";

#[derive(Default)]
struct Documentation {
    widgets: HashMap<(ViewId, Kind), Widget>,
    requests: Vec<(ViewId, Kind, usize, bool)>,
    closed: Vec<(ViewId, Kind)>,
    links: Vec<String>,
    generation: u64,
    pending: bool,
    image: Option<egui::TextureHandle>,
    image_rect: Option<Rect>,
    image_ids: Vec<Id>,
}

fn rich(text: &str) -> Arc<RichDocument> {
    Arc::new(RichDocument {
        blocks: vec![Block::Paragraph(vec![Inline::Text(Span {
            text: text.into(),
            ..Default::default()
        })])],
    })
}

impl Provider for Documentation {
    fn available(&self, _: &EditorStore, _: ViewId, _: Kind) -> bool {
        true
    }
    fn version(&self, _: &EditorStore, _: ViewId) -> String {
        self.generation.to_string()
    }
    fn triggers(&self, _: &EditorStore, _: ViewId) -> SignatureTriggers {
        let options = serde_json::from_value(
            json!({ "triggerCharacters": ["(", ","], "retriggerCharacters": [")"] }),
        )
        .unwrap();
        SignatureTriggers::from_options([&options])
    }
    fn word(&self, store: &EditorStore, view: ViewId, byte: usize) -> Option<Range<usize>> {
        let document = store
            .documents()
            .snapshot(store.views().get(view)?.document)
            .ok()?;
        let line = taide_native_editor::editing::line_content_range(
            &document,
            document.rope.byte_to_line(byte),
        );
        let text = document.rope.byte_slice(line.clone()).to_string();
        let start = text[..byte - line.start]
            .char_indices()
            .rev()
            .find(|(_, character)| !character.is_alphanumeric())
            .map_or(0, |(index, character)| index + character.len_utf8());
        let end = text[byte - line.start..]
            .char_indices()
            .find(|(_, character)| !character.is_alphanumeric())
            .map_or(text.len(), |(index, _)| index + byte - line.start);
        Some(line.start + start..line.start + end)
    }
    fn request(
        &mut self,
        store: &EditorStore,
        view: ViewId,
        kind: Kind,
        byte: usize,
        keyboard: bool,
    ) -> Result<bool, EditorError> {
        self.requests.push((view, kind, byte, keyboard));
        let content = if self.pending {
            None
        } else {
            Some(match kind {
                Kind::Hover => Content::Hover(vec![Part { range: self.word(store, view, byte).unwrap(), documents: vec![rich("hover documentation")] }]),
                Kind::Signature => Content::Signature {
                    model: Arc::new(Signatures::new(serde_json::from_value(json!({
                        "signatures": [
                            { "label": "f(x)", "activeParameter": 0, "parameters": [{ "label": "x" }] },
                            { "label": "other(x)", "activeParameter": 0 }
                        ], "activeSignature": 0, "activeParameter": 0
                    })).unwrap()).unwrap()), parameter: Some(rich("parameter docs")), documentation: Some(rich("signature docs")),
                },
            })
        };
        self.widgets.insert(
            (view, kind),
            Widget {
                token: self.requests.len().to_string(),
                byte,
                keyboard,
                pending: self.pending,
                content,
            },
        );
        Ok(true)
    }
    fn current(&self, _: &EditorStore, view: ViewId, kind: Kind) -> Option<Widget> {
        self.widgets.get(&(view, kind)).cloned()
    }
    fn close(&mut self, view: ViewId, kind: Kind) {
        self.closed.push((view, kind));
        self.widgets.remove(&(view, kind));
    }
    fn cycle(&mut self, _: &EditorStore, view: ViewId, forward: bool) -> bool {
        let Some(Content::Signature { model, .. }) = self
            .widgets
            .get_mut(&(view, Kind::Signature))
            .and_then(|widget| widget.content.as_mut())
        else {
            return false;
        };
        Arc::make_mut(model).next(forward, true)
    }
    fn open_link(&mut self, _: &Ui, target: &str) -> bool {
        self.links.push(target.into());
        true
    }
    fn image(
        &mut self,
        ui: &mut Ui,
        _: &str,
        alt: &str,
        dimensions: taide_native_editor::documentation::ImageDimensions,
    ) -> Option<egui::Response> {
        let texture = self.image.as_ref()?;
        let size = vec2(
            dimensions.width.unwrap_or_default() as f32,
            dimensions.height.unwrap_or_default() as f32,
        );
        let response = ui.add(
            egui::Image::new(texture)
                .fit_to_exact_size(size)
                .maintain_aspect_ratio(false)
                .alt_text(alt)
                .sense(egui::Sense::click()),
        );
        self.image_rect = Some(response.rect);
        self.image_ids.push(response.id);
        Some(response)
    }
}

#[test]
fn 표시한_문서_이미지의_링크는_클릭과_본문_포커스를_보존한다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, Vec::new(), &[Command::ShowHover]);
    fixture.documentation.image = Some(fixture.context.load_texture(
        "synthetic-doc-image",
        egui::ColorImage::filled([2, 1], Color32::WHITE),
        egui::TextureOptions::LINEAR,
    ));
    let target = "file:///synthetic/source.rs#L2,5";
    fixture
        .documentation
        .widgets
        .get_mut(&(fixture.view, Kind::Hover))
        .unwrap()
        .content = Some(Content::Hover(vec![Part {
        range: 0..5,
        documents: vec![Arc::new(RichDocument {
            blocks: vec![Block::Paragraph(vec![Inline::Image {
                source: "synthetic://image".into(),
                alt: "diagram".into(),
                title: "image title".into(),
                link: Some(taide_native_editor::documentation::Link {
                    target: target.into(),
                    title: String::new(),
                }),
                dimensions: taide_native_editor::documentation::ImageDimensions {
                    width: Some(40),
                    height: Some(20),
                },
            }])],
        })],
    }]));
    let second_target = "file:///synthetic/second.rs#L3,1";
    let Some(Content::Hover(parts)) = fixture
        .documentation
        .widgets
        .get_mut(&(fixture.view, Kind::Hover))
        .unwrap()
        .content
        .as_mut()
    else {
        panic!()
    };
    let mut second = parts[0].documents[0].as_ref().clone();
    let Block::Paragraph(inlines) = &mut second.blocks[0] else {
        panic!()
    };
    let Inline::Image {
        link: Some(link), ..
    } = &mut inlines[0]
    else {
        panic!()
    };
    link.target = second_target.into();
    parts[0].documents.push(Arc::new(second));
    fixture.frame(0.2, Vec::new(), &[]);
    fixture.frame(0.3, Vec::new(), &[]);
    let rect = fixture.documentation.image_rect.unwrap();
    assert_eq!(rect.size(), vec2(40.0, 20.0));
    let ids = fixture
        .documentation
        .image_ids
        .iter()
        .rev()
        .take(2)
        .copied()
        .collect::<Vec<_>>();
    assert_ne!(ids[0], ids[1]);
    fixture.frame(
        0.4,
        vec![
            Event::PointerMoved(rect.center()),
            Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &[],
    );
    fixture.frame(
        0.5,
        vec![Event::PointerButton {
            pos: rect.center(),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    assert_eq!(fixture.documentation.links, [second_target]);
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(fixture.body.unwrap()))
    );
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 짧은_호버는_내용보다_큰_너비와_높이로_늘어나지않는다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, Vec::new(), &[Command::ShowHover]);
    fixture.frame(0.2, Vec::new(), &[]);
    fixture.frame(0.3, Vec::new(), &[]);
    let before = fixture.popup(Kind::Hover).unwrap();
    let start = pos2(before.right(), before.center().y);
    fixture.frame(
        0.4,
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
    let end = start + vec2(100.0, 0.0);
    fixture.frame(0.5, vec![Event::PointerMoved(end)], &[]);
    fixture.frame(
        0.6,
        vec![Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    fixture.frame(0.7, Vec::new(), &[]);
    let after = fixture.popup(Kind::Hover).unwrap();
    assert!(
        after.width() <= before.width() + 1.0,
        "before={before:?}, after={after:?}"
    );
    let start = pos2(after.center().x, after.bottom());
    let end = start + vec2(0.0, 100.0);
    fixture.frame(
        0.8,
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
    fixture.frame(0.9, vec![Event::PointerMoved(end)], &[]);
    fixture.frame(
        1.0,
        vec![Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    fixture.frame(1.1, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).unwrap().height() <= before.height() + 1.0);
}

#[test]
fn 읽기전용_문서의_도움말은_편집을_허용하지않고_같은_모델의_다른_뷰와_분리된다() {
    let mut fixture = Fixture::new();
    let document = fixture
        .store
        .open_file(
            std::path::PathBuf::from("/synthetic/readonly.rs"),
            taide_model::file::OpenedFile {
                path: "/synthetic/readonly.rs".into(),
                content: TEXT.into(),
                language_id: "rust".into(),
                byte_size: u32::try_from(TEXT.len()).unwrap(),
                line_count: 3,
                tier: taide_model::file::FileSizeTier::Normal,
                read_only: true,
                encoding_lossy: false,
                modified_ms: 0.0,
                editor_config: Default::default(),
            },
        )
        .unwrap();
    let first = fixture
        .store
        .attach_view(
            ViewKey {
                window: "readonly".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let second = fixture
        .store
        .attach_view(
            ViewKey {
                window: "readonly".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    fixture.document = document;
    fixture.view = first;
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, Vec::new(), &[Command::ShowHover]);
    fixture.frame(0.2, Vec::new(), &[]);
    fixture.frame(0.3, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).is_some());
    fixture.frame_expect_errors(
        0.4,
        vec![Event::Text("blocked".into())],
        &[],
        &[EditorError::ReadOnly],
    );
    assert_eq!(fixture.text(), TEXT);
    assert_eq!(
        fixture
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        0
    );
    fixture.view = second;
    fixture.body = None;
    fixture.frame(0.5, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).is_none());
    fixture.frame(
        0.6,
        Vec::new(),
        &[
            Command::ShowHover,
            Command::Signature(SignatureCommand::Trigger),
        ],
    );
    fixture.frame(0.7, Vec::new(), &[]);
    fixture.frame(0.8, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).is_some());
    assert!(fixture.popup(Kind::Signature).is_some());
    assert!(
        fixture
            .documentation
            .requests
            .iter()
            .any(|(view, kind, _, _)| *view == first && *kind == Kind::Hover)
    );
    assert!(
        fixture
            .documentation
            .requests
            .iter()
            .any(|(view, kind, _, _)| *view == second && *kind == Kind::Signature)
    );
    assert_eq!(fixture.text(), TEXT);
    assert!(!fixture.store.documents().snapshot(document).unwrap().dirty);
}

struct Fixture {
    context: Context,
    store: EditorStore,
    document: DocumentId,
    view: ViewId,
    editor: NativeEditor,
    documentation: Documentation,
    geometry: Option<EditorGeometry>,
    body: Option<Id>,
    popups: taide_native_ui::editor_documentation::Geometry,
    painted: Vec<String>,
}

impl Fixture {
    fn new() -> Self {
        let context = Context::default();
        context.set_os(egui::os::OperatingSystem::Mac);
        let mut fonts = egui::FontDefinitions::default();
        fonts.families.insert(
            FontFamily::Name(taide_native_ui::font_families::EDITOR_BOLD_FAMILY.into()),
            fonts.families[&FontFamily::Monospace].clone(),
        );
        context.set_fonts(fonts);
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 4,
            max_views: 4,
            max_undo_groups: 4,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let document = store
            .open_untitled(TabId::new(), TEXT.into(), "rust".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "fixture".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                document,
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
            documentation: Documentation::default(),
            geometry: None,
            body: None,
            popups: Default::default(),
            painted: Vec::new(),
        }
    }

    fn frame(&mut self, time: f64, events: Vec<Event>, commands: &[Command]) -> EditorOutput {
        self.frame_expect_errors(time, events, commands, &[])
    }

    fn frame_expect_errors(
        &mut self,
        time: f64,
        events: Vec<Event>,
        commands: &[Command],
        expected: &[EditorError],
    ) -> EditorOutput {
        let mut rendered = None;
        let mut presentation = EditorPresentation::default();
        presentation.options.documentation_colors = Some(Colors {
            background: Color32::DARK_GRAY,
            foreground: Color32::WHITE,
            border: Color32::GRAY,
            highlight: Color32::YELLOW,
            link: Color32::LIGHT_BLUE,
            code_background: Color32::BLACK,
            shadow: Color32::BLACK,
        });
        let initial = self.body.is_none();
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                rendered = Some(
                    self.editor
                        .show_request(
                            ui,
                            &mut self.store,
                            self.view,
                            EditorRequest {
                                request_focus: initial,
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
                                syntax_folds: None,
                                documentation: Some(&mut self.documentation),
                                documentation_commands: commands,
                                #[cfg(feature = "native-host")]
                                completion: None,
                                #[cfg(feature = "native-host")]
                                completion_commands: &[],
                            },
                        )
                        .unwrap(),
                );
            },
        );
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
        assert_eq!(rendered.errors, expected);
        self.geometry = Some(rendered.geometry.clone());
        self.body = Some(rendered.response.id);
        self.popups = rendered.documentation_geometry;
        rendered
    }

    fn text(&self) -> String {
        self.store
            .documents()
            .snapshot(self.document)
            .unwrap()
            .rope
            .to_string()
    }
    fn popup(&self, kind: Kind) -> Option<Rect> {
        match kind {
            Kind::Hover => self.popups.hover,
            Kind::Signature => self.popups.signature,
        }
    }
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
fn 포인터_호버는_원본_지연과_sticky_숨김을_따르고_본문을_편집하지않는다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    let point = fixture
        .geometry
        .as_ref()
        .unwrap()
        .caret_rect("first\nme".len())
        .unwrap()
        .center()
        + vec2(1.0, 0.0);
    fixture.frame(0.05, vec![Event::PointerMoved(point)], &[]);
    fixture.frame(0.19, Vec::new(), &[]);
    assert!(fixture.documentation.requests.is_empty());
    fixture.frame(0.21, Vec::new(), &[]);
    assert_eq!(fixture.documentation.requests.len(), 1);
    assert!(fixture.popup(Kind::Hover).is_none());
    fixture.frame(0.36, Vec::new(), &[]);
    fixture.frame(0.37, Vec::new(), &[]);
    let popup = fixture.popup(Kind::Hover).unwrap();
    fixture.frame(0.40, vec![Event::PointerMoved(popup.center())], &[]);
    let moved_popup = fixture.popup(Kind::Hover);
    assert!(
        fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover)),
        "before={popup:?}; after={moved_popup:?}; requests={:?}; closed={:?}",
        fixture.documentation.requests,
        fixture.documentation.closed
    );
    fixture.frame(1.0, Vec::new(), &[]);
    assert!(
        fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover)),
        "before={popup:?}; moved={moved_popup:?}; after={:?}; requests={:?}; closed={:?}",
        fixture.popup(Kind::Hover),
        fixture.documentation.requests,
        fixture.documentation.closed
    );
    fixture.frame(1.1, vec![Event::PointerGone], &[]);
    fixture.frame(1.35, Vec::new(), &[]);
    assert!(
        fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover))
    );
    fixture.frame(1.42, Vec::new(), &[]);
    assert!(
        !fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover))
    );
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 명시_호버의_두번째_실행은_focus하고_escape_뒤_같은_frame_문자를_본문에_전달한다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, Vec::new(), &[Command::ShowHover]);
    fixture.frame(0.2, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).is_some());
    fixture.frame(0.3, Vec::new(), &[Command::ShowHover]);
    let popup_id = fixture.body.unwrap().with("documentation-hover");
    assert!(fixture.context.memory(|memory| memory.has_focus(popup_id)));
    assert_eq!(fixture.documentation.requests.len(), 1);
    fixture.frame(
        0.4,
        vec![key(Key::Escape, Modifiers::NONE), Event::Text("Q".into())],
        &[],
    );
    assert_eq!(fixture.text(), format!("Q{TEXT}"));
    assert!(
        !fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover))
    );
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(fixture.body.unwrap()))
    );
}

#[test]
fn 자동_시그니처는_trailing_요청과_복수서명_키를_소비하고_다음_문자와_escape를_보존한다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, vec![Event::Text("f(".into())], &[]);
    fixture.frame(0.2, Vec::new(), &[]);
    assert!(fixture.documentation.requests.is_empty());
    fixture.frame(0.23, Vec::new(), &[]);
    fixture.frame(0.24, Vec::new(), &[]);
    assert_eq!(fixture.documentation.requests.len(), 1);
    assert_eq!(fixture.documentation.requests[0].1, Kind::Signature);
    assert!(fixture.popup(Kind::Signature).is_some());
    fixture.frame(
        0.3,
        vec![
            key(Key::ArrowDown, Modifiers::NONE),
            Event::Text("x".into()),
        ],
        &[],
    );
    let Some(Content::Signature { model, .. }) = fixture.documentation.widgets
        [&(fixture.view, Kind::Signature)]
        .content
        .as_ref()
    else {
        panic!("expected signature")
    };
    assert_eq!(model.index(), 1);
    assert_eq!(fixture.text(), format!("f(x{TEXT}"));
    fixture.frame(
        0.4,
        vec![key(Key::Escape, Modifiers::SHIFT), Event::Text("Z".into())],
        &[],
    );
    assert!(
        !fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Signature))
    );
    assert_eq!(fixture.text(), format!("f(xZ{TEXT}"));
}

#[test]
fn provider_교체와_본문_선택변경은_문서_호버를_닫는다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, Vec::new(), &[Command::ShowHover]);
    fixture.frame(0.2, Vec::new(), &[]);
    assert!(
        fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover))
    );
    fixture.documentation.generation += 1;
    fixture.frame(0.3, Vec::new(), &[]);
    assert!(
        !fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover))
    );
    fixture.frame(0.4, Vec::new(), &[Command::ShowHover]);
    fixture.frame(0.5, vec![key(Key::ArrowRight, Modifiers::NONE)], &[]);
    assert!(
        !fixture
            .documentation
            .widgets
            .contains_key(&(fixture.view, Kind::Hover))
    );
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 보류_호버는_900ms_대기표시를_보이고_부분응답_문서를_함께_유지한다() {
    let mut fixture = Fixture::new();
    fixture.documentation.pending = true;
    fixture.frame(0.0, Vec::new(), &[]);
    let point = fixture
        .geometry
        .as_ref()
        .unwrap()
        .caret_rect("first\nme".len())
        .unwrap()
        .center()
        + vec2(1.0, 0.0);
    fixture.frame(0.05, vec![Event::PointerMoved(point)], &[]);
    fixture.frame(0.21, Vec::new(), &[]);
    fixture.frame(0.94, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).is_none());
    fixture.frame(0.96, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).is_some());
    fixture.frame(0.97, Vec::new(), &[]);
    assert!(fixture.painted.iter().any(|text| text == "Loading..."));
    fixture
        .documentation
        .widgets
        .get_mut(&(fixture.view, Kind::Hover))
        .unwrap()
        .content = Some(Content::Hover(vec![Part {
        range: 6..12,
        documents: vec![rich("partial documentation")],
    }]));
    fixture.frame(1.0, Vec::new(), &[]);
    assert!(
        fixture
            .painted
            .iter()
            .any(|text| text == "partial documentation")
    );
    assert!(fixture.painted.iter().any(|text| text == "Loading..."));
    assert_eq!(fixture.documentation.requests.len(), 1);
}

#[test]
fn 시그니처_다음_버튼은_본문_focus를_보존하고_같은_frame_입력을_막지않는다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(
        0.1,
        Vec::new(),
        &[Command::Signature(SignatureCommand::Trigger)],
    );
    fixture.frame(0.2, Vec::new(), &[]);
    fixture.frame(0.3, Vec::new(), &[]);
    let next = fixture
        .context
        .read_response(
            fixture
                .body
                .unwrap()
                .with("documentation-signature")
                .with("next"),
        )
        .unwrap()
        .rect
        .center();
    fixture.frame(
        0.4,
        vec![
            Event::PointerMoved(next),
            Event::PointerButton {
                pos: next,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            Event::PointerButton {
                pos: next,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
            Event::Text("Z".into()),
        ],
        &[],
    );
    assert_eq!(fixture.text(), format!("Z{TEXT}"));
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(fixture.body.unwrap()))
    );
    let Some(Content::Signature { model, .. }) = fixture.documentation.widgets
        [&(fixture.view, Kind::Signature)]
        .content
        .as_ref()
    else {
        panic!("expected signature")
    };
    assert_eq!(model.index(), 1);
}

#[test]
fn 포커스한_호버의_provider_교체는_본문에_포커스와_같은_frame_문자를_돌려준다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, Vec::new(), &[Command::ShowHover]);
    fixture.frame(0.2, Vec::new(), &[]);
    fixture.frame(0.3, Vec::new(), &[Command::ShowHover]);
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(fixture.body.unwrap().with("documentation-hover")))
    );
    fixture.documentation.generation += 1;
    fixture.frame(0.4, vec![Event::Text("Z".into())], &[]);
    assert_eq!(fixture.text(), format!("Z{TEXT}"));
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(fixture.body.unwrap()))
    );
    assert!(fixture.popup(Kind::Hover).is_none());
}

#[test]
fn 호버의_오른쪽_크기조절은_내용과_본문_focus를_유지한다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(0.1, Vec::new(), &[Command::ShowHover]);
    fixture
        .documentation
        .widgets
        .get_mut(&(fixture.view, Kind::Hover))
        .unwrap()
        .content = Some(Content::Hover(vec![Part {
        range: 0..5,
        documents: vec![rich(&"long documentation ".repeat(40))],
    }]));
    fixture.frame(0.2, Vec::new(), &[]);
    fixture.frame(0.3, Vec::new(), &[]);
    let before = fixture.popup(Kind::Hover).unwrap();
    let start = pos2(before.right(), before.center().y);
    fixture.frame(
        0.4,
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
    let end = start - vec2(100.0, 0.0);
    fixture.frame(0.5, vec![Event::PointerMoved(end)], &[]);
    fixture.frame(
        0.6,
        vec![Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &[],
    );
    fixture.frame(0.7, Vec::new(), &[]);
    let after = fixture.popup(Kind::Hover).unwrap();
    assert!(
        after.width() < before.width() - 80.0,
        "before={before:?}, after={after:?}"
    );
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(fixture.body.unwrap()))
    );
    assert_eq!(fixture.documentation.requests.len(), 1);
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn ime_조합_중_명시_도움말은_조합과_본문_입력을_보존한다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(
        0.1,
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
    fixture.frame(
        0.2,
        Vec::new(),
        &[
            Command::ShowHover,
            Command::Signature(SignatureCommand::Trigger),
        ],
    );
    assert!(fixture.documentation.requests.is_empty());
    assert!(
        fixture
            .store
            .views()
            .get(fixture.view)
            .unwrap()
            .composition
            .is_some()
    );
    fixture.frame(
        0.3,
        vec![Event::Ime(egui::ImeEvent::Commit("한".into()))],
        &[],
    );
    assert_eq!(fixture.text(), format!("한{TEXT}"));
    assert!(fixture.documentation.requests.is_empty());
}

#[test]
fn 명시_호버는_command_chord의_modifier가_눌려있어도_표시한다() {
    let mut fixture = Fixture::new();
    fixture.frame(0.0, Vec::new(), &[]);
    fixture.frame(
        0.1,
        vec![Event::ModifiersChanged(
            Modifiers::MAC_CMD | Modifiers::COMMAND,
        )],
        &[Command::ShowHover],
    );
    fixture.frame(0.2, Vec::new(), &[]);
    assert!(fixture.popup(Kind::Hover).is_some());
    assert_eq!(fixture.documentation.requests.len(), 1);
    assert_eq!(fixture.text(), TEXT);
}
