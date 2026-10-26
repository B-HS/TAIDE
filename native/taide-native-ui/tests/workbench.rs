use std::sync::Mutex;

use egui::epaint::Shadow;
use egui::{Color32, CornerRadius, Rect, Stroke, Ui, pos2, vec2};
use taide_model::agent::{AgentActivity, BlockedReason, DetectedAgent};
use taide_model::app_event::AppEvent;
use taide_model::ids::{PaneId, ProjectGroupId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{
    AuxWindowLayout, DropEdge, PaneNode, ProjectLayout, SplitDir, Tab, TabKind,
};
use taide_model::paths::AppPaths;
use taide_model::project::{
    ProjectDisplay, ProjectGroup, ProjectRef, SessionShellState, ShellSlotEdge, ShellSlotTree,
    WindowChrome,
};
use taide_native_ui::commands::{ShellIntent, ShellMutation, dispatch, request_close_tab};
use taide_native_ui::presentation::{apply_visuals, visuals};
use taide_native_ui::project_rail::{RailColors, RailIcon};
use taide_native_ui::shell::{
    NativeShell, ShellColors, ShellSurfaces, TITLE_HEIGHT, WelcomeContent, WelcomeRecent,
    WelcomeShortcutRow, WindowScope,
};
use taide_native_ui::snapshot::{ShellSnapshot, project_group_sections};
use taide_native_ui::split::{MIN_PANE_SIZE, child_rects, normalized_sizes, resized_pair};
use taide_runtime::{AppState, EventSink, theme_actions};

const WINDOW_SIZE: [f32; 2] = [1280.0, 800.0];
const AUX_SLOT: u32 = 1;
const SPLIT_THICKNESS: f32 = 1.0;
const RESIZE_DELTA: f32 = 60.0;
const COLOR: Color32 = Color32::GRAY;
const PROBLEMS_DIVIDER_THICKNESS: f32 = 5.0;
const PROBLEMS_KEYBOARD_FRACTION: f32 = 0.05;
const PROBLEMS_MAX_FRACTION: f32 = 0.7;
const GEOMETRY_EPSILON: f32 = egui::emath::GUI_ROUNDING;
const PROBLEMS_MIN_HEIGHT: f32 = 120.0;
const PROBLEMS_DEFAULT_HEIGHT: f32 = 220.0;
const RESIZED_WINDOW_HEIGHT: f32 = 1100.0;
const LOW_WINDOW_HEIGHTS: [f32; 2] = [180.0, 220.0];
const PANEL_PERCENT_PRECISION: f32 = 1000.0;
const CSS_BORDER_WIDTH: f32 = 1.0;
const CSS_RADIUS_MEDIUM: u8 = 6;
const CSS_RADIUS_LARGE: u8 = 8;
const CSS_SHADOW_OVERLAY: ([i8; 2], u8) = ([0, 2], 8);
const CSS_SHADOW_OVERLAY_LARGE: ([i8; 2], u8) = ([0, 8], 24);
const CLICK_INTERVAL_SECONDS: f64 = 0.05;
const DRAG_DISTANCE: f32 = 40.0;
const OVERFLOWING_TAB_COUNT: usize = 24;
const WHEEL_DELTA: f32 = -200.0;
const WHEEL_SETTLE_FRAMES: usize = 30;
const FRAME_SECONDS: f64 = 1.0 / 60.0;
const TAB_ICON_SIDE: f32 = 14.0;
const TAB_ICON_GAP: f32 = 6.0;
const PIXEL_SNAP_TOLERANCE: f32 = 0.5;
const RAIL_HOVER: Color32 = Color32::from_rgb(1, 2, 3);
const RAIL_ACTIVE: Color32 = Color32::from_rgb(4, 5, 6);
const RAIL_ACCENT: Color32 = Color32::from_rgb(7, 8, 9);
const AGENT_WORKING: Color32 = Color32::from_rgb(10, 200, 10);
const AGENT_AWAITING: Color32 = Color32::from_rgb(200, 200, 10);
const AGENT_IDLE: Color32 = Color32::from_rgb(10, 10, 200);
const AGENT_UNKNOWN: Color32 = Color32::from_rgb(90, 90, 90);
const LANE_RED: u8 = 240;
const RAIL_GLYPH_ICON_SIDE: f32 = 20.0;
const RAIL_BUTTON_SIDE: f32 = 40.0;
const ROOT_MISSING_OPACITY: f32 = 0.5;
const TOOLTIP_HOVER_SECONDS: f64 = 10.0;
const TOOLTIP_SECONDARY_OPACITY: f32 = 0.7;
const TOOLTIP_FADE_SECONDS: f64 = 0.5;
const TOOLTIP_SETTLE_SECONDS: f64 = 1.0;

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[derive(Default)]
struct Surfaces {
    panes: Vec<(ProjectId, PaneId, TabId, Rect)>,
    explorers: Vec<(ProjectId, ShellSlotId)>,
    status_projects: Vec<Option<ProjectId>>,
    problems_open: std::collections::HashSet<ShellSlotId>,
    problems: Vec<(ShellSlotId, Rect)>,
    tab_icons: Vec<(TabId, Rect, Color32)>,
    welcome: Option<WelcomeContent>,
    rail: Vec<WelcomeRecent>,
    agents: std::collections::HashMap<ProjectId, Vec<DetectedAgent>>,
    is_badge_disabled: bool,
    is_reduced_motion: bool,
    rail_icons: Vec<(String, Rect, Color32, f32)>,
}

impl ShellSurfaces for Surfaces {
    fn rail_recent_projects(&self) -> Vec<WelcomeRecent> {
        self.rail.clone()
    }
    fn project_agents(&self, project: &ProjectId) -> Vec<DetectedAgent> {
        self.agents.get(project).cloned().unwrap_or_default()
    }
    fn is_agent_status_badge_enabled(&self) -> bool {
        !self.is_badge_disabled
    }
    fn is_reduced_motion(&self) -> bool {
        self.is_reduced_motion
    }
    fn rail_icon(&mut self, ui: &Ui, rect: Rect, icon: RailIcon<'_>, color: Color32) {
        self.rail_icons
            .push((format!("{icon:?}"), rect, color, ui.opacity()));
    }
    fn welcome_content(&self, _: Option<&ProjectId>) -> WelcomeContent {
        self.welcome.clone().unwrap_or_default()
    }
    fn text(&self, key: &str) -> String {
        key.to_owned()
    }
    fn text_with_args(&self, key: &str, arguments: &[(&str, &str)]) -> String {
        format!("{key}: {arguments:?}")
    }
    fn branch(&self, _: &ProjectId) -> Option<&str> {
        None
    }
    fn problems_open(&self, slot: &ShellSlotId) -> bool {
        self.problems_open.contains(slot)
    }
    fn problems_panel(&mut self, ui: &mut Ui, _: &ProjectId, slot: &ShellSlotId) {
        self.problems
            .push((slot.clone(), ui.available_rect_before_wrap()));
        ui.label("test-only problems");
    }
    fn explorer(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        slot: &ShellSlotId,
        _: &mut Vec<ShellIntent>,
    ) {
        self.explorers.push((project.clone(), slot.clone()));
        ui.label("test-only explorer");
    }
    fn tab_content(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        _: &mut Vec<ShellIntent>,
    ) {
        self.panes
            .push((project.clone(), pane.clone(), tab.id.clone(), ui.max_rect()));
        ui.label(&tab.title);
    }
    fn tab_icon(&mut self, _: &Ui, rect: Rect, tab: &Tab, title_color: Color32) {
        self.tab_icons.push((tab.id.clone(), rect, title_color));
    }
    fn status_bar(&mut self, ui: &mut Ui, project: Option<&ProjectId>, _: &mut Vec<ShellIntent>) {
        self.status_projects.push(project.cloned());
        ui.label("test-only status");
    }
}

fn shell(scope: WindowScope) -> NativeShell {
    NativeShell {
        scope,
        colors: ShellColors {
            background: COLOR,
            foreground: COLOR,
            sidebar: COLOR,
            muted: COLOR,
            border: COLOR,
            focus_border: COLOR,
            tab_border: COLOR,
            active_tab: COLOR,
            inactive_tab: COLOR,
            active_indicator: COLOR,
            editor_background: COLOR,
            editor_foreground: COLOR,
            rail: RailColors {
                item_hover: RAIL_HOVER,
                item_active: RAIL_ACTIVE,
                accent: RAIL_ACCENT,
                agent_working: AGENT_WORKING,
                agent_awaiting: AGENT_AWAITING,
                agent_idle: AGENT_IDLE,
                agent_unknown: AGENT_UNKNOWN,
                lanes: std::array::from_fn(|lane| Color32::from_rgb(LANE_RED, lane as u8, 0)),
            },
        },
        has_title_bar: true,
    }
}

fn frame(
    context: &egui::Context,
    shell: &NativeShell,
    snapshot: &ShellSnapshot,
    time: f64,
    events: Vec<egui::Event>,
) -> (Surfaces, egui::FullOutput) {
    let mut surfaces = Surfaces::default();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            pos2(0.0, 0.0),
            vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]),
        )),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut output = context.run_ui(input, |ui| {
        shell.show(ui, snapshot, &mut surfaces);
    });
    output.textures_delta.clear();
    (surfaces, output)
}

fn render(shell: &NativeShell, snapshot: &ShellSnapshot) -> Surfaces {
    frame(&egui::Context::default(), shell, snapshot, 0.0, Vec::new()).0
}

fn rendered_texts(shell: &NativeShell, snapshot: &ShellSnapshot) -> (Surfaces, Vec<String>) {
    let (surfaces, output) = frame(&egui::Context::default(), shell, snapshot, 0.0, Vec::new());
    let texts = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    (surfaces, texts)
}

fn surface_layout() -> ProjectLayout {
    let mut layout = taide_layout::service::default_layout();
    let PaneNode::Leaf { tabs, active, .. } = &mut layout.root else {
        panic!("expected leaf")
    };
    *active = tabs
        .iter()
        .find(|tab| !matches!(tab.kind, TabKind::Welcome))
        .map(|tab| tab.id.clone());
    layout
}

fn rgb(hex: u32) -> Color32 {
    let [_, red, green, blue] = hex.to_be_bytes();
    Color32::from_rgb(red, green, blue)
}

