use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use taide_model::error::AppErrorKind;
use taide_model::ids::{ProjectId, TabId};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_runtime::{file_actions, AppState};
use uuid::Uuid;

const ACTION_TIMEOUT: Duration = Duration::from_secs(5);

struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    state: AppState,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-file-actions-{}", Uuid::new_v4()));
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

#[tokio::test]
async fn 파일_권한_확인이_overlay_조회보다_먼저이며_cli와_raw_읽기를_보존한다() {
    let fixture = Fixture::new();
    let outside = fixture.dir.join("outside.txt");
    std::fs::write(&outside, "outside").expect("외부 파일 생성");
    let path = outside.to_string_lossy().into_owned();
    let overlay_loaded = AtomicBool::new(false);
    let error = file_actions::file_open(&fixture.state, path.clone(), || {
        overlay_loaded.store(true, Ordering::SeqCst);
        Vec::new()
    })
    .await
    .expect_err("외부 파일 거절");
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    assert!(!overlay_loaded.load(Ordering::SeqCst));
    assert!(file_actions::file_read_raw(&fixture.state, path.clone())
        .await
        .is_err_and(|error| error.kind() == AppErrorKind::Forbidden));
    assert!(file_actions::file_delete(&fixture.state, path.clone())
        .await
        .is_err_and(|error| error.kind() == AppErrorKind::Forbidden));
    assert_eq!(std::fs::read_to_string(&outside).expect("거절 뒤 파일 확인"), "outside");

    fixture.state.authorize_cli_opened_path(&outside);
    let opened = file_actions::file_open(&fixture.state, path.clone(), || {
        overlay_loaded.store(true, Ordering::SeqCst);
        Vec::new()
    })
    .await
    .expect("CLI 승인 파일 열기");
    assert_eq!(opened.content, "outside");
    assert!(overlay_loaded.load(Ordering::SeqCst));
    file_actions::file_save(&fixture.state, path.clone(), "saved".to_string())
        .await
        .expect("CLI 저장");
    assert_eq!(file_actions::file_read_raw(&fixture.state, path).await.expect("raw 읽기"), b"saved");
}

#[tokio::test]
async fn 생성과_이름변경과_복사는_프로젝트_안에서만_동작한다() {
    let fixture = Fixture::new();
    let source = fixture.path("source.txt");
    let renamed = fixture.path("renamed.txt");
    let copied = fixture.path("copied.txt");
    file_actions::file_create(&fixture.state, source.clone(), false)
        .await
        .expect("파일 생성");
    file_actions::file_save(&fixture.state, source.clone(), "source".to_string())
        .await
        .expect("파일 저장");
    file_actions::file_rename(&fixture.state, source.clone(), renamed.clone())
        .await
        .expect("이름 변경");
    assert!(!PathBuf::from(source).exists());
    file_actions::file_copy(&fixture.state, renamed.clone(), copied.clone())
        .await
        .expect("파일 복사");
    assert_eq!(std::fs::read_to_string(copied).expect("복사 파일 읽기"), "source");
    let outside = fixture.dir.join("outside-copy.txt").to_string_lossy().into_owned();
    assert!(file_actions::file_copy(&fixture.state, renamed, outside.clone())
        .await
        .is_err_and(|error| error.kind() == AppErrorKind::Forbidden));
    assert!(!PathBuf::from(outside).exists());
    let directory = fixture.path("directory");
    file_actions::file_create(&fixture.state, directory.clone(), true)
        .await
        .expect("디렉터리 생성");
    assert!(PathBuf::from(directory).is_dir());
}

