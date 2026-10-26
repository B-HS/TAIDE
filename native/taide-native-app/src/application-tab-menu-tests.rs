use super::project_rail_tests::{
    Fixture, click_label, insert_project, isolated_state, launch, node_rect, pointer, project_ref,
    step,
};
use super::*;
use taide_model::agent::{AgentActivity, DetectedAgent};
use taide_model::layout::PaneNode;
use taide_model::project::ShellSlotTree;
use taide_native_ui::shell::{TAB_HEIGHT, TITLE_HEIGHT};

const DEADLINE: Duration = Duration::from_secs(10);
const AGENT_GLYPH_TEXTURE_PREFIX: &str = "native-glyph-Sparkles-";
const AGENT_SESSION: &str = "synthetic-agent-session";

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

fn tab(title: &str, kind: TabKind) -> Tab {
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
    tab(
        path.file_name().unwrap().to_str().unwrap(),
        TabKind::File {
            path: path.to_str().unwrap().into(),
        },
    )
}

fn terminal_tab(title: &str, session: &str) -> Tab {
    tab(
        title,
        TabKind::Terminal {
            session_id: session.into(),
            cwd: None,
        },
    )
}

fn tab_bar() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(f32::INFINITY, TITLE_HEIGHT + TAB_HEIGHT),
    )
}

fn open_tab_menu(application: &mut NativeApplication, context: &egui::Context, title: &str) {
    let output = step(application, context, Vec::new());
    let target = node_rect(&output, title, tab_bar()).unwrap_or_else(|| panic!("{title}"));
    step(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Secondary),
    );
}

fn submenu_rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .filter(|(_, node)| node.label().is_some_and(|text| text.starts_with(label)))
        .find_map(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
}

fn has_agent_glyph(context: &egui::Context) -> bool {
    context
        .tex_manager()
        .read()
        .allocated()
        .any(|(_, meta)| meta.name.starts_with(AGENT_GLYPH_TEXTURE_PREFIX))
}

#[test]
fn 실제_app_탭은_에이전트_터미널_glyph와_다시열기_탐색기_reveal_rename을_반영한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-native-tab-menu-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    let nested = root.join("src").join("nested");
    let assets = root.join("assets");
    for directory in [&data, &nested, &assets] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let source = nested.join("deep.rs");
    let image = assets.join("pic.png");
    std::fs::write(&source, "fn synthetic() {}\n").unwrap();
    std::fs::write(&image, "synthetic").unwrap();
    let source_path = source.to_str().unwrap().to_owned();
    let image_path = image.to_str().unwrap().to_owned();
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", false);
    let tabs = [
        file_tab(&source),
        file_tab(&image),
        terminal_tab("agent-term", AGENT_SESSION),
        terminal_tab("plain-term", "synthetic-plain-session"),
    ];
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
        *leaf = tabs.to_vec();
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
        session.focused_shell_slot = Some(slot.clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    step_until(application, &context, "tabs", |_, output| {
        tabs.iter()
            .all(|tab| node_rect(output, &tab.title, tab_bar()).is_some())
    });
    assert!(!has_agent_glyph(&context));
    let agent = DetectedAgent {
        session_id: AGENT_SESSION.into(),
        name: "claude".into(),
        pid: 1,
        activity: AgentActivity::Working,
        blocked_reason: None,
    };
    step_until(application, &context, "agent glyph", |application, _| {
        application
            .services
            .agents
            .diff(&project, std::slice::from_ref(&agent));
        has_agent_glyph(&context)
    });

    assert_eq!(
        application.open_with.surface(&image_path),
        crate::open_with::Surface::Preview(crate::open_with::PreviewKind::Image)
    );
    for (key, expected) in [
        ("explorer.openWithEditor", crate::open_with::Surface::Editor),
        (
            "explorer.openWithPreview",
            crate::open_with::Surface::Preview(crate::open_with::PreviewKind::Image),
        ),
    ] {
        open_tab_menu(application, &context, "pic.png");
        let submenu_label = presentation::message(&application.locale, "tab.reopenEditorWith", &[]);
        let output = step(application, &context, Vec::new());
        let submenu =
            submenu_rect(&output, &submenu_label).unwrap_or_else(|| panic!("{submenu_label}"));
        step(
            application,
            &context,
            vec![egui::Event::PointerMoved(submenu.center())],
        );
        let label = presentation::message(&application.locale, key, &[]);
        let beside = egui::Rect::from_min_max(
            egui::pos2(submenu.right(), 0.0),
            egui::pos2(f32::INFINITY, f32::INFINITY),
        );
        let output = step_until(application, &context, key, |_, output| {
            node_rect(output, &label, beside).is_some()
        });
        let item = node_rect(&output, &label, beside).unwrap();
        step(
            application,
            &context,
            pointer(item.center(), egui::PointerButton::Primary),
        );
        assert_eq!(
            application.open_with.surface(&image_path),
            expected,
            "{key}"
        );
    }

    application.symbol_sidebars.entry(&project, &slot).view = crate::symbol_sidebar::View::Outline;
    open_tab_menu(application, &context, "deep.rs");
    let label = presentation::message(&application.locale, "tab.revealInExplorerView", &[]);
    click_label(application, &context, &label, |_| egui::Rect::EVERYTHING);
    step_until(application, &context, "reveal", |application, _| {
        application.explorers.get(&project).is_some_and(|explorer| {
            explorer.selected.as_deref() == Some(source_path.as_str()) && explorer.rename.is_none()
        }) && application
            .trees
            .get(&project)
            .is_some_and(|page| page.rows.iter().any(|row| row.path == source_path))
            && !services.state.layouts.read()[&project]
                .shell_view
                .sidebar_collapsed
    });
    assert_eq!(
        application.symbol_sidebars.entry(&project, &slot).view,
        crate::symbol_sidebar::View::Files
    );

    open_tab_menu(application, &context, "deep.rs");
    let label = presentation::message(&application.locale, "tab.rename", &[]);
    click_label(application, &context, &label, |_| egui::Rect::EVERYTHING);
    step_until(application, &context, "rename", |application, _| {
        application.explorers.get(&project).is_some_and(|explorer| {
            explorer
                .rename
                .as_ref()
                .is_some_and(|draft| draft.row.path == source_path && draft.name == "deep.rs")
        })
    });
}
