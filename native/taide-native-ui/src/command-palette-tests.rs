use super::*;
use egui::{PointerButton, Pos2, RawInput, Rect};
use taide_model::{ids::ProjectId, paths::AppPaths};
use taide_runtime::{AppState, theme_actions};

const SCREEN: [f32; 2] = [1000.0, 800.0];
const NARROW_SCREEN: [f32; 2] = [600.0, 800.0];
const FRAME_STEP: f64 = 0.1;
const COLLATION_LOCALE: &str = "en-US";
const ROOT: &str = "/synthetic/project";
const ACTIVE_FILE: &str = "/synthetic/project/src/main.rs";
const EXIT_FRAME_STEP: f64 = 0.02;
const EXIT_FRAME_COUNT: usize = 4;
const PREVIOUS_FOCUS: &str = "synthetic-previous-focus";
const PREVIOUS_FOCUS_SIDE: f32 = 8.0;
const BACKGROUND_BUTTON: &str = "synthetic-background-button";
const BACKGROUND_BUTTON_CENTER: Pos2 = Pos2::new(4.0, 12.0);
const OUTSIDE_DIALOG: Pos2 = Pos2::new(20.0, 780.0);
const SINGLE_LINE_ROW_HEIGHT: f32 = 32.0;
const TWO_LINE_ROW_HEIGHT: f32 = 48.0;
const OVERSIZED_FILE_COUNT: usize = 250;
const ENTER_SCALE: f32 = 0.95;
const GEOMETRY_TOLERANCE: f32 = 0.01;
const LIST_ITEM_ICON_SIDE: f32 = 16.0;
const SPINNER_SIDE: f32 = 12.0;
const SETTLE_FRAME_COUNT: usize = 3;

struct Scene {
    context: egui::Context,
    palette: Palette,
    locale: ResolvedLocale,
    commands: CommandContext,
    root: Option<String>,
    paths: Option<Vec<String>>,
    revision: u64,
    is_pending: bool,
    is_refreshing: bool,
    active_file: Option<String>,
    overrides: Option<String>,
    screen: [f32; 2],
    enabled: bool,
    time: f64,
    frame_step: f64,
    should_focus_previous: bool,
    background_clicks: usize,
    unconsumed_keys: Vec<Key>,
    #[cfg(feature = "native-host")]
    symbols: Vec<taide_native_editor::document_symbols::Symbol>,
    #[cfg(feature = "native-host")]
    symbols_pending: bool,
}

impl Scene {
    fn new() -> Self {
        let context = egui::Context::default();
        context.set_os(egui::os::OperatingSystem::Mac);
        Self {
            context,
            palette: Palette::new(&theme(), COLLATION_LOCALE, true).unwrap(),
            locale: ResolvedLocale {
                id: "en".into(),
                name: "English".into(),
                messages: serde_json::from_str(include_str!(
                    "../../../crates/taide-locale/resources/locales/en.json"
                ))
                .unwrap(),
                warnings: Vec::new(),
            },
            commands: CommandContext::default(),
            root: None,
            paths: None,
            revision: 0,
            is_pending: false,
            is_refreshing: false,
            active_file: None,
            overrides: None,
            screen: SCREEN,
            enabled: true,
            time: 0.0,
            frame_step: FRAME_STEP,
            should_focus_previous: false,
            background_clicks: 0,
            unconsumed_keys: Vec::new(),
            #[cfg(feature = "native-host")]
            symbols: Vec::new(),
            #[cfg(feature = "native-host")]
            symbols_pending: false,
        }
    }

    fn with_project(mut self, paths: &[&str]) -> Self {
        self.commands.active_project = Some(ProjectId::new());
        self.root = Some(ROOT.into());
        self.set_paths(paths);
        self
    }

    fn set_paths(&mut self, paths: &[&str]) {
        self.paths = Some(paths.iter().map(|path| format!("{ROOT}/{path}")).collect());
        self.revision += 1;
    }

