use std::future::pending;
use std::time::Duration;

use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;

#[tokio::test]
async fn 장기_작업은_이름별로_한_번만_등록되고_종료시_취소된다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (sender, receiver) = oneshot::channel::<()>();

    assert!(supervisor.spawn("agent-poll", async move {
        let _sender = sender;
        pending::<()>().await;
    }));
    assert!(!supervisor.spawn("agent-poll", async {}));
    assert_eq!(supervisor.tracked_count(), 1);

    supervisor.stop_all();

    assert_eq!(supervisor.tracked_count(), 0);
    assert!(tokio::time::timeout(Duration::from_secs(1), receiver).await.unwrap().is_err());
    assert!(!supervisor.spawn("agent-poll", async {}));
    supervisor.stop_all();
}

#[tokio::test]
async fn 완료된_이름별_작업은_같은_이름으로_다시_등록할_수_있다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (sender, receiver) = oneshot::channel::<()>();

    assert!(supervisor.spawn("boot", async move {
        sender.send(()).expect("완료 신호");
    }));
    receiver.await.expect("첫 작업 완료");
    tokio::time::timeout(Duration::from_secs(1), async {
        while supervisor.tracked_count() > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("완료 항목 회수");
    assert!(supervisor.spawn("boot", pending()));
    supervisor.stop_all();
}

#[tokio::test]
async fn 반복_작업은_같은_이름으로_각각_추적하고_종료시_취소한다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (first_sender, first_receiver) = oneshot::channel::<()>();
    let (second_sender, second_receiver) = oneshot::channel::<()>();

    assert!(supervisor.spawn_transient("menu-open-recent", async move {
        let _sender = first_sender;
        pending::<()>().await;
    }));
    assert!(supervisor.spawn_transient("menu-open-recent", async move {
        let _sender = second_sender;
        pending::<()>().await;
    }));
    assert_eq!(supervisor.tracked_count(), 2);

    supervisor.stop_all();
    assert_eq!(supervisor.tracked_count(), 0);
    assert!(tokio::time::timeout(Duration::from_secs(1), first_receiver).await.unwrap().is_err());
    assert!(tokio::time::timeout(Duration::from_secs(1), second_receiver)
        .await
        .unwrap()
        .is_err());
    assert!(!supervisor.spawn_transient("menu-open-recent", async {}));
}

#[tokio::test]
async fn 완료된_반복_작업은_추적_목록에서_회수된다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (sender, receiver) = oneshot::channel::<()>();

    assert!(supervisor.spawn_transient("menu-clear-recent", async move {
        sender.send(()).expect("완료 신호");
    }));
    receiver.await.expect("반복 작업 완료");
    tokio::time::timeout(Duration::from_secs(1), async {
        while supervisor.tracked_count() > 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("반복 작업 회수");
}

#[test]
fn 앱_조립은_장기_작업과_자동_시작을_등록하고_종료시_취소한다() {
    let app = include_str!("../src/lib.rs");
    let setup = app.split_once("app.manage(services.windows.clone());").unwrap().1;
    let boot = setup
        .split_once("if app.state::<AppState>().settings.read().agent_hooks_enabled {")
        .unwrap()
        .1;
    let boot = boot.split_once("let ide_reconcile_handle =").unwrap().0;
    let exit = app.split_once(".run(|app_handle, event| {").unwrap().1;

    assert!(app.contains("TaskSupervisor::new(tauri::async_runtime::handle().inner().clone())"));
    assert!(setup.contains("app.manage(services.tasks.clone());"));
    assert!(boot.contains(".spawn(\"agent-hooks-boot\""));
    assert!(boot.contains(".spawn(\"ide-boot\""));
    assert!(boot.contains(".spawn(\"remote-boot\""));
    assert!(!boot.contains("tauri::async_runtime::spawn("));
    assert!(setup.contains(".spawn(\"ide-reconcile\""));
    assert!(setup.contains(".spawn(\"agent-poll\""));
    assert!(setup.contains(".spawn(\"layout-flush\""));
    assert!(exit.contains("app_handle.state::<TaskSupervisor>().stop_all()"));
}

#[test]
fn 메뉴와_보조_창의_반복_작업은_감독_범위에_등록된다() {
    let app = include_str!("../src/lib.rs");
    let return_tabs = app.split_once("fn plan_return_of_auxiliary_window_tabs(").unwrap().1;
    let return_tabs = return_tabs.split_once("fn dispatch_menu_action(").unwrap().0;
    let menu = app.split_once("fn dispatch_menu_action(").unwrap().1;
    let menu = menu.split_once("fn recent_project_root(").unwrap().0;

    assert!(return_tabs.contains("spawn_transient(\"auxiliary-tab-return\""));
    assert!(menu.contains("spawn_transient(\"menu-clear-recent\""));
    assert!(menu.contains("spawn_transient(\"menu-open-recent\""));
    assert!(!return_tabs.contains("tauri::async_runtime::spawn("));
    assert!(!menu.contains("tauri::async_runtime::spawn("));
}

#[test]
fn 프로젝트_attach의_hook_재조정은_감독_범위에_등록된다() {
    let capability = include_str!("../src/domain/agent/capability.rs");

    assert!(capability.contains("app.state::<TaskSupervisor>().spawn_transient(\"agent-hooks-attach\""));
    assert!(!capability.contains("tauri::async_runtime::spawn(async move {"));
}

#[test]
fn 창_flush와_복원_작업은_감독_범위에_등록된다() {
    let commands = include_str!("../src/domain/window/commands.rs");

    assert!(commands.contains("spawn_transient(\"auxiliary-window-flush\""));
    assert!(commands.contains("spawn_transient(\"hot-exit-timeout\""));
    assert!(commands.contains("spawn_transient(\"auxiliary-window-restore\""));
    assert!(!commands.contains("tauri::async_runtime::spawn(async move {"));
}