fn fixture() -> ShellSnapshot {
    let projects = [ProjectId::new(), ProjectId::new()];
    let slots = [ShellSlotId::new(), ShellSlotId::new()];
    let tree = ShellSlotTree::Split {
        dir: SplitDir::Horizontal,
        children: projects
            .iter()
            .zip(&slots)
            .map(|(project, slot)| ShellSlotTree::Leaf {
                slot_id: slot.clone(),
                project_id: project.clone(),
            })
            .collect(),
        sizes: vec![50.0, 50.0],
    };
    ShellSnapshot {
        projects: projects
            .iter()
            .map(|id| ProjectRef {
                id: id.clone(),
                root: "/synthetic".into(),
                name: id.to_string(),
                display: ProjectDisplay::default(),
                root_missing: false,
            })
            .collect(),
        groups: Vec::new(),
        shell: SessionShellState {
            tree: Some(tree),
            focused: Some(slots[1].clone()),
            window_chrome: WindowChrome::default(),
        },
        layouts: projects
            .into_iter()
            .map(|project| (project, surface_layout()))
            .collect(),
        hide_status_in_zen: true,
        resizer_thickness: SPLIT_THICKNESS,
        welcome_on_empty_editor: true,
    }
}

#[test]
fn 실제_레이아웃_렌더는_두_프로젝트와_zen_보조창_범위를_분리한다() {
    let mut snapshot = fixture();
    let normal = render(&shell(WindowScope::Main), &snapshot);
    assert_eq!(normal.panes.len(), snapshot.projects.len());
    assert_eq!(normal.explorers.len(), snapshot.projects.len());
    for (project, slot) in &normal.explorers {
        assert_eq!(
            taide_native_ui::snapshot::slot_project(snapshot.shell.tree.as_ref().unwrap(), slot),
            Some(project)
        );
    }
    assert_eq!(
        normal.status_projects,
        vec![snapshot.focused_project().cloned()]
    );
    assert!(!normal.panes[0].3.intersects(normal.panes[1].3));
    let layout = snapshot.layouts.get_mut(&snapshot.projects[0].id).unwrap();
    let auxiliary = surface_layout();
    let expected_tab = match &auxiliary.root {
        PaneNode::Leaf { active, .. } => active.clone().unwrap(),
        _ => panic!("expected leaf"),
    };
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: AUX_SLOT,
        root: auxiliary.root,
        focused_pane: auxiliary.focused_pane,
    });
    snapshot.shell.window_chrome.zen = true;
    let zen = render(&shell(WindowScope::Main), &snapshot);
    assert_eq!(zen.panes.len(), 1);
    assert_eq!(&zen.panes[0].0, snapshot.focused_project().unwrap());
    assert!(zen.explorers.is_empty());
    assert!(zen.status_projects.is_empty());
    let auxiliary = render(
        &shell(WindowScope::Auxiliary {
            project: snapshot.projects[0].id.clone(),
            slot: AUX_SLOT,
        }),
        &snapshot,
    );
    assert_eq!(auxiliary.panes.len(), 1);
    assert_eq!(auxiliary.panes[0].2, expected_tab);
    assert!(auxiliary.explorers.is_empty());
    assert!(auxiliary.status_projects.is_empty());
}

#[test]
fn problems_panel은_열린_슬롯의_편집기_영역만_분리한다() {
    let mut snapshot = fixture();
    snapshot.resizer_thickness = PROBLEMS_DIVIDER_THICKNESS;
    let slot = snapshot.shell.focused.clone().unwrap();
    let baseline = render(&shell(WindowScope::Main), &snapshot);
    let mut surfaces = Surfaces {
        problems_open: std::collections::HashSet::from([slot.clone()]),
        ..Default::default()
    };
    let context = egui::Context::default();
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]),
            )),
            ..Default::default()
        },
        |ui| {
            shell(WindowScope::Main).show(ui, &snapshot, &mut surfaces);
        },
    );
    output.textures_delta.clear();
    assert_eq!(surfaces.problems.len(), 1);
    let (mounted, panel) = &surfaces.problems[0];
    assert_eq!(mounted, &slot);
    assert!(panel.height() >= 120.0 && panel.height() <= 220.0);
    let project = snapshot.focused_project().unwrap();
    let content = surfaces
        .panes
        .iter()
        .find(|pane| &pane.0 == project)
        .unwrap()
        .3;
    assert!(!content.intersects(*panel));
    assert!(content.bottom() <= panel.top());
    assert_eq!(panel.top() - content.bottom(), PROBLEMS_DIVIDER_THICKNESS);
    let unaffected = surfaces
        .panes
        .iter()
        .find(|pane| &pane.0 != project)
        .unwrap();
    assert_eq!(
        unaffected.3,
        baseline
            .panes
            .iter()
            .find(|pane| pane.0 == unaffected.0)
            .unwrap()
            .3
    );
    let divider = egui::Id::new(("problems-divider", &slot));
    let initial_height = panel.height();
    let extent = panel.height() + content.height() + taide_native_ui::shell::TAB_HEIGHT;
    context.memory_mut(|memory| memory.request_focus(divider));
    let run = |events: Vec<egui::Event>, size: [f32; 2], is_open: bool| {
        let mut surfaces = Surfaces::default();
        if is_open {
            surfaces.problems_open.insert(slot.clone());
        }
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(size[0], size[1]),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                shell(WindowScope::Main).show(ui, &snapshot, &mut surfaces);
            },
        );
        output.textures_delta.clear();
        surfaces
    };
    let key = |key| {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }]
    };
    run(Vec::new(), WINDOW_SIZE, true);
    run(key(egui::Key::ArrowUp), WINDOW_SIZE, true);
    let resized = run(Vec::new(), WINDOW_SIZE, true);
    assert!(
        (resized.problems[0].1.height() - initial_height - extent * PROBLEMS_KEYBOARD_FRACTION)
            .abs()
            < GEOMETRY_EPSILON,
        "initial={initial_height}, resized={}, extent={extent}, focused={:?}",
        resized.problems[0].1.height(),
        context.memory(|memory| memory.focused())
    );
    run(key(egui::Key::End), WINDOW_SIZE, true);
    let minimum = run(Vec::new(), WINDOW_SIZE, true);
    assert!((minimum.problems[0].1.height() - PROBLEMS_MIN_HEIGHT).abs() < GEOMETRY_EPSILON);
    run(key(egui::Key::Home), WINDOW_SIZE, true);
    let maximum = run(Vec::new(), WINDOW_SIZE, true);
    assert!(
        (maximum.problems[0].1.height() - extent * PROBLEMS_MAX_FRACTION).abs() < GEOMETRY_EPSILON
    );
    run(Vec::new(), WINDOW_SIZE, true);
    let start = context.read_response(divider).unwrap().rect.center();
    let pointer = |position, pressed| {
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    };
    run(pointer(start, true), WINDOW_SIZE, true);
    let moved = start + vec2(0.0, RESIZE_DELTA);
    run(vec![egui::Event::PointerMoved(moved)], WINDOW_SIZE, true);
    let dragged = run(pointer(moved, false), WINDOW_SIZE, true);
    assert!(
        (maximum.problems[0].1.height() - dragged.problems[0].1.height() - RESIZE_DELTA).abs()
            < GEOMETRY_EPSILON,
        "maximum={}, dragged={}, response={:?}",
        maximum.problems[0].1.height(),
        dragged.problems[0].1.height(),
        context.read_response(divider)
    );
    let closed = run(Vec::new(), WINDOW_SIZE, false);
    assert!(closed.problems.is_empty());
    let reopened = run(Vec::new(), WINDOW_SIZE, true);
    assert!((reopened.problems[0].1.height() - PROBLEMS_DEFAULT_HEIGHT).abs() < GEOMETRY_EPSILON);
}

#[test]
fn problems_낮은창은_원본의_상충_최소크기_flex배분과_고정_separator를_유지한다() {
    let mut snapshot = fixture();
    snapshot.resizer_thickness = PROBLEMS_DIVIDER_THICKNESS;
    let slot = snapshot.shell.focused.clone().unwrap();
    let divider = egui::Id::new(("problems-divider", &slot));
    let context = egui::Context::default();
    context.enable_accesskit();
    let run = |events, height| {
        let mut surfaces = Surfaces {
            problems_open: std::collections::HashSet::from([slot.clone()]),
            ..Default::default()
        };
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(WINDOW_SIZE[0], height),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                shell(WindowScope::Main).show(ui, &snapshot, &mut surfaces);
            },
        );
        output.textures_delta.clear();
        let access = output.platform_output.accesskit_update.as_ref().unwrap();
        let node = access
            .nodes
            .iter()
            .find(|(id, _)| *id == divider.accesskit_id())
            .unwrap()
            .1
            .clone();
        let editor = access
            .nodes
            .iter()
            .find(|(id, _)| *id == node.controls()[0])
            .unwrap()
            .1
            .bounds()
            .unwrap();
        (surfaces.problems[0].1, editor, node)
    };
    for height in LOW_WINDOW_HEIGHTS {
        let (panel, editor, node) = run(Vec::new(), height);
        let extent = panel.bottom() - editor.y0 as f32 - PROBLEMS_DIVIDER_THICKNESS;
        let percent_total = taide_native_ui::split::PERCENT_TOTAL;
        let editor_weight = (1.0 - PROBLEMS_MAX_FRACTION) * percent_total;
        let panel_weight =
            ((PROBLEMS_MIN_HEIGHT / extent * percent_total * PANEL_PERCENT_PRECISION).round()
                / PANEL_PERCENT_PRECISION)
                .min(percent_total);
        assert!(panel_weight > PROBLEMS_MAX_FRACTION * percent_total);
        let expected = extent * panel_weight / (editor_weight + panel_weight);
        assert!(
            (panel.height() - expected).abs() < GEOMETRY_EPSILON,
            "extent={extent}, actual={}, expected={expected}",
            panel.height()
        );
        assert!(
            (editor.y1 as f32 - editor.y0 as f32 + panel.height() - extent).abs()
                < GEOMETRY_EPSILON
        );
        for value in [
            node.numeric_value(),
            node.min_numeric_value(),
            node.max_numeric_value(),
        ] {
            assert!((value.unwrap() as f32 - editor_weight).abs() < GEOMETRY_EPSILON);
        }
        context.memory_mut(|memory| memory.request_focus(divider));
        run(Vec::new(), height);
        for key in [
            egui::Key::ArrowUp,
            egui::Key::ArrowDown,
            egui::Key::Home,
            egui::Key::End,
        ] {
            run(
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                height,
            );
            let (unchanged, _, _) = run(
                vec![egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                height,
            );
            assert!((unchanged.height() - expected).abs() < GEOMETRY_EPSILON);
        }
    }
    let (restored, editor, _) = run(Vec::new(), WINDOW_SIZE[1]);
    let extent = restored.bottom() - editor.y0 as f32 - PROBLEMS_DIVIDER_THICKNESS;
    assert!((restored.height() - extent * PROBLEMS_MAX_FRACTION).abs() < GEOMETRY_EPSILON);
}

