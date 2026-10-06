#![cfg(unix)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use taide_infra::terminal_scan::ScanEvent;
use taide_model::{
    app_event::AppEvent, ids::ProjectId, paths::AppPaths, project::Project,
    terminal::PtySpawnOptions,
};
use taide_native_app::{
    bootstrap::services,
    terminal_dispatch::EffectPorts,
    terminal_frames,
    terminal_host::{Hub, InputResult, Limits},
    terminal_writer,
};
use taide_native_terminal::{Rgb, Size, WindowSize, input::NativeInput, session::Phase};
use taide_runtime::{AppServices, AppState, EventSink, TaskSupervisor};
use tokio::{
    sync::{Notify, oneshot},
    time::timeout,
};

const BYTES: usize = 256 * 1024;
const COUNT: usize = 64;
const VISITS: usize = 4096;
const COLUMNS: u16 = 80;
const ROWS: u16 = 24;
const HISTORY: usize = 128;
const TIMEOUT: Duration = Duration::from_secs(3);
const CELL_WIDTH: u16 = 8;
const CELL_HEIGHT: u16 = 16;
const IDE_PORT: u32 = 12345;
const MENU_SCREEN: [f32; 2] = [640.0, 320.0];
const SMALL_MENU_SCREEN: [f32; 2] = [200.0, 220.0];
const RESIZED_COLUMNS: u16 = 96;
const RESIZED_ROWS: u16 = 32;

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);
impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    services: Arc<AppServices>,
    events: Arc<Sink>,
    project: ProjectId,
    tasks: TaskSupervisor,
}
impl Fixture {
    fn new() -> Self {
        let events = Arc::new(Sink::default());
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-native-terminal-host-{}", ProjectId::new())),
        ));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: env!("CARGO_MANIFEST_DIR").into(),
                name: "synthetic terminal".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        Self {
            services: services(state, tasks.clone(), events.clone()),
            project,
            events,
            tasks,
        }
    }

    fn opts(&self) -> PtySpawnOptions {
        PtySpawnOptions {
            project_id: self.project.clone(),
            cwd: env!("CARGO_MANIFEST_DIR").into(),
            shell: Some(env!("CARGO_BIN_EXE_native-terminal-queue-fixture").into()),
            cols: COLUMNS,
            rows: ROWS,
            scrollback_bytes: None,
        }
    }

    async fn finish(&self) {
        self.services.terminal.shutdown();
        timeout(TIMEOUT, self.services.terminal.wait_for_idle())
            .await
            .unwrap()
            .unwrap();
        timeout(TIMEOUT, self.tasks.shutdown()).await.unwrap();
        assert_eq!(self.tasks.tracked_count(), 0);
    }
}

fn limits() -> Limits {
    Limits {
        sessions: 1,
        core: Default::default(),
        frames: terminal_frames::Limits {
            bytes: BYTES,
            count: COUNT,
            visits: VISITS,
        },
        writer: terminal_writer::Limits {
            bytes: BYTES,
            count: COUNT,
        },
    }
}

fn terminal_appearance() -> taide_native_app::terminal_surface::Appearance {
    const FONT_SIZE: u32 = 13;
    let colors = [
        "background",
        "foreground",
        "cursor",
        "selection",
        "black",
        "red",
        "green",
        "yellow",
        "blue",
        "magenta",
        "cyan",
        "white",
        "brightBlack",
        "brightRed",
        "brightGreen",
        "brightYellow",
        "brightBlue",
        "brightMagenta",
        "brightCyan",
        "brightWhite",
    ]
    .into_iter()
    .map(|key| (key, "#aabbcc"))
    .collect::<std::collections::BTreeMap<_, _>>();
    let theme = serde_json::from_value(serde_json::json!({"id":"synthetic", "name":"synthetic", "type":"dark", "colors":{}, "syntax":{}, "terminal":colors})).unwrap();
    taide_native_app::terminal_surface::Appearance::new(&theme, FONT_SIZE).unwrap()
}

#[derive(Clone, Default)]
struct Output {
    titles: Arc<Mutex<Vec<String>>>,
    ready: Arc<Notify>,
}
impl Output {
    fn ports(&self) -> EffectPorts {
        let output = self.clone();
        EffectPorts {
            command_colors: Default::default(),
            updated: Arc::new(|| {}),
            color: Arc::new(|_| Ok(Rgb { r: 0, g: 0, b: 0 })),
            geometry: Arc::new(|| {
                Ok(WindowSize {
                    num_cols: COLUMNS,
                    num_lines: ROWS,
                    cell_width: CELL_WIDTH,
                    cell_height: CELL_HEIGHT,
                })
            }),
            event: Arc::new(|_| Ok(())),
            stream: Arc::new(move |event| {
                if let ScanEvent::Title(title) = event {
                    output.titles.lock().unwrap().push(title.clone());
                    output.ready.notify_one();
                }
                Ok(())
            }),
        }
    }

    async fn wait_ready(&self) {
        self.wait_title("native-ready").await;
    }

    async fn wait_title(&self, expected: &str) {
        timeout(TIMEOUT, async {
            loop {
                if self
                    .titles
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|title| title == expected)
                {
                    return;
                }
                self.ready.notified().await;
            }
        })
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn native_cursor_settings는_첫_실제_pty_출력_전에_설정되고_live_변경에서_grid를_보존한다() {
    use taide_model::settings::TerminalCursorStyle;
    use taide_native_terminal::{CursorShape, CursorStyle, GridDimensions};
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    session
        .snapshot(|snapshot| {
            assert_eq!(
                snapshot.core.cursor_style().unwrap(),
                CursorStyle {
                    shape: CursorShape::Beam,
                    blinking: true
                }
            );
        })
        .unwrap();
    let before = session
        .snapshot(|snapshot| {
            (
                snapshot.revision,
                snapshot.core.grid().unwrap().total_lines(),
                snapshot.core.content().unwrap().cursor.point,
            )
        })
        .unwrap();
    {
        let mut settings = fixture.services.state.settings.write();
        settings.terminal_cursor_style = TerminalCursorStyle::Underline;
        settings.terminal_cursor_blink = false;
    }
    assert!(hub.configure_cursors().unwrap());
    assert!(!hub.configure_cursors().unwrap());
    session
        .snapshot(|snapshot| {
            assert_eq!(
                snapshot.core.cursor_style().unwrap(),
                CursorStyle {
                    shape: CursorShape::Underline,
                    blinking: false
                }
            );
            assert_eq!(snapshot.revision, before.0);
            assert_eq!(snapshot.core.grid().unwrap().total_lines(), before.1);
            assert_eq!(snapshot.core.content().unwrap().cursor.point, before.2);
        })
        .unwrap();
    hub.close(&id).await.unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
}

#[tokio::test]
async fn terminal_commands는_실제_pty_marker_gutter와_mod_이동을_연결한다() {
    use eframe::egui::{self, Event, Modifiers, Rect, pos2, vec2};
    use taide_model::{
        ids::{PaneId, TabId},
        layout::{PaneNode, Tab, TabKind},
    };
    use taide_native_app::{
        host::HostCommand,
        terminal_surface::{Request, Views},
    };
    use taide_native_terminal::{CommandColors, GridDimensions};
    const SCREEN: [f32; 2] = [640.0, 320.0];
    const FRAME_STEP: f64 = 0.2;
    const EXPECTED_COMMANDS: usize = 40;
    const GUTTER_WIDTH: f32 = 2.0;
    const SUCCESS: Rgb = Rgb { r: 1, g: 2, b: 3 };
    const FAILURE: Rgb = Rgb { r: 4, g: 5, b: 6 };
    let fixture = Fixture::new();
    let hub = Arc::new(Hub::new(fixture.services.clone(), limits()).unwrap());
    let output = Output::default();
    let mut ports = output.ports();
    ports.command_colors = CommandColors {
        success: Some(SUCCESS),
        failure: Some(FAILURE),
    };
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_COMMANDS".into(), "1".into())] },
            ports,
        )
        .await
        .unwrap();
    output.wait_ready().await;
    let session = hub.get(&id).unwrap();
    let (current, epoch) = session
        .snapshot(|snapshot| {
            assert_eq!(
                snapshot.core.command_blocks().unwrap().len(),
                EXPECTED_COMMANDS
            );
            (
                snapshot.core.grid().unwrap().history_size(),
                snapshot.core.selection_stamp().unwrap().input_epoch,
            )
        })
        .unwrap();
    assert!(current > 1);
    let pane = PaneId::new();
    let tab = TabId::new();
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = pane.clone();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: TabKind::Terminal {
                session_id: id.clone(),
                cwd: None,
            },
            title: "synthetic commands".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(tab.clone()),
    };
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::<HostCommand>::new();
    let mut time = 0.0;
    let mut draw = |events, enabled| {
        time += FRAME_STEP;
        let mut result = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events,
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    views
                        .show(
                            ui,
                            Request {
                                pane: &pane,
                                tab: &tab,
                                session_id: &id,
                                hub: &hub,
                                services: &fixture.services,
                                appearance: &appearance,
                                locale: &locale,
                                request_focus: true,
                                commands: &mut commands,
                            },
                        )
                        .unwrap();
                });
                views
                    .finish_frame(&hub, &fixture.services, ui.ctx())
                    .unwrap();
                if ui.ctx().current_pass_index() == 0 {
                    ui.ctx()
                        .request_discard("synthetic command keymap multi-pass");
                }
            },
        );
        result.textures_delta.clear();
        commands.clear();
        result
    };
    let text = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                _ => None,
            })
            .collect::<String>()
    };
    let mod_key = if cfg!(target_os = "macos") {
        Modifiers::MAC_CMD | Modifiers::COMMAND
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    };
    let key = |key| Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: mod_key,
    };
    let first = draw(Vec::new(), true);
    assert!(!text(&first).contains(&format!("command-{}", current - 1)));
    let up = draw(vec![key(egui::Key::ArrowUp)], true);
    assert!(text(&up).contains(&format!("command-{}", current - 1)));
    assert!(up.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.rect.width() == GUTTER_WIDTH && rect.fill == egui::Color32::from_rgb(FAILURE.r, FAILURE.g, FAILURE.b))));
    let more = draw(vec![key(egui::Key::ArrowUp)], true);
    assert!(text(&more).contains(&format!("command-{}", current - 2)));
    let next = draw(vec![key(egui::Key::ArrowDown)], true);
    assert!(!text(&next).contains(&format!("command-{}", current - 2)));
    assert!(text(&next).contains(&format!("command-{}", current - 1)));
    draw(vec![key(egui::Key::ArrowUp)], false);
    let restored = draw(Vec::new(), true);
    assert!(text(&restored).contains(&format!("command-{}", current - 1)));
    fixture.services.state.settings.write().keymap_overrides =
        Some(r#"[{"actionId":"terminal-jump-to-previous-command","key":"j","mods":[]}]"#.into());
    let bare_key = |key| Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    };
    let rebound = draw(vec![bare_key(egui::Key::J), Event::Text("j".into())], true);
    assert!(text(&rebound).contains(&format!("command-{}", current - 2)));
    assert_eq!(rebound.platform_output.num_completed_passes, 2);
    fixture.services.state.settings.write().keymap_overrides = Some(
        r#"[{"actionId":"terminal-jump-to-previous-command","key":"u","mods":["mod"],"chord":{"key":"j","mods":[]}},{"actionId":"terminal-jump-to-next-command","key":"u","mods":["mod"],"chord":{"key":"l","mods":[]}}]"#.into(),
    );
    let prefix = draw(vec![key(egui::Key::U)], true);
    assert!(text(&prefix).contains(&format!("command-{}", current - 2)));
    let sibling = draw(vec![bare_key(egui::Key::L), Event::Text("l".into())], true);
    assert!(text(&sibling).contains(&format!("command-{}", current - 1)));
    draw(vec![key(egui::Key::U)], true);
    let unmatched = draw(vec![bare_key(egui::Key::Q), Event::Text("q".into())], true);
    assert!(text(&unmatched).contains(&format!("command-{}", current - 1)));
    assert_eq!(
        session
            .snapshot(|snapshot| snapshot.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        epoch
    );
    let input = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("continue\n"),
            BYTES,
        )
        .unwrap();
    let InputResult::Write(receipt) = input else {
        panic!("expected fixture input")
    };
    receipt.wait().await.unwrap();
    output.wait_title("native-finished").await;
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.snapshot(|snapshot| snapshot.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn terminal_links는_실제_hover_click과_승인_host_수명과_pty회수를_연결한다() {
    use eframe::egui::{self, Event, Modifiers, Rect, pos2, vec2};
    use taide_model::{
        error::AppResult,
        ids::{PaneId, TabId},
        layout::{PaneNode, Tab, TabKind},
    };
    use taide_native_app::{
        host::{HostBridge, HostCommand, HostReply, Terminals},
        terminal_surface::{Request, Views},
        terminal_tabs::{MenuTarget, Tabs},
    };
    use taide_runtime::{PlatformServices, PlatformServicesState};
    const SCREEN: [f32; 2] = [640.0, 320.0];
    const CELL_CENTER: f32 = 0.5;
    const FRAME_STEP: f64 = 0.2;
    const OSC_URI: &str = "https://example.com/target";
    const PLAIN_URI: &str = "https://example.com/plain";
    #[derive(Default)]
    struct Platform(Mutex<Vec<String>>);
    impl PlatformServices for Platform {
        fn open_path(&self, _: &std::path::Path) -> AppResult<()> {
            panic!("unexpected file opener")
        }
        fn reveal_item_in_dir(&self, _: &std::path::Path) -> AppResult<()> {
            panic!("unexpected reveal")
        }
        fn open_url(&self, url: &str) -> AppResult<()> {
            self.0.lock().unwrap().push(url.into());
            Ok(())
        }
        fn send_notification(&self, _: &str, _: &str) -> AppResult<()> {
            panic!("unexpected notification")
        }
    }
    let mut fixture = Fixture::new();
    let platform = Arc::new(Platform::default());
    Arc::get_mut(&mut fixture.services).unwrap().platform =
        PlatformServicesState::new(platform.clone());
    let tabs = Arc::new(Tabs::new(fixture.services.clone(), limits()).unwrap());
    let hub = tabs.hub().clone();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_LINKS".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let pane = PaneId::new();
    let tab = TabId::new();
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = pane.clone();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: TabKind::Terminal {
                session_id: id.clone(),
                cwd: None,
            },
            title: "synthetic links".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(tab.clone()),
    };
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout.clone());
    let source = MenuTarget {
        project: fixture.project.clone(),
        pane: pane.clone(),
        tab: tab.clone(),
        session: id.clone(),
    };
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let mut time = 0.0;
    let mut draw =
        |views: &mut Views, commands: &mut Vec<HostCommand>, events: Vec<Event>, enabled: bool| {
            time += FRAME_STEP;
            let mut rendered = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        pos2(0.0, 0.0),
                        vec2(SCREEN[0], SCREEN[1]),
                    )),
                    events,
                    time: Some(time),
                    ..Default::default()
                },
                |ui| {
                    if !enabled {
                        ui.disable();
                    }
                    views
                        .show(
                            ui,
                            Request {
                                pane: &pane,
                                tab: &tab,
                                session_id: &id,
                                hub: &hub,
                                services: &fixture.services,
                                appearance: &appearance,
                                locale: &locale,
                                request_focus: true,
                                commands,
                            },
                        )
                        .unwrap();
                    views
                        .finish_frame(&hub, &fixture.services, ui.ctx())
                        .unwrap();
                },
            );
            rendered.textures_delta.clear();
            rendered
        };
    draw(&mut views, &mut commands, Vec::new(), true);
    commands.clear();
    let (width, height) = context.fonts_mut(|fonts| {
        (
            fonts.glyph_width(&appearance.font, 'M'),
            fonts.row_height(&appearance.font),
        )
    });
    let positions = [
        pos2(width * CELL_CENTER, height * CELL_CENTER),
        pos2(width * CELL_CENTER, height * (1.0 + CELL_CENTER)),
    ];
    let modifier = if cfg!(target_os = "macos") {
        Modifiers::MAC_CMD | Modifiers::COMMAND
    } else {
        Modifiers::CTRL | Modifiers::COMMAND
    };
    let click = |position, pressed, modifiers| Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    let mut accepted = Vec::new();
    for (position, modifiers, expected, enabled) in [
        (positions[0], Modifiers::NONE, None, true),
        (positions[0], Modifiers::ALT, Some(OSC_URI), true),
        (positions[1], modifier, Some(PLAIN_URI), true),
        (positions[0], Modifiers::ALT, None, false),
    ] {
        let rendered = draw(
            &mut views,
            &mut commands,
            vec![Event::PointerMoved(position)],
            enabled,
        );
        if enabled {
            assert_eq!(
                rendered.platform_output.cursor_icon,
                egui::CursorIcon::PointingHand
            )
        }
        draw(
            &mut views,
            &mut commands,
            vec![click(position, true, modifiers)],
            enabled,
        );
        draw(
            &mut views,
            &mut commands,
            vec![click(position, false, modifiers)],
            enabled,
        );
        let links = std::mem::take(&mut commands)
            .into_iter()
            .filter(|command| matches!(command, HostCommand::OpenTerminalUrl { .. }))
            .collect::<Vec<_>>();
        assert_eq!(links.len(), usize::from(expected.is_some()));
        if let Some(expected) = expected {
            let HostCommand::OpenTerminalUrl { uri, .. } = &links[0] else {
                unreachable!()
            };
            assert_eq!(uri, expected);
            accepted.extend(links);
        }
    }
    draw(
        &mut views,
        &mut commands,
        vec![
            Event::PointerMoved(positions[0]),
            click(positions[0], true, Modifiers::ALT),
        ],
        true,
    );
    draw(
        &mut views,
        &mut commands,
        vec![
            Event::PointerMoved(positions[1]),
            click(positions[1], false, Modifiers::ALT),
        ],
        true,
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, HostCommand::OpenTerminalUrl { .. }))
    );
    draw(
        &mut views,
        &mut commands,
        vec![
            Event::PointerMoved(positions[0]),
            click(positions[0], true, Modifiers::ALT),
            Event::PointerMoved(positions[1]),
            click(positions[1], false, Modifiers::ALT),
        ],
        true,
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, HostCommand::OpenTerminalUrl { .. }))
    );
    let owner = views
        .paste_target(egui::ViewportId::ROOT, &pane, &tab)
        .unwrap();
    let signal = Arc::new(Notify::new());
    let wake = signal.clone();
    let mut host = HostBridge::connect_with_clipboard_ports(
        fixture.services.clone(),
        Arc::new(move || wake.notify_one()),
        Arc::new(|_| panic!("unexpected clipboard write")),
        Arc::new(|| panic!("unexpected clipboard read")),
        Some(Terminals {
            tabs: tabs.clone(),
            environment: Arc::new(|_| Box::pin(async { Vec::new() })),
        }),
    )
    .unwrap();
    for command in accepted {
        host.submit(command).unwrap();
        let result = timeout(TIMEOUT, async {
            loop {
                if let Some(reply) = host.poll() {
                    break reply;
                }
                signal.notified().await
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            result,
            HostReply::TerminalUrlOpened { result: Ok(()) }
        ));
    }
    assert_eq!(*platform.0.lock().unwrap(), [OSC_URI, PLAIN_URI]);
    for uri in [
        "file:///synthetic.txt",
        "javascript:alert(1)",
        "https://trusted.example@evil.example/",
        "https://example.com/\u{202e}spoof",
    ] {
        host.submit(HostCommand::OpenTerminalUrl {
            owner: owner.clone(),
            source: source.clone(),
            uri: uri.into(),
        })
        .unwrap();
        let result = timeout(TIMEOUT, async {
            loop {
                if let Some(reply) = host.poll() {
                    break reply;
                }
                signal.notified().await
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            result,
            HostReply::TerminalUrlOpened { result: Err(_) }
        ));
    }
    if let PaneNode::Leaf { active, .. } = &mut fixture
        .services
        .state
        .layouts
        .write()
        .get_mut(&fixture.project)
        .unwrap()
        .root
    {
        *active = None
    }
    host.submit(HostCommand::OpenTerminalUrl {
        owner: owner.clone(),
        source: source.clone(),
        uri: OSC_URI.into(),
    })
    .unwrap();
    let result = timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = host.poll() {
                break reply;
            }
            signal.notified().await
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        result,
        HostReply::TerminalUrlOpened { result: Err(_) }
    ));
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let mut rendered = context.run_ui(Default::default(), |ui| {
        views
            .finish_frame(&hub, &fixture.services, ui.ctx())
            .unwrap()
    });
    rendered.textures_delta.clear();
    host.submit(HostCommand::OpenTerminalUrl {
        owner,
        source,
        uri: OSC_URI.into(),
    })
    .unwrap();
    let result = timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = host.poll() {
                break reply;
            }
            signal.notified().await
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        result,
        HostReply::TerminalUrlOpened { result: Err(_) }
    ));
    assert_eq!(platform.0.lock().unwrap().len(), 2);
    let InputResult::Write(receipt) = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("continue\n"),
            BYTES,
        )
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap();
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.snapshot(|state| state.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(tabs);
    drop(hub);
    fixture.finish().await;
}

