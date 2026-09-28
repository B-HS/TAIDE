use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt;
use parking_lot::Mutex;

use taide_infra::secret::test_support::InMemorySecretStore;
use taide_infra::secret::{SecretAccount, SecretStore};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::paths::AppPaths;
use taide_model::settings::Settings;
use taide_model::sync::SyncDownloadResult;
use taide_runtime::sync_actions::{self, SyncGistPort};
use taide_runtime::{settings_actions, AppState, EventSink, TaskSupervisor};
use tokio::sync::oneshot;
use uuid::Uuid;

struct NoGist;

struct HeldUploadGist {
    started: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<Option<oneshot::Receiver<()>>>,
    completed: Arc<AtomicUsize>,
    should_create: bool,
}

impl HeldUploadGist {
    async fn wait_remote(&self) {
        self.started.lock().take().unwrap().send(()).ok();
        let release = self.release.lock().take().unwrap();
        release.await.unwrap();
        self.completed.fetch_add(1, Ordering::SeqCst);
    }
}

impl SyncGistPort for HeldUploadGist {
    async fn discover_sync_gist(&self, _: &str, _: Option<&str>) -> AppResult<Option<(String, String)>> {
        panic!("upload는 discover를 호출하지 않는다")
    }

    async fn create_gist(&self, _: &str, payload_json: &str) -> AppResult<(String, String)> {
        assert!(self.should_create);
        assert!(!payload_json.is_empty());
        self.wait_remote().await;
        Ok((GIST_ID.to_string(), AFTER.to_string()))
    }

    async fn update_gist(&self, _: &str, gist_id: &str, payload_json: &str) -> AppResult<String> {
        assert!(!self.should_create);
        assert_eq!(gist_id, GIST_ID);
        assert!(!payload_json.is_empty());
        self.wait_remote().await;
        Ok(AFTER.to_string())
    }

    async fn fetch_gist(&self, _: &str, _: &str) -> AppResult<(String, String)> {
        panic!("upload는 fetch를 호출하지 않는다")
    }
}

const FIXTURE_TOKEN: &str = "synthetic-not-a-credential";
const GIST_ID: &str = "fixture-gist";
const BEFORE: &str = "2026-09-20T00:00:00Z";
const AFTER: &str = "2026-09-21T00:00:00Z";
const LIVE_FONT_SIZE: u32 = 19;
const ACTION_TIMEOUT: Duration = Duration::from_secs(5);
const ROOT_WAIT_PROBE: Duration = Duration::from_millis(20);

struct Fixture {
    state: AppState,
    secret: InMemorySecretStore,
}

impl Fixture {
    fn new() -> Self {
        Self {
            state: AppState::new(AppPaths::new(
                std::env::temp_dir().join(format!("taide-sync-action-{}", Uuid::new_v4())),
            )),
            secret: InMemorySecretStore::default(),
        }
    }

    fn connect(&self, gist: Option<&str>) {
        self.secret.set(SecretAccount::GithubSync, FIXTURE_TOKEN).unwrap();
        let mut settings = self.state.settings.write();
        settings.sync_gist_id = gist.map(str::to_string);
        settings.sync_last_synced_at = gist.map(|_| BEFORE.to_string());
    }

    fn persisted(&self) -> Settings {
        taide_settings::service::parse_settings_json(&std::fs::read_to_string(self.state.paths.settings_file()).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.state.paths.data_dir.exists() {
            std::fs::remove_dir_all(&self.state.paths.data_dir).expect("자기 UUID fixture만 정리");
        }
    }
}

struct Events {
    state: AppState,
    recorded: Mutex<Vec<AppEvent>>,
}

impl Events {
    fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
            recorded: Mutex::new(Vec::new()),
        }
    }
}

impl EventSink for Events {
    fn publish(&self, event: AppEvent) {
        assert!(self.state.begin_mutation().now_or_never().is_none());
        let persisted =
            taide_settings::service::parse_settings_json(&std::fs::read_to_string(self.state.paths.settings_file()).unwrap()).unwrap();
        assert_eq!(persisted, *self.state.settings.read());
        self.recorded.lock().push(event);
    }
}

type BeforeGistCall<'a> = Box<dyn Fn(&str, &str) + Send + Sync + 'a>;

