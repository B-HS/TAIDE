use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, pointer, project_ref, step,
};
use super::*;
use taide_model::project::ShellSlotTree;
use taide_native_ui::error_boundary::{Region, inject_instance_panic, inject_panic};
use taide_native_ui::shell::{EXPLORER_DEFAULT_WIDTH, PROJECT_RAIL_WIDTH};

const DEADLINE: Duration = Duration::from_secs(10);

fn fixture(name: &str) -> Fixture {
    let fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-error-boundary-{name}-{}",
            ProjectId::new()
        )),
        application: None,
    };
    std::fs::create_dir_all(fixture.directory.join("data")).unwrap();
    fixture
}

fn label(application: &NativeApplication, key: &str) -> String {
    presentation::message(&application.locale, key, &[])
}

fn alert(output: &egui::FullOutput) -> Option<egui::Rect> {
    let update = output.platform_output.accesskit_update.as_ref()?;
    update.nodes.iter().find_map(|(_, node)| {
        (node.role() == egui::accesskit::Role::Alert)
            .then(|| node.bounds())
            .flatten()
            .map(|bounds| {
                egui::Rect::from_min_max(
                    egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                    egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                )
            })
    })
}

fn step_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&egui::FullOutput) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = step(application, context, Vec::new());
        if ready(&output) {
            return output;
        }
        assert!(Instant::now() < deadline, "phase={phase}");
        std::thread::yield_now();
    }
}

fn retry(application: &mut NativeApplication, context: &egui::Context, output: &egui::FullOutput) {
    let retry = node_rect(
        output,
        &label(application, "common.retry"),
        egui::Rect::EVERYTHING,
    )
    .expect("fallback retry button");
    step(
        application,
        context,
        pointer(retry.center(), egui::PointerButton::Primary),
    );
}

fn has_text(output: &egui::FullOutput, text: &str) -> bool {
    output.shapes.iter().any(
        |clipped| matches!(&clipped.shape, egui::Shape::Text(shape) if shape.galley.text() == text),
    )
}

fn assert_fallback(application: &NativeApplication, output: &egui::FullOutput, region: Region) {
    for key in [region.label_key(), "errorBoundary.description"] {
        assert!(has_text(output, &label(application, key)), "{key}");
    }
    assert!(
        node_rect(
            output,
            &label(application, "common.retry"),
            egui::Rect::EVERYTHING
        )
        .is_some()
    );
    assert!(alert(output).is_some());
}

#[test]
fn 실제_app은_상태표시줄과_rail_panic을_그_영역만_fallback으로_바꾸고_재시도로_복구한다() {
    let mut fixture = fixture("shell");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    let alpha = ProjectId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut session = state.session.write();
        let mut project = project_ref(&alpha, &root, "alpha");
        project.display.icon = Some("rocket".into());
        session.projects = vec![project];
        session.active_project = Some(alpha.clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let rail = egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(PROJECT_RAIL_WIDTH, f32::INFINITY),
    );
    let problems = label(application, "problems.toggleAriaLabel");
    step_until(application, &context, "shell", |output| {
        node_rect(output, "alpha", rail).is_some()
            && node_rect(output, &problems, egui::Rect::EVERYTHING).is_some()
    });
    application.system_usage.detail_open = true;
    inject_panic(&context, Region::StatusBar);
    let output = step(application, &context, Vec::new());
    assert!(!application.system_usage.detail_open);
    assert_fallback(application, &output, Region::StatusBar);
    assert!(node_rect(&output, &problems, egui::Rect::EVERYTHING).is_none());
    assert!(node_rect(&output, "alpha", rail).is_some());
    retry(application, &context, &output);
    let output = step(application, &context, Vec::new());
    assert!(alert(&output).is_none());
    assert!(node_rect(&output, &problems, egui::Rect::EVERYTHING).is_some());
    let icons_before = context
        .tex_manager()
        .read()
        .allocated()
        .filter(|(_, meta)| meta.name.starts_with("native-glyph-Rocket-"))
        .count();
    assert!(icons_before > 0);
    inject_panic(&context, Region::ActivityBar);
    let output = step(application, &context, Vec::new());
    assert_fallback(application, &output, Region::ActivityBar);
    assert!(node_rect(&output, "alpha", rail).is_none());
    assert!(node_rect(&output, &problems, egui::Rect::EVERYTHING).is_some());
    retry(application, &context, &output);
    let output = step(application, &context, Vec::new());
    assert!(alert(&output).is_none());
    assert!(node_rect(&output, "alpha", rail).is_some());
}

#[test]
fn 실제_app은_welcome_panic을_welcome만_fallback으로_바꾸고_재시도로_복구한다() {
    let mut fixture = fixture("welcome");
    let state = isolated_state(fixture.directory.join("data"));
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let open_folder = label(application, "app.openFolder");
    let problems = label(application, "problems.toggleAriaLabel");
    step_until(application, &context, "welcome", |output| {
        node_rect(output, &open_folder, egui::Rect::EVERYTHING).is_some()
    });
    inject_panic(&context, Region::Welcome);
    let output = step(application, &context, Vec::new());
    assert_fallback(application, &output, Region::Welcome);
    assert!(node_rect(&output, &open_folder, egui::Rect::EVERYTHING).is_none());
    assert!(node_rect(&output, &problems, egui::Rect::EVERYTHING).is_some());
    retry(application, &context, &output);
    let output = step(application, &context, Vec::new());
    assert!(alert(&output).is_none());
    assert!(node_rect(&output, &open_folder, egui::Rect::EVERYTHING).is_some());
}

