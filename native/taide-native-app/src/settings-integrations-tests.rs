use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::Value;
use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::app::AppInfo;
use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::paths::AppPaths;
use taide_model::settings::{Settings, SettingsPatch};
use taide_runtime::{
    AppState, EventSink, IdeSaveFile, PlatformServicesState, RemoteDispatchLimiter, TaskSupervisor,
    save_file_within_open_projects, settings_actions,
};
use tokio::sync::Notify;

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const QUEUE_BYTES: usize = 256 * 1024;
const QUEUE_COUNT: usize = 64;
const QUEUE_VISITS: usize = 4096;

struct Secrets;

impl SecretStore for Secrets {
    fn set(&self, _: SecretAccount, _: &str) -> AppResult<()> {
        panic!("secret writes are outside this test")
    }

    fn get(&self, _: SecretAccount) -> AppResult<Option<String>> {
        panic!("secret reads are outside this test")
    }

    fn delete(&self, _: SecretAccount) -> AppResult<()> {
        panic!("secret deletes are outside this test")
    }
}

#[derive(Default)]
struct Control {
    trace: Mutex<Vec<&'static str>>,
    entered: Notify,
    release: Notify,
}

impl EventSink for Control {
    fn publish(&self, event: AppEvent) {
        assert!(matches!(event, AppEvent::SettingsChanged { .. }));
        self.trace.lock().unwrap().push("event");
    }
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    control: Arc<Control>,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "taide-native-settings-integrations-{}",
            ProjectId::new()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState::new(AppPaths::new(directory.clone()));
        {
            let mut settings = state.settings.write();
            settings.ide_integration_enabled = false;
            settings.agent_hooks_enabled = false;
            settings.remote_access_enabled = false;
        }
        let control = Arc::new(Control::default());
        let services = Arc::new(AppServices::new(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            RemoteDispatchLimiter::new(1),
            PlatformServicesState::new(Arc::new(crate::bootstrap::NativePlatform)),
            SecretStoreState(Arc::new(Secrets)),
            IdeSaveFile(save_file_within_open_projects),
            control.clone(),
        ));
        Self {
            directory,
            services,
            control,
        }
    }

    fn observer(&self, label: &'static str, paused: bool) -> Toggle {
        let expected = self.services.clone();
        let control = self.control.clone();
        Arc::new(move |services, current, updated| {
            assert!(Arc::ptr_eq(&services, &expected));
            let control = control.clone();
            Box::pin(async move {
                assert!(!current);
                assert!(updated);
                let live = services.state.settings.read().clone();
                assert!(live.ide_integration_enabled);
                assert!(live.agent_hooks_enabled);
                assert!(live.remote_access_enabled);
                let saved: Settings = serde_json::from_slice(
                    &std::fs::read(services.state.paths.settings_file()).unwrap(),
                )
                .unwrap();
                assert_eq!(saved, live);
                control.trace.lock().unwrap().push(label);
                if paused {
                    control.entered.notify_one();
                    control.release.notified().await;
                }
            })
        })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn 실제_persist_live뒤_ide_hooks_remote가_순서대로_끝나야_settings_changed를_보낸다() {
    let fixture = Fixture::new();
    let integrations = Arc::new(Integrations {
        ide: fixture.observer("ide", false),
        hooks: fixture.observer("hooks", true),
        remote: fixture.observer("remote", false),
    });
    let reconcile = integrations.reconcile();
    let worker = fixture.services.clone();
    let mut update = Box::pin(fixture.services.tasks.run_nonabortable_result(
        "synthetic-settings-update",
        async move {
            settings_actions::settings_update(
                &worker.state,
                SettingsPatch {
                    ide_integration_enabled: Some(true),
                    agent_hooks_enabled: Some(true),
                    remote_access_enabled: Some(true),
                    ..Default::default()
                },
                |current, updated| reconcile(worker.clone(), current, updated),
                worker.events.as_ref(),
            )
            .await
        },
    ));
    tokio::time::timeout(DEADLINE, async {
        tokio::select! {
            result = &mut update => panic!("settings returned before hooks completed: {result:?}"),
            _ = fixture.control.entered.notified() => {}
        }
    })
    .await
    .unwrap();
    assert_eq!(*fixture.control.trace.lock().unwrap(), ["ide", "hooks"]);
    let mut mutation = Box::pin(fixture.services.state.begin_mutation());
    assert!(futures_util::poll!(mutation.as_mut()).is_pending());
    drop(mutation);
    drop(update);
    assert_eq!(fixture.services.tasks.tracked_count(), 1);
    fixture.control.release.notify_one();
    tokio::time::timeout(DEADLINE, fixture.services.tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(
        *fixture.control.trace.lock().unwrap(),
        ["ide", "hooks", "remote", "event"],
    );
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn production_remote_dispatch_reconcile_역참조는_cycle없이_drop되고_소유자_종료후_bind하지_않는다()
 {
    let fixture = Fixture::new();
    let hub = Arc::new(
        crate::terminal_host::Hub::new(
            fixture.services.clone(),
            crate::terminal_host::Limits {
                sessions: 1,
                core: Default::default(),
                frames: crate::terminal_frames::Limits {
                    bytes: QUEUE_BYTES,
                    count: QUEUE_COUNT,
                    visits: QUEUE_VISITS,
                },
                writer: crate::terminal_writer::Limits {
                    bytes: QUEUE_BYTES,
                    count: QUEUE_COUNT,
                },
            },
        )
        .unwrap(),
    );
    let hub_owner = Arc::downgrade(&hub);
    let ide = Arc::new(crate::ide_server::Ports::new(
        crate::ide_tools::LayoutActions::new(hub),
        "synthetic-version".into(),
    ));
    let mut integration_owner = Weak::new();
    let remote = Arc::new_cyclic(|remote| {
        let integrations = Arc::new(Integrations::new(ide.clone(), remote.clone()));
        integration_owner = Arc::downgrade(&integrations);
        let remaining = crate::remote_ws::Dispatch {
            json: Arc::new(|_, _, _, _| {
                Box::pin(async {
                    Err::<String, Value>(crate::remote_gateway::error_value(AppError::Forbidden(
                        "synthetic remaining denied".into(),
                    )))
                })
            }),
            raw: Arc::new(|_, _, _| {
                Box::pin(async {
                    Err::<Vec<u8>, Value>(crate::remote_gateway::error_value(AppError::Forbidden(
                        "synthetic remaining denied".into(),
                    )))
                })
            }),
        };
        let dispatch = crate::remote_preferences::extend_backend(
            crate::remote_preferences::Ports {
                info: AppInfo {
                    name: "synthetic".into(),
                    version: "synthetic".into(),
                    platform: "synthetic".into(),
                    arch: "synthetic".into(),
                },
                reconcile: integrations.reconcile(),
            },
            remaining,
        );
        crate::remote_http::Ports {
            socket: crate::remote_ws::socket_action(Arc::new(dispatch)),
            assets: Arc::new(|_| panic!("asset lookup is outside this test")),
        }
    });
    drop(ide);
    let remote_owner = Arc::downgrade(&remote);
    let retained = integration_owner.upgrade().unwrap().reconcile();
    let settings = fixture.services.state.settings.read().clone();
    retained(fixture.services.clone(), settings.clone(), settings.clone()).await;
    let enabled = Settings {
        remote_access_enabled: true,
        ..settings.clone()
    };
    retained(fixture.services.clone(), enabled.clone(), settings.clone()).await;
    assert!(fixture.control.trace.lock().unwrap().is_empty());
    drop(remote);
    assert!(remote_owner.upgrade().is_none());
    retained(fixture.services.clone(), settings, enabled).await;
    assert!(!fixture.services.remote.is_running());
    assert!(fixture.control.trace.lock().unwrap().is_empty());
    drop(retained);
    assert!(integration_owner.upgrade().is_none());
    assert!(hub_owner.upgrade().is_none());
    fixture.services.tasks.shutdown().await;
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}
