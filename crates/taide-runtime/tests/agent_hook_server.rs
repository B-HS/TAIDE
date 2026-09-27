use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use taide_agent::store::{AgentHooksStore, HooksServerInfo};
use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_model::error::{AppError, AppErrorKind};
use taide_runtime::agent_hook_server::{start_hooks_server, stop_hooks_server};
use taide_runtime::{ExitDrain, TaskSupervisor};
use taide_terminal::store::TerminalStore;
use tokio::sync::oneshot;

const FIRST_PORT: u16 = 12345;
const SECOND_PORT: u16 = 12346;
const FIXTURE_TIMEOUT_MS: u64 = 2_000;
const PENDING_OWNER_MS: u64 = 20;

fn server(port: u16, token: &str) -> HooksServerInfo {
    HooksServerInfo {
        port,
        token: token.to_string(),
    }
}

fn tasks() -> TaskSupervisor {
    TaskSupervisor::new(tokio::runtime::Handle::current())
}

#[tokio::test]
async fn cached_서버는_닫힌_감독자에서도_기존_정보를_반환하고_모든_port를_건너뛴다() {
    let store = AgentHooksStore::new();
    let tasks = tasks();
    let existing = server(FIRST_PORT, "first");
    let handle = tasks
        .spawn_transient_handle("fixture-existing", async { std::future::pending::<()>().await })
        .unwrap();
    store.set_server(existing.clone(), handle);
    tasks.stop_all();
    let result = start_hooks_server(
        &store,
        &tasks,
        || async { panic!("cached bind") },
        |()| panic!("cached accept"),
        || panic!("cached shutdown check"),
    )
    .await
    .unwrap();
    assert_eq!(result.port, existing.port);
    assert_eq!(result.token, existing.token);
    stop_hooks_server(&store);
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
        .await
        .unwrap();
    assert!(store.server_info().is_none());
}

