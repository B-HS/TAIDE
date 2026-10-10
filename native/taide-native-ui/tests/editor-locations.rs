#![cfg(feature = "native-host")]

use std::sync::Arc;

use egui::{
    Color32, Context, Event, FontId, Id, Key, Modifiers, RawInput, Rect, Ui, Vec2, pos2, vec2,
};
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{DocumentId, EditorError};
use taide_native_editor::lsp::{LspRange as Range, Position};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::symbol_locations::{Command, Locations, Mode, Target};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_geometry::EditorGeometry;
use taide_native_ui::editor_locations::{Colors, Provider, Widget};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorPresentation, EditorRequest, NativeEditor,
};

const SCREEN: Vec2 = vec2(640.0, 480.0);
const LINE_HEIGHT: f32 = 20.0;
const FONT_SIZE: f32 = 14.0;
const BYTES: usize = 4096;
const SOURCE: &str = "/synthetic/source.rs";
const TARGET: &str = "/synthetic/target.rs";

struct Preview {
    editor: NativeEditor,
    widget: Option<Widget>,
    document: DocumentId,
    view: ViewId,
    focus: Option<Id>,
    opened: Vec<(usize, bool)>,
    available: bool,
    requested: Vec<(usize, Mode)>,
    reads: std::cell::Cell<usize>,
    keyboard: Option<(String, usize)>,
}

impl Provider for Preview {
    fn keyboard_link(&self, _: &EditorStore, _: ViewId) -> Option<(String, usize)> {
        self.keyboard.clone()
    }
    fn clear_link(&mut self, _: ViewId) {
        self.keyboard = None;
    }
    fn link_preview(
        &mut self,
        _: &EditorStore,
        _: ViewId,
        _: usize,
    ) -> Option<taide_native_ui::editor_locations::LinkPreview> {
        self.keyboard.as_ref()?;
        let mut code = egui::text::LayoutJob::default();
        code.append(
            "styled definition",
            0.0,
            egui::TextFormat {
                font_id: FontId::monospace(FONT_SIZE),
                color: Color32::RED,
                ..Default::default()
            },
        );
        Some(taide_native_ui::editor_locations::LinkPreview {
            bytes: 0.."source".len(),
            text: "fallback definition".into(),
            code: Some(code),
        })
    }
    fn preserve_focus(&mut self, _: ViewId, focus: taide_native_ui::editor_locations::Focus) {
        if let Some(widget) = &mut self.widget {
            widget.focus = Some((
                widget.focus.map_or(0, |(generation, _)| generation + 1),
                focus,
            ));
        }
    }
    fn definition_available(&self, _: &EditorStore, _: ViewId) -> bool {
        self.available
    }
    fn source_word(
        &self,
        store: &EditorStore,
        view: ViewId,
        byte: usize,
    ) -> Option<std::ops::Range<usize>> {
        let document = store
            .documents()
            .snapshot(store.views().get(view)?.document)
            .ok()?;
        let line = document.rope.byte_to_line(byte);
        let start = document.rope.line_to_byte(line);
        let end = start + document.rope.line(line).len_bytes();
        (byte < end
            && !document
                .rope
                .byte_slice(byte..)
                .chars()
                .next()?
                .is_whitespace())
        .then_some(start..end)
    }
    fn request_at(
        &mut self,
        _: &mut EditorStore,
        _: ViewId,
        byte: usize,
        mode: Mode,
    ) -> Result<bool, EditorError> {
        self.requested.push((byte, mode));
        Ok(true)
    }

