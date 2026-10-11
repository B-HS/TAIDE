#![cfg(all(feature = "native-host", feature = "inspection"))]

use std::sync::Arc;

use egui::{Color32, Context, Event, FontId, Id, Key, Modifiers, RawInput, Rect, Ui};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::DocumentId;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_code_actions::{
    self, Action, Bulb, Colors, Command, Group, Labels, Provider, Session,
};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorOutput, EditorPresentation, EditorRequest, NativeEditor,
};

const TEXT: &str = "first\nmethod(x)\nend";
const SCREEN: egui::Vec2 = egui::vec2(800.0, 600.0);
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const BYTE_LIMIT: usize = 4096;
const TOKEN: u128 = 23;
const FRAME_STEP: f64 = 0.1;

#[derive(Default)]
struct Actions {
    session: Option<Session>,
    message: Option<editor_code_actions::Message>,
    selected: Vec<(usize, bool)>,
    opened: usize,
    closed: usize,
}

impl Provider for Actions {
    fn message(&self, _: &EditorStore, _: ViewId) -> Option<editor_code_actions::Message> {
        self.message.clone()
    }
    fn current(&self, _: &EditorStore, _: ViewId) -> Option<Session> {
        self.session.clone()
    }
    fn open(&mut self, _: &EditorStore, _: ViewId, token: u128) {
        assert_eq!(token, TOKEN);
        self.opened += 1;
        self.session.as_mut().unwrap().menu_open = true;
    }
    fn select(&mut self, _: &EditorStore, _: ViewId, token: u128, index: usize, preview: bool) {
        assert_eq!(token, TOKEN);
        assert!(
            self.session.as_ref().unwrap().actions[index]
                .disabled
                .is_none()
        );
        self.selected.push((index, preview));
        self.session = None;
    }
    fn close(&mut self, _: ViewId, token: u128) {
        assert_eq!(token, TOKEN);
        self.closed += 1;
        self.session = None;
        self.message = None;
    }
}

fn action(title: &str, kind: Option<&str>, preferred: bool, disabled: Option<&str>) -> Action {
    Action {
        title: title.into(),
        kind: kind.map(str::to_owned),
        preferred,
        disabled: disabled.map(str::to_owned),
    }
}

fn candidates() -> Arc<[Action]> {
    vec![
        action("Custom 名", None, false, None),
        action(
            "Unavailable",
            Some("quickfix"),
            true,
            Some("현재 선택에서는 적용할 수 없습니다"),
        ),
        action("Imports", Some("source.organizeImports"), false, None),
        action(
            "Extract method",
            Some("refactor.extract.function"),
            false,
            None,
        ),
        action("Preferred fix", Some("quickfix"), true, None),
    ]
    .into()
}

