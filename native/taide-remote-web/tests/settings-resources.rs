use std::collections::BTreeMap;

use serde_json::json;
use taide_model::{
    error::AppErrorKind,
    ids::{PaneId, ProjectId, TabId},
    locale::ResolvedLocale,
    settings::{EditorRenderWhitespace, Settings},
    theme::{ResolvedTheme, ThemeType},
};
use taide_native_ui::{
    command_score,
    settings_code_controls::{Change as CodeChange, Selection, Switch},
    settings_controls::Change,
    settings_owner::Owner,
    settings_resources::{Cache, Kind, RESOURCE_GC_TIME, Reply, Resources},
    settings_view::{Appearance, Output, Views},
    theme_editor_tokens::COLORS,
};
use taide_remote_web::{ResponsePayload, settings_resources::ResourceReads};

const SCREEN: [f32; 2] = [1100.0, 900.0];
const FRAME_STEP: f64 = 0.1;
const FONT_SEQ: u32 = 11;
const SHELL_SEQ: u32 = 12;
const STALE_SEQ: u32 = 13;
const FAILED_SEQ: u32 = 14;
const UNKNOWN_SEQ: u32 = 15;
const SCORE_EPSILON: f64 = 0.000000000000001;
const SEARCH_MATCHES: usize = 2;
const SHELL_MIN_HEIGHT: f32 = 46.0;
const SHELL_INSET_X: f32 = 13.0;
const SHELL_INSET_Y: f32 = 7.0;
const PATH_REPEATS: usize = 120;
const POPUP_MID_TIME: f64 = 0.075;
const POPUP_END_PADDING: f64 = 0.001;
const POPUP_SCALE_EPSILON: f32 = 0.000001;
const POPUP_SCALE_DELTA: f32 = 0.05;
const POPUP_MID_PROGRESS: f32 = 0.5;
const KEYMAP_BUTTON_HEIGHT: f32 = 32.0;

#[test]
fn 원본_keymap_section은_목차와_설명_및_기존편집기_버튼을_표시한다() {
    let mut surface = Surface::new();
    surface.locale.messages.insert(
        "settings.keymapDescription".into(),
        "Synthetic keymap description".into(),
    );
    surface.context.enable_accesskit();
    let first = surface.show(Vec::new());
    surface.click(&first, "settings.keymap");
    let keymap = surface.settled_scroll();
    let button = keymap
        .traces
        .iter()
        .find(|trace| trace.field == "settings.keymapOpenEditor")
        .unwrap();
    assert_eq!(button.rect.height(), KEYMAP_BUTTON_HEIGHT);
    let drawing = surface.drawing.as_ref().unwrap();
    assert!(drawing.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "Synthetic keymap description")));
    assert!(
        drawing
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(id, node)| *id == button.id.accesskit_id()
                && node.role() == egui::accesskit::Role::Button
                && node.label() == Some("settings.keymapOpenEditor"))
    );
    let clicked = surface.click(&keymap, "settings.keymapOpenEditor");
    assert!(clicked.open_keybindings);
    assert!(clicked.changes.is_empty());
    assert!(!surface.show(Vec::new()).open_keybindings);
    surface.enabled = false;
    assert!(
        !surface
            .click(&keymap, "settings.keymapOpenEditor")
            .open_keybindings
    );
}

