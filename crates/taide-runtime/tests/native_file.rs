use std::future::Future;
use std::path::PathBuf;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use taide_model::error::AppErrorKind;
use taide_model::file::{FileSizeTier, REFUSED_FILE_BYTES};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_runtime::native_file_actions::open_document_file;
use taide_runtime::{AppState, TaskSupervisor};
use tokio::sync::oneshot;
use tokio::time::timeout;
use uuid::Uuid;

const TEST_TIMEOUT: Duration = Duration::from_secs(3);

struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    state: AppState,
    project_id: ProjectId,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-native-file-{}", Uuid::new_v4()));
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let project_id = ProjectId::new();
        let state = AppState::new(AppPaths::new(dir.join("data")));
        state.projects.write().insert(
            project_id.clone(),
            Project {
                id: project_id.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic-native-file".into(),
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
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

#[tokio::test]
async fn native_열기는_canonical_식별과_표시_경로를_분리하고_기존_파일_권한과_정책을_유지한다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let path = fixture.root.join("한 e\u{301} #% .txt");
    let text = "한𐐀e\u{301}\r\n中文";
    std::fs::write(&path, text).unwrap();
    let display_path = path.to_str().unwrap().to_owned();
    let opened = open_document_file(&fixture.state, &tasks, display_path.clone(), Vec::new)
        .await
        .unwrap();
    assert_eq!(opened.display_path, display_path);
    assert_eq!(opened.canonical_path, std::fs::canonicalize(&path).unwrap());
    assert_eq!(opened.project_id, Some(fixture.project_id.clone()));
    assert_eq!(opened.file.path, opened.canonical_path.to_str().unwrap());
    assert_eq!(opened.file.content, text);
    assert_eq!(opened.file.tier, FileSizeTier::Normal);
    assert!(!opened.file.read_only);

    let outside = fixture.dir.join("cli.txt");
    std::fs::write(&outside, "synthetic-cli").unwrap();
    let outside_path = outside.to_str().unwrap().to_owned();
    let error = open_document_file(&fixture.state, &tasks, outside_path.clone(), || {
        panic!("거절된 경로의 overlay를 읽으면 안 됩니다")
    })
    .await
    .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);

    #[cfg(unix)]
    {
        let alias = fixture.root.join("alias.txt");
        std::os::unix::fs::symlink(&path, &alias).unwrap();
        let alias_path = alias.to_str().unwrap().to_owned();
        let aliased = open_document_file(&fixture.state, &tasks, alias_path.clone(), Vec::new)
            .await
            .unwrap();
        assert_eq!(aliased.canonical_path, opened.canonical_path);
        assert_eq!(aliased.display_path, alias_path);
        let escape = fixture.root.join("escape.txt");
        std::os::unix::fs::symlink(&outside, &escape).unwrap();
        let error = open_document_file(&fixture.state, &tasks, escape.to_str().unwrap().into(), Vec::new)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Forbidden);
    }

    fixture.state.authorize_cli_opened_path(&outside);
    let cli = open_document_file(&fixture.state, &tasks, outside_path, Vec::new).await.unwrap();
    assert_eq!(cli.project_id, None);
    assert_eq!(cli.file.content, "synthetic-cli");

    let lossy = fixture.root.join("lossy.txt");
    std::fs::write(&lossy, [0xc3, 0x28]).unwrap();
    let lossy = open_document_file(&fixture.state, &tasks, lossy.to_str().unwrap().into(), Vec::new)
        .await
        .unwrap();
    assert!(lossy.file.encoding_lossy);
    assert!(lossy.file.read_only);

    let refused = fixture.root.join("refused.txt");
    std::fs::File::create(&refused).unwrap().set_len(REFUSED_FILE_BYTES).unwrap();
    let refused = open_document_file(&fixture.state, &tasks, refused.to_str().unwrap().into(), Vec::new)
        .await
        .unwrap();
    assert_eq!(refused.file.tier, FileSizeTier::Refused);
    assert!(refused.file.content.is_empty());
    assert!(refused.file.read_only);

    {
        let _guard = fixture.state.begin_mutation().await;
        fixture.state.projects.write().remove(&fixture.project_id);
    }
    let error = open_document_file(&fixture.state, &tasks, display_path.clone(), Vec::new)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    tasks.stop_all();
    let error = open_document_file(&fixture.state, &tasks, display_path, Vec::new)
        .await
        .unwrap_err();
    assert_eq!(error.kind(), AppErrorKind::Forbidden);
    timeout(TEST_TIMEOUT, tasks.shutdown()).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(!fixture.dir.join("data").exists());
}

#[tokio::test]
async fn 취소된_native_열기의_실제_worker가_끝날_때까지_프로젝트_닫기와_root_shutdown은_기다린다() {
    let fixture = Fixture::new();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let path = fixture.root.join("held.txt");
    std::fs::write(&path, "held").unwrap();
    let state = fixture.state.clone();
    let request_tasks = tasks.clone();
    let (started, started_rx) = oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let request = tokio::spawn(async move {
        open_document_file(&state, &request_tasks, path.to_str().unwrap().into(), move || {
            started.send(()).ok();
            held.recv().unwrap();
            Vec::new()
        })
        .await
    });
    timeout(TEST_TIMEOUT, started_rx).await.unwrap().unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    let mut closing = std::pin::pin!(fixture.state.begin_mutation());
    let mut shutdown = std::pin::pin!(tasks.shutdown());
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(closing.as_mut().poll(&mut context), Poll::Pending));
    assert!(matches!(shutdown.as_mut().poll(&mut context), Poll::Pending));
    release.send(()).unwrap();
    timeout(TEST_TIMEOUT, shutdown).await.unwrap();
    let _guard = timeout(TEST_TIMEOUT, closing).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}
