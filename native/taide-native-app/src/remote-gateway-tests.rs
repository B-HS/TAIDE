use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::settings::Settings;
use taide_remote::command_policy::{REMOTE_ALLOWED_COMMANDS, REMOTE_DENIED_COMMANDS};
use taide_runtime::{AppState, EventSink, TaskSupervisor};

use super::*;
use crate::remote_ws::{ChannelFactory, ResponseBody};

const FONT_SIZE: u32 = 18;
const CURRENT_HOST: &str = "desktop.test";
const CURRENT_SHELL: &str = "synthetic-desktop-shell";
const CURRENT_URL: &str = "https://desktop.test";

#[derive(Default)]
struct Sink(Mutex<Vec<AppEvent>>);

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    dispatch: Dispatch,
    calls: Arc<AtomicUsize>,
    reconciles: Arc<AtomicUsize>,
    sink: Arc<Sink>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-remote-policy-{}", ProjectId::new()));
        std::fs::create_dir_all(&directory).unwrap();
        let directory = directory.canonicalize().unwrap();
        let state = AppState::new(AppPaths::new(directory.clone()));
        let sink = Arc::new(Sink::default());
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            sink.clone(),
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let json_calls = calls.clone();
        let raw_calls = calls.clone();
        let reconciles = Arc::new(AtomicUsize::new(0));
        let json_reconciles = reconciles.clone();
        let backend = Dispatch {
            json: Arc::new(move |services, name, args, _| {
                let calls = json_calls.clone();
                let reconciles = json_reconciles.clone();
                Box::pin(async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    match name.as_str() {
                        "settings_update" => {
                            let patch = argument(&args, "patch").map_err(error_value)?;
                            let updated = taide_runtime::settings_actions::settings_update(
                                &services.state,
                                patch,
                                |_, _| async move {
                                    reconciles.fetch_add(1, Ordering::SeqCst);
                                },
                                services.events.as_ref(),
                            )
                            .await
                            .map_err(error_value)?;
                            Ok(serde_json::to_string(&updated).unwrap())
                        }
                        "app_file_write" => {
                            let target = argument(&args, "target").map_err(error_value)?;
                            let content = argument(&args, "content").map_err(error_value)?;
                            taide_runtime::app_actions::app_file_write(
                                &services.state,
                                target,
                                content,
                                |next| async {
                                    taide_runtime::settings_actions::apply_and_broadcast(
                                        &services.state,
                                        next,
                                        |_, _| async move {
                                            reconciles.fetch_add(1, Ordering::SeqCst);
                                        },
                                        services.events.as_ref(),
                                    )
                                    .await
                                },
                            )
                            .await
                            .map_err(error_value)?;
                            Ok("null".into())
                        }
                        "agent_hooks_install" => {
                            let name: String = argument(&args, "agentName").map_err(error_value)?;
                            let scope = taide_agent::service::hook_scope_for_agent(&name)
                                .map_err(error_value)?;
                            Ok(serde_json::to_string(&scope).unwrap())
                        }
                        _ => Ok(args.to_string()),
                    }
                })
            }),
            raw: Arc::new(move |_, name, args| {
                let calls = raw_calls.clone();
                Box::pin(async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(name, "file_read_raw");
                    assert_eq!(args["owner"], "remote");
                    Ok(vec![0, 255])
                })
            }),
        };
        Self {
            directory,
            services,
            dispatch: with_policy(backend),
            calls,
            reconciles,
            sink,
        }
    }

    async fn json(&self, name: &str, args: Value) -> Result<String, Value> {
        let channels: ChannelFactory = Arc::new(|_| {
            Box::new(|_: ResponseBody| panic!("policy fixture must not use channels"))
        });
        (self.dispatch.json)(self.services.clone(), name.into(), args, channels).await
    }

    fn assert_protected(&self, settings: &Settings) {
        assert!(!settings.remote_password_only_login);
        assert_eq!(settings.remote_allowed_hosts, [CURRENT_HOST]);
        assert_eq!(settings.shell_override.as_deref(), Some(CURRENT_SHELL));
        assert_eq!(settings.ai_omlx_base_url.as_deref(), Some(CURRENT_URL));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn 단일_정책_목록은_거부_시_backend를_실행하지_않고_owner와_hook_scope를_지킨다() {
    let fixture = Fixture::new();
    for (name, policy) in REMOTE_DENIED_COMMANDS {
        let expected = error_value(policy.denial_error(name));
        assert_eq!(fixture.json(name, Value::Null).await.unwrap_err(), expected);
        assert_eq!(
            (fixture.dispatch.raw)(fixture.services.clone(), (*name).into(), Value::Null)
                .await
                .unwrap_err(),
            expected
        );
    }
    for name in ["synthetic_unclassified", "file_read_raw"] {
        assert_eq!(
            fixture.json(name, Value::Null).await.unwrap_err(),
            error_value(RemoteDenialPolicy::Unclassified.denial_error(name))
        );
    }
    assert_eq!(
        (fixture.dispatch.raw)(fixture.services.clone(), "settings_get".into(), Value::Null)
            .await
            .unwrap_err(),
        error_value(RemoteDenialPolicy::Unclassified.denial_error("settings_get"))
    );
    for name in ["codex", "gemini", "opencode", "pi"] {
        let result = fixture
            .json(
                "agent_hooks_install",
                serde_json::json!({"projectId":"synthetic-project","agentName":name}),
            )
            .await;
        assert_eq!(
            result.unwrap_err(),
            error_value(
                RemoteDenialPolicy::DesktopCliInterception.denial_error("agent_hooks_install")
            )
        );
    }
    let invalid = fixture
        .json(
            "agent_hooks_install",
            serde_json::json!({"agentName":"codex"}),
        )
        .await
        .unwrap_err();
    assert_eq!(invalid["code"], "InvalidArgument");
    assert!(
        invalid["message"]
            .as_str()
            .unwrap()
            .starts_with("projectId:")
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 0);
    let project = fixture
        .json(
            "agent_hooks_install",
            serde_json::json!({"projectId":"synthetic-project","agentName":"claude"}),
        )
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&project).unwrap(),
        serde_json::json!(HookInstallScope::Project)
    );
    let unknown = fixture
        .json(
            "agent_hooks_install",
            serde_json::json!({"projectId":"synthetic-project","agentName":"unknown-fixture"}),
        )
        .await
        .unwrap_err();
    assert_eq!(unknown["code"], "InvalidArgument");
    assert_eq!(fixture.calls.load(Ordering::SeqCst), 2);
    let args = serde_json::json!({"owner":"main","input":{"owner":"editor"},"batch":[{"owner":null},{"nested":{"owner":"other"}}],"unchanged":true});
    let reply = fixture.json("layout_set_view_state", args).await.unwrap();
    let reply: Value = serde_json::from_str(&reply).unwrap();
    assert_eq!(reply["owner"], "remote");
    assert_eq!(reply["input"]["owner"], "remote");
    assert_eq!(reply["batch"][0]["owner"], "remote");
    assert_eq!(reply["batch"][1]["nested"]["owner"], "remote");
    assert_eq!(reply["unchanged"], true);
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
    assert!(REMOTE_ALLOWED_COMMANDS.contains(&"agent_hooks_uninstall"));
    assert!(REMOTE_ALLOWED_COMMANDS.contains(&"pty_spawn"));
}

