use std::collections::HashMap;
use std::path::PathBuf;

use taide_lib::domain::layout::service as layout_service;
use taide_lib::domain::layout::types::{DropEdge, PaneNode, TabKind};
use taide_lib::domain::project::service as project_service;
use taide_lib::domain::project::types::SessionState;
use taide_lib::paths::AppPaths;

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("taide-it-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn leaf_tab_kinds(node: &PaneNode) -> Vec<TabKind> {
    match node {
        PaneNode::Leaf { tabs, .. } => tabs.iter().map(|tab| tab.kind.clone()).collect(),
        PaneNode::Split { children, .. } => children.iter().flat_map(leaf_tab_kinds).collect(),
    }
}

fn split_count(node: &PaneNode) -> usize {
    match node {
        PaneNode::Leaf { .. } => 0,
        PaneNode::Split { children, .. } => 1 + children.iter().map(split_count).sum::<usize>(),
    }
}

#[test]
fn 재시작하면_프로젝트와_탭_스플릿_활성상태가_복원된다() {
    let data = TempDir::new("data");
    let workspace = TempDir::new("ws");
    std::fs::create_dir_all(workspace.path().join(".git")).expect("create .git");

    let paths = AppPaths::new(data.path().clone());

    let mut session = SessionState::default();
    let mut projects = HashMap::new();
    let opened = project_service::open_project(&paths, &mut session, &mut projects, workspace.path()).expect("open project");
    let project_id = opened.project.id.clone();

    let mut layout = layout_service::default_layout();
    let target_pane = layout.focused_pane.clone();
    let file_tab = layout_service::open_tab(
        &mut layout,
        &target_pane,
        taide_lib::domain::layout::types::Tab {
            id: taide_lib::ids::TabId::new(),
            kind: TabKind::File {
                path: "src/main.rs".to_string(),
            },
            title: "main.rs".to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        },
        false,
    )
    .expect("open file tab");

    let focused = layout.focused_pane.clone();
    layout_service::split(&mut layout, &focused, DropEdge::Right, &file_tab).expect("split");
    layout_service::activate_tab(&mut layout, &file_tab).expect("activate");

    let expected_kinds = leaf_tab_kinds(&layout.root);
    let expected_splits = split_count(&layout.root);
    let expected_focus = layout.focused_pane.clone();

    layout_service::save_layout(&paths, &project_id, &layout).expect("save layout");

    let reopened_paths = AppPaths::new(data.path().clone());
    let (restored_session, restored_projects, warnings) = project_service::restore_session(&reopened_paths).expect("restore");
    let restored_layout = layout_service::load_layout(&reopened_paths, &project_id);

    assert!(warnings.is_empty(), "복원 경고가 없어야 한다: {warnings:?}");
    assert_eq!(restored_session.projects.len(), 1);
    assert_eq!(restored_session.projects[0].id, project_id);
    assert_eq!(restored_session.active_project, Some(project_id.clone()));
    assert_eq!(restored_projects.len(), 1);
    assert!(!restored_projects[0].root_missing);

    assert_eq!(split_count(&restored_layout.root), expected_splits, "스플릿 구조가 복원되어야 한다");
    assert_eq!(
        leaf_tab_kinds(&restored_layout.root),
        expected_kinds,
        "탭 구성과 순서가 복원되어야 한다"
    );
    assert_eq!(restored_layout.focused_pane, expected_focus, "포커스된 pane 이 복원되어야 한다");
}

#[test]
fn 루트가_사라진_프로젝트는_제거되지_않고_root_missing_으로_표시된다() {
    let data = TempDir::new("data-missing");
    let paths = AppPaths::new(data.path().clone());

    let workspace = TempDir::new("ws-missing");
    let workspace_path = workspace.path().clone();

    let mut session = SessionState::default();
    let mut projects = HashMap::new();
    project_service::open_project(&paths, &mut session, &mut projects, &workspace_path).expect("open project");

    drop(workspace);

    let (restored_session, restored_projects, _) = project_service::restore_session(&paths).expect("restore");

    assert_eq!(restored_session.projects.len(), 1, "루트가 없어도 목록에서 지우지 않는다");
    assert!(
        restored_projects[0].root_missing,
        "root_missing 으로 표시해 사용자가 재연결/제거를 고르게 한다"
    );
}

