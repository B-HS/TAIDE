use super::*;
use taide_model::{paths::AppPaths, project::Project};
use taide_native_editor::document::{Edit, UndoGroup};
use taide_native_editor::store::Transaction;

const DEADLINE: Duration = Duration::from_secs(10);
const FRAME_SIZE: egui::Vec2 = egui::vec2(1000.0, 700.0);
const TEXT: &str = "{\n  \"\u{1f600}\": {\n    \"value\": 1\n  },\n  \"list\": [1, 2]\n}";
const INDEX: &[u8] = b"<!doctype html><title>owned native JSON fixture</title>";
const MODULE: &[u8] = b"\0asm\x01\0\0\0";
const MAX_SELECTION_EXPANSIONS: usize = 8;
const MAX_SAVE_UNDOS: usize = 4;
const SMART_SELECTION_MODIFIERS: egui::Modifiers = egui::Modifiers {
    alt: false,
    ctrl: true,
    shift: true,
    mac_cmd: true,
    command: true,
};

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

#[track_caller]
fn wait(
    application: &mut NativeApplication,
    context: &egui::Context,
    ready: impl Fn(&NativeApplication) -> bool,
) {
    let caller = std::panic::Location::caller();
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
        .unwrap_or_else(|error| {
            let shell = application.controller.snapshot();
            let runtime_layouts = application.services.state.layouts.read().keys().cloned().collect::<Vec<_>>();
            let runtime_projects = application.services.state.projects.read().keys().cloned().collect::<Vec<_>>();
            panic!(
                "{caller}: {error}; project={:?}; snapshot layouts={:?}; runtime layouts={runtime_layouts:?}; runtime projects={runtime_projects:?}; status={:?}; native={:?}; validation={:?}; previews={:?}; views={:?}; focus={:?}; tasks={}",
                shell.focused_project(), shell.layouts.keys().collect::<Vec<_>>(), application.status,
                application.editor_json.documents(),
                application.editor_json_validation.documents(),
                application.editor_locations.preview_views(),
                application.store.documents().snapshots().flat_map(|snapshot| application.store.views().for_document(snapshot.id).map(|view| (snapshot.id, view.id, view.key.clone())).collect::<Vec<_>>()).collect::<Vec<_>>(),
                context.memory(|memory| memory.focused()),
                application.services.tasks.tracked_count(),
            )
        });
    });
}

fn assert_json_selection(
    application: &mut NativeApplication,
    context: &egui::Context,
    view: ViewId,
    property: &str,
) {
    let previous = application.store.views().get(view).unwrap().clone();
    let before = application
        .store
        .documents()
        .snapshot(previous.document)
        .unwrap();
    let text = before.rope.to_string();
    let start = text.find(property).unwrap();
    let caret = start + 2;
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: caret,
                    head: caret,
                }],
            },
            previous.scroll.clone(),
            previous.folds.clone(),
        )
        .unwrap();
    paint(application, context);
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    let mut expansions = 0;
    for _ in 0..MAX_SELECTION_EXPANSIONS {
        let selection = application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .clone();
        super::inlay_hints_tests::paint(
            application,
            context,
            super::inlay_hints_tests::key(egui::Key::ArrowRight, SMART_SELECTION_MODIFIERS),
        );
        wait(application, context, |application| {
            application.store.views().get(view).unwrap().selection != selection
        });
        expansions += 1;
        let selected = application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections[0];
        if selected.anchor == start && selected.head == start + property.len() {
            break;
        }
    }
    let selected = application
        .store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections[0];
    assert_eq!(
        (selected.anchor, selected.head),
        (start, start + property.len())
    );
    for _ in 0..expansions {
        let selection = application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .clone();
        super::inlay_hints_tests::paint(
            application,
            context,
            super::inlay_hints_tests::key(egui::Key::ArrowLeft, SMART_SELECTION_MODIFIERS),
        );
        wait(application, context, |application| {
            application.store.views().get(view).unwrap().selection != selection
        });
    }
    let selected = application
        .store
        .views()
        .get(view)
        .unwrap()
        .selection
        .selections[0];
    assert_eq!((selected.anchor, selected.head), (caret, caret));
    let after = application
        .store
        .documents()
        .snapshot(previous.document)
        .unwrap();
    assert_eq!(
        (
            after.key,
            after.revision,
            after.rope.to_string(),
            after.metadata.read_only
        ),
        (before.key, before.revision, text, before.metadata.read_only)
    );
    application
        .store
        .set_view_state(view, previous.selection, previous.scroll, previous.folds)
        .unwrap();
    paint(application, context);
}

