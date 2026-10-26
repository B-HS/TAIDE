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
use taide_native_ui::project_drag::{DRAGGING_OPACITY, is_dragging, slot_drop_preview};
use taide_native_ui::project_rail::{RailColors, RailIcon};
use taide_native_ui::shell::{
    NativeShell, PROJECT_RAIL_WIDTH, SLOT_HEADER_HEIGHT, STATUS_HEIGHT, ShellColors, ShellSurfaces,
    TITLE_HEIGHT, WelcomeContent, WelcomeRecent, WelcomeShortcutRow, WindowScope,
};
use taide_native_ui::snapshot::{ShellSnapshot, project_group_sections};
use taide_native_ui::split::{MIN_PANE_SIZE, child_rects, normalized_sizes, resized_pair};
use taide_native_ui::tab_drag;
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
const DROP_TARGET: Color32 = Color32::from_rgb(11, 12, 250);
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
const GROUP_HEADER_SIZE: [f32; 2] = [44.0, 17.5];
const GROUP_NAME_MAX_WIDTH: f32 = 22.0;
const GROUP_NAME_FONT_SIZE: f32 = 9.0;
const GROUP_CHEVRON_POINTS: usize = 3;
const BELOW_ACTIVATION_DISTANCE: f32 = 3.0;
const ABOVE_ACTIVATION_DISTANCE: f32 = 5.0;
const DRAG_PREVIEW_OPACITY: f32 = 0.9;
const DROP_PREVIEW_FILL_OPACITY: f32 = 0.25;
const SLOT_EDGE_INSET: f32 = 10.0;
const SLOT_HEADER_PROBE: f32 = 12.0;
const EGUI_ITEM_SPACING: f32 = 8.0;
const AUTO_SCROLL_EDGE_PROBE: f32 = 24.0;
const AUTO_SCROLL_HOLD_FRAMES: usize = 6;
const TAB_ROW_GAP_PROBE: f32 = TAB_ICON_SIDE + TAB_ICON_GAP + EGUI_ITEM_SPACING / 2.0;

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
    agent_tab_icons: Vec<(TabId, Option<Color32>)>,
    reopen_with_extension: Option<&'static str>,
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
    fn tab_icon(
        &mut self,
        _: &Ui,
        rect: Rect,
        tab: &Tab,
        agent_color: Option<Color32>,
        title_color: Color32,
    ) {
        self.tab_icons.push((tab.id.clone(), rect, title_color));
        self.agent_tab_icons.push((tab.id.clone(), agent_color));
    }
    fn can_reopen_with(&self, path: &str) -> bool {
        self.reopen_with_extension
            .is_some_and(|extension| path.ends_with(extension))
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
            drop_target: DROP_TARGET,
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

fn scoped_frame(
    context: &egui::Context,
    scope: WindowScope,
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
    let shell = shell(scope);
    let mut intents = Vec::new();
    let mut output = context.run_ui(input, |ui| {
        intents = shell.show(ui, snapshot, &mut surfaces);
    });
    output.textures_delta.clear();
    (surfaces, intents, output)
}

struct MenuDriver {
    context: egui::Context,
    time: f64,
    surfaces: fn() -> Surfaces,
}

impl MenuDriver {
    fn step(
        &mut self,
        scope: &WindowScope,
        snapshot: &ShellSnapshot,
        events: Vec<egui::Event>,
    ) -> (Surfaces, Vec<ShellIntent>, egui::FullOutput) {
        self.time += 1.0;
        scoped_frame(
            &self.context,
            scope.clone(),
            snapshot,
            (self.surfaces)(),
            self.time,
            events,
        )
    }

    fn open_menu(
        &mut self,
        scope: &WindowScope,
        snapshot: &ShellSnapshot,
        title: &str,
    ) -> egui::FullOutput {
        let (_, _, output) = self.step(scope, snapshot, Vec::new());
        let (_, tab) = text_shape(&output, title).unwrap_or_else(|| panic!("{title}"));
        let _ = self.step(
            scope,
            snapshot,
            pointer(tab.center(), egui::PointerButton::Secondary),
        );
        self.step(scope, snapshot, Vec::new()).2
    }

    fn dismiss(
        &mut self,
        scope: &WindowScope,
        snapshot: &ShellSnapshot,
        output: &egui::FullOutput,
    ) {
        let (_, outside) = text_shape(output, "test-only status").unwrap();
        let _ = self.step(
            scope,
            snapshot,
            pointer(outside.center(), egui::PointerButton::Primary),
        );
    }

    fn choose(
        &mut self,
        scope: &WindowScope,
        snapshot: &ShellSnapshot,
        output: &egui::FullOutput,
        key: &str,
    ) -> Vec<ShellIntent> {
        let item = welcome_text_rect(output, key).unwrap_or_else(|| panic!("{key}"));
        self.step(
            scope,
            snapshot,
            pointer(item.center(), egui::PointerButton::Primary),
        )
        .1
    }
}

fn single_slot_snapshot(tabs: Vec<Tab>) -> (ShellSnapshot, ProjectId, ShellSlotId) {
    let mut snapshot = fixture();
    let project = snapshot.projects[0].id.clone();
    let slot = ShellSlotId::new();
    snapshot.shell.tree = Some(ShellSlotTree::Leaf {
        slot_id: slot.clone(),
        project_id: project.clone(),
    });
    snapshot.shell.focused = Some(slot.clone());
    snapshot.layouts.retain(|id, _| id == &project);
    snapshot.projects.truncate(1);
    snapshot.projects[0].root = "/synthetic/root".into();
    let layout = snapshot.layouts.get_mut(&project).unwrap();
    let PaneNode::Leaf {
        tabs: leaf, active, ..
    } = &mut layout.root
    else {
        panic!("expected leaf")
    };
    *active = tabs.first().map(|tab| tab.id.clone());
    *leaf = tabs;
    (snapshot, project, slot)
}

fn synthetic_tab(title: &str, kind: TabKind) -> Tab {
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

fn terminal_kind(session: &str) -> TabKind {
    TabKind::Terminal {
        session_id: session.into(),
        cwd: None,
    }
}

#[test]
fn 탭_아이콘은_에이전트가_감지된_터미널만_활동색과_세션_tooltip을_보인다() {
    let activities = [
        ("awaiting", AgentActivity::AwaitingInput, AGENT_AWAITING),
        ("working", AgentActivity::Working, AGENT_WORKING),
        ("idle", AgentActivity::Idle, AGENT_IDLE),
        ("unknown", AgentActivity::Unknown, AGENT_UNKNOWN),
    ];
    let mut tabs = activities
        .iter()
        .map(|(session, ..)| synthetic_tab(session, terminal_kind(session)))
        .collect::<Vec<_>>();
    tabs.extend([
        synthetic_tab("plain", terminal_kind("synthetic-plain")),
        synthetic_tab("fresh", terminal_kind("")),
        synthetic_tab(
            "main.rs",
            TabKind::File {
                path: "/synthetic/root/main.rs".into(),
            },
        ),
    ]);
    let (snapshot, project, _) = single_slot_snapshot(tabs.clone());
    let mut agents = activities
        .iter()
        .map(|(session, activity, _)| {
            let reason =
                (*activity == AgentActivity::AwaitingInput).then_some(BlockedReason::Question);
            rail_agent(session, "claude", *activity, reason)
        })
        .collect::<Vec<_>>();
    agents.push(rail_agent("", "ghost", AgentActivity::Working, None));
    agents.push(rail_agent("main.rs", "file", AgentActivity::Working, None));
    let surfaces = || Surfaces {
        agents: std::collections::HashMap::from([(project.clone(), agents.clone())]),
        ..Default::default()
    };
    let context = egui::Context::default();
    let (recorded, _, _) = scoped_frame(
        &context,
        WindowScope::Main,
        &snapshot,
        surfaces(),
        0.0,
        Vec::new(),
    );
    let agent_color = |tab: &Tab| {
        recorded
            .agent_tab_icons
            .iter()
            .find(|(id, _)| id == &tab.id)
            .unwrap_or_else(|| panic!("{}", tab.title))
            .1
    };
    for (tab, (_, _, color)) in tabs.iter().zip(&activities) {
        assert_eq!(agent_color(tab), Some(*color), "{}", tab.title);
    }
    for tab in &tabs[activities.len()..] {
        assert_eq!(agent_color(tab), None, "{}", tab.title);
    }
    let icon = |tab: &Tab| {
        recorded
            .tab_icons
            .iter()
            .find(|(id, ..)| id == &tab.id)
            .unwrap()
            .1
    };
    let tooltip = |icon: Rect, start: f64| {
        let _ = scoped_frame(
            &context,
            WindowScope::Main,
            &snapshot,
            surfaces(),
            start + TOOLTIP_HOVER_SECONDS,
            vec![egui::Event::PointerMoved(icon.center())],
        );
        let _ = scoped_frame(
            &context,
            WindowScope::Main,
            &snapshot,
            surfaces(),
            start + TOOLTIP_HOVER_SECONDS + TOOLTIP_SETTLE_SECONDS,
            Vec::new(),
        );
        scoped_frame(
            &context,
            WindowScope::Main,
            &snapshot,
            surfaces(),
            start + TOOLTIP_HOVER_SECONDS + TOOLTIP_SETTLE_SECONDS + TOOLTIP_FADE_SECONDS,
            Vec::new(),
        )
        .2
    };
    let output = tooltip(icon(&tabs[0]), 0.0);
    let expected = format!(
        "agent.sessionTooltip: {:?}",
        [("name", "claude"), ("status", "agent.blocked.question")]
    );
    let (_, text) = text_shape(&output, &expected).unwrap_or_else(|| panic!("{expected}"));
    assert!(text.top() >= icon(&tabs[0]).bottom(), "{text:?}");
    let plain = tooltip(icon(&tabs[activities.len()]), TOOLTIP_HOVER_SECONDS * 2.0);
    assert!(
        !texts_of(&plain)
            .iter()
            .any(|text| text.starts_with("agent.sessionTooltip")),
        "{:?}",
        texts_of(&plain)
    );
}

#[test]
fn 탭_메뉴는_주창_파일탭에_탐색기_reveal_rename과_미리보기파일_다시열기를_요청한다() {
    let image = synthetic_tab(
        "pic.png",
        TabKind::File {
            path: "/synthetic/root/assets/pic.png".into(),
        },
    );
    let source = synthetic_tab(
        "lib.rs",
        TabKind::File {
            path: "/synthetic/root/src/lib.rs".into(),
        },
    );
    let terminal = synthetic_tab("term-tab", terminal_kind("synthetic-shell"));
    let (mut snapshot, project, slot) = single_slot_snapshot(vec![image, source, terminal]);
    let mut driver = MenuDriver {
        context: egui::Context::default(),
        time: 0.0,
        surfaces: || Surfaces {
            reopen_with_extension: Some(".png"),
            ..Default::default()
        },
    };
    let main = WindowScope::Main;

    let menu = driver.open_menu(&main, &snapshot, "pic.png");
    let texts = texts_of(&menu);
    let position = |key: &str| {
        texts
            .iter()
            .position(|text| text == key)
            .unwrap_or_else(|| panic!("{key}: {texts:?}"))
    };
    assert!(position("explorer.revealInFinder") < position("tab.revealInExplorerView"));
    assert!(position("tab.revealInExplorerView") < position("tab.rename"));
    assert!(position("tab.rename") < position("tab.reopenEditorWith"));
    assert!(position("tab.reopenEditorWith") < position("tab.split"));
    for hidden in ["tab.openChanges", "git.fileHistory", "tab.moveToNewWindow"] {
        assert!(!texts.iter().any(|text| text == hidden), "{hidden}");
    }
    let intents = driver.choose(&main, &snapshot, &menu, "tab.revealInExplorerView");
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::RevealInExplorer { project: owner, slot: target, path, rename: false }
                if owner == &project && target == &slot && path == "/synthetic/root/assets/pic.png"
        )),
        "{intents:?}"
    );
    let menu = driver.open_menu(&main, &snapshot, "pic.png");
    let intents = driver.choose(&main, &snapshot, &menu, "tab.rename");
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::RevealInExplorer { project: owner, slot: target, path, rename: true }
                if owner == &project && target == &slot && path == "/synthetic/root/assets/pic.png"
        )),
        "{intents:?}"
    );
    for (key, expected_preview) in [
        ("explorer.openWithEditor", false),
        ("explorer.openWithPreview", true),
    ] {
        let menu = driver.open_menu(&main, &snapshot, "pic.png");
        let submenu = welcome_text_rect(&menu, "tab.reopenEditorWith").unwrap();
        let _ = driver.step(
            &main,
            &snapshot,
            vec![egui::Event::PointerMoved(submenu.center())],
        );
        let mut output = None;
        for _ in 0..WHEEL_SETTLE_FRAMES {
            output = Some(driver.step(&main, &snapshot, Vec::new()).2);
        }
        let intents = driver.choose(&main, &snapshot, &output.unwrap(), key);
        assert!(
            intents.iter().any(|intent| matches!(
                intent,
                ShellIntent::ReopenWith { path, preview }
                    if path == "/synthetic/root/assets/pic.png" && *preview == expected_preview
            )),
            "{key}: {intents:?}"
        );
    }

    let menu = driver.open_menu(&main, &snapshot, "lib.rs");
    let texts = texts_of(&menu);
    assert!(texts.iter().any(|text| text == "tab.rename"), "{texts:?}");
    assert!(
        !texts.iter().any(|text| text == "tab.reopenEditorWith"),
        "{texts:?}"
    );
    driver.dismiss(&main, &snapshot, &menu);
    let menu = driver.open_menu(&main, &snapshot, "term-tab");
    let texts = texts_of(&menu);
    assert!(texts.iter().any(|text| text == "tab.split"), "{texts:?}");
    for hidden in [
        "tab.revealInExplorerView",
        "tab.rename",
        "tab.reopenEditorWith",
    ] {
        assert!(!texts.iter().any(|text| text == hidden), "{hidden}");
    }
    driver.dismiss(&main, &snapshot, &menu);

    let layout = snapshot.layouts.get_mut(&project).unwrap();
    let auxiliary = layout.root.clone();
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: AUX_SLOT,
        root: auxiliary,
        focused_pane: layout.focused_pane.clone(),
    });
    let auxiliary = WindowScope::Auxiliary {
        project: project.clone(),
        slot: AUX_SLOT,
    };
    let menu = driver.open_menu(&auxiliary, &snapshot, "pic.png");
    let texts = texts_of(&menu);
    assert!(
        texts.iter().any(|text| text == "tab.reopenEditorWith"),
        "{texts:?}"
    );
    for hidden in ["tab.revealInExplorerView", "tab.rename"] {
        assert!(!texts.iter().any(|text| text == hidden), "{hidden}");
    }
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

