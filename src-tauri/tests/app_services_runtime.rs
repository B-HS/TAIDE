use std::future::pending;
use std::sync::Arc;

use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_runtime::{AppServices, AppState, TaskSupervisor};

#[tokio::test]
async fn 앱_서비스와_기존_상태_복제본은_같은_인스턴스를_공유한다() {
    let services = Arc::new(AppServices::new(
        AppState::new(AppPaths::new(std::env::temp_dir())),
        TaskSupervisor::new(tokio::runtime::Handle::current()),
    ));
    let legacy_state = services.state.clone();
    let legacy_search = services.search.clone();
    let legacy_ai_requests = services.ai_requests.clone();
    let legacy_tree = services.tree.clone();
    let legacy_plugin = services.plugin.clone();
    let legacy_tasks = services.tasks.clone();

    legacy_state.begin_shutdown();
    assert!(services.state.is_shutting_down());

    let cancelled = legacy_search.begin("main", "panel");
    services.search.cancel("main", "panel");
    assert!(cancelled.load(std::sync::atomic::Ordering::SeqCst));

    let (_token, receiver) = legacy_ai_requests.begin("main", "req-1").expect("first request");
    services.ai_requests.cancel("main", "req-1");
    assert!(receiver.await.is_ok());

    let project_id = ProjectId::new();
    legacy_tree
        .0
        .write()
        .insert(project_id.clone(), taide_tree::service::new_tree_state(std::env::temp_dir()));
    services.tree.remove(&project_id);
    assert!(!legacy_tree.0.read().contains_key(&project_id));

    *legacy_plugin.0.write() = Some(Vec::new());
    assert!(services.plugin.0.read().is_some());

    assert!(legacy_tasks.spawn("shared-test", pending()));
    assert_eq!(services.tasks.tracked_count(), 1);
    services.tasks.stop_all();
}

#[test]
fn 앱_조립은_같은_서비스_복제본을_기존_상태에_등록한다() {
    let setup = include_str!("../src/lib.rs");

    assert!(setup.contains("let services = Arc::new(AppServices::new("));
    assert!(setup.contains("app.manage(services.state.clone());"));
    assert!(setup.contains("app.manage(services.search.clone());"));
    assert!(setup.contains("app.manage(services.ai_requests.clone());"));
    assert!(setup.contains("app.manage(services.tree.clone());"));
    assert!(setup.contains("app.manage(services.terminal.clone());"));
    assert!(setup.contains("app.manage(services.plugin.clone());"));
    assert!(setup.contains("app.manage(services.lsp.clone());"));
    assert!(setup.contains("app.manage(services.lsp_install.clone());"));
    assert!(setup.contains("app.manage(services.system_usage.clone());"));
    assert!(setup.contains("app.manage(services.tasks.clone());"));
    assert!(setup.contains("app.manage(services);"));
}
