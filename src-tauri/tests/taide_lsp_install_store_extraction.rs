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
    assert!(command.contains("state.is_shutting_down()"));
    assert!(command.contains("install_store.shutdown()"));
    assert!(command.contains("install_store.is_stopped()"));
    assert!(command.contains("taide_runtime::lsp_install_actions::run_download_install("));
    assert!(command.contains("&TauriEventSink(&app)"));
    assert!(command.contains("&install_guard.lease()"));
    assert!(!commands.contains("tokio::task::spawn_blocking("));

    let root = include_str!("../src/lib.rs");
    let exit = root.split_once(".run(|app_handle, event|").unwrap().1;
    assert!(exit.contains("tauri::RunEvent::ExitRequested"));
    assert!(exit.contains("tauri::RunEvent::Exit"));
    let admission = exit.find("state::<LspInstallStore>().shutdown()").unwrap();
    let resources = exit.find("state::<TerminalStore>().kill_all()").unwrap();
    assert!(admission < resources);
}