/// The open-polyline chevron stroked inside `header`, with its stroke color.
fn group_chevron(output: &egui::FullOutput, header: Rect) -> (Vec<egui::Pos2>, Color32) {
    output
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Path(path)
                if path.points.len() == GROUP_CHEVRON_POINTS
                    && !path.closed
                    && path.points.iter().all(|point| header.contains(*point)) =>
            {
                match path.stroke.color {
                    egui::epaint::ColorMode::Solid(color) => Some((path.points.clone(), color)),
                    egui::epaint::ColorMode::UV(_) => None,
                }
            }
            _ => None,
        })
        .expect("group header chevron")
}

#[test]
fn 프로젝트_그룹_헤더는_그룹색_tint와_접힘_chevron과_44px_말줄임과_그룹메뉴를_원본처럼_보인다() {
    let mut snapshot = fixture();
    let ids: Vec<_> = snapshot
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    let tinted_name = "tinted group with a long name";
    snapshot.groups = vec![
        ProjectGroup {
            id: ProjectGroupId::new(),
            name: tinted_name.into(),
            color: Some("lane3".into()),
            members: vec![ids[0].clone()],
            collapsed: false,
        },
        ProjectGroup {
            id: ProjectGroupId::new(),
            name: "plain".into(),
            color: Some("lane99".into()),
            members: vec![ids[1].clone()],
            collapsed: true,
        },
    ];
    let groups: Vec<_> = snapshot
        .groups
        .iter()
        .map(|group| group.id.clone())
        .collect();
    let lane_three = Color32::from_rgb(LANE_RED, 2, 0);
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());

    let (tinted, _) = access_node(&output, tinted_name).expect("tinted header is accessible");
    let (plain, _) = access_node(&output, "plain").expect("plain header is accessible");
    for header in [tinted, plain] {
        assert_eq!(
            header.size(),
            vec2(GROUP_HEADER_SIZE[0], GROUP_HEADER_SIZE[1])
        );
        assert!((header.center().x - PROJECT_RAIL_WIDTH / 2.0).abs() < PIXEL_SNAP_TOLERANCE);
    }
    assert!(tinted.bottom() <= plain.top());
    let (member, _) = access_node(&output, &snapshot.projects[0].name).unwrap();
    assert!(tinted.bottom() <= member.top() && member.bottom() <= plain.top());
    assert!(
        access_node(&output, &snapshot.projects[1].name).is_none(),
        "a collapsed group hides its members"
    );

    let (label, label_rect) = text_shape(&output, tinted_name).unwrap();
    let format = &label.galley.job.sections[0].format;
    assert_eq!(format.font_id.size, GROUP_NAME_FONT_SIZE);
    assert_eq!(format.color, lane_three);
    assert_eq!(label.fallback_color, lane_three);
    assert!(
        label_rect.width() <= GROUP_NAME_MAX_WIDTH + PIXEL_SNAP_TOLERANCE,
        "the name truncates beside the chevron: {label_rect:?}"
    );
    assert!(
        tinted
            .expand(PIXEL_SNAP_TOLERANCE)
            .contains_rect(label_rect)
    );
    let (label, _) = text_shape(&output, "plain").unwrap();
    assert_eq!(
        label.galley.job.sections[0].format.color, COLOR,
        "an unknown color token falls back to the default icon color"
    );
    let (points, color) = group_chevron(&output, tinted);
    assert_eq!(color, lane_three);
    assert!(
        points[1].y > points[0].y && points[2].x > points[1].x && points[2].y < points[1].y,
        "open: chevron down"
    );
    let (points, color) = group_chevron(&output, plain);
    assert_eq!(color, COLOR);
    assert!(
        points[1].x > points[0].x && points[2].x < points[1].x && points[2].y > points[1].y,
        "folded: chevron right"
    );
    let hover_fills = |output: &egui::FullOutput| {
        output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.fill == RAIL_HOVER
                        && (shape.rect.min - tinted.min).length() < PIXEL_SNAP_TOLERANCE
                        && (shape.rect.max - tinted.max).length() < PIXEL_SNAP_TOLERANCE =>
                {
                    Some(clipped.clip_rect)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(hover_fills(&output).is_empty());
    let _ = menu_step(
        &context,
        &snapshot,
        &mut time,
        vec![egui::Event::PointerMoved(tinted.center())],
    );
    let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());
    let fills = hover_fills(&output);
    assert_eq!(fills.len(), 1);
    assert!(
        fills[0].contains_rect(tinted),
        "the 44px hover fill is not clipped to the narrower scroll content: {:?} vs {tinted:?}",
        fills[0]
    );

    for (name, group, collapsed) in [
        (tinted_name, &groups[0], true),
        ("plain", &groups[1], false),
    ] {
        time += 1.0;
        let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());
        let (header, _) = access_node(&output, name).unwrap();
        let (intents, _) = menu_step(
            &context,
            &snapshot,
            &mut time,
            pointer(header.center(), egui::PointerButton::Primary),
        );
        assert!(
            intents.iter().any(|intent| matches!(
                intent,
                ShellIntent::Mutate(ShellMutation::SetGroupCollapsed { group: id, collapsed: value })
                    if id == group && *value == collapsed
            )),
            "{name}: {intents:?}"
        );
    }

    let output = open_project_menu(&context, &snapshot, &mut time, tinted_name);
    let order = [
        "projectGroup.open",
        "projectGroup.edit",
        "projectGroup.delete",
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
    for key in order {
        assert_eq!(menu_node_disabled(&output, key), Some(false), "{key}");
        let intents = choose_project_menu(&context, &snapshot, &mut time, tinted_name, key);
        assert!(
            intents.iter().any(|intent| match intent {
                ShellIntent::OpenProjectGroup(id) => key == order[0] && id == &groups[0],
                ShellIntent::EditProjectGroup(id) => key == order[1] && id == &groups[0],
                ShellIntent::DeleteProjectGroup(id) => key == order[2] && id == &groups[0],
                _ => false,
            }),
            "{key}: {intents:?}"
        );
        assert!(
            !intents.iter().any(|intent| matches!(
                intent,
                ShellIntent::Mutate(ShellMutation::SetGroupCollapsed { .. })
            )),
            "{key}: a menu choice does not fold the group: {intents:?}"
        );
    }
}

