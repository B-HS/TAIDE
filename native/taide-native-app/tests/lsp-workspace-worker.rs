use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use taide_lsp::native::protocol::lsp_types::{self, WorkspaceEdit};
use taide_model::app_event::AppEvent;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::services;
use taide_native_app::lsp::Reply;
use taide_native_app::lsp_workspace::apply_open;
use taide_native_app::lsp_workspace_worker::{self, uri_path};
use taide_native_editor::document::DocumentKey;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{AppServices, AppState, EventSink, TaskSupervisor};

const TIMEOUT: Duration = Duration::from_secs(5);
const QUEUE_CAPACITY: usize = 8;
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const FILE_MODE: u32 = 0o640;
const FILE_MODE_MASK: u32 = 0o777;

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

fn create(path: &Path, overwrite: bool, ignore_if_exists: bool) -> WorkspaceEdit {
    serde_json::from_value(serde_json::json!({"documentChanges":[{
        "kind":"create", "uri":taide_lsp::service::workspace_folder_uri(path.to_str().unwrap()),
        "options":{"overwrite":overwrite,"ignoreIfExists":ignore_if_exists}
    }]}))
    .unwrap()
}

#[test]
fn 실제_workspace_create는_생성후_편집과_옵션_우선순위_순차실패_root를_보존한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(std::env::temp_dir().join(format!(
                "taide-native-workspace-create-{}", ProjectId::new()
            )));
            let root = directory.0.join("root");
            std::fs::create_dir_all(&root).unwrap();
            let root = std::fs::canonicalize(root).unwrap();
            let project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            state.projects.write().insert(project.clone(), Project {
                id: project.clone(), root: root.to_str().unwrap().into(), name: "synthetic create".into(),
                capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default(),
            });
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT, max_views: VIEW_LIMIT, max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            }).unwrap();
            let path = root.join("nested/new 文.txt");
            let uri = taide_lsp::service::workspace_folder_uri(path.to_str().unwrap());
            let create_then_edit = serde_json::from_value(serde_json::json!({"documentChanges":[
                {"kind":"create","uri":uri},
                {"textDocument":{"uri":uri,"version":null},"edits":[{
                    "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},
                    "newText":"created:文\r\n"
                }]}
            ]})).unwrap();
            apply(&services, &mut store, create_then_edit, Some(vec![root.to_str().unwrap().into()])).await.unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "created:文\r\n");
            assert!(apply(&services, &mut store, create(&path, false, false), None).await.is_err());
            apply(&services, &mut store, create(&path, false, true), None).await.unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "created:文\r\n");
            let document = store.open_file(path.clone(), taide_file::service::open_file(&path, &[], false).unwrap()).unwrap();
            let view = store.attach_view(ViewKey {window: "main".into(), pane: PaneId::new(), tab: TabId::new()}, document).unwrap();
            replace_selections(&mut store, view, "unsaved:", None).unwrap();
            taide_file::service::mirror_dirty(&state.paths, &project, &path, path.to_str().unwrap(), "restorable").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(FILE_MODE)).unwrap();
            }
            apply(&services, &mut store, create(&path, true, true), None).await.unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
            assert!(!taide_file::service::has_mirror(&state.paths, &project, &path).unwrap());
            assert_eq!(store.documents().snapshot(document).unwrap().rope.to_string(), "unsaved:created:文\r\n");
            assert!(store.documents().snapshot(document).unwrap().dirty);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & FILE_MODE_MASK, FILE_MODE);
            }
            let prefix = root.join("prefix.txt");
            let outside = directory.0.join("outside.txt");
            let after = root.join("after.txt");
            let sequence = serde_json::from_value(serde_json::json!({"documentChanges":[
                {"kind":"create","uri":taide_lsp::service::workspace_folder_uri(prefix.to_str().unwrap())},
                {"kind":"create","uri":taide_lsp::service::workspace_folder_uri(outside.to_str().unwrap())},
                {"kind":"create","uri":taide_lsp::service::workspace_folder_uri(after.to_str().unwrap())}
            ]})).unwrap();
            assert!(apply(&services, &mut store, sequence, None).await.is_err());
            assert!(prefix.is_file());
            assert!(!outside.exists());
            assert!(!after.exists());
            std::fs::write(&outside, "CLI").unwrap();
            state.authorize_cli_opened_path(&outside);
            assert!(apply(&services, &mut store, create(&outside, false, true), None).await.is_err());
            assert_eq!(std::fs::read_to_string(&outside).unwrap(), "CLI");
            apply(&services, &mut store, create(&outside, true, false), None).await.unwrap();
            assert_eq!(std::fs::read_to_string(&outside).unwrap(), "");
            std::fs::write(&outside, "scope").unwrap();
            assert!(apply(&services, &mut store, create(&outside, true, false), Some(vec![root.to_str().unwrap().into()])).await.is_err());
            assert_eq!(std::fs::read_to_string(&outside).unwrap(), "scope");
            #[cfg(unix)]
            {
                let target = directory.0.join("unapproved.txt");
                std::fs::write(&target, "outside").unwrap();
                let link = root.join("escape.txt");
                std::os::unix::fs::symlink(&target, &link).unwrap();
                assert!(apply(&services, &mut store, create(&link, true, false), None).await.is_err());
                assert_eq!(std::fs::read_to_string(target).unwrap(), "outside");
            }
            tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}
struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn edit(path: &Path, start: u32, end: u32, text: &str) -> WorkspaceEdit {
    let uri = taide_lsp::service::workspace_folder_uri(path.to_str().unwrap());
    serde_json::from_value(serde_json::json!({"changes":{uri:[{
        "range":{"start":{"line":0,"character":start},"end":{"line":0,"character":end}},"newText":text
    }]}})).unwrap()
}

async fn apply(
    services: &Arc<AppServices>,
    store: &mut EditorStore,
    edit: WorkspaceEdit,
    roots: Option<Vec<String>>,
) -> taide_model::error::AppResult<()> {
    apply_with_documents(services, store, edit, roots, Vec::new()).await
}

async fn apply_with_documents(
    services: &Arc<AppServices>,
    store: &mut EditorStore,
    edit: WorkspaceEdit,
    roots: Option<Vec<String>>,
    documents: Vec<taide_native_app::lsp::ProtocolDocument>,
) -> taide_model::error::AppResult<()> {
    let (sender, mut replies) = tokio::sync::mpsc::channel(QUEUE_CAPACITY);
    let services_for_edit = services.clone();
    let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let (completion, mut completed) = tokio::sync::oneshot::channel();
    let mut activity = None;
    let worker = services
        .tasks
        .spawn_transient_handle("synthetic-workspace-edit", async move {
            let result = lsp_workspace_worker::apply(
                &services_for_edit,
                edit,
                documents,
                roots,
                &sender,
                &repaint,
            )
            .await;
            let _result = completion.send(result);
        })
        .unwrap();
    let result = tokio::time::timeout(TIMEOUT, async {
        loop {
            tokio::select! {
                result = &mut completed => break result.unwrap(),
                reply = replies.recv() => {
                    let Some(reply) = reply else {break completed.await.unwrap();};
                    match reply {
                        Reply::DocumentQuery {path, completion} => {
                            let document = store.documents().find(&DocumentKey::File(path)).and_then(|document| store.documents().snapshot(document).ok());
                            let _result = completion.send(document);
                        }
                        Reply::WorkspaceEdit(event) => {
                            let outcome = apply_open(store, &event);
                            assert_eq!(outcome.failure, None);
                            let _result = event.completion.send(true);
                        }
                        Reply::DeletePrepare(event) => {
                            activity = Some(event.activity.clone());
                            let result = taide_native_app::workspace_delete::prepare_documents(store, &event.path, &std::collections::HashSet::new());
                            let _result = event.completion.send(result);
                        }
                        Reply::RenamePrepare(event) => {
                            activity = Some(event.activity.clone());
                            let result = taide_native_app::workspace_rename::prepare_documents(store, &event.from, &std::collections::HashMap::new());
                            let _result = event.completion.send(result);
                        }
                        Reply::Renamed(event) => {
                            assert!(tokio::time::timeout(Duration::ZERO, services.state.begin_owned_mutation()).await.is_err());
                            assert!(event.warnings.is_empty());
                            let result = taide_native_app::workspace_rename::commit_documents(store, &event);
                            let _result = event.completion.send(result);
                        }
                        Reply::Deleted(event) => {
                            assert!(tokio::time::timeout(Duration::ZERO, services.state.begin_owned_mutation()).await.is_err());
                            let result = taide_native_app::workspace_delete::commit_documents(store, &event, &std::collections::HashSet::new());
                            let _result = event.completion.send(result);
                        }
                        _ => panic!("unexpected workspace worker reply"),
                    }
                }
            }
        }
    }).await.unwrap();
    worker.await.unwrap();
    if let Some(activity) = activity {
        assert!(!activity.is_active());
    }
    result
}