#[test]
fn 원본_popup의_tab_loop와_닫힘_presence는_검색_및_옵션_포커스와_transform을_회수한다() {
    let mut surface = Surface::new();
    surface.context.enable_accesskit();
    let first = surface.show(Vec::new());
    for request in first.resources {
        if request.kind() == Kind::Fonts {
            assert!(surface.views.accept_resource(Reply::Fonts {
                request,
                result: Ok(Vec::new())
            }));
        }
    }
    let original = surface.show(Vec::new());
    surface.click(&original, "settings.editor");
    let editor = surface.settled_scroll();
    let trigger = editor
        .traces
        .iter()
        .find(|trace| trace.field == "settings.editorFontFamily")
        .unwrap()
        .id;
    surface.click(&editor, "settings.editorFontFamily");
    let visible = surface.show(Vec::new());
    let search = visible
        .traces
        .iter()
        .find(|trace| trace.field == "font-search")
        .unwrap()
        .id;
    let layer = surface.context.read_response(search).unwrap().layer_id;
    surface.key(egui::Key::Tab);
    assert_eq!(
        surface.context.memory(|memory| memory.focused()),
        Some(search)
    );
    surface.key_with_modifiers(egui::Key::Tab, egui::Modifiers::SHIFT);
    assert_eq!(
        surface.context.memory(|memory| memory.focused()),
        Some(search)
    );
    let closing = surface.key(egui::Key::Escape);
    assert!(
        closing
            .traces
            .iter()
            .any(|trace| trace.field == "font-search")
    );
    assert_eq!(
        surface.context.memory(|memory| memory.focused()),
        Some(search)
    );
    let closing_nodes = &surface
        .drawing
        .as_ref()
        .unwrap()
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes;
    let trigger_node = &closing_nodes
        .iter()
        .find(|(id, _)| *id == trigger.accesskit_id())
        .unwrap()
        .1;
    assert_eq!(trigger_node.is_expanded(), Some(false));
    assert_eq!(trigger_node.controls().len(), 1);
    surface.show_after(POPUP_MID_TIME, Vec::new());
    let transform = surface.context.layer_transform_to_global(layer).unwrap();
    let expected = 1.0 - POPUP_SCALE_DELTA * taide_native_ui::css_motion::ease(POPUP_MID_PROGRESS);
    assert!((transform.scaling - expected).abs() < POPUP_SCALE_EPSILON);
    let closed = surface.show_after(POPUP_MID_TIME + POPUP_END_PADDING, Vec::new());
    assert!(
        !closed
            .traces
            .iter()
            .any(|trace| trace.field == "font-search")
    );
    assert_eq!(
        surface.context.memory(|memory| memory.focused()),
        Some(trigger)
    );
    assert_eq!(
        surface
            .context
            .layer_transform_to_global(layer)
            .unwrap_or(egui::emath::TSTransform::IDENTITY),
        egui::emath::TSTransform::IDENTITY
    );
    surface.click(&closed, "settings.editorRenderWhitespace");
    surface.show(Vec::new());
    let dialog = surface
        .drawing
        .as_ref()
        .unwrap()
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.role() == egui::accesskit::Role::Dialog)
        .unwrap()
        .0;
    assert_eq!(
        surface
            .context
            .memory(|memory| memory.focused())
            .unwrap()
            .accesskit_id(),
        dialog
    );
    surface.key(egui::Key::Tab);
    assert_eq!(
        surface
            .context
            .memory(|memory| memory.focused())
            .unwrap()
            .accesskit_id(),
        dialog
    );
    surface.key_with_modifiers(egui::Key::Tab, egui::Modifiers::SHIFT);
    assert_eq!(
        surface
            .context
            .memory(|memory| memory.focused())
            .unwrap()
            .accesskit_id(),
        dialog
    );
    surface.key(egui::Key::Escape);
    surface.show_after(
        POPUP_MID_TIME + POPUP_MID_TIME + POPUP_END_PADDING,
        Vec::new(),
    );
    assert!(
        !surface
            .drawing
            .as_ref()
            .unwrap()
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::Dialog)
    );
}

