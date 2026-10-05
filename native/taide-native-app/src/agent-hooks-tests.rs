use std::path::PathBuf;
use std::sync::Mutex;

use taide_agent::constants::{AGENT_NAME_GEMINI, AGENT_NAME_PI};
use taide_agent::hook_files;
use taide_model::agent::{AgentActivity, DetectedAgent};
use taide_model::error::AppErrorKind;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_runtime::AppState;

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const TOKEN_BYTES: usize = 32;
const SYNTHETIC_PID: u32 = 42;
const ACCEPTED_TASKS: usize = 2;

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    home: String,
    root: String,
    project: ProjectId,
    services: Arc<AppServices>,
    sink: Arc<Sink>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-hook-{}", ProjectId::new()));
        let root = directory.join("project").to_str().unwrap().to_string();
        let home = directory.join("home").to_str().unwrap().to_string();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.clone(),
                name: "synthetic hook project".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        Self {
            directory,
            home,
            root,
            project,
            services,
            sink,
        }
    }

    fn body(&self, event: &str) -> String {
        serde_json::json!({"hook_event_name": event, "cwd": self.root}).to_string()
    }

    async fn finish(&self) {
        stop(&self.services);
        self.services.tasks.shutdown().await;
        assert!(self.services.agent_hooks.server_info().is_none());
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        stop(&self.services);
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

async fn request(port: u16, path: &str, body: &str) -> String {
    let stream = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();
    request_on(stream, path, body).await
}

async fn request_on(mut stream: TcpStream, path: &str, body: &str) -> String {
    let headers = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        body.len(),
    );
    stream.write_all(headers.as_bytes()).await.unwrap();
    stream.write_all(body.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}

fn assert_status(response: &str, status: &str) {
    assert_eq!(
        response,
        format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
    );
}

#[tokio::test]
async fn native_agent_hooks는_실제_loopback_인증_상태_이벤트와_stop_재시작을_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let services = &fixture.services;
        let server = ensure_started(services.clone()).await.unwrap();
        assert_eq!(server.token.len(), TOKEN_BYTES);
        assert!(server.token.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(server.port, 0);
        let cached = ensure_started(services.clone()).await.unwrap();
        assert_eq!(cached.port, server.port);
        assert!(cached.token == server.token);

        let body = fixture.body("BeforeAgent");
        for path in [
            HOOKS_HTTP_PATH.to_string(),
            format!("{HOOKS_HTTP_PATH}?token=wrong"),
            format!("/wrong?token={}", server.token),
        ] {
            assert_status(&request(server.port, &path, &body).await, "403 Forbidden");
        }
        assert!(fixture.sink.0.lock().unwrap().is_empty());
        assert_eq!(
            services
                .agent_hooks
                .fresh_project_override(&fixture.project, AGENT_NAME_GEMINI),
            None,
        );

        let authenticated = format!("{HOOKS_HTTP_PATH}?token={}", server.token);
        assert_status(
            &request(server.port, &authenticated, "{ synthetic-invalid-json").await,
            "400 Bad Request",
        );
        assert_status(
            &request(
                server.port,
                &authenticated,
                &fixture.body("UserPromptSubmit"),
            )
            .await,
            "200 OK",
        );
        assert_eq!(
            services
                .agent_hooks
                .fresh_project_override(&fixture.project, AGENT_NAME_CLAUDE),
            Some(AgentActivity::Working),
        );

        services.agents.diff(
            &fixture.project,
            &[DetectedAgent {
                session_id: "synthetic-gemini-session".into(),
                name: AGENT_NAME_GEMINI.into(),
                pid: SYNTHETIC_PID,
                activity: AgentActivity::Idle,
                blocked_reason: None,
            }],
        );
        let gemini_path = format!("{authenticated}&agent={AGENT_NAME_GEMINI}");
        assert_status(&request(server.port, &gemini_path, &body).await, "200 OK");
        assert_eq!(
            services.agents.agents_for(&fixture.project)[0].activity,
            AgentActivity::Working
        );
        {
            let events = fixture.sink.0.lock().unwrap();
            let [AppEvent::AgentStateChanged { project_id, agents }] = events.as_slice() else {
                panic!("one actual agent state event expected");
            };
            assert_eq!(project_id, &fixture.project);
            assert_eq!(agents, &services.agents.agents_for(&fixture.project));
        }
        assert_status(&request(server.port, &gemini_path, &body).await, "200 OK");
        assert_eq!(fixture.sink.0.lock().unwrap().len(), 1);

        let pending = TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, server.port))
            .await
            .unwrap();
        while services.tasks.tracked_count() < ACCEPTED_TASKS {
            tokio::task::yield_now().await;
        }
        stop(services);
        assert!(services.agent_hooks.server_info().is_none());
        assert_eq!(
            services
                .agent_hooks
                .fresh_project_override(&fixture.project, AGENT_NAME_GEMINI),
            None,
        );
        assert_status(
            &request_on(pending, &gemini_path, &body).await,
            "503 Service Unavailable",
        );

        let restarted = ensure_started(services.clone()).await.unwrap();
        assert!(restarted.token != server.token);
        assert_status(
            &request(restarted.port, &gemini_path, &body).await,
            "403 Forbidden",
        );
        fixture.finish().await;

        let error = ensure_started(services.clone()).await.err().unwrap();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        assert_eq!(services.tasks.tracked_count(), 0);

        let closing = Fixture::new();
        closing.services.state.begin_shutdown();
        let error = ensure_started(closing.services.clone())
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        closing.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_agent_hooks_toggle은_합성_home의_설치된_hook만_갱신하고_실제_서버를_끈다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let services = &fixture.services;
        let home = fixture.home.clone();
        apply_toggle_with_home(services.clone(), false, false, || {
            panic!("unchanged must not resolve home")
        })
        .await;
        assert!(services.agent_hooks.server_info().is_none());

        let path = service::user_level_hooks_path(AGENT_NAME_GEMINI, Some(&home)).unwrap();
        let original = service::inject_taide_command_hook_entries(
            serde_json::json!({"synthetic-user": "preserve"}),
            service::managed_hook_events_for(AGENT_NAME_GEMINI),
            "taide hook --url http://127.0.0.1:1/claude/hook?token=synthetic&taide=1",
            service::user_level_hook_command_timeout(AGENT_NAME_GEMINI),
        );
        hook_files::write_user_level_hooks(&path, &original).unwrap();
        let owned_path = service::user_level_hooks_path(AGENT_NAME_PI, Some(&home)).unwrap();
        hook_files::write_owned_hook_file(&owned_path, "synthetic-user-owned-file").unwrap();

        services.state.settings.write().agent_hooks_enabled = true;
        apply_toggle_with_home(services.clone(), false, true, || Some(home.clone())).await;
        let server = services.agent_hooks.server_info().unwrap();
        let updated = hook_files::read_user_level_hooks(&path).unwrap();
        let url = service::build_hook_url(&server, AGENT_NAME_GEMINI);
        let expected = service::inject_taide_command_hook_entries(
            original,
            service::managed_hook_events_for(AGENT_NAME_GEMINI),
            &service::build_command_hook_shell_command(TAIDE_CLI_TARGET_PATH, &url),
            service::user_level_hook_command_timeout(AGENT_NAME_GEMINI),
        );
        assert_eq!(updated, expected);
        assert_eq!(updated["synthetic-user"], "preserve");
        assert_eq!(
            std::fs::read_to_string(&owned_path).unwrap(),
            "synthetic-user-owned-file"
        );
        assert!(
            !PathBuf::from(&fixture.root)
                .join(".claude/settings.local.json")
                .exists()
        );
        assert_status(
            &request(
                server.port,
                &format!(
                    "{HOOKS_HTTP_PATH}?token={}&agent={AGENT_NAME_GEMINI}",
                    server.token
                ),
                &fixture.body("BeforeAgent"),
            )
            .await,
            "200 OK",
        );
        apply_toggle_with_home(services.clone(), true, true, || {
            panic!("unchanged must not resolve home")
        })
        .await;
        assert!(services.agent_hooks.server_info().unwrap().token == server.token);

        services.state.settings.write().agent_hooks_enabled = false;
        apply_toggle_with_home(services.clone(), true, false, || Some(home)).await;
        assert!(services.agent_hooks.server_info().is_none());
        let removed = hook_files::read_user_level_hooks(&path).unwrap();
        assert_eq!(removed["synthetic-user"], "preserve");
        assert!(!service::has_taide_marker_anywhere(&removed));
        assert_eq!(
            std::fs::read_to_string(&owned_path).unwrap(),
            "synthetic-user-owned-file"
        );
        fixture.finish().await;
    })
    .await
    .unwrap();
}
