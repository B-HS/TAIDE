use super::inlay_hints_tests::{
    body_view, fixture_with_view_state, fixture_with_view_state_and_setup, key, paint, wait,
};
use super::*;
use taide_model::layout::PaneNode;
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet};

const ANCHOR_COLUMN: u32 = 11;
const STORED_LINE: u32 = 2;
const ANCHOR_BYTE: usize = 18;
const HEAD_COLUMN: u32 = 3;
const HEAD_BYTE: usize = 8;
const SHIFT_HEAD_BYTE: usize = 12;
const SHIFT_HEAD_COLUMN: u32 = 5;
const STABLE_FRAMES: usize = 16;
const LONG_LINE_COUNT: usize = 144;
const WIDE_RUNS: usize = 40;
const FIRST_VISIBLE_LINE: usize = 45;
const SCROLL_LEFT: f32 = 72.0;
const SCROLL_DELTA_TOP: f32 = -7.0;
const LEGACY_SCROLL_TOP: f32 = 320.0;
const NAVIGATION_LINE: u32 = 100;
const INVALID_UTF8: u8 = 0xff;
const IDE_TEXT: &str = "class\n  \u{1f600}method\nend";

#[test]
fn 실제_ide_파일_열기와_닫힘은_본문의_최신_위치를_저장한다() {
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        IDE_TEXT,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let selection = SelectionSet {
        primary: 0,
        selections: vec![Selection {
            anchor: ANCHOR_BYTE,
            head: HEAD_BYTE,
        }],
    };
    application
        .store
        .set_view_state(
            view,
            selection.clone(),
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    let services = application.services.clone();
    let actions = application.application_ports.ide.actions.clone();
    let arguments = serde_json::json!({ "filePath": fixture.peek_path, "preview": true });
    let opened = application.runtime.spawn(async move {
        crate::ide_tools::dispatch(services, &actions, "openFile", &arguments).await
    });
    wait(application, context, "IDE file open", |_| {
        opened.is_finished()
    });
    assert!(application.runtime.block_on(opened).unwrap().is_ok());
    let layout = application.services.state.layouts.read()[&fixture.project].clone();
    let tab = crate::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| tab.id == fixture.tab)
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(
        tab.view_state
            .as_deref()
            .expect("IDE open must capture the previous view"),
    )
    .unwrap();
    assert_eq!(
        saved["cursorState"][0]["selectionStart"],
        serde_json::json!({ "lineNumber": STORED_LINE, "column": ANCHOR_COLUMN })
    );
    assert_eq!(
        saved["cursorState"][0]["position"],
        serde_json::json!({ "lineNumber": STORED_LINE, "column": SHIFT_HEAD_COLUMN })
    );
    application
        .store
        .set_view_state(
            view,
            selection.clone(),
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let services = application.services.clone();
    let actions = application.application_ports.ide.actions.clone();
    let arguments = serde_json::json!({ "tab_name": tab.title });
    let closed = application.runtime.spawn(async move {
        crate::ide_tools::dispatch(services, &actions, "close_tab", &arguments).await
    });
    wait(application, context, "IDE file close", |_| {
        closed.is_finished()
    });
    assert!(application.runtime.block_on(closed).unwrap().is_ok());
    let layout = application.services.state.layouts.read()[&fixture.project].clone();
    let closed = layout
        .closed_tabs
        .iter()
        .find(|closed| closed.tab.id == fixture.tab)
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(
        closed
            .tab
            .view_state
            .as_deref()
            .expect("IDE close must capture the latest inactive view"),
    )
    .unwrap();
    assert_eq!(
        saved["cursorState"][0]["position"],
        serde_json::json!({ "lineNumber": STORED_LINE, "column": HEAD_COLUMN })
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), IDE_TEXT);
    let document = application
        .store
        .documents()
        .snapshot(application.store.views().get(view).unwrap().document)
        .unwrap();
    assert_eq!(
        document.rope.to_string(),
        std::fs::read_to_string(&fixture.path).unwrap()
    );
    assert!(!document.dirty);
}