#[test]
fn 조합중_검색의_enter_escape는_설정선택이나_popup_닫기로_소비되지_않는다() {
    let mut surface = Surface::new();
    surface.context.enable_accesskit();
    let first = surface.show(Vec::new());
    for request in first.resources {
        if request.kind() == Kind::Fonts {
            assert!(surface.views.accept_resource(Reply::Fonts {
                request,
                result: Ok(Vec::new()),
            }));
        }
    }
    let original = surface.show(Vec::new());
    surface.click(&original, "settings.editor");
    let editor = surface.settled_scroll();
    surface.click(&editor, "settings.editorFontFamily");
    let opened = surface.show(Vec::new());
    let input = opened
        .traces
        .iter()
        .find(|trace| trace.field == "font-search")
        .unwrap()
        .id;
    assert_eq!(
        surface.context.memory(|memory| memory.focused()),
        Some(input)
    );
    let composing = surface.show(vec![egui::Event::Ime(egui::ImeEvent::Preedit {
        text: "漢".into(),
        active_range_chars: None,
    })]);
    assert!(composing.changes.is_empty());
    let commit_key = surface.key(egui::Key::Enter);
    assert!(commit_key.changes.is_empty());
    surface.key(egui::Key::Escape);
    let still_open = surface.show(Vec::new());
    assert!(
        still_open
            .traces
            .iter()
            .any(|trace| trace.field == "font-search")
    );
    surface.show(vec![egui::Event::Ime(egui::ImeEvent::Commit("漢".into()))]);
    surface.key(egui::Key::Escape);
    let closed = surface.show_after(
        POPUP_MID_TIME + POPUP_MID_TIME + POPUP_END_PADDING,
        Vec::new(),
    );
    assert!(
        !closed
            .traces
            .iter()
            .any(|trace| trace.field == "font-search")
    );
}

#[test]
fn 새_긴_셸_경로는_여러_줄로_행_안에_배치되고_버튼은_전체_이름과_pressed를_노출한다() {
    let mut surface = Surface::new();
    surface.context.enable_accesskit();
    let path = format!("/synthetic/{}", "long-component/".repeat(PATH_REPEATS));
    let first = surface.show(Vec::new());
    for request in first.resources {
        let reply = match request.kind() {
            Kind::Fonts => Reply::Fonts {
                request,
                result: Ok(Vec::new()),
            },
            Kind::Shells => Reply::Shells {
                request,
                result: Ok(vec![taide_model::terminal::ShellProfile {
                    id: "long-shell".into(),
                    name: "Synthetic long shell".into(),
                    path: path.clone(),
                    args: Vec::new(),
                }]),
            },
        };
        assert!(surface.views.accept_resource(reply));
    }
    surface.settings.shell_override = Some(path.clone());
    let original = surface.show(Vec::new());
    surface.click(&original, "settings.terminal");
    let terminal = surface.settled_scroll();
    let profile = terminal
        .traces
        .iter()
        .find(|trace| trace.field == "shell-profile")
        .unwrap();
    assert!(profile.rect.height() > SHELL_MIN_HEIGHT);
    let drawing = surface.drawing.as_ref().unwrap();
    let rendered = drawing
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == path && profile.rect.contains(text.pos) =>
            {
                Some(text)
            }
            _ => None,
        })
        .unwrap();
    assert!(rendered.galley.rows.len() > 1);
    assert!(rendered.pos.x >= profile.rect.left() + SHELL_INSET_X);
    assert!(rendered.pos.x + rendered.galley.size().x <= profile.rect.right() - SHELL_INSET_X);
    assert!(rendered.pos.y + rendered.galley.size().y <= profile.rect.bottom() - SHELL_INSET_Y);
    let node = drawing
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(id, _)| *id == profile.id.accesskit_id())
        .unwrap();
    assert_eq!(node.1.role(), egui::accesskit::Role::Button);
    assert_eq!(node.1.toggled(), Some(egui::accesskit::Toggled::True));
    assert_eq!(
        node.1.label().unwrap(),
        format!("Synthetic long shell {path}")
    );
    assert!(
        drawing
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == egui::accesskit::Role::ListItem)
    );
}