#[tokio::test]
async fn file_link는_실제_pty_worker_현재project_preview_tab과_utf16_reveal을_연결한다() {
    use eframe::egui::{self, Event, Modifiers, Rect, pos2, vec2};
    use taide_model::ids::{PaneId, TabId};
    use taide_model::layout::{PaneNode, SplitDir, Tab, TabKind};
    use taide_native_app::host::{HostBridge, HostCommand, HostReply, Terminals};
    use taide_native_app::terminal_surface::{Request, Views};
    use taide_native_app::terminal_tabs::Tabs;
    use taide_native_editor::store::{EditorLimits, EditorStore};
    use taide_native_editor::view::{SelectionSet, ViewKey};
    use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};
    const SCREEN: [f32; 2] = [640.0, 320.0];
    const FRAME_STEP: f64 = 0.2;
    const CELL_CENTER: f32 = 0.5;
    const FILE_START: f32 = 5.0;
    const GHOST_START: f32 = 28.0;
    const FONT_SIZE: f32 = 14.0;
    const LINE_HEIGHT: f32 = 20.0;
    const PADDING: f32 = 8.0;
    const VIEW_LIMIT: usize = 4;
    const DOC_LIMIT: usize = 2;
    const UNDO_LIMIT: usize = 8;
    struct SyntheticDirectory(std::path::PathBuf);
    impl Drop for SyntheticDirectory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    async fn next_reply(host: &mut HostBridge, ready: &Notify) -> HostReply {
        timeout(TIMEOUT, async {
            loop {
                if let Some(reply) = host.poll() {
                    return reply;
                }
                ready.notified().await;
            }
        })
        .await
        .unwrap()
    }
    let fixture = Fixture::new();
    let directory = SyntheticDirectory(
        std::env::temp_dir().join(format!("taide-native-file-links-{}", ProjectId::new())),
    );
    let root = directory.0.join("root");
    let other_root = directory.0.join("other");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&other_root).unwrap();
    let file_text = "zero\r\n𐐀e\u{301}漢\r\nlast";
    let path = other_root.join("editor.rs");
    std::fs::write(&path, file_text).unwrap();
    let path = path.canonicalize().unwrap().to_str().unwrap().to_owned();
    fixture
        .services
        .state
        .projects
        .write()
        .get_mut(&fixture.project)
        .unwrap()
        .root = root.to_str().unwrap().into();
    let other_project = ProjectId::new();
    fixture.services.state.projects.write().insert(
        other_project.clone(),
        Project {
            id: other_project.clone(),
            root: other_root.to_str().unwrap().into(),
            name: "other synthetic project".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let other_layout = taide_layout::service::default_layout();
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(other_project.clone(), other_layout.clone());
    let tabs = Arc::new(Tabs::new(fixture.services.clone(), limits()).unwrap());
    let hub = tabs.hub().clone();
    let output = Output::default();
    let mut opts = fixture.opts();
    opts.cwd = root.to_str().unwrap().into();
    let id = hub
        .spawn(
            opts,
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_FILE_LINKS".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let pane = PaneId::new();
    let target_pane = PaneId::new();
    let tab = TabId::new();
    let target_tab = TabId::new();
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = target_pane.clone();
    layout.root = PaneNode::Split {
        id: PaneId::new(),
        dir: SplitDir::Horizontal,
        sizes: vec![CELL_CENTER, CELL_CENTER],
        children: vec![
            PaneNode::Leaf {
                id: pane.clone(),
                tabs: vec![Tab {
                    id: tab.clone(),
                    kind: TabKind::Terminal {
                        session_id: id.clone(),
                        cwd: None,
                    },
                    title: "synthetic".into(),
                    pinned: false,
                    preview: false,
                    dirty: false,
                    view_state: None,
                }],
                active: Some(tab.clone()),
            },
            PaneNode::Leaf {
                id: target_pane.clone(),
                tabs: vec![Tab {
                    id: target_tab.clone(),
                    kind: TabKind::File { path: path.clone() },
                    title: "existing".into(),
                    pinned: false,
                    preview: false,
                    dirty: false,
                    view_state: None,
                }],
                active: Some(target_tab.clone()),
            },
        ],
    };
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let mut time = 0.0;
    let mut draw = |views: &mut Views, commands: &mut Vec<HostCommand>, events: Vec<Event>| {
        time += FRAME_STEP;
        let mut rendered = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events,
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                views
                    .show(
                        ui,
                        Request {
                            pane: &pane,
                            tab: &tab,
                            session_id: &id,
                            hub: &hub,
                            services: &fixture.services,
                            appearance: &appearance,
                            locale: &locale,
                            request_focus: true,
                            commands,
                        },
                    )
                    .unwrap();
                views
                    .finish_frame(&hub, &fixture.services, ui.ctx())
                    .unwrap();
            },
        );
        rendered.textures_delta.clear();
        rendered
    };
    draw(&mut views, &mut commands, Vec::new());
    commands.clear();
    let (width, height) = context.fonts_mut(|fonts| {
        (
            fonts.glyph_width(&appearance.font, 'M'),
            fonts.row_height(&appearance.font),
        )
    });
    let point = pos2(width * (FILE_START + CELL_CENTER), height * CELL_CENTER);
    let ghost_point = pos2(width * (GHOST_START + CELL_CENTER), height * CELL_CENTER);
    draw(&mut views, &mut commands, vec![Event::PointerMoved(point)]);
    let resolution = std::mem::take(&mut commands)
        .into_iter()
        .find(|command| matches!(command, HostCommand::ResolveTerminalFileLinks { .. }))
        .unwrap();
    let HostCommand::ResolveTerminalFileLinks {
        owner,
        source,
        request,
    } = &resolution
    else {
        unreachable!()
    };
    let owner = owner.clone();
    let source = source.clone();
    let request = request.clone();
    assert_eq!(request.candidates(), ["../other/editor.rs", "ghost.rs"]);
    let signal = Arc::new(Notify::new());
    let wake = signal.clone();
    let mut host = HostBridge::connect_with_clipboard_ports(
        fixture.services.clone(),
        Arc::new(move || wake.notify_one()),
        Arc::new(|_| panic!("unexpected clipboard write")),
        Arc::new(|| panic!("unexpected clipboard read")),
        Some(Terminals {
            tabs: tabs.clone(),
            environment: Arc::new(|_| Box::pin(async { Vec::new() })),
        }),
    )
    .unwrap();
    host.submit(resolution).unwrap();
    let HostReply::TerminalFileLinks {
        owner: reply_owner,
        request: reply_request,
        result,
    } = next_reply(&mut host, &signal).await
    else {
        panic!("missing resolution")
    };
    let resolved = result.unwrap();
    assert_eq!(resolved, [Some(path.clone()), None]);
    assert!(views.accept_file_links(&reply_owner, &reply_request, Ok(resolved.clone())));
    let rendered = draw(&mut views, &mut commands, vec![Event::PointerMoved(point)]);
    assert_eq!(
        rendered.platform_output.cursor_icon,
        egui::CursorIcon::PointingHand
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, HostCommand::ResolveTerminalFileLinks { .. }))
    );
    std::fs::write(root.join("ghost.rs"), "created after the cached null").unwrap();
    let rendered = draw(
        &mut views,
        &mut commands,
        vec![Event::PointerMoved(ghost_point)],
    );
    assert_ne!(
        rendered.platform_output.cursor_icon,
        egui::CursorIcon::PointingHand
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, HostCommand::ResolveTerminalFileLinks { .. }))
    );
    let click = |pressed, modifiers| Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    };
    draw(
        &mut views,
        &mut commands,
        vec![
            Event::PointerMoved(point),
            click(true, Modifiers::NONE),
            click(false, Modifiers::NONE),
        ],
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, HostCommand::OpenTerminalFile { .. }))
    );
    draw(
        &mut views,
        &mut commands,
        vec![click(true, Modifiers::ALT), click(false, Modifiers::ALT)],
    );
    let command = std::mem::take(&mut commands)
        .into_iter()
        .find(|command| matches!(command, HostCommand::OpenTerminalFile { .. }))
        .unwrap();
    let HostCommand::OpenTerminalFile { link, .. } = &command else {
        unreachable!()
    };
    let link = link.clone();
    host.submit(command).unwrap();
    let HostReply::TerminalFileOpened { result } = next_reply(&mut host, &signal).await else {
        panic!("missing file open")
    };
    let opened = result.unwrap();
    assert_eq!(opened.project, fixture.project);
    assert_eq!(opened.pane, target_pane);
    assert_eq!(opened.tab, target_tab);
    assert_eq!((opened.line, opened.column), (2.0, 3.0));
    assert_eq!(opened.path, path);
    assert_eq!(
        fixture.services.state.layouts.read()[&other_project],
        other_layout
    );
    let mut reveals = taide_native_app::editor_reveal::Reveals::default();
    assert!(reveals.queue(
        &opened,
        &fixture.services.state.layouts.read(),
        std::time::Instant::now()
    ));
    for (name, preview) in [("permanent.rs", false), ("preview.rs", true)] {
        let new_path = root.join(name);
        std::fs::write(&new_path, file_text).unwrap();
        fixture.services.state.settings.write().enable_preview_tabs = preview;
        let mut request_link = link.clone();
        request_link.path = new_path.canonicalize().unwrap().to_str().unwrap().into();
        host.submit(HostCommand::OpenTerminalFile {
            owner: owner.clone(),
            source: source.clone(),
            link: request_link,
        })
        .unwrap();
        let HostReply::TerminalFileOpened { result } = next_reply(&mut host, &signal).await else {
            panic!("missing preview open")
        };
        let opened = result.unwrap();
        let PaneNode::Leaf { tabs, .. } =
            taide_layout::service::find_leaf(&opened.layout.root, &target_pane).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(
            tabs.iter()
                .find(|tab| tab.id == opened.tab)
                .unwrap()
                .preview,
            preview
        );
    }
    let outside = directory.0.join("outside.rs");
    std::fs::write(&outside, "not an approved project").unwrap();
    let mut outside_link = link.clone();
    outside_link.path = outside.to_str().unwrap().into();
    let before = fixture.services.state.layouts.read().clone();
    host.submit(HostCommand::OpenTerminalFile {
        owner: owner.clone(),
        source: source.clone(),
        link: outside_link,
    })
    .unwrap();
    assert!(matches!(
        next_reply(&mut host, &signal).await,
        HostReply::TerminalFileOpened { result: Err(_) }
    ));
    assert_eq!(*fixture.services.state.layouts.read(), before);
    let mut hidden = context.run_ui(Default::default(), |ui| {
        views
            .finish_frame(&hub, &fixture.services, ui.ctx())
            .unwrap();
    });
    hidden.textures_delta.clear();
    assert!(!views.accept_file_links(&reply_owner, &reply_request, Ok(resolved)));
    host.submit(HostCommand::OpenTerminalFile {
        owner: owner.clone(),
        source: source.clone(),
        link: link.clone(),
    })
    .unwrap();
    assert!(matches!(
        next_reply(&mut host, &signal).await,
        HostReply::TerminalFileOpened { result: Err(_) }
    ));
    host.submit(HostCommand::ResolveTerminalFileLinks {
        owner,
        source,
        request,
    })
    .unwrap();
    assert!(matches!(
        next_reply(&mut host, &signal).await,
        HostReply::TerminalFileLinks { result: Err(_), .. }
    ));
    assert_eq!(*fixture.services.state.layouts.read(), before);
    host.submit(HostCommand::OpenDocument(path.clone()))
        .unwrap();
    let HostReply::Opened {
        project, result, ..
    } = next_reply(&mut host, &signal).await
    else {
        panic!("missing document admission")
    };
    assert_eq!(project, Some(other_project));
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOC_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: UNDO_LIMIT,
        max_document_bytes: BYTES,
    })
    .unwrap();
    let document = result.unwrap().commit(&mut store).unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: target_pane,
                tab: target_tab.clone(),
            },
            document,
        )
        .unwrap();
    let other = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    let editor = NativeEditor {
        appearance: EditorAppearance {
            font: egui::FontId::monospace(FONT_SIZE),
            line_height: LINE_HEIGHT,
            horizontal_padding: PADDING,
            background: egui::Color32::BLACK,
            foreground: egui::Color32::WHITE,
            muted: egui::Color32::GRAY,
            selection: egui::Color32::BLUE,
            cursor: egui::Color32::WHITE,
            current_line: egui::Color32::DARK_GRAY,
            line_numbers: true,
            indent: "    ".into(),
        },
    };
    let mut rendered = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        },
        |ui| {
            let position = reveals
                .consume(
                    &target_tab,
                    &path,
                    ui.ctx().viewport_id(),
                    std::time::Instant::now(),
                )
                .unwrap();
            editor
                .reveal(ui, &mut store, view, position.line, position.column)
                .unwrap();
            let shown = editor.show(ui, &mut store, view, true).unwrap();
            assert!(shown.response.has_focus());
            assert!(!shown.changed);
            assert!(shown.errors.is_empty());
        },
    );
    rendered.textures_delta.clear();
    assert_eq!(
        store.views().get(view).unwrap().selection.selections[0].head,
        "zero\r\n𐐀".len()
    );
    assert_eq!(
        store.views().get(other).unwrap().selection,
        SelectionSet::default()
    );
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        file_text
    );
    assert!(!store.documents().snapshot(document).unwrap().dirty);
    let InputResult::Write(receipt) = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("continue\n"),
            BYTES,
        )
        .unwrap()
    else {
        panic!("fixture input not encoded")
    };
    timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap();
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.snapshot(|state| state.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(tabs);
    drop(hub);
    fixture.finish().await;
}

#[tokio::test]
async fn clear_session은_실제_pty에_명령을_보내지_않고_같은_core로_계속_입력하고_join한다() {
    use taide_native_terminal::GridDimensions;
    const INPUT_BYTES: usize = 64 * 1024;
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let previous = session
        .snapshot(|state| {
            let grid = state.core.grid().unwrap();
            (
                state.revision,
                state.core.selection_stamp().unwrap(),
                grid.cursor.clone(),
            )
        })
        .unwrap();
    assert!(previous.2.point.line > taide_native_terminal::Line(0));
    assert!(session.clear_current_row().unwrap());
    session
        .snapshot(|state| {
            let grid = state.core.grid().unwrap();
            let stamp = state.core.selection_stamp().unwrap();
            assert_eq!(state.phase, Phase::Running);
            assert_eq!(state.revision, previous.0);
            assert_eq!(stamp.input_epoch, previous.1.input_epoch);
            assert_ne!(stamp.buffer_epoch, previous.1.buffer_epoch);
            assert_eq!(grid.history_size(), 0);
            assert_eq!(grid.cursor.point.line, taide_native_terminal::Line(0));
            assert_eq!(grid.cursor.point.column, previous.2.point.column);
        })
        .unwrap();
    assert!(!session.clear_current_row().unwrap());
    let InputResult::Write(receipt) = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("continue\n"),
            INPUT_BYTES,
        )
        .unwrap()
    else {
        panic!("clear continuation input was not admitted");
    };
    receipt.wait().await.unwrap();
    let completed = timeout(TIMEOUT, session.wait_dispatch()).await;
    let (phase, epoch) = session
        .snapshot(|state| {
            (
                state.phase,
                state.core.selection_stamp().unwrap().input_epoch,
            )
        })
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(matches!(completed, Ok(Ok(()))));
    assert_eq!(phase, Phase::Exited(Some(0)));
    assert_eq!(epoch, previous.1.input_epoch + 1);
}

