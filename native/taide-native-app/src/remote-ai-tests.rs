use std::collections::BTreeSet;
use std::future::{Future, poll_fn};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::Poll;
use std::time::Duration;

use axum::Router;
use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::routing::{get, post};
use serde_json::json;
use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::ai::AiProviderId;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::paths::AppPaths;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{
    AppState, EventSink, IdeSaveFile, PlatformServicesState, RemoteDispatchLimiter, TaskSupervisor,
    save_file_within_open_projects,
};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use super::*;
use crate::remote_gateway;
use crate::remote_ws::ChannelFactory;

const COMMAND_COUNT: usize = 6;
const REMOTE_CONCURRENT: usize = 128;
const DEADLINE: Duration = Duration::from_secs(3);
const CAPTURE_CAPACITY: usize = 8;
const BODY_LIMIT: usize = 256 * 1024;
const COMPLETE_TOKENS: usize = 256;
const INSTRUCT_TOKENS: usize = 4096;
const MODEL: &str = "synthetic-chat";
const PREFIX: &str = "synthetic-before";
const SUFFIX: &str = "synthetic-after";
const SELECTION: &str = "synthetic-selection";
const INSTRUCTION: &str = "synthetic-instruction";
const DIFF: &str = "synthetic-diff";
const RECENT: &str = "synthetic-recent";
const TEXT: &str = "  synthetic-result\n";

#[derive(Default)]
struct Secrets {
    reads: Mutex<Vec<SecretAccount>>,
    failed: AtomicBool,
}

impl SecretStore for Secrets {
    fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
        panic!("remote AI must not write credentials");
    }

    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        self.reads.lock().unwrap().push(account);
        if self.failed.load(Ordering::Acquire) {
            return Err(AppError::Internal("synthetic store failure".into()));
        }
        Ok(None)
    }

    fn delete(&self, _: SecretAccount) -> AppResult<()> {
        panic!("remote AI must not delete credentials");
    }
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        panic!("AI action unexpectedly published {event:?}");
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    secrets: Arc<Secrets>,
    backend: Dispatch,
}

impl Fixture {
    fn new(base_url: Option<String>) -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-ai-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        {
            let mut settings = state.settings.write();
            settings.ai_provider = Some(AiProviderId::Omlx);
            settings.ai_model = Some(MODEL.into());
            settings.ai_omlx_base_url = base_url;
        }
        let secrets = Arc::new(Secrets::default());
        let services = Arc::new(AppServices::new(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            RemoteDispatchLimiter::new(REMOTE_CONCURRENT),
            PlatformServicesState::new(Arc::new(crate::bootstrap::NativePlatform)),
            SecretStoreState(secrets.clone()),
            IdeSaveFile(save_file_within_open_projects),
            Arc::new(Sink),
        ));
        let backend = remote_gateway::with_policy(extend_backend(Dispatch {
            json: Arc::new(|_, name, args, _| {
                Box::pin(async move {
                    assert_eq!(name, "settings_get");
                    Ok(args.to_string())
                })
            }),
            raw: Arc::new(|_, name, _| {
                Box::pin(async move {
                    assert_eq!(name, "file_read_raw");
                    Ok(vec![0])
                })
            }),
        }));
        Self {
            directory,
            services,
            secrets,
            backend,
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory = Arc::new(|_| panic!("AI does not create channels"));
        let result = tokio::time::timeout(
            DEADLINE,
            (self.backend.json)(self.services.clone(), name.into(), args, channels),
        )
        .await
        .unwrap()?;
        Ok(serde_json::from_str(&result).unwrap())
    }

