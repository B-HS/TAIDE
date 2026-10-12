use super::inlay_hints_tests::{body_view, fixture_with_provider, key, paint, wait};
use super::selection_ranges_tests::run_palette;
use super::*;
use taide_native_editor::view::{Selection, SelectionSet};

const EMPTY: egui::Modifiers = egui::Modifiers::NONE;
const SHIFT: egui::Modifiers = egui::Modifiers {
    shift: true,
    ..EMPTY
};
const STABLE_FRAMES: usize = 16;
const DEBOUNCE: Duration = Duration::from_millis(300);
const CHANGE_DELAY: Duration = Duration::from_millis(100);
const AUXILIARY_SLOT: u32 = 1;

fn start_ide(application: &NativeApplication, directory: &std::path::Path) {
    assert!(
        application
            .services
            .ide
            .mark_started(
                0,
                String::new(),
                directory.join("synthetic-ide"),
                application.runtime.handle().spawn(std::future::pending()),
            )
            .is_some()
    );
}

fn select(application: &mut NativeApplication, view: ViewId, anchor: usize, head: usize) {
    let current = application.store.views().get(view).unwrap().clone();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor, head }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
}

fn focus(application: &mut NativeApplication, context: &egui::Context, view: ViewId) {
    paint(application, context, Vec::new());
    let id = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(id));
    paint(application, context, Vec::new());
}

fn add_auxiliary_file(
    application: &mut NativeApplication,
    project: &ProjectId,
    path: &str,
) -> (WindowScope, PaneId, TabId) {
    use taide_model::layout::{AuxWindowLayout, PaneNode};
    let pane = PaneId::new();
    let tab = TabId::new();
    let mut layout = application.services.state.layouts.read()[project].clone();
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: AUXILIARY_SLOT,
        root: PaneNode::Leaf {
            id: pane.clone(),
            tabs: vec![Tab {
                id: tab.clone(),
                kind: TabKind::File { path: path.into() },
                title: "current.rs".into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }],
            active: Some(tab.clone()),
        },
        focused_pane: pane.clone(),
    });
    taide_runtime::layout_actions::finish_mutation(
        application.services.events.as_ref(),
        &application.services.state,
        project,
        &mut layout,
    );
    application
        .services
        .state
        .layouts
        .write()
        .insert(project.clone(), layout.clone());
    application
        .controller
        .apply_layouts(HashMap::from([(project.clone(), layout)]));
    (
        WindowScope::Auxiliary {
            project: project.clone(),
            slot: AUXILIARY_SLOT,
        },
        pane,
        tab,
    )
}