fn primary_button(position: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

/// The rail button or header named `label`; a slot header also carries a project's name.
fn rail_node(output: &egui::FullOutput, label: &str) -> Rect {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("accesskit is enabled")
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(label))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            Rect::from_min_max(
                pos2(bounds.x0 as f32, bounds.y0 as f32),
                pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .find(|rect| rect.right() <= PROJECT_RAIL_WIDTH)
        .unwrap_or_else(|| panic!("{label}"))
}

fn rail_step(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    events: Vec<egui::Event>,
    intents: &mut Vec<ShellIntent>,
) -> (Surfaces, egui::FullOutput) {
    *time += FRAME_SECONDS;
    let (surfaces, frame_intents, output) =
        rail_frame(context, snapshot, Surfaces::default(), *time, events);
    intents.extend(frame_intents);
    (surfaces, output)
}

/// Presses at `from`, moves through `path` a frame at a time, sends `before_release`, then
/// releases where the path ends. Returns every frame's intents, the paint of the last move and
/// whether the drag was active then.
fn drag_pointer(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    (from, path): (egui::Pos2, &[egui::Pos2]),
    (before_release, is_dragging): (Vec<egui::Event>, fn(&egui::Context) -> bool),
) -> (Vec<ShellIntent>, Surfaces, egui::FullOutput, bool) {
    let mut intents = Vec::new();
    rail_step(
        context,
        snapshot,
        time,
        primary_button(from, true),
        &mut intents,
    );
    let held = path
        .iter()
        .map(|point| {
            rail_step(
                context,
                snapshot,
                time,
                vec![egui::Event::PointerMoved(*point)],
                &mut intents,
            )
        })
        .last()
        .expect("a drag moves the pointer");
    let was_dragging = is_dragging(context);
    if !before_release.is_empty() {
        rail_step(context, snapshot, time, before_release, &mut intents);
    }
    let release = path.last().copied().unwrap_or(from);
    rail_step(
        context,
        snapshot,
        time,
        primary_button(release, false),
        &mut intents,
    );
    rail_step(context, snapshot, time, Vec::new(), &mut intents);
    assert!(!is_dragging(context), "a release always ends the drag");
    (intents, held.0, held.1, was_dragging)
}

fn drag_rail(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    gesture: (egui::Pos2, &[egui::Pos2]),
    before_release: Vec<egui::Event>,
) -> (Vec<ShellIntent>, Surfaces, egui::FullOutput, bool) {
    drag_pointer(
        context,
        snapshot,
        time,
        gesture,
        (before_release, is_dragging),
    )
}

fn drag_tab(
    context: &egui::Context,
    snapshot: &ShellSnapshot,
    time: &mut f64,
    gesture: (egui::Pos2, &[egui::Pos2]),
    before_release: Vec<egui::Event>,
) -> (Vec<ShellIntent>, Surfaces, egui::FullOutput, bool) {
    drag_pointer(
        context,
        snapshot,
        time,
        gesture,
        (before_release, tab_drag::is_dragging),
    )
}

fn is_activation(intent: &ShellIntent) -> bool {
    matches!(
        intent,
        ShellIntent::Mutate(ShellMutation::ActivateProject(_))
    )
}

fn is_reorder(intent: &ShellIntent) -> bool {
    matches!(
        intent,
        ShellIntent::ReorderProjects(_) | ShellIntent::ReorderProjectGroups(_)
    )
}

#[test]
fn 프로젝트_rail_drag는_4px을_넘어야_시작해_유령과_미리보기와_밀림을_보이고_전역순서를_재정렬한다()
{
    let mut snapshot = fixture();
    let template = snapshot.projects[0].clone();
    snapshot.projects.push(ProjectRef {
        id: ProjectId::new(),
        name: "third".into(),
        ..template
    });
    let ids: Vec<_> = snapshot
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());
    let icons: Vec<Rect> = snapshot
        .projects
        .iter()
        .map(|project| rail_node(&output, &project.name))
        .collect();

    let (intents, _, _, was_dragging) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (
            icons[0].center(),
            &[icons[0].center() + vec2(BELOW_ACTIVATION_DISTANCE, 0.0)],
        ),
        Vec::new(),
    );
    assert!(!was_dragging, "within 4px the press is still pending");
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::Mutate(ShellMutation::ActivateProject(id)) if id == &ids[0]
        )),
        "a press that never travels past 4px stays a click: {intents:?}"
    );
    assert!(!intents.iter().any(is_reorder), "{intents:?}");

    let (intents, _, _, was_dragging) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (
            icons[0].center(),
            &[icons[0].center() + vec2(0.0, ABOVE_ACTIVATION_DISTANCE)],
        ),
        Vec::new(),
    );
    assert!(was_dragging, "past 4px the press becomes a drag");
    assert!(
        !intents
            .iter()
            .any(|intent| is_activation(intent) || is_reorder(intent)),
        "a drag released on itself neither activates nor reorders: {intents:?}"
    );

    let (intents, surfaces, held, _) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (
            icons[0].center(),
            &[
                icons[0].center() + vec2(0.0, ABOVE_ACTIVATION_DISTANCE),
                icons[1].center(),
                icons[2].center(),
            ],
        ),
        Vec::new(),
    );
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::ReorderProjects(order)
                if order == &vec![ids[1].clone(), ids[2].clone(), ids[0].clone()]
        )),
        "{intents:?}"
    );
    assert!(!intents.iter().any(is_activation), "{intents:?}");
    let glyphs: Vec<(egui::Pos2, f32)> = surfaces
        .rail_icons
        .iter()
        .map(|(_, rect, _, opacity)| (rect.center(), *opacity))
        .collect();
    let painted = |center: egui::Pos2, opacity: f32| {
        glyphs.iter().any(|(painted, painted_opacity)| {
            (*painted - center).length() < PIXEL_SNAP_TOLERANCE
                && (painted_opacity - opacity).abs() < f32::EPSILON
        })
    };
    assert_eq!(glyphs.len(), icons.len() + 1, "{glyphs:?}");
    assert!(
        painted(icons[2].center(), DRAG_PREVIEW_OPACITY),
        "the preview follows the pointer: {glyphs:?}"
    );
    assert!(
        painted(icons[2].center(), DRAGGING_OPACITY),
        "the faded source takes the place it would land in: {glyphs:?}"
    );
    assert!(
        painted(icons[0].center(), 1.0) && painted(icons[1].center(), 1.0),
        "the icons it passed move up one place: {glyphs:?}"
    );
    assert!(
        held.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape)
                if shape.fill == RAIL_ACTIVE.gamma_multiply(DRAG_PREVIEW_OPACITY)
                    && (shape.rect.center() - icons[2].center()).length() < PIXEL_SNAP_TOLERANCE
                    && shape.rect.size() == icons[2].size())),
        "the preview is a 40px item-active tile"
    );
}