#[test]
fn problems_separator는_실제_editor를_제어하고_창_resize와_f6_그룹포커스를_보존한다() {
    let mut snapshot = fixture();
    snapshot.resizer_thickness = PROBLEMS_DIVIDER_THICKNESS;
    let slot = snapshot.shell.focused.clone().unwrap();
    let project = snapshot.focused_project().unwrap();
    let divider = egui::Id::new(("problems-divider", &slot));
    let context = egui::Context::default();
    context.enable_accesskit();
    let run = |events, height| {
        let mut surfaces = Surfaces {
            problems_open: std::collections::HashSet::from([slot.clone()]),
            ..Default::default()
        };
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(WINDOW_SIZE[0], height),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                shell(WindowScope::Main).show(ui, &snapshot, &mut surfaces);
            },
        );
        output.textures_delta.clear();
        (surfaces, output)
    };
    let (initial, output) = run(Vec::new(), WINDOW_SIZE[1]);
    let access = output.platform_output.accesskit_update.as_ref().unwrap();
    let node = access
        .nodes
        .iter()
        .find(|(id, _)| *id == divider.accesskit_id())
        .unwrap()
        .1
        .clone();
    assert_eq!(node.role(), egui::accesskit::Role::Splitter);
    assert_eq!(node.controls().len(), 1);
    let controlled = access
        .nodes
        .iter()
        .find(|(id, _)| *id == node.controls()[0])
        .unwrap()
        .1
        .clone();
    assert_eq!(controlled.role(), egui::accesskit::Role::Group);
    let content = initial
        .panes
        .iter()
        .find(|pane| &pane.0 == project)
        .unwrap()
        .3;
    let bounds = controlled.bounds().unwrap();
    assert!((bounds.x0 as f32 - content.left()).abs() < GEOMETRY_EPSILON);
    assert!((bounds.y1 as f32 - content.bottom()).abs() < GEOMETRY_EPSILON);
    assert!(!controlled.children().is_empty());
    let extent =
        content.height() + taide_native_ui::shell::TAB_HEIGHT + initial.problems[0].1.height();
    let fraction = initial.problems[0].1.height() / extent;
    let (resized, _) = run(Vec::new(), RESIZED_WINDOW_HEIGHT);
    let content = resized
        .panes
        .iter()
        .find(|pane| &pane.0 == project)
        .unwrap()
        .3;
    let resized_extent =
        content.height() + taide_native_ui::shell::TAB_HEIGHT + resized.problems[0].1.height();
    assert!((resized.problems[0].1.height() - fraction * resized_extent).abs() < GEOMETRY_EPSILON);
    context.memory_mut(|memory| memory.request_focus(divider));
    run(Vec::new(), RESIZED_WINDOW_HEIGHT);
    for modifiers in [egui::Modifiers::NONE, egui::Modifiers::SHIFT] {
        run(
            vec![egui::Event::Key {
                key: egui::Key::F6,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            RESIZED_WINDOW_HEIGHT,
        );
        assert_eq!(context.memory(|memory| memory.focused()), Some(divider));
        assert!(!context.input(|input| input.key_pressed(egui::Key::F6)));
        run(
            vec![egui::Event::Key {
                key: egui::Key::F6,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers,
            }],
            RESIZED_WINDOW_HEIGHT,
        );
    }
}

#[test]
fn 분할_좌표와_resize는_퍼센트_보존과_최소크기를_유지한다() {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]));
    assert_eq!(normalized_sizes(&[f32::NAN], 2), vec![50.0, 50.0]);
    for dir in [SplitDir::Horizontal, SplitDir::Vertical] {
        let regions = child_rects(rect, dir, &[25.0, 75.0], SPLIT_THICKNESS);
        assert_eq!(regions.len(), 2);
        assert!(rect.contains_rect(regions[0]));
        assert!(rect.contains_rect(regions[1]));
        assert!(!regions[0].intersects(regions[1]));
    }
    let resized = resized_pair(&[50.0, 50.0], 0, RESIZE_DELTA, WINDOW_SIZE[0]).unwrap();
    assert_eq!(resized.iter().sum::<f32>(), 100.0);
    let clamped = resized_pair(&resized, 0, -WINDOW_SIZE[0], WINDOW_SIZE[0]).unwrap();
    assert_eq!(clamped[0] / 100.0 * WINDOW_SIZE[0], MIN_PANE_SIZE);
    assert!(resized_pair(&resized, 1, RESIZE_DELTA, WINDOW_SIZE[0]).is_none());
}

struct CssVariables {
    theme: &'static str,
    is_dark: bool,
    background: u32,
    foreground: u32,
    border: u32,
    ring: u32,
    secondary: u32,
    secondary_foreground: u32,
    button_hover: u32,
    input_background: u32,
    input_border: u32,
    list_active: u32,
    accent_foreground: u32,
    app_accent: u32,
    muted_foreground: u32,
    destructive: u32,
    warning: u32,
    menu_background: u32,
    menu_border: u32,
    menu_item_hover: u32,
    shadow_alpha: u8,
}

const BUILTIN_CSS_VARIABLES: [CssVariables; 2] = [
    CssVariables {
        theme: "taide-dark",
        is_dark: true,
        background: 0x1e1e2e,
        foreground: 0xcdd6f4,
        border: 0x313244,
        ring: 0x89b4fa,
        secondary: 0x313244,
        secondary_foreground: 0xcdd6f4,
        button_hover: 0x45475a,
        input_background: 0x1e1e2e,
        input_border: 0x313244,
        list_active: 0x45475a,
        accent_foreground: 0xcdd6f4,
        app_accent: 0x89b4fa,
        muted_foreground: 0xa6adc8,
        destructive: 0xf38ba8,
        warning: 0xf9e2af,
        menu_background: 0x181825,
        menu_border: 0x313244,
        menu_item_hover: 0x45475a,
        shadow_alpha: 0x66,
    },
    CssVariables {
        theme: "taide-light",
        is_dark: false,
        background: 0xeff1f5,
        foreground: 0x4c4f69,
        border: 0xccd0da,
        ring: 0x1e66f5,
        secondary: 0xccd0da,
        secondary_foreground: 0x4c4f69,
        button_hover: 0xbcc0cc,
        input_background: 0xeff1f5,
        input_border: 0xccd0da,
        list_active: 0xbcc0cc,
        accent_foreground: 0x4c4f69,
        app_accent: 0x1e66f5,
        muted_foreground: 0x6c6f85,
        destructive: 0xd20f39,
        warning: 0xba7718,
        menu_background: 0xe6e9ef,
        menu_border: 0xccd0da,
        menu_item_hover: 0xbcc0cc,
        shadow_alpha: 0x26,
    },
];

#[test]
fn 테마_토큰은_원본_css변수의_egui_visuals로_변환되어_두_테마_슬롯에_적용된다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-visuals-{}", ProjectId::new())),
    ));
    for css in BUILTIN_CSS_VARIABLES {
        let theme = theme_actions::theme_get(&state, css.theme.into()).unwrap();
        let converted = visuals(&theme).unwrap();
        let shadow = Color32::from_rgba_unmultiplied(0, 0, 0, css.shadow_alpha);
        assert_eq!(converted.dark_mode, css.is_dark);
        assert_eq!(converted.panel_fill, rgb(css.background));
        assert_eq!(converted.text_color(), rgb(css.foreground));
        assert_eq!(
            converted.widgets.noninteractive.bg_stroke,
            Stroke::new(CSS_BORDER_WIDTH, rgb(css.border))
        );
        assert_eq!(converted.widgets.inactive.weak_bg_fill, rgb(css.secondary));
        assert_eq!(converted.widgets.inactive.bg_fill, rgb(css.secondary));
        assert_eq!(
            converted.widgets.inactive.bg_stroke,
            Stroke::new(CSS_BORDER_WIDTH, rgb(css.input_border))
        );
        assert_eq!(
            converted.widgets.hovered.weak_bg_fill,
            rgb(css.button_hover)
        );
        assert_eq!(
            converted.widgets.active.bg_stroke,
            Stroke::new(CSS_BORDER_WIDTH, rgb(css.ring))
        );
        for widget in [
            converted.widgets.inactive,
            converted.widgets.hovered,
            converted.widgets.active,
        ] {
            assert_eq!(widget.text_color(), rgb(css.secondary_foreground));
            assert_eq!(widget.corner_radius, CornerRadius::same(CSS_RADIUS_MEDIUM));
        }
        assert_eq!(
            converted.widgets.open.weak_bg_fill,
            rgb(css.menu_item_hover)
        );
        assert_eq!(converted.text_edit_bg_color(), rgb(css.input_background));
        assert_eq!(converted.text_cursor.stroke.color, rgb(css.foreground));
        assert_eq!(converted.selection.bg_fill, rgb(css.list_active));
        assert_eq!(
            converted.selection.stroke,
            Stroke::new(CSS_BORDER_WIDTH, rgb(css.accent_foreground))
        );
        assert_eq!(converted.hyperlink_color, rgb(css.app_accent));
        assert_eq!(converted.weak_text_color(), rgb(css.muted_foreground));
        assert_eq!(converted.error_fg_color, rgb(css.destructive));
        assert_eq!(converted.warn_fg_color, rgb(css.warning));
        assert_eq!(converted.window_fill, rgb(css.menu_background));
        assert_eq!(
            converted.window_stroke,
            Stroke::new(CSS_BORDER_WIDTH, rgb(css.menu_border))
        );
        assert_eq!(
            converted.menu_corner_radius,
            CornerRadius::same(CSS_RADIUS_MEDIUM)
        );
        assert_eq!(
            converted.window_corner_radius,
            CornerRadius::same(CSS_RADIUS_LARGE)
        );
        assert_eq!(
            converted.popup_shadow,
            Shadow {
                offset: CSS_SHADOW_OVERLAY.0,
                blur: CSS_SHADOW_OVERLAY.1,
                spread: 0,
                color: shadow,
            }
        );
        assert_eq!(
            converted.window_shadow,
            Shadow {
                offset: CSS_SHADOW_OVERLAY_LARGE.0,
                blur: CSS_SHADOW_OVERLAY_LARGE.1,
                spread: 0,
                color: shadow,
            }
        );
        let context = egui::Context::default();
        apply_visuals(&context, &converted);
        for slot in [egui::Theme::Dark, egui::Theme::Light] {
            assert_eq!(context.style_of(slot).visuals, converted);
        }
        let mut incomplete = theme;
        incomplete.colors.remove("menu.background");
        assert!(visuals(&incomplete).is_err());
    }
}

