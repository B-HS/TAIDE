use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use taide_infra::secret::SecretStoreState;
use taide_model::settings::SETTINGS_SCHEMA_VERSION;

use super::settings_sections_tests::{
    Keychain, Scene, errors, missing_cli_agents, nodes, successes, texts_of,
};
use super::*;
use crate::host::HostBridge;
use crate::settings_ports::{Gist, GistClient, GistFuture, Ports};
use crate::settings_sync_time::format_system_time;

const OLD: &str = "2026-10-01T00:00:00Z";
const NEW: &str = "2026-10-04T00:00:00Z";
const NEWER: &str = "2026-10-06T00:00:00Z";
const GIST: &str = "fixture-gist";
const PAT: &str = "ghp_fixture";
const FONT: u32 = 19;
const PULLED_FONT: u32 = 21;
const LINK_PREFIX: &str = "http://127.0.0.1:";

fn payload(font: u32) -> String {
    serde_json::json!({
        "schemaVersion": SETTINGS_SCHEMA_VERSION,
        "updatedAt": NEW,
        "settings": {"editorFontSize": font},
        "themes": [],
        "locales": [],
    })
    .to_string()
}

struct FakeGist {
    discovered: Mutex<Result<Option<(String, String)>, String>>,
    updated_at: Mutex<Result<String, String>>,
    fetched: Mutex<Result<(String, String), String>>,
    uploads: AtomicUsize,
}

impl FakeGist {
    fn new(fetched_at: &str, font: u32) -> Self {
        Self {
            discovered: Mutex::new(Ok(Some((GIST.into(), OLD.into())))),
            updated_at: Mutex::new(Ok(NEW.into())),
            fetched: Mutex::new(Ok((fetched_at.into(), payload(font)))),
            uploads: AtomicUsize::new(0),
        }
    }
}

fn answer<T: Clone + Send + 'static>(value: &Mutex<Result<T, String>>) -> GistFuture<'static, T> {
    let value = value.lock().unwrap().clone().map_err(AppError::Internal);
    Box::pin(async move { value })
}

impl GistClient for FakeGist {
    fn discover<'a>(
        &'a self,
        token: &'a str,
        _: Option<&'a str>,
    ) -> GistFuture<'a, Option<(String, String)>> {
        assert_eq!(token, PAT);
        answer(&self.discovered)
    }

    fn create<'a>(&'a self, _: &'a str, _: &'a str) -> GistFuture<'a, (String, String)> {
        Box::pin(async { Err(AppError::Internal("fixture gist is never created".into())) })
    }

    fn update<'a>(
        &'a self,
        token: &'a str,
        gist_id: &'a str,
        _: &'a str,
    ) -> GistFuture<'a, String> {
        assert_eq!((token, gist_id), (PAT, GIST));
        self.uploads.fetch_add(1, Ordering::SeqCst);
        answer(&self.updated_at)
    }

    fn fetch<'a>(&'a self, token: &'a str, gist_id: &'a str) -> GistFuture<'a, (String, String)> {
        assert_eq!((token, gist_id), (PAT, GIST));
        answer(&self.fetched)
    }
}

fn sync_scene(
    keychain: &Arc<Keychain>,
    gist: &Arc<FakeGist>,
    configure: impl FnOnce(&taide_runtime::AppState),
) -> Scene {
    let mut scene = Scene::launch(
        Some(SecretStoreState::with_store(keychain.clone())),
        configure,
    );
    scene.ports(Ports {
        gist: Gist(gist.clone()),
        ..Default::default()
    });
    scene
}

fn click_label(scene: &mut Scene, label: &str) {
    let output = scene.step(Vec::new());
    let node = *nodes(&output, label)
        .first()
        .unwrap_or_else(|| panic!("node exists: {label}"));
    scene.click(node.center());
}

fn count_of(texts: &[String], value: &str) -> usize {
    texts.iter().filter(|text| *text == value).count()
}