    fn current(&mut self, _: &EditorStore, _: ViewId) -> Option<Widget> {
        self.widget.clone()
    }
    fn execute(
        &mut self,
        _: &mut EditorStore,
        _: ViewId,
        command: Command,
    ) -> Result<bool, EditorError> {
        match command {
            Command::Close => self.widget = None,
            Command::Next | Command::Previous => {
                let widget = self.widget.as_mut().unwrap();
                widget.selected = widget
                    .model
                    .next(widget.selected.unwrap(), command == Command::Next);
            }
            Command::Select { index, .. } => self.widget.as_mut().unwrap().selected = Some(index),
            Command::OpenSelected { side } => self
                .opened
                .push((self.widget.as_ref().unwrap().selected.unwrap(), side)),
            Command::GotoSelected => self
                .opened
                .push((self.widget.as_ref().unwrap().selected.unwrap(), false)),
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn render_preview(
        &mut self,
        ui: &mut Ui,
        store: &mut EditorStore,
        _: ViewId,
        rect: Rect,
        focus: bool,
    ) -> Result<Vec<Id>, EditorError> {
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("synthetic-preview")
                .max_rect(rect),
        );
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        let output = self.editor.show_with_input_route(
            &mut ui,
            store,
            self.view,
            focus,
            |_, _, _| false,
            |response| response.ctx.keyboard_input_route(response.id),
        )?;
        self.focus = Some(output.response.id);
        Ok(vec![output.response.id])
    }
    fn preview_document(&self, _: &EditorStore, _: &Target) -> Option<DocumentId> {
        self.reads.set(self.reads.get() + 1);
        Some(self.document)
    }
    fn file_label(&self, _: &Target) -> String {
        "target.rs".into()
    }
    fn word_start(&self, _: &EditorStore, _: DocumentId, byte: usize) -> usize {
        byte
    }
}

struct Fixture {
    context: Context,
    editor: NativeEditor,
    store: EditorStore,
    source: ViewId,
    provider: Preview,
    presentation: EditorPresentation,
    body: Option<Id>,
    time: f64,
}

impl Fixture {
    fn new(read_only: bool) -> Self {
        let editor = NativeEditor {
            appearance: EditorAppearance {
                font: FontId::monospace(FONT_SIZE),
                line_height: LINE_HEIGHT,
                horizontal_padding: 8.0,
                background: Color32::BLACK,
                foreground: Color32::WHITE,
                muted: Color32::GRAY,
                selection: Color32::BLUE,
                cursor: Color32::WHITE,
                current_line: Color32::DARK_GRAY,
                line_numbers: true,
                indent: "    ".into(),
            },
        };
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 2,
            max_views: 2,
            max_undo_groups: 8,
            max_document_bytes: BYTES,
        })
        .unwrap();
        let open = |store: &mut EditorStore, path: &str, content: &str, read_only| {
            let document = store
                .open_file(
                    path.into(),
                    OpenedFile {
                        path: path.into(),
                        content: content.into(),
                        language_id: "rust".into(),
                        byte_size: content.len().try_into().unwrap(),
                        line_count: content.lines().count().try_into().unwrap(),
                        tier: FileSizeTier::Normal,
                        read_only,
                        encoding_lossy: false,
                        modified_ms: 0.0,
                        editor_config: EditorConfigOptions::default(),
                    },
                )
                .unwrap();
            let view = store
                .attach_view(
                    ViewKey {
                        window: "synthetic".into(),
                        pane: PaneId::new(),
                        tab: TabId::new(),
                    },
                    document,
                )
                .unwrap();
            (document, view)
        };
        let (_, source) = open(&mut store, SOURCE, "source\nsecond\nthird\nend", read_only);
        let (document, view) = open(&mut store, TARGET, "def\n  \u{1f600}target\nend", false);
        let targets = [
            Range::new(Position::new(0, 0), Position::new(0, 3)),
            Range::new(Position::new(1, 4), Position::new(1, 10)),
        ]
        .map(|range| Target {
            uri: "file:///synthetic/target.rs".parse().unwrap(),
            range,
            selection: range,
            origin: None,
        });
        let provider = Preview {
            editor: NativeEditor {
                appearance: editor.appearance.clone(),
            },
            widget: Some(Widget {
                token: "synthetic".into(),
                position: 0,
                title: "Definitions".into(),
                model: Arc::new(Locations::new(targets.to_vec())),
                selected: Some(0),
                focus: None,
            }),
            document,
            view,
            focus: None,
            opened: Vec::new(),
            available: false,
            requested: Vec::new(),
            reads: std::cell::Cell::new(0),
            keyboard: None,
        };
        let mut presentation = EditorPresentation::default();
        presentation.options.location_colors = Some(Colors {
            border: Color32::BLUE,
            background: Color32::BLACK,
            heading_background: Color32::DARK_GRAY,
            heading: Color32::WHITE,
            detail: Color32::GRAY,
            tree_background: Color32::DARK_GRAY,
            selection_background: Color32::BLUE,
            selection_foreground: Color32::WHITE,
            highlight: Color32::YELLOW,
            highlight_border: Color32::RED,
            link: Color32::LIGHT_BLUE,
        });
        Self {
            context: Context::default(),
            editor,
            store,
            source,
            provider,
            presentation,
            body: None,
            time: 0.0,
        }
    }
    fn frame(&mut self, events: Vec<Event>, focus: Option<Id>) -> (EditorGeometry, Vec<String>) {
        self.frame_modifiers(events, focus, Modifiers::NONE)
    }
    fn frame_modifiers(
        &mut self,
        events: Vec<Event>,
        focus: Option<Id>,
        modifiers: Modifiers,
    ) -> (EditorGeometry, Vec<String>) {
        if let Some(focus) = focus {
            self.context
                .memory_mut(|memory| memory.request_focus(focus));
        }
        self.time += 1.0;
        let mut geometry = None;
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
                time: Some(self.time),
                events: std::iter::once(Event::ModifiersChanged(modifiers))
                    .chain(events)
                    .collect(),
                ..Default::default()
            },
            |ui| {
                let output = self
                    .editor
                    .show_request(
                        ui,
                        &mut self.store,
                        self.source,
                        EditorRequest {
                            request_focus: false,
                            keymap: |_: &Ui, _: &Event, _: bool| false,
                            route: |response: &egui::Response| {
                                response.ctx.keyboard_input_route(response.id)
                            },
                            presentation: &self.presentation,
                            tokens: |_: &EditorStore| None,
                            language: None,
                            decorations: &[],
                            fold_commands: &[],
                            fold_controls: None,
                            syntax_folds: None,
                            #[cfg(feature = "native-host")]
                            documentation: None,
                            #[cfg(feature = "native-host")]
                            documentation_commands: &[],
                            #[cfg(feature = "native-host")]
                            completion: None,
                            #[cfg(feature = "native-host")]
                            completion_commands: &[],
                            problems: None,
                            locations: Some(&mut self.provider),
                        },
                    )
                    .unwrap();
                self.body = Some(output.response.id);
                geometry = Some(output.geometry);
            },
        );
        let mut text = Vec::new();
        for shape in &output.shapes {
            text_in(&shape.shape, &mut text);
        }
        output.textures_delta.clear();
        (geometry.unwrap(), text)
    }
    fn text(&self, view: ViewId) -> String {
        self.store
            .documents()
            .snapshot(self.store.views().get(view).unwrap().document)
            .unwrap()
            .rope
            .to_string()
    }
}