#[test]
fn 프로젝트_rail_구역은_열린_순서와_마지막_그룹_소속으로_한번씩만_배치한다() {
    let mut snapshot = fixture();
    let template = snapshot.projects[0].clone();
    snapshot.projects.push(ProjectRef {
        id: ProjectId::new(),
        ..template
    });
    let ids: Vec<_> = snapshot
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    let group = |members: Vec<ProjectId>| ProjectGroup {
        id: ProjectGroupId::new(),
        name: String::new(),
        color: None,
        members,
        collapsed: false,
    };
    snapshot.groups = vec![
        group(vec![ids[2].clone(), ids[0].clone(), ProjectId::new()]),
        group(vec![ids[0].clone()]),
    ];
    let rail = project_group_sections(&snapshot.projects, &snapshot.groups);
    let member_ids = |index: usize| -> Vec<&ProjectId> {
        rail.sections[index]
            .1
            .iter()
            .map(|project| &project.id)
            .collect()
    };
    assert_eq!(rail.sections.len(), snapshot.groups.len());
    assert_eq!(rail.sections[0].0.id, snapshot.groups[0].id);
    assert_eq!(member_ids(0), [&ids[2]]);
    assert_eq!(member_ids(1), [&ids[0]]);
    assert_eq!(
        rail.ungrouped
            .iter()
            .map(|project| &project.id)
            .collect::<Vec<_>>(),
        [&ids[1]]
    );
    snapshot.groups.reverse();
    let rail = project_group_sections(&snapshot.projects, &snapshot.groups);
    assert!(rail.sections[0].1.is_empty());
    assert_eq!(
        rail.sections[1]
            .1
            .iter()
            .map(|project| &project.id)
            .collect::<Vec<_>>(),
        [&ids[0], &ids[2]]
    );
}

#[test]
fn 빈_pane과_welcome_탭은_설정과_창_범위에_따라_welcome과_파일없음_안내로_나뉜다() {
    let count = |texts: &[String], key: &str| texts.iter().filter(|text| *text == key).count();
    let mut snapshot = fixture();
    let project = snapshot.projects[0].id.clone();
    let mut welcome_tab = snapshot.clone();
    for layout in snapshot.layouts.values_mut() {
        let PaneNode::Leaf { tabs, active, .. } = &mut layout.root else {
            panic!("expected leaf")
        };
        tabs.clear();
        *active = None;
    }
    let (surfaces, texts) = rendered_texts(&shell(WindowScope::Main), &snapshot);
    assert!(surfaces.panes.is_empty());
    assert_eq!(count(&texts, "app.openFolderHint"), snapshot.projects.len());
    assert_eq!(count(&texts, "editor.noFileOpen"), 0);
    snapshot.welcome_on_empty_editor = false;
    let (surfaces, texts) = rendered_texts(&shell(WindowScope::Main), &snapshot);
    assert!(surfaces.panes.is_empty());
    assert_eq!(count(&texts, "app.openFolderHint"), 0);
    assert_eq!(count(&texts, "editor.noFileOpen"), snapshot.projects.len());
    welcome_tab.welcome_on_empty_editor = false;
    for layout in welcome_tab.layouts.values_mut() {
        *layout = taide_layout::service::default_layout();
    }
    let (surfaces, texts) = rendered_texts(&shell(WindowScope::Main), &welcome_tab);
    assert!(surfaces.panes.is_empty());
    assert_eq!(
        count(&texts, "app.openFolderHint"),
        welcome_tab.projects.len()
    );
    snapshot.welcome_on_empty_editor = true;
    let empty = snapshot.layouts[&project].clone();
    snapshot
        .layouts
        .get_mut(&project)
        .unwrap()
        .auxiliary_windows
        .push(AuxWindowLayout {
            slot: AUX_SLOT,
            root: empty.root,
            focused_pane: empty.focused_pane,
        });
    let (_, texts) = rendered_texts(
        &shell(WindowScope::Auxiliary {
            project,
            slot: AUX_SLOT,
        }),
        &snapshot,
    );
    assert_eq!(count(&texts, "app.openFolderHint"), 0);
    assert_eq!(count(&texts, "editor.noFileOpen"), 1);
}

#[test]
fn 타이틀바는_mac_전용_drag_region이고_없으면_본문이_창_위에서_시작한다() {
    let snapshot = fixture();
    let titled = shell(WindowScope::Main);
    let plain = NativeShell {
        has_title_bar: false,
        ..shell(WindowScope::Main)
    };
    let with_title = render(&titled, &snapshot);
    let without_title = render(&plain, &snapshot);
    for (project, _, _, rect) in &with_title.panes {
        let other = without_title
            .panes
            .iter()
            .find(|pane| &pane.0 == project)
            .unwrap()
            .3;
        assert_eq!(rect.top() - other.top(), TITLE_HEIGHT);
    }
    let position = pos2(WINDOW_SIZE[0] / 2.0, TITLE_HEIGHT / 2.0);
    let button = |pressed| egui::Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let starts_drag = |output: &egui::FullOutput| {
        output
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .any(|command| matches!(command, egui::ViewportCommand::StartDrag))
    };
    let drag = |shell: &NativeShell| {
        let context = egui::Context::default();
        frame(&context, shell, &snapshot, 0.0, Vec::new());
        let (_, pressed) = frame(
            &context,
            shell,
            &snapshot,
            CLICK_INTERVAL_SECONDS,
            vec![egui::Event::PointerMoved(position), button(true)],
        );
        let (_, dragged) = frame(
            &context,
            shell,
            &snapshot,
            CLICK_INTERVAL_SECONDS * 2.0,
            vec![egui::Event::PointerMoved(
                position + vec2(DRAG_DISTANCE, 0.0),
            )],
        );
        (starts_drag(&pressed), starts_drag(&dragged))
    };
    assert_eq!(drag(&titled), (false, true));
    assert_eq!(drag(&plain), (false, false));
    let context = egui::Context::default();
    frame(&context, &titled, &snapshot, 0.0, Vec::new());
    let mut is_maximize_requested = false;
    for (step, pressed) in [true, false, true, false].into_iter().enumerate() {
        let (_, output) = frame(
            &context,
            &titled,
            &snapshot,
            CLICK_INTERVAL_SECONDS * (step + 1) as f64,
            vec![egui::Event::PointerMoved(position), button(pressed)],
        );
        assert!(!starts_drag(&output));
        is_maximize_requested |= output
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .any(|command| matches!(command, egui::ViewportCommand::Maximized(true)));
    }
    assert!(is_maximize_requested);
}

#[test]
fn 탭바는_세로_휠을_가로_스크롤로_바꾼다() {
    let mut snapshot = fixture();
    let project = snapshot.focused_project().unwrap().clone();
    let layout = snapshot.layouts.get_mut(&project).unwrap();
    let PaneNode::Leaf { tabs, active, .. } = &mut layout.root else {
        panic!("expected leaf")
    };
    *tabs = (0..OVERFLOWING_TAB_COUNT)
        .map(|index| Tab {
            id: TabId::new(),
            kind: TabKind::Settings,
            title: format!("synthetic-tab-{index}"),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        })
        .collect();
    *active = tabs.first().map(|tab| tab.id.clone());
    let first_title = tabs[0].title.clone();
    let shell = shell(WindowScope::Main);
    let context = egui::Context::default();
    context.enable_accesskit();
    let tab_left = |output: &egui::FullOutput| {
        output
            .platform_output
            .accesskit_update
            .as_ref()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some(first_title.as_str()))
            .and_then(|(_, node)| node.bounds())
            .unwrap()
            .x0
    };
    let (surfaces, output) = frame(&context, &shell, &snapshot, 0.0, Vec::new());
    let before = tab_left(&output);
    let content = surfaces
        .panes
        .iter()
        .find(|pane| pane.0 == project)
        .unwrap()
        .3;
    let hover = pos2(
        content.center().x,
        content.top() - taide_native_ui::shell::TAB_HEIGHT / 2.0,
    );
    let mut output = frame(
        &context,
        &shell,
        &snapshot,
        FRAME_SECONDS,
        vec![
            egui::Event::PointerMoved(hover),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, WHEEL_DELTA),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    )
    .1;
    for step in 2..WHEEL_SETTLE_FRAMES {
        output = frame(
            &context,
            &shell,
            &snapshot,
            FRAME_SECONDS * step as f64,
            Vec::new(),
        )
        .1;
    }
    assert!(tab_left(&output) < before);
}

#[test]
fn 탭은_제목_앞에_14px_아이콘_칸을_두고_6px_뒤에_제목을_그린다() {
    let mut snapshot = fixture();
    let project = snapshot.focused_project().unwrap().clone();
    let layout = snapshot.layouts.get_mut(&project).unwrap();
    let PaneNode::Leaf { tabs, active, .. } = &mut layout.root else {
        panic!("expected leaf")
    };
    *tabs = ["synthetic-first", "synthetic-second"]
        .into_iter()
        .map(|title| Tab {
            id: TabId::new(),
            kind: TabKind::Settings,
            title: title.into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        })
        .collect();
    *active = tabs.first().map(|tab| tab.id.clone());
    let tabs = tabs.clone();
    let (surfaces, output) = frame(
        &egui::Context::default(),
        &shell(WindowScope::Main),
        &snapshot,
        0.0,
        Vec::new(),
    );
    let content = surfaces
        .panes
        .iter()
        .find(|pane| pane.0 == project)
        .unwrap()
        .3;
    let bar_top = content.top() - taide_native_ui::shell::TAB_HEIGHT;
    let title = |tab: &Tab| {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text)
                    if text.galley.text() == tab.title
                        && (bar_top..content.top()).contains(&text.pos.y) =>
                {
                    Some((
                        Rect::from_min_size(text.pos, text.galley.size()),
                        text.fallback_color,
                    ))
                }
                _ => None,
            })
            .unwrap()
    };
    let icons = tabs
        .iter()
        .map(|tab| {
            let recorded = surfaces
                .tab_icons
                .iter()
                .filter(|(id, _, _)| id == &tab.id)
                .collect::<Vec<_>>();
            assert_eq!(recorded.len(), 1, "{}", tab.title);
            (recorded[0].1, recorded[0].2)
        })
        .collect::<Vec<_>>();
    for (tab, (icon, icon_color)) in tabs.iter().zip(&icons) {
        let (title, title_color) = title(tab);
        assert_eq!(icon.size(), vec2(TAB_ICON_SIDE, TAB_ICON_SIDE));
        assert_eq!(
            icon.min,
            icon.min.round(),
            "{}: 아이콘은 픽셀 격자에 맞춰 그린다",
            tab.title
        );
        assert!(
            (title.left() - icon.right() - TAB_ICON_GAP).abs() <= PIXEL_SNAP_TOLERANCE,
            "{}: {icon:?} {title:?}",
            tab.title
        );
        assert!(
            (title.center().y - icon.center().y).abs() <= PIXEL_SNAP_TOLERANCE,
            "{}: {icon:?} {title:?}",
            tab.title
        );
        assert!(icon.top() >= bar_top && icon.bottom() <= content.top());
        assert_eq!(
            *icon_color, title_color,
            "{}: 종류 색이 없는 아이콘은 제목 글자색을 따른다",
            tab.title
        );
    }
    assert_ne!(
        icons[0].1, icons[1].1,
        "활성 탭과 비활성 탭의 제목 글자색이 다르다"
    );
    assert!(
        (icons[0].0.left() - content.left()).abs() <= PIXEL_SNAP_TOLERANCE,
        "{icons:?} {content:?}"
    );
    assert!(icons[1].0.left() - icons[0].0.left() >= taide_native_ui::shell::TAB_MIN_WIDTH);
}