#[tokio::test]
async fn clipboard_paste는_빈내용_상한_숨김_수명과_active_identity를_실제_pty에서_보존한다() {
    use eframe::egui::{self, Rect, pos2, vec2};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use taide_model::{
        ids::{PaneId, TabId},
        layout::{PaneNode, Tab, TabKind},
    };
    use taide_native_app::{
        host::{HostBridge, HostCommand, HostReply},
        terminal_surface::{Request, Views},
    };
    const SCREEN: [f32; 2] = [640.0, 320.0];
    const OVER_BUDGET: usize = 64 * 1024 + 1;
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let pane = PaneId::new();
    let tab = TabId::new();
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = pane.clone();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: TabKind::Terminal {
                session_id: id.clone(),
                cwd: None,
            },
            title: "synthetic terminal".into(),
            preview: false,
            pinned: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(tab.clone()),
    };
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout.clone());
    let context = egui::Context::default();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut views = Views::default();
    let mut commands = Vec::new();
    let draw = |views: &mut Views, commands: &mut Vec<HostCommand>| {
        let mut rendered = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                ..Default::default()
            },
            |ui| {
                views
                    .show(
                        ui,
                        Request {
                            pane: &pane,
                            tab: &tab,
                            session_id: &id,
                            hub: &hub,
                            services: &fixture.services,
                            appearance: &appearance,
                            locale: &locale,
                            request_focus: true,
                            commands,
                        },
                    )
                    .unwrap();
                views
                    .finish_frame(&hub, &fixture.services, ui.ctx())
                    .unwrap();
            },
        );
        rendered.textures_delta.clear();
    };
    draw(&mut views, &mut commands);
    let first = views
        .paste_target(egui::ViewportId::ROOT, &pane, &tab)
        .unwrap();
    let reads = Arc::new(AtomicUsize::new(0));
    let counter = reads.clone();
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect_with_clipboard_ports(
        fixture.services.clone(),
        Arc::new(move || signal.notify_one()),
        Arc::new(|_| panic!("paste must not write OS clipboard")),
        Arc::new(move || match counter.fetch_add(1, Ordering::SeqCst) {
            0 => Ok(String::new()),
            1 => Ok("x".repeat(OVER_BUDGET)),
            2 => Ok("continue\n".into()),
            _ => panic!("unexpected clipboard read"),
        }),
        None,
    )
    .unwrap();
    for oversized in [false, true] {
        host.submit(HostCommand::ReadTerminalClipboard(first.clone()))
            .unwrap();
        let (target, result) = timeout(TIMEOUT, async {
            loop {
                if let Some(HostReply::TerminalClipboard { target, result }) = host.poll() {
                    return (target, result);
                }
                ready.notified().await;
            }
        })
        .await
        .unwrap();
        if oversized {
            assert!(result.is_err());
        } else {
            assert!(
                views
                    .accept_paste(&target, &result.unwrap(), &hub, &fixture.services)
                    .unwrap()
            );
        }
    }
    assert_eq!(
        session
            .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        epoch + 1
    );
    let mut inactive = layout.clone();
    let PaneNode::Leaf { active, .. } = &mut inactive.root else {
        panic!("fixture root must be a leaf")
    };
    *active = None;
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), inactive);
    assert!(
        !views
            .accept_paste(&first, "poison", &hub, &fixture.services)
            .unwrap()
    );
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let mut hidden = context.run_ui(egui::RawInput::default(), |ui| {
        ui.label("another tab");
        views
            .finish_frame(&hub, &fixture.services, ui.ctx())
            .unwrap();
    });
    hidden.textures_delta.clear();
    assert!(
        !views
            .accept_paste(&first, "poison", &hub, &fixture.services)
            .unwrap()
    );
    host.submit(HostCommand::ReadTerminalClipboard(first))
        .unwrap();
    timeout(TIMEOUT, async {
        loop {
            if let Some(HostReply::TerminalClipboard { result, .. }) = host.poll() {
                assert!(result.is_err());
                break;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    draw(&mut views, &mut commands);
    let current = views
        .paste_target(egui::ViewportId::ROOT, &pane, &tab)
        .unwrap();
    host.submit(HostCommand::ReadTerminalClipboard(current))
        .unwrap();
    let accepted = timeout(TIMEOUT, async {
        loop {
            if let Some(HostReply::TerminalClipboard { target, result }) = host.poll() {
                break views
                    .accept_paste(&target, &result.unwrap(), &hub, &fixture.services)
                    .unwrap();
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap();
    let completed = timeout(TIMEOUT, session.wait_dispatch()).await;
    let phase = session.snapshot(|state| state.phase).unwrap();
    hub.close(&id).await.unwrap();
    timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(accepted);
    assert!(matches!(completed, Ok(Ok(()))));
    assert_eq!(phase, Phase::Exited(Some(0)));
    assert_eq!(reads.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn terminal_menu는_실제_secondary_popup과_원자적_split_new_kill의_소유권을_보존한다() {
    verify_terminal_menu_host(false).await;
}

#[tokio::test]
async fn terminal_menu는_실제_keyboard_split_new_kill을_host_worker에_전달한다() {
    verify_terminal_menu_host(true).await;
}

async fn verify_terminal_menu_host(is_keyboard: bool) {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_model::{
        ids::PaneId,
        layout::{DropEdge, PaneNode, TabKind},
    };
    use taide_native_app::{
        host::{HostBridge, HostCommand, HostReply, Terminals},
        terminal_surface::{Request, Views},
        terminal_tabs::{MenuOperation, MenuTarget, Tabs},
    };
    const SCREEN: [f32; 2] = [640.0, 320.0];
    let fixture = Fixture::new();
    fixture.services.state.settings.write().shell_override = fixture.opts().shell;
    let mut layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    let tab = taide_native_app::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(tab.kind, TabKind::Terminal { .. }))
        .unwrap()
        .id
        .clone();
    let PaneNode::Leaf { active, .. } = &mut layout.root else {
        panic!("fixture root must be a leaf")
    };
    *active = Some(tab.clone());
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let native = Arc::new(Tabs::new(fixture.services.clone(), limits()).unwrap());
    let output = Output::default();
    let id = native
        .attach(
            tab.clone(),
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    output.wait_ready().await;
    let session = native.hub().get(&id).unwrap();
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let context = egui::Context::default();
    if is_keyboard {
        context.enable_accesskit();
    }
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut views = Views::default();
    let mut commands = Vec::new();
    let response_id = std::cell::Cell::new(None);
    let frame = |events: Vec<Event>, views: &mut Views, commands: &mut Vec<HostCommand>| {
        let mut raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            ..Default::default()
        };
        views.raw_input(&context, &mut raw);
        let mut rendered = context.run_ui(raw, |ui| {
            response_id.set(Some(
                views
                    .show(
                        ui,
                        Request {
                            pane: &pane,
                            tab: &tab,
                            session_id: &id,
                            hub: native.hub(),
                            services: &fixture.services,
                            appearance: &appearance,
                            locale: &locale,
                            request_focus: false,
                            commands,
                        },
                    )
                    .unwrap()
                    .id,
            ));
            views
                .finish_frame(native.hub(), &fixture.services, ui.ctx())
                .unwrap();
        });
        rendered.textures_delta.clear();
        rendered
    };
    let initial = frame(Vec::new(), &mut views, &mut commands);
    drop(initial);
    let focus = context.memory(|memory| memory.focused());
    if focus.is_none() {
        let mut focused = context.run_ui(egui::RawInput::default(), |ui| {
            views
                .show(
                    ui,
                    Request {
                        pane: &pane,
                        tab: &tab,
                        session_id: &id,
                        hub: native.hub(),
                        services: &fixture.services,
                        appearance: &appearance,
                        locale: &locale,
                        request_focus: true,
                        commands: &mut commands,
                    },
                )
                .unwrap();
        });
        focused.textures_delta.clear();
    }
    let menu_position = context
        .read_response(context.memory(|memory| memory.focused()).unwrap())
        .unwrap()
        .rect
        .center();
    for pressed in [true, false] {
        drop(frame(
            vec![
                Event::PointerMoved(menu_position),
                Event::PointerButton {
                    pos: menu_position,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut views,
            &mut commands,
        ));
    }
    let opened = frame(Vec::new(), &mut views, &mut commands);
    for key in [
        "terminal.copy",
        "terminal.paste",
        "terminal.selectAll",
        "terminal.clear",
        "tab.split",
        "tab.newTerminal",
        "terminal.kill",
    ] {
        assert!(opened.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == key)), "missing menu item {key}");
    }
    drop(opened);
    drop(frame(
        vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
        &mut views,
        &mut commands,
    ));
    drop(frame(Vec::new(), &mut views, &mut commands));
    assert_eq!(context.memory(|memory| memory.focused()), response_id.get());
    assert_eq!(
        session
            .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        epoch
    );
    let target = MenuTarget {
        project: fixture.project.clone(),
        pane: pane.clone(),
        tab: tab.clone(),
        session: id.clone(),
    };
    assert!(
        commands
            .iter()
            .all(|command| matches!(command, HostCommand::ResizeTerminal { .. }))
    );
    commands.clear();
    let signal = Arc::new(Notify::new());
    let wake = signal.clone();
    let mut host = is_keyboard.then(|| {
        HostBridge::connect_with_clipboard_ports(
            fixture.services.clone(),
            Arc::new(move || wake.notify_one()),
            Arc::new(|_| panic!("menu host must not access clipboard")),
            Arc::new(|| panic!("menu host must not access clipboard")),
            Some(Terminals {
                tabs: native.clone(),
                environment: Arc::new(|_| Box::pin(async { Vec::new() })),
            }),
        )
        .unwrap()
    });
    let mut keyboard_closes = Vec::new();
    let mut keyboard_targets = Vec::new();
    let mut keyboard_edges = Vec::new();
    let mut choose = |root_label: &str, child_label: Option<&str>, key| {
        let terminal_id = context.memory(|memory| memory.focused()).unwrap();
        let position = context.read_response(terminal_id).unwrap().rect.center();
        for pressed in [true, false] {
            drop(frame(
                vec![Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
                &mut views,
                &mut commands,
            ));
        }
        let focus_event = |label: &str, shown: &egui::FullOutput| {
            let target = shown
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(label))
                .unwrap()
                .0;
            Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_node: target,
                target_tree: egui::accesskit::TreeId::ROOT,
                data: None,
            })
        };
        let shown = frame(Vec::new(), &mut views, &mut commands);
        let focus = focus_event(root_label, &shown);
        drop(frame(vec![focus], &mut views, &mut commands));
        if let Some(label) = child_label {
            drop(frame(
                [true, false]
                    .map(|pressed| Event::Key {
                        key: egui::Key::ArrowRight,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                    .into(),
                &mut views,
                &mut commands,
            ));
            let shown = frame(Vec::new(), &mut views, &mut commands);
            let focus = focus_event(label, &shown);
            drop(frame(vec![focus], &mut views, &mut commands));
        }
        drop(frame(
            vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut views,
            &mut commands,
        ));
        let terminal_focus = context.memory(|memory| memory.focused());
        let shown = frame(
            vec![Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut views,
            &mut commands,
        );
        keyboard_closes.push(
            terminal_focus == Some(terminal_id)
                && !shown
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .any(|(_, node)| node.role() == egui::accesskit::Role::Menu),
        );
        let command = commands
            .pop()
            .expect("actual menu keydown must queue a terminal command");
        assert!(commands.is_empty());
        if let HostCommand::TerminalMenu {
            target,
            operation,
            title,
        } = &command
        {
            keyboard_targets.push(
                target.project == fixture.project
                    && target.pane == pane
                    && target.tab == tab
                    && target.session == id
                    && title
                        == if matches!(operation, MenuOperation::Kill) {
                            ""
                        } else {
                            "terminal.title"
                        },
            );
        } else {
            keyboard_targets.push(false);
        }
        command
    };
    for (edge, label, key) in [
        (DropEdge::Left, "editorArea.splitLeft", egui::Key::Space),
        (DropEdge::Right, "editorArea.splitRight", egui::Key::Enter),
        (DropEdge::Top, "editorArea.splitTop", egui::Key::Space),
        (DropEdge::Bottom, "editorArea.splitBottom", egui::Key::Enter),
    ] {
        let before = fixture.services.state.layouts.read()[&fixture.project].clone();
        if let Some(host) = &mut host {
            let command = choose("tab.split", Some(label), key);
            keyboard_edges.push(matches!(&command, HostCommand::TerminalMenu {
                operation: MenuOperation::Split(actual), ..
            } if *actual == edge));
            host.submit(command).unwrap();
            timeout(TIMEOUT, signal.notified()).await.unwrap();
            assert!(host.poll().is_none());
        } else {
            assert!(
                native
                    .menu(
                        target.clone(),
                        MenuOperation::Split(edge),
                        "terminal".into()
                    )
                    .await
                    .unwrap()
                    .is_none()
            );
        }
        let after = fixture.services.state.layouts.read()[&fixture.project].clone();
        assert_eq!(after.revision, before.revision + 1);
        let new = taide_native_app::tabs::tabs_in(&after.root)
            .into_iter()
            .find(|tab| {
                !taide_native_app::tabs::tabs_in(&before.root)
                    .iter()
                    .any(|old| old.id == tab.id)
            })
            .unwrap();
        assert!(
            matches!(&new.kind, TabKind::Terminal { session_id, cwd: Some(cwd) } if session_id.is_empty() && cwd == env!("CARGO_MANIFEST_DIR"))
        );
        assert!(!new.preview && !new.pinned);
    }
    session
        .metadata()
        .update_cwd("/synthetic-outside-root".into());
    let before = fixture.services.state.layouts.read()[&fixture.project].clone();
    native
        .menu(
            target.clone(),
            MenuOperation::Split(DropEdge::Right),
            "terminal".into(),
        )
        .await
        .unwrap();
    let after = fixture.services.state.layouts.read()[&fixture.project].clone();
    let new = taide_native_app::tabs::tabs_in(&after.root)
        .into_iter()
        .find(|tab| {
            !taide_native_app::tabs::tabs_in(&before.root)
                .iter()
                .any(|old| old.id == tab.id)
        })
        .unwrap();
    assert!(matches!(&new.kind, TabKind::Terminal { cwd: None, .. }));
    if let Some(host) = &mut host {
        host.submit(choose("tab.newTerminal", None, egui::Key::Space))
            .unwrap();
        timeout(TIMEOUT, signal.notified()).await.unwrap();
        assert!(host.poll().is_none());
    } else {
        native
            .menu(target.clone(), MenuOperation::New, "terminal".into())
            .await
            .unwrap();
    }
    let mut current = fixture.services.state.layouts.read()[&fixture.project].clone();
    let PaneNode::Leaf { tabs, active, .. } =
        taide_layout::service::find_leaf(&current.root, &pane).unwrap()
    else {
        panic!("source pane must exist")
    };
    let new = tabs
        .iter()
        .find(|candidate| Some(&candidate.id) == active.as_ref())
        .unwrap();
    assert_ne!(new.id, tab);
    assert!(
        matches!(&new.kind, TabKind::Terminal { cwd: None, session_id } if session_id.is_empty())
    );
    assert!(
        native
            .menu(target.clone(), MenuOperation::Kill, String::new())
            .await
            .is_err()
    );
    assert_eq!(
        fixture.services.state.layouts.read()[&fixture.project],
        current
    );
    let active_tab = new.id.clone();
    taide_layout::service::activate_tab(&mut current, &tab).unwrap();
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), current.clone());
    for stale in [
        MenuTarget {
            session: "replaced".into(),
            ..target.clone()
        },
        MenuTarget {
            pane: PaneId::new(),
            ..target.clone()
        },
        MenuTarget {
            project: ProjectId::new(),
            ..target.clone()
        },
    ] {
        assert!(
            native
                .menu(stale, MenuOperation::Kill, String::new())
                .await
                .is_err()
        );
        assert_eq!(
            fixture.services.state.layouts.read()[&fixture.project],
            current
        );
    }
    let closed = if let Some(host) = &mut host {
        host.submit(choose("terminal.kill", None, egui::Key::Enter))
            .unwrap();
        timeout(TIMEOUT, signal.notified()).await.unwrap();
        let Some(HostReply::Closed { result, .. }) = host.poll() else {
            panic!("actual menu kill must return a Closed reply")
        };
        result.unwrap()
    } else {
        native
            .menu(target, MenuOperation::Kill, String::new())
            .await
            .unwrap()
            .unwrap()
    };
    assert_eq!(closed.tab.id, tab);
    assert!(native.hub().get(&id).is_none());
    assert!(
        taide_native_app::tabs::tabs_in(
            &fixture.services.state.layouts.read()[&fixture.project].root
        )
        .iter()
        .any(|tab| tab.id == active_tab)
    );
    views.cancel_inputs();
    if let Some(host) = host {
        timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
    }
    drop(session);
    drop(native);
    fixture.finish().await;
    if is_keyboard {
        assert_eq!(keyboard_closes, [true, true, true, true, true, true]);
        assert_eq!(keyboard_targets, [true, true, true, true, true, true]);
        assert_eq!(keyboard_edges, [true, true, true, true]);
    }
}

#[tokio::test]
async fn terminal_menu는_원본_shift_f10을_가로채지_않고_앞뒤_문자를_전송한다() {
    verify_terminal_menu_batch(false, false, false, false, None, false, (false, None)).await;
}

#[tokio::test]
async fn terminal_menu는_우클릭_이전_문자를_잃지_않고_이후_문자를_차단한다() {
    verify_terminal_menu_batch(true, false, false, false, None, false, (false, None)).await;
}

#[tokio::test]
async fn terminal_menu는_실제_focus_owner와_press_loss_global_release_restore_wire를_보존한다() {
    for is_mouse in [false, true] {
        verify_terminal_menu_batch(true, true, is_mouse, false, None, false, (false, None)).await;
    }
}

#[tokio::test]
async fn terminal_menu는_실제_ax_focus의_메뉴항목_자식과_닫힌_트리_회수를_보존한다() {
    verify_terminal_menu_batch(true, true, false, true, None, false, (false, None)).await;
}

#[tokio::test]
async fn terminal_menu는_실제_split_ax와_방향키_열기_복귀의_소유권을_보존한다() {
    verify_terminal_menu_batch(
        true,
        true,
        false,
        true,
        Some((eframe::egui::Key::ArrowRight, MENU_SCREEN)),
        false,
        (false, None),
    )
    .await;
}

#[tokio::test]
async fn terminal_menu는_enter_space로_split을_열고_첫_항목에_포커스를_준다() {
    for key in [eframe::egui::Key::Enter, eframe::egui::Key::Space] {
        verify_terminal_menu_batch(
            true,
            true,
            false,
            true,
            Some((key, MENU_SCREEN)),
            false,
            (false, None),
        )
        .await;
    }
}

#[tokio::test]
async fn terminal_menu는_모든_split이_disabled여도_submenu_owner와_복귀를_보존한다() {
    verify_terminal_menu_batch(
        true,
        true,
        false,
        true,
        Some((eframe::egui::Key::ArrowRight, SMALL_MENU_SCREEN)),
        false,
        (false, None),
    )
    .await;
}

#[tokio::test]
async fn terminal_menu는_root_child의_비순환_탐색_disabled_건너뛰기_tab과_modifier를_보존한다() {
    verify_terminal_menu_batch(
        true,
        true,
        false,
        true,
        Some((eframe::egui::Key::ArrowRight, MENU_SCREEN)),
        true,
        (false, None),
    )
    .await;
}

#[tokio::test]
async fn terminal_menu는_실제_label_검색_수명_반복문자와_space_억제를_보존한다() {
    verify_terminal_menu_batch(
        true,
        true,
        false,
        true,
        Some((eframe::egui::Key::ArrowRight, MENU_SCREEN)),
        false,
        (true, None),
    )
    .await;
}

#[tokio::test]
async fn terminal_menu는_같은_batch_첫_검색문자_뒤_space의_leaf_선택을_억제한다() {
    verify_terminal_menu_batch(
        true,
        true,
        false,
        true,
        None,
        false,
        (false, Some(eframe::egui::Key::S)),
    )
    .await;
}

#[tokio::test]
async fn terminal_menu는_같은_batch_방향키의_지연_focus를_즉시_연쇄하지_않는다() {
    verify_terminal_menu_batch(
        true,
        true,
        false,
        true,
        None,
        false,
        (false, Some(eframe::egui::Key::ArrowDown)),
    )
    .await;
}

#[tokio::test]
async fn terminal_menu는_owner_end_enter_선택과_같은_batch_닫힘_뒤_문자_차단을_보존한다() {
    verify_terminal_menu_batch(
        true,
        true,
        false,
        true,
        None,
        false,
        (false, Some(eframe::egui::Key::End)),
    )
    .await;
}

async fn verify_terminal_menu_batch(
    is_pointer: bool,
    is_focus: bool,
    is_mouse: bool,
    check_menu_tree: bool,
    submenu_key: Option<(eframe::egui::Key, [f32; 2])>,
    check_navigation: bool,
    input_checks: (bool, Option<eframe::egui::Key>),
) {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_model::layout::{PaneNode, TabKind};
    use taide_native_app::terminal_surface::{Request, Views};
    let (check_search, mixed_key) = input_checks;
    let check_mixed_input = mixed_key.is_some();
    let check_submenu = submenu_key.is_some();
    let screen = submenu_key.map(|(_, screen)| screen).unwrap_or(MENU_SCREEN);
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: if check_search || check_mixed_input {
            serde_json::from_str(include_str!(
                "../../../crates/taide-locale/resources/locales/en.json"
            ))
            .unwrap()
        } else {
            Default::default()
        },
        warnings: Vec::new(),
    };
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async {
                vec![(
                    "TAIDE_NATIVE_FIXTURE_MENU_KEY".into(),
                    if is_mouse {
                        "4"
                    } else if is_focus {
                        "3"
                    } else if is_pointer {
                        "2"
                    } else {
                        "1"
                    }
                    .into(),
                )]
            },
            output.ports(),
        )
        .await
        .unwrap();
    output.wait_ready().await;
    let session = hub.get(&id).unwrap();
    let epoch_before = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let mut layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    let tab = taide_native_app::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(tab.kind, TabKind::Terminal { .. }))
        .unwrap()
        .id
        .clone();
    let PaneNode::Leaf { active, tabs, .. } = &mut layout.root else {
        panic!("fixture root must be leaf")
    };
    *active = Some(tab.clone());
    let source = tabs
        .iter_mut()
        .find(|candidate| candidate.id == tab)
        .unwrap();
    source.kind = TabKind::Terminal {
        session_id: id.clone(),
        cwd: None,
    };
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let context = egui::Context::default();
    if check_menu_tree {
        context.enable_accesskit();
    }
    let appearance = terminal_appearance();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let mut menu_accessibility = None;
    let mut final_accessibility = None;
    let mut submenu_accessibility = None;
    let mut submenu_closed = None;
    let mut split_target = None;
    let mut navigation = Vec::new();
    let mut owner_selection = None;
    let clock = std::cell::Cell::new(0.0);
    let cancel_search_key = std::cell::Cell::new(false);
    const SEARCH_EXPIRY_ADVANCE: f64 = 1.1;
    let resolved_label = |key: &str| {
        locale
            .messages
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.into())
    };
    let mut frame = |events, request_focus| {
        let mut raw = egui::RawInput {
            events,
            time: (check_search || check_mixed_input).then_some(clock.get()),
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(screen[0], screen[1]),
            )),
            ..Default::default()
        };
        views.raw_input(&context, &mut raw);
        let mut menu_opened = false;
        let mut position = None;
        let mut rendered = context.run_ui(raw, |ui| {
            if cancel_search_key.get() {
                context.input_mut(|input| {
                    input
                        .events
                        .retain(|event| !matches!(event, Event::Key { .. }))
                });
            }
            let response = views
                .show(
                    ui,
                    Request {
                        pane: &pane,
                        tab: &tab,
                        session_id: &id,
                        hub: &hub,
                        services: &fixture.services,
                        appearance: &appearance,
                        locale: &locale,
                        request_focus,
                        commands: &mut commands,
                    },
                )
                .unwrap();
            menu_opened = response.context_menu_opened();
            position = Some(if is_focus {
                response.rect.min + vec2(1.0, 1.0)
            } else {
                response.rect.center()
            });
            views
                .finish_frame(&hub, &fixture.services, &context)
                .unwrap();
        });
        rendered.textures_delta.clear();
        let accessibility = rendered.platform_output.accesskit_update.take();
        if check_menu_tree {
            final_accessibility = accessibility.clone();
            if menu_opened {
                menu_accessibility = final_accessibility.clone();
            }
        }
        (
            menu_opened,
            position.unwrap(),
            context.memory(|memory| memory.focused()),
            accessibility,
        )
    };
    frame(Vec::new(), true);
    let (_, position, terminal_focus, _) = frame(Vec::new(), false);
    let key = |pressed| Event::Key {
        key: egui::Key::F10,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::SHIFT,
    };
    let events = if is_pointer {
        vec![
            Event::Text(if is_focus { "con" } else { "continue\n" }.into()),
            Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            Event::Text("blocked".into()),
        ]
    } else {
        vec![
            Event::Text("con".into()),
            key(true),
            key(false),
            Event::Text("tinue\n".into()),
        ]
    };
    let (menu_opened, _, opened_focus, _) = frame(events, false);
    let mut released_open = menu_opened;
    let mut closed_open = false;
    let mut restored_focus = terminal_focus;
    let mut visible_focus = opened_focus;
    if is_focus {
        if check_menu_tree {
            (_, _, visible_focus, _) = frame(Vec::new(), false);
        }
        (released_open, _, _, _) = frame(
            vec![Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            false,
        );
        if let Some(mixed_key) = mixed_key {
            let tree = frame(Vec::new(), false).3.unwrap();
            let clear = tree
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(resolved_label("terminal.clear").as_str()))
                .unwrap()
                .0;
            if mixed_key != egui::Key::End {
                frame(
                    vec![Event::AccessKitActionRequest(
                        egui::accesskit::ActionRequest {
                            action: egui::accesskit::Action::Focus,
                            target_node: clear,
                            target_tree: egui::accesskit::TreeId::ROOT,
                            data: None,
                        },
                    )],
                    false,
                );
            }
            let keys = if mixed_key == egui::Key::S {
                vec![(egui::Key::S, Some("s")), (egui::Key::Space, Some(" "))]
            } else if mixed_key == egui::Key::End {
                vec![
                    (mixed_key, None),
                    (egui::Key::Enter, None),
                    (egui::Key::T, Some("t")),
                ]
            } else {
                vec![(mixed_key, None), (mixed_key, None)]
            };
            let events = keys
                .into_iter()
                .flat_map(|(key, text)| {
                    let key_event = |pressed| Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    };
                    std::iter::once(key_event(true))
                        .chain(text.map(|text| Event::Text(text.into())))
                        .chain(std::iter::once(key_event(false)))
                })
                .collect();
            let (mixed_open, _, mixed_focus, _) = frame(events, false);
            if mixed_key == egui::Key::End {
                owner_selection = Some((mixed_open, mixed_focus));
                frame(Vec::new(), false);
            } else {
                navigation.push((frame(Vec::new(), false).3, "tab.split"));
            }
        }
        if check_submenu {
            let character = |key, text: &str, modifiers| {
                vec![
                    Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    },
                    Event::Text(text.into()),
                    Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers,
                    },
                ]
            };
            if check_search {
                for (key, text, modifiers, label, reset) in [
                    (
                        egui::Key::C,
                        "c",
                        egui::Modifiers::NONE,
                        "terminal.clear",
                        false,
                    ),
                    (
                        egui::Key::C,
                        "c",
                        egui::Modifiers::NONE,
                        "terminal.clear",
                        false,
                    ),
                    (egui::Key::S, "S", egui::Modifiers::SHIFT, "tab.split", true),
                    (egui::Key::P, "p", egui::Modifiers::NONE, "tab.split", false),
                    (
                        egui::Key::Space,
                        " ",
                        egui::Modifiers::NONE,
                        "tab.split",
                        false,
                    ),
                    (
                        egui::Key::A,
                        "\u{1f600}",
                        egui::Modifiers::NONE,
                        "tab.split",
                        true,
                    ),
                    (egui::Key::P, "p", egui::Modifiers::CTRL, "tab.split", false),
                    (egui::Key::P, "p", egui::Modifiers::ALT, "tab.split", false),
                    (
                        egui::Key::P,
                        "p",
                        egui::Modifiers::NONE,
                        "terminal.paste",
                        true,
                    ),
                    (
                        egui::Key::Space,
                        " ",
                        egui::Modifiers::NONE,
                        "terminal.paste",
                        false,
                    ),
                ] {
                    if reset {
                        clock.set(clock.get() + SEARCH_EXPIRY_ADVANCE);
                    }
                    frame(character(key, text, modifiers), false);
                    navigation.push((frame(Vec::new(), false).3, label));
                }
                for event in [
                    Event::Ime(egui::ImeEvent::Commit("S".into())),
                    Event::Paste("S".into()),
                    Event::Text("S".into()),
                ] {
                    frame(vec![event], false);
                    navigation.push((frame(Vec::new(), false).3, "terminal.paste"));
                }
                clock.set(clock.get() + SEARCH_EXPIRY_ADVANCE);
                cancel_search_key.set(true);
                frame(character(egui::Key::S, "S", egui::Modifiers::NONE), false);
                cancel_search_key.set(false);
                navigation.push((frame(Vec::new(), false).3, "terminal.paste"));
            }
            if check_navigation {
                for (key, modifiers, label) in [
                    (egui::Key::ArrowUp, egui::Modifiers::NONE, "terminal.kill"),
                    (egui::Key::ArrowDown, egui::Modifiers::NONE, "terminal.kill"),
                    (egui::Key::Home, egui::Modifiers::NONE, "terminal.paste"),
                    (egui::Key::ArrowUp, egui::Modifiers::NONE, "terminal.paste"),
                    (
                        egui::Key::ArrowDown,
                        egui::Modifiers::SHIFT,
                        "terminal.paste",
                    ),
                    (egui::Key::PageDown, egui::Modifiers::NONE, "terminal.kill"),
                    (egui::Key::PageUp, egui::Modifiers::NONE, "terminal.paste"),
                    (
                        egui::Key::ArrowDown,
                        egui::Modifiers::NONE,
                        "terminal.selectAll",
                    ),
                    (egui::Key::Tab, egui::Modifiers::NONE, "terminal.selectAll"),
                    (egui::Key::Tab, egui::Modifiers::SHIFT, "terminal.selectAll"),
                    (
                        egui::Key::ArrowRight,
                        egui::Modifiers::NONE,
                        "terminal.selectAll",
                    ),
                    (egui::Key::End, egui::Modifiers::NONE, "terminal.kill"),
                ] {
                    frame(
                        [true, false]
                            .map(|pressed| Event::Key {
                                key,
                                physical_key: None,
                                pressed,
                                repeat: false,
                                modifiers,
                            })
                            .into(),
                        false,
                    );
                    navigation.push((frame(Vec::new(), false).3, label));
                }
            }
            let tree = frame(Vec::new(), false).3.unwrap();
            let trigger = tree
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(resolved_label("tab.split").as_str()))
                .unwrap()
                .0;
            split_target = Some(trigger);
            frame(
                vec![Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::Focus,
                        target_node: trigger,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        data: None,
                    },
                )],
                false,
            );
            let arrow = |key| {
                [true, false]
                    .map(|pressed| Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    })
                    .into()
            };
            frame(arrow(submenu_key.unwrap().0), false);
            submenu_accessibility = frame(Vec::new(), false).3;
            if check_search {
                for label in [
                    "editorArea.splitRight",
                    "editorArea.splitTop",
                    "editorArea.splitBottom",
                    "editorArea.splitLeft",
                ] {
                    frame(character(egui::Key::S, "s", egui::Modifiers::NONE), false);
                    navigation.push((frame(Vec::new(), false).3, label));
                }
                clock.set(clock.get() + SEARCH_EXPIRY_ADVANCE);
                for (key, text) in [
                    (egui::Key::S, "S"),
                    (egui::Key::P, "p"),
                    (egui::Key::L, "l"),
                    (egui::Key::I, "i"),
                    (egui::Key::T, "t"),
                    (egui::Key::Space, " "),
                    (egui::Key::U, "u"),
                ] {
                    frame(character(key, text, egui::Modifiers::NONE), false);
                    frame(Vec::new(), false);
                }
                navigation.push((frame(Vec::new(), false).3, "editorArea.splitTop"));
            }
            if check_navigation {
                for (key, modifiers, label) in [
                    (
                        egui::Key::ArrowUp,
                        egui::Modifiers::NONE,
                        "editorArea.splitLeft",
                    ),
                    (
                        egui::Key::ArrowDown,
                        egui::Modifiers::NONE,
                        "editorArea.splitRight",
                    ),
                    (
                        egui::Key::End,
                        egui::Modifiers::NONE,
                        "editorArea.splitBottom",
                    ),
                    (
                        egui::Key::ArrowDown,
                        egui::Modifiers::NONE,
                        "editorArea.splitBottom",
                    ),
                    (
                        egui::Key::Home,
                        egui::Modifiers::NONE,
                        "editorArea.splitLeft",
                    ),
                    (
                        egui::Key::PageDown,
                        egui::Modifiers::NONE,
                        "editorArea.splitBottom",
                    ),
                    (
                        egui::Key::PageUp,
                        egui::Modifiers::NONE,
                        "editorArea.splitLeft",
                    ),
                    (
                        egui::Key::ArrowDown,
                        egui::Modifiers::SHIFT,
                        "editorArea.splitLeft",
                    ),
                    (
                        egui::Key::Tab,
                        egui::Modifiers::NONE,
                        "editorArea.splitLeft",
                    ),
                    (
                        egui::Key::ArrowRight,
                        egui::Modifiers::NONE,
                        "editorArea.splitLeft",
                    ),
                ] {
                    frame(
                        [true, false]
                            .map(|pressed| Event::Key {
                                key,
                                physical_key: None,
                                pressed,
                                repeat: false,
                                modifiers,
                            })
                            .into(),
                        false,
                    );
                    navigation.push((frame(Vec::new(), false).3, label));
                }
            }
            frame(arrow(egui::Key::ArrowLeft), false);
            submenu_closed = frame(Vec::new(), false).3;
        }
        let close_events = if owner_selection.is_some() {
            Vec::new()
        } else {
            [true, false]
                .map(|pressed| Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                })
                .into()
        };
        (closed_open, _, restored_focus, _) = frame(close_events, false);
        frame(vec![Event::Text("tinue\n".into())], false);
    }
    let completed = timeout(TIMEOUT, session.wait_dispatch()).await;
    let phase = session.snapshot(|state| state.phase).unwrap();
    let epoch_after = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    if let Some((open, focus)) = owner_selection {
        assert!(!open, "owner End then Enter must select and close the menu");
        assert_eq!(focus, terminal_focus);
        let selections: Vec<_> = commands
            .iter()
            .filter_map(|command| {
                if let taide_native_app::host::HostCommand::TerminalMenu {
                    target,
                    operation,
                    title,
                } = command
                {
                    Some((target, operation, title))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(selections.len(), 1);
        assert!(matches!(
            selections[0].1,
            taide_native_app::terminal_tabs::MenuOperation::Kill
        ));
        assert_eq!(selections[0].0.session, id);
        assert_eq!(selections[0].0.project, fixture.project);
        assert_eq!(selections[0].0.pane, pane);
        assert_eq!(selections[0].0.tab, tab);
        assert!(selections[0].2.is_empty());
    }
    for (index, (tree, label)) in navigation.into_iter().enumerate() {
        let tree = tree.expect("actual menu navigation must produce an accessibility tree");
        let target = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some(resolved_label(label).as_str()))
            .unwrap_or_else(|| panic!("navigation {index}: missing actual item {label}"))
            .0;
        assert_eq!(tree.focus, target, "navigation {index}: {label}");
    }
    assert_eq!(
        menu_opened, is_pointer,
        "native menu open disagrees with source input"
    );
    assert!(
        completed.is_ok(),
        "source key and surrounding text were not delivered"
    );
    assert_eq!(phase, Phase::Exited(Some(0)));
    if is_focus {
        assert!(
            opened_focus.is_some() && opened_focus != terminal_focus,
            "source menu must own keyboard focus after press"
        );
        assert!(released_open, "source release must keep the menu open");
        assert!(!closed_open, "Escape must close the actual menu");
        assert_eq!(restored_focus, terminal_focus);
    } else if is_pointer {
        assert_eq!(epoch_after, epoch_before + 1);
    }
    if check_menu_tree && !check_submenu && !check_mixed_input {
        assert_eq!(
            visible_focus, opened_focus,
            "visible menu lost focus before release"
        );
        let tree = menu_accessibility.unwrap();
        let owner = tree.nodes.iter().find(|(id, _)| *id == tree.focus).unwrap();
        assert_eq!(owner.1.role(), egui::accesskit::Role::Menu);
        assert!(owner.1.supports_action(egui::accesskit::Action::Focus));
        let mut pending = vec![tree.focus];
        let mut descendants = Vec::new();
        while let Some(id) = pending.pop() {
            let (_, node) = tree.nodes.iter().find(|(target, _)| *target == id).unwrap();
            descendants.push(node);
            pending.extend(node.children());
        }
        for key in [
            "terminal.copy",
            "terminal.paste",
            "terminal.selectAll",
            "terminal.clear",
            "tab.split",
            "tab.newTerminal",
            "terminal.kill",
        ] {
            let item = descendants.iter().find(|node| node.label() == Some(key));
            assert!(item.is_some(), "menu focus owner has no descendant {key}");
            let item = item.unwrap();
            assert_eq!(item.role(), egui::accesskit::Role::MenuItem);
            assert!(item.supports_action(egui::accesskit::Action::Click));
            assert_eq!(item.is_disabled(), key == "terminal.copy");
            if key == "tab.split" {
                assert_eq!(item.has_popup(), Some(egui::accesskit::HasPopup::Menu));
            }
        }
        assert_eq!(
            descendants
                .iter()
                .filter(|node| node.role() == egui::accesskit::Role::Splitter)
                .count(),
            3
        );
        assert!(
            !final_accessibility
                .unwrap()
                .nodes
                .iter()
                .any(|(_, node)| node.role() == egui::accesskit::Role::Menu)
        );
    }
    if check_submenu {
        let tree = submenu_accessibility.unwrap();
        let trigger = split_target.unwrap();
        let split = &tree.nodes.iter().find(|(id, _)| *id == trigger).unwrap().1;
        assert_eq!(
            split.is_expanded(),
            Some(true),
            "keyboard must open the actual submenu"
        );
        assert_eq!(split.controls().len(), 1);
        let owner_id = split.controls()[0];
        let owner = &tree.nodes.iter().find(|(id, _)| *id == owner_id).unwrap().1;
        assert_eq!(owner.role(), egui::accesskit::Role::Menu);
        assert_eq!(owner.labelled_by(), &[trigger]);
        let mut pending = vec![owner_id];
        let mut descendants = Vec::new();
        while let Some(id) = pending.pop() {
            let (_, node) = tree.nodes.iter().find(|(target, _)| *target == id).unwrap();
            descendants.push((id, node));
            pending.extend(node.children());
        }
        for key in [
            "editorArea.splitLeft",
            "editorArea.splitRight",
            "editorArea.splitTop",
            "editorArea.splitBottom",
        ] {
            let (_, item) = descendants
                .iter()
                .find(|(_, node)| node.label() == Some(resolved_label(key).as_str()))
                .unwrap();
            assert_eq!(item.role(), egui::accesskit::Role::MenuItem);
            assert_eq!(item.is_disabled(), screen == SMALL_MENU_SCREEN);
        }
        let first = descendants
            .iter()
            .find(|(_, node)| node.label() == Some(resolved_label("editorArea.splitLeft").as_str()))
            .unwrap()
            .0;
        assert_eq!(
            tree.focus,
            if screen == SMALL_MENU_SCREEN {
                owner_id
            } else {
                first
            }
        );
        let closed = submenu_closed.unwrap();
        assert_eq!(closed.focus, trigger);
        let split = &closed
            .nodes
            .iter()
            .find(|(id, _)| *id == trigger)
            .unwrap()
            .1;
        assert_eq!(split.is_expanded(), Some(false));
        assert!(split.controls().is_empty());
        assert!(!closed.nodes.iter().any(|(id, _)| *id == owner_id));
    }
}

#[tokio::test]
async fn terminal_menu_actions는_실제_click으로_disabled_copy_select_clear_paste와_focus를_연결한다()
 {
    verify_terminal_menu_actions(None).await;
}

#[tokio::test]
async fn terminal_menu_actions는_enter_space_keydown의_disabled_copy_select_clear_paste를_연결한다()
{
    for key in [eframe::egui::Key::Space, eframe::egui::Key::Enter] {
        verify_terminal_menu_actions(Some(key)).await;
    }
}

async fn verify_terminal_menu_actions(keyboard: Option<eframe::egui::Key>) {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_model::{
        ids::{PaneId, TabId},
        layout::{PaneNode, Tab, TabKind},
    };
    use taide_native_app::{
        host::HostCommand,
        terminal_surface::{Request, Views},
    };
    const SCREEN: [f32; 2] = [640.0, 320.0];
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    output.wait_ready().await;
    let session = hub.get(&id).unwrap();
    let pane = PaneId::new();
    let tab = TabId::new();
    let mut layout = taide_layout::service::default_layout();
    layout.focused_pane = pane.clone();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: TabKind::Terminal {
                session_id: id.clone(),
                cwd: None,
            },
            title: "terminal".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(tab.clone()),
    };
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let context = egui::Context::default();
    if keyboard.is_some() {
        context.enable_accesskit();
    }
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut views = Views::default();
    let mut commands = Vec::new();
    let mut focus_once = true;
    let menu_is_open = std::cell::Cell::new(false);
    let mut keydown_closes = Vec::new();
    let mut frame = |events: Vec<Event>, views: &mut Views, commands: &mut Vec<HostCommand>| {
        let mut raw = egui::RawInput {
            events,
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        };
        views.raw_input(&context, &mut raw);
        let mut rendered = context.run_ui(raw, |ui| {
            let response = views
                .show(
                    ui,
                    Request {
                        pane: &pane,
                        tab: &tab,
                        session_id: &id,
                        hub: &hub,
                        services: &fixture.services,
                        appearance: &appearance,
                        locale: &locale,
                        request_focus: focus_once,
                        commands,
                    },
                )
                .unwrap();
            menu_is_open.set(response.context_menu_opened());
            views
                .finish_frame(&hub, &fixture.services, ui.ctx())
                .unwrap();
        });
        focus_once = false;
        rendered.textures_delta.clear();
        rendered
    };
    drop(frame(Vec::new(), &mut views, &mut commands));
    let focused = context.memory(|memory| memory.focused()).unwrap();
    commands.clear();
    let position = |rendered: &egui::FullOutput, key: &str| {
        rendered
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == key => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing menu action {key}"))
    };
    let mut copied = false;
    let mut pasted = None;
    for key in [
        "terminal.copy",
        "terminal.selectAll",
        "terminal.copy",
        "terminal.clear",
        "terminal.paste",
    ] {
        drop(frame(
            vec![
                Event::PointerMoved(context.read_response(focused).unwrap().rect.center()),
                Event::PointerButton {
                    pos: context.read_response(focused).unwrap().rect.center(),
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            &mut views,
            &mut commands,
        ));
        drop(frame(
            vec![Event::PointerButton {
                pos: context.read_response(focused).unwrap().rect.center(),
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut views,
            &mut commands,
        ));
        let shown = frame(Vec::new(), &mut views, &mut commands);
        let clicked = if let Some(keyboard) = keyboard {
            let target = shown
                .platform_output
                .accesskit_update
                .as_ref()
                .unwrap()
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(key))
                .unwrap()
                .0;
            drop(shown);
            drop(frame(
                vec![Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::Focus,
                        target_node: target,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        data: None,
                    },
                )],
                &mut views,
                &mut commands,
            ));
            drop(frame(
                vec![Event::Key {
                    key: keyboard,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                &mut views,
                &mut commands,
            ));
            keydown_closes.push(!menu_is_open.get());
            frame(
                vec![Event::Key {
                    key: keyboard,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                &mut views,
                &mut commands,
            )
        } else {
            let click = position(&shown, key);
            drop(shown);
            drop(frame(
                vec![
                    Event::PointerMoved(click),
                    Event::PointerButton {
                        pos: click,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                ],
                &mut views,
                &mut commands,
            ));
            frame(
                vec![Event::PointerButton {
                    pos: click,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
                &mut views,
                &mut commands,
            )
        };
        if key == "terminal.copy" && !copied {
            if let Some(HostCommand::CopyTerminalSelection(text)) = commands.pop() {
                assert!(text.contains("한𐐀e\u{301}"));
                copied = true;
            } else {
                assert!(commands.is_empty());
                assert!(clicked.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "terminal.selectAll")));
                drop(frame(
                    vec![Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Default::default(),
                    }],
                    &mut views,
                    &mut commands,
                ));
            }
        }
        if key == "terminal.clear" {
            session
                .snapshot(|state| {
                    assert_eq!(
                        state.core.grid().unwrap().cursor.point.line,
                        taide_native_terminal::Line(0)
                    );
                    assert_eq!(state.phase, Phase::Running);
                })
                .unwrap();
        }
        if key == "terminal.paste" {
            let Some(HostCommand::ReadTerminalClipboard(target)) = commands.pop() else {
                panic!("menu paste did not queue its target")
            };
            pasted = Some(target);
        }
        drop(clicked);
        drop(frame(Vec::new(), &mut views, &mut commands));
        assert_eq!(context.memory(|memory| memory.focused()), Some(focused));
    }
    assert!(copied);
    assert!(
        views
            .accept_paste(&pasted.unwrap(), "continue\n", &hub, &fixture.services)
            .unwrap()
    );
    let completed = timeout(TIMEOUT, session.wait_dispatch()).await;
    let phase = session.snapshot(|state| state.phase).unwrap();
    hub.close(&id).await.unwrap();
    views.cancel_inputs();
    drop(session);
    drop(hub);
    fixture.finish().await;
    if keyboard.is_some() {
        assert_eq!(keydown_closes, [false, true, true, true, true]);
    }
    assert!(matches!(completed, Ok(Ok(()))));
    assert_eq!(phase, Phase::Exited(Some(0)));
}

#[tokio::test]
async fn mouse_surface는_actual_pty의_binary_ff와_live_sgr_전환을_보존하고_join한다() {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_native_app::terminal_surface::{Request, Views};
    use taide_native_terminal::{
        Mode,
        input::{MouseAction, MouseButton, MouseInput},
    };
    const MOUSE_COLUMNS: u16 = 240;
    const DEFAULT_LAST_COLUMN: u16 = 222;
    const SCREEN: [f32; 2] = [480.0, 240.0];
    const INPUT_CELL: [f32; 2] = [2.5, 1.5];
    const MIXED_INPUT_COUNT: u64 = 7;
    const INPUT_POLL: Duration = Duration::from_millis(16);
    let fixture = Fixture::new();
    let hub = Hub::new(
        fixture.services.clone(),
        Limits {
            writer: terminal_writer::Limits {
                count: 1,
                ..limits().writer
            },
            ..limits()
        },
    )
    .unwrap();
    let output = Output::default();
    let mut options = fixture.opts();
    options.cols = MOUSE_COLUMNS;
    let id = hub
        .spawn(
            options,
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_MOUSE".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    assert_eq!(
        session
            .snapshot(|snapshot| snapshot.core.mode().unwrap() & Mode::MOUSE_MODE)
            .unwrap(),
        Mode::MOUSE_REPORT_CLICK
    );
    let left = MouseButton::Left;
    for action in [MouseAction::Press(left), MouseAction::Release(left)] {
        let InputResult::Write(receipt) = session
            .input(
                &fixture.services,
                NativeInput::Mouse(MouseInput {
                    column: DEFAULT_LAST_COLUMN,
                    row: 0,
                    pixels: None,
                    action,
                    modifiers: Default::default(),
                }),
                BYTES,
            )
            .unwrap()
        else {
            panic!("binary mouse did not enter the writer")
        };
        timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap();
    }
    output.wait_title("native-mouse-sgr-ready").await;
    assert_eq!(
        session
            .snapshot(
                |snapshot| snapshot.core.mode().unwrap() & (Mode::MOUSE_MODE | Mode::SGR_MOUSE)
            )
            .unwrap(),
        Mode::MOUSE_DRAG | Mode::SGR_MOUSE
    );
    let context = egui::Context::default();
    let epoch_before = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let pane = taide_model::ids::PaneId::new();
    let tab = taide_model::ids::TabId::new();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut frame = |events, pane: &taide_model::ids::PaneId, tab: &taide_model::ids::TabId| {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            focused: true,
            ..Default::default()
        };
        views.raw_input(&context, &mut input);
        let _ = context.run_logic(&input, |_| {});
        let replayed_events = input.events.len();
        views.raw_input_with_replay(&context, &mut input, replayed_events);
        let mut position = None;
        let mut output = context.run_ui(input, |ui| {
            if ui.ctx().current_pass_index() == 0 {
                ui.ctx().request_discard("synthetic mouse PTY multi-pass");
            }
            let response = views
                .show(
                    ui,
                    Request {
                        pane,
                        tab,
                        session_id: &id,
                        hub: &hub,
                        services: &fixture.services,
                        appearance: &appearance,
                        locale: &locale,
                        request_focus: true,
                        commands: &mut commands,
                    },
                )
                .unwrap();
            position = Some(ui.fonts_mut(|fonts| {
                response.rect.min
                    + vec2(
                        fonts.glyph_width(&appearance.font, 'M') * INPUT_CELL[0],
                        fonts.row_height(&appearance.font) * INPUT_CELL[1],
                    )
            }));
        });
        output.textures_delta.clear();
        position.unwrap()
    };
    let position = frame(Vec::new(), &pane, &tab);
    let modifiers = egui::Modifiers {
        shift: true,
        alt: true,
        ctrl: true,
        mac_cmd: true,
        command: true,
    };
    frame(
        vec![
            Event::Text("a".into()),
            Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Middle,
                pressed: true,
                modifiers,
            },
            Event::Text("b".into()),
            Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Middle,
                pressed: false,
                modifiers,
            },
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: vec2(0.0, 1.0),
                phase: egui::TouchPhase::Move,
                modifiers: Default::default(),
            },
            Event::Text("c".into()),
        ],
        &pane,
        &tab,
    );
    frame(
        vec![Event::Text("d".into())],
        &taide_model::ids::PaneId::new(),
        &taide_model::ids::TabId::new(),
    );
    assert_eq!(
        session
            .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        epoch_before + MIXED_INPUT_COUNT - 1
    );
    timeout(TIMEOUT, async {
        while !session.is_finished() {
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            tokio::time::sleep(INPUT_POLL).await;
        }
    })
    .await
    .unwrap();
    views
        .flush_inputs(&hub, &fixture.services, &context)
        .unwrap();
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.snapshot(|snapshot| snapshot.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    assert_eq!(
        session
            .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        epoch_before + MIXED_INPUT_COUNT
    );
    assert!(
        output
            .titles
            .lock()
            .unwrap()
            .iter()
            .any(|title| title == "native-mouse-finished")
    );
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
}

#[tokio::test]
async fn terminal_editor의_ax_focus_전후_문자는_실제_pty와_문서에_분리된다() {
    verify_terminal_editor_focus_input(false, false, false, false, None, false, None).await;
}

#[tokio::test]
async fn terminal_editor의_ax_focus_왕복은_미확정_조합을_취소하고_pty입력을_보존한다() {
    verify_terminal_editor_focus_input(true, false, false, false, None, false, None).await;
}

#[tokio::test]
async fn terminal_views의_ax_focus_전후_문자는_draw_순서와_무관하게_pty에_전달된다() {
    verify_terminal_editor_focus_input(false, true, false, false, None, false, None).await;
}

#[tokio::test]
async fn terminal_views의_ax_focus_보고는_문자_사이_실제_wire_순서를_보존한다() {
    verify_terminal_editor_focus_input(false, true, true, false, None, false, None).await;
}

#[tokio::test]
async fn terminal_views의_ax_focus_wire는_discard_재렌더에서_중복되지_않는다() {
    verify_terminal_editor_focus_input(false, true, true, true, None, false, None).await;
}

#[tokio::test]
async fn terminal_views의_입력전_focus_loss_gain은_draw_순서와_무관하게_먼저_전송된다() {
    verify_terminal_editor_focus_input(false, true, true, false, Some(1), false, None).await;
}

#[tokio::test]
async fn terminal_views의_잠긴_tab과_방향키는_ax_focus_이전_wire_순서를_보존한다() {
    verify_terminal_editor_focus_input(false, true, true, false, None, true, None).await;
}

#[tokio::test]
async fn terminal_views의_motion과_캡처_wheel은_ax_focus_전후_wire를_보존한다() {
    verify_terminal_editor_focus_input(
        false,
        true,
        true,
        false,
        None,
        false,
        Some(PassivePointerMode::Sgr),
    )
    .await;
}

#[tokio::test]
async fn terminal_views의_alt_wheel은_ax_focus_이전_방향키를_보존한다() {
    verify_terminal_editor_focus_input(
        false,
        true,
        true,
        false,
        None,
        false,
        Some(PassivePointerMode::Alternate),
    )
    .await;
}

#[tokio::test]
async fn terminal_views의_mousedown은_기존_문자_뒤_focus_보고_앞_입력을_보존한다() {
    for button in [
        eframe::egui::PointerButton::Primary,
        eframe::egui::PointerButton::Middle,
    ] {
        verify_terminal_editor_focus_input(
            false,
            true,
            true,
            false,
            None,
            false,
            Some(PassivePointerMode::Press(button)),
        )
        .await;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PassivePointerMode {
    Sgr,
    Alternate,
    Press(eframe::egui::PointerButton),
    PressThenAx,
}

#[tokio::test]
async fn terminal_views의_완료_click은_뒤에_온_ax_focus를_덮지_않는다() {
    verify_terminal_editor_focus_input(
        false,
        true,
        true,
        false,
        None,
        false,
        Some(PassivePointerMode::PressThenAx),
    )
    .await;
}

async fn verify_terminal_editor_focus_input(
    is_cancel: bool,
    is_second_terminal: bool,
    is_focus_reporting: bool,
    is_multi_pass: bool,
    initial_focus: Option<usize>,
    is_navigation: bool,
    passive_pointer_mode: Option<PassivePointerMode>,
) {
    use eframe::egui::{self, Event, FontId, Rect, pos2, vec2};
    use taide_model::ids::{PaneId, TabId};
    use taide_native_app::terminal_surface::{Request, Views};
    use taide_native_editor::{
        store::{EditorLimits, EditorStore},
        view::ViewKey,
    };
    use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};
    const SCREEN: [f32; 2] = [640.0, 240.0];
    const TARGET_COUNT: usize = 2;
    const FONT: f32 = 14.0;
    const LINE: f32 = 20.0;
    const PADDING: f32 = 8.0;
    const DOCUMENT_BYTES: usize = 1024;
    const UNDO_GROUPS: usize = 8;
    const AX_FOCUS_ENV: &str = "TAIDE_NATIVE_FIXTURE_AX_FOCUS";
    const PASS_COUNT: usize = 2;
    for reverse in [false, true] {
        let fixture = Fixture::new();
        let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
        let output = Output::default();
        let session_id = hub
            .spawn(
                fixture.opts(),
                HISTORY,
                async {
                    if is_focus_reporting {
                        let mode = if let Some(mode) = passive_pointer_mode {
                            match mode {
                                PassivePointerMode::Sgr => "4",
                                PassivePointerMode::Alternate => "5",
                                PassivePointerMode::Press(egui::PointerButton::Primary) => "6",
                                PassivePointerMode::Press(egui::PointerButton::Middle) => "7",
                                PassivePointerMode::PressThenAx => "8",
                                PassivePointerMode::Press(_) => {
                                    panic!("unsupported synthetic pointer button")
                                }
                            }
                        } else if is_navigation {
                            "3"
                        } else if initial_focus.is_some() {
                            "2"
                        } else {
                            "1"
                        };
                        return vec![(AX_FOCUS_ENV.into(), mode.into())];
                    }
                    Vec::new()
                },
                output.ports(),
            )
            .await
            .unwrap();
        output.wait_ready().await;
        let session = hub.get(&session_id).unwrap();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_document_bytes: DOCUMENT_BYTES,
            max_undo_groups: UNDO_GROUPS,
        })
        .unwrap();
        let editor_tab = TabId::new();
        let document = store
            .open_untitled(editor_tab.clone(), "", "plaintext".into())
            .unwrap();
        let editor_view = store
            .attach_view(
                ViewKey {
                    window: "main".into(),
                    pane: PaneId::new(),
                    tab: editor_tab,
                },
                document,
            )
            .unwrap();
        let appearance = terminal_appearance();
        let editor = NativeEditor {
            appearance: EditorAppearance {
                font: FontId::monospace(FONT),
                line_height: LINE,
                horizontal_padding: PADDING,
                background: egui::Color32::BLACK,
                foreground: egui::Color32::WHITE,
                muted: egui::Color32::GRAY,
                selection: egui::Color32::GRAY,
                cursor: egui::Color32::WHITE,
                current_line: egui::Color32::BLACK,
                line_numbers: true,
                indent: "\t".into(),
            },
        };
        let locale = taide_model::locale::ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            messages: Default::default(),
            warnings: Vec::new(),
        };
        let panes = [PaneId::new(), PaneId::new()];
        let tabs = [TabId::new(), TabId::new()];
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut views = Views::default();
        let mut commands = Vec::new();
        let mut render = |events| {
            let mut ids = Vec::new();
            let mut input = egui::RawInput {
                events,
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                ..Default::default()
            };
            let should_discard = is_multi_pass && input.events.iter().any(|event| matches!(event,
                Event::AccessKitActionRequest(request) if request.action == egui::accesskit::Action::Focus));
            views.raw_input(&context, &mut input);
            let mut rendered = context.run_ui(input, |ui| {
                ids.clear();
                if should_discard && ui.ctx().current_pass_index() == 0 {
                    ui.ctx()
                        .request_discard("synthetic terminal accessibility input pass");
                }
                let mut order = [0, 1];
                if reverse {
                    order.reverse();
                }
                for index in order {
                    let width = SCREEN[0] / TARGET_COUNT as f32;
                    let rect = Rect::from_min_size(
                        pos2(index as f32 * width, 0.0),
                        vec2(width, SCREEN[1]),
                    );
                    let response = ui
                        .scope_builder(
                            egui::UiBuilder::new()
                                .id(egui::Id::new(("terminal-editor-event-owner", index)))
                                .max_rect(rect),
                            |ui| {
                                if index == 0 || is_second_terminal {
                                    return views
                                        .show(
                                            ui,
                                            Request {
                                                pane: &panes[index],
                                                tab: &tabs[index],
                                                session_id: &session_id,
                                                hub: &hub,
                                                services: &fixture.services,
                                                appearance: &appearance,
                                                locale: &locale,
                                                request_focus: false,
                                                commands: &mut commands,
                                            },
                                        )
                                        .unwrap();
                                }
                                editor
                                    .show_with_input_route(
                                        ui,
                                        &mut store,
                                        editor_view,
                                        false,
                                        |_, _, _| false,
                                        |response| response.ctx.keyboard_input_route(response.id),
                                    )
                                    .unwrap()
                                    .response
                            },
                        )
                        .inner;
                    ids.push((index, response.id, response.rect));
                }
                views
                    .finish_frame(&hub, &fixture.services, &context)
                    .unwrap();
            });
            if should_discard {
                assert_eq!(rendered.platform_output.num_completed_passes, PASS_COUNT);
            }
            rendered.textures_delta.clear();
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            ids.sort_by_key(|(index, _, _)| *index);
            ids
        };
        let ids = render(Vec::new());
        context.memory_mut(|memory| memory.request_focus(ids[0].1));
        render(Vec::new());
        render(vec![
            Event::Ime(egui::ImeEvent::Preedit {
                text: "old-composition".into(),
                active_range_chars: None,
            }),
            Event::Text("suppressed".into()),
        ]);
        let focus = |index: usize| {
            Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: ids[index].1.accesskit_id(),
                data: None,
            })
        };
        let preedit = Event::Ime(egui::ImeEvent::Preedit {
            text: "new-composition".into(),
            active_range_chars: None,
        });
        let expected = if is_second_terminal {
            if let Some(index) = initial_focus {
                context.memory_mut(|memory| memory.request_focus(ids[index].1));
            }
            let mut events = vec![Event::Ime(egui::ImeEvent::Commit("con".into()))];
            if matches!(
                passive_pointer_mode,
                Some(PassivePointerMode::Sgr | PassivePointerMode::Alternate)
            ) {
                let position = ids[0].2.min + vec2(1.0, 1.0);
                events.extend([
                    Event::PointerMoved(position),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: vec2(0.0, 1.0),
                        modifiers: egui::Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    },
                ]);
                if passive_pointer_mode == Some(PassivePointerMode::Alternate) {
                    events.push(Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: vec2(0.0, -1.0),
                        modifiers: egui::Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    });
                }
            }
            if is_navigation {
                for key in [egui::Key::Tab, egui::Key::ArrowLeft] {
                    events.push(Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers: Default::default(),
                    });
                }
            }
            let pressed_button = match passive_pointer_mode {
                Some(PassivePointerMode::Press(button)) => Some(button),
                Some(PassivePointerMode::PressThenAx) => Some(egui::PointerButton::Primary),
                _ => None,
            };
            if let Some(button) = pressed_button {
                let position = ids[1].2.min + vec2(1.0, 1.0);
                events.extend([
                    Event::PointerButton {
                        pos: position,
                        button,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    Event::Text("tinue\n".into()),
                    Event::PointerButton {
                        pos: position,
                        button,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                if passive_pointer_mode == Some(PassivePointerMode::PressThenAx) {
                    events.push(focus(0));
                }
            } else {
                events.extend([focus(1), Event::Text("tinue\n".into())]);
            }
            render(events);
            if passive_pointer_mode == Some(PassivePointerMode::PressThenAx) {
                assert_eq!(
                    context.memory(|memory| memory.focused()),
                    Some(ids[0].1),
                    "reverse={reverse}: completed terminal click replaced later AX focus"
                );
            }
            ""
        } else if is_cancel {
            render(vec![
                Event::Text("suppressed".into()),
                focus(1),
                preedit,
                Event::Text("suppressed".into()),
                focus(0),
                Event::Text("continue\n".into()),
            ]);
            ""
        } else {
            render(vec![
                Event::Ime(egui::ImeEvent::Commit("continue\n".into())),
                focus(1),
                preedit,
                Event::Text("suppressed".into()),
                Event::Ime(egui::ImeEvent::Commit("after".into())),
            ]);
            "after"
        };
        let composition_cleared = store
            .views()
            .get(editor_view)
            .unwrap()
            .composition
            .is_none();
        let actual = store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string();
        let completed = timeout(TIMEOUT, session.wait_dispatch()).await;
        let phase = session.snapshot(|state| state.phase).unwrap();
        timeout(TIMEOUT, hub.close(&session_id))
            .await
            .unwrap()
            .unwrap();
        drop(session);
        drop(hub);
        fixture.finish().await;
        assert_eq!(actual, expected, "reverse={reverse}");
        assert!(composition_cleared, "reverse={reverse}");
        assert!(
            completed.is_ok(),
            "reverse={reverse}: terminal input was not delivered"
        );
        assert_eq!(phase, Phase::Exited(Some(0)), "reverse={reverse}");
    }
}

#[tokio::test]
async fn focus_surface는_숨김_logic_blur와_view_전환을_순서대로_보고한다() {
    use eframe::egui::{self, Event, ImeEvent, Rect, pos2, vec2};
    use taide_native_app::terminal_surface::{Request, Views};
    const SCREEN: [f32; 2] = [480.0, 240.0];
    const BLUR_LOGIC_TICKS: usize = 2;
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_FOCUS".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let initial_epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let pane = taide_model::ids::PaneId::new();
    let tab = taide_model::ids::TabId::new();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut frame =
        |views: &mut Views, events, target: Option<&taide_model::ids::TabId>, viewport| {
            let mut input = egui::RawInput {
                viewport_id: viewport,
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events,
                focused: true,
                ..Default::default()
            };
            if viewport != egui::ViewportId::ROOT {
                input.viewports.insert(
                    viewport,
                    egui::ViewportInfo {
                        parent: Some(egui::ViewportId::ROOT),
                        ..Default::default()
                    },
                );
            }
            views.raw_input(&context, &mut input);
            let mut rendered = context.run_ui(input, |ui| {
                if let Some(tab) = target {
                    views
                        .show(
                            ui,
                            Request {
                                pane: &pane,
                                tab,
                                session_id: &id,
                                hub: &hub,
                                services: &fixture.services,
                                appearance: &appearance,
                                locale: &locale,
                                request_focus: true,
                                commands: &mut commands,
                            },
                        )
                        .unwrap();
                } else {
                    ui.label("another editor");
                }
                views
                    .finish_frame(&hub, &fixture.services, &context)
                    .unwrap();
            });
            rendered.textures_delta.clear();
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
        };
    let preedit = || {
        Event::Ime(ImeEvent::Preedit {
            text: "discarded-composition".into(),
            active_range_chars: None,
        })
    };
    let root = egui::ViewportId::ROOT;
    frame(&mut views, vec![preedit()], Some(&tab), root);
    frame(&mut views, Vec::new(), None, root);
    frame(&mut views, vec![preedit()], Some(&tab), root);
    let unfocused = egui::RawInput {
        focused: false,
        ..Default::default()
    };
    for _ in 0..BLUR_LOGIC_TICKS {
        let _ = context.run_logic(&unfocused, |context| {
            views
                .flush_inputs(&hub, &fixture.services, context)
                .unwrap();
        });
    }
    frame(&mut views, vec![Event::Text("x".into())], Some(&tab), root);
    frame(
        &mut views,
        Vec::new(),
        Some(&taide_model::ids::TabId::new()),
        root,
    );
    frame(&mut views, Vec::new(), None, root);
    let auxiliary = egui::ViewportId::from_hash_of("synthetic-focus-auxiliary");
    frame(&mut views, vec![preedit()], Some(&tab), auxiliary);
    frame(&mut views, Vec::new(), None, root);
    let completed = timeout(TIMEOUT, session.wait_dispatch()).await;
    let phase = session.snapshot(|state| state.phase).unwrap();
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(
        completed.is_ok(),
        "focus lifecycle input did not reach the child"
    );
    assert_eq!(phase, Phase::Exited(Some(0)));
    assert_eq!(epoch, initial_epoch + 1);
}

#[tokio::test]
async fn query_order는_기존_pending_뒤_새_문자_앞에_실제_응답을_배치한다() {
    const INPUT_POLL: Duration = Duration::from_millis(16);
    const INPUT_COUNT: u64 = 3;
    let fixture = Fixture::new();
    let hub = Hub::new(
        fixture.services.clone(),
        Limits {
            writer: terminal_writer::Limits {
                count: 1,
                ..limits().writer
            },
            ..limits()
        },
    )
    .unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_QUERY_ORDER".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let epoch = session
        .snapshot(|snapshot| snapshot.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let InputResult::Write(start) = session
        .input(&fixture.services, NativeInput::CommittedText("s"), BYTES)
        .unwrap()
    else {
        panic!("initial input was not accepted")
    };
    let InputResult::Pending(older) = session
        .queue_input(
            &fixture.services,
            NativeInput::CommittedText("x"),
            BYTES,
            true,
        )
        .unwrap()
    else {
        panic!("deferred input was not retained")
    };
    timeout(TIMEOUT, start.wait()).await.unwrap().unwrap();
    output.wait_title("native-query-issued").await;
    let mut pending = std::collections::VecDeque::from([older]);
    match session
        .input(&fixture.services, NativeInput::CommittedText("z"), BYTES)
        .unwrap()
    {
        InputResult::Write(receipt) => timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap(),
        InputResult::Pending(input) => pending.push_back(input),
        InputResult::Local(_) => panic!("text became local input"),
    }
    let drained = timeout(TIMEOUT, async {
        while let Some(input) = pending.pop_front() {
            match session.retry_input(&fixture.services, input)? {
                InputResult::Write(receipt) => receipt.wait().await?,
                InputResult::Pending(input) => {
                    pending.push_front(input);
                    tokio::time::sleep(INPUT_POLL).await;
                }
                InputResult::Local(_) => {
                    return Err(taide_model::error::AppError::Internal(
                        "retry became local input".into(),
                    ));
                }
            }
        }
        Ok::<_, taide_model::error::AppError>(())
    })
    .await;
    let completed = timeout(TIMEOUT, session.wait_dispatch()).await;
    let snapshot = session
        .snapshot(|state| {
            (
                state.phase,
                state.core.selection_stamp().unwrap().input_epoch,
            )
        })
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(pending);
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(
        matches!(drained, Ok(Ok(()))),
        "pending input was not drained: {drained:?}"
    );
    assert!(
        matches!(completed, Ok(Ok(()))),
        "query completion failed: {completed:?}"
    );
    assert_eq!(snapshot, (Phase::Exited(Some(0)), epoch + INPUT_COUNT));
}

#[tokio::test]
async fn focus_pressure는_포화_중_보고를_합쳐_보존하고_새_문자의_추월을_막는다() {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_native_app::terminal_surface::{Request, Views};
    const SCREEN: [f32; 2] = [480.0, 240.0];
    const FILL_INPUTS: usize = 63;
    const ACCEPTED_TEXTS: u64 = 64;
    const INPUT_POLL: Duration = Duration::from_millis(16);
    let fixture = Fixture::new();
    let hub = Hub::new(
        fixture.services.clone(),
        Limits {
            writer: terminal_writer::Limits {
                count: 1,
                ..limits().writer
            },
            ..limits()
        },
    )
    .unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_FOCUS_PRESSURE".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let initial_epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let pane = taide_model::ids::PaneId::new();
    let tab = taide_model::ids::TabId::new();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut frame = |views: &mut Views, events, hidden| {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            focused: true,
            ..Default::default()
        };
        views.raw_input(&context, &mut input);
        let mut rendered = context.run_ui(input, |ui| {
            if hidden {
                ui.label("another editor");
            } else {
                views
                    .show(
                        ui,
                        Request {
                            pane: &pane,
                            tab: &tab,
                            session_id: &id,
                            hub: &hub,
                            services: &fixture.services,
                            appearance: &appearance,
                            locale: &locale,
                            request_focus: true,
                            commands: &mut commands,
                        },
                    )
                    .unwrap();
            }
            views
                .finish_frame(&hub, &fixture.services, &context)
                .unwrap();
        });
        rendered.textures_delta.clear();
    };
    frame(
        &mut views,
        (0..FILL_INPUTS).map(|_| Event::Text("x".into())).collect(),
        false,
    );
    frame(&mut views, Vec::new(), true);
    frame(&mut views, Vec::new(), false);
    frame(&mut views, Vec::new(), true);
    frame(&mut views, vec![Event::Text("not-admitted".into())], false);
    let reports = timeout(TIMEOUT, async {
        while !output
            .titles
            .lock()
            .unwrap()
            .iter()
            .any(|title| title == "native-focus-pressure-ready")
        {
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            tokio::time::sleep(INPUT_POLL).await;
        }
    })
    .await;
    if reports.is_ok() {
        frame(&mut views, vec![Event::Text("z".into())], false);
        frame(&mut views, Vec::new(), true);
    }
    let completed = timeout(TIMEOUT, async {
        while !session.is_finished() {
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            tokio::time::sleep(INPUT_POLL).await;
        }
    })
    .await;
    let phase = session.snapshot(|state| state.phase).unwrap();
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(
        reports.is_ok(),
        "focus reports were lost at the input budget"
    );
    assert!(
        completed.is_ok(),
        "focus reports were overtaken by new text"
    );
    assert_eq!(phase, Phase::Exited(Some(0)));
    assert_eq!(epoch, initial_epoch + ACCEPTED_TEXTS);
}

#[tokio::test]
async fn hidden_mouse는_포화_뒤_숨긴_view의_release를_원래_좌표로_전송한다() {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_native_app::terminal_surface::{Request, Views};
    const SCREEN: [f32; 2] = [480.0, 240.0];
    const INPUT_CELL: [f32; 2] = [2.5, 1.5];
    const FILL_INPUTS: usize = 63;
    const TOTAL_INPUTS: u64 = 68;
    const SECOND_VIEW_OFFSET: f32 = 40.0;
    const INPUT_POLL: Duration = Duration::from_millis(16);
    let fixture = Fixture::new();
    let hub = Hub::new(
        fixture.services.clone(),
        Limits {
            writer: terminal_writer::Limits {
                count: 1,
                ..limits().writer
            },
            ..limits()
        },
    )
    .unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_HIDDEN_MOUSE".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let initial_epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let pane = taide_model::ids::PaneId::new();
    let tab = taide_model::ids::TabId::new();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut frame = |views: &mut Views, events, hidden, target: &taide_model::ids::TabId| {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            focused: true,
            ..Default::default()
        };
        views.raw_input(&context, &mut input);
        let mut position = None;
        let mut rendered = context.run_ui(input, |ui| {
            if hidden {
                ui.label("another tab");
                return;
            }
            if target != &tab {
                ui.add_space(SECOND_VIEW_OFFSET);
            }
            let response = views
                .show(
                    ui,
                    Request {
                        pane: &pane,
                        tab: target,
                        session_id: &id,
                        hub: &hub,
                        services: &fixture.services,
                        appearance: &appearance,
                        locale: &locale,
                        request_focus: true,
                        commands: &mut commands,
                    },
                )
                .unwrap();
            position = Some(ui.fonts_mut(|fonts| {
                response.rect.min
                    + vec2(
                        fonts.glyph_width(&appearance.font, 'M') * INPUT_CELL[0],
                        fonts.row_height(&appearance.font) * INPUT_CELL[1],
                    )
            }));
        });
        rendered.textures_delta.clear();
        position
    };
    let position = frame(&mut views, Vec::new(), false, &tab).unwrap();
    let button = |pressed| Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let mut input = (0..FILL_INPUTS)
        .map(|_| Event::Text("x".into()))
        .collect::<Vec<_>>();
    input.push(button(true));
    frame(&mut views, input, false, &tab);
    frame(&mut views, Vec::new(), true, &tab);
    frame(&mut views, vec![button(false)], true, &tab);
    let first_cycle = timeout(TIMEOUT, async {
        while !output
            .titles
            .lock()
            .unwrap()
            .iter()
            .any(|title| title == "native-hidden-cycle-ready")
        {
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            tokio::time::sleep(INPUT_POLL).await;
        }
    })
    .await;
    if first_cycle.is_ok() {
        frame(&mut views, Vec::new(), false, &tab);
        frame(&mut views, vec![button(true)], false, &tab);
        frame(&mut views, Vec::new(), true, &tab);
        frame(
            &mut views,
            vec![button(false), Event::Text("z".into())],
            false,
            &taide_model::ids::TabId::new(),
        );
    }
    let completed = timeout(TIMEOUT, async {
        while !session.is_finished() {
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            tokio::time::sleep(INPUT_POLL).await;
        }
    })
    .await;
    let phase = session.snapshot(|state| state.phase).unwrap();
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(first_cycle.is_ok(), "hidden release was not delivered");
    assert!(completed.is_ok(), "hidden release was overtaken");
    assert_eq!(phase, Phase::Exited(Some(0)));
    assert_eq!(epoch, initial_epoch + TOTAL_INPUTS);
}

