use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::ids::ProjectId;
use taide_model::notification::{NotificationCategory, NotificationDelivery, NotificationSuppressionReason};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::system::AppDataPathKind;
use taide_runtime::{notification_actions, system_actions, AppState, PlatformServices};
use uuid::Uuid;

struct Fixture {
    dir: PathBuf,
    root: PathBuf,
    state: AppState,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-os-actions-{}", Uuid::new_v4()));
        let root = dir.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let state = AppState::new(AppPaths::new(dir.join("data")));
        let project_id = ProjectId::new();
        state.projects.write().insert(
            project_id.clone(),
            Project {
                id: project_id,
                root: root.to_string_lossy().into_owned(),
                name: "fixture".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        Self { dir, root, state }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).expect("fixture 정리");
    }
}

#[derive(Default)]
struct RecordingPlatform {
    calls: Mutex<Vec<(&'static str, String, String)>>,
    should_fail: AtomicBool,
}

impl RecordingPlatform {
    fn record(&self, operation: &'static str, primary: &str, secondary: &str) -> AppResult<()> {
        self.calls
            .lock()
            .unwrap()
            .push((operation, primary.to_string(), secondary.to_string()));
        if self.should_fail.load(Ordering::SeqCst) {
            return Err(AppError::Internal("fixture failure".to_string()));
        }
        Ok(())
    }
}

impl PlatformServices for RecordingPlatform {
    fn open_path(&self, path: &Path) -> AppResult<()> {
        self.record("open_path", &path.to_string_lossy(), "")
    }

    fn reveal_item_in_dir(&self, path: &Path) -> AppResult<()> {
        self.record("reveal", &path.to_string_lossy(), if path.is_dir() { "directory" } else { "entry" })
    }

    fn open_url(&self, url: &str) -> AppResult<()> {
        self.record("open_url", url, "")
    }

    fn send_notification(&self, title: &str, body: &str) -> AppResult<()> {
        self.record("notification", title, body)
    }
}

#[tokio::test]
async fn 시스템_경로는_정규화한_루트_내_경로만_전달하며_조회_잠금을_추가하지_않는다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    let file = fixture.root.join("fixture.rs");
    std::fs::write(&file, "fixture").unwrap();
    let path = file.to_string_lossy();
    let guard = fixture.state.begin_mutation().await;
    system_actions::system_open_path(&fixture.state, &platform, &path).unwrap();
    system_actions::system_reveal_path(&fixture.state, &platform, &path).unwrap();
    system_actions::system_open_in_browser(&fixture.state, &platform, &path).unwrap();
    drop(guard);
    assert_eq!(
        *platform.calls.lock().unwrap(),
        [
            ("open_path", path.to_string(), String::new()),
            ("reveal", path.to_string(), "entry".to_string()),
            ("open_url", taide_system::service::file_url(&file), String::new()),
        ]
    );
    let missing = fixture.root.join("not-created.rs");
    system_actions::system_open_path(&fixture.state, &platform, &missing.to_string_lossy()).unwrap();
    assert_eq!(platform.calls.lock().unwrap().last().unwrap().1, missing.to_string_lossy());
}

#[test]
fn 루트_밖과_cli_허용_경로도_시스템_포트로_넘기지_않는다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    let outside = fixture.dir.join("outside.rs");
    std::fs::write(&outside, "fixture").unwrap();
    fixture
        .state
        .cli_opened_paths
        .write()
        .insert(std::fs::canonicalize(&outside).unwrap());
    let path = outside.to_string_lossy();
    assert_eq!(
        system_actions::system_open_path(&fixture.state, &platform, &path)
            .unwrap_err()
            .kind(),
        AppErrorKind::Forbidden
    );
    assert_eq!(
        system_actions::system_reveal_path(&fixture.state, &platform, &path)
            .unwrap_err()
            .kind(),
        AppErrorKind::Forbidden
    );
    assert_eq!(
        system_actions::system_open_in_browser(&fixture.state, &platform, &path)
            .unwrap_err()
            .kind(),
        AppErrorKind::Forbidden
    );
    assert!(platform.calls.lock().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn 루트_밖을_가리키는_symlink는_시스템_포트에_전달하지_않는다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    let outside = fixture.dir.join("outside.rs");
    std::fs::write(&outside, "fixture").unwrap();
    let link = fixture.root.join("link.rs");
    std::os::unix::fs::symlink(outside, &link).unwrap();
    assert_eq!(
        system_actions::system_open_path(&fixture.state, &platform, &link.to_string_lossy())
            .unwrap_err()
            .kind(),
        AppErrorKind::Forbidden
    );
    assert!(platform.calls.lock().unwrap().is_empty());
}

#[test]
fn 외부_url은_기존_gate_검증과_트림_뒤에만_포트에_전달된다() {
    let platform = RecordingPlatform::default();
    for url in [
        "file:///fixture",
        "javascript:fixture",
        "https://example.invalid/a b",
        "https://trusted.invalid@other.invalid",
        "https://exa\u{202E}mple.invalid",
    ] {
        assert!(system_actions::system_open_external_url(&platform, url).is_err());
    }
    assert!(platform.calls.lock().unwrap().is_empty());
    system_actions::system_open_external_url(&platform, "  HTTPS://example.invalid/path@2x?q=a@b  ").unwrap();
    assert_eq!(
        *platform.calls.lock().unwrap(),
        [("open_url", "HTTPS://example.invalid/path@2x?q=a@b".to_string(), String::new())]
    );
}

