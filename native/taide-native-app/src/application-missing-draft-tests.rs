use super::inlay_hints_tests::{
    Fixture, body_view, fixture_with_view_state, fixture_with_view_state_and_setup, paint, wait,
};
use super::*;
use std::collections::VecDeque;
use std::sync::Mutex;
use taide_model::layout::{AuxWindowLayout, PaneNode};

const TEXT: &str = "class\n  body\nend";
const DRAFT: &str = "복구 초안\r\nsecond line";
const AUXILIARY_SLOT: u32 = 1;
const UPDATED_DRAFT: &str = "새 복구 초안\r\nlatest content";

#[test]
fn 실제_untitled_저장도_같은_picker의_취소와_요청_설정_저장을_보존한다() {
    let mut fixture =
        fixture_with_view_state("--native-inlay-hints", false, |_, _, _| true, None, TEXT);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    assert!(
        application.submit(HostCommand::NewUntitled {
            project: fixture.project.clone(),
            pane: application.controller.snapshot().layouts[&fixture.project]
                .focused_pane
                .clone()
        })
    );
    wait(application, context, "untitled document", |application| {
        application.untitled.values().any(|document| {
            application
                .store
                .views()
                .for_document(document.id)
                .next()
                .is_some()
        })
    });
    let tab = application.untitled.keys().next().unwrap().clone();
    let document = application.untitled[&tab].id;
    let view = application
        .store
        .views()
        .for_document(document)
        .next()
        .unwrap()
        .id;
    taide_native_editor::editing::replace_selections(&mut application.store, view, DRAFT, None)
        .unwrap();
    let layout = application.controller.snapshot().layouts[&fixture.project].clone();
    let title = crate::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|candidate| candidate.id == tab)
        .unwrap()
        .title
        .clone();
    let target = PathBuf::from(&fixture.path).with_file_name("untitled.txt");
    let options = Arc::new(Mutex::new(VecDeque::from([None, Some(target.clone())])));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let captured = calls.clone();
    application.save_picker = Arc::new(move |_, request| {
        captured.lock().unwrap().push(request);
        options.lock().unwrap().pop_front().unwrap()
    });
    application.save_untitled(&tab, &eframe::Frame::_new_kittest());
    assert!(application.saving_untitled.is_none());
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        DRAFT
    );
    assert!(!target.exists());
    application.save_untitled(&tab, &eframe::Frame::_new_kittest());
    wait(
        application,
        context,
        "untitled saved by picker",
        |application| {
            application.saving_untitled.is_none()
                && application.files.contains_key(target.to_str().unwrap())
        },
    );
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    for request in calls.iter() {
        assert_eq!(
            request.directory,
            PathBuf::from(&fixture.path).parent().unwrap()
        );
        assert_eq!(request.name, title);
        assert_eq!(
            request.title,
            presentation::message(&application.locale, "tab.saveAsTitle", &[])
        );
    }
    assert_eq!(std::fs::read_to_string(&target).unwrap(), DRAFT);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
    eframe::App::on_exit(application);
    assert_eq!(application.services.tasks.tracked_count(), 0);
}

