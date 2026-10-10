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
    ready: impl Fn(&mut NativeApplication) -> bool,
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
fn actual_app의_문서_선택_peek와_저장_포맷은_현재_편집_폭과_소유를_보존한다() {
    use std::os::unix::fs::PermissionsExt;
    use taide_native_editor::indent::{IndentOptions, IndentationChange};
    const EDIT_SIZE: u32 = 4;
    const CHANGED_SIZE: u32 = 2;
    const DISPLAY_SIZE: u32 = 8;
    const EXECUTABLE_MODE: u32 = 0o700;
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!(
            "taide-native-format-options-app-{}",
            ProjectId::new()
        )),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("project");
    let bin = fixture.directory.join("bin");
    for directory in [&data, &root, &bin] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let file = root.join("current.rs");
    std::fs::write(&file, "input").unwrap();
    let path = file.canonicalize().unwrap().to_str().unwrap().to_owned();
    let root = root.canonicalize().unwrap().to_str().unwrap().to_owned();
    let state = AppState::new(AppPaths::new(data));
    {
        let mut settings = state.settings.write();
        settings.remote_access_enabled = false;
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
        settings.format_on_save = true;
        settings.editor_tab_size = EDIT_SIZE;
        settings.editor_insert_spaces = true;
        settings.editor_detect_indentation = false;
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
            name: "synthetic formatting".into(),
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
    layout.focused_pane = pane;
    state.layouts.write().insert(project.clone(), layout);
    {
        let mut session = state.session.write();
        session.projects = vec![taide_model::project::ProjectRef {
            id: project.clone(),
            root,
            name: "synthetic formatting".into(),
            display: Default::default(),
            root_missing: false,
        }];
        session.active_project = Some(project.clone());
        session.focused_shell_slot = Some(slot.clone());
        session.shell_slots = Some(taide_model::project::ShellSlotTree::Leaf {
            slot_id: slot,
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
            bin.join("taide"),
            Vec::new(),
        )
        .unwrap(),
    );
    let application = fixture.application.as_mut().unwrap();
    let mut output = context.run_ui(egui::RawInput::default(), |ui| {
        eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest())
    });
    output.textures_delta.clear();
    wait_for(
        application,
        &context,
        "format-document-read",
        |application| application.files.contains_key(&path),
    );
    let document = application.files[&path].id;
    let mock = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/native-lsp-mock");
    assert!(mock.is_file());
    let mock = mock.to_str().unwrap().replace('\'', "'\\''");
    let executable = bin.join("rust-analyzer");
    std::fs::write(
        &executable,
        format!("#!/bin/sh\nexec '{mock}' --native-format-options\n"),
    )
    .unwrap();
    std::fs::set_permissions(
        &executable,
        std::fs::Permissions::from_mode(EXECUTABLE_MODE),
    )
    .unwrap();
    let runtime = application.runtime.handle().clone();
    runtime
        .block_on(application.lsp.take().unwrap().disconnect())
        .unwrap();
    let repaint = context.clone();
    application.lsp = Some(
        crate::lsp::LspBridge::connect(
            application.services.clone(),
            bin.into_os_string(),
            Arc::new(move || repaint.request_repaint()),
        )
        .unwrap(),
    );
    let configuration = crate::presentation_refresh::indent_configuration(
        &application.services.state.settings.read(),
    );
    for (size, spaces) in [(EDIT_SIZE, true), (CHANGED_SIZE, false)] {
        let change = if spaces {
            IndentationChange::UseSpaces(size)
        } else {
            IndentationChange::UseTabs(size)
        };
        application
            .store
            .set_indentation(document, configuration, change)
            .unwrap();
        application
            .store
            .set_indentation(
                document,
                configuration,
                IndentationChange::DisplaySize(DISPLAY_SIZE),
            )
            .unwrap();
        let before = application.store.documents().snapshot(document).unwrap();
        let input = format!("{}!", before.rope);
        application
            .store
            .apply(
                document,
                Transaction {
                    revision: before.revision,
                    edits: vec![Edit {
                        bytes: before.rope.len_bytes()..before.rope.len_bytes(),
                        text: "!".into(),
                    }],
                    group: UndoGroup(0),
                    origin: None,
                    selection_after: None,
                },
            )
            .unwrap();
        application.request_tab_save(tab.clone(), Some(document), &eframe::Frame::_new_kittest());
        wait_for(
            application,
            &context,
            "format-document-save",
            |application| {
                !application
                    .store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .dirty
            },
        );
        let saved = application.store.documents().snapshot(document).unwrap();
        assert_eq!(
            saved.rope.to_string(),
            format!("formatted:{size}:{spaces}:{input}")
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            saved.rope.to_string()
        );
        let options = saved.model_indentation(IndentOptions {
            tab_size: EDIT_SIZE,
            insert_spaces: true,
        });
        assert_eq!(
            (options.tab_size, options.indent_size, options.insert_spaces),
            (DISPLAY_SIZE, size, spaces)
        );
    }
    let paint = |application: &mut NativeApplication, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 700.0),
                )),
                events,
                ..Default::default()
            },
            |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
        );
        output.textures_delta.clear();
    };
    let key = |key, modifiers| egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    };
    paint(application, Vec::new());
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == tab)
        .unwrap()
        .id;
    let focus = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == egui::ViewportId::ROOT && *target == view).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    let saved = application.store.documents().snapshot(document).unwrap();
    let disk = std::fs::read_to_string(&file).unwrap();
    paint(
        application,
        vec![key(
            egui::Key::F,
            egui::Modifiers::SHIFT | egui::Modifiers::ALT,
        )],
    );
    let expected = format!("formatted:2:false:{}", saved.rope);
    wait_for(
        application,
        &context,
        "actual-format-document-key",
        |application| {
            application
                .store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string()
                == expected
        },
    );
    assert!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), disk);
    assert!(application.store.undo(document).unwrap());
    let restored = application.store.documents().snapshot(document).unwrap();
    assert_eq!(restored.rope, saved.rope);
    assert!(restored.dirty);
    let input = restored.rope.to_string().find("input").unwrap();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: input,
                    head: input + "input".len(),
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    paint(application, Vec::new());
    context.memory_mut(|memory| memory.request_focus(focus));
    let command = egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND;
    paint(application, vec![key(egui::Key::K, command)]);
    paint(application, vec![key(egui::Key::F, command)]);
    let expected = restored
        .rope
        .to_string()
        .replacen("input", "range:2:false:input", 1);
    wait_for(
        application,
        &context,
        "actual-format-selection-chord",
        |application| {
            application
                .store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string()
                == expected
        },
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), disk);
    assert!(application.store.undo(document).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope,
        saved.rope
    );

    let peek_file = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .join("peek.rs");
    let peek_text = "한\u{1f600}name";
    std::fs::write(&peek_file, peek_text).unwrap();
    let peek_path = peek_file.to_str().unwrap().to_owned();
    let providers = application.lsp.as_ref().unwrap().formatting_providers(
        &project,
        &application.store.documents().snapshot(document).unwrap(),
        taide_native_editor::formatting::Command::Document,
    );
    let identity = *providers.iter().next().unwrap();
    let request = application
        .editor_locations
        .begin(
            project.clone(),
            &application.store,
            view,
            taide_native_editor::symbol_locations::Kind::Definition,
            taide_native_editor::symbol_locations::Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    let range = taide_native_editor::lsp::LspRange::new(
        taide_native_editor::lsp::Position::new(0, 3),
        taide_native_editor::lsp::Position::new(0, 7),
    );
    application
        .editor_locations
        .accept(
            &request,
            &application.store,
            providers,
            Ok(crate::editor_locations::Response {
                groups: vec![crate::editor_locations::Group {
                    provider: identity,
                    targets: vec![taide_native_editor::symbol_locations::Target {
                        uri: taide_lsp::service::workspace_folder_uri(&peek_path)
                            .parse()
                            .unwrap(),
                        range,
                        selection: range,
                        origin: None,
                    }],
                }],
            }),
        )
        .unwrap()
        .unwrap();
    application.editor_locations.show(view);
    paint(application, Vec::new());
    let peek_key = taide_native_editor::document::DocumentKey::File(peek_path.clone().into());
    wait_for(
        application,
        &context,
        "actual-format-peek-load",
        |application| application.store.documents().find(&peek_key).is_some(),
    );
    paint(application, Vec::new());
    let peek_document = application.store.documents().find(&peek_key).unwrap();
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    application
        .store
        .set_indentation(
            peek_document,
            configuration,
            IndentationChange::UseSpaces(EDIT_SIZE),
        )
        .unwrap();
    application
        .store
        .set_indentation(
            peek_document,
            configuration,
            IndentationChange::DisplaySize(DISPLAY_SIZE),
        )
        .unwrap();
    let mut readonly =
        taide_file::service::open_file(std::path::Path::new(&path), &[], false).unwrap();
    readonly.read_only = true;
    application
        .store
        .observe_file(document, std::path::Path::new(&path), readonly)
        .unwrap();
    let owner_before = application.store.documents().snapshot(document).unwrap();
    wait_for(
        application,
        &context,
        "actual-format-peek-provider",
        |application| {
            !application
                .lsp
                .as_ref()
                .unwrap()
                .formatting_providers(
                    &project,
                    &application
                        .store
                        .documents()
                        .snapshot(peek_document)
                        .unwrap(),
                    taide_native_editor::formatting::Command::Document,
                )
                .is_empty()
        },
    );
    paint(application, Vec::new());
    let peek_focus = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == egui::ViewportId::ROOT && *target == preview).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(peek_focus));
    application.palette.open(
        &context,
        taide_native_ui::command_registry::PaletteEntry::Commands,
    );
    paint(application, Vec::new());
    paint(application, Vec::new());
    let label = crate::command_registry::registry()
        .unwrap()
        .command("monaco.editor.action.formatDocument")
        .unwrap()
        .label(&application.locale);
    paint(application, vec![egui::Event::Text(label)]);
    assert!(
        application
            .palette
            .inspection()
            .rows
            .iter()
            .any(|row| row.key == "monaco.editor.action.formatDocument"
                && row.is_enabled
                && row.is_selected),
        "palette={:?}; source={:?}; focus={:?}; preview={:?}",
        application.palette.inspection(),
        application.focused_editor_source(&context, &tab, Some(document)),
        application.palette.source_focus(),
        preview
    );
    paint(
        application,
        vec![key(egui::Key::Enter, egui::Modifiers::NONE)],
    );
    wait_for(
        application,
        &context,
        "actual-format-peek-palette",
        |application| {
            application
                .store
                .documents()
                .snapshot(peek_document)
                .unwrap()
                .rope
                .to_string()
                == format!("formatted:4:true:{peek_text}")
        },
    );
    let owner_after = application.store.documents().snapshot(document).unwrap();
    assert_eq!(
        (
            owner_after.rope,
            owner_after.revision,
            owner_after.dirty,
            owner_after.metadata.read_only
        ),
        (
            owner_before.rope,
            owner_before.revision,
            owner_before.dirty,
            true
        )
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), disk);
    assert_eq!(std::fs::read_to_string(&peek_file).unwrap(), peek_text);
    assert!(application.store.undo(peek_document).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .rope
            .to_string(),
        peek_text
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
    let before_clipboard = application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string();
    let bridge = application.bridge.take().unwrap();
    application.runtime.block_on(async {
        tokio::time::timeout(DEADLINE, bridge.disconnect())
            .await
            .unwrap()
            .unwrap();
    });
    let clipboard_reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let captured = clipboard_reads.clone();
    application.bridge = Some(
        HostBridge::connect_with_clipboard_ports(
            services.clone(),
            Arc::new(|| {}),
            Arc::new(|_| panic!("unexpected actual clipboard write")),
            Arc::new(move || {
                if captured.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    return Ok("synthetic-native-clipboard".into());
                }
                Err(AppError::Forbidden(
                    "synthetic clipboard read failed".into(),
                ))
            }),
            None,
        )
        .unwrap(),
    );
    let view = application
        .store
        .attach_view(
            taide_native_editor::view::ViewKey {
                window: "synthetic-completion-clipboard".into(),
                pane: owner.pane.clone(),
                tab: owner.tab.clone(),
            },
            document,
        )
        .unwrap();
    let request = application
        .editor_completion
        .begin(
            &application.store,
            crate::editor_completion::Context {
                project: owner.project.clone(),
                source: view,
                owner: view,
                word: 0..0,
                viewport: egui::ViewportId::ROOT,
                automatic: false,
            },
            HashSet::new(),
        )
        .unwrap();
    let snippets = [taide_model::snippet::SnippetFile {
        file_name: "synthetic.code-snippets".into(),
        snippets: std::collections::BTreeMap::from([(
            "Clipboard".into(),
            taide_model::snippet::SnippetEntry {
                prefix: taide_model::snippet::SnippetStringOrList::Single("paste".into()),
                body: taide_model::snippet::SnippetStringOrList::Single(
                    "${CLIPBOARD:empty}".into(),
                ),
                description: None,
                scope: None,
            },
        )]),
    }];
    assert!(application.editor_completion.supply(
        &application.store,
        &request,
        &snippets,
        &services.tasks,
        Arc::new(|| {})
    ));
    assert!(
        application
            .editor_completion
            .pending(&application.store, view)
    );
    application.dispatch_completion(&context);
    wait_for(
        application,
        &context,
        "completion-clipboard",
        |application| {
            application
                .editor_completion
                .clipboard(&application.store, view)
                .is_some()
        },
    );
    assert_eq!(
        application
            .editor_completion
            .clipboard(&application.store, view)
            .unwrap()
            .as_str(),
        "synthetic-native-clipboard"
    );
    assert!(
        !application
            .editor_completion
            .pending(&application.store, view)
    );
    assert_eq!(clipboard_reads.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        before_clipboard
    );
    application.editor_completion.close(view);
    assert!(
        application
            .editor_completion
            .clipboard(&application.store, view)
            .is_none()
    );
    let failed = application
        .editor_completion
        .begin(
            &application.store,
            crate::editor_completion::Context {
                project: owner.project.clone(),
                source: view,
                owner: view,
                word: 0..0,
                viewport: egui::ViewportId::ROOT,
                automatic: false,
            },
            HashSet::new(),
        )
        .unwrap();
    assert!(application.editor_completion.supply(
        &application.store,
        &failed,
        &snippets,
        &services.tasks,
        Arc::new(|| {})
    ));
    application.dispatch_completion(&context);
    wait_for(
        application,
        &context,
        "completion-clipboard-failed",
        |application| {
            application
                .editor_completion
                .request(&application.store, view)
                .is_none()
        },
    );
    assert_eq!(clipboard_reads.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert!(application.toasts.is_showing(
        taide_native_ui::toast::Kind::Error,
        &crate::toast::describe_error(
            &application.locale,
            &AppError::Forbidden("synthetic clipboard read failed".into())
        )
    ));
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        before_clipboard
    );
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
fn actual_app의_아웃라인_workspace_심볼과_구문_접기는_현재_pane과_reveal을_소비한다() {
    use std::os::unix::fs::PermissionsExt;
    const SCREEN_WIDTH: f32 = 1000.0;
    const SCREEN_HEIGHT: f32 = 700.0;
    const CARET: usize = 8;
    const WORKSPACE_CARET: usize = 12;
    const PICKED_INDENTATION_SIZE: u32 = 2;
    const BASE_INDENTATION_SIZE: u32 = 4;
    const DISPLAY_TAB_SIZE: u32 = 8;
    const END_HIGHLIGHT: usize = 19;
    const EXECUTABLE_MODE: u32 = 0o700;
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
    std::fs::write(&file, "class\n  \u{1f600}method\nend").unwrap();
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
    let paint = |application: &mut NativeApplication, events| {
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
                )),
                events,
                ..Default::default()
            },
            |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
        );
        output.textures_delta.clear();
        output
    };
    paint(application, Vec::new());
    wait_for(
        application,
        &context,
        "navigation-document-read",
        |application| application.files.contains_key(&path),
    );
    let document = application.files[&path].id;
    paint(application, Vec::new());
    let indentation_before = application.store.documents().snapshot(document).unwrap();
    for (id, size, spaces) in [
        (
            "monaco.editor.action.indentUsingTabs",
            PICKED_INDENTATION_SIZE,
            false,
        ),
        (
            "monaco.editor.action.indentUsingSpaces",
            PICKED_INDENTATION_SIZE,
            true,
        ),
        (
            "monaco.editor.action.changeTabDisplaySize",
            DISPLAY_TAB_SIZE,
            true,
        ),
    ] {
        application.palette_commands.push(id.into());
        paint(application, Vec::new());
        paint(application, Vec::new());
        assert!(application.palette.is_picking_indentation(), "{id}");
        paint(
            application,
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                egui::Event::Text(size.to_string()),
                egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: Some(egui::Key::Enter),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: Some(egui::Key::Enter),
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        paint(application, Vec::new());
        assert!(!application.palette.is_open(), "{id}");
        assert!(application.indentation_request.is_none());
        let snapshot = application.store.documents().snapshot(document).unwrap();
        let options = snapshot.model_indentation(application.editor.indent_options(&snapshot));
        assert_eq!(
            (options.tab_size, options.indent_size, options.insert_spaces),
            (size, PICKED_INDENTATION_SIZE, spaces),
            "{id}"
        );
        assert_eq!(
            (snapshot.rope, snapshot.revision, snapshot.dirty),
            (
                indentation_before.rope.clone(),
                indentation_before.revision,
                indentation_before.dirty
            )
        );
    }
    application
        .store
        .set_indentation(
            document,
            crate::presentation_refresh::indent_configuration(
                &application.services.state.settings.read(),
            ),
            taide_native_editor::indent::IndentationChange::DisplaySize(PICKED_INDENTATION_SIZE),
        )
        .unwrap();
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Indentation(taide_native_editor::indent::Command::ToTabs),
    ));
    paint(application, Vec::new());
    let converted = application.store.documents().snapshot(document).unwrap();
    assert_eq!(converted.rope.to_string(), "class\n\t\u{1f600}method\nend");
    assert!(!converted.indent_options.unwrap().insert_spaces);
    assert!(application.store.undo(document).unwrap());
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Indentation(taide_native_editor::indent::Command::Detect),
    ));
    paint(application, Vec::new());
    let snapshot = application.store.documents().snapshot(document).unwrap();
    assert_eq!(
        application.editor.indent_options(&snapshot),
        taide_native_editor::indent::IndentOptions {
            tab_size: 2,
            insert_spaces: true,
        },
        "actual App must detect the two-space document indentation"
    );
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
                tab: tab.clone(),
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
    let output = paint(application, Vec::new());
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
    let mock = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/native-lsp-mock");
    assert!(mock.is_file());
    let mock = mock.to_str().unwrap().replace('\'', "'\\''");
    let bin = fixture.directory.join("bin");
    let executable = bin.join("rust-analyzer");
    std::fs::write(
        &executable,
        format!("#!/bin/sh\nexec '{mock}' --native-workspace-symbols-folding-highlights\n"),
    )
    .unwrap();
    std::fs::set_permissions(
        &executable,
        std::fs::Permissions::from_mode(EXECUTABLE_MODE),
    )
    .unwrap();
    let runtime = application.runtime.handle().clone();
    runtime
        .block_on(application.lsp.take().unwrap().disconnect())
        .unwrap();
    let repaint = context.clone();
    application.lsp = Some(
        crate::lsp::LspBridge::connect(
            application.services.clone(),
            bin.into_os_string(),
            Arc::new(move || repaint.request_repaint()),
        )
        .unwrap(),
    );
    application.reconcile_lsp();
    application.palette.open(
        &context,
        taide_native_ui::command_registry::PaletteEntry::WorkspaceSymbols,
    );
    paint(application, Vec::new());
    paint(application, Vec::new());
    paint(application, vec![egui::Event::Text("query".into())]);
    assert_eq!(application.palette.workspace_query(), Some("query"));
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                paint(application, Vec::new());
                let index = application.workspace_symbols.index(Some(&project));
                if !index.is_pending && index.entries.is_some_and(|symbols| symbols.len() == 2) {
                    assert_eq!(index.entries.unwrap()[1].name, "Workspace:query");
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    let before_tabs =
        crate::tabs::tabs_in(&application.controller.snapshot().layouts[&project].root)
            .into_iter()
            .map(|tab| tab.id.clone())
            .collect::<Vec<_>>();
    paint(
        application,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!application.palette.is_open());
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                paint(application, Vec::new());
                let selections = &application.store.views().get(view).unwrap().selection;
                if selections.selections[selections.primary].head == WORKSPACE_CARET {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    assert_eq!(
        crate::tabs::tabs_in(&application.controller.snapshot().layouts[&project].root)
            .into_iter()
            .map(|tab| tab.id.clone())
            .collect::<Vec<_>>(),
        before_tabs
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                paint(application, Vec::new());
                if application
                    .editor_folding
                    .model(&project, &snapshot)
                    .is_some()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    let highlight_before = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet::default(),
            highlight_before.scroll.clone(),
            highlight_before.folds.clone(),
        )
        .unwrap();
    let focus = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == egui::ViewportId::ROOT && *target == view).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(application, Vec::new());
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                paint(application, Vec::new());
                if application
                    .editor_highlights
                    .display(
                        &application.store,
                        egui::ViewportId::ROOT,
                        view,
                        application.editor_highlight_colors,
                    )
                    .is_some_and(|display| !display.layer.items().is_empty())
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    let display = application
        .editor_highlights
        .display(
            &application.store,
            egui::ViewportId::ROOT,
            view,
            application.editor_highlight_colors,
        )
        .unwrap();
    assert!(
        display
            .layer
            .items()
            .iter()
            .any(|item| item.bytes == (0..5))
    );
    let backgrounds = display
        .layer
        .items()
        .iter()
        .filter_map(|item| {
            if let taide_native_editor::decoration::DecorationKind::Inline(style) = item.kind {
                return style
                    .background
                    .map(|[r, g, b, a]| egui::Color32::from_rgba_unmultiplied(r, g, b, a));
            }
            None
        })
        .collect::<Vec<_>>();
    let highlighted = paint(application, Vec::new());
    assert!(
        highlighted.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Rect(rect) => backgrounds.contains(&rect.fill),
            egui::Shape::Text(text) => text
                .galley
                .job
                .sections
                .iter()
                .any(|section| backgrounds.contains(&section.format.background)),
            _ => false,
        }),
        "actual application must paint the LSP highlight background"
    );
    for (shift, expected) in [
        (false, WORKSPACE_CARET),
        (false, END_HIGHLIGHT),
        (false, 0),
        (true, END_HIGHLIGHT),
        (true, WORKSPACE_CARET),
        (true, 0),
    ] {
        let modifiers = if shift {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        };
        paint(
            application,
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::Key {
                    key: egui::Key::F7,
                    physical_key: Some(egui::Key::F7),
                    pressed: true,
                    repeat: false,
                    modifiers,
                },
                egui::Event::Key {
                    key: egui::Key::F7,
                    physical_key: Some(egui::Key::F7),
                    pressed: false,
                    repeat: false,
                    modifiers,
                },
            ],
        );
        paint(application, Vec::new());
        assert_eq!(
            application
                .store
                .views()
                .get(view)
                .unwrap()
                .selection
                .selections[0]
                .head,
            expected,
            "actual F7 shift={shift}"
        );
        assert!(application.editor_highlights.has_highlights(
            &application.store,
            egui::ViewportId::ROOT,
            view
        ));
    }
    let overrides_before = application
        .services
        .state
        .settings
        .read()
        .keymap_overrides
        .clone();
    application.services.state.settings.write().keymap_overrides = Some(
        r#"[{"actionId":"monaco.editor.action.wordHighlight.trigger","key":"F8","mods":[]}]"#
            .into(),
    );
    application.editor_highlights.close(egui::ViewportId::ROOT);
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Key {
                key: egui::Key::F8,
                physical_key: Some(egui::Key::F8),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key: egui::Key::F8,
                physical_key: Some(egui::Key::F8),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    paint(application, Vec::new());
    assert!(!application.editor_highlights.has_highlights(
        &application.store,
        egui::ViewportId::ROOT,
        view
    ));
    let trigger_deadline = application
        .editor_highlights
        .rendering_deadline(egui::ViewportId::ROOT)
        .unwrap();
    assert!(
        trigger_deadline > Instant::now(),
        "actual F8 must dispatch the delayed explicit trigger"
    );
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                paint(application, Vec::new());
                if application.editor_highlights.has_highlights(
                    &application.store,
                    egui::ViewportId::ROOT,
                    view,
                ) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    application.services.state.settings.write().keymap_overrides = overrides_before;
    assert!(Instant::now() >= trigger_deadline);
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    application
        .store
        .set_view_state(
            view,
            highlight_before.selection,
            highlight_before.scroll,
            highlight_before.folds,
        )
        .unwrap();
    let syntax = application
        .editor_folding
        .model(&project, &snapshot)
        .unwrap();
    assert_eq!(syntax.regions()[0].end_line, 2);
    assert_eq!(syntax.kind(0), Some("imports"));
    application
        .fold_commands
        .push((tab.clone(), FoldCommand::ToggleImports));
    let folded = paint(application, Vec::new());
    assert_eq!(
        application.store.views().get(view).unwrap().folds,
        vec![snapshot.rope.line_to_byte(1)..snapshot.rope.len_bytes()]
    );
    assert!(!folded.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("\u{1f600}method"))));
    application
        .store
        .request_selection_reveal(view, WORKSPACE_CARET..WORKSPACE_CARET, false)
        .unwrap();
    let current = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: WORKSPACE_CARET,
                    head: WORKSPACE_CARET,
                }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    paint(application, Vec::new());
    assert!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .folds
            .is_empty()
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    std::fs::write(
        &executable,
        format!("#!/bin/sh\nexec '{mock}' --native-documentation-peek-highlights\n"),
    )
    .unwrap();
    runtime
        .block_on(application.lsp.take().unwrap().disconnect())
        .unwrap();
    let repaint = context.clone();
    application.lsp = Some(
        crate::lsp::LspBridge::connect(
            application.services.clone(),
            fixture.directory.join("bin").into_os_string(),
            Arc::new(move || repaint.request_repaint()),
        )
        .unwrap(),
    );
    application.reconcile_lsp();
    paint(application, Vec::new());
    wait_for(application, &context, "locations-ready", |application| {
        !application
            .lsp
            .as_ref()
            .unwrap()
            .location_providers(
                &project,
                &application.store.documents().snapshot(document).unwrap(),
                taide_native_editor::symbol_locations::Kind::Definition,
            )
            .is_empty()
    });
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Documentation(taide_native_editor::documentation::Command::ShowHover),
    ));
    paint(application, Vec::new());
    wait_for(
        application,
        &context,
        "main-body-hover-response",
        |application| {
            application
                .editor_documentation
                .payload(
                    &application.store,
                    view,
                    crate::editor_documentation::Kind::Hover,
                )
                .is_some()
        },
    );
    let body_hover = application
        .editor_documentation
        .request(
            &application.store,
            view,
            crate::editor_documentation::Kind::Hover,
        )
        .unwrap()
        .clone();
    assert_eq!(body_hover.owner, view);
    assert_eq!(body_hover.source, view);
    assert_eq!(body_hover.snapshot.id, document);
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                let output = paint(application, Vec::new());
                if output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text)
                    if text.galley.job.text == "fn primary() {}"
                    && text.galley.job.sections.iter().map(|section| section.format.color).collect::<HashSet<_>>().len() > 1)) { break; }
                tokio::task::yield_now().await;
            }
        }).await.expect("actual main-body documentation code did not receive TextMate colors");
    });
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Documentation(taide_native_editor::documentation::Command::ShowHover),
    ));
    paint(application, Vec::new());
    assert_eq!(
        application
            .editor_documentation
            .request(
                &application.store,
                view,
                crate::editor_documentation::Kind::Hover
            )
            .unwrap()
            .token,
        body_hover.token
    );
    assert!(
        context
            .memory(|memory| memory.focused())
            .is_some_and(|focus| application
                .editor_keymap_targets
                .get(&(egui::ViewportId::ROOT, focus))
                .is_some_and(|(owner, _)| *owner == view))
    );
    paint(
        application,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(body_hover.is_cancelled());
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Documentation(taide_native_editor::documentation::Command::Signature(
            taide_native_editor::documentation::SignatureCommand::Trigger,
        )),
    ));
    paint(application, Vec::new());
    wait_for(
        application,
        &context,
        "main-body-signature-response",
        |application| {
            application
                .editor_documentation
                .payload(
                    &application.store,
                    view,
                    crate::editor_documentation::Kind::Signature,
                )
                .is_some()
        },
    );
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Documentation(taide_native_editor::documentation::Command::Signature(
            taide_native_editor::documentation::SignatureCommand::Next,
        )),
    ));
    paint(application, Vec::new());
    assert!(
        matches!(application.editor_documentation.payload(&application.store, view, crate::editor_documentation::Kind::Signature), Some(crate::editor_documentation::Payload::Signature(signature)) if signature.model.index() == 1)
    );
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Documentation(taide_native_editor::documentation::Command::Signature(
            taide_native_editor::documentation::SignatureCommand::Close,
        )),
    ));
    paint(application, Vec::new());
    assert!(
        application
            .editor_documentation
            .payload(
                &application.store,
                view,
                crate::editor_documentation::Kind::Signature
            )
            .is_none()
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Request {
            kind: taide_native_editor::symbol_locations::Kind::Definition,
            mode: taide_native_editor::symbol_locations::Mode::Peek,
        }),
    ));
    paint(application, Vec::new());
    wait_for(
        application,
        &context,
        "actual-peek-response",
        |application| {
            application
                .editor_locations
                .current(view)
                .is_some_and(|session| {
                    session.shown
                        && session
                            .model
                            .as_ref()
                            .is_some_and(|model| model.targets().len() == 1)
                })
        },
    );
    paint(application, Vec::new());
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    assert_eq!(
        application.store.views().get(preview).unwrap().document,
        document
    );
    let preview_tab = application
        .store
        .views()
        .get(preview)
        .unwrap()
        .key
        .tab
        .clone();
    application.submit(HostCommand::SetDirty {
        tab: preview_tab.clone(),
        dirty: true,
    });
    assert!(!application.pending_dirty.contains_key(&preview_tab));
    application.pending_dirty.insert(preview_tab.clone(), true);
    assert!(application.flush_dirty());
    assert!(!application.pending_dirty.contains_key(&preview_tab));
    let current = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection { anchor: 0, head: 0 }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    paint(
        application,
        vec![egui::Event::Key {
            key: egui::Key::F12,
            physical_key: Some(egui::Key::F12),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        WORKSPACE_CARET
    );
    assert!(application.editor_locations.current(view).unwrap().shown);
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );

    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Request {
            kind: taide_native_editor::symbol_locations::Kind::Definition,
            mode: taide_native_editor::symbol_locations::Mode::Hover,
        }),
    ));
    paint(application, Vec::new());
    wait_for(
        application,
        &context,
        "actual-keyboard-definition-hover",
        |application| {
            application
                .editor_locations
                .hover(view)
                .is_some_and(|session| session.request.keyboard && session.model.is_some())
        },
    );
    paint(application, Vec::new());
    let hovered = paint(application, Vec::new());
    assert!(hovered.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("method\n"))));
    assert!(application.editor_locations.current(view).unwrap().shown);
    paint(
        application,
        vec![egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: Some(egui::Key::ArrowRight),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(application.editor_locations.hover(view).is_none());

    let peek_path = std::path::Path::new(&path)
        .parent()
        .unwrap()
        .join("peek.rs");
    std::fs::write(&peek_path, "class\n  \u{1f600}method\nend").unwrap();
    let peek_path = peek_path.to_str().unwrap().to_owned();
    let providers = application.lsp.as_ref().unwrap().location_providers(
        &project,
        &application.store.documents().snapshot(document).unwrap(),
        taide_native_editor::symbol_locations::Kind::Definition,
    );
    let identity = *providers.iter().next().unwrap();
    let request = application
        .editor_locations
        .begin(
            project.clone(),
            &application.store,
            view,
            taide_native_editor::symbol_locations::Kind::Definition,
            taide_native_editor::symbol_locations::Mode::Peek,
            providers.clone(),
            None,
        )
        .unwrap();
    let range = taide_native_editor::lsp::LspRange::new(
        taide_native_editor::lsp::Position::new(1, 4),
        taide_native_editor::lsp::Position::new(1, 10),
    );
    let target = taide_native_editor::symbol_locations::Target {
        uri: taide_lsp::service::workspace_folder_uri(&peek_path)
            .parse()
            .unwrap(),
        range,
        selection: range,
        origin: None,
    };
    let source_target = taide_native_editor::symbol_locations::Target {
        uri: taide_lsp::service::workspace_folder_uri(&path)
            .parse()
            .unwrap(),
        range,
        selection: range,
        origin: None,
    };
    application
        .editor_locations
        .accept(
            &request,
            &application.store,
            providers,
            Ok(crate::editor_locations::Response {
                groups: vec![crate::editor_locations::Group {
                    provider: identity,
                    targets: vec![source_target, target],
                }],
            }),
        )
        .unwrap()
        .unwrap();
    application.editor_locations.show(view);
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Select {
            index: 1,
            focus_preview: false,
        }),
    ));
    paint(application, Vec::new());
    let key = taide_native_editor::document::DocumentKey::File(peek_path.clone().into());
    wait_for(
        application,
        &context,
        "actual-peek-file-read",
        |application| application.store.documents().find(&key).is_some(),
    );
    paint(application, Vec::new());
    let peek_document = application.store.documents().find(&key).unwrap();
    assert!(application.peek_models.owns(peek_document));
    assert!(!application.files.contains_key(&peek_path));
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    let focus = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (owner, _))| {
            (*viewport == egui::ViewportId::ROOT && *owner == preview).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    let indentation_overrides = application
        .services
        .state
        .settings
        .read()
        .keymap_overrides
        .clone();
    application.services.state.settings.write().keymap_overrides = Some(r#"[{"actionId":"monaco.editor.action.indentUsingTabs","key":"F9","mods":[]},{"actionId":"monaco.editor.action.indentUsingSpaces","key":"F10","mods":[]},{"actionId":"monaco.editor.action.changeTabDisplaySize","key":"F11","mods":[]}]"#.into());
    let peek_indentation_before = application
        .store
        .documents()
        .snapshot(peek_document)
        .unwrap();
    let body_indentation_before = application.store.documents().snapshot(document).unwrap();
    for (key, size, spaces) in [
        (egui::Key::F9, PICKED_INDENTATION_SIZE, false),
        (egui::Key::F10, PICKED_INDENTATION_SIZE, true),
        (egui::Key::F11, DISPLAY_TAB_SIZE, true),
    ] {
        context.memory_mut(|memory| memory.request_focus(focus));
        paint(
            application,
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        paint(application, Vec::new());
        assert!(application.palette.is_picking_indentation(), "{key:?}");
        paint(
            application,
            vec![
                egui::Event::Text(size.to_string()),
                egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: Some(egui::Key::Enter),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: Some(egui::Key::Enter),
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        paint(application, Vec::new());
        assert!(!application.palette.is_open());
        assert!(context.memory(|memory| memory.has_focus(focus)));
        let snapshot = application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap();
        let options = snapshot.model_indentation(application.editor.indent_options(&snapshot));
        assert_eq!(
            (options.tab_size, options.indent_size, options.insert_spaces),
            (size, PICKED_INDENTATION_SIZE, spaces),
            "{key:?}"
        );
        assert_eq!(
            (snapshot.rope, snapshot.revision, snapshot.dirty),
            (
                peek_indentation_before.rope.clone(),
                peek_indentation_before.revision,
                peek_indentation_before.dirty
            )
        );
        let body = application.store.documents().snapshot(document).unwrap();
        assert_eq!(
            (body.indent_options, body.indent_size),
            (
                body_indentation_before.indent_options,
                body_indentation_before.indent_size
            )
        );
    }
    application.services.state.settings.write().keymap_overrides = indentation_overrides;
    application
        .store
        .set_indentation(
            peek_document,
            crate::presentation_refresh::indent_configuration(
                &application.services.state.settings.read(),
            ),
            taide_native_editor::indent::IndentationChange::DisplaySize(PICKED_INDENTATION_SIZE),
        )
        .unwrap();
    application.reconcile_lsp();
    wait_for(
        application,
        &context,
        "pure-peek-documentation-provider",
        |application| {
            !application
                .lsp
                .as_ref()
                .unwrap()
                .documentation_providers(
                    &project,
                    &application
                        .store
                        .documents()
                        .snapshot(peek_document)
                        .unwrap(),
                    crate::editor_documentation::Kind::Hover,
                )
                .is_empty()
        },
    );
    assert!(
        !application
            .lsp
            .as_ref()
            .unwrap()
            .highlight_providers(
                &project,
                &application
                    .store
                    .documents()
                    .snapshot(peek_document)
                    .unwrap()
            )
            .is_empty(),
        "pure peek must have an advertised highlight provider"
    );
    assert!(
        context.memory(|memory| memory.has_focus(focus)),
        "pure peek focus lost: {:?}",
        context.memory(|memory| memory.focused())
    );
    paint(application, Vec::new());
    let observed_peek = application
        .editor_highlights
        .current_request(egui::ViewportId::ROOT)
        .unwrap_or_else(|| {
            panic!(
                "peek not observed: selection={:?}; composition={:?}; focus={:?}",
                application.store.views().get(preview).unwrap().selection,
                application.store.views().get(preview).unwrap().composition,
                context.memory(|memory| memory.focused()),
            )
        });
    assert_eq!(observed_peek.source, preview);
    assert!(
        observed_peek.is_active(
            &application.services.state.layouts.read()[&project],
            &application.shell.scope
        ),
        "peek owner must be an active tab"
    );
    wait_for(
        application,
        &context,
        "pure-peek-highlights",
        |application| {
            paint(application, Vec::new());
            application.editor_highlights.has_highlights(
                &application.store,
                egui::ViewportId::ROOT,
                preview,
            )
        },
    );
    let peek_highlight_before = application.store.views().get(preview).unwrap().clone();
    let main_highlight_before = application.store.views().get(view).unwrap().clone();
    let peek_highlight_request = application
        .editor_highlights
        .current_request(egui::ViewportId::ROOT)
        .unwrap();
    assert_eq!(peek_highlight_request.source, preview);
    assert_eq!(peek_highlight_request.owner, view);
    assert_eq!(peek_highlight_request.snapshot.id, peek_document);
    assert!(!application.editor_highlights.has_highlights(
        &application.store,
        egui::ViewportId::ROOT,
        view
    ));
    let peek_paint = paint(application, Vec::new());
    assert!(
        peek_paint.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Rect(rect) => backgrounds.contains(&rect.fill),
            egui::Shape::Text(text) => text
                .galley
                .job
                .sections
                .iter()
                .any(|section| backgrounds.contains(&section.format.background)),
            _ => false,
        }),
        "actual peek must paint its own document highlights"
    );
    for (shift, expected) in [
        (false, END_HIGHLIGHT),
        (false, 0),
        (true, END_HIGHLIGHT),
        (true, WORKSPACE_CARET),
    ] {
        let modifiers = if shift {
            egui::Modifiers::SHIFT
        } else {
            egui::Modifiers::NONE
        };
        paint(
            application,
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::Key {
                    key: egui::Key::F7,
                    physical_key: Some(egui::Key::F7),
                    pressed: true,
                    repeat: false,
                    modifiers,
                },
                egui::Event::Key {
                    key: egui::Key::F7,
                    physical_key: Some(egui::Key::F7),
                    pressed: false,
                    repeat: false,
                    modifiers,
                },
            ],
        );
        paint(application, Vec::new());
        assert_eq!(
            application
                .store
                .views()
                .get(preview)
                .unwrap()
                .selection
                .selections[0]
                .head,
            expected,
            "peek F7 shift={shift}"
        );
        assert_eq!(
            application.store.views().get(view).unwrap().selection,
            main_highlight_before.selection
        );
        assert!(!peek_highlight_request.is_cancelled());
    }
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Highlight(crate::command_registry::HighlightCommand::Previous),
    ));
    paint(application, Vec::new());
    assert_eq!(
        application
            .store
            .views()
            .get(preview)
            .unwrap()
            .selection
            .selections[0]
            .head,
        0
    );
    assert_eq!(
        application.store.views().get(view).unwrap().selection,
        main_highlight_before.selection
    );
    application
        .store
        .set_view_state(
            preview,
            peek_highlight_before.selection.clone(),
            peek_highlight_before.scroll.clone(),
            peek_highlight_before.folds.clone(),
        )
        .unwrap();
    let peek_overrides_before = application
        .services
        .state
        .settings
        .read()
        .keymap_overrides
        .clone();
    application.services.state.settings.write().keymap_overrides = Some(
        r#"[{"actionId":"monaco.editor.action.wordHighlight.trigger","key":"F8","mods":[]}]"#
            .into(),
    );
    application.editor_highlights.close(egui::ViewportId::ROOT);
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Key {
                key: egui::Key::F8,
                physical_key: Some(egui::Key::F8),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key: egui::Key::F8,
                physical_key: Some(egui::Key::F8),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    let peek_trigger_deadline = application
        .editor_highlights
        .rendering_deadline(egui::ViewportId::ROOT)
        .unwrap();
    assert!(peek_trigger_deadline > Instant::now());
    wait_for(
        application,
        &context,
        "pure-peek-explicit-highlights",
        |application| {
            paint(application, Vec::new());
            application.editor_highlights.has_highlights(
                &application.store,
                egui::ViewportId::ROOT,
                preview,
            )
        },
    );
    assert!(Instant::now() >= peek_trigger_deadline);
    application.services.state.settings.write().keymap_overrides = peek_overrides_before;
    application
        .store
        .set_view_state(
            preview,
            peek_highlight_before.selection,
            peek_highlight_before.scroll,
            peek_highlight_before.folds,
        )
        .unwrap();
    let suspended_peek_lsp = application.lsp.take().unwrap();
    wait_for(
        application,
        &context,
        "peek-textual-without-lsp",
        |application| {
            paint(application, Vec::new());
            application
                .editor_highlights
                .current_request(egui::ViewportId::ROOT)
                .is_some_and(|request| request.source == preview)
                && application
                    .editor_highlights
                    .display(
                        &application.store,
                        egui::ViewportId::ROOT,
                        preview,
                        application.editor_highlight_colors,
                    )
                    .is_some_and(|display| display.layer.items().len() == 2)
        },
    );
    let textual_peek_request = application
        .editor_highlights
        .current_request(egui::ViewportId::ROOT)
        .unwrap();
    assert_eq!(textual_peek_request.owner, view);
    assert_eq!(textual_peek_request.snapshot.id, peek_document);
    assert!(!application.editor_highlights.has_highlights(
        &application.store,
        egui::ViewportId::ROOT,
        view
    ));
    let textual_peek_paint = paint(application, Vec::new());
    assert!(
        textual_peek_paint
            .shapes
            .iter()
            .any(|shape| match &shape.shape {
                egui::Shape::Rect(rect) => backgrounds.contains(&rect.fill),
                egui::Shape::Text(text) => text
                    .galley
                    .job
                    .sections
                    .iter()
                    .any(|section| backgrounds.contains(&section.format.background)),
                _ => false,
            }),
        "actual peek paints textual highlights without LSP"
    );
    application.lsp = Some(suspended_peek_lsp);
    wait_for(
        application,
        &context,
        "peek-lsp-after-textual",
        |application| {
            paint(application, Vec::new());
            application
                .editor_highlights
                .current_request(egui::ViewportId::ROOT)
                .is_some_and(|request| {
                    request.source == preview && request.token != textual_peek_request.token
                })
                && application
                    .editor_highlights
                    .display(
                        &application.store,
                        egui::ViewportId::ROOT,
                        preview,
                        application.editor_highlight_colors,
                    )
                    .is_some_and(|display| display.layer.items().len() == 6)
        },
    );
    assert!(textual_peek_request.is_cancelled());
    let peek_edit_highlight_request = application
        .editor_highlights
        .current_request(egui::ViewportId::ROOT)
        .unwrap();
    assert_eq!(peek_edit_highlight_request.snapshot.id, peek_document);
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .revision,
        peek_highlight_request.snapshot.revision
    );
    context.set_os(egui::os::OperatingSystem::Mac);
    let documentation_key = |key| egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
    };
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND),
            documentation_key(egui::Key::K),
            documentation_key(egui::Key::I),
        ],
    );
    paint(application, Vec::new());
    assert!(
        application
            .editor_documentation
            .request(
                &application.store,
                preview,
                crate::editor_documentation::Kind::Hover
            )
            .is_some(),
        "peek hover request was not created; focus={:?}, preview={preview:?}, status={:?}",
        context.memory(|memory| memory.focused()),
        application.status
    );
    wait_for(
        application,
        &context,
        "pure-peek-hover-response",
        |application| {
            application
                .editor_documentation
                .payload(
                    &application.store,
                    preview,
                    crate::editor_documentation::Kind::Hover,
                )
                .is_some()
        },
    );
    let request = application
        .editor_documentation
        .request(
            &application.store,
            preview,
            crate::editor_documentation::Kind::Hover,
        )
        .unwrap()
        .clone();
    assert_eq!(request.owner, view);
    assert_eq!(request.snapshot.id, peek_document);
    assert_eq!(request.project, project);
    paint(application, Vec::new());
    let hover = paint(application, Vec::new());
    assert!(hover.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("primary docs"))));
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(
                egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            ),
            egui::Event::Key {
                key: egui::Key::Space,
                physical_key: Some(egui::Key::Space),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::MAC_CMD
                    | egui::Modifiers::COMMAND
                    | egui::Modifiers::SHIFT,
            },
        ],
    );
    paint(application, Vec::new());
    wait_for(
        application,
        &context,
        "pure-peek-signature-response",
        |application| {
            application
                .editor_documentation
                .payload(
                    &application.store,
                    preview,
                    crate::editor_documentation::Kind::Signature,
                )
                .is_some()
        },
    );
    let Some(crate::editor_documentation::Payload::Signature(signature)) =
        application.editor_documentation.payload(
            &application.store,
            preview,
            crate::editor_documentation::Kind::Signature,
        )
    else {
        panic!("expected peek signature")
    };
    assert_eq!(signature.model.parameter_range(), Some(2..6));
    paint(application, Vec::new());
    let signature = paint(application, Vec::new());
    assert!(signature.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "**literal parameter**")));
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: Some(egui::Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            },
        ],
    );
    assert!(
        application
            .editor_documentation
            .payload(
                &application.store,
                preview,
                crate::editor_documentation::Kind::Signature
            )
            .is_none()
    );
    assert!(!application.files.contains_key(&peek_path));
    for _ in 0..2 {
        paint(
            application,
            vec![
                documentation_key(egui::Key::K),
                egui::Event::Key {
                    key: egui::Key::F2,
                    physical_key: Some(egui::Key::F2),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        paint(application, Vec::new());
    }
    assert!(context.memory(|memory| memory.has_focus(focus)));
    assert!(
        application
            .terminal_views
            .chord_status(&context, Instant::now())
            .shortcut
            .is_some(),
        "peek local commands preserve the earlier CtrlK chord until Escape"
    );
    paint(application, vec![egui::Event::Text("x".into())]);
    assert!(request.is_cancelled());
    assert!(peek_edit_highlight_request.is_cancelled());
    assert!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .dirty
    );
    assert_eq!(
        application.draft_project(peek_document),
        Some(project.clone())
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    let command = egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND;
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(command),
            egui::Event::Key {
                key: egui::Key::F,
                physical_key: Some(egui::Key::F),
                pressed: true,
                repeat: false,
                modifiers: command,
            },
        ],
    );
    paint(application, Vec::new());
    assert!(application.editor_locations.preview_find_visible(view));
    assert!(!application.editor_find[&view].visible);
    let replace = command | egui::Modifiers::ALT;
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(replace),
            egui::Event::Key {
                key: egui::Key::F,
                physical_key: Some(egui::Key::F),
                pressed: true,
                repeat: false,
                modifiers: replace,
            },
        ],
    );
    paint(application, Vec::new());
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Text("xmapped".into()),
        ],
    );
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(command),
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed: true,
                repeat: false,
                modifiers: command,
            },
        ],
    );
    assert!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .rope
            .to_string()
            .contains("xmapped")
    );
    assert!(
        !application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .rope
            .to_string()
            .contains("xmethod")
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: Some(egui::Key::Escape),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(!application.editor_locations.preview_find_visible(view));
    assert!(application.editor_locations.current(view).unwrap().shown);
    assert!(
        application
            .terminal_views
            .chord_status(&context, Instant::now())
            .shortcut
            .is_none(),
        "peek find Escape cancels the earlier CtrlK chord"
    );
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(command),
            egui::Event::Key {
                key: egui::Key::S,
                physical_key: Some(egui::Key::S),
                pressed: true,
                repeat: false,
                modifiers: command,
            },
        ],
    );
    wait_for(
        application,
        &context,
        "actual-preview-save",
        |application| {
            !application
                .store
                .documents()
                .snapshot(peek_document)
                .unwrap()
                .dirty
        },
    );
    assert!(
        std::fs::read_to_string(&peek_path)
            .unwrap()
            .contains("xmapped")
    );
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Text("y".into()),
        ],
    );
    let dirty_text = application
        .store
        .documents()
        .snapshot(peek_document)
        .unwrap()
        .rope
        .to_string();
    assert!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .dirty
    );
    let target = application
        .editor_locations
        .widget(&application.store, view)
        .unwrap()
        .model
        .targets()[1]
        .clone();
    let expected_caret = taide_native_editor::lsp::range_to_bytes(
        &application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap(),
        target.selection,
    )
    .unwrap()
    .start;
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::GotoSelected),
    ));
    paint(application, Vec::new());
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                paint(application, Vec::new());
                if application.files.get(&peek_path).is_some_and(|file| {
                    application.store.views().for_document(file.id).any(|view| {
                        view.selection.selections[view.selection.primary].head == expected_caret
                            && application
                                .editor_locations
                                .current(view.id)
                                .is_some_and(|session| session.shown)
                    })
                }) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
    assert_eq!(application.files[&peek_path].id, peek_document);
    let active = application
        .controller
        .snapshot()
        .focused_tab()
        .unwrap()
        .id
        .clone();
    let destination = application
        .store
        .views()
        .for_document(peek_document)
        .find(|view| view.key.tab == active)
        .unwrap();
    assert_eq!(destination.selection.selections[0].head, expected_caret);
    let destination_view = destination.id;
    assert!(application.editor_locations.current(view).is_none());
    assert!(
        application
            .editor_locations
            .current(destination_view)
            .unwrap()
            .shown
    );
    assert_eq!(
        application
            .editor_locations
            .current(destination_view)
            .unwrap()
            .model
            .as_ref()
            .unwrap()
            .targets()
            .len(),
        2
    );
    assert!(!application.peek_models.owns(peek_document));
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .rope
            .to_string(),
        dirty_text
    );
    assert!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .dirty
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );

    for (modifiers, source_view, expected_view) in [
        (egui::Modifiers::NONE, destination_view, view),
        (egui::Modifiers::SHIFT, view, destination_view),
    ] {
        let navigation_tab = application
            .store
            .views()
            .get(source_view)
            .unwrap()
            .key
            .tab
            .clone();
        application
            .controller
            .submit(taide_native_ui::commands::ShellMutation::ActivateTab(
                navigation_tab.clone(),
            ))
            .unwrap();
        wait_for(
            application,
            &context,
            "F4-source-activation",
            |application| {
                application
                    .controller
                    .snapshot()
                    .focused_tab()
                    .is_some_and(|tab| tab.id == navigation_tab)
            },
        );
        paint(application, Vec::new());
        let navigation_focus = application
            .editor_keymap_targets
            .iter()
            .filter(|((viewport, id), (target, _))| {
                *viewport == egui::ViewportId::ROOT
                    && *target == source_view
                    && taide_native_ui::editor_surface::NativeEditor::is_body_focus_target(
                        &context, *id,
                    )
            })
            .max_by_key(|(_, (_, pass))| *pass)
            .map(|((_, id), _)| *id)
            .unwrap();
        context.memory_mut(|memory| memory.request_focus(navigation_focus));
        paint(application, Vec::new());
        paint(
            application,
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::Key {
                    key: egui::Key::F4,
                    physical_key: Some(egui::Key::F4),
                    pressed: true,
                    repeat: false,
                    modifiers,
                },
                egui::Event::Key {
                    key: egui::Key::F4,
                    physical_key: Some(egui::Key::F4),
                    pressed: false,
                    repeat: false,
                    modifiers,
                },
            ],
        );
        let navigation = runtime.block_on(async {
            tokio::time::timeout(DEADLINE, async {
                loop {
                    eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                    paint(application, Vec::new());
                    if application
                        .editor_locations
                        .current(expected_view)
                        .is_some_and(|session| session.shown)
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
        });
        assert!(
            navigation.is_ok(),
            "F4 source={source_view:?}; destination={expected_view:?}; focus={:?}; focused_tab={:?}; session={:?}; status={:?}",
            context.memory(|memory| memory.focused()),
            application
                .controller
                .snapshot()
                .focused_tab()
                .map(|tab| &tab.id),
            application
                .editor_locations
                .current(expected_view)
                .map(|session| (
                    session.shown,
                    session.model.as_ref().map(|model| model.targets().len()),
                    &session.request.source_key,
                )),
            application.status,
        );
        let session = application.editor_locations.current(expected_view).unwrap();
        assert_eq!(session.model.as_ref().unwrap().targets().len(), 2);
        assert_eq!(
            session.request.source_key,
            application.store.views().get(expected_view).unwrap().key
        );
        assert_eq!(
            application.controller.snapshot().focused_tab().unwrap().id,
            session.request.source_key.tab
        );
    }
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .rope
            .to_string(),
        dirty_text
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        snapshot.revision
    );
    let closing_preview = application
        .editor_locations
        .current(destination_view)
        .unwrap()
        .preview
        .unwrap();
    let closing_preview_focus = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == egui::ViewportId::ROOT && *target == closing_preview).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(closing_preview_focus));
    wait_for(
        application,
        &context,
        "peek-highlights-before-close",
        |application| {
            paint(application, Vec::new());
            application
                .editor_highlights
                .current_request(egui::ViewportId::ROOT)
                .is_some_and(|request| request.source == closing_preview)
                && application.editor_highlights.has_highlights(
                    &application.store,
                    egui::ViewportId::ROOT,
                    closing_preview,
                )
        },
    );
    let closed_peek_highlights = application
        .editor_highlights
        .current_request(egui::ViewportId::ROOT)
        .unwrap();
    application.document_edits.push((
        application
            .store
            .views()
            .get(destination_view)
            .unwrap()
            .key
            .tab
            .clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Close),
    ));
    paint(application, Vec::new());
    paint(application, Vec::new());
    assert!(
        application
            .editor_locations
            .current(destination_view)
            .is_none()
    );
    assert!(closed_peek_highlights.is_cancelled());
    let readonly_path = std::path::Path::new(&peek_path);
    let readonly_save = application.store.save_snapshot(peek_document).unwrap();
    std::fs::write(readonly_path, readonly_save.rope().to_string()).unwrap();
    assert!(application.store.mark_saved(readonly_save, None).unwrap());
    let mut readonly_file = taide_file::service::open_file(readonly_path, &[], false).unwrap();
    readonly_file.read_only = true;
    application
        .store
        .refresh_clean_file(peek_document, readonly_path, readonly_file)
        .unwrap();
    let readonly_before = application
        .store
        .documents()
        .snapshot(peek_document)
        .unwrap();
    assert!(readonly_before.metadata.read_only);
    assert_eq!(
        readonly_before.metadata.tier,
        taide_model::file::FileSizeTier::Normal
    );
    let root_before_closing = application
        .store
        .views()
        .get(destination_view)
        .unwrap()
        .clone();
    application
        .store
        .set_view_state(
            destination_view,
            taide_native_editor::view::SelectionSet::default(),
            root_before_closing.scroll,
            root_before_closing.folds,
        )
        .unwrap();
    paint(application, Vec::new());
    let root_closing_focus = application
        .editor_keymap_targets
        .iter()
        .filter(|((viewport, id), (target, _))| {
            *viewport == egui::ViewportId::ROOT
                && *target == destination_view
                && taide_native_ui::editor_surface::NativeEditor::is_body_focus_target(
                    &context, *id,
                )
        })
        .max_by_key(|(_, (_, pass))| *pass)
        .map(|((_, id), _)| *id)
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(root_closing_focus));
    let readonly_tab = application
        .store
        .views()
        .get(destination_view)
        .unwrap()
        .key
        .tab
        .clone();
    let readonly_indentation_before = application
        .store
        .documents()
        .snapshot(peek_document)
        .unwrap();
    application
        .palette_commands
        .push("monaco.editor.action.indentUsingTabs".into());
    paint(application, Vec::new());
    paint(application, Vec::new());
    assert!(application.palette.is_picking_indentation());
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Text(BASE_INDENTATION_SIZE.to_string()),
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    paint(application, Vec::new());
    assert!(!application.palette.is_open());
    let readonly_indentation_after = application
        .store
        .documents()
        .snapshot(peek_document)
        .unwrap();
    let readonly_options = readonly_indentation_after.model_indentation(
        application
            .editor
            .indent_options(&readonly_indentation_after),
    );
    assert_eq!(
        (
            readonly_options.tab_size,
            readonly_options.indent_size,
            readonly_options.insert_spaces
        ),
        (BASE_INDENTATION_SIZE, BASE_INDENTATION_SIZE, false)
    );
    assert!(readonly_indentation_after.metadata.read_only);
    assert_eq!(
        (
            readonly_indentation_after.rope,
            readonly_indentation_after.revision,
            readonly_indentation_after.dirty
        ),
        (
            readonly_indentation_before.rope,
            readonly_indentation_before.revision,
            readonly_indentation_before.dirty
        )
    );
    let configuration = crate::presentation_refresh::indent_configuration(
        &application.services.state.settings.read(),
    );
    let original = readonly_indentation_before.indent_options.unwrap();
    application
        .store
        .set_indentation(
            peek_document,
            configuration,
            if original.insert_spaces {
                taide_native_editor::indent::IndentationChange::UseSpaces(original.tab_size)
            } else {
                taide_native_editor::indent::IndentationChange::UseTabs(original.tab_size)
            },
        )
        .unwrap();
    let suspended_lsp = application.lsp.take().unwrap();
    wait_for(
        application,
        &context,
        "readonly-textual-without-lsp",
        |application| {
            paint(application, Vec::new());
            application.editor_highlights.has_highlights(
                &application.store,
                egui::ViewportId::ROOT,
                destination_view,
            )
        },
    );
    let textual_request = application
        .editor_highlights
        .current_request(egui::ViewportId::ROOT)
        .unwrap();
    assert_eq!(textual_request.source, destination_view);
    assert_eq!(
        textual_request.snapshot.metadata.tier,
        taide_model::file::FileSizeTier::Normal
    );
    assert!(textual_request.snapshot.metadata.read_only);
    let textual_display = application
        .editor_highlights
        .display(
            &application.store,
            egui::ViewportId::ROOT,
            destination_view,
            application.editor_highlight_colors,
        )
        .unwrap();
    assert_eq!(textual_display.layer.items().len(), 2);
    assert!(
        textual_display
            .layer
            .items()
            .iter()
            .all(|item| item.bytes == (0..5))
    );
    let textual_paint = paint(application, Vec::new());
    assert!(
        textual_paint.shapes.iter().any(|shape| match &shape.shape {
            egui::Shape::Rect(rect) => backgrounds.contains(&rect.fill),
            egui::Shape::Text(text) => text
                .galley
                .job
                .sections
                .iter()
                .any(|section| backgrounds.contains(&section.format.background)),
            _ => false,
        }),
        "actual readonly editor paints textual highlights without LSP"
    );
    assert!(!textual_request.is_cancelled());
    application.lsp = Some(suspended_lsp);
    wait_for(
        application,
        &context,
        "root-highlights-before-exit",
        |application| {
            paint(application, Vec::new());
            application
                .controller
                .snapshot()
                .focused_tab()
                .is_some_and(|tab| tab.id == readonly_tab)
                && application
                    .editor_highlights
                    .current_request(egui::ViewportId::ROOT)
                    .is_some_and(|request| request.source == destination_view)
                && application.editor_highlights.has_highlights(
                    &application.store,
                    egui::ViewportId::ROOT,
                    destination_view,
                )
        },
    );
    assert!(textual_request.is_cancelled());
    let closing_highlights = application
        .editor_highlights
        .current_request(egui::ViewportId::ROOT)
        .unwrap();
    assert!(context.memory(|memory| memory.has_focus(root_closing_focus)));
    assert_eq!(closing_highlights.owner, destination_view);
    assert_eq!(
        application.controller.snapshot().focused_tab().unwrap().id,
        application
            .store
            .views()
            .get(destination_view)
            .unwrap()
            .key
            .tab
    );
    assert!(application.pending_disk_choice.is_none());
    let expected_readonly = taide_native_editor::lsp::range_to_bytes(
        &readonly_before,
        taide_native_editor::lsp::LspRange::new(
            taide_native_editor::lsp::Position::new(2, 0),
            taide_native_editor::lsp::Position::new(2, 3),
        ),
    )
    .unwrap();
    application.document_edits.push((
        readonly_tab.clone(),
        DocumentEdit::Highlight(crate::command_registry::HighlightCommand::Previous),
    ));
    paint(application, Vec::new());
    assert_eq!(
        application
            .store
            .views()
            .get(destination_view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        expected_readonly.start,
        "readonly queued highlight command"
    );
    application.document_edits.push((
        readonly_tab.clone(),
        DocumentEdit::Highlight(crate::command_registry::HighlightCommand::Next),
    ));
    paint(application, Vec::new());
    assert_eq!(
        application
            .store
            .views()
            .get(destination_view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        0,
        "readonly queued highlight command return"
    );
    assert_eq!(
        context.memory(|memory| memory.focused()),
        Some(root_closing_focus)
    );
    assert!(
        application.editor_highlights.has_highlights(
            &application.store,
            egui::ViewportId::ROOT,
            destination_view
        ),
        "readonly cached marks after queued movement"
    );
    assert_eq!(
        application.editor_highlights.source_for_owner(
            &application.store,
            egui::ViewportId::ROOT,
            destination_view
        ),
        destination_view
    );
    let readonly_shell = application.controller.snapshot();
    let readonly_project = readonly_shell.focused_project().unwrap();
    let readonly_global_key = taide_native_editor::view::ViewKey {
        window: WINDOW_LABEL.into(),
        pane: readonly_shell.layouts[readonly_project]
            .focused_pane
            .clone(),
        tab: readonly_tab.clone(),
    };
    assert_eq!(
        application.store.views().find(&readonly_global_key),
        Some(destination_view),
        "readonly global editor binding"
    );
    let readonly_actions = crate::command_dispatch::active_editor_actions(
        &application.store,
        Some(&application.store.views().get(destination_view).unwrap().key),
    )
    .unwrap();
    assert!(readonly_actions.contains("editor.action.wordHighlight.prev"));
    let readonly_context = crate::command_dispatch::context(
        &application.controller.snapshot(),
        &application.shell.scope,
        Some(readonly_actions),
    );
    assert!(crate::command_dispatch::accepts(
        "monaco.editor.action.wordHighlight.prev",
        true,
        &readonly_context
    ));
    assert_eq!(
        context.keyboard_focus_before_events(),
        Some(root_closing_focus)
    );
    assert!(
        application
            .terminal_views
            .chord_status(&context, Instant::now())
            .shortcut
            .is_none(),
        "Escape cancels the earlier CtrlK chord before readonly highlight keys"
    );
    paint(
        application,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::SHIFT),
            egui::Event::Key {
                key: egui::Key::F7,
                physical_key: Some(egui::Key::F7),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Event::Key {
                key: egui::Key::F7,
                physical_key: Some(egui::Key::F7),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            },
        ],
    );
    let readonly_key_queued = application.document_edits.iter().any(|(owner, edit)| {
        *owner == readonly_tab
            && matches!(
                edit,
                DocumentEdit::Highlight(crate::command_registry::HighlightCommand::Previous)
            )
    });
    let readonly_first_key_caret = application
        .store
        .views()
        .get(destination_view)
        .unwrap()
        .selection
        .selections[0]
        .head;
    paint(application, Vec::new());
    assert_eq!(
        application
            .store
            .views()
            .get(destination_view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        expected_readonly.start,
        "readonly F7 first={readonly_first_key_caret}; queued={readonly_key_queued}; busy={}; disk_choice={}; tab_close={}; shutting_down={}; status={:?}",
        application.workspace_busy(),
        application.pending_disk_choice.is_some(),
        application.pending_tab_close.is_some(),
        application.services.state.is_shutting_down(),
        application.status
    );
    paint(application, vec![egui::Event::Text("z".into())]);
    let readonly_after = application
        .store
        .documents()
        .snapshot(peek_document)
        .unwrap();
    assert!(readonly_after.metadata.read_only);
    assert_eq!(readonly_after.revision, readonly_before.revision);
    assert_eq!(readonly_after.rope, readonly_before.rope);
    assert!(!closing_highlights.is_cancelled());
    let baseline_documents = application
        .store
        .documents()
        .versions()
        .map(|version| version.id)
        .collect::<HashSet<_>>();
    let closing_tab = application
        .controller
        .snapshot()
        .focused_tab()
        .unwrap()
        .id
        .clone();
    application.document_edits.push((
        closing_tab,
        DocumentEdit::Documentation(taide_native_editor::documentation::Command::ShowHover),
    ));
    paint(application, Vec::new());
    wait_for(
        application,
        &context,
        "active-documentation-before-exit",
        |application| {
            application
                .editor_documentation
                .payload(
                    &application.store,
                    destination_view,
                    crate::editor_documentation::Kind::Hover,
                )
                .is_some()
        },
    );
    paint(application, Vec::new());
    let closing_hover = application
        .editor_documentation
        .request(
            &application.store,
            destination_view,
            crate::editor_documentation::Kind::Hover,
        )
        .unwrap()
        .clone();
    application.close(&context);
    assert!(closing_hover.is_cancelled());
    assert!(closing_highlights.is_cancelled());
    assert_eq!(
        baseline_documents,
        application
            .store
            .documents()
            .versions()
            .map(|version| version.id)
            .collect()
    );
    wait_for(application, &context, "navigation-exit", |application| {
        application.is_exit_ready
    });
    assert_eq!(application.services.tasks.tracked_count(), 0);
    eframe::App::on_exit(application);
    drop(fixture.application.take());
}