#[test]
fn 실제_remote_탭_명령과_프로젝트_종료는_최신_본문_위치를_보존한다() {
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        IDE_TEXT,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let selection = SelectionSet {
        primary: 0,
        selections: vec![Selection {
            anchor: ANCHOR_BYTE,
            head: HEAD_BYTE,
        }],
    };
    application
        .store
        .set_view_state(
            view,
            selection.clone(),
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    let services = application.services.clone();
    let dispatch = application.application_ports.dispatch.clone();
    let arguments = serde_json::json!({ "projectId": fixture.project, "kind": TabKind::File { path: fixture.peek_path.clone() }, "title": "remote peer", "preview": true });
    let opened = application.runtime.spawn(async move {
        let channels: crate::remote_ws::ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("layout command must not create a channel")));
        (dispatch.json)(services, "layout_open_tab".into(), arguments, channels).await
    });
    wait(application, context, "remote file open", |_| {
        opened.is_finished()
    });
    let opened = application.runtime.block_on(opened).unwrap();
    assert!(opened.is_ok(), "{opened:?}");
    let layout = application.services.state.layouts.read()[&fixture.project].clone();
    let tab = crate::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| tab.id == fixture.tab)
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(
        tab.view_state
            .as_deref()
            .expect("remote open must capture the previous view"),
    )
    .unwrap();
    assert_eq!(
        saved["cursorState"][0]["position"],
        serde_json::json!({ "lineNumber": STORED_LINE, "column": SHIFT_HEAD_COLUMN })
    );
    application
        .store
        .set_view_state(view, selection, ScrollPosition::default(), Vec::new())
        .unwrap();
    let services = application.services.clone();
    let dispatch = application.application_ports.dispatch.clone();
    let arguments = serde_json::json!({ "tabId": fixture.tab });
    let closed = application.runtime.spawn(async move {
        let channels: crate::remote_ws::ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("layout command must not create a channel")));
        (dispatch.json)(services, "layout_close_tab".into(), arguments, channels).await
    });
    wait(application, context, "remote file close", |_| {
        closed.is_finished()
    });
    assert!(application.runtime.block_on(closed).unwrap().is_ok());
    let layout = application.services.state.layouts.read()[&fixture.project].clone();
    let closed = layout
        .closed_tabs
        .iter()
        .find(|closed| closed.tab.id == fixture.tab)
        .unwrap();
    let saved: serde_json::Value =
        serde_json::from_str(closed.tab.view_state.as_deref().unwrap()).unwrap();
    assert_eq!(
        saved["cursorState"][0]["position"],
        serde_json::json!({ "lineNumber": STORED_LINE, "column": HEAD_COLUMN })
    );
    let peer = crate::tabs::tabs_in(&layout.root)
        .into_iter()
        .find(|tab| matches!(&tab.kind, TabKind::File { path } if path == &fixture.peek_path))
        .unwrap()
        .id
        .clone();
    wait(application, context, "remote peer view", |application| {
        application
            .files
            .get(&fixture.peek_path)
            .is_some_and(|file| {
                application
                    .store
                    .views()
                    .for_document(file.id)
                    .any(|view| view.key.tab == peer)
            })
    });
    let peer_view = body_view(application, &fixture.peek_path, &peer);
    application
        .store
        .set_view_state(
            peer_view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: 1, head: 1 }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let services = application.services.clone();
    let dispatch = application.application_ports.dispatch.clone();
    let arguments = serde_json::json!({ "projectId": fixture.project });
    let project_closed = application.runtime.spawn(async move {
        let channels: crate::remote_ws::ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("project command must not create a channel")));
        (dispatch.json)(services, "project_close".into(), arguments, channels).await
    });
    wait(application, context, "remote project close", |_| {
        project_closed.is_finished()
    });
    assert!(
        application
            .runtime
            .block_on(project_closed)
            .unwrap()
            .is_ok()
    );
    assert!(
        !application
            .services
            .state
            .projects
            .read()
            .contains_key(&fixture.project)
    );
    assert!(
        !application
            .services
            .state
            .layouts
            .read()
            .contains_key(&fixture.project)
    );
    let persisted =
        taide_layout::service::load_layout(&application.services.state.paths, &fixture.project);
    let peer = crate::tabs::tabs_in(&persisted.root)
        .into_iter()
        .find(|tab| tab.id == peer)
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(
        peer.view_state
            .as_deref()
            .expect("project close must flush the latest peer position before detach"),
    )
    .unwrap();
    assert_eq!(
        saved["cursorState"][0]["position"],
        serde_json::json!({ "lineNumber": 1, "column": 2 })
    );
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), IDE_TEXT);
}

