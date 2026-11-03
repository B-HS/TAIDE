use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use super::*;
use crate::application::inlay_hints_tests::key;
use crate::application::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, project_ref, step,
};
use crate::application::selection_ranges_tests::run_palette;
use crate::symbol_sidebar::View;
use taide_model::ids::{ShellSlotId, TabId};
use taide_model::layout::{PaneNode, SplitDir, Tab};
use taide_model::project::ShellSlotTree;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::toast::Kind;
use taide_runtime::AppState;

const DEADLINE: Duration = Duration::from_secs(10);
const SETTLE_FRAMES: usize = 4;
const HALF: f32 = 50.0;
const AUXILIARY_SCREEN: [f32; 2] = [1000.0, 700.0];
const NEW_WINDOW_SLOT: u32 = 1;
const NEXT_WINDOW_SLOT: u32 = 2;
const NEW_WINDOW_LABEL: &str = "editor-1";
const WELCOME_TITLE: &str = "Welcome";
const REVEAL_THEME: &str = "vscode-light-modern";
const REVEAL_LANGUAGE: &str = "ko";
const IME_DEBUG_COMMAND: &str = "terminal.copyImeDebug";

type RoutedEvents = Rc<RefCell<HashMap<egui::ViewportId, Vec<egui::Event>>>>;

fn fixture(name: &str) -> Fixture {
    let directory =
        std::env::temp_dir().join(format!("taide-native-commands-{name}-{}", ProjectId::new()));
    std::fs::create_dir_all(directory.join("data")).unwrap();
    Fixture {
        directory,
        application: None,
    }
}

