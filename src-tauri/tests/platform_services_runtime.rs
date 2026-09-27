use std::path::Path;
use std::sync::{Arc, Mutex};

use taide_lib::platform::services::TauriPlatformServices;
use taide_model::app_event::AppEvent;
use taide_model::error::AppResult;
use taide_runtime::{EventSink, PlatformServices, PlatformServicesState};

#[derive(Default)]
struct RecordingPlatform(Mutex<Vec<&'static str>>);

#[test]
fn 플랫폼_어댑터는_이벤트_포트를_소유하고_기존_매핑을_재사용한다() {
    let _: fn(&TauriPlatformServices, AppEvent) = <TauriPlatformServices as EventSink>::publish;
    let adapter = include_str!("../src/platform/services.rs");
    assert!(adapter.contains("TauriEventSink(&self.0).publish(event);"));
}

impl PlatformServices for RecordingPlatform {
    fn open_path(&self, _path: &Path) -> AppResult<()> {
        self.0.lock().expect("호출 기록 잠금").push("open_path");
        Ok(())
    }

    fn reveal_item_in_dir(&self, _path: &Path) -> AppResult<()> {
        self.0.lock().expect("호출 기록 잠금").push("reveal_item_in_dir");
        Ok(())
    }

    fn open_url(&self, _url: &str) -> AppResult<()> {
        self.0.lock().expect("호출 기록 잠금").push("open_url");
        Ok(())
    }

    fn send_notification(&self, _title: &str, _body: &str) -> AppResult<()> {
        self.0.lock().expect("호출 기록 잠금").push("send_notification");
        Ok(())
    }
}

#[test]
fn 플랫폼_포트_복제본은_동일한_구현을_공유한다() {
    let implementation = Arc::new(RecordingPlatform::default());
    let platform = PlatformServicesState::new(implementation.clone());
    let legacy_platform = platform.clone();

    legacy_platform.0.open_path(Path::new("/tmp/example")).expect("경로 열기");
    platform.0.reveal_item_in_dir(Path::new("/tmp/example")).expect("항목 표시");
    legacy_platform.0.open_url("https://example.com").expect("URL 열기");
    platform.0.send_notification("안전한 제목", "안전한 본문").expect("알림 전달");

    assert_eq!(
        implementation.0.lock().expect("호출 기록 잠금").as_slice(),
        ["open_path", "reveal_item_in_dir", "open_url", "send_notification"]
    );
}

#[test]
fn 알림_명령은_마스킹과_focus_gate_뒤에_플랫폼에_전달한다() {
    let commands = include_str!("../src/domain/notification/commands.rs");
    let actions = include_str!("../../crates/taide-runtime/src/notification_actions.rs");
    let notify = actions.split_once("pub fn notification_notify<F>(").unwrap().1;

    assert!(notify.find("masked_notification_text(title, body)").unwrap() < notify.find("state.settings.read().clone()").unwrap());
    assert!(notify.find("state.settings.read().clone()").unwrap() < notify.find("has_focused_window()").unwrap());
    assert!(
        notify.find("has_focused_window()").unwrap()
            < notify
                .find("service::decide_delivery(&settings, category, any_window_focused)")
                .unwrap()
    );
    assert!(
        notify
            .find("service::decide_delivery(&settings, category, any_window_focused)")
            .unwrap()
            < notify.find("platform.send_notification(&title, &body)").unwrap()
    );
    assert!(commands.contains("notification_actions::notification_notify(&state, platform.0.as_ref(), category, &title, &body, ||"));
    assert!(commands.contains("app.webview_windows().values().any(|window| window.is_focused().unwrap_or(false))"));
    assert!(!commands.contains("state.settings.read()"));
    assert!(!commands.contains("app.notification()"));
    assert!(commands.contains("platform.0.open_url(crate::constants::MACOS_NOTIFICATION_SETTINGS_URL)"));
}

#[test]
fn 시스템_명령은_입력_검증_뒤_플랫폼_포트를_사용한다() {
    let commands = include_str!("../src/domain/system/commands.rs");
    let actions = include_str!("../../crates/taide-runtime/src/system_actions.rs");
    let adapter = include_str!("../src/platform/services.rs");
    let open_path = actions.split_once("pub fn system_open_path(").unwrap().1;
    let open_path = open_path.split_once("pub fn system_reveal_path(").unwrap().0;
    let external_url = actions.split_once("pub fn system_open_external_url(").unwrap().1;
    let external_url = external_url.split_once("pub fn system_open_app_data_path(").unwrap().0;

    assert!(actions.contains("root_guard::resolve_owning_project(&projects, Path::new(path))?"));
    assert!(
        open_path.find("resolve_within_open_project(state, path)?").unwrap() < open_path.find("platform.open_path(&resolved)").unwrap()
    );
    assert!(external_url.find("validate_external_url(url)?").unwrap() < external_url.find("platform.open_url(&validated)").unwrap());
    assert!(actions.contains("platform.reveal_item_in_dir(&resolved)"));
    assert!(actions.contains("platform.open_url(&file_url(&resolved))"));
    for action in [
        "system_open_path",
        "system_reveal_path",
        "system_open_in_browser",
        "system_open_external_url",
        "system_open_app_data_path",
    ] {
        assert!(commands.contains(&format!("system_actions::{action}(")));
    }
    assert!(!commands.contains("resolve_within_open_project"));
    assert!(!commands.contains("tauri_plugin_opener::"));
    assert!(adapter.contains("tauri_plugin_opener::open_path("));
    assert!(adapter.contains("tauri_plugin_opener::reveal_item_in_dir("));
    assert!(adapter.contains("tauri_plugin_opener::open_url("));
}
