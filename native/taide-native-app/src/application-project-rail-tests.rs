use super::inlay_hints_tests::paint;
use super::*;
use taide_model::agent::{AgentActivity, DetectedAgent};
use taide_model::ids::{ProjectGroupId, ShellSlotId};
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay, ProjectGroup, ProjectRef, ShellSlotTree};

const DEADLINE: Duration = Duration::from_secs(10);
const RAIL_ICON_TEXTURE_PREFIX: &str = "native-glyph-Rocket-";
const DIALOG_SETTLE_FRAMES: usize = 8;
const MIRROR_DEBOUNCE_HOLD: Duration = Duration::from_secs(60);
const UNSAVED_DRAFT: &str = "unsaved beta draft";

struct Fixture {
    directory: PathBuf,
    application: Option<NativeApplication>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(application) = &mut self.application {
            eframe::App::on_exit(application);
        }
        drop(self.application.take());
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn has_label(output: &egui::FullOutput, label: &str) -> bool {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .is_some_and(|update| {
            update
                .nodes
                .iter()
                .any(|(_, node)| node.label() == Some(label))
        })
}

fn has_text(output: &egui::FullOutput, text: &str) -> bool {
    output.shapes.iter().any(
        |clipped| matches!(&clipped.shape, egui::Shape::Text(shape) if shape.galley.text() == text),
    )
}

fn paint_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    before: impl Fn(&NativeApplication),
    ready: impl Fn(&egui::FullOutput) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        before(application);
        let output = paint(application, context, Vec::new());
        if ready(&output) {
            return output;
        }
        assert!(Instant::now() < deadline, "phase={phase}");
        std::thread::yield_now();
    }
}

fn project_ref(id: &ProjectId, root: &std::path::Path, name: &str) -> ProjectRef {
    ProjectRef {
        id: id.clone(),
        root: root.to_str().unwrap().into(),
        name: name.into(),
        display: ProjectDisplay::default(),
        root_missing: false,
    }
}

/// Synthetic app data with the IDE server and agent hooks off.
fn isolated_state(data: PathBuf) -> AppState {
    let state = AppState::new(AppPaths::new(data));
    {
        let mut settings = state.settings.write();
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
        std::fs::write(
            state.paths.settings_file(),
            serde_json::to_vec(&*settings).unwrap(),
        )
        .unwrap();
    }
    state
}

fn insert_project(
    state: &AppState,
    id: &ProjectId,
    root: &std::path::Path,
    name: &str,
    root_missing: bool,
) {
    state.projects.write().insert(
        id.clone(),
        Project {
            id: id.clone(),
            root: root.to_str().unwrap().into(),
            name: name.into(),
            capabilities: Vec::new(),
            root_missing,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    state
        .layouts
        .write()
        .insert(id.clone(), taide_layout::service::default_layout());
}

fn launch(fixture: &mut Fixture, state: AppState) -> egui::Context {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let context = egui::Context::default();
    context.enable_accesskit();
    fixture.application = Some(
        NativeApplication::new(
            &eframe::CreationContext::_new_kittest(context.clone()),
            runtime,
            state,
            tasks,
            fixture.directory.join("bin/taide"),
            Vec::new(),
        )
        .unwrap(),
    );
    context
}

fn step(
    application: &mut NativeApplication,
    context: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    paint(application, context, events)
}

/// The accessible widget named `label` inside `within`: the rail and a project's slot header both
/// carry the project name, and the Welcome card and the dialog both show "Save".
fn node_rect(output: &egui::FullOutput, label: &str, within: egui::Rect) -> Option<egui::Rect> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .filter(|(_, node)| node.label() == Some(label))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .find(|rect| within.contains_rect(*rect))
}

fn rail_area() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(taide_native_ui::shell::PROJECT_RAIL_WIDTH, f32::INFINITY),
    )
}

fn dialog_area(context: &egui::Context) -> egui::Rect {
    context
        .memory(|memory| {
            memory.area_rect(egui::Id::new(
                taide_native_ui::project_display_dialog::DIALOG_ID,
            ))
        })
        .expect("project display dialog is shown")
}

fn pointer(position: egui::Pos2, button: egui::PointerButton) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::PointerButton {
            pos: position,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        })
        .fold(
            vec![egui::Event::PointerMoved(position)],
            |mut events, event| {
                events.push(event);
                events
            },
        )
}

