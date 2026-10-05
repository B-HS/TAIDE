use super::*;
use std::{path::PathBuf, sync::Arc, time::Duration};
use taide_model::{
    app_event::AppEvent,
    ids::{PaneId, ProjectId},
    layout::{AuxWindowLayout, TabKind},
    paths::AppPaths,
};
use taide_native_editor::{
    document::{Edit, UndoGroup},
    store::{EditorLimits, Transaction},
    view::ViewKey,
};
use taide_runtime::{AppServices, EventSink, TaskSupervisor};

use crate::host::{HostBridge, HostCommand, HostReply};

const DEADLINE: Duration = Duration::from_secs(5);
const DOCUMENT_LIMIT: usize = 8;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 128 * 1024;
const DRAFT: &str = "synthetic dirty settings";
const SAVED_FONT_SIZE: u32 = 19;
const LATE_EDIT: &str = "\n ";
const INVALID_JSON: &str = "{invalid synthetic settings";

fn replace_app_file_draft(store: &mut EditorStore, document: DocumentId, text: String) {
    let snapshot = store.documents().snapshot(document).unwrap();
    store
        .apply(
            document,
            Transaction {
                revision: snapshot.revision,
                edits: vec![Edit {
                    bytes: 0..snapshot.rope.len_bytes(),
                    text,
                }],
                group: UndoGroup(0),
                origin: None,
                selection_after: None,
            },
        )
        .unwrap();
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    main: Owner,
    auxiliary: Owner,
}

