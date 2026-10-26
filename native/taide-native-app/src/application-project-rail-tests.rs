use super::inlay_hints_tests::paint;
use super::*;
use taide_model::agent::{AgentActivity, DetectedAgent};
use taide_model::ids::{ProjectGroupId, ShellSlotId};
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectDisplay, ProjectGroup, ProjectRef, ShellSlotTree};

const DEADLINE: Duration = Duration::from_secs(10);
const WATCHER_ATTACH_DEADLINE: Duration = Duration::from_secs(60);
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

fn label(application: &NativeApplication, key: &str) -> String {
    presentation::message(&application.locale, key, &[])
}

fn modal_area(id: &'static str) -> impl Fn(&egui::Context) -> egui::Rect {
    move |context| {
        context
            .memory(|memory| memory.area_rect(egui::Id::new(id)))
            .unwrap_or_else(|| panic!("{id} is shown"))
    }
}

/// The accessible menu entry whose name starts with `label` (a submenu trigger also names its
/// arrow): a group's name is also its rail header, which the rail contains and a menu does not.
fn menu_entry(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .filter(|(_, node)| node.label().is_some_and(|name| name.starts_with(label)))
        .filter_map(|(_, node)| node.bounds())
        .map(|bounds| {
            egui::Rect::from_min_max(
                egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
            )
        })
        .find(|rect| !rail_area().contains_rect(*rect))
}

fn choose_group_submenu(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &str,
    entry: &str,
) {
    settle_snapshot(application, context);
    let output = step(application, context, Vec::new());
    let button = node_rect(&output, project, rail_area()).unwrap_or_else(|| panic!("{project}"));
    step(
        application,
        context,
        pointer(button.center(), egui::PointerButton::Secondary),
    );
    let add_to = label(application, "projectGroup.addTo");
    let output = step(application, context, Vec::new());
    let trigger = menu_entry(&output, &add_to).expect("group submenu trigger");
    step(
        application,
        context,
        vec![egui::Event::PointerMoved(trigger.center())],
    );
    let output = paint_until(
        application,
        context,
        "group submenu",
        |_| {},
        |output| menu_entry(output, entry).is_some(),
    );
    let target = menu_entry(&output, entry).unwrap();
    step(
        application,
        context,
        pointer(target.center(), egui::PointerButton::Primary),
    );
}

fn settle_frames(application: &mut NativeApplication, context: &egui::Context) {
    for _ in 0..DIALOG_SETTLE_FRAMES {
        step(application, context, Vec::new());
    }
}