fn assert_json_formatting(
    application: &mut NativeApplication,
    context: &egui::Context,
    view: ViewId,
    property: &str,
) {
    use super::selection_ranges_tests::run_palette;
    let previous = application.store.views().get(view).unwrap().clone();
    let before = application
        .store
        .documents()
        .snapshot(previous.document)
        .unwrap();
    let original = before.rope.to_string();
    paint(application, context);
    let body = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    if before.metadata.read_only {
        super::inlay_hints_tests::paint(
            application,
            context,
            super::inlay_hints_tests::key(
                egui::Key::F,
                egui::Modifiers::ALT | egui::Modifiers::SHIFT,
            ),
        );
        paint(application, context);
        let current = application.store.documents().snapshot(before.id).unwrap();
        assert_eq!(
            (current.revision, current.rope),
            (before.revision, before.rope)
        );
        return;
    }
    run_palette(application, context, "monaco.editor.action.formatDocument");
    wait(application, context, |application| {
        application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string()
            != original
    });
    assert!(application.store.undo(before.id).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string(),
        original
    );
    let start = original.find(property).unwrap();
    let compact = property.replace(": ", ":");
    let current = application.store.documents().snapshot(before.id).unwrap();
    application
        .store
        .apply(
            before.id,
            Transaction {
                revision: current.revision,
                edits: vec![Edit {
                    bytes: start..start + property.len(),
                    text: compact.clone(),
                }],
                group: UndoGroup(1),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: start,
                    head: start + compact.len(),
                }],
            },
            previous.scroll.clone(),
            previous.folds.clone(),
        )
        .unwrap();
    paint(application, context);
    context.memory_mut(|memory| memory.request_focus(body));
    run_palette(application, context, "monaco.editor.action.formatSelection");
    wait(application, context, |application| {
        application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string()
            == original
    });
    assert!(application.store.undo(before.id).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string(),
        original.replacen(property, &compact, 1)
    );
    assert!(application.store.undo(before.id).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string(),
        original
    );
    let paste_setting = application
        .services
        .state
        .settings
        .read()
        .editor_format_on_paste;
    application
        .services
        .state
        .settings
        .write()
        .editor_format_on_paste = true;
    application
        .store
        .set_view_state(
            view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: 0,
                    head: original.len(),
                }],
            },
            previous.scroll.clone(),
            previous.folds.clone(),
        )
        .unwrap();
    paint(application, context);
    context.memory_mut(|memory| memory.request_focus(body));
    const PASTED: &str = "{\"json\":1}";
    super::inlay_hints_tests::paint(
        application,
        context,
        vec![egui::Event::Paste(PASTED.into())],
    );
    wait(application, context, |application| {
        let text = application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string();
        text.contains("\"json\": 1") && text.contains('\n')
    });
    assert!(application.store.undo(before.id).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string(),
        PASTED
    );
    assert!(application.store.undo(before.id).unwrap());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(before.id)
            .unwrap()
            .rope
            .to_string(),
        original
    );
    application
        .services
        .state
        .settings
        .write()
        .editor_format_on_paste = paste_setting;
    application
        .store
        .set_view_state(view, previous.selection, previous.scroll, previous.folds)
        .unwrap();
    wait(application, context, |application| {
        let current = application.store.documents().snapshot(before.id).unwrap();
        application.editor_symbols.builtin_model(&current).is_some()
    });
}

fn replace_json_text(
    application: &mut NativeApplication,
    document: DocumentId,
    view: ViewId,
    text: &str,
) {
    let current = application.store.documents().snapshot(document).unwrap();
    application
        .store
        .apply(
            document,
            Transaction {
                revision: current.revision,
                edits: vec![Edit {
                    bytes: 0..current.rope.len_bytes(),
                    text: text.into(),
                }],
                group: UndoGroup(1),
                origin: Some(view),
                selection_after: None,
            },
        )
        .unwrap();
}

