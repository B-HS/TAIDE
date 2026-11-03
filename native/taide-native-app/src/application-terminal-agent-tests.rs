use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, pointer, project_ref, step,
};
use super::*;
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use taide_ide::protocol::{RPC_INVALID_PARAMS, ToolError};
use taide_model::agent::{AgentActivity, DetectedAgent};
use taide_model::ids::ShellSlotId;
use taide_model::layout::{AuxWindowLayout, PaneNode, SplitDir};
use taide_model::project::{ShellSlotTree, WindowChromePatch};
use taide_native_terminal::input::{Key, Modifiers, NativeInput};
use taide_native_ui::shell::{TAB_HEIGHT, TITLE_HEIGHT};

const DEADLINE: Duration = Duration::from_secs(10);
const INPUT_BYTES: usize = 4096;
const EXECUTABLE_MODE: u32 = 0o700;
const CENTER_TOLERANCE: f32 = 1.0;
const COMMAND_GUTTER_WIDTH: f32 = 2.0;
const SETTLE_FRAMES: usize = 4;
const QUIET_READY: &str = "quiet-ready";
const AGENT_GLYPH_TEXTURE_PREFIX: &str = "native-glyph-Sparkles-";
const MAIN_AGENT_SESSION: &str = "synthetic-main-agent-session";
const AUXILIARY_AGENT_SESSION: &str = "synthetic-auxiliary-agent-session";
const AUXILIARY_TERMINAL_TITLE: &str = "auxiliary-agent";
const AUXILIARY_SLOT: u32 = 1;
const AUXILIARY_SCREEN: [f32; 2] = [1000.0, 700.0];
const SYNTHETIC_SOURCE: &str = "fn synthetic() {}\n";
const KEPT_TITLE: &str = "보관.rs";
const MISSING_TITLE: &str = "없는 탭";
const DIRTY_EDIT: &str = "changed ";
const CLOSE_TAB_EVENT: &str = "ide:close-tab-requested";
const DIFF_TITLES: [&str; 2] = ["first-diff", "second-diff"];
const CLI_BUNDLE_DIR: &str = "TAIDE.app/Contents/MacOS";
const CLI_APP_EXECUTABLE: &str = "taide-native";
const CLI_SIDECAR: &str = "taide-cli";
const CLI_LINK_DIR: &str = "cli-link";
const CLI_LINK_NAME: &str = "taide";
const CLI_INSTALL: &str = "cli.installShellCommand";
const CLI_UNINSTALL: &str = "cli.uninstallShellCommand";
const CLI_CONNECT: &str = "cli.connectExternalEditor";
const TASK_PACKAGE_MANIFEST: &str = r#"{"scripts":{"build":"synthetic-build"}}"#;
const TASK_NPM_SCRIPT: &str = "build";
const TASK_MAKEFILE: &str = "lint:\n\ttrue\n";
const TASK_CARGO_MANIFEST: &str = "[package]\nname = \"synthetic\"\n";
const CARGO_CHECK: &str = "cargo check";
const TASK_ROWS: [&str; 7] = [
    TASK_NPM_SCRIPT,
    "lint",
    "cargo build",
    "cargo test",
    "cargo run",
    CARGO_CHECK,
    "cargo clippy",
];
const AUXILIARY_FILE: &str = "auxiliary.rs";

fn command_shift() -> egui::Modifiers {
    egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND | egui::Modifiers::SHIFT
}

struct Terminals {
    fixture: Fixture,
    context: egui::Context,
    project: ProjectId,
    tabs: Vec<TabId>,
}

fn launch_terminals(name: &str, count: usize) -> Terminals {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-native-{name}-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("project");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "terminals", false);
    let tabs = (0..count)
        .map(|index| Tab {
            id: TabId::new(),
            kind: TabKind::Terminal {
                session_id: String::new(),
                cwd: None,
            },
            title: format!("terminal-{index}"),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        })
        .collect::<Vec<_>>();
    let ids = tabs.iter().map(|tab| tab.id.clone()).collect::<Vec<_>>();
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        let PaneNode::Leaf {
            tabs: leaf, active, ..
        } = &mut layout.root
        else {
            panic!("expected leaf")
        };
        *active = Some(ids[0].clone());
        *leaf = tabs;
        layout.shell_view.sidebar_collapsed = true;
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&project, &root, "terminals")];
        session.active_project = Some(project.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: project.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    Terminals {
        fixture,
        context,
        project,
        tabs: ids,
    }
}

fn quiet_shell(directory: &std::path::Path) -> String {
    let path = directory.join("quiet-shell");
    std::fs::write(
        &path,
        format!("#!/bin/sh\nstty -echo\nprintf '{QUIET_READY}\\n'\nexec /bin/cat\n"),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    path.to_str().unwrap().into()
}

fn step_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication, &egui::FullOutput) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = step(application, context, Vec::new());
        if ready(application, &output) {
            return output;
        }
        assert!(
            Instant::now() < deadline,
            "phase={phase}; status={:?}",
            application.status
        );
        std::thread::yield_now();
    }
}

fn session(
    application: &NativeApplication,
    project: &ProjectId,
    tab: &TabId,
) -> Option<Arc<crate::terminal_host::Session>> {
    let layouts = application.services.state.layouts.read();
    crate::tabs::tabs_in(&layouts.get(project)?.root)
        .into_iter()
        .find(|candidate| &candidate.id == tab)
        .and_then(|tab| match &tab.kind {
            TabKind::Terminal { session_id, .. } => application
                .terminal_agent_state
                .terminals
                .hub()
                .get(session_id),
            _ => None,
        })
}

fn screen_text(session: &crate::terminal_host::Session) -> String {
    session
        .snapshot(|snapshot| {
            snapshot
                .core
                .content()
                .unwrap()
                .display_iter
                .map(|cell| cell.c)
                .collect()
        })
        .unwrap()
}

fn activate(application: &mut NativeApplication, context: &egui::Context, tab: &TabId) {
    application
        .runtime
        .handle()
        .clone()
        .block_on(taide_runtime::layout_actions::layout_activate_tab(
            application.services.events.as_ref(),
            &application.services.state,
            tab.clone(),
        ))
        .unwrap();
    for _ in 0..SETTLE_FRAMES {
        step(application, context, Vec::new());
    }
}

fn type_line(
    session: &crate::terminal_host::Session,
    services: &taide_runtime::AppServices,
    line: &str,
) {
    session
        .input(services, NativeInput::CommittedText(line), INPUT_BYTES)
        .unwrap();
}

fn text_shapes(output: &egui::FullOutput) -> Vec<&egui::epaint::TextShape> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

fn text_rect(text: &egui::epaint::TextShape) -> egui::Rect {
    text.galley.rect.translate(text.pos.to_vec2())
}

fn text_color(text: &egui::epaint::TextShape) -> egui::Color32 {
    let color = text.galley.job.sections[0].format.color;
    if color == egui::Color32::PLACEHOLDER {
        text.fallback_color
    } else {
        color
    }
}

