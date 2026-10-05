use std::sync::Mutex;

use egui::{Color32, Rect, Ui, pos2, vec2};
use taide_model::app_event::AppEvent;
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{AuxWindowLayout, PaneNode, SplitDir, Tab};
use taide_model::paths::AppPaths;
use taide_model::project::{
    ProjectDisplay, ProjectRef, SessionShellState, ShellSlotTree, WindowChrome,
};
use taide_native_ui::commands::{ShellIntent, ShellMutation, dispatch, request_close_tab};
use taide_native_ui::shell::{NativeShell, ShellColors, ShellSurfaces, WindowScope};
use taide_native_ui::snapshot::ShellSnapshot;
use taide_native_ui::split::{MIN_PANE_SIZE, child_rects, normalized_sizes, resized_pair};
use taide_runtime::{AppState, EventSink};

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
        },
    }
}

fn render(shell: &NativeShell, snapshot: &ShellSnapshot) -> Surfaces {
    let mut surfaces = Surfaces::default();
    let context = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            pos2(0.0, 0.0),
            vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]),
        )),
        ..Default::default()
    };
    let mut output = context.run_ui(input, |ui| {
        shell.show(ui, snapshot, &mut surfaces);
    });
    output.textures_delta.clear();
    surfaces
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
            .map(|project| (project, taide_layout::service::default_layout()))
            .collect(),
        hide_status_in_zen: true,
        resizer_thickness: SPLIT_THICKNESS,
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
    let auxiliary = taide_layout::service::default_layout();
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
