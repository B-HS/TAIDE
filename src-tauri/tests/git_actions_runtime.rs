use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use taide_git::store::{GitStore, StatusRead};
use taide_model::app_event::AppEvent;
use taide_model::error::AppError;
use taide_model::git::{DiffMode, GitStatus};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_runtime::{git_actions, AppState, EventSink};
use uuid::Uuid;

const GIT_COMMAND_COUNT: usize = 41;

#[derive(Default)]
struct RecordingSink(Mutex<Vec<AppEvent>>);

impl EventSink for RecordingSink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

fn fixture() -> (AppState, GitStore, ProjectId) {
    let dir = std::env::temp_dir().join(format!("taide-git-actions-{}", Uuid::new_v4()));
    let state = AppState::new(AppPaths::new(dir));
    let store = GitStore::new();
    let project = ProjectId::from("prj-fixture-missing".to_string());
    (state, store, project)
}

#[tokio::test]
async fn 미개방_프로젝트의_조회와_변경은_git_실행과_이벤트_없이_거절한다() {
    let (state, store, project) = fixture();
    let sink = RecordingSink::default();
    let errors = [
        git_actions::git_init(&sink, &state, &store, project.clone()).await.unwrap_err(),
        git_actions::git_diff_staged_text(&state, &store, project.clone())
            .await
            .unwrap_err(),
        git_actions::git_show_file(&state, &store, project.clone(), "HEAD".to_string(), "fixture.rs".to_string())
            .await
            .unwrap_err(),
        git_actions::git_log(&state, &store, project.clone(), 0, 1).await.unwrap_err(),
        git_actions::git_stage(&sink, &state, &store, project.clone(), Vec::new())
            .await
            .unwrap_err(),
        git_actions::git_unstage(&sink, &state, &store, project.clone(), Vec::new())
            .await
            .unwrap_err(),
        git_actions::git_push(&sink, &state, &store, project.clone()).await.unwrap_err(),
        git_actions::git_pull(&sink, &state, &store, project.clone()).await.unwrap_err(),
        git_actions::git_fetch(&sink, &state, &store, project.clone()).await.unwrap_err(),
        git_actions::git_branches(&state, &store, project.clone()).await.unwrap_err(),
        git_actions::git_tags(&state, &store, project.clone()).await.unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(error, AppError::NotFound(message) if message == format!("project not open: {project}")));
    }
    assert!(sink.0.lock().unwrap().is_empty());
    assert!(store.cached_repo_root(&project).is_none());
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn status는_루트_오류보다_먼저_구독_port를_호출한다() {
    let (state, store, project) = fixture();
    let called = AtomicBool::new(false);
    let error = git_actions::git_status(|| called.store(true, Ordering::SeqCst), &state, &store, project)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::NotFound(_)));
    assert!(called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn fresh_status는_기존_공유_cache를_소비하고_git_io를_시작하지_않는다() {
    let (state, store, project) = fixture();
    store.cache_repo_root(project.clone(), state.paths.data_dir.join("missing-repo"));
    let StatusRead::Stale(pending) = store.read_status(&project, Instant::now()) else {
        panic!("초기 cache는 stale이어야 한다");
    };
    let expected = GitStatus {
        rows: Vec::new(),
        branch: Some("fixture".to_string()),
        ahead: 1,
        behind: 0,
        has_remote: false,
    };
    store.finish_status(&project, pending, &expected);
    let called = AtomicBool::new(false);
    let result = git_actions::git_status(|| called.store(true, Ordering::SeqCst), &state, &store, project.clone())
        .await
        .unwrap();
    assert_eq!(result, expected);
    assert!(called.load(Ordering::SeqCst));
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn diff의_plugin_port는_루트_오류_뒤에는_호출하지_않는다() {
    let (state, store, project) = fixture();
    let called = AtomicBool::new(false);
    let error = git_actions::git_diff_file(
        &state,
        &store,
        || {
            called.store(true, Ordering::SeqCst);
            Vec::new()
        },
        project,
        "fixture.rs".to_string(),
        DiffMode::WorkdirVsIndex,
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AppError::NotFound(_)));
    assert!(!called.load(Ordering::SeqCst));
    assert!(!state.paths.data_dir.exists());
}

#[test]
fn 기존_모든_git_command는_runtime으로_위임한다() {
    let source = include_str!("../src/domain/git/commands.rs");
    let production = source.split("#[cfg(test)]").next().unwrap();
    let names = production
        .lines()
        .filter_map(|line| line.strip_prefix("pub async fn "))
        .filter_map(|line| line.split_once('(').map(|(name, _)| name))
        .collect::<Vec<_>>();
    assert_eq!(names.len(), GIT_COMMAND_COUNT);
    for name in names {
        assert!(production.contains(&format!("git_actions::{name}(")), "{name}");
    }
    assert!(!production.contains("tauri::async_runtime::spawn_blocking("));
    assert!(!production.contains("fn resolve_repo_root("));
}