#[test]
fn 실제_remote_슬롯_전환_닫힘과_host_프로젝트_열기는_최신_위치를_저장한다() {
    use std::path::Path;
    use taide_model::ids::ShellSlotId;
    use taide_model::layout::SplitDir;
    use taide_model::project::{Project, ProjectRef, ShellSlotTree};

    let second = ProjectId::new();
    let second_slot = ShellSlotId::new();
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        IDE_TEXT,
        |application, _, project, _| {
            let first_root = application.services.state.projects.read()[project]
                .root
                .clone();
            let second_root = Path::new(&first_root)
                .parent()
                .unwrap()
                .join("second-project");
            std::fs::create_dir_all(&second_root).unwrap();
            let second_root = second_root.to_str().unwrap().to_owned();
            application.services.state.projects.write().insert(
                second.clone(),
                Project {
                    id: second.clone(),
                    root: second_root.clone(),
                    name: "second".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            application
                .services
                .state
                .layouts
                .write()
                .insert(second.clone(), taide_layout::service::default_layout());
            let mut session = application.services.state.session.write();
            session.projects.push(ProjectRef {
                id: second.clone(),
                root: second_root,
                name: "second".into(),
                display: Default::default(),
                root_missing: false,
            });
            let first_slot = session.shell_slots.take().unwrap();
            session.shell_slots = Some(ShellSlotTree::Split {
                dir: SplitDir::Horizontal,
                children: vec![
                    first_slot,
                    ShellSlotTree::Leaf {
                        slot_id: second_slot.clone(),
                        project_id: second.clone(),
                    },
                ],
                sizes: vec![1.0, 1.0],
            });
        },
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let first_slot = application
        .services
        .state
        .session
        .read()
        .focused_shell_slot
        .clone()
        .unwrap();
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.store.views().get(view).unwrap().document;
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: ANCHOR_BYTE,
                    head: HEAD_BYTE,
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
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
        SHIFT_HEAD_BYTE
    );
    for (name, arguments, selection, line, column) in [
        (
            "session_focus_shell_slot",
            serde_json::json!({"slotId": second_slot}),
            Selection {
                anchor: ANCHOR_BYTE,
                head: SHIFT_HEAD_BYTE,
            },
            STORED_LINE,
            SHIFT_HEAD_COLUMN,
        ),
        (
            "shell_slot_close",
            serde_json::json!({"slotId": first_slot}),
            Selection {
                anchor: ANCHOR_BYTE,
                head: HEAD_BYTE,
            },
            STORED_LINE,
            HEAD_COLUMN,
        ),
        (
            "project_activate",
            serde_json::json!({"projectId": fixture.project}),
            Selection { anchor: 1, head: 1 },
            1,
            2,
        ),
    ] {
        application
            .store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![selection],
                },
                ScrollPosition::default(),
                Vec::new(),
            )
            .unwrap();
        let services = application.services.clone();
        let dispatch = application.application_ports.dispatch.clone();
        let command = name.to_owned();
        let result = application.runtime.spawn(async move {
            let channels: crate::remote_ws::ChannelFactory =
                Arc::new(|_| Box::new(|_| panic!("project command has no channels")));
            (dispatch.json)(services, command, arguments, channels).await
        });
        wait(application, context, name, |_| result.is_finished());
        let result = application.runtime.block_on(result).unwrap();
        assert!(result.is_ok(), "{name}: {result:?}");
        let layouts = application.services.state.layouts.read();
        let tab = crate::tabs::tabs_in(&layouts[&fixture.project].root)
            .into_iter()
            .find(|tab| tab.id == fixture.tab)
            .unwrap();
        let saved: serde_json::Value =
            serde_json::from_str(tab.view_state.as_deref().unwrap()).unwrap();
        assert_eq!(
            saved["cursorState"][0]["position"],
            serde_json::json!({"lineNumber": line, "column": column}),
            "{name}"
        );
    }
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: ANCHOR_BYTE,
                    head: HEAD_BYTE,
                }],
            },
            ScrollPosition::default(),
            Vec::new(),
        )
        .unwrap();
    let root = application.services.state.projects.read()[&fixture.project]
        .root
        .clone();
    assert!(application.submit(HostCommand::OpenProject(root)));
    wait(
        application,
        context,
        "host project location",
        |application| {
            let layouts = application.services.state.layouts.read();
            crate::tabs::tabs_in(&layouts[&fixture.project].root)
                .into_iter()
                .find(|tab| tab.id == fixture.tab)
                .and_then(|tab| tab.view_state.as_deref())
                .and_then(|saved| serde_json::from_str::<serde_json::Value>(saved).ok())
                .is_some_and(|saved| {
                    saved["cursorState"][0]["position"]
                        == serde_json::json!({"lineNumber": STORED_LINE, "column": HEAD_COLUMN})
                })
        },
    );
    let current = application.store.documents().snapshot(document).unwrap();
    assert_eq!(current.rope.to_string(), IDE_TEXT);
    assert!(!current.dirty);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), IDE_TEXT);
}