struct Gist<'a> {
    state: &'a AppState,
    expected: &'static str,
    is_guard_held: bool,
    discovered: Option<(String, String)>,
    content: String,
    should_fail: bool,
    before: BeforeGistCall<'a>,
    calls: &'a Mutex<Vec<&'static str>>,
}

impl<'a> Gist<'a> {
    fn new(state: &'a AppState, expected: &'static str, is_guard_held: bool, calls: &'a Mutex<Vec<&'static str>>) -> Self {
        Self {
            state,
            expected,
            is_guard_held,
            discovered: Some((GIST_ID.to_string(), AFTER.to_string())),
            content: "invalid fixture JSON".to_string(),
            should_fail: false,
            before: Box::new(|_, _| {}),
            calls,
        }
    }

    fn check(&self, operation: &'static str, token: &str, id: &str, content: &str) -> AppResult<()> {
        assert_eq!(operation, self.expected);
        assert_eq!(token, FIXTURE_TOKEN);
        assert_eq!(self.state.begin_mutation().now_or_never().is_none(), self.is_guard_held);
        self.calls.lock().push(operation);
        (self.before)(id, content);
        if self.should_fail {
            return Err(AppError::Internal("fixture gist failure".to_string()));
        }
        Ok(())
    }
}

impl SyncGistPort for Gist<'_> {
    async fn discover_sync_gist(&self, token: &str, preferred_id: Option<&str>) -> AppResult<Option<(String, String)>> {
        self.check("discover", token, preferred_id.unwrap_or_default(), "")?;
        Ok(self.discovered.clone())
    }

    async fn create_gist(&self, token: &str, payload_json: &str) -> AppResult<(String, String)> {
        self.check("create", token, "", payload_json)?;
        tokio::task::yield_now().await;
        assert!(self.state.begin_mutation().now_or_never().is_none());
        Ok((GIST_ID.to_string(), AFTER.to_string()))
    }

    async fn update_gist(&self, token: &str, gist_id: &str, payload_json: &str) -> AppResult<String> {
        self.check("update", token, gist_id, payload_json)?;
        Ok(AFTER.to_string())
    }

    async fn fetch_gist(&self, token: &str, gist_id: &str) -> AppResult<(String, String)> {
        self.check("fetch", token, gist_id, "")?;
        Ok((AFTER.to_string(), self.content.clone()))
    }
}

fn no_client() -> NoGist {
    panic!("이 gate는 client를 만들지 않는다")
}

async fn no_apply(_: Settings) -> AppResult<Settings> {
    panic!("이 판정은 settings를 적용하지 않는다")
}

impl SyncGistPort for NoGist {
    async fn discover_sync_gist(&self, _: &str, _: Option<&str>) -> AppResult<Option<(String, String)>> {
        panic!("이 gate는 discover를 호출하지 않는다")
    }

    async fn create_gist(&self, _: &str, _: &str) -> AppResult<(String, String)> {
        panic!("이 gate는 create를 호출하지 않는다")
    }

    async fn update_gist(&self, _: &str, _: &str, _: &str) -> AppResult<String> {
        panic!("이 gate는 update를 호출하지 않는다")
    }

    async fn fetch_gist(&self, _: &str, _: &str) -> AppResult<(String, String)> {
        panic!("이 gate는 fetch를 호출하지 않는다")
    }
}

#[tokio::test]
async fn 미연결_status는_client_factory를_호출하지_않는다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-sync-action-{}", Uuid::new_v4())),
    ));
    let secret = InMemorySecretStore::default();
    let status = sync_actions::sync_status(&state, &secret, || -> NoGist {
        panic!("미연결은 client를 만들지 않는다")
    })
    .await
    .unwrap();
    assert!(!status.connected);
    assert!(!status.has_gist);
    assert!(!state.paths.data_dir.exists());
}

#[tokio::test]
async fn status는_guard_없이_fetch하고_remote_오류는_기존_snapshot으로_반환한다() {
    for should_fail in [false, true] {
        let fixture = Fixture::new();
        fixture.connect(Some(GIST_ID));
        let calls = Mutex::new(Vec::new());
        let mut gist = Gist::new(&fixture.state, "fetch", false, &calls);
        gist.should_fail = should_fail;
        let status = sync_actions::sync_status(&fixture.state, &fixture.secret, || gist).await.unwrap();
        assert!(status.connected && status.has_gist);
        assert_eq!(status.last_synced_at.as_deref(), Some(BEFORE));
        assert_eq!(status.remote_newer, if should_fail { None } else { Some(true) });
        assert_eq!(*calls.lock(), ["fetch"]);
        assert!(!fixture.state.paths.data_dir.exists());
    }
}