fn text_in(shape: &egui::epaint::Shape, text: &mut Vec<String>) {
    match shape {
        egui::epaint::Shape::Text(shape) => text.push(shape.galley.text().into()),
        egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                text_in(shape, text);
            }
        }
        _ => {}
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
fn 참조_zone은_원본_제목과_미리보기와_행을_표시하고_다음_본문을_아래로_민다() {
    let mut f = Fixture::new(false);
    let (geometry, text) = f.frame(Vec::new(), None);
    assert!(
        text.iter().any(|text| text.contains("Definitions (2)")),
        "{text:?}"
    );
    assert!(text.iter().any(|text| text.contains("target")), "{text:?}");
    assert!(
        geometry.caret_rect("source\n".len()).unwrap().top()
            >= LINE_HEIGHT * (taide_native_ui::editor_locations::DEFAULT_LINES + 1.0)
    );
    f.provider.widget = None;
    let (geometry, _) = f.frame(Vec::new(), None);
    assert!(geometry.caret_rect("source\n".len()).unwrap().top() < LINE_HEIGHT * 2.0);
}

#[test]
fn 키보드_정의_hover는_포인터_없이_코드를_표시하고_다음_키에서_닫는다() {
    let mut f = Fixture::new(false);
    f.provider.widget = None;
    f.provider.available = true;
    f.frame(Vec::new(), None);
    f.provider.keyboard = Some(("keyboard-request".into(), 0));
    f.frame(Vec::new(), Some(f.body.unwrap()));
    let (_, text) = f.frame(Vec::new(), Some(f.body.unwrap()));
    assert!(
        text.iter().any(|text| text.contains("styled definition")),
        "{text:?}"
    );
    assert!(!text.iter().any(|text| text.contains("fallback definition")));
    assert!(f.provider.requested.is_empty());
    let (_, text) = f.frame(vec![key(Key::ArrowRight, Modifiers::NONE)], None);
    assert!(f.provider.keyboard.is_none());
    assert!(!text.iter().any(|text| text.contains("styled definition")));
    assert_eq!(
        f.store.views().get(f.source).unwrap().selection.selections[0].head,
        1
    );
}