fn assert_json_file_save(
    application: &mut NativeApplication,
    context: &egui::Context,
    view: ViewId,
    auto_save: bool,
) {
    let document = application.store.views().get(view).unwrap().document;
    let before = application.store.documents().snapshot(document).unwrap();
    let taide_native_editor::document::DocumentKey::File(path) = &before.key else {
        panic!("file fixture required")
    };
    let original = before.rope.to_string();
    let flags = {
        let mut settings = application.services.state.settings.write();
        let flags = (
            settings.format_on_save,
            settings.trim_trailing_whitespace_on_save,
            settings.insert_final_newline_on_save,
        );
        settings.format_on_save = true;
        settings.trim_trailing_whitespace_on_save = true;
        settings.insert_final_newline_on_save = true;
        flags
    };
    const RAW: &str = "{\"save\":1}   ";
    replace_json_text(application, document, view, RAW);
    let options = application.save_formatting_options(document).unwrap();
    let expected = format!(
        "{{\n{}\"save\": 1\n}}\n",
        " ".repeat(options.tab_size.try_into().unwrap())
    );
    let snapshot = application.store.save_snapshot(document).unwrap();
    let command = if auto_save {
        HostCommand::AutoSave {
            path: path.to_str().unwrap().into(),
            snapshot,
        }
    } else {
        HostCommand::Save {
            path: path.to_str().unwrap().into(),
            snapshot,
        }
    };
    assert!(application.submit(command));
    wait(application, context, |application| {
        !application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
            && std::fs::read_to_string(path).is_ok_and(|text| text == expected)
    });
    let saved = application.store.documents().snapshot(document).unwrap();
    assert_eq!(saved.rope.to_string(), expected);
    for _ in 0..MAX_SAVE_UNDOS {
        if application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string()
            == RAW
        {
            break;
        }
        assert!(application.store.undo(document).unwrap());
    }
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        RAW
    );
    if application.lsp.is_none() && !auto_save {
        let snapshot = application.store.save_snapshot(document).unwrap();
        assert!(application.submit(HostCommand::Save {
            path: path.to_str().unwrap().into(),
            snapshot
        }));
        application.pending_format_saves[&document]
            .epoch
            .invalidate();
        wait(application, context, |application| {
            !application.pending_format_saves.contains_key(&document)
        });
        assert_eq!(std::fs::read_to_string(path).unwrap(), expected);
        assert_eq!(
            application
                .store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            RAW
        );
        application.persistence.settled(document);
    }
    {
        let mut settings = application.services.state.settings.write();
        settings.format_on_save = false;
        settings.trim_trailing_whitespace_on_save = false;
        settings.insert_final_newline_on_save = false;
    }
    replace_json_text(application, document, view, &original);
    let snapshot = application.store.save_snapshot(document).unwrap();
    assert!(application.submit(HostCommand::Save {
        path: path.to_str().unwrap().into(),
        snapshot
    }));
    wait(application, context, |application| {
        !application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .dirty
            && std::fs::read_to_string(path).is_ok_and(|text| text == original)
    });
    {
        let mut settings = application.services.state.settings.write();
        settings.format_on_save = flags.0;
        settings.trim_trailing_whitespace_on_save = flags.1;
        settings.insert_final_newline_on_save = flags.2;
    }
    wait(application, context, |application| {
        let current = application.store.documents().snapshot(document).unwrap();
        application.editor_symbols.builtin_model(&current).is_some()
    });
}

