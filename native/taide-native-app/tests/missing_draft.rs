use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::missing_draft::{self, MissingDraft};
use taide_runtime::{
    AppServices, AppState, EventSink, TaskSupervisor, file_actions, layout_actions,
};
use tokio::runtime::Runtime;
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(3);
const DRAFT: &str = "한글 draft\r\n日本語";

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
    project: ProjectId,
    tab: TabId,
    services: Arc<AppServices>,
}

impl Fixture {
    async fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-missing-{}", ProjectId::new()));
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("missing.rs");
        std::fs::write(&path, "disk").unwrap();
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic missing draft".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        state
            .layouts
            .write()
            .insert(project.clone(), taide_layout::service::default_layout());
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state, tasks, Arc::new(Sink));
        layout_actions::layout_open_tab(
            services.events.as_ref(),
            &services.state,
            project.clone(),
            TabKind::File {
                path: path.to_str().unwrap().into(),
            },
            "missing.rs".into(),
            None,
            false,
        )
        .await
        .unwrap();
        let tab = taide_layout::service::all_roots(&services.state.layouts.read()[&project])
            .flat_map(taide_native_app::tabs::tabs_in)
            .find(|tab| matches!(&tab.kind, TabKind::File { path: current } if Path::new(current) == path))
            .unwrap()
            .id
            .clone();
        file_actions::file_mirror_dirty(
            &services.state,
            &services.tasks,
            project.clone(),
            path.to_str().unwrap().into(),
            DRAFT.into(),
        )
        .await
        .unwrap();
        layout_actions::layout_set_dirty(
            services.events.as_ref(),
            &services.state,
            tab.clone(),
            true,
        )
        .await
        .unwrap();
        std::fs::remove_file(&path).unwrap();
        Self {
            directory,
            path,
            project,
            tab,
            services,
        }
    }

    async fn draft(&self) -> MissingDraft {
        missing_draft::prepare(
            &self.services.state,
            &self.services.tasks,
            self.path.to_str().unwrap().into(),
        )
        .await
        .unwrap()
        .unwrap()
        .commit()
        .unwrap()
    }

    fn tabs(&self) -> Vec<Tab> {
        taide_layout::service::all_roots(&self.services.state.layouts.read()[&self.project])
            .flat_map(taide_native_app::tabs::tabs_in)
            .cloned()
            .collect()
    }

    fn mirror_content(&self) -> String {
        taide_file::service::list_mirrors(&self.services.state.paths, &self.project)
            .unwrap()
            .into_iter()
            .find(|mirror| Path::new(&mirror.path) == self.path)
            .unwrap()
            .content
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
}

async fn reply(bridge: &mut HostBridge, ready: &Notify) -> HostReply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap()
}

#[test]
fn missing_draft는_cli_승인_외부파일을_project_mirror로_변환하지_않는다() {
    runtime().block_on(async {
        let fixture = Fixture::new().await;
        let external = fixture.directory.join("external.rs");
        std::fs::write(&external, "synthetic CLI").unwrap();
        fixture.services.state.authorize_cli_opened_path(&external);
        std::fs::remove_file(&external).unwrap();
        assert!(
            missing_draft::prepare(
                &fixture.services.state,
                &fixture.services.tasks,
                external.to_str().unwrap().into()
            )
            .await
            .unwrap()
            .is_none()
        );
        assert!(
            missing_draft::prepare(
                &fixture.services.state,
                &fixture.services.tasks,
                fixture
                    .directory
                    .join("unauthorized.rs")
                    .to_str()
                    .unwrap()
                    .into()
            )
            .await
            .is_err()
        );
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    });
}