#[test]
fn 참조_순환의_본문과_preview_포커스는_새_widget에서도_인계한다() {
    use taide_native_ui::editor_locations::Focus;
    for preview in [false, true] {
        let mut f = Fixture::new(false);
        f.frame(Vec::new(), None);
        let original = if preview {
            f.provider.focus.unwrap()
        } else {
            f.body.unwrap()
        };
        f.frame(vec![key(Key::F4, Modifiers::NONE)], Some(original));
        assert_eq!(
            f.provider.widget.as_ref().unwrap().focus.unwrap().1,
            if preview { Focus::Preview } else { Focus::Body }
        );
        f.provider.widget.as_mut().unwrap().token = "transferred".into();
        f.frame(Vec::new(), None);
        assert_eq!(f.context.memory(|memory| memory.focused()), Some(original));
        let tree = f.body.unwrap().with("location-tree");
        f.frame(Vec::new(), Some(tree));
        f.frame(Vec::new(), None);
        assert_eq!(f.context.memory(|memory| memory.focused()), Some(tree));
    }
}

#[test]
fn 트리_연속_키와_옆_열기_및_escape_뒤_본문_입력은_순서를_보존한다() {
    let mut f = Fixture::new(false);
    f.frame(Vec::new(), None);
    f.frame(
        vec![
            key(Key::ArrowDown, Modifiers::NONE),
            key(Key::Enter, Modifiers::COMMAND),
        ],
        None,
    );
    assert_eq!(f.provider.opened, vec![(1, true)]);
    f.frame(
        vec![
            key(Key::ArrowUp, Modifiers::NONE),
            key(Key::ArrowDown, Modifiers::NONE),
            Event::Text("ignored".into()),
        ],
        None,
    );
    assert_eq!(f.provider.widget.as_ref().unwrap().selected, Some(1));
    assert_eq!(f.text(f.source), "source\nsecond\nthird\nend");
    f.frame(
        vec![key(Key::Escape, Modifiers::NONE), Event::Text("x".into())],
        None,
    );
    assert!(f.provider.widget.is_none());
    assert_eq!(f.text(f.source), "xsource\nsecond\nthird\nend");
}