#[test]
fn 새_picker는_실제_ax_펼침_선택_관계와_disabled_입력_및_escape_회수를_보존한다() {
    let mut surface = Surface::new();
    surface.context.enable_accesskit();
    let first = surface.show(Vec::new());
    for request in first.resources {
        if request.kind() == Kind::Fonts {
            assert!(surface.views.accept_resource(Reply::Fonts {
                request,
                result: Ok(vec![taide_model::font::FontFamily {
                    name: "Synthetic Mono".into(),
                    monospaced: true
                }])
            }));
        }
    }
    let original = surface.show(Vec::new());
    surface.click(&original, "settings.editor");
    let editor = surface.settled_scroll();
    let trigger = editor
        .traces
        .iter()
        .find(|trace| trace.field == "settings.editorFontFamily")
        .unwrap();
    let trigger_id = trigger.id.accesskit_id();
    let closed = surface
        .drawing
        .as_ref()
        .unwrap()
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap();
    let node = &closed
        .nodes
        .iter()
        .find(|(id, _)| *id == trigger_id)
        .unwrap()
        .1;
    assert_eq!(node.is_expanded(), Some(false));
    assert_eq!(node.toggled(), None);
    assert_eq!(node.label(), Some("settings.fontFamilySelectPlaceholder"));
    surface.click(&editor, "settings.editorFontFamily");
    surface.show(Vec::new());
    let opened = surface
        .drawing
        .as_ref()
        .unwrap()
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap();
    let trigger = &opened
        .nodes
        .iter()
        .find(|(id, _)| *id == trigger_id)
        .unwrap()
        .1;
    assert_eq!(trigger.is_expanded(), Some(true));
    let popup = opened
        .nodes
        .iter()
        .find(|(_, node)| node.role() == egui::accesskit::Role::Dialog)
        .unwrap();
    assert_eq!(trigger.controls(), &[popup.0]);
    let list = opened
        .nodes
        .iter()
        .find(|(_, node)| node.role() == egui::accesskit::Role::ListBox)
        .unwrap();
    let input = opened
        .nodes
        .iter()
        .find(|(_, node)| node.role() == egui::accesskit::Role::EditableComboBox)
        .unwrap();
    assert_eq!(input.1.controls(), &[list.0]);
    assert_eq!(input.1.is_expanded(), Some(true));
    assert_eq!(list.1.active_descendant(), input.1.active_descendant());
    assert!(
        opened
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == egui::accesskit::Role::ListBoxOption)
            .count()
            > 1
    );
    surface.enabled = false;
    let blocked = surface.key(egui::Key::End);
    assert!(blocked.changes.is_empty());
    let disabled = surface
        .drawing
        .as_ref()
        .unwrap()
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap();
    assert!(
        disabled
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == egui::accesskit::Role::ListBoxOption)
            .all(|(_, node)| node.is_disabled())
    );
    assert!(surface.key(egui::Key::Enter).changes.is_empty());
    surface.enabled = true;
    let enabled = surface.show(Vec::new());
    surface.click(&enabled, "font-search");
    surface.key(egui::Key::End);
    let selected = surface
        .drawing
        .as_ref()
        .unwrap()
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap();
    let option = selected
        .nodes
        .iter()
        .find(|(_, node)| {
            node.role() == egui::accesskit::Role::ListBoxOption && node.is_selected() == Some(true)
        })
        .unwrap();
    assert_eq!(option.1.label(), Some("Synthetic Mono"));
    assert_eq!(
        selected
            .nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::ListBox)
            .unwrap()
            .1
            .active_descendant(),
        Some(option.0)
    );
    surface.key(egui::Key::Escape);
    surface.show_after(
        POPUP_MID_TIME + POPUP_MID_TIME + POPUP_END_PADDING,
        Vec::new(),
    );
    let cleared = surface
        .drawing
        .as_ref()
        .unwrap()
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap();
    assert!(!cleared.nodes.iter().any(|(_, node)| matches!(
        node.role(),
        egui::accesskit::Role::ListBox
            | egui::accesskit::Role::ListBoxOption
            | egui::accesskit::Role::EditableComboBox
    )));
    let trigger = &cleared
        .nodes
        .iter()
        .find(|(id, _)| *id == trigger_id)
        .unwrap()
        .1;
    assert_eq!(trigger.is_expanded(), Some(false));
    assert!(trigger.controls().is_empty());
}