    async fn finish(&self) {
        self.services.ai_requests.shutdown();
        tokio::time::timeout(DEADLINE, self.services.ai_requests.wait_for_idle())
            .await
            .unwrap();
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.ai_requests.shutdown();
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn request(command: &str, id: &str) -> Value {
    match command {
        "ai_inline_complete" => json!({"request":{
            "requestId":id,"owner":"main","provider":"omlx","model":MODEL,
            "prefix":PREFIX,"suffix":SUFFIX,"language":"rust","filePath":"synthetic.rs",
        }}),
        "ai_inline_edit" => json!({"request":{
            "requestId":id,"owner":"main","selection":SELECTION,"instruction":INSTRUCTION,
            "prefix":PREFIX,"suffix":SUFFIX,"language":"rust","filePath":"synthetic.rs",
        }}),
        "ai_commit_message" => json!({"request":{
            "requestId":id,"owner":"main","diffText":DIFF,"recentCommits":RECENT,
        }}),
        _ => panic!("unexpected test command {command}"),
    }
}

fn chat_response(text: &str, finish_reason: &str) -> Value {
    json!({"choices":[{"message":{"content":text},"finish_reason":finish_reason}]})
}

struct Captured {
    path: String,
    body: Value,
}

struct ServerState {
    captured: mpsc::Sender<Captured>,
    release: watch::Sender<bool>,
    reply: Mutex<(StatusCode, Value)>,
    active: AtomicUsize,
}

struct Active(Arc<ServerState>);

impl Drop for Active {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}

async fn handler(
    State(state): State<Arc<ServerState>>,
    request: Request,
) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    state.active.fetch_add(1, Ordering::AcqRel);
    let _active = Active(state.clone());
    assert!(!request.headers().contains_key(header::AUTHORIZATION));
    let path = request.uri().path().to_owned();
    let bytes = to_bytes(request.into_body(), BODY_LIMIT).await.unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    state
        .captured
        .send(Captured {
            path: path.clone(),
            body,
        })
        .await
        .unwrap();
    let mut release = state.release.subscribe();
    while !*release.borrow_and_update() {
        release.changed().await.unwrap();
    }
    let (status, body) = if path == "/v1/models" {
        (StatusCode::OK, json!({"data":[{"id":MODEL}]}))
    } else {
        state.reply.lock().unwrap().clone()
    };
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}

struct Server {
    url: String,
    state: Arc<ServerState>,
    captured: mpsc::Receiver<Captured>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<std::io::Result<()>>>,
}

impl Server {
    async fn start(blocked: bool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (captured, receiver) = mpsc::channel(CAPTURE_CAPACITY);
        let (release, _) = watch::channel(!blocked);
        let state = Arc::new(ServerState {
            captured,
            release,
            reply: Mutex::new((StatusCode::OK, chat_response(TEXT, "stop"))),
            active: AtomicUsize::new(0),
        });
        let router = Router::new()
            .route("/v1/models", get(handler))
            .route("/v1/chat/completions", post(handler))
            .with_state(state.clone());
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = stopped.await;
                })
                .await
        });
        Self {
            url,
            state,
            captured: receiver,
            stop: Some(stop),
            task: Some(task),
        }
    }

    async fn next(&mut self) -> Captured {
        tokio::time::timeout(DEADLINE, self.captured.recv())
            .await
            .unwrap()
            .unwrap()
    }

    async fn finish(&mut self) {
        self.state.release.send_replace(true);
        self.stop.take().unwrap().send(()).unwrap();
        tokio::time::timeout(DEADLINE, self.task.take().unwrap())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(self.state.active.load(Ordering::Acquire), 0);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.state.release.send_replace(true);
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

#[tokio::test]
async fn catalog_정책_타입_remaining과_합성_secret_status를_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMAND_COUNT);
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    let previous: BTreeSet<_> = [
        crate::remote_preferences::COMMANDS,
        crate::remote_ide::COMMANDS,
        crate::remote_files::JSON_COMMANDS,
        crate::remote_layout::COMMANDS,
        crate::remote_projects::COMMANDS,
        crate::remote_search::COMMANDS,
        crate::remote_plugins::COMMANDS,
        crate::remote_git::COMMANDS,
        crate::remote_agents::COMMANDS,
        crate::remote_lsp::COMMANDS,
        crate::remote_terminal::COMMANDS,
        crate::remote_utilities::COMMANDS,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();
    assert!(declared.is_disjoint(&previous));
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-ai.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new(None);
    fixture.services.state.begin_shutdown();
    assert_eq!(
        fixture.call("ai_token_status", json!({})).await.unwrap(),
        json!({"ollamaCloud":false,"codex":false,"omlx":false})
    );
    assert_eq!(
        *fixture.secrets.reads.lock().unwrap(),
        [SecretAccount::AiOllamaCloud, SecretAccount::AiCodex]
    );
    for provider in ["ollamaCloud", "codex", "omlx"] {
        assert_eq!(
            fixture
                .call("ai_list_models", json!({"provider":provider}))
                .await
                .unwrap_err()["code"],
            "InvalidArgument"
        );
    }
    for (name, args) in [
        ("ai_list_models", json!({"provider":"invalid"})),
        ("ai_inline_complete", json!({"request":{}})),
        ("ai_inline_edit", json!({"request":{}})),
        ("ai_commit_message", json!({"request":{}})),
        ("ai_request_cancel", json!({})),
    ] {
        assert_eq!(
            fixture.call(name, args).await.unwrap_err()["code"],
            "InvalidArgument"
        );
    }
    for name in ["ai_set_token", "ai_clear_token", "sync_connect"] {
        assert_eq!(
            fixture.call(name, json!({})).await.unwrap_err()["message"]["kind"],
            "Forbidden"
        );
    }
    assert_eq!(
        fixture
            .call("settings_get", json!({"owner":"main"}))
            .await
            .unwrap()["owner"],
        "remote"
    );
    assert_eq!(
        (fixture.backend.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            Value::Null
        )
        .await
        .unwrap(),
        vec![0]
    );
    fixture.finish().await;
}