#[tokio::test]
async fn 미러_생성은_mutation을_기다리지_않고_목록과_prune과_clear를_보존한다() {
    let fixture = Fixture::new();
    let target = fixture.path("target.txt");
    let another = fixture.path("another.txt");
    std::fs::write(&target, "disk").expect("대상 생성");
    std::fs::write(&another, "disk").expect("다른 대상 생성");
    let first_tab = TabId::new();
    let second_tab = TabId::new();
    let guard = fixture.state.begin_mutation().await;
    let baseline = tokio::time::timeout(
        ACTION_TIMEOUT,
        file_actions::file_mirror_dirty(&fixture.state, fixture.project_id.clone(), target.clone(), "draft".to_string()),
    )
    .await
    .expect("미러는 전역 mutation을 기다리지 않음")
    .expect("미러 생성");
    assert!(baseline.is_some());
    tokio::time::timeout(
        ACTION_TIMEOUT,
        file_actions::file_mirror_untitled(
            &fixture.state,
            fixture.project_id.clone(),
            first_tab.clone(),
            "untitled".to_string(),
        ),
    )
    .await
    .expect("untitled 미러는 전역 mutation을 기다리지 않음")
    .expect("untitled 미러 생성");
    assert!(tokio::time::timeout(
        ACTION_TIMEOUT,
        file_actions::file_create(&fixture.state, fixture.path("blocked.txt"), false)
    )
    .await
    .is_err());
    assert!(!fixture.root.join("blocked.txt").exists());
    drop(guard);

    file_actions::file_mirror_dirty(&fixture.state, fixture.project_id.clone(), another, "other".to_string())
        .await
        .expect("두 번째 미러");
    file_actions::file_prune_mirrors(&fixture.state, fixture.project_id.clone(), vec![target.clone()])
        .await
        .expect("미러 prune");
    let mirrors = file_actions::file_list_mirrors(&fixture.state, fixture.project_id.clone())
        .await
        .expect("미러 목록");
    assert_eq!(mirrors.len(), 1);
    assert_eq!(mirrors[0].content, "draft");
    file_actions::file_clear_mirror(&fixture.state, fixture.project_id.clone(), target)
        .await
        .expect("미러 clear");
    assert!(file_actions::file_list_mirrors(&fixture.state, fixture.project_id.clone())
        .await
        .expect("빈 미러 목록")
        .is_empty());

    file_actions::file_mirror_untitled(&fixture.state, fixture.project_id.clone(), second_tab, "other".to_string())
        .await
        .expect("두 번째 untitled");
    file_actions::file_prune_untitled_mirrors(&fixture.state, fixture.project_id.clone(), vec![first_tab.clone()])
        .await
        .expect("untitled prune");
    let mirrors = file_actions::file_list_untitled_mirrors(&fixture.state, fixture.project_id.clone())
        .await
        .expect("untitled 목록");
    assert_eq!(mirrors.len(), 1);
    assert_eq!(mirrors[0].tab_id, first_tab);
    file_actions::file_clear_untitled_mirror(&fixture.state, fixture.project_id.clone(), first_tab)
        .await
        .expect("untitled clear");
    assert!(file_actions::file_list_untitled_mirrors(&fixture.state, fixture.project_id.clone())
        .await
        .expect("빈 untitled 목록")
        .is_empty());
    assert!(file_actions::file_mirror_untitled(
        &fixture.state,
        fixture.project_id.clone(),
        TabId::from("../escape".to_string()),
        "x".to_string()
    )
    .await
    .is_err());
    assert!(file_actions::file_list_mirrors(&fixture.state, ProjectId::new()).await.is_err());
}

#[test]
fn 파일_adapter는_15개_runtime_action과_창_flush_확인만_조립한다() {
    let source = include_str!("../src/domain/file/commands.rs");
    for action in [
        "file_open",
        "file_save",
        "file_create",
        "file_rename",
        "file_delete",
        "file_copy",
        "file_mirror_dirty",
        "file_list_mirrors",
        "file_clear_mirror",
        "file_prune_mirrors",
        "file_mirror_untitled",
        "file_list_untitled_mirrors",
        "file_clear_untitled_mirror",
        "file_prune_untitled_mirrors",
        "file_read_raw",
    ] {
        assert!(source.contains(&format!("file_actions::{action}(")), "{action} adapter 위임");
    }
    assert!(!source.contains("spawn_blocking"));
    assert!(!source.contains("root_guard::"));
    assert!(source.contains("(plugins.language_overlays)(&app)"));
    assert!(source.contains("state.complete_flush(&scope, window.label())"));
    assert!(source.contains("app.exit(0)"));
    assert!(source.contains("tauri::ipc::Response::new(bytes)"));
}