#[test]
fn 참조_트리의_접힌_파일은_화살표로_숨은_자식_대신_다음_파일을_선택한다() {
    let mut f = Fixture::new(false);
    let targets = f.provider.widget.as_ref().unwrap().model.targets().to_vec();
    let targets = ["file:///synthetic/a.rs", "file:///synthetic/b.rs"]
        .into_iter()
        .flat_map(|uri| {
            targets.iter().cloned().map(move |mut target| {
                target.uri = uri.parse().unwrap();
                target
            })
        })
        .collect();
    f.provider.widget.as_mut().unwrap().model = Arc::new(Locations::new(targets));
    f.frame(Vec::new(), None);
    f.frame(
        vec![
            key(Key::ArrowLeft, Modifiers::NONE),
            key(Key::ArrowLeft, Modifiers::NONE),
        ],
        None,
    );
    f.frame(vec![key(Key::ArrowDown, Modifiers::NONE)], None);
    assert_eq!(f.provider.widget.as_ref().unwrap().selected, Some(0));
    f.frame(vec![key(Key::ArrowRight, Modifiers::NONE)], None);
    assert_eq!(f.provider.widget.as_ref().unwrap().selected, Some(2));
    f.frame(vec![key(Key::ArrowDown, Modifiers::NONE)], None);
    assert_eq!(f.provider.widget.as_ref().unwrap().selected, Some(3));
    f.frame(vec![key(Key::ArrowDown, Modifiers::NONE)], None);
    assert_eq!(f.provider.widget.as_ref().unwrap().selected, Some(3));
    f.frame(vec![key(Key::F4, Modifiers::NONE)], None);
    assert_eq!(f.provider.widget.as_ref().unwrap().selected, Some(0));
}

#[test]
fn 미리보기_편집은_읽기전용_원본문과_다른_문서를_공유하고_본문으로_새지_않는다() {
    let mut f = Fixture::new(true);
    f.frame(Vec::new(), None);
    let preview = f.provider.focus.unwrap();
    f.frame(vec![Event::Text("x".into())], Some(preview));
    assert_eq!(f.text(f.provider.view), "xdef\n  \u{1f600}target\nend");
    assert_eq!(f.text(f.source), "source\nsecond\nthird\nend");
    assert!(
        f.store
            .documents()
            .snapshot(f.provider.document)
            .unwrap()
            .dirty
    );
    f.frame(vec![key(Key::F12, Modifiers::SHIFT)], Some(preview));
    assert_eq!(f.provider.widget.as_ref().unwrap().selected, Some(1));
}

#[test]
fn 빈_참조는_no_results를_표시하고_외부_본문_키는_위젯에서_소비하지_않는다() {
    let mut f = Fixture::new(false);
    f.provider.widget.as_mut().unwrap().model = Arc::new(Locations::default());
    f.provider.widget.as_mut().unwrap().selected = None;
    let (_, text) = f.frame(Vec::new(), None);
    assert!(text.iter().any(|text| text == "No results"));
    let body = f.body.unwrap();
    f.frame(vec![Event::Text("x".into())], Some(body));
    assert_eq!(f.text(f.source), "xsource\nsecond\nthird\nend");
}

fn pointer(point: egui::Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    }
}

#[test]
fn 정의_클릭은_플랫폼_보조키와_눌렀던_본문_줄을_확인한다() {
    for os in [
        egui::os::OperatingSystem::Mac,
        egui::os::OperatingSystem::Windows,
        egui::os::OperatingSystem::Nix,
    ] {
        for side in [false, true] {
            let mut f = Fixture::new(true);
            f.context.set_os(os);
            f.provider.widget = None;
            f.provider.available = true;
            let (geometry, _) = f.frame(Vec::new(), None);
            let point = geometry.caret_rect(2).unwrap().center();
            let modifiers = Modifiers {
                ctrl: !os.is_mac(),
                mac_cmd: os.is_mac(),
                command: true,
                alt: side,
                ..Modifiers::NONE
            };
            f.frame_modifiers(
                vec![Event::PointerMoved(point), pointer(point, true, modifiers)],
                None,
                modifiers,
            );
            assert!(
                !f.provider
                    .requested
                    .iter()
                    .any(|(_, mode)| *mode != Mode::Hover)
            );
            f.frame_modifiers(vec![pointer(point, false, modifiers)], None, modifiers);
            assert_eq!(
                f.provider
                    .requested
                    .iter()
                    .filter(|(_, mode)| *mode != Mode::Hover)
                    .copied()
                    .collect::<Vec<_>>(),
                vec![(2, if side { Mode::Aside } else { Mode::GoTo })]
            );
            assert_eq!(f.text(f.source), "source\nsecond\nthird\nend");
        }
    }
}

