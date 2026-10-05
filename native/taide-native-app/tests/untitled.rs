use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::untitled;
use taide_native_editor::document::DocumentKey;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{
    AppServices, AppState, EventSink, TaskSupervisor, file_actions, layout_actions,
    native_file_actions,
};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(3);
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 1024;
const DRAFT: &str = "한글 untitled\r\n日本語";

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    project: ProjectId,
    services: Arc<AppServices>,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-untitled-{}", ProjectId::new()));
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic untitled".into(),
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
            project,
            services: services(state, tasks, Arc::new(Sink)),
        }
    }

    fn tabs(&self) -> Vec<Tab> {
        taide_layout::service::all_roots(&self.services.state.layouts.read()[&self.project])
            .flat_map(taide_native_app::tabs::tabs_in)
            .cloned()
            .collect()
    }

    async fn open(&self) -> TabId {
        layout_actions::layout_open_untitled(
            self.services.events.as_ref(),
            &self.services.state,
            self.project.clone(),
            None,
        )
        .await
        .unwrap();
        let tab = self
            .tabs()
            .into_iter()
            .find(|tab| matches!(tab.kind, TabKind::Untitled { .. }))
            .unwrap()
            .id;
        file_actions::file_mirror_untitled(
            &self.services.state,
            self.project.clone(),
            tab.clone(),
            DRAFT.into(),
        )
        .await
        .unwrap();
        layout_actions::layout_set_dirty(
            self.services.events.as_ref(),
            &self.services.state,
            tab.clone(),
            true,
        )
        .await
        .unwrap();
        tab
    }

    fn destination(&self) -> String {
        self.directory
            .join("root/saved.rs")
            .to_str()
            .unwrap()
            .into()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
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
fn untitled_host는_새_탭_mirror_편집_저장_guarded_전환과_cleanup을_연결한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let fixture = Fixture::new();
            let ready = Arc::new(Notify::new());
            let repaint = ready.clone();
            let mut bridge = HostBridge::connect(
                fixture.services.clone(),
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            let pane = fixture.services.state.layouts.read()[&fixture.project]
                .focused_pane
                .clone();
            bridge
                .submit(HostCommand::NewUntitled {
                    project: fixture.project.clone(),
                    pane: pane.clone(),
                })
                .unwrap();
            tokio::time::timeout(TIMEOUT, ready.notified())
                .await
                .unwrap();
            let tab = fixture
                .tabs()
                .into_iter()
                .find(|tab| matches!(tab.kind, TabKind::Untitled { .. }))
                .unwrap()
                .id;
            file_actions::file_mirror_untitled(
                &fixture.services.state,
                fixture.project.clone(),
                tab.clone(),
                DRAFT.into(),
            )
            .await
            .unwrap();
            bridge
                .submit(HostCommand::OpenUntitled(tab.clone()))
                .unwrap();
            let HostReply::UntitledOpened { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected untitled admission");
            };
            let mut store = store();
            let document = result.unwrap().commit(&mut store).unwrap();
            assert!(store.documents().snapshot(document).unwrap().dirty);
            let view = store
                .attach_view(
                    ViewKey {
                        window: "main".into(),
                        pane,
                        tab: tab.clone(),
                    },
                    document,
                )
                .unwrap();
            replace_selections(&mut store, view, "edited ", None).unwrap();
            let written = store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string();
            bridge
                .submit(HostCommand::SaveUntitled {
                    tab: tab.clone(),
                    destination: fixture.destination(),
                    snapshot: store.save_snapshot(document).unwrap(),
                })
                .unwrap();
            let HostReply::UntitledSaved { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected prepared untitled save");
            };
            let prepared = result.unwrap();
            assert!(fixture.tabs().iter().any(|candidate| candidate.id == tab
                && matches!(candidate.kind, TabKind::Untitled { .. })));
            assert_eq!(
                std::fs::read_to_string(fixture.destination()).unwrap(),
                written
            );
            assert_eq!(
                taide_file::service::list_untitled_mirrors(
                    &fixture.services.state.paths,
                    &fixture.project
                )
                .unwrap()
                .len(),
                1
            );
            let mut waiting = Box::pin(fixture.services.state.begin_mutation());
            let mut context = Context::from_waker(Waker::noop());
            assert!(matches!(waiting.as_mut().poll(&mut context), Poll::Pending));
            let converted = prepared.commit(&mut store).unwrap();
            assert!(matches!(
                waiting.as_mut().poll(&mut context),
                Poll::Ready(_)
            ));
            drop(waiting);
            assert!(converted.clean);
            assert_eq!(converted.file_tab, tab);
            assert_eq!(
                store.documents().snapshot(document).unwrap().key,
                DocumentKey::File(converted.canonical.clone())
            );
            assert_eq!(store.views().get(view).unwrap().document, document);
            assert!(fixture.tabs().iter().any(|candidate| candidate.id == tab
                && !candidate.dirty
                && matches!(candidate.kind, TabKind::File { .. })));
            untitled::cleanup_mirror(
                &fixture.services,
                converted.project,
                converted.source_tab,
                converted.mirror,
            )
            .await
            .unwrap();
            assert!(
                taide_file::service::list_untitled_mirrors(
                    &fixture.services.state.paths,
                    &fixture.project
                )
                .unwrap()
                .is_empty()
            );
            bridge.disconnect().await.unwrap();
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        });
}