#[test]
fn 실제_preview_교체_뒤_같은_메모리_모델을_다시_열면_마지막_선택과_scroll이_복원된다() {
    let text = format!(
        "class\n  \u{1f600}method\n{}",
        (0..LONG_LINE_COUNT)
            .map(|index| format!("row {index}\n"))
            .collect::<String>()
    );
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        None,
        &text,
        |application, _, project, tab| {
            let mut layouts = application.services.state.layouts.write();
            let PaneNode::Leaf { tabs, .. } = &mut layouts.get_mut(project).unwrap().root else {
                panic!("expected leaf");
            };
            tabs.iter_mut()
                .find(|candidate| &candidate.id == tab)
                .unwrap()
                .preview = true;
        },
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: ANCHOR_BYTE,
                    head: HEAD_BYTE,
                }],
            },
            ScrollPosition {
                x: 0.0,
                y: LEGACY_SCROLL_TOP,
            },
            Vec::new(),
        )
        .unwrap();
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    for _ in 0..STABLE_FRAMES {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
    }
    let previous = application.store.views().get(view).unwrap();
    let scroll = previous.scroll.clone();
    let original = application
        .store
        .documents()
        .snapshot(previous.document)
        .unwrap();
    let pane = previous.key.pane.clone();
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: Some(pane.clone()),
        path: fixture.peek_path.clone(),
        preview: true
    }));
    wait(application, context, "preview replaced", |application| {
        let snapshot = application.controller.snapshot();
        !crate::tabs::tabs_in(&snapshot.layouts[&fixture.project].root)
            .iter()
            .any(|tab| tab.id == fixture.tab)
            && application.files.contains_key(&fixture.peek_path)
    });
    assert!(application.submit(HostCommand::OpenFileTab {
        project: fixture.project.clone(),
        pane: Some(pane),
        path: fixture.path.clone(),
        preview: true
    }));
    wait(
        application,
        context,
        "preview model reopened",
        |application| {
            let snapshot = application.controller.snapshot();
            let tab = crate::tabs::tabs_in(&snapshot.layouts[&fixture.project].root)
                .into_iter()
                .find(|tab| matches!(&tab.kind, TabKind::File { path } if path == &fixture.path));
            tab.is_some_and(|tab| {
                application.files.get(&fixture.path).is_some_and(|file| {
                    application
                        .store
                        .views()
                        .for_document(file.id)
                        .any(|view| view.key.tab == tab.id)
                })
            })
        },
    );
    let snapshot = application.controller.snapshot();
    let tab = crate::tabs::tabs_in(&snapshot.layouts[&fixture.project].root)
        .into_iter()
        .find(|tab| matches!(&tab.kind, TabKind::File { path } if path == &fixture.path))
        .unwrap();
    assert_ne!(tab.id, fixture.tab);
    let reopened = body_view(application, &fixture.path, &tab.id);
    let current = application.store.views().get(reopened).unwrap();
    assert_eq!(current.document, original.id);
    assert_eq!(
        current.selection.selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: SHIFT_HEAD_BYTE
        }],
        "same-session model position was lost with its preview tab"
    );
    assert_eq!(current.scroll, scroll);
    let document = application
        .store
        .documents()
        .snapshot(current.document)
        .unwrap();
    assert_eq!(document.rope.to_string(), text);
    assert_eq!(document.revision, original.revision);
    assert!(!document.dirty);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), text);
}

#[test]
fn 실제_셸_탭_전환과_host_preview_열기는_이전_본문의_최신_위치를_저장한다() {
    let text = format!(
        "class\n  \u{1f600}method\n{}",
        (0..LONG_LINE_COUNT)
            .map(|index| format!("row {index}\n"))
            .collect::<String>()
    );
    for is_host in [false, true] {
        let other = TabId::new();
        let mut fixture = fixture_with_view_state_and_setup(
            "--native-inlay-hints",
            false,
            |_, _, _| true,
            None,
            &text,
            |application, _, project, _| {
                if is_host {
                    return;
                }
                let mut layouts = application.services.state.layouts.write();
                let PaneNode::Leaf { tabs, .. } = &mut layouts.get_mut(project).unwrap().root
                else {
                    panic!("expected fixture leaf");
                };
                tabs.push(Tab {
                    id: other.clone(),
                    kind: TabKind::Welcome,
                    title: "other".into(),
                    pinned: false,
                    preview: false,
                    dirty: false,
                    view_state: None,
                });
            },
        );
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let view = body_view(application, &fixture.path, &fixture.tab);
        application
            .store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: ANCHOR_BYTE,
                        head: HEAD_BYTE,
                    }],
                },
                ScrollPosition {
                    x: 0.0,
                    y: LEGACY_SCROLL_TOP,
                },
                Vec::new(),
            )
            .unwrap();
        let focus = crate::editor_context_menu::body(context, view).unwrap();
        context.memory_mut(|memory| memory.request_focus(focus));
        paint(
            application,
            context,
            key(
                egui::Key::ArrowRight,
                egui::Modifiers {
                    shift: true,
                    ..egui::Modifiers::NONE
                },
            ),
        );
        for _ in 0..STABLE_FRAMES {
            eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
            paint(application, context, Vec::new());
        }
        let pane = application
            .store
            .views()
            .get(view)
            .unwrap()
            .key
            .pane
            .clone();
        if is_host {
            assert!(application.submit(HostCommand::OpenFileTab {
                project: fixture.project.clone(),
                pane: Some(pane),
                path: fixture.peek_path.clone(),
                preview: true
            }));
        } else {
            application
                .controller
                .submit(ShellMutation::ActivateTab(other.clone()))
                .unwrap();
        }
        wait(
            application,
            context,
            "transition view state",
            |application| {
                let snapshot = application.controller.snapshot();
                let PaneNode::Leaf { tabs, active, .. } = &snapshot.layouts[&fixture.project].root
                else {
                    return false;
                };
                if !is_host {
                    return active.as_ref() == Some(&other);
                }
                tabs.iter().any(|tab| {
                    matches!(&tab.kind, TabKind::File { path } if path == &fixture.peek_path)
                        && active.as_ref() == Some(&tab.id)
                }) && application.files.contains_key(&fixture.peek_path)
            },
        );
        let snapshot = application.controller.snapshot();
        let previous = crate::tabs::tabs_in(&snapshot.layouts[&fixture.project].root)
            .into_iter()
            .find(|tab| tab.id == fixture.tab)
            .unwrap();
        let saved = previous.view_state.as_deref().unwrap_or_else(|| {
            panic!("latest position must be saved before leaving the body; is_host={is_host}")
        });
        let saved: serde_json::Value = serde_json::from_str(saved).unwrap();
        assert_eq!(
            saved["cursorState"][0]["selectionStart"],
            serde_json::json!({"lineNumber": STORED_LINE, "column": ANCHOR_COLUMN})
        );
        assert_eq!(
            saved["cursorState"][0]["position"],
            serde_json::json!({"lineNumber": STORED_LINE, "column": SHIFT_HEAD_COLUMN})
        );
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), text);
    }
}

