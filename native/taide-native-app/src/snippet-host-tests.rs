use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use eframe::egui::{self, Event, PointerButton, Pos2, RawInput, Rect};
use taide_model::{
    app_event::AppEvent,
    error::AppError,
    ids::{PaneId, ProjectId, TabId},
    layout::{PaneNode, Tab, TabKind},
    paths::AppPaths,
};
use taide_native_ui::{
    icons::Icons,
    settings_owner::Owner,
    settings_view::Appearance,
    snippet_edit::Outcome,
    snippet_editor::{Editor, Output},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor, locale_actions, theme_actions};

use crate::host::{HostBridge, HostCommand, HostReply};

const DEADLINE: Duration = Duration::from_secs(5);
const FRAME_STEP: f64 = 0.1;
const SCREEN: [f32; 2] = [1000.0, 900.0];

struct Events;

impl EventSink for Events {
    fn publish(&self, _: AppEvent) {}
}

#[test]
fn snippet_host의_승인된_실제_저장은_큐_대기중_unmount에도_보존되고_늦은_조회는_거절된다() {
    let directory =
        std::env::temp_dir().join(format!("taide-m8-snippet-host-{}", ProjectId::new()));
    std::fs::create_dir_all(&directory).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let mut layout = taide_layout::service::default_layout();
    layout.root = PaneNode::Leaf {
        id: owner.pane.clone(),
        tabs: vec![Tab {
            id: owner.tab.clone(),
            kind: TabKind::Settings,
            title: "Synthetic".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(owner.tab.clone()),
    };
    layout.focused_pane = owner.pane.clone();
    state.layouts.write().insert(owner.project.clone(), layout);
    let theme = theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
    let appearance = Appearance::new(&theme).unwrap();
    let snippets = taide_native_ui::snippet_editor::Appearance::new(&theme).unwrap();
    let locale = locale_actions::locale_get(&state, "en".into()).unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = crate::bootstrap::services(state.clone(), tasks, Arc::new(Events));
    let (started, ready) = tokio::sync::oneshot::channel();
    let started = Mutex::new(Some(started));
    let (release, gate) = mpsc::channel();
    let gate = Mutex::new(gate);
    let writer = Arc::new(move |_: &str| {
        started.lock().unwrap().take().unwrap().send(()).unwrap();
        gate.lock().unwrap().recv_timeout(DEADLINE).unwrap();
        Ok(())
    });
    let mut bridge = HostBridge::connect_with_clipboard_ports(
        services.clone(),
        Arc::new(|| {}),
        writer,
        Arc::new(|| {
            Err(AppError::Forbidden(
                "synthetic clipboard read is not allowed".into(),
            ))
        }),
        None,
    )
    .unwrap();
    let context = egui::Context::default();
    let mut icons = Icons::new().unwrap();
    let mut editor = Editor::new(owner);
    let mut time = 0.0;
    let mut show = |editor: &mut Editor, events: Vec<Event>| {
        time += FRAME_STEP;
        let mut output = Output::default();
        let mut drawing = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                icons.prepare(ui.ctx()).unwrap();
                let mut pass = editor.show(ui, &locale, &appearance, &snippets, &icons);
                output.requests.append(&mut pass.requests);
                output.traces = pass.traces;
                output.interactions = pass.interactions;
            },
        );
        drawing.textures_delta.clear();
        output
    };
    let list = show(&mut editor, Vec::new()).requests.pop().unwrap();
    bridge
        .submit(HostCommand::SnippetEdit(list.clone()))
        .unwrap();
    let receive = |bridge: &mut HostBridge| {
        runtime.block_on(async {
            tokio::time::timeout(DEADLINE, async {
                loop {
                    if let Some(reply) = bridge.poll() {
                        return reply;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap()
        })
    };
    let HostReply::SnippetEdit(reply) = receive(&mut bridge) else {
        panic!("expected snippet list");
    };
    assert!(matches!(&reply.result, Ok(Outcome::Listed(files)) if files.is_empty()));
    assert!(editor.accept(reply));
    editor.state_mut().new_file.open = true;
    show(&mut editor, Vec::new());
    let modal = show(&mut editor, Vec::new());
    let (_, id, rect) = modal
        .traces
        .iter()
        .find(|(name, _, _)| name == "dialog-confirm")
        .unwrap();
    let point = rect.center();
    assert!(modal.interactions[id].contains(point));
    let save = show(
        &mut editor,
        vec![
            Event::PointerMoved(point),
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    )
    .requests
    .pop()
    .unwrap();
    bridge
        .submit(HostCommand::CopyText("synthetic gate".into()))
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, ready)
            .await
            .unwrap()
            .unwrap()
    });
    bridge
        .submit(HostCommand::SnippetEdit(save.clone()))
        .unwrap();
    drop(editor);
    state.layouts.write().clear();
    assert!(!save.is_active());
    assert!(bridge.submit(HostCommand::SnippetEdit(list)).is_err());
    release.send(()).unwrap();
    assert!(matches!(
        receive(&mut bridge),
        HostReply::CopiedText { result: Ok(()) }
    ));
    let HostReply::SnippetEdit(reply) = receive(&mut bridge) else {
        panic!("expected snippet save");
    };
    assert!(matches!(&reply.result, Ok(Outcome::Saved(file)) if file.file_name == "rust.json"));
    assert_eq!(
        std::fs::read_to_string(state.paths.snippets_dir().join("rust.json")).unwrap(),
        "{}"
    );
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, bridge.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, services.tasks.shutdown())
            .await
            .unwrap();
    });
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn snippet_전역_catalog는_settings없이_실제_host를_조회하고_변경_삭제와_종료_세대를_보존한다() {
    let directory =
        std::env::temp_dir().join(format!("taide-m8-snippet-catalog-{}", ProjectId::new()));
    std::fs::create_dir_all(&directory).unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    assert!(state.layouts.read().is_empty());
    let services = crate::bootstrap::services(
        state.clone(),
        TaskSupervisor::new(runtime.handle().clone()),
        Arc::new(Events),
    );
    let mut bridge = HostBridge::connect_with_clipboard_ports(
        services.clone(),
        Arc::new(|| {}),
        Arc::new(|_| {
            Err(AppError::Forbidden(
                "synthetic clipboard is disabled".into(),
            ))
        }),
        Arc::new(|| {
            Err(AppError::Forbidden(
                "synthetic clipboard is disabled".into(),
            ))
        }),
        None,
    )
    .unwrap();
    let mut views = taide_native_ui::settings_view::Views::default();
    let mut read = |views: &mut taide_native_ui::settings_view::Views| {
        let request = views.next_snippet_read().unwrap();
        assert!(views.next_snippet_read().is_none());
        bridge
            .submit(HostCommand::ReadSnippetCatalog(request))
            .unwrap();
        let reply = runtime.block_on(async {
            tokio::time::timeout(DEADLINE, async {
                loop {
                    if let Some(reply) = bridge.poll() {
                        return reply;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap()
        });
        let HostReply::SnippetCatalog(reply) = reply else {
            panic!("expected app-global snippet catalog");
        };
        assert!(views.accept_snippet_catalog(reply, std::time::Instant::now()));
        assert!(views.next_snippet_read().is_none());
    };
    read(&mut views);
    assert!(views.snippet_catalog().files().is_empty());
    taide_runtime::snippet_actions::snippet_save(
        &state,
        "rust.json".into(),
        r#"{"Synthetic":{"prefix":"s","body":"before $0"}}"#.into(),
    )
    .unwrap();
    views.refresh_snippets();
    read(&mut views);
    let old = views.snippet_catalog().snapshot().unwrap();
    views.begin_frame();
    views.finish_frame();
    assert!(Arc::ptr_eq(
        &old,
        &views.snippet_catalog().snapshot().unwrap()
    ));
    assert_eq!(
        taide_native_ui::snippet_completion::collect(views.snippet_catalog().files(), "rust")[0]
            .body,
        "before $0"
    );
    taide_runtime::snippet_actions::snippet_save(
        &state,
        "rust.json".into(),
        r#"{"Synthetic":{"prefix":"s","body":"after $0"}}"#.into(),
    )
    .unwrap();
    views.refresh_snippets();
    read(&mut views);
    assert_eq!(
        taide_native_ui::snippet_completion::collect(views.snippet_catalog().files(), "rust")[0]
            .body,
        "after $0"
    );
    assert!(!Arc::ptr_eq(
        &old,
        &views.snippet_catalog().snapshot().unwrap()
    ));
    taide_runtime::snippet_actions::snippet_delete(&state, "rust.json".into()).unwrap();
    views.refresh_snippets();
    read(&mut views);
    assert!(views.snippet_catalog().files().is_empty());
    views.refresh_snippets();
    let retired = views.next_snippet_read().unwrap();
    views.clear();
    assert!(!retired.is_active());
    let reply = runtime.block_on(retired.execute(&services));
    assert!(reply.result.is_err());
    assert!(!views.accept_snippet_catalog(reply, std::time::Instant::now()));
    runtime.block_on(async {
        tokio::time::timeout(DEADLINE, bridge.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, services.tasks.shutdown())
            .await
            .unwrap();
    });
    std::fs::remove_dir_all(&directory).unwrap();
}
