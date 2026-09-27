use std::future::{poll_fn, Future};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::{AuxWindowLayout, PaneNode, ProjectLayout, Tab, TabKind, TabWindowTarget};
use taide_model::paths::AppPaths;
use taide_runtime::{layout_actions, AppState, EventSink, WindowRegistry};
use tokio::sync::oneshot;
use uuid::Uuid;

const UNREGISTERED_SLOT: u32 = 3;

struct Fixture {
    dir: PathBuf,
    state: AppState,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-layout-window-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let state = AppState::new(AppPaths::new(dir.join("data")));
        let project_id = ProjectId::new();
        state
            .layouts
            .write()
            .insert(project_id.clone(), taide_layout::service::default_layout());
        Self { dir, state, project_id }
    }

    fn add_file(&self, name: &str) -> TabId {
        let mut layouts = self.state.layouts.write();
        let layout = layouts.get_mut(&self.project_id).unwrap();
        let pane_id = layout.focused_pane.clone();
        let tab = Tab {
            id: TabId::new(),
            kind: TabKind::File {
                path: self.dir.join(name).to_string_lossy().into_owned(),
            },
            title: name.to_string(),
            pinned: true,
            preview: false,
            dirty: true,
            view_state: Some("fixture-view-state".to_string()),
        };
        taide_layout::service::open_tab(layout, &pane_id, tab, false).unwrap()
    }

    fn move_to_auxiliary(&self, tab_id: &TabId) -> u32 {
        let mut layouts = self.state.layouts.write();
        let layout = layouts.get_mut(&self.project_id).unwrap();
        let slot = taide_layout::service::next_window_slot(layout);
        taide_layout::service::move_tab_to_new_window(layout, tab_id, slot).unwrap();
        slot
    }

    fn sink(&self) -> SnapshotEventSink {
        SnapshotEventSink {
            state: self.state.clone(),
            project_id: self.project_id.clone(),
            records: Mutex::new(Vec::new()),
            trace: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

struct SnapshotEventSink {
    state: AppState,
    project_id: ProjectId,
    records: Mutex<Vec<(AppEvent, ProjectLayout, bool)>>,
    trace: Arc<Mutex<Vec<&'static str>>>,
}

impl EventSink for SnapshotEventSink {
    fn publish(&self, event: AppEvent) {
        assert_mutation_locked(&self.state);
        let snapshot = self.state.layouts.read()[&self.project_id].clone();
        let is_dirty = self.state.dirty_layouts.read().contains(&self.project_id);
        self.records.lock().unwrap().push((event, snapshot, is_dirty));
        self.trace.lock().unwrap().push("event");
    }
}

fn assert_mutation_locked(state: &AppState) {
    let mut acquisition = Box::pin(state.begin_mutation());
    let mut context = Context::from_waker(Waker::noop());
    assert!(acquisition.as_mut().poll(&mut context).is_pending());
}

#[tokio::test]
async fn 새_창은_이동_전에_같은_guard에서_열고_이벤트_뒤_상태를_기록한다() {
    let fixture = Fixture::new();
    let tab_id = fixture.add_file("new.rs");
    let initial = fixture.state.layouts.read()[&fixture.project_id].clone();
    let sink = fixture.sink();
    let windows = WindowRegistry::default();
    let registered = windows.clone();
    let state = fixture.state.clone();
    let trace = sink.trace.clone();
    let (release, wait) = oneshot::channel();
    let guard = fixture.state.begin_mutation().await;
    let mut action = Box::pin(layout_actions::layout_move_tab_to_window(
        &sink,
        &fixture.state,
        &windows,
        tab_id.clone(),
        TabWindowTarget::NewAuxiliary,
        move |project_id, slot| async move {
            assert_mutation_locked(&state);
            assert!(state.layouts.read()[&project_id].auxiliary_windows.is_empty());
            trace.lock().unwrap().push("open");
            wait.await.unwrap();
            registered.register("fixture-new".to_string(), project_id, slot);
            Ok("fixture-new".to_string())
        },
        |_| panic!("새 창의 성공 이동은 닫지 않습니다"),
    ));
    poll_fn(|cx| {
        assert!(action.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(sink.trace.lock().unwrap().is_empty());
    drop(guard);
    poll_fn(|cx| {
        assert!(action.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert_eq!(*sink.trace.lock().unwrap(), ["open"]);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], initial);
    release.send(()).unwrap();
    let updated = action.await.unwrap();
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], updated);
    assert_eq!(*sink.trace.lock().unwrap(), ["open", "event"]);
    let records = sink.records.lock().unwrap();
    assert_eq!(records[0].1, initial);
    assert!(records[0].2);
    assert_eq!(
        records[0].0,
        AppEvent::LayoutChanged {
            project_id: fixture.project_id.clone(),
            revision: updated.revision
        }
    );
    let PaneNode::Leaf { tabs, .. } = &updated.auxiliary_windows[0].root else {
        panic!("보조 leaf")
    };
    assert_eq!(tabs[0].id, tab_id);
    assert!(tabs[0].dirty && tabs[0].pinned);
    assert_eq!(tabs[0].view_state.as_deref(), Some("fixture-view-state"));
}

#[tokio::test]
async fn 없는_탭과_슬롯과_창_생성_실패는_상태와_이벤트를_변경하지_않는다() {
    let fixture = Fixture::new();
    let tab_id = fixture.add_file("failed.rs");
    let initial = fixture.state.layouts.read()[&fixture.project_id].clone();
    let sink = fixture.sink();
    let windows = WindowRegistry::default();
    let missing = layout_actions::layout_move_tab_to_window(
        &sink,
        &fixture.state,
        &windows,
        TabId::new(),
        TabWindowTarget::NewAuxiliary,
        |_, _| async { panic!("없는 탭에 창을 열지 않습니다") },
        |_| panic!("없는 탭은 창을 닫지 않습니다"),
    )
    .await
    .unwrap_err();
    assert_eq!(missing.kind(), AppErrorKind::NotFound);
    let slot = layout_actions::layout_move_tab_to_window(
        &sink,
        &fixture.state,
        &windows,
        tab_id.clone(),
        TabWindowTarget::Existing { slot: UNREGISTERED_SLOT },
        |_, _| async { panic!("기존 슬롯에는 창을 만들지 않습니다") },
        |_| panic!("실패한 이동은 창을 닫지 않습니다"),
    )
    .await
    .unwrap_err();
    assert_eq!(slot.kind(), AppErrorKind::NotFound);
    let failure = layout_actions::layout_move_tab_to_window(
        &sink,
        &fixture.state,
        &windows,
        tab_id,
        TabWindowTarget::NewAuxiliary,
        |_, _| async { Err(AppError::Internal("synthetic open failure".to_string())) },
        |_| panic!("생성 실패한 창은 닫지 않습니다"),
    )
    .await
    .unwrap_err();
    assert_eq!(failure.kind(), AppErrorKind::Internal);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], initial);
    assert!(sink.records.lock().unwrap().is_empty());
    assert!(fixture.state.dirty_layouts.read().is_empty());
}

#[tokio::test]
async fn 기존과_main_이동은_빈_layout부터_제거하고_등록된_창만_이벤트_전에_닫는다() {
    let fixture = Fixture::new();
    let first = fixture.add_file("first.rs");
    let first_slot = fixture.move_to_auxiliary(&first);
    let second = fixture.add_file("second.rs");
    let second_slot = fixture.move_to_auxiliary(&second);
    let empty_pane = PaneId::new();
    fixture
        .state
        .layouts
        .write()
        .get_mut(&fixture.project_id)
        .unwrap()
        .auxiliary_windows
        .push(AuxWindowLayout {
            slot: UNREGISTERED_SLOT,
            root: PaneNode::Leaf {
                id: empty_pane.clone(),
                tabs: Vec::new(),
                active: None,
            },
            focused_pane: empty_pane,
        });
    let sink = fixture.sink();
    let windows = WindowRegistry::default();
    windows.register("fixture-first".to_string(), fixture.project_id.clone(), first_slot);
    windows.register("fixture-second".to_string(), fixture.project_id.clone(), second_slot);
    let updated = layout_actions::layout_move_tab_to_window(
        &sink,
        &fixture.state,
        &windows,
        first.clone(),
        TabWindowTarget::Existing { slot: second_slot },
        |_, _| async { panic!("기존 창에는 새 창을 만들지 않습니다") },
        |label| {
            assert_mutation_locked(&fixture.state);
            assert_eq!(label, "fixture-first");
            assert!(fixture.state.layouts.read()[&fixture.project_id]
                .auxiliary_windows
                .iter()
                .any(|window| window.slot == first_slot));
            sink.trace.lock().unwrap().push("close");
        },
    )
    .await
    .unwrap();
    assert_eq!(updated.auxiliary_windows.len(), 1);
    assert_eq!(updated.auxiliary_windows[0].slot, second_slot);
    assert_eq!(*sink.trace.lock().unwrap(), ["close", "event"]);
    assert_eq!(windows.label_for(&fixture.project_id, first_slot).as_deref(), Some("fixture-first"));
    layout_actions::layout_move_tab_to_window(
        &sink,
        &fixture.state,
        &windows,
        first,
        TabWindowTarget::Main,
        |_, _| async { panic!("main 이동에 창을 만들지 않습니다") },
        |_| panic!("아직 탭이 남은 창을 닫지 않습니다"),
    )
    .await
    .unwrap();
    let final_layout = layout_actions::layout_move_tab_to_window(
        &sink,
        &fixture.state,
        &windows,
        second,
        TabWindowTarget::Main,
        |_, _| async { panic!("main 이동에 창을 만들지 않습니다") },
        |label| assert_eq!(label, "fixture-second"),
    )
    .await
    .unwrap();
    assert!(final_layout.auxiliary_windows.is_empty());
}

#[tokio::test]
async fn 복귀는_mirror가_없는_file만_dirty를_정리하고_상태_기록_뒤_이벤트를_발행한다() {
    let fixture = Fixture::new();
    let shared = fixture.add_file("shared.rs");
    let mirrored = fixture.add_file("mirrored.rs");
    let slot = fixture.move_to_auxiliary(&mirrored);
    let (path, untitled_id) = {
        let mut layouts = fixture.state.layouts.write();
        let layout = layouts.get_mut(&fixture.project_id).unwrap();
        let PaneNode::Leaf { tabs, .. } = &mut layout.root else {
            panic!("main leaf")
        };
        let shared_tab = tabs.iter_mut().find(|tab| tab.id == shared).unwrap();
        shared_tab.dirty = false;
        let mut phantom = shared_tab.clone();
        phantom.id = TabId::new();
        phantom.dirty = true;
        let pane = layout.auxiliary_windows[0].focused_pane.clone();
        taide_layout::service::open_tab(layout, &pane, phantom, false).unwrap();
        let untitled_id = TabId::new();
        taide_layout::service::open_tab(
            layout,
            &pane,
            Tab {
                id: untitled_id.clone(),
                kind: TabKind::Untitled { index: 1 },
                title: "fixture untitled".to_string(),
                dirty: true,
                pinned: false,
                preview: false,
                view_state: None,
            },
            false,
        )
        .unwrap();
        let PaneNode::Leaf { tabs, .. } = &layout.auxiliary_windows[0].root else {
            panic!("auxiliary leaf")
        };
        let TabKind::File { path } = &tabs.iter().find(|tab| tab.id == mirrored).unwrap().kind else {
            panic!("file")
        };
        (path.clone(), untitled_id)
    };
    taide_file::service::mirror_dirty(
        &fixture.state.paths,
        &fixture.project_id,
        Path::new(&path),
        &path,
        "synthetic draft",
    )
    .unwrap();
    let before = fixture.state.layouts.read()[&fixture.project_id].clone();
    let sink = fixture.sink();
    let guard = fixture.state.begin_mutation().await;
    let mut returning = Box::pin(layout_actions::return_auxiliary_window_tabs(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        slot,
    ));
    poll_fn(|cx| {
        assert!(returning.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], before);
    assert!(sink.records.lock().unwrap().is_empty());
    drop(guard);
    returning.await;
    {
        let after = fixture.state.layouts.read()[&fixture.project_id].clone();
        assert!(after.auxiliary_windows.is_empty());
        let PaneNode::Leaf { tabs, active, .. } = &after.root else {
            panic!("main leaf")
        };
        assert!(!tabs.iter().find(|tab| tab.id == shared).unwrap().dirty);
        assert!(tabs.iter().find(|tab| tab.id == mirrored).unwrap().dirty);
        assert!(tabs.iter().find(|tab| tab.id == untitled_id).unwrap().dirty);
        let PaneNode::Leaf { active: before_active, .. } = before.root else {
            panic!("main leaf")
        };
        assert_eq!(active, &before_active);
        let records = sink.records.lock().unwrap();
        assert_eq!(records[0].1, after);
        assert!(records[0].2);
        assert_eq!(
            records[0].0,
            AppEvent::LayoutChanged {
                project_id: fixture.project_id.clone(),
                revision: after.revision
            }
        );
    }
    assert_eq!(
        taide_file::service::list_mirrors(&fixture.state.paths, &fixture.project_id)
            .unwrap()
            .len(),
        1
    );
    sink.records.lock().unwrap().clear();
    fixture.state.dirty_layouts.write().clear();
    layout_actions::return_auxiliary_window_tabs(&sink, &fixture.state, fixture.project_id.clone(), slot).await;
    assert!(sink.records.lock().unwrap().is_empty());
    assert!(fixture.state.dirty_layouts.read().is_empty());
}

#[tokio::test]
async fn mirror_조회_실패는_기존처럼_빈_snapshot으로_복귀한다() {
    let fixture = Fixture::new();
    let tab_id = fixture.add_file("phantom.rs");
    let slot = fixture.move_to_auxiliary(&tab_id);
    let buffer = fixture.state.paths.buffers_dir(&fixture.project_id);
    std::fs::create_dir_all(buffer.parent().unwrap()).unwrap();
    std::fs::write(&buffer, "not a directory").unwrap();
    assert!(taide_file::service::list_mirrors(&fixture.state.paths, &fixture.project_id).is_err());
    let sink = fixture.sink();
    layout_actions::return_auxiliary_window_tabs(&sink, &fixture.state, fixture.project_id.clone(), slot).await;
    let layout = fixture.state.layouts.read()[&fixture.project_id].clone();
    assert!(layout.auxiliary_windows.is_empty());
    let PaneNode::Leaf { tabs, .. } = &layout.root else {
        panic!("main leaf")
    };
    assert!(!tabs.iter().find(|tab| tab.id == tab_id).unwrap().dirty);
    assert_eq!(sink.records.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn 이미_닫힌_프로젝트와_없는_슬롯의_복귀는_이벤트와_dirty를_추가하지_않는다() {
    let fixture = Fixture::new();
    let sink = fixture.sink();
    let before = fixture.state.layouts.read()[&fixture.project_id].clone();
    layout_actions::return_auxiliary_window_tabs(&sink, &fixture.state, fixture.project_id.clone(), UNREGISTERED_SLOT).await;
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], before);
    fixture.state.layouts.write().remove(&fixture.project_id);
    layout_actions::return_auxiliary_window_tabs(&sink, &fixture.state, fixture.project_id.clone(), UNREGISTERED_SLOT).await;
    assert!(sink.records.lock().unwrap().is_empty());
    assert!(fixture.state.dirty_layouts.read().is_empty());
}

#[test]
fn 순수_service의_방어적_이동_실패는_새_layout_슬롯을_롤백한다() {
    let mut layout = taide_layout::service::default_layout();
    let before = layout.clone();
    let slot = taide_layout::service::next_window_slot(&layout);
    assert!(taide_layout::service::move_tab_to_new_window(&mut layout, &TabId::new(), slot).is_err());
    assert_eq!(layout, before);
}

#[test]
fn 창_생성_close_adapter와_runtime_정책은_원래_순서로_분리된다() {
    let app = include_str!("../../../src-tauri/src/lib.rs");
    let command = app.split_once("async fn layout_move_tab_to_window(").unwrap().1;
    let command = command.split_once("fn plan_return_of_auxiliary_window_tabs(").unwrap().0;
    assert!(command.contains("layout_actions::layout_move_tab_to_window("));
    assert!(command.contains("open_auxiliary_window(app, state, windows, project_id, slot).await?"));
    assert!(command.contains("Ok(info.label)"));
    assert!(command.contains("app.get_webview_window(label)"));
    assert!(command.contains("let _ = webview_window.close()"));
    assert!(!command.contains("state.begin_mutation()"));
    let returning = app.split_once("fn plan_return_of_auxiliary_window_tabs(").unwrap().1;
    let returning = returning.split_once("/// Routes one app-menu").unwrap().0;
    assert!(returning.contains("spawn_transient(\"auxiliary-tab-return\""));
    assert!(returning.contains("layout_actions::return_auxiliary_window_tabs("));
    assert!(!returning.contains("begin_mutation()"));
    assert!(!returning.contains("list_mirrors("));
    let actions = include_str!("../src/layout_actions.rs");
    let movement = actions.split_once("pub async fn layout_move_tab_to_window<").unwrap().1;
    let movement = movement.split_once("pub async fn return_auxiliary_window_tabs(").unwrap().0;
    assert!(
        movement.find("open_auxiliary_window(project_id.clone(), slot).await?").unwrap()
            < movement.find("service::move_tab_to_new_window(").unwrap()
    );
    assert!(movement.find("service::move_tab_to_new_window(").unwrap() < movement.find("close_window(&label)").unwrap());
    assert!(movement.find("close_window(&label)").unwrap() < movement.find("cleanup_emptied_auxiliary_windows(").unwrap());
    assert!(movement.find("finish_mutation(").unwrap() < movement.find("*state.layouts.write() = layouts").unwrap());
}
