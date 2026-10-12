use std::path::PathBuf;
use std::time::Duration;

use taide_ide::store::{PendingDiff, PendingSave};
use taide_model::ide::IdeDiffOutcome;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use tokio::net::TcpStream;

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const VIEW_STATE_FAILURE: &str = "synthetic view state failure";
const DRAFT_CONTENT: &str = "unsaved content";

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _event: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    marker: PathBuf,
    services: Arc<AppServices>,
}

#[tokio::test]
async fn 위치_캡처_오류가_있어도_미저장_본문을_mirror에_보존하고_종료_오류를_보고한다() {
    let project = ProjectId::new();
    let tab = TabId::new();
    let directory =
        std::env::temp_dir().join(format!("taide-native-view-state-exit-error-{project}"));
    std::fs::create_dir_all(&directory).unwrap();
    let fixture = Fixture {
        marker: directory.join("unused-marker"),
        services: bootstrap::services(
            AppState::new(AppPaths::new(directory.clone())),
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Sink),
        ),
        directory,
    };
    let services = &fixture.services;
    services.state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: fixture.directory.to_str().unwrap().to_owned(),
            name: "synthetic".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let result = shutdown(
        services.clone(),
        None,
        None,
        None,
        Ok(vec![Draft {
            project: project.clone(),
            target: DraftTarget::Untitled(tab.clone()),
            content: DRAFT_CONTENT.into(),
        }]),
        Err(AppError::InvalidArgument(VIEW_STATE_FAILURE.into())),
    )
    .await;
    assert!(
        matches!(finish_direct_exit(services, result).await, Err(AppError::InvalidArgument(message)) if message == VIEW_STATE_FAILURE)
    );
    let mirrors =
        taide_file::service::list_untitled_mirrors(&services.state.paths, &project).unwrap();
    assert_eq!(
        mirrors.len(),
        1,
        "view state failure must not discard the draft"
    );
    assert_eq!(mirrors[0].tab_id, tab);
    assert_eq!(mirrors[0].content, DRAFT_CONTENT);
    assert!(services.state.is_shutting_down());
    assert_eq!(services.tasks.tracked_count(), 0);
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
                    Ok(Arc::from([])),
                )
                .await;
                assert!(matches!(
                    finish_direct_exit(services, result).await,
                    Err(AppError::InvalidArgument(message)) if message == "synthetic draft failure"
                ));
            } else {
                let result = shutdown(
                    services.clone(),
                    None,
                    None,
                    None,
                    Ok(Vec::new()),
                    Ok(Arc::from([])),
                )
                .await;
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