#[test]
fn 프로젝트를_슬롯_가장자리와_가운데에_떨구면_분할_교체하고_자기교체_헤더_esc는_무동작이다() {
    let snapshot = fixture();
    let ids: Vec<_> = snapshot
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    let Some(ShellSlotTree::Split { children, .. }) = &snapshot.shell.tree else {
        panic!("fixture splits two slots")
    };
    let slots: Vec<ShellSlotId> = children
        .iter()
        .filter_map(|child| match child {
            ShellSlotTree::Leaf { slot_id, .. } => Some(slot_id.clone()),
            ShellSlotTree::Split { .. } => None,
        })
        .collect();
    let body = Rect::from_min_max(
        pos2(PROJECT_RAIL_WIDTH, TITLE_HEIGHT),
        pos2(WINDOW_SIZE[0], WINDOW_SIZE[1] - STATUS_HEIGHT),
    );
    let regions = child_rects(body, SplitDir::Horizontal, &[50.0, 50.0], SPLIT_THICKNESS);
    let slot_body = |index: usize| {
        let mut rect = regions[index];
        rect.min.y += SLOT_HEADER_HEIGHT;
        rect
    };
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());
    let icons: Vec<Rect> = snapshot
        .projects
        .iter()
        .map(|project| rail_node(&output, &project.name))
        .collect();
    let start = |index: usize| icons[index].center() + vec2(ABOVE_ACTIVATION_DISTANCE, 0.0);
    let opened = |intents: &[ShellIntent]| {
        intents
            .iter()
            .filter_map(|intent| match intent {
                ShellIntent::OpenProjectInSlot {
                    project,
                    slot,
                    edge,
                } => Some((project.clone(), slot.clone(), *edge)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    let right_zone = pos2(
        slot_body(1).right() - SLOT_EDGE_INSET,
        slot_body(1).center().y,
    );
    let (intents, _, held, _) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (icons[0].center(), &[start(0), right_zone]),
        Vec::new(),
    );
    assert_eq!(
        opened(&intents),
        vec![(ids[0].clone(), slots[1].clone(), ShellSlotEdge::Right)],
        "{intents:?}"
    );
    assert!(
        !intents
            .iter()
            .any(|intent| is_activation(intent) || is_reorder(intent))
    );
    let preview = slot_drop_preview(slot_body(1), DropEdge::Right);
    assert!(
        held.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape)
                if shape.fill == DROP_TARGET.gamma_multiply(DROP_PREVIEW_FILL_OPACITY)
                    && shape.stroke == Stroke::new(CSS_BORDER_WIDTH, DROP_TARGET)
                    && (shape.rect.min - preview.min).length() < PIXEL_SNAP_TOLERANCE
                    && (shape.rect.max - preview.max).length() < PIXEL_SNAP_TOLERANCE)),
        "the right half of the hovered slot is previewed: {preview:?}"
    );

    let (intents, ..) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (icons[0].center(), &[start(0), slot_body(1).center()]),
        Vec::new(),
    );
    assert_eq!(
        opened(&intents),
        vec![(ids[0].clone(), slots[1].clone(), ShellSlotEdge::Replace)],
        "the centre replaces the slot's project"
    );

    let (intents, ..) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (icons[1].center(), &[start(1), slot_body(1).center()]),
        Vec::new(),
    );
    assert!(
        opened(&intents).is_empty(),
        "a project dropped on the centre of its own slot asks for nothing: {intents:?}"
    );

    let header = pos2(slot_body(1).center().x, TITLE_HEIGHT + SLOT_HEADER_PROBE);
    let (intents, _, held, _) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (icons[0].center(), &[start(0), header]),
        Vec::new(),
    );
    assert!(
        opened(&intents).is_empty() && !intents.iter().any(is_reorder),
        "a release outside the rail that misses every zone does nothing: {intents:?}"
    );
    assert!(
        !held.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape) if shape.stroke.color == DROP_TARGET)),
        "no zone is previewed over a slot header"
    );

    let escape = [true, false]
        .map(|pressed| egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .to_vec();
    let (intents, ..) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (icons[0].center(), &[start(0), right_zone]),
        escape,
    );
    assert!(
        opened(&intents).is_empty() && !intents.iter().any(is_activation),
        "Escape cancels the drag: {intents:?}"
    );
}