#[test]
fn 실제_ide_선택은_초기_reveal을_제외하고_utf16과_readonly_본문_수명을_보존한다() {
    let mut fixture = fixture_with_provider("--native-inlay-hints", false, |_, _, _| true);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let current = application.store.views().get(view).unwrap().clone();
    let document = current.document;
    let mut file =
        taide_file::service::open_file(std::path::Path::new(&fixture.path), &[], false).unwrap();
    file.read_only = true;
    application
        .store
        .observe_file(document, std::path::Path::new(&fixture.path), file)
        .unwrap();
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 8, head: 8 }],
            },
            current.scroll,
            current.folds,
        )
        .unwrap();
    let before = application.store.documents().snapshot(document).unwrap();
    assert!(before.metadata.read_only);
    let runtime = application.runtime.handle().clone();
    assert!(
        application
            .services
            .ide
            .mark_started(
                0,
                String::new(),
                fixture.directory.join("synthetic-ide"),
                runtime.spawn(std::future::pending()),
            )
            .is_some()
    );
    let mut notifications = application.services.ide.subscribe();
    paint(application, context, Vec::new());
    let id = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(id));
    paint(application, context, Vec::new());
    assert!(application.services.ide.current_selection().is_none());
    assert!(notifications.try_recv().is_err());
    paint(application, context, key(egui::Key::ArrowRight, SHIFT));
    assert!(application.services.ide.current_selection().is_none());
    application.system_usage.detail_open = true;
    paint(application, context, Vec::new());
    wait(
        application,
        context,
        "IDE Unicode selection",
        |application| {
            application
                .services
                .ide
                .current_selection()
                .is_some_and(|selection| selection.text == "\u{1f600}" && !selection.is_empty)
        },
    );
    let selected = application.services.ide.current_selection().unwrap();
    assert!(application.system_usage.detail_open);
    application.system_usage.detail_open = false;
    paint(application, context, Vec::new());
    context.memory_mut(|memory| memory.request_focus(id));
    paint(application, context, Vec::new());
    assert_eq!(selected.project_id, fixture.project);
    assert_eq!(selected.path, fixture.path);
    assert_eq!(
        (
            selected.start_line,
            selected.start_character,
            selected.end_line,
            selected.end_character,
        ),
        (1, 2, 1, 4)
    );
    assert_eq!(application.services.ide.latest_selection(), Some(selected));
    assert!(
        notifications
            .try_recv()
            .unwrap()
            .contains("selection_changed")
    );
    for _ in 0..STABLE_FRAMES {
        paint(application, context, Vec::new());
    }
    assert!(notifications.try_recv().is_err());
    paint(application, context, key(egui::Key::ArrowLeft, SHIFT));
    wait(application, context, "IDE empty selection", |application| {
        application
            .services
            .ide
            .current_selection()
            .is_some_and(|selection| selection.is_empty && selection.text.is_empty())
    });
    assert_eq!(
        application.services.ide.latest_selection().unwrap().text,
        "\u{1f600}"
    );
    paint(application, context, key(egui::Key::ArrowRight, EMPTY));
    paint(application, context, key(egui::Key::ArrowLeft, SHIFT));
    wait(
        application,
        context,
        "IDE reversed selection",
        |application| {
            application
                .services
                .ide
                .current_selection()
                .is_some_and(|selection| !selection.is_empty && selection.text == "\u{1f600}")
        },
    );
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: 12,
            head: 8
        }]
    );
    let after = application.store.documents().snapshot(document).unwrap();
    assert_eq!(
        (
            after.rope.to_string(),
            after.revision,
            after.dirty,
            after.metadata.read_only
        ),
        (
            before.rope.to_string(),
            before.revision,
            before.dirty,
            before.metadata.read_only
        )
    );
    paint(application, context, key(egui::Key::End, SHIFT));
    application.request_close_tabs(vec![fixture.tab.clone()]);
    wait(application, context, "IDE closed body", |application| {
        !application.has_layout_tab(&fixture.tab)
            && application.services.ide.current_selection().is_none()
    });
    let latest = application.services.ide.latest_selection();
    assert!(latest.is_some());
    for _ in 0..STABLE_FRAMES {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
        assert!(application.services.ide.current_selection().is_none());
        assert_eq!(application.services.ide.latest_selection(), latest);
    }
    assert_eq!(
        std::fs::read_to_string(&fixture.path).unwrap(),
        before.rope.to_string()
    );
}