fn file_tab(root: &std::path::Path, name: &str) -> Tab {
    let path = root.join(name);
    std::fs::write(&path, "synthetic\n").unwrap();
    Tab {
        id: TabId::new(),
        kind: TabKind::File {
            path: path.to_str().unwrap().into(),
        },
        title: name.into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
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

fn settle(application: &mut NativeApplication, context: &egui::Context) {
    for _ in 0..SETTLE_FRAMES {
        step(application, context, Vec::new());
    }
}

fn step_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let deadline = Instant::now() + DEADLINE;
    loop {
        step(application, context, Vec::new());
        if ready(application) {
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

fn pane_tabs(state: &AppState, project: &ProjectId, pane: &PaneId) -> (Vec<Tab>, Option<TabId>) {
    let layouts = state.layouts.read();
    let Some(PaneNode::Leaf { tabs, active, .. }) =
        taide_layout::service::find_leaf(&layouts[project].root, pane)
    else {
        panic!("expected leaf")
    };
    (tabs.clone(), active.clone())
}

fn visible_toasts(application: &NativeApplication) -> Vec<(Kind, String)> {
    application
        .toasts
        .inspection()
        .into_iter()
        .filter(|toast| !toast.is_dismissed)
        .map(|toast| (toast.kind, toast.title))
        .collect()
}

#[test]
fn 실제_app_팔레트_view_explorer는_포커스_슬롯만_접힘을_풀고_파일_보기로_바꾼다() {
    let mut fixture = fixture("explorer");
    let names = ["alpha", "beta"];
    let roots = names.map(|name| fixture.directory.join(name));
    let projects = [ProjectId::new(), ProjectId::new()];
    let slots = [ShellSlotId::new(), ShellSlotId::new()];
    let state = isolated_state(fixture.directory.join("data"));
    for ((project, root), name) in projects.iter().zip(&roots).zip(names) {
        std::fs::create_dir_all(root).unwrap();
        insert_project(&state, project, root, name, false);
        let tab = file_tab(root, "main.txt");
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(project).unwrap();
        layout.root = PaneNode::Leaf {
            id: layout.focused_pane.clone(),
            active: Some(tab.id.clone()),
            tabs: vec![tab],
        };
        layout.shell_view.sidebar_collapsed = true;
    }
    {
        let mut session = state.session.write();
        session.projects = projects
            .iter()
            .zip(&roots)
            .zip(names)
            .map(|((project, root), name)| project_ref(project, root, name))
            .collect();
        session.active_project = Some(projects[1].clone());
        session.shell_slots = Some(ShellSlotTree::Split {
            dir: SplitDir::Horizontal,
            children: projects
                .iter()
                .zip(&slots)
                .map(|(project, slot)| ShellSlotTree::Leaf {
                    slot_id: slot.clone(),
                    project_id: project.clone(),
                })
                .collect(),
            sizes: vec![HALF, HALF],
        });
        session.focused_shell_slot = Some(slots[1].clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    settle(application, &context);
    for (project, slot) in projects.iter().zip(&slots) {
        application
            .explorer_search_state
            .symbol_sidebars
            .entry(project, slot)
            .view = View::Outline;
    }
    let collapsed = |index: usize| {
        services.state.layouts.read()[&projects[index]]
            .shell_view
            .sidebar_collapsed
    };

    run_palette(application, &context, "view.explorer");
    step_until(application, &context, "focused slot explorer", |_| {
        !collapsed(1)
    });
    let mut view = |index: usize| {
        application
            .explorer_search_state
            .symbol_sidebars
            .entry(&projects[index], &slots[index])
            .view
    };
    assert_eq!(view(1), View::Files);
    assert_eq!(view(0), View::Outline);
    settle(application, &context);
    assert!(collapsed(0));
}

#[test]
fn 실제_app_팔레트_view_welcome은_포커스_pane에_한번만_열고_재실행은_활성화한다() {
    let mut fixture = fixture("welcome");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    let project = ProjectId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &project, &root, "alpha", false);
    let left_tab = file_tab(&root, "left.txt");
    let right_tab = file_tab(&root, "right.txt");
    let (left, right) = (PaneId::new(), PaneId::new());
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        layout.root = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![
                PaneNode::Leaf {
                    id: left.clone(),
                    tabs: vec![left_tab.clone()],
                    active: Some(left_tab.id.clone()),
                },
                PaneNode::Leaf {
                    id: right.clone(),
                    tabs: vec![right_tab.clone()],
                    active: Some(right_tab.id.clone()),
                },
            ],
            sizes: vec![HALF, HALF],
        };
        layout.focused_pane = right.clone();
    }
    focus_single_slot(&state, &project, &root, "alpha");
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let state = application.services.state.clone();
    settle(application, &context);

    run_palette(application, &context, "view.welcome");
    step_until(application, &context, "welcome tab", |_| {
        pane_tabs(&state, &project, &right).0.len() == 2
    });
    let (tabs, active) = pane_tabs(&state, &project, &right);
    let welcome = tabs
        .iter()
        .find(|tab| tab.kind == TabKind::Welcome)
        .unwrap()
        .clone();
    assert_eq!(welcome.title, WELCOME_TITLE);
    assert!(!welcome.preview);
    assert_eq!(active, Some(welcome.id.clone()));
    let (left_tabs, left_active) = pane_tabs(&state, &project, &left);
    assert_eq!(
        left_tabs.iter().map(|tab| &tab.id).collect::<Vec<_>>(),
        [&left_tab.id]
    );
    assert_eq!(left_active, Some(left_tab.id.clone()));
    assert_eq!(state.layouts.read()[&project].focused_pane, right);

    application
        .controller
        .submit(ShellMutation::ActivateTab(right_tab.id.clone()))
        .unwrap();
    step_until(application, &context, "right tab active", |_| {
        pane_tabs(&state, &project, &right).1 == Some(right_tab.id.clone())
    });
    run_palette(application, &context, "view.welcome");
    step_until(application, &context, "welcome reactivated", |_| {
        pane_tabs(&state, &project, &right).1 == Some(welcome.id.clone())
    });
    settle(application, &context);
    let (tabs, _) = pane_tabs(&state, &project, &right);
    assert_eq!(
        tabs.iter().map(|tab| &tab.id).collect::<Vec<_>>(),
        [&right_tab.id, &welcome.id]
    );
    assert_eq!(pane_tabs(&state, &project, &left).0.len(), 1);
}

#[test]
fn 실제_app_프로젝트가_없으면_팔레트_view_welcome은_프로젝트_먼저_열기_안내만_보인다() {
    let mut fixture = fixture("welcome-empty");
    let state = isolated_state(fixture.directory.join("data"));
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    settle(application, &context);
    let layouts = application.services.state.layouts.read().len();
    application.toasts = crate::toast::Toasts::with_actions().unwrap();
    let notice = presentation::message(&application.locale, "app.openProjectFirst", &[]);

    run_palette(application, &context, "view.welcome");
    step_until(
        application,
        &context,
        "open project notice",
        |application| !visible_toasts(application).is_empty(),
    );
    settle(application, &context);
    assert_eq!(visible_toasts(application), [(Kind::Info, notice)]);
    assert_eq!(application.services.state.layouts.read().len(), layouts);
}

fn command_shift() -> egui::Modifiers {
    let command = if cfg!(target_os = "macos") {
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
    } else {
        egui::Modifiers::CTRL | egui::Modifiers::COMMAND
    };
    command | egui::Modifiers::SHIFT
}

fn route_viewports(context: &egui::Context) -> RoutedEvents {
    let events = RoutedEvents::default();
    let shared = events.clone();
    context.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
        let id = viewport.ids.this;
        let events = shared.borrow_mut().remove(&id).unwrap_or_default();
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
    });
    events
}

fn unfocused_main_step(application: &mut NativeApplication, context: &egui::Context) {
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
}

fn auxiliary_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let deadline = Instant::now() + DEADLINE;
    loop {
        unfocused_main_step(application, context);
        if ready(application) {
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

fn run_auxiliary_palette(
    application: &mut NativeApplication,
    context: &egui::Context,
    events: &RoutedEvents,
    viewport: egui::ViewportId,
    command: &str,
) {
    events
        .borrow_mut()
        .insert(viewport, key(egui::Key::P, command_shift()));
    auxiliary_until(application, context, "auxiliary palette", |application| {
        application.commands_state.palette.is_open()
            && application
                .commands_state
                .palette
                .inspection()
                .has_input_focus
    });
    assert_eq!(application.shell_state.overlay_viewport, viewport);
    let label = crate::command_registry::registry()
        .unwrap()
        .command(command)
        .unwrap()
        .label(&application.locale);
    events
        .borrow_mut()
        .insert(viewport, vec![egui::Event::Text(label)]);
    auxiliary_until(application, context, command, |application| {
        application
            .commands_state
            .palette
            .inspection()
            .rows
            .iter()
            .any(|row| row.key == command && row.is_enabled && row.is_selected)
    });
    events
        .borrow_mut()
        .insert(viewport, key(egui::Key::Enter, egui::Modifiers::NONE));
    unfocused_main_step(application, context);
}

#[test]
fn 실제_app_팔레트_탭_창이동은_새창과_메인으로_옮기고_빈_pane은_무시한다() {
    let mut fixture = fixture("tab-window");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    let project = ProjectId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &project, &root, "alpha", false);
    let moved = file_tab(&root, "moved.txt");
    let main_pane = {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        layout.root = PaneNode::Leaf {
            id: layout.focused_pane.clone(),
            tabs: vec![moved.clone()],
            active: Some(moved.id.clone()),
        };
        layout.focused_pane.clone()
    };
    focus_single_slot(&state, &project, &root, "alpha");
    let context = launch(&mut fixture, state);
    let events = route_viewports(&context);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let window_tabs = |slot: u32| -> Option<Vec<TabId>> {
        services.state.layouts.read()[&project]
            .auxiliary_windows
            .iter()
            .find(|window| window.slot == slot)
            .map(|window| {
                crate::tabs::tabs_in(&window.root)
                    .into_iter()
                    .map(|tab| tab.id.clone())
                    .collect()
            })
    };
    let main_tabs = || {
        pane_tabs(&services.state, &project, &main_pane)
            .0
            .into_iter()
            .map(|tab| tab.id)
            .collect::<Vec<_>>()
    };
    let is_window_open = |application: &NativeApplication, slot: u32| {
        application
            .shell_state
            .auxiliary_windows
            .find(&project, slot)
            .is_some()
    };
    settle(application, &context);

    run_palette(application, &context, "tab.moveToNewWindow");
    step_until(
        application,
        &context,
        "moved to a new window",
        |application| {
            window_tabs(NEW_WINDOW_SLOT) == Some(vec![moved.id.clone()])
                && is_window_open(application, NEW_WINDOW_SLOT)
                && main_tabs().is_empty()
        },
    );
    assert_eq!(
        services
            .windows
            .label_for(&project, NEW_WINDOW_SLOT)
            .as_deref(),
        Some(NEW_WINDOW_LABEL)
    );
    let viewport = application
        .shell_state
        .auxiliary_windows
        .find(&project, NEW_WINDOW_SLOT)
        .unwrap()
        .viewport;

    let revision = services.state.layouts.read()[&project].revision;
    run_palette(application, &context, "tab.moveToNewWindow");
    settle(application, &context);
    assert_eq!(services.state.layouts.read()[&project].revision, revision);
    assert!(!is_window_open(application, NEXT_WINDOW_SLOT));
    assert_eq!(application.shell_state.auxiliary_windows.windows().len(), 1);

    run_auxiliary_palette(
        application,
        &context,
        &events,
        viewport,
        "tab.moveToMainWindow",
    );
    auxiliary_until(application, &context, "back to main", |application| {
        main_tabs() == vec![moved.id.clone()]
            && window_tabs(NEW_WINDOW_SLOT).is_none()
            && !is_window_open(application, NEW_WINDOW_SLOT)
    });
    assert!(
        services
            .windows
            .label_for(&project, NEW_WINDOW_SLOT)
            .is_none()
    );
    assert_eq!(
        pane_tabs(&services.state, &project, &main_pane).1,
        Some(moved.id.clone())
    );
}

#[test]
fn 실제_app_첫_프레임은_창을_보이기_전에_설정한_테마_배경과_언어로_그려진다() {
    let mut fixture = fixture("reveal");
    let state = isolated_state(fixture.directory.join("data"));
    {
        let mut settings = state.settings.write();
        settings.theme_id = REVEAL_THEME.into();
        settings.follow_system_theme = false;
        settings.language = REVEAL_LANGUAGE.into();
        std::fs::write(
            state.paths.settings_file(),
            serde_json::to_vec(&*settings).unwrap(),
        )
        .unwrap();
    }
    let theme = taide_runtime::theme_actions::theme_get(&state, REVEAL_THEME.into()).unwrap();
    let background = crate::presentation::color(&theme, "app.background").unwrap();
    let egui_default = egui::Visuals::default().panel_fill;
    assert_ne!(background, egui_default);

    let context = launch(&mut fixture, state);
    let unpainted = context.global_style().visuals.clone();
    assert_eq!(unpainted.panel_fill, background);
    assert!(!unpainted.dark_mode);
    let application = fixture.application.as_mut().unwrap();
    assert_eq!(application.locale.id, REVEAL_LANGUAGE);

    let first = step(application, &context, Vec::new());
    let rect_fills = first
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => Some(rect.fill),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(rect_fills.contains(&background));
    assert!(!rect_fills.contains(&egui_default));
    assert_eq!(context.global_style().visuals.panel_fill, background);
}

#[test]
fn 실제_app_팔레트_ime_디버그_복사는_비활성_행이라_클릭해도_실행되지_않는다() {
    let mut fixture = fixture("ime-debug");
    let state = isolated_state(fixture.directory.join("data"));
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    settle(application, &context);
    application.toasts = crate::toast::Toasts::with_actions().unwrap();
    application.commands_state.palette.open(
        &context,
        taide_native_ui::command_registry::PaletteEntry::Commands,
    );
    settle(application, &context);
    let label = crate::command_registry::registry()
        .unwrap()
        .command(IME_DEBUG_COMMAND)
        .unwrap()
        .label(&application.locale);
    step(application, &context, vec![egui::Event::Text(label)]);
    settle(application, &context);
    let row = application
        .commands_state
        .palette
        .inspection()
        .rows
        .iter()
        .find(|row| row.key == IME_DEBUG_COMMAND)
        .cloned()
        .unwrap();
    assert!(!row.is_enabled && !row.is_selected, "{row:?}");

    let center = row.rect.center();
    step(
        application,
        &context,
        vec![egui::Event::PointerMoved(center)],
    );
    for pressed in [true, false] {
        step(
            application,
            &context,
            vec![egui::Event::PointerButton {
                pos: center,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    settle(application, &context);
    assert!(application.commands_state.palette.is_open());
    assert!(application.commands_state.palette_commands.is_empty());
    assert!(visible_toasts(application).is_empty());
}