#[test]
fn 실제_app_그룹은_새그룹_생성_그룹추가_수정_열기결과_toast와_삭제확인을_runtime에_반영한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-project-groups-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let names = ["alpha", "beta", "gamma"];
    let roots = names.map(|name| fixture.directory.join(name));
    std::fs::create_dir_all(&data).unwrap();
    for root in &roots {
        std::fs::create_dir_all(root).unwrap();
    }
    let ids = [ProjectId::new(), ProjectId::new(), ProjectId::new()];
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    for ((id, root), name) in ids.iter().zip(&roots).zip(names) {
        let record = Project {
            id: id.clone(),
            root: std::fs::canonicalize(root)
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            name: name.into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        };
        let file = state.paths.project_file(id);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, serde_json::to_vec(&record).unwrap()).unwrap();
    }
    for index in 0..2 {
        insert_project(&state, &ids[index], &roots[index], names[index], false);
    }
    {
        let mut session = state.session.write();
        session.projects = (0..2)
            .map(|index| project_ref(&ids[index], &roots[index], names[index]))
            .collect();
        session.active_project = Some(ids[0].clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: ids[0].clone(),
        });
        session.focused_shell_slot = Some(slot);
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
    let groups = || services.state.session.read().groups.clone();
    let group_dialog = modal_area(taide_native_ui::project_group_dialog::DIALOG_ID);
    let delete_dialog = modal_area(taide_native_ui::project_group_dialog::DELETE_DIALOG_ID);
    let lane_three = format!("{} 3", label(application, "projectGroup.color"));

    let new_group = label(application, "projectGroup.newGroup");
    choose_group_submenu(application, &context, "alpha", &new_group);
    settle_frames(application, &context);
    assert!(application.project_group_dialog.is_open());
    step(
        application,
        &context,
        vec![egui::Event::Text("QA team".into())],
    );
    click_label(application, &context, &lane_three, &group_dialog);
    let create = label(application, "projectGroup.create");
    click_label(application, &context, &create, &group_dialog);
    paint_until(
        application,
        &context,
        "group create",
        |_| {},
        |_| {
            matches!(groups().as_slice(), [group]
                if group.name == "QA team"
                    && group.color.as_deref() == Some("lane3")
                    && group.members == [ids[0].clone()])
        },
    );
    let deadline = Instant::now() + DEADLINE;
    while application.project_group_dialog.is_open() {
        assert!(Instant::now() < deadline, "created group dialog closes");
        step(application, &context, Vec::new());
        std::thread::yield_now();
    }
    let group = groups()[0].id.clone();

    choose_group_submenu(application, &context, "beta", "QA team");
    paint_until(
        application,
        &context,
        "add to group",
        |_| {},
        |_| groups()[0].members == [ids[0].clone(), ids[1].clone()],
    );

    choose_rail_menu(application, &context, "QA team", "projectGroup.edit");
    settle_frames(application, &context);
    assert!(application.project_group_dialog.is_open());
    step(application, &context, vec![egui::Event::Text(" 2".into())]);
    click_label(application, &context, &lane_three, &group_dialog);
    let save = label(application, "common.save");
    click_label(application, &context, &save, &group_dialog);
    paint_until(
        application,
        &context,
        "group edit",
        |_| {},
        |_| {
            let groups = groups();
            groups[0].name == "QA team 2" && groups[0].color.is_none()
        },
    );
    let deadline = Instant::now() + DEADLINE;
    while application.project_group_dialog.is_open() {
        assert!(Instant::now() < deadline, "edited group dialog closes");
        step(application, &context, Vec::new());
        std::thread::yield_now();
    }

    assert!(application.submit(HostCommand::SetProjectGroupMembers {
        group: group.clone(),
        members: ids.to_vec(),
    }));
    paint_until(
        application,
        &context,
        "closed member",
        |_| {},
        |_| groups()[0].members == ids,
    );
    let _os_watch_registration = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(crate::projects::NativeProjects::exclusive_os_watch_registration());
    choose_rail_menu(application, &context, "QA team 2", "projectGroup.open");
    let result = presentation::message(
        &application.locale,
        "projectGroup.openResult",
        &[("opened", "1"), ("skipped", "2")],
    );
    let deadline = Instant::now() + WATCHER_ATTACH_DEADLINE;
    let mut seen = HashSet::new();
    while !application
        .toasts
        .is_showing(taide_native_ui::toast::Kind::Success, &result)
    {
        seen.extend(
            application
                .toasts
                .inspection()
                .into_iter()
                .map(|toast| toast.title),
        );
        assert!(
            Instant::now() < deadline,
            "group open result toast {result:?}: {seen:?} open={}",
            services.state.projects.read().len()
        );
        step(application, &context, Vec::new());
        std::thread::yield_now();
    }
    assert!(services.state.projects.read().contains_key(&ids[2]));
    assert!(services.state.watchers.read().contains_key(&ids[2]));

    choose_rail_menu(application, &context, "QA team 2", "projectGroup.delete");
    settle_frames(application, &context);
    assert!(application.project_group_delete.is_open());
    let delete = label(application, "projectGroup.delete");
    click_label(application, &context, &delete, &delete_dialog);
    paint_until(
        application,
        &context,
        "group delete",
        |_| {},
        |_| groups().is_empty(),
    );
    assert!(!application.project_group_delete.is_open());
    assert!(
        ids.iter()
            .all(|id| services.state.projects.read().contains_key(id)),
        "deleting a group leaves its members open"
    );
}

const DRAG_ACTIVATION_PROBE: f32 = 6.0;
const SLOT_EDGE_INSET: f32 = 10.0;

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