#[tokio::test]
async fn 원격_patch와_전체_settings_쓰기의_보호값은_실제_파일과_이벤트에서도_유지된다() {
    let fixture = Fixture::new();
    {
        let mut settings = fixture.services.state.settings.write();
        settings.remote_password_only_login = false;
        settings.remote_allowed_hosts = vec![CURRENT_HOST.into()];
        settings.shell_override = Some(CURRENT_SHELL.into());
        settings.ai_omlx_base_url = Some(CURRENT_URL.into());
    }
    let patch = serde_json::json!({"remotePasswordOnlyLogin":true,"remoteAllowedHosts":["attacker.test"],"shellOverride":"synthetic-attacker-shell","aiOmlxBaseUrl":"https://attacker.test","remoteAccessEnabled":false,"editorFontSize":FONT_SIZE});
    let updated = fixture
        .json("settings_update", serde_json::json!({"patch":patch}))
        .await
        .unwrap();
    let updated: Settings = serde_json::from_str(&updated).unwrap();
    fixture.assert_protected(&updated);
    assert_eq!(updated.editor_font_size, FONT_SIZE);
    assert!(!updated.remote_access_enabled);
    let next = Settings {
        remote_password_only_login: true,
        remote_allowed_hosts: vec!["attacker.test".into()],
        shell_override: Some("synthetic-attacker-shell".into()),
        ai_omlx_base_url: Some("https://attacker.test".into()),
        editor_font_size: FONT_SIZE + 1,
        ..Default::default()
    };
    assert_eq!(fixture.json("app_file_write", serde_json::json!({"target":{"kind":"settings"},"content":serde_json::to_string(&next).unwrap()})).await.unwrap(), "null");
    let persisted: Settings = serde_json::from_slice(
        &std::fs::read(fixture.services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    fixture.assert_protected(&persisted);
    assert_eq!(persisted.editor_font_size, FONT_SIZE + 1);
    fixture.assert_protected(&fixture.services.state.settings.read());
    assert_eq!(fixture.reconciles.load(Ordering::SeqCst), 2);
    {
        let events = fixture.sink.0.lock().unwrap();
        assert_eq!(events.len(), 2);
        for event in events.iter() {
            let AppEvent::SettingsChanged { settings } = event else {
                panic!("settings event expected")
            };
            fixture.assert_protected(settings);
        }
    }
    let calls = fixture.calls.load(Ordering::SeqCst);
    let bad = fixture
        .json(
            "app_file_write",
            serde_json::json!({"target":{"kind":"settings"},"content":"invalid JSON"}),
        )
        .await
        .unwrap_err();
    assert_eq!(bad["code"], "Localized");
    assert_eq!(bad["message"]["key"], "error.settings.jsonInvalid");
    assert_eq!(fixture.calls.load(Ordering::SeqCst), calls);
    assert!(
        fixture
            .json(
                "settings_update",
                serde_json::json!({"patch":{"editorFontSize":"invalid"}})
            )
            .await
            .is_err()
    );
    assert_eq!(fixture.calls.load(Ordering::SeqCst), calls);
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}
