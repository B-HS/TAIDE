use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::error::{AppError, AppErrorKind};
use taide_runtime::project_build::run_project_build;
use taide_runtime::{ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;

const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const PENDING_PROBE_MS: u64 = 20;

struct Release(Option<std::sync::mpsc::Sender<()>>);

impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            sender.send(()).ok();
        }
    }
}

struct Resource {
    dropped: Arc<AtomicBool>,
    started_drop: Option<oneshot::Sender<()>>,
    release_drop: Option<std::sync::mpsc::Receiver<()>>,
}

impl Drop for Resource {
    fn drop(&mut self) {
        if let Some(sender) = self.started_drop.take() {
            sender.send(()).ok();
        }
        if let Some(receiver) = self.release_drop.take() {
            receiver.recv().unwrap();
        }
        self.dropped.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn 성공한_build는_결과_resource와_최종_operation을_같이_소유한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = dropped.clone();
    let built = run_project_build(&tasks, move || Resource {
        dropped: marker,
        started_drop: None,
        release_drop: None,
    })
    .await
    .unwrap();
    assert!(!built.value.dropped.load(Ordering::SeqCst));
    assert_eq!(tasks.tracked_count(), 1);
    drop(built);
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 닫힌_감독자는_build_factory를_실행하지_않는다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let called = Arc::new(AtomicBool::new(false));
    let marker = called.clone();
    tasks.stop_all();
    let result = run_project_build(&tasks, move || marker.store(true, Ordering::SeqCst)).await;
    assert!(matches!(result, Err(AppError::Forbidden(_))));
    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn build_panic은_join_오류를_반환하고_operation과_worker를_회수한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    match run_project_build(&tasks, || panic!("fixture project build panic")).await {
        Err(error) => assert_eq!(error.kind(), AppErrorKind::Internal),
        Ok(_) => panic!("panic은 성공 결과가 아니다"),
    }
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 요청_abort_뒤_정상_root는_시작한_worker와_버려진_resource_drop을_기다린다() {
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let (started, started_rx) = oneshot::channel();
    let (release_work, work_rx) = std::sync::mpsc::channel();
    let release_work = Release(Some(release_work));
    let (dropping, dropping_rx) = oneshot::channel();
    let (release_drop, drop_rx) = std::sync::mpsc::channel();
    let release_drop = Release(Some(release_drop));
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = dropped.clone();
    let request_tasks = tasks.clone();
    let request = tokio::spawn(async move {
        run_project_build(&request_tasks, move || {
            started.send(()).ok();
            work_rx.recv().unwrap();
            Resource {
                dropped: marker,
                started_drop: Some(dropping),
                release_drop: Some(drop_rx),
            }
        })
        .await
    });
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
        .await
        .unwrap()
        .unwrap();
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
    let mut drain = ExitDrain::default();
    let (ready, mut ready_rx) = oneshot::channel();
    assert!(drain.begin(
        &runtime,
        tasks.clone(),
        LspInstallStore::new(),
        LspStore::new(),
        TerminalStore::new(),
        move || {
            ready.send(()).ok();
        }
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    drop(release_work);
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), dropping_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!dropped.load(Ordering::SeqCst));
    drop(release_drop);
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(drain.is_ready());
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn worker_완료_뒤에도_post_await_commit과_event의_owner를_기다린다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = dropped.clone();
    {
        let built = run_project_build(&tasks, move || Resource {
            dropped: marker,
            started_drop: None,
            release_drop: None,
        })
        .await
        .unwrap();
        let resource = built.value;
        drop(resource);
        assert!(dropped.load(Ordering::SeqCst));
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), tasks.shutdown())
            .await
            .is_err());
        assert_eq!(tasks.tracked_count(), 1);
    }
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 반환된_resource의_drop이_완료되기_전_operation을_반납하지_않는다() {
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let (dropping, dropping_rx) = oneshot::channel();
    let (release, drop_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let dropped = Arc::new(AtomicBool::new(false));
    let marker = dropped.clone();
    let built = run_project_build(&tasks, move || Resource {
        dropped: marker,
        started_drop: Some(dropping),
        release_drop: Some(drop_rx),
    })
    .await
    .unwrap();
    let dropping_worker = runtime.spawn_blocking(move || drop(built));
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), dropping_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), tasks.shutdown())
        .await
        .is_err());
    assert!(!dropped.load(Ordering::SeqCst));
    assert_eq!(tasks.tracked_count(), 1);
    drop(release);
    dropping_worker.await.unwrap();
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
        .await
        .unwrap();
    assert!(dropped.load(Ordering::SeqCst));
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 같은_이름의_독립_build는_자기_결과와_owner를_각각_회수한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (first, second) = tokio::join!(run_project_build(&tasks, || "first"), run_project_build(&tasks, || "second"));
    let first = first.unwrap();
    let second = second.unwrap();
    assert_eq!(first.value, "first");
    assert_eq!(second.value, "second");
    assert_eq!(tasks.tracked_count(), 2);
    drop(first);
    assert_eq!(tasks.tracked_count(), 1);
    drop(second);
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn native_attach와_restore는_등록된_감독자로_build하고_guard_뒤_원래_순서로_commit한다() {
    let native = include_str!("../../../src-tauri/src/domain/project/commands.rs");
    let attach = native
        .split_once("async fn attach_project_capabilities(app:")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0;
    let restore = native
        .split_once("pub(crate) fn restore_project_watchers(")
        .unwrap()
        .1
        .split_once("\n}\n")
        .unwrap()
        .0;
    for body in [attach, restore] {
        assert!(body.contains("run_project_build("));
        assert!(body.contains("state::<TaskSupervisor>()"));
        assert!(!body.contains("tauri::async_runtime::spawn_blocking"));
        assert!(body.find("run_project_build(").unwrap() < body.find("begin_mutation()").unwrap());
    }
    assert!(attach.contains("built.value"));
    assert!(attach.find("commit_attachments(").unwrap() < attach.find("GitStatusChanged").unwrap());
    assert!(restore.contains("built.value"));
    assert!(restore.find("register_file(&state").unwrap() < restore.find("register_git(&state").unwrap());
    assert!(restore.find("drop(_guard)").unwrap() < restore.find("AppEvent::FsChanged").unwrap());
}

#[tokio::test]
async fn 기존_미감독_nested_worker_패턴은_작업이_미완료여도_root를_해제한다() {
    let runtime = tokio::runtime::Handle::current();
    let tasks = TaskSupervisor::new(runtime.clone());
    let (started, started_rx) = oneshot::channel();
    let (release, release_rx) = std::sync::mpsc::channel();
    let release = Release(Some(release));
    let (finished, mut finished_rx) = oneshot::channel();
    assert!(tasks.spawn("fixture-old-project-restore", async move {
        runtime
            .spawn_blocking(move || {
                started.send(()).ok();
                release_rx.recv().unwrap();
                finished.send(()).ok();
            })
            .await
            .unwrap();
    }));
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), &mut finished_rx)
        .await
        .is_err());
    drop(release);
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), finished_rx)
        .await
        .unwrap()
        .unwrap();
}