#[test]
fn 실제_탭_닫힘은_최신_선택과_scroll을_최근_닫힘에_보존하고_다시_열때_복원한다() {
    let text = format!(
        "class\n  \u{1f600}method\n{}",
        (0..LONG_LINE_COUNT)
            .map(|index| format!("row {index}\n"))
            .collect::<String>()
    );
    let mut fixture =
        fixture_with_view_state("--native-inlay-hints", false, |_, _, _| true, None, &text);
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    application
        .store
        .set_view_state(
            view,
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: ANCHOR_BYTE,
                    head: HEAD_BYTE,
                }],
            },
            ScrollPosition {
                x: 0.0,
                y: LEGACY_SCROLL_TOP,
            },
            Vec::new(),
        )
        .unwrap();
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    for _ in 0..STABLE_FRAMES {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
    }
    let current = application.store.views().get(view).unwrap();
    let scroll = current.scroll.clone();
    assert_eq!(
        current.selection.selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: SHIFT_HEAD_BYTE
        }]
    );
    application.request_tab_close(fixture.tab.clone());
    wait(application, context, "closed view state", |application| {
        application.controller.snapshot().layouts[&fixture.project]
            .closed_tabs
            .iter()
            .any(|closed| closed.tab.id == fixture.tab)
    });
    let snapshot = application.controller.snapshot();
    let closed = snapshot.layouts[&fixture.project]
        .closed_tabs
        .iter()
        .find(|closed| closed.tab.id == fixture.tab)
        .unwrap();
    let saved = closed
        .tab
        .view_state
        .as_deref()
        .expect("latest live position must reach closed-tab history");
    let saved: serde_json::Value = serde_json::from_str(saved).unwrap();
    assert_eq!(
        saved["cursorState"][0]["position"],
        serde_json::json!({"lineNumber": STORED_LINE, "column": SHIFT_HEAD_COLUMN})
    );
    application
        .controller
        .submit(ShellMutation::ReopenClosed(fixture.project.clone()))
        .unwrap();
    wait(application, context, "reopened view state", |application| {
        let snapshot = application.controller.snapshot();
        let has_tab = crate::tabs::tabs_in(&snapshot.layouts[&fixture.project].root)
            .iter()
            .any(|tab| tab.id == fixture.tab);
        has_tab
            && application.files.get(&fixture.path).is_some_and(|file| {
                application
                    .store
                    .views()
                    .for_document(file.id)
                    .any(|view| view.key.tab == fixture.tab)
            })
    });
    let reopened = body_view(application, &fixture.path, &fixture.tab);
    let current = application.store.views().get(reopened).unwrap();
    assert_eq!(
        current.selection.selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: SHIFT_HEAD_BYTE
        }]
    );
    assert_eq!(current.scroll, scroll);
    let document = application
        .store
        .documents()
        .snapshot(current.document)
        .unwrap();
    assert_eq!(document.rope.to_string(), text);
    assert!(!document.dirty);
    assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), text);
}