#[test]
fn 실제_app은_재시도로_다시_그린_상태표시줄의_문제_토글_클릭을_focused_slot에_적용한다() {
    let mut fixture = fixture("status-apply");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&alpha, &root, "alpha")];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot.clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let problems = label(application, "problems.toggleAriaLabel");
    step_until(application, &context, "status", |output| {
        node_rect(output, &problems, egui::Rect::EVERYTHING).is_some()
    });
    inject_panic(&context, Region::StatusBar);
    let output = step(application, &context, Vec::new());
    assert_fallback(application, &output, Region::StatusBar);
    retry(application, &context, &output);
    let output = step(application, &context, Vec::new());
    let toggle = node_rect(&output, &problems, egui::Rect::EVERYTHING).expect("problems toggle");
    assert!(!application.problems.is_open(&slot));
    step(
        application,
        &context,
        pointer(toggle.center(), egui::PointerButton::Primary),
    );
    assert!(application.problems.is_open(&slot));
}

fn texture_ids(context: &egui::Context, prefix: &str) -> Vec<egui::TextureId> {
    context
        .tex_manager()
        .read()
        .allocated()
        .filter(|(_, meta)| meta.name.starts_with(prefix))
        .map(|(id, _)| *id)
        .collect()
}

#[test]
fn 실제_app은_사이드바_패널_panic을_그_패널만_fallback으로_바꾸고_그_pass_요청과_패널_상태를_버리며_재시도로_새로_mount한다()
 {
    let mut fixture = fixture("sidebar");
    let root = fixture.directory.join("alpha");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("a.txt");
    std::fs::write(&file, "synthetic\n").unwrap();
    let path = file.to_str().unwrap().to_owned();
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(fixture.directory.join("data"));
    insert_project(&state, &alpha, &root, "alpha", false);
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&alpha, &root, "alpha")];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot.clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let rail = egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(PROJECT_RAIL_WIDTH, f32::INFINITY),
    );
    let panel = egui::Rect::from_min_max(
        egui::pos2(PROJECT_RAIL_WIDTH, 0.0),
        egui::pos2(PROJECT_RAIL_WIDTH + EXPLORER_DEFAULT_WIDTH, f32::INFINITY),
    );
    let problems = label(application, "problems.toggleAriaLabel");
    let output = step_until(application, &context, "explorer", |output| {
        node_rect(output, "a.txt", panel).is_some()
    });
    let switch = texture_ids(&context, "native-navigation-FolderTree-");
    assert!(!switch.is_empty());
    assert!(output.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::Mesh(mesh) if switch.contains(&mesh.texture_id)
    )));
    let row = node_rect(&output, "a.txt", panel).unwrap();
    step(
        application,
        &context,
        pointer(row.center(), egui::PointerButton::Primary),
    );
    assert_eq!(
        application.explorers[&alpha].selected.as_deref(),
        Some(path.as_str())
    );
    let delete = vec![egui::Event::Key {
        key: egui::Key::Backspace,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    }];
    let owner = application.explorer_clipboards.mount(&slot).unwrap();
    inject_instance_panic(&context, Region::SidebarPanel, egui::Id::new(&slot));
    let output = step(application, &context, delete.clone());
    assert_fallback(application, &output, Region::SidebarPanel);
    assert!(node_rect(&output, "a.txt", panel).is_none());
    assert!(node_rect(&output, "alpha", rail).is_some());
    assert!(node_rect(&output, &problems, egui::Rect::EVERYTHING).is_some());
    assert!(application.pending_entry_delete.is_none());
    assert!(application.explorers[&alpha].selected.is_none());
    assert_ne!(application.explorer_clipboards.mount(&slot).unwrap(), owner);
    retry(application, &context, &output);
    let output = step_until(application, &context, "remount", |output| {
        node_rect(output, "a.txt", panel).is_some()
    });
    assert!(alert(&output).is_none());
    let row = node_rect(&output, "a.txt", panel).unwrap();
    step(
        application,
        &context,
        pointer(row.center(), egui::PointerButton::Primary),
    );
    step(application, &context, delete);
    assert!(
        application
            .pending_entry_delete
            .as_ref()
            .is_some_and(|pending| pending.request.path == path)
    );
    application.symbol_sidebars.entry(&alpha, &slot).view = crate::symbol_sidebar::View::Outline;
    step(application, &context, Vec::new());
    inject_instance_panic(&context, Region::SidebarPanel, egui::Id::new(&slot));
    let output = step(application, &context, Vec::new());
    assert_fallback(application, &output, Region::SidebarPanel);
    assert!(application.pending_entry_delete.is_none());
    assert_eq!(
        application.symbol_sidebars.entry(&alpha, &slot).view,
        crate::symbol_sidebar::View::Files
    );
}