#[tokio::test]
async fn headless_input_budget은_포화에서_선택을_유지하고_취소된_대기를_전송하지_않는다() {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_native_app::terminal_surface::{Request, Views};
    const SCREEN: [f32; 2] = [480.0, 240.0];
    const QUEUE_INPUTS: usize = 64;
    let fixture = Fixture::new();
    let hub = Hub::new(
        fixture.services.clone(),
        Limits {
            writer: terminal_writer::Limits {
                count: 1,
                ..limits().writer
            },
            ..limits()
        },
    )
    .unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let initial_epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let pane = taide_model::ids::PaneId::new();
    let tab = taide_model::ids::TabId::new();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut events = (0..QUEUE_INPUTS)
        .map(|_| Event::Text("x".into()))
        .collect::<Vec<_>>();
    events.push(Event::Text("overflow".into()));
    events.push(Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            mac_cmd: true,
            command: true,
            ..Default::default()
        },
    });
    events.push(Event::Copy);
    let mut rendered = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            focused: true,
            ..Default::default()
        },
        |ui| {
            views
                .show(
                    ui,
                    Request {
                        pane: &pane,
                        tab: &tab,
                        session_id: &id,
                        hub: &hub,
                        services: &fixture.services,
                        appearance: &appearance,
                        locale: &locale,
                        request_focus: true,
                        commands: &mut commands,
                    },
                )
                .unwrap();
        },
    );
    rendered.textures_delta.clear();
    let copied = rendered.platform_output.commands.iter().any(|command| {
        matches!(command, egui::OutputCommand::CopyText(text) if text.contains("한𐐀e\u{301}\n"))
    });
    let overflow = rendered.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("budget exceeded"))
    });
    views.cancel_inputs();
    views
        .flush_inputs(&hub, &fixture.services, &context)
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    let final_epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(copied, "full writer queue blocked local selection");
    assert!(overflow, "input overflow was not reported");
    assert_eq!(final_epoch, initial_epoch + 1);
}

