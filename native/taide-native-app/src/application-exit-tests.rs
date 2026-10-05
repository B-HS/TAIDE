use std::path::PathBuf;
use std::time::Duration;

use taide_ide::store::{PendingDiff, PendingSave};
use taide_model::ide::IdeDiffOutcome;
use taide_model::paths::AppPaths;
use tokio::net::TcpStream;

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _event: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    marker: PathBuf,
    services: Arc<AppServices>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        crate::remote_http::stop(&self.services);
        crate::agent_hooks::stop(&self.services);
        crate::ide_server::stop(&self.services);
        self.services.tasks.stop_all();
        let _ = std::fs::remove_file(&self.marker);
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn 실제_app_exit는_서버_세션_pending_lockfile과_감독_소유권을_종료한다() {
    tokio::time::timeout(DEADLINE, async {
        for direct_failure in [false, true] {
            let identity = ProjectId::new();
            let directory = std::env::temp_dir().join(format!("taide-native-app-exit-{identity}"));
            std::fs::create_dir_all(&directory).unwrap();
            let marker = std::env::temp_dir().join(format!("taide-wait-{identity}"));
            std::fs::write(&marker, b"synthetic wait marker").unwrap();
            let fixture = Fixture {
                services: bootstrap::services(
                    AppState::new(AppPaths::new(directory.clone())),
                    TaskSupervisor::new(tokio::runtime::Handle::current()),
                    Arc::new(Sink),
                ),
                directory,
                marker,
            };
            let services = &fixture.services;
            services
                .agents
                .register_wait_marker(fixture.marker.to_str().unwrap().into());
            let search = services.search.begin("synthetic", "exit");
            services.state.settings.write().ide_integration_enabled = true;
            let ide_ports = Arc::new(crate::ide_server::Ports::with_test_directory(
                crate::ide_tools::LayoutActions {
                    open_file_tab: |_, _, _, _, _| {
                        Box::pin(async { panic!("exit fixture must not open a tab") })
                    },
                    close_tab: Arc::new(|_, _| {
                        Box::pin(async { panic!("exit fixture must not close a tab") })
                    }),
                },
            ));
            let ide = crate::ide_server::start(services.clone(), ide_ports)
                .await
                .unwrap();
            let hooks = crate::agent_hooks::ensure_started(services.clone())
                .await
                .unwrap();
            let remote = crate::remote_http::start(
                services.clone(),
                Arc::new(crate::remote_http::Ports {
                    assets: Arc::new(|_| panic!("exit fixture must not resolve an asset")),
                    socket: Arc::new(|_, _, _| {
                        Box::pin(async { panic!("exit fixture must not dispatch a socket") })
                    }),
                }),
            )
            .await
            .unwrap();
            let lockfile =
                taide_ide::lockfile::lockfile_path(&fixture.directory.join("ide"), ide.port);
            assert!(lockfile.is_file());
            let session = services.remote.issue_session_without_nonce();
            assert!(services.remote.has_active_session(&session));
            let (save, saved) = oneshot::channel();
            services
                .ide
                .insert_pending_save("synthetic-save".into(), PendingSave { responder: save });
            let (diff, rejected) = oneshot::channel();
            services.ide.insert_pending_diff(
                "synthetic-diff".into(),
                PendingDiff {
                    project_id: ProjectId::new(),
                    new_path: fixture.directory.join("synthetic.txt"),
                    responder: diff,
                },
            );
            if direct_failure {
                services.state.begin_shutdown();
                let result = shutdown(
                    services.clone(),
                    None,
                    None,
                    None,
                    Err(AppError::InvalidArgument("synthetic draft failure".into())),
                )
                .await;
                assert!(matches!(
                    finish_direct_exit(services, result).await,
                    Err(AppError::InvalidArgument(message)) if message == "synthetic draft failure"
                ));
            } else {
                let result = shutdown(services.clone(), None, None, None, Ok(Vec::new())).await;
                finish_direct_exit(services, result).await.unwrap();
            }
            assert!(services.state.is_shutting_down());
            assert_eq!(services.tasks.tracked_count(), 0);
            assert!(
                !services.ide.is_running(),
                "actual App exit left IDE state running"
            );
            assert!(services.agent_hooks.server_info().is_none());
            assert!(!services.remote.is_running());
            assert!(!services.remote.has_active_session(&session));
            assert!(!lockfile.exists());
            assert!(!fixture.marker.exists());
            assert!(search.load(std::sync::atomic::Ordering::SeqCst));
            assert!(
                services
                    .tasks
                    .begin_operation("synthetic-after-exit")
                    .is_none()
            );
            assert!(!saved.await.unwrap());
            assert!(matches!(
                rejected.await.unwrap(),
                (IdeDiffOutcome::Rejected, None)
            ));
            for port in [ide.port, u32::from(hooks.port), remote.port] {
                assert!(
                    TcpStream::connect((
                        std::net::Ipv4Addr::LOCALHOST,
                        u16::try_from(port).unwrap()
                    ))
                    .await
                    .is_err()
                );
            }
        }
    })
    .await
    .unwrap();
}