fn status_message(
    output: &egui::FullOutput,
    restart: &str,
) -> Option<(String, egui::Color32, egui::Pos2)> {
    let texts = text_shapes(output);
    let button = text_rect(texts.iter().find(|text| text.galley.text() == restart)?).center();
    texts
        .iter()
        .filter(|text| text.galley.text() != restart)
        .filter(|text| {
            let rect = text_rect(text);
            (rect.center().x - button.x).abs() < CENTER_TOLERANCE && rect.bottom() <= button.y
        })
        .max_by(|left, right| {
            text_rect(left)
                .bottom()
                .total_cmp(&text_rect(right).bottom())
        })
        .map(|text| (text.galley.text().to_owned(), text_color(text), button))
}

fn gutters(output: &egui::FullOutput, colors: &[egui::Color32]) -> Vec<(i32, egui::Color32)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect)
                if rect.rect.width() == COMMAND_GUTTER_WIDTH && colors.contains(&rect.fill) =>
            {
                Some((rect.rect.top().round() as i32, rect.fill))
            }
            _ => None,
        })
        .collect()
}

fn rgb_color(color: Option<taide_native_terminal::Rgb>) -> egui::Color32 {
    let color = color.unwrap();
    egui::Color32::from_rgb(color.r, color.g, color.b)
}

#[test]
fn 실제_터미널_실행실패_화면은_원본_오류색과_재시작_종료색을_보존한다() {
    let Terminals {
        mut fixture,
        context,
        project,
        tabs,
    } = launch_terminals("terminal-failure", 1);
    let missing = fixture.directory.join("missing-shell");
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().shell_override =
        Some(missing.to_str().unwrap().into());
    let restart = presentation::message(&application.locale, "terminal.restart", &[]);
    let output = step_until(
        application,
        &context,
        "spawn failure",
        |application, output| {
            application
                .toasts
                .inspection()
                .iter()
                .any(|toast| toast.kind == taide_native_ui::toast::Kind::Error)
                && status_message(output, &restart).is_some()
        },
    );
    let toast = application
        .toasts
        .inspection()
        .into_iter()
        .find(|toast| toast.kind == taide_native_ui::toast::Kind::Error)
        .unwrap();
    let (message, color, button) = status_message(&output, &restart).unwrap();
    assert_eq!(message, toast.title);
    let theme = &application.settings_state.resolved_theme;
    assert_eq!(
        color,
        presentation::color(theme, "statusIndicator.error").unwrap()
    );
    assert!(session(application, &project, &tabs[0]).is_none());

    application.services.state.settings.write().shell_override = Some("/bin/cat".into());
    step(
        application,
        &context,
        pointer(button, egui::PointerButton::Primary),
    );
    step_until(
        application,
        &context,
        "restarted session",
        |application, output| {
            session(application, &project, &tabs[0]).is_some()
                && status_message(output, &restart).is_none()
        },
    );
    let restarted = session(application, &project, &tabs[0]).unwrap();
    type_line(&restarted, &application.services, "abc\n");
    step_until(application, &context, "restarted echo", |_, _| {
        screen_text(&restarted).contains("abc")
    });

    restarted
        .input(
            &application.services,
            NativeInput::Key {
                key: Key::Character('d'),
                modifiers: Modifiers {
                    control: true,
                    ..Default::default()
                },
            },
            INPUT_BYTES,
        )
        .unwrap();
    let exited = presentation::message(&application.locale, "terminal.processExited", &[]);
    let output = step_until(application, &context, "exit screen", |_, output| {
        status_message(output, &restart).is_some_and(|(message, ..)| message.starts_with(&exited))
    });
    let (_, color, _) = status_message(&output, &restart).unwrap();
    assert_eq!(
        color,
        presentation::color(
            &application.settings_state.resolved_theme,
            "appSidebar.iconDefault"
        )
        .unwrap()
    );
}

#[test]
fn 실제_터미널_실행중_설정은_scrollback_커서_글자크기_셸을_반영한다() {
    use taide_model::settings::TerminalCursorStyle;
    use taide_native_terminal::{CursorShape, CursorStyle, GridDimensions};
    const LIVE_SCROLLBACK: u32 = 50;
    const LARGER_FONT_SIZE: u32 = 24;
    let Terminals {
        mut fixture,
        context,
        project,
        tabs,
    } = launch_terminals("terminal-settings", 2);
    let shell = quiet_shell(&fixture.directory);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().shell_override = Some("/bin/cat".into());
    step_until(application, &context, "first terminal", |application, _| {
        session(application, &project, &tabs[0]).is_some()
    });
    let first = session(application, &project, &tabs[0]).unwrap();
    assert_ne!(
        first.snapshot(|snapshot| snapshot.core.history()).unwrap(),
        LIVE_SCROLLBACK as usize
    );

    application
        .services
        .state
        .settings
        .write()
        .terminal_scrollback = LIVE_SCROLLBACK;
    step_until(application, &context, "live scrollback", |_, _| {
        first.snapshot(|snapshot| snapshot.core.history()).unwrap() == LIVE_SCROLLBACK as usize
    });

    {
        let mut settings = application.services.state.settings.write();
        settings.terminal_cursor_style = TerminalCursorStyle::Block;
        settings.terminal_cursor_blink = false;
    }
    step_until(application, &context, "live cursor", |_, _| {
        first
            .snapshot(|snapshot| snapshot.core.cursor_style().unwrap())
            .unwrap()
            == CursorStyle {
                shape: CursorShape::Block,
                blinking: false,
            }
    });

    let columns = || {
        first
            .snapshot(|snapshot| snapshot.core.grid().unwrap().columns())
            .unwrap()
    };
    let before = columns();
    application
        .services
        .state
        .settings
        .write()
        .terminal_font_size = LARGER_FONT_SIZE;
    step_until(application, &context, "live font size", |application, _| {
        application
            .terminal_agent_state
            .terminal_appearance
            .font
            .size
            == LARGER_FONT_SIZE as f32
            && columns() < before
    });

    application.services.state.settings.write().shell_override = Some(shell.clone());
    for _ in 0..SETTLE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert_eq!(first.metadata().shell(), "/bin/cat");
    activate(application, &context, &tabs[1]);
    step_until(
        application,
        &context,
        "second terminal",
        |application, _| session(application, &project, &tabs[1]).is_some(),
    );
    let second = session(application, &project, &tabs[1]).unwrap();
    assert_eq!(second.metadata().shell(), shell);
    assert_eq!(
        second.snapshot(|snapshot| snapshot.core.history()).unwrap(),
        LIVE_SCROLLBACK as usize
    );
    assert_eq!(first.metadata().shell(), "/bin/cat");
}