#[tokio::test]
async fn hidden_wheel은_포화_뒤_숨긴_view의_스크롤을_다른_view_문자보다_먼저_전송한다() {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_native_app::terminal_surface::{Request, Views};
    const SCREEN: [f32; 2] = [480.0, 240.0];
    const FILL_INPUTS: usize = 64;
    const TOTAL_INPUTS: u64 = 67;
    const INPUT_POLL: Duration = Duration::from_millis(16);
    let fixture = Fixture::new();
    let hub = Hub::new(
        fixture.services.clone(),
        Limits {
            writer: terminal_writer::Limits {
                count: 1,
                ..limits().writer
            },
            ..limits()
        },
    )
    .unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_HIDDEN_WHEEL".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let initial_epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let pane = taide_model::ids::PaneId::new();
    let tab = taide_model::ids::TabId::new();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let mut frame = |views: &mut Views, events, hidden, target: &taide_model::ids::TabId| {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            focused: true,
            ..Default::default()
        };
        views.raw_input(&context, &mut input);
        let mut rendered = context.run_ui(input, |ui| {
            if hidden {
                ui.label("another tab");
            } else {
                views
                    .show(
                        ui,
                        Request {
                            pane: &pane,
                            tab: target,
                            session_id: &id,
                            hub: &hub,
                            services: &fixture.services,
                            appearance: &appearance,
                            locale: &locale,
                            request_focus: true,
                            commands: &mut commands,
                        },
                    )
                    .unwrap();
            }
            views
                .finish_frame(&hub, &fixture.services, ui.ctx())
                .unwrap();
        });
        rendered.textures_delta.clear();
    };
    let wheel = |amount| Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, amount),
        phase: egui::TouchPhase::Move,
        modifiers: Default::default(),
    };
    frame(&mut views, Vec::new(), false, &tab);
    let mut input = vec![Event::PointerMoved(pos2(SCREEN[0] / 2.0, SCREEN[1] / 2.0))];
    input.extend((0..FILL_INPUTS).map(|_| Event::Text("x".into())));
    input.push(wheel(1.0));
    frame(&mut views, input, false, &tab);
    frame(&mut views, Vec::new(), true, &tab);
    frame(&mut views, Vec::new(), true, &tab);
    let first_cycle = timeout(TIMEOUT, async {
        while !output
            .titles
            .lock()
            .unwrap()
            .iter()
            .any(|title| title == "native-hidden-wheel-ready")
        {
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            tokio::time::sleep(INPUT_POLL).await;
        }
    })
    .await;
    if first_cycle.is_ok() {
        frame(&mut views, Vec::new(), false, &tab);
        frame(
            &mut views,
            vec![wheel(-1.0), Event::Text("z".into())],
            false,
            &taide_model::ids::TabId::new(),
        );
    }
    let completed = timeout(TIMEOUT, async {
        while !session.is_finished() {
            views
                .flush_inputs(&hub, &fixture.services, &context)
                .unwrap();
            tokio::time::sleep(INPUT_POLL).await;
        }
    })
    .await;
    let phase = session.snapshot(|state| state.phase).unwrap();
    let epoch = session
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    drop(session);
    drop(hub);
    fixture.finish().await;
    assert!(first_cycle.is_ok(), "hidden wheel was not delivered");
    assert!(completed.is_ok(), "hidden wheel was overtaken");
    assert_eq!(phase, Phase::Exited(Some(0)));
    assert_eq!(epoch, initial_epoch + TOTAL_INPUTS);
}

