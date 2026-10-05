use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::TabKind;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::persistence::DraftEpoch;
use taide_native_app::save;
use taide_native_editor::document::EditorError;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::save_cleanup::CleanupFlags;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{ScrollPosition, Selection, SelectionSet, ViewKey};
use taide_runtime::{AppState, EventSink, TaskSupervisor, layout_actions};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(3);
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const INITIAL: &str = "head \t\r\n尾 \t";
const EDITOR_CONFIG: &str =
    "root = true\n\n[*.txt]\ntrim_trailing_whitespace = true\ninsert_final_newline = true\n";

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
fn 실제_저장은_editorconfig_자동_커서_보호_수동_정리와_늦은_초안을_연결한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir().join(format!("taide-native-save-{}", ProjectId::new())),
            );
            let root = directory.0.join("root");
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(root.join(".editorconfig"), EDITOR_CONFIG).unwrap();
            let path = root.join("editor.txt");
            std::fs::write(&path, INITIAL).unwrap();
            let path = std::fs::canonicalize(path)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            let project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            state.settings.write().editor_config_enabled = true;
            state.projects.write().insert(
                project.clone(),
                Project {
                    id: project.clone(),
                    root: root.to_str().unwrap().into(),
                    name: "synthetic save".into(),
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
                "editor.txt".into(),
                None,
                false,
            )
            .await
            .unwrap();
            let ready = Arc::new(Notify::new());
            let signal = ready.clone();
            let mut bridge =
                HostBridge::connect(services, Arc::new(move || signal.notify_one())).unwrap();
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            bridge
                .submit(HostCommand::OpenDocument(path.clone()))
                .unwrap();
            let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected document");
            };
            let document = result.unwrap().commit(&mut store).unwrap();
            let layout = state.layouts.read()[&project].clone();
            let tab = taide_native_app::tabs::tabs_in(&layout.root)
                .into_iter()
                .find(|tab| matches!(tab.kind, TabKind::File { .. }))
                .unwrap()
                .id
                .clone();
            let view = store
                .attach_view(
                    ViewKey {
                        window: "main".into(),
                        pane: layout.focused_pane,
                        tab,
                    },
                    document,
                )
                .unwrap();
            let flags = CleanupFlags {
                trim_trailing_whitespace: false,
                insert_final_newline: false,
            };
            let clean = store.save_snapshot(document).unwrap();
            let initial_revision = clean.revision();
            assert!(
                save::prepare(&mut store, clean, Some(view), flags, false)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                store.documents().snapshot(document).unwrap().revision,
                initial_revision
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), INITIAL);
            replace_selections(&mut store, view, "draft ", None).unwrap();
            let stale = store.save_snapshot(document).unwrap();
            replace_selections(&mut store, view, "next ", None).unwrap();
            assert!(matches!(
                save::prepare(&mut store, stale, Some(view), flags, true),
                Err(EditorError::StaleRevision)
            ));
            let end = store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .len_bytes();
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection {
                            anchor: end,
                            head: end,
                        }],
                    },
                    ScrollPosition::default(),
                    Vec::new(),
                )
                .unwrap();
            let requested = store.save_snapshot(document).unwrap();
            let prepared = save::prepare(&mut store, requested, Some(view), flags, true)
                .unwrap()
                .unwrap();
            assert!(prepared.changed);
            assert_eq!(
                prepared.snapshot.rope().to_string(),
                "draft next head\r\n尾 \t\r\n"
            );
            assert_eq!(
                store.views().get(view).unwrap().selection.selections[0].head,
                "draft next head\r\n尾 \t".len()
            );
            let epoch = DraftEpoch::default();
            bridge
                .submit(HostCommand::SaveTracked {
                    path: path.clone(),
                    snapshot: prepared.snapshot,
                    epoch: epoch.clone(),
                })
                .unwrap();
            replace_selections(&mut store, view, "later", None).unwrap();
            let HostReply::Saved { snapshot, result } = reply(&mut bridge, &ready).await else {
                panic!("expected saved snapshot");
            };
            let written = result.unwrap();
            assert!(!epoch.is_current());
            assert_eq!(written.content, "draft next head\r\n尾 \t\r\n");
            assert!(
                !store
                    .mark_saved(snapshot, Some(written.modified_ms))
                    .unwrap()
            );
            assert!(store.documents().snapshot(document).unwrap().dirty);
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                "draft next head\r\n尾 \tlater\r\n"
            );
            let end = store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .len_bytes();
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection {
                            anchor: end - "\r\n".len(),
                            head: end - "\r\n".len(),
                        }],
                    },
                    ScrollPosition::default(),
                    Vec::new(),
                )
                .unwrap();
            replace_selections(&mut store, view, " \t", None).unwrap();
            let requested = store.save_snapshot(document).unwrap();
            let prepared = save::prepare(&mut store, requested, Some(view), flags, false)
                .unwrap()
                .unwrap();
            assert!(prepared.changed);
            assert_eq!(
                prepared.snapshot.rope().to_string(),
                "draft next head\r\n尾 \tlater\r\n"
            );
            bridge
                .submit(HostCommand::SaveTracked {
                    path: path.clone(),
                    snapshot: prepared.snapshot,
                    epoch: DraftEpoch::default(),
                })
                .unwrap();
            let HostReply::Saved { snapshot, result } = reply(&mut bridge, &ready).await else {
                panic!("expected explicit save");
            };
            let written = result.unwrap();
            assert!(
                store
                    .mark_saved(snapshot, Some(written.modified_ms))
                    .unwrap()
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), written.content);
            let clean = store.save_snapshot(document).unwrap();
            assert!(
                save::prepare(&mut store, clean.clone(), Some(view), flags, false)
                    .unwrap()
                    .is_none()
            );
            let mut readonly = written;
            readonly.read_only = true;
            store
                .observe_file(document, std::path::Path::new(&path), readonly)
                .unwrap();
            assert!(matches!(
                save::prepare(&mut store, clean, Some(view), flags, false),
                Err(EditorError::ReadOnly)
            ));
            let untitled = store
                .open_untitled(TabId::new(), "draft", "plaintext".into())
                .unwrap();
            let requested = store.save_snapshot(untitled).unwrap();
            assert!(matches!(
                save::prepare(&mut store, requested, None, flags, false),
                Err(EditorError::InvalidIdentity)
            ));
            bridge.disconnect().await.unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}