#[tokio::test]
async fn uncached_닫힌_감독자는_bind_accept_shutdown_port를_호출하지_않는다() {
    let store = AgentHooksStore::new();
    let tasks = tasks();
    tasks.stop_all();
    let error = start_hooks_server(
        &store,
        &tasks,
        || async { panic!("closed bind") },
        |()| panic!("closed accept"),
        || panic!("closed shutdown check"),
    )
    .await
    .err()
    .expect("닫힌 감독자에서는 실패해야 한다");
    assert_eq!(error.kind(), AppErrorKind::Internal);
    assert!(store.server_info().is_none());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn bind_실패는_accept_store_shutdown을_건드리지_않고_owner를_반납한다() {
    let store = AgentHooksStore::new();
    let tasks = tasks();
    let error = start_hooks_server(
        &store,
        &tasks,
        || async { Err::<(HooksServerInfo, ()), _>(AppError::Internal("fixture bind failed".to_string())) },
        |()| panic!("failed bind accept"),
        || panic!("failed bind shutdown check"),
    )
    .await
    .err()
    .expect("bind 실패를 반환해야 한다");
    assert_eq!(error.to_string(), "operation failed: fixture bind failed");
    assert_eq!(tasks.tracked_count(), 0);
    assert!(store.server_info().is_none());
}

struct Binding(Arc<AtomicUsize>);

impl Drop for Binding {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn accept_등록_실패는_바인딩을_drop하고_store를_비운다() {
    let store = AgentHooksStore::new();
    let tasks = tasks();
    let dropped = Arc::new(AtomicUsize::new(0));
    let error = start_hooks_server(
        &store,
        &tasks,
        || async { Ok((server(FIRST_PORT, "candidate"), Binding(dropped.clone()))) },
        |binding| {
            drop(binding);
            None
        },
        || panic!("failed registration shutdown check"),
    )
    .await
    .err()
    .expect("등록 실패를 반환해야 한다");
    assert_eq!(error.to_string(), "operation failed: hook server task supervisor stopped");
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
    assert_eq!(tasks.tracked_count(), 0);
    assert!(store.server_info().is_none());
}

#[tokio::test]
async fn shutdown_검사_실패는_후보_accept를_취소하고_store에_저장하지_않는다() {
    let store = AgentHooksStore::new();
    let tasks = tasks();
    let error = start_hooks_server(
        &store,
        &tasks,
        || async { Ok((server(FIRST_PORT, "candidate"), ())) },
        |()| tasks.spawn_transient_handle("fixture-accept", async { std::future::pending::<()>().await }),
        || true,
    )
    .await
    .err()
    .expect("종료 중 시작을 거절해야 한다");
    assert_eq!(error.to_string(), "operation failed: hook server unavailable during shutdown");
    assert!(store.server_info().is_none());
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn 중복_bind는_첫_서버를_유지하고_뒤늦은_accept를_취소하며_stop은_override를_비운다() {
    let store = AgentHooksStore::new();
    let tasks = tasks();
    let first = server(FIRST_PORT, "first");
    let second = server(SECOND_PORT, "second");
    let result = start_hooks_server(
        &store,
        &tasks,
        || async {
            let first_handle = tasks
                .spawn_transient_handle("fixture-first", async { std::future::pending::<()>().await })
                .unwrap();
            store.set_server(first.clone(), first_handle);
            Ok((second, ()))
        },
        |()| tasks.spawn_transient_handle("fixture-second", async { std::future::pending::<()>().await }),
        || false,
    )
    .await
    .unwrap();
    assert_eq!(result.token, "first");
    assert_eq!(store.server_info().unwrap().port, FIRST_PORT);
    let id = taide_model::ids::ProjectId::new();
    store.set_project_override(id.clone(), "gemini".to_string(), taide_model::agent::AgentActivity::Working);
    stop_hooks_server(&store);
    assert!(store.server_info().is_none());
    assert_eq!(store.fresh_project_override(&id, "gemini"), None);
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn bind_await의_caller_취소까지_정상_root가_operation을_기다린다() {
    let store = AgentHooksStore::new();
    let runtime = tokio::runtime::Handle::current();
    let tasks = tasks();
    let request_store = store.clone();
    let request_tasks = tasks.clone();
    let (started, started_rx) = oneshot::channel();
    let request = tokio::spawn(async move {
        start_hooks_server(
            &request_store,
            &request_tasks,
            || async move {
                started.send(()).ok();
                std::future::pending::<Result<(HooksServerInfo, ()), AppError>>().await
            },
            |()| panic!("cancelled bind accept"),
            || panic!("cancelled bind shutdown check"),
        )
        .await
    });
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), started_rx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tasks.tracked_count(), 1);
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
        },
    ));
    assert!(tokio::time::timeout(Duration::from_millis(PENDING_OWNER_MS), &mut ready_rx)
        .await
        .is_err());
    assert!(!drain.is_ready());
    request.abort();
    assert!(request.await.is_err_and(|error| error.is_cancelled()));
    tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), ready_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(drain.is_ready());
    assert!(store.server_info().is_none());
    assert_eq!(tasks.tracked_count(), 0);
}

#[test]
fn native는_loopback_bind_accept_connection을_유지하고_runtime에_같은_store_supervisor를_전달한다() {
    let native = include_str!("../../../src-tauri/src/domain/agent/hooks.rs");
    assert!(native.contains("TcpListener::bind(\"127.0.0.1:0\")"));
    assert!(native.contains("spawn_transient_handle(\"agent-hooks-accept\""));
    assert!(native.contains("spawn_transient(\"agent-hooks-connection\""));
    assert!(native.contains("agent_hook_server::start_hooks_server("));
    assert!(native.contains("agent_hook_server::stop_hooks_server(&store)"));
    let runtime = include_str!("../src/agent_hook_server.rs");
    for step in [
        "store.server_info()",
        ".begin_operation(",
        "bind().await",
        "start_accept(binding)",
        "is_shutting_down()",
        "store.set_server(info, accept_handle)",
    ] {
        assert!(runtime.contains(step));
    }
}
