use std::sync::atomic::Ordering;

use taide_lsp::install::LspInstallStore;
use taide_model::lsp::LspServerId;

#[test]
fn lsp_설치_슬롯은_중복을_막고_취소와_drop_후_재진입을_허용한다() {
    let store = LspInstallStore::new();
    let server_id = LspServerId::from("test-server");
    let guard = store.begin(&server_id).expect("첫 설치는 슬롯을 얻는다");

    assert!(store.begin(&server_id).is_none());
    let cancellation_token = guard.cancellation_token();
    store.cancel(&server_id);
    assert!(cancellation_token.load(Ordering::SeqCst));

    drop(guard);
    assert!(store.begin(&server_id).is_some());
}

#[test]
fn 설치_adapter는_runtime_다운로드와_감독자를_주입하고_shutdown에서_입장을_닫는다() {
    let commands = include_str!("../src/domain/lsp/commands.rs");
    let command = commands.split_once("pub async fn lsp_install(").unwrap().1;
    let command = command.split_once("pub async fn lsp_install_cancel(").unwrap().0;
    assert!(command.contains("tasks: State<'_, TaskSupervisor>"));
    assert!(command.contains("&TauriEventSink(&app)"));
    assert!(command.contains("lsp_actions::lsp_install(&TauriEventSink(&app), &state, &install_store, &tasks, server_id).await"));
    let actions = include_str!("../../crates/taide-runtime/src/lsp_actions.rs");
    let action = actions.split_once("pub async fn lsp_install(").unwrap().1;
    assert!(action.contains("state.is_shutting_down()"));
    assert!(action.contains("install_store.shutdown()"));
    assert!(action.contains("install_store.is_stopped()"));
    assert!(action.contains("crate::lsp_install_actions::run_download_install("));
    assert!(action.contains("crate::lsp_install_toolchain::run_toolchain_install("));
    assert!(action.contains("&install_guard.lease()"));
    assert!(!commands.contains("tokio::task::spawn_blocking("));
    assert!(!commands.contains("tokio::process::Command::new("));
    assert!(!commands.contains("fn capture_output_tail("));

    let root = include_str!("../src/lib.rs");
    let exit = root.split_once(".run(move |app_handle, event|").unwrap().1;
    assert!(exit.contains("tauri::RunEvent::ExitRequested"));
    assert!(exit.contains("tauri::RunEvent::Exit"));
    let admission = exit.find("state::<LspInstallStore>().shutdown()").unwrap();
    let resources = exit.find("state::<TerminalStore>().shutdown()").unwrap();
    assert!(admission < resources);
    let drain = exit.split_once("if matches!(&event, tauri::RunEvent::Exit)").unwrap().1;
    assert!(exit.contains("api.prevent_exit()"));
    assert!(exit.contains("exit_drain.begin("));
    assert!(drain.contains("tauri::async_runtime::block_on("));
    let tasks = exit.find("state::<TaskSupervisor>().stop_all()").unwrap();
    let direct_drain = exit.find("exit_drain.wait_for_direct_exit(").unwrap();
    assert!(tasks < direct_drain);
    assert!(drain.contains("(*app_handle.state::<LspInstallStore>()).clone(),"));
    let coordinator = include_str!("../../crates/taide-runtime/src/exit_drain.rs");
    let direct = coordinator.split_once("pub async fn wait_for_direct_exit(").unwrap().1;
    assert!(direct.contains("Self::wait_for_owned_resources("));
    let tasks = coordinator.find("tasks.shutdown().await").unwrap();
    let slots = coordinator.find("installs.wait_for_idle().await").unwrap();
    let processes = coordinator.find("processes.wait_for_idle().await").unwrap();
    let terminals = coordinator.find("terminals.wait_for_idle().await").unwrap();
    let ready = coordinator.find("ready.store(true, Ordering::Release)").unwrap();
    let exit = coordinator.find("on_ready();").unwrap();
    assert!(tasks < slots && slots < processes && processes < terminals && terminals < ready && ready < exit);
}