#[test]
fn 폰트_popup은_클릭_후_새_pass에서_검색_입력을_자동으로_포커스한다() {
    let mut surface = Surface::new();
    let first = surface.show(Vec::new());
    for request in first.resources {
        if request.kind() == Kind::Fonts {
            surface.views.accept_resource(Reply::Fonts {
                request,
                result: Ok(vec![
                    taide_model::font::FontFamily {
                        name: "Synthetic Mono".into(),
                        monospaced: true,
                    },
                    taide_model::font::FontFamily {
                        name: "Synthetic Sans".into(),
                        monospaced: false,
                    },
                ]),
            });
        }
    }
    let original = surface.show(Vec::new());
    surface.click(&original, "settings.editor");
    let editor = surface.settled_scroll();
    let position = editor
        .traces
        .iter()
        .find(|trace| trace.field == "settings.editorFontFamily")
        .unwrap()
        .rect
        .center();
    surface.show(vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    surface.show(vec![egui::Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    let focused = surface.show(Vec::new());
    let search = focused
        .traces
        .iter()
        .find(|trace| trace.field == "font-search")
        .unwrap();
    assert_eq!(
        surface.context.memory(|memory| memory.focused()),
        Some(search.id),
        "captured focus before request: {}",
        search.has_focus
    );
    surface.key(egui::Key::ArrowLeft);
    assert_eq!(
        surface.context.memory(|memory| memory.focused()),
        Some(search.id)
    );
}

struct Surface {
    views: Views,
    context: egui::Context,
    owner: Owner,
    appearance: Appearance,
    locale: ResolvedLocale,
    settings: Settings,
    time: f64,
    scroll: Option<(egui::Id, egui::Vec2, egui::Rect)>,
    drawing: Option<egui::FullOutput>,
    enabled: bool,
}

impl Surface {
    fn new() -> Self {
        let theme = ResolvedTheme {
            id: "taide-dark".into(),
            name: "Synthetic".into(),
            theme_type: ThemeType::Dark,
            colors: COLORS
                .iter()
                .flat_map(|(namespace, keys)| {
                    keys.iter()
                        .map(move |key| (format!("{namespace}.{key}"), "#123456".into()))
                })
                .collect(),
            syntax: BTreeMap::new(),
            terminal: BTreeMap::new(),
            token_colors: None,
            syntax_overrides: Vec::new(),
            warnings: Vec::new(),
            author: None,
            license: None,
            source: None,
        };
        Self {
            views: Views::default(),
            context: egui::Context::default(),
            owner: Owner {
                project: ProjectId::new(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            appearance: Appearance::new(&theme).unwrap(),
            locale: ResolvedLocale {
                id: "en".into(),
                name: "English".into(),
                messages: BTreeMap::new(),
                warnings: Vec::new(),
            },
            settings: Settings::default(),
            time: 0.0,
            scroll: None,
            drawing: None,
            enabled: true,
        }
    }

    fn show(&mut self, events: Vec<egui::Event>) -> Output {
        self.time += FRAME_STEP;
        self.views.begin_frame();
        let mut output = Output::default();
        let mut drawing = self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                if !self.enabled {
                    ui.disable();
                }
                output = self.views.show(
                    ui,
                    self.owner.clone(),
                    &self.settings,
                    &self.locale,
                    &self.appearance,
                );
            },
        );
        drawing.textures_delta.clear();
        self.drawing = Some(drawing);
        self.views.finish_frame();
        assert!(output.error.is_none());
        self.scroll = output.scroll;
        output
    }

    fn click(&mut self, output: &Output, key: &str) -> Output {
        let trace = output
            .traces
            .iter()
            .find(|trace| trace.field == key)
            .unwrap_or_else(|| panic!("missing {key}"));
        let position = trace.rect.center();
        assert!(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(SCREEN[0], SCREEN[1]))
                .contains(position),
            "offscreen {key}: {position:?}; scroll {:?}",
            self.scroll
        );
        self.show(vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ])
    }

    fn key(&mut self, key: egui::Key) -> Output {
        self.key_with_modifiers(key, egui::Modifiers::NONE)
    }

    fn key_with_modifiers(&mut self, key: egui::Key, modifiers: egui::Modifiers) -> Output {
        self.show(vec![egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }])
    }

    fn show_after(&mut self, duration: f64, events: Vec<egui::Event>) -> Output {
        self.time += duration - FRAME_STEP;
        self.show(events)
    }

    fn settled_scroll(&mut self) -> Output {
        self.time += f64::from(egui::style::ScrollAnimation::default().duration.max);
        self.show(Vec::new());
        self.show(Vec::new())
    }
}

