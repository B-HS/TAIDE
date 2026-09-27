use std::future::{poll_fn, Future};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::task::Poll;

use taide_model::app_event::AppEvent;
use taide_model::error::AppErrorKind;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{PaneNode, TabKind};
use taide_model::paths::AppPaths;
use taide_runtime::{layout_actions, AppState, EventSink};
use uuid::Uuid;

struct Fixture {
    dir: PathBuf,
    state: AppState,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-layout-service-actions-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("테스트 디렉터리 생성");
        let state = AppState::new(AppPaths::new(dir.clone()));
        let project_id = ProjectId::new();
        state
            .layouts
            .write()
            .insert(project_id.clone(), taide_layout::service::default_layout());
        Self { dir, state, project_id }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("테스트 디렉터리 정리");
    }
}

#[derive(Default)]
struct RecordingEventSink(Mutex<Vec<AppEvent>>);

impl EventSink for RecordingEventSink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[tokio::test]
async fn 공통_열기는_잠금과_preview_설정과_dirty_및_이벤트를_보존한다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    fixture.state.settings.write().enable_preview_tabs = false;
    let initial = fixture.state.layouts.read()[&fixture.project_id].clone();
    let guard = fixture.state.begin_mutation().await;
    let mut open = Box::pin(layout_actions::open_tab_and_finish(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        TabKind::File {
            path: "fixture.rs".to_string(),
        },
        "fixture".to_string(),
        None,
        true,
    ));
    poll_fn(|cx| {
        assert!(open.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], initial);
    assert!(sink.0.lock().unwrap().is_empty());
    drop(guard);
    let updated = open.await.expect("공통 열기");
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], updated);
    let PaneNode::Leaf { tabs, .. } = &updated.root else {
        panic!("단일 pane 유지")
    };
    let tab = tabs.iter().find(|tab| tab.title == "fixture").expect("추가된 파일 탭");
    assert!(!tab.preview);
    assert!(fixture.state.dirty_layouts.read().contains(&fixture.project_id));
    assert_eq!(
        *sink.0.lock().unwrap(),
        [AppEvent::LayoutChanged {
            project_id: fixture.project_id.clone(),
            revision: updated.revision
        }]
    );
}

#[tokio::test]
async fn 닫기_observer는_상태_commit_뒤_같은_mutation_잠금_안에서_호출된다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    let opened = layout_actions::open_tab_and_finish(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        TabKind::File {
            path: "fixture.rs".to_string(),
        },
        "fixture".to_string(),
        None,
        false,
    )
    .await
    .expect("공통 열기");
    let tab_id = taide_layout::service::find_tab_by_title(&opened.root, "fixture").expect("추가된 파일 탭");
    sink.0.lock().unwrap().clear();
    let called = AtomicBool::new(false);
    let (project_id, closed, updated) = layout_actions::close_tab_and_finish(&sink, &fixture.state, &tab_id, |tab| {
        assert_eq!(tab.id, tab_id);
        let current = fixture.state.layouts.read()[&fixture.project_id].clone();
        let PaneNode::Leaf { tabs, .. } = current.root else {
            panic!("단일 pane 유지")
        };
        assert!(!tabs.iter().any(|tab| tab.id == tab_id));
        assert_eq!(sink.0.lock().unwrap().len(), 1);
        let mut acquisition = Box::pin(fixture.state.begin_mutation());
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        assert!(acquisition.as_mut().poll(&mut context).is_pending());
        called.store(true, Ordering::SeqCst);
    })
    .await
    .expect("공통 닫기");
    assert_eq!(project_id, fixture.project_id);
    assert_eq!(closed.tab.id, tab_id);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], updated);
    assert!(called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn 없는_탭_닫기는_상태와_이벤트와_observer를_변경하지_않는다() {
    let fixture = Fixture::new();
    let sink = RecordingEventSink::default();
    let initial = fixture.state.layouts.read()[&fixture.project_id].clone();
    let called = AtomicBool::new(false);
    let error = layout_actions::close_tab_and_finish(&sink, &fixture.state, &TabId::new(), |_| called.store(true, Ordering::SeqCst))
        .await
        .expect_err("없는 탭 거절");
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], initial);
    assert!(sink.0.lock().unwrap().is_empty());
    assert!(!called.load(Ordering::SeqCst));
    assert!(fixture.state.dirty_layouts.read().is_empty());
}

#[test]
fn 저장_실패도_기존처럼_dirty_drain을_되돌리지_않는다() {
    let fixture = Fixture::new();
    let path = fixture.state.paths.layout_file(&fixture.project_id);
    std::fs::create_dir_all(&path).expect("저장 실패 fixture");
    fixture.state.dirty_layouts.write().insert(fixture.project_id.clone());
    layout_actions::flush_dirty_layouts(&fixture.state);
    assert!(fixture.state.dirty_layouts.read().is_empty());
    assert!(path.is_dir());
}

#[test]
fn 기존_service_경로는_runtime을_소비하고_실제_observer만_adapter에_남는다() {
    let source = include_str!("../src/domain/layout/service.rs");
    assert!(source.contains("pub use taide_runtime::layout_actions::finish_mutation"));
    assert!(source.contains("pub(crate) use taide_runtime::layout_actions::flush_dirty_layouts"));
    assert!(source.contains("layout_actions::open_tab_and_finish("));
    assert!(source.contains("layout_actions::close_tab_and_finish("));
    assert!(source.contains("app.state::<LayoutTabClosedObservers>().notify(app, tab)"));
    assert!(!source.contains("begin_mutation("));
}
