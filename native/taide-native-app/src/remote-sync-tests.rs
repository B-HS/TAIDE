use std::collections::BTreeSet;
use std::future::{Future, poll_fn};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::Poll;
use std::time::Duration;

use serde_json::json;
use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::paths::AppPaths;
use taide_model::settings::{SETTINGS_SCHEMA_VERSION, Settings, SettingsPatch};
use taide_model::sync::SyncPayload;
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{
    AppState, EventSink, IdeSaveFile, PlatformServicesState, RemoteDispatchLimiter, TaskSupervisor,
    save_file_within_open_projects,
};
use tokio::sync::{Notify, watch};

use super::*;
use crate::remote_gateway;
use crate::remote_ws::ChannelFactory;

const COMMAND_COUNT: usize = 3;
const REMOTE_CONCURRENT: usize = 128;
const DEADLINE: Duration = Duration::from_secs(3);
const GIST: &str = "synthetic-gist";
const OTHER_GIST: &str = "other-gist";
const OLD: &str = "2026-10-01T00:00:00Z";
const NEW: &str = "2026-10-04T00:00:00Z";
const FONT: u32 = 19;
const LIVE_FONT: u32 = 21;
const THEME: &str = "synthetic-sync-theme";
const LOCALE: &str = "synthetic-sync-locale";
const LOCAL_OMLX: &str = "http://127.0.0.1:1";

struct Secrets {
    connected: AtomicBool,
    marker: String,
}

impl SecretStore for Secrets {
    fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
        panic!("remote sync must not write credentials");
    }

    fn get(&self, account: SecretAccount) -> AppResult<Option<String>> {
        assert_eq!(account, SecretAccount::GithubSync);
        Ok(self
            .connected
            .load(Ordering::Acquire)
            .then(|| self.marker.clone()))
    }

    fn delete(&self, _: SecretAccount) -> AppResult<()> {
        panic!("remote sync must not delete credentials");
    }
}

#[derive(Default)]
struct Sink {
    events: Mutex<Vec<AppEvent>>,
    trace: Mutex<Vec<&'static str>>,
    apply_started: Notify,
    apply_release: watch::Sender<bool>,
}

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.trace.lock().unwrap().push(match &event {
            AppEvent::SettingsChanged { .. } => "settings",
            AppEvent::SyncStateChanged { .. } => "sync",
            _ => panic!("unexpected sync event {event:?}"),
        });
        self.events.lock().unwrap().push(event);
    }
}

struct Call {
    kind: &'static str,
    id: Option<String>,
    payload: Option<String>,
}

struct Control {
    marker: String,
    factories: AtomicUsize,
    active: AtomicUsize,
    calls: Mutex<Vec<Call>>,
    started: Notify,
    release: watch::Sender<bool>,
    fetched: Mutex<Result<(String, String), String>>,
}

struct Active(Arc<Control>);

impl Drop for Active {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
struct Gist(Arc<Control>);

impl Gist {
    async fn enter(
        &self,
        token: &str,
        kind: &'static str,
        id: Option<&str>,
        payload: Option<&str>,
    ) -> Active {
        assert_eq!(token, self.0.marker);
        self.0.active.fetch_add(1, Ordering::AcqRel);
        let active = Active(self.0.clone());
        self.0.calls.lock().unwrap().push(Call {
            kind,
            id: id.map(str::to_owned),
            payload: payload.map(str::to_owned),
        });
        self.0.started.notify_one();
        let mut release = self.0.release.subscribe();
        while !*release.borrow_and_update() {
            release.changed().await.unwrap();
        }
        active
    }
}

impl SyncGistPort for Gist {
    async fn discover_sync_gist(
        &self,
        _: &str,
        _: Option<&str>,
    ) -> AppResult<Option<(String, String)>> {
        panic!("remote sync never discovers or connects an account");
    }

    async fn create_gist(&self, token: &str, payload: &str) -> AppResult<(String, String)> {
        let _active = self.enter(token, "create", None, Some(payload)).await;
        Ok((GIST.into(), NEW.into()))
    }

    async fn update_gist(&self, token: &str, id: &str, payload: &str) -> AppResult<String> {
        let _active = self.enter(token, "update", Some(id), Some(payload)).await;
        Ok(NEW.into())
    }