struct ReadTwice(AtomicUsize);

impl SecretStore for ReadTwice {
    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        assert_eq!(account, SecretAccount::GithubSync);
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            return Ok(None);
        }
        Err(AppError::Internal("fixture second read".to_string()))
    }

    fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
        panic!("status set 금지")
    }
    fn delete(&self, _: SecretAccount) -> AppResult<()> {
        panic!("status delete 금지")
    }
}

#[tokio::test]
async fn status는_미연결이어도_기존_두번째_secret_read_오류를_보존한다() {
    let fixture = Fixture::new();
    let secret = ReadTwice(AtomicUsize::new(0));
    let error = sync_actions::sync_status(&fixture.state, &secret, no_client).await.unwrap_err();
    assert!(matches!(error, AppError::Internal(message) if message == "fixture second read"));
    assert_eq!(secret.0.load(Ordering::SeqCst), 2);
    assert!(!fixture.state.paths.data_dir.exists());
}

#[tokio::test]
async fn connect_빈_입력과_discovery_실패는_secret_설정_이벤트를_변경하지_않는다() {
    let fixture = Fixture::new();
    let events = Events::new(&fixture.state);
    let error = sync_actions::sync_connect(&fixture.state, &fixture.secret, " \n".to_string(), no_client, &events)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::InvalidArgument(message) if message == "token must not be empty"));
    let calls = Mutex::new(Vec::new());
    let mut gist = Gist::new(&fixture.state, "discover", false, &calls);
    gist.should_fail = true;
    assert!(
        sync_actions::sync_connect(&fixture.state, &fixture.secret, FIXTURE_TOKEN.to_string(), || gist, &events)
            .await
            .is_err()
    );
    assert!(fixture.secret.get(SecretAccount::GithubSync).unwrap().is_none());
    assert!(fixture.state.settings.read().sync_gist_id.is_none());
    assert!(events.recorded.lock().is_empty());
    assert!(!fixture.state.paths.data_dir.exists());
}

#[tokio::test]
async fn connect는_preferred_검색_뒤_live_설정과_같은_gist의_시각만_보존한다() {
    for discovered_id in [Some(GIST_ID), Some("other-fixture-gist"), None] {
        let fixture = Fixture::new();
        fixture.connect(Some(GIST_ID));
        let events = Events::new(&fixture.state);
        let calls = Mutex::new(Vec::new());
        let mut gist = Gist::new(&fixture.state, "discover", false, &calls);
        gist.discovered = discovered_id.map(|id| (id.to_string(), AFTER.to_string()));
        gist.before = Box::new(|id, _| {
            assert_eq!(id, GIST_ID);
            fixture.state.settings.write().editor_font_size = LIVE_FONT_SIZE;
        });
        let status = sync_actions::sync_connect(&fixture.state, &fixture.secret, FIXTURE_TOKEN.to_string(), || gist, &events)
            .await
            .unwrap();
        assert!(status.connected);
        let expected_time = if discovered_id == Some(GIST_ID) { Some(BEFORE) } else { None };
        assert_eq!(status.last_synced_at.as_deref(), expected_time);
        assert_eq!(status.remote_newer, discovered_id.map(|_| true));
        let persisted = fixture.persisted();
        assert_eq!(persisted.sync_gist_id.as_deref(), discovered_id);
        assert_eq!(persisted.editor_font_size, LIVE_FONT_SIZE);
        assert_eq!(*events.recorded.lock(), [AppEvent::SyncStateChanged { status }]);
    }
}

#[tokio::test]
async fn connect는_discovery_중_gist_변경을_재검증하고_credential을_덮지_않는다() {
    let fixture = Fixture::new();
    fixture.connect(Some(GIST_ID));
    let events = Events::new(&fixture.state);
    let calls = Mutex::new(Vec::new());
    let mut gist = Gist::new(&fixture.state, "discover", false, &calls);
    gist.before = Box::new(|_, _| {
        fixture.state.settings.write().sync_gist_id = None;
    });
    let error = sync_actions::sync_connect(&fixture.state, &fixture.secret, FIXTURE_TOKEN.to_string(), || gist, &events)
        .await
        .unwrap_err();
    assert!(
        matches!(error, AppError::InvalidArgument(message) if message == "sync configuration changed while connecting — retry the connection")
    );
    assert!(fixture.state.settings.read().sync_gist_id.is_none());
    assert_eq!(
        fixture.secret.get(SecretAccount::GithubSync).unwrap().as_deref(),
        Some(FIXTURE_TOKEN)
    );
    assert!(events.recorded.lock().is_empty());
    assert!(!fixture.state.paths.data_dir.exists());
}

