use super::project_rail_tests::{Fixture, isolated_state, launch};
use super::*;
use taide_native_ui::toast::Kind;

const DEADLINE: Duration = Duration::from_secs(10);
const SCREEN: [f32; 2] = [1000.0, 700.0];
const OVERLAY_OPACITY: f32 = 0.85;
const OVERLAY_BORDER: f32 = 4.0;

#[derive(Debug)]
struct SyntheticDrop(PathBuf);

impl egui::DroppedFile for SyntheticDrop {
    fn path(&self) -> &std::path::Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|error| error.to_string())
    }
}

fn screen() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(SCREEN[0], SCREEN[1]))
}

fn input(hovered: &[&std::path::Path], dropped: &[&std::path::Path]) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(screen()),
        hovered_files: hovered
            .iter()
            .map(|path| egui::HoveredFile {
                path: Some(path.to_path_buf()),
                ..Default::default()
            })
            .collect(),
        dropped_files: dropped
            .iter()
            .map(|path| Arc::new(SyntheticDrop(path.to_path_buf())) as egui::DroppedFileHandle)
            .collect(),
        ..Default::default()
    }
}

fn run(
    application: &mut NativeApplication,
    context: &egui::Context,
    input: egui::RawInput,
) -> egui::FullOutput {
    let mut output = context.run_ui(input, |ui| {
        eframe::App::logic(application, ui.ctx(), &mut eframe::Frame::_new_kittest());
        eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest());
    });
    output.textures_delta.clear();
    output
}

fn frame_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let deadline = Instant::now() + DEADLINE;
    while !ready(application) {
        run(application, context, input(&[], &[]));
        assert!(
            Instant::now() < deadline,
            "phase={phase} status={:?}",
            application.status
        );
        std::thread::yield_now();
    }
}

fn has_text(output: &egui::FullOutput, text: &str) -> bool {
    output.shapes.iter().any(
        |clipped| matches!(&clipped.shape, egui::Shape::Text(shape) if shape.galley.text() == text),
    )
}

fn project_at(application: &NativeApplication, root: &std::path::Path) -> Option<ProjectId> {
    let root = root.to_str().unwrap();
    application
        .services
        .state
        .projects
        .read()
        .values()
        .find(|project| project.root == root)
        .map(|project| project.id.clone())
}

fn file_tab(application: &NativeApplication, project: &ProjectId, path: &str) -> Option<Tab> {
    let layouts = application.services.state.layouts.read();
    taide_layout::service::all_roots(layouts.get(project)?)
        .flat_map(crate::tabs::tabs_in)
        .find(|tab| matches!(&tab.kind, TabKind::File { path: open } if open == path))
        .cloned()
}

fn fixture(name: &str) -> (Fixture, PathBuf) {
    let fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-file-drop-{name}-{}",
            ProjectId::new()
        )),
        application: None,
    };
    std::fs::create_dir_all(fixture.directory.join("data")).unwrap();
    let root = fixture.directory.canonicalize().unwrap();
    (fixture, root)
}

#[test]
fn 실제_app은_끄는_동안_drop_안내를_그리고_프로젝트_없이_놓은_파일은_안내한다() {
    let (mut fixture, root) = fixture("overlay");
    let loose = root.join("loose.txt");
    std::fs::write(&loose, "loose").unwrap();
    let state = isolated_state(fixture.directory.join("data"));
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let label = presentation::message(&application.locale, "app.dropToOpen", &[]);
    run(application, &context, input(&[], &[]));

    let output = run(application, &context, input(&[&loose], &[]));
    assert!(has_text(&output, &label), "status={:?}", application.status);
    let background = application
        .file_drop_colors
        .background
        .gamma_multiply(OVERLAY_OPACITY);
    assert!(output.shapes.iter().any(|clipped| matches!(
        &clipped.shape,
        egui::Shape::Rect(shape) if shape.rect == screen() && shape.fill == background
    )));
    let accent = application.file_drop_colors.accent;
    let dashes = output
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(
                &clipped.shape,
                egui::Shape::LineSegment { stroke, .. }
                    if stroke.width == OVERLAY_BORDER && stroke.color == accent
            )
        })
        .count();
    assert!(dashes > 4, "dashes={dashes}");

    let output = run(application, &context, input(&[&loose], &[]));
    assert!(has_text(&output, &label));
    let output = run(application, &context, input(&[], &[]));
    assert!(!has_text(&output, &label));

    let output = run(application, &context, input(&[], &[&loose]));
    assert!(!has_text(&output, &label));
    let notice = presentation::message(&application.locale, "app.openProjectFirst", &[]);
    frame_until(application, &context, "open project first", |application| {
        application.toasts.is_showing(Kind::Info, &notice)
    });
    assert!(application.services.state.projects.read().is_empty());
}

#[test]
fn 실제_app은_놓은_폴더를_프로젝트로_열고_뒤따르는_파일은_그_프로젝트의_preview_탭으로_연다() {
    let (mut fixture, root) = fixture("open");
    let alpha = root.join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    let notes = alpha.join("notes.md");
    std::fs::write(&notes, "# notes").unwrap();
    let todo = alpha.join("todo.md");
    std::fs::write(&todo, "- todo").unwrap();
    let outside = root.join("outside.txt");
    std::fs::write(&outside, "outside").unwrap();
    let state = isolated_state(fixture.directory.join("data"));
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    run(application, &context, input(&[], &[]));

    run(application, &context, input(&[], &[&alpha, &notes]));
    let notes_path = notes.to_str().unwrap().to_owned();
    frame_until(application, &context, "folder then file", |application| {
        project_at(application, &alpha)
            .is_some_and(|project| file_tab(application, &project, &notes_path).is_some())
    });
    let project = project_at(application, &alpha).unwrap();
    assert!(
        file_tab(application, &project, &notes_path)
            .unwrap()
            .preview
    );
    frame_until(
        application,
        &context,
        "opened project focus",
        |application| application.controller.snapshot().focused_project() == Some(&project),
    );

    let todo_path = todo.to_str().unwrap().to_owned();
    run(application, &context, input(&[], &[&todo]));
    frame_until(
        application,
        &context,
        "file into focused project",
        |application| file_tab(application, &project, &todo_path).is_some_and(|tab| tab.preview),
    );

    run(application, &context, input(&[], &[&outside]));
    frame_until(application, &context, "outside file error", |application| {
        application
            .toasts
            .inspection()
            .iter()
            .any(|toast| toast.kind == Kind::Error)
    });
    assert!(file_tab(application, &project, outside.to_str().unwrap()).is_none());
    assert_eq!(application.services.state.projects.read().len(), 1);
}

#[test]
fn 실제_app은_미리보기_web_view가_받은_drag에도_안내를_그리고_놓은_폴더를_프로젝트로_연다() {
    let (mut fixture, root) = fixture("web");
    let alpha = root.join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    let state = isolated_state(fixture.directory.join("data"));
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let label = presentation::message(&application.locale, "app.dropToOpen", &[]);
    run(application, &context, input(&[], &[]));

    let preview = TabId::new();
    let drags = application.web_views.drags().clone();
    assert!(drags.enter(&preview, std::slice::from_ref(&alpha)));
    let output = run(application, &context, input(&[], &[]));
    assert!(has_text(&output, &label), "status={:?}", application.status);
    assert!(drags.drop_paths(&preview, std::slice::from_ref(&alpha)));
    let output = run(application, &context, input(&[], &[]));
    assert!(!has_text(&output, &label));
    assert!(drags.take_dropped().is_empty());
    frame_until(application, &context, "web view drop", |application| {
        project_at(application, &alpha).is_some()
    });
}
