use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, project_ref, step,
};
use super::*;
use taide_model::layout::PaneNode;
use taide_model::project::ShellSlotTree;
use taide_native_ui::shell::{TAB_HEIGHT, TITLE_HEIGHT};

const DEADLINE: Duration = Duration::from_secs(10);
const DRAG_ACTIVATION_PROBE: f32 = 6.0;
const PANE_EDGE_INSET: f32 = 10.0;

fn tab_bar() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(f32::INFINITY, TITLE_HEIGHT + TAB_HEIGHT),
    )
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
        assert!(Instant::now() < deadline, "phase={phase}");
        std::thread::yield_now();
    }
}

/// Presses the tab named `title`, carries it past the activation distance to `target` and
/// releases there.
fn drag_tab(
    application: &mut NativeApplication,
    context: &egui::Context,
    title: &str,
    target: impl Fn(&egui::FullOutput) -> egui::Pos2,
) {
    let output = step(application, context, Vec::new());
    let source = node_rect(&output, title, tab_bar()).unwrap_or_else(|| panic!("{title}"));
    let target = target(&output);
    step(application, context, primary_button(source.center(), true));
    for point in [
        source.center() + egui::vec2(0.0, DRAG_ACTIVATION_PROBE),
        target,
    ] {
        step(application, context, vec![egui::Event::PointerMoved(point)]);
    }
    assert!(taide_native_ui::tab_drag::is_dragging(context));
    step(application, context, primary_button(target, false));
}

fn leaf_titles(node: &PaneNode) -> Vec<Vec<String>> {
    match node {
        PaneNode::Leaf { tabs, .. } => vec![tabs.iter().map(|tab| tab.title.clone()).collect()],
        PaneNode::Split { children, .. } => children.iter().flat_map(leaf_titles).collect(),
    }
}

#[test]
fn 실제_app_탭_drag는_재정렬과_가장자리_분할을_runtime_layout에_반영한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-native-tab-drag-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let titles = ["first.txt", "second.txt", "third.txt"];
    let tabs: Vec<Tab> = titles
        .iter()
        .map(|title| {
            let path = root.join(title);
            std::fs::write(&path, "synthetic\n").unwrap();
            Tab {
                id: TabId::new(),
                kind: TabKind::File {
                    path: path.to_str().unwrap().into(),
                },
                title: (*title).into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }
        })
        .collect();
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        let PaneNode::Leaf {
            tabs: leaf, active, ..
        } = &mut layout.root
        else {
            panic!("expected leaf")
        };
        *active = Some(tabs[0].id.clone());
        *leaf = tabs;
        layout.shell_view.sidebar_collapsed = true;
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&project, &root, "alpha")];
        session.active_project = Some(project.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: project.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let layout_titles = || leaf_titles(&services.state.layouts.read()[&project].root);
    step_until(application, &context, "tabs", |_, output| {
        titles
            .iter()
            .all(|title| node_rect(output, title, tab_bar()).is_some())
    });

    drag_tab(application, &context, "third.txt", |output| {
        node_rect(output, "first.txt", tab_bar()).unwrap().center()
    });
    step_until(application, &context, "reorder", |_, output| {
        let left = |title| node_rect(output, title, tab_bar()).map(|rect| rect.left());
        layout_titles() == vec![vec!["third.txt", "first.txt", "second.txt"]]
            && left("third.txt") < left("first.txt")
    });

    drag_tab(application, &context, "second.txt", |_| {
        let screen = context.content_rect();
        egui::pos2(screen.right() - PANE_EDGE_INSET, screen.center().y)
    });
    step_until(application, &context, "split", |_, _| {
        layout_titles() == vec![vec!["third.txt", "first.txt"], vec!["second.txt"]]
    });
    assert!(!taide_native_ui::tab_drag::is_dragging(&context));
}