#[tokio::test]
async fn native_mutation은_기존_runtime의_revision_이벤트와_dirty_보호를_사용한다() {
    let dir = std::env::temp_dir().join(format!("taide-native-workbench-{}", ProjectId::new()));
    let state = AppState::new(AppPaths::new(dir.clone()));
    let project = ProjectId::new();
    let layout = taide_layout::service::default_layout();
    let PaneNode::Leaf { tabs, .. } = &layout.root else {
        panic!("expected leaf")
    };
    let target = tabs[1].id.clone();
    let pane = layout.focused_pane.clone();
    state.layouts.write().insert(project.clone(), layout);
    let sink = Sink::default();
    dispatch(&sink, &state, ShellMutation::ActivateTab(target.clone()))
        .await
        .unwrap();
    dispatch(
        &sink,
        &state,
        ShellMutation::PinTab {
            tab: target.clone(),
            pinned: true,
        },
    )
    .await
    .unwrap();
    dispatch(
        &sink,
        &state,
        ShellMutation::SetSidebarCollapsed {
            project: project.clone(),
            collapsed: true,
        },
    )
    .await
    .unwrap();
    let snapshot = ShellSnapshot::read(&state).await;
    let layout = &snapshot.layouts[&project];
    let tab = taide_native_ui::snapshot::active_tab(&layout.root, &pane).unwrap();
    assert_eq!(tab.id, target);
    assert!(tab.pinned);
    assert!(request_close_tab(tab).is_err());
    let mut dirty = tab.clone();
    dirty.pinned = false;
    dirty.dirty = true;
    assert!(
        matches!(request_close_tab(&dirty).unwrap(), ShellIntent::RequestCloseTab(id) if id == target)
    );
    assert!(layout.shell_view.sidebar_collapsed);
    assert!(layout.revision > 0);
    assert!(state.dirty_layouts.read().contains(&project));
    assert_eq!(sink.0.lock().unwrap().len(), 3);
    assert!(!dir.exists());
}

fn welcome_frame(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    content: &WelcomeContent,
    time: f64,
    events: Vec<egui::Event>,
) -> (Vec<ShellIntent>, egui::FullOutput) {
    let mut surfaces = Surfaces {
        welcome: Some(content.clone()),
        ..Default::default()
    };
    let shell = shell(WindowScope::Main);
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            pos2(0.0, 0.0),
            vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]),
        )),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut intents = Vec::new();
    let mut output = context.run_ui(input, |ui| {
        intents = shell.show(ui, snapshot, &mut surfaces);
    });
    output.textures_delta.clear();
    (intents, output)
}

fn welcome_text_rect(output: &egui::FullOutput, needle: &str) -> Option<Rect> {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) if text.galley.text().contains(needle) => {
                Some(text.visual_bounding_rect())
            }
            _ => None,
        })
}

#[test]
fn welcome는_최근_프로젝트_root누락_비활성과_조회실패와_단축키카드를_원본처럼_보인다() {
    let mut snapshot = fixture();
    snapshot.projects.clear();
    snapshot.shell.tree = None;
    snapshot.shell.focused = None;
    snapshot.layouts.clear();
    let open = ProjectId::new();
    let missing = ProjectId::new();
    let content = WelcomeContent {
        recent: Some(vec![
            WelcomeRecent {
                id: open.clone(),
                name: "alpha".into(),
                root: "/work/alpha".into(),
                root_missing: false,
            },
            WelcomeRecent {
                id: missing.clone(),
                name: "beta".into(),
                root: "/work/beta".into(),
                root_missing: true,
            },
        ]),
        shortcuts: vec![
            WelcomeShortcutRow {
                description: "Quick Open File".into(),
                label: "⌘P".into(),
            },
            WelcomeShortcutRow {
                description: "Save".into(),
                label: String::new(),
            },
        ],
    };
    let context = egui::Context::default();
    let (_, output) = welcome_frame(&context, &snapshot, &content, 0.0, Vec::new());
    let texts = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for expected in [
        "app.recentItems",
        "app.keyboardShortcutsTitle",
        "Quick Open File",
        "⌘P",
        "Save",
    ] {
        assert!(
            texts.iter().any(|text| text == expected),
            "{expected}: {texts:?}"
        );
    }
    assert!(texts.iter().any(|text| text == "alpha\n/work/alpha"));
    assert!(
        texts
            .iter()
            .any(|text| text == "beta\napp.recentProjectRootMissing /work/beta")
    );
    let click = |rect: Rect| {
        let position = rect.center();
        vec![
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
        ]
    };
    let beta = welcome_text_rect(&output, "beta").unwrap();
    let (intents, _) = welcome_frame(&context, &snapshot, &content, FRAME_SECONDS, click(beta));
    assert!(
        !intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::SelectRecentProject { .. }))
    );
    let alpha = welcome_text_rect(&output, "alpha").unwrap();
    let (intents, _) = welcome_frame(
        &context,
        &snapshot,
        &content,
        FRAME_SECONDS * 2.0,
        click(alpha),
    );
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::SelectRecentProject { project, root } if *project == open && root == "/work/alpha"
    )));
    let unavailable = WelcomeContent {
        recent: None,
        shortcuts: Vec::new(),
    };
    let (_, output) = welcome_frame(
        &egui::Context::default(),
        &snapshot,
        &unavailable,
        0.0,
        Vec::new(),
    );
    assert!(welcome_text_rect(&output, "app.recentProjectsUnavailable").is_some());
    assert!(welcome_text_rect(&output, "app.recentItems").is_none());
    assert!(welcome_text_rect(&output, "app.keyboardShortcutsTitle").is_none());
}

