use std::future::pending;
use std::path::Path;
use std::sync::{Arc, Mutex};

use taide_agent::store::HooksServerInfo;
use taide_infra::secret::test_support::InMemorySecretStore;
use taide_infra::secret::{SecretAccount, SecretStoreState};
use taide_model::agent::AgentActivity;
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_runtime::{
    AppServices, AppState, EventSink, IdeSaveFile, PlatformServices, PlatformServicesState, RemoteDispatchLimiter, TaskSupervisor,
};

#[derive(Default)]
struct TestPlatform(Mutex<Vec<AppEvent>>);

impl EventSink for TestPlatform {
    fn publish(&self, event: AppEvent) {
        self.0.lock().expect("이벤트 기록 잠금").push(event);
    }
}

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

fn test_ide_save(state: &AppState, path: &Path, content: &str) -> AppResult<()> {
    assert!(state.is_shutting_down());
    assert_eq!(path, Path::new("fixture-path"));
    assert_eq!(content, "fixture-content");
    Ok(())
}

#[tokio::test]
async fn 앱_서비스와_기존_상태_복제본은_같은_인스턴스를_공유한다() {
    let secret_port = SecretStoreState(Arc::new(InMemorySecretStore::default()));
    let injected_secret_port = secret_port.0.clone();
    let platform = Arc::new(TestPlatform::default());
    let event_port: Arc<dyn EventSink> = platform.clone();
    let services = Arc::new(AppServices::new(
        AppState::new(AppPaths::new(std::env::temp_dir())),
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        RemoteDispatchLimiter::new(1),
        PlatformServicesState::new(platform.clone()),
        secret_port,
        IdeSaveFile(test_ide_save),
        event_port.clone(),
    ));
    let legacy_state = services.state.clone();
    let legacy_search = services.search.clone();
    let legacy_ai_requests = services.ai_requests.clone();
    let legacy_tree = services.tree.clone();
    let legacy_plugin = services.plugin.clone();
    let legacy_agents = services.agents.clone();
    let legacy_git = services.git.clone();
    let legacy_remote = services.remote.clone();
    let legacy_ide = services.ide.clone();
    let legacy_secrets = services.secrets.clone();
    let legacy_ide_save_file = services.ide_save_file.clone();
    let shared_events = services.events.clone();
    let legacy_agent_hooks = services.agent_hooks.clone();
    let legacy_windows = services.windows.clone();
    let legacy_tasks = services.tasks.clone();
    let legacy_terminal = services.terminal.clone();
    let terminal_lease = legacy_terminal.begin_spawn().expect("공유 터미널 입장");
    services.terminal.shutdown();
    assert!(legacy_terminal.begin_spawn().is_err());
    drop(terminal_lease);
    services.terminal.wait_for_idle().await.expect("공유 터미널 대기");

    legacy_state.begin_shutdown();
    assert!(services.state.is_shutting_down());
    (legacy_ide_save_file.0)(&services.state, Path::new("fixture-path"), "fixture-content").expect("주입 저장 포트");
    (services.ide_save_file.0)(&legacy_state, Path::new("fixture-path"), "fixture-content").expect("복제 저장 포트");
    assert!(Arc::ptr_eq(&services.events, &event_port));
    let first_event = AppEvent::ThemeChanged {
        theme_id: "fixture-theme".to_string(),
    };
    let second_event = AppEvent::GitStatusChanged {
        project_id: ProjectId::new(),
    };
    shared_events.publish(first_event.clone());
    services.events.publish(second_event.clone());
    assert_eq!(
        platform.0.lock().expect("공유 이벤트 기록").as_slice(),
        &[first_event, second_event]
    );

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

    legacy_agents.register_wait_marker("marker".to_string());
    assert_eq!(services.agents.take_all_markers(), vec!["marker".to_string()]);
    assert!(legacy_agents.take_all_markers().is_empty());

    let git_root = std::path::PathBuf::from("shared-repo");
    assert!(Arc::ptr_eq(
        &legacy_git.push_fetch_lock(&git_root),
        &services.git.push_fetch_lock(&git_root),
    ));

    legacy_remote.set_password_configured(true);
    assert!(services.remote.status().password_configured);
    let session = legacy_remote.issue_session_without_nonce();
    assert!(services.remote.has_active_session(&session));
    services.remote.revoke_all_sessions();
    assert!(!legacy_remote.has_active_session(&session));

    legacy_ide.publish_diagnostics(project_id.clone(), Vec::new());
    assert_eq!(services.ide.diagnostics(None), Some(Vec::new()));
    let mut ide_notifications = services.ide.subscribe();
    legacy_ide.broadcast("notification".to_string());
    assert_eq!(ide_notifications.recv().await.expect("공유 IDE 알림"), "notification");

    assert!(Arc::ptr_eq(&legacy_secrets.0, &injected_secret_port));
    legacy_secrets.0.set(SecretAccount::AiCodex, "fixture-value").expect("메모리 저장");
    assert_eq!(
        services.secrets.0.get(SecretAccount::AiCodex).expect("메모리 조회").as_deref(),
        Some("fixture-value")
    );
    services.secrets.0.delete(SecretAccount::AiCodex).expect("메모리 제거");
    assert!(legacy_secrets.0.get(SecretAccount::AiCodex).expect("메모리 공유 제거").is_none());

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
    assert!(setup.contains("let platform = Arc::new(TauriPlatformServices(app.handle().clone()));"));
    assert!(setup.contains("PlatformServicesState::new(platform.clone())"));
    assert!(setup.contains("app.manage(services.state.clone());"));
    assert!(setup.contains("app.manage(services.search.clone());"));
    assert!(setup.contains("app.manage(services.ai_requests.clone());"));
    assert!(setup.contains("app.manage(services.tree.clone());"));
    assert!(setup.contains("app.manage(services.terminal.clone());"));
    assert!(setup.contains("app.manage(services.plugin.clone());"));
    assert!(setup.contains("app.manage(services.agents.clone());"));
    assert!(setup.contains("app.manage(services.git.clone());"));
    assert!(setup.contains("app.manage(services.remote.clone());"));
    assert!(setup.contains("app.manage(services.ide.clone());"));
    assert!(setup.contains("app.manage(services.ide_save_file.clone());"));
    assert!(setup.contains("IdeSaveFile(save_ide_diff_file)"));
    assert!(setup.contains("app.manage(services.secrets.clone());"));
    assert!(setup.contains("SecretStoreState::new(app.config().identifier.clone())"));
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