#[tokio::test]
async fn disconnect는_guard_안에서_secret_삭제_저장_state_이벤트_순서를_유지한다() {
    let fixture = Fixture::new();
    fixture.connect(Some(GIST_ID));
    fixture.state.settings.write().editor_font_size = LIVE_FONT_SIZE;
    let events = Events::new(&fixture.state);
    let status = sync_actions::sync_disconnect(&fixture.state, &fixture.secret, &events)
        .await
        .unwrap();
    assert!(!status.connected && !status.has_gist);
    assert!(status.last_synced_at.is_none());
    assert!(fixture.secret.get(SecretAccount::GithubSync).unwrap().is_none());
    assert_eq!(fixture.persisted().editor_font_size, LIVE_FONT_SIZE);
    assert_eq!(*events.recorded.lock(), [AppEvent::SyncStateChanged { status }]);
}

#[tokio::test]
async fn upload_download의_미연결과_missing_gist는_client_apply_전에_거절한다() {
    let fixture = Fixture::new();
    let events = Events::new(&fixture.state);
    let upload = sync_actions::sync_upload(&fixture.state, &fixture.secret, no_client, &events)
        .await
        .unwrap_err();
    let download = sync_actions::sync_download(&fixture.state, &fixture.secret, no_client, no_apply, true, &events)
        .await
        .unwrap_err();
    assert!(matches!(upload, AppError::InvalidArgument(message) if message == "GitHub sync is not connected"));
    assert!(matches!(download, AppError::InvalidArgument(message) if message == "GitHub sync is not connected"));
    fixture.connect(None);
    let error = sync_actions::sync_download(&fixture.state, &fixture.secret, no_client, no_apply, true, &events)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::InvalidArgument(message) if message == "no sync gist is configured yet — upload once first"));
    assert!(events.recorded.lock().is_empty());
    assert!(!fixture.state.paths.data_dir.exists());
}

#[tokio::test]
async fn update는_snapshot_payload를_전송하고_guard_해제_중_live_설정과_secret_변경을_보존한다() {
    let fixture = Fixture::new();
    fixture.connect(Some(GIST_ID));
    fixture.state.settings.write().shell_override = Some("fixture-never-executed".to_string());
    std::fs::create_dir_all(fixture.state.paths.themes_dir()).unwrap();
    std::fs::create_dir_all(fixture.state.paths.locales_dir()).unwrap();
    std::fs::write(fixture.state.paths.themes_dir().join("fixture.json"), "fixture raw theme").unwrap();
    std::fs::write(fixture.state.paths.locales_dir().join("fixture.json"), "fixture raw locale").unwrap();
    let snapshot = fixture.state.settings.read().clone();
    let calls = Mutex::new(Vec::new());
    let events = Events::new(&fixture.state);
    let mut gist = Gist::new(&fixture.state, "update", false, &calls);
    gist.before = Box::new(|id, content| {
        assert_eq!(id, GIST_ID);
        let payload: taide_model::sync::SyncPayload = serde_json::from_str(content).unwrap();
        assert_eq!(
            serde_json::to_value(&payload.settings).unwrap(),
            serde_json::to_value(taide_sync::service::settings_to_sync_patch(&snapshot)).unwrap(),
        );
        assert!(payload.settings.shell_override.is_none());
        assert_eq!(payload.themes[0].json, "fixture raw theme");
        assert_eq!(payload.locales[0].json, "fixture raw locale");
        fixture.state.settings.write().editor_font_size = LIVE_FONT_SIZE;
        fixture.secret.delete(SecretAccount::GithubSync).unwrap();
    });
    let status = sync_actions::sync_upload(&fixture.state, &fixture.secret, || gist, &events)
        .await
        .unwrap();
    assert!(!status.connected && status.has_gist);
    assert_eq!(status.last_synced_at.as_deref(), Some(AFTER));
    assert_eq!(fixture.persisted().editor_font_size, LIVE_FONT_SIZE);
    assert_eq!(fixture.persisted().shell_override, snapshot.shell_override);
    assert_eq!(*events.recorded.lock(), [AppEvent::SyncStateChanged { status }]);
}

