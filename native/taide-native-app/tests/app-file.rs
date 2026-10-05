use std::{path::PathBuf, sync::Arc, time::Duration};

use taide_model::{
    app::{AppFileTarget, PromptTemplateId},
    app_event::AppEvent,
    ids::{PaneId, ProjectId, TabId},
    layout::{PaneNode, Tab, TabKind},
    paths::AppPaths,
    project::Project,
};
use taide_native_app::{
    app_file::{Owner, Session},
    bootstrap::services,
    persistence::{DraftEpoch, MirrorJob, write_mirror},
};
use taide_native_editor::{
    document::{DocumentKey, Edit, UndoGroup},
    store::{EditorLimits, EditorStore, Transaction},
};
use taide_runtime::{AppServices, AppState, EventSink, TaskSupervisor};

const DEADLINE: Duration = Duration::from_secs(5);
const DOCUMENT_LIMIT: usize = 8;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 128 * 1024;
const FONT_SIZE: u32 = 19;
const DRAFT: &str = "native dirty draft";
const INVALID_JSON: &str = "{ invalid synthetic JSON";
const REQUESTED_FONT_SIZE: u32 = 9999;
const SANITIZED_FONT_SIZE: u32 = 48;

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    owner: Owner,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-app-file-{}", ProjectId::new()));
        let root = directory.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.settings.write().editor_font_size = FONT_SIZE;
        let owner = Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
            target: AppFileTarget::Settings,
        };
        state.projects.write().insert(
            owner.project.clone(),
            Project {
                id: owner.project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic app file".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Leaf {
            id: owner.pane.clone(),
            tabs: vec![Tab {
                id: owner.tab.clone(),
                kind: TabKind::AppFile {
                    target: owner.target,
                },
                title: "settings.json".into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }],
            active: Some(owner.tab.clone()),
        };
        layout.focused_pane = owner.pane.clone();
        state.layouts.write().insert(owner.project.clone(), layout);
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        Self {
            directory,
            services: services(state, tasks, Arc::new(Sink)),
            owner,
        }
    }

    fn set_target(&mut self, target: AppFileTarget) {
        self.owner.target = target;
        let mut layouts = self.services.state.layouts.write();
        let PaneNode::Leaf { tabs, .. } = &mut layouts.get_mut(&self.owner.project).unwrap().root
        else {
            panic!("expected leaf");
        };
        tabs[0].kind = TabKind::AppFile { target };
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn native_app_file_read는_실제_fallback_override와_owner_폐기_문서경계를_보호한다() {
    tokio::time::timeout(DEADLINE, async {
        let mut fixture = Fixture::new();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let session = Session::new(fixture.owner.clone());
        let first = session.read_request();
        let second = session.read_request();
        assert!(session.owns(&first));
        assert!(first.same_request(&first.clone()));
        assert!(!first.same_request(&second));
        assert!(!Session::new(fixture.owner.clone()).owns(&first));
        let prepared = first.execute(&fixture.services).await.unwrap();
        assert!(prepared.request().same_request(&first));
        assert_eq!(fixture.services.tasks.tracked_count(), 1);
        let document = prepared.commit(&mut store).unwrap();
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        let current = store.documents().snapshot(document).unwrap();
        assert_eq!(
            serde_json::from_str::<taide_model::settings::Settings>(&current.rope.to_string())
                .unwrap()
                .editor_font_size,
            FONT_SIZE
        );
        assert_eq!(current.key, DocumentKey::AppFile(AppFileTarget::Settings));
        assert!(!fixture.services.state.paths.settings_file().exists());
        store
            .apply(
                document,
                Transaction {
                    revision: current.revision,
                    edits: vec![Edit {
                        bytes: 0..current.rope.len_bytes(),
                        text: DRAFT.into(),
                    }],
                    group: UndoGroup(0),
                    origin: None,
                    selection_after: None,
                },
            )
            .unwrap();
        std::fs::create_dir_all(&fixture.services.state.paths.data_dir).unwrap();
        std::fs::write(
            fixture.services.state.paths.settings_file(),
            "external override",
        )
        .unwrap();
        let prepared = second.execute(&fixture.services).await.unwrap();
        {
            let mut layouts = fixture.services.state.layouts.write();
            let PaneNode::Leaf { active, .. } =
                &mut layouts.get_mut(&fixture.owner.project).unwrap().root
            else {
                panic!("expected leaf");
            };
            *active = None;
        }
        assert!(fixture.owner.exists(&fixture.services.state));
        assert_eq!(prepared.commit(&mut store).unwrap(), document);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            DRAFT
        );
        assert!(store.documents().snapshot(document).unwrap().dirty);
        let job = MirrorJob {
            snapshot: store.documents().snapshot(document).unwrap(),
            project: fixture.owner.project.clone(),
            epoch: DraftEpoch::default(),
        };
        assert!(write_mirror(&fixture.services, job).await.is_err());
        assert_eq!(
            std::fs::read_to_string(fixture.services.state.paths.settings_file()).unwrap(),
            "external override"
        );
        assert!(fixture.services.state.cli_opened_paths.read().is_empty());
        let mut wrong = fixture.owner.clone();
        wrong.project = ProjectId::new();
        assert!(
            Session::new(wrong)
                .read_request()
                .execute(&fixture.services)
                .await
                .is_err()
        );
        let doomed = first.execute(&fixture.services).await.unwrap();
        drop(session);
        assert!(doomed.commit(&mut store).is_err());
        assert!(first.execute(&fixture.services).await.is_err());
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        let surviving = Session::new(fixture.owner.clone());
        let late = surviving
            .read_request()
            .execute(&fixture.services)
            .await
            .unwrap();
        fixture.set_target(AppFileTarget::Prompt {
            id: PromptTemplateId::AutoTabDefault,
        });
        assert!(late.commit(&mut store).is_err());
        assert_eq!(store.documents().len(), 1);
        for id in [
            PromptTemplateId::AutoTabDefault,
            PromptTemplateId::InlineEditDefault,
            PromptTemplateId::CommitMessageDefault,
        ] {
            fixture.set_target(AppFileTarget::Prompt { id });
            let session = Session::new(fixture.owner.clone());
            let prompt = session
                .read_request()
                .execute(&fixture.services)
                .await
                .unwrap()
                .commit(&mut store)
                .unwrap();
            let text = store.documents().snapshot(prompt).unwrap().rope.to_string();
            assert!(
                serde_json::from_str::<serde_json::Value>(&text)
                    .unwrap()
                    .is_object()
            );
            assert_ne!(prompt, document);
        }
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        let cancelled = Session::new(fixture.owner.clone());
        let request = cancelled.read_request();
        let guard = fixture.services.state.begin_owned_mutation().await;
        let pending = request.execute(&fixture.services);
        tokio::pin!(pending);
        tokio::select! {
            biased;
            _ = &mut pending => panic!("read bypassed the mutation guard"),
            _ = tokio::task::yield_now() => {}
        }
        assert_eq!(fixture.services.tasks.tracked_count(), 1);
        drop(cancelled);
        drop(guard);
        assert!(pending.await.is_err());
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        fixture.services.tasks.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_app_file_refresh는_clean_외부설정만_채택하고_live_dirty를_유지한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        let session = Session::new(fixture.owner.clone());
        let document = session
            .read_request()
            .execute(&fixture.services)
            .await
            .unwrap()
            .commit(&mut store)
            .unwrap();
        std::fs::create_dir_all(&fixture.services.state.paths.data_dir).unwrap();
        std::fs::write(
            fixture.services.state.paths.settings_file(),
            "{\"editorFontSize\":20}",
        )
        .unwrap();
        assert_eq!(
            session
                .read_request()
                .execute(&fixture.services)
                .await
                .unwrap()
                .commit(&mut store)
                .unwrap(),
            document
        );
        let clean = store.documents().snapshot(document).unwrap();
        assert_eq!(clean.rope.to_string(), "{\"editorFontSize\":20}");
        assert!(!clean.dirty);
        assert!(clean.revision > 0);
        assert!(clean.metadata.disk_modified_ms.is_none());
        store
            .apply(
                document,
                Transaction {
                    revision: clean.revision,
                    edits: vec![Edit {
                        bytes: 0..clean.rope.len_bytes(),
                        text: DRAFT.into(),
                    }],
                    group: UndoGroup(clean.revision),
                    origin: None,
                    selection_after: None,
                },
            )
            .unwrap();
        assert_eq!(
            session
                .read_request()
                .execute(&fixture.services)
                .await
                .unwrap()
                .commit(&mut store)
                .unwrap(),
            document
        );
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            DRAFT
        );
        assert!(store.documents().snapshot(document).unwrap().dirty);
        assert!(!store.has_disk_conflict(document).unwrap());
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        fixture.services.tasks.shutdown().await;
    })
    .await
    .unwrap();
}

