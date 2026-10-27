use super::project_rail_tests::{Fixture, insert_project, isolated_state, launch, project_ref};
use super::*;
use taide_model::layout::{AuxWindowLayout, PaneNode, SplitDir};
use taide_model::project::{ShellSlotTree, WindowChromePatch};

const DEADLINE: Duration = Duration::from_secs(10);
const MAIN_SIZE: [f32; 2] = [1180.0, 760.0];
const AUXILIARY_SIZE: [f32; 2] = [900.0, 600.0];
const WINDOW_ORIGIN: [f32; 2] = [64.0, 48.0];
const AUXILIARY_SLOT: u32 = 1;
const OBSERVED_FRAMES: usize = 4;
const RESIZED_SLOTS: [f32; 2] = [35.0, 65.0];
const EVEN_SLOTS: [f32; 2] = [50.0, 50.0];
const NOTES_FILE: &str = "notes.txt";

fn window_info(size: [f32; 2]) -> egui::ViewportInfo {
    let rect = egui::Rect::from_min_size(
        egui::pos2(WINDOW_ORIGIN[0], WINDOW_ORIGIN[1]),
        egui::vec2(size[0], size[1]),
    );
    egui::ViewportInfo {
        inner_rect: Some(rect),
        outer_rect: Some(rect),
        minimized: Some(false),
        maximized: Some(false),
        fullscreen: Some(false),
        ..Default::default()
    }
}

fn frame(
    application: &mut NativeApplication,
    context: &egui::Context,
    root: Option<&egui::ViewportInfo>,
) -> egui::FullOutput {
    eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(MAIN_SIZE[0], MAIN_SIZE[1]),
        )),
        ..Default::default()
    };
    if let Some(root) = root {
        input.viewports = std::iter::once((egui::ViewportId::ROOT, root.clone())).collect();
    }
    let mut output = context.run_ui(input, |ui| {
        eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest())
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
        frame(application, context, None);
        assert!(
            Instant::now() < deadline,
            "phase={phase} status={:?}",
            application.status
        );
        std::thread::yield_now();
    }
}

fn render_auxiliary_viewports(context: &egui::Context, info: egui::ViewportInfo) {
    context.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(move |context, mut viewport| {
        let id = viewport.ids.this;
        let mut output = context.run_ui(
            egui::RawInput {
                viewport_id: id,
                viewports: std::iter::once((id, info.clone())).collect(),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(AUXILIARY_SIZE[0], AUXILIARY_SIZE[1]),
                )),
                ..Default::default()
            },
            |ui| (viewport.viewport_ui_cb)(ui),
        );
        output.textures_delta.clear();
    });
}