#[test]
fn 실제_sync는_가짜_gist와_keychain으로_연결_업로드_다운로드_해제와_상태_사건을_반영한다() {
    let keychain = Arc::new(Keychain::default());
    let gist = Arc::new(FakeGist::new(NEW, FONT));
    let mut scene = sync_scene(&keychain, &gist, |_| {});
    let description = scene.text("settings.syncDescription");
    let placeholder = scene.text("settings.syncTokenPlaceholder");
    let connect = scene.text("settings.syncConnect");
    let upload = scene.text("settings.syncUploadNow");
    let badge = scene.text("settings.syncRemoteNewerBadge");
    let never = scene.text("settings.syncLastSyncedNever");
    let output = scene.until("disconnected", |_, output| {
        !texts_of(output, &description).is_empty()
    });
    scene.type_into(
        texts_of(&output, &placeholder)[0].center(),
        "  ghp_fixture  ",
    );
    click_label(&mut scene, &connect);
    let output = scene.until("connected", |_, output| {
        !texts_of(output, &upload).is_empty()
    });
    assert_eq!(
        keychain.values.lock().unwrap().get("github").cloned(),
        Some(PAT.into())
    );
    assert_eq!(
        scene
            .application()
            .services
            .state
            .settings
            .read()
            .sync_gist_id
            .as_deref(),
        Some(GIST)
    );
    assert_eq!(texts_of(&output, GIST).len(), 1);
    assert_eq!(texts_of(&output, &never).len(), 1);
    assert_eq!(texts_of(&output, &badge).len(), 1);
    assert!(texts_of(&output, &description).is_empty());

    click_label(&mut scene, &upload);
    let uploaded = scene.text("settings.syncUploadSuccess");
    let synced = format_system_time(NEW).unwrap();
    scene.until("uploaded", |application, output| {
        successes(application).contains(&uploaded) && !texts_of(output, &synced).is_empty()
    });
    assert_eq!(gist.uploads.load(Ordering::SeqCst), 1);
    *gist.updated_at.lock().unwrap() = Err("fixture update refused".into());
    click_label(&mut scene, &upload);
    let upload_failed = scene.text("settings.syncUploadFailed");
    scene.until("upload failed", |application, _| {
        errors(application).contains(&upload_failed)
    });

    let download = scene.text("settings.syncDownloadNow");
    click_label(&mut scene, &download);
    let downloaded = scene.text("settings.syncDownloadSuccess");
    scene.until("downloaded", |application, _| {
        successes(application).contains(&downloaded)
            && application.services.state.settings.read().editor_font_size == FONT
    });
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(written["editorFontSize"], serde_json::json!(FONT));

    scene
        .application()
        .services
        .events
        .publish(AppEvent::SyncStateChanged {
            status: SyncStatus {
                connected: true,
                has_gist: true,
                last_synced_at: Some(NEWER.into()),
                remote_newer: Some(true),
            },
        });
    let newer = format_system_time(NEWER).unwrap();
    scene.until("state event", |_, output| {
        !texts_of(output, &newer).is_empty() && !texts_of(output, &badge).is_empty()
    });

    let disconnect = scene.text("settings.syncDisconnect");
    click_label(&mut scene, &disconnect);
    let disconnected = scene.text("settings.syncDisconnected");
    scene.until("disconnected again", |application, output| {
        successes(application).contains(&disconnected) && !texts_of(output, &description).is_empty()
    });
    assert!(keychain.values.lock().unwrap().get("github").is_none());
    assert_eq!(
        scene
            .application()
            .services
            .state
            .settings
            .read()
            .sync_gist_id,
        None
    );

    *gist.discovered.lock().unwrap() = Err("fixture discover refused".into());
    let output = scene.step(Vec::new());
    scene.type_into(texts_of(&output, &placeholder)[0].center(), PAT);
    click_label(&mut scene, &connect);
    scene.until("connect failed", |application, _| {
        errors(application).contains(&"fixture discover refused".to_string())
    });
    assert!(keychain.values.lock().unwrap().get("github").is_none());
}