#[test]
fn 정의_클릭은_나중에_누른_보조키_다른줄_여백_미지원과_스크롤_변경을_거절한다() {
    for invalidation in [
        "late-modifier",
        "other-line",
        "margin",
        "unsupported",
        "scroll",
        "edited",
        "selection",
        "wrap",
        "drag",
    ] {
        let mut f = Fixture::new(false);
        f.context.set_os(egui::os::OperatingSystem::Mac);
        f.provider.widget = None;
        f.provider.available = invalidation != "unsupported";
        let (geometry, _) = f.frame(Vec::new(), None);
        let point = if invalidation == "margin" {
            pos2(
                geometry.content_rect.right() - 1.0,
                geometry.caret_rect(2).unwrap().center().y,
            )
        } else {
            geometry.caret_rect(2).unwrap().center()
        };
        let modifiers = Modifiers::MAC_CMD | Modifiers::COMMAND;
        let down = if invalidation == "late-modifier" {
            Modifiers::NONE
        } else {
            modifiers
        };
        f.frame_modifiers(
            vec![Event::PointerMoved(point), pointer(point, true, down)],
            None,
            down,
        );
        let up = if invalidation == "other-line" {
            geometry.caret_rect("source\nse".len()).unwrap().center()
        } else if invalidation == "drag" {
            geometry.caret_rect(4).unwrap().center()
        } else {
            point
        };
        if invalidation == "scroll" {
            let current = f.store.views().get(f.source).unwrap().clone();
            f.store
                .set_view_state(
                    f.source,
                    current.selection,
                    taide_native_editor::view::ScrollPosition { x: 1.0, y: 0.0 },
                    current.folds,
                )
                .unwrap();
        }
        if invalidation == "selection" {
            let current = f.store.views().get(f.source).unwrap().clone();
            f.store
                .set_view_state(
                    f.source,
                    taide_native_editor::view::SelectionSet {
                        primary: 0,
                        selections: vec![taide_native_editor::view::Selection {
                            anchor: 1,
                            head: 1,
                        }],
                    },
                    current.scroll,
                    current.folds,
                )
                .unwrap();
        }
        if invalidation == "wrap" {
            f.presentation.options.word_wrap = true;
        }
        if invalidation == "edited" {
            let document = f.store.views().get(f.source).unwrap().document;
            f.store
                .apply(
                    document,
                    taide_native_editor::store::Transaction {
                        revision: 0,
                        edits: vec![taide_native_editor::document::Edit {
                            bytes: 0..0,
                            text: "x".into(),
                        }],
                        group: taide_native_editor::document::UndoGroup(0),
                        origin: Some(f.source),
                        selection_after: None,
                    },
                )
                .unwrap();
        }
        f.frame_modifiers(
            vec![Event::PointerMoved(up), pointer(up, false, modifiers)],
            None,
            modifiers,
        );
        assert!(
            !f.provider
                .requested
                .iter()
                .any(|(_, mode)| *mode != Mode::Hover),
            "{invalidation}: {:?}",
            f.provider.requested
        );
        if invalidation == "unsupported" {
            assert!(f.provider.requested.is_empty());
        }
    }
}

