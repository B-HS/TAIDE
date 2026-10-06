use std::sync::Mutex;

use egui::epaint::Shadow;
use egui::{Color32, CornerRadius, Rect, Stroke, Ui, pos2, vec2};
use taide_model::app_event::AppEvent;
use taide_model::ids::{PaneId, ProjectGroupId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{AuxWindowLayout, PaneNode, ProjectLayout, SplitDir, Tab, TabKind};
use taide_model::paths::AppPaths;
use taide_model::project::{
    ProjectDisplay, ProjectGroup, ProjectRef, SessionShellState, ShellSlotTree, WindowChrome,
};
use taide_native_ui::commands::{ShellIntent, ShellMutation, dispatch, request_close_tab};
use taide_native_ui::presentation::{apply_visuals, visuals};
use taide_native_ui::shell::{NativeShell, ShellColors, ShellSurfaces, TITLE_HEIGHT, WindowScope};
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
}

impl ShellSurfaces for Surfaces {
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
            active_tab: COLOR,
            inactive_tab: COLOR,
            active_indicator: COLOR,
            editor_background: COLOR,
            editor_foreground: COLOR,
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