#[tokio::test]
async fn 입력_byte_상한은_provider와_store_입장_전에_검증하고_원본_오류를_보존한다() {
    let fixture = Fixture::new(None);
    for (command, field, limit) in [
        ("ai_inline_complete", "prefix", 32 * 1024),
        ("ai_inline_complete", "suffix", 16 * 1024),
        ("ai_inline_edit", "selection", 100 * 1024),
        ("ai_inline_edit", "instruction", 4 * 1024),
        ("ai_commit_message", "diffText", 64 * 1024),
        ("ai_commit_message", "recentCommits", 8 * 1024),
    ] {
        let mut args = request(command, "bounded");
        args["request"][field] = "가".repeat(limit / "가".len() + 1).into();
        let error = fixture.call(command, args).await.unwrap_err();
        assert_eq!(error["code"], "InvalidArgument");
        assert!(error["message"].as_str().unwrap().contains(field));
        assert!(error["message"].as_str().unwrap().contains("bytes"));
        assert!(fixture.secrets.reads.lock().unwrap().is_empty());
        assert!(
            fixture
                .services
                .ai_requests
                .begin("remote", "bounded")
                .is_some()
        );
    }
    let mut args = request("ai_inline_complete", "exact");
    args["request"]["prefix"] = "x".repeat(32 * 1024).into();
    args["request"]["provider"] = "codex".into();
    let error = fixture.call("ai_inline_complete", args).await.unwrap_err();
    assert_eq!(error["code"], "InvalidArgument");
    assert_eq!(error["message"], "Codex access token is not configured");
    assert_eq!(
        *fixture.secrets.reads.lock().unwrap(),
        [SecretAccount::AiCodex]
    );
    fixture.services.state.settings.write().ai_provider = None;
    let error = fixture
        .call("ai_inline_edit", request("ai_inline_edit", "defaults"))
        .await
        .unwrap_err();
    assert_eq!(error["message"], "AI provider is not configured");
    fixture.secrets.failed.store(true, Ordering::Release);
    let error = fixture
        .call("ai_token_status", json!({}))
        .await
        .unwrap_err();
    assert_eq!(
        error,
        json!({"code":"Internal","message":"synthetic store failure"})
    );
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_loopback_models_생성3경로는_prompt_default_wire와_응답을_보존한다() {
    let mut server = Server::start(false).await;
    let fixture = Fixture::new(Some(server.url.clone()));
    assert_eq!(
        fixture.call("ai_token_status", json!({})).await.unwrap()["omlx"],
        true
    );
    assert_eq!(
        fixture
            .call("ai_list_models", json!({"provider":"omlx"}))
            .await
            .unwrap(),
        json!([{"modelId":MODEL,"displayName":null}])
    );
    let captured = server.next().await;
    assert_eq!(captured.path, "/v1/models");
    assert_eq!(captured.body, Value::Null);
    for (command, id, tokens, markers) in [
        (
            "ai_inline_complete",
            "complete",
            COMPLETE_TOKENS,
            vec![PREFIX, SUFFIX, "<PREFIX>", "<SUFFIX>"],
        ),
        (
            "ai_inline_edit",
            "edit",
            INSTRUCT_TOKENS,
            vec![PREFIX, SUFFIX, SELECTION, INSTRUCTION, "<SELECTION>"],
        ),
        (
            "ai_commit_message",
            "commit",
            INSTRUCT_TOKENS,
            vec![DIFF, RECENT, "<DIFF>", "<RECENT_COMMITS>"],
        ),
    ] {
        let response = fixture.call(command, request(command, id)).await.unwrap();
        assert_eq!(response, json!({"requestId":id,"text":TEXT}));
        let captured = server.next().await;
        assert_eq!(captured.path, "/v1/chat/completions");
        assert_eq!(captured.body["model"], MODEL);
        assert_eq!(captured.body["stream"], false);
        assert_eq!(captured.body["max_tokens"], tokens);
        assert_eq!(captured.body["messages"][0]["role"], "system");
        assert_eq!(captured.body["messages"][1]["role"], "user");
        let user = captured.body["messages"][1]["content"].as_str().unwrap();
        assert!(markers.iter().all(|marker| user.contains(marker)));
        if command != "ai_commit_message" {
            let system = captured.body["messages"][0]["content"].as_str().unwrap();
            assert!(system.contains("rust"));
            assert!(system.contains("synthetic.rs"));
        }
    }
    assert_eq!(
        fixture
            .secrets
            .reads
            .lock()
            .unwrap()
            .iter()
            .filter(|account| **account == SecretAccount::AiOmlx)
            .count(),
        4
    );
    fixture.finish().await;
    server.finish().await;
}

#[tokio::test]
async fn 실제_pending_http의_remote_owner_취소_drop_shutdown은_요청_identity를_회수한다() {
    let mut server = Server::start(true).await;
    let fixture = Fixture::new(Some(server.url.clone()));
    let store = &fixture.services.ai_requests;
    let (main, mut main_cancelled) = store.begin("main", "same").unwrap();
    let mut complete =
        Box::pin(fixture.call("ai_inline_complete", request("ai_inline_complete", "same")));
    tokio::select! {
        result = &mut complete => panic!("HTTP must remain pending: {result:?}"),
        _ = server.next() => {}
    }
    assert!(store.begin("remote", "same").is_none());
    let error = fixture
        .call("ai_inline_complete", request("ai_inline_complete", "same"))
        .await
        .unwrap_err();
    assert_eq!(error["code"], "InvalidArgument");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("already in flight")
    );
    assert_eq!(
        fixture
            .call(
                "ai_request_cancel",
                json!({"owner":"main","requestId":"same"})
            )
            .await
            .unwrap(),
        Value::Null
    );
    assert_eq!(
        complete.await.unwrap(),
        json!({"requestId":"same","text":null})
    );
    assert_eq!(
        main_cancelled.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    );
    assert!(store.begin("main", "same").is_none());
    let mut edit = Box::pin(fixture.call("ai_inline_edit", request("ai_inline_edit", "dropped")));
    tokio::select! {
        result = &mut edit => panic!("HTTP must remain pending: {result:?}"),
        _ = server.next() => {}
    }
    drop(edit);
    let (new, _) = store.begin("remote", "dropped").unwrap();
    drop(new);
    let mut commit = Box::pin(fixture.call(
        "ai_commit_message",
        request("ai_commit_message", "shutdown"),
    ));
    tokio::select! {
        result = &mut commit => panic!("HTTP must remain pending: {result:?}"),
        _ = server.next() => {}
    }
    store.shutdown();
    let mut idle = Box::pin(store.wait_for_idle());
    assert!(poll_fn(|context| Poll::Ready(idle.as_mut().poll(context).is_pending())).await);
    assert!(store.begin("remote", "new").is_none());
    assert_eq!(
        commit.await.unwrap(),
        json!({"requestId":"shutdown","text":null})
    );
    main_cancelled.await.unwrap();
    drop(main);
    tokio::time::timeout(DEADLINE, idle).await.unwrap();
    assert_eq!(
        fixture
            .call(
                "ai_inline_complete",
                request("ai_inline_complete", "stopped")
            )
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    fixture.finish().await;
    server.finish().await;
}

#[tokio::test]
async fn 실제_provider의_empty_응답_truncation과_localized_http_오류를_보존한다() {
    let mut server = Server::start(false).await;
    let fixture = Fixture::new(Some(server.url.clone()));
    *server.state.reply.lock().unwrap() = (StatusCode::OK, chat_response(" \n", "stop"));
    assert_eq!(
        fixture
            .call("ai_inline_complete", request("ai_inline_complete", "empty"))
            .await
            .unwrap(),
        json!({"requestId":"empty","text":null})
    );
    server.next().await;
    *server.state.reply.lock().unwrap() = (StatusCode::OK, chat_response(TEXT, "length"));
    let error = fixture
        .call("ai_inline_edit", request("ai_inline_edit", "truncated"))
        .await
        .unwrap_err();
    assert_eq!(error["code"], "Internal");
    assert_eq!(
        error["message"],
        "omlx response was truncated at the output token limit"
    );
    server.next().await;
    *server.state.reply.lock().unwrap() = (
        StatusCode::UNAUTHORIZED,
        json!({"error":"synthetic rejection"}),
    );
    let error = fixture
        .call(
            "ai_commit_message",
            request("ai_commit_message", "unauthorized"),
        )
        .await
        .unwrap_err();
    assert_eq!(error["message"]["key"], "error.ai.unauthorized");
    assert_eq!(error["message"]["kind"], "Internal");
    server.next().await;
    fixture.finish().await;
    server.finish().await;
}