#[tokio::test]
async fn update_중_disconnect와_repoint는_되쓰기를_생략하고_이벤트를_발행하지_않는다() {
    for new_id in [None, Some("fixture-repoint")] {
        let fixture = Fixture::new();
        fixture.connect(Some(GIST_ID));
        let calls = Mutex::new(Vec::new());
        let events = Events::new(&fixture.state);
        let mut gist = Gist::new(&fixture.state, "update", false, &calls);
        gist.before = Box::new(|_, _| {
            let mut settings = fixture.state.settings.write();
            settings.sync_gist_id = new_id.map(str::to_string);
            settings.sync_last_synced_at = None;
            fixture.secret.delete(SecretAccount::GithubSync).unwrap();
        });
        let status = sync_actions::sync_upload(&fixture.state, &fixture.secret, || gist, &events)
            .await
            .unwrap();
        assert!(!status.connected);
        assert_eq!(status.has_gist, new_id.is_some());
        assert!(status.last_synced_at.is_none());
        assert_eq!(fixture.state.settings.read().sync_gist_id.as_deref(), new_id);
        assert!(events.recorded.lock().is_empty());
        assert!(!fixture.state.paths.settings_file().exists());
    }
}

#[tokio::test]
async fn 최초_create의_await_동안_guard를_유지해_두_upload가_gist를_하나만_생성한다() {
    let fixture = Fixture::new();
    fixture.connect(None);
    let calls = Mutex::new(Vec::new());
    let events = Events::new(&fixture.state);
    let first_gist = Gist::new(&fixture.state, "create", true, &calls);
    let second_gist = Gist::new(&fixture.state, "update", false, &calls);
    let first = sync_actions::sync_upload(&fixture.state, &fixture.secret, || first_gist, &events);
    let second = sync_actions::sync_upload(&fixture.state, &fixture.secret, || second_gist, &events);
    tokio::pin!(first, second);
    assert!(first.as_mut().now_or_never().is_none());
    assert!(second.as_mut().now_or_never().is_none());
    assert_eq!(*calls.lock(), ["create"]);
    assert!(first.await.unwrap().has_gist);
    assert!(second.await.unwrap().has_gist);
    assert_eq!(*calls.lock(), ["create", "update"]);
    assert_eq!(fixture.persisted().sync_gist_id.as_deref(), Some(GIST_ID));
    assert_eq!(events.recorded.lock().len(), 2);
}

#[tokio::test]
async fn upload_remote_실패는_로컬_저장_state_이벤트를_변경하지_않는다() {
    for id in [None, Some(GIST_ID)] {
        let fixture = Fixture::new();
        fixture.connect(id);
        let snapshot = fixture.state.settings.read().clone();
        let calls = Mutex::new(Vec::new());
        let events = Events::new(&fixture.state);
        let mut gist = Gist::new(&fixture.state, if id.is_some() { "update" } else { "create" }, id.is_none(), &calls);
        gist.should_fail = true;
        let error = sync_actions::sync_upload(&fixture.state, &fixture.secret, || gist, &events)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), AppErrorKind::Internal);
        assert_eq!(*fixture.state.settings.read(), snapshot);
        assert!(events.recorded.lock().is_empty());
        assert!(!fixture.state.paths.data_dir.exists());
    }
}