#[test]
fn 그룹_헤더_drag는_헤더끼리만_재정렬하고_자리를_바꿔_보이며_아이콘_위_드롭은_무동작이다() {
    let mut snapshot = fixture();
    let ids: Vec<_> = snapshot
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    snapshot.groups = vec![
        ProjectGroup {
            id: ProjectGroupId::new(),
            name: "first".into(),
            color: Some("lane3".into()),
            members: vec![ids[0].clone()],
            collapsed: false,
        },
        ProjectGroup {
            id: ProjectGroupId::new(),
            name: "second".into(),
            color: None,
            members: vec![ids[1].clone()],
            collapsed: false,
        },
    ];
    let groups: Vec<_> = snapshot
        .groups
        .iter()
        .map(|group| group.id.clone())
        .collect();
    let lane_three = Color32::from_rgb(LANE_RED, 2, 0);
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());
    let first = rail_node(&output, "first");
    let second = rail_node(&output, "second");
    let member = rail_node(&output, &snapshot.projects[1].name);

    let (intents, _, held, _) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (
            second.center(),
            &[
                second.center() - vec2(0.0, ABOVE_ACTIVATION_DISTANCE),
                first.center(),
            ],
        ),
        Vec::new(),
    );
    assert!(
        intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::ReorderProjectGroups(order)
                if order == &vec![groups[1].clone(), groups[0].clone()]
        )),
        "{intents:?}"
    );
    assert!(
        !intents.iter().any(|intent| matches!(
            intent,
            ShellIntent::ReorderProjects(_)
                | ShellIntent::Mutate(ShellMutation::SetGroupCollapsed { .. })
        )),
        "a header drag neither folds nor touches the project order: {intents:?}"
    );
    assert_eq!(
        group_chevron(&held, first).1,
        COLOR,
        "the held header takes the first header's place"
    );
    assert_eq!(
        group_chevron(&held, second).1,
        lane_three,
        "and the first header moves down to the second's"
    );
    assert!(
        !held.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape) if shape.fill == RAIL_ACTIVE.gamma_multiply(DRAG_PREVIEW_OPACITY))),
        "only a project carries a preview"
    );

    let (intents, ..) = drag_rail(
        &context,
        &snapshot,
        &mut time,
        (
            first.center(),
            &[
                first.center() + vec2(0.0, ABOVE_ACTIVATION_DISTANCE),
                member.center(),
            ],
        ),
        Vec::new(),
    );
    assert!(
        !intents.iter().any(|intent| is_reorder(intent)
            || matches!(
                intent,
                ShellIntent::Mutate(ShellMutation::SetGroupCollapsed { .. })
            )),
        "a header dropped on an icon writes neither order: {intents:?}"
    );
}