#[test]
fn untitled_전환은_실제_같은_pane_파일_탭과_합류하고_늦은_편집과_새_mirror를_유지한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let fixture = Fixture::new();
            std::fs::write(fixture.destination(), "disk").unwrap();
            let canonical = std::fs::canonicalize(fixture.destination()).unwrap();
            let path = canonical.to_str().unwrap().to_owned();
            layout_actions::layout_open_tab(
                fixture.services.events.as_ref(),
                &fixture.services.state,
                fixture.project.clone(),
                TabKind::File { path: path.clone() },
                "saved.rs".into(),
                None,
                false,
            )
            .await
            .unwrap();
            let target_tab = fixture
                .tabs()
                .into_iter()
                .find(
                    |tab| matches!(&tab.kind, TabKind::File { path: current } if current == &path),
                )
                .unwrap()
                .id;
            let tab = fixture.open().await;
            let mut store = store();
            let opened = native_file_actions::open_document_file(
                &fixture.services.state,
                &fixture.services.tasks,
                path,
                Vec::new,
            )
            .await
            .unwrap();
            let target = taide_native_ui::document_admission::prepare_opened_document(
                &fixture.services.state,
                &fixture.services.tasks,
                opened,
            )
            .await
            .unwrap()
            .commit(&mut store)
            .unwrap();
            let pane = fixture.services.state.layouts.read()[&fixture.project]
                .focused_pane
                .clone();
            let target_view = store
                .attach_view(
                    ViewKey {
                        window: "main".into(),
                        pane: pane.clone(),
                        tab: target_tab.clone(),
                    },
                    target,
                )
                .unwrap();
            let document = untitled::prepare(&fixture.services, tab.clone())
                .await
                .unwrap()
                .commit(&mut store)
                .unwrap();
            let view = store
                .attach_view(
                    ViewKey {
                        window: "main".into(),
                        pane,
                        tab: tab.clone(),
                    },
                    document,
                )
                .unwrap();
            let prepared = untitled::prepare_save(
                &fixture.services,
                tab.clone(),
                fixture.destination(),
                store.save_snapshot(document).unwrap(),
            )
            .await
            .unwrap();
            replace_selections(&mut store, view, "later ", None).unwrap();
            let converted = prepared.commit(&mut store).unwrap();
            assert!(!converted.clean);
            assert_eq!(converted.merged_document, Some(target));
            assert_eq!(converted.file_tab, target_tab);
            assert_eq!(store.documents().len(), 1);
            assert_eq!(store.views().get(target_view).unwrap().document, document);
            assert!(store.views().get(view).is_none());
            assert!(store.documents().snapshot(document).unwrap().dirty);
            assert_eq!(
                std::fs::read_to_string(fixture.destination()).unwrap(),
                DRAFT
            );
            assert!(fixture.tabs().iter().all(|candidate| candidate.id != tab));
            assert!(
                fixture
                    .tabs()
                    .iter()
                    .any(|candidate| candidate.id == target_tab && candidate.dirty)
            );
            file_actions::file_mirror_untitled(
                &fixture.services.state,
                fixture.project.clone(),
                tab.clone(),
                "new mirror".into(),
            )
            .await
            .unwrap();
            untitled::cleanup_mirror(
                &fixture.services,
                converted.project,
                converted.source_tab,
                converted.mirror,
            )
            .await
            .unwrap();
            assert_eq!(
                taide_file::service::list_untitled_mirrors(
                    &fixture.services.state.paths,
                    &fixture.project
                )
                .unwrap()[0]
                    .content,
                "new mirror"
            );
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        });
}

