use super::*;
use taide_model::{paths::AppPaths, project::Project};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::Transaction;

const DEADLINE: Duration = Duration::from_secs(10);
const FRAME_SIZE: egui::Vec2 = egui::vec2(1000.0, 700.0);
const TEXT: &str = "{\n  \"\u{1f600}\": {\n    \"value\": 1\n  },\n  \"list\": [1, 2]\n}";
const INDEX: &[u8] = b"<!doctype html><title>owned native JSON fixture</title>";
const MODULE: &[u8] = b"\0asm\x01\0\0\0";

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

fn paint(application: &mut NativeApplication, context: &egui::Context) -> egui::FullOutput {
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    output.textures_delta.clear();
    output
}

fn wait(
    application: &mut NativeApplication,
    context: &egui::Context,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let runtime = application.runtime.handle().clone();
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
                paint(application, context);
                if ready(application) {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    });
}

#[test]
fn actual_app의_json은_외부서버없이_심볼_접기와_변경된_결과를_소비한다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-json-app-{}", ProjectId::new())),
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
    let file = root.join("current.json");
    std::fs::write(&file, TEXT).unwrap();
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
            name: "owned native JSON".into(),
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
            title: "current.json".into(),
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
            name: "owned native JSON".into(),
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
            fixture.directory.join("bin/taide"),
            Vec::new(),
        )
        .unwrap(),
    );
    let application = fixture.application.as_mut().unwrap();
    let lsp = application.lsp.take().unwrap();
    application.runtime.block_on(lsp.disconnect()).unwrap();
    wait(application, &context, |application| {
        application
            .files
            .get(&path)
            .and_then(|file| application.store.documents().snapshot(file.id).ok())
            .is_some_and(|snapshot| {
                application
                    .editor_symbols
                    .palette(Some(&project), Some(&snapshot))
                    .entries
                    .is_some()
            })
    });
    assert!(application.lsp.is_none());
    let document = application.files[&path].id;
    let snapshot = application.store.documents().snapshot(document).unwrap();
    assert_eq!(snapshot.metadata.language_id, "json");
    let palette = application
        .editor_symbols
        .palette(Some(&project), Some(&snapshot));
    let generation = palette.generation;
    let value = palette
        .entries
        .unwrap()
        .iter()
        .position(|symbol| symbol.name == "value")
        .unwrap();
    let position = application
        .editor_symbols
        .position(&project, &snapshot, generation, value)
        .unwrap();
    assert_eq!((position.line, position.column), (3.0, 5.0));
    assert!(
        application
            .editor_symbols
            .sticky(&project, &snapshot)
            .is_some()
    );
    let folds = application
        .editor_folding
        .model(&project, &snapshot)
        .unwrap();
    assert_eq!(folds.regions().len(), 2);
    let view = application
        .store
        .views()
        .find(&ViewKey {
            window: WINDOW_LABEL.into(),
            pane,
            tab: tab.clone(),
        })
        .unwrap();
    application
        .fold_commands
        .push((tab.clone(), FoldCommand::FoldAll));
    let folded = paint(application, &context);
    assert!(
        !application
            .store
            .views()
            .get(view)
            .unwrap()
            .folds
            .is_empty()
    );
    assert!(!folded.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("value"))));
    application
        .fold_commands
        .push((tab, FoldCommand::UnfoldAll));
    paint(application, &context);
    let start = TEXT.find("value").unwrap();
    application
        .store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: start..start + "value".len(),
                    text: "updated".into(),
                }],
                group: UndoGroup(1),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let current = application.store.documents().snapshot(document).unwrap();
    assert!(
        application
            .editor_symbols
            .position(&project, &current, generation, value)
            .is_none()
    );
    wait(application, &context, |application| {
        application
            .editor_symbols
            .palette(Some(&project), Some(&current))
            .entries
            .is_some_and(|symbols| symbols.iter().any(|symbol| symbol.name == "updated"))
    });
    assert_ne!(
        application
            .editor_symbols
            .palette(Some(&project), Some(&current))
            .generation,
        generation
    );
    assert_eq!(std::fs::read_to_string(file).unwrap(), TEXT);
    const DEEP_OBJECT_COUNT: usize = 2048;
    let deep = format!(
        "{}null{}",
        "{\"long_property_name\":".repeat(DEEP_OBJECT_COUNT),
        "}".repeat(DEEP_OBJECT_COUNT)
    );
    application
        .store
        .apply(
            document,
            Transaction {
                revision: current.revision,
                edits: vec![Edit {
                    bytes: 0..current.rope.len_bytes(),
                    text: deep,
                }],
                group: UndoGroup(2),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    let failed = application.store.documents().snapshot(document).unwrap();
    wait(application, &context, |application| {
        !application
            .editor_symbols
            .palette(Some(&project), Some(&failed))
            .is_pending
    });
    assert!(application.editor_symbols.builtin_model(&failed).is_none());
    assert!(application.toasts.is_showing(
        taide_native_ui::toast::Kind::Error,
        &crate::toast::describe_error(
            &application.locale,
            &editor_error(taide_native_editor::document::EditorError::Capacity)
        )
    ));
    assert!(application.store.undo(document).unwrap());
    let current = application.store.documents().snapshot(document).unwrap();
    wait(application, &context, |application| {
        application.editor_symbols.builtin_model(&current).is_some()
    });
    assert_eq!(current.rope.to_string(), TEXT.replace("value", "updated"));
    let mut metadata = current.metadata.clone();
    metadata.language_id = "plaintext".into();
    application
        .store
        .retarget_file(&current, PathBuf::from(&path), metadata)
        .unwrap();
    wait(application, &context, |application| {
        !application.editor_json.documents().contains(&document)
    });
    let plain = application.store.documents().snapshot(document).unwrap();
    assert!(application.editor_symbols.builtin_model(&plain).is_none());
    let mut metadata = plain.metadata.clone();
    metadata.language_id = "json".into();
    metadata.read_only = true;
    application
        .store
        .retarget_file(&plain, PathBuf::from(&path), metadata)
        .unwrap();
    wait(application, &context, |application| {
        let snapshot = application.store.documents().snapshot(document).unwrap();
        application
            .editor_symbols
            .builtin_model(&snapshot)
            .is_some()
            && application
                .editor_folding
                .builtin_model(&snapshot)
                .is_some()
    });
    let readonly = application.store.documents().snapshot(document).unwrap();
    assert!(readonly.metadata.read_only);
    let mut metadata = readonly.metadata.clone();
    metadata.tier = taide_model::file::FileSizeTier::Large;
    application
        .store
        .retarget_file(&readonly, PathBuf::from(&path), metadata)
        .unwrap();
    wait(application, &context, |application| {
        !application.editor_json.documents().contains(&document)
    });
    let large = application.store.documents().snapshot(document).unwrap();
    assert!(
        !application
            .editor_symbols
            .palette(Some(&project), Some(&large))
            .is_pending
    );
    assert!(application.editor_symbols.builtin_model(&large).is_none());
    assert!(application.editor_folding.builtin_model(&large).is_none());
    let pane = application
        .store
        .views()
        .get(view)
        .unwrap()
        .key
        .pane
        .clone();
    assert!(application.submit(HostCommand::NewUntitled {
        project: project.clone(),
        pane: pane.clone()
    }));
    let runtime = application.runtime.handle().clone();
    let untitled = runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, &context, &mut eframe::Frame::_new_kittest());
                let tab = application
                    .controller
                    .snapshot()
                    .layouts
                    .get(&project)
                    .and_then(|layout| {
                        crate::tabs::tabs_in(&layout.root)
                            .into_iter()
                            .find(|tab| matches!(tab.kind, TabKind::Untitled { .. }))
                            .map(|tab| tab.id.clone())
                    });
                if let Some(tab) = tab {
                    return tab;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    });
    let untitled_document = application
        .store
        .open_untitled(untitled.clone(), TEXT, "json".into())
        .unwrap();
    wait(application, &context, |application| {
        application.untitled.contains_key(&untitled)
            && application
                .store
                .documents()
                .snapshot(untitled_document)
                .is_ok_and(|snapshot| {
                    application
                        .editor_symbols
                        .builtin_model(&snapshot)
                        .is_some()
                })
    });
    let untitled_view = application
        .store
        .views()
        .find(&ViewKey {
            window: WINDOW_LABEL.into(),
            pane,
            tab: untitled.clone(),
        })
        .unwrap();
    let before = application
        .store
        .documents()
        .snapshot(untitled_document)
        .unwrap();
    assert_eq!(
        before.key,
        taide_native_editor::document::DocumentKey::Untitled(untitled)
    );
    application.palette.open(
        &context,
        taide_native_ui::command_registry::PaletteEntry::Files,
    );
    wait(application, &context, |application| {
        application.palette.inspection().has_input_focus
    });
    super::inlay_hints_tests::paint(
        application,
        &context,
        vec![egui::Event::Text("@value".into())],
    );
    paint(application, &context);
    assert_eq!(
        application
            .palette
            .inspection()
            .rows
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        ["value"],
        "palette={:?}; model={:?}",
        application.palette.inspection(),
        application
            .editor_symbols
            .palette(Some(&project), Some(&before))
            .document,
    );
    super::inlay_hints_tests::paint(
        application,
        &context,
        super::inlay_hints_tests::key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    paint(application, &context);
    assert!(!application.palette.is_open());
    assert_eq!(
        application
            .store
            .views()
            .get(untitled_view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        TEXT.find("\"value\"").unwrap()
    );
    let slot = application
        .services
        .state
        .session
        .read()
        .focused_shell_slot
        .clone()
        .unwrap();
    application.symbol_sidebars.entry(&project, &slot).view = crate::symbol_sidebar::View::Outline;
    paint(application, &context);
    let outline = egui::Id::new(("symbol-outline", &project, &slot));
    assert!(context.read_response(outline).is_some());
    context.memory_mut(|memory| memory.request_focus(outline));
    super::inlay_hints_tests::paint(
        application,
        &context,
        super::inlay_hints_tests::key(egui::Key::ArrowDown, egui::Modifiers::NONE),
    );
    super::inlay_hints_tests::paint(
        application,
        &context,
        super::inlay_hints_tests::key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    paint(application, &context);
    assert_eq!(
        application
            .store
            .views()
            .get(untitled_view)
            .unwrap()
            .selection
            .selections[0]
            .head,
        TEXT.find("\"\u{1f600}\"").unwrap()
    );
    let after = application
        .store
        .documents()
        .snapshot(untitled_document)
        .unwrap();
    assert_eq!(
        (after.revision, after.rope.to_string()),
        (before.revision, before.rope.to_string())
    );
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
    assert!(application.editor_json.documents().is_empty());
}

#[test]
fn actual_app의_json_peek는_현재_문서_구조와_키_접기_및_닫힘을_소비한다() {
    use super::inlay_hints_tests::{body_view, fixture_with_view_state_and_setup, key};
    use taide_native_editor::symbol_locations::{Command, Kind, Mode};
    const PEEK_LINE_COUNT: usize = 40;
    const SCROLL_LINE: usize = 10;
    let peek_text = format!(
        "{{\n\"outer\": {{\n{}\n}}\n}}",
        (0..PEEK_LINE_COUNT)
            .map(|index| format!("\"value_{index}\": {index}"))
            .collect::<Vec<_>>()
            .join(",\n")
    );
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-locations-cross-file",
        false,
        |application, project, document| {
            application.lsp.as_ref().is_some_and(|lsp| {
                let snapshot = application.store.documents().snapshot(document).unwrap();
                !lsp.location_providers(project, &snapshot, Kind::Definition)
                    .is_empty()
            })
        },
        None,
        "fn owner() {}",
        |application, path, _, _| {
            let path = std::path::Path::new(path).with_file_name("peek.rs");
            std::fs::write(&path, &peek_text).unwrap();
            let path = path.canonicalize().unwrap();
            application
                .store
                .open_file(
                    path.clone(),
                    taide_model::file::OpenedFile {
                        path: path.to_str().unwrap().into(),
                        content: std::fs::read_to_string(&path).unwrap(),
                        language_id: "json".into(),
                        byte_size: peek_text.len().try_into().unwrap(),
                        line_count: peek_text.lines().count().try_into().unwrap(),
                        tier: taide_model::file::FileSizeTier::Normal,
                        read_only: false,
                        encoding_lossy: false,
                        modified_ms: 0.0,
                        editor_config: Default::default(),
                    },
                )
                .unwrap();
        },
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let owner = body_view(application, &fixture.path, &fixture.tab);
    let owner_document = application.store.views().get(owner).unwrap().document;
    let owner_before = application
        .store
        .documents()
        .snapshot(owner_document)
        .unwrap();
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(Command::Request {
            kind: Kind::Definition,
            mode: Mode::Peek,
        }),
    ));
    wait(application, context, |application| {
        application
            .editor_locations
            .current(owner)
            .is_some_and(|session| session.preview.is_some())
    });
    let preview = application
        .editor_locations
        .current(owner)
        .unwrap()
        .preview
        .unwrap();
    let document = application.store.views().get(preview).unwrap().document;
    assert_ne!(document, owner_document);
    let before = application.store.documents().snapshot(document).unwrap();
    assert_eq!(before.metadata.language_id, "json");
    wait(application, context, |application| {
        let snapshot = application.store.documents().snapshot(document).unwrap();
        application
            .editor_symbols
            .builtin_model(&snapshot)
            .is_some()
            && application
                .editor_folding
                .builtin_model(&snapshot)
                .is_some()
    });
    let current = application.store.documents().snapshot(document).unwrap();
    assert_eq!(current.rope.to_string(), peek_text);
    assert_eq!(
        application
            .editor_symbols
            .palette(Some(&fixture.project), Some(&current))
            .entries
            .unwrap()
            .len(),
        PEEK_LINE_COUNT + 1
    );
    assert!(
        application
            .lsp
            .as_ref()
            .unwrap()
            .location_providers(&fixture.project, &current, Kind::Definition)
            .is_empty()
    );
    application.services.state.settings.write().keymap_overrides =
        Some(r#"[{"actionId":"monaco.editor.foldAll","key":"F9","mods":[]}]"#.into());
    application
        .services
        .state
        .settings
        .write()
        .editor_sticky_scroll_enabled = true;
    paint(application, context);
    let previous = application.store.views().get(preview).unwrap().clone();
    let byte = current.rope.line_to_byte(SCROLL_LINE);
    application
        .store
        .set_view_state(
            preview,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: byte,
                    head: byte,
                }],
            },
            taide_native_editor::view::ScrollPosition {
                x: 0.0,
                y: application.editor.appearance.line_height * SCROLL_LINE as f32,
            },
            previous.folds,
        )
        .unwrap();
    paint(application, context);
    let body = crate::editor_context_menu::body(context, preview).unwrap();
    assert!(
        context
            .read_response(body.with(("sticky-line", 0usize)))
            .is_some(),
        "JSON peek must display its own sticky scope"
    );
    let focus = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (view, _))| {
            (*viewport == context.viewport_id() && *view == preview).then_some(*id)
        })
        .expect("JSON preview must be rendered and register its focus target");
    context.memory_mut(|memory| memory.request_focus(focus));
    super::inlay_hints_tests::paint(
        application,
        context,
        key(egui::Key::F9, egui::Modifiers::NONE),
    );
    paint(application, context);
    assert!(
        !application
            .store
            .views()
            .get(preview)
            .unwrap()
            .folds
            .is_empty()
    );
    assert!(
        application
            .store
            .views()
            .get(owner)
            .unwrap()
            .folds
            .is_empty()
    );
    let owner_after = application
        .store
        .documents()
        .snapshot(owner_document)
        .unwrap();
    assert_eq!(
        (owner_after.revision, owner_after.rope.to_string()),
        (owner_before.revision, owner_before.rope.to_string())
    );
    super::inlay_hints_tests::paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    wait(application, context, |application| {
        !application.editor_json.documents().contains(&document)
    });
    assert!(application.editor_symbols.builtin_model(&current).is_none());
    assert!(application.editor_folding.builtin_model(&current).is_none());
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