/// The tab named `title`: an active tab's content repeats its title below the tab bar.
fn tab_node(output: &egui::FullOutput, title: &str) -> Rect {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("accesskit is enabled")
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(title))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            Rect::from_min_max(
                pos2(bounds.x0 as f32, bounds.y0 as f32),
                pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .min_by(|upper, lower| upper.top().total_cmp(&lower.top()))
        .unwrap_or_else(|| panic!("{title}"))
}

/// Every text painted as `text`, with its position and the opacity its opaque color was faded to.
fn painted_texts(output: &egui::FullOutput, text: &str) -> Vec<(egui::Pos2, f32)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => Some((
                shape.pos,
                f32::from(shape.fallback_color.a()) / f32::from(u8::MAX),
            )),
            _ => None,
        })
        .collect()
}

fn is_faded_to(opacity: f32, expected: f32) -> bool {
    (opacity - expected).abs() <= 1.0 / f32::from(u8::MAX)
}

fn tab_drops(intents: &[ShellIntent]) -> Vec<&ShellMutation> {
    intents
        .iter()
        .filter_map(|intent| match intent {
            ShellIntent::Mutate(
                mutation @ (ShellMutation::MoveTab { .. } | ShellMutation::SplitTab { .. }),
            ) => Some(mutation),
            _ => None,
        })
        .collect()
}

fn is_moved(intents: &[ShellIntent], tab: &TabId, pane: &PaneId, index: u32) -> bool {
    matches!(
        tab_drops(intents).as_slice(),
        [ShellMutation::MoveTab { tab: moved, pane: target, index: at }]
            if moved == tab && target == pane && *at == index
    )
}

fn is_split(intents: &[ShellIntent], tab: &TabId, pane: &PaneId, edge: DropEdge) -> bool {
    matches!(
        tab_drops(intents).as_slice(),
        [ShellMutation::SplitTab { tab: moved, pane: target, edge: at }]
            if moved == tab && target == pane && *at == edge
    )
}

fn is_tab_activated(intents: &[ShellIntent], tab: &TabId) -> bool {
    intents.iter().any(
        |intent| matches!(intent, ShellIntent::Mutate(ShellMutation::ActivateTab(id)) if id == tab),
    )
}

fn leaf_of(tabs: Vec<Tab>) -> PaneNode {
    PaneNode::Leaf {
        id: PaneId::new(),
        active: tabs.first().map(|tab| tab.id.clone()),
        tabs,
    }
}

fn leaf_id(node: &PaneNode) -> PaneId {
    match node {
        PaneNode::Leaf { id, .. } | PaneNode::Split { id, .. } => id.clone(),
    }
}