fn click_label(
    application: &mut NativeApplication,
    context: &egui::Context,
    label: &str,
    within: impl Fn(&egui::Context) -> egui::Rect,
) {
    let output = step(application, context, Vec::new());
    let target = node_rect(&output, label, within(context)).unwrap_or_else(|| panic!("{label}"));
    step(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
}

/// Paints until the shell snapshot has caught up with the session, so a rail button is looked up
/// where the next frame draws it.
fn settle_snapshot(application: &mut NativeApplication, context: &egui::Context) {
    let deadline = Instant::now() + DEADLINE;
    loop {
        step(application, context, Vec::new());
        let snapshot = application.controller.snapshot();
        let session = application.services.state.session.read().clone();
        if snapshot.projects == session.projects
            && snapshot.groups == session.groups
            && snapshot.shell.tree == session.shell_slots
        {
            return;
        }
        assert!(Instant::now() < deadline, "shell snapshot catches up");
        std::thread::yield_now();
    }
}

fn choose_rail_menu(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &str,
    key: &str,
) {
    settle_snapshot(application, context);
    let output = step(application, context, Vec::new());
    let button = node_rect(&output, project, rail_area()).unwrap_or_else(|| panic!("{project}"));
    step(
        application,
        context,
        pointer(button.center(), egui::PointerButton::Secondary),
    );
    let label = presentation::message(&application.locale, key, &[]);
    click_label(application, context, &label, |_| egui::Rect::EVERYTHING);
}

#[test]
fn 실제_app_rail은_표시_glyph와_폴링된_에이전트_상태와_배지_설정을_반영한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-project-rail-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let alpha_root = fixture.directory.join("alpha");
    let beta_root = fixture.directory.join("beta");
    for directory in [&data, &alpha_root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let alpha = ProjectId::new();
    let beta = ProjectId::new();
    let state = isolated_state(data);
    for (id, root, name) in [(&alpha, &alpha_root, "alpha"), (&beta, &beta_root, "beta")] {
        insert_project(&state, id, root, name, id == &beta);
    }
    {
        let mut session = state.session.write();
        let mut labelled = project_ref(&alpha, &alpha_root, "alpha");
        labelled.display = ProjectDisplay {
            icon: None,
            label: Some("AL".into()),
            color: Some("lane2".into()),
        };
        let mut missing = project_ref(&beta, &beta_root, "beta");
        missing.display.icon = Some("rocket".into());
        missing.root_missing = true;
        session.projects = vec![labelled, missing];
        session.active_project = Some(alpha.clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let output = paint_until(
        application,
        &context,
        "display",
        |_| {},
        |output| has_label(output, "alpha") && has_label(output, "beta"),
    );
    assert!(has_text(&output, "AL"));
    assert!(
        context
            .tex_manager()
            .read()
            .allocated()
            .any(|(_, meta)| meta.name.starts_with(RAIL_ICON_TEXTURE_PREFIX))
    );
    let working = DetectedAgent {
        session_id: "synthetic-rail-session".into(),
        name: "claude".into(),
        pid: 1,
        activity: AgentActivity::Working,
        blocked_reason: None,
    };
    let inject = |application: &NativeApplication| {
        application
            .services
            .agents
            .diff(&alpha, std::slice::from_ref(&working));
    };
    let working_label = format!(
        "alpha — {}",
        presentation::message(&application.locale, "agent.status.working", &[])
    );
    paint_until(application, &context, "agent badge", &inject, |output| {
        has_label(output, &working_label)
    });
    services.state.settings.write().agent_status_badge_enabled = false;
    let output = paint_until(application, &context, "badge disabled", &inject, |output| {
        has_label(output, "alpha") && !services.agents.agents_for(&alpha).is_empty()
    });
    assert!(!has_label(&output, &working_label));
}

#[test]
fn 실제_app_rail_메뉴는_슬롯열기_그룹제거_표시초기화_표시dialog_저장과_프로젝트_닫기를_runtime에_반영한다()
 {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-project-menu-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let alpha_root = fixture.directory.join("alpha");
    let beta_root = fixture.directory.join("beta");
    for directory in [&data, &alpha_root, &beta_root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let alpha = ProjectId::new();
    let beta = ProjectId::new();
    let slot = ShellSlotId::new();
    let group = ProjectGroupId::new();
    let state = isolated_state(data);
    for (id, root, name) in [(&alpha, &alpha_root, "alpha"), (&beta, &beta_root, "beta")] {
        insert_project(&state, id, root, name, false);
    }
    {
        let mut session = state.session.write();
        let mut labelled = project_ref(&alpha, &alpha_root, "alpha");
        labelled.display.label = Some("AL".into());
        session.projects = vec![labelled, project_ref(&beta, &beta_root, "beta")];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot.clone());
        session.groups = vec![ProjectGroup {
            id: group.clone(),
            name: "team".into(),
            color: None,
            members: vec![beta.clone()],
            collapsed: false,
        }];
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    paint_until(
        application,
        &context,
        "rail",
        |_| {},
        |output| has_label(output, "alpha") && has_label(output, "beta"),
    );
    let session = || services.state.session.read().clone();
    let is_split_right = || {
        matches!(
            &session().shell_slots,
            Some(ShellSlotTree::Split { children, .. })
                if matches!(
                    children.as_slice(),
                    [
                        ShellSlotTree::Leaf { project_id: left, .. },
                        ShellSlotTree::Leaf { project_id: right, .. },
                    ] if left == &alpha && right == &beta
                )
        )
    };
    let is_group_empty = || {
        session()
            .groups
            .iter()
            .any(|candidate| candidate.id == group && candidate.members.is_empty())
    };
    let alpha_display = || {
        session()
            .projects
            .into_iter()
            .find(|project| project.id == alpha)
            .map(|project| project.display)
    };
    let saved_display = ProjectDisplay {
        icon: None,
        label: Some("QA".into()),
        color: None,
    };
    let is_beta_closed = || {
        !services.state.projects.read().contains_key(&beta)
            && session().projects.iter().all(|project| project.id != beta)
    };

    choose_rail_menu(application, &context, "beta", "shellSlot.openToTheRight");
    paint_until(
        application,
        &context,
        "open right",
        |_| {},
        |_| is_split_right(),
    );
    choose_rail_menu(application, &context, "beta", "projectGroup.removeFrom");
    paint_until(
        application,
        &context,
        "ungroup",
        |_| {},
        |_| is_group_empty(),
    );
    let is_reset = || alpha_display() == Some(ProjectDisplay::default());
    choose_rail_menu(application, &context, "alpha", "project.displayReset");
    paint_until(
        application,
        &context,
        "display reset",
        |_| {},
        |_| is_reset(),
    );

    choose_rail_menu(application, &context, "alpha", "project.displayMenu");
    for _ in 0..DIALOG_SETTLE_FRAMES {
        step(application, &context, Vec::new());
    }
    assert!(application.project_display_dialog.is_open());
    let text = |key: &str| presentation::message(&application.locale, key, &[]);
    let (mode, save) = (text("project.displayModeLabel"), text("common.save"));
    click_label(application, &context, &mode, dialog_area);
    for _ in 0..DIALOG_SETTLE_FRAMES {
        step(application, &context, Vec::new());
    }
    step(application, &context, vec![egui::Event::Text("QA".into())]);
    click_label(application, &context, &save, dialog_area);
    let is_saved = || alpha_display().as_ref() == Some(&saved_display);
    paint_until(application, &context, "dialog save", |_| {}, |_| is_saved());
    let deadline = Instant::now() + DEADLINE;
    while application.project_display_dialog.is_open() {
        assert!(Instant::now() < deadline, "saved display dialog closes");
        step(application, &context, Vec::new());
        std::thread::yield_now();
    }

    let pane = services.state.layouts.read()[&beta].focused_pane.clone();
    let beta_untitled = |application: &NativeApplication| {
        application
            .controller
            .snapshot()
            .layouts
            .get(&beta)
            .into_iter()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(crate::tabs::tabs_in)
            .find(|tab| matches!(tab.kind, TabKind::Untitled { .. }))
            .map(|tab| tab.id.clone())
    };
    assert!(beta_untitled(application).is_none());
    assert!(application.submit(HostCommand::NewUntitled {
        project: beta.clone(),
        pane,
    }));
    let deadline = Instant::now() + DEADLINE;
    let tab = loop {
        step(application, &context, Vec::new());
        if let Some(tab) = beta_untitled(application) {
            break tab;
        }
        assert!(Instant::now() < deadline, "beta untitled tab");
        std::thread::yield_now();
    };
    let document = application
        .store
        .open_untitled(tab.clone(), UNSAVED_DRAFT, "plaintext".into())
        .unwrap();
    while !application.untitled.contains_key(&tab) {
        assert!(Instant::now() < deadline, "beta untitled document");
        step(application, &context, Vec::new());
        std::thread::yield_now();
    }
    let revision = application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .revision;
    application.persistence.changed(
        document,
        revision,
        Instant::now() + MIRROR_DEBOUNCE_HOLD,
        true,
        Duration::ZERO,
    );

    choose_rail_menu(application, &context, "beta", "project.close");
    paint_until(application, &context, "close", |_| {}, |_| is_beta_closed());
    let mirrors = taide_file::service::list_untitled_mirrors(&services.state.paths, &beta).unwrap();
    assert!(
        mirrors
            .iter()
            .any(|mirror| mirror.tab_id == tab && mirror.content == UNSAVED_DRAFT),
        "the close mirrors a draft whose debounce has not elapsed"
    );
    let deadline = Instant::now() + DEADLINE;
    while application.untitled.contains_key(&tab) {
        assert!(
            Instant::now() < deadline,
            "closed project releases its documents"
        );
        step(application, &context, Vec::new());
        std::thread::yield_now();
    }
    assert!(application.store.documents().snapshot(document).is_err());
    assert!(
        application
            .drafts()
            .unwrap()
            .iter()
            .all(|draft| draft.project != beta)
    );
}