struct Fixture {
    context: Context,
    store: EditorStore,
    document: DocumentId,
    view: ViewId,
    editor: NativeEditor,
    actions: Actions,
    time: f64,
    is_enabled: bool,
    window_focused: bool,
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
                    window: "code-actions-fixture".into(),
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
            actions: Actions::default(),
            time: 0.0,
            is_enabled: true,
            window_focused: true,
        }
    }

    fn open(&mut self) -> EditorOutput {
        self.frame(Vec::new(), TEXT);
        self.actions.session = Some(Session {
            token: TOKEN,
            anchor: 6,
            menu_open: true,
            include_disabled: true,
            show_headers: true,
            actions: candidates(),
            bulb: None,
            labels: Labels::default(),
        });
        self.frame(Vec::new(), TEXT);
        self.frame(Vec::new(), TEXT)
    }

    fn frame(&mut self, events: Vec<Event>, expected: &str) -> EditorOutput {
        let before = self.store.documents().snapshot(self.document).unwrap();
        let unchanged = before.rope.to_string() == expected;
        self.time += FRAME_STEP;
        let mut rendered = None;
        let presentation = EditorPresentation::default();
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(self.time),
                events,
                focused: self.window_focused,
                ..Default::default()
            },
            |ui| {
                if !self.is_enabled {
                    ui.disable();
                }
                rendered = Some(
                    self.editor
                        .show_request_with_code_actions(
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
                            Some((
                                &mut self.actions,
                                Colors {
                                    background: Color32::DARK_GRAY,
                                    foreground: Color32::WHITE,
                                    disabled: Color32::GRAY,
                                    selected_background: Color32::BLUE,
                                    selected_foreground: Color32::WHITE,
                                    hover_background: Color32::LIGHT_BLUE,
                                    border: Color32::GRAY,
                                    shadow: Color32::BLACK,
                                    lightbulb: Color32::YELLOW,
                                    lightbulb_auto_fix: Color32::LIGHT_YELLOW,
                                    message_background: Color32::DARK_GRAY,
                                    message_foreground: Color32::WHITE,
                                    message_border: Color32::BLUE,
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
        let snapshot = self.store.documents().snapshot(self.document).unwrap();
        assert_eq!(snapshot.rope.to_string(), expected);
        if unchanged {
            assert_eq!(snapshot.revision, before.revision);
            assert_eq!(snapshot.dirty, before.dirty);
            assert!(rendered.formatting_inputs.is_empty());
        }
        rendered
    }

    fn click(&mut self, id: Id) {
        let position = self
            .context
            .read_response(id)
            .expect("visible code action control")
            .rect
            .center();
        self.frame(
            vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            TEXT,
        );
        let pressed = self.context.read_response(id).unwrap();
        assert!(
            pressed.is_pointer_button_down_on(),
            "control did not receive press: {pressed:?}; focus: {:?}; open/close: {}/{}",
            self.context.memory(|memory| memory.focused()),
            self.actions.opened,
            self.actions.closed,
        );
        self.frame(
            vec![Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
            TEXT,
        );
        let released = self.context.read_response(id).unwrap();
        assert!(
            released.clicked(),
            "control did not receive click: {released:?}; focus: {:?}; session: {:?}; open/close: {}/{}",
            self.context.memory(|memory| memory.focused()),
            self.actions
                .session
                .as_ref()
                .map(|session| session.menu_open),
            self.actions.opened,
            self.actions.closed,
        );
    }
}

fn key_with(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn 커서_안내는_메뉴_포커스를_빼앗지_않고_escape와_본문_입력을_보존한다() {
    let mut fixture = Fixture::new();
    let first = fixture.frame(Vec::new(), TEXT);
    fixture
        .context
        .memory_mut(|memory| memory.request_focus(first.response.id));
    fixture.actions.message = Some(editor_code_actions::Message {
        token: TOKEN,
        anchor: 6,
        text: "No code actions available".into(),
    });
    fixture.frame(Vec::new(), TEXT);
    let rendered = fixture.frame(Vec::new(), TEXT);
    let response = fixture
        .context
        .read_response(editor_code_actions::message_id(rendered.response.id, TOKEN))
        .unwrap();
    assert!(SCREEN.x >= response.rect.right());
    assert!(SCREEN.y >= response.rect.bottom());
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(rendered.response.id))
    );
    assert!(
        fixture
            .context
            .read_response(editor_code_actions::focus_id(rendered.response.id, TOKEN))
            .is_none()
    );
    fixture.frame(vec![key_with(Key::Escape, Modifiers::NONE)], TEXT);
    assert!(fixture.actions.message.is_none());
    assert_eq!(fixture.actions.closed, 1);
    assert!(fixture.actions.selected.is_empty());
    fixture.frame(vec![Event::Text("x".into())], &format!("x{TEXT}"));
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
}

#[test]
fn 커서_안내는_비활성화와_창_본문_포커스_상실로_회수한다() {
    for state in [0, 1, 2] {
        let mut fixture = Fixture::new();
        let first = fixture.frame(Vec::new(), TEXT);
        fixture
            .context
            .memory_mut(|memory| memory.request_focus(first.response.id));
        fixture.actions.message = Some(editor_code_actions::Message {
            token: TOKEN,
            anchor: 6,
            text: "Unavailable here".into(),
        });
        fixture.frame(Vec::new(), TEXT);
        match state {
            0 => fixture.is_enabled = false,
            1 => fixture.window_focused = false,
            _ => fixture
                .context
                .memory_mut(|memory| memory.surrender_focus(first.response.id)),
        }
        fixture.frame(Vec::new(), TEXT);
        assert!(fixture.actions.message.is_none());
        assert_eq!(fixture.actions.closed, 1);
        assert!(fixture.actions.selected.is_empty());
    }
}

fn key(key: Key) -> Event {
    key_with(key, Modifiers::NONE)
}

#[test]
fn 원본_여섯_명령은_source_제외_계층_preferred와_단일_자동_적용을_구분한다() {
    let actions = candidates();
    for (id, command) in [
        ("quickFix", Command::QuickFix),
        ("refactor", Command::Refactor),
        ("sourceAction", Command::Source),
        ("organizeImports", Command::OrganizeImports),
        ("fixAll", Command::FixAll),
        ("autoFix", Command::AutoFix),
    ] {
        assert_eq!(
            Command::from_action(&format!("monaco.editor.action.{id}")),
            Some(command)
        );
        assert_eq!(
            Command::from_action(&format!("editor.action.{id}")),
            Some(command)
        );
    }
    assert_eq!(Command::from_action("editor.action.rename"), None);
    assert!(Command::QuickFix.accepts(&actions[0]));
    assert!(!Command::QuickFix.accepts(&actions[2]));
    assert!(Command::QuickFix.accepts(&actions[3]));
    assert!(Command::Refactor.accepts(&actions[3]));
    assert!(!Command::Refactor.accepts(&actions[0]));
    assert!(Command::Source.accepts(&actions[2]));
    assert!(Command::OrganizeImports.accepts(&actions[2]));
    assert!(!Command::FixAll.accepts(&actions[2]));
    assert!(Command::AutoFix.accepts(&actions[4]));
    assert!(!Command::AutoFix.accepts(&action("not preferred", Some("quickfix"), false, None)));
    assert!(!Command::Refactor.accepts(&action("wrong prefix", Some("refactoring"), false, None)));
    assert!(Command::Refactor.intersects("refactor.extract"));
    assert!(Command::OrganizeImports.intersects("source"));
    assert!(!Command::QuickFix.intersects("source.fixAll"));
    assert_eq!(
        Group::of(&action("unknown", Some("refactor.extractor"), false, None)),
        Group::Other
    );
    for command in [Command::QuickFix, Command::Refactor, Command::Source] {
        assert!(!command.auto_apply_single());
    }
    for command in [Command::OrganizeImports, Command::FixAll, Command::AutoFix] {
        assert!(command.auto_apply_single());
    }
}

#[test]
fn 실제_목록은_그룹_순서와_비활성_건너뛰기_순환_선택을_보존한다() {
    let mut fixture = Fixture::new();
    let output = fixture.open();
    let editor = output.response.id;
    let menu = editor_code_actions::focus_id(editor, TOKEN);
    assert!(output.focus_ids.contains(&menu));
    assert!(fixture.context.memory(|memory| memory.has_focus(menu)));
    let top = |index| {
        fixture
            .context
            .read_response(editor_code_actions::action_id(editor, TOKEN, index))
            .unwrap()
            .rect
            .top()
    };
    assert!(top(4) < top(3));
    assert!(top(3) < top(2));
    assert!(top(2) < top(0));
    assert!(
        fixture
            .context
            .read_response(editor_code_actions::action_id(editor, TOKEN, 1))
            .is_none()
    );
    fixture.frame(
        vec![
            key(Key::ArrowUp),
            key(Key::ArrowDown),
            key(Key::ArrowDown),
            key(Key::Enter),
        ],
        TEXT,
    );
    assert_eq!(fixture.actions.selected, [(3, false)]);
    assert_eq!(fixture.actions.closed, 0);
    fixture.frame(Vec::new(), TEXT);
    assert_eq!(fixture.actions.selected.len(), 1);
}

#[test]
fn 움직이지_않는_포인터는_방향키로_바꾼_선택을_되돌리지_않는다() {
    let mut fixture = Fixture::new();
    let editor = fixture.open().response.id;
    fixture.frame(Vec::new(), TEXT);
    let position = fixture
        .context
        .read_response(editor_code_actions::action_id(editor, TOKEN, 4))
        .unwrap()
        .rect
        .center();
    fixture.frame(vec![Event::PointerMoved(position)], TEXT);
    fixture.frame(vec![key(Key::ArrowDown)], TEXT);
    fixture.frame(vec![key(Key::Enter)], TEXT);
    assert_eq!(fixture.actions.selected, [(3, false)]);
}

#[test]
fn enter와_escape_뒤_같은_프레임의_본문_입력은_유실되지_않는다() {
    for accept in [true, false] {
        let mut fixture = Fixture::new();
        fixture.open();
        let output = fixture.frame(
            vec![
                key(if accept { Key::Enter } else { Key::Escape }),
                Event::Text("名".into()),
            ],
            &format!("名{TEXT}"),
        );
        assert_eq!(fixture.actions.selected.len(), usize::from(accept));
        assert_eq!(fixture.actions.closed, usize::from(!accept));
        assert!(
            fixture
                .context
                .memory(|memory| memory.has_focus(output.response.id))
        );
        assert!(
            fixture
                .store
                .documents()
                .snapshot(fixture.document)
                .unwrap()
                .dirty
        );
        assert!(fixture.store.undo(fixture.document).unwrap());
        fixture.frame(Vec::new(), TEXT);
    }
}

#[test]
fn ime_조합_중_선택을_보류하고_확정_뒤_한_번만_수락한다() {
    let mut fixture = Fixture::new();
    fixture.open();
    fixture.frame(
        vec![
            Event::Ime(egui::ImeEvent::Preedit {
                text: "수정".into(),
                active_range_chars: None,
            }),
            key(Key::Enter),
        ],
        TEXT,
    );
    assert!(fixture.actions.selected.is_empty());
    fixture.frame(
        vec![
            Event::Ime(egui::ImeEvent::Commit("수정".into())),
            key(Key::Enter),
        ],
        TEXT,
    );
    assert_eq!(fixture.actions.selected, [(4, false)]);
}

#[test]
fn 기본_수락과_미리보기_키는_선택_의도를_구분해_공급자에_전달한다() {
    for (pressed, preview) in [(Key::Period, false), (Key::Enter, true)] {
        let mut fixture = Fixture::new();
        fixture.open();
        fixture.frame(
            vec![key_with(
                pressed,
                Modifiers {
                    command: true,
                    mac_cmd: true,
                    ..Modifiers::NONE
                },
            )],
            TEXT,
        );
        assert_eq!(fixture.actions.selected, [(4, preview)]);
    }
}

#[test]
fn 비활성_표시를_전환해도_키_포커스와_선택_거절을_유지한다() {
    let mut fixture = Fixture::new();
    let editor = fixture.open().response.id;
    fixture.click(editor_code_actions::disabled_toggle_id(editor, TOKEN));
    fixture.frame(Vec::new(), TEXT);
    let disabled = editor_code_actions::action_id(editor, TOKEN, 1);
    assert!(fixture.context.read_response(disabled).is_some());
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(editor_code_actions::focus_id(editor, TOKEN)))
    );
    fixture.click(disabled);
    assert!(fixture.actions.selected.is_empty());
    assert!(fixture.actions.session.is_some());
    fixture.frame(vec![key(Key::Enter)], TEXT);
    assert_eq!(fixture.actions.selected, [(4, false)]);
}

#[test]
fn 비활성화와_창_포커스_상실은_목록을_회수한다() {
    for disable in [true, false] {
        let mut fixture = Fixture::new();
        let editor = fixture.open().response.id;
        fixture.is_enabled = !disable;
        fixture.window_focused = disable;
        fixture.frame(Vec::new(), TEXT);
        assert_eq!(fixture.actions.closed, 1);
        assert!(fixture.actions.selected.is_empty());
        assert!(
            !fixture
                .context
                .memory(|memory| memory.has_focus(editor_code_actions::focus_id(editor, TOKEN)))
        );
    }
}

#[test]
fn 현재_전구를_클릭하면_본문_편집_없이_같은_공급자_목록을_연다() {
    let mut fixture = Fixture::new();
    let editor = fixture.open().response.id;
    let selection = fixture
        .store
        .views()
        .get(fixture.view)
        .unwrap()
        .selection
        .clone();
    fixture.frame(vec![key(Key::Escape)], TEXT);
    fixture.actions.session = Some(Session {
        token: TOKEN,
        anchor: 6,
        menu_open: false,
        include_disabled: false,
        show_headers: true,
        actions: candidates(),
        bulb: Some(Bulb {
            anchor: 6,
            gutter: true,
        }),
        labels: Labels::default(),
    });
    fixture.frame(Vec::new(), TEXT);
    fixture.click(editor_code_actions::bulb_id(editor, TOKEN));
    assert_eq!(fixture.actions.opened, 1);
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().selection,
        selection
    );
    fixture.frame(Vec::new(), TEXT);
    assert!(
        fixture
            .context
            .memory(|memory| memory.has_focus(editor_code_actions::focus_id(editor, TOKEN)))
    );
    fixture.frame(vec![key(Key::Enter)], TEXT);
    assert_eq!(fixture.actions.selected, [(4, false)]);
}

#[test]
fn 마우스로_선택한_항목은_한_번만_수락하고_본문_선택을_보존한다() {
    let mut fixture = Fixture::new();
    let editor = fixture.open().response.id;
    let selection = fixture
        .store
        .views()
        .get(fixture.view)
        .unwrap()
        .selection
        .clone();
    fixture.click(editor_code_actions::action_id(editor, TOKEN, 3));
    assert_eq!(fixture.actions.selected, [(3, false)]);
    assert_eq!(
        fixture.store.views().get(fixture.view).unwrap().selection,
        selection
    );
    fixture.frame(Vec::new(), TEXT);
    assert_eq!(fixture.actions.selected.len(), 1);
    assert!(fixture.context.memory(|memory| memory.has_focus(editor)));
}

#[test]
fn 모든_항목이_비활성이면_사유를_표시하되_수락하지_않고_취소한다() {
    let mut fixture = Fixture::new();
    fixture.frame(Vec::new(), TEXT);
    fixture.actions.session = Some(Session {
        token: TOKEN,
        anchor: 6,
        menu_open: true,
        include_disabled: true,
        show_headers: true,
        actions: vec![action("Unavailable", Some("quickfix"), false, Some(""))].into(),
        bulb: None,
        labels: Labels::default(),
    });
    let editor = fixture.frame(Vec::new(), TEXT).response.id;
    fixture.frame(Vec::new(), TEXT);
    assert!(
        fixture
            .context
            .read_response(editor_code_actions::action_id(editor, TOKEN, 0))
            .is_some()
    );
    assert!(
        fixture
            .context
            .read_response(editor_code_actions::disabled_toggle_id(editor, TOKEN))
            .is_none()
    );
    fixture.frame(
        vec![key(Key::ArrowDown), key(Key::Enter), key(Key::Space)],
        TEXT,
    );
    assert!(fixture.actions.selected.is_empty());
    assert!(fixture.actions.session.is_some());
    fixture.frame(vec![key(Key::Escape)], TEXT);
    assert_eq!(fixture.actions.closed, 1);
    assert!(fixture.context.memory(|memory| memory.has_focus(editor)));
}
