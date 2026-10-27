use super::inlay_hints_tests::key;
use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, project_ref, step,
};
use super::*;
use taide_model::layout::{PaneNode, SplitDir};
use taide_model::project::ShellSlotTree;

const DEADLINE: Duration = Duration::from_secs(10);
const SETTLE_FRAMES: usize = 4;
const HALF: f32 = 50.0;
const PACKAGE_MANIFEST: &str = r#"{"scripts":{"build":"synthetic-build","lint":"synthetic-lint"}}"#;
const MAKEFILE: &str = "check:\n\ttrue\n";
const LINT_PAYLOAD: &str = "npm run 'lint'\n";

fn command_shift() -> egui::Modifiers {
    let command = if cfg!(target_os = "macos") {
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
    } else {
        egui::Modifiers::CTRL | egui::Modifiers::COMMAND
    };
    command | egui::Modifiers::SHIFT
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
        assert!(Instant::now() < deadline, "phase={phase}");
        std::thread::yield_now();
    }
}

fn sidebar_view(
    application: &mut NativeApplication,
    project: &ProjectId,
    slot: &ShellSlotId,
) -> crate::symbol_sidebar::View {
    application.symbol_sidebars.entry(project, slot).view
}

#[test]
fn 실제_app_탐색기_키는_포커스_슬롯만_접힘을_풀고_파일_보기로_바꾸며_git_키는_통과시킨다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-explorer-key-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let roots = [
        fixture.directory.join("alpha"),
        fixture.directory.join("beta"),
    ];
    for directory in [&data, &roots[0], &roots[1]] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let projects = [ProjectId::new(), ProjectId::new()];
    let slots = [ShellSlotId::new(), ShellSlotId::new()];
    let names = ["alpha", "beta"];
    let state = isolated_state(data);
    for ((project, root), name) in projects.iter().zip(&roots).zip(names) {
        insert_project(&state, project, root, name, false);
        state
            .layouts
            .write()
            .get_mut(project)
            .unwrap()
            .shell_view
            .sidebar_collapsed = true;
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
        application.symbol_sidebars.entry(project, slot).view =
            crate::symbol_sidebar::View::Outline;
    }
    let collapsed = |index: usize| {
        services.state.layouts.read()[&projects[index]]
            .shell_view
            .sidebar_collapsed
    };

    step(
        application,
        &context,
        key(egui::Key::G, egui::Modifiers::CTRL | egui::Modifiers::SHIFT),
    );
    settle(application, &context);
    assert!(collapsed(0) && collapsed(1));
    for (project, slot) in projects.iter().zip(&slots) {
        assert_eq!(
            sidebar_view(application, project, slot),
            crate::symbol_sidebar::View::Outline
        );
    }

    step(application, &context, key(egui::Key::E, command_shift()));
    step_until(application, &context, "focused slot explorer", |_| {
        !collapsed(1)
    });
    assert_eq!(
        sidebar_view(application, &projects[1], &slots[1]),
        crate::symbol_sidebar::View::Files
    );
    settle(application, &context);
    assert!(collapsed(0));
    assert_eq!(
        sidebar_view(application, &projects[0], &slots[0]),
        crate::symbol_sidebar::View::Outline
    );
}

#[test]
fn 실제_app_태스크_실행은_팔레트_명령에서_포커스_프로젝트_태스크를_감지해_포커스_pane_터미널로_보낸다()
 {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-task-runner-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(root.join("package.json"), PACKAGE_MANIFEST).unwrap();
    std::fs::write(root.join("Makefile"), MAKEFILE).unwrap();
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", false);
    if let PaneNode::Leaf { tabs, .. } = &mut state.layouts.write().get_mut(&project).unwrap().root
    {
        tabs.retain(|tab| !matches!(tab.kind, TabKind::Terminal { .. }));
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
    settle(application, &context);

    step(application, &context, key(egui::Key::P, command_shift()));
    step_until(application, &context, "command palette", |application| {
        application.palette.is_open() && application.palette.inspection().has_input_focus
    });
    let title = presentation::message(&application.locale, "task.runTask", &[]);
    step(
        application,
        &context,
        vec![egui::Event::Text(title.clone())],
    );
    step_until(application, &context, "run task command", |application| {
        application
            .palette
            .inspection()
            .rows
            .iter()
            .any(|row| row.is_selected && row.key == "task.runTask")
    });
    step(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    step_until(application, &context, "detected tasks", |application| {
        application.palette.is_picking_tasks() && !application.palette.inspection().rows.is_empty()
    });
    let inspection = application.palette.inspection().clone();
    assert_eq!(inspection.placeholder, title);
    assert_eq!(inspection.heading, None);
    let message = |key: &str| presentation::message(&application.locale, key, &[]);
    assert_eq!(
        inspection
            .rows
            .iter()
            .map(|row| (row.label.clone(), row.detail.clone()))
            .collect::<Vec<_>>(),
        [
            ("build".to_owned(), Some(message("task.sourceNpm"))),
            ("lint".to_owned(), Some(message("task.sourceNpm"))),
            ("check".to_owned(), Some(message("task.sourceMake"))),
        ]
    );

    step(
        application,
        &context,
        vec![egui::Event::Text("lint".into())],
    );
    step_until(application, &context, "filtered task", |application| {
        let rows = &application.palette.inspection().rows;
        rows.len() == 1 && rows[0].label == "lint" && rows[0].is_selected
    });
    step(
        application,
        &context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    assert!(!application.palette.is_open());
    let deadline = Instant::now() + DEADLINE;
    while !application
        .editor_terminal_writes
        .iter()
        .any(|delivery| delivery.payload == LINT_PAYLOAD)
    {
        assert!(Instant::now() < deadline, "task terminal delivery");
        eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
        std::thread::yield_now();
    }
    let delivery = application
        .editor_terminal_writes
        .iter()
        .find(|delivery| delivery.payload == LINT_PAYLOAD)
        .unwrap();
    assert_eq!(delivery.project, project);
    let layouts = services.state.layouts.read();
    let layout = &layouts[&project];
    let Some(PaneNode::Leaf { tabs, active, .. }) =
        taide_layout::service::find_leaf(&layout.root, &layout.focused_pane)
    else {
        panic!("focused pane")
    };
    assert_eq!(active.as_ref(), Some(&delivery.tab));
    let tab = tabs.iter().find(|tab| tab.id == delivery.tab).unwrap();
    assert_eq!(
        tab.title,
        presentation::message(&application.locale, "terminal.title", &[])
    );
    assert!(
        matches!(&tab.kind, TabKind::Terminal { session_id, cwd }
            if session_id.is_empty() && cwd.as_deref() == root.to_str()),
        "{:?}",
        tab.kind
    );
}