#[test]
fn 실제_두_설정섹션은_독립목록_응답수명과_원본_picker_검색_입력을_소비한다() {
    for (value, search, expected) in [
        ("Synthetic Mono", "sm", 0.8908218089100001),
        ("system-default", "sys", 0.99),
        ("Synthetic Mono", "syn mo", 0.16725926426522342),
        ("Synthetic Mono", "snt", 0.16811488683),
        ("system-default", "xyz", 0.0),
        ("ab", "ba", 0.1),
        ("Foo_Bar/baz", "fb", 0.7918416079200001),
        ("한글 폰트", "ㅎ", 0.0),
        ("한글 폰트", "한폰", 0.891),
        ("AA", "a", 0.989901),
        ("abc", "acb", 0.1),
        ("", "", 1.0),
        ("İMono", "im", 0.16809807534131702),
        ("a-b_c", "abc", 0.7200000000000001),
    ] {
        assert!(
            (command_score::score(value, search) - expected).abs() < SCORE_EPSILON,
            "score {value:?}/{search:?}: {}",
            command_score::score(value, search)
        );
    }
    let mut surface = Surface::new();
    let first = surface.show(Vec::new());
    let fonts = first
        .resources
        .iter()
        .find(|request| request.kind() == Kind::Fonts)
        .unwrap()
        .clone();
    let shells = first
        .resources
        .iter()
        .find(|request| request.kind() == Kind::Shells)
        .unwrap()
        .clone();
    assert_eq!(ResourceReads::call(&fonts).command, "font_list");
    assert_eq!(ResourceReads::call(&shells).command, "shell_profiles");
    assert!(ResourceReads::call(&fonts).args.is_null());
    assert!(surface.show(Vec::new()).resources.is_empty());
    let mut reads = ResourceReads::default();
    reads.sent(fonts.clone(), FONT_SEQ);
    reads.sent(shells.clone(), SHELL_SEQ);
    let font_result = Ok(ResponsePayload::Json(json!([
        {"name":"Synthetic Mono","monospaced":true}, {"name":"Synthetic Sans","monospaced":false}
    ])));
    assert!(!reads.response(UNKNOWN_SEQ, &font_result));
    assert!(reads.response(FONT_SEQ, &font_result));
    assert!(!reads.response(FONT_SEQ, &font_result));
    for reply in reads.take_finished() {
        assert!(surface.views.accept_resource(reply));
    }
    let view = surface.views.inspection().get(&surface.owner).unwrap();
    assert_eq!(view.resources.fonts().unwrap().len(), 2);
    assert!(view.resources.shells().is_none());
    let shell_result = Ok(ResponsePayload::Json(json!([
        {"id":"synthetic","name":"Synthetic shell","path":"/synthetic/shell","args":[]}
    ])));
    assert!(reads.response(SHELL_SEQ, &shell_result));
    for reply in reads.take_finished() {
        assert!(surface.views.accept_resource(reply));
    }
    assert!(surface.show(Vec::new()).resources.is_empty());
    let original = surface.show(Vec::new());
    for field in Switch::ALL {
        assert!(
            original
                .traces
                .iter()
                .any(|trace| trace.field == field.label()),
            "missing {}",
            field.label()
        );
    }
    for field in [
        "settings.editorFontSize",
        "settings.editorTabSize",
        "settings.autoSaveDelayMs",
        "settings.terminalFontSize",
        "settings.terminalScrollback",
        "settings.editorFontFamily",
        "settings.terminalFontFamily",
        "settings.editorRulers",
        "settings.shell",
        "settings.editorRenderWhitespace",
        "settings.editorCursorStyle",
        "settings.editorCursorBlinking",
        "settings.terminalCursorStyle",
        "shell-profile",
    ] {
        assert!(
            original.traces.iter().any(|trace| trace.field == field),
            "missing {field}"
        );
    }
    surface.click(&original, "settings.editor");
    assert_eq!(
        surface
            .views
            .inspection()
            .get(&surface.owner)
            .unwrap()
            .active,
        taide_native_ui::settings_controls::Section::Editor
    );
    let editor = surface.settled_scroll();
    let switched = surface.click(&editor, "settings.formatOnSave");
    assert_eq!(
        switched.changes,
        [Change::Code(CodeChange::Switch(
            Switch::FormatOnSave,
            !surface.settings.format_on_save
        ))]
    );
    let editor = surface.show(Vec::new());
    surface.click(&editor, "settings.editorFontFamily");
    surface.show(Vec::new());
    let searching = surface.show(vec![egui::Event::Text("sm".into())]);
    assert_eq!(
        searching
            .traces
            .iter()
            .filter(|trace| trace.field == "picker-option")
            .count(),
        SEARCH_MATCHES
    );
    let selected = surface.key(egui::Key::Enter);
    assert_eq!(
        selected.changes,
        [Change::Code(CodeChange::Font(
            taide_native_ui::settings_code_controls::Font::Editor,
            Some("Synthetic Mono".into())
        ))]
    );
    let editor = surface.show(Vec::new());
    surface.click(&editor, "settings.editorRenderWhitespace");
    surface.show(Vec::new());
    surface.key(egui::Key::End);
    let selected = surface.key(egui::Key::Enter);
    assert_eq!(
        selected.changes,
        [Change::Code(CodeChange::Selection(Selection::Whitespace(
            EditorRenderWhitespace::All
        )))]
    );
    let editor = surface.show(Vec::new());
    surface.click(&editor, "settings.terminal");
    let terminal = surface.settled_scroll();
    let selected = surface.click(&terminal, "shell-profile");
    assert_eq!(
        selected.changes,
        [Change::Code(CodeChange::Shell("/synthetic/shell".into()))]
    );

    surface.views.refresh_resources();
    let stale = surface
        .show(Vec::new())
        .resources
        .into_iter()
        .find(|request| request.kind() == Kind::Fonts)
        .unwrap();
    reads.sent(stale.clone(), STALE_SEQ);
    surface.views.begin_frame();
    surface.views.finish_frame();
    let remounted = surface.show(Vec::new());
    assert!(remounted.resources.is_empty());
    assert!(reads.response(STALE_SEQ, &font_result));
    for reply in reads.take_finished() {
        assert!(surface.views.accept_resource(reply));
    }
    assert_eq!(
        surface
            .views
            .inspection()
            .get(&surface.owner)
            .unwrap()
            .resources
            .fonts()
            .unwrap()
            .len(),
        2
    );
    surface.views.refresh_resources();
    let new = surface
        .show(Vec::new())
        .resources
        .into_iter()
        .find(|request| request.kind() == Kind::Fonts)
        .unwrap();
    reads.sent(new, FAILED_SEQ);
    assert!(reads.response(FAILED_SEQ, &Ok(ResponsePayload::Binary(vec![0]))));
    for reply in reads.take_finished() {
        assert!(surface.views.accept_resource(reply));
    }
    let view = surface.views.inspection().get(&surface.owner).unwrap();
    assert!(view.resources.fonts().unwrap().is_empty());
    assert_eq!(
        view.resources.error(Kind::Fonts).unwrap().kind(),
        AppErrorKind::Internal
    );
    assert!(surface.show(Vec::new()).resources.is_empty());
    let mut resource = Resources::default();
    let requests = resource.requests(&surface.owner);
    let wrong = requests
        .iter()
        .find(|request| request.kind() == Kind::Shells)
        .unwrap()
        .clone();
    assert!(!resource.accept(Reply::Fonts {
        request: wrong,
        result: Ok(Vec::new())
    }));
    let mut reads = ResourceReads::default();
    for (seq, request) in requests.into_iter().enumerate() {
        reads.sent(request, u32::try_from(seq).unwrap());
    }
    reads.disconnected();
    let finished = reads.take_finished();
    assert_eq!(finished.len(), Kind::ALL.len());
    for reply in finished {
        assert!(resource.accept(reply));
    }
    reads.disconnected();
    assert!(reads.take_finished().is_empty());
    assert!(resource.requests(&surface.owner).is_empty());
}

