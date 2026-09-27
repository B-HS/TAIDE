use std::future::{poll_fn, Future};
use std::path::PathBuf;
use std::sync::Mutex;
use std::task::Poll;

use taide_model::app_event::AppEvent;
use taide_model::error::AppErrorKind;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{DropEdge, OpenTabInSplitRequest, PaneNode, ProjectLayout, TabKind, TabPathChange};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_runtime::{layout_actions, AppState, EventSink};
use uuid::Uuid;

struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    state: AppState,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-layout-command-actions-{}", Uuid::new_v4()));
        let root = dir.join("project");
        std::fs::create_dir_all(&root).expect("프로젝트 fixture 생성");
        let root = std::fs::canonicalize(root).expect("fixture 정규 경로");
        let state = AppState::new(AppPaths::new(dir.join("data")));
        let project_id = ProjectId::new();
        state.projects.write().insert(
            project_id.clone(),
            Project {
                id: project_id.clone(),
                root: root.to_string_lossy().into_owned(),
                name: "fixture".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        state
            .layouts
            .write()
            .insert(project_id.clone(), taide_layout::service::default_layout());
        Self {
            dir,
            root,
            state,
            project_id,
        }
    }

    fn sink(&self) -> SnapshotEventSink {
        SnapshotEventSink {
            state: self.state.clone(),
            project_id: self.project_id.clone(),
            records: Mutex::new(Vec::new()),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("fixture 정리");
    }
}

struct SnapshotEventSink {
    state: AppState,
    project_id: ProjectId,
    records: Mutex<Vec<(AppEvent, ProjectLayout)>>,
}

impl EventSink for SnapshotEventSink {
    fn publish(&self, event: AppEvent) {
        let snapshot = self.state.layouts.read()[&self.project_id].clone();
        self.records.lock().unwrap().push((event, snapshot));
    }
}

#[tokio::test]
async fn 공통_command는_잠금과_이벤트_후_상태_commit_순서를_보존한다() {
    let fixture = Fixture::new();
    let sink = fixture.sink();
    let initial = layout_actions::layout_get(&fixture.state, fixture.project_id.clone())
        .await
        .unwrap();
    let tab_id = taide_layout::service::find_tab_by_title(&initial.root, "Welcome").unwrap();
    let guard = fixture.state.begin_mutation().await;
    let mut mutation = Box::pin(layout_actions::layout_set_dirty(&sink, &fixture.state, tab_id.clone(), true));
    poll_fn(|cx| {
        assert!(mutation.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], initial);
    assert!(sink.records.lock().unwrap().is_empty());
    drop(guard);
    let updated = mutation.await.unwrap();
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], updated);
    assert_eq!(updated.revision, initial.revision + 1);
    let PaneNode::Leaf { tabs, .. } = &updated.root else {
        panic!("단일 pane 유지")
    };
    assert!(tabs.iter().find(|tab| tab.id == tab_id).unwrap().dirty);
    {
        let records = sink.records.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].1, initial);
        assert_eq!(
            records[0].0,
            AppEvent::LayoutChanged {
                project_id: fixture.project_id.clone(),
                revision: updated.revision,
            }
        );
    }
    sink.records.lock().unwrap().clear();
    fixture.state.dirty_layouts.write().clear();
    let error = layout_actions::layout_activate_tab(&sink, &fixture.state, TabId::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], updated);
    assert!(sink.records.lock().unwrap().is_empty());
    assert!(fixture.state.dirty_layouts.read().is_empty());
}