#[test]
fn 실제_터미널_명령_장식은_탭_전환에_남고_숨은_세션은_새_테마_색을_쓴다() {
    const NEXT_SUCCESS: &str = "#123456";
    const NEXT_FAILURE: &str = "#654321";
    let Terminals {
        mut fixture,
        context,
        project,
        tabs,
    } = launch_terminals("terminal-commands", 2);
    let shell = quiet_shell(&fixture.directory);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().shell_override = Some(shell);
    step_until(application, &context, "first terminal", |application, _| {
        session(application, &project, &tabs[0])
            .is_some_and(|first| screen_text(&first).contains(QUIET_READY))
    });
    let first = session(application, &project, &tabs[0]).unwrap();
    let initial = application
        .terminal_agent_state
        .terminal_appearance
        .command_colors();
    let initial_success = rgb_color(initial.success);
    let next_success = presentation::parse_color(NEXT_SUCCESS, "statusIndicator.success").unwrap();
    let next_failure = presentation::parse_color(NEXT_FAILURE, "statusIndicator.error").unwrap();
    let watched = [initial_success, next_success, next_failure];
    type_line(
        &first,
        &application.services,
        "\x1b]133;A\x07\x1b]133;C\x07first\x1b]133;D;0\x07\n",
    );
    let shown = step_until(application, &context, "first gutter", |_, output| {
        !gutters(output, &watched).is_empty()
    });
    let before = gutters(&shown, &watched);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].1, initial_success);

    activate(application, &context, &tabs[1]);
    step_until(
        application,
        &context,
        "second terminal",
        |application, _| session(application, &project, &tabs[1]).is_some(),
    );
    let mut theme = application.settings_state.resolved_theme.clone();
    theme
        .colors
        .insert("statusIndicator.success".into(), NEXT_SUCCESS.into());
    theme
        .colors
        .insert("statusIndicator.error".into(), NEXT_FAILURE.into());
    application.settings_state.resolved_theme = theme.clone();
    application.settings_state.preview_theme = Some(theme);
    step_until(application, &context, "theme refresh", |application, _| {
        application
            .terminal_agent_state
            .terminal_appearance
            .command_colors()
            != initial
    });
    step(application, &context, Vec::new());
    type_line(
        &first,
        &application.services,
        "\x1b]133;A\x07\x1b]133;C\x07hidden\x1b]133;D;1\x07\n",
    );
    step_until(application, &context, "hidden command", |_, _| {
        first
            .snapshot(|snapshot| snapshot.core.command_blocks().unwrap().len())
            .unwrap()
            == 2
    });

    activate(application, &context, &tabs[0]);
    let returned = step_until(application, &context, "returned gutters", |_, output| {
        gutters(output, &watched).len() == 2
    });
    let after = gutters(&returned, &watched);
    assert_eq!(after[0], before[0]);
    assert_eq!(after[1].1, next_failure);
}

type ViewportOutputs = std::rc::Rc<std::cell::RefCell<HashMap<egui::ViewportId, egui::FullOutput>>>;

fn record_viewports(context: &egui::Context) -> ViewportOutputs {
    let outputs = ViewportOutputs::default();
    let recorded = outputs.clone();
    context.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
        let id = viewport.ids.this;
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: id,
                viewports: std::iter::once((id, egui::ViewportInfo::default())).collect(),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(AUXILIARY_SCREEN[0], AUXILIARY_SCREEN[1]),
                )),
                ..Default::default()
            },
            |ui| (viewport.viewport_ui_cb)(ui),
        );
        output.textures_delta.clear();
        recorded.borrow_mut().insert(id, output);
    });
    outputs
}

fn workspace_fixture(name: &str) -> Fixture {
    let fixture = Fixture {
        directory: std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("taide-native-{name}-{}", ProjectId::new())),
        application: None,
    };
    std::fs::create_dir_all(fixture.directory.join("data")).unwrap();
    fixture
}

fn workspace_tab(title: &str, kind: TabKind) -> Tab {
    Tab {
        id: TabId::new(),
        kind,
        title: title.into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

fn file_tab(path: &std::path::Path) -> Tab {
    workspace_tab(
        path.file_name().unwrap().to_str().unwrap(),
        TabKind::File {
            path: path.to_str().unwrap().into(),
        },
    )
}

fn terminal_tab(title: &str, session_id: &str) -> Tab {
    workspace_tab(
        title,
        TabKind::Terminal {
            session_id: session_id.into(),
            cwd: None,
        },
    )
}

fn place_tabs(state: &AppState, project: &ProjectId, tabs: Vec<Tab>, active: &TabId) {
    let mut layouts = state.layouts.write();
    let layout = layouts.get_mut(project).unwrap();
    let PaneNode::Leaf {
        tabs: leaf,
        active: current,
        ..
    } = &mut layout.root
    else {
        panic!("expected leaf")
    };
    *current = Some(active.clone());
    *leaf = tabs;
}

fn focus_single_slot(state: &AppState, project: &ProjectId, root: &std::path::Path, name: &str) {
    let slot = ShellSlotId::new();
    let mut session = state.session.write();
    session.projects = vec![project_ref(project, root, name)];
    session.active_project = Some(project.clone());
    session.shell_slots = Some(ShellSlotTree::Leaf {
        slot_id: slot.clone(),
        project_id: project.clone(),
    });
    session.focused_shell_slot = Some(slot);
}

fn tab_bar() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(f32::INFINITY, TITLE_HEIGHT + TAB_HEIGHT),
    )
}

fn agent_glyph_colors(
    context: &egui::Context,
    output: &egui::FullOutput,
    within: egui::Rect,
) -> Vec<egui::Color32> {
    let textures = context
        .tex_manager()
        .read()
        .allocated()
        .filter(|(_, meta)| meta.name.starts_with(AGENT_GLYPH_TEXTURE_PREFIX))
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh)
                if textures.contains(&mesh.texture_id)
                    && within.contains(mesh.calc_bounds().center()) =>
            {
                mesh.vertices.first().map(|vertex| vertex.color)
            }
            _ => None,
        })
        .collect()
}

fn auxiliary_agent_colors(
    context: &egui::Context,
    outputs: &ViewportOutputs,
    viewport: egui::ViewportId,
) -> Vec<egui::Color32> {
    outputs
        .borrow()
        .get(&viewport)
        .map(|output| agent_glyph_colors(context, output, egui::Rect::EVERYTHING))
        .unwrap_or_default()
}

fn synthetic_agents(activity: AgentActivity) -> Vec<DetectedAgent> {
    [MAIN_AGENT_SESSION, AUXILIARY_AGENT_SESSION]
        .into_iter()
        .map(|session_id| DetectedAgent {
            session_id: session_id.into(),
            name: "claude".into(),
            pid: 1,
            activity,
            blocked_reason: None,
        })
        .collect()
}

fn record_commands(output: egui::FullOutput, commands: &mut Vec<egui::ViewportCommand>) {
    commands.extend(
        output
            .viewport_output
            .into_values()
            .flat_map(|viewport| viewport.commands),
    );
}

fn step_until_collecting(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication, &egui::FullOutput) -> bool,
    commands: &mut Vec<egui::ViewportCommand>,
) {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = step(application, context, Vec::new());
        let done = ready(application, &output);
        record_commands(output, commands);
        if done {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "phase={phase}; status={:?}",
            application.status
        );
        std::thread::yield_now();
    }
}

fn call_ide_tool(
    application: &mut NativeApplication,
    context: &egui::Context,
    name: &'static str,
    arguments: Value,
    commands: &mut Vec<egui::ViewportCommand>,
) -> Result<Value, ToolError> {
    let services = application.services.clone();
    let actions = application.application_ports.ide.actions.clone();
    let call = application.runtime.spawn(async move {
        crate::ide_tools::dispatch(services, &actions, name, &arguments).await
    });
    let deadline = Instant::now() + DEADLINE;
    while !call.is_finished() {
        record_commands(step(application, context, Vec::new()), commands);
        assert!(Instant::now() < deadline, "ide tool {name}");
        std::thread::yield_now();
    }
    application.runtime.block_on(call).unwrap()
}

fn tool_text(result: &Value) -> &str {
    result["content"][0]["text"].as_str().unwrap()
}

fn expect_tool(result: Result<Value, ToolError>) -> Value {
    result.unwrap_or_else(|error| panic!("{}: {}", error.code, error.message))
}