#[test]
fn missing_draft_host는_초안을_복원하고_다른_경로_저장후_원래_탭을_유지한다() {
    runtime().block_on(async {
        let fixture = Fixture::new().await;
        let ready = Arc::new(Notify::new());
        let notification = ready.clone();
        let mut bridge = HostBridge::connect(
            fixture.services.clone(),
            Arc::new(move || notification.notify_one()),
        )
        .unwrap();
        bridge
            .submit(HostCommand::OpenDocument(
                fixture.path.to_str().unwrap().into(),
            ))
            .unwrap();
        let HostReply::MissingDraft { prepared, .. } = reply(&mut bridge, &ready).await else {
            panic!("expected missing draft");
        };
        let draft = prepared.commit().unwrap();
        assert_eq!(draft.mirror.content, DRAFT);
        assert!(draft.mirror.source_missing);
        assert_eq!(fixture.mirror_content(), DRAFT);
        let target = fixture.path.with_file_name("saved.rs");
        bridge
            .submit(HostCommand::SaveMissingDraft {
                tab: fixture.tab.clone(),
                draft,
                destination: target.to_str().unwrap().into(),
            })
            .unwrap();
        let HostReply::MissingDraftSaved { result, .. } = reply(&mut bridge, &ready).await else {
            panic!("expected completed save");
        };
        let saved = result.unwrap();
        assert_eq!(saved.opened.content, DRAFT);
        assert_eq!(saved.destination, std::fs::canonicalize(&target).unwrap());
        assert!(!fixture.path.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), DRAFT);
        let tabs = fixture.tabs();
        assert_eq!(
            tabs.iter()
                .filter(|tab| matches!(tab.kind, TabKind::File { .. }))
                .count(),
            2
        );
        assert!(tabs.iter().any(|tab| tab.id == fixture.tab && !tab.dirty));
        assert!(
            tabs.iter().any(
                |tab| matches!(&tab.kind, TabKind::File { path } if Path::new(path) == target)
            )
        );
        assert!(
            taide_file::service::list_mirrors(&fixture.services.state.paths, &fixture.project)
                .unwrap()
                .is_empty()
        );
        bridge.disconnect().await.unwrap();
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    });
}

#[test]
fn missing_draft는_같은_경로를_재생성하고_미소비_token과_shutdown에서_초안을_유지한다() {
    runtime().block_on(async {
        let fixture = Fixture::new().await;
        let token = missing_draft::prepare(
            &fixture.services.state,
            &fixture.services.tasks,
            fixture.path.to_str().unwrap().into(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(fixture.services.tasks.tracked_count() > 0);
        drop(token);
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        assert_eq!(fixture.mirror_content(), DRAFT);
        let draft = fixture.draft().await;
        let saved = missing_draft::save(
            &fixture.services,
            fixture.tab.clone(),
            draft,
            fixture.path.to_str().unwrap().into(),
        )
        .await
        .unwrap();
        assert_eq!(saved.source, saved.destination);
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), DRAFT);
        let tabs = fixture.tabs();
        let files = tabs
            .iter()
            .filter(|tab| matches!(tab.kind, TabKind::File { .. }))
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, fixture.tab);
        assert!(!files[0].dirty);
        assert!(
            taide_file::service::list_mirrors(&fixture.services.state.paths, &fixture.project)
                .unwrap()
                .is_empty()
        );
        let stopped = Fixture::new().await;
        let token = missing_draft::prepare(
            &stopped.services.state,
            &stopped.services.tasks,
            stopped.path.to_str().unwrap().into(),
        )
        .await
        .unwrap()
        .unwrap();
        stopped.services.state.begin_shutdown();
        assert!(token.commit().is_err());
        assert_eq!(stopped.mirror_content(), DRAFT);
        assert_eq!(stopped.services.tasks.tracked_count(), 0);
    });
}