#[test]
fn 탭_drag는_4px을_넘어야_시작해_유령과_미리보기와_밀림을_보이고_고정경계를_지켜_재정렬한다() {
    let titles = ["pinned.rs", "a.rs", "b.rs", "c.rs"];
    let mut tabs: Vec<Tab> = titles
        .iter()
        .map(|title| synthetic_tab(title, TabKind::Settings))
        .collect();
    tabs[0].pinned = true;
    let ids: Vec<TabId> = tabs.iter().map(|tab| tab.id.clone()).collect();
    let (snapshot, project, _) = single_slot_snapshot(tabs);
    let pane = leaf_id(&snapshot.layouts[&project].root);
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());
    let nodes: Vec<Rect> = titles
        .iter()
        .map(|title| tab_node(&output, title))
        .collect();
    let resting = |title: &str| painted_texts(&output, title)[0].0;
    let (b_text, c_text) = (resting("b.rs"), resting("c.rs"));
    let step = nodes[3].left() - nodes[2].left();
    let lift = |index: usize| nodes[index].center() + vec2(0.0, ABOVE_ACTIVATION_DISTANCE);

    let (intents, _, _, was_dragging) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (
            nodes[2].center(),
            &[nodes[2].center() + vec2(BELOW_ACTIVATION_DISTANCE, 0.0)],
        ),
        Vec::new(),
    );
    assert!(!was_dragging, "within 4px the press is still pending");
    assert!(is_tab_activated(&intents, &ids[2]), "{intents:?}");
    assert!(tab_drops(&intents).is_empty(), "{intents:?}");

    let (intents, _, _, was_dragging) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (nodes[2].center(), &[lift(2)]),
        Vec::new(),
    );
    assert!(was_dragging, "past 4px the press becomes a drag");
    assert!(
        !is_tab_activated(&intents, &ids[2]),
        "a drag never activates on release: {intents:?}"
    );
    assert!(
        is_moved(&intents, &ids[2], &pane, 2),
        "released on itself it keeps its index, as the original moveTab does: {intents:?}"
    );

    let pointer = nodes[3].center();
    let (intents, _, held, _) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (nodes[2].center(), &[lift(2), pointer]),
        Vec::new(),
    );
    assert!(is_moved(&intents, &ids[2], &pane, 3), "{intents:?}");
    assert!(!is_tab_activated(&intents, &ids[2]), "{intents:?}");
    let dragged = painted_texts(&held, "b.rs");
    let shown = |position: egui::Pos2, opacity: f32| {
        dragged.iter().any(|(painted, painted_opacity)| {
            (painted.x - position.x).abs() <= PIXEL_SNAP_TOLERANCE * 2.0
                && (painted.y - position.y).abs() <= PIXEL_SNAP_TOLERANCE * 2.0
                && is_faded_to(*painted_opacity, opacity)
        })
    };
    assert_eq!(dragged.len(), 2, "{dragged:?}");
    assert!(
        shown(b_text + vec2(step, 0.0), DRAGGING_OPACITY),
        "the faded source takes the place it would land in: {dragged:?}"
    );
    assert!(
        shown(b_text + (pointer - nodes[2].center()), DRAG_PREVIEW_OPACITY),
        "the preview follows the pointer: {dragged:?}"
    );
    assert!(
        painted_texts(&held, "c.rs")
            .iter()
            .any(|(painted, opacity)| (painted.x - (c_text.x - step)).abs()
                <= PIXEL_SNAP_TOLERANCE * 2.0
                && is_faded_to(*opacity, 1.0)),
        "the tab it passed moves back one place"
    );

    let (intents, ..) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (nodes[1].center(), &[lift(1), nodes[0].center()]),
        Vec::new(),
    );
    assert!(
        is_moved(&intents, &ids[1], &pane, 1),
        "an unpinned tab stops right after the pinned run: {intents:?}"
    );

    let (intents, _, held, _) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (nodes[0].center(), &[lift(0), nodes[3].center()]),
        Vec::new(),
    );
    assert!(
        is_moved(&intents, &ids[0], &pane, 1),
        "a pinned tab never leaves the pinned run: {intents:?}"
    );
    assert!(
        painted_texts(&held, "c.rs")
            .iter()
            .all(|(painted, _)| (painted.x - c_text.x).abs() <= PIXEL_SNAP_TOLERANCE),
        "the two runs are separate sortable lists, so nothing shifts"
    );

    let gap = pos2(
        nodes[2].left() + step - TAB_ROW_GAP_PROBE,
        nodes[2].center().y,
    );
    let (intents, ..) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (nodes[1].center(), &[lift(1), gap]),
        Vec::new(),
    );
    assert!(
        is_moved(&intents, &ids[1], &pane, 2),
        "the gap after a tab still drops on that tab, as the flush original tabs do: {intents:?}"
    );

    let filler = pos2(nodes[3].left() + step, nodes[3].center().y);
    let (intents, ..) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (nodes[2].center(), &[lift(2), filler]),
        Vec::new(),
    );
    assert!(
        is_moved(&intents, &ids[2], &pane, 4),
        "the tab bar's empty space appends: {intents:?}"
    );
}