#[tokio::test]
async fn headless_wheel은_실제_alt_pty에_한_방향키씩_전달하고_join한다() {
    use eframe::egui::{self, Event, Rect, pos2, vec2};
    use taide_native_app::terminal_surface::{Request, Views};
    use taide_native_terminal::Mode;
    const SCREEN: [f32; 2] = [480.0, 240.0];
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_WHEEL".into(), "1".into())] },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    session
        .snapshot(|snapshot| {
            assert!(
                snapshot
                    .core
                    .mode()
                    .unwrap()
                    .contains(Mode::ALT_SCREEN | Mode::APP_CURSOR)
            );
        })
        .unwrap();
    let layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    let tab = taide_native_app::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(tab.kind, taide_model::layout::TabKind::Terminal { .. }))
        .unwrap()
        .id
        .clone();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let mut frame = |events| {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(SCREEN[0], SCREEN[1]),
            )),
            events,
            focused: true,
            ..Default::default()
        };
        views.raw_input(&context, &mut input);
        let mut output = context.run_ui(input, |ui| {
            views
                .show(
                    ui,
                    Request {
                        pane: &pane,
                        tab: &tab,
                        session_id: &id,
                        hub: &hub,
                        services: &fixture.services,
                        appearance: &appearance,
                        locale: &locale,
                        request_focus: true,
                        commands: &mut commands,
                    },
                )
                .unwrap();
        });
        output.textures_delta.clear();
    };
    frame(Vec::new());
    frame(vec![
        Event::PointerMoved(pos2(SCREEN[0] / 2.0, SCREEN[1] / 2.0)),
        Event::Text("a".into()),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            modifiers: Default::default(),
        },
        Event::Ime(egui::ImeEvent::Commit("b".into())),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: vec2(0.0, -1.0),
            phase: egui::TouchPhase::Move,
            modifiers: Default::default(),
        },
        Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        },
    ]);
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.snapshot(|snapshot| snapshot.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    assert!(
        output
            .titles
            .lock()
            .unwrap()
            .iter()
            .any(|title| title == "native-wheel-finished")
    );
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn headless_surface의_measured_attach와_ime_확정은_실제_pty에_연결된다() {
    use eframe::egui::{self, Event, ImeEvent, Rect, pos2, vec2};
    use taide_native_app::{
        host::HostCommand,
        terminal_surface::{Appearance, Request, Views},
        terminal_tabs::Tabs,
    };
    const SCREEN: [f32; 2] = [480.0, 240.0];
    const FONT_SIZE: u32 = 13;
    let fixture = Fixture::new();
    fixture.services.state.settings.write().shell_override = fixture.opts().shell;
    fixture.services.state.settings.write().terminal_scrollback = HISTORY as u32;
    let layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    let tab = taide_native_app::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(tab.kind, taide_model::layout::TabKind::Terminal { .. }))
        .unwrap()
        .id
        .clone();
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let native = Tabs::new(fixture.services.clone(), limits()).unwrap();
    let colors = [
        "background",
        "foreground",
        "cursor",
        "selection",
        "black",
        "red",
        "green",
        "yellow",
        "blue",
        "magenta",
        "cyan",
        "white",
        "brightBlack",
        "brightRed",
        "brightGreen",
        "brightYellow",
        "brightBlue",
        "brightMagenta",
        "brightCyan",
        "brightWhite",
    ]
    .into_iter()
    .map(|key| (key, "#aabbcc"))
    .collect::<std::collections::BTreeMap<_, _>>();
    let theme = serde_json::from_value(serde_json::json!({"id":"synthetic", "name":"synthetic", "type":"dark", "colors":{}, "syntax":{}, "terminal":colors})).unwrap();
    let appearance = Appearance::new(&theme, FONT_SIZE).unwrap();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: [
            ("terminal.restart".into(), "Restart".into()),
            ("terminal.processExited".into(), "Process exited".into()),
        ]
        .into(),
        warnings: Vec::new(),
    };
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let frame = |session_id: &str,
                 events: Vec<Event>,
                 views: &mut Views,
                 commands: &mut Vec<HostCommand>| {
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                events,
                focused: true,
                ..Default::default()
            },
            |ui| {
                views
                    .show(
                        ui,
                        Request {
                            pane: &pane,
                            tab: &tab,
                            session_id,
                            hub: native.hub(),
                            services: &fixture.services,
                            appearance: &appearance,
                            locale: &locale,
                            request_focus: true,
                            commands,
                        },
                    )
                    .unwrap();
            },
        );
        output.textures_delta.clear();
        output
    };
    drop(frame(
        "",
        vec![Event::Text("discard-before-failure".into())],
        &mut views,
        &mut commands,
    ));
    drop(frame("", Vec::new(), &mut views, &mut commands));
    let HostCommand::AttachTerminal {
        tab: attached,
        size,
        ports,
    } = commands.pop().unwrap()
    else {
        panic!("measured surface did not attach")
    };
    assert!(commands.is_empty());
    assert_eq!(attached, tab);
    assert!(size.columns > 2 && size.rows > 1);
    fixture.services.state.settings.write().shell_override =
        Some("/synthetic/missing-taide-shell".into());
    let result = native
        .attach(tab.clone(), size, async { Vec::new() }, ports)
        .await;
    views.attached(tab.clone(), &result, native.hub(), &fixture.services);
    assert!(result.is_err());
    let click_restart = |output: &egui::FullOutput| {
        let position = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "Restart" => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap();
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ]
    };
    let failed = frame("", Vec::new(), &mut views, &mut commands);
    assert!(failed.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("missing-taide-shell"))));
    drop(frame("", click_restart(&failed), &mut views, &mut commands));
    fixture.services.state.settings.write().shell_override = fixture.opts().shell;
    drop(frame(
        "",
        vec![Event::Text("con".into())],
        &mut views,
        &mut commands,
    ));
    let HostCommand::AttachTerminal {
        tab: retry,
        size,
        mut ports,
    } = commands.pop().unwrap()
    else {
        panic!("failed surface did not retry")
    };
    assert_eq!(retry, tab);
    assert!(commands.is_empty());
    let output = Output::default();
    ports.stream = output.ports().stream;
    let result = native
        .attach(tab.clone(), size, async { Vec::new() }, ports)
        .await;
    views.attached(tab.clone(), &result, native.hub(), &fixture.services);
    let id = result.unwrap();
    let session = native.hub().get(&id).unwrap();
    output.wait_ready().await;
    let copied = frame(
        &id,
        vec![
            Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    mac_cmd: true,
                    command: true,
                    ..Default::default()
                },
            },
            Event::Copy,
        ],
        &mut views,
        &mut commands,
    );
    assert!(copied.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text.contains("한𐐀e\u{301}\n") && !text.contains("  "))), "copy output: {:?}", copied.platform_output.commands);
    let mut rendered = frame(
        &id,
        vec![Event::Ime(ImeEvent::Preedit {
            text: "tinue".into(),
            active_range_chars: None,
        })],
        &mut views,
        &mut commands,
    );
    assert!(rendered.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "한")
    ));
    rendered.textures_delta.clear();
    assert_eq!(
        session.snapshot(|snapshot| snapshot.phase).unwrap(),
        Phase::Running
    );
    drop(frame(
        &id,
        vec![
            Event::Ime(ImeEvent::Commit("tinue".into())),
            Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            },
        ],
        &mut views,
        &mut commands,
    ));
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        session.snapshot(|snapshot| snapshot.phase).unwrap(),
        Phase::Exited(Some(0))
    );
    let exited = frame(&id, Vec::new(), &mut views, &mut commands);
    assert!(exited.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Process exited (0)")));
    drop(frame(
        &id,
        click_restart(&exited),
        &mut views,
        &mut commands,
    ));
    let HostCommand::RestartTerminal {
        tab: restarted,
        size,
        mut ports,
    } = commands.pop().unwrap()
    else {
        panic!("exited surface did not restart")
    };
    assert_eq!(restarted, tab);
    let retired = Arc::downgrade(&session);
    drop(session);
    let next_output = Output::default();
    ports.stream = next_output.ports().stream;
    let restarted = native
        .restart(tab.clone(), size, async { Vec::new() }, ports)
        .await;
    views.attached(tab.clone(), &restarted, native.hub(), &fixture.services);
    let next_id = restarted.unwrap();
    assert_ne!(next_id, id);
    assert!(retired.upgrade().is_none());
    assert!(native.hub().get(&id).is_none());
    next_output.wait_ready().await;
    assert!(
        native
            .restart(
                tab,
                size,
                async { panic!("running restart must not resolve env") },
                Output::default().ports()
            )
            .await
            .is_err()
    );
    timeout(TIMEOUT, native.close(&next_id))
        .await
        .unwrap()
        .unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_root_spawn은_단일_session_final_sync_raw_attach와_실제_join을_보존한다() {
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = timeout(
        TIMEOUT,
        hub.spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    let session = hub.get(&id).unwrap();
    let view = hub.get(&id).unwrap();
    assert!(Arc::ptr_eq(&session, &view));
    output.wait_ready().await;
    assert_eq!(
        session.snapshot(|state| state.phase).unwrap(),
        Phase::Running
    );
    assert!(
        hub.spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports()
        )
        .await
        .is_err()
    );
    let raw = Arc::new(Mutex::new(Vec::new()));
    let observed = raw.clone();
    let attached = fixture
        .services
        .terminal
        .attach(&id, move |bytes| {
            let mut captured = observed.lock().unwrap();
            if captured.len() + bytes.len() > BYTES {
                return false;
            }
            captured.extend_from_slice(bytes);
            true
        })
        .unwrap();
    assert!(attached.replay_bytes > taide_terminal::session::TERMINAL_REPLAY_PREAMBLE.len() as u32);
    let InputResult::Write(receipt) = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("continue\n"),
            BYTES,
        )
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap();
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert!(session.is_finished());
    assert_eq!(session.failure(), None);
    session
        .snapshot(|state| {
            assert_eq!(state.phase, Phase::Exited(Some(0)));
            assert_eq!(state.exit_code, Some(Some(0)));
            assert!(state.core.grid().is_ok());
            assert!(state.revision > 1);
        })
        .unwrap();
    assert!(
        output
            .titles
            .lock()
            .unwrap()
            .iter()
            .any(|title| title == "native-finished")
    );
    assert!(!session.metadata().snapshot(&id).running);
    assert!(
        session
            .input(&fixture.services, NativeInput::CommittedText("late"), BYTES)
            .is_err()
    );
    {
        let published = fixture.events.0.lock().unwrap();
        assert!(
            matches!(&published[0], AppEvent::TerminalSpawned { session_id, .. } if session_id == &id)
        );
        assert_eq!(published.iter().filter(|event| matches!(event, AppEvent::TerminalExited { session_id, code: Some(0) } if session_id == &id)).count(), 1);
        let raw = raw.lock().unwrap();
        assert!(String::from_utf8_lossy(&raw).contains("row-999"));
        assert!(String::from_utf8_lossy(&raw).contains("final-sync"));
    }
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    assert!(hub.get(&id).is_none());
    drop(session);
    drop(view);
    fixture.finish().await;
}

