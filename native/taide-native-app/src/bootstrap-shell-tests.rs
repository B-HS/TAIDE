use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::project::WindowChromePatch;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::controller::ShellConnection;

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Paint(Mutex<Vec<AppEvent>>);

impl EventSink for Paint {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Directory(PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn 실제_app_shell과_services_두_발행자는_relay_paint_snapshot을_한번씩_거친다() {
    let directory = Directory(
        std::env::temp_dir().join(format!("taide-native-bootstrap-shell-{}", ProjectId::new())),
    );
    std::fs::create_dir_all(&directory.0).unwrap();
    let state = AppState::new(AppPaths::new(directory.0.clone()));
    let paint = Arc::new(Paint::default());
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let (assembly, connection) =
        connect_shell(state, tasks.clone(), paint.clone(), Arc::new(|| {}))
            .await
            .unwrap();
    let ShellConnection {
        mut controller,
        events: shell_events,
        worker,
    } = connection;
    let services = &assembly.services;
    let mut remote = services.remote.subscribe_events();
    controller
        .submit(ShellMutation::SetWindowChrome(WindowChromePatch {
            zen: Some(true),
            ..Default::default()
        }))
        .unwrap();
    let snapshot = tokio::time::timeout(DEADLINE, controller.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(snapshot.shell.window_chrome.zen);
    assert!(controller.take_error().is_none());
    assert!(matches!(
        paint.0.lock().unwrap().as_slice(),
        [AppEvent::WindowChromeChanged { chrome }] if chrome.zen
    ));
    let chrome: Value = serde_json::from_str(&remote.try_recv().unwrap()).unwrap();
    assert_eq!(chrome["event"], "session:window-chrome-changed");
    let chrome_payload: Value = serde_json::from_str(chrome["payload"].as_str().unwrap()).unwrap();
    assert_eq!(chrome_payload["chrome"]["zen"], true);
    services.events.publish(AppEvent::ThemeChanged {
        theme_id: "synthetic".into(),
    });
    tokio::time::timeout(DEADLINE, controller.changed())
        .await
        .unwrap()
        .unwrap();
    let theme: Value = serde_json::from_str(&remote.try_recv().unwrap()).unwrap();
    assert_eq!(theme["event"], "theme:changed");
    assert!(matches!(
        remote.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    assert!(matches!(
        paint.0.lock().unwrap().as_slice(),
        [
            AppEvent::WindowChromeChanged { .. },
            AppEvent::ThemeChanged { .. }
        ]
    ));
    let owner = Arc::downgrade(services);
    drop(controller);
    drop(shell_events);
    tokio::time::timeout(DEADLINE, worker)
        .await
        .unwrap()
        .unwrap();
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
    drop(assembly);
    assert!(owner.upgrade().is_none());
}