#[tokio::test]
async fn 열기와_split은_파일_gate를_공유하고_untitled_변환은_cli_예외를_허용하지_않는다() {
    let fixture = Fixture::new();
    let sink = fixture.sink();
    let initial = fixture.state.layouts.read()[&fixture.project_id].clone();
    let missing = fixture.root.join("missing.rs").to_string_lossy().into_owned();
    let error = layout_actions::layout_open_tab(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        TabKind::File { path: missing.clone() },
        "missing".to_string(),
        None,
        false,
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    let error = layout_actions::layout_open_tab_in_split(
        &sink,
        &fixture.state,
        OpenTabInSplitRequest {
            project_id: fixture.project_id.clone(),
            target_pane: initial.focused_pane.clone(),
            edge: DropEdge::Right,
            kind: TabKind::File { path: missing },
            title: "missing split".to_string(),
            preview: true,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::NotFound);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], initial);
    assert!(sink.records.lock().unwrap().is_empty());
    let outside = fixture.dir.join("cli.rs");
    std::fs::write(&outside, "fixture").unwrap();
    let outside = std::fs::canonicalize(outside).unwrap();
    let path = outside.to_string_lossy().into_owned();
    let error = layout_actions::layout_open_tab_in_split(
        &sink,
        &fixture.state,
        OpenTabInSplitRequest {
            project_id: fixture.project_id.clone(),
            target_pane: initial.focused_pane.clone(),
            edge: DropEdge::Right,
            kind: TabKind::File { path: path.clone() },
            title: "outside".to_string(),
            preview: true,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    fixture.state.cli_opened_paths.write().insert(outside);
    fixture.state.settings.write().enable_preview_tabs = false;
    let opened = layout_actions::layout_open_tab(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        TabKind::File { path: path.clone() },
        "cli".to_string(),
        None,
        true,
    )
    .await
    .unwrap();
    let PaneNode::Leaf { tabs, .. } = &opened.root else {
        panic!("단일 pane 유지")
    };
    assert!(!tabs.iter().find(|tab| tab.title == "cli").unwrap().preview);
    let split = layout_actions::layout_open_tab_in_split(
        &sink,
        &fixture.state,
        OpenTabInSplitRequest {
            project_id: fixture.project_id.clone(),
            target_pane: opened.focused_pane.clone(),
            edge: DropEdge::Right,
            kind: TabKind::File { path: path.clone() },
            title: "cli split".to_string(),
            preview: true,
        },
    )
    .await
    .unwrap();
    let PaneNode::Split { children, .. } = &split.root else {
        panic!("split 생성")
    };
    assert!(children
        .iter()
        .any(|child| matches!(child, PaneNode::Leaf { tabs, .. } if tabs.iter().any(|tab| tab.title == "cli split" && !tab.preview))));
    let untitled = layout_actions::layout_open_untitled(&sink, &fixture.state, fixture.project_id.clone(), None)
        .await
        .unwrap();
    let tab_id = taide_layout::service::find_tab_by_title(&untitled.root, "Untitled-1").unwrap();
    sink.records.lock().unwrap().clear();
    fixture.state.dirty_layouts.write().clear();
    let error = layout_actions::layout_convert_untitled(&sink, &fixture.state, tab_id, path)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], untitled);
    assert!(sink.records.lock().unwrap().is_empty());
    assert!(fixture.state.dirty_layouts.read().is_empty());
}

#[tokio::test]
async fn 닫힌_탭만의_개명은_revision과_이벤트_없이_상태와_dirty를_기록한다() {
    let fixture = Fixture::new();
    let sink = fixture.sink();
    let from = fixture.root.join("old.rs");
    let to = fixture.root.join("new.rs");
    std::fs::write(&from, "fixture").unwrap();
    let opened = layout_actions::layout_open_tab(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        TabKind::File {
            path: from.to_string_lossy().into_owned(),
        },
        "old.rs".to_string(),
        None,
        false,
    )
    .await
    .unwrap();
    let tab_id = taide_layout::service::find_tab_by_title(&opened.root, "old.rs").unwrap();
    let (_, _, closed) = layout_actions::close_tab_and_finish(&sink, &fixture.state, &tab_id, |_| {})
        .await
        .unwrap();
    std::fs::rename(&from, &to).unwrap();
    sink.records.lock().unwrap().clear();
    fixture.state.dirty_layouts.write().clear();
    let result = layout_actions::layout_apply_path_change(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        TabPathChange::Renamed {
            from: from.to_string_lossy().into_owned(),
            to: to.to_string_lossy().into_owned(),
        },
    )
    .await
    .unwrap();
    assert!(result.moved.is_empty());
    assert!(result.closed_paths.is_empty());
    assert_eq!(result.layout.revision, closed.revision);
    assert_eq!(
        result.layout.closed_tabs[0].tab.kind,
        TabKind::File {
            path: to.to_string_lossy().into_owned()
        }
    );
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], result.layout);
    assert!(fixture.state.dirty_layouts.read().contains(&fixture.project_id));
    assert!(sink.records.lock().unwrap().is_empty());
    let error = layout_actions::layout_apply_path_change(
        &sink,
        &fixture.state,
        fixture.project_id.clone(),
        TabPathChange::Deleted { path: "/".to_string() },
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    assert_eq!(fixture.state.layouts.read()[&fixture.project_id], result.layout);
    assert!(sink.records.lock().unwrap().is_empty());
}

#[test]
fn command는_runtime_action이나_기존_닫기_observer_adapter에_위임한다() {
    let source = include_str!("../src/domain/layout/commands.rs");
    for declaration in source.split("pub async fn ").skip(1) {
        let name = declaration.split('(').next().unwrap();
        if name == "layout_close_tab" {
            continue;
        }
        assert!(declaration.contains(&format!("layout_actions::{name}(")), "{name}");
    }
    assert!(source.contains("service::close_tab_and_finish(&app, &state, &tab_id)"));
    assert!(!source.contains("begin_mutation("));
    assert!(!source.contains("fn ensure_file_tab_target_exists("));
    assert!(!source.contains("fn run_layout_mutation"));
}