    async fn fetch_gist(&self, token: &str, id: &str) -> AppResult<(String, String)> {
        let _active = self.enter(token, "fetch", Some(id), None).await;
        self.0
            .fetched
            .lock()
            .unwrap()
            .clone()
            .map_err(AppError::Internal)
    }
}

fn theme_json() -> String {
    json!({"version":1,"id":THEME,"name":"Synthetic sync","type":"dark"}).to_string()
}

fn locale_json() -> String {
    json!({"version":1,"id":LOCALE,"name":"Synthetic sync","messages":{"synthetic":"value"}})
        .to_string()
}

fn payload() -> String {
    json!({
        "schemaVersion":SETTINGS_SCHEMA_VERSION,"updatedAt":NEW,
        "settings":{"editorFontSize":FONT,"shellOverride":"untrusted-shell",
            "remoteAccessEnabled":true,"remotePasswordOnlyLogin":false,
            "remoteAllowedHosts":["untrusted.invalid"],"aiOmlxBaseUrl":"http://untrusted.invalid"},
        "themes":[{"id":THEME,"json":theme_json()}],
        "locales":[{"id":LOCALE,"json":locale_json()}],
    })
    .to_string()
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    secrets: Arc<Secrets>,
    control: Arc<Control>,
    sink: Arc<Sink>,
    backend: Dispatch,
}

impl Fixture {
    fn new(connected: bool, has_gist: bool, blocked: bool) -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-sync-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        {
            let mut settings = state.settings.write();
            settings.sync_gist_id = has_gist.then(|| GIST.into());
            settings.sync_last_synced_at = has_gist.then(|| OLD.into());
            settings.ai_omlx_base_url = Some(LOCAL_OMLX.into());
            settings.shell_override = Some("local-shell".into());
            settings.remote_password_only_login = true;
            settings.remote_allowed_hosts = vec!["local.invalid".into()];
        }
        let secrets = Arc::new(Secrets {
            connected: AtomicBool::new(connected),
            marker: uuid::Uuid::new_v4().to_string(),
        });
        let (apply_release, _) = watch::channel(true);
        let sink = Arc::new(Sink {
            apply_release,
            ..Default::default()
        });
        let services = Arc::new(AppServices::new(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            RemoteDispatchLimiter::new(REMOTE_CONCURRENT),
            PlatformServicesState::new(Arc::new(crate::bootstrap::NativePlatform)),
            SecretStoreState(secrets.clone()),
            IdeSaveFile(save_file_within_open_projects),
            sink.clone(),
        ));
        let (release, _) = watch::channel(!blocked);
        let control = Arc::new(Control {
            marker: secrets.marker.clone(),
            factories: AtomicUsize::new(0),
            active: AtomicUsize::new(0),
            calls: Mutex::new(Vec::new()),
            started: Notify::new(),
            release,
            fetched: Mutex::new(Ok((NEW.into(), payload()))),
        });
        let factory = control.clone();
        let observed = sink.clone();
        let ports = Ports {
            create_client: Arc::new(move || {
                factory.factories.fetch_add(1, Ordering::AcqRel);
                Gist(factory.clone())
            }),
            reconcile: Arc::new(move |services, _, updated| {
                let observed = observed.clone();
                Box::pin(async move {
                    assert_eq!(*services.state.settings.read(), updated);
                    let persisted: Settings = serde_json::from_slice(
                        &std::fs::read(services.state.paths.settings_file()).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(persisted, updated);
                    observed.trace.lock().unwrap().push("reconcile");
                    observed.apply_started.notify_one();
                    let mut release = observed.apply_release.subscribe();
                    while !*release.borrow_and_update() {
                        release.changed().await.unwrap();
                    }
                })
            }),
        };
        let backend = remote_gateway::with_policy(extend_backend(
            ports,
            Dispatch {
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
            },
        ));
        Self {
            directory,
            services,
            secrets,
            control,
            sink,
            backend,
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory = Arc::new(|_| panic!("sync does not create channels"));
        let result = tokio::time::timeout(
            DEADLINE,
            (self.backend.json)(self.services.clone(), name.into(), args, channels),
        )
        .await
        .unwrap()?;
        Ok(serde_json::from_str(&result).unwrap())
    }

    async fn finish(&self) {
        self.control.release.send_replace(true);
        self.sink.apply_release.send_replace(true);
        tokio::time::timeout(DEADLINE, self.services.tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(self.services.tasks.tracked_count(), 0);
        assert_eq!(self.control.active.load(Ordering::Acquire), 0);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.control.release.send_replace(true);
        self.sink.apply_release.send_replace(true);
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

async fn started<F: Future>(future: std::pin::Pin<&mut F>, notification: &Notify) {
    tokio::time::timeout(DEADLINE, async {
        tokio::select! {
            _ = future => panic!("operation must remain pending"),
            () = notification.notified() => {}
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn catalog_정책_status_factory_비실행_오류와_remaining을_보존한다() {
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
        crate::remote_ai::COMMANDS,
    ]
    .into_iter()
    .flatten()
    .copied()
    .collect();
    assert!(declared.is_disjoint(&previous));
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let arms: BTreeSet<_> = pattern
        .captures_iter(include_str!("remote-sync.rs"))
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new(false, false, false);
    fixture.services.state.begin_shutdown();
    assert_eq!(
        fixture.call("sync_status", json!({})).await.unwrap(),
        json!({"connected":false,"hasGist":false,"lastSyncedAt":null,"remoteNewer":null})
    );
    for (name, args) in [
        ("sync_upload", json!({})),
        ("sync_download", json!({"force":false})),
    ] {
        assert_eq!(
            fixture.call(name, args).await.unwrap_err()["message"],
            "GitHub sync is not connected"
        );
    }
    for name in ["sync_connect", "sync_disconnect"] {
        assert_eq!(
            fixture.call(name, json!({})).await.unwrap_err()["message"]["kind"],
            "Forbidden"
        );
    }
    assert_eq!(
        fixture
            .call("sync_download", json!({"force":"false"}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    assert_eq!(fixture.control.factories.load(Ordering::Acquire), 0);
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

    let fixture = Fixture::new(true, true, false);
    assert_eq!(
        fixture.call("sync_status", json!({})).await.unwrap()["remoteNewer"],
        true
    );
    *fixture.control.fetched.lock().unwrap() = Err("synthetic fetch failure".into());
    assert_eq!(
        fixture.call("sync_status", json!({})).await.unwrap()["remoteNewer"],
        Value::Null
    );
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_upload_snapshot과_설정_filter_에셋_수집_이벤트_되쓰기를_보존한다() {
    let fixture = Fixture::new(true, true, true);
    let paths = &fixture.services.state.paths;
    std::fs::create_dir_all(paths.themes_dir()).unwrap();
    std::fs::create_dir_all(paths.locales_dir()).unwrap();
    std::fs::write(
        paths.themes_dir().join(format!("{THEME}.json")),
        theme_json(),
    )
    .unwrap();
    std::fs::write(
        paths.locales_dir().join(format!("{LOCALE}.json")),
        locale_json(),
    )
    .unwrap();
    let original_font = fixture.services.state.settings.read().editor_font_size;
    let mut upload = Box::pin(fixture.call("sync_upload", json!({})));
    started(upload.as_mut(), &fixture.control.started).await;
    settings_actions::settings_update(
        &fixture.services.state,
        SettingsPatch {
            editor_font_size: Some(LIVE_FONT),
            ..Default::default()
        },
        |_, _| async {},
        fixture.sink.as_ref(),
    )
    .await
    .unwrap();
    fixture.control.release.send_replace(true);
    let response = upload.await.unwrap();
    assert_eq!(
        response,
        json!({"connected":true,"hasGist":true,"lastSyncedAt":NEW,"remoteNewer":null})
    );
    assert_eq!(
        fixture.services.state.settings.read().editor_font_size,
        LIVE_FONT
    );
    let uploaded = {
        let calls = fixture.control.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].kind, "update");
        assert_eq!(calls[0].id.as_deref(), Some(GIST));
        calls[0].payload.clone().unwrap()
    };
    let raw: Value = serde_json::from_str(&uploaded).unwrap();
    assert_eq!(raw["settings"]["editorFontSize"], original_font);
    for key in [
        "shellOverride",
        "remoteAccessEnabled",
        "remotePasswordOnlyLogin",
        "remoteAllowedHosts",
        "aiOmlxBaseUrl",
    ] {
        assert!(raw["settings"].get(key).is_none_or(Value::is_null));
    }
    let payload: SyncPayload = serde_json::from_value(raw).unwrap();
    assert_eq!(payload.themes[0].id, THEME);
    assert_eq!(payload.themes[0].json, theme_json());
    assert_eq!(payload.locales[0].id, LOCALE);
    assert!(payload.updated_at.ends_with('Z'));
    assert_eq!(*fixture.sink.trace.lock().unwrap(), ["settings", "sync"]);
    let persisted: Settings =
        serde_json::from_slice(&std::fs::read(paths.settings_file()).unwrap()).unwrap();
    assert_eq!(persisted, *fixture.services.state.settings.read());
    fixture.finish().await;
}

#[tokio::test]
async fn 실제_download_conflict_force_보호필드_설정_reconcile와_에셋_순서를_보존한다() {
    let fixture = Fixture::new(true, true, false);
    *fixture.control.fetched.lock().unwrap() = Ok((NEW.into(), "malformed".into()));
    assert_eq!(
        fixture
            .call("sync_download", json!({"force":false}))
            .await
            .unwrap(),
        json!({"kind":"conflict","remoteUpdatedAt":NEW})
    );
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    let error = fixture
        .call("sync_download", json!({"force":true}))
        .await
        .unwrap_err();
    assert_eq!(error["message"], "sync payload from the gist was malformed");
    let mut newer_schema: Value = serde_json::from_str(&payload()).unwrap();
    newer_schema["schemaVersion"] = (SETTINGS_SCHEMA_VERSION + 1).into();
    *fixture.control.fetched.lock().unwrap() = Ok((NEW.into(), newer_schema.to_string()));
    assert_eq!(
        fixture
            .call("sync_download", json!({"force":true}))
            .await
            .unwrap_err()["code"],
        "InvalidArgument"
    );
    *fixture.control.fetched.lock().unwrap() = Ok((NEW.into(), payload()));
    assert_eq!(
        fixture
            .call("sync_download", json!({"force":true}))
            .await
            .unwrap(),
        json!({"kind":"applied","status":{"connected":true,"hasGist":true,"lastSyncedAt":NEW,"remoteNewer":null}})
    );
    let settings = fixture.services.state.settings.read().clone();
    assert_eq!(settings.editor_font_size, FONT);
    assert_eq!(settings.ai_omlx_base_url.as_deref(), Some(LOCAL_OMLX));
    assert_eq!(settings.shell_override.as_deref(), Some("local-shell"));
    assert!(!settings.remote_access_enabled);
    assert!(settings.remote_password_only_login);
    assert_eq!(settings.remote_allowed_hosts, ["local.invalid"]);
    assert_eq!(
        *fixture.sink.trace.lock().unwrap(),
        ["reconcile", "settings", "sync"]
    );
    let paths = &fixture.services.state.paths;
    assert!(paths.themes_dir().join(format!("{THEME}.json")).exists());
    assert!(paths.locales_dir().join(format!("{LOCALE}.json")).exists());
    fixture.finish().await;
}

#[tokio::test]
async fn upload_최초_create_guard와_취소_worker_종료_및_stale_되쓰기를_보존한다() {
    let fixture = Fixture::new(true, false, true);
    let mut upload = Box::pin(fixture.call("sync_upload", json!({})));
    started(upload.as_mut(), &fixture.control.started).await;
    let mut mutation = Box::pin(fixture.services.state.begin_mutation());
    assert!(poll_fn(|context| Poll::Ready(mutation.as_mut().poll(context).is_pending())).await);
    drop(mutation);
    drop(upload);
    assert_eq!(fixture.services.tasks.tracked_count(), 1);
    assert_eq!(fixture.control.calls.lock().unwrap()[0].kind, "create");
    let mut shutdown = Box::pin(fixture.services.tasks.shutdown());
    assert!(poll_fn(|context| Poll::Ready(shutdown.as_mut().poll(context).is_pending())).await);
    fixture.control.release.send_replace(true);
    tokio::time::timeout(DEADLINE, shutdown).await.unwrap();
    assert_eq!(
        fixture
            .services
            .state
            .settings
            .read()
            .sync_gist_id
            .as_deref(),
        Some(GIST)
    );
    assert_eq!(*fixture.sink.trace.lock().unwrap(), ["sync"]);
    fixture.finish().await;

    let fixture = Fixture::new(true, true, true);
    let mut upload = Box::pin(fixture.call("sync_upload", json!({})));
    started(upload.as_mut(), &fixture.control.started).await;
    {
        let _guard = fixture.services.state.begin_mutation().await;
        fixture.services.state.settings.write().sync_gist_id = None;
        fixture.services.state.settings.write().sync_last_synced_at = None;
        fixture.secrets.connected.store(false, Ordering::Release);
    }
    fixture.control.release.send_replace(true);
    let response = upload.await.unwrap();
    assert_eq!(
        response,
        json!({"connected":false,"hasGist":false,"lastSyncedAt":null,"remoteNewer":null})
    );
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    assert!(!fixture.services.state.paths.settings_file().exists());
    fixture.finish().await;
}

#[tokio::test]
async fn download_fetch_취소와_동시_gist_sync_변경은_apply_전에_회수하거나_재시도를_요구한다() {
    let fixture = Fixture::new(true, true, true);
    let mut download = Box::pin(fixture.call("sync_download", json!({"force":true})));
    started(download.as_mut(), &fixture.control.started).await;
    assert_eq!(fixture.control.active.load(Ordering::Acquire), 1);
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    drop(download);
    assert_eq!(fixture.control.active.load(Ordering::Acquire), 0);
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    assert!(!fixture.services.state.paths.settings_file().exists());
    fixture.finish().await;

    for change_gist in [true, false] {
        let fixture = Fixture::new(true, true, true);
        let mut download = Box::pin(fixture.call("sync_download", json!({"force":true})));
        started(download.as_mut(), &fixture.control.started).await;
        {
            let _guard = fixture.services.state.begin_mutation().await;
            let mut settings = fixture.services.state.settings.write();
            if change_gist {
                settings.sync_gist_id = Some(OTHER_GIST.into());
            } else {
                settings.sync_last_synced_at = Some(NEW.into());
            }
        }
        fixture.control.release.send_replace(true);
        let error = download.await.unwrap_err();
        assert_eq!(error["code"], "InvalidArgument");
        let expected = if change_gist {
            "the configured sync gist changed while downloading — retry the download"
        } else {
            "another sync completed while downloading — retry the download"
        };
        assert_eq!(error["message"], expected);
        assert!(fixture.sink.events.lock().unwrap().is_empty());
        assert!(!fixture.services.state.paths.settings_file().exists());
        fixture.finish().await;
    }
}

#[tokio::test]
async fn download_apply_취소_뒤에도_reconcile_에셋_이벤트를_완료하고_감독자_거절을_보존한다() {
    let fixture = Fixture::new(true, true, false);
    fixture.sink.apply_release.send_replace(false);
    let mut download = Box::pin(fixture.call("sync_download", json!({"force":true})));
    started(download.as_mut(), &fixture.sink.apply_started).await;
    drop(download);
    assert_eq!(fixture.services.tasks.tracked_count(), 1);
    assert_eq!(*fixture.sink.trace.lock().unwrap(), ["reconcile"]);
    assert_eq!(
        fixture.services.state.settings.read().editor_font_size,
        FONT
    );
    assert!(
        !fixture
            .services
            .state
            .paths
            .themes_dir()
            .join(format!("{THEME}.json"))
            .exists()
    );
    let mut shutdown = Box::pin(fixture.services.tasks.shutdown());
    assert!(poll_fn(|context| Poll::Ready(shutdown.as_mut().poll(context).is_pending())).await);
    fixture.sink.apply_release.send_replace(true);
    tokio::time::timeout(DEADLINE, shutdown).await.unwrap();
    assert_eq!(
        *fixture.sink.trace.lock().unwrap(),
        ["reconcile", "settings", "sync"]
    );
    assert!(
        fixture
            .services
            .state
            .paths
            .themes_dir()
            .join(format!("{THEME}.json"))
            .exists()
    );
    fixture.finish().await;

    let fixture = Fixture::new(true, true, false);
    fixture.services.tasks.stop_all();
    assert_eq!(
        fixture.call("sync_upload", json!({})).await.unwrap_err()["code"],
        "Forbidden"
    );
    assert_eq!(fixture.control.factories.load(Ordering::Acquire), 0);
    assert_eq!(
        fixture
            .call("sync_download", json!({"force":true}))
            .await
            .unwrap_err()["code"],
        "Forbidden"
    );
    assert_eq!(fixture.control.calls.lock().unwrap().len(), 1);
    assert!(fixture.sink.events.lock().unwrap().is_empty());
    fixture.finish().await;
}