fn delete(path: &Path, ignore_missing: bool) -> WorkspaceEdit {
    serde_json::from_value(serde_json::json!({"documentChanges":[{
        "kind":"delete", "uri":taide_lsp::service::workspace_folder_uri(path.to_str().unwrap()),
        "options":{"ignoreIfNotExists":ignore_missing,"recursive":false}
    }]}))
    .unwrap()
}

fn rename(from: &Path, to: &Path, overwrite: bool, ignore_exists: bool) -> WorkspaceEdit {
    serde_json::from_value(serde_json::json!({"documentChanges":[{
        "kind":"rename", "oldUri":taide_lsp::service::workspace_folder_uri(from.to_str().unwrap()),
        "newUri":taide_lsp::service::workspace_folder_uri(to.to_str().unwrap()),
        "options":{"overwrite":overwrite,"ignoreIfExists":ignore_exists}
    }]}))
    .unwrap()
}

#[test]
fn 실제_workspace_rename는_모든_프로젝트_탭과_dirty_본문_미러를_옮기고_후속_편집을_새_uri에_적용한다()
 {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let directory = Directory(std::env::temp_dir().join(format!("taide-native-workspace-rename-{}", ProjectId::new())));
        let root = directory.0.join("root");
        let nested = root.join("nested");
        let folder = nested.join("work");
        std::fs::create_dir_all(&folder).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let nested = std::fs::canonicalize(nested).unwrap();
        let folder = std::fs::canonicalize(folder).unwrap();
        let projects = [ProjectId::new(), ProjectId::new()];
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        for (id, path) in projects.iter().zip([&root, &nested]) {
            state.projects.write().insert(id.clone(), Project {
                id: id.clone(), root: path.to_str().unwrap().into(), name: "synthetic rename".into(),
                capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default(),
            });
            state.layouts.write().insert(id.clone(), taide_layout::service::default_layout());
        }
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
        let path = folder.join("old 文.txt");
        let target = folder.join("new 文.rs");
        let unrelated = nested.join("work-other.txt");
        std::fs::write(&path, "disk\r\n").unwrap();
        std::fs::write(&unrelated, "untouched").unwrap();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT, max_views: VIEW_LIMIT, max_undo_groups: HISTORY_LIMIT, max_document_bytes: BYTE_LIMIT,
        }).unwrap();
        let document = store.open_file(path.clone(), taide_file::service::open_file(&path, &[], false).unwrap()).unwrap();
        let mut views = Vec::new();
        for project in &projects {
            taide_runtime::layout_actions::layout_open_tab(services.events.as_ref(), &state, project.clone(),
                taide_model::layout::TabKind::File {path: path.to_str().unwrap().into()}, "old 文.txt".into(), None, false).await.unwrap();
            let (pane, tab) = {
                let layouts = state.layouts.read();
                let layout = &layouts[project];
                let tab = taide_native_app::tabs::tabs_in(&layout.root).into_iter()
                    .find(|tab| matches!(&tab.kind, taide_model::layout::TabKind::File {path: file} if Path::new(file) == path)).unwrap().id.clone();
                (layout.focused_pane.clone(), tab)
            };
            views.push(store.attach_view(ViewKey {window: project.to_string(), pane, tab}, document).unwrap());
        }
        replace_selections(&mut store, views[0], "latest:", None).unwrap();
        for project in &projects {
            taide_file::service::mirror_dirty(&state.paths, project, &path, path.to_str().unwrap(), "debounced old draft").unwrap();
        }
        let collision = folder.join("collision.txt");
        std::fs::write(&collision, "keep target").unwrap();
        assert!(apply(&services, &mut store, rename(&path, &collision, false, false), None).await.is_err());
        apply(&services, &mut store, rename(&path, &collision, false, true), None).await.unwrap();
        assert!(apply(&services, &mut store, rename(&path, &collision, true, true), None).await.is_err());
        assert_eq!(std::fs::read_to_string(&collision).unwrap(), "keep target");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "disk\r\n");
        assert!(apply(&services, &mut store, rename(&path, &root.join("outside-nested.rs"), false, false), None).await.is_err());
        assert!(apply(&services, &mut store, rename(&path, &directory.0.join("outside.rs"), false, false), None).await.is_err());
        assert!(apply(&services, &mut store, rename(&path, &target, false, false), Some(vec![root.join("scope").to_str().unwrap().into()])).await.is_err());
        let late_save = store.save_snapshot(document).unwrap();
        let mut stale_file = taide_file::service::open_file(&path, &[], false).unwrap();
        stale_file.path = target.to_str().unwrap().into();
        let stale_target = store.open_file(target.clone(), stale_file).unwrap();
        let target_view = store.attach_view(ViewKey {window:"stale".into(), pane:PaneId::new(), tab:TabId::new()}, stale_target).unwrap();
        replace_selections(&mut store, target_view, "stale dirty target:", None).unwrap();
        let old_uri = taide_lsp::service::workspace_folder_uri(path.to_str().unwrap());
        let new_uri = taide_lsp::service::workspace_folder_uri(target.to_str().unwrap());
        let context = taide_native_app::lsp::ProtocolDocument {snapshot: store.documents().snapshot(document).unwrap(), uri:old_uri.clone(), revision:Some(0)};
        let sequence = serde_json::from_value(serde_json::json!({"documentChanges":[
            {"kind":"rename","oldUri":old_uri,"newUri":new_uri},
            {"textDocument":{"uri":new_uri,"version":null},"edits":[{
                "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"server:"
            }]}
        ]})).unwrap();
        apply_with_documents(&services, &mut store, sequence, None, vec![context]).await.unwrap();
        let current = store.documents().snapshot(document).unwrap();
        assert_eq!(current.key, DocumentKey::File(target.clone()));
        assert_eq!(current.rope.to_string(), "server:latest:disk\r\n");
        assert!(current.dirty);
        assert_eq!(current.metadata.language_id, "rust");
        assert!(!path.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "disk\r\n");
        assert!(store.documents().snapshot(stale_target).is_err());
        assert_eq!(store.views().get(target_view).unwrap().document, document);
        assert!(store.mark_saved(late_save, None).is_err());
        for project in &projects {
            let mirrors = taide_file::service::list_mirrors(&state.paths, project).unwrap();
            assert_eq!(mirrors.len(), 1);
            assert_eq!(mirrors[0].path, target.to_str().unwrap());
            assert_eq!(mirrors[0].content, "latest:disk\r\n");
            assert!(!taide_file::service::has_mirror(&state.paths, project, &path).unwrap());
            assert!(taide_native_app::tabs::tabs_in(&state.layouts.read()[project].root).iter()
                .any(|tab| matches!(&tab.kind, taide_model::layout::TabKind::File {path} if Path::new(path) == target)));
        }
        let relocated_folder = nested.join("renamed");
        apply(&services, &mut store, rename(&folder, &relocated_folder, false, false), None).await.unwrap();
        let relocated = relocated_folder.join(target.file_name().unwrap());
        assert_eq!(store.documents().snapshot(document).unwrap().key, DocumentKey::File(relocated.clone()));
        assert_eq!(store.documents().snapshot(document).unwrap().rope.to_string(), "server:latest:disk\r\n");
        assert!(!folder.exists());
        for project in &projects {
            let mirrors = taide_file::service::list_mirrors(&state.paths, project).unwrap();
            assert_eq!(mirrors.len(), 1);
            assert_eq!(mirrors[0].path, relocated.to_str().unwrap());
            assert_eq!(mirrors[0].content, "server:latest:disk\r\n");
        }
        assert_eq!(std::fs::read_to_string(unrelated).unwrap(), "untouched");
        let closed_source = relocated_folder.join("closed.txt");
        let closed_target = relocated_folder.join("closed-new.txt");
        std::fs::write(&closed_source, "closed disk").unwrap();
        for project in &projects {
            let mut layouts = state.layouts.write();
            let layout = layouts.get_mut(project).unwrap();
            let pane = layout.focused_pane.clone();
            let source_tab = taide_model::layout::Tab {
                id: TabId::new(), kind: taide_model::layout::TabKind::File {path:closed_source.to_str().unwrap().into()},
                title: "closed.txt".into(), pinned: false, preview: false, dirty: false, view_state: None,
            };
            let tab = taide_layout::service::open_tab(layout, &pane, source_tab, false).unwrap();
            taide_layout::service::close_tab(layout, &tab).unwrap();
        }
        apply(&services, &mut store, rename(&closed_source, &closed_target, false, false), None).await.unwrap();
        for project in &projects {
            assert!(matches!(&state.layouts.read()[project].closed_tabs[0].tab.kind, taide_model::layout::TabKind::File {path} if Path::new(path) == closed_target));
        }
        let unopened = relocated_folder.join("unopened.txt");
        let unopened_target = relocated_folder.join("unopened-new.txt");
        std::fs::write(&unopened, "unopened").unwrap();
        apply(&services, &mut store, rename(&unopened, &unopened_target, false, false), None).await.unwrap();
        assert_eq!(std::fs::read_to_string(unopened_target).unwrap(), "unopened");
        tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn 실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let directory = Directory(std::env::temp_dir().join(format!("taide-native-workspace-delete-{}", ProjectId::new())));
        let root = directory.0.join("root");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let nested = std::fs::canonicalize(nested).unwrap();
        let projects = [ProjectId::new(), ProjectId::new()];
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        for (id, path) in projects.iter().zip([&root, &nested]) {
            state.projects.write().insert(id.clone(), Project {
                id: id.clone(), root: path.to_str().unwrap().into(), name: "synthetic delete".into(),
                capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default(),
            });
            state.layouts.write().insert(id.clone(), taide_layout::service::default_layout());
        }
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
        let path = nested.join("delete-synthetic.txt");
        let other = nested.join("delete-synthetic.txt-other");
        std::fs::write(&path, "disk").unwrap();
        std::fs::write(&other, "unrelated").unwrap();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT, max_views: VIEW_LIMIT, max_undo_groups: HISTORY_LIMIT, max_document_bytes: BYTE_LIMIT,
        }).unwrap();
        let document = store.open_file(path.clone(), taide_file::service::open_file(&path, &[], false).unwrap()).unwrap();
        let unrelated = store.open_file(other.clone(), taide_file::service::open_file(&other, &[], false).unwrap()).unwrap();
        let mut views = Vec::new();
        for project in &projects {
            taide_runtime::layout_actions::layout_open_tab(services.events.as_ref(), &state, project.clone(),
                taide_model::layout::TabKind::File {path: path.to_str().unwrap().into()}, "delete-synthetic.txt".into(), None, false).await.unwrap();
            let (pane, tab) = {
                let layouts = state.layouts.read();
                let layout = &layouts[project];
                let tab = taide_native_app::tabs::tabs_in(&layout.root).into_iter()
                    .find(|tab| matches!(&tab.kind, taide_model::layout::TabKind::File {path: file} if std::path::Path::new(file) == path)).unwrap().id.clone();
                (layout.focused_pane.clone(), tab)
            };
            views.push(store.attach_view(ViewKey {window: project.to_string(), pane, tab}, document).unwrap());
        }
        replace_selections(&mut store, views[0], "unsaved:", None).unwrap();
        assert!(apply(&services, &mut store, delete(&path, false), None).await.is_err());
        assert!(path.is_file());
        assert_eq!(store.views().len(), projects.len());
        let save = store.save_snapshot(document).unwrap();
        taide_runtime::file_actions::file_save(&state, &tasks, path.to_str().unwrap().into(), save.rope().to_string()).await.unwrap();
        store.mark_saved(save, None).unwrap();
        taide_file::service::mirror_dirty(&state.paths, &projects[1], &path, path.to_str().unwrap(), "mirror").unwrap();
        assert!(apply(&services, &mut store, delete(&path, false), None).await.is_err());
        assert!(path.is_file());
        assert_eq!(taide_file::service::list_mirrors(&state.paths, &projects[1]).unwrap()[0].content, "mirror");
        taide_file::service::clear_mirror(&state.paths, &projects[1], &path).unwrap();
        assert!(apply(&services, &mut store, delete(&path, false), Some(vec![root.join("scope").to_str().unwrap().into()])).await.is_err());
        assert!(path.is_file());
        let missing = nested.join("missing.txt");
        apply(&services, &mut store, delete(&missing, true), None).await.unwrap();
        assert!(!missing.exists());
        assert!(apply(&services, &mut store, delete(&directory.0.join("outside.txt"), true), None).await.is_err());
        let uri = taide_lsp::service::workspace_folder_uri(path.to_str().unwrap());
        let context = taide_native_app::lsp::ProtocolDocument {
            snapshot: store.documents().snapshot(document).unwrap(), uri: uri.clone(), revision: Some(0),
        };
        let replace = serde_json::from_value(serde_json::json!({"documentChanges":[
            {"kind":"delete","uri":uri},
            {"kind":"create","uri":uri},
            {"textDocument":{"uri":uri,"version":null},"edits":[{
                "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},"newText":"recreated"
            }]}
        ]})).unwrap();
        apply_with_documents(&services, &mut store, replace, None, vec![context]).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "recreated");
        assert!(store.documents().snapshot(document).is_err());
        assert!(store.views().is_empty());
        assert_eq!(store.documents().snapshot(unrelated).unwrap().rope.to_string(), "unrelated");
        assert_eq!(std::fs::read_to_string(other).unwrap(), "unrelated");
        for project in &projects {
            let layouts = state.layouts.read();
            let layout = &layouts[project];
            assert!(!taide_native_app::tabs::tabs_in(&layout.root).into_iter()
                .any(|tab| matches!(&tab.kind, taide_model::layout::TabKind::File {path: file} if std::path::Path::new(file) == path)));
            assert_eq!(layout.closed_tabs.len(), 1);
        }
        tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn 실제_workspace_worker는_미열림_파일과_비활성_버퍼를_구분하고_uri_root_lossy_mirror를_보호한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir().join(format!("taide-native-workspace-{}", ProjectId::new())),
            );
            let root = directory.0.join("root");
            std::fs::create_dir_all(&root).unwrap();
            let root = std::fs::canonicalize(root).unwrap();
            let project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            state.projects.write().insert(
                project.clone(),
                Project {
                    id: project.clone(),
                    root: root.to_str().unwrap().into(),
                    name: "synthetic workspace".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let path = root.join("字 😀.txt");
            std::fs::write(&path, "a😀한\r\nend").unwrap();
            let uri: lsp_types::Uri =
                taide_lsp::service::workspace_folder_uri(path.to_str().unwrap())
                    .parse()
                    .unwrap();
            assert_eq!(uri_path(&uri).unwrap(), path);
            apply(
                &services,
                &mut store,
                edit(&path, 1, 3, "末"),
                Some(vec![root.to_str().unwrap().into()]),
            )
            .await
            .unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "a末한\r\nend");
            assert!(store.documents().is_empty());
            let background = root.join("background.txt");
            std::fs::write(&background, "background disk").unwrap();
            let file = taide_file::service::open_file(&background, &[], false).unwrap();
            let document = store.open_file(background.clone(), file).unwrap();
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
            replace_selections(&mut store, view, "unsaved:", None).unwrap();
            apply(
                &services,
                &mut store,
                edit(&background, 0, 0, "server:"),
                None,
            )
            .await
            .unwrap();
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                "server:unsaved:background disk"
            );
            assert!(store.documents().snapshot(document).unwrap().dirty);
            assert_eq!(
                std::fs::read_to_string(&background).unwrap(),
                "background disk"
            );
            let lossy = root.join("lossy.txt");
            std::fs::write(&lossy, [0xff, b'x']).unwrap();
            assert!(
                apply(&services, &mut store, edit(&lossy, 0, 0, "bad:"), None)
                    .await
                    .is_err()
            );
            assert_eq!(std::fs::read(&lossy).unwrap(), [0xff, b'x']);
            let mirrored = root.join("mirrored.txt");
            std::fs::write(&mirrored, "disk").unwrap();
            taide_file::service::mirror_dirty(
                &state.paths,
                &project,
                &mirrored,
                mirrored.to_str().unwrap(),
                "restorable draft",
            )
            .unwrap();
            assert!(
                apply(
                    &services,
                    &mut store,
                    edit(&mirrored, 0, 0, "overwrite:"),
                    None
                )
                .await
                .is_err()
            );
            assert_eq!(std::fs::read_to_string(&mirrored).unwrap(), "disk");
            assert!(taide_file::service::has_mirror(&state.paths, &project, &mirrored).unwrap());
            assert_eq!(
                taide_file::service::list_mirrors(&state.paths, &project).unwrap()[0].content,
                "restorable draft"
            );
            let restricted = root.join("scope");
            std::fs::create_dir_all(&restricted).unwrap();
            assert!(
                apply(
                    &services,
                    &mut store,
                    edit(&path, 0, 0, "outside:"),
                    Some(vec![restricted.to_str().unwrap().into()])
                )
                .await
                .is_err()
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "a末한\r\nend");
            for value in [
                "https://example.com/file",
                "file://remote/synthetic.txt",
                "file:///synthetic.txt?query=1",
                "file:///synthetic.txt#fragment",
                "file:///bad%FF.txt",
                "file:///bad%00.txt",
                "file:relative.txt",
            ] {
                assert!(
                    uri_path(&value.parse().unwrap()).is_err(),
                    "URI should be rejected: {value}"
                );
            }
            tokio::time::timeout(TIMEOUT, tasks.shutdown())
                .await
                .unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}