impl Fixture {
    fn new() -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-native-app-file-views-{}", ProjectId::new()));
        std::fs::create_dir_all(&directory).unwrap();
        let state = AppState::new(AppPaths::new(directory.clone()));
        let main = Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
            target: AppFileTarget::Settings,
        };
        let auxiliary = Owner {
            project: main.project.clone(),
            pane: PaneId::new(),
            tab: TabId::new(),
            target: main.target,
        };
        let leaf = |owner: &Owner| PaneNode::Leaf {
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
        let mut layout = taide_layout::service::default_layout();
        layout.root = leaf(&main);
        layout.focused_pane = main.pane.clone();
        layout.auxiliary_windows.push(AuxWindowLayout {
            slot: 1,
            root: leaf(&auxiliary),
            focused_pane: auxiliary.pane.clone(),
        });
        state.layouts.write().insert(main.project.clone(), layout);
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        Self {
            directory,
            services: crate::bootstrap::services(state, tasks, Arc::new(Sink)),
            main,
            auxiliary,
        }
    }

    fn store(&self) -> EditorStore {
        EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[tokio::test]
async fn app_file_save_host는_공유_pending과_reconcile_대기_추가편집_실패재시도를_보호한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let mut views = Views::default();
        let read = views.begin(&fixture.main, &store, true).unwrap();
        assert!(views.accept(&read, read.execute(&fixture.services).await, &mut store));
        let document = views.document(&fixture.main).unwrap();
        assert!(
            views
                .begin_write(&fixture.main, &mut store)
                .unwrap()
                .is_none()
        );
        assert!(views.begin(&fixture.auxiliary, &store, true).is_none());
        views.changed(document);
        let clean_draft = views
            .begin_write(&fixture.main, &mut store)
            .unwrap()
            .unwrap();
        assert!(views.cancel_write(&clean_draft));
        assert_eq!(
            views.save_owner(&fixture.auxiliary.tab, Some(document)),
            Some(fixture.auxiliary.clone())
        );
        assert!(views.save_owner(&fixture.main.tab, None).is_none());
        let mut settings = fixture.services.state.settings.read().clone();
        settings.editor_font_size = SAVED_FONT_SIZE;
        let content = serde_json::to_string(&settings).unwrap();
        replace_app_file_draft(&mut store, document, content.clone());
        let request = views
            .begin_write(&fixture.main, &mut store)
            .unwrap()
            .unwrap();
        assert!(
            views
                .begin_write(&fixture.auxiliary, &mut store)
                .unwrap()
                .is_none()
        );
        let foreign = Session::new(fixture.main.clone());
        let unrelated = foreign
            .write_request(store.save_snapshot(document).unwrap())
            .unwrap();
        assert!(!views.cancel_write(&unrelated));
        let ready = Arc::new(tokio::sync::Notify::new());
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let signal = ready.clone();
        let apply_entered = entered.clone();
        let apply_release = release.clone();
        let apply_calls = calls.clone();
        let mut host = HostBridge::connect_with_settings_ports(
            fixture.services.clone(),
            Arc::new(move || signal.notify_one()),
            Arc::new(|_| panic!("clipboard write is outside this test")),
            Arc::new(|| panic!("clipboard read is outside this test")),
            None,
            Arc::new(move |services, current, updated| {
                let entered = apply_entered.clone();
                let release = apply_release.clone();
                let calls = apply_calls.clone();
                Box::pin(async move {
                    assert_ne!(current.editor_font_size, updated.editor_font_size);
                    assert_eq!(updated.editor_font_size, SAVED_FONT_SIZE);
                    assert_eq!(*services.state.settings.read(), updated);
                    let stored: taide_model::settings::Settings = serde_json::from_slice(
                        &std::fs::read(services.state.paths.settings_file()).unwrap(),
                    )
                    .unwrap();
                    assert_eq!(stored, updated);
                    calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    entered.notify_one();
                    release.notified().await;
                })
            }),
        )
        .unwrap();
        host.submit(HostCommand::WriteAppFile(request.clone()))
            .unwrap();
        entered.notified().await;
        assert!(host.poll().is_none());
        assert!(store.documents().snapshot(document).unwrap().dirty);
        views.observe_settings_revision(1);
        assert!(
            views
                .begin_write(&fixture.auxiliary, &mut store)
                .unwrap()
                .is_none()
        );
        replace_app_file_draft(&mut store, document, format!("{content}{LATE_EDIT}"));
        release.notify_one();
        ready.notified().await;
        let HostReply::AppFileWritten {
            request: reply,
            result,
        } = host.poll().unwrap()
        else {
            panic!("expected typed write completion");
        };
        assert!(reply.same_request(&request));
        assert_eq!(
            views
                .accept_write(&reply, result, &mut store)
                .unwrap()
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
            format!("{content}{LATE_EDIT}")
        );
        assert!(store.documents().snapshot(document).unwrap().dirty);
        assert!(
            views
                .accept_write(
                    &reply,
                    Err(AppError::Internal("duplicate completion".into())),
                    &mut store
                )
                .is_none()
        );
        let canonical =
            std::fs::read_to_string(fixture.services.state.paths.settings_file()).unwrap();
        assert_ne!(canonical, content);
        assert_eq!(
            serde_json::from_str::<taide_model::settings::Settings>(&canonical).unwrap(),
            settings
        );
        replace_app_file_draft(&mut store, document, INVALID_JSON.into());
        let invalid = views
            .begin_write(&fixture.auxiliary, &mut store)
            .unwrap()
            .unwrap();
        host.submit(HostCommand::WriteAppFile(invalid.clone()))
            .unwrap();
        ready.notified().await;
        let HostReply::AppFileWritten { request, result } = host.poll().unwrap() else {
            panic!("expected typed invalid write completion");
        };
        assert!(request.same_request(&invalid));
        assert!(
            views
                .accept_write(&request, result, &mut store)
                .unwrap()
                .is_err()
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(
            std::fs::read_to_string(fixture.services.state.paths.settings_file()).unwrap(),
            canonical
        );
        assert!(store.documents().snapshot(document).unwrap().dirty);
        let retry = views
            .begin_write(&fixture.main, &mut store)
            .unwrap()
            .unwrap();
        assert!(views.cancel_write(&retry));
        assert!(!views.cancel_write(&retry));
        let request = views
            .begin_write(&fixture.auxiliary, &mut store)
            .unwrap()
            .unwrap();
        let signal = ready.clone();
        let mut disconnected = HostBridge::connect_with_clipboard_ports(
            fixture.services.clone(),
            Arc::new(move || signal.notify_one()),
            Arc::new(|_| panic!("clipboard write is outside this test")),
            Arc::new(|| panic!("clipboard read is outside this test")),
            None,
        )
        .unwrap();
        disconnected
            .submit(HostCommand::WriteAppFile(request.clone()))
            .unwrap();
        ready.notified().await;
        let HostReply::AppFileWritten {
            request: reply,
            result,
        } = disconnected.poll().unwrap()
        else {
            panic!("expected missing integration rejection");
        };
        assert!(matches!(
            views.accept_write(&reply, result, &mut store),
            Some(Err(AppError::Forbidden(_)))
        ));
        assert_eq!(
            std::fs::read_to_string(fixture.services.state.paths.settings_file()).unwrap(),
            canonical
        );
        let pending = views
            .begin_write(&fixture.main, &mut store)
            .unwrap()
            .unwrap();
        let layout = fixture.services.state.layouts.read()[&fixture.main.project].clone();
        let tab = crate::tabs::tabs_in(&layout.root)[0].clone();
        views.release_closed(&tab, &mut store).unwrap();
        assert!(!views.cancel_write(&pending));
        assert!(
            views
                .begin_write(&fixture.auxiliary, &mut store)
                .unwrap()
                .is_some()
        );
        views.clear();
        host.disconnect().await.unwrap();
        disconnected.disconnect().await.unwrap();
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_app_file_views는_공유문서_설정갱신_늦은응답과_마지막닫기를_보호한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let mut store = fixture.store();
        let mut views = Views::default();
        assert!(views.begin(&fixture.main, &store, false).is_none());
        let first = views.begin(&fixture.main, &store, true).unwrap();
        assert!(views.begin(&fixture.main, &store, true).is_none());
        assert!(views.document(&fixture.main).is_none());
        assert!(!views.failed(&fixture.main));
        assert!(views.accept(&first, first.execute(&fixture.services).await, &mut store));
        let document = views.document(&fixture.main).unwrap();
        assert!(views.begin(&fixture.auxiliary, &store, true).is_none());
        assert_eq!(views.document(&fixture.auxiliary), Some(document));
        assert!(!views.accept(
            &first,
            Err(AppError::Internal("duplicate".into())),
            &mut store
        ));
        for owner in [&fixture.main, &fixture.auxiliary] {
            store
                .attach_view(
                    ViewKey {
                        window: owner.pane.to_string(),
                        pane: owner.pane.clone(),
                        tab: owner.tab.clone(),
                    },
                    document,
                )
                .unwrap();
        }
        views.observe_settings_revision(1);
        let stale = views.begin(&fixture.main, &store, true).unwrap();
        let prepared = stale.execute(&fixture.services).await.unwrap();
        views.observe_settings_revision(1);
        assert!(views.begin(&fixture.main, &store, true).is_none());
        views.observe_settings_revision(2);
        assert!(!views.accept(&stale, Ok(prepared), &mut store));
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        let fresh = views.begin(&fixture.main, &store, true).unwrap();
        std::fs::write(
            fixture.services.state.paths.settings_file(),
            "{\"editorFontSize\":20}",
        )
        .unwrap();
        assert!(views.accept(&fresh, fresh.execute(&fixture.services).await, &mut store));
        let snapshot = store.documents().snapshot(document).unwrap();
        assert_eq!(snapshot.rope.to_string(), "{\"editorFontSize\":20}");
        store
            .apply(
                document,
                Transaction {
                    revision: snapshot.revision,
                    edits: vec![Edit {
                        bytes: 0..snapshot.rope.len_bytes(),
                        text: DRAFT.into(),
                    }],
                    group: UndoGroup(0),
                    origin: None,
                    selection_after: None,
                },
            )
            .unwrap();
        views.observe_settings_revision(3);
        let dirty_refresh = views.begin(&fixture.main, &store, true).unwrap();
        assert!(views.accept(
            &dirty_refresh,
            dirty_refresh.execute(&fixture.services).await,
            &mut store
        ));
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
            target_tabs(&fixture.services.state, fixture.main.target).len(),
            2
        );
        {
            let mut layouts = fixture.services.state.layouts.write();
            let layout = layouts.get_mut(&fixture.main.project).unwrap();
            taide_layout::service::set_dirty(layout, &fixture.main.tab, true).unwrap();
            taide_layout::service::set_dirty(layout, &fixture.auxiliary.tab, true).unwrap();
            let PaneNode::Leaf { active, .. } = &mut layout.root else {
                panic!("expected main leaf");
            };
            *active = None;
        }
        views
            .reconcile(&fixture.services.state, &mut store)
            .unwrap();
        assert_eq!(store.views().len(), 2);
        let tab = fixture.services.state.layouts.read()[&fixture.main.project]
            .root
            .clone();
        let tab = crate::tabs::tabs_in(&tab)[0].clone();
        let mut batch = crate::tab_close_batch::Batch::new(vec![tab]).unwrap();
        assert!(matches!(
            batch.next(),
            Some(crate::tab_close_batch::Task::Close(_))
        ));
        let closed = crate::tabs::close(&fixture.services, fixture.main.tab.clone(), false)
            .await
            .unwrap();
        assert!(closed.tab.dirty);
        views.release_closed(&closed.tab, &mut store).unwrap();
        assert_eq!(store.views().len(), 1);
        assert_eq!(views.document(&fixture.auxiliary), Some(document));
        let closed = crate::tabs::close(&fixture.services, fixture.auxiliary.tab.clone(), false)
            .await
            .unwrap();
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            DRAFT
        );
        views.release_closed(&closed.tab, &mut store).unwrap();
        views
            .reconcile(&fixture.services.state, &mut store)
            .unwrap();
        assert_eq!(store.views().len(), 0);
        assert!(store.documents().snapshot(document).unwrap().dirty);
        assert!(store.release_document(document).is_err());
        assert!(views.entries.is_empty());
        assert!(views.loaded.contains(&fixture.main.target));
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
        fixture.services.tasks.shutdown().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn native_app_file_host는_창별설정열기와_읽기실패_폐기수명을_보존한다() {
    tokio::time::timeout(DEADLINE, async {
        let fixture = Fixture::new();
        let ready = Arc::new(tokio::sync::Notify::new());
        let signal = ready.clone();
        let mut bridge = HostBridge::connect_with_clipboard_ports(
            fixture.services.clone(),
            Arc::new(move || signal.notify_one()),
            Arc::new(|_| panic!("unexpected clipboard write")),
            Arc::new(|| panic!("unexpected clipboard read")),
            None,
        )
        .unwrap();
        let layout = fixture.services.state.layouts.read()[&fixture.main.project].clone();
        for scope in [
            taide_native_ui::shell::WindowScope::Main,
            taide_native_ui::shell::WindowScope::Auxiliary {
                project: fixture.main.project.clone(),
                slot: 1,
            },
        ] {
            let command = settings_command(&fixture.main.project, &layout, &scope).unwrap();
            let HostCommand::OpenAppFile { pane, target, .. } = &command else {
                panic!("expected app file open");
            };
            assert_eq!(*target, AppFileTarget::Settings);
            assert_eq!(
                *pane,
                if matches!(scope, taide_native_ui::shell::WindowScope::Main) {
                    fixture.main.pane.clone()
                } else {
                    fixture.auxiliary.pane.clone()
                }
            );
            bridge.submit(command).unwrap();
            ready.notified().await;
            assert!(bridge.poll().is_none());
        }
        assert_eq!(owners(&fixture.services.state).len(), 2);
        assert!(
            settings_command(
                &ProjectId::new(),
                &layout,
                &taide_native_ui::shell::WindowScope::Auxiliary {
                    project: fixture.main.project.clone(),
                    slot: 1
                }
            )
            .is_none()
        );
        let mut store = fixture.store();
        let mut views = Views::default();
        let request = views.begin(&fixture.main, &store, true).unwrap();
        bridge
            .submit(HostCommand::ReadAppFile(request.clone()))
            .unwrap();
        ready.notified().await;
        let Some(HostReply::AppFile {
            request: reply_request,
            result,
        }) = bridge.poll()
        else {
            panic!("expected app file read reply");
        };
        assert!(request.same_request(&reply_request));
        assert!(views.accept(&reply_request, result, &mut store));
        assert!(views.document(&fixture.main).is_some());
        views.observe_settings_revision(1);
        let failed = views.begin(&fixture.main, &store, true).unwrap();
        assert!(views.accept(
            &failed,
            Err(AppError::Io("synthetic read failure".into())),
            &mut store
        ));
        assert!(views.failed(&fixture.main));
        assert!(views.document(&fixture.main).is_none());
        views.observe_settings_revision(2);
        let cancelled = views.begin(&fixture.main, &store, true).unwrap();
        views.clear();
        assert!(cancelled.execute(&fixture.services).await.is_err());
        drop(bridge.disconnect());
        fixture.services.tasks.shutdown().await;
        assert_eq!(fixture.services.tasks.tracked_count(), 0);
    })
    .await
    .unwrap();
}
