use super::*;
use std::os::unix::fs::PermissionsExt;
use taide_model::{paths::AppPaths, project::Project};
use taide_native_editor::view::{Selection, SelectionSet};
use taide_native_ui::editor_code_actions::Command;

const TEXT: &str = "class\n  \u{1f600}method\nend";
const DEADLINE: Duration = Duration::from_secs(10);
const FRAME_SIZE: egui::Vec2 = egui::vec2(1000.0, 700.0);
const EXECUTABLE_MODE: u32 = 0o700;
const STABILITY_FRAMES: usize = 16;
const CLIENT_REFERENCE_STEPS: usize = 2;

struct Fixture {
    directory: PathBuf,
    path: String,
    peek_path: String,
    project: ProjectId,
    tab: TabId,
    context: egui::Context,
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

fn paint(application: &mut NativeApplication, context: &egui::Context, events: Vec<egui::Event>) {
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
            events,
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    output.textures_delta.clear();
}

fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
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

fn wait(
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
        "phase={phase}; status={:?}; tasks={}; request={:?}; applying={}; focus={:?}; source={:?}",
        application.status,
        application.services.tasks.tracked_count(),
        application
            .editor_code_actions
            .state
            .request()
            .map(|request| (
                request.invocation.command,
                request.invocation.automatic,
                request.is_cancelled()
            )),
        application.editor_code_actions.state.is_applying(),
        context.memory(|memory| memory.focused()),
        application
            .editor_code_actions
            .state
            .request()
            .and_then(|request| application
                .store
                .documents()
                .snapshot(request.snapshot.id)
                .ok())
            .map(|snapshot| snapshot.rope.to_string())
    );
}

fn fixture(mode: &str) -> Fixture {
    let directory = std::env::temp_dir().join(format!(
        "taide-native-code-actions-app-{}",
        ProjectId::new()
    ));
    let data = directory.join("data");
    let root = directory.join("project");
    let bin = directory.join("bin");
    for path in [&data, &root, &bin] {
        std::fs::create_dir_all(path).unwrap();
    }
    let file = root.join("current.rs");
    let peek = root.join("peek.rs");
    std::fs::write(&file, TEXT).unwrap();
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
            name: "synthetic code actions".into(),
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
            name: "synthetic code actions".into(),
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
    let application = NativeApplication::new(
        &eframe::CreationContext::_new_kittest(context.clone()),
        runtime,
        state,
        tasks,
        bin.join("taide"),
        Vec::new(),
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
        if mode == "--native-code-actions-unsupported" {
            return application
                .lsp
                .as_ref()
                .unwrap()
                .code_action_diagnostics(&fixture.project, document)
                .values()
                .any(|revision| *revision != 0);
        }
        !application
            .lsp
            .as_ref()
            .unwrap()
            .code_action_providers(
                &fixture.project,
                &application.store.documents().snapshot(document).unwrap(),
                Command::QuickFix,
            )
            .is_empty()
    });
    paint(application, &fixture.context, Vec::new());
    fixture
}

fn focus(application: &mut NativeApplication, context: &egui::Context, source: ViewId) {
    paint(application, context, Vec::new());
    let id = application
        .editor_keymap_targets
        .iter()
        .find_map(|((viewport, id), (view, _))| {
            (*viewport == context.viewport_id() && *view == source).then_some(*id)
        })
        .unwrap();
    context.memory_mut(|memory| memory.request_focus(id));
}

fn select_first(application: &mut NativeApplication, context: &egui::Context, source: ViewId) {
    select_with_modifiers(application, context, source, egui::Modifiers::NONE);
}

fn select_with_modifiers(
    application: &mut NativeApplication,
    context: &egui::Context,
    source: ViewId,
    modifiers: egui::Modifiers,
) {
    wait(application, context, "list", |application| {
        application
            .editor_code_actions
            .state
            .listed(&application.store, source)
            .is_some_and(|(request, _, open)| !request.invocation.automatic && open)
    });
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    paint(application, context, key(egui::Key::Enter, modifiers));
}

