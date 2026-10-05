use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use taide_model::app::{AppFileTarget, PromptTemplateId};
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::theme::{Theme, ThemeType};
use taide_remote::command_policy::REMOTE_ALLOWED_COMMANDS;
use taide_runtime::{AppState, EventSink, TaskSupervisor};

use super::*;
use crate::remote_gateway;
use crate::remote_ws::{ChannelFactory, ResponseBody};

const DEADLINE: Duration = Duration::from_secs(5);
const FONT_SIZE: u32 = 18;
const SYNTHETIC_THEME: &str = "synthetic-remote-theme";
const SNIPPET_FILE: &str = "synthetic.code-snippets";
const SNIPPET_CONTENT: &str =
    "{\n  \"Synthetic\": {\"prefix\": \"native\", \"body\": [\"synthetic text\"]}\n}";
const EDITOR_THEME: &str = "synthetic-editor-theme";

#[derive(Default)]
struct Sink {
    events: Mutex<Vec<AppEvent>>,
    trace: Arc<Mutex<Vec<&'static str>>>,
}

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.trace.lock().unwrap().push(match &event {
            AppEvent::SettingsChanged { .. } => "settings",
            AppEvent::ThemeChanged { .. } => "theme",
            _ => "other",
        });
        self.events.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    dispatch: Dispatch,
    sink: Arc<Sink>,
    delegated: Arc<AtomicUsize>,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "taide-native-remote-preferences-{}",
            ProjectId::new()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let directory = directory.canonicalize().unwrap();
        let state = AppState::new(AppPaths::new(directory.clone()));
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        let observed = sink.trace.clone();
        let reconcile: Reconcile = Arc::new(move |services, _, updated| {
            let observed = observed.clone();
            Box::pin(async move {
                assert_eq!(*services.state.settings.read(), updated);
                let persisted: Settings = serde_json::from_slice(
                    &std::fs::read(services.state.paths.settings_file()).unwrap(),
                )
                .unwrap();
                assert_eq!(persisted, updated);
                observed.lock().unwrap().push("reconcile");
            })
        });
        let delegated = Arc::new(AtomicUsize::new(0));
        let json_delegated = delegated.clone();
        let raw_delegated = delegated.clone();
        let remaining = Dispatch {
            json: Arc::new(move |_, name, args, channels| {
                let delegated = json_delegated.clone();
                Box::pin(async move {
                    assert_eq!(name, "layout_get");
                    delegated.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(args["owner"], "remote");
                    channels("7".into())(ResponseBody::Json("null".into())).unwrap();
                    Ok(args.to_string())
                })
            }),
            raw: Arc::new(move |_, name, args| {
                let delegated = raw_delegated.clone();
                Box::pin(async move {
                    assert_eq!(name, "file_read_raw");
                    assert_eq!(args["owner"], "remote");
                    delegated.fetch_add(1, Ordering::SeqCst);
                    Ok(vec![0, 255])
                })
            }),
        };
        let ports = Ports {
            info: AppInfo {
                name: "TAIDE".into(),
                version: "synthetic-version".into(),
                platform: "synthetic-platform".into(),
                arch: "synthetic-arch".into(),
            },
            reconcile,
        };
        Self {
            directory,
            services,
            dispatch: remote_gateway::with_policy(extend_backend(ports, remaining)),
            sink,
            delegated,
        }
    }

    async fn call(&self, name: &str, args: Value) -> Result<Value, Value> {
        let channels: ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("preferences commands must not use channels")));
        let text = (self.dispatch.json)(self.services.clone(), name.into(), args, channels).await?;
        Ok(serde_json::from_str(&text).unwrap())
    }

    async fn ok(&self, name: &str, args: Value) -> Value {
        self.call(name, args)
            .await
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn 원격_theme_editor는_실제_owner_중복guard와_삭제fallback을_보존한다() {
    use taide_model::ids::{PaneId, TabId};
    use taide_model::layout::{PaneNode, Tab, TabKind};
    let fixture = Fixture::new();
    let project = ProjectId::new();
    let pane = PaneId::new();
    let tab = TabId::new();
    let mut layout = taide_layout::service::default_layout();
    layout.root = PaneNode::Leaf {
        id: pane.clone(),
        tabs: vec![Tab {
            id: tab.clone(),
            kind: TabKind::Settings,
            title: "Settings".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(tab.clone()),
    };
    layout.focused_pane = pane.clone();
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(project.clone(), layout.clone());
    let context = serde_json::json!({"projectId":project,"paneId":pane,"tabId":tab,"sourceThemeId":"taide-dark","isCreate":true});
    let theme = serde_json::json!({"version":1,"id":EDITOR_THEME,"name":"Synthetic editor theme","type":"dark","extends":"taide-dark"});
    let save = serde_json::json!({"theme":theme,"editor":context});
    let created = fixture.ok("theme_save", save.clone()).await;
    assert_eq!(created["id"], EDITOR_THEME);
    let duplicate = fixture.call("theme_save", save.clone()).await.unwrap_err();
    assert_eq!(duplicate["code"], "InvalidArgument");
    let edit = serde_json::json!({"projectId":project,"paneId":pane,"tabId":tab,"sourceThemeId":EDITOR_THEME,"isCreate":false});
    let mut updated = theme.clone();
    updated["name"] = serde_json::json!("Updated synthetic theme");
    fixture
        .ok(
            "theme_save",
            serde_json::json!({"theme":updated,"editor":edit}),
        )
        .await;
    let wrong = serde_json::json!({"projectId":project,"paneId":pane,"tabId":TabId::new(),"sourceThemeId":EDITOR_THEME,"isCreate":false});
    let refused = fixture
        .call(
            "theme_save",
            serde_json::json!({"theme":theme,"editor":wrong}),
        )
        .await
        .unwrap_err();
    assert_eq!(refused["code"], "Forbidden");
    assert_eq!(
        fixture
            .ok("theme_get", serde_json::json!({"themeId":EDITOR_THEME}))
            .await["name"],
        "Updated synthetic theme"
    );
    let wrong_source = serde_json::json!({"projectId":project,"paneId":pane,"tabId":tab,"sourceThemeId":"taide-light","isCreate":false});
    let mismatch = fixture
        .call(
            "theme_save",
            serde_json::json!({"theme":theme,"editor":wrong_source}),
        )
        .await
        .unwrap_err();
    assert_eq!(mismatch["code"], "InvalidArgument");

    {
        let mut settings = fixture.services.state.settings.write();
        settings.theme_id = EDITOR_THEME.into();
        settings.follow_system_theme = true;
    }
    fixture
        .ok(
            "theme_delete",
            serde_json::json!({"themeId":EDITOR_THEME,"editor":edit}),
        )
        .await;
    assert_eq!(
        fixture.services.state.settings.read().theme_id,
        "taide-dark"
    );
    assert!(fixture.services.state.settings.read().follow_system_theme);
    assert!(
        fixture
            .sink
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, AppEvent::SettingsChanged { .. }))
    );
    assert_eq!(
        fixture
            .call("theme_get", serde_json::json!({"themeId":EDITOR_THEME}))
            .await
            .unwrap_err()["code"],
        "NotFound"
    );
    let null = fixture
        .call(
            "theme_save",
            serde_json::json!({"theme":theme,"editor":null}),
        )
        .await
        .unwrap_err();
    assert_eq!(null["code"], "InvalidArgument");

    let guard = fixture.services.state.begin_owned_mutation().await;
    let dispatch = fixture.dispatch.json.clone();
    let services = fixture.services.clone();
    let queued = tokio::spawn(async move {
        let channels: ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("theme editor must not use channels")));
        dispatch(services, "theme_save".into(), save, channels).await
    });
    tokio::task::yield_now().await;
    let PaneNode::Leaf { active, .. } = &mut layout.root else {
        unreachable!()
    };
    *active = None;
    fixture
        .services
        .state
        .layouts
        .write()
        .insert(project, layout);
    drop(guard);
    assert_eq!(queued.await.unwrap().unwrap_err()["code"], "Forbidden");
    assert!(
        !fixture
            .services
            .state
            .paths
            .themes_dir()
            .join(format!("{EDITOR_THEME}.json"))
            .exists()
    );
}