    fn frame(&mut self, events: Vec<Event>) -> Output {
        self.time += self.frame_step;
        let mut output = None;
        let mut drawing = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(self.screen[0], self.screen[1]),
                )),
                time: Some(self.time),
                focused: true,
                events,
                ..Default::default()
            },
            |ui| {
                let previous = ui.interact(
                    Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(PREVIOUS_FOCUS_SIDE)),
                    Id::new(PREVIOUS_FOCUS),
                    Sense::focusable_noninteractive(),
                );
                if std::mem::take(&mut self.should_focus_previous) {
                    previous.request_focus();
                }
                let button = ui.interact(
                    Rect::from_center_size(
                        BACKGROUND_BUTTON_CENTER,
                        egui::Vec2::splat(PREVIOUS_FOCUS_SIDE),
                    ),
                    Id::new(BACKGROUND_BUTTON),
                    Sense::click(),
                );
                self.background_clicks += usize::from(button.clicked());
                output = Some(
                    self.palette
                        .show(
                            ui.ctx(),
                            Scope {
                                locale: &self.locale,
                                commands: &self.commands,
                                keymap_overrides: self.overrides.as_deref(),
                                files: FileIndex {
                                    root: self.root.as_deref(),
                                    paths: self.paths.as_deref(),
                                    revision: self.revision,
                                    is_pending: self.is_pending,
                                    is_refreshing: self.is_refreshing,
                                },
                                active_file: self.active_file.as_deref(),
                                #[cfg(feature = "native-host")]
                                symbols: SymbolIndex {
                                    entries: Some(&self.symbols),
                                    generation: self.revision,
                                    is_pending: self.symbols_pending,
                                },
                            },
                            self.enabled,
                        )
                        .unwrap(),
                );
                self.unconsumed_keys = ui.input(|input| {
                    input
                        .events
                        .iter()
                        .filter_map(|event| match event {
                            Event::Key {
                                key, pressed: true, ..
                            } => Some(*key),
                            _ => None,
                        })
                        .collect()
                });
            },
        );
        drawing.textures_delta.clear();
        output.unwrap()
    }

    fn open(&mut self, entry: PaletteEntry) {
        self.palette.open(&self.context, entry);
        self.frame(Vec::new());
        self.frame(Vec::new());
    }

    fn type_text(&mut self, text: &str) -> Output {
        let output = self.frame(vec![Event::Text(text.into())]);
        self.frame(Vec::new());
        output
    }

    fn press(&mut self, key: Key) -> Output {
        self.press_with(key, Modifiers::NONE)
    }

    fn press_with(&mut self, key: Key, modifiers: Modifiers) -> Output {
        let output = self.frame(vec![key_event(key, modifiers)]);
        if self.palette.is_open() {
            self.frame(Vec::new());
        }
        output
    }

    fn click(&mut self, position: Pos2) -> Output {
        self.frame(vec![Event::PointerMoved(position)]);
        self.frame(vec![pointer_button(position, true)]);
        self.frame(vec![pointer_button(position, false)])
    }

    fn inspection(&self) -> Inspection {
        self.palette.inspection().clone()
    }

    fn selected(&self) -> Option<String> {
        self.inspection()
            .rows
            .into_iter()
            .find(|row| row.is_selected)
            .map(|row| row.key)
    }

    fn labels(&self) -> Vec<String> {
        self.inspection()
            .rows
            .into_iter()
            .map(|row| row.label)
            .collect()
    }

    fn focused(&self) -> Option<Id> {
        self.context.memory(|memory| memory.focused())
    }

    fn focus_previous(&mut self) {
        self.context
            .memory_mut(|memory| memory.request_focus(Id::new(PREVIOUS_FOCUS)));
        self.frame(Vec::new());
        assert_eq!(self.focused(), Some(Id::new(PREVIOUS_FOCUS)));
    }

    fn runnable_commands(&self) -> Vec<String> {
        registry()
            .unwrap()
            .commands()
            .iter()
            .filter(|command| command.is_registered(true) && command.is_runnable(&self.commands))
            .map(|command| command.id.clone())
            .collect()
    }
}

fn theme() -> ResolvedTheme {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-command-palette-ui-{}", ProjectId::new())),
    ));
    theme_actions::theme_get(&state, "taide-dark".into()).unwrap()
}

