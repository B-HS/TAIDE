use super::*;

use taide_model::{app::AppFileTarget, paths::AppPaths, project::Project, settings::Settings};
use taide_native_editor::{
    document::{Edit, UndoGroup},
    store::Transaction,
};

const DEADLINE: Duration = Duration::from_secs(10);
const SAVED_FONT: u32 = 19;
const INDEX: &[u8] = b"<!doctype html><title>synthetic Rust remote bundle</title>";
const MODULE: &[u8] = b"\0asm\x01\0\0\0";

struct Fixture {
    directory: PathBuf,
    application: Option<NativeApplication>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(application) = self.application.as_mut() {
            eframe::App::on_exit(application);
        }
        drop(self.application.take());
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn wait_for(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let runtime = application.runtime.handle().clone();
    let result = runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
                if ready(application) {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
    });
    assert!(
        result.is_ok(),
        "phase={phase}; remote={}; shutting_down={}; closing={}; host={}; tasks={}; status={:?}",
        application.services.remote.is_running(),
        application.services.state.is_shutting_down(),
        application.closing.is_some(),
        application.bridge.is_some(),
        application.services.tasks.tracked_count(),
        application.status
    );
}

#[test]
fn actual_app_constructor는_bundle_startup_settings_file_save와_정상_exit_owner를_연결한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-actual-app-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let project_root = fixture.directory.join("project");
    let public = fixture.directory.join("bin/remote-public");
    for directory in [&data, &project_root, &public] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(public.join("index.html"), INDEX).unwrap();
    std::fs::write(public.join("app.wasm"), MODULE).unwrap();
    std::fs::write(
        public.join("bundle-manifest.json"),
        serde_json::to_vec(&serde_json::json!({
            "format":"taide-rust-remote-v1","entryWasm":"app.wasm","files":["index.html","app.wasm"]
        }))
        .unwrap(),
    )
    .unwrap();
    let state = AppState::new(AppPaths::new(data));
    {
        let mut settings = state.settings.write();
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
        settings.remote_access_enabled = true;
        std::fs::write(
            state.paths.settings_file(),
            serde_json::to_vec(&*settings).unwrap(),
        )
        .unwrap();
    }
    let owner = crate::app_file::Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
        target: AppFileTarget::Settings,
    };
    state.projects.write().insert(
        owner.project.clone(),
        Project {
            id: owner.project.clone(),
            root: project_root.to_str().unwrap().into(),
            name: "synthetic actual App".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let mut layout = taide_layout::service::default_layout();
    layout.root = taide_model::layout::PaneNode::Leaf {
        id: owner.pane.clone(),
        tabs: vec![Tab {
            id: owner.tab.clone(),
            kind: TabKind::AppFile {
                target: owner.target,
            },
            title: "settings.json".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(owner.tab.clone()),
    };
    layout.focused_pane = owner.pane.clone();
    state.layouts.write().insert(owner.project.clone(), layout);
    state.session.write().active_project = Some(owner.project.clone());
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let context = egui::Context::default();
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
    let application = fixture.application.as_mut().unwrap();
    let services = application.services.clone();
    let ports = Arc::downgrade(&application.application_ports);
    let hub = Arc::downgrade(application.terminals.hub());
    assert_eq!(
        (application.application_ports.remote.assets)("index.html")
            .unwrap()
            .bytes,
        INDEX
    );
    assert!(
        services.remote.is_running(),
        "prepared bundle did not start the loopback remote server"
    );
    assert!(!services.ide.is_running());
    assert!(services.agent_hooks.server_info().is_none());
    assert!((application.application_ports.remote.assets)("bundle-manifest.json").is_none());
    let read = application
        .app_file_views
        .begin(&owner, &application.store, true)
        .unwrap();
    assert!(application.submit(HostCommand::ReadAppFile(read)));
    wait_for(application, &context, "app-file-read", |application| {
        application.app_file_views.document(&owner).is_some()
    });
    let document = application.app_file_views.document(&owner).unwrap();
    let mut settings = services.state.settings.read().clone();
    settings.remote_access_enabled = false;
    settings.editor_font_size = SAVED_FONT;
    let snapshot = application.store.documents().snapshot(document).unwrap();
    application
        .store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text: serde_json::to_string(&settings).unwrap(),
                }],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
    application.app_file_views.changed(document);
    application.request_tab_save(
        owner.tab.clone(),
        Some(document),
        &eframe::Frame::_new_kittest(),
    );
    wait_for(application, &context, "app-file-save", |application| {
        !services.remote.is_running()
            && services.state.settings.read().editor_font_size == SAVED_FONT
            && !application
                .store
                .documents()
                .snapshot(document)
                .unwrap()
                .dirty
    });
    let saved: Settings =
        serde_json::from_slice(&std::fs::read(services.state.paths.settings_file()).unwrap())
            .unwrap();
    assert_eq!(saved, *services.state.settings.read());
    assert_eq!(application.pending_dirty.get(&owner.tab), Some(&false));
    application.close(&context);
    wait_for(application, &context, "app-exit", |application| {
        application.is_exit_ready
    });
    assert!(services.state.is_shutting_down());
    assert_eq!(services.tasks.tracked_count(), 0);
    eframe::App::on_exit(application);
    drop(fixture.application.take());
    assert!(ports.upgrade().is_none());
    assert!(hub.upgrade().is_none());
}

#[test]
fn actual_app의_파일본문과_아웃라인은_같은_문서_심볼과_현재_pane_caret를_소비한다() {
    const SCREEN_WIDTH: f32 = 1000.0;
    const SCREEN_HEIGHT: f32 = 700.0;
    const CARET: usize = 8;
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-navigation-app-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("project");
    let public = fixture.directory.join("bin/remote-public");
    for directory in [&data, &root, &public] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(public.join("index.html"), INDEX).unwrap();
    std::fs::write(public.join("app.wasm"), MODULE).unwrap();
    let file = root.join("current.rs");
    std::fs::write(&file, "class\n  method\nend").unwrap();
    let path = file.canonicalize().unwrap().to_str().unwrap().to_owned();
    let root = root.canonicalize().unwrap().to_str().unwrap().to_owned();
    let state = AppState::new(AppPaths::new(data));
    {
        let mut settings = state.settings.write();
        settings.remote_access_enabled = false;
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
    }
    let project = ProjectId::new();
    let pane = PaneId::new();
    let tab = TabId::new();
    let slot = ShellSlotId::new();
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.clone(),
            name: "synthetic navigation".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let mut layout = taide_layout::service::default_layout();
    layout.root = taide_model::layout::PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: TabKind::File { path: path.clone() },
            title: "current.rs".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(tab.clone()),
    };
    layout.focused_pane = pane.clone();
    state.layouts.write().insert(project.clone(), layout);
    {
        let mut session = state.session.write();
        session.projects = vec![taide_model::project::ProjectRef {
            id: project.clone(),
            root,
            name: "synthetic navigation".into(),
            display: Default::default(),
            root_missing: false,
        }];
        session.active_project = Some(project.clone());
        session.focused_shell_slot = Some(slot.clone());
        session.shell_slots = Some(taide_model::project::ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: project.clone(),
        });
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let context = egui::Context::default();
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
    let application = fixture.application.as_mut().unwrap();
    let paint = |application: &mut NativeApplication| {
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
                )),
                ..Default::default()
            },
            |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
        );
        output.textures_delta.clear();
        output
    };
    paint(application);
    wait_for(
        application,
        &context,
        "navigation-document-read",
        |application| application.files.contains_key(&path),
    );
    let document = application.files[&path].id;
    let snapshot = application.store.documents().snapshot(document).unwrap();
    let response = serde_json::from_value(serde_json::json!([{"name":"Outer","kind":5,"range":{"start":{"line":0,"character":0},"end":{"line":2,"character":3}},"selectionRange":{"start":{"line":0,"character":0},"end":{"line":0,"character":5}},"children":[{"name":"method","kind":6,"range":{"start":{"line":1,"character":2},"end":{"line":1,"character":8}},"selectionRange":{"start":{"line":1,"character":2},"end":{"line":1,"character":8}}}]}])).unwrap();
    let model = Arc::new(
        taide_native_editor::document_symbols::DocumentSymbols::new(
            &snapshot,
            &taide_lsp::service::workspace_folder_uri(&path)
                .parse()
                .unwrap(),
            Some(response),
        )
        .unwrap(),
    );
    application.editor_symbols = crate::editor_symbols::State::default();
    let request = application
        .editor_symbols
        .observe(&project, snapshot.clone(), HashSet::new(), Instant::now())
        .unwrap();
    assert!(
        application
            .editor_symbols
            .accept(
                &request,
                &snapshot,
                HashSet::new(),
                Ok(crate::editor_symbols::Response {
                    palette_provider: None,
                    palette: model,
                    groups: Vec::new(),
                    error: None
                })
            )
            .unwrap()
    );
    let view = application
        .store
        .attach_view(
            ViewKey {
                window: WINDOW_LABEL.into(),
                pane,
                tab,
            },
            document,
        )
        .unwrap();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: CARET,
                    head: CARET,
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    application.symbol_sidebars.entry(&project, &slot).view = crate::symbol_sidebar::View::Outline;
    let output = paint(application);
    let labels = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        labels.iter().filter(|text| **text == "Outer").count() >= 2,
        "labels={labels:?}"
    );
    assert!(
        labels.iter().filter(|text| **text == "method").count() >= 2,
        "labels={labels:?}"
    );
    assert!(labels.iter().any(|text| *text == "current.rs"));
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    application.close(&context);
    wait_for(application, &context, "navigation-exit", |application| {
        application.is_exit_ready
    });
    assert_eq!(application.services.tasks.tracked_count(), 0);
    eframe::App::on_exit(application);
    drop(fixture.application.take());
}
