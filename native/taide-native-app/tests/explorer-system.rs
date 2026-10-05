use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::{platform_url, services};
use taide_native_app::host::{ClipboardWriter, HostBridge, HostCommand, HostReply};
use taide_runtime::{AppState, EventSink, PlatformServices, PlatformServicesState, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(5);

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

#[derive(Default)]
struct Platform(Mutex<Vec<(String, String)>>);
impl PlatformServices for Platform {
    fn open_path(&self, _: &Path) -> AppResult<()> {
        panic!("unexpected open-path port")
    }
    fn reveal_item_in_dir(&self, path: &Path) -> AppResult<()> {
        self.0
            .lock()
            .unwrap()
            .push(("reveal".into(), path.to_str().unwrap().into()));
        if path.file_name().unwrap() == "fail.html" {
            return Err(AppError::Internal("synthetic reveal refusal".into()));
        }
        Ok(())
    }
    fn open_url(&self, url: &str) -> AppResult<()> {
        self.0
            .lock()
            .unwrap()
            .push(("browser".into(), platform_url(url)?));
        Ok(())
    }
    fn send_notification(&self, _: &str, _: &str) -> AppResult<()> {
        panic!("unexpected notification port")
    }
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("taide-native-explorer-system-{}", ProjectId::new()));
        std::fs::create_dir_all(path.join("root")).unwrap();
        std::fs::write(path.join("root/文 #100%.HTML"), "synthetic html").unwrap();
        std::fs::write(path.join("root/fail.html"), "synthetic failure").unwrap();
        std::fs::write(path.join("outside.html"), "outside unchanged").unwrap();
        Self(std::fs::canonicalize(path).unwrap())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

async fn reply(bridge: &mut HostBridge, signal: &Notify) -> HostReply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            signal.notified().await;
        }
    })
    .await
    .unwrap()
}

#[test]
fn 파일_url_port은_실제경로를_보존하고_외부scheme_제한을_유지한다() {
    let path = Path::new("/synthetic/文 #100%?.html");
    let encoded = platform_url("file:///synthetic/文 #100%?.html").unwrap();
    assert_eq!(encoded, "file:///synthetic/%E6%96%87%20%23100%25%3F.html");
    assert_eq!(
        url::Url::parse(&encoded).unwrap().to_file_path().unwrap(),
        path
    );
    assert_eq!(
        platform_url("https://example.com/file.html").unwrap(),
        "https://example.com/file.html"
    );
    for refused in [
        "file://relative.html",
        "javascript:alert(1)",
        "https://trusted.example@evil.example/",
    ] {
        assert!(platform_url(refused).is_err());
    }
    assert!(taide_infra::external_url::validate_external_url(&encoded).is_err());
}

#[test]
fn 경로메뉴_host는_검증된_platform_호출_복사실패와_worker_회수를_보존한다() {
    let fixture = Fixture::new();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let state = AppState::new(AppPaths::new(fixture.0.join("data")));
            let project = ProjectId::new();
            let root = fixture.0.join("root");
            state.projects.write().insert(
                project.clone(),
                Project {
                    id: project,
                    root: root.to_str().unwrap().into(),
                    name: "synthetic explorer system".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let platform = Arc::new(Platform::default());
            let mut services = services(state.clone(), tasks.clone(), Arc::new(Sink));
            Arc::get_mut(&mut services).unwrap().platform =
                PlatformServicesState::new(platform.clone());
            let copies = Arc::new(Mutex::new(Vec::<String>::new()));
            let captured_copies = copies.clone();
            let owner = Arc::new(());
            let released = Arc::downgrade(&owner);
            let clipboard: ClipboardWriter = Arc::new(move |text| {
                assert!(Arc::strong_count(&owner) >= 1);
                captured_copies.lock().unwrap().push(text.into());
                if text == "deny" {
                    return Err(AppError::Internal("synthetic clipboard refusal".into()));
                }
                Ok(())
            });
            let signal = Arc::new(Notify::new());
            let ready = signal.clone();
            let mut bridge = HostBridge::connect_with_clipboard(
                services,
                Arc::new(move || ready.notify_one()),
                clipboard,
            )
            .unwrap();
            for text in ["文 #100%.HTML", "deny", "last successful copy"] {
                bridge.submit(HostCommand::CopyText(text.into())).unwrap();
                let HostReply::CopiedText { result } = reply(&mut bridge, &signal).await else {
                    panic!("copy reply");
                };
                assert_eq!(result.is_ok(), text != "deny");
            }
            assert_eq!(
                *copies.lock().unwrap(),
                vec!["文 #100%.HTML", "deny", "last successful copy"]
            );
            let path = root.join("文 #100%.HTML");
            for command in [
                HostCommand::RevealPath(path.to_str().unwrap().into()),
                HostCommand::OpenInBrowser(path.to_str().unwrap().into()),
            ] {
                bridge.submit(command).unwrap();
                let HostReply::SystemFinished { result } = reply(&mut bridge, &signal).await else {
                    panic!("system reply");
                };
                result.unwrap();
            }
            let calls = platform.0.lock().unwrap().clone();
            assert_eq!(calls[0], ("reveal".into(), path.to_str().unwrap().into()));
            assert_eq!(calls[1].0, "browser");
            assert!(calls[1].1.ends_with("/%E6%96%87%20%23100%25.HTML"));
            assert_eq!(
                url::Url::parse(&calls[1].1)
                    .unwrap()
                    .to_file_path()
                    .unwrap(),
                path
            );
            for command in [
                HostCommand::RevealPath(fixture.0.join("outside.html").to_str().unwrap().into()),
                HostCommand::OpenInBrowser("relative.html".into()),
            ] {
                bridge.submit(command).unwrap();
                let HostReply::SystemFinished { result } = reply(&mut bridge, &signal).await else {
                    panic!("refused system reply");
                };
                assert!(result.is_err());
            }
            assert_eq!(platform.0.lock().unwrap().len(), calls.len());
            #[cfg(unix)]
            {
                let alias = root.join("escape.html");
                std::os::unix::fs::symlink(fixture.0.join("outside.html"), &alias).unwrap();
                bridge
                    .submit(HostCommand::OpenInBrowser(alias.to_str().unwrap().into()))
                    .unwrap();
                let HostReply::SystemFinished { result } = reply(&mut bridge, &signal).await else {
                    panic!("alias refusal");
                };
                assert!(result.is_err());
                assert_eq!(platform.0.lock().unwrap().len(), calls.len());
            }
            bridge
                .submit(HostCommand::RevealPath(
                    root.join("fail.html").to_str().unwrap().into(),
                ))
                .unwrap();
            let HostReply::SystemFinished { result } = reply(&mut bridge, &signal).await else {
                panic!("platform failure reply");
            };
            assert!(result.is_err());
            bridge.disconnect().await.unwrap();
            assert!(released.upgrade().is_none());
            assert_eq!(std::fs::read_to_string(path).unwrap(), "synthetic html");
            assert_eq!(
                std::fs::read_to_string(fixture.0.join("outside.html")).unwrap(),
                "outside unchanged"
            );
        });
}