fn key_event(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn pointer_button(position: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn preedit(text: &str) -> Event {
    Event::Ime(egui::ImeEvent::Preedit {
        text: text.into(),
        active_range_chars: None,
    })
}

fn command() -> Modifiers {
    Modifiers {
        mac_cmd: true,
        command: true,
        ..Default::default()
    }
}

fn control() -> Modifiers {
    Modifiers {
        ctrl: true,
        ..Default::default()
    }
}

#[test]
fn 팔레트는_진입_질의로_열려_입력에_포커스하고_캐럿을_끝에_둔다() {
    let mut scene = Scene::new();
    assert_eq!(scene.frame(Vec::new()), Output::default());
    assert!(!scene.palette.is_open() && !scene.inspection().is_open);

    scene.open(PaletteEntry::Commands);
    let opened = scene.inspection();
    assert!(scene.palette.is_open() && opened.is_open && opened.has_input_focus);
    assert_eq!(opened.query, ">");
    assert!(!scene.palette.observes_files());
    let dialog = opened.dialog.unwrap();
    assert_eq!(dialog.width(), MAX_WIDTH);
    assert_eq!(dialog.center().x, SCREEN[0] / 2.0);
    assert!((dialog.center().y - SCREEN[1] / 2.0).abs() <= 1.0);
    let input = opened.input.unwrap();
    assert_eq!(input.height(), INPUT_ROW_HEIGHT);
    assert_eq!(input.width(), MAX_WIDTH - BORDER_WIDTH * 2.0);

    scene.type_text("set");
    assert_eq!(scene.inspection().query, ">set");
    let caret = egui::TextEdit::load_state(&scene.context, input_id(&scene.context))
        .unwrap()
        .cursor
        .char_range()
        .unwrap();
    assert!(caret.is_empty());
    assert_eq!(
        caret.primary.index,
        egui::text::CharIndex(">set".chars().count())
    );

    scene.palette.open(&scene.context, PaletteEntry::Files);
    scene.frame(Vec::new());
    let files = scene.inspection();
    assert!(files.is_open && files.has_input_focus);
    assert_eq!(files.query, "");
    assert_eq!(files.placeholder, "Search by file name...");
    assert!(scene.palette.observes_files());

    scene
        .palette
        .open(&scene.context, PaletteEntry::WorkspaceSymbols);
    scene.frame(Vec::new());
    scene.type_text("x");
    assert_eq!(scene.inspection().query, "#x");

    scene.screen = NARROW_SCREEN;
    scene.frame(Vec::new());
    scene.frame(Vec::new());
    assert_eq!(
        scene.inspection().dialog.unwrap().width(),
        NARROW_SCREEN[0] - SCREEN_MARGIN * 2.0
    );
}

#[test]
fn 파일_모드는_상대_경로를_두_줄로_보이고_로딩_갱신_상한을_표시한다() {
    let mut without_project = Scene::new();
    without_project.open(PaletteEntry::Files);
    let empty = without_project.inspection();
    assert_eq!(empty.heading.as_deref(), Some("Files"));
    assert_eq!(empty.empty_message.as_deref(), Some("No results"));
    assert!(empty.rows.is_empty() && empty.refreshing.is_none());

    let mut scene = Scene::new().with_project(&[]);
    scene.paths = None;
    scene.is_pending = true;
    scene.is_refreshing = true;
    scene.open(PaletteEntry::Files);
    let loading = scene.inspection();
    assert_eq!(loading.empty_message.as_deref(), Some("Loading..."));
    assert_eq!(loading.refreshing.as_deref(), Some("Refreshing"));

    scene.is_pending = false;
    scene.is_refreshing = false;
    scene.set_paths(&["src/main.rs", "README.md", "src/widgets/palette.rs"]);
    scene.frame(Vec::new());
    let listed = scene.inspection();
    assert!(listed.empty_message.is_none() && listed.refreshing.is_none());
    assert_eq!(
        listed
            .rows
            .iter()
            .map(|row| (row.key.as_str(), row.label.as_str(), row.detail.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("/synthetic/project/src/main.rs", "main.rs", Some("src")),
            ("/synthetic/project/README.md", "README.md", None),
            (
                "/synthetic/project/src/widgets/palette.rs",
                "palette.rs",
                Some("src/widgets")
            ),
        ]
    );
    assert!(listed.rows.iter().all(|row| row.is_enabled));
    assert_eq!(listed.rows[0].rect.height(), TWO_LINE_ROW_HEIGHT);
    assert_eq!(listed.rows[1].rect.height(), SINGLE_LINE_ROW_HEIGHT);
    assert_eq!(
        listed.rows[0].rect.width(),
        MAX_WIDTH - (BORDER_WIDTH + GROUP_PADDING) * 2.0
    );
    assert_eq!(
        scene.selected().as_deref(),
        Some("/synthetic/project/src/main.rs")
    );

    scene.type_text("palette widgets");
    assert_eq!(scene.labels(), ["palette.rs"]);
    scene.type_text(" zzzz");
    let unmatched = scene.inspection();
    assert!(unmatched.rows.is_empty());
    assert_eq!(unmatched.empty_message.as_deref(), Some("No results"));
    assert_eq!(unmatched.heading.as_deref(), Some("Files"));

    let many = (0..OVERSIZED_FILE_COUNT)
        .map(|index| format!("src/file-{index:03}.rs"))
        .collect::<Vec<_>>();
    let mut limited =
        Scene::new().with_project(&many.iter().map(String::as_str).collect::<Vec<_>>());
    limited.open(PaletteEntry::Files);
    assert_eq!(limited.inspection().rows.len(), FILE_RESULT_LIMIT);
    limited.type_text("file");
    assert_eq!(limited.inspection().rows.len(), FILE_RESULT_LIMIT);
    assert_eq!(limited.inspection().list.unwrap().height(), LIST_MAX_HEIGHT);
}

#[test]
fn 명령_모드는_등록된_전체_명령을_보이고_실행할_수_없는_명령을_비활성으로_둔다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    let listed = scene.inspection();
    let registered = registry()
        .unwrap()
        .commands()
        .iter()
        .filter(|command| command.is_registered(true))
        .collect::<Vec<_>>();
    assert_eq!(listed.heading.as_deref(), Some("Commands"));
    assert_eq!(listed.placeholder, "Type a command to run...");
    assert_eq!(
        listed
            .rows
            .iter()
            .map(|row| (row.key.as_str(), row.label.clone(), row.is_enabled))
            .collect::<Vec<_>>(),
        registered
            .iter()
            .map(|command| (
                command.id.as_str(),
                command.label(&scene.locale),
                command.is_runnable(&scene.commands)
            ))
            .collect::<Vec<_>>()
    );
    let row = |key: &str| {
        listed
            .rows
            .iter()
            .find(|row| row.key == key)
            .unwrap()
            .clone()
    };
    assert!(!row("window.reload").is_enabled);
    assert!(row("settings.open").is_enabled);
    assert_eq!(row("settings.open").label, "App: Settings");
    assert_eq!(row("settings.open").shortcut, None);
    assert_eq!(row("file.quickOpen").shortcut.as_deref(), Some("⌘P"));
    assert_eq!(row("keybindings.open").shortcut.as_deref(), Some("⌘K ⌘S"));
    assert_eq!(row("settings.open").rect.height(), SINGLE_LINE_ROW_HEIGHT);
    assert_eq!(
        scene.selected(),
        scene.runnable_commands().first().cloned(),
        "첫 활성 항목이 선택된다"
    );

    scene.overrides = Some(r#"[{"actionId":"settings.open","key":",","mods":["mod"]}]"#.into());
    scene.frame(Vec::new());
    assert_eq!(
        scene
            .inspection()
            .rows
            .iter()
            .find(|row| row.key == "settings.open")
            .unwrap()
            .shortcut
            .as_deref(),
        Some("⌘,")
    );

    scene.type_text("app settings");
    assert_eq!(
        scene.labels()[..2],
        ["App: Settings", "App: Open settings.json"]
    );
    scene.type_text(" zzzz");
    let unmatched = scene.inspection();
    assert!(unmatched.rows.is_empty());
    assert_eq!(unmatched.heading.as_deref(), Some("Commands"));
    assert_eq!(unmatched.empty_message.as_deref(), Some("No results"));
}

#[test]
fn 줄_이동과_심볼_모드는_활성_파일과_프로젝트_유무에_따른_빈_상태를_보인다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Files);
    scene.type_text(":12:4");
    let no_file = scene.inspection();
    assert!(no_file.rows.is_empty() && no_file.heading.is_none());
    assert_eq!(no_file.empty_message.as_deref(), Some("Open a file first"));

    scene.active_file = Some(ACTIVE_FILE.into());
    scene.frame(Vec::new());
    let target = scene.inspection();
    assert!(target.heading.is_none() && target.empty_message.is_none());
    assert_eq!(scene.labels(), ["12:4"]);
    assert_eq!(scene.selected().as_deref(), Some(LINE_ITEM_KEY));

    scene.palette.set_query(":abc".into());
    scene.frame(Vec::new());
    let invalid = scene.inspection();
    assert!(invalid.rows.is_empty());
    assert_eq!(invalid.empty_message.as_deref(), Some("No results"));
    scene.palette.set_query(":7".into());
    scene.frame(Vec::new());
    assert_eq!(scene.labels(), ["7"]);

    scene.palette.set_query("@handle".into());
    scene.frame(Vec::new());
    let symbols = scene.inspection();
    assert_eq!(symbols.heading.as_deref(), Some("Symbols"));
    assert_eq!(symbols.empty_message.as_deref(), Some("No results"));
    assert!(symbols.rows.is_empty());
    scene.active_file = None;
    scene.frame(Vec::new());
    assert_eq!(
        scene.inspection().empty_message.as_deref(),
        Some("Open a file first")
    );

    scene.palette.set_query("#handle".into());
    scene.frame(Vec::new());
    let workspace = scene.inspection();
    assert_eq!(workspace.heading.as_deref(), Some("Workspace Symbols"));
    assert_eq!(
        workspace.empty_message.as_deref(),
        Some("Open a project first")
    );
    scene.commands.active_project = Some(ProjectId::new());
    scene.frame(Vec::new());
    let with_project = scene.inspection();
    assert!(with_project.rows.is_empty());
    assert_eq!(with_project.empty_message.as_deref(), Some("No results"));
}

#[test]
fn 탐색_키는_활성_항목_사이만_이동하고_양_끝에서_멈춘다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    let runnable = scene.runnable_commands();
    assert!(runnable.len() > 2 && runnable.len() < scene.inspection().rows.len());
    let at = |index: usize| Some(runnable[index].clone());
    let last = runnable.len() - 1;
    assert_eq!(scene.selected(), at(0));

    scene.press(Key::ArrowDown);
    assert_eq!(scene.selected(), at(1));
    scene.press(Key::ArrowUp);
    assert_eq!(scene.selected(), at(0));
    scene.press(Key::ArrowUp);
    assert_eq!(scene.selected(), at(0), "처음에서 위로 돌아가지 않는다");
    scene.press(Key::End);
    assert_eq!(scene.selected(), at(last));
    scene.press(Key::ArrowDown);
    assert_eq!(scene.selected(), at(last), "끝에서 아래로 돌아가지 않는다");
    scene.press(Key::Home);
    assert_eq!(scene.selected(), at(0));
    scene.press_with(Key::ArrowDown, command());
    assert_eq!(scene.selected(), at(last));
    scene.press_with(Key::ArrowUp, command());
    assert_eq!(scene.selected(), at(0));
    scene.press_with(Key::N, control());
    assert_eq!(scene.selected(), at(1));
    scene.press_with(Key::J, control());
    assert_eq!(scene.selected(), at(2));
    scene.press_with(Key::P, control());
    assert_eq!(scene.selected(), at(1));
    scene.press_with(Key::K, control());
    assert_eq!(scene.selected(), at(0));
    scene.press_with(
        Key::N,
        Modifiers {
            shift: true,
            ..control()
        },
    );
    assert_eq!(scene.selected(), at(0));
    scene.press_with(
        Key::ArrowDown,
        Modifiers {
            alt: true,
            ..Default::default()
        },
    );
    assert_eq!(scene.selected(), at(1));
    scene.press(Key::PageDown);
    scene.press(Key::PageUp);
    assert_eq!(
        scene.selected(),
        at(1),
        "PageUp/PageDown 은 선택을 옮기지 않는다"
    );

    scene.press(Key::Home);
    scene.type_text("a");
    assert_eq!(
        scene.inspection().query,
        ">a",
        "탐색 키는 입력 캐럿을 옮기지 않는다"
    );
    let filtered = scene.inspection();
    assert_eq!(
        scene.selected(),
        filtered
            .rows
            .iter()
            .find(|row| row.is_enabled)
            .map(|row| row.key.clone()),
        "질의가 바뀌면 첫 활성 항목으로 돌아간다"
    );
    scene.press(Key::Tab);
    assert!(
        scene.inspection().has_input_focus,
        "Tab 은 입력 포커스를 옮기지 않는다"
    );
}

