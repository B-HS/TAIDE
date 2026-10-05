use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use taide_infra::secret::{SecretAccount, SecretStore, SecretStoreState};
use taide_model::paths::AppPaths;
use taide_runtime::{
    AppState, EventSink, IdeSaveFile, PlatformServicesState, RemoteDispatchLimiter, TaskSupervisor,
    save_file_within_open_projects,
};
use tokio::sync::Notify;

use super::*;

const DEADLINE: Duration = Duration::from_secs(5);
const REMOTE_CONCURRENT: usize = 1;
const SYNTHETIC_IDE_PORT: u32 = 12345;

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
struct Events(Mutex<Vec<AppEvent>>);

impl EventSink for Events {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

struct Fixture {
    directory: PathBuf,
    root: String,
    services: Arc<AppServices>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-project-hooks-{}", ProjectId::new()));
        std::fs::create_dir_all(directory.join("project")).unwrap();
        let directory = directory.canonicalize().unwrap();
        let root = directory.join("project").to_str().unwrap().to_owned();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.settings.write().agent_hooks_enabled = true;
        state.settings.write().ide_integration_enabled = false;
        let services = Arc::new(AppServices::new(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            RemoteDispatchLimiter::new(REMOTE_CONCURRENT),
            PlatformServicesState::new(Arc::new(crate::bootstrap::NativePlatform)),
            SecretStoreState(Arc::new(Secrets)),
            IdeSaveFile(save_file_within_open_projects),
            Arc::new(Events::default()),
        ));
        Self {
            directory,
            root,
            services,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.state.watchers.write().clear();
        self.services.state.git_watchers.write().clear();
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[derive(Default)]
struct Control {
    entered: Notify,
    release: Notify,
    calls: AtomicUsize,
    finished: AtomicUsize,
}

#[tokio::test]
async fn hooks_enabled_프로젝트는_거절없이_열리고_guard밖_reconcile을_기다리지_않는다() {
    let fixture = Fixture::new();
    let control = Arc::new(Control::default());
    let worker_control = control.clone();
    let expected = fixture.services.clone();
    let projects = NativeProjects::with_hook_reconcile(
        fixture.services.clone(),
        Arc::new(move |services| {
            let control = worker_control.clone();
            assert!(Arc::ptr_eq(&services, &expected));
            Box::pin(async move {
                {
                    let _guard = services.state.begin_mutation().await;
                    assert!(services.state.settings.read().agent_hooks_enabled);
                    assert_eq!(services.state.projects.read().len(), 1);
                    control.calls.fetch_add(1, Ordering::AcqRel);
                }
                control.entered.notify_one();
                control.release.notified().await;
                control.finished.fetch_add(1, Ordering::AcqRel);
            })
        }),
    );
    let project = tokio::time::timeout(DEADLINE, projects.open(fixture.root.clone()))
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(DEADLINE, control.entered.notified())
        .await
        .unwrap();
    assert_eq!(control.calls.load(Ordering::Acquire), 1);
    assert_eq!(control.finished.load(Ordering::Acquire), 0);
    assert!(
        fixture
            .services
            .state
            .watchers
            .read()
            .contains_key(&project.id)
    );
    assert!(
        fixture
            .services
            .state
            .layouts
            .read()
            .contains_key(&project.id)
    );
    assert_eq!(fixture.services.tasks.tracked_count(), 1);
    let reopened = projects.open(fixture.root.clone()).await.unwrap();
    assert_eq!(reopened.id, project.id);
    assert_eq!(control.calls.load(Ordering::Acquire), 1);
    tokio::time::timeout(DEADLINE, fixture.services.tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(control.finished.load(Ordering::Acquire), 0);
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
}

#[tokio::test]
async fn hooks_off_생산용_factory는_서버없이_restore하고_종료후_새_attach를_거절한다() {
    let fixture = Fixture::new();
    fixture.services.state.settings.write().agent_hooks_enabled = false;
    let projects = NativeProjects::new(fixture.services.clone());
    let project = projects.open(fixture.root.clone()).await.unwrap();
    projects.restore_watchers().await.unwrap();
    assert!(fixture.services.agent_hooks.server_info().is_none());
    assert_eq!(fixture.services.state.watchers.read().len(), 1);
    assert_eq!(fixture.services.state.layouts.read().len(), 1);
    project_actions::project_close(
        fixture.services.events.as_ref(),
        &fixture.services.state,
        &projects,
        project.id,
    )
    .await
    .unwrap();
    assert!(fixture.services.state.watchers.read().is_empty());
    assert!(fixture.services.state.layouts.read().is_empty());
    tokio::time::timeout(DEADLINE, fixture.services.tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(fixture.services.tasks.tracked_count(), 0);
    let error = projects.open(fixture.root.clone()).await.unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
    assert!(fixture.services.state.projects.read().is_empty());
}

#[tokio::test]
async fn 원본처럼_watcher와_lockfile_준비_실패는_프로젝트_commit을_거절하지_않는다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        fixture.services.state.settings.write().agent_hooks_enabled = false;
        let projects = NativeProjects::new(fixture.services.clone());
        let project = Project {
            id: ProjectId::new(),
            root: fixture.root.clone(),
            name: "synthetic disappearing root".into(),
            capabilities: vec![CapabilityKind::Terminal],
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        };
        fixture
            .services
            .state
            .projects
            .write()
            .insert(project.id.clone(), project.clone());
        std::fs::remove_dir(&fixture.root).unwrap();
        projects
            .attach_project_capabilities(&project)
            .await
            .unwrap();
        assert!(
            fixture
                .services
                .state
                .projects
                .read()
                .contains_key(&project.id)
        );
        assert!(
            fixture
                .services
                .state
                .layouts
                .read()
                .contains_key(&project.id)
        );
        assert!(fixture.services.state.watchers.read().is_empty());
        assert!(fixture.services.state.git_watchers.read().is_empty());
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);

        let fixture = Fixture::new();
        fixture.services.state.settings.write().agent_hooks_enabled = false;
        let blocked = fixture.directory.join("blocked-ide-directory");
        std::fs::write(&blocked, b"synthetic non-directory").unwrap();
        let server = fixture
            .services
            .tasks
            .spawn_transient_handle("synthetic-ide-owner", std::future::pending())
            .unwrap();
        assert!(
            fixture
                .services
                .ide
                .mark_started(SYNTHETIC_IDE_PORT, "synthetic".into(), blocked, server)
                .is_some()
        );
        let projects = NativeProjects::new(fixture.services.clone());
        let project = projects.open(fixture.root.clone()).await.unwrap();
        assert_eq!(project.capabilities, vec![CapabilityKind::Terminal]);
        assert!(
            fixture
                .services
                .state
                .projects
                .read()
                .contains_key(&project.id)
        );
        assert!(
            fixture
                .services
                .state
                .layouts
                .read()
                .contains_key(&project.id)
        );
        assert!(
            fixture
                .services
                .state
                .watchers
                .read()
                .contains_key(&project.id)
        );
        assert_eq!(
            projects.detected_kinds(Path::new(&fixture.root)),
            vec![CapabilityKind::Terminal]
        );
        {
            let _guard = fixture.services.state.begin_mutation().await;
        }
        crate::ide_server::stop(&fixture.services);
        fixture.services.state.stop_watchers();
        fixture.services.state.watcher_stops.wait_for_idle().await;
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}
