use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::PaneNode;
use taide_model::paths::AppPaths;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::controller::ShellController;
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const DEADLINE: Duration = Duration::from_secs(3);

#[tokio::test]
async fn 완료_layout은_비동기_snapshot보다_먼저_반영되고_이전_revision과_닫힌_프로젝트를_복원하지_않는다()
 {
    let state = AppState::new(AppPaths::new(std::env::temp_dir().join(format!(
        "taide-native-committed-layout-{}",
        ProjectId::new()
    ))));
    let project = ProjectId::new();
    state
        .session
        .write()
        .projects
        .push(taide_model::project::ProjectRef {
            id: project.clone(),
            root: "/synthetic".into(),
            name: "synthetic".into(),
            root_missing: false,
            display: Default::default(),
        });
    let original = taide_layout::service::default_layout();
    let PaneNode::Leaf { tabs, .. } = &original.root else {
        panic!("expected leaf");
    };
    let tab = tabs[0].id.clone();
    state
        .layouts
        .write()
        .insert(project.clone(), original.clone());
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let connection =
        ShellController::connect(state.clone(), &tasks, Arc::new(Sink), Arc::new(|| {}))
            .await
            .unwrap();
    let mut controller = connection.controller;
    let guard = state.begin_owned_mutation().await;
    let mut committed = original.clone();
    taide_layout::service::close_tab(&mut committed, &tab).unwrap();
    assert!(committed.revision > original.revision);
    state
        .layouts
        .write()
        .insert(project.clone(), committed.clone());
    connection.events.publish(AppEvent::LayoutChanged {
        project_id: project.clone(),
        revision: committed.revision,
    });
    controller.apply_layouts(std::collections::HashMap::from([(
        project.clone(),
        committed.clone(),
    )]));
    let immediate = controller.snapshot();
    assert_eq!(immediate.layouts[&project].revision, committed.revision);
    let PaneNode::Leaf { tabs, .. } = &immediate.layouts[&project].root else {
        panic!("expected leaf");
    };
    assert!(tabs.iter().all(|candidate| candidate.id != tab));
    controller.apply_layouts(std::collections::HashMap::from([(
        project.clone(),
        original,
    )]));
    assert_eq!(
        controller.snapshot().layouts[&project].revision,
        committed.revision
    );
    drop(guard);
    let reflected = tokio::time::timeout(DEADLINE, controller.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reflected.layouts[&project].revision, committed.revision);
    assert!(Arc::ptr_eq(&reflected, &controller.snapshot()));
    state.session.write().projects.clear();
    state.layouts.write().remove(&project);
    connection.events.publish(AppEvent::LayoutChanged {
        project_id: project.clone(),
        revision: committed.revision,
    });
    let closed = tokio::time::timeout(DEADLINE, controller.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(closed.project(&project).is_none());
    assert!(!closed.layouts.contains_key(&project));
    drop(controller);
    tokio::time::timeout(DEADLINE, connection.worker)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!state.paths.data_dir.exists());
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

#[tokio::test]
async fn 실제_controller는_runtime_명령과_외부_이벤트를_갱신하고_소유_worker를_회수한다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-controller-{}", ProjectId::new())),
    ));
    let project = ProjectId::new();
    let layout = taide_layout::service::default_layout();
    let PaneNode::Leaf { tabs, .. } = &layout.root else {
        panic!("expected leaf")
    };
    let tab = tabs[0].id.clone();
    state.layouts.write().insert(project.clone(), layout);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let repaint_count = Arc::new(AtomicUsize::new(0));
    let repaint = repaint_count.clone();
    let connection = ShellController::connect(
        state.clone(),
        &tasks,
        Arc::new(Sink),
        Arc::new(move || {
            repaint.fetch_add(1, Ordering::Relaxed);
        }),
    )
    .await
    .unwrap();
    let mut controller = connection.controller;
    controller
        .submit(ShellMutation::PinTab {
            tab: tab.clone(),
            pinned: true,
        })
        .unwrap();
    tokio::time::timeout(DEADLINE, async {
        loop {
            let snapshot = controller.changed().await.unwrap();
            let layout = &snapshot.layouts[&project];
            if taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane)
                .unwrap()
                .pinned
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    taide_runtime::layout_actions::layout_set_preview(
        connection.events.as_ref(),
        &state,
        tab.clone(),
        true,
    )
    .await
    .unwrap();
    tokio::time::timeout(DEADLINE, async {
        loop {
            let snapshot = controller.changed().await.unwrap();
            let layout = &snapshot.layouts[&project];
            if taide_native_ui::snapshot::active_tab(&layout.root, &layout.focused_pane)
                .unwrap()
                .preview
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    controller
        .submit(ShellMutation::ActivateTab(TabId::new()))
        .unwrap();
    controller.submit(ShellMutation::KeepTab(tab)).unwrap();
    tokio::time::timeout(DEADLINE, async {
        while controller.take_error().is_none() {
            controller.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(controller.take_error().is_none());
    assert!(repaint_count.load(Ordering::Relaxed) > 0);
    drop(controller);
    tokio::time::timeout(DEADLINE, connection.worker)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!state.paths.data_dir.exists());
}