#[test]
fn 탭을_pane_가장자리와_가운데와_다른_pane_탭에_떨구면_분할과_이동을_요청하고_esc는_취소한다() {
    let left = ["left.rs", "move.rs"].map(|title| synthetic_tab(title, TabKind::Settings));
    let right = ["right.rs", "other.rs"].map(|title| synthetic_tab(title, TabKind::Settings));
    let moving = left[1].id.clone();
    let (mut snapshot, project, _) = single_slot_snapshot(Vec::new());
    let layout = snapshot.layouts.get_mut(&project).unwrap();
    let children = vec![leaf_of(left.to_vec()), leaf_of(right.to_vec())];
    let panes: Vec<PaneId> = children.iter().map(leaf_id).collect();
    layout.root = PaneNode::Split {
        id: PaneId::new(),
        dir: SplitDir::Horizontal,
        children,
        sizes: vec![50.0, 50.0],
    };
    layout.focused_pane = panes[0].clone();
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let mut intents = Vec::new();
    let (surfaces, output) = rail_step(&context, &snapshot, &mut time, Vec::new(), &mut intents);
    let body = |pane: &PaneId| {
        surfaces
            .panes
            .iter()
            .find(|(_, owner, ..)| owner == pane)
            .map(|(.., rect)| *rect)
            .expect("both panes show their active tab")
    };
    let source = tab_node(&output, "move.rs");
    let resting_icon = surfaces
        .tab_icons
        .iter()
        .find(|(id, ..)| id == &moving)
        .map(|(_, rect, _)| *rect)
        .unwrap();
    let lifted = source.center() + vec2(0.0, ABOVE_ACTIVATION_DISTANCE);
    let drop_at = |target: egui::Pos2, time: &mut f64| {
        drag_tab(
            &context,
            &snapshot,
            time,
            (source.center(), &[lifted, target]),
            Vec::new(),
        )
    };

    let right_zone = pos2(
        body(&panes[1]).right() - SLOT_EDGE_INSET,
        body(&panes[1]).center().y,
    );
    let (intents, held_surfaces, held, was_dragging) = drop_at(right_zone, &mut time);
    assert!(was_dragging);
    assert!(
        is_split(&intents, &moving, &panes[1], DropEdge::Right),
        "{intents:?}"
    );
    let preview = slot_drop_preview(body(&panes[1]), DropEdge::Right);
    assert!(
        held.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape)
                if shape.fill == DROP_TARGET.gamma_multiply(DROP_PREVIEW_FILL_OPACITY)
                    && shape.stroke == Stroke::new(CSS_BORDER_WIDTH, DROP_TARGET)
                    && (shape.rect.min - preview.min).length() < PIXEL_SNAP_TOLERANCE
                    && (shape.rect.max - preview.max).length() < PIXEL_SNAP_TOLERANCE)),
        "the right half of the hovered pane is previewed: {preview:?}"
    );
    let icons: Vec<Rect> = held_surfaces
        .tab_icons
        .iter()
        .filter(|(id, ..)| id == &moving)
        .map(|(_, rect, _)| *rect)
        .collect();
    let carried = resting_icon.translate(right_zone - source.center());
    assert_eq!(icons.len(), 2, "the source and its DragOverlay copy");
    assert!(
        (icons[1].min - carried.min).length() <= PIXEL_SNAP_TOLERANCE * 2.0,
        "the overlay tab follows the pointer: {icons:?} {carried:?}"
    );
    assert!(
        painted_texts(&held, "move.rs")
            .iter()
            .any(|(_, opacity)| is_faded_to(*opacity, DRAG_PREVIEW_OPACITY)),
        "the overlay is drawn at 90%"
    );

    let (intents, ..) = drop_at(body(&panes[1]).center(), &mut time);
    assert!(
        is_moved(&intents, &moving, &panes[1], 2),
        "the centre appends to that pane: {intents:?}"
    );

    let (intents, ..) = drop_at(tab_node(&output, "other.rs").center(), &mut time);
    assert!(
        is_moved(&intents, &moving, &panes[1], 1),
        "another pane's tab takes its index: {intents:?}"
    );

    let top_zone = pos2(
        body(&panes[0]).center().x,
        body(&panes[0]).top() + SLOT_EDGE_INSET,
    );
    let (intents, ..) = drop_at(top_zone, &mut time);
    assert!(
        is_split(&intents, &moving, &panes[0], DropEdge::Top),
        "its own pane splits too: {intents:?}"
    );

    let add_menu = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some("tab.newTabMenu"))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| pos2(bounds.x0 as f32, bounds.y0 as f32) + vec2(1.0, 1.0))
        .find(|corner| corner.x < body(&panes[0]).right())
        .unwrap();
    let (intents, _, held, _) = drop_at(add_menu, &mut time);
    assert!(
        tab_drops(&intents).is_empty(),
        "the + button takes no drop: {intents:?}"
    );
    assert!(
        !held.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape) if shape.stroke.color == DROP_TARGET)),
        "nothing is previewed outside the zones"
    );

    let escape = [true, false]
        .map(|pressed| egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .to_vec();
    let (intents, ..) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (source.center(), &[lifted, right_zone]),
        escape,
    );
    assert!(
        tab_drops(&intents).is_empty() && !is_tab_activated(&intents, &moving),
        "Escape cancels the drag: {intents:?}"
    );
}

#[test]
fn 탭은_자기_슬롯의_editor_area_안에서만_떨어진다() {
    let mut snapshot = fixture();
    let projects: Vec<ProjectId> = snapshot
        .projects
        .iter()
        .map(|project| project.id.clone())
        .collect();
    let away = synthetic_tab("away.rs", TabKind::Settings);
    let layout = snapshot.layouts.get_mut(&projects[0]).unwrap();
    layout.root = leaf_of(vec![
        synthetic_tab("keep.rs", TabKind::Settings),
        away.clone(),
    ]);
    layout.focused_pane = leaf_id(&layout.root);
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let mut intents = Vec::new();
    let (surfaces, output) = rail_step(&context, &snapshot, &mut time, Vec::new(), &mut intents);
    let other_slot = surfaces
        .panes
        .iter()
        .find(|(owner, ..)| owner == &projects[1])
        .map(|(.., rect)| *rect)
        .unwrap();
    let source = tab_node(&output, "away.rs");
    let (intents, _, held, was_dragging) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (
            source.center(),
            &[
                source.center() + vec2(0.0, ABOVE_ACTIVATION_DISTANCE),
                other_slot.center(),
            ],
        ),
        Vec::new(),
    );
    assert!(was_dragging);
    assert!(
        tab_drops(&intents).is_empty(),
        "another slot's panes belong to another DndContext: {intents:?}"
    );
    assert!(
        !held.shapes.iter().any(|clipped| matches!(&clipped.shape,
            egui::Shape::Rect(shape) if shape.stroke.color == DROP_TARGET)),
        "and show no zone"
    );
}

#[test]
fn 넘친_탭바는_탭을_오른쪽_가장자리로_끌면_자동으로_스크롤하고_pane_영역에서는_멈춘다() {
    let tabs: Vec<Tab> = (0..OVERFLOWING_TAB_COUNT)
        .map(|index| synthetic_tab(&format!("synthetic-tab-{index}"), TabKind::Settings))
        .collect();
    let (snapshot, ..) = single_slot_snapshot(tabs);
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut time = 0.0;
    let (_, output) = menu_step(&context, &snapshot, &mut time, Vec::new());
    let first = |output: &egui::FullOutput| tab_node(output, "synthetic-tab-0").left();
    let resting = first(&output);
    let source = tab_node(&output, "synthetic-tab-1");
    let lifted = source.center() + vec2(0.0, ABOVE_ACTIVATION_DISTANCE);
    let add_menu = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("tab.newTabMenu"))
        .and_then(|(_, node)| node.bounds())
        .unwrap();
    let near_edge = pos2(
        add_menu.x0 as f32 - AUTO_SCROLL_EDGE_PROBE,
        source.center().y,
    );
    let below_edge = near_edge + vec2(0.0, taide_native_ui::shell::TAB_HEIGHT * 2.0);
    let escape = [true, false]
        .map(|pressed| egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .to_vec();
    let held_at = |point: egui::Pos2| {
        std::iter::once(lifted)
            .chain(std::iter::repeat_n(point, AUTO_SCROLL_HOLD_FRAMES))
            .collect::<Vec<_>>()
    };

    let path = held_at(below_edge);
    let (_, _, held, _) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (source.center(), &path),
        escape.clone(),
    );
    assert_eq!(
        first(&held),
        resting,
        "a pane zone has no scrollable tab bar above it"
    );

    let path = held_at(near_edge);
    let (intents, _, held, _) = drag_tab(
        &context,
        &snapshot,
        &mut time,
        (source.center(), &path),
        escape,
    );
    assert!(
        first(&held) < resting,
        "the hovered tab bar scrolls toward the edge it is held at"
    );
    assert!(tab_drops(&intents).is_empty(), "{intents:?}");
}
