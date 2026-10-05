use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::layout::{Tab, TabKind};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::file_sync;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_editor::document::DiskChoice;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{
    AppServices, AppState, EventSink, TaskSupervisor, file_actions, layout_actions,
};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(3);
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 1024;
const AUXILIARY_SLOT: u32 = 1;

#[test]
fn 취소된_저장_generation은_이름변경된_이전_경로를_다시_생성하지_않는다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let fixture = Fixture::new();
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let file = taide_file::service::open_file(&fixture.canonical, &[], false).unwrap();
            let document = store.open_file(fixture.canonical.clone(), file).unwrap();
            let snapshot = store.save_snapshot(document).unwrap();
            let renamed = fixture.canonical.with_file_name("renamed.rs");
            std::fs::rename(&fixture.canonical, &renamed).unwrap();
            let epoch = taide_native_app::persistence::DraftEpoch::default();
            epoch.invalidate();
            let result =
                file_sync::save_snapshot(&fixture.services, fixture.path(), snapshot, Some(epoch))
                    .await;
            assert!(result.is_err());
            assert!(!fixture.canonical.exists());
            assert_eq!(std::fs::read_to_string(renamed).unwrap(), "disk");
            fixture.services.tasks.shutdown().await;
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        });
}

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    canonical: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-disk-{}", ProjectId::new()));
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("editor.rs");
        std::fs::write(&path, "disk").unwrap();
        let canonical = std::fs::canonicalize(&path).unwrap();
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic conflict".into(),
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
        Self {
            directory,
            canonical,
            project,
            services: services(state, tasks, Arc::new(Sink)),
        }
    }

    fn path(&self) -> String {
        self.canonical.to_str().unwrap().into()
    }

    fn tabs(&self) -> Vec<Tab> {
        taide_layout::service::all_roots(&self.services.state.layouts.read()[&self.project])
            .flat_map(taide_native_app::tabs::tabs_in)
            .filter(|tab| matches!(tab.kind, TabKind::File { .. }))
            .cloned()
            .collect()
    }

    async fn open(&self, path: String) -> Tab {
        layout_actions::layout_open_tab(
            self.services.events.as_ref(),
            &self.services.state,
            self.project.clone(),
            TabKind::File { path: path.clone() },
            "editor.rs".into(),
            None,
            false,
        )
        .await
        .unwrap();
        self.tabs()
            .into_iter()
            .find(|tab| matches!(&tab.kind, TabKind::File { path: current } if current == &path))
            .unwrap()
    }

    async fn mirror(&self, content: &str) {
        file_actions::file_mirror_dirty(
            &self.services.state,
            &self.services.tasks,
            self.project.clone(),
            self.path(),
            content.into(),
        )
        .await
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
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
#[cfg(unix)]
fn 실제_host의_충돌_선택은_공유_layout_stale_초안과_mirror_버전을_보호한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let fixture = Fixture::new();
            let main_tab = fixture.open(fixture.path()).await;
            let alias = fixture.directory.join("root/alias.rs");
            std::os::unix::fs::symlink(&fixture.canonical, &alias).unwrap();
            let auxiliary_tab = fixture.open(alias.to_str().unwrap().into()).await;
            {
                let mut layouts = fixture.services.state.layouts.write();
                taide_layout::service::move_tab_to_new_window(
                    layouts.get_mut(&fixture.project).unwrap(),
                    &auxiliary_tab.id,
                    AUXILIARY_SLOT,
                )
                .unwrap();
            }
            for tab in [&main_tab.id, &auxiliary_tab.id] {
                layout_actions::layout_set_dirty(
                    fixture.services.events.as_ref(),
                    &fixture.services.state,
                    tab.clone(),
                    true,
                )
                .await
                .unwrap();
            }
            fixture.mirror("draft").await;
            let ready = Arc::new(Notify::new());
            let signal = ready.clone();
            let mut bridge = HostBridge::connect(
                fixture.services.clone(),
                Arc::new(move || signal.notify_one()),
            )
            .unwrap();
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            bridge
                .submit(HostCommand::OpenDocument(fixture.path()))
                .unwrap();
            let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected document");
            };
            let admitted = result.unwrap().commit_with_notice(&mut store).unwrap();
            let document = admitted.document;
            assert_eq!(admitted.restored_conflict, Some(false));
            let main_pane = fixture.services.state.layouts.read()[&fixture.project]
                .focused_pane
                .clone();
            let auxiliary_pane = fixture.services.state.layouts.read()[&fixture.project]
                .auxiliary_windows[0]
                .focused_pane
                .clone();
            let main_view = store
                .attach_view(
                    ViewKey {
                        window: "main".into(),
                        pane: main_pane,
                        tab: main_tab.id,
                    },
                    document,
                )
                .unwrap();
            store
                .attach_view(
                    ViewKey {
                        window: "auxiliary".into(),
                        pane: auxiliary_pane,
                        tab: auxiliary_tab.id,
                    },
                    document,
                )
                .unwrap();
            std::fs::write(&fixture.canonical, "external").unwrap();
            bridge
                .submit(HostCommand::ObserveFile {
                    path: fixture.path(),
                    document,
                })
                .unwrap();
            let HostReply::ObservedFile { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected observation");
            };
            assert!(
                result
                    .unwrap()
                    .commit_observation(&mut store, document)
                    .unwrap()
            );
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                "draft"
            );
            let revision = store.documents().snapshot(document).unwrap().revision;
            bridge
                .submit(HostCommand::ChooseDisk {
                    path: fixture.path(),
                    document,
                    revision,
                    choice: DiskChoice::KeepMine,
                })
                .unwrap();
            let HostReply::DiskChosen { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected keep mine");
            };
            let kept = result
                .unwrap()
                .commit_disk_choice(
                    &mut store,
                    document,
                    revision,
                    DiskChoice::KeepMine,
                    fixture.services.events.as_ref(),
                )
                .unwrap();
            assert!(kept.dirty);
            assert!(!store.has_disk_conflict(document).unwrap());
            assert!(fixture.tabs().iter().all(|tab| tab.dirty));
            assert_eq!(
                std::fs::read_to_string(&fixture.canonical).unwrap(),
                "external"
            );
            assert_eq!(
                taide_file::service::list_mirrors(&fixture.services.state.paths, &fixture.project)
                    .unwrap()[0]
                    .content,
                "draft"
            );
            std::fs::write(&fixture.canonical, "second external").unwrap();
            bridge
                .submit(HostCommand::ChooseDisk {
                    path: fixture.path(),
                    document,
                    revision,
                    choice: DiskChoice::ViewDisk,
                })
                .unwrap();
            let HostReply::DiskChosen { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected guarded stale choice");
            };
            let prepared = result.unwrap();
            replace_selections(&mut store, main_view, "later ", None).unwrap();
            assert!(
                prepared
                    .commit_disk_choice(
                        &mut store,
                        document,
                        revision,
                        DiskChoice::ViewDisk,
                        fixture.services.events.as_ref()
                    )
                    .is_err()
            );
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                "later draft"
            );
            assert!(fixture.tabs().iter().all(|tab| tab.dirty));
            assert_eq!(
                taide_file::service::list_mirrors(&fixture.services.state.paths, &fixture.project)
                    .unwrap()[0]
                    .content,
                "draft"
            );
            let revision = store.documents().snapshot(document).unwrap().revision;
            bridge
                .submit(HostCommand::ChooseDisk {
                    path: fixture.path(),
                    document,
                    revision,
                    choice: DiskChoice::ViewDisk,
                })
                .unwrap();
            let HostReply::DiskChosen { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected view disk");
            };
            let viewed = result
                .unwrap()
                .commit_disk_choice(
                    &mut store,
                    document,
                    revision,
                    DiskChoice::ViewDisk,
                    fixture.services.events.as_ref(),
                )
                .unwrap();
            assert!(!viewed.dirty);
            assert!(fixture.tabs().iter().all(|tab| !tab.dirty));
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                "second external"
            );
            assert_eq!(store.views().for_document(document).count(), 2);
            fixture.mirror("newer draft").await;
            file_sync::cleanup_mirror(
                &fixture.services,
                fixture.project.clone(),
                viewed.canonical.clone(),
                viewed.mirror,
            )
            .await
            .unwrap();
            let mirrors =
                taide_file::service::list_mirrors(&fixture.services.state.paths, &fixture.project)
                    .unwrap();
            assert_eq!(mirrors.len(), 1);
            assert_eq!(mirrors[0].content, "newer draft");
            file_sync::cleanup_mirror(
                &fixture.services,
                fixture.project.clone(),
                viewed.canonical,
                mirrors.into_iter().next(),
            )
            .await
            .unwrap();
            assert!(
                taide_file::service::list_mirrors(&fixture.services.state.paths, &fixture.project)
                    .unwrap()
                    .is_empty()
            );
            bridge
                .submit(HostCommand::ObserveFile {
                    path: fixture.path(),
                    document,
                })
                .unwrap();
            let HostReply::ObservedFile { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected droppable observation");
            };
            drop(result.unwrap());
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                "second external"
            );
            bridge.disconnect().await.unwrap();
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        });
}