/// Presses the rail entry named `label`, carries it past the activation distance to `target`
/// and releases there.
fn drag_rail_entry(
    application: &mut NativeApplication,
    context: &egui::Context,
    label: &str,
    target: impl Fn(&egui::FullOutput) -> egui::Pos2,
) {
    settle_snapshot(application, context);
    let output = step(application, context, Vec::new());
    let source = node_rect(&output, label, rail_area()).unwrap_or_else(|| panic!("{label}"));
    let target = target(&output);
    step(application, context, primary_button(source.center(), true));
    for point in [
        source.center() + egui::vec2(0.0, DRAG_ACTIVATION_PROBE),
        target,
    ] {
        step(application, context, vec![egui::Event::PointerMoved(point)]);
    }
    step(application, context, primary_button(target, false));
}

#[test]
fn 실제_app_rail_drag는_프로젝트_순서_그룹_순서와_슬롯_분할을_runtime에_반영한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-project-drag-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let names = ["alpha", "beta", "gamma"];
    let roots = names.map(|name| fixture.directory.join(name));
    std::fs::create_dir_all(&data).unwrap();
    for root in &roots {
        std::fs::create_dir_all(root).unwrap();
    }
    let ids = [ProjectId::new(), ProjectId::new(), ProjectId::new()];
    let groups = [ProjectGroupId::new(), ProjectGroupId::new()];
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    for ((id, root), name) in ids.iter().zip(&roots).zip(names) {
        insert_project(&state, id, root, name, false);
    }
    {
        let mut session = state.session.write();
        session.projects = ids
            .iter()
            .zip(&roots)
            .zip(names)
            .map(|((id, root), name)| project_ref(id, root, name))
            .collect();
        session.active_project = Some(ids[0].clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: ids[0].clone(),
        });
        session.focused_shell_slot = Some(slot);
        session.groups = ["first", "second"]
            .iter()
            .zip(&groups)
            .zip(&ids)
            .map(|((name, group), member)| ProjectGroup {
                id: group.clone(),
                name: (*name).into(),
                color: None,
                members: vec![member.clone()],
                collapsed: false,
            })
            .collect();
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    paint_until(
        application,
        &context,
        "rail",
        |_| {},
        |output| has_label(output, "gamma") && has_label(output, "second"),
    );
    let session = || services.state.session.read().clone();
    let entry = |label: &'static str| {
        move |output: &egui::FullOutput| {
            node_rect(output, label, rail_area())
                .unwrap_or_else(|| panic!("{label}"))
                .center()
        }
    };

    drag_rail_entry(application, &context, "gamma", entry("beta"));
    paint_until(
        application,
        &context,
        "project reorder",
        |_| {},
        |_| {
            session()
                .projects
                .iter()
                .map(|project| &project.id)
                .eq([&ids[0], &ids[2], &ids[1]])
        },
    );
    assert_eq!(
        session()
            .groups
            .iter()
            .map(|group| group.members.clone())
            .collect::<Vec<_>>(),
        vec![vec![ids[0].clone()], vec![ids[1].clone()]],
        "a reorder never touches membership"
    );

    drag_rail_entry(application, &context, "second", entry("first"));
    paint_until(
        application,
        &context,
        "group reorder",
        |_| {},
        |_| {
            session()
                .groups
                .iter()
                .map(|group| &group.id)
                .eq([&groups[1], &groups[0]])
        },
    );

    drag_rail_entry(application, &context, "gamma", |_| {
        let screen = context.content_rect();
        egui::pos2(screen.right() - SLOT_EDGE_INSET, screen.center().y)
    });
    paint_until(
        application,
        &context,
        "slot split",
        |_| {},
        |_| {
            matches!(
                &session().shell_slots,
                Some(ShellSlotTree::Split { children, .. })
                    if matches!(
                        children.as_slice(),
                        [
                            ShellSlotTree::Leaf { project_id: left, .. },
                            ShellSlotTree::Leaf { project_id: right, .. },
                        ] if left == &ids[0] && right == &ids[2]
                    )
            )
        },
    );
    assert!(!taide_native_ui::project_drag::is_dragging(&context));
}