#[test]
fn 탭바_하단선은_layout의_포커스_pane만_focus_border로_그린다() {
    const FOCUS: Color32 = Color32::from_rgb(1, 2, 3);
    const TAB_BORDER: Color32 = Color32::from_rgb(4, 5, 6);
    let mut snapshot = fixture();
    let project = snapshot.projects[0].id.clone();
    let layout = snapshot.layouts.get_mut(&project).unwrap();
    let PaneNode::Leaf { tabs, .. } = &layout.root else {
        panic!("expected leaf")
    };
    let moved = tabs[0].id.clone();
    let first = layout.focused_pane.clone();
    taide_layout::service::split(layout, &first, taide_model::layout::DropEdge::Right, &moved)
        .unwrap();
    let PaneNode::Split { children, .. } = &layout.root else {
        panic!("expected split")
    };
    let leaves = children
        .iter()
        .map(|child| match child {
            PaneNode::Leaf { id, .. } => id.clone(),
            PaneNode::Split { .. } => panic!("expected leaves"),
        })
        .collect::<Vec<_>>();
    let mut shell = shell(WindowScope::Main);
    shell.colors.focus_border = FOCUS;
    shell.colors.tab_border = TAB_BORDER;
    let lines = |snapshot: &ShellSnapshot| {
        let (surfaces, output) =
            frame(&egui::Context::default(), &shell, snapshot, 0.0, Vec::new());
        let lines = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::LineSegment { points, stroke } if points[0].y == points[1].y => {
                    Some((points[0], points[1], stroke.color))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        (surfaces, lines)
    };
    for (index, focused) in leaves.iter().enumerate() {
        snapshot.layouts.get_mut(&project).unwrap().focused_pane = focused.clone();
        let (_, lines) = lines(&snapshot);
        let half = WINDOW_SIZE[0] / 2.0;
        let lines = lines
            .into_iter()
            .filter(|(start, _, _)| start.x < half)
            .collect::<Vec<_>>();
        let focus = lines
            .iter()
            .filter(|(_, _, color)| *color == FOCUS)
            .collect::<Vec<_>>();
        let plain = lines
            .iter()
            .filter(|(_, _, color)| *color == TAB_BORDER)
            .collect::<Vec<_>>();
        assert_eq!(focus.len(), 1, "{lines:?}");
        assert_eq!(plain.len(), 1, "{lines:?}");
        let sibling = plain
            .iter()
            .filter(|(start, _, _)| (start.y - focus[0].0.y).abs() < 0.01)
            .min_by(|left, right| {
                (left.0.x - focus[0].0.x)
                    .abs()
                    .total_cmp(&(right.0.x - focus[0].0.x).abs())
            })
            .unwrap();
        if index == 0 {
            assert!(focus[0].0.x < sibling.0.x);
        } else {
            assert!(focus[0].0.x > sibling.0.x);
        }
    }
    snapshot.shell.window_chrome.zen = true;
    let (_, lines) = lines(&snapshot);
    assert!(
        lines
            .iter()
            .all(|(_, _, color)| *color != FOCUS && *color != TAB_BORDER)
    );
}

fn tab_menu_frame(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: f64,
    events: Vec<egui::Event>,
) -> (Vec<ShellIntent>, egui::FullOutput) {
    welcome_frame(context, snapshot, &WelcomeContent::default(), time, events)
}

fn texts_of(output: &egui::FullOutput) -> Vec<String> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

fn pointer(position: egui::Pos2, button: egui::PointerButton) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos: position,
            button,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

#[test]
fn 탭_메뉴와_빈공간_메뉴는_원본_항목과_고정탭_제외_닫기와_분할을_요청한다() {
    let mut snapshot = fixture();
    let project = snapshot.projects[0].id.clone();
    snapshot.shell.tree = Some(ShellSlotTree::Leaf {
        slot_id: ShellSlotId::new(),
        project_id: project.clone(),
    });
    snapshot.layouts.retain(|id, _| id == &project);
    snapshot.projects.truncate(1);
    snapshot.projects[0].root = "/synthetic/root".into();
    let layout = snapshot.layouts.get_mut(&project).unwrap();
    let pane = layout.focused_pane.clone();
    let PaneNode::Leaf { tabs, active, .. } = &mut layout.root else {
        panic!("expected leaf")
    };
    let file = |title: &str, pinned: bool, dirty: bool, preview: bool| Tab {
        id: TabId::new(),
        kind: TabKind::File {
            path: format!("/synthetic/root/src/{title}"),
        },
        title: title.into(),
        pinned,
        preview,
        dirty,
        view_state: None,
    };
    tabs.clear();
    tabs.extend([
        file("pinned.rs", true, false, false),
        file("a.rs", false, true, false),
        file("b.rs", false, false, true),
        file("c.rs", false, false, false),
    ]);
    let ids = tabs.iter().map(|tab| tab.id.clone()).collect::<Vec<_>>();
    *active = Some(ids[2].clone());
    let context = egui::Context::default();
    let (_, output) = tab_menu_frame(&context, &snapshot, 0.0, Vec::new());
    let b_tab = welcome_text_rect(&output, "b.rs").unwrap();
    let mut time = FRAME_SECONDS;
    let (_, output) = tab_menu_frame(
        &context,
        &snapshot,
        time,
        pointer(b_tab.center(), egui::PointerButton::Secondary),
    );
    time += FRAME_SECONDS;
    let (_, output2) = tab_menu_frame(&context, &snapshot, time, Vec::new());
    let menu = texts_of(&output2);
    for key in [
        "tab.close",
        "tab.closeOthers",
        "tab.closeToRight",
        "tab.closeSaved",
        "tab.closeAll",
        "tab.pin",
        "tab.keepOpen",
        "explorer.copyPath",
        "tab.copyRelativePath",
        "explorer.revealInFinder",
        "tab.split",
    ] {
        assert!(menu.iter().any(|text| text == key), "{key}: {menu:?}");
    }
    drop(output);
    let mut choose = |snapshot: &ShellSnapshot, key: &str, time: &mut f64| {
        let (_, output) = tab_menu_frame(&context, snapshot, *time, Vec::new());
        let item = welcome_text_rect(&output, key).unwrap_or_else(|| panic!("{key}"));
        *time += FRAME_SECONDS;
        let (intents, _) = tab_menu_frame(
            &context,
            snapshot,
            *time,
            pointer(item.center(), egui::PointerButton::Primary),
        );
        *time += FRAME_SECONDS;
        intents
    };
    let intents = choose(&snapshot, "tab.closeOthers", &mut time);
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::RequestCloseTabs(closing) if closing == &vec![ids[1].clone(), ids[3].clone()]
    )), "{intents:?}");
    let reopen = |time: &mut f64, target: Rect, button| {
        *time += 1.0;
        let _ = tab_menu_frame(&context, &snapshot, *time, Vec::new());
        *time += FRAME_SECONDS;
        let _ = tab_menu_frame(&context, &snapshot, *time, pointer(target.center(), button));
        *time += FRAME_SECONDS;
    };
    reopen(&mut time, b_tab, egui::PointerButton::Secondary);
    let intents = choose(&snapshot, "tab.closeToRight", &mut time);
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::RequestCloseTabs(closing) if closing == &vec![ids[3].clone()]
    )));
    reopen(&mut time, b_tab, egui::PointerButton::Secondary);
    let intents = choose(&snapshot, "tab.closeSaved", &mut time);
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::RequestCloseTabs(closing) if closing == &vec![ids[2].clone(), ids[3].clone()]
    )));
    reopen(&mut time, b_tab, egui::PointerButton::Secondary);
    let intents = choose(&snapshot, "tab.copyRelativePath", &mut time);
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::CopyText(text) if text == "src/b.rs"
    )));
    reopen(&mut time, b_tab, egui::PointerButton::Secondary);
    let intents = choose(&snapshot, "tab.pin", &mut time);
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::Mutate(ShellMutation::PinTab { tab, pinned: true }) if tab == &ids[2]
    )));
    reopen(&mut time, b_tab, egui::PointerButton::Secondary);
    let (_, output) = tab_menu_frame(&context, &snapshot, time, Vec::new());
    let split = welcome_text_rect(&output, "tab.split").unwrap();
    time += FRAME_SECONDS;
    let _ = tab_menu_frame(
        &context,
        &snapshot,
        time,
        vec![egui::Event::PointerMoved(split.center())],
    );
    time += FRAME_SECONDS;
    for _ in 0..WHEEL_SETTLE_FRAMES {
        let _ = tab_menu_frame(&context, &snapshot, time, Vec::new());
        time += FRAME_SECONDS;
    }
    let intents = choose(&snapshot, "editorArea.splitRight", &mut time);
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::Mutate(ShellMutation::SplitTab { pane: target, edge: DropEdge::Right, tab })
            if target == &pane && tab == &ids[2]
    )), "{intents:?}");

    let (_, output) = tab_menu_frame(&context, &snapshot, time, Vec::new());
    let last = welcome_text_rect(&output, "c.rs").unwrap();
    let filler = pos2(last.right() + 60.0, last.center().y);
    time += FRAME_SECONDS;
    let (_, _) = tab_menu_frame(
        &context,
        &snapshot,
        time,
        pointer(filler, egui::PointerButton::Secondary),
    );
    time += FRAME_SECONDS;
    let (_, output) = tab_menu_frame(&context, &snapshot, time, Vec::new());
    let menu = texts_of(&output);
    for key in [
        "tab.newUntitledFile",
        "tab.newTerminal",
        "tab.closeSaved",
        "tab.closeAll",
        "tab.split",
        "tab.openWelcome",
    ] {
        assert!(menu.iter().any(|text| text == key), "{key}: {menu:?}");
    }
    assert!(!menu.iter().any(|text| text == "keymap.reopenClosedTab"));
    let intents = choose(&snapshot, "tab.openWelcome", &mut time);
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::OpenWelcome { project: owner, pane: target } if owner == &project && target == &pane
    )));
    time += 1.0;
    let _ = tab_menu_frame(
        &context,
        &snapshot,
        time,
        pointer(filler, egui::PointerButton::Primary),
    );
    time += FRAME_SECONDS;
    let (intents, _) = tab_menu_frame(
        &context,
        &snapshot,
        time,
        pointer(filler, egui::PointerButton::Primary),
    );
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::NewUntitled { pane: target, .. } if target == &pane
        )),
        "{intents:?}"
    );
}

#[test]
fn rail_추가메뉴는_경로_finder_최근프로젝트와_root누락_비활성을_보인다() {
    let snapshot = fixture();
    let recent = vec![
        WelcomeRecent {
            id: ProjectId::new(),
            name: "gamma".into(),
            root: "/work/gamma".into(),
            root_missing: false,
        },
        WelcomeRecent {
            id: ProjectId::new(),
            name: "delta".into(),
            root: "/work/delta".into(),
            root_missing: true,
        },
    ];
    let shell = shell(WindowScope::Main);
    let context = egui::Context::default();
    context.enable_accesskit();
    let run = |time: f64, events: Vec<egui::Event>| {
        let mut surfaces = Surfaces {
            rail: recent.clone(),
            ..Default::default()
        };
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                pos2(0.0, 0.0),
                vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]),
            )),
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut intents = Vec::new();
        let mut output = context.run_ui(input, |ui| {
            intents = shell.show(ui, &snapshot, &mut surfaces);
        });
        output.textures_delta.clear();
        (intents, output)
    };
    let (_, output) = run(0.0, Vec::new());
    let add = output
        .platform_output
        .accesskit_update
        .as_ref()
        .and_then(|update| {
            update.nodes.iter().find_map(|(_, node)| {
                (node.label() == Some("sidebar.addProjectMenu"))
                    .then(|| node.bounds())
                    .flatten()
            })
        })
        .map(|bounds| {
            Rect::from_min_max(
                pos2(bounds.x0 as f32, bounds.y0 as f32),
                pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .expect("add menu button is in the accessibility tree");
    let _ = run(
        FRAME_SECONDS,
        pointer(add.center(), egui::PointerButton::Primary),
    );
    let (_, output) = run(FRAME_SECONDS * 2.0, Vec::new());
    let texts = texts_of(&output);
    for key in [
        "sidebar.openByPath",
        "sidebar.openViaFinder",
        "app.recentItems",
    ] {
        assert!(texts.iter().any(|text| text == key), "{key}: {texts:?}");
    }
    assert!(texts.iter().any(|text| text == "gamma\n/work/gamma"));
    assert!(
        texts
            .iter()
            .any(|text| text == "delta\napp.recentProjectRootMissing /work/delta")
    );
    let delta = welcome_text_rect(&output, "delta").unwrap();
    let (intents, _) = run(
        FRAME_SECONDS * 3.0,
        pointer(delta.center(), egui::PointerButton::Primary),
    );
    assert!(
        !intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::SelectRecentProject { .. }))
    );
    let (_, output) = run(FRAME_SECONDS * 4.0, Vec::new());
    let by_path = welcome_text_rect(&output, "sidebar.openByPath").unwrap();
    let (intents, _) = run(
        FRAME_SECONDS * 5.0,
        pointer(by_path.center(), egui::PointerButton::Primary),
    );
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::OpenProjectByPath))
    );
}

fn rail_frame(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    mut surfaces: Surfaces,
    time: f64,
    events: Vec<egui::Event>,
) -> (Surfaces, Vec<ShellIntent>, egui::FullOutput) {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            pos2(0.0, 0.0),
            vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]),
        )),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut intents = Vec::new();
    let mut output = context.run_ui(input, |ui| {
        intents = shell(WindowScope::Main).show(ui, snapshot, &mut surfaces);
    });
    output.textures_delta.clear();
    (surfaces, intents, output)
}

fn access_node(
    output: &egui::FullOutput,
    label: &str,
) -> Option<(Rect, Option<egui::accesskit::Toggled>)> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.label() == Some(label)).then(|| {
                node.bounds().map(|bounds| {
                    (
                        Rect::from_min_max(
                            pos2(bounds.x0 as f32, bounds.y0 as f32),
                            pos2(bounds.x1 as f32, bounds.y1 as f32),
                        ),
                        node.toggled(),
                    )
                })
            })?
        })
}

fn text_shape<'a>(
    output: &'a egui::FullOutput,
    text: &str,
) -> Option<(&'a egui::epaint::TextShape, Rect)> {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                Some((shape, shape.visual_bounding_rect()))
            }
            _ => None,
        })
}

fn rail_agent(
    session: &str,
    name: &str,
    activity: AgentActivity,
    reason: Option<BlockedReason>,
) -> DetectedAgent {
    DetectedAgent {
        session_id: session.into(),
        name: name.into(),
        pid: 1,
        activity,
        blocked_reason: reason,
    }
}