#[tokio::test]
async fn upload_요청_취소_뒤_create와_update의_로컬_완료를_root가_기다린다() {
    for should_create in [true, false] {
        let fixture = Fixture::new();
        let state = fixture.state.clone();
        let secret = Arc::new(InMemorySecretStore::default());
        secret.set(SecretAccount::GithubSync, FIXTURE_TOKEN).unwrap();
        if !should_create {
            state.settings.write().sync_gist_id = Some(GIST_ID.to_string());
        }
        let events = Arc::new(Events::new(&state));
        let request_events = events.clone();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let request_tasks = tasks.clone();
        let completed = Arc::new(AtomicUsize::new(0));
        let (started, started_rx) = oneshot::channel();
        let (release, held) = oneshot::channel();
        let gist = HeldUploadGist {
            started: Mutex::new(Some(started)),
            release: Mutex::new(Some(held)),
            completed: completed.clone(),
            should_create,
        };
        let request = tokio::spawn(async move {
            request_tasks
                .run_nonabortable_result("sync-upload", async move {
                    sync_actions::sync_upload(&state, secret.as_ref(), || gist, request_events.as_ref()).await
                })
                .await
        });
        tokio::time::timeout(ACTION_TIMEOUT, started_rx).await.unwrap().unwrap();
        assert_eq!(completed.load(Ordering::SeqCst), 0);
        assert!(!fixture.state.paths.settings_file().exists());
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        tasks.stop_all();
        let shutdown = tasks.shutdown();
        tokio::pin!(shutdown);
        assert!(tokio::time::timeout(ROOT_WAIT_PROBE, &mut shutdown).await.is_err());
        release.send(()).unwrap();
        tokio::time::timeout(ACTION_TIMEOUT, shutdown).await.unwrap();
        assert_eq!(completed.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.persisted().sync_gist_id.as_deref(), Some(GIST_ID));
        assert_eq!(fixture.persisted().sync_last_synced_at.as_deref(), Some(AFTER));
        assert_eq!(events.recorded.lock().len(), 1);
    }
}

#[tokio::test]
async fn download의_retry와_conflict는_malformed_parse와_force보다_기존_우선순위를_유지한다() {
    for scenario in ["gist", "sync", "conflict"] {
        let fixture = Fixture::new();
        fixture.connect(Some(GIST_ID));
        let calls = Mutex::new(Vec::new());
        let events = Events::new(&fixture.state);
        let mut gist = Gist::new(&fixture.state, "fetch", false, &calls);
        gist.before = Box::new(|_, _| {
            let mut settings = fixture.state.settings.write();
            match scenario {
                "gist" => {
                    settings.sync_gist_id = None;
                    settings.sync_last_synced_at = Some(AFTER.to_string());
                }
                "sync" => settings.sync_last_synced_at = Some(AFTER.to_string()),
                "conflict" => {}
                _ => panic!("fixture scenario"),
            }
        });
        let result = sync_actions::sync_download(&fixture.state, &fixture.secret, || gist, no_apply, scenario != "conflict", &events).await;
        match scenario {
            "gist" => assert!(
                matches!(result, Err(AppError::InvalidArgument(message)) if message == "the configured sync gist changed while downloading — retry the download")
            ),
            "sync" => assert!(
                matches!(result, Err(AppError::InvalidArgument(message)) if message == "another sync completed while downloading — retry the download")
            ),
            "conflict" => assert_eq!(
                result.unwrap(),
                SyncDownloadResult::Conflict {
                    remote_updated_at: AFTER.to_string()
                }
            ),
            _ => panic!("fixture scenario"),
        }
        assert!(events.recorded.lock().is_empty());
        assert!(!fixture.state.paths.data_dir.exists());
    }
}

#[tokio::test]
async fn download의_remote_malformed_future_schema는_apply와_파일_쓰기_전에_거절한다() {
    for scenario in ["remote", "malformed", "schema"] {
        let fixture = Fixture::new();
        fixture.connect(Some(GIST_ID));
        let snapshot = fixture.state.settings.read().clone();
        let calls = Mutex::new(Vec::new());
        let events = Events::new(&fixture.state);
        let mut gist = Gist::new(&fixture.state, "fetch", false, &calls);
        gist.should_fail = scenario == "remote";
        if scenario == "schema" {
            gist.content = serde_json::json!({"schemaVersion": taide_model::settings::SETTINGS_SCHEMA_VERSION + 1, "updatedAt": AFTER, "settings": {}}).to_string();
        }
        let error = sync_actions::sync_download(&fixture.state, &fixture.secret, || gist, no_apply, true, &events)
            .await
            .unwrap_err();
        if scenario == "malformed" {
            assert!(matches!(error, AppError::Internal(message) if message == "sync payload from the gist was malformed"));
        }
        assert_eq!(*fixture.state.settings.read(), snapshot);
        assert!(events.recorded.lock().is_empty());
        assert!(!fixture.state.paths.data_dir.exists());
    }
}