fn long_source(f: &mut Fixture) {
    const SOURCE_LINES: usize = 100;
    let document = f.store.views().get(f.source).unwrap().document;
    let snapshot = f.store.documents().snapshot(document).unwrap();
    let text = (0..SOURCE_LINES)
        .map(|line| format!("line{line:03}\n"))
        .collect();
    f.store
        .apply(
            document,
            taide_native_editor::store::Transaction {
                revision: snapshot.revision,
                edits: vec![taide_native_editor::document::Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text,
                }],
                group: taide_native_editor::document::UndoGroup(0),
                origin: Some(f.source),
                selection_after: Some(taide_native_editor::view::SelectionSet {
                    primary: 0,
                    selections: vec![taide_native_editor::view::Selection { anchor: 0, head: 0 }],
                }),
            },
        )
        .unwrap();
}

#[test]
fn 이동_reveal은_화면밖_줄을_원본_상단_간격으로_드러내고_이미_보이는_줄은_보존한다() {
    const TARGET_LINE: usize = 60;
    const TOP_LINES: f32 = 5.0;
    const TOP_RATIO: f32 = 0.2;
    let mut f = Fixture::new(false);
    f.provider.widget = None;
    long_source(&mut f);
    f.frame(Vec::new(), None);
    let current = f.store.views().get(f.source).unwrap().clone();
    let document = f.store.documents().snapshot(current.document).unwrap();
    let byte = document.rope.line_to_byte(TARGET_LINE);
    f.store
        .set_view_state(
            f.source,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: byte,
                    head: byte,
                }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    f.store
        .request_selection_reveal_near_top(f.source, byte..byte)
        .unwrap();
    let (geometry, _) = f.frame(Vec::new(), None);
    let gap = (LINE_HEIGHT * TOP_LINES).max(geometry.rect.height() * TOP_RATIO);
    assert!((geometry.caret_rect(byte).unwrap().top() - geometry.rect.top() - gap).abs() < 1.0);
    let scroll = geometry.scroll;
    f.store
        .request_selection_reveal_near_top(f.source, byte..byte)
        .unwrap();
    assert_eq!(f.frame(Vec::new(), None).0.scroll, scroll);
}

#[test]
fn 큰_참조_트리는_보이는_행만_읽고_트리_휠을_원본문에_전달하지_않는다() {
    const REFERENCES: u32 = 1000;
    const WHEEL_DELTA: f32 = -46.0;
    const READ_LIMIT: usize = 24;
    let mut f = Fixture::new(false);
    long_source(&mut f);
    let targets = (0..REFERENCES)
        .map(|index| {
            let range = Range::new(Position::new(index, 0), Position::new(index, 1));
            Target {
                uri: "file:///synthetic/target.rs".parse().unwrap(),
                range,
                selection: range,
                origin: None,
            }
        })
        .collect();
    f.provider.widget.as_mut().unwrap().model = Arc::new(Locations::new(targets));
    let (geometry, _) = f.frame(Vec::new(), None);
    f.provider.reads.set(0);
    let point = pos2(
        geometry.rect.right() - LINE_HEIGHT,
        geometry.rect.top() + LINE_HEIGHT * 4.0,
    );
    let (after, _) = f.frame(
        vec![
            Event::PointerMoved(point),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, WHEEL_DELTA),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            },
        ],
        None,
    );
    assert_eq!(after.scroll, geometry.scroll);
    assert!(
        f.provider.reads.get() <= READ_LIMIT,
        "{} row reads",
        f.provider.reads.get()
    );
}

#[test]
fn 닫기_버튼의_키_동작은_후속_본문_입력에_포커스를_돌려준다() {
    for key in [Key::Enter, Key::Space] {
        let mut f = Fixture::new(false);
        f.frame(Vec::new(), None);
        let close = f.body.unwrap().with("location-close");
        f.frame(
            vec![
                Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                },
                Event::Text("x".into()),
            ],
            Some(close),
        );
        assert!(f.provider.widget.is_none());
        assert_eq!(f.text(f.source), "xsource\nsecond\nthird\nend");
    }
}