#[cfg(feature = "native-host")]
#[test]
fn 문서_심볼은_이름_fuzzy_한줄_breadcrumb_입력과_현재_세대_선택을_보존한다() {
    use taide_native_editor::document_symbols::{Symbol, SymbolKind};
    let mut scene = Scene::new().with_project(&[]);
    scene.active_file = Some(ACTIVE_FILE.into());
    scene.symbols_pending = true;
    scene.open(PaletteEntry::Files);
    scene.type_text("@");
    assert!(scene.palette.observes_symbols());
    assert_eq!(
        scene.inspection().empty_message.as_deref(),
        Some("Loading...")
    );
    scene.symbols_pending = false;
    scene.symbols = ["Class", "method", "me\u{301}thod"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| Symbol {
            name: name.into(),
            detail: "signature is not displayed".into(),
            kind: SymbolKind::FUNCTION,
            tags: Vec::new(),
            parent: (index > 0).then_some(0),
            container_label: if index == 0 {
                String::new()
            } else {
                "Class > nested".into()
            },
            bytes: index..index + 1,
            selection: index..index + 1,
        })
        .collect();
    scene.frame(Vec::new());
    assert_eq!(scene.labels(), ["Class", "method", "méthod"]);
    for row in scene.inspection().rows {
        assert_eq!(row.icon, Icon::Braces);
        assert_eq!(row.rect.height(), SINGLE_LINE_ROW_HEIGHT);
    }
    assert_eq!(
        scene.inspection().rows[1].detail.as_deref(),
        Some("Class > nested")
    );
    scene.type_text("mth");
    assert_eq!(scene.labels(), ["method", "méthod"]);
    scene.press(Key::ArrowDown);
    let generation = scene.revision;
    assert_eq!(
        scene.press(Key::Enter).action,
        Some(Action::RevealSymbol {
            generation,
            index: 2
        })
    );
    assert!(!scene.palette.is_open());
    scene.open(PaletteEntry::Files);
    scene.type_text("@");
    let row = scene.inspection().rows[0].clone();
    assert_eq!(
        scene.click(row.rect.center()).action,
        Some(Action::RevealSymbol {
            generation,
            index: 0
        })
    );
    scene.open(PaletteEntry::Files);
    scene.type_text("@");
    scene.active_file = None;
    scene.frame(Vec::new());
    assert!(scene.inspection().rows.is_empty());
    assert_eq!(
        scene.inspection().empty_message.as_deref(),
        Some("Open a file first")
    );
}