fn opened_navigation(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &ProjectId,
    path: &str,
) -> ViewId {
    wait(application, context, "navigation-tab", |application| {
        application
            .services
            .state
            .layouts
            .read()
            .get(project)
            .is_some_and(|layout| {
                let taide_model::layout::PaneNode::Leaf { tabs, .. } = &layout.root else {
                    return false;
                };
                tabs.iter().any(
                    |tab| matches!(&tab.kind, TabKind::File { path: actual } if actual == path),
                )
            })
    });
    paint(application, context, Vec::new());
    wait(application, context, "navigation-file", |application| {
        application.files.contains_key(path)
    });
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    let layouts = application.services.state.layouts.read();
    let taide_model::layout::PaneNode::Leaf { tabs, .. } = &layouts[project].root else {
        panic!("fixture navigation must retain its pane");
    };
    let tab = tabs
        .iter()
        .find(|tab| matches!(&tab.kind, TabKind::File { path: actual } if actual == path))
        .unwrap();
    application
        .store
        .views()
        .for_document(application.files[path].id)
        .find(|view| view.key.tab == tab.id)
        .unwrap()
        .id
}

#[test]
fn 실제_app의_다른_uri_참조_명령은_원본_파일과_utf16_위치에_peek를_옮긴다() {
    for mode in [
        "--native-code-actions-references-other",
        "--native-code-actions-ra-references-other",
    ] {
        let mut fixture = fixture(mode);
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let document = application.files[&fixture.path].id;
        let view = application
            .store
            .views()
            .for_document(document)
            .find(|view| view.key.tab == fixture.tab)
            .unwrap()
            .id;
        let before = application.store.documents().snapshot(document).unwrap();
        let selection = application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .clone();
        focus(application, context, view);
        paint(
            application,
            context,
            key(
                egui::Key::Period,
                egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
            ),
        );
        select_first(application, context, view);
        let origin = opened_navigation(application, context, &fixture.project, &fixture.peek_path);
        let session = application.editor_locations.current(origin).unwrap();
        assert!(session.shown);
        assert_eq!(
            session.request.position,
            taide_lsp::native::protocol::lsp_types::Position::new(1, 4)
        );
        assert_eq!(
            session.request.snapshot.id,
            application.files[&fixture.peek_path].id
        );
        assert_eq!(session.model.as_ref().unwrap().targets().len(), 1);
        let preview = session.preview.unwrap();
        assert_eq!(
            application.store.views().get(preview).unwrap().document,
            document
        );
        assert_eq!(
            application.store.views().get(view).unwrap().selection,
            selection
        );
        assert!(application.editor_locations.current(view).is_none());
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
                .rope,
            before.rope
        );
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
        assert_eq!(std::fs::read_to_string(&fixture.peek_path).unwrap(), TEXT);
        eframe::App::on_exit(application);
        assert_eq!(application.services.tasks.tracked_count(), 0);
    }
}