fn auxiliary_window(root: &std::path::Path) -> AuxWindowLayout {
    let pane = PaneId::new();
    let tab = Tab {
        id: TabId::new(),
        kind: TabKind::File {
            path: root.join(NOTES_FILE).to_str().unwrap().into(),
        },
        title: NOTES_FILE.into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    AuxWindowLayout {
        slot: AUXILIARY_SLOT,
        root: PaneNode::Leaf {
            id: pane.clone(),
            active: Some(tab.id.clone()),
            tabs: vec![tab],
        },
        focused_pane: pane,
    }
}

fn saved_window_states(
    data: &std::path::Path,
) -> std::collections::BTreeMap<String, crate::window_state::WindowState> {
    serde_json::from_slice(&std::fs::read(crate::window_state::path(data)).unwrap()).unwrap()
}

#[test]
fn 실제_app은_메인_창_크기만_추적해_종료때_data_dir에_저장하고_보조_창은_원본_기본_크기로_연다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-window-state-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(root.join(NOTES_FILE), "synthetic\n").unwrap();
    let alpha = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data.clone());
    insert_project(&state, &alpha, &root, "alpha", false);
    state
        .layouts
        .write()
        .get_mut(&alpha)
        .unwrap()
        .auxiliary_windows = vec![auxiliary_window(&root)];
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&alpha, &root, "alpha")];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: alpha.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    render_auxiliary_viewports(&context, window_info(AUXILIARY_SIZE));
    let application = fixture.application.as_mut().unwrap();
    let auxiliary = application
        .auxiliary_windows
        .find(&alpha, AUXILIARY_SLOT)
        .expect("the persisted auxiliary window is restored")
        .viewport;

    let main = window_info(MAIN_SIZE);
    let mut output = frame(application, &context, Some(&main));
    for _ in 0..OBSERVED_FRAMES {
        output = frame(application, &context, Some(&main));
    }
    let auxiliary_builder = &output
        .viewport_output
        .get(&auxiliary)
        .expect("the root pass requests the auxiliary viewport")
        .builder;
    assert_eq!(
        auxiliary_builder.inner_size,
        Some(egui::vec2(
            crate::auxiliary_windows::DEFAULT_SIZE[0],
            crate::auxiliary_windows::DEFAULT_SIZE[1]
        ))
    );
    assert_eq!(auxiliary_builder.position, None);
    assert!(!crate::window_state::path(&data).exists());

    let mut application = fixture.application.take().unwrap();
    eframe::App::on_exit(&mut application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
    drop(application);

    let saved = saved_window_states(&data);
    assert_eq!(
        saved.keys().map(String::as_str).collect::<Vec<_>>(),
        vec![crate::window_state::MAIN_LABEL]
    );
    assert_eq!(
        saved[crate::window_state::MAIN_LABEL],
        crate::window_state::WindowState {
            width: MAIN_SIZE[0],
            height: MAIN_SIZE[1],
            maximized: false,
            fullscreen: false,
        }
    );
    let restored = crate::window_state::main_viewport(egui::ViewportBuilder::default(), &data);
    assert_eq!(
        restored.inner_size,
        Some(egui::vec2(MAIN_SIZE[0], MAIN_SIZE[1]))
    );
    assert_eq!(restored.position, None);
}