#[test]
fn 추가_ide_대기와_전송거절은_수명과_동일문서_소유를_보존한다() {
    let mut fixture = fixture_with_provider("--native-inlay-hints", false, |_, _, _| true);
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    start_ide(application, &fixture.directory);
    let runtime = application.runtime.handle().clone();
    let project = fixture.project.clone();
    let path = fixture.path.clone();
    let mut notifications = application.services.ide.subscribe();
    let mut state = crate::editor_ide_selection::State::default();
    let frame =
        |state: &mut crate::editor_ide_selection::State, application: &NativeApplication, now| {
            state.begin_frame(true);
            state.observe(
                &application.store,
                &project,
                &WindowScope::Main,
                view,
                &path,
                now,
            );
            state.finish_frame(&application.store, now)
        };
    let now = Instant::now();
    assert!(frame(&mut state, application, now).is_empty());
    select(application, view, 0, 5);
    assert!(frame(&mut state, application, now).is_empty());
    select(application, view, 8, 12);
    assert!(frame(&mut state, application, now + CHANGE_DELAY).is_empty());
    assert!(frame(&mut state, application, now + DEBOUNCE).is_empty());
    let due = now + CHANGE_DELAY + DEBOUNCE;
    let mut requests = frame(&mut state, application, due);
    assert_eq!(requests.len(), 1);
    let request = requests.pop().unwrap().unwrap();
    state.retry(request, due);
    assert_eq!(state.next_deadline(), Some(due + DEBOUNCE));
    assert!(frame(&mut state, application, due + CHANGE_DELAY).is_empty());
    let mut requests = frame(&mut state, application, due + DEBOUNCE);
    assert_eq!(requests.len(), 1);
    runtime
        .block_on(
            requests
                .pop()
                .unwrap()
                .unwrap()
                .apply(&application.services),
        )
        .unwrap();
    assert_eq!(
        application.services.ide.current_selection().unwrap().text,
        "\u{1f600}"
    );
    assert!(
        notifications
            .try_recv()
            .unwrap()
            .contains("selection_changed")
    );
    let change = due + DEBOUNCE + CHANGE_DELAY;
    select(application, view, 8, 8);
    assert!(frame(&mut state, application, change).is_empty());
    let mut requests = frame(&mut state, application, change + DEBOUNCE);
    assert_eq!(requests.len(), 1);
    let queued = requests.pop().unwrap().unwrap();
    state.begin_frame(true);
    let clears = state.finish_frame(&application.store, change + DEBOUNCE);
    assert_eq!(clears.len(), 1);
    for clear in clears {
        runtime
            .block_on(clear.unwrap().apply(&application.services))
            .unwrap();
    }
    runtime
        .block_on(queued.apply(&application.services))
        .unwrap();
    assert!(application.services.ide.current_selection().is_none());
    assert_eq!(
        application.services.ide.latest_selection().unwrap().text,
        "\u{1f600}"
    );
    assert!(notifications.try_recv().is_err());
    assert!(state.next_deadline().is_none());
    let restarted = change + DEBOUNCE + CHANGE_DELAY;
    assert!(frame(&mut state, application, restarted).is_empty());
    select(application, view, 0, 5);
    assert!(frame(&mut state, application, restarted).is_empty());
    let mut requests = frame(&mut state, application, restarted + DEBOUNCE);
    assert_eq!(requests.len(), 1);
    let queued = requests.pop().unwrap().unwrap();
    drop(state);
    runtime
        .block_on(queued.apply(&application.services))
        .unwrap();
    assert!(application.services.ide.current_selection().is_none());
    let (scope, pane, tab) = add_auxiliary_file(application, &project, &path);
    let auxiliary = application
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
    let mut state = crate::editor_ide_selection::State::default();
    let mirror_frame = |state: &mut crate::editor_ide_selection::State,
                        application: &NativeApplication,
                        now,
                        has_main| {
        state.begin_frame(true);
        if has_main {
            state.observe(
                &application.store,
                &project,
                &WindowScope::Main,
                view,
                &path,
                now,
            );
        }
        state.observe(&application.store, &project, &scope, auxiliary, &path, now);
        state.finish_frame(&application.store, now)
    };
    let mirrored = restarted + DEBOUNCE + CHANGE_DELAY;
    assert!(mirror_frame(&mut state, application, mirrored, true).is_empty());
    select(application, view, 0, 1);
    select(application, auxiliary, 1, 3);
    assert!(mirror_frame(&mut state, application, mirrored, true).is_empty());
    let requests = mirror_frame(&mut state, application, mirrored + DEBOUNCE, true);
    assert_eq!(requests.len(), 2);
    for request in requests {
        runtime
            .block_on(request.unwrap().apply(&application.services))
            .unwrap();
    }
    assert_eq!(
        application.services.ide.current_selection().unwrap().text,
        "la"
    );
    let clears = mirror_frame(&mut state, application, mirrored + DEBOUNCE, false);
    assert_eq!(clears.len(), 1);
    for clear in clears {
        runtime
            .block_on(clear.unwrap().apply(&application.services))
            .unwrap();
    }
    assert_eq!(
        application.services.ide.current_selection().unwrap().text,
        "la"
    );
    state.begin_frame(true);
    for clear in state.finish_frame(&application.store, mirrored + DEBOUNCE) {
        runtime
            .block_on(clear.unwrap().apply(&application.services))
            .unwrap();
    }
    assert!(application.services.ide.current_selection().is_none());
    assert_eq!(
        application.services.ide.latest_selection().unwrap().text,
        "la"
    );
    let reopened = mirrored + DEBOUNCE + CHANGE_DELAY;
    assert!(mirror_frame(&mut state, application, reopened, false).is_empty());
    select(application, auxiliary, 3, 5);
    assert!(mirror_frame(&mut state, application, reopened, false).is_empty());
    let mut requests = mirror_frame(&mut state, application, reopened + DEBOUNCE, false);
    assert_eq!(requests.len(), 1);
    application
        .services
        .state
        .layouts
        .write()
        .get_mut(&project)
        .unwrap()
        .auxiliary_windows
        .clear();
    runtime
        .block_on(
            requests
                .pop()
                .unwrap()
                .unwrap()
                .apply(&application.services),
        )
        .unwrap();
    assert!(application.services.ide.current_selection().is_none());
    assert_eq!(
        application.services.ide.latest_selection().unwrap().text,
        "la"
    );
}