#[test]
fn 실제_sync_충돌은_로컬_유지로_한번_업로드하고_원격_당김으로_한번_강제_다운로드한다() {
    let keychain = Arc::new(Keychain::default());
    keychain.values.lock().unwrap().insert("github", PAT.into());
    let gist = Arc::new(FakeGist::new(NEW, FONT));
    let mut scene = sync_scene(&keychain, &gist, |state| {
        let mut settings = state.settings.write();
        settings.sync_gist_id = Some(GIST.into());
        settings.sync_last_synced_at = Some(OLD.into());
    });
    let badge = scene.text("settings.syncRemoteNewerBadge");
    let download = scene.text("settings.syncDownloadNow");
    let title = scene.text("settings.syncConflictTitle");
    let keep = scene.text("settings.syncConflictKeepLocal");
    let pull = scene.text("settings.syncConflictPullRemote");
    scene.until("remote newer", |_, output| {
        !texts_of(output, &badge).is_empty()
    });
    click_label(&mut scene, &download);
    scene.until("conflict", |_, output| {
        !texts_of(output, &title).is_empty() && !nodes(output, &keep).is_empty()
    });
    assert_eq!(
        scene
            .application()
            .services
            .state
            .settings
            .read()
            .editor_font_size,
        taide_model::settings::Settings::default().editor_font_size
    );
    click_label(&mut scene, &keep);
    let uploaded = scene.text("settings.syncUploadSuccess");
    scene.until("kept local", |application, output| {
        successes(application).contains(&uploaded) && texts_of(output, &title).is_empty()
    });
    assert_eq!(gist.uploads.load(Ordering::SeqCst), 1);

    *gist.fetched.lock().unwrap() = Ok((NEWER.into(), payload(PULLED_FONT)));
    click_label(&mut scene, &download);
    scene.until("conflict again", |_, output| {
        !nodes(output, &pull).is_empty()
    });
    click_label(&mut scene, &pull);
    let downloaded = scene.text("settings.syncDownloadSuccess");
    scene.until("pulled", |application, output| {
        successes(application).contains(&downloaded)
            && application.services.state.settings.read().editor_font_size == PULLED_FONT
            && texts_of(output, &title).is_empty()
    });
    assert_eq!(gist.uploads.load(Ordering::SeqCst), 1);
    assert_eq!(
        scene
            .application()
            .services
            .state
            .settings
            .read()
            .sync_last_synced_at
            .as_deref(),
        Some(NEWER)
    );
    let successes_now = successes(scene.application());
    assert_eq!(count_of(&successes_now, &uploaded), 1);
    assert_eq!(count_of(&successes_now, &downloaded), 1);
}

struct Clipboard {
    writes: Arc<Mutex<Vec<String>>>,
    refuse: Arc<AtomicBool>,
}

fn remote_scene(keychain: &Arc<Keychain>) -> (Scene, Clipboard, Arc<Mutex<Vec<bool>>>) {
    let mut scene = Scene::launch(Some(SecretStoreState::with_store(keychain.clone())), |_| {});
    let writes = Arc::new(Mutex::new(Vec::<String>::new()));
    let refuse = Arc::new(AtomicBool::new(false));
    let toggles = Arc::new(Mutex::new(Vec::<bool>::new()));
    {
        let writer = writes.clone();
        let refused = refuse.clone();
        let toggled = toggles.clone();
        let repaint = scene.context().clone();
        let application = scene.application();
        let bridge = HostBridge::connect_with_settings_ports(
            application.services.clone(),
            Arc::new(move || repaint.request_repaint()),
            Arc::new(move |text| {
                if refused.load(Ordering::SeqCst) {
                    return Err(AppError::Internal("fixture clipboard refused".into()));
                }
                writer.lock().unwrap().push(text.into());
                Ok(())
            }),
            Arc::new(|| Ok(String::new())),
            Some(crate::host::Terminals {
                tabs: application.terminal_agent_state.terminals.clone(),
                environment: crate::terminal_environment::provider(),
            }),
            Arc::new(move |_, current, updated| {
                if current.remote_access_enabled != updated.remote_access_enabled {
                    toggled.lock().unwrap().push(updated.remote_access_enabled);
                }
                Box::pin(async {})
            }),
        )
        .unwrap();
        let old = application.bridge.replace(bridge).unwrap();
        application.runtime.block_on(old.disconnect()).unwrap();
    }
    (scene, Clipboard { writes, refuse }, toggles)
}