fn is_window_focus(command: &egui::ViewportCommand) -> bool {
    matches!(
        command,
        egui::ViewportCommand::Focus | egui::ViewportCommand::Visible(true)
    )
}

fn file_view(application: &NativeApplication, path: &str, tab: &TabId) -> Option<ViewId> {
    let document = application.files.get(path)?.id;
    application
        .store
        .views()
        .for_document(document)
        .find(|view| &view.key.tab == tab)
        .map(|view| view.id)
}

fn active_file_view(
    application: &NativeApplication,
    project: &ProjectId,
    path: &str,
) -> Option<ViewId> {
    let snapshot = application.controller.snapshot();
    let PaneNode::Leaf {
        tabs,
        active: Some(active),
        ..
    } = &snapshot.layouts.get(project)?.root
    else {
        return None;
    };
    let tab = tabs.iter().find(|tab| {
        &tab.id == active && matches!(&tab.kind, TabKind::File { path: open } if open == path)
    })?;
    file_view(application, path, &tab.id)
}

fn body_marker(path: &std::path::Path) -> String {
    format!("{}_body", path.file_stem().unwrap().to_str().unwrap())
}

fn paints_body(output: &egui::FullOutput, path: &std::path::Path) -> bool {
    let marker = body_marker(path);
    text_shapes(output)
        .iter()
        .any(|text| text.galley.text().contains(&marker))
}

fn focused_file(application: &NativeApplication) -> Option<String> {
    let snapshot = application.controller.snapshot();
    match &snapshot.focused_tab()?.kind {
        TabKind::File { path } => Some(path.clone()),
        _ => None,
    }
}

