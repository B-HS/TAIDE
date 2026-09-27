use std::future::pending;
use std::time::Duration;

use taide_lib::domain::agent::commands::{AgentHooksStore, HooksServerInfo};
use taide_lib::domain::ide::store::IdeStore;
use taide_lib::domain::remote::commands::RemoteStore;
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

#[tokio::test]
async fn 반환된_작업_핸들은_감독자_종료에_함께_취소된다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (sender, receiver) = oneshot::channel::<()>();
    let handle = supervisor
        .spawn_transient_handle("server", async move {
            let _sender = sender;
            pending::<()>().await;
        })
        .expect("작업 핸들");

    assert_eq!(supervisor.tracked_count(), 1);
    supervisor.stop_all();

    assert!(handle.await.unwrap_err().is_cancelled());
    assert!(receiver.await.is_err());
    assert!(supervisor.spawn_transient_handle("server", async {}).is_none());
}

#[tokio::test]
async fn 반환된_작업_핸들의_완료와_직접_취소는_추적에서_회수된다() {
    let supervisor = TaskSupervisor::new(tokio::runtime::Handle::current());
    let completed = supervisor.spawn_transient_handle("server", async {}).expect("완료 작업 핸들");

    completed.await.expect("작업 완료");
    assert_eq!(supervisor.tracked_count(), 0);

    let pending = supervisor.spawn_transient_handle("server", pending()).expect("대기 작업 핸들");
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    assert_eq!(supervisor.tracked_count(), 0);
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

#[test]
fn 시작시_프로젝트_watcher_복원은_감독_범위에_등록된다() {
    let commands = include_str!("../src/domain/project/commands.rs");
    let restore = commands.split_once("pub(crate) fn restore_project_watchers(").unwrap().1;
    let restore = restore.split_once("#[cfg(test)]").unwrap().0;

    assert!(restore.contains("spawn(\"project-watchers-restore\""));
    assert!(!restore.contains("tauri::async_runtime::spawn(async move {"));
}

#[test]
fn lsp_종료와_재시작_지연_작업은_감독_범위에_등록된다() {
    let commands = include_str!("../src/domain/lsp/commands.rs");
    let process = commands.split_once("fn spawn_process(").unwrap().1;
    let process = process.split_once("fn channel_sink(").unwrap().0;
    let restart = commands.split_once("fn handle_process_exit(").unwrap().1;
    let restart = restart.split_once("async fn shutdown_entry(").unwrap().0;

    assert!(process.contains("spawn_transient(\"lsp-process-exit\""));
    assert!(!process.contains("tokio::spawn(async move {"));
    assert!(restart.contains("spawn_transient(\"lsp-auto-restart\""));
    assert!(restart.contains("spawn_transient(\"lsp-healthy-reset\""));
    assert!(!restart.contains("tokio::spawn(async move {"));
}

#[tokio::test]
async fn hook_서버_중복_시작은_먼저_등록한_핸들을_유지한다() {
    let store = AgentHooksStore::default();
    let (first_sender, first_receiver) = oneshot::channel::<()>();
    let (second_sender, second_receiver) = oneshot::channel::<()>();
    let first_handle = tokio::spawn(async move {
        let _sender = first_sender;
        pending::<()>().await;
    });
    let second_handle = tokio::spawn(async move {
        let _sender = second_sender;
        pending::<()>().await;
    });

    let first = store.set_server(
        HooksServerInfo {
            port: 1,
            token: "first".to_string(),
        },
        first_handle,
    );
    let second = store.set_server(
        HooksServerInfo {
            port: 1,
            token: "second".to_string(),
        },
        second_handle,
    );

    assert_eq!(first.token, "first");
    assert_eq!(second.token, "first");
    assert_eq!(store.server_info().expect("기존 서버 정보").token, "first");
    assert!(tokio::time::timeout(Duration::from_secs(1), second_receiver)
        .await
        .unwrap()
        .is_err());
    store.take_server().expect("기존 서버 핸들").abort();
    assert!(tokio::time::timeout(Duration::from_secs(1), first_receiver).await.unwrap().is_err());
}

#[test]
fn hook_서버와_연결_작업은_감독하고_앱_종료에서_중지한다() {
    let hooks = include_str!("../src/domain/agent/hooks.rs");
    let app = include_str!("../src/lib.rs");

    assert!(hooks.contains("spawn_transient_handle(\"agent-hooks-accept\""));
    assert!(hooks.contains("spawn_transient(\"agent-hooks-connection\""));
    assert!(app.contains("domain::agent::hooks::stop_hooks_server(app_handle);"));
}

#[tokio::test]
async fn 원격_서버_중복_시작은_기존_핸들을_유지한다() {
    const FIRST_PORT: u32 = 1;
    const SECOND_PORT: u32 = 2;

    let store = RemoteStore::default();
    let (first_sender, first_receiver) = oneshot::channel::<()>();
    let (second_sender, second_receiver) = oneshot::channel::<()>();
    let first_handle = tauri::async_runtime::spawn(async move {
        let _sender = first_sender;
        pending::<()>().await;
    });
    let second_handle = tauri::async_runtime::spawn(async move {
        let _sender = second_sender;
        pending::<()>().await;
    });
    let (first_shutdown, _) = tokio::sync::watch::channel(());
    let (second_shutdown, _) = tokio::sync::watch::channel(());

    assert!(store.mark_started(FIRST_PORT, first_shutdown, first_handle));
    assert!(!store.mark_started(SECOND_PORT, second_shutdown, second_handle));
    assert_eq!(store.status().port, FIRST_PORT);
    assert!(tokio::time::timeout(Duration::from_secs(1), second_receiver)
        .await
        .unwrap()
        .is_err());

    store
        .take_shutdown_state()
        .expect("기존 서버 상태")
        .server_handle
        .expect("기존 서버 핸들")
        .abort();
    assert!(tokio::time::timeout(Duration::from_secs(1), first_receiver).await.unwrap().is_err());
}

#[test]
fn 원격_서버_작업은_감독_범위에_등록된다() {
    let commands = include_str!("../src/domain/remote/commands.rs");
    let bind = commands.split_once("async fn bind_and_start(").unwrap().1;
    let bind = bind.split_once("/// Refreshes").unwrap().0;

    assert!(bind.contains("spawn_transient_handle(\"remote-server\""));
    assert!(!bind.contains("tauri::async_runtime::spawn(async move {"));
}

#[tokio::test]
async fn ide_서버_중복_시작은_기존_핸들과_lockfile_상태를_유지한다() {
    const FIRST_PORT: u32 = 1;
    const SECOND_PORT: u32 = 2;

    let store = IdeStore::default();
    let (first_sender, first_receiver) = oneshot::channel::<()>();
    let (second_sender, second_receiver) = oneshot::channel::<()>();
    let first_handle = tauri::async_runtime::spawn(async move {
        let _sender = first_sender;
        pending::<()>().await;
    });
    let second_handle = tauri::async_runtime::spawn(async move {
        let _sender = second_sender;
        pending::<()>().await;
    });
    let dir = std::path::PathBuf::from("/tmp/ide");

    assert!(store
        .mark_started(FIRST_PORT, "first".to_string(), dir.clone(), first_handle)
        .is_some());
    assert!(store
        .mark_started(SECOND_PORT, "second".to_string(), dir.clone(), second_handle)
        .is_none());
    assert_eq!(store.status().port, FIRST_PORT);
    assert_eq!(store.lockfile_context(), Some((FIRST_PORT, "first".to_string(), dir)));
    assert!(tokio::time::timeout(Duration::from_secs(1), second_receiver)
        .await
        .unwrap()
        .is_err());

    store
        .take_shutdown_state()
        .expect("기존 서버 상태")
        .server_handle
        .expect("기존 서버 핸들")
        .abort();
    assert!(tokio::time::timeout(Duration::from_secs(1), first_receiver).await.unwrap().is_err());
}

#[test]
fn ide_서버_작업은_감독_범위에_등록된다() {
    let commands = include_str!("../src/domain/ide/commands.rs");
    let bind = commands.split_once("async fn bind_and_start(").unwrap().1;
    let bind = bind.split_once("#[tauri::command]").unwrap().0;

    assert!(bind.contains("spawn_transient_handle(\"ide-server\""));
    assert!(!bind.contains("tauri::async_runtime::spawn(async move {"));
}

#[tokio::test]
async fn ide_저장소는_종료_후_연결_등록을_취소한다() {
    const IDE_PORT: u32 = 1;

    let store = IdeStore::default();
    assert!(store
        .mark_started(
            IDE_PORT,
            "token".to_string(),
            std::path::PathBuf::from("/tmp/ide"),
            tauri::async_runtime::spawn(async {}),
        )
        .is_some());
    let (active_sender, active_receiver) = oneshot::channel::<()>();
    let active_handle = tauri::async_runtime::spawn(async move {
        let _sender = active_sender;
        pending::<()>().await;
    });
    assert!(store.register_connection(active_handle));

    let shutdown = store.take_shutdown_state().expect("IDE 서버 상태");
    for handle in shutdown.connection_handles {
        handle.abort();
    }
    assert!(tokio::time::timeout(Duration::from_secs(1), active_receiver)
        .await
        .unwrap()
        .is_err());

    let (sender, receiver) = oneshot::channel::<()>();
    let handle = tauri::async_runtime::spawn(async move {
        let _sender = sender;
        pending::<()>().await;
    });

    assert!(!store.register_connection(handle));
    assert!(tokio::time::timeout(Duration::from_secs(1), receiver).await.unwrap().is_err());
}

#[test]
fn ide_연결과_자식_작업은_함께_종료되는_범위에_등록된다() {
    let server = include_str!("../src/domain/ide/server.rs");
    let connection = server.split_once("async fn handle_connection(").unwrap().1;
    let connection = connection.split_once("pub async fn accept_loop(").unwrap().0;
    let accept = server.split_once("pub async fn accept_loop(").unwrap().1;
    let accept = accept.split_once("#[cfg(test)]").unwrap().0;

    assert!(connection.contains("JoinSet::new()"));
    assert!(connection.contains("connection_tasks.shutdown().await"));
    assert!(!connection.contains("tauri::async_runtime::spawn(async move {"));
    assert!(accept.contains("spawn_transient_handle(\"ide-connection\""));
    assert!(!accept.contains("tauri::async_runtime::spawn(async move {"));
}

#[test]
fn 원격_websocket_작업은_연결_종료_계약을_유지하며_감독된다() {
    let source = include_str!("../src/domain/remote/ws.rs");
    let socket = source.split_once("pub async fn handle_socket(").unwrap().1;
    let socket = socket.split_once("#[cfg(test)]").unwrap().0;

    assert!(socket.contains("spawn_transient_handle(\"remote-ws-writer\""));
    assert!(socket.contains("spawn_transient_handle(\"remote-ws-events\""));
    assert!(socket.contains("spawn_transient(\"remote-ws-request\""));
    assert!(socket.contains("REMOTE_WS_WRITER_SHUTDOWN_TIMEOUT_MS"));
    assert!(!socket.contains("tauri::async_runtime::spawn(async move {"));
}

#[test]
fn 원격_서버_종료_대기는_감독하고_등록_실패시_직접_취소한다() {
    let source = include_str!("../src/domain/remote/commands.rs");
    let stop = source.split_once("pub fn stop_server(").unwrap().1;
    let stop = stop.split_once("#[tauri::command]").unwrap().0;

    assert!(stop.contains("spawn_transient(\"remote-server-stop\""));
    assert!(stop.contains("abort_handle.abort()"));
    assert!(stop.contains("REMOTE_SHUTDOWN_GRACE_MS"));
    assert!(!stop.contains("tauri::async_runtime::spawn(async move {"));
}