#[test]
fn 실제_정상과_직접_종료는_마지막_본문_선택과_scroll을_layout에_저장한다() {
    let text = format!(
        "class\n  \u{1f600}method\n{}",
        (0..LONG_LINE_COUNT)
            .map(|index| format!("row {index} {}\n", "wide text ".repeat(WIDE_RUNS)))
            .collect::<String>()
    );
    for is_direct in [true, false] {
        let mut fixture =
            fixture_with_view_state("--native-inlay-hints", false, |_, _, _| true, None, &text);
        let mut application = fixture.application.take().unwrap();
        let view = body_view(&application, &fixture.path, &fixture.tab);
        application
            .store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 0,
                    selections: vec![Selection {
                        anchor: ANCHOR_BYTE,
                        head: HEAD_BYTE,
                    }],
                },
                ScrollPosition {
                    x: SCROLL_LEFT,
                    y: LEGACY_SCROLL_TOP,
                },
                Vec::new(),
            )
            .unwrap();
        let focus = crate::editor_context_menu::body(&fixture.context, view).unwrap();
        fixture
            .context
            .memory_mut(|memory| memory.request_focus(focus));
        paint(
            &mut application,
            &fixture.context,
            key(
                egui::Key::ArrowRight,
                egui::Modifiers {
                    shift: true,
                    ..egui::Modifiers::NONE
                },
            ),
        );
        for _ in 0..STABLE_FRAMES {
            eframe::App::logic(
                &mut application,
                &fixture.context,
                &mut eframe::Frame::_new_kittest(),
            );
            paint(&mut application, &fixture.context, Vec::new());
        }
        let current = application.store.views().get(view).unwrap();
        assert_eq!(
            current.selection.selections,
            vec![Selection {
                anchor: ANCHOR_BYTE,
                head: SHIFT_HEAD_BYTE
            }]
        );
        let scroll = current.scroll.clone();
        let display = current.display.clone().unwrap();
        let line_height = application.editor.appearance.line_height;
        let original = application
            .store
            .documents()
            .snapshot(current.document)
            .unwrap();
        if is_direct {
            eframe::App::on_exit(&mut application);
        } else {
            application.close(&fixture.context);
            wait(
                &mut application,
                &fixture.context,
                "view state exit",
                |application| application.is_exit_ready,
            );
        }
        let layout =
            taide_layout::service::load_layout(&application.services.state.paths, &fixture.project);
        let tab = crate::tabs::tabs_in(&layout.root)
            .into_iter()
            .find(|tab| tab.id == fixture.tab)
            .unwrap();
        let serialized = tab
            .view_state
            .as_deref()
            .expect("latest view state must survive app shutdown");
        let saved: serde_json::Value = serde_json::from_str(serialized).unwrap();
        assert_eq!(
            saved["cursorState"][0]["selectionStart"],
            serde_json::json!({"lineNumber": STORED_LINE, "column": ANCHOR_COLUMN})
        );
        assert_eq!(
            saved["cursorState"][0]["position"],
            serde_json::json!({"lineNumber": STORED_LINE, "column": SHIFT_HEAD_COLUMN})
        );
        assert_eq!(
            saved["viewState"]["scrollLeft"].as_f64(),
            Some(f64::from(scroll.x))
        );
        let restored = application
            .store
            .attach_view(
                ViewKey {
                    window: "persisted-view".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                original.id,
            )
            .unwrap();
        crate::editor_view_state::State::default()
            .restore(&mut application.store, restored, Some(serialized))
            .unwrap();
        assert_eq!(
            application
                .store
                .views()
                .get(restored)
                .unwrap()
                .selection
                .selections,
            vec![Selection {
                anchor: ANCHOR_BYTE,
                head: SHIFT_HEAD_BYTE
            }]
        );
        let anchor = application
            .store
            .take_scroll_restore(restored)
            .unwrap()
            .unwrap();
        let restored_top = taide_native_editor::display_layout::VerticalLayout::new(
            line_height,
            display.row_count(),
        )
        .row_top(
            display.row_of_byte(
                &original,
                anchor
                    .byte
                    .expect("rendered view must preserve its model anchor"),
            ),
        ) - anchor.delta_top;
        assert_eq!(restored_top, scroll.y);
        let current = application.store.documents().snapshot(original.id).unwrap();
        assert_eq!(current.rope.to_string(), text);
        assert_eq!(current.revision, original.revision);
        assert_eq!(current.dirty, original.dirty);
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), text);
        assert_eq!(application.services.tasks.tracked_count(), 0);
    }
}

