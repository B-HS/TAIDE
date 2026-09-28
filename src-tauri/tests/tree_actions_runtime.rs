use std::future::{poll_fn, Future};
use std::path::PathBuf;
use std::task::Poll;
use std::time::Duration;

use taide_model::error::AppErrorKind;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_runtime::{tree_actions, AppState, TaskSupervisor, TreeStore};
use uuid::Uuid;

const ACTION_TIMEOUT: Duration = Duration::from_secs(5);

struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    state: AppState,
    store: TreeStore,
    tasks: TaskSupervisor,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-tree-actions-{}", Uuid::new_v4()));
        let root = dir.join("project");
        std::fs::create_dir_all(root.join("sub")).expect("프로젝트 디렉터리 생성");
        std::fs::write(root.join("sub/child.txt"), "child").expect("하위 파일 생성");
        let state = AppState::new(AppPaths::new(dir.join("data")));
        let project_id = ProjectId::new();
        state.projects.write().insert(
            project_id.clone(),
            Project {
                id: project_id.clone(),
                root: root.to_string_lossy().into_owned(),
                name: "project".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        Self {
            dir,
            root,
            state,
            store: TreeStore::new(),
            tasks: TaskSupervisor::new(tokio::runtime::Handle::current()),
            project_id,
        }
    }

    fn path(&self, name: &str) -> String {
        self.root.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("테스트 디렉터리 정리");
    }
}

#[tokio::test]
async fn 다섯_action은_페이지와_캐시와_확장과_접기와_새로고침을_보존한다() {
    let fixture = Fixture::new();
    let page = tree_actions::tree_rows(
        &fixture.state,
        &fixture.store,
        &fixture.tasks,
        fixture.project_id.clone(),
        0,
        Some(1),
    )
    .await
    .expect("초기 페이지");
    assert_eq!(page.total, 1);
    assert_eq!(page.rows[0].name, "sub");
    assert!(!page.rows[0].expanded);
    let empty = tree_actions::tree_rows(
        &fixture.state,
        &fixture.store,
        &fixture.tasks,
        fixture.project_id.clone(),
        1,
        Some(1),
    )
    .await
    .expect("페이지 범위");
    assert_eq!(empty.total, page.total);
    assert!(empty.rows.is_empty());
    let expanded = tree_actions::tree_toggle(
        &fixture.state,
        &fixture.store,
        &fixture.tasks,
        fixture.project_id.clone(),
        fixture.path("sub"),
    )
    .await
    .expect("펼치기");
    assert!(expanded.rows.iter().any(|row| row.name == "child.txt" && row.depth == 1));
    let collapsed = tree_actions::tree_collapse_all(&fixture.state, &fixture.store, &fixture.tasks, fixture.project_id.clone())
        .await
        .expect("모두 접기");
    assert_eq!(collapsed, page);
    let revealed = tree_actions::tree_reveal(
        &fixture.state,
        &fixture.store,
        &fixture.tasks,
        fixture.project_id.clone(),
        fixture.path("sub/child.txt"),
    )
    .await
    .expect("파일 표시");
    assert_eq!(revealed, expanded);
    std::fs::write(fixture.path("sub/new.txt"), "new").expect("새 파일");
    let refreshed = tree_actions::tree_refresh(
        &fixture.state,
        &fixture.store,
        &fixture.tasks,
        fixture.project_id.clone(),
        fixture.path("sub"),
    )
    .await
    .expect("새로고침");
    assert_eq!(refreshed.total, revealed.total + 1);
    assert!(refreshed.rows.iter().any(|row| row.name == "new.txt"));
}

#[tokio::test]
async fn 없는_프로젝트의_다섯_action은_캐시를_만들지_않는다() {
    let fixture = Fixture::new();
    let missing = ProjectId::new();
    let results = [
        tree_actions::tree_rows(&fixture.state, &fixture.store, &fixture.tasks, missing.clone(), 0, None).await,
        tree_actions::tree_toggle(&fixture.state, &fixture.store, &fixture.tasks, missing.clone(), fixture.path("sub")).await,
        tree_actions::tree_collapse_all(&fixture.state, &fixture.store, &fixture.tasks, missing.clone()).await,
        tree_actions::tree_reveal(
            &fixture.state,
            &fixture.store,
            &fixture.tasks,
            missing.clone(),
            fixture.path("sub/child.txt"),
        )
        .await,
        tree_actions::tree_refresh(&fixture.state, &fixture.store, &fixture.tasks, missing, fixture.path("sub")).await,
    ];
    for result in results {
        assert_eq!(result.expect_err("없는 프로젝트 거절").kind(), AppErrorKind::NotFound);
    }
    assert!(fixture.store.0.read().is_empty());
}

#[tokio::test]
async fn 전역_mutation이_대기하는_동안에도_조회는_진행하고_뒤의_toggle을_잃지_않는다() {
    let fixture = Fixture::new();
    let guard = fixture.state.begin_mutation().await;
    let mut toggle = Box::pin(tree_actions::tree_toggle(
        &fixture.state,
        &fixture.store,
        &fixture.tasks,
        fixture.project_id.clone(),
        fixture.path("sub"),
    ));
    poll_fn(|cx| {
        assert!(toggle.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    let page = tokio::time::timeout(
        ACTION_TIMEOUT,
        tree_actions::tree_rows(&fixture.state, &fixture.store, &fixture.tasks, fixture.project_id.clone(), 0, None),
    )
    .await
    .expect("조회는 전역 mutation 잠금을 기다리지 않음")
    .expect("조회 성공");
    assert_eq!(page.total, 1);
    drop(guard);
    let toggled = toggle.await.expect("대기한 toggle 완료");
    assert!(toggled.rows.iter().any(|row| row.name == "child.txt"));
    let cached = tree_actions::tree_rows(&fixture.state, &fixture.store, &fixture.tasks, fixture.project_id.clone(), 0, None)
        .await
        .expect("수정한 캐시 조회");
    assert_eq!(cached, toggled);
}

#[tokio::test]
async fn 먼저_시작한_수정은_대기_중_닫힌_프로젝트_캐시를_부활시키지_않는다() {
    let fixture = Fixture::new();
    let guard = fixture.state.begin_mutation().await;
    let mut toggle = Box::pin(tree_actions::tree_toggle(
        &fixture.state,
        &fixture.store,
        &fixture.tasks,
        fixture.project_id.clone(),
        fixture.path("sub"),
    ));
    poll_fn(|cx| {
        assert!(toggle.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    fixture.state.projects.write().remove(&fixture.project_id);
    fixture.store.remove(&fixture.project_id);
    drop(guard);
    assert_eq!(toggle.await.expect_err("종료한 프로젝트 거절").kind(), AppErrorKind::NotFound);
    assert!(fixture.store.0.read().is_empty());
}

#[test]
fn tree_adapter는_다섯_action을_위임하고_perf_span과_공개_store_경로를_유지한다() {
    let commands = include_str!("../src/domain/tree/commands.rs");
    for action in ["tree_rows", "tree_toggle", "tree_collapse_all", "tree_reveal", "tree_refresh"] {
        assert!(commands.contains(&format!("tree_actions::{action}(")));
    }
    assert!(commands.contains("pub use taide_runtime::TreeStore"));
    assert!(commands.contains("perf::span(SpanSlot::TreeToggle)"));
    assert!(commands.contains("perf::span(SpanSlot::TreeReveal)"));
    assert!(!commands.contains("spawn_blocking("));
    assert!(!commands.contains("begin_mutation("));
    assert!(!commands.contains("tree_store.0"));
}