#[test]
fn app_data는_허용된_네_경로를_생성한_뒤에_reveal한다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    for (kind, path) in [
        (AppDataPathKind::Plugins, fixture.state.paths.plugins_dir()),
        (AppDataPathKind::Themes, fixture.state.paths.themes_dir()),
        (AppDataPathKind::Locales, fixture.state.paths.locales_dir()),
        (AppDataPathKind::Snippets, fixture.state.paths.snippets_dir()),
    ] {
        assert!(!path.exists());
        system_actions::system_open_app_data_path(&fixture.state, &platform, kind).unwrap();
        assert!(path.is_dir());
        assert_eq!(
            platform.calls.lock().unwrap().last().unwrap(),
            &("reveal", path.to_string_lossy().into_owned(), "directory".to_string())
        );
    }
}

#[test]
fn app_data_디렉터리_생성_실패는_포트를_호출하지_않는다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    std::fs::create_dir_all(&fixture.state.paths.data_dir).unwrap();
    std::fs::write(fixture.state.paths.plugins_dir(), "fixture").unwrap();
    assert!(system_actions::system_open_app_data_path(&fixture.state, &platform, AppDataPathKind::Plugins).is_err());
    assert!(platform.calls.lock().unwrap().is_empty());
}

#[test]
fn 알림은_설정_snapshot_뒤_focus를_확인하고_마스킹한_문자열만_전달한다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    let token = format!("ghp_{}", Uuid::new_v4().simple());
    let title = format!("fixture {token}");
    let body = format!("https://fixture:{token}@example.invalid/");
    let called = AtomicBool::new(false);
    let decision = notification_actions::notification_notify(&fixture.state, &platform, NotificationCategory::Error, &title, &body, || {
        fixture.state.settings.write().notifications_enabled = false;
        called.store(true, Ordering::SeqCst);
        false
    })
    .unwrap();
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(decision, NotificationDelivery::Delivered);
    assert_eq!(
        *platform.calls.lock().unwrap(),
        [(
            "notification",
            "fixture [redacted:github]".to_string(),
            "https://fixture:[redacted:url_password]@example.invalid/".to_string()
        )]
    );
    assert!(!fixture.state.settings.read().notifications_enabled);
}

#[test]
fn 알림_억제_사유와_focus_확인_및_live_설정을_보존한다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    fixture.state.settings.write().notifications_enabled = false;
    let called = AtomicBool::new(false);
    let decision =
        notification_actions::notification_notify(&fixture.state, &platform, NotificationCategory::Error, "fixture", "body", || {
            called.store(true, Ordering::SeqCst);
            true
        })
        .unwrap();
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(
        decision,
        NotificationDelivery::Suppressed(NotificationSuppressionReason::NotificationsDisabled)
    );
    fixture.state.settings.write().notifications_enabled = true;
    fixture.state.settings.write().notify_error = false;
    assert_eq!(
        notification_actions::notification_notify(&fixture.state, &platform, NotificationCategory::Error, "fixture", "body", || true)
            .unwrap(),
        NotificationDelivery::Suppressed(NotificationSuppressionReason::CategoryDisabled)
    );
    fixture.state.settings.write().notify_error = true;
    assert_eq!(
        notification_actions::notification_notify(&fixture.state, &platform, NotificationCategory::Error, "fixture", "body", || true)
            .unwrap(),
        NotificationDelivery::Suppressed(NotificationSuppressionReason::WindowFocused)
    );
    assert!(platform.calls.lock().unwrap().is_empty());
    fixture.state.settings.write().notifications_only_when_unfocused = false;
    assert_eq!(
        notification_actions::notification_notify(&fixture.state, &platform, NotificationCategory::Error, "fixture", "body", || true)
            .unwrap(),
        NotificationDelivery::Delivered
    );
    assert_eq!(
        *platform.calls.lock().unwrap(),
        [("notification", "fixture".to_string(), "body".to_string())]
    );
}

#[test]
fn 시스템과_알림의_포트_실패를_성공으로_바꾸지_않는다() {
    let fixture = Fixture::new();
    let platform = RecordingPlatform::default();
    platform.should_fail.store(true, Ordering::SeqCst);
    assert_eq!(
        system_actions::system_open_path(&fixture.state, &platform, &fixture.root.to_string_lossy())
            .unwrap_err()
            .kind(),
        AppErrorKind::Internal
    );
    assert_eq!(
        system_actions::system_open_external_url(&platform, "https://example.invalid")
            .unwrap_err()
            .kind(),
        AppErrorKind::Internal
    );
    assert_eq!(
        system_actions::system_open_app_data_path(&fixture.state, &platform, AppDataPathKind::Themes)
            .unwrap_err()
            .kind(),
        AppErrorKind::Internal
    );
    assert_eq!(
        notification_actions::notification_notify(&fixture.state, &platform, NotificationCategory::Error, "fixture", "body", || false)
            .unwrap_err()
            .kind(),
        AppErrorKind::Internal
    );
}

#[test]
fn command는_runtime_정책에_위임하고_실제_focus와_os_설정_adapter만_보존한다() {
    let system = include_str!("../src/domain/system/commands.rs");
    let notification = include_str!("../src/domain/notification/commands.rs");
    for name in [
        "system_open_path",
        "system_reveal_path",
        "system_open_in_browser",
        "system_open_external_url",
        "system_open_app_data_path",
    ] {
        assert!(system.contains(&format!("system_actions::{name}(")));
    }
    assert!(!system.contains("fn resolve_within_open_project("));
    assert!(notification.contains("notification_actions::notification_notify("));
    assert!(notification.contains("app.webview_windows().values().any(|window| window.is_focused().unwrap_or(false))"));
    assert!(notification.contains("platform.0.open_url(crate::constants::MACOS_NOTIFICATION_SETTINGS_URL)"));
    assert!(!notification.contains("fn masked_notification_text("));
    assert!(!notification.contains("state.settings.read()"));
}