fn file_tab_paths(application: &NativeApplication, project: &ProjectId) -> Vec<String> {
    let layouts = application.services.state.layouts.read();
    let mut paths = crate::tabs::tabs_in(&layouts[project].root)
        .into_iter()
        .filter_map(|tab| match &tab.kind {
            TabKind::File { path } => Some(path.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn snapshot_tabs(application: &NativeApplication, project: &ProjectId) -> Vec<Tab> {
    let snapshot = application.controller.snapshot();
    crate::tabs::tabs_in(&snapshot.layouts[project].root)
        .into_iter()
        .cloned()
        .collect()
}

fn close_tab_requests(
    frames: &mut tokio::sync::broadcast::Receiver<String>,
) -> Vec<(String, Option<String>)> {
    let mut requests = Vec::new();
    loop {
        let frame = match frames.try_recv() {
            Ok(frame) => frame,
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
            Err(_) => return requests,
        };
        let frame: Value = serde_json::from_str(&frame).unwrap();
        if frame["event"] == CLOSE_TAB_EVENT {
            let payload: Value = serde_json::from_str(frame["payload"].as_str().unwrap()).unwrap();
            requests.push((
                payload["tabName"].as_str().unwrap().to_owned(),
                payload["requestId"].as_str().map(str::to_owned),
            ));
        }
    }
}

#[test]
fn 실제_app_agent_상태_변경은_zen_메인창과_보조창_탭바에_다시_그려진다() {
    let probe = egui::Context::default();
    let sink = PaintSink {
        context: probe.clone(),
        trees: Arc::default(),
        file_indexes: Arc::default(),
        presentation: Arc::default(),
        git_cache: Arc::default(),
        app_menu: Arc::default(),
        providers: Arc::default(),
        settings: Arc::default(),
    };
    let mut fixture = workspace_fixture("agent-display");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    let main_file = root.join("main.rs");
    let auxiliary_file = root.join("auxiliary.rs");
    for path in [&main_file, &auxiliary_file] {
        std::fs::write(path, SYNTHETIC_SOURCE).unwrap();
    }
    let project = ProjectId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &project, &root, "alpha", false);
    let main_tabs = vec![
        file_tab(&main_file),
        terminal_tab("main-agent", MAIN_AGENT_SESSION),
    ];
    let main_active = main_tabs[0].id.clone();
    place_tabs(&state, &project, main_tabs, &main_active);
    let auxiliary_tabs = vec![
        file_tab(&auxiliary_file),
        terminal_tab(AUXILIARY_TERMINAL_TITLE, AUXILIARY_AGENT_SESSION),
    ];
    let auxiliary_active = auxiliary_tabs[0].id.clone();
    let auxiliary_pane = PaneId::new();
    state
        .layouts
        .write()
        .get_mut(&project)
        .unwrap()
        .auxiliary_windows = vec![AuxWindowLayout {
        slot: AUXILIARY_SLOT,
        root: PaneNode::Leaf {
            id: auxiliary_pane.clone(),
            tabs: auxiliary_tabs,
            active: Some(auxiliary_active),
        },
        focused_pane: auxiliary_pane,
    }];
    focus_single_slot(&state, &project, &root, "alpha");
    state.session.write().window_chrome.zen = true;
    let context = launch(&mut fixture, state);
    let viewports = record_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let viewport = application
        .shell_state
        .auxiliary_windows
        .find(&project, AUXILIARY_SLOT)
        .unwrap()
        .viewport;
    let theme = application.settings_state.resolved_theme.clone();
    let working = presentation::color(&theme, "appSidebar.iconAgentWorking").unwrap();
    let idle = presentation::color(&theme, "appSidebar.iconAgentIdle").unwrap();
    assert_ne!(working, idle);

    let output = step_until(application, &context, "auxiliary tab bar", |_, _| {
        viewports.borrow().get(&viewport).is_some_and(|output| {
            text_shapes(output)
                .iter()
                .any(|text| text.galley.text() == AUXILIARY_TERMINAL_TITLE)
        })
    });
    assert!(application.controller.snapshot().shell.window_chrome.zen);
    assert!(auxiliary_agent_colors(&context, &viewports, viewport).is_empty());
    assert!(agent_glyph_colors(&context, &output, egui::Rect::EVERYTHING).is_empty());

    let output = step_until(
        application,
        &context,
        "auxiliary working",
        |application, _| {
            application
                .services
                .agents
                .diff(&project, &synthetic_agents(AgentActivity::Working));
            auxiliary_agent_colors(&context, &viewports, viewport) == [working]
        },
    );
    assert!(agent_glyph_colors(&context, &output, egui::Rect::EVERYTHING).is_empty());

    let idle_event = || AppEvent::AgentStateChanged {
        project_id: project.clone(),
        agents: synthetic_agents(AgentActivity::Idle),
    };
    let run_probe = || {
        probe
            .run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear()
    };
    for _ in 0..SETTLE_FRAMES {
        if !probe.has_requested_repaint() {
            break;
        }
        run_probe();
    }
    assert!(!probe.has_requested_repaint());
    sink.publish(idle_event());
    assert!(probe.has_requested_repaint());
    run_probe();
    let event_repaint = probe.repaint_causes();
    assert_eq!(event_repaint.len(), 1);
    let published = std::cell::Cell::new(false);
    step_until(application, &context, "event repaint", |application, _| {
        let repainted = published.get() && context.repaint_causes().contains(&event_repaint[0]);
        application
            .services
            .agents
            .diff(&project, &synthetic_agents(AgentActivity::Idle));
        application.services.events.publish(idle_event());
        published.set(true);
        repainted
    });
    let output = step_until(application, &context, "auxiliary idle", |application, _| {
        application
            .services
            .agents
            .diff(&project, &synthetic_agents(AgentActivity::Idle));
        auxiliary_agent_colors(&context, &viewports, viewport) == [idle]
    });
    assert!(agent_glyph_colors(&context, &output, egui::Rect::EVERYTHING).is_empty());

    application
        .controller
        .submit(ShellMutation::SetWindowChrome(WindowChromePatch {
            zen: Some(false),
            ..Default::default()
        }))
        .unwrap();
    step_until(application, &context, "main idle", |application, output| {
        application
            .services
            .agents
            .diff(&project, &synthetic_agents(AgentActivity::Idle));
        !application.controller.snapshot().shell.window_chrome.zen
            && agent_glyph_colors(&context, output, tab_bar()) == [idle]
            && auxiliary_agent_colors(&context, &viewports, viewport) == [idle]
    });
}

#[test]
fn 실제_app_ide_openfile은_대상_프로젝트_포커스_pane에_본문을_그리고_창을_앞으로_가져오지_않는다() {
    let mut fixture = workspace_fixture("ide-open-file");
    let alpha_root = fixture.directory.join("alpha");
    let beta_root = fixture.directory.join("beta");
    for directory in [&alpha_root, &beta_root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let files = [
        alpha_root.join("first.rs"),
        alpha_root.join("second.rs"),
        alpha_root.join("third.rs"),
        beta_root.join("beta.rs"),
        fixture.directory.join("outside.rs"),
    ];
    for path in &files {
        std::fs::write(path, format!("fn {}() {{}}\n", body_marker(path))).unwrap();
    }
    let [first, second, third, beta_file, outside] = files
        .each_ref()
        .map(|path| path.to_str().unwrap().to_owned());
    let alpha = ProjectId::new();
    let beta = ProjectId::new();
    let alpha_slot = ShellSlotId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &alpha, &alpha_root, "alpha", false);
    insert_project(&state, &beta, &beta_root, "beta", false);
    let opened = file_tab(&files[0]);
    let opened_id = opened.id.clone();
    place_tabs(&state, &alpha, vec![opened], &opened_id);
    {
        let mut session = state.session.write();
        session.projects = vec![
            project_ref(&alpha, &alpha_root, "alpha"),
            project_ref(&beta, &beta_root, "beta"),
        ];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Split {
            dir: SplitDir::Horizontal,
            children: vec![
                ShellSlotTree::Leaf {
                    slot_id: alpha_slot.clone(),
                    project_id: alpha.clone(),
                },
                ShellSlotTree::Leaf {
                    slot_id: ShellSlotId::new(),
                    project_id: beta.clone(),
                },
            ],
            sizes: vec![1.0, 1.0],
        });
        session.focused_shell_slot = Some(alpha_slot.clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let mut commands = Vec::new();
    step_until_collecting(
        application,
        &context,
        "first file",
        |application, output| {
            active_file_view(application, &alpha, &first).is_some()
                && paints_body(output, &files[0])
        },
        &mut commands,
    );
    commands.clear();

    let opened = expect_tool(call_ide_tool(
        application,
        &context,
        "openFile",
        json!({"filePath": second, "preview": false}),
        &mut commands,
    ));
    assert_eq!(tool_text(&opened), format!("Opened file: {second}"));
    step_until_collecting(
        application,
        &context,
        "second file",
        |application, output| {
            active_file_view(application, &alpha, &second).is_some()
                && focused_file(application).as_deref() == Some(second.as_str())
                && paints_body(output, &files[1])
                && !paints_body(output, &files[0])
        },
        &mut commands,
    );

    let opened = expect_tool(call_ide_tool(
        application,
        &context,
        "openFile",
        json!({"filePath": third, "preview": false, "makeFrontmost": false}),
        &mut commands,
    ));
    assert_eq!(
        serde_json::from_str::<Value>(tool_text(&opened)).unwrap(),
        json!({"success": true, "filePath": third, "languageId": "rust"})
    );
    step_until_collecting(
        application,
        &context,
        "third file",
        |application, output| {
            active_file_view(application, &alpha, &third).is_some()
                && focused_file(application).as_deref() == Some(third.as_str())
                && paints_body(output, &files[2])
                && !paints_body(output, &files[1])
        },
        &mut commands,
    );
    let mut expected = vec![first.clone(), second.clone(), third.clone()];
    expected.sort();
    assert_eq!(file_tab_paths(application, &alpha), expected);

    let error = call_ide_tool(
        application,
        &context,
        "openFile",
        json!({"filePath": outside}),
        &mut commands,
    )
    .unwrap_err();
    assert_eq!(error.code, RPC_INVALID_PARAMS);
    for _ in 0..SETTLE_FRAMES {
        record_commands(step(application, &context, Vec::new()), &mut commands);
    }
    assert_eq!(file_tab_paths(application, &alpha), expected);
    assert!(file_tab_paths(application, &beta).is_empty());
    assert_eq!(focused_file(application).as_deref(), Some(third.as_str()));

    let opened = expect_tool(call_ide_tool(
        application,
        &context,
        "openFile",
        json!({"filePath": beta_file, "preview": false}),
        &mut commands,
    ));
    assert_eq!(tool_text(&opened), format!("Opened file: {beta_file}"));
    step_until_collecting(
        application,
        &context,
        "beta file",
        |application, output| {
            active_file_view(application, &beta, &beta_file).is_some()
                && paints_body(output, &files[3])
                && paints_body(output, &files[2])
        },
        &mut commands,
    );
    assert_eq!(file_tab_paths(application, &alpha), expected);
    assert_eq!(focused_file(application).as_deref(), Some(third.as_str()));
    {
        let session = application.services.state.session.read();
        assert_eq!(session.focused_shell_slot.as_ref(), Some(&alpha_slot));
        assert_eq!(session.active_project.as_ref(), Some(&alpha));
    }
    assert!(!commands.iter().any(is_window_focus), "{commands:?}");
}

#[test]
fn 실제_app_ide_close_tab은_dirty_확인없이_닫고_터미널_세션과_대기_diff를_화면에서_정리한다() {
    use taide_ide::store::PendingDiff;
    use taide_model::ide::IdeDiffOutcome;
    let mut fixture = workspace_fixture("ide-close-tab");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    let kept = root.join(KEPT_TITLE);
    let diff_target = root.join("diff.rs");
    for path in [&kept, &diff_target] {
        std::fs::write(path, SYNTHETIC_SOURCE).unwrap();
    }
    let kept_path = kept.to_str().unwrap().to_owned();
    let diff_path = diff_target.to_str().unwrap().to_owned();
    let project = ProjectId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &project, &root, "alpha", false);
    let file = file_tab(&kept);
    let terminal = terminal_tab("terminal-0", "");
    let file_id = file.id.clone();
    let terminal_id = terminal.id.clone();
    place_tabs(&state, &project, vec![file, terminal], &terminal_id);
    focus_single_slot(&state, &project, &root, "alpha");
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().shell_override = Some("/bin/cat".into());
    step_until(
        application,
        &context,
        "terminal session",
        |application, _| session(application, &project, &terminal_id).is_some(),
    );
    let terminal = snapshot_tabs(application, &project)
        .into_iter()
        .find(|tab| tab.id == terminal_id)
        .unwrap();
    let TabKind::Terminal { session_id, .. } = &terminal.kind else {
        panic!("expected terminal")
    };
    activate(application, &context, &file_id);
    step_until(application, &context, "kept file", |application, output| {
        file_view(application, &kept_path, &file_id).is_some()
            && node_rect(output, KEPT_TITLE, tab_bar()).is_some()
    });
    let view = file_view(application, &kept_path, &file_id).unwrap();
    taide_native_editor::editing::replace_selections(
        &mut application.store,
        view,
        DIRTY_EDIT,
        None,
    )
    .unwrap();
    let document = application.files[&kept_path].id;
    assert!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
    let mut frames = application.services.remote.subscribe_events();

    let before = snapshot_tabs(application, &project);
    let closed = expect_tool(call_ide_tool(
        application,
        &context,
        "close_tab",
        json!({"tab_name": MISSING_TITLE}),
        &mut Vec::new(),
    ));
    assert_eq!(tool_text(&closed), "TAB_CLOSED");
    for _ in 0..SETTLE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert_eq!(snapshot_tabs(application, &project), before);
    assert!(close_tab_requests(&mut frames).is_empty());

    let closed = expect_tool(call_ide_tool(
        application,
        &context,
        "close_tab",
        json!({"tab_name": KEPT_TITLE}),
        &mut Vec::new(),
    ));
    assert_eq!(tool_text(&closed), "TAB_CLOSED");
    step_until(
        application,
        &context,
        "dirty closed",
        |application, output| {
            !snapshot_tabs(application, &project)
                .iter()
                .any(|tab| tab.id == file_id)
                && node_rect(output, KEPT_TITLE, tab_bar()).is_none()
        },
    );
    assert!(application.pending_tab_close.is_none());
    assert_eq!(std::fs::read_to_string(&kept).unwrap(), SYNTHETIC_SOURCE);
    assert_eq!(
        close_tab_requests(&mut frames),
        [(KEPT_TITLE.to_owned(), None)]
    );

    let closed = expect_tool(call_ide_tool(
        application,
        &context,
        "close_tab",
        json!({"tab_name": terminal.title}),
        &mut Vec::new(),
    ));
    assert_eq!(tool_text(&closed), "TAB_CLOSED");
    step_until(
        application,
        &context,
        "terminal closed",
        |application, output| {
            application
                .terminal_agent_state
                .terminals
                .hub()
                .get(session_id)
                .is_none()
                && !snapshot_tabs(application, &project)
                    .iter()
                    .any(|tab| tab.id == terminal_id)
                && node_rect(output, &terminal.title, tab_bar()).is_none()
        },
    );
    assert_eq!(
        close_tab_requests(&mut frames),
        [(terminal.title.clone(), None)]
    );

    let mut pending = DIFF_TITLES.map(|title| {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (responder, response) = oneshot::channel();
        let owner = application.services.ide.insert_pending_diff_owned(
            request_id.clone(),
            PendingDiff {
                project_id: project.clone(),
                new_path: diff_target.clone(),
                responder,
            },
        );
        application
            .runtime
            .block_on(taide_runtime::layout_actions::layout_open_tab(
                application.services.events.as_ref(),
                &application.services.state,
                project.clone(),
                TabKind::ClaudeDiff {
                    request_id: request_id.clone(),
                    path: diff_path.clone(),
                },
                title.into(),
                None,
                false,
            ))
            .unwrap();
        (request_id, response, owner)
    });
    step_until(application, &context, "diff tabs", |_, output| {
        DIFF_TITLES
            .iter()
            .all(|title| node_rect(output, title, tab_bar()).is_some())
    });
    let closed = expect_tool(call_ide_tool(
        application,
        &context,
        "closeAllDiffTabs",
        json!({}),
        &mut Vec::new(),
    ));
    assert_eq!(tool_text(&closed), "CLOSED_2_DIFF_TABS");
    for (_, response, _) in &mut pending {
        assert!(matches!(
            response.try_recv(),
            Ok((IdeDiffOutcome::TabClosed, None))
        ));
    }
    step_until(
        application,
        &context,
        "diff tabs closed",
        |application, output| {
            snapshot_tabs(application, &project).is_empty()
                && DIFF_TITLES
                    .iter()
                    .all(|title| node_rect(output, title, tab_bar()).is_none())
        },
    );
    let mut requests = close_tab_requests(&mut frames);
    requests.sort();
    let expected = DIFF_TITLES
        .iter()
        .zip(&pending)
        .map(|(title, (request_id, ..))| ((*title).to_owned(), Some(request_id.clone())))
        .collect::<Vec<_>>();
    assert_eq!(requests, expected);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CliScript {
    Link,
    Cancel,
    Fail,
    Hold,
}

struct CliProbe {
    target: std::path::PathBuf,
    link: std::path::PathBuf,
    script: Mutex<CliScript>,
    calls: AtomicUsize,
    is_held: AtomicBool,
    release: Mutex<Option<std::sync::mpsc::Receiver<()>>>,
}

impl CliProbe {
    fn run(&self) -> AppResult<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let script = *self.script.lock().unwrap();
        if script == CliScript::Hold {
            let release = self.release.lock().unwrap().take().unwrap();
            self.is_held.store(true, Ordering::SeqCst);
            release
                .recv_timeout(DEADLINE)
                .map_err(|_| AppError::Internal("held osascript was not released".into()))?;
        }
        match script {
            CliScript::Fail => Err(AppError::Internal("osascript failed: synthetic".into())),
            CliScript::Cancel => Ok(()),
            CliScript::Link | CliScript::Hold => {
                std::os::unix::fs::symlink(&self.target, &self.link)?;
                Ok(())
            }
        }
    }

    fn set(&self, script: CliScript) {
        *self.script.lock().unwrap() = script;
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

fn connect_cli(
    application: &mut NativeApplication,
    context: &egui::Context,
    directory: &std::path::Path,
) -> Arc<CliProbe> {
    let bundle = directory.join(CLI_BUNDLE_DIR);
    let link = directory.join(CLI_LINK_DIR).join(CLI_LINK_NAME);
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::create_dir_all(directory.join(CLI_LINK_DIR)).unwrap();
    let executable = bundle.join(CLI_APP_EXECUTABLE);
    let target = bundle.join(CLI_SIDECAR);
    for path in [&executable, &target] {
        std::fs::write(path, "").unwrap();
    }
    let probe = Arc::new(CliProbe {
        target,
        link: link.clone(),
        script: Mutex::new(CliScript::Link),
        calls: AtomicUsize::new(0),
        is_held: AtomicBool::new(false),
        release: Mutex::new(None),
    });
    let script = probe.clone();
    let ports = crate::agent_cli::Ports {
        executable: Arc::new(move || Ok(executable.clone())),
        link,
        osascript: Arc::new(move |_| script.run()),
    };
    let repaint = context.clone();
    let runtime = application.runtime.handle().clone();
    runtime.block_on(async {
        let bridge = HostBridge::connect_with_agent_cli(
            application.services.clone(),
            Arc::new(move || repaint.request_repaint()),
            crate::host::Terminals {
                tabs: application.terminal_agent_state.terminals.clone(),
                environment: crate::terminal_environment::provider(),
            },
            ports,
        )
        .unwrap();
        let old = application.bridge.replace(bridge).unwrap();
        old.disconnect().await.unwrap();
    });
    probe
}

fn visible_toasts(application: &NativeApplication) -> Vec<(taide_native_ui::toast::Kind, String)> {
    application
        .toasts
        .inspection()
        .into_iter()
        .filter(|toast| !toast.is_dismissed)
        .map(|toast| (toast.kind, toast.title))
        .collect()
}

fn run_cli_command(
    application: &mut NativeApplication,
    context: &egui::Context,
    probe: &CliProbe,
    command: &str,
    script: CliScript,
) -> Vec<(taide_native_ui::toast::Kind, String)> {
    probe.set(script);
    application.toasts = crate::toast_layers::ToastLayers::new().unwrap();
    application.terminal_agent_state.cli_status = None;
    super::selection_ranges_tests::run_palette(application, context, command);
    step_until(application, context, command, |application, _| {
        application.terminal_agent_state.cli_status.is_some()
            || !visible_toasts(application).is_empty()
    });
    visible_toasts(application)
}

fn is_live_cli(application: &NativeApplication) -> Option<bool> {
    application
        .terminal_agent_state
        .cli_status
        .as_ref()
        .map(|status| status.installed && !status.dangling)
}

#[test]
fn 실제_app_cli_셸_명령은_팔레트에서_실행되어_원본_toast를_보이고_취소는_성공_문구를_보이지_않는다()
{
    use taide_native_ui::toast::Kind;
    let mut fixture = workspace_fixture("agent-cli");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("package.json"), TASK_PACKAGE_MANIFEST).unwrap();
    let main_file = root.join("main.rs");
    std::fs::write(&main_file, SYNTHETIC_SOURCE).unwrap();
    let project = ProjectId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &project, &root, "alpha", false);
    let main_tab = file_tab(&main_file);
    let main_active = main_tab.id.clone();
    place_tabs(&state, &project, vec![main_tab], &main_active);
    focus_single_slot(&state, &project, &root, "alpha");
    let context = launch(&mut fixture, state);
    let directory = fixture.directory.clone();
    let application = fixture.application.as_mut().unwrap();
    for _ in 0..SETTLE_FRAMES {
        step(application, &context, Vec::new());
    }
    let probe = connect_cli(application, &context, &directory);
    let message = |application: &NativeApplication, key: &str| {
        presentation::message(&application.locale, key, &[])
    };
    let install_success = message(application, "settings.cliInstallSuccess");
    let install_failed = message(application, "settings.cliInstallFailed");
    let uninstall_success = message(application, "settings.cliUninstallSuccess");
    let connected = message(application, "settings.cliExternalEditorConnected");

    let toasts = run_cli_command(
        application,
        &context,
        &probe,
        CLI_INSTALL,
        CliScript::Cancel,
    );
    assert!(toasts.is_empty(), "{toasts:?}");
    assert_eq!(is_live_cli(application), Some(false));
    assert_eq!(probe.calls(), 1);

    let toasts = run_cli_command(application, &context, &probe, CLI_INSTALL, CliScript::Fail);
    assert_eq!(toasts, [(Kind::Error, install_failed.clone())]);
    assert_eq!(is_live_cli(application), None);
    assert_eq!(probe.calls(), 2);

    let toasts = run_cli_command(application, &context, &probe, CLI_INSTALL, CliScript::Link);
    assert_eq!(toasts, [(Kind::Success, install_success.clone())]);
    assert_eq!(is_live_cli(application), Some(true));
    assert_eq!(std::fs::read_link(&probe.link).unwrap(), probe.target);
    assert_eq!(probe.calls(), 3);

    let toasts = run_cli_command(application, &context, &probe, CLI_CONNECT, CliScript::Fail);
    assert_eq!(toasts, [(Kind::Success, connected)]);
    assert_eq!(is_live_cli(application), Some(true));
    assert_eq!(probe.calls(), 3);

    let toasts = run_cli_command(
        application,
        &context,
        &probe,
        CLI_UNINSTALL,
        CliScript::Fail,
    );
    assert_eq!(toasts, [(Kind::Success, uninstall_success)]);
    assert_eq!(is_live_cli(application), Some(false));
    assert!(std::fs::symlink_metadata(&probe.link).is_err());
    assert_eq!(probe.calls(), 3);

    let toasts = run_cli_command(
        application,
        &context,
        &probe,
        CLI_CONNECT,
        CliScript::Cancel,
    );
    assert_eq!(toasts, [(Kind::Error, install_failed)]);
    assert_eq!(is_live_cli(application), Some(false));
    assert_eq!(probe.calls(), 4);

    let (release, held) = std::sync::mpsc::channel();
    *probe.release.lock().unwrap() = Some(held);
    probe.set(CliScript::Hold);
    application.toasts = crate::toast_layers::ToastLayers::new().unwrap();
    application.terminal_agent_state.cli_status = None;
    super::selection_ranges_tests::run_palette(application, &context, CLI_INSTALL);
    step_until(application, &context, "held osascript", |_, _| {
        probe.is_held.load(Ordering::SeqCst)
    });
    super::selection_ranges_tests::run_palette(application, &context, "task.runTask");
    step_until(
        application,
        &context,
        "tasks while osascript is held",
        |application, _| {
            application.commands_state.palette.is_picking_tasks()
                && application
                    .commands_state
                    .palette
                    .inspection()
                    .rows
                    .iter()
                    .any(|row| row.label == TASK_NPM_SCRIPT)
        },
    );
    assert_eq!(is_live_cli(application), None);
    assert!(visible_toasts(application).is_empty());
    release.send(()).unwrap();
    step_until(
        application,
        &context,
        "released osascript",
        |application, _| application.terminal_agent_state.cli_status.is_some(),
    );
    assert_eq!(
        visible_toasts(application),
        [(Kind::Success, install_success)]
    );
    assert_eq!(is_live_cli(application), Some(true));
    assert_eq!(probe.calls(), 5);
}

#[derive(Default)]
struct RoutedViewports {
    events: HashMap<egui::ViewportId, Vec<egui::Event>>,
    outputs: HashMap<egui::ViewportId, egui::FullOutput>,
}

impl RoutedViewports {
    fn shows(&self, viewport: egui::ViewportId, text: &str) -> bool {
        self.outputs
            .get(&viewport)
            .is_some_and(|output| shows_text(output, text))
    }
}

fn shows_text(output: &egui::FullOutput, text: &str) -> bool {
    text_shapes(output)
        .iter()
        .any(|shape| shape.galley.text() == text)
}

fn route_viewports(context: &egui::Context) -> std::rc::Rc<std::cell::RefCell<RoutedViewports>> {
    let viewports = std::rc::Rc::new(std::cell::RefCell::new(RoutedViewports::default()));
    let shared = viewports.clone();
    context.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
        let id = viewport.ids.this;
        let events = shared.borrow_mut().events.remove(&id).unwrap_or_default();
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: id,
                viewports: std::iter::once((id, egui::ViewportInfo::default())).collect(),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(AUXILIARY_SCREEN[0], AUXILIARY_SCREEN[1]),
                )),
                events,
                ..Default::default()
            },
            |ui| (viewport.viewport_ui_cb)(ui),
        );
        output.textures_delta.clear();
        shared.borrow_mut().outputs.insert(id, output);
    });
    viewports
}

