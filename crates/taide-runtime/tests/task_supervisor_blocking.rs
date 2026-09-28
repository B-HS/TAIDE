use std::future::pending;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use taide_runtime::TaskSupervisor;
use tokio::runtime::Builder;
use tokio::sync::oneshot;

const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const TWO_WORKERS: usize = 2;

#[tokio::test]
async fn 같은_이름의_blocking_작업은_각각_추적하고_완료시_회수한다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (first_started, first_start) = oneshot::channel();
    let (first_release, first_wait) = mpsc::channel();
    let (second_started, second_start) = oneshot::channel();
    let (second_release, second_wait) = mpsc::channel();
    let first = supervisor
        .spawn_blocking_transient_handle("menu-refresh", move || {
            first_started.send(()).unwrap();
            first_wait.recv_timeout(TEST_TIMEOUT).unwrap();
        })
        .unwrap();
    let second = supervisor
        .spawn_blocking_transient_handle("menu-refresh", move || {
            second_started.send(()).unwrap();
            second_wait.recv_timeout(TEST_TIMEOUT).unwrap();
        })
        .unwrap();
    tokio::time::timeout(TEST_TIMEOUT, first_start).await.unwrap().unwrap();
    tokio::time::timeout(TEST_TIMEOUT, second_start).await.unwrap().unwrap();
    assert_eq!(supervisor.tracked_count(), TWO_WORKERS);
    first_release.send(()).unwrap();
    second_release.send(()).unwrap();
    tokio::time::timeout(TEST_TIMEOUT, first).await.unwrap().unwrap();
    tokio::time::timeout(TEST_TIMEOUT, second).await.unwrap().unwrap();
    assert_eq!(supervisor.tracked_count(), 0);
    supervisor
        .spawn_blocking_transient_handle("menu-refresh", || {})
        .unwrap()
        .await
        .unwrap();
    assert_eq!(supervisor.tracked_count(), 0);
}

#[tokio::test]
async fn 종료는_async_waiter를_취소하지만_시작한_blocking_작업은_완료까지_추적한다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (started, start) = oneshot::channel();
    let (release, wait) = mpsc::channel();
    let handle = supervisor
        .spawn_blocking_transient_handle("menu-refresh", move || {
            started.send(()).unwrap();
            wait.recv_timeout(TEST_TIMEOUT).unwrap();
        })
        .unwrap();
    tokio::time::timeout(TEST_TIMEOUT, start).await.unwrap().unwrap();
    let (finished, finish) = oneshot::channel();
    let observer = supervisor
        .spawn_transient_handle("menu-refresh-completion", async move {
            handle.await.unwrap();
            finished.send(()).unwrap();
        })
        .unwrap();
    assert_eq!(supervisor.tracked_count(), TWO_WORKERS);
    supervisor.stop_all();
    supervisor.stop_all();
    assert!(tokio::time::timeout(TEST_TIMEOUT, observer)
        .await
        .unwrap()
        .unwrap_err()
        .is_cancelled());
    assert!(finish.await.is_err());
    assert_eq!(supervisor.tracked_count(), 1);
    release.send(()).unwrap();
    tokio::time::timeout(TEST_TIMEOUT, async {
        while supervisor.tracked_count() > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(supervisor.spawn_blocking_transient_handle("menu-refresh", || {}).is_none());
}

#[test]
fn 종료시_queued_blocking_작업은_실행하지_않고_캡처와_추적을_회수한다() {
    let runtime = Builder::new_current_thread().enable_time().max_blocking_threads(1).build().unwrap();
    runtime.block_on(async {
        let supervisor = TaskSupervisor::new(runtime.handle().clone());
        let (started, start) = oneshot::channel();
        let (release, wait) = mpsc::channel();
        let blocker = runtime.spawn_blocking(move || {
            started.send(()).unwrap();
            wait.recv_timeout(TEST_TIMEOUT).unwrap();
        });
        tokio::time::timeout(TEST_TIMEOUT, start).await.unwrap().unwrap();
        let called = Arc::new(AtomicBool::new(false));
        let operation_called = called.clone();
        let (captured, capture) = oneshot::channel::<()>();
        let handle = supervisor
            .spawn_blocking_transient_handle("menu-refresh", move || {
                let _captured = captured;
                operation_called.store(true, Ordering::SeqCst);
            })
            .unwrap();
        assert_eq!(supervisor.tracked_count(), 1);
        supervisor.stop_all();
        release.send(()).unwrap();
        tokio::time::timeout(TEST_TIMEOUT, blocker).await.unwrap().unwrap();
        assert!(tokio::time::timeout(TEST_TIMEOUT, handle)
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled());
        assert!(capture.await.is_err());
        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(supervisor.tracked_count(), 0);
    });
}

#[tokio::test]
async fn 종료_뒤_blocking_등록은_실행하지_않고_캡처를_해제한다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    supervisor.stop_all();
    let (captured, capture) = oneshot::channel::<()>();
    let called = Arc::new(AtomicBool::new(false));
    let operation_called = called.clone();
    assert!(supervisor
        .spawn_blocking_transient_handle("menu-refresh", move || {
            let _captured = captured;
            operation_called.store(true, Ordering::SeqCst);
        })
        .is_none());
    assert!(capture.await.is_err());
    assert!(!called.load(Ordering::SeqCst));
    assert!(!supervisor.spawn_transient("observer", pending()));
}

#[tokio::test]
async fn blocking_panic은_join_오류로_확인하고_추적을_회수한다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let handle = supervisor
        .spawn_blocking_transient_handle("menu-refresh", || panic!("synthetic worker panic"))
        .unwrap();
    assert!(tokio::time::timeout(TEST_TIMEOUT, handle).await.unwrap().unwrap_err().is_panic());
    assert_eq!(supervisor.tracked_count(), 0);
    supervisor
        .spawn_blocking_transient_handle("menu-refresh", || {})
        .unwrap()
        .await
        .unwrap();
    assert_eq!(supervisor.tracked_count(), 0);
}

#[test]
fn 메뉴_listener는_blocking_worker와_결과_waiter를_감독하고_언어_gate를_보존한다() {
    let app = include_str!("../../../src-tauri/src/lib.rs");
    let schedule = app.split_once("fn schedule_menu_refresh(").unwrap().1;
    let schedule = schedule.split_once("fn listen_for_app_menu_refresh(").unwrap().0;
    let listener = app.split_once("fn listen_for_app_menu_refresh(").unwrap().1;
    let listener = listener.split_once("/// `pty_spawn`/`pty_attach`").unwrap().0;
    assert!(schedule.contains("spawn_blocking_transient_handle(name, move || refresh(&handle))"));
    assert!(schedule.contains("spawn_transient(\"menu-refresh-completion\""));
    assert!(schedule.contains("refresh.await"));
    assert!(listener.contains("\"menu-recent-refresh\""));
    assert!(listener.contains("domain::window::menu::refresh_recent_menu"));
    assert!(listener.contains("\"menu-language-refresh\""));
    assert!(listener.contains("domain::window::commands::refresh_app_menu"));
    assert!(listener.contains("if *drawn == changed.settings.language"));
    assert!(listener.find("drop(drawn)").unwrap() < listener.find("\"menu-language-refresh\"").unwrap());
    assert!(!listener.contains("tauri::async_runtime::spawn_blocking("));
}
