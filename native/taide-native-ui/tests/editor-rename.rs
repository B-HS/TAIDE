#![cfg(all(feature = "native-host", feature = "inspection"))]

use egui::{Color32, Context, Event, FontId, Key, Modifiers, RawInput, Rect, Ui};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_rename::{Colors, Provider, Session};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorOutput, EditorPresentation, EditorRequest, NativeEditor,
};

const TEXT: &str = "first\nmethod(x)\nend";
const SCREEN: egui::Vec2 = egui::vec2(800.0, 600.0);
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const BYTE_LIMIT: usize = 4096;
const TOKEN: u128 = 17;
const FIRST_TIME: f64 = 0.0;
const FRAME_STEP: f64 = 0.1;

#[derive(Default)]
struct Rename {
    session: Option<Session>,
    names: Vec<String>,
    cancelled: usize,
}

impl Provider for Rename {
    fn current(&self, _: &EditorStore, _: ViewId) -> Option<Session> {
        self.session.clone()
    }
    fn accept(&mut self, _: &EditorStore, _: ViewId, token: u128, name: String) {
        assert_eq!(token, TOKEN);
        self.names.push(name);
        self.session = None;
    }
    fn cancel(&mut self, _: ViewId, token: u128) {
        assert_eq!(token, TOKEN);
        self.cancelled += 1;
        self.session = None;
    }
}

struct Fixture {
    context: Context,
    store: EditorStore,
    document: DocumentId,
    view: ViewId,
    editor: NativeEditor,
    rename: Rename,
    time: f64,
    is_enabled: bool,
}

impl Fixture {
    fn new() -> Self {
        let context = Context::default();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 4,
            max_views: 4,
            max_undo_groups: 4,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), TEXT.into(), "rust".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "rename-fixture".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        let editor = NativeEditor {
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
        };
        Self {
            context,
            store,
            document,
            view,
            editor,
            rename: Rename::default(),
            time: FIRST_TIME,
            is_enabled: true,
        }
    }

    fn open(&mut self, name: &str, selection: std::ops::Range<usize>) -> EditorOutput {
        self.frame(Vec::new());
        self.rename.session = Some(Session {
            token: TOKEN,
            range: 6..12,
            name: name.into(),
            selection,
            columns: "method".len(),
        });
        self.frame(Vec::new())
    }

    fn frame(&mut self, events: Vec<Event>) -> EditorOutput {
        self.frame_with_text(events, TEXT)
    }

    fn frame_with_text(&mut self, events: Vec<Event>, expected: &str) -> EditorOutput {
        self.time += FRAME_STEP;
        let mut rendered = None;
        let presentation = EditorPresentation::default();
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                if !self.is_enabled {
                    ui.disable();
                }
                rendered = Some(
                    self.editor
                        .show_request_with_rename(
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
                            Some((
                                &mut self.rename,
                                Colors {
                                    background: Color32::DARK_GRAY,
                                    foreground: Color32::WHITE,
                                    border: Color32::GRAY,
                                    shadow: Color32::BLACK,
                                },
                            )),
                        )
                        .unwrap(),
                );
            },
        );
        output.textures_delta.clear();
        let rendered = rendered.unwrap();
        assert!(rendered.errors.is_empty());
        assert_eq!(
            self.store
                .documents()
                .snapshot(self.document)
                .unwrap()
                .rope
                .to_string(),
            expected
        );
        if expected == TEXT {
            assert_eq!(
                self.store
                    .documents()
                    .snapshot(self.document)
                    .unwrap()
                    .revision,
                0
            );
            assert!(rendered.formatting_inputs.is_empty());
        }
        rendered
    }
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

#[test]
fn 이름_입력과_enter는_본문과_포맷에_누출되지_않고_한_번만_적용한다() {
    let mut fixture = Fixture::new();
    let output = fixture.open("method", 0..6);
    let input = taide_native_ui::editor_rename::focus_id(output.response.id, TOKEN);
    assert!(output.focus_ids.contains(&input));
    assert!(fixture.context.memory(|memory| memory.has_focus(input)));
    fixture.frame(vec![Event::Text("renamed".into()), key(Key::Enter)]);
    assert_eq!(fixture.rename.names, ["renamed"]);
    assert_eq!(fixture.rename.cancelled, 0);
    fixture.frame(Vec::new());
    assert_eq!(fixture.rename.names.len(), 1);
}

#[test]
fn 초기_부분_선택은_유니코드_문자_경계를_보존한다() {
    let mut fixture = Fixture::new();
    fixture.open("\u{1f600}method", 1..4);
    fixture.frame(vec![Event::Text("new".into()), key(Key::Enter)]);
    assert_eq!(fixture.rename.names, ["\u{1f600}newhod"]);
}

#[test]
fn 같은_프레임의_enter_뒤_글자는_새_이름_대신_본문에_전달한다() {
    let mut fixture = Fixture::new();
    fixture.open("method", 0..6);
    let output = fixture.frame_with_text(
        vec![
            Event::Text("renamed".into()),
            key(Key::Enter),
            Event::Text("post".into()),
        ],
        "postfirst\nmethod(x)\nend",
    );
    assert_eq!(fixture.rename.names, ["renamed"]);
    assert_eq!(fixture.rename.cancelled, 0);
    assert!(output.changed);
    fixture.frame_with_text(Vec::new(), "postfirst\nmethod(x)\nend");
    assert_eq!(fixture.rename.names.len(), 1);
}

#[test]
fn 같은_프레임의_escape_뒤_글자는_취소한_입력_대신_본문에_전달한다() {
    let mut fixture = Fixture::new();
    fixture.open("method", 0..6);
    let output = fixture.frame_with_text(
        vec![
            Event::Text("never".into()),
            key(Key::Escape),
            Event::Text("post".into()),
        ],
        "postfirst\nmethod(x)\nend",
    );
    assert!(fixture.rename.names.is_empty());
    assert_eq!(fixture.rename.cancelled, 1);
    assert!(output.changed);
}

#[test]
fn escape와_다른_위젯_포커스는_편집_없이_취소한다() {
    let mut fixture = Fixture::new();
    fixture.open("method", 0..6);
    fixture.frame(vec![Event::Text("never".into()), key(Key::Escape)]);
    assert_eq!(fixture.rename.cancelled, 1);
    assert!(fixture.rename.names.is_empty());
    fixture.open("method", 0..6);
    fixture
        .context
        .memory_mut(|memory| memory.request_focus(egui::Id::new("outside-rename")));
    fixture.frame(Vec::new());
    assert_eq!(fixture.rename.cancelled, 2);
    let output = fixture.open("method", 0..6);
    let input = taide_native_ui::editor_rename::focus_id(output.response.id, TOKEN);
    fixture.is_enabled = false;
    fixture.frame(Vec::new());
    assert_eq!(fixture.rename.cancelled, 3);
    assert!(!fixture.context.memory(|memory| memory.has_focus(input)));
}

#[test]
fn ime_조합_중_enter는_적용하지_않고_확정_후에만_적용한다() {
    let mut fixture = Fixture::new();
    fixture.open("method", 0..6);
    fixture.frame(vec![
        Event::Ime(egui::ImeEvent::Preedit {
            text: "이름".into(),
            active_range_chars: None,
        }),
        key(Key::Enter),
    ]);
    assert!(fixture.rename.names.is_empty());
    assert_eq!(fixture.rename.cancelled, 0);
    fixture.frame(vec![Event::Ime(egui::ImeEvent::Commit("이름".into()))]);
    fixture.frame(vec![key(Key::Enter)]);
    assert_eq!(fixture.rename.names, ["이름"]);
}