fn links(output: &egui::FullOutput) -> Vec<String> {
    super::settings_sections_tests::painted(output)
        .into_iter()
        .map(|(text, _)| text)
        .filter(|text| text.starts_with(LINK_PREFIX))
        .collect()
}

#[test]
fn 실제_remote_상태_링크_복사_폐기와_토글은_loopback_서버와_가짜_클립보드와_토글로_원본대로_동작한다()
 {
    let keychain = Arc::new(Keychain::default());
    let (mut scene, clipboard, toggles) = remote_scene(&keychain);
    let stopped = scene.text("remote.statusStopped");
    let warning = scene.text("remote.securityWarning");
    scene.until("stopped", |_, output| {
        !texts_of(output, &stopped).is_empty() && !texts_of(output, &warning).is_empty()
    });
    let status = {
        let application = scene.application();
        let services = application.services.clone();
        let ports = application.application_ports.remote.clone();
        application
            .runtime
            .block_on(crate::remote_http::start(services, ports))
            .unwrap()
    };
    let locale = scene.application().locale.clone();
    let running = presentation::message(
        &locale,
        "remote.statusRunning",
        &[("port", &status.port.to_string())],
    );
    let clients = presentation::message(&locale, "remote.clientCountLabel", &[("count", "0")]);
    scene.until("running", |_, output| {
        !texts_of(output, &running).is_empty() && !texts_of(output, &clients).is_empty()
    });

    let issue = scene.text("remote.issueLink");
    let copied = scene.text("remote.linkCopied");
    click_label(&mut scene, &issue);
    let output = scene.until("issued", |application, output| {
        successes(application).contains(&copied) && !links(output).is_empty()
    });
    let written = clipboard.writes.lock().unwrap().clone();
    assert_eq!(written.len(), 1);
    assert!(written[0].starts_with(&format!("{LINK_PREFIX}{}/?t=", status.port)));
    assert_eq!(links(&output), written);

    clipboard.refuse.store(true, Ordering::SeqCst);
    click_label(&mut scene, &issue);
    let output = scene.until("issued without copy", |_, output| {
        links(output).len() == 1 && links(output) != written
    });
    assert_eq!(clipboard.writes.lock().unwrap().len(), 1);
    assert_eq!(count_of(&successes(scene.application()), &copied), 1);
    assert!(errors(scene.application()).is_empty());
    assert_eq!(links(&output).len(), 1);

    let revoke = scene.text("remote.revokeSessions");
    click_label(&mut scene, &revoke);
    let revoked = scene.text("remote.sessionsRevoked");
    scene.until("revoked", |application, _| {
        successes(application).contains(&revoked)
    });

    let toggle = scene.text("remote.enableToggle");
    click_label(&mut scene, &toggle);
    scene.until("toggled", |application, output| {
        application
            .services
            .state
            .settings
            .read()
            .remote_access_enabled
            && links(output).is_empty()
    });
    assert_eq!(*toggles.lock().unwrap(), vec![true]);
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(written["remoteAccessEnabled"], serde_json::json!(true));
    crate::remote_http::stop(&scene.application().services);
    scene.until("stopped again", |_, output| {
        !texts_of(output, &stopped).is_empty()
    });
}