#[test]
fn 선택_항목은_목록_스크롤_안에서_보이도록_유지된다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    let initial = scene.inspection();
    let list = initial.list.unwrap();
    assert_eq!(list.height(), LIST_MAX_HEIGHT);
    assert_eq!(list.width(), MAX_WIDTH - BORDER_WIDTH * 2.0);
    let visible = |scene: &Scene| {
        let inspection = scene.inspection();
        let list = inspection.list.unwrap();
        let selected = inspection
            .rows
            .into_iter()
            .find(|row| row.is_selected)
            .unwrap();
        list.contains_rect(selected.rect)
    };
    assert!(visible(&scene));
    scene.press(Key::End);
    scene.frame(Vec::new());
    assert!(visible(&scene));
    assert!(
        scene.inspection().rows[0].rect.bottom() < list.top(),
        "끝 항목을 보이려고 목록이 스크롤된다"
    );
    scene.press(Key::Home);
    scene.frame(Vec::new());
    assert!(visible(&scene));

    scene.press(Key::End);
    scene.frame(Vec::new());
    scene.press(Key::Escape);
    scene.open(PaletteEntry::Commands);
    assert!(
        scene.inspection().rows[0].rect.top() >= list.top(),
        "다시 열면 목록이 처음 위치에서 시작한다"
    );
}

#[test]
fn enter는_선택_항목을_실행하고_동작으로_닫힌_팔레트는_이전_포커스로_돌아가지_않는다() {
    let mut scene = Scene::new().with_project(&["src/main.rs", "README.md"]);
    scene.active_file = Some(ACTIVE_FILE.into());
    scene.focus_previous();
    scene.open(PaletteEntry::Commands);
    scene.type_text("app settings");
    assert_eq!(scene.selected().as_deref(), Some("settings.open"));
    let output = scene.press(Key::Enter);
    assert_eq!(
        output.action,
        Some(Action::RunCommand("settings.open".into()))
    );
    assert!(!scene.palette.is_open());
    assert_eq!(scene.palette.query, "");
    scene.frame(Vec::new());
    assert_eq!(
        scene.focused(),
        None,
        "동작으로 닫히면 이전 포커스를 되돌리지 않는다"
    );
    assert!(!scene.inspection().is_open);

    scene.open(PaletteEntry::Commands);
    assert_eq!(scene.inspection().query, ">", "닫을 때 질의가 초기화된다");
    scene.type_text("quick open");
    assert_eq!(scene.selected().as_deref(), Some("file.quickOpen"));
    let switched = scene.press(Key::Enter);
    assert_eq!(switched.action, None);
    assert!(
        scene.palette.is_open(),
        "파일 검색 전환은 팔레트를 닫지 않는다"
    );
    assert_eq!(scene.inspection().query, "");
    assert_eq!(scene.inspection().heading.as_deref(), Some("Files"));

    scene.press(Key::ArrowDown);
    let opened = scene.press(Key::Enter);
    assert_eq!(
        opened.action,
        Some(Action::OpenFile("/synthetic/project/README.md".into()))
    );
    assert!(!scene.palette.is_open());

    scene.open(PaletteEntry::Files);
    scene.type_text(":12:4");
    let revealed = scene.press(Key::Enter);
    assert_eq!(
        revealed.action,
        Some(Action::RevealLine(LineTarget {
            line: 12.0,
            column: 4.0
        }))
    );
    assert!(!scene.palette.is_open());

    scene.open(PaletteEntry::Commands);
    scene.type_text("zzzz");
    assert_eq!(scene.press(Key::Enter), Output::default());
    assert!(
        scene.palette.is_open(),
        "선택 항목이 없으면 Enter 는 아무것도 하지 않는다"
    );
    scene.palette.set_query(">reload window".into());
    scene.frame(Vec::new());
    assert_eq!(scene.labels()[0], "Window: Reload Window");
    assert_eq!(
        scene.selected(),
        None,
        "실행할 수 없는 명령은 선택되지 않는다"
    );
    assert_eq!(scene.press(Key::Enter), Output::default());
    assert!(scene.palette.is_open());
}