fn main_fixture() -> (Fixture, TabId) {
    let tab = TabId::new();
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        TEXT,
        |application, path, project, _| {
            let source = PathBuf::from(path).with_file_name("peek.rs");
            let services = application.services.clone();
            application
                .runtime
                .handle()
                .clone()
                .block_on(taide_runtime::file_actions::file_mirror_dirty(
                    &services.state,
                    &services.tasks,
                    project.clone(),
                    source.to_str().unwrap().into(),
                    DRAFT.into(),
                ))
                .unwrap();
            std::fs::remove_file(&source).unwrap();
            let mut layout = services.state.layouts.read()[project].clone();
            let PaneNode::Leaf { tabs, .. } = &mut layout.root else {
                panic!("expected fixture leaf")
            };
            tabs.push(Tab {
                id: tab.clone(),
                kind: TabKind::File {
                    path: source.to_str().unwrap().into(),
                },
                title: "peek.rs".into(),
                pinned: false,
                preview: false,
                dirty: true,
                view_state: None,
            });
            taide_runtime::layout_actions::finish_mutation(
                services.events.as_ref(),
                &services.state,
                project,
                &mut layout,
            );
            services
                .state
                .layouts
                .write()
                .insert(project.clone(), layout.clone());
            application
                .controller
                .apply_layouts(HashMap::from([(project.clone(), layout)]));
        },
    );
    let application = fixture.application.as_mut().unwrap();
    application
        .controller
        .submit(ShellMutation::ActivateTab(tab.clone()))
        .unwrap();
    wait(
        application,
        &fixture.context,
        "main missing draft",
        |application| application.missing_drafts.contains_key(&fixture.peek_path),
    );
    (fixture, tab)
}

