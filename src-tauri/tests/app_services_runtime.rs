use std::future::pending;
use std::path::Path;
use std::sync::Arc;

use taide_agent::store::HooksServerInfo;
use taide_model::agent::AgentActivity;
use taide_model::error::AppResult;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_runtime::{AppServices, AppState, PlatformServices, PlatformServicesState, RemoteDispatchLimiter, TaskSupervisor};

struct TestPlatform;

impl PlatformServices for TestPlatform {
    fn open_path(&self, _path: &Path) -> AppResult<()> {
        Ok(())
    }

    fn reveal_item_in_dir(&self, _path: &Path) -> AppResult<()> {
        Ok(())
    }

    fn open_url(&self, _url: &str) -> AppResult<()> {
        Ok(())
    }

    fn send_notification(&self, _title: &str, _body: &str) -> AppResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn 앱_서비스와_기존_상태_복제본은_같은_인스턴스를_공유한다() {
    let services = Arc::new(AppServices::new(
        AppState::new(AppPaths::new(std::env::temp_dir())),
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        RemoteDispatchLimiter::new(1),
        PlatformServicesState::new(Arc::new(TestPlatform)),
    ));
    let legacy_state = services.state.clone();
    let legacy_search = services.search.clone();
    let legacy_ai_requests = services.ai_requests.clone();
    let legacy_tree = services.tree.clone();
    let legacy_plugin = services.plugin.clone();
    let legacy_agent_hooks = services.agent_hooks.clone();
    let legacy_windows = services.windows.clone();
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

    let hook_info = HooksServerInfo {
        port: 1,
        token: "token".to_string(),
    };
    legacy_agent_hooks.set_server(hook_info, tokio::spawn(pending()));
    assert_eq!(services.agent_hooks.server_info().expect("공유 hook 서버").port, 1);
    let hook_project = ProjectId::new();
    legacy_agent_hooks.set_project_override(hook_project.clone(), "codex".to_string(), AgentActivity::AwaitingInput);
    assert_eq!(
        services.agent_hooks.fresh_project_override(&hook_project, "codex"),
        Some(AgentActivity::AwaitingInput)
    );
    services.agent_hooks.take_server().expect("공유 hook 핸들").abort();

    let window_project_id = ProjectId::new();
    legacy_windows.register("editor-1".to_string(), window_project_id.clone(), 1);
    assert_eq!(services.windows.label_for(&window_project_id, 1).as_deref(), Some("editor-1"));
    assert_eq!(services.windows.forget("editor-1"), Some((window_project_id, 1)));

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
    assert!(setup.contains("app.manage(services.agent_hooks.clone());"));
    assert!(setup.contains("app.manage(services.lsp.clone());"));
    assert!(setup.contains("app.manage(services.lsp_install.clone());"));
    assert!(setup.contains("app.manage(services.system_usage.clone());"));
    assert!(setup.contains("app.manage(services.remote_dispatch_limiter.clone());"));
    assert!(setup.contains("app.manage(services.platform.clone());"));
    assert!(setup.contains("app.manage(services.windows.clone());"));
    assert!(setup.contains("app.manage(services.tasks.clone());"));
    assert!(setup.contains("app.manage(services);"));
}