#[tokio::test]
async fn 활성_session의_root_stop은_actor_drop에서_원본_store와_child를_회수한다() {
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    fixture.tasks.stop_all();
    assert!(
        timeout(TIMEOUT, session.wait_dispatch())
            .await
            .unwrap()
            .is_err()
    );
    assert_eq!(
        session.failure(),
        Some(taide_native_app::terminal_host::Failure::Supervisor)
    );
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    fixture.finish().await;
    assert!(session.snapshot(|state| state.exit_code.is_some()).unwrap());
    assert!(!session.metadata().snapshot(&id).running);
}

#[tokio::test]
async fn spawn_취소와_닫힌_project는_registry와_admission을_남기지_않는다() {
    let fixture = Fixture::new();
    let hub = Arc::new(Hub::new(fixture.services.clone(), limits()).unwrap());
    let output = Output::default();
    let (entered, ready) = oneshot::channel();
    let blocked = hub.clone();
    let opts = fixture.opts();
    let ports = output.ports();
    let request = tokio::spawn(async move {
        blocked
            .spawn(
                opts,
                HISTORY,
                async {
                    let _ = entered.send(());
                    std::future::pending::<Vec<(String, String)>>().await
                },
                ports,
            )
            .await
    });
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    fixture
        .services
        .state
        .projects
        .write()
        .remove(&fixture.project);
    assert!(
        hub.spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports()
        )
        .await
        .is_err()
    );
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    assert!(fixture.events.0.lock().unwrap().is_empty());
    assert!(
        Hub::new(
            fixture.services.clone(),
            Limits {
                writer: terminal_writer::Limits {
                    bytes: 0,
                    count: COUNT
                },
                ..limits()
            }
        )
        .is_err()
    );
    fixture.finish().await;
    let terminal = taide_native_terminal::session::SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    terminal
        .fail_and_stop(taide_native_terminal::session::Failure::Delivery)
        .unwrap();
    assert!(terminal.advance(b"late").is_err());
}