#[test]
fn 목록은_전체_마운트가_공유하며_진행응답_보존과_실패_재마운트_및_개별_만료를_따른다() {
    let mut cache = Cache::default();
    let owner = Surface::new().owner;
    let other = Surface::new().owner;
    let start = std::time::Instant::now();
    cache.observe(start, true);
    let requests = cache.requests(&owner);
    let fonts = requests
        .iter()
        .find(|request| request.kind() == Kind::Fonts)
        .unwrap()
        .clone();
    let shells = requests
        .iter()
        .find(|request| request.kind() == Kind::Shells)
        .unwrap()
        .clone();
    cache.observe(start, true);
    assert!(cache.requests(&other).is_empty());
    assert!(cache.accept(
        Reply::Shells {
            request: shells,
            result: Ok(Vec::new())
        },
        start
    ));
    cache.unobserve(start);
    let expired = start + RESOURCE_GC_TIME;
    cache.unobserve(expired);
    assert!(cache.snapshot().shells().is_none());
    assert!(cache.accept(
        Reply::Fonts {
            request: fonts,
            result: Ok(vec![taide_model::font::FontFamily {
                name: "Synthetic Mono".into(),
                monospaced: true
            }])
        },
        expired
    ));
    cache.observe(expired, true);
    let requests = cache.requests(&other);
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].kind(), Kind::Shells);
    assert_eq!(cache.snapshot().fonts().unwrap()[0].name, "Synthetic Mono");
    let first = cache.snapshot();
    let second = cache.snapshot();
    assert!(std::ptr::eq(
        first.fonts().unwrap(),
        second.fonts().unwrap()
    ));
    assert!(
        cache.accept(
            requests[0]
                .clone()
                .failed(taide_model::error::AppError::Internal(
                    "Synthetic refusal".into()
                )),
            expired
        )
    );
    cache.observe(expired, false);
    assert!(cache.requests(&other).is_empty());
    cache.observe(expired, true);
    let retry = cache.requests(&other);
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].kind(), Kind::Shells);
    assert!(cache.accept(
        Reply::Shells {
            request: retry[0].clone(),
            result: Ok(Vec::new())
        },
        expired
    ));
    cache.unobserve(expired);
    cache.observe(
        expired + RESOURCE_GC_TIME - std::time::Duration::from_nanos(1),
        true,
    );
    assert!(cache.requests(&other).is_empty());
    let active = expired + RESOURCE_GC_TIME;
    cache.observe(active, false);
    assert!(cache.requests(&owner).is_empty());
    cache.unobserve(active);
    cache.observe(active + RESOURCE_GC_TIME, true);
    let after_gc = cache.requests(&owner);
    assert_eq!(after_gc.len(), Kind::ALL.len());
    cache.refresh();
    assert!(
        !cache.accept(
            after_gc[0]
                .clone()
                .failed(taide_model::error::AppError::Internal("Stale".into())),
            active + RESOURCE_GC_TIME
        )
    );
}