fn replace_document(
    store: &mut EditorStore,
    document: taide_native_editor::document::DocumentId,
    text: &str,
) {
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text: text.into(),
                }],
                group: UndoGroup(snapshot.revision),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

fn editor_store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap()
}

#[tokio::test]
async fn native_app_file_write는_실제_settings_apply와_canonical_추가편집을_보호한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let mut store = editor_store();
        let session = Session::new(fixture.owner.clone());
        let document = session
            .read_request()
            .execute(&fixture.services)
            .await
            .unwrap()
            .commit(&mut store)
            .unwrap();
        replace_document(&mut store, document, INVALID_JSON);
        let malformed = session
            .write_request(store.save_snapshot(document).unwrap())
            .unwrap();
        assert!(
            malformed
                .execute(&fixture.services, |_| async {
                    panic!("invalid JSON reached settings apply");
                })
                .await
                .is_err()
        );
        assert!(!fixture.services.state.paths.settings_file().exists());
        assert_eq!(
            fixture.services.state.settings.read().editor_font_size,
            FONT_SIZE
        );
        let mut settings = fixture.services.state.settings.read().clone();
        settings.editor_font_size = REQUESTED_FONT_SIZE;
        let content = serde_json::to_string(&settings).unwrap();
        replace_document(&mut store, document, &content);
        let request = session
            .write_request(store.save_snapshot(document).unwrap())
            .unwrap();
        let other = session
            .write_request(store.save_snapshot(document).unwrap())
            .unwrap();
        assert!(session.owns_write(&request));
        assert!(!Session::new(fixture.owner.clone()).owns_write(&request));
        assert!(request.same_request(&request.clone()));
        assert!(!request.same_request(&other));
        let services = fixture.services.clone();
        let prepared = request
            .execute(&fixture.services, move |parsed| async move {
                assert_eq!(parsed.editor_font_size, REQUESTED_FONT_SIZE);
                taide_runtime::settings_actions::apply_and_broadcast(
                    &services.state,
                    parsed,
                    move |current, updated| async move {
                        assert_eq!(current.editor_font_size, FONT_SIZE);
                        assert_eq!(updated.editor_font_size, SANITIZED_FONT_SIZE);
                    },
                    services.events.as_ref(),
                )
                .await
            })
            .await
            .unwrap();
        assert!(prepared.request().same_request(&request));
        assert_eq!(fixture.services.tasks.tracked_count(), 1);
        assert_eq!(
            fixture.services.state.settings.read().editor_font_size,
            SANITIZED_FONT_SIZE
        );
        let canonical =
            std::fs::read_to_string(fixture.services.state.paths.settings_file()).unwrap();
        assert_ne!(canonical, content);
        assert_eq!(prepared.commit(&mut store).unwrap(), document);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            canonical
        );
        assert!(!store.documents().snapshot(document).unwrap().dirty);
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        let snapshot = store.save_snapshot(document).unwrap();
        replace_document(&mut store, document, DRAFT);
        let request = session.write_request(snapshot).unwrap();
        let services = fixture.services.clone();
        let prepared = request
            .execute(&fixture.services, move |parsed| async move {
                taide_runtime::settings_actions::apply_and_broadcast(
                    &services.state,
                    parsed,
                    |current, updated| async move {
                        assert_eq!(current, updated);
                    },
                    services.events.as_ref(),
                )
                .await
            })
            .await
            .unwrap();
        assert_eq!(prepared.commit(&mut store).unwrap(), document);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            DRAFT
        );
        assert!(store.documents().snapshot(document).unwrap().dirty);
        assert_eq!(
            std::fs::read_to_string(fixture.services.state.paths.settings_file()).unwrap(),
            canonical
        );
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        let guard = fixture.services.state.begin_owned_mutation().await;
        let legacy = taide_runtime::app_actions::app_file_write(
            &fixture.services.state,
            AppFileTarget::Settings,
            INVALID_JSON.into(),
            |_| async {
                panic!("legacy invalid JSON reached settings apply");
            },
        );
        tokio::pin!(legacy);
        tokio::select! {
            biased;
            _ = &mut legacy => panic!("legacy app file write bypassed its mutation guard"),
            _ = tokio::task::yield_now() => {}
        }
        drop(guard);
        assert!(legacy.await.is_err());
        assert_eq!(
            std::fs::read_to_string(fixture.services.state.paths.settings_file()).unwrap(),
            canonical
        );
        fixture.services.tasks.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_app_file_write는_prompt_검증과_취소된_소유자의_디스크권한을_보호한다() {
    tokio::time::timeout(DEADLINE, async {
        let mut fixture = Fixture::new();
        let mut store = editor_store();
        for id in [PromptTemplateId::AutoTabDefault, PromptTemplateId::InlineEditDefault, PromptTemplateId::CommitMessageDefault] {
            let target = AppFileTarget::Prompt { id };
            fixture.set_target(target);
            let session = Session::new(fixture.owner.clone());
            let document = session.read_request().execute(&fixture.services).await.unwrap().commit(&mut store).unwrap();
            let content = format!("{}\n", store.documents().snapshot(document).unwrap().rope);
            replace_document(&mut store, document, &content);
            let request = session.write_request(store.save_snapshot(document).unwrap()).unwrap();
            let prepared = request.execute(&fixture.services, |_| async { panic!("prompt called settings apply"); }).await.unwrap();
            assert_eq!(prepared.commit(&mut store).unwrap(), document);
            assert!(!store.documents().snapshot(document).unwrap().dirty);
            let path = fixture.services.state.paths.prompts_dir().join(format!("{}.json", id.as_str()));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
            replace_document(&mut store, document, INVALID_JSON);
            let invalid = session.write_request(store.save_snapshot(document).unwrap()).unwrap();
            let error = invalid.execute(&fixture.services, |_| async { panic!("invalid prompt called settings apply"); }).await.err().unwrap();
            assert!(matches!(error, taide_model::error::AppError::Localized(ref error) if error.key == "error.app.promptTemplateInvalidJson"));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
            replace_document(&mut store, document, &content);
            let cancelled = session.write_request(store.save_snapshot(document).unwrap()).unwrap();
            let guard = fixture.services.state.begin_owned_mutation().await;
            let pending = cancelled.execute(&fixture.services, |_| async { panic!("cancelled prompt called settings apply"); });
            tokio::pin!(pending);
            tokio::select! {
                biased;
                _ = &mut pending => panic!("write bypassed the owned mutation guard"),
                _ = tokio::task::yield_now() => {}
            }
            assert_eq!(fixture.services.tasks.tracked_count(), 1);
            drop(session);
            drop(guard);
            assert!(pending.await.is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
            assert_eq!(fixture.services.tasks.tracked_count(), 0);
            let wrong = Session::new(Owner { target: AppFileTarget::Settings, ..fixture.owner.clone() });
            assert!(wrong.write_request(store.save_snapshot(document).unwrap()).is_err());
        }
        assert!(!fixture.services.state.paths.settings_file().exists());
        assert!(fixture.services.state.cli_opened_paths.read().is_empty());
        fixture.services.tasks.shutdown().await;
    }).await.unwrap();
}