fn assert_json_untitled_save(
    application: &mut NativeApplication,
    context: &egui::Context,
    view: ViewId,
    project: &ProjectId,
    cancel_first: bool,
) {
    let original = application.store.views().get(view).unwrap().key.clone();
    let source = application
        .store
        .documents()
        .snapshot(application.store.views().get(view).unwrap().document)
        .unwrap();
    let taide_native_editor::document::DocumentKey::File(path) = source.key else {
        panic!("owned source file required")
    };
    let destination = path.parent().unwrap().join("native-saved.json");
    const RAW: &str = "{\"draft\":1}  ";
    let (tab, document, _) = new_json_untitled(application, context, project, &original.pane, RAW);
    let previous_picker = application.save_picker.clone();
    let selected = destination.clone();
    application.save_picker = Arc::new(move |_, _| Some(selected.clone()));
    let flags = {
        let mut settings = application.services.state.settings.write();
        let flags = (
            settings.format_on_save,
            settings.trim_trailing_whitespace_on_save,
            settings.insert_final_newline_on_save,
        );
        settings.format_on_save = true;
        settings.trim_trailing_whitespace_on_save = true;
        settings.insert_final_newline_on_save = true;
        flags
    };
    let options = application.save_formatting_options(document).unwrap();
    let expected = format!(
        "{{\n{}\"draft\": 1\n}}\n",
        " ".repeat(options.tab_size.try_into().unwrap())
    );
    if cancel_first {
        application.save_untitled(&tab, &eframe::Frame::_new_kittest());
        assert_eq!(application.saving_untitled, Some(tab.clone()));
        application.pending_format_saves[&document]
            .epoch
            .invalidate();
        wait(application, context, |application| {
            application.saving_untitled.is_none()
                && !application.pending_format_saves.contains_key(&document)
        });
        assert!(!destination.exists());
        let current = application.store.documents().snapshot(document).unwrap();
        assert_eq!(
            current.key,
            taide_native_editor::document::DocumentKey::Untitled(tab.clone())
        );
        assert_eq!(current.rope.to_string(), RAW);
        application.persistence.settled(document);
    }
    application.save_untitled(&tab, &eframe::Frame::_new_kittest());
    assert_eq!(application.saving_untitled, Some(tab.clone()));
    wait(application, context, |application| {
        !application.untitled.contains_key(&tab)
            && application
                .store
                .documents()
                .snapshot(document)
                .is_ok_and(|snapshot| {
                    !snapshot.dirty
                        && snapshot.metadata.language_id == "json"
                        && snapshot.key
                            == taide_native_editor::document::DocumentKey::File(destination.clone())
                })
            && std::fs::read_to_string(&destination).is_ok_and(|text| text == expected)
    });
    let file_tab = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap()
        .key
        .tab
        .clone();
    assert_ne!(application.pending_dirty.get(&file_tab), Some(&true));
    let saved_tab = application
        .services
        .state
        .layouts
        .read()
        .values()
        .flat_map(taide_layout::service::all_roots)
        .flat_map(crate::tabs::tabs_in)
        .find(|tab| tab.id == file_tab)
        .cloned()
        .unwrap();
    assert!(!saved_tab.dirty);
    assert!(application.submit(HostCommand::CloseTab {
        tab: file_tab.clone(),
        discard: false
    }));
    wait(application, context, |application| {
        !application.has_layout_tab(&file_tab)
            && !application.editor_json.documents().contains(&document)
    });
    application.save_picker = previous_picker;
    {
        let mut settings = application.services.state.settings.write();
        settings.format_on_save = flags.0;
        settings.trim_trailing_whitespace_on_save = flags.1;
        settings.insert_final_newline_on_save = flags.2;
    }
    application
        .controller
        .submit(ShellMutation::ActivateTab(original.tab))
        .unwrap();
    application
        .controller
        .submit(ShellMutation::FocusPane(original.pane))
        .unwrap();
    wait(application, context, |application| {
        application.store.views().get(view).is_some()
            && crate::editor_context_menu::body(context, view)
                .is_some_and(|body| context.memory(|memory| memory.has_focus(body)))
    });
}

