use std::future::{poll_fn, Future};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::task::Poll;

use taide_model::error::AppErrorKind;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::search::{ReplaceSkipReason, SearchQuery};
use taide_runtime::{search_actions, AppState, SearchStore};
use uuid::Uuid;

struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    state: AppState,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-search-actions-{}", Uuid::new_v4()));
        let root = dir.join("project");
        std::fs::create_dir_all(&root).expect("프로젝트 디렉터리 생성");
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

fn query(text: &str) -> SearchQuery {
    SearchQuery {
        text: text.to_string(),
        case_sensitive: false,
        whole_word: false,
        regex: false,
        include_glob: None,
        exclude_glob: None,
        context_lines: 0,
        respect_gitignore: true,
        scope_dir: None,
    }
}

#[tokio::test]
async fn 검색과_목록은_공유_프로젝트_루트를_사용하고_batch와_완료_정리를_보존한다() {
    let fixture = Fixture::new();
    let path = fixture.path("a.txt");
    std::fs::write(&path, "needle\nother\n").expect("검색 파일");
    let store = SearchStore::new();
    let batches = Arc::new(Mutex::new(Vec::new()));
    let recorded = batches.clone();
    let count = search_actions::search_run(
        &fixture.state,
        &store,
        fixture.project_id.clone(),
        "main".to_string(),
        "panel".to_string(),
        query("needle"),
        move |batch| recorded.lock().unwrap().push(batch),
    )
    .await
    .expect("검색 실행");
    assert_eq!(count, 1);
    {
        let records = batches.lock().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].matches.len(), 1);
        assert_eq!(records[0].matches[0].line, 1);
    }
    assert_eq!(
        search_actions::search_list_files(&fixture.state, fixture.project_id.clone())
            .await
            .expect("파일 목록"),
        [path]
    );
    let missing = ProjectId::new();
    assert!(search_actions::search_list_files(&fixture.state, missing)
        .await
        .is_err_and(|error| error.kind() == AppErrorKind::NotFound));
    let first = store.begin("main", "panel");
    search_actions::search_run(
        &fixture.state,
        &store,
        fixture.project_id.clone(),
        "main".to_string(),
        "panel".to_string(),
        query(""),
        |_| {},
    )
    .await
    .expect("빈 검색 완료");
    assert!(first.load(Ordering::SeqCst));
    search_actions::search_cancel(&fixture.state, &store, "main".to_string(), "panel".to_string())
        .await
        .expect("완료 세션 취소 무해");
}

#[tokio::test]
async fn 검색_취소는_전역_잠금을_기다리고_해당_창과_세션에만_적용된다() {
    let fixture = Fixture::new();
    let store = SearchStore::new();
    let main = store.begin("main", "panel");
    let other = store.begin("other", "panel");
    let guard = fixture.state.begin_mutation().await;
    let mut cancel = Box::pin(search_actions::search_cancel(
        &fixture.state,
        &store,
        "main".to_string(),
        "panel".to_string(),
    ));
    poll_fn(|cx| {
        assert!(cancel.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert!(!main.load(Ordering::SeqCst));
    drop(guard);
    cancel.await.expect("검색 취소");
    assert!(main.load(Ordering::SeqCst));
    assert!(!other.load(Ordering::SeqCst));
}

#[tokio::test]
async fn 치환은_외부_경로를_제외하고_파일별_결과와_skip을_집계한다() {
    let fixture = Fixture::new();
    let matched = fixture.path("matched.txt");
    let no_match = fixture.path("no-match.txt");
    let binary = fixture.path("binary.bin");
    let outside = fixture.dir.join("outside.txt");
    std::fs::write(&matched, "needle").expect("치환 파일");
    std::fs::write(&no_match, "other").expect("불일치 파일");
    std::fs::write(&binary, b"needle\0").expect("바이너리 파일");
    std::fs::write(&outside, "needle").expect("외부 파일");
    let result = search_actions::search_replace(
        &fixture.state,
        fixture.project_id.clone(),
        query("needle"),
        "changed".to_string(),
        Some(vec![
            matched.clone(),
            no_match.clone(),
            binary.clone(),
            outside.to_string_lossy().into_owned(),
        ]),
    )
    .await
    .expect("치환 실행");
    assert_eq!(result.changed_files, 1);
    assert_eq!(result.replaced_matches, 1);
    assert_eq!(result.skipped_count, 1);
    assert_eq!(result.skipped.len(), 1);
    assert_eq!(
        PathBuf::from(&result.skipped[0].path),
        std::fs::canonicalize(&binary).expect("skip 결과는 기존 root guard의 정규 경로"),
    );
    assert_eq!(result.skipped[0].reason, ReplaceSkipReason::Binary);
    let changes = [matched.clone(), no_match.clone(), binary.clone()]
        .into_iter()
        .map(|path| FsChange {
            kind: FsChangeKind::Modified,
            paths: vec![std::fs::canonicalize(path)
                .expect("watcher 정규 경로")
                .to_string_lossy()
                .into_owned()],
            from_app: false,
        })
        .collect();
    let resolved = taide_infra::self_write::resolve_from_app(&fixture.state.self_writes, changes);
    assert!(resolved[0].from_app);
    assert!(!resolved[1].from_app);
    assert!(!resolved[2].from_app);
    assert_eq!(std::fs::read_to_string(matched).expect("치환 결과"), "changed");
    assert_eq!(std::fs::read_to_string(no_match).expect("불일치 보존"), "other");
    assert_eq!(std::fs::read_to_string(outside).expect("외부 보존"), "needle");
}

#[test]
fn 검색_adapter는_네_공개_action을_위임하고_channel과_계측만_유지한다() {
    let commands = include_str!("../src/domain/search/commands.rs");
    for action in ["search_run", "search_cancel", "search_replace", "search_list_files"] {
        assert!(commands.contains(&format!("search_actions::{action}(")));
    }
    assert!(commands.contains("on_match.send(batch)"));
    assert!(!commands.contains("spawn_blocking("));
    assert!(!commands.contains("begin_mutation("));
    assert!(!commands.contains("begin_mutation_blocking("));
}