fn unfocused_main_step(
    application: &mut NativeApplication,
    context: &egui::Context,
) -> egui::FullOutput {
    eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(AUXILIARY_SCREEN[0], AUXILIARY_SCREEN[1]),
            )),
            focused: false,
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    output.textures_delta.clear();
    output
}

fn auxiliary_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    viewports: &std::cell::RefCell<RoutedViewports>,
    phase: &str,
    ready: impl Fn(&NativeApplication, &RoutedViewports) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = unfocused_main_step(application, context);
        if ready(application, &viewports.borrow()) {
            return output;
        }
        assert!(
            Instant::now() < deadline,
            "phase={phase}; status={:?}",
            application.status
        );
        std::thread::yield_now();
    }
}

#[test]
fn 실제_app_보조창_task_runner는_그_창에_세_출처를_보이고_그_창_포커스_pane_새_터미널로_실행한다() {
    let mut fixture = workspace_fixture("task-runner-auxiliary");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("package.json"), TASK_PACKAGE_MANIFEST).unwrap();
    std::fs::write(root.join("Makefile"), TASK_MAKEFILE).unwrap();
    std::fs::write(root.join("Cargo.toml"), TASK_CARGO_MANIFEST).unwrap();
    let main_file = root.join("main.rs");
    let auxiliary_file = root.join(AUXILIARY_FILE);
    for path in [&main_file, &auxiliary_file] {
        std::fs::write(path, SYNTHETIC_SOURCE).unwrap();
    }
    let project = ProjectId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &project, &root, "alpha", false);
    let main_tab = file_tab(&main_file);
    let main_active = main_tab.id.clone();
    place_tabs(&state, &project, vec![main_tab], &main_active);
    let auxiliary_tab = file_tab(&auxiliary_file);
    let auxiliary_pane = PaneId::new();
    state
        .layouts
        .write()
        .get_mut(&project)
        .unwrap()
        .auxiliary_windows = vec![AuxWindowLayout {
        slot: AUXILIARY_SLOT,
        root: PaneNode::Leaf {
            id: auxiliary_pane.clone(),
            tabs: vec![auxiliary_tab.clone()],
            active: Some(auxiliary_tab.id.clone()),
        },
        focused_pane: auxiliary_pane.clone(),
    }];
    focus_single_slot(&state, &project, &root, "alpha");
    let context = launch(&mut fixture, state);
    let viewports = route_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    application.services.state.settings.write().shell_override = Some("/bin/cat".into());
    let viewport = application
        .shell_state
        .auxiliary_windows
        .find(&project, AUXILIARY_SLOT)
        .unwrap()
        .viewport;
    let message = |key: &str| presentation::message(&application.locale, key, &[]);
    let run_task = message("task.runTask");
    let terminal_title = message("terminal.title");
    let [npm, make, cargo] = ["task.sourceNpm", "task.sourceMake", "task.sourceCargo"].map(message);
    let label = crate::command_registry::registry()
        .unwrap()
        .command("task.runTask")
        .unwrap()
        .label(&application.locale);

    auxiliary_until(
        application,
        &context,
        &viewports,
        "auxiliary window",
        |_, viewports| viewports.shows(viewport, AUXILIARY_FILE),
    );
    viewports.borrow_mut().events.insert(
        viewport,
        super::inlay_hints_tests::key(egui::Key::P, command_shift()),
    );
    auxiliary_until(
        application,
        &context,
        &viewports,
        "auxiliary palette",
        |application, _| {
            application.commands_state.palette.is_open()
                && application
                    .commands_state
                    .palette
                    .inspection()
                    .has_input_focus
        },
    );
    assert_eq!(application.shell_state.overlay_viewport, viewport);
    viewports
        .borrow_mut()
        .events
        .insert(viewport, vec![egui::Event::Text(label)]);
    auxiliary_until(
        application,
        &context,
        &viewports,
        "run task row",
        |application, _| {
            application
                .commands_state
                .palette
                .inspection()
                .rows
                .iter()
                .any(|row| row.is_selected && row.key == "task.runTask")
        },
    );
    viewports.borrow_mut().events.insert(
        viewport,
        super::inlay_hints_tests::key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let main_output = auxiliary_until(
        application,
        &context,
        &viewports,
        "auxiliary tasks",
        |application, viewports| {
            application.commands_state.palette.is_picking_tasks()
                && application.commands_state.palette.inspection().rows.len() == TASK_ROWS.len()
                && viewports.shows(viewport, CARGO_CHECK)
        },
    );
    let inspection = application.commands_state.palette.inspection().clone();
    assert_eq!(inspection.placeholder, run_task);
    let expected = TASK_ROWS
        .iter()
        .zip([&npm, &make, &cargo, &cargo, &cargo, &cargo, &cargo])
        .map(|(label, source)| ((*label).to_owned(), Some(source.clone())))
        .collect::<Vec<_>>();
    assert_eq!(
        inspection
            .rows
            .iter()
            .map(|row| (row.label.clone(), row.detail.clone()))
            .collect::<Vec<_>>(),
        expected
    );
    for text in [
        CARGO_CHECK,
        TASK_NPM_SCRIPT,
        npm.as_str(),
        make.as_str(),
        cargo.as_str(),
    ] {
        assert!(viewports.borrow().shows(viewport, text), "{text}");
        assert!(!shows_text(&main_output, text), "{text}");
    }

    viewports
        .borrow_mut()
        .events
        .insert(viewport, vec![egui::Event::Text(CARGO_CHECK.into())]);
    auxiliary_until(
        application,
        &context,
        &viewports,
        "filtered task",
        |application, _| {
            let rows = &application.commands_state.palette.inspection().rows;
            rows.len() == 1 && rows[0].label == CARGO_CHECK && rows[0].is_selected
        },
    );
    viewports.borrow_mut().events.insert(
        viewport,
        super::inlay_hints_tests::key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    unfocused_main_step(application, &context);
    assert!(!application.commands_state.palette.is_open());
    let payload = format!("{CARGO_CHECK}\n");
    let deadline = Instant::now() + DEADLINE;
    while !application
        .editor_terminal_writes
        .iter()
        .any(|delivery| delivery.payload == payload)
    {
        assert!(
            Instant::now() < deadline,
            "auxiliary task terminal delivery"
        );
        eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
        std::thread::yield_now();
    }
    let delivery = application
        .editor_terminal_writes
        .iter()
        .find(|delivery| delivery.payload == payload)
        .unwrap();
    assert_eq!(delivery.project, project);
    let layouts = application.services.state.layouts.read();
    let layout = &layouts[&project];
    let window = layout
        .auxiliary_windows
        .iter()
        .find(|window| window.slot == AUXILIARY_SLOT)
        .unwrap();
    let PaneNode::Leaf { id, tabs, active } = &window.root else {
        panic!("auxiliary leaf")
    };
    assert_eq!(id, &auxiliary_pane);
    assert_eq!(active.as_ref(), Some(&delivery.tab));
    let tab = tabs.iter().find(|tab| tab.id == delivery.tab).unwrap();
    assert_eq!(tab.title, terminal_title);
    assert!(
        matches!(&tab.kind, TabKind::Terminal { cwd, .. } if cwd.as_deref() == root.to_str()),
        "{:?}",
        tab.kind
    );
    assert!(
        crate::tabs::tabs_in(&layout.root)
            .iter()
            .all(|tab| !matches!(tab.kind, TabKind::Terminal { .. }))
    );
}