fn new_json_untitled(
    application: &mut NativeApplication,
    context: &egui::Context,
    project: &ProjectId,
    pane: &PaneId,
    text: &str,
) -> (TabId, DocumentId, ViewId) {
    let existing = application
        .controller
        .snapshot()
        .layouts
        .get(project)
        .into_iter()
        .flat_map(taide_layout::service::all_roots)
        .flat_map(crate::tabs::tabs_in)
        .map(|tab| tab.id.clone())
        .collect::<HashSet<_>>();
    assert!(application.submit(HostCommand::NewUntitled {
        project: project.clone(),
        pane: pane.clone()
    }));
    let runtime = application.runtime.handle().clone();
    let tab = runtime.block_on(async {
        tokio::time::timeout(DEADLINE, async {
            loop {
                eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
                let tab = application
                    .controller
                    .snapshot()
                    .layouts
                    .get(project)
                    .into_iter()
                    .flat_map(taide_layout::service::all_roots)
                    .flat_map(crate::tabs::tabs_in)
                    .find(|tab| {
                        matches!(tab.kind, TabKind::Untitled { .. }) && !existing.contains(&tab.id)
                    })
                    .map(|tab| tab.id.clone());
                if let Some(tab) = tab {
                    break tab;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    });
    let document = application
        .store
        .open_untitled(tab.clone(), text, "json".into())
        .unwrap();
    wait(application, context, |application| {
        application.untitled.contains_key(&tab)
            && application
                .store
                .documents()
                .snapshot(document)
                .is_ok_and(|snapshot| {
                    application
                        .editor_symbols
                        .builtin_model(&snapshot)
                        .is_some()
                })
    });
    let view = application
        .store
        .views()
        .find(&ViewKey {
            window: WINDOW_LABEL.into(),
            pane: pane.clone(),
            tab: tab.clone(),
        })
        .unwrap();
    (tab, document, view)
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
    let other_root = fixture.directory.join("other-project");
    std::fs::create_dir_all(&other_root).unwrap();
    let other_root = other_root
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let other_project = ProjectId::new();
    state.projects.write().insert(
        other_project.clone(),
        Project {
            id: other_project.clone(),
            root: other_root.clone(),
            name: "owned other JSON".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    state
        .session
        .write()
        .projects
        .push(taide_model::project::ProjectRef {
            id: other_project.clone(),
            root: other_root,
            name: "owned other JSON".into(),
            display: Default::default(),
            root_missing: false,
        });
    let other_layout = taide_layout::service::load_layout(&state.paths, &other_project);
    let other_pane = other_layout.focused_pane.clone();
    state
        .layouts
        .write()
        .insert(other_project.clone(), other_layout);
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
    let document = application.files[&path].id;
    let initial_view = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap()
        .id;
    assert!(application.lsp.as_ref().is_some_and(|lsp| {
        !lsp.has_save_formatter(&application.store.documents().snapshot(document).unwrap())
    }));
    assert_json_file_save(application, &context, initial_view, false);
    let lsp = application.lsp.take().unwrap();
    application.runtime.block_on(lsp.disconnect()).unwrap();
    assert_json_file_save(application, &context, initial_view, false);
    assert_json_file_save(application, &context, initial_view, true);
    assert_json_untitled_save(application, &context, initial_view, &project, false);
    assert!(application.lsp.is_none());
    assert_json_formatting(application, &context, initial_view, "\"value\": 1");
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
    assert_json_selection(application, &context, view, "\"value\": 1");
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
    assert!(
        !application
            .editor_json_validation
            .documents()
            .contains(&document)
    );
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
    assert!(
        application
            .editor_json_validation
            .documents()
            .contains(&document)
    );
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
    assert!(
        !application
            .editor_json_validation
            .documents()
            .contains(&document)
    );
    let pane = application
        .store
        .views()
        .get(view)
        .unwrap()
        .key
        .pane
        .clone();
    let (untitled, untitled_document, untitled_view) =
        new_json_untitled(application, &context, &project, &pane, TEXT);
    assert_json_formatting(application, &context, untitled_view, "\"value\": 1");
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
    assert_json_selection(application, &context, untitled_view, "\"value\": 1");
    const INVALID: &str = "/*note*/{\"\u{1f600}\":1,}";
    application
        .store
        .apply(
            untitled_document,
            Transaction {
                revision: after.revision,
                edits: vec![Edit {
                    bytes: 0..TEXT.len(),
                    text: INVALID.into(),
                }],
                group: UndoGroup(7),
                origin: Some(untitled_view),
                selection_after: None,
            },
        )
        .unwrap();
    wait(application, &context, |application| {
        application.lsp_diagnostics.counts()[crate::diagnostics::ERROR] == 2
    });
    let invalid = application
        .store
        .documents()
        .snapshot(untitled_document)
        .unwrap();
    let markers = application
        .lsp_diagnostics
        .display(&application.store, &invalid)
        .unwrap();
    assert_eq!(markers.markers().len(), 2);
    let comment = markers
        .markers()
        .iter()
        .find(|marker| marker.message.code.as_deref() == Some("521"))
        .unwrap();
    let body = crate::editor_context_menu::body(&context, untitled_view).unwrap();
    assert!(
        context
            .read_response(body.with((
                "diagnostic-hover",
                comment.bytes.start,
                comment.bytes.end,
                0usize
            )))
            .is_some()
    );
    let taide_native_editor::document::DocumentKey::Untitled(tab) = &invalid.key else {
        panic!("expected actual untitled model")
    };
    application.problems.toggle(&slot);
    paint(application, &context);
    let row = egui::Id::new(("problem-row", &slot, &invalid.key, Some(1usize)));
    let point = context
        .read_response(row)
        .expect("actual untitled problem row")
        .rect
        .center();
    for pressed in [true, false] {
        super::inlay_hints_tests::paint(
            application,
            &context,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(!application.problem_opens.is_empty());
    wait(application, &context, |application| {
        application
            .store
            .views()
            .get(untitled_view)
            .unwrap()
            .selection
            .selections[0]
            .head
            == INVALID.find(',').unwrap()
    });
    application.document_edits.push((
        tab.clone(),
        DocumentEdit::Problem(taide_native_editor::problem_navigation::Command::Next),
    ));
    paint(application, &context);
    {
        let mut provider = crate::editor_problems::Provider {
            scope: &application.shell.scope,
            state: &mut application.editor_problems,
            diagnostics: &application.lsp_diagnostics,
            project: Some(project.clone()),
            commands: &mut Vec::new(),
            viewport: context.viewport_id(),
        };
        assert!(
            taide_native_ui::editor_problems::Provider::current(
                &mut provider,
                &application.store,
                untitled_view
            )
            .is_some()
        );
    }
    const OTHER_JSON: &str = "{\"\u{1f600}\":1}/*other*/";
    application
        .controller
        .submit(ShellMutation::ActivateProject(other_project.clone()))
        .unwrap();
    wait(application, &context, |application| {
        let snapshot = application.controller.snapshot();
        snapshot.focused_project() == Some(&other_project)
    });
    let (other_tab, other_document, other_view) = new_json_untitled(
        application,
        &context,
        &other_project,
        &other_pane,
        OTHER_JSON,
    );
    wait(application, &context, |application| {
        application
            .lsp_diagnostics
            .problems(&application.store)
            .iter()
            .any(|problem| problem.document == other_document)
    });
    let other_before = application
        .store
        .documents()
        .snapshot(other_document)
        .unwrap();
    application
        .controller
        .submit(ShellMutation::ActivateProject(project.clone()))
        .unwrap();
    application
        .controller
        .submit(ShellMutation::ActivateTab(tab.clone()))
        .unwrap();
    wait(application, &context, |application| {
        let snapshot = application.controller.snapshot();
        snapshot.focused_project() == Some(&project)
            && snapshot
                .layouts
                .get(&project)
                .and_then(|layout| {
                    taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane)
                })
                .is_some_and(|entry| entry.id == *tab)
    });
    let previous = application
        .store
        .views()
        .get(untitled_view)
        .unwrap()
        .clone();
    application
        .store
        .set_view_state(
            untitled_view,
            taide_native_editor::view::SelectionSet {
                primary: 0,
                selections: vec![taide_native_editor::view::Selection {
                    anchor: INVALID.len(),
                    head: INVALID.len(),
                }],
            },
            previous.scroll,
            previous.folds,
        )
        .unwrap();
    let body = crate::editor_context_menu::body(&context, untitled_view).unwrap();
    context.memory_mut(|memory| memory.request_focus(body));
    super::inlay_hints_tests::paint(
        application,
        &context,
        super::inlay_hints_tests::key(egui::Key::F8, egui::Modifiers::NONE),
    );
    wait(application, &context, |application| {
        application.controller.snapshot().focused_project() == Some(&other_project)
            && application
                .store
                .views()
                .get(other_view)
                .is_some_and(|view| {
                    view.selection.selections[0].head == OTHER_JSON.find("/*").unwrap()
                })
            && crate::editor_context_menu::body(&context, other_view)
                .is_some_and(|body| context.memory(|memory| memory.has_focus(body)))
    });
    {
        let mut provider = crate::editor_problems::Provider {
            scope: &application.shell.scope,
            state: &mut application.editor_problems,
            diagnostics: &application.lsp_diagnostics,
            project: Some(other_project.clone()),
            commands: &mut Vec::new(),
            viewport: context.viewport_id(),
        };
        let widget = taide_native_ui::editor_problems::Provider::current(
            &mut provider,
            &application.store,
            other_view,
        )
        .unwrap();
        assert_eq!(widget.coordinate.problem.document, other_document);
        assert_eq!(widget.position, OTHER_JSON.find("/*").unwrap());
    }
    let body = crate::editor_context_menu::body(&context, other_view).unwrap();
    assert_eq!(context.memory(|memory| memory.focused()), Some(body));
    super::inlay_hints_tests::paint(
        application,
        &context,
        super::inlay_hints_tests::key(
            egui::Key::F8,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    wait(application, &context, |application| {
        application.controller.snapshot().focused_project() == Some(&project)
            && application
                .store
                .views()
                .get(untitled_view)
                .is_some_and(|view| view.selection.selections[0].head == INVALID.find(',').unwrap())
    });
    let slot = application
        .services
        .state
        .session
        .read()
        .focused_shell_slot
        .clone()
        .unwrap();
    if !application.problems.is_open(&slot) {
        application.problems.toggle(&slot);
    }
    paint(application, &context);
    let row = egui::Id::new(("problem-row", &slot, &other_before.key, Some(0usize)));
    let point = context
        .read_response(row)
        .expect("other project untitled row must remain available")
        .rect
        .center();
    for pressed in [true, false] {
        super::inlay_hints_tests::paint(
            application,
            &context,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    assert!(!application.problem_opens.is_empty());
    wait(application, &context, |application| {
        application.controller.snapshot().focused_project() == Some(&other_project)
            && application
                .store
                .views()
                .get(other_view)
                .is_some_and(|view| {
                    view.selection.selections[0].head == OTHER_JSON.find("/*").unwrap()
                })
            && crate::editor_context_menu::body(&context, other_view)
                .is_some_and(|body| context.memory(|memory| memory.has_focus(body)))
    });
    let other_after = application
        .store
        .documents()
        .snapshot(other_document)
        .unwrap();
    assert_eq!(
        context.memory(|memory| memory.focused()),
        crate::editor_context_menu::body(&context, other_view)
    );
    assert_eq!(
        (other_after.revision, other_after.rope.to_string()),
        (other_before.revision, OTHER_JSON.into())
    );
    assert!(application.submit(HostCommand::CloseTab {
        tab: other_tab.clone(),
        discard: true
    }));
    wait(application, &context, |application| {
        !application.untitled.contains_key(&other_tab)
            && !application
                .editor_json_validation
                .documents()
                .contains(&other_document)
    });
    application
        .controller
        .submit(ShellMutation::ActivateProject(project.clone()))
        .unwrap();
    application
        .controller
        .submit(ShellMutation::ActivateTab(tab.clone()))
        .unwrap();
    wait(application, &context, |application| {
        application.controller.snapshot().focused_project() == Some(&project)
    });
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(untitled_document)
            .unwrap()
            .rope
            .to_string(),
        INVALID
    );
    assert!(application.store.undo(untitled_document).unwrap());
    wait(application, &context, |application| {
        application.lsp_diagnostics.counts()[crate::diagnostics::ERROR] == 0
    });
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(untitled_document)
            .unwrap()
            .rope
            .to_string(),
        TEXT
    );
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
    assert!(application.editor_json.documents().is_empty());
    assert!(application.editor_json_validation.documents().is_empty());
}

#[test]
fn actual_app의_json_peek는_현재_문서_구조와_키_접기_및_닫힘을_소비한다() {
    assert_json_peek(false);
    assert_json_peek(true);
}

#[test]
fn actual_app의_json_미저장_저장취소는_파일을_쓰지않고_재시도를_허용한다() {
    let mut fixture = super::inlay_hints_tests::fixture_with_language(TEXT, "json");
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let lsp = application.lsp.take().unwrap();
    application.runtime.block_on(lsp.disconnect()).unwrap();
    let view = super::inlay_hints_tests::body_view(application, &fixture.path, &fixture.tab);
    assert_json_untitled_save(application, context, view, &fixture.project, true);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

fn assert_json_peek(read_only: bool) {
    use super::inlay_hints_tests::{body_view, fixture_with_view_state_and_setup, key};
    use taide_native_editor::symbol_locations::{Command, Kind, Mode};
    const PEEK_LINE_COUNT: usize = 40;
    const SCROLL_LINE: usize = 10;
    let peek_text = format!(
        "/*note*/{{\n\"outer\": {{\n{}\n}}\n}}",
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
                        read_only,
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
    assert_json_formatting(application, context, preview, "\"value_0\": 0");
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
    assert_eq!(current.metadata.read_only, read_only);
    application
        .store
        .request_scroll_restore(preview, Some(0), 0.0)
        .unwrap();
    wait(application, context, |application| {
        application
            .lsp_diagnostics
            .display(&application.store, &current)
            .is_some_and(|markers| {
                markers
                    .markers()
                    .iter()
                    .any(|marker| marker.message.code.as_deref() == Some("521"))
            })
            && crate::editor_context_menu::body(context, preview).is_some_and(|body| {
                context
                    .read_response(body.with((
                        "diagnostic-hover",
                        0usize,
                        "/*note*/".len(),
                        0usize,
                    )))
                    .is_some()
            })
    });
    let body = crate::editor_context_menu::body(context, preview).unwrap();
    let marker = context
        .read_response(body.with(("diagnostic-hover", 0usize, "/*note*/".len(), 0usize)))
        .expect("peek must paint its own JSON diagnostic");
    let point = marker.rect.center();
    super::inlay_hints_tests::paint(application, context, vec![egui::Event::PointerMoved(point)]);
    let ready = context.input(|input| input.time)
        + f64::from(context.global_style().interaction.tooltip_delay)
        + 1.0;
    let mut prepared_tooltip = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
            time: Some(ready),
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    prepared_tooltip.textures_delta.clear();
    let mut tooltip = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
            time: Some(ready + 1.0),
            ..Default::default()
        },
        |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
    );
    tooltip.textures_delta.clear();
    let mut shapes = tooltip
        .shapes
        .iter()
        .map(|shape| &shape.shape)
        .collect::<Vec<_>>();
    let mut has_message = false;
    let mut rendered_text = Vec::new();
    while let Some(shape) = shapes.pop() {
        match shape {
            egui::Shape::Vec(children) => shapes.extend(children),
            egui::Shape::Text(text) => {
                rendered_text.push(text.galley.text().to_owned());
                has_message |= text
                    .galley
                    .text()
                    .contains("Comments are not permitted in JSON.")
            }
            _ => {}
        }
    }
    assert!(
        has_message,
        "actual peek hover must display the current JSON diagnostic: {rendered_text:?}"
    );
    let owner_snapshot = application
        .store
        .documents()
        .snapshot(owner_document)
        .unwrap();
    assert!(
        application
            .lsp_diagnostics
            .display(&application.store, &owner_snapshot)
            .unwrap()
            .markers()
            .iter()
            .all(|marker| marker.message.code.as_deref() != Some("521"))
    );
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
    assert_json_selection(application, context, preview, "\"value_0\": 0");
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
    let focus = body.with(("sticky-line", 0usize));
    assert!(context.read_response(focus).is_some());
    assert_eq!(
        application
            .editor_keymap_targets
            .get(&(context.viewport_id(), focus))
            .map(|(view, _)| *view),
        Some(preview)
    );
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
    let focused = context.memory(|memory| memory.focused()).and_then(|id| {
        application
            .editor_keymap_targets
            .get(&(context.viewport_id(), id))
            .map(|(view, _)| *view)
    });
    assert_eq!(
        focused,
        Some(preview),
        "peek must own Escape before closing; focus={:?}",
        context.memory(|memory| memory.focused())
    );
    assert_eq!(context.memory(|memory| memory.focused()), Some(body));
    super::inlay_hints_tests::paint(
        application,
        context,
        key(egui::Key::Escape, egui::Modifiers::NONE),
    );
    assert!(
        application.editor_locations.current(owner).is_none(),
        "Escape did not close its owned peek; focus={:?}; preview still exists={}",
        context.memory(|memory| memory.focused()),
        application.store.views().get(preview).is_some()
    );
    wait(application, context, |application| {
        !application.editor_json.documents().contains(&document)
    });
    assert!(application.editor_symbols.builtin_model(&current).is_none());
    assert!(application.editor_folding.builtin_model(&current).is_none());
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}