#[test]
fn missing_draft_실패는_경계_dirty_목적지_새_mirror_복구된_원본과_사라진_탭을_보호한다() {
    runtime().block_on(async {
        let fixture = Fixture::new().await;
        let draft = fixture.draft().await;
        let target = fixture.path.with_file_name("destination.rs");
        let original_layout = fixture.services.state.layouts.read().clone();
        assert!(
            missing_draft::save(
                &fixture.services,
                fixture.tab.clone(),
                draft.clone(),
                fixture
                    .directory
                    .join("outside.rs")
                    .to_str()
                    .unwrap()
                    .into()
            )
            .await
            .is_err()
        );
        assert!(!fixture.directory.join("outside.rs").exists());
        std::fs::create_dir(&target).unwrap();
        assert!(
            missing_draft::save(
                &fixture.services,
                fixture.tab.clone(),
                draft.clone(),
                target.to_str().unwrap().into()
            )
            .await
            .is_err()
        );
        assert_eq!(*fixture.services.state.layouts.read(), original_layout);
        assert_eq!(fixture.mirror_content(), DRAFT);
        std::fs::remove_dir(&target).unwrap();
        std::fs::write(&target, "destination disk").unwrap();
        layout_actions::layout_open_tab(
            fixture.services.events.as_ref(),
            &fixture.services.state,
            fixture.project.clone(),
            TabKind::File {
                path: target.to_str().unwrap().into(),
            },
            "destination.rs".into(),
            None,
            false,
        )
        .await
        .unwrap();
        let destination_tab = fixture
            .tabs()
            .into_iter()
            .find(|tab| matches!(&tab.kind, TabKind::File { path } if Path::new(path) == target))
            .unwrap();
        layout_actions::layout_set_dirty(
            fixture.services.events.as_ref(),
            &fixture.services.state,
            destination_tab.id.clone(),
            true,
        )
        .await
        .unwrap();
        assert!(
            missing_draft::save(
                &fixture.services,
                fixture.tab.clone(),
                draft.clone(),
                target.to_str().unwrap().into()
            )
            .await
            .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "destination disk"
        );
        layout_actions::layout_set_dirty(
            fixture.services.events.as_ref(),
            &fixture.services.state,
            destination_tab.id,
            false,
        )
        .await
        .unwrap();
        file_actions::file_mirror_dirty(
            &fixture.services.state,
            &fixture.services.tasks,
            fixture.project.clone(),
            target.to_str().unwrap().into(),
            "destination draft".into(),
        )
        .await
        .unwrap();
        assert!(
            missing_draft::save(
                &fixture.services,
                fixture.tab.clone(),
                draft.clone(),
                target.to_str().unwrap().into()
            )
            .await
            .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "destination disk"
        );
        file_actions::file_clear_mirror(
            &fixture.services.state,
            fixture.project.clone(),
            target.to_str().unwrap().into(),
        )
        .await
        .unwrap();
        std::fs::write(&fixture.path, "external recovery").unwrap();
        assert!(
            missing_draft::save(
                &fixture.services,
                fixture.tab.clone(),
                draft.clone(),
                target.to_str().unwrap().into()
            )
            .await
            .is_err()
        );
        assert_eq!(
            std::fs::read_to_string(&fixture.path).unwrap(),
            "external recovery"
        );
        std::fs::remove_file(&fixture.path).unwrap();
        file_actions::file_mirror_dirty(
            &fixture.services.state,
            &fixture.services.tasks,
            fixture.project.clone(),
            fixture.path.to_str().unwrap().into(),
            "new source draft".into(),
        )
        .await
        .unwrap();
        assert!(
            missing_draft::save(
                &fixture.services,
                fixture.tab.clone(),
                draft,
                target.to_str().unwrap().into()
            )
            .await
            .is_err()
        );
        assert_eq!(fixture.mirror_content(), "new source draft");
        let draft = fixture.draft().await;
        fixture
            .services
            .state
            .layouts
            .write()
            .remove(&fixture.project);
        assert!(
            missing_draft::save(
                &fixture.services,
                fixture.tab.clone(),
                draft,
                target.to_str().unwrap().into()
            )
            .await
            .is_err()
        );
        assert_eq!(fixture.mirror_content(), "new source draft");
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    });
}