#[test]
fn escape와_바깥_클릭은_닫으면서_이전_포커스를_되돌리고_질의를_비운다() {
    let mut scene = Scene::new();
    scene.focus_previous();
    scene.open(PaletteEntry::Commands);
    scene.type_text("set");
    assert!(scene.inspection().has_input_focus);
    assert_eq!(scene.press(Key::Escape), Output::default());
    assert!(!scene.palette.is_open());
    scene.frame(Vec::new());
    assert_eq!(scene.focused(), Some(Id::new(PREVIOUS_FOCUS)));
    scene.frame(Vec::new());
    assert_eq!(
        scene.focused(),
        Some(Id::new(PREVIOUS_FOCUS)),
        "되돌린 포커스가 다음 프레임에도 유지된다"
    );

    scene.open(PaletteEntry::Files);
    assert_eq!(scene.inspection().query, "");
    scene.type_text(">set");
    let inside = scene.inspection().input.unwrap().center();
    assert_eq!(scene.click(inside), Output::default());
    assert!(scene.palette.is_open(), "팔레트 안쪽 클릭은 닫지 않는다");
    scene.frame(Vec::new());
    assert!(scene.inspection().has_input_focus);
    assert_eq!(scene.click(OUTSIDE_DIALOG), Output::default());
    assert!(!scene.palette.is_open(), "바깥 클릭은 닫는다");
    scene.frame(Vec::new());
    scene.frame(Vec::new());
    assert_eq!(scene.focused(), Some(Id::new(PREVIOUS_FOCUS)));

    scene.open(PaletteEntry::Commands);
    scene.press(Key::Escape);
    scene.open(PaletteEntry::Files);
    scene.press(Key::Escape);
    scene.frame(Vec::new());
    scene.frame(Vec::new());
    assert_eq!(
        scene.focused(),
        Some(Id::new(PREVIOUS_FOCUS)),
        "닫힌 직후 다시 열었다 닫아도 처음 포커스로 돌아간다"
    );
}

#[test]
fn ime_조합_중_enter와_escape는_실행하거나_닫지_않는다() {
    let mut scene = Scene::new().with_project(&["문서/설정.md", "문서/설치.md", "src/main.rs"]);
    scene.open(PaletteEntry::Files);
    scene.frame(vec![preedit("설")]);
    scene.frame(Vec::new());
    let composing = scene.inspection();
    assert_eq!(composing.query, "설", "조합 중인 글자도 질의에 반영된다");
    assert_eq!(scene.labels(), ["설정.md", "설치.md"]);
    let first = Some("/synthetic/project/문서/설정.md".to_owned());
    assert_eq!(scene.selected(), first);
    assert_eq!(scene.press(Key::Enter), Output::default());
    assert!(scene.palette.is_open());
    assert_eq!(scene.press(Key::Escape), Output::default());
    assert!(scene.palette.is_open());
    assert_eq!(scene.selected(), first);
    assert!(scene.inspection().has_input_focus);

    let committed = scene.frame(vec![
        Event::Ime(egui::ImeEvent::Commit("설".into())),
        key_event(Key::Enter, Modifiers::NONE),
    ]);
    assert_eq!(
        committed,
        Output::default(),
        "조합을 확정하는 Enter 는 항목을 실행하지 않는다"
    );
    assert!(scene.palette.is_open());
    scene.frame(Vec::new());
    assert_eq!(scene.inspection().query, "설");
    scene.press(Key::ArrowDown);
    assert_eq!(
        scene.press(Key::Enter).action,
        Some(Action::OpenFile("/synthetic/project/문서/설치.md".into()))
    );

    scene.open(PaletteEntry::Files);
    scene.frame(vec![preedit("설")]);
    scene.frame(Vec::new());
    assert_eq!(scene.selected(), first);
    scene.press(Key::ArrowDown);
    scene.press(Key::End);
    assert_eq!(
        scene.selected(),
        first,
        "조합 중에는 탐색 키도 선택을 옮기지 않는다"
    );
}

#[test]
fn 포인터_이동과_클릭은_활성_항목만_선택하고_실행한다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    let listed = scene.inspection();
    let list = listed.list.unwrap();
    let visible = listed
        .rows
        .iter()
        .filter(|row| list.contains_rect(row.rect))
        .cloned()
        .collect::<Vec<_>>();
    let disabled = visible.iter().find(|row| !row.is_enabled).unwrap().clone();
    let target = visible
        .iter()
        .rfind(|row| row.is_enabled && !row.is_selected && row.key != "file.quickOpen")
        .unwrap()
        .clone();
    let initial = scene.selected();

    scene.frame(vec![Event::PointerMoved(disabled.rect.center())]);
    scene.frame(Vec::new());
    assert_eq!(
        scene.selected(),
        initial,
        "비활성 항목은 포인터로 선택되지 않는다"
    );
    assert_eq!(scene.click(disabled.rect.center()), Output::default());
    assert!(scene.palette.is_open());

    scene.frame(vec![Event::PointerMoved(target.rect.center())]);
    scene.frame(Vec::new());
    assert_eq!(scene.selected(), Some(target.key.clone()));
    scene.frame(Vec::new());
    scene.press(Key::ArrowUp);
    assert_ne!(
        scene.selected(),
        Some(target.key.clone()),
        "멈춘 포인터는 키보드 선택을 되돌리지 않는다"
    );
    let output = scene.click(target.rect.center());
    assert_eq!(output.action, Some(Action::RunCommand(target.key)));
    assert!(!scene.palette.is_open());
}

#[test]
fn 팔레트가_처리한_키는_소비되어_뒤에_그려지는_화면으로_전달되지_않는다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    scene.frame(vec![
        key_event(Key::ArrowDown, Modifiers::NONE),
        key_event(Key::ArrowUp, Modifiers::NONE),
        key_event(Key::Home, Modifiers::NONE),
        key_event(Key::End, Modifiers::NONE),
        key_event(Key::N, control()),
        key_event(Key::P, control()),
        key_event(Key::A, Modifiers::NONE),
    ]);
    assert_eq!(
        scene.unconsumed_keys,
        [Key::A],
        "팔레트가 다루지 않는 키만 남는다"
    );
    scene.frame(vec![key_event(Key::Enter, Modifiers::NONE)]);
    assert!(scene.unconsumed_keys.is_empty());
    scene.open(PaletteEntry::Commands);
    scene.frame(vec![key_event(Key::Escape, Modifiers::NONE)]);
    assert!(scene.unconsumed_keys.is_empty() && !scene.palette.is_open());
    scene.frame(vec![key_event(Key::Escape, Modifiers::NONE)]);
    assert_eq!(
        scene.unconsumed_keys,
        [Key::Escape],
        "닫힌 팔레트는 키를 소비하지 않는다"
    );
}