#[test]
fn 프로젝트_rail은_라벨_아이콘_기본_glyph와_사용자색_root누락_흐림을_원본처럼_그린다() {
    let mut snapshot = fixture();
    let template = snapshot.projects[0].clone();
    snapshot.projects.push(ProjectRef {
        id: ProjectId::new(),
        name: "gamma".into(),
        ..template
    });
    snapshot.projects[0].display = ProjectDisplay {
        icon: Some("rocket".into()),
        label: Some(" 가나다라마 ".into()),
        color: Some("lane3".into()),
    };
    snapshot.projects[0].root_missing = true;
    snapshot.projects[1].display = ProjectDisplay {
        icon: Some(" rocket ".into()),
        label: Some("  ".into()),
        color: Some("lane99".into()),
    };
    let context = egui::Context::default();
    context.enable_accesskit();
    let (surfaces, _, output) =
        rail_frame(&context, &snapshot, Surfaces::default(), 0.0, Vec::new());
    let names: Vec<_> = snapshot
        .projects
        .iter()
        .map(|project| project.name.clone())
        .collect();
    let buttons: Vec<_> = names
        .iter()
        .map(|name| access_node(&output, name).expect("rail project button is accessible"))
        .collect();
    assert_eq!(
        buttons
            .iter()
            .map(|(_, toggled)| *toggled)
            .collect::<Vec<_>>(),
        [
            Some(egui::accesskit::Toggled::False),
            Some(egui::accesskit::Toggled::True),
            Some(egui::accesskit::Toggled::False)
        ]
    );
    for (rect, _) in &buttons {
        assert_eq!(rect.size(), egui::Vec2::splat(RAIL_BUTTON_SIDE));
    }
    let (label, label_rect) =
        text_shape(&output, "가나다라").expect("label glyph replaces the icon");
    let format = &label.galley.job.sections[0].format;
    assert_eq!(format.font_id.size, 9.0);
    assert!(format.extra_letter_spacing < 0.0);
    assert_eq!(
        label.fallback_color,
        Color32::from_rgb(LANE_RED, 2, 0).gamma_multiply(ROOT_MISSING_OPACITY)
    );
    assert!(
        buttons[0]
            .0
            .expand(PIXEL_SNAP_TOLERANCE)
            .contains_rect(label_rect)
    );
    assert!(text_shape(&output, &names[0][..1]).is_none());
    assert_eq!(surfaces.rail_icons.len(), 2);
    let (icon, rect, color, opacity) = &surfaces.rail_icons[0];
    assert_eq!(icon, &format!("{:?}", RailIcon::Project(Some("rocket"))));
    assert_eq!(*color, COLOR);
    assert_eq!(*opacity, 1.0);
    assert_eq!(rect.size(), egui::Vec2::splat(RAIL_GLYPH_ICON_SIDE));
    assert!((rect.center() - buttons[1].0.center()).length() < PIXEL_SNAP_TOLERANCE);
    let (icon, rect, _, _) = &surfaces.rail_icons[1];
    assert_eq!(icon, &format!("{:?}", RailIcon::Project(None)));
    assert!((rect.center() - buttons[2].0.center()).length() < PIXEL_SNAP_TOLERANCE);
    let rects: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(shape) => Some((shape.rect, shape.fill, clipped.clip_rect)),
            _ => None,
        })
        .collect();
    assert!(rects.iter().any(|(rect, fill, _)| {
        *fill == RAIL_ACTIVE
            && (rect.min - buttons[1].0.min).length() < PIXEL_SNAP_TOLERANCE
            && rect.size() == buttons[1].0.size()
    }));
    let (indicator, _, clip) = rects
        .iter()
        .find(|(_, fill, _)| *fill == RAIL_ACCENT)
        .expect("active project draws the accent indicator");
    assert_eq!(indicator.size(), vec2(2.0, 20.0));
    assert!(indicator.right() < buttons[1].0.left());
    assert!(clip.contains_rect(*indicator));
    let mut snapshot = snapshot;
    snapshot.projects[0].display.color = None;
    let (_, _, output) = rail_frame(
        &context,
        &snapshot,
        Surfaces::default(),
        FRAME_SECONDS,
        Vec::new(),
    );
    let (label, _) = text_shape(&output, "가나다라").unwrap();
    assert_eq!(
        label.fallback_color,
        COLOR.gamma_multiply(ROOT_MISSING_OPACITY)
    );
}

#[test]
fn 프로젝트_rail_에이전트_배지는_대표상태_모양과_설정_꺼짐_reduced_motion을_따른다() {
    let snapshot = fixture();
    let first = snapshot.projects[0].clone();
    let second = snapshot.projects[1].clone();
    let agents = std::collections::HashMap::from([
        (
            first.id.clone(),
            vec![
                rail_agent("idle", "codex", AgentActivity::Idle, None),
                rail_agent(
                    "blocked",
                    "claude",
                    AgentActivity::AwaitingInput,
                    Some(BlockedReason::Permission),
                ),
            ],
        ),
        (
            second.id.clone(),
            vec![rail_agent("busy", "claude", AgentActivity::Working, None)],
        ),
    ]);
    let surfaces = |is_badge_disabled: bool, is_reduced_motion: bool| Surfaces {
        agents: agents.clone(),
        is_badge_disabled,
        is_reduced_motion,
        ..Default::default()
    };
    let pulse_middle = 1.0;
    let context = egui::Context::default();
    context.enable_accesskit();
    let (_, _, output) = rail_frame(
        &context,
        &snapshot,
        surfaces(false, false),
        pulse_middle,
        Vec::new(),
    );
    let blocked_label = format!("{} — agent.blocked.permission", first.name);
    let working_label = format!("{} — agent.status.working", second.name);
    let (first_button, _) =
        access_node(&output, &blocked_label).expect("status joins the accessible name");
    let (second_button, _) = access_node(&output, &working_label).unwrap();
    let diamond = output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Path(path) if path.fill == AGENT_AWAITING => {
                Some((Rect::from_points(&path.points), clipped.clip_rect))
            }
            _ => None,
        })
        .expect("awaiting input draws the rotated square");
    let badge_center = first_button.right_bottom() - egui::Vec2::splat(6.0);
    assert!((diamond.0.center() - badge_center).length() < PIXEL_SNAP_TOLERANCE);
    assert!(diamond.0.right() > first_button.right());
    assert!(diamond.1.contains_rect(diamond.0));
    let circles = |output: &egui::FullOutput| -> Vec<egui::epaint::CircleShape> {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Circle(circle) => Some(*circle),
                _ => None,
            })
            .collect()
    };
    let working = circles(&output)
        .into_iter()
        .find(|circle| circle.fill != Color32::TRANSPARENT && circle.radius == 5.0)
        .expect("working draws a filled dot");
    assert_eq!(working.fill, AGENT_WORKING.gamma_multiply(0.5));
    assert!(
        (working.center - (second_button.right_bottom() - egui::Vec2::splat(5.0))).length()
            < PIXEL_SNAP_TOLERANCE
    );
    let (_, _, output) = rail_frame(
        &context,
        &snapshot,
        surfaces(false, true),
        pulse_middle + FRAME_SECONDS,
        Vec::new(),
    );
    assert!(
        circles(&output)
            .iter()
            .any(|circle| circle.fill == AGENT_WORKING && circle.radius == 5.0)
    );
    let (_, _, output) = rail_frame(
        &context,
        &snapshot,
        surfaces(true, false),
        pulse_middle + FRAME_SECONDS * 2.0,
        Vec::new(),
    );
    assert!(access_node(&output, &first.name).is_some());
    assert!(access_node(&output, &second.name).is_some());
    assert!(access_node(&output, &blocked_label).is_none());
    assert!(
        !circles(&output)
            .iter()
            .any(|circle| circle.fill == AGENT_WORKING.gamma_multiply(0.5)
                || circle.fill == AGENT_WORKING)
    );
    assert!(!output.shapes.iter().any(
        |clipped| matches!(&clipped.shape, egui::Shape::Path(path) if path.fill == AGENT_AWAITING)
    ));
    let quiet = std::collections::HashMap::from([
        (
            first.id.clone(),
            vec![rail_agent(
                "unknown",
                "gemini",
                AgentActivity::Unknown,
                None,
            )],
        ),
        (
            second.id.clone(),
            vec![rail_agent("idle", "codex", AgentActivity::Idle, None)],
        ),
    ]);
    let (_, _, output) = rail_frame(
        &context,
        &snapshot,
        Surfaces {
            agents: quiet,
            ..Default::default()
        },
        pulse_middle + FRAME_SECONDS * 3.0,
        Vec::new(),
    );
    assert!(access_node(&output, &format!("{} — agent.status.unknown", first.name)).is_some());
    assert!(access_node(&output, &format!("{} — agent.status.idle", second.name)).is_some());
    for color in [AGENT_UNKNOWN, AGENT_IDLE] {
        assert!(circles(&output).iter().any(|circle| {
            circle.stroke.color == color
                && circle.stroke.width == 1.0
                && circle.fill == Color32::TRANSPARENT
        }));
    }
}

#[test]
fn 프로젝트_rail_tooltip은_이름_root누락_경로와_모든_에이전트_세션을_오른쪽에_보인다() {
    let mut snapshot = fixture();
    snapshot.projects[0].root_missing = true;
    let project = snapshot.projects[0].clone();
    let agents = std::collections::HashMap::from([(
        project.id.clone(),
        vec![
            rail_agent("busy", "claude", AgentActivity::Working, None),
            rail_agent(
                "asking",
                "codex",
                AgentActivity::AwaitingInput,
                Some(BlockedReason::Question),
            ),
        ],
    )]);
    let surfaces = || Surfaces {
        agents: agents.clone(),
        is_badge_disabled: true,
        ..Default::default()
    };
    let context = egui::Context::default();
    context.enable_accesskit();
    let (_, _, output) = rail_frame(&context, &snapshot, surfaces(), 0.0, Vec::new());
    let (button, _) = access_node(&output, &project.name).unwrap();
    let _ = rail_frame(
        &context,
        &snapshot,
        surfaces(),
        TOOLTIP_HOVER_SECONDS,
        vec![egui::Event::PointerMoved(button.center())],
    );
    let _ = rail_frame(
        &context,
        &snapshot,
        surfaces(),
        TOOLTIP_HOVER_SECONDS + TOOLTIP_SETTLE_SECONDS,
        Vec::new(),
    );
    let (surfaces, intents, output) = rail_frame(
        &context,
        &snapshot,
        surfaces(),
        TOOLTIP_HOVER_SECONDS + TOOLTIP_SETTLE_SECONDS + TOOLTIP_FADE_SECONDS,
        Vec::new(),
    );
    assert!(intents.is_empty());
    let texts = texts_of(&output);
    let expected = [
        "app.recentProjectRootMissing".to_owned(),
        "/synthetic".to_owned(),
        format!(
            "agent.sessionTooltip: {:?}",
            [("name", "claude"), ("status", "agent.status.working")]
        ),
        format!(
            "agent.sessionTooltip: {:?}",
            [("name", "codex"), ("status", "agent.blocked.question")]
        ),
    ];
    for text in &expected {
        assert!(texts.contains(text), "{text}: {texts:?}");
    }
    let (warning, warning_rect) = text_shape(&output, "app.recentProjectRootMissing").unwrap();
    assert!(warning_rect.left() >= button.right());
    let (root, _) = text_shape(&output, "/synthetic").unwrap();
    assert_eq!(
        warning.galley.job.sections[0].format.color,
        Color32::PLACEHOLDER
    );
    assert_eq!(
        root.galley.job.sections[0].format.color,
        context
            .global_style()
            .visuals
            .text_color()
            .gamma_multiply(TOOLTIP_SECONDARY_OPACITY)
    );
    let (icon, rect, _, opacity) = surfaces
        .rail_icons
        .iter()
        .find(|(icon, ..)| icon == &format!("{:?}", RailIcon::RootMissing))
        .expect("root missing tooltip line carries the warning glyph");
    assert_eq!(icon, "RootMissing");
    assert_eq!(rect.size(), egui::Vec2::splat(12.0));
    assert!(rect.right() < warning_rect.left());
    assert_eq!(*opacity, 1.0);
    let (_, rect, _, opacity) = surfaces
        .rail_icons
        .iter()
        .find(|(icon, ..)| icon == &format!("{:?}", RailIcon::Project(None)))
        .unwrap();
    assert!(button.contains_rect(*rect));
    assert_eq!(*opacity, ROOT_MISSING_OPACITY);
}