async fn wait_until(predicate: impl Fn() -> bool) {
    tokio::time::timeout(DEADLINE, async {
        while !predicate() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn 명령_목록은_실제_arm과_일치하고_다른_backend와_channel_raw를_보존한다() {
    let declared: BTreeSet<_> = COMMANDS.iter().copied().collect();
    assert_eq!(declared.len(), COMMANDS.len());
    assert!(
        declared
            .iter()
            .all(|name| REMOTE_ALLOWED_COMMANDS.contains(name))
    );
    let pattern = regex::Regex::new(r#"(?m)^        "([a-z_]+)" =>"#).unwrap();
    let source = include_str!("remote-preferences.rs");
    let arms: BTreeSet<_> = pattern
        .captures_iter(source)
        .map(|capture| capture.get(1).unwrap().as_str())
        .collect();
    assert_eq!(declared, arms);
    let fixture = Fixture::new();
    let delivered = Arc::new(AtomicUsize::new(0));
    let observed = delivered.clone();
    let channels: ChannelFactory = Arc::new(move |id| {
        assert_eq!(id, "7");
        let observed = observed.clone();
        Box::new(move |body| {
            let ResponseBody::Json(text) = body else {
                panic!("JSON channel expected")
            };
            assert_eq!(text, "null");
            observed.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    });
    (fixture.dispatch.json)(
        fixture.services.clone(),
        "layout_get".into(),
        serde_json::json!({"owner":"main"}),
        channels,
    )
    .await
    .unwrap();
    assert_eq!(
        (fixture.dispatch.raw)(
            fixture.services.clone(),
            "file_read_raw".into(),
            serde_json::json!({"owner":"main"})
        )
        .await
        .unwrap(),
        vec![0, 255]
    );
    assert_eq!(delivered.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.delegated.load(Ordering::SeqCst), 2);
    assert!(
        fixture
            .call("remote_issue_link", Value::Null)
            .await
            .is_err()
    );
    assert!(
        fixture
            .call("unknown-fixture-command", Value::Null)
            .await
            .is_err()
    );
    assert_eq!(fixture.delegated.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn 실제_preferences_19명령은_합성_파일_상태_이벤트를_왕복한다() {
    let fixture = Fixture::new();
    let info = fixture.ok("app_get_info", Value::Null).await;
    assert_eq!(info["version"], "synthetic-version");
    assert_eq!(info["platform"], "synthetic-platform");
    let before = fixture.ok("settings_get", Value::Null).await;
    assert_eq!(
        before["editorFontSize"],
        fixture.services.state.settings.read().editor_font_size
    );
    let patched = fixture.ok("settings_update", serde_json::json!({"patch":{"editorFontSize":FONT_SIZE,"remotePasswordOnlyLogin":true}})).await;
    assert_eq!(patched["editorFontSize"], FONT_SIZE);
    assert_eq!(patched["remotePasswordOnlyLogin"], false);
    let theme = Theme {
        version: 1,
        id: SYNTHETIC_THEME.into(),
        name: "Synthetic remote theme".into(),
        theme_type: ThemeType::Dark,
        extends: Some("taide-dark".into()),
        palette: Default::default(),
        colors: Default::default(),
        syntax: Default::default(),
        terminal: Default::default(),
        token_colors: None,
        author: None,
        license: None,
        source: None,
    };
    assert_eq!(
        fixture
            .ok("theme_save", serde_json::json!({"theme":theme}))
            .await["id"],
        SYNTHETIC_THEME
    );
    assert!(
        fixture
            .ok("theme_list", Value::Null)
            .await
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == SYNTHETIC_THEME)
    );
    assert_eq!(
        fixture
            .ok("theme_get", serde_json::json!({"themeId":SYNTHETIC_THEME}))
            .await["id"],
        SYNTHETIC_THEME
    );
    let themed = fixture
        .ok(
            "settings_set_theme",
            serde_json::json!({"themeId":SYNTHETIC_THEME}),
        )
        .await;
    assert_eq!(themed["themeId"], SYNTHETIC_THEME);
    assert_eq!(
        fixture
            .ok(
                "theme_get_current",
                serde_json::json!({"systemTheme":"light"})
            )
            .await["id"],
        SYNTHETIC_THEME
    );
    let locales = fixture.ok("locale_list", Value::Null).await;
    let locale_id = locales.as_array().unwrap().first().unwrap()["id"]
        .as_str()
        .unwrap();
    assert_eq!(
        fixture
            .ok("locale_get", serde_json::json!({"localeId":locale_id}))
            .await["id"],
        locale_id
    );
    assert!(
        !fixture
            .ok(
                "locale_get_current",
                serde_json::json!({"systemLanguage":"en-US"})
            )
            .await["messages"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fixture
            .ok(
                "snippet_save",
                serde_json::json!({"fileName":SNIPPET_FILE,"content":SNIPPET_CONTENT})
            )
            .await["fileName"],
        SNIPPET_FILE
    );
    assert_eq!(
        std::fs::read_to_string(
            fixture
                .services
                .state
                .paths
                .snippets_dir()
                .join(SNIPPET_FILE)
        )
        .unwrap(),
        SNIPPET_CONTENT
    );
    assert_eq!(
        fixture
            .ok("snippet_list", Value::Null)
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fixture
        .ok(
            "snippet_delete",
            serde_json::json!({"fileName":SNIPPET_FILE}),
        )
        .await;
    assert!(
        !fixture
            .services
            .state
            .paths
            .snippets_dir()
            .join(SNIPPET_FILE)
            .exists()
    );
    let target = AppFileTarget::Settings;
    let content = fixture
        .ok("app_file_read", serde_json::json!({"target":target}))
        .await;
    let mut settings: Settings = serde_json::from_str(content.as_str().unwrap()).unwrap();
    settings.editor_font_size = FONT_SIZE + 1;
    settings.remote_password_only_login = true;
    fixture.ok("app_file_write", serde_json::json!({"target":target,"content":serde_json::to_string(&settings).unwrap()})).await;
    let persisted: Settings = serde_json::from_slice(
        &std::fs::read(fixture.services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(persisted.editor_font_size, FONT_SIZE + 1);
    assert!(!persisted.remote_password_only_login);
    let prompt_target = AppFileTarget::Prompt {
        id: PromptTemplateId::AutoTabDefault,
    };
    let bundled = fixture
        .ok("app_file_read", serde_json::json!({"target":prompt_target}))
        .await;
    let mut prompt: Value = serde_json::from_str(bundled.as_str().unwrap()).unwrap();
    prompt["fim"]["prompt"] = Value::String("synthetic prompt".into());
    let prompt = prompt.to_string();
    fixture
        .ok(
            "app_file_write",
            serde_json::json!({"target":prompt_target,"content":prompt}),
        )
        .await;
    assert_eq!(
        fixture
            .ok("app_file_read", serde_json::json!({"target":prompt_target}))
            .await,
        prompt
    );
    fixture
        .ok(
            "theme_delete",
            serde_json::json!({"themeId":SYNTHETIC_THEME}),
        )
        .await;
    assert!(
        !fixture
            .services
            .state
            .paths
            .themes_dir()
            .join(format!("{SYNTHETIC_THEME}.json"))
            .exists()
    );
    let session = fixture.services.remote.issue_session_without_nonce();
    assert_eq!(
        fixture.ok("remote_status", Value::Null).await["running"],
        false
    );
    fixture.ok("remote_revoke_sessions", Value::Null).await;
    assert!(!fixture.services.remote.has_active_session(&session));
    assert_eq!(
        *fixture.sink.trace.lock().unwrap(),
        [
            "reconcile",
            "settings",
            "reconcile",
            "settings",
            "theme",
            "reconcile",
            "settings"
        ]
    );
    assert_eq!(fixture.delegated.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    assert!(
        fixture
            .call("theme_get", serde_json::json!({"themeId":"../outside"}))
            .await
            .is_err()
    );
    assert!(
        fixture
            .call(
                "snippet_save",
                serde_json::json!({"fileName":"../outside.json","content":SNIPPET_CONTENT})
            )
            .await
            .is_err()
    );
    assert!(
        fixture
            .call("settings_set_theme", serde_json::json!({"themeId":123}))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn 설정_쓰기3진입점은_요청_취소와_감독자_종료_뒤에도_admitted_작업을_완료한다() {
    let next = Settings {
        editor_font_size: FONT_SIZE,
        ..Default::default()
    };
    let requests = [
        (
            "settings_update",
            serde_json::json!({"patch":{"editorFontSize":FONT_SIZE}}),
        ),
        (
            "settings_set_theme",
            serde_json::json!({"themeId":"taide-light"}),
        ),
        (
            "app_file_write",
            serde_json::json!({"target":{"kind":"settings"},"content":serde_json::to_string(&next).unwrap()}),
        ),
    ];
    for (name, args) in requests {
        let fixture = Fixture::new();
        let repeated_args = args.clone();
        let held = fixture.services.state.begin_owned_mutation().await;
        let services = fixture.services.clone();
        let dispatch = fixture.dispatch.json.clone();
        let channels: ChannelFactory =
            Arc::new(|_| Box::new(|_| panic!("no channels in settings writes")));
        let request =
            tokio::spawn(async move { dispatch(services, name.into(), args, channels).await });
        wait_until(|| fixture.services.tasks.tracked_count() > 0).await;
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        let tasks = fixture.services.tasks.clone();
        let shutdown = tokio::spawn(async move { tasks.shutdown().await });
        tokio::task::yield_now().await;
        assert!(!shutdown.is_finished());
        assert!(fixture.sink.events.lock().unwrap().is_empty());
        drop(held);
        tokio::time::timeout(DEADLINE, shutdown)
            .await
            .unwrap()
            .unwrap();
        let persisted: Settings = serde_json::from_slice(
            &std::fs::read(fixture.services.state.paths.settings_file()).unwrap(),
        )
        .unwrap();
        if name == "settings_set_theme" {
            assert_eq!(persisted.theme_id, "taide-light");
            assert_eq!(
                *fixture.sink.trace.lock().unwrap(),
                ["reconcile", "settings", "theme"]
            );
        } else {
            assert_eq!(persisted.editor_font_size, FONT_SIZE);
            assert_eq!(
                *fixture.sink.trace.lock().unwrap(),
                ["reconcile", "settings"]
            );
        }
        assert_eq!(*fixture.services.state.settings.read(), persisted);
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        let rejection = fixture.call(name, repeated_args).await.unwrap_err();
        assert_eq!(rejection["code"], "Forbidden");
    }
}