#[test]
fn 실제_본문은_저장된_utf16_역방향_선택을_복원하고_후속_layout에서_재복원하지_않는다() {
    let saved = serde_json::json!({
        "cursorState": [{
            "inSelectionMode": true,
            "selectionStart": {"lineNumber": STORED_LINE, "column": ANCHOR_COLUMN},
            "position": {"lineNumber": STORED_LINE, "column": HEAD_COLUMN}
        }],
        "viewState": {
            "scrollLeft": 0,
            "firstPosition": {"lineNumber": 1, "column": 1},
            "firstPositionDeltaTop": 0
        },
        "contributionsState": {}
    })
    .to_string();
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        Some(saved),
        "class\n  \u{1f600}method\nend",
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    let original = application.store.documents().snapshot(document).unwrap();
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: HEAD_BYTE,
        }],
        "persisted Unicode selection must reach the live model"
    );
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    for _ in 0..STABLE_FRAMES {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
    }
    let changed = Selection {
        anchor: ANCHOR_BYTE,
        head: SHIFT_HEAD_BYTE,
    };
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![changed]
    );
    let reset = serde_json::json!({
        "cursorState": [{
            "selectionStart": {"lineNumber": 1, "column": 1},
            "position": {"lineNumber": 1, "column": 1}
        }],
        "viewState": {"scrollLeft": 0, "scrollTopWithoutViewZones": 0}
    })
    .to_string();
    application
        .runtime
        .handle()
        .clone()
        .block_on(taide_runtime::layout_actions::layout_set_view_state(
            application.services.events.as_ref(),
            &application.services.state,
            fixture.tab.clone(),
            Some(reset.clone()),
        ))
        .unwrap();
    wait(application, context, "layout metadata", |application| {
        crate::tabs::tabs_in(&application.controller.snapshot().layouts[&fixture.project].root)
            .iter()
            .any(|tab| tab.id == fixture.tab && tab.view_state.as_ref() == Some(&reset))
    });
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![changed]
    );
    let current = application.store.documents().snapshot(document).unwrap();
    assert_eq!(current.rope.to_string(), original.rope.to_string());
    assert_eq!(current.revision, original.revision);
    assert_eq!(current.dirty, original.dirty);
    assert_eq!(
        std::fs::read_to_string(&fixture.path).unwrap(),
        original.rope.to_string()
    );
}

#[test]
fn 실제_초기_명시적_줄_이동은_저장된_scroll보다_우선한다() {
    let text = (0..LONG_LINE_COUNT)
        .map(|index| format!("row {index}\n"))
        .collect::<String>();
    let saved = serde_json::json!({
        "cursorState": [{"position": {"lineNumber": 1, "column": 1}}],
        "viewState": {
            "firstPosition": {"lineNumber": FIRST_VISIBLE_LINE, "column": 1},
            "firstPositionDeltaTop": SCROLL_DELTA_TOP
        }
    })
    .to_string();
    let mut fixture = fixture_with_view_state_and_setup(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        Some(saved),
        &text,
        |application, path, project, tab| {
            assert!(application.reveals.queue_position(
                crate::editor_reveal::Target {
                    project,
                    tab,
                    path,
                    viewport: egui::ViewportId::ROOT,
                },
                crate::editor_reveal::Position {
                    line: f64::from(NAVIGATION_LINE),
                    column: 1.0
                },
                &application.services.state.layouts.read(),
                Instant::now(),
            ));
        },
    );
    let application = fixture.application.as_mut().unwrap();
    let view = body_view(application, &fixture.path, &fixture.tab);
    let state = application.store.views().get(view).unwrap();
    let document = application
        .store
        .documents()
        .snapshot(state.document)
        .unwrap();
    let byte = document
        .rope
        .line_to_byte(usize::try_from(NAVIGATION_LINE).unwrap() - 1);
    assert_eq!(
        state.selection.selections,
        vec![Selection {
            anchor: byte,
            head: byte
        }]
    );
    let display = state.display.as_ref().unwrap();
    let saved_byte = document.rope.line_to_byte(FIRST_VISIBLE_LINE - 1);
    let layout = taide_native_editor::display_layout::VerticalLayout::new(
        application.editor.appearance.line_height,
        display.row_count(),
    );
    let saved_top = layout.row_top(display.row_of_byte(&document, saved_byte)) - SCROLL_DELTA_TOP;
    assert!(
        state.scroll.y > saved_top,
        "explicit navigation was replaced by persisted scroll: {:?}",
        state.scroll
    );
    assert_eq!(document.rope.to_string(), text);
    assert!(!document.dirty);
}