fn downloaded_content() -> String {
    serde_json::json!({
        "schemaVersion": taide_model::settings::SETTINGS_SCHEMA_VERSION,
        "updatedAt": AFTER,
        "settings": {"editorFontSize": LIVE_FONT_SIZE, "shellOverride": "fixture-never-executed", "remoteAccessEnabled": true, "aiOmlxBaseUrl": "https://fixture.invalid"},
        "themes": [{"id": "fixture-sync-theme", "json": "{\"version\":1,\"id\":\"fixture-sync-theme\",\"name\":\"Fixture\",\"type\":\"dark\"}"}],
        "locales": [{"id": "fixture-sync-locale", "json": "{\"version\":1,\"id\":\"fixture-sync-locale\",\"name\":\"Fixture\",\"messages\":{}}"}],
    }).to_string()
}

#[tokio::test]
async fn download는_guard_안에서_보호_설정_apply_뒤_theme_locale와_sync_event를_적용한다() {
    let fixture = Fixture::new();
    fixture.connect(Some(GIST_ID));
    let original = fixture.state.settings.read().clone();
    let calls = Mutex::new(Vec::new());
    let events = Events::new(&fixture.state);
    let mut gist = Gist::new(&fixture.state, "fetch", false, &calls);
    gist.content = downloaded_content();
    let result = sync_actions::sync_download(
        &fixture.state,
        &fixture.secret,
        || gist,
        |settings| async {
            assert!(fixture.state.begin_mutation().now_or_never().is_none());
            assert_eq!(settings.editor_font_size, LIVE_FONT_SIZE);
            assert_eq!(settings.shell_override, original.shell_override);
            assert_eq!(settings.remote_access_enabled, original.remote_access_enabled);
            assert_eq!(settings.ai_omlx_base_url, original.ai_omlx_base_url);
            assert!(!fixture.state.paths.themes_dir().exists());
            assert!(!fixture.state.paths.locales_dir().exists());
            settings_actions::apply_and_broadcast(&fixture.state, settings, |_, _| async { tokio::task::yield_now().await }, &events).await
        },
        true,
        &events,
    )
    .await
    .unwrap();
    let SyncDownloadResult::Applied { status } = result else {
        panic!("force 적용")
    };
    assert!(status.connected && status.has_gist);
    assert_eq!(status.last_synced_at.as_deref(), Some(AFTER));
    assert_eq!(fixture.persisted().editor_font_size, LIVE_FONT_SIZE);
    assert!(fixture.state.paths.themes_dir().join("fixture-sync-theme.json").is_file());
    assert!(fixture.state.paths.locales_dir().join("fixture-sync-locale.json").is_file());
    assert!(matches!(
        events.recorded.lock().as_slice(),
        [AppEvent::SettingsChanged { .. }, AppEvent::SyncStateChanged { .. }]
    ));
}

#[tokio::test]
async fn download_apply_입장_뒤_요청이_취소돼도_guard와_theme_locale_event를_끝까지_소유한다() {
    let fixture = Fixture::new();
    fixture.connect(Some(GIST_ID));
    let calls = Mutex::new(Vec::new());
    let mut gist = Gist::new(&fixture.state, "fetch", false, &calls);
    gist.content = downloaded_content();
    let prepared = sync_actions::prepare_sync_download(&fixture.state, &fixture.secret, || gist)
        .await
        .unwrap();
    let state = fixture.state.clone();
    let events = Arc::new(Events::new(&fixture.state));
    let request_events = events.clone();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let request_tasks = tasks.clone();
    let (started, started_rx) = oneshot::channel();
    let (release, held) = oneshot::channel();
    let request = tokio::spawn(async move {
        request_tasks
            .run_nonabortable_result("sync-download-apply", async move {
                sync_actions::apply_sync_download(
                    &state,
                    prepared,
                    |settings| {
                        let state = state.clone();
                        let events = request_events.clone();
                        async move {
                            settings_actions::apply_and_broadcast(
                                &state,
                                settings,
                                |_, _| async move {
                                    started.send(()).ok();
                                    held.await.unwrap();
                                },
                                events.as_ref(),
                            )
                            .await
                        }
                    },
                    true,
                    request_events.as_ref(),
                )
                .await
            })
            .await
    });
    tokio::time::timeout(ACTION_TIMEOUT, started_rx).await.unwrap().unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    tasks.stop_all();
    assert!(tokio::time::timeout(ROOT_WAIT_PROBE, fixture.state.begin_mutation()).await.is_err());
    let shutdown = tasks.shutdown();
    tokio::pin!(shutdown);
    assert!(tokio::time::timeout(ROOT_WAIT_PROBE, &mut shutdown).await.is_err());
    release.send(()).unwrap();
    tokio::time::timeout(ACTION_TIMEOUT, shutdown).await.unwrap();
    assert_eq!(fixture.persisted().editor_font_size, LIVE_FONT_SIZE);
    assert!(fixture.state.paths.themes_dir().join("fixture-sync-theme.json").is_file());
    assert!(fixture.state.paths.locales_dir().join("fixture-sync-locale.json").is_file());
    assert!(matches!(
        events.recorded.lock().as_slice(),
        [AppEvent::SettingsChanged { .. }, AppEvent::SyncStateChanged { .. }]
    ));
}

