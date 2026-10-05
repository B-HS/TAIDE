use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use taide_model::app_event::AppEvent;
use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::TabKind;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::persistence::{MIRROR_DEBOUNCE, Persistence};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{AppState, EventSink, TaskSupervisor, layout_actions};
use tokio::sync::Notify;

const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const AUTO_SAVE: Duration = Duration::from_secs(1);
const BEFORE_DEADLINE: Duration = Duration::from_millis(1);
const NEXT_EDIT: Duration = Duration::from_millis(250);
const TIMEOUT: Duration = Duration::from_secs(3);
const PATH: &str = "/synthetic/persistence.rs";

fn store() -> EditorStore {
    EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap()
}

fn file() -> OpenedFile {
    OpenedFile {
        path: PATH.into(),
        content: "disk".into(),
        language_id: "rust".into(),
        byte_size: "disk".len().try_into().unwrap(),
        line_count: 1,
        tier: FileSizeTier::Normal,
        read_only: false,
        encoding_lossy: false,
        modified_ms: 1.0,
        editor_config: EditorConfigOptions::default(),
    }
}

#[test]
fn 문서별_지연은_재편집_저장중_편집_stale_reply와_실패를_결정적으로_구별한다() {
    let mut store = store();
    let document = store.open_file(PATH.into(), file()).unwrap();
    let untitled = store
        .open_untitled(TabId::new(), "", "plaintext".into())
        .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            document,
        )
        .unwrap();
    replace_selections(&mut store, view, "draft ", None).unwrap();
    let project = ProjectId::new();
    let now = Instant::now();
    let mut persistence = Persistence::default();
    persistence.changed(
        document,
        store.documents().snapshot(document).unwrap().revision,
        now,
        true,
        AUTO_SAVE,
    );
    persistence.changed(untitled, 0, now, true, Duration::ZERO);
    assert!(
        persistence
            .due(now + MIRROR_DEBOUNCE - BEFORE_DEADLINE)
            .is_empty()
    );
    replace_selections(&mut store, view, "next ", None).unwrap();
    persistence.changed(
        document,
        store.documents().snapshot(document).unwrap().revision,
        now + NEXT_EDIT,
        true,
        AUTO_SAVE,
    );
    let due = persistence.due(now + MIRROR_DEBOUNCE);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].document, untitled);
    assert!(due[0].mirror);
    assert!(!due[0].save);
    persistence.settled(untitled);
    let job = persistence
        .mirror_job(
            store.documents().snapshot(document).unwrap(),
            project.clone(),
        )
        .unwrap();
    persistence.mirror_started(&job);
    assert!(
        persistence
            .due(now + NEXT_EDIT + MIRROR_DEBOUNCE)
            .is_empty()
    );
    replace_selections(&mut store, view, "later ", None).unwrap();
    persistence.changed(
        document,
        store.documents().snapshot(document).unwrap().revision,
        now + MIRROR_DEBOUNCE,
        true,
        AUTO_SAVE,
    );
    persistence.mirror_finished(&job, true, now + MIRROR_DEBOUNCE);
    assert!(persistence.due(now + MIRROR_DEBOUNCE + MIRROR_DEBOUNCE)[0].mirror);
    let epoch = persistence.begin_save(document).unwrap();
    assert!(persistence.begin_save(document).is_none());
    persistence.submission_failed(document);
    assert!(persistence.due(now + AUTO_SAVE + AUTO_SAVE)[0].save);
    let retried = persistence.begin_save(document).unwrap();
    assert!(retried.is_current());
    assert!(persistence.due(now + AUTO_SAVE + AUTO_SAVE).is_empty());
    assert!(persistence.next_wake().is_none());
    replace_selections(&mut store, view, "during save ", None).unwrap();
    persistence.changed(
        document,
        store.documents().snapshot(document).unwrap().revision,
        now,
        true,
        AUTO_SAVE,
    );
    assert!(persistence.due(now + AUTO_SAVE).is_empty());
    epoch.invalidate();
    persistence.save_finished(document, now, true, true, AUTO_SAVE);
    let due = persistence.due(now);
    assert_eq!(due.len(), 1);
    assert!(due[0].mirror);
    assert!(!due[0].save);
    let fresh = persistence
        .mirror_job(
            store.documents().snapshot(document).unwrap(),
            project.clone(),
        )
        .unwrap();
    persistence.mirror_started(&fresh);
    persistence.mirror_finished(&job, true, now);
    assert!(persistence.due(now).is_empty());
    persistence.mirror_finished(&fresh, true, now);
    assert!(persistence.due(now + AUTO_SAVE)[0].save);
    persistence.disable_auto_save(document);
    assert!(persistence.next_wake().is_none());
    persistence.settled(document);
    assert!(!fresh.epoch.is_current());
    persistence.changed(
        document,
        store.documents().snapshot(document).unwrap().revision,
        now,
        true,
        Duration::ZERO,
    );
    let failed = persistence
        .mirror_job(store.documents().snapshot(document).unwrap(), project)
        .unwrap();
    assert!(failed.epoch.is_current());
    assert!(persistence.due(now + MIRROR_DEBOUNCE)[0].mirror);
    persistence.mirror_started(&failed);
    persistence.mirror_finished(&failed, false, now);
    assert!(persistence.next_wake().is_none());
    assert!(persistence.due(now + AUTO_SAVE).is_empty());
    let pending_save = persistence.begin_save(document).unwrap();
    persistence.cancel_inflight();
    assert!(!pending_save.is_current());
    assert!(persistence.begin_save(document).is_some());
    persistence.submission_failed(document);
}

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
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
fn 실제_worker는_저장뒤_stale_mirror_늦은_초안_readonly_실패와_untitled를_보호한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir().join(format!("taide-native-persistence-{}", ProjectId::new())),
            );
            let root = directory.0.join("root");
            std::fs::create_dir_all(&root).unwrap();
            let path = root.join("editor.rs");
            std::fs::write(&path, "disk").unwrap();
            let path = std::fs::canonicalize(path)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            state.projects.write().insert(
                project.clone(),
                Project {
                    id: project.clone(),
                    root: root.to_str().unwrap().into(),
                    name: "synthetic persistence".into(),
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
            let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
            layout_actions::layout_open_tab(
                services.events.as_ref(),
                &state,
                project.clone(),
                TabKind::File { path: path.clone() },
                "editor.rs".into(),
                None,
                false,
            )
            .await
            .unwrap();
            let ready = Arc::new(Notify::new());
            let signal = ready.clone();
            let mut bridge =
                HostBridge::connect(services.clone(), Arc::new(move || signal.notify_one()))
                    .unwrap();
            let mut store = store();
            bridge
                .submit(HostCommand::OpenDocument(path.clone()))
                .unwrap();
            let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected document");
            };
            let document = result.unwrap().commit(&mut store).unwrap();
            let tab = taide_native_app::tabs::tabs_in(&state.layouts.read()[&project].root)
                .into_iter()
                .find(|tab| matches!(tab.kind, TabKind::File { .. }))
                .unwrap()
                .id
                .clone();
            let view = store
                .attach_view(
                    ViewKey {
                        window: "main".into(),
                        pane: state.layouts.read()[&project].focused_pane.clone(),
                        tab,
                    },
                    document,
                )
                .unwrap();
            replace_selections(&mut store, view, "draft ", None).unwrap();
            let now = Instant::now();
            let mut persistence = Persistence::default();
            persistence.changed(
                document,
                store.documents().snapshot(document).unwrap().revision,
                now,
                true,
                AUTO_SAVE,
            );
            let old = persistence
                .mirror_job(
                    store.documents().snapshot(document).unwrap(),
                    project.clone(),
                )
                .unwrap();
            bridge
                .submit(HostCommand::MirrorDraft(old.clone()))
                .unwrap();
            let HostReply::DraftMirrored { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected mirror");
            };
            assert!(result.unwrap());
            assert_eq!(
                taide_file::service::list_mirrors(&state.paths, &project).unwrap()[0].content,
                "draft disk"
            );
            let epoch = persistence.begin_save(document).unwrap();
            bridge
                .submit(HostCommand::SaveTracked {
                    path: path.clone(),
                    snapshot: store.save_snapshot(document).unwrap(),
                    epoch: epoch.clone(),
                })
                .unwrap();
            bridge
                .submit(HostCommand::MirrorDraft(old.clone()))
                .unwrap();
            replace_selections(&mut store, view, "late ", None).unwrap();
            persistence.changed(
                document,
                store.documents().snapshot(document).unwrap().revision,
                now,
                true,
                AUTO_SAVE,
            );
            let HostReply::Saved { snapshot, result } = reply(&mut bridge, &ready).await else {
                panic!("expected save");
            };
            let disk = result.unwrap();
            assert!(!epoch.is_current());
            assert!(!store.mark_saved(snapshot, Some(disk.modified_ms)).unwrap());
            let HostReply::DraftMirrored { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected stale mirror");
            };
            assert!(!result.unwrap());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "draft disk");
            assert!(
                taide_file::service::list_mirrors(&state.paths, &project)
                    .unwrap()
                    .is_empty()
            );
            persistence.save_finished(document, now, true, true, AUTO_SAVE);
            let fresh = persistence
                .mirror_job(
                    store.documents().snapshot(document).unwrap(),
                    project.clone(),
                )
                .unwrap();
            bridge
                .submit(HostCommand::MirrorDraft(fresh.clone()))
                .unwrap();
            let HostReply::DraftMirrored { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected late draft mirror");
            };
            assert!(result.unwrap());
            assert_eq!(
                taide_file::service::list_mirrors(&state.paths, &project).unwrap()[0].content,
                "draft late disk"
            );
            let epoch = persistence.begin_save(document).unwrap();
            let snapshot = store.save_snapshot(document).unwrap();
            std::fs::write(&path, [u8::MAX]).unwrap();
            bridge
                .submit(HostCommand::SaveTracked {
                    path: path.clone(),
                    snapshot,
                    epoch: epoch.clone(),
                })
                .unwrap();
            let HostReply::Saved { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected refused readonly save");
            };
            assert!(result.is_err());
            assert!(epoch.is_current());
            assert_eq!(std::fs::read(&path).unwrap(), [u8::MAX]);
            assert_eq!(
                taide_file::service::list_mirrors(&state.paths, &project).unwrap()[0].content,
                "draft late disk"
            );
            layout_actions::layout_open_untitled(
                services.events.as_ref(),
                &state,
                project.clone(),
                None,
            )
            .await
            .unwrap();
            let tab = taide_native_app::tabs::tabs_in(&state.layouts.read()[&project].root)
                .into_iter()
                .find(|tab| matches!(tab.kind, TabKind::Untitled { .. }))
                .unwrap()
                .id
                .clone();
            let untitled = store
                .restore_untitled(tab.clone(), Some("untitled draft"))
                .unwrap();
            persistence.changed(untitled, 0, now, true, Duration::ZERO);
            let job = persistence
                .mirror_job(
                    store.documents().snapshot(untitled).unwrap(),
                    project.clone(),
                )
                .unwrap();
            bridge
                .submit(HostCommand::MirrorDraft(job.clone()))
                .unwrap();
            let HostReply::DraftMirrored { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected untitled mirror");
            };
            assert!(result.unwrap());
            assert_eq!(
                taide_file::service::list_untitled_mirrors(&state.paths, &project).unwrap()[0]
                    .content,
                "untitled draft"
            );
            persistence.settled(untitled);
            bridge.submit(HostCommand::MirrorDraft(job)).unwrap();
            let HostReply::DraftMirrored { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected cancelled untitled mirror");
            };
            assert!(!result.unwrap());
            assert_eq!(
                taide_file::service::list_untitled_mirrors(&state.paths, &project).unwrap()[0]
                    .content,
                "untitled draft"
            );
            bridge.disconnect().await.unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}