#[test]
fn 실제_remote_비밀번호와_password_only와_허용_호스트는_가짜_keychain과_settings_json에_반영된다() {
    let keychain = Arc::new(Keychain::default());
    let (mut scene, _, _) = remote_scene(&keychain);
    let locale = scene.application().locale.clone();
    let not_configured = scene.text("remote.passwordNotConfigured");
    let configured = scene.text("remote.passwordConfigured");
    let placeholder = scene.text("remote.passwordPlaceholder");
    let too_short = presentation::message(&locale, "remote.passwordTooShort", &[("min", "8")]);
    let output = scene.until("password row", |_, output| {
        !texts_of(output, &not_configured).is_empty()
    });
    scene.type_into(texts_of(&output, &placeholder)[0].center(), "short");
    scene.until("too short", |_, output| {
        !texts_of(output, &too_short).is_empty()
    });
    let set = scene.text("remote.passwordSet");
    scene.step(vec![egui::Event::Text("-enough".into())]);
    let output = scene.until("long enough", |_, output| {
        texts_of(output, &too_short).is_empty()
    });
    assert!(!nodes(&output, &set).is_empty());
    click_label(&mut scene, &set);
    let revoked = scene.text("remote.passwordSessionsRevoked");
    let only = scene.text("remote.passwordOnlyToggle");
    let output = scene.until("password set", |application, output| {
        successes(application).contains(&revoked)
            && !texts_of(output, &configured).is_empty()
            && !nodes(output, &only).is_empty()
    });
    let stored = keychain
        .values
        .lock()
        .unwrap()
        .get("remote")
        .cloned()
        .unwrap();
    assert_ne!(stored, "short-enough");
    assert!(texts_of(&output, &placeholder).len() == 1);

    click_label(&mut scene, &only);
    scene.until("password only", |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .remote_password_only_login
    });
    let clear = scene.text("remote.passwordClear");
    click_label(&mut scene, &clear);
    scene.until("password cleared", |application, output| {
        count_of(&successes(application), &revoked) == 2
            && !texts_of(output, &not_configured).is_empty()
    });
    assert!(keychain.values.lock().unwrap().get("remote").is_none());

    let hosts = scene.text("remote.allowedHostsPlaceholder");
    let output = scene.step(Vec::new());
    scene.type_into(
        texts_of(&output, &hosts)[0].center(),
        " Tunnel.Example.com ",
    );
    scene.keys(&[egui::Key::Enter]);
    scene.until("host added", |application, _| {
        application
            .services
            .state
            .settings
            .read()
            .remote_allowed_hosts
            == vec!["tunnel.example.com".to_string()]
    });
    let row = scene.until("host row", |_, output| {
        !texts_of(output, "tunnel.example.com").is_empty()
    });
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        written["remoteAllowedHosts"],
        serde_json::json!(["tunnel.example.com"])
    );
    assert_eq!(written["remotePasswordOnlyLogin"], serde_json::json!(true));
    let remove = presentation::message(
        &locale,
        "remote.allowedHostsRemove",
        &[("host", "tunnel.example.com")],
    );
    scene.click(nodes(&row, &remove)[0].center());
    scene.until("host removed", |application, output| {
        application
            .services
            .state
            .settings
            .read()
            .remote_allowed_hosts
            .is_empty()
            && texts_of(output, "tunnel.example.com").is_empty()
    });
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(scene.application().services.state.paths.settings_file()).unwrap(),
    )
    .unwrap();
    assert_eq!(written["remoteAllowedHosts"], serde_json::json!([]));
}

const SYNCED_THEME: &str = "synced-theme";
const SYNCED_THEME_NAME: &str = "Synced Theme";
const SYNCED_ACCENT: &str = "#abcdef";
const BUILTIN_DARK: &str = "taide-dark";

fn theme_payload() -> String {
    let theme = serde_json::json!({
        "version": taide_model::theme::THEME_SCHEMA_VERSION,
        "id": SYNCED_THEME,
        "name": SYNCED_THEME_NAME,
        "type": "dark",
        "extends": BUILTIN_DARK,
        "colors": { "app.accent": SYNCED_ACCENT },
    });
    serde_json::json!({
        "schemaVersion": SETTINGS_SCHEMA_VERSION,
        "updatedAt": NEW,
        "settings": { "themeId": SYNCED_THEME },
        "themes": [{ "id": SYNCED_THEME, "json": theme.to_string() }],
        "locales": [],
    })
    .to_string()
}

