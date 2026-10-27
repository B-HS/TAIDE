use super::project_rail_tests::{Fixture, isolated_state, launch};
use super::*;
use crate::app_menu::{
    Bar, CLEAR_RECENT_ID, COPY_ID, Model, PASTE_ID, QUIT_ID, REDO_ID, Row, SELECT_ALL_ID, UNDO_ID,
    recent_id,
};
use taide_model::project::Project;
use taide_native_ui::toast::Kind;

const DEADLINE: Duration = Duration::from_secs(10);
const SCREEN: [f32; 2] = [1000.0, 700.0];
const NEWEST: f64 = 3.0;
const MIDDLE: f64 = 2.0;
const OLDEST: f64 = 1.0;

#[derive(Clone, Default)]
pub(super) struct Recording(Arc<std::sync::Mutex<Vec<Model>>>);

impl Bar for Recording {
    fn apply(&mut self, model: &Model) {
        self.0.lock().unwrap().push(model.clone());
    }
}

impl Recording {
    fn last(&self) -> Option<Model> {
        self.0.lock().unwrap().last().cloned()
    }
}

fn hooked_input(
    application: &mut NativeApplication,
    context: &egui::Context,
    root: egui::ViewportInfo,
) -> egui::RawInput {
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(SCREEN[0], SCREEN[1]),
        )),
        viewports: std::iter::once((egui::ViewportId::ROOT, root)).collect(),
        ..Default::default()
    };
    eframe::App::raw_input_hook(application, context, &mut input);
    input
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

fn frame(application: &mut NativeApplication, context: &egui::Context) -> egui::FullOutput {
    let input = hooked_input(application, context, Default::default());
    run(application, context, input)
}

fn frame_until(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let deadline = Instant::now() + DEADLINE;
    while !ready(application) {
        frame(application, context);
        assert!(
            Instant::now() < deadline,
            "phase={phase} status={:?}",
            application.status
        );
        std::thread::yield_now();
    }
}

fn root_commands(output: &egui::FullOutput) -> &[egui::ViewportCommand] {
    output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .map_or(&[], |viewport| viewport.commands.as_slice())
}

fn lists(model: &Model, project: &ProjectId) -> bool {
    let id = recent_id(project);
    model
        .recent
        .iter()
        .any(|row| matches!(row, Row::Item { id: row_id, .. } if *row_id == id))
}

#[test]
fn 실제_app_메뉴는_최근_목록을_그리고_열기_지우기_편집_종료를_원본_경로로_보낸다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-native-app-menu-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let paths = taide_model::paths::AppPaths::new(data.clone());
    let state = isolated_state(data);
    let record = |name: &str, last_opened_at: f64| {
        let root = fixture.directory.join(name);
        std::fs::create_dir_all(&root).unwrap();
        let project = Project {
            id: ProjectId::new(),
            root: root.canonicalize().unwrap().to_str().unwrap().into(),
            name: name.into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at,
            display: Default::default(),
        };
        let file = state.paths.project_file(&project.id);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, serde_json::to_vec(&project).unwrap()).unwrap();
        project
    };
    let reopened = record("reopened", NEWEST);
    let drafted = record("drafted", MIDDLE);
    let forgotten = record("forgotten", OLDEST);
    let buffers = state.paths.buffers_dir(&drafted.id);
    std::fs::create_dir_all(&buffers).unwrap();
    std::fs::write(buffers.join("synthetic.json"), "{}").unwrap();
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let bar = Recording::default();
    let (sender, receiver) = std::sync::mpsc::channel();
    application
        .app_menu
        .connect(Box::new(bar.clone()), receiver);
    let initial = Model::new(
        &application.locale,
        &[reopened.clone(), drafted.clone(), forgotten.clone()],
    );
    frame_until(application, &context, "initial recent menu", |_| {
        bar.last().as_ref() == Some(&initial)
    });

    sender.send(recent_id(&reopened.id)).unwrap();
    frame_until(application, &context, "open recent", |application| {
        application
            .services
            .state
            .projects
            .read()
            .contains_key(&reopened.id)
    });

    sender.send(CLEAR_RECENT_ID.into()).unwrap();
    let kept = presentation::message(
        &application.locale,
        "project.clearRecentKeptDrafts",
        &[("count", "1")],
    );
    frame_until(application, &context, "clear recent", |application| {
        application.toasts.is_showing(Kind::Info, &kept)
    });
    assert!(!paths.project_dir(&forgotten.id).exists());
    assert!(paths.project_file(&drafted.id).exists());
    assert!(paths.project_file(&reopened.id).exists());
    frame_until(application, &context, "cleared recent menu", |_| {
        bar.last().is_some_and(|model| {
            lists(&model, &reopened.id)
                && lists(&model, &drafted.id)
                && !lists(&model, &forgotten.id)
        })
    });

    for (id, command) in [
        (COPY_ID, egui::ViewportCommand::RequestCopy),
        (PASTE_ID, egui::ViewportCommand::RequestPaste),
    ] {
        sender.send(id.into()).unwrap();
        let output = frame(application, &context);
        assert!(root_commands(&output).contains(&command), "{id}");
    }
    let command = if cfg!(target_os = "macos") {
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
    } else {
        egui::Modifiers::CTRL | egui::Modifiers::COMMAND
    };
    for (id, key, modifiers) in [
        (UNDO_ID, egui::Key::Z, command),
        (REDO_ID, egui::Key::Z, command | egui::Modifiers::SHIFT),
        (SELECT_ALL_ID, egui::Key::A, command),
    ] {
        sender.send(id.into()).unwrap();
        frame(application, &context);
        let input = hooked_input(application, &context, Default::default());
        assert!(
            input.events.contains(&egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers,
            }),
            "{id}: {:?}",
            input.events
        );
        run(application, &context, input);
        let input = hooked_input(application, &context, Default::default());
        assert!(
            !input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Key { .. })),
            "{id} is delivered once"
        );
        run(application, &context, input);
    }

    sender.send(QUIT_ID.into()).unwrap();
    let output = frame(application, &context);
    assert!(root_commands(&output).contains(&egui::ViewportCommand::Close));
    assert!(application.closing.is_none() && !application.is_exit_ready);
    let close = egui::ViewportInfo {
        events: vec![egui::ViewportEvent::Close],
        ..Default::default()
    };
    let input = hooked_input(application, &context, close);
    let output = run(application, &context, input);
    assert!(root_commands(&output).contains(&egui::ViewportCommand::CancelClose));
    frame_until(application, &context, "quit handshake", |application| {
        application.is_exit_ready
    });
}