#[tokio::test]
async fn 활성_close는_실제_actor를_join하고_보유_view의_admission은_반납하지_않는다() {
    let fixture = Fixture::new();
    let hub = Hub::new(
        fixture.services.clone(),
        Limits {
            writer: terminal_writer::Limits {
                bytes: 1,
                count: COUNT,
            },
            ..limits()
        },
    )
    .unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    let view = hub.get(&id).unwrap();
    output.wait_ready().await;
    let initial_epoch = view
        .snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
        .unwrap();
    assert!(
        view.input(
            &fixture.services,
            NativeInput::CommittedText("rejected"),
            BYTES,
        )
        .is_err()
    );
    assert_eq!(
        view.snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        initial_epoch
    );
    assert!(matches!(
        view.input(&fixture.services, NativeInput::Preedit("합성"), BYTES)
            .unwrap(),
        InputResult::Local(taide_native_terminal::input::InputAction::Ignore)
    ));
    let InputResult::Pending(pending) = view
        .queue_input(
            &fixture.services,
            NativeInput::CommittedText("cancelled"),
            BYTES,
            true,
        )
        .unwrap()
    else {
        panic!("deferred input was not retained")
    };
    assert_eq!(
        view.snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        initial_epoch
    );
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    assert!(view.is_finished());
    assert_eq!(view.failure(), None);
    assert!(matches!(
        view.snapshot(|state| state.phase).unwrap(),
        Phase::Exited(_)
    ));
    assert!(hub.get(&id).is_none());
    assert!(view.retry_input(&fixture.services, pending).is_err());
    assert_eq!(
        view.snapshot(|state| state.core.selection_stamp().unwrap().input_epoch)
            .unwrap(),
        initial_epoch
    );
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    assert!(matches!(
        hub.spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports()
        )
        .await,
        Err(taide_model::error::AppError::InvalidArgument(_))
    ));
    drop(view);
    fixture
        .services
        .state
        .projects
        .write()
        .remove(&fixture.project);
    assert!(matches!(
        hub.spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports()
        )
        .await,
        Err(taide_model::error::AppError::NotFound(_))
    ));
    fixture.finish().await;
}

#[tokio::test]
async fn 종료한_child의_마지막_query는_응답하지_않고_최종_grid를_보존한다() {
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let mut ports = output.ports();
    ports.color = Arc::new(|_| {
        Err(taide_model::error::AppError::Forbidden(
            "synthetic exited query must not be resolved".into(),
        ))
    });
    ports.geometry = Arc::new(|| {
        Err(taide_model::error::AppError::Forbidden(
            "synthetic exited geometry must not be resolved".into(),
        ))
    });
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_FINAL_QUERY".into(), "1".into())] },
            ports,
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let InputResult::Write(receipt) = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("continue\n"),
            BYTES,
        )
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap();
    let result = timeout(TIMEOUT, session.wait_dispatch()).await.unwrap();
    let snapshot = session
        .snapshot(|state| (state.phase, state.core.grid().is_ok()))
        .unwrap();
    let failure = session.failure();
    let _ = hub.close(&id).await;
    fixture.finish().await;
    assert!(
        result.is_ok(),
        "ended query discarded the final state: {failure:?}"
    );
    assert_eq!(snapshot, (Phase::Exited(Some(0)), true));
    assert!(
        output
            .titles
            .lock()
            .unwrap()
            .iter()
            .any(|title| title == "native-finished")
    );
}

#[tokio::test]
async fn 실제_new_terminal은_독립탭을_만들고_native_attach는_같은_core를_재사용한다() {
    use taide_model::layout::{PaneNode, TabKind};
    use taide_native_app::{
        host::{HostBridge, HostCommand},
        terminal_tabs::Tabs,
    };

    let fixture = Fixture::new();
    fixture.services.state.settings.write().shell_override = fixture.opts().shell;
    fixture.services.state.settings.write().terminal_scrollback = HISTORY as u32;
    let mut layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: Vec::new(),
        active: None,
    };
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let signal = Arc::new(Notify::new());
    let ready = signal.clone();
    let mut host = HostBridge::connect(
        fixture.services.clone(),
        Arc::new(move || ready.notify_one()),
    )
    .unwrap();
    for count in [1, 2] {
        host.submit(HostCommand::NewTerminal {
            project: fixture.project.clone(),
            pane: pane.clone(),
            title: "합성 터미널".into(),
        })
        .unwrap();
        timeout(TIMEOUT, async {
            loop {
                assert!(host.poll().is_none());
                if taide_native_app::tabs::tabs_in(
                    &fixture.services.state.layouts.read()[&fixture.project].root,
                )
                .len()
                    == count
                {
                    break;
                }
                signal.notified().await;
            }
        })
        .await
        .unwrap();
    }
    let tab = {
        let layouts = fixture.services.state.layouts.read();
        let tabs = taide_native_app::tabs::tabs_in(&layouts[&fixture.project].root);
        assert_ne!(tabs[0].id, tabs[1].id);
        assert!(tabs.iter().all(|tab| tab.title == "합성 터미널"
            && !tab.preview
            && matches!(&tab.kind, TabKind::Terminal { session_id, .. } if session_id.is_empty())));
        tabs[0].id.clone()
    };
    let native = Tabs::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    let id = native
        .attach(tab.clone(), size, async { Vec::new() }, output.ports())
        .await
        .unwrap();
    output.wait_ready().await;
    let session = native.hub().get(&id).unwrap();
    let duplicate = native
        .attach(
            tab.clone(),
            size,
            async { panic!("live attach must not resolve spawn env") },
            output.ports(),
        )
        .await
        .unwrap();
    assert_eq!(id, duplicate);
    assert!(Arc::ptr_eq(
        &session,
        &native.hub().get(&duplicate).unwrap()
    ));
    let closed = taide_native_app::tabs::close(&fixture.services, tab, false)
        .await
        .unwrap();
    assert!(matches!(closed.tab.kind, TabKind::Terminal { session_id, .. } if session_id == id));
    timeout(TIMEOUT, native.close(&id)).await.unwrap().unwrap();
    assert!(session.is_finished());
    assert!(native.hub().get(&id).is_none());
    timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn native_attach_대기중_닫힌탭은_생성된_실제_pty와_hub_entry를_회수한다() {
    use taide_native_app::terminal_tabs::Tabs;
    let fixture = Fixture::new();
    fixture.services.state.settings.write().shell_override = fixture.opts().shell;
    fixture.services.state.settings.write().terminal_scrollback = HISTORY as u32;
    let layout = taide_layout::service::default_layout();
    let tab = taide_native_app::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(tab.kind, taide_model::layout::TabKind::Terminal { .. }))
        .unwrap()
        .id
        .clone();
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let native = Arc::new(Tabs::new(fixture.services.clone(), limits()).unwrap());
    let worker = native.clone();
    let output = Output::default();
    let (entered, wait) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let request_tab = tab.clone();
    let request = tokio::spawn(async move {
        worker
            .attach(
                request_tab,
                Size {
                    columns: COLUMNS,
                    rows: ROWS,
                },
                async {
                    let _ = entered.send(());
                    released.await.unwrap();
                    Vec::new()
                },
                output.ports(),
            )
            .await
    });
    timeout(TIMEOUT, wait).await.unwrap().unwrap();
    taide_native_app::tabs::close(&fixture.services, tab, false)
        .await
        .unwrap();
    release.send(()).unwrap();
    assert!(matches!(
        timeout(TIMEOUT, request).await.unwrap().unwrap(),
        Err(taide_model::error::AppError::NotFound(_))
    ));
    {
        let events = fixture.events.0.lock().unwrap();
        let id = events
            .iter()
            .find_map(|event| match event {
                AppEvent::TerminalSpawned { session_id, .. } => Some(session_id),
                _ => None,
            })
            .unwrap();
        assert!(native.hub().get(id).is_none());
        assert!(
            fixture
                .services
                .terminal
                .sessions_for_project(&fixture.project)
                .is_empty()
        );
    }
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_host_attach와_close는_공유_hub와_원본_환경을_사용한다() {
    use taide_native_app::host::{HostBridge, HostCommand, HostReply, Terminals};
    use taide_native_app::terminal_tabs::Tabs;
    let fixture = Fixture::new();
    fixture.services.state.settings.write().shell_override = fixture.opts().shell;
    fixture.services.state.settings.write().terminal_scrollback = HISTORY as u32;
    let layout = taide_layout::service::default_layout();
    let tab = taide_native_app::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(tab.kind, taide_model::layout::TabKind::Terminal { .. }))
        .unwrap()
        .id
        .clone();
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let server = tokio::spawn(async {});
    fixture
        .services
        .ide
        .mark_started(
            IDE_PORT,
            String::new(),
            fixture.services.state.paths.data_dir.clone(),
            server,
        )
        .unwrap();
    let native = Arc::new(Tabs::new(fixture.services.clone(), limits()).unwrap());
    let signal = Arc::new(Notify::new());
    let ready = signal.clone();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let captured = observed.clone();
    let environment: taide_native_app::terminal_environment::Environment =
        Arc::new(move |services| {
            let captured = captured.clone();
            Box::pin(async move {
                let env = taide_runtime::terminal_env::resolve(
                    &services.state,
                    &services.ide,
                    "synthetic-version",
                    Some("/synthetic/taide"),
                )
                .await;
                *captured.lock().unwrap() = env.clone();
                env
            })
        });
    let mut host = HostBridge::connect_with_ports(
        fixture.services.clone(),
        Arc::new(move || ready.notify_one()),
        Arc::new(|_| panic!("terminal operation must not access clipboard")),
        Some(Terminals {
            tabs: native.clone(),
            environment,
        }),
    )
    .unwrap();
    let output = Output::default();
    host.submit(HostCommand::AttachTerminal {
        tab: tab.clone(),
        size: Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        ports: output.ports(),
    })
    .unwrap();
    let reply = timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = host.poll() {
                break reply;
            }
            signal.notified().await;
        }
    })
    .await
    .unwrap();
    let HostReply::TerminalAttached {
        tab: attached,
        result,
    } = reply
    else {
        panic!("unexpected terminal reply")
    };
    assert_eq!(attached, tab);
    let id = result.unwrap();
    output.wait_ready().await;
    assert_eq!(
        *observed.lock().unwrap(),
        vec![
            ("CLAUDE_CODE_SSE_PORT".into(), IDE_PORT.to_string()),
            ("EDITOR".into(), "/synthetic/taide --wait".into()),
            ("VISUAL".into(), "/synthetic/taide --wait".into()),
            ("TAIDE_AGENT_PROTOCOL_VERSION".into(), "1".into()),
            ("TAIDE_APP_VERSION".into(), "synthetic-version".into()),
        ]
    );
    let view = native.hub().get(&id).unwrap();
    host.submit(HostCommand::CloseTab {
        tab,
        discard: false,
    })
    .unwrap();
    let reply = timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = host.poll() {
                break reply;
            }
            signal.notified().await;
        }
    })
    .await
    .unwrap();
    let HostReply::Closed { result, .. } = reply else {
        panic!("unexpected close reply")
    };
    result.unwrap();
    assert!(view.is_finished());
    assert!(native.hub().get(&id).is_none());
    assert!(
        fixture
            .services
            .terminal
            .sessions_for_project(&fixture.project)
            .is_empty()
    );
    if let Some(stopped) = fixture.services.ide.take_shutdown_state() {
        stopped.server_handle.unwrap().await.unwrap();
    }
    timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn live_sync_deadline과_실제_resize는_같은_core와_frame_순서를_유지한다() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use taide_native_terminal::GridDimensions;
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let updates = Arc::new(AtomicUsize::new(0));
    let observed = updates.clone();
    let mut ports = output.ports();
    ports.updated = Arc::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { vec![("TAIDE_NATIVE_FIXTURE_LIVE_SYNC".into(), "1".into())] },
            ports,
        )
        .await
        .unwrap();
    let session = hub.get(&id).unwrap();
    output.wait_ready().await;
    let InputResult::Write(receipt) = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("sync\n"),
            BYTES,
        )
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap();
    timeout(TIMEOUT, async {
        loop {
            if output
                .titles
                .lock()
                .unwrap()
                .iter()
                .any(|title| title == "native-live")
            {
                break;
            }
            output.ready.notified().await;
        }
    })
    .await
    .unwrap();
    session
        .snapshot(|state| {
            assert_eq!(state.phase, Phase::Running);
            assert_eq!(state.core.sync_deadline(), None);
            assert!(
                state
                    .core
                    .content()
                    .unwrap()
                    .display_iter
                    .any(|cell| cell.c == 'l')
            );
        })
        .unwrap();
    let size = Size {
        columns: RESIZED_COLUMNS,
        rows: RESIZED_ROWS,
    };
    assert!(
        timeout(TIMEOUT, session.resize(&fixture.services, size))
            .await
            .unwrap()
            .unwrap()
    );
    assert!(
        !timeout(TIMEOUT, session.resize(&fixture.services, size))
            .await
            .unwrap()
            .unwrap()
    );
    assert!(
        session
            .resize(
                &fixture.services,
                Size {
                    columns: 0,
                    rows: 0
                }
            )
            .await
            .is_err()
    );
    session
        .snapshot(|state| {
            assert_eq!(state.phase, Phase::Running);
            assert_eq!(
                state.core.grid().unwrap().columns(),
                usize::from(RESIZED_COLUMNS)
            );
            assert_eq!(
                state.core.grid().unwrap().screen_lines(),
                usize::from(RESIZED_ROWS)
            );
        })
        .unwrap();
    let InputResult::Write(receipt) = session
        .input(
            &fixture.services,
            NativeInput::CommittedText("continue\n"),
            BYTES,
        )
        .unwrap()
    else {
        panic!("fixture input was not encoded")
    };
    timeout(TIMEOUT, receipt.wait()).await.unwrap().unwrap();
    timeout(TIMEOUT, session.wait_dispatch())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.failure(), None);
    assert!(updates.load(Ordering::SeqCst) > 1);
    assert!(session.resize(&fixture.services, size).await.is_err());
    session
        .snapshot(|state| {
            assert_eq!(state.phase, Phase::Exited(Some(0)));
            assert!(state.core.grid().is_ok());
        })
        .unwrap();
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    fixture.finish().await;
}

#[tokio::test]
async fn 포커스없는_단일_view는_측정_크기로_resize를_요청하고_공유_view는_focus_owner를_유지한다() {
    use eframe::egui::{self, Rect, pos2, vec2};
    use taide_model::ids::{PaneId, TabId};
    use taide_native_app::{
        host::HostCommand,
        terminal_surface::{Request, Views},
    };
    const SCREEN: [f32; 2] = [640.0, 320.0];
    const SOLE_VIEW: [f32; 2] = [400.0, 200.0];
    const SHARED_VIEW_LEFT: f32 = 420.0;
    const SHARED_VIEW: [f32; 2] = [200.0, 100.0];
    let fixture = Fixture::new();
    let hub = Hub::new(fixture.services.clone(), limits()).unwrap();
    let output = Output::default();
    let id = hub
        .spawn(
            fixture.opts(),
            HISTORY,
            async { Vec::new() },
            output.ports(),
        )
        .await
        .unwrap();
    output.wait_ready().await;
    let context = egui::Context::default();
    let mut views = Views::default();
    let mut commands = Vec::new();
    let appearance = terminal_appearance();
    let locale = taide_model::locale::ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: Default::default(),
        warnings: Vec::new(),
    };
    let sole = (
        PaneId::new(),
        TabId::new(),
        Rect::from_min_size(pos2(0.0, 0.0), vec2(SOLE_VIEW[0], SOLE_VIEW[1])),
    );
    let shared = (
        PaneId::new(),
        TabId::new(),
        Rect::from_min_size(
            pos2(SHARED_VIEW_LEFT, 0.0),
            vec2(SHARED_VIEW[0], SHARED_VIEW[1]),
        ),
    );
    let frame = |shown: &[(&(PaneId, TabId, Rect), bool)],
                 views: &mut Views,
                 commands: &mut Vec<HostCommand>| {
        let mut rendered = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                ..Default::default()
            },
            |ui| {
                for ((pane, tab, rect), request_focus) in shown {
                    ui.scope_builder(egui::UiBuilder::new().max_rect(*rect), |ui| {
                        views
                            .show(
                                ui,
                                Request {
                                    pane,
                                    tab,
                                    session_id: &id,
                                    hub: &hub,
                                    services: &fixture.services,
                                    appearance: &appearance,
                                    locale: &locale,
                                    request_focus: *request_focus,
                                    commands: &mut *commands,
                                },
                            )
                            .unwrap();
                    });
                }
            },
        );
        rendered.textures_delta.clear();
    };
    frame(&[(&sole, false)], &mut views, &mut commands);
    assert_eq!(context.memory(|memory| memory.focused()), None);
    let [
        HostCommand::ResizeTerminal {
            session,
            size: sole_size,
        },
    ] = commands.as_slice()
    else {
        panic!("unfocused sole view did not request its measured size")
    };
    assert_eq!(session, &id);
    let sole_size = *sole_size;
    assert_ne!(
        sole_size,
        Size {
            columns: COLUMNS,
            rows: ROWS
        }
    );
    commands.clear();
    views.resized(&id);
    frame(
        &[(&sole, false), (&shared, false)],
        &mut views,
        &mut commands,
    );
    commands.clear();
    views.resized(&id);
    frame(
        &[(&sole, false), (&shared, false)],
        &mut views,
        &mut commands,
    );
    assert!(commands.is_empty());
    frame(
        &[(&sole, false), (&shared, true)],
        &mut views,
        &mut commands,
    );
    let [
        HostCommand::ResizeTerminal {
            session,
            size: owner_size,
        },
    ] = commands.as_slice()
    else {
        panic!("focused view of a shared session did not own the size")
    };
    assert_eq!(session, &id);
    assert_ne!(*owner_size, sole_size);
    timeout(TIMEOUT, hub.close(&id)).await.unwrap().unwrap();
    fixture.finish().await;
}