fn menu_step(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    events: Vec<egui::Event>,
) -> (Vec<ShellIntent>, egui::FullOutput) {
    *time += FRAME_SECONDS;
    let (_, intents, output) = rail_frame(context, snapshot, Surfaces::default(), *time, events);
    (intents, output)
}

/// The rightmost text with exactly `text`: a menu opens over or beside the rail, so a group name
/// shared with a rail header resolves to the menu entry.
fn menu_text_rect(output: &egui::FullOutput, text: &str) -> Option<Rect> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => {
                Some(shape.visual_bounding_rect())
            }
            _ => None,
        })
        .max_by(|left, right| left.left().total_cmp(&right.left()))
}

fn menu_node_disabled(output: &egui::FullOutput, label: &str) -> Option<bool> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(label))
        .filter_map(|(_, node)| node.bounds().map(|bounds| (bounds.x0, node.is_disabled())))
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, is_disabled)| is_disabled)
}

fn open_project_menu(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    name: &str,
) -> egui::FullOutput {
    *time += 1.0;
    let escape = [true, false]
        .map(|pressed| egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .to_vec();
    let _ = menu_step(context, snapshot, time, escape);
    let (_, output) = menu_step(context, snapshot, time, Vec::new());
    let (button, _) = access_node(&output, name).unwrap_or_else(|| panic!("{name}"));
    let _ = menu_step(
        context,
        snapshot,
        time,
        pointer(button.center(), egui::PointerButton::Secondary),
    );
    menu_step(context, snapshot, time, Vec::new()).1
}

fn choose_project_menu(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    name: &str,
    label: &str,
) -> Vec<ShellIntent> {
    let output = open_project_menu(context, snapshot, time, name);
    let item = menu_text_rect(&output, label).unwrap_or_else(|| panic!("{label}"));
    menu_step(
        context,
        snapshot,
        time,
        pointer(item.center(), egui::PointerButton::Primary),
    )
    .0
}

fn hover_group_submenu(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    name: &str,
) -> egui::FullOutput {
    let output = open_project_menu(context, snapshot, time, name);
    let trigger = menu_text_rect(&output, "projectGroup.addTo").expect("group submenu trigger");
    let _ = menu_step(
        context,
        snapshot,
        time,
        vec![egui::Event::PointerMoved(trigger.center())],
    );
    for _ in 0..WHEEL_SETTLE_FRAMES {
        let _ = menu_step(context, snapshot, time, Vec::new());
    }
    menu_step(context, snapshot, time, Vec::new()).1
}

#[test]
fn 프로젝트_rail_메뉴는_원본_순서의_닫기_슬롯열기_그룹_경로_표시_항목과_비활성_조건을_따른다() {
    let mut snapshot = fixture();
    let template = snapshot.projects[0].clone();
    snapshot.projects.push(ProjectRef {
        id: ProjectId::new(),
        ..template
    });
    for (project, (name, root)) in snapshot.projects.iter_mut().zip([
        ("alpha", "/work/alpha"),
        ("beta", "/work/beta"),
        ("gamma", "/work/gamma"),
    ]) {
        project.name = name.into();
        project.root = root.into();
    }
    snapshot.projects[0].display.label = Some("AL".into());
    let ids: Vec<_> = snapshot
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    let group = |name: &str, members: Vec<ProjectId>| ProjectGroup {
        id: ProjectGroupId::new(),
        name: name.into(),
        color: None,
        members,
        collapsed: false,
    };
    snapshot.groups = vec![
        group("first-group", vec![ids[1].clone()]),
        group("second-group", vec![ids[0].clone(), ids[1].clone()]),
    ];
    let groups: Vec<_> = snapshot
        .groups
        .iter()
        .map(|group| group.id.clone())
        .collect();
    let Some(ShellSlotTree::Split { children, .. }) = &snapshot.shell.tree else {
        panic!("fixture splits two slots")
    };
    let Some(ShellSlotTree::Leaf { slot_id: first, .. }) = children.first() else {
        panic!("fixture leaf")
    };
    let first = first.clone();
    let focused = snapshot.shell.focused.clone().unwrap();
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;

    let output = open_project_menu(&context, &snapshot, &mut time, "alpha");
    let order = [
        "project.close",
        "shellSlot.openToTheRight",
        "shellSlot.openBelow",
        "shellSlot.openToTheLeft",
        "shellSlot.openAbove",
        "projectGroup.addTo",
        "projectGroup.removeFrom",
        "project.openInFileManager",
        "project.copyPath",
        "project.displayMenu",
        "project.displayReset",
    ];
    let tops: Vec<f32> = order
        .iter()
        .map(|key| {
            menu_text_rect(&output, key)
                .unwrap_or_else(|| panic!("{key}: {:?}", texts_of(&output)))
                .top()
        })
        .collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    for key in order.into_iter().filter(|key| *key != "projectGroup.addTo") {
        assert_eq!(menu_node_disabled(&output, key), Some(false), "{key}");
    }

    let intents = choose_project_menu(&context, &snapshot, &mut time, "alpha", "project.close");
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::CloseProject(id) if id == &ids[0])),
        "{intents:?}"
    );
    for (key, expected) in [
        ("shellSlot.openToTheRight", ShellSlotEdge::Right),
        ("shellSlot.openBelow", ShellSlotEdge::Bottom),
        ("shellSlot.openToTheLeft", ShellSlotEdge::Left),
        ("shellSlot.openAbove", ShellSlotEdge::Top),
    ] {
        let intents = choose_project_menu(&context, &snapshot, &mut time, "alpha", key);
        assert!(
            intents.iter().any(|intent| matches!(
                intent,
                ShellIntent::OpenProjectInSlot { project, slot, edge }
                    if project == &ids[0] && slot == &focused && *edge == expected
            )),
            "{key}: {intents:?}"
        );
    }
    let intents = choose_project_menu(
        &context,
        &snapshot,
        &mut time,
        "alpha",
        "projectGroup.removeFrom",
    );
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::SetProjectGroupMembers { group, members }
            if group == &groups[1] && members == &vec![ids[1].clone()]
    )));
    for (key, path_intent) in [
        ("project.openInFileManager", true),
        ("project.copyPath", false),
    ] {
        let intents = choose_project_menu(&context, &snapshot, &mut time, "alpha", key);
        assert!(
            intents.iter().any(|intent| match intent {
                ShellIntent::OpenPath(path) => path_intent && path == "/work/alpha",
                ShellIntent::CopyText(text) => !path_intent && text == "/work/alpha",
                _ => false,
            }),
            "{key}: {intents:?}"
        );
    }
    let intents = choose_project_menu(
        &context,
        &snapshot,
        &mut time,
        "alpha",
        "project.displayMenu",
    );
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::EditProjectDisplay(id) if id == &ids[0]))
    );
    let intents = choose_project_menu(
        &context,
        &snapshot,
        &mut time,
        "alpha",
        "project.displayReset",
    );
    assert!(intents.iter().any(|intent| matches!(
        intent,
        ShellIntent::SetProjectDisplay { project, patch }
            if project == &ids[0]
                && [&patch.icon, &patch.label, &patch.color]
                    .iter()
                    .all(|axis| axis.as_deref() == Some(""))
    )));

    let output = hover_group_submenu(&context, &snapshot, &mut time, "alpha");
    let submenu = ["first-group", "second-group", "projectGroup.newGroup"]
        .map(|label| menu_text_rect(&output, label).unwrap_or_else(|| panic!("{label}")));
    assert!(submenu.windows(2).all(|pair| pair[0].top() < pair[1].top()));
    assert_eq!(menu_node_disabled(&output, "first-group"), Some(false));
    assert_eq!(
        menu_node_disabled(&output, "second-group"),
        Some(true),
        "the current group is not offered again"
    );
    let (intents, _) = menu_step(
        &context,
        &snapshot,
        &mut time,
        pointer(submenu[0].center(), egui::PointerButton::Primary),
    );
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::SetProjectGroupMembers { group, members }
                if group == &groups[0] && members == &vec![ids[1].clone(), ids[0].clone()]
        )),
        "{intents:?}"
    );
    let output = hover_group_submenu(&context, &snapshot, &mut time, "alpha");
    let new_group = menu_text_rect(&output, "projectGroup.newGroup").unwrap();
    let (intents, _) = menu_step(
        &context,
        &snapshot,
        &mut time,
        pointer(new_group.center(), egui::PointerButton::Primary),
    );
    assert!(
        intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::NewProjectGroup(id) if id == &ids[0]))
    );

    let output = open_project_menu(&context, &snapshot, &mut time, "gamma");
    assert_eq!(
        menu_node_disabled(&output, "projectGroup.removeFrom"),
        Some(true)
    );
    assert_eq!(
        menu_node_disabled(&output, "project.displayReset"),
        Some(true)
    );
    let intents = choose_project_menu(
        &context,
        &snapshot,
        &mut time,
        "gamma",
        "projectGroup.removeFrom",
    );
    assert!(
        !intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::SetProjectGroupMembers { .. }))
    );

    snapshot.shell.focused = Some(ShellSlotId::new());
    let intents = choose_project_menu(
        &context,
        &snapshot,
        &mut time,
        "gamma",
        "shellSlot.openToTheRight",
    );
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::OpenProjectInSlot { project, slot, .. }
                if project == &ids[2] && slot == &first
        )),
        "a stale focus falls back to the first slot: {intents:?}"
    );
    snapshot.shell.tree = None;
    snapshot.shell.focused = None;
    let output = open_project_menu(&context, &snapshot, &mut time, "gamma");
    for key in &order[1..5] {
        assert_eq!(menu_node_disabled(&output, key), Some(true), "{key}");
    }
    let intents = choose_project_menu(
        &context,
        &snapshot,
        &mut time,
        "gamma",
        "shellSlot.openToTheRight",
    );
    assert!(
        !intents
            .iter()
            .any(|intent| matches!(intent, ShellIntent::OpenProjectInSlot { .. }))
    );
}
