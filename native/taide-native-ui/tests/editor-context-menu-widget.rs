#![cfg(feature = "native-host")]

use egui::{
    Color32, Event, FontId, FullOutput, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2,
    vec2,
};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Selection, SelectionSet, ViewId, ViewKey};
use taide_native_ui::editor_context_menu::{self, Action, Context, Output, Request};
use taide_native_ui::editor_surface::{
    EditorAppearance, EditorOutput, EditorPresentation, NativeEditor,
};

const TEXT: &str = "ab\u{1f600}cd\nword\nlast";
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const FRAME_STEP: f64 = 0.1;
const BYTE_LIMIT: usize = 4096;
const UNDO_LIMIT: usize = 4;
const SCREEN: egui::Vec2 = vec2(700.0, 450.0);
const POINTER_OFFSET: f32 = 1.0;
const SCROLL_TEXT_REPEATS: usize = 80;

struct Fixture {
    context: egui::Context,
    store: EditorStore,
    view: ViewId,
    editor: NativeEditor,
    time: f64,
    menu: Context,
    enabled: bool,
    excluded: Vec<Rect>,
}

impl Fixture {
    fn new() -> Self {
        Self::with_text(TEXT)
    }

    fn with_text(text: &str) -> Self {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: UNDO_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), text, "rust".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "menu-widget-fixture".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        Self {
            context: egui::Context::default(),
            store,
            view,
            time: 0.0,
            menu: Context::default(),
            enabled: true,
            excluded: Vec::new(),
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
                    line_numbers: true,
                    indent: "    ".into(),
                },
            },
        }
    }

    fn frame(&mut self, events: Vec<Event>, keyboard: bool) -> (EditorOutput, Output, FullOutput) {
        self.time += FRAME_STEP;
        let mut shown = None;
        let mut menu = None;
        let mut invocations = Vec::new();
        let mut opened = false;
        let mut selection_changed = false;
        let context = self.context.clone();
        let mut painted = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                if !self.enabled {
                    ui.disable();
                }
                let output = self
                    .editor
                    .show_presented(
                        ui,
                        &mut self.store,
                        self.view,
                        self.time == FRAME_STEP,
                        |_, _, _| false,
                        |response| response.ctx.keyboard_input_route(response.id),
                        &EditorPresentation::default(),
                    )
                    .unwrap();
                let context = self.menu;
                let menu_output = editor_context_menu::show(
                    ui,
                    &mut self.store,
                    Request {
                        source: self.view,
                        owner: self.view,
                        response: &output.response,
                        geometry: &output.geometry,
                        keyboard_open: keyboard,
                        excluded: &self.excluded,
                    },
                    |store, view| {
                        let current = store.views().get(view).unwrap();
                        let context = Context {
                            has_selection: current
                                .selection
                                .selections
                                .iter()
                                .any(|selection| selection.anchor != selection.head),
                            ..context
                        };
                        editor_context_menu::groups(context, "Peek".into(), |_, label| {
                            (label.into(), Some("Test Key".into()))
                        })
                    },
                )
                .unwrap();
                if let Some(invocation) = &menu_output.invocation {
                    invocations.push(invocation.clone());
                }
                opened |= menu_output.opened;
                selection_changed |= menu_output.selection_changed;
                menu = Some(menu_output);
                shown = Some(output);
            },
        );
        painted.textures_delta.clear();
        assert!(
            invocations.len() <= 1,
            "a frame must emit at most one menu command"
        );
        let mut menu = menu.unwrap();
        menu.invocation = invocations.into_iter().next();
        menu.opened |= opened;
        menu.selection_changed |= selection_changed;
        (shown.unwrap(), menu, painted)
    }

    fn selection(&self) -> SelectionSet {
        self.store.views().get(self.view).unwrap().selection.clone()
    }

    fn select(&mut self, selection: SelectionSet) {
        let view = self.store.views().get(self.view).unwrap();
        self.store
            .set_view_state(
                self.view,
                selection,
                view.scroll.clone(),
                view.folds.clone(),
            )
            .unwrap();
    }

    fn open_at(&mut self, position: Pos2) -> Output {
        let (_, menu, _) = self.frame(
            vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: PointerButton::Secondary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ],
            false,
        );
        menu
    }

    fn text(&self) -> String {
        self.store
            .documents()
            .snapshot(self.store.views().get(self.view).unwrap().document)
            .unwrap()
            .rope
            .to_string()
    }
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn 실제_우클릭_메뉴는_다중_선택과_주_선택_및_키_표시를_보존한다() {
    let mut fixture = Fixture::new();
    let selection = SelectionSet {
        primary: 1,
        selections: vec![
            Selection { anchor: 2, head: 0 },
            Selection {
                anchor: 13,
                head: 9,
            },
        ],
    };
    fixture.select(selection.clone());
    let (shown, _, _) = fixture.frame(Vec::new(), false);
    let point = shown.geometry.caret_rect(11).unwrap().center() + vec2(POINTER_OFFSET, 0.0);
    let opened = fixture.open_at(point);
    assert!(opened.opened);
    assert!(opened.geometry.rect.is_some());
    assert_eq!(fixture.selection(), selection);
    let (_, menu, painted) = fixture.frame(Vec::new(), false);
    assert!(
        menu.geometry
            .items
            .iter()
            .any(|(action, _)| *action == Action::Copy)
    );
    assert!(painted.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Test Key")
    ));
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 본문_밖_선택과_빈_본문_우클릭은_정확한_문자_위치와_eof를_선택한다() {
    let mut fixture = Fixture::new();
    let (shown, _, _) = fixture.frame(Vec::new(), false);
    let point = shown.geometry.caret_rect(6).unwrap().center();
    let opened = fixture.open_at(point);
    assert!(opened.selection_changed);
    assert_eq!(
        fixture.selection().selections,
        [Selection { anchor: 6, head: 6 }]
    );
    fixture.frame(vec![key(Key::Escape)], false);
    fixture.frame(
        vec![Event::PointerButton {
            pos: point,
            button: PointerButton::Secondary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        false,
    );
    let (shown, _, _) = fixture.frame(Vec::new(), false);
    let point = pos2(
        shown.geometry.content_rect.center().x,
        shown.geometry.content_rect.bottom() - LINE_HEIGHT,
    );
    let opened = fixture.open_at(point);
    assert!(opened.opened);
    assert_eq!(
        fixture.selection().selections,
        [Selection {
            anchor: TEXT.len(),
            head: TEXT.len()
        }]
    );
}

#[test]
fn gutter와_명시한_위젯_영역은_메뉴와_선택을_열지_않는다() {
    for excluded in [false, true] {
        let mut fixture = Fixture::new();
        let (shown, _, _) = fixture.frame(Vec::new(), false);
        let before = fixture.selection();
        let point = if excluded {
            shown.geometry.caret_rect(6).unwrap().center()
        } else {
            pos2(
                shown.geometry.gutter_rect.center().x,
                shown.geometry.caret_rect(0).unwrap().center().y,
            )
        };
        if excluded {
            fixture.excluded = vec![Rect::from_center_size(
                point,
                vec2(LINE_HEIGHT, LINE_HEIGHT),
            )];
        }
        let opened = fixture.open_at(point);
        assert!(!opened.opened);
        assert!(opened.geometry.rect.is_none());
        assert_eq!(fixture.selection(), before);
        fixture.frame(vec![Event::Text("x".into())], false);
        assert_eq!(fixture.text(), format!("x{TEXT}"), "excluded={excluded}");
    }
}

#[test]
fn 키보드_메뉴의_같은_프레임_방향과_실행은_본문에_줄바꿈을_넣지_않는다() {
    let mut fixture = Fixture::new();
    fixture.frame(Vec::new(), false);
    let (_, opened, _) = fixture.frame(Vec::new(), true);
    assert!(opened.opened);
    let (_, menu, _) = fixture.frame(
        vec![key(Key::ArrowDown), key(Key::ArrowDown), key(Key::Enter)],
        false,
    );
    assert_eq!(
        menu.invocation.as_ref().map(|invocation| invocation.action),
        Some(Action::Command("editor.action.changeAll"))
    );
    assert_eq!(fixture.text(), TEXT);
    let (_, closed, _) = fixture.frame(Vec::new(), false);
    assert!(closed.geometry.rect.is_none());
}

#[test]
fn peek_하위_메뉴는_오른쪽_끝_실행을_보존한다() {
    let mut fixture = Fixture::new();
    fixture.menu.definition = true;
    fixture.menu.type_definition = true;
    fixture.frame(Vec::new(), false);
    fixture.frame(Vec::new(), true);
    fixture.frame(
        vec![
            key(Key::ArrowDown),
            key(Key::ArrowDown),
            key(Key::ArrowDown),
            key(Key::ArrowDown),
        ],
        false,
    );
    let (_, submenu, _) = fixture.frame(vec![key(Key::ArrowRight)], false);
    assert!(
        submenu
            .geometry
            .items
            .iter()
            .any(|(action, _)| *action == Action::Command("editor.action.peekTypeDefinition"))
    );
    let (_, selected, _) = fixture.frame(vec![key(Key::End), key(Key::Enter)], false);
    assert_eq!(
        selected
            .invocation
            .as_ref()
            .map(|invocation| invocation.action),
        Some(Action::Command("editor.action.peekTypeDefinition"))
    );
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 문자_검색과_반복_문자는_메뉴_항목을_순환하고_escape는_본문_포커스를_돌린다() {
    let mut fixture = Fixture::new();
    fixture.frame(Vec::new(), false);
    fixture.frame(Vec::new(), true);
    fixture.frame(vec![Event::Text("c".into())], false);
    fixture.frame(vec![Event::Text("c".into())], false);
    let (_, selected, _) = fixture.frame(vec![key(Key::Enter)], false);
    assert_eq!(
        selected
            .invocation
            .as_ref()
            .map(|invocation| invocation.action),
        Some(Action::Cut)
    );
    fixture.frame(Vec::new(), true);
    let (shown, menu, _) = fixture.frame(vec![key(Key::Escape)], false);
    assert!(menu.invocation.is_none());
    let (next, closed, _) = fixture.frame(Vec::new(), false);
    assert!(closed.geometry.rect.is_none());
    assert!(shown.response.has_focus() || next.response.has_focus());
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 스크롤과_공급_조건_변경과_비활성은_이전_메뉴를_회수한다() {
    for mutation in ["scroll", "provider", "disabled"] {
        let mut fixture = if mutation == "scroll" {
            Fixture::with_text(&TEXT.repeat(SCROLL_TEXT_REPEATS))
        } else {
            Fixture::new()
        };
        let before = fixture.text();
        fixture.frame(Vec::new(), false);
        fixture.frame(Vec::new(), true);
        match mutation {
            "scroll" => {
                let view = fixture.store.views().get(fixture.view).unwrap();
                let mut scroll = view.scroll.clone();
                scroll.y = LINE_HEIGHT;
                fixture
                    .store
                    .set_view_state(
                        fixture.view,
                        view.selection.clone(),
                        scroll,
                        view.folds.clone(),
                    )
                    .unwrap();
            }
            "provider" => fixture.menu.definition = true,
            "disabled" => fixture.enabled = false,
            _ => unreachable!(),
        }
        let (_, menu, _) = fixture.frame(Vec::new(), false);
        assert!(menu.geometry.rect.is_none(), "{mutation}");
        assert!(menu.invocation.is_none());
        assert_eq!(fixture.text(), before);
    }
}

#[test]
fn 직렬_문자_검색과_enter는_같은_프레임에서_선택한_항목을_한_번_실행한다() {
    let mut fixture = Fixture::new();
    fixture.frame(Vec::new(), false);
    fixture.frame(Vec::new(), true);
    let (_, output, _) = fixture.frame(vec![Event::Text("p".into()), key(Key::Enter)], false);
    assert_eq!(
        output
            .invocation
            .as_ref()
            .map(|invocation| invocation.action),
        Some(Action::Paste)
    );
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 직렬_방향_하위_메뉴_선택과_enter는_같은_프레임_순서를_보존한다() {
    let mut fixture = Fixture::new();
    fixture.menu.definition = true;
    fixture.menu.type_definition = true;
    fixture.frame(Vec::new(), false);
    fixture.frame(Vec::new(), true);
    let (_, output, _) = fixture.frame(
        vec![
            key(Key::ArrowDown),
            key(Key::ArrowDown),
            key(Key::ArrowDown),
            key(Key::ArrowDown),
            key(Key::ArrowRight),
            key(Key::End),
            key(Key::Enter),
        ],
        false,
    );
    assert_eq!(
        output
            .invocation
            .as_ref()
            .map(|invocation| invocation.action),
        Some(Action::Command("editor.action.peekTypeDefinition"))
    );
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 왼쪽과_escape는_하위_메뉴만_닫고_루트_포커스를_보존한다() {
    for close_key in [Key::ArrowLeft, Key::Escape] {
        let mut fixture = Fixture::new();
        fixture.menu.definition = true;
        fixture.frame(Vec::new(), false);
        fixture.frame(Vec::new(), true);
        fixture.frame(
            vec![
                key(Key::ArrowDown),
                key(Key::ArrowDown),
                key(Key::ArrowDown),
                key(Key::ArrowRight),
            ],
            false,
        );
        let (_, output, _) = fixture.frame(vec![key(close_key)], false);
        assert!(output.geometry.rect.is_some());
        assert!(
            !output
                .geometry
                .items
                .iter()
                .any(|(action, _)| *action == Action::Command("editor.action.peekDefinition"))
        );
        assert!(output.invocation.is_none());
        let (_, selected, _) = fixture.frame(vec![key(Key::ArrowUp), key(Key::Enter)], false);
        assert_eq!(
            selected
                .invocation
                .as_ref()
                .map(|invocation| invocation.action),
            Some(Action::ToggleMinimap)
        );
        assert_eq!(fixture.text(), TEXT);
    }
}

#[test]
fn 실제_항목_클릭은_메뉴_소유_선택을_보존하고_명령을_한_번만_반환한다() {
    let mut fixture = Fixture::new();
    fixture.frame(Vec::new(), false);
    let (_, menu, _) = fixture.frame(Vec::new(), true);
    let position = menu
        .geometry
        .items
        .iter()
        .find(|(action, _)| *action == Action::Copy)
        .unwrap()
        .1
        .center();
    let (_, pressed, _) = fixture.frame(
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        false,
    );
    assert!(pressed.invocation.is_none());
    let (_, selected, _) = fixture.frame(
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        false,
    );
    let invocation = selected.invocation.unwrap();
    assert_eq!(invocation.action, Action::Copy);
    assert_eq!(invocation.session.source, fixture.view);
    assert!(invocation.session.matches(
        &fixture.store,
        fixture.view,
        fixture.view,
        egui::ViewportId::ROOT
    ));
    let (_, closed, _) = fixture.frame(Vec::new(), false);
    assert!(closed.invocation.is_none());
    assert!(closed.geometry.rect.is_none());
    assert_eq!(fixture.text(), TEXT);
}

#[test]
fn 우클릭과_메뉴_키가_같은_프레임에_와도_본문_입력으로_새지_않는다() {
    let mut fixture = Fixture::new();
    let (shown, _, _) = fixture.frame(Vec::new(), false);
    let position = shown.geometry.caret_rect(9).unwrap().center();
    let (_, output, _) = fixture.frame(
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Secondary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
            key(Key::ArrowDown),
            key(Key::ArrowDown),
            key(Key::Enter),
        ],
        false,
    );
    assert_eq!(
        fixture.text(),
        TEXT,
        "opened={}, rect={:?}, selection={:?}",
        output.opened,
        output.geometry.rect,
        fixture.selection()
    );
    assert_eq!(
        output
            .invocation
            .as_ref()
            .map(|invocation| invocation.action),
        Some(Action::Command("editor.action.changeAll")),
        "opened={}, rect={:?}, selection={:?}, remaining={:?}",
        output.opened,
        output.geometry.rect,
        fixture.selection(),
        fixture.context.input(|input| input.events.clone())
    );
    assert_eq!(fixture.text(), TEXT);
    assert_eq!(
        fixture.selection().selections,
        [Selection { anchor: 9, head: 9 }]
    );
}