#[test]
fn 비활성_팔레트는_키와_포인터에_반응하지_않는다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    let selected = scene.selected();
    scene.enabled = false;
    scene.frame(Vec::new());
    assert_eq!(scene.press(Key::ArrowDown), Output::default());
    assert_eq!(scene.selected(), selected);
    assert_eq!(scene.press(Key::Enter), Output::default());
    assert_eq!(scene.press(Key::Escape), Output::default());
    assert_eq!(scene.click(OUTSIDE_DIALOG), Output::default());
    assert!(scene.palette.is_open());
    scene.enabled = true;
    scene.frame(Vec::new());
    scene.press(Key::Escape);
    assert!(!scene.palette.is_open());
}

#[test]
fn 팔레트_색상은_모든_builtin_테마의_정본_키로_초기화된다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-command-palette-theme-{}", ProjectId::new())),
    ));
    let themes = theme_actions::theme_list(&state).unwrap();
    assert!(!themes.is_empty());
    for theme in themes.into_iter().filter(|theme| theme.builtin) {
        let resolved = theme_actions::theme_get(&state, theme.id).unwrap();
        let appearance = Appearance::new(&resolved).unwrap();
        assert_eq!(
            appearance.match_highlight,
            color(&resolved, "panel.matchHighlight").unwrap()
        );
        assert_eq!(
            appearance.selection_ring,
            color(&resolved, "app.accent").unwrap()
        );
    }
    let mut broken = theme();
    broken.colors.remove("panel.matchHighlight");
    assert!(Appearance::new(&broken).is_err());
}

fn dialog_layer() -> egui::LayerId {
    modal::layer_id(Id::new(MODAL_ID))
}

fn dialog_scale(scene: &Scene) -> Option<f32> {
    scene
        .context
        .layer_transform_to_global(dialog_layer())
        .map(|transform| transform.scaling)
}

fn top_modal_layer(scene: &Scene) -> Option<egui::LayerId> {
    scene.context.memory(|memory| memory.top_modal_layer())
}

fn is_near(left: f32, right: f32) -> bool {
    (left - right).abs() < GEOMETRY_TOLERANCE
}

fn settle(scene: &mut Scene) {
    for _ in 0..SETTLE_FRAME_COUNT {
        scene.frame(Vec::new());
    }
}

#[test]
fn 열림_전환은_200ms_동안_본문을_95퍼센트에서_키우고_첫_프레임부터_입력을_받는다() {
    let mut scene = Scene::new();
    scene.frame(Vec::new());
    scene.palette.open(&scene.context, PaletteEntry::Commands);
    scene.frame(Vec::new());
    let entering = dialog_scale(&scene).unwrap();
    assert!(is_near(entering, ENTER_SCALE));
    assert!(scene.inspection().is_open);
    assert_eq!(top_modal_layer(&scene), Some(dialog_layer()));

    scene.frame(Vec::new());
    let halfway = dialog_scale(&scene).unwrap();
    assert!(halfway > entering && halfway < 1.0);
    assert!(scene.inspection().has_input_focus);
    scene.type_text("set");
    assert_eq!(scene.inspection().query, ">set");
    assert_eq!(dialog_scale(&scene), None, "200ms 뒤에는 제 크기로 그린다");
    assert_eq!(scene.inspection().dialog.unwrap().width(), MAX_WIDTH);
}

#[test]
fn 닫힘_전환_동안_레이어는_남아_있지만_키를_받지_않고_200ms_뒤에_사라진다() {
    let mut scene = Scene::new();
    scene.focus_previous();
    scene.open(PaletteEntry::Commands);
    settle(&mut scene);
    assert_eq!(dialog_scale(&scene), None);

    scene.press(Key::Escape);
    assert!(!scene.palette.is_open());
    scene.frame(vec![
        key_event(Key::Enter, Modifiers::NONE),
        key_event(Key::Escape, Modifiers::NONE),
    ]);
    let leaving = dialog_scale(&scene).unwrap();
    assert!(leaving < 1.0 && leaving > ENTER_SCALE);
    assert_eq!(scene.unconsumed_keys, [Key::Enter, Key::Escape]);
    assert_eq!(scene.inspection(), Inspection::default());
    assert_eq!(scene.focused(), Some(Id::new(PREVIOUS_FOCUS)));

    settle(&mut scene);
    assert_eq!(dialog_scale(&scene), None);
    assert_eq!(top_modal_layer(&scene), None);
    assert!(
        !scene
            .context
            .dismissal_layers()
            .contains(&Id::new(MODAL_ID))
    );
    scene.frame(Vec::new());
    assert_eq!(scene.focused(), Some(Id::new(PREVIOUS_FOCUS)));
}

#[test]
fn 동작으로_닫힌_뒤_뒤_화면_위젯이_요청한_포커스는_닫힘_전환이_끝난_뒤에도_유지된다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    settle(&mut scene);
    scene.type_text("app settings");
    assert_eq!(
        scene.press(Key::Enter).action,
        Some(Action::RunCommand("settings.open".into()))
    );
    assert!(!scene.palette.is_open());

    scene.frame_step = EXIT_FRAME_STEP;
    scene.should_focus_previous = true;
    for _ in 0..EXIT_FRAME_COUNT {
        scene.frame(Vec::new());
        assert!(
            dialog_scale(&scene).is_some(),
            "닫힘 전환이 아직 진행 중이다"
        );
        assert_eq!(
            scene.focused(),
            Some(Id::new(PREVIOUS_FOCUS)),
            "닫힘 전환 중인 팔레트는 뒤 화면 위젯의 포커스를 빼앗지 않는다"
        );
    }
    scene.frame_step = FRAME_STEP;
    settle(&mut scene);
    assert_eq!(dialog_scale(&scene), None);
    assert_eq!(scene.focused(), Some(Id::new(PREVIOUS_FOCUS)));
}