#[test]
fn 실제_lossy_readonly_본문은_새_live_view의_위치를_복원하고_선택만_이동한다() {
    let saved = serde_json::json!({
        "cursorState": [{
            "selectionStart": {"lineNumber": STORED_LINE, "column": ANCHOR_COLUMN},
            "position": {"lineNumber": STORED_LINE, "column": HEAD_COLUMN}
        }],
        "viewState": {"scrollLeft": 0, "scrollTopWithoutViewZones": 0}
    })
    .to_string();
    let text = "class\n  \u{1f600}method\nend";
    let mut fixture = fixture_with_view_state(
        "--native-inlay-hints",
        false,
        |_, _, _| true,
        Some(saved),
        text,
    );
    let application = fixture.application.as_mut().unwrap();
    let context = &fixture.context;
    let previous_view = body_view(application, &fixture.path, &fixture.tab);
    let document = application.files[&fixture.path].id;
    let bytes = [text.as_bytes(), &[INVALID_UTF8]].concat();
    std::fs::write(&fixture.path, &bytes).unwrap();
    let opened = application
        .runtime
        .handle()
        .clone()
        .block_on(taide_runtime::native_file_actions::open_document_file(
            &application.services.state,
            &application.services.tasks,
            fixture.path.clone(),
            Vec::new,
        ))
        .unwrap();
    assert!(opened.file.read_only && opened.file.encoding_lossy);
    application
        .store
        .observe_file(document, &opened.canonical_path, opened.file)
        .unwrap();
    application.store.detach_view(previous_view).unwrap();
    paint(application, context, Vec::new());
    let view = body_view(application, &fixture.path, &fixture.tab);
    assert_ne!(view, previous_view);
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: HEAD_BYTE,
        }]
    );
    let original = application.store.documents().snapshot(document).unwrap();
    assert!(original.metadata.read_only && original.metadata.lossy);
    let focus = crate::editor_context_menu::body(context, view).unwrap();
    context.memory_mut(|memory| memory.request_focus(focus));
    paint(
        application,
        context,
        vec![egui::Event::Text("blocked".into())],
    );
    paint(
        application,
        context,
        key(
            egui::Key::ArrowRight,
            egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        ),
    );
    for _ in 0..STABLE_FRAMES {
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        paint(application, context, Vec::new());
    }
    assert_eq!(
        application
            .store
            .views()
            .get(view)
            .unwrap()
            .selection
            .selections,
        vec![Selection {
            anchor: ANCHOR_BYTE,
            head: SHIFT_HEAD_BYTE,
        }]
    );
    let current = application.store.documents().snapshot(document).unwrap();
    assert_eq!(current.rope.to_string(), original.rope.to_string());
    assert_eq!(current.revision, original.revision);
    assert_eq!(current.dirty, original.dirty);
    assert!(current.metadata.read_only);
    assert_eq!(std::fs::read(&fixture.path).unwrap(), bytes);
}

#[test]
fn 실제_본문은_커서와_독립된_첫_모델_위치와_legacy_scroll을_즉시_복원한다() {
    let text = (0..LONG_LINE_COUNT)
        .map(|index| format!("row {index} {}\n", "wide text ".repeat(WIDE_RUNS)))
        .collect::<String>();
    for (scroll, is_anchored) in [
        (
            serde_json::json!({
                "scrollLeft": SCROLL_LEFT,
                "firstPosition": {"lineNumber": FIRST_VISIBLE_LINE, "column": 1},
                "firstPositionDeltaTop": SCROLL_DELTA_TOP,
            }),
            true,
        ),
        (
            serde_json::json!({
                "scrollLeft": SCROLL_LEFT,
                "scrollTopWithoutViewZones": LEGACY_SCROLL_TOP,
            }),
            false,
        ),
    ] {
        let saved = serde_json::json!({
            "cursorState": [{"position": {"lineNumber": 1, "column": 1}}],
            "viewState": scroll,
        })
        .to_string();
        let mut fixture = fixture_with_view_state(
            "--native-inlay-hints",
            false,
            |_, _, _| true,
            Some(saved),
            &text,
        );
        let application = fixture.application.as_mut().unwrap();
        let context = &fixture.context;
        let view = body_view(application, &fixture.path, &fixture.tab);
        let state = application.store.views().get(view).unwrap();
        let document = application
            .store
            .documents()
            .snapshot(state.document)
            .unwrap();
        let expected_top = if is_anchored {
            let display = state.display.as_ref().unwrap();
            let byte = document.rope.line_to_byte(FIRST_VISIBLE_LINE - 1);
            let layout = taide_native_editor::display_layout::VerticalLayout::new(
                application.editor.appearance.line_height,
                display.row_count(),
            );
            layout.row_top(display.row_of_byte(&document, byte)) - SCROLL_DELTA_TOP
        } else {
            LEGACY_SCROLL_TOP
        };
        assert_eq!(
            state.selection.selections,
            vec![Selection { anchor: 0, head: 0 }]
        );
        assert_eq!(state.scroll.x, SCROLL_LEFT);
        assert_eq!(state.scroll.y, expected_top);
        for _ in 0..STABLE_FRAMES {
            eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
            paint(application, context, Vec::new());
        }
        let state = application.store.views().get(view).unwrap();
        assert_eq!(state.scroll.x, SCROLL_LEFT);
        assert_eq!(state.scroll.y, expected_top);
        let current = application
            .store
            .documents()
            .snapshot(state.document)
            .unwrap();
        assert_eq!(current.revision, document.revision);
        assert!(!current.dirty);
        assert_eq!(current.rope.to_string(), text);
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), text);
    }
}