#[test]
fn 활성_프로젝트를_닫으면_남은_프로젝트가_활성화된다() {
    let data = TempDir::new("data-close");
    let paths = AppPaths::new(data.path().clone());
    let first = TempDir::new("ws-first");
    let second = TempDir::new("ws-second");

    let mut session = SessionState::default();
    let mut projects = HashMap::new();

    let first_opened = project_service::open_project(&paths, &mut session, &mut projects, first.path()).expect("open first");
    let second_opened = project_service::open_project(&paths, &mut session, &mut projects, second.path()).expect("open second");

    assert_eq!(
        session.active_project,
        Some(second_opened.project.id.clone()),
        "새로 연 프로젝트가 활성화되어야 한다"
    );

    project_service::close_project(&paths, &mut session, &mut projects, &second_opened.project.id).expect("close active");

    assert_eq!(
        session.active_project,
        Some(first_opened.project.id),
        "활성 프로젝트를 닫으면 남은 프로젝트로 넘어가야 한다"
    );

    let remaining = session.active_project.clone().expect("active");
    project_service::close_project(&paths, &mut session, &mut projects, &remaining).expect("close last");

    assert_eq!(session.active_project, None, "마지막 프로젝트를 닫으면 빈 상태가 된다");
}

#[test]
fn 레이아웃_파일이_없으면_기본_레이아웃으로_시작한다() {
    let data = TempDir::new("data-nolayout");
    let paths = AppPaths::new(data.path().clone());
    let layout = layout_service::load_layout(&paths, &taide_lib::ids::ProjectId::new());

    let kinds = leaf_tab_kinds(&layout.root);
    assert!(kinds.contains(&TabKind::Welcome));
    assert!(kinds.iter().any(|kind| matches!(kind, TabKind::Terminal { .. })));
}

#[test]
fn 재시작_후_untitled_탭이_사라진다() {
    let data = TempDir::new("data-untitled");
    let workspace = TempDir::new("ws-untitled");
    std::fs::create_dir_all(workspace.path().join(".git")).expect("create .git");

    let paths = AppPaths::new(data.path().clone());

    let mut session = SessionState::default();
    let mut projects = HashMap::new();
    let opened = project_service::open_project(&paths, &mut session, &mut projects, workspace.path()).expect("open project");
    let project_id = opened.project.id.clone();

    let mut layout = layout_service::default_layout();
    let target_pane = layout.focused_pane.clone();
    layout_service::open_tab(
        &mut layout,
        &target_pane,
        taide_lib::domain::layout::types::Tab {
            id: taide_lib::ids::TabId::new(),
            kind: TabKind::Untitled { index: 1 },
            title: "Untitled-1".to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        },
        false,
    )
    .expect("open untitled tab");

    layout_service::save_layout(&paths, &project_id, &layout).expect("save layout");

    let reopened_paths = AppPaths::new(data.path().clone());
    let restored_layout = layout_service::load_layout(&reopened_paths, &project_id);

    let restored_kinds = leaf_tab_kinds(&restored_layout.root);
    assert!(
        !restored_kinds.iter().any(|kind| matches!(kind, TabKind::Untitled { .. })),
        "untitled 탭은 재시작 후 남지 않아야 한다"
    );
}

#[test]
fn 스플릿_사이즈는_퍼센트_단위로_합이_100이_된다() {
    let mut layout = layout_service::default_layout();
    let target_pane = layout.focused_pane.clone();
    let tab = layout_service::open_tab(
        &mut layout,
        &target_pane,
        taide_lib::domain::layout::types::Tab {
            id: taide_lib::ids::TabId::new(),
            kind: TabKind::File { path: "a.rs".to_string() },
            title: "a.rs".to_string(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        },
        false,
    )
    .expect("open tab");

    layout_service::split(&mut layout, &target_pane, DropEdge::Right, &tab).expect("split");

    let PaneNode::Split { sizes, children, .. } = &layout.root else {
        panic!("루트가 Split 이어야 한다");
    };

    assert_eq!(sizes.len(), children.len());
    let total: f32 = sizes.iter().sum();
    assert!(
        (total - 100.0).abs() < 0.001,
        "sizes 는 퍼센트(0..100) 단위여야 한다 — react-resizable-panels v4 Layout 과 동일 단위. 실제 합: {total}"
    );
}