#[tokio::test]
async fn download_apply_실패는_theme_locale와_sync_event를_적용하지_않는다() {
    let fixture = Fixture::new();
    fixture.connect(Some(GIST_ID));
    let calls = Mutex::new(Vec::new());
    let events = Events::new(&fixture.state);
    let mut gist = Gist::new(&fixture.state, "fetch", false, &calls);
    gist.content = downloaded_content();
    let error = sync_actions::sync_download(
        &fixture.state,
        &fixture.secret,
        || gist,
        |_| async { Err(AppError::Internal("fixture apply failure".to_string())) },
        true,
        &events,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AppError::Internal(message) if message == "fixture apply failure"));
    assert!(!fixture.state.paths.data_dir.exists());
    assert!(events.recorded.lock().is_empty());
}

#[tokio::test]
async fn connect_disconnect의_설정_저장_실패는_기존_secret_먼저_변경_순서를_유지한다() {
    for connect in [false, true] {
        let fixture = Fixture::new();
        if !connect {
            fixture.connect(Some(GIST_ID));
        }
        let original = fixture.state.settings.read().clone();
        std::fs::create_dir_all(fixture.state.paths.settings_file()).unwrap();
        let calls = Mutex::new(Vec::new());
        let events = Events::new(&fixture.state);
        let result = if connect {
            let gist = Gist::new(&fixture.state, "discover", false, &calls);
            sync_actions::sync_connect(&fixture.state, &fixture.secret, FIXTURE_TOKEN.to_string(), || gist, &events).await
        } else {
            sync_actions::sync_disconnect(&fixture.state, &fixture.secret, &events).await
        };
        assert!(result.is_err());
        assert_eq!(fixture.secret.get(SecretAccount::GithubSync).unwrap().is_some(), connect);
        assert_eq!(*fixture.state.settings.read(), original);
        assert!(events.recorded.lock().is_empty());
        assert!(fixture.state.paths.settings_file().is_dir());
    }
}

#[test]
fn sync_commands는_같은_공유_secret_lazy_http_apply와_events를_주입한다() {
    let source = include_str!("../src/domain/sync/commands.rs");
    for name in ["sync_status", "sync_connect", "sync_disconnect", "sync_upload"] {
        assert!(source.contains(&format!("sync_actions::{name}(")));
    }
    assert_eq!(source.matches("secret.0.as_ref()").count(), 4);
    assert!(source.contains("sync_actions::sync_upload(&state, secret.as_ref()"));
    assert_eq!(source.matches("SyncGistHttpPort::new").count(), 4);
    assert_eq!(source.matches("&TauriEventSink(&app)").count(), 4);
    assert!(source.contains("let apply_settings = apply_settings.0;"));
    assert!(source.contains("|settings| apply_settings(&app, &state, settings)"));
    assert!(source.contains("sync_actions::prepare_sync_download("));
    assert!(source.contains(".run_nonabortable_result(\"sync-download-apply\""));
    assert!(source.contains("sync_actions::apply_sync_download("));
    assert!(
        source.find("sync_actions::prepare_sync_download(").unwrap()
            < source.find(".run_nonabortable_result(\"sync-download-apply\"").unwrap()
    );
    let github = include_str!("../src/domain/sync/github.rs");
    assert!(github.contains("impl SyncGistPort for SyncGistHttpPort"));
    assert!(github.contains("Self(outbound_http_client(HttpClientProfile::Api))"));
}

#[test]
fn native_sync_upload는_요청_취소와_독립된_완료_owner를_사용한다() {
    let source = include_str!("../src/domain/sync/commands.rs");
    assert!(source.contains(".run_nonabortable_result(\"sync-upload\""));
}