#[test]
fn 실제_app_재시작은_프로젝트_slot_트리_zen_보조_창과_메인_창_크기를_디스크에서_복원한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-session-restore-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let alpha_root = fixture.directory.join("alpha");
    let beta_root = fixture.directory.join("beta");
    for directory in [&data, &alpha_root, &beta_root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(alpha_root.join(NOTES_FILE), "synthetic\n").unwrap();
    let alpha = ProjectId::new();
    let beta = ProjectId::new();
    let alpha_slot = ShellSlotId::new();
    let beta_slot = ShellSlotId::new();
    let state = isolated_state(data.clone());
    insert_project(&state, &alpha, &alpha_root, "alpha", false);
    insert_project(&state, &beta, &beta_root, "beta", false);
    state
        .layouts
        .write()
        .get_mut(&alpha)
        .unwrap()
        .auxiliary_windows = vec![auxiliary_window(&alpha_root)];
    state
        .dirty_layouts
        .write()
        .extend([alpha.clone(), beta.clone()]);
    {
        let mut session = state.session.write();
        session.projects = vec![
            project_ref(&alpha, &alpha_root, "alpha"),
            project_ref(&beta, &beta_root, "beta"),
        ];
        session.active_project = Some(alpha.clone());
        session.shell_slots = Some(ShellSlotTree::Split {
            dir: SplitDir::Horizontal,
            children: vec![
                ShellSlotTree::Leaf {
                    slot_id: alpha_slot.clone(),
                    project_id: alpha.clone(),
                },
                ShellSlotTree::Leaf {
                    slot_id: beta_slot.clone(),
                    project_id: beta.clone(),
                },
            ],
            sizes: EVEN_SLOTS.to_vec(),
        });
        session.focused_shell_slot = Some(alpha_slot.clone());
    }
    let context = launch(&mut fixture, state);
    render_auxiliary_viewports(&context, window_info(AUXILIARY_SIZE));
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    for mutation in [
        ShellMutation::ResizeSlots {
            path: Vec::new(),
            sizes: RESIZED_SLOTS.to_vec(),
        },
        ShellMutation::FocusSlot(beta_slot.clone()),
        ShellMutation::SetWindowChrome(WindowChromePatch {
            zen: Some(true),
            sidebar_rail_collapsed: Some(true),
        }),
    ] {
        application.controller.submit(mutation).unwrap();
    }
    frame_until(application, &context, "session mutations", |_| {
        let session = services.state.session.read();
        session.window_chrome.zen
            && session.focused_shell_slot.as_ref() == Some(&beta_slot)
            && matches!(&session.shell_slots, Some(ShellSlotTree::Split { sizes, .. }) if sizes == &RESIZED_SLOTS.to_vec())
    });
    let main = window_info(MAIN_SIZE);
    for _ in 0..OBSERVED_FRAMES {
        frame(application, &context, Some(&main));
    }
    let helper = application.helper_executable.clone();
    let mut application = fixture.application.take().unwrap();
    eframe::App::on_exit(&mut application);
    assert_eq!(services.tasks.tracked_count(), 0);
    drop(services);
    drop(application);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let (state, warnings) = runtime
        .block_on(crate::bootstrap::restore(
            crate::bootstrap::LaunchConfig {
                data_dir: data.clone(),
            },
            &tasks,
        ))
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let restored = crate::window_state::main_viewport(egui::ViewportBuilder::default(), &data);
    assert_eq!(
        restored.inner_size,
        Some(egui::vec2(MAIN_SIZE[0], MAIN_SIZE[1]))
    );
    let context = egui::Context::default();
    render_auxiliary_viewports(&context, window_info(AUXILIARY_SIZE));
    fixture.application = Some(
        NativeApplication::new(
            &eframe::CreationContext::_new_kittest(context.clone()),
            runtime,
            state,
            tasks,
            helper,
            warnings,
        )
        .unwrap(),
    );
    let application = fixture.application.as_mut().unwrap();
    frame(application, &context, None);
    {
        let session = application.services.state.session.read();
        assert_eq!(
            session
                .projects
                .iter()
                .map(|project| project.id.clone())
                .collect::<Vec<_>>(),
            vec![alpha.clone(), beta.clone()]
        );
        assert_eq!(session.active_project.as_ref(), Some(&beta));
        assert_eq!(session.focused_shell_slot.as_ref(), Some(&beta_slot));
        assert!(session.window_chrome.zen);
        assert!(session.window_chrome.sidebar_rail_collapsed);
        let Some(ShellSlotTree::Split {
            children, sizes, ..
        }) = &session.shell_slots
        else {
            panic!("the split slot tree is restored");
        };
        assert_eq!(sizes, &RESIZED_SLOTS.to_vec());
        assert_eq!(
            children,
            &vec![
                ShellSlotTree::Leaf {
                    slot_id: alpha_slot,
                    project_id: alpha.clone(),
                },
                ShellSlotTree::Leaf {
                    slot_id: beta_slot,
                    project_id: beta.clone(),
                },
            ]
        );
    }
    let snapshot = application.controller.snapshot();
    assert!(snapshot.shell.window_chrome.zen);
    assert_eq!(snapshot.focused_project(), Some(&beta));
    assert_eq!(
        application
            .services
            .windows
            .label_for(&alpha, AUXILIARY_SLOT)
            .as_deref(),
        Some("editor-1")
    );
    assert!(
        application
            .auxiliary_windows
            .find(&alpha, AUXILIARY_SLOT)
            .is_some()
    );
    let mut application = fixture.application.take().unwrap();
    eframe::App::on_exit(&mut application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
    assert_eq!(
        crate::window_state::load(&data).map(|state| [state.width, state.height]),
        Some(MAIN_SIZE)
    );
}