fn click_save(application: &mut NativeApplication, context: &egui::Context) {
    let output = paint(application, context, Vec::new());
    let label = presentation::message(&application.locale, "editor.saveDraftAs", &[]);
    let position = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.pos + text.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .expect("missing source Save As button");
    paint(
        application,
        context,
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    paint(
        application,
        context,
        vec![egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}

#[test]
fn 실제_초안_저장_버튼은_취소와_다른_동일_경로_저장_재열기를_보존한다() {
    for is_same in [false, true] {
        let (mut fixture, tab) = main_fixture();
        let destination = if is_same {
            PathBuf::from(&fixture.peek_path)
        } else {
            fixture.directory.join("project/recovered.txt")
        };
        let selected = Arc::new(Mutex::new(VecDeque::from([
            None,
            Some(destination.clone()),
        ])));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let picker_calls = calls.clone();
        let application = fixture.application.as_mut().unwrap();
        application.save_picker = Arc::new(move |_, request| {
            picker_calls.lock().unwrap().push(request);
            selected.lock().unwrap().pop_front().unwrap()
        });
        let context = &fixture.context;
        click_save(application, context);
        assert!(application.saving_missing_draft.is_none());
        assert_eq!(calls.lock().unwrap().len(), 1);
        assert_eq!(
            application.missing_drafts[&fixture.peek_path]
                .mirror
                .content,
            DRAFT
        );
        assert!(!destination.exists());
        click_save(application, context);
        wait(application, context, "Save As body loaded", |application| {
            application.saving_missing_draft.is_none()
                && application
                    .files
                    .contains_key(destination.to_str().unwrap())
        });
        let requests = calls.lock().unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests.iter() {
            assert_eq!(
                request.directory,
                PathBuf::from(&fixture.peek_path).parent().unwrap()
            );
            assert_eq!(request.name, "peek.rs");
            assert_eq!(
                request.title,
                presentation::message(&application.locale, "tab.saveAsTitle", &[])
            );
        }
        let layout = application.services.state.layouts.read()[&fixture.project].clone();
        let source = crate::tabs::tabs_in(&layout.root)
            .into_iter()
            .find(|candidate| candidate.id == tab)
            .unwrap();
        assert!(!source.dirty);
        assert_eq!(PathBuf::from(&fixture.peek_path).exists(), is_same);
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), DRAFT);
        let document = application
            .store
            .documents()
            .snapshot(application.files[destination.to_str().unwrap()].id)
            .unwrap();
        assert_eq!(document.rope.to_string(), DRAFT);
        assert!(!document.dirty);
        assert!(
            taide_file::service::list_mirrors(&application.services.state.paths, &fixture.project)
                .unwrap()
                .is_empty()
        );
        let target = crate::tabs::tabs_in(&layout.root).into_iter().find(|candidate| matches!(&candidate.kind, TabKind::File {path} if PathBuf::from(path) == destination)).unwrap().id.clone();
        application.request_tab_close(target.clone());
        wait(
            application,
            context,
            "saved draft tab closed",
            |application| {
                application.pending_tab_close.is_none()
                    && !crate::tabs::tabs_in(
                        &application.controller.snapshot().layouts[&fixture.project].root,
                    )
                    .iter()
                    .any(|tab| tab.id == target)
            },
        );
        application
            .controller
            .submit(ShellMutation::ReopenClosed(fixture.project.clone()))
            .unwrap();
        wait(
            application,
            context,
            "saved draft tab reopened",
            |application| {
                application
                    .files
                    .get(destination.to_str().unwrap())
                    .is_some_and(|file| {
                        application
                            .store
                            .views()
                            .for_document(file.id)
                            .any(|view| view.key.tab == target)
                    })
            },
        );
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
        eframe::App::on_exit(application);
        assert_eq!(application.services.tasks.tracked_count(), 0);
    }
}

#[test]
fn 실제_초안_저장_오류는_목적지와_최신_mirror를_보호하고_새_초안을_다시_표시한다() {
    let (mut fixture, _) = main_fixture();
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let original = body_view(application, &fixture.path, &fixture.tab);
    taide_native_editor::editing::replace_selections(
        &mut application.store,
        original,
        "changed ",
        None,
    )
    .unwrap();
    let dirty = application
        .store
        .documents()
        .snapshot(application.store.views().get(original).unwrap().document)
        .unwrap();
    let outside = fixture.directory.join("outside.txt");
    let directory = fixture.directory.join("project/directory-target");
    std::fs::create_dir(&directory).unwrap();
    let destination = fixture.directory.join("project/recovered.txt");
    let options = Arc::new(Mutex::new(VecDeque::from([
        Some(PathBuf::from(&fixture.path)),
        Some(outside.clone()),
        Some(directory.clone()),
        Some(destination.clone()),
        Some(destination.clone()),
    ])));
    application.save_picker = Arc::new(move |_, _| options.lock().unwrap().pop_front().unwrap());
    for _ in ["dirty", "outside"] {
        click_save(application, context);
        assert!(application.saving_missing_draft.is_none());
        assert_eq!(
            application.missing_drafts[&fixture.peek_path]
                .mirror
                .content,
            DRAFT
        );
        assert_eq!(
            application
                .store
                .documents()
                .snapshot(dirty.id)
                .unwrap()
                .rope
                .to_string(),
            dirty.rope.to_string()
        );
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
        assert!(!outside.exists());
    }
    click_save(application, context);
    wait(
        application,
        context,
        "save destination failure",
        |application| {
            application.saving_missing_draft.is_none()
                && !application.loading.contains(&fixture.peek_path)
        },
    );
    assert_eq!(
        application.missing_drafts[&fixture.peek_path]
            .mirror
            .content,
        DRAFT
    );
    assert!(directory.is_dir());
    let services = application.services.clone();
    application
        .runtime
        .handle()
        .clone()
        .block_on(taide_runtime::file_actions::file_mirror_dirty(
            &services.state,
            &services.tasks,
            fixture.project.clone(),
            fixture.peek_path.clone(),
            UPDATED_DRAFT.into(),
        ))
        .unwrap();
    click_save(application, context);
    wait(
        application,
        context,
        "changed mirror save rejected",
        |application| application.saving_missing_draft.is_none(),
    );
    assert!(!destination.exists());
    wait(
        application,
        context,
        "latest draft refreshed",
        |application| {
            application
                .missing_drafts
                .get(&fixture.peek_path)
                .is_some_and(|draft| draft.mirror.content == UPDATED_DRAFT)
        },
    );
    click_save(application, context);
    wait(application, context, "latest draft saved", |application| {
        application.saving_missing_draft.is_none()
            && application
                .files
                .contains_key(destination.to_str().unwrap())
    });
    assert_eq!(
        std::fs::read_to_string(&destination).unwrap(),
        UPDATED_DRAFT
    );
    assert_eq!(
        application
            .store
            .documents()
            .snapshot(dirty.id)
            .unwrap()
            .rope
            .to_string(),
        dirty.rope.to_string()
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
}

#[test]
fn 실제_보조_영역의_삭제_초안은_현재_창에_새_파일을_열고_원래_탭을_유지한다() {
    let tab = TabId::new();
    let pane = PaneId::new();
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        TEXT,
        |application, path, project, _| {
            let source = PathBuf::from(path).with_file_name("peek.rs");
            let services = application.services.clone();
            application
                .runtime
                .handle()
                .clone()
                .block_on(taide_runtime::file_actions::file_mirror_dirty(
                    &services.state,
                    &services.tasks,
                    project.clone(),
                    source.to_str().unwrap().into(),
                    DRAFT.into(),
                ))
                .unwrap();
            std::fs::remove_file(&source).unwrap();
            let mut layout = services.state.layouts.read()[project].clone();
            layout.auxiliary_windows.push(AuxWindowLayout {
                slot: AUXILIARY_SLOT,
                root: PaneNode::Leaf {
                    id: pane.clone(),
                    tabs: vec![Tab {
                        id: tab.clone(),
                        kind: TabKind::File {
                            path: source.to_str().unwrap().into(),
                        },
                        title: "peek.rs".into(),
                        pinned: false,
                        preview: false,
                        dirty: true,
                        view_state: None,
                    }],
                    active: Some(tab.clone()),
                },
                focused_pane: pane.clone(),
            });
            taide_runtime::layout_actions::finish_mutation(
                services.events.as_ref(),
                &services.state,
                project,
                &mut layout,
            );
            services
                .state
                .layouts
                .write()
                .insert(project.clone(), layout.clone());
            application
                .controller
                .apply_layouts(HashMap::from([(project.clone(), layout)]));
        },
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    application.shell.scope = WindowScope::Auxiliary {
        project: fixture.project.clone(),
        slot: AUXILIARY_SLOT,
    };
    wait(
        application,
        context,
        "missing source draft",
        |application| application.missing_drafts.contains_key(&fixture.peek_path),
    );
    let output = paint(application, context, Vec::new());
    for key in ["editor.sourceDeleted", "editor.saveDraftAs"] {
        let label = presentation::message(&application.locale, key, &[]);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains(&label))));
    }
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains(DRAFT))));
    assert!(!application.files.contains_key(&fixture.peek_path));
    paint(application, context, vec![egui::Event::Text("Z".into())]);
    assert_eq!(
        application.missing_drafts[&fixture.peek_path]
            .mirror
            .content,
        DRAFT
    );
    let destination = fixture.directory.join("project/recovered.txt");
    assert!(application.submit(HostCommand::SaveMissingDraft {
        tab: tab.clone(),
        draft: application.missing_drafts[&fixture.peek_path].clone(),
        destination: destination.to_str().unwrap().into(),
    }));
    wait(application, context, "missing draft saved", |application| {
        !application.missing_drafts.contains_key(&fixture.peek_path)
    });
    let layout = application.services.state.layouts.read()[&fixture.project].clone();
    let auxiliary = &layout
        .auxiliary_windows
        .iter()
        .find(|window| window.slot == AUXILIARY_SLOT)
        .unwrap()
        .root;
    assert!(crate::tabs::tabs_in(auxiliary).iter().any(|candidate| matches!(&candidate.kind, TabKind::File {path} if PathBuf::from(path) == destination)), "saved draft opened outside its source window");
    assert!(
        crate::tabs::tabs_in(auxiliary)
            .iter()
            .any(|candidate| candidate.id == tab && !candidate.dirty)
    );
    assert!(!PathBuf::from(&fixture.peek_path).exists());
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), DRAFT);
    wait(application, context, "saved draft body", |application| {
        application
            .files
            .contains_key(destination.to_str().unwrap())
    });
    assert!(
        taide_file::service::list_mirrors(&application.services.state.paths, &fixture.project)
            .unwrap()
            .is_empty()
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), TEXT);
}
