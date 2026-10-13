use super::*;
use std::os::unix::fs::PermissionsExt;
use taide_model::{paths::AppPaths, project::Project};
use taide_native_editor::view::{Selection, SelectionSet};

const TEXT: &str = "class\n  \u{1f600}method\nend";
const DEADLINE: Duration = Duration::from_secs(10);
const FRAME_SIZE: egui::Vec2 = egui::vec2(1000.0, 700.0);
const EXECUTABLE_MODE: u32 = 0o700;
const STABILITY_FRAMES: usize = 16;

pub(super) struct Fixture {
    pub(super) directory: PathBuf,
    pub(super) path: String,
    pub(super) peek_path: String,
    pub(super) project: ProjectId,
    pub(super) tab: TabId,
    pub(super) context: egui::Context,
    pub(super) application: Option<NativeApplication>,
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

pub(super) fn paint(
    application: &mut NativeApplication,
    context: &egui::Context,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
            events,
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    output.textures_delta.clear();
    output
}

pub(super) fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}

pub(super) fn wait(
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
                paint(application, context, Vec::new());
                if ready(application) {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
    });
    let token_kinds = if result.is_err() {
        let documents = application
            .store
            .documents()
            .snapshots()
            .collect::<Vec<_>>();
        documents
            .iter()
            .map(|snapshot| {
                let lease = crate::editor_syntax::SyntaxLease::new(
                    &mut application.editor_syntax,
                    snapshot.id,
                );
                (
                    snapshot.metadata.language_id.clone(),
                    taide_native_editor::language_configuration::LineSyntax::accurate_tokens(
                        &lease, snapshot, 0,
                    ),
                )
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    assert!(
        result.is_ok(),
        "phase={phase}; status={:?}; tasks={}; lsp={:?}; documents={:?}; loading={}; token_kinds={token_kinds:?}",
        application.status,
        application.services.tasks.tracked_count(),
        application
            .controller
            .snapshot()
            .projects
            .iter()
            .map(|project| application
                .lsp
                .as_ref()
                .and_then(|lsp| lsp.summary(&project.id)))
            .collect::<Vec<_>>(),
        application
            .store
            .documents()
            .snapshots()
            .map(|snapshot| (
                snapshot.metadata.language_id,
                snapshot.metadata.tier,
                snapshot.revision
            ))
            .collect::<Vec<_>>(),
        application.loading.len()
    );
}

fn fixture(mode: &str, is_suggest_preview_enabled: bool) -> Fixture {
    fixture_with_provider(
        mode,
        is_suggest_preview_enabled,
        |application, project, document| {
            !application
                .lsp
                .as_ref()
                .unwrap()
                .inlay_hint_providers(
                    project,
                    &application.store.documents().snapshot(document).unwrap(),
                )
                .is_empty()
        },
    )
}

pub(super) fn fixture_with_provider(
    mode: &str,
    is_suggest_preview_enabled: bool,
    provider_ready: impl Fn(&NativeApplication, &ProjectId, DocumentId) -> bool,
) -> Fixture {
    fixture_with_view_state(mode, is_suggest_preview_enabled, provider_ready, None, TEXT)
}

pub(super) fn fixture_with_view_state(
    mode: &str,
    is_suggest_preview_enabled: bool,
    provider_ready: impl Fn(&NativeApplication, &ProjectId, DocumentId) -> bool,
    view_state: Option<String>,
    text: &str,
) -> Fixture {
    fixture_with_view_state_and_setup(
        mode,
        is_suggest_preview_enabled,
        provider_ready,
        view_state,
        text,
        |_, _, _, _| {},
    )
}

pub(super) fn fixture_with_view_state_and_setup(
    mode: &str,
    is_suggest_preview_enabled: bool,
    provider_ready: impl Fn(&NativeApplication, &ProjectId, DocumentId) -> bool,
    view_state: Option<String>,
    text: &str,
    setup: impl FnOnce(&mut NativeApplication, &str, &ProjectId, &TabId),
) -> Fixture {
    fixture_with_ports(
        mode,
        is_suggest_preview_enabled,
        provider_ready,
        view_state,
        text,
        None,
        "rs",
        setup,
    )
}

pub(super) fn fixture_with_view_state_and_secrets(
    mode: &str,
    is_suggest_preview_enabled: bool,
    provider_ready: impl Fn(&NativeApplication, &ProjectId, DocumentId) -> bool,
    text: &str,
    secrets: taide_infra::secret::SecretStoreState,
) -> Fixture {
    fixture_with_ports(
        mode,
        is_suggest_preview_enabled,
        provider_ready,
        None,
        text,
        Some(secrets),
        "rs",
        |_, _, _, _| {},
    )
}

pub(super) fn fixture_with_language(text: &str, extension: &str) -> Fixture {
    fixture_with_ports(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        text,
        None,
        extension,
        |application, path, _, _| {
            application
                .open_with
                .set(path.into(), crate::open_with::Mode::Editor)
        },
    )
}

fn fixture_with_ports(
    mode: &str,
    is_suggest_preview_enabled: bool,
    provider_ready: impl Fn(&NativeApplication, &ProjectId, DocumentId) -> bool,
    view_state: Option<String>,
    text: &str,
    secrets: Option<taide_infra::secret::SecretStoreState>,
    extension: &str,
    setup: impl FnOnce(&mut NativeApplication, &str, &ProjectId, &TabId),
) -> Fixture {
    let directory =
        std::env::temp_dir().join(format!("taide-native-inlay-hints-app-{}", ProjectId::new()));
    let data = directory.join("data");
    let root = directory.join("project");
    let bin = directory.join("bin");
    for path in [&data, &root, &bin] {
        std::fs::create_dir_all(path).unwrap();
    }
    let filename = format!("current.{extension}");
    let file = root.join(&filename);
    let peek = root.join("peek.rs");
    std::fs::write(&file, text).unwrap();
    std::fs::write(&peek, TEXT).unwrap();
    let path = file.canonicalize().unwrap().to_str().unwrap().to_owned();
    let peek_path = peek.canonicalize().unwrap().to_str().unwrap().to_owned();
    let root = root.canonicalize().unwrap().to_str().unwrap().to_owned();
    let state = AppState::new(AppPaths::new(data));
    {
        let mut settings = state.settings.write();
        settings.remote_access_enabled = false;
        settings.ide_integration_enabled = false;
        settings.agent_hooks_enabled = false;
        settings.editor_suggest_preview = is_suggest_preview_enabled;
        settings.language = "en-US".into();
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
            name: "synthetic inlay hints".into(),
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
            title: filename,
            pinned: false,
            preview: false,
            dirty: false,
            view_state,
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
            name: "synthetic inlay hints".into(),
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
    context.set_os(egui::os::OperatingSystem::Mac);
    let application = NativeApplication::new_with_secret_store(
        &eframe::CreationContext::_new_kittest(context.clone()),
        runtime,
        state,
        tasks,
        bin.join("taide"),
        Vec::new(),
        secrets,
    )
    .unwrap();
    let mut fixture = Fixture {
        directory,
        path,
        peek_path,
        project,
        tab,
        context,
        application: Some(application),
    };
    let application = fixture.application.as_mut().unwrap();
    setup(application, &fixture.path, &fixture.project, &fixture.tab);
    paint(application, &fixture.context, Vec::new());
    wait(application, &fixture.context, "document", |application| {
        application.files.contains_key(&fixture.path)
    });
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
    std::fs::write(&executable, format!("#!/bin/sh\nexec '{mock}' {mode}\n")).unwrap();
    std::fs::set_permissions(
        &executable,
        std::fs::Permissions::from_mode(EXECUTABLE_MODE),
    )
    .unwrap();
    application
        .runtime
        .handle()
        .clone()
        .block_on(application.lsp.take().unwrap().disconnect())
        .unwrap();
    let repaint = fixture.context.clone();
    application.lsp = Some(
        crate::lsp::LspBridge::connect(
            application.services.clone(),
            bin.into_os_string(),
            Arc::new(move || repaint.request_repaint()),
        )
        .unwrap(),
    );
    let document = application.files[&fixture.path].id;
    wait(application, &fixture.context, "provider", |application| {
        provider_ready(application, &fixture.project, document)
    });
    paint(application, &fixture.context, Vec::new());
    fixture
}

pub(super) fn body_view(application: &NativeApplication, path: &str, tab: &TabId) -> ViewId {
    application
        .store
        .views()
        .for_document(application.files[path].id)
        .find(|view| &view.key.tab == tab)
        .unwrap()
        .id
}

fn hints(
    application: &NativeApplication,
    context: &egui::Context,
    view: ViewId,
) -> Option<taide_native_ui::editor_inlay_hints::Presentation> {
    application.editor_inlay_hints.display(
        &application.store,
        context.viewport_id(),
        view,
        application.editor_inlay_hint_colors,
    )
}

pub(super) fn text_contains(output: &egui::FullOutput, label: &str) -> bool {
    output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains(label)))
}

#[test]
fn 실제_app의_ghost_미리보기와_힌트는_공존하고_수락과_undo는_원문만_편집한다() {
    let mut fixture = fixture("--native-inlay-hints", true);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = body_view(application, &fixture.path, &fixture.tab);
    wait(application, context, "initial hints", |application| {
        hints(application, context, view).is_some_and(|hints| hints.items.len() == 3)
    });
    let before = application.store.documents().snapshot(document).unwrap();
    let caret = TEXT.find("\nend").unwrap();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: caret,
                    head: caret,
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    let editor_id = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == context.viewport_id() && *target == view).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(editor_id));
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Completion(taide_native_editor::completion::Command::Trigger),
    ));
    wait(application, context, "completion", |application| {
        application
            .editor_completion
            .request(&application.store, view)
            .is_some()
            && !application
                .editor_completion
                .pending(&application.store, view)
    });
    paint(application, context, Vec::new());
    let display = application.store.take_display(view).unwrap();
    let map = display.as_ref().unwrap();
    assert!(!map.preview_views().is_empty());
    assert_eq!(map.inlay_hints().len(), 3);
    assert!(
        map.row_projection(1)
            .unwrap()
            .units
            .iter()
            .any(|unit| unit.injected && unit.hint.is_none())
    );
    assert!(
        map.row_projection(1)
            .unwrap()
            .units
            .iter()
            .any(|unit| unit.hint.is_some() && !unit.is_preview)
    );
    application.store.set_display(view, display).unwrap();
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        before.revision
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Completion(taide_native_editor::completion::Command::Accept {
            alternate: false,
        }),
    ));
    paint(application, context, Vec::new());
    let accepted = application
        .store
        .documents()
        .snapshot(document)
        .unwrap()
        .rope
        .to_string();
    assert!(accepted.contains("method(a)"), "{accepted}");
    assert!(
        !accepted.contains("arg: T") && !accepted.contains("default") && !accepted.contains(": T0")
    );
    paint(
        application,
        context,
        key(
            egui::Key::Z,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
    wait(application, context, "undo hints", |application| {
        hints(application, context, view).is_some_and(|hints| hints.items.len() == 3)
    });
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
}

#[test]
fn 실제_app의_본문과_peek에_서버_힌트를_표시하고_편집_readonly_tier와_종료를_회수한다() {
    let mut fixture = fixture("--native-inlay-hints", false);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = body_view(application, &fixture.path, &fixture.tab);
    wait(application, context, "body hints", |application| {
        hints(application, context, view).is_some_and(|hints| hints.items.len() == 3)
    });
    let initial = application
        .editor_inlay_hints
        .request(context.viewport_id(), view)
        .unwrap()
        .clone();
    let before = application.store.documents().snapshot(document).unwrap();
    let output = paint(application, context, Vec::new());
    assert!(text_contains(&output, ":\u{a0}T0"));
    assert!(text_contains(&output, "arg:\u{a0}T"));
    assert_eq!(before.rope.to_string(), TEXT);
    for _ in 0..STABILITY_FRAMES {
        paint(application, context, Vec::new());
    }
    assert_eq!(
        application
            .editor_inlay_hints
            .request(context.viewport_id(), view)
            .unwrap()
            .token,
        initial.token
    );

    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Request {
            kind: taide_native_editor::symbol_locations::Kind::Definition,
            mode: taide_native_editor::symbol_locations::Mode::Peek,
        }),
    ));
    wait(application, context, "peek shown", |application| {
        application
            .editor_locations
            .current(view)
            .is_some_and(|session| session.preview.is_some())
    });
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    wait(application, context, "peek hints", |application| {
        hints(application, context, preview).is_some_and(|hints| hints.items.len() == 3)
    });
    assert_eq!(
        application.store.views().get(preview).unwrap().document,
        document
    );
    assert!(hints(application, context, view).unwrap().items.len() == 3);
    application.document_edits.push((
        fixture.tab.clone(),
        DocumentEdit::Location(taide_native_editor::symbol_locations::Command::Close),
    ));
    wait(application, context, "peek close", |application| {
        application.editor_locations.current(view).is_none()
            && application
                .editor_inlay_hints
                .request(context.viewport_id(), preview)
                .is_none()
    });
    assert!(
        application
            .editor_inlay_hints
            .request(context.viewport_id(), preview)
            .is_none()
    );

    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: TEXT.len(),
                    head: TEXT.len(),
                }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    let editor_id = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (target, _))| {
            (*viewport == context.viewport_id() && *target == view).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(editor_id));
    paint(application, context, vec![egui::Event::Text("x".into())]);
    assert!(initial.is_cancelled());
    assert!(hints(application, context, view).is_none());
    wait(application, context, "latest mirror hints", |application| {
        hints(application, context, view)
            .is_some_and(|hints| hints.items.iter().any(|hint| hint.label == ": T1"))
    });
    assert!(text_contains(
        &paint(application, context, Vec::new()),
        ":\u{a0}T1"
    ));
    let edited = application.store.documents().snapshot(document).unwrap();
    assert_eq!(edited.rope.to_string(), format!("{TEXT}x"));
    assert_eq!(edited.revision, before.revision + 1);

    for tier in [
        taide_model::file::FileSizeTier::Large,
        taide_model::file::FileSizeTier::ReadOnly,
        taide_model::file::FileSizeTier::Normal,
    ] {
        application
            .store
            .observe_file(
                document,
                std::path::Path::new(&fixture.path),
                taide_model::file::OpenedFile {
                    path: fixture.path.clone(),
                    content: edited.rope.to_string(),
                    language_id: "rust".into(),
                    byte_size: edited.rope.len_bytes().try_into().unwrap(),
                    line_count: edited.rope.len_lines().try_into().unwrap(),
                    tier,
                    read_only: true,
                    encoding_lossy: false,
                    modified_ms: 0.0,
                    editor_config: Default::default(),
                },
            )
            .unwrap();
        paint(application, context, Vec::new());
        if tier == taide_model::file::FileSizeTier::Normal {
            wait(application, context, "readonly hints", |application| {
                hints(application, context, view).is_some_and(|hints| !hints.items.is_empty())
            });
            paint(
                application,
                context,
                vec![egui::Event::Text("blocked".into())],
            );
            assert_eq!(
                application
                    .store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                edited.rope.to_string()
            );
        } else {
            assert!(hints(application, context, view).is_none());
            assert!(!text_contains(
                &paint(application, context, Vec::new()),
                "arg:\u{a0}T"
            ));
        }
    }
    let final_request = application
        .editor_inlay_hints
        .request(context.viewport_id(), view)
        .unwrap()
        .clone();
    eframe::App::on_exit(application);
    assert!(final_request.is_cancelled());
    assert_eq!(application.services.tasks.tracked_count(), 0);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    assert_eq!(std::fs::read_to_string(&fixture.peek_path).unwrap(), TEXT);
}