#[test]
fn 실제_app의_client_goto는_location_link의_선택_범위_시작으로_이동한다() {
    let mut fixture = fixture("--native-code-actions-goto-other");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == fixture.tab)
        .unwrap()
        .id;
    focus(application, context, view);
    paint(
        application,
        context,
        key(
            egui::Key::Period,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    select_first(application, context, view);
    let target = opened_navigation(application, context, &fixture.project, &fixture.peek_path);
    let selections = &application.store.views().get(target).unwrap().selection;
    let selection = &selections.selections[selections.primary];
    let start = TEXT.find("method").unwrap();
    assert_eq!(selection.anchor, start);
    assert_eq!(selection.head, start);
    assert!(application.editor_locations.current(target).is_none());
    assert!(application.editor_locations.current(view).is_none());
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    assert_eq!(std::fs::read_to_string(&fixture.peek_path).unwrap(), TEXT);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

fn wait_prefix(
    application: &mut NativeApplication,
    context: &egui::Context,
    document: DocumentId,
    prefix: &str,
) {
    wait(application, context, prefix, |application| {
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
            .starts_with(prefix)
            && !application.editor_code_actions.state.is_applying()
    });
}

#[test]
fn 실제_app의_여섯_수정_명령은_키와_팔레트에서_편집_후_명령과_미리보기_소유를_보존한다() {
    let mut fixture = fixture("--native-code-actions");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == fixture.tab)
        .unwrap()
        .id;
    let head = TEXT.find("method").unwrap();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    focus(application, context, view);
    paint(
        application,
        context,
        key(
            egui::Key::Period,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    assert!(
        application
            .editor_code_actions
            .state
            .request()
            .is_some_and(|request| !request.invocation.automatic),
        "manual Quick Fix was not dispatched"
    );
    select_with_modifiers(
        application,
        context,
        view,
        egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
    );
    assert!(application.editor_code_actions.state.request().is_some());
    assert!(
        application
            .editor_code_actions
            .state
            .listed(&application.store, view)
            .is_none()
    );
    wait_prefix(application, context, document, "applied:");
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    assert!(application.store.undo(document).unwrap());
    for (key_name, modifiers, prefix, needs_selection) in [
        (
            egui::Key::R,
            egui::Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
            "extract:",
            true,
        ),
        (
            egui::Key::O,
            egui::Modifiers {
                alt: true,
                shift: true,
                ..Default::default()
            },
            "imports:",
            false,
        ),
        (
            egui::Key::Period,
            egui::Modifiers {
                alt: true,
                command: true,
                mac_cmd: true,
                ..Default::default()
            },
            "applied:",
            false,
        ),
    ] {
        focus(application, context, view);
        paint(application, context, key(key_name, modifiers));
        if needs_selection {
            select_first(application, context, view);
        }
        wait_prefix(application, context, document, prefix);
        assert!(application.store.undo(document).unwrap());
    }
    for (id, prefix, needs_selection) in [
        ("monaco.editor.action.sourceAction", "imports:", true),
        ("monaco.editor.action.fixAll", "all:", false),
    ] {
        focus(application, context, view);
        application.palette.open(
            context,
            taide_native_ui::command_registry::PaletteEntry::Commands,
        );
        paint(application, context, Vec::new());
        paint(application, context, Vec::new());
        let label = crate::command_registry::registry()
            .unwrap()
            .command(id)
            .unwrap()
            .label(&application.locale);
        paint(application, context, vec![egui::Event::Text(label)]);
        assert!(
            application
                .palette
                .inspection()
                .rows
                .iter()
                .any(|row| row.key == id && row.is_enabled && row.is_selected),
            "{:?}",
            application.palette.inspection()
        );
        paint(
            application,
            context,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        if needs_selection {
            select_first(application, context, view);
        }
        wait_prefix(application, context, document, prefix);
        assert!(application.store.undo(document).unwrap());
    }
    focus(application, context, view);
    paint(
        application,
        context,
        key(
            egui::Key::Period,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    wait(application, context, "client-list", |application| {
        application
            .editor_code_actions
            .state
            .listed(&application.store, view)
            .is_some()
    });
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    for _ in 0..CLIENT_REFERENCE_STEPS {
        paint(
            application,
            context,
            key(egui::Key::ArrowDown, egui::Modifiers::NONE),
        );
    }
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    wait(application, context, "client-peek", |application| {
        application
            .editor_locations
            .current(view)
            .is_some_and(|session| session.shown)
    });
    paint(application, context, Vec::new());
    wait(application, context, "peek-read", |application| {
        application
            .store
            .documents()
            .find(&taide_native_editor::document::DocumentKey::File(
                PathBuf::from(&fixture.peek_path),
            ))
            .is_some()
    });
    paint(application, context, Vec::new());
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    let peek_document = application.store.views().get(preview).unwrap().document;
    wait(application, context, "peek-provider", |application| {
        !application
            .lsp
            .as_ref()
            .unwrap()
            .code_action_providers(
                &fixture.project,
                &application
                    .store
                    .documents()
                    .snapshot(peek_document)
                    .unwrap(),
                Command::QuickFix,
            )
            .is_empty()
    });
    let mut readonly =
        taide_file::service::open_file(std::path::Path::new(&fixture.path), &[], false).unwrap();
    readonly.read_only = true;
    application
        .store
        .observe_file(document, std::path::Path::new(&fixture.path), readonly)
        .unwrap();
    let owner_before = application.store.documents().snapshot(document).unwrap();
    focus(application, context, preview);
    paint(
        application,
        context,
        key(
            egui::Key::Period,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    select_first(application, context, preview);
    wait_prefix(application, context, peek_document, "applied:");
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .revision,
        owner_before.revision
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope,
        owner_before.rope
    );
    assert!(application.editor_locations.current(view).unwrap().shown);
    assert_eq!(std::fs::read_to_string(&fixture.peek_path).unwrap(), TEXT);
    assert!(application.store.undo(peek_document).unwrap());
    eframe::App::on_exit(application);
    assert!(application.editor_code_actions.state.request().is_none());
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_app의_빈_미지원_수정은_커서_안내와_escape_후_본문_입력을_보존한다() {
    for mode in [
        "--native-code-actions-null",
        "--native-code-actions-unsupported",
    ] {
        let mut fixture = fixture(mode);
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let document = application.files[&fixture.path].id;
        let view = application
            .store
            .views()
            .for_document(document)
            .find(|view| view.key.tab == fixture.tab)
            .unwrap()
            .id;
        focus(application, context, view);
        paint(
            application,
            context,
            key(
                egui::Key::Period,
                egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
            ),
        );
        wait(application, context, "unavailable-message", |application| {
            application
                .editor_code_actions
                .state
                .message(&application.store, view)
                .is_some_and(|message| message.text == "No code actions available")
        });
        paint(application, context, Vec::new());
        paint(application, context, Vec::new());
        let token = application
            .editor_code_actions
            .state
            .request()
            .unwrap()
            .token;
        let id = application
            .editor_keymap_targets
            .iter()
            .find_map(|((viewport, id), (source, _))| {
                (*viewport == context.viewport_id() && *source == view).then_some(*id)
            })
            .unwrap();
        assert!(
            context
                .read_response(taide_native_ui::editor_code_actions::message_id(
                    id,
                    token.as_u128()
                ))
                .is_some()
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
        paint(
            application,
            context,
            key(egui::Key::Escape, egui::Modifiers::NONE),
        );
        assert!(
            application
                .editor_code_actions
                .state
                .message(&application.store, view)
                .is_none()
        );
        paint(application, context, vec![egui::Event::Text("x".into())]);
        assert_eq!(
            application
                .store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            format!("x{TEXT}")
        );
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
        eframe::App::on_exit(application);
        assert_eq!(application.services.tasks.tracked_count(), 0);
    }
}

#[test]
fn 실제_app의_비활성_자동수정_안내는_본문과_peek에서_사유와_입력_소유를_보존한다() {
    let mut fixture = fixture("--native-code-actions-disabled");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == fixture.tab)
        .unwrap()
        .id;
    let modifiers = egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    focus(application, context, view);
    paint(application, context, key(egui::Key::Period, modifiers));
    wait(
        application,
        context,
        "disabled-root-message",
        |application| {
            application
                .editor_code_actions
                .state
                .message(&application.store, view)
                .is_some_and(|message| message.text == "Unavailable here")
        },
    );
    paint(application, context, Vec::new());
    paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    assert!(
        application
            .editor_code_actions
            .state
            .message(&application.store, view)
            .is_none()
    );
    paint(
        application,
        context,
        key(
            egui::Key::Period,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    select_first(application, context, view);
    wait(application, context, "disabled-peek", |application| {
        application
            .editor_locations
            .current(view)
            .is_some_and(|session| session.shown)
    });
    paint(application, context, Vec::new());
    wait(application, context, "disabled-peek-read", |application| {
        application
            .store
            .documents()
            .find(&taide_native_editor::document::DocumentKey::File(
                PathBuf::from(&fixture.peek_path),
            ))
            .is_some()
    });
    paint(application, context, Vec::new());
    let preview = application
        .editor_locations
        .current(view)
        .unwrap()
        .preview
        .unwrap();
    let peek_document = application.store.views().get(preview).unwrap().document;
    wait(
        application,
        context,
        "disabled-peek-provider",
        |application| {
            !application
                .lsp
                .as_ref()
                .unwrap()
                .code_action_providers(
                    &fixture.project,
                    &application
                        .store
                        .documents()
                        .snapshot(peek_document)
                        .unwrap(),
                    Command::AutoFix,
                )
                .is_empty()
        },
    );
    let mut readonly =
        taide_file::service::open_file(std::path::Path::new(&fixture.path), &[], false).unwrap();
    readonly.read_only = true;
    application
        .store
        .observe_file(document, std::path::Path::new(&fixture.path), readonly)
        .unwrap();
    let before = application.store.documents().snapshot(document).unwrap();
    focus(application, context, preview);
    paint(application, context, key(egui::Key::Period, modifiers));
    wait(
        application,
        context,
        "disabled-peek-message",
        |application| {
            application
                .editor_code_actions
                .state
                .message(&application.store, preview)
                .is_some_and(|message| message.text == "Unavailable here")
        },
    );
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    assert!(
        application
            .editor_code_actions
            .state
            .message(&application.store, preview)
            .is_none()
    );
    assert!(application.editor_locations.current(view).unwrap().shown);
    let selections = &application.store.views().get(preview).unwrap().selection;
    let selection = &selections.selections[selections.primary];
    let mut expected = application
        .store
        .documents()
        .snapshot(peek_document)
        .unwrap()
        .rope
        .to_string();
    expected.replace_range(
        selection.anchor.min(selection.head)..selection.anchor.max(selection.head),
        "x",
    );
    paint(application, context, vec![egui::Event::Text("x".into())]);
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(peek_document)
            .unwrap()
            .rope
            .to_string(),
        expected
    );
    let after = application.store.documents().snapshot(document).unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.rope, before.rope);
    assert!(after.metadata.read_only);
    assert_eq!(std::fs::read_to_string(&fixture.peek_path).unwrap(), TEXT);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_app의_명시_수정은_공백_위치에서도_근접_진단을_조회한다() {
    let mut fixture = fixture("--native-code-actions-nearby");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == fixture.tab)
        .unwrap()
        .id;
    let head = TEXT.find('\n').unwrap() + 1;
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    focus(application, context, view);
    paint(
        application,
        context,
        key(
            egui::Key::Period,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    wait(application, context, "nearby-list", |application| {
        application
            .editor_code_actions
            .state
            .listed(&application.store, view)
            .is_some_and(|(_, actions, open)| open && actions.len() == 2)
    });
    select_first(application, context, view);
    wait_prefix(application, context, document, "near:");
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    assert!(application.store.undo(document).unwrap());
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_app의_코드_수정은_닫힌_파일과_생성_이름변경을_명령_전에_적용한다() {
    let mut fixture = fixture("--native-code-actions-resource");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == fixture.tab)
        .unwrap()
        .id;
    let root = std::path::Path::new(&fixture.path).parent().unwrap();
    assert!(!application.files.contains_key(&fixture.peek_path));
    assert!(!root.join("created.rs").exists());
    assert!(!root.join("generated.rs").exists());
    focus(application, context, view);
    paint(
        application,
        context,
        key(
            egui::Key::Period,
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND,
        ),
    );
    select_first(application, context, view);
    wait_prefix(application, context, document, "resource:");
    assert_eq!(
        std::fs::read_to_string(&fixture.peek_path).unwrap(),
        format!("closed:{TEXT}")
    );
    assert!(!root.join("created.rs").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("generated.rs")).unwrap(),
        "created"
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    assert!(!application.files.contains_key(&fixture.peek_path));
    assert!(application.store.undo(document).unwrap());
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_app의_자동_전구는_공백_왕복과_목록_취소_후에도_다시_사용한다() {
    let mut fixture = fixture("--native-code-actions");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == fixture.tab)
        .unwrap()
        .id;
    let head = TEXT.find("method").unwrap();
    let move_to = |application: &mut NativeApplication, head| {
        application
            .store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection { anchor: head, head }],
                },
                Default::default(),
                Vec::new(),
            )
            .unwrap();
        focus(application, context, view);
        paint(application, context, Vec::new());
    };
    move_to(application, head);
    wait(application, context, "automatic-actions", |application| {
        application
            .editor_code_actions
            .state
            .listed(&application.store, view)
            .is_some_and(|(request, actions, open)| {
                request.invocation.automatic && !actions.is_empty() && !open
            })
    });
    let previous = application
        .editor_code_actions
        .state
        .request()
        .unwrap()
        .clone();
    move_to(application, TEXT.find('\n').unwrap() + 1);
    assert!(application.editor_code_actions.state.request().is_none());
    assert!(previous.is_cancelled());
    move_to(application, head);
    wait(
        application,
        context,
        "automatic-return-from-whitespace",
        |application| {
            application
                .editor_code_actions
                .state
                .listed(&application.store, view)
                .is_some_and(|(request, actions, _)| {
                    request.token != previous.token && !actions.is_empty()
                })
        },
    );
    let request = application
        .editor_code_actions
        .state
        .request()
        .unwrap()
        .clone();
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    let editor = context.memory(|memory| memory.focused()).unwrap();
    let bulb = context
        .read_response(taide_native_ui::editor_code_actions::bulb_id(
            editor,
            request.token.as_u128(),
        ))
        .unwrap();
    let selection = application
        .store
        .views()
        .get(view)
        .unwrap()
        .selection
        .clone();
    for pressed in [true, false] {
        paint(
            application,
            context,
            vec![
                egui::Event::PointerMoved(bulb.rect.center()),
                egui::Event::PointerButton {
                    pos: bulb.rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert_eq!(
        application.store.views().get(view).unwrap().selection,
        selection
    );
    assert!(
        application
            .editor_code_actions
            .state
            .listed(&application.store, view)
            .is_some_and(|(_, _, open)| open)
    );
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    assert!(!request.is_cancelled());
    assert!(
        application
            .editor_code_actions
            .state
            .listed(&application.store, view)
            .is_some_and(|(current, actions, open)| current.token == request.token
                && !actions.is_empty()
                && !open)
    );
    for _ in 0..STABILITY_FRAMES {
        paint(application, context, Vec::new());
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    }
    assert_eq!(
        application
            .editor_code_actions
            .state
            .request()
            .unwrap()
            .token,
        request.token
    );
    assert!(
        application
            .editor_code_actions
            .next_refresh(Instant::now())
            .is_none()
    );
    assert!(application.editor_code_actions.state.open(
        &application.store,
        view,
        request.token.as_u128()
    ));
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    paint(
        application,
        context,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    wait_prefix(application, context, document, "applied:");
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

#[test]
fn 실제_app의_빈_자동_응답은_반복하지_않고_문서_변경_후에만_새_요청을_만든다() {
    let mut fixture = fixture("--native-code-actions-error");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let document = application.files[&fixture.path].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .find(|view| view.key.tab == fixture.tab)
        .unwrap()
        .id;
    let head = TEXT.find("method").unwrap();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            },
            Default::default(),
            Vec::new(),
        )
        .unwrap();
    focus(application, context, view);
    paint(application, context, Vec::new());
    wait(application, context, "automatic-empty", |application| {
        application
            .editor_code_actions
            .state
            .listed(&application.store, view)
            .is_some_and(|(request, actions, open)| {
                request.invocation.automatic && actions.is_empty() && !open
            })
    });
    let token = application
        .editor_code_actions
        .state
        .request()
        .unwrap()
        .token;
    for _ in 0..STABILITY_FRAMES {
        paint(application, context, Vec::new());
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
    }
    assert_eq!(
        application
            .editor_code_actions
            .state
            .request()
            .unwrap()
            .token,
        token
    );
    assert!(
        application
            .editor_code_actions
            .next_refresh(Instant::now())
            .is_none()
    );
    paint(application, context, vec![egui::Event::Text("x".into())]);
    paint(application, context, Vec::new());
    wait(
        application,
        context,
        "changed-automatic-empty",
        |application| {
            application
                .editor_code_actions
                .state
                .listed(&application.store, view)
                .is_some_and(|(request, actions, _)| request.token != token && actions.is_empty())
        },
    );
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