#[test]
fn 실제_sync_다운로드는_새_테마와_테마_설정을_적용해_테마_목록과_화면_테마와_settings_json_탭을_갱신한다()
 {
    use taide_model::layout::PaneNode;
    use taide_native_editor::document::DocumentKey;
    use taide_native_ui::commands::ShellMutation;

    let keychain = Arc::new(Keychain::default());
    keychain.values.lock().unwrap().insert("github", PAT.into());
    let gist = Arc::new(FakeGist::new(NEW, FONT));
    *gist.fetched.lock().unwrap() = Ok((NEW.into(), theme_payload()));
    let mut scene = sync_scene(&keychain, &gist, |state| {
        let mut settings = state.settings.write();
        settings.sync_gist_id = Some(GIST.into());
        settings.sync_last_synced_at = Some(NEW.into());
    });
    scene.ports(Ports {
        gist: Gist(gist.clone()),
        agents: missing_cli_agents(),
        ..Default::default()
    });

    let settings_tab = {
        let application = scene.application();
        let state = &application.services.state;
        let project = state.session.read().active_project.clone().unwrap();
        let layouts = state.layouts.read();
        let PaneNode::Leaf { active, .. } = &layouts[&project].root else {
            panic!("default layout is one pane");
        };
        active.clone().expect("settings tab is active")
    };

    let open = scene.text("app.openSettingsFile");
    let output = scene.until("settings file button", |_, output| {
        !nodes(output, &open).is_empty()
    });
    let rect = *nodes(&output, &open).first().unwrap();
    scene.click(rect.center());
    let app_file = DocumentKey::AppFile(taide_model::app::AppFileTarget::Settings);
    scene.until("settings file tab", |application, _| {
        application.store.documents().find(&app_file).is_some()
    });
    let file_tab = {
        let application = scene.application();
        let state = &application.services.state;
        let project = state.session.read().active_project.clone().unwrap();
        let layouts = state.layouts.read();
        let PaneNode::Leaf { active, .. } = &layouts[&project].root else {
            panic!("default layout is one pane");
        };
        active.clone().expect("settings file tab is active")
    };

    assert!(
        scene
            .application()
            .controller
            .submit(ShellMutation::ActivateTab(settings_tab))
            .is_ok()
    );
    let download = scene.text("settings.syncDownloadNow");
    scene.until("download button", |_, output| {
        !nodes(output, &download).is_empty()
    });
    click_label(&mut scene, &download);
    let success = scene.text("settings.syncDownloadSuccess");
    scene.until("downloaded", |application, _| {
        successes(application).contains(&success)
            && application.services.state.settings.read().theme_id == SYNCED_THEME
    });
    assert!(
        scene
            .application()
            .services
            .state
            .paths
            .themes_dir()
            .join("synced-theme.json")
            .exists()
    );
    scene.until("theme applied", |application, _| {
        application.settings_state.resolved_theme.id == SYNCED_THEME
            && !application.settings_state.presentation_failures.theme
    });
    scene.until("catalog", |application, _| {
        application
            .settings_state
            .settings_views
            .inspection()
            .values()
            .any(|view| {
                view.catalog.as_ref().is_some_and(|catalog| {
                    catalog
                        .themes
                        .as_ref()
                        .is_ok_and(|themes| themes.iter().any(|theme| theme.id == SYNCED_THEME))
                })
            })
    });
    assert!(
        scene
            .application()
            .controller
            .submit(ShellMutation::ActivateTab(file_tab))
            .is_ok()
    );
    scene.until("settings json tab", |application, _| {
        let documents = application.store.documents();
        let Some(document) = documents.find(&app_file) else {
            return false;
        };
        let Ok(snapshot) = documents.snapshot(document) else {
            return false;
        };
        serde_json::from_str::<serde_json::Value>(&snapshot.rope.to_string())
            .ok()
            .and_then(|value| value["themeId"].as_str().map(|id| id == SYNCED_THEME))
            .unwrap_or(false)
    });
}