#[test]
fn 추가_ide_보조창과_untitled_appfile은_본문_공급_범위를_보존한다() {
    use taide_model::app::AppFileTarget;
    use taide_native_editor::document::DocumentKey;
    let mut fixture = fixture_with_provider("--native-inlay-hints", false, |_, _, _| true);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    start_ide(application, &fixture.directory);
    let main = body_view(application, &fixture.path, &fixture.tab);
    focus(application, context, main);
    paint(application, context, key(egui::Key::ArrowRight, SHIFT));
    wait(application, context, "IDE main source", |application| {
        application
            .services
            .ide
            .current_selection()
            .is_some_and(|selection| selection.text == "c")
    });
    let main_selection = application
        .store
        .views()
        .get(main)
        .unwrap()
        .selection
        .clone();
    let (scope, pane, tab) = add_auxiliary_file(application, &fixture.project, &fixture.path);
    application.shell.scope = scope.clone();
    let view_key = ViewKey {
        window: WINDOW_LABEL.into(),
        pane: pane.clone(),
        tab: tab.clone(),
    };
    wait(
        application,
        context,
        "IDE auxiliary mounted",
        |application| {
            application.store.views().find(&view_key).is_some()
                && application.services.ide.current_selection().is_none()
        },
    );
    let auxiliary = application.store.views().find(&view_key).unwrap();
    focus(application, context, auxiliary);
    paint(application, context, key(egui::Key::ArrowRight, SHIFT));
    paint(application, context, key(egui::Key::ArrowRight, SHIFT));
    wait(
        application,
        context,
        "IDE auxiliary source",
        |application| {
            application
                .services
                .ide
                .current_selection()
                .is_some_and(|selection| selection.text == "cl")
        },
    );
    assert_eq!(
        application.store.views().get(main).unwrap().selection,
        main_selection
    );
    wait(application, context, "IDE peek provider", |application| {
        !application
            .lsp
            .as_ref()
            .unwrap()
            .location_providers(
                &fixture.project,
                &application
                    .store
                    .documents()
                    .snapshot(application.store.views().get(auxiliary).unwrap().document)
                    .unwrap(),
                taide_native_editor::symbol_locations::Kind::Definition,
            )
            .is_empty()
    });
    let selected = application.services.ide.current_selection();
    run_palette(application, context, "monaco.editor.action.peekDefinition");
    wait(application, context, "IDE peek mounted", |application| {
        application
            .editor_locations
            .current(auxiliary)
            .is_some_and(|session| session.preview.is_some())
    });
    let preview = application
        .editor_locations
        .current(auxiliary)
        .unwrap()
        .preview
        .unwrap();
    let preview_selection = application
        .store
        .views()
        .get(preview)
        .unwrap()
        .selection
        .clone();
    focus(application, context, preview);
    paint(application, context, key(egui::Key::ArrowLeft, SHIFT));
    assert_ne!(
        application.store.views().get(preview).unwrap().selection,
        preview_selection
    );
    assert_eq!(application.services.ide.current_selection(), selected);
    assert!(application.editor_ide_selection.next_deadline().is_none());
    application.editor_locations.close(auxiliary);
    paint(application, context, Vec::new());
    assert!(application.submit(HostCommand::NewUntitled {
        project: fixture.project.clone(),
        pane: pane.clone()
    }));
    wait(application, context, "IDE untitled tab", |application| {
        !application.untitled.is_empty() && application.services.ide.current_selection().is_none()
    });
    let untitled = application.untitled.keys().next().unwrap().clone();
    let view_key = ViewKey {
        window: WINDOW_LABEL.into(),
        pane: pane.clone(),
        tab: untitled,
    };
    wait(application, context, "IDE untitled body", |application| {
        application.store.views().find(&view_key).is_some()
    });
    let untitled = application.store.views().find(&view_key).unwrap();
    focus(application, context, untitled);
    paint(
        application,
        context,
        vec![egui::Event::Text("scratch".into())],
    );
    paint(application, context, key(egui::Key::ArrowLeft, SHIFT));
    let document = application.store.views().get(untitled).unwrap().document;
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "scratch"
    );
    assert!(application.services.ide.is_running());
    assert!(application.editor_ide_selection.next_deadline().is_none());
    assert!(application.services.ide.current_selection().is_none());
    assert_eq!(
        application.services.ide.latest_selection().unwrap().text,
        "cl"
    );
    let settings_path = application.services.state.paths.settings_file();
    std::fs::write(&settings_path, "{\n  \"editorFontSize\": 14\n}\n").unwrap();
    let settings_before = std::fs::read(&settings_path).unwrap();
    assert!(application.submit(HostCommand::OpenAppFile {
        project: fixture.project.clone(),
        pane: pane.clone(),
        target: AppFileTarget::Settings,
        title: "settings.json".into()
    }));
    wait(application, context, "IDE appfile body", |application| {
        application
            .store
            .documents()
            .find(&DocumentKey::AppFile(AppFileTarget::Settings))
            .is_some()
    });
    let tab = {
        let layouts = application.services.state.layouts.read();
        let (root, _) = crate::symbol_sidebar::window_tree(
            &fixture.project,
            &layouts[&fixture.project],
            &scope,
        )
        .unwrap();
        crate::tabs::tabs_in(root)
            .into_iter()
            .find(|tab| matches!(tab.kind, TabKind::AppFile { .. }))
            .unwrap()
            .id
            .clone()
    };
    let view_key = ViewKey {
        window: WINDOW_LABEL.into(),
        pane,
        tab,
    };
    wait(application, context, "IDE appfile mounted", |application| {
        application.store.views().find(&view_key).is_some()
    });
    let appfile = application.store.views().find(&view_key).unwrap();
    focus(application, context, appfile);
    paint(application, context, key(egui::Key::ArrowRight, SHIFT));
    assert!(application.services.ide.is_running());
    assert!(application.editor_ide_selection.next_deadline().is_none());
    assert!(application.services.ide.current_selection().is_none());
    assert_eq!(
        application.services.ide.latest_selection().unwrap().text,
        "cl"
    );
    assert_eq!(std::fs::read(&settings_path).unwrap(), settings_before);
}