#[test]
fn 닫힘_전환_동안_뒤_화면은_포인터_입력을_받지_않고_전환이_끝나면_다시_받는다() {
    let mut scene = Scene::new();
    scene.open(PaletteEntry::Commands);
    settle(&mut scene);
    scene.press(Key::Escape);
    assert!(!scene.palette.is_open());

    scene.frame_step = EXIT_FRAME_STEP;
    scene.click(BACKGROUND_BUTTON_CENTER);
    assert!(
        dialog_scale(&scene).is_some(),
        "클릭이 닫힘 전환 안에서 끝났다"
    );
    assert_eq!(
        scene.background_clicks, 0,
        "닫힘 전환이 끝날 때까지 뒤 화면의 포인터 입력을 막는다"
    );

    scene.frame_step = FRAME_STEP;
    settle(&mut scene);
    assert_eq!(dialog_scale(&scene), None);
    scene.click(BACKGROUND_BUTTON_CENTER);
    assert_eq!(scene.background_clicks, 1);
}

#[test]
fn 닫힘_전환_중에_다시_열면_열림_전환을_처음부터_시작하고_입력에_포커스한다() {
    let mut scene = Scene::new();
    scene.focus_previous();
    scene.open(PaletteEntry::Commands);
    settle(&mut scene);
    scene.press(Key::Escape);
    scene.palette.open(&scene.context, PaletteEntry::Files);
    scene.frame(Vec::new());
    assert!(is_near(dialog_scale(&scene).unwrap(), ENTER_SCALE));
    let reopened = scene.inspection();
    assert!(reopened.is_open && reopened.has_input_focus);
    assert_eq!(reopened.heading.as_deref(), Some("Files"));
}

#[test]
fn reduced_motion에서는_전환_없이_열리고_닫힌_다음_프레임에_사라진다() {
    let mut scene = Scene::new();
    scene.palette.set_reduced_motion(true);
    scene.palette.open(&scene.context, PaletteEntry::Commands);
    scene.frame(Vec::new());
    assert_eq!(dialog_scale(&scene), None);
    assert!(scene.inspection().is_open);
    assert_eq!(top_modal_layer(&scene), Some(dialog_layer()));

    scene.frame(Vec::new());
    scene.press(Key::Escape);
    scene.frame(Vec::new());
    assert_eq!(top_modal_layer(&scene), None);
    assert!(
        !scene
            .context
            .dismissal_layers()
            .contains(&Id::new(MODAL_ID))
    );
}

#[test]
fn 입력과_항목은_모드별_고정_아이콘을_왼쪽에_두고_글자를_8px_뒤에_둔다() {
    let mut scene = Scene::new().with_project(&["src/main.rs", "README.md"]);
    scene.active_file = Some(ACTIVE_FILE.into());
    scene.is_refreshing = true;
    scene.open(PaletteEntry::Files);
    settle(&mut scene);
    let files = scene.inspection();
    let input = files.input.unwrap();
    let search = files.input_icon.unwrap();
    assert_eq!(search.size(), egui::Vec2::splat(Icon::Search.size()));
    assert_eq!(Icon::Search.size(), LIST_ITEM_ICON_SIDE);
    assert!(is_near(search.left(), input.left() + INPUT_PADDING_X));
    assert!(is_near(search.center().y, input.center().y));
    let field = files.input_field.unwrap();
    assert!(is_near(field.left(), search.right() + INPUT_ICON_GAP));
    assert!(is_near(field.right(), input.right() - INPUT_PADDING_X));

    let assert_rows = |inspection: &Inspection, icon: Icon| {
        assert!(!inspection.rows.is_empty());
        for row in &inspection.rows {
            assert_eq!(row.icon, icon, "{}", row.key);
            assert_eq!(row.icon_rect.size(), egui::Vec2::splat(LIST_ITEM_ICON_SIDE));
            assert!(is_near(
                row.icon_rect.left(),
                row.rect.left() + ITEM_PADDING_X
            ));
            assert!(is_near(row.icon_rect.center().y, row.rect.center().y));
            assert!(is_near(row.label_left, row.icon_rect.right() + ITEM_GAP));
        }
    };
    assert_rows(&files, Icon::File);
    assert_eq!(files.rows[0].rect.height(), TWO_LINE_ROW_HEIGHT);
    assert_eq!(files.rows[1].rect.height(), SINGLE_LINE_ROW_HEIGHT);

    let (spinner, angle) = files.refreshing_icon.unwrap();
    assert_eq!(spinner.size(), egui::Vec2::splat(SPINNER_SIDE));
    assert!(spinner.left() > files.rows[0].rect.left() + HEADING_PADDING_X);
    assert!(spinner.bottom() <= files.rows[0].rect.top());
    scene.frame(Vec::new());
    let (_, next_angle) = scene.inspection().refreshing_icon.unwrap();
    let turned = (next_angle - angle).rem_euclid(std::f32::consts::TAU);
    assert!(
        is_near(
            turned,
            (FRAME_STEP / SPIN_SECONDS) as f32 * std::f32::consts::TAU
        ),
        "1초에 한 바퀴를 등속으로 돈다"
    );
    scene.is_refreshing = false;
    scene.frame(Vec::new());
    assert_eq!(scene.inspection().refreshing_icon, None);

    scene.palette.open(&scene.context, PaletteEntry::Commands);
    scene.frame(Vec::new());
    assert_rows(&scene.inspection(), Icon::Terminal);

    scene.palette.set_query(":12".into());
    scene.frame(Vec::new());
    let line = scene.inspection();
    assert_eq!(scene.labels(), ["12"]);
    assert_rows(&line, Icon::CornerDownLeft);
}