#[test]
fn untitled_실패와_취소_token은_layout와_mirror를_보존하고_dirty_목적지와_shutdown을_거절한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let fixture = Fixture::new();
            let tab = fixture.open().await;
            let mut store = store();
            let document = untitled::prepare(&fixture.services, tab.clone())
                .await
                .unwrap()
                .commit(&mut store)
                .unwrap();
            let snapshot = store.save_snapshot(document).unwrap();
            let before = fixture.services.state.layouts.read().clone();
            assert!(
                untitled::prepare_save(
                    &fixture.services,
                    tab.clone(),
                    fixture
                        .directory
                        .join("outside.rs")
                        .to_str()
                        .unwrap()
                        .into(),
                    snapshot.clone()
                )
                .await
                .is_err()
            );
            assert!(!fixture.directory.join("outside.rs").exists());
            std::fs::create_dir(fixture.destination()).unwrap();
            assert!(
                untitled::prepare_save(
                    &fixture.services,
                    tab.clone(),
                    fixture.destination(),
                    snapshot.clone()
                )
                .await
                .is_err()
            );
            assert_eq!(*fixture.services.state.layouts.read(), before);
            std::fs::remove_dir(fixture.destination()).unwrap();
            std::fs::write(fixture.destination(), "disk").unwrap();
            layout_actions::layout_open_tab(
                fixture.services.events.as_ref(),
                &fixture.services.state,
                fixture.project.clone(),
                TabKind::File {
                    path: fixture.destination(),
                },
                "saved.rs".into(),
                None,
                false,
            )
            .await
            .unwrap();
            let target = fixture
                .tabs()
                .into_iter()
                .find(|tab| matches!(tab.kind, TabKind::File { .. }))
                .unwrap()
                .id;
            layout_actions::layout_set_dirty(
                fixture.services.events.as_ref(),
                &fixture.services.state,
                target.clone(),
                true,
            )
            .await
            .unwrap();
            let failure = untitled::prepare_save(
                &fixture.services,
                tab.clone(),
                fixture.destination(),
                snapshot.clone(),
            )
            .await
            .err()
            .unwrap();
            assert!(failure.to_string().contains("unsaved draft"));
            assert_eq!(
                std::fs::read_to_string(fixture.destination()).unwrap(),
                "disk"
            );
            layout_actions::layout_set_dirty(
                fixture.services.events.as_ref(),
                &fixture.services.state,
                target,
                false,
            )
            .await
            .unwrap();
            file_actions::file_mirror_dirty(
                &fixture.services.state,
                &fixture.services.tasks,
                fixture.project.clone(),
                fixture.destination(),
                "destination draft".into(),
            )
            .await
            .unwrap();
            let failure = untitled::prepare_save(
                &fixture.services,
                tab.clone(),
                fixture.destination(),
                snapshot.clone(),
            )
            .await
            .err()
            .unwrap();
            assert!(failure.to_string().contains("hot-exit draft"));
            file_actions::file_clear_mirror(
                &fixture.services.state,
                fixture.project.clone(),
                fixture.destination(),
            )
            .await
            .unwrap();
            let prepared = untitled::prepare_save(
                &fixture.services,
                tab.clone(),
                fixture.destination(),
                snapshot,
            )
            .await
            .unwrap();
            drop(prepared);
            assert!(fixture.tabs().iter().any(|candidate| candidate.id == tab
                && matches!(candidate.kind, TabKind::Untitled { .. })));
            assert_eq!(
                taide_file::service::list_untitled_mirrors(
                    &fixture.services.state.paths,
                    &fixture.project
                )
                .unwrap()[0]
                    .content,
                DRAFT
            );
            assert_eq!(
                store.documents().snapshot(document).unwrap().key,
                DocumentKey::Untitled(tab.clone())
            );
            let prepared = untitled::prepare(&fixture.services, tab).await.unwrap();
            fixture.services.state.begin_shutdown();
            assert!(prepared.commit(&mut store).is_err());
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
        });
}
