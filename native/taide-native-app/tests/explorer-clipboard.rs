use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::error::AppError;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::layout::TabKind;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::tree::TreeEntryKind;
use taide_native_app::bootstrap::services;
use taide_native_app::explorer_clipboard::{
    Entry, Mode, Reply as PasteReply, Request, unique_name,
};
use taide_native_app::lsp::{LspBridge, Reply};
use taide_native_editor::document::DocumentKey;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{AppState, EventSink, TaskSupervisor, layout_actions};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(5);
const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 1024;
const ATTEMPT_LIMIT: usize = 8;

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct RescanSink(AppState);
impl EventSink for RescanSink {
    fn publish(&self, event: AppEvent) {
        if matches!(event, AppEvent::FsRescanRequired { .. }) {
            assert!(self.0.projects.try_write().is_some());
        }
    }
}

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn request(from: &Path, target: &Path, mode: Mode, kind: TreeEntryKind) -> Request {
    Request {
        owner: None,
        token: 1,
        entry: Entry {
            mode,
            path: from.to_str().unwrap().into(),
            kind,
        },
        target: target.to_str().unwrap().into(),
        sibling_names: BTreeSet::new(),
        conflict_suffix: "copy".into(),
    }
}

fn file_tabs(layout: &taide_model::layout::ProjectLayout) -> Vec<&taide_model::layout::Tab> {
    taide_native_app::tabs::tabs_in(&layout.root)
        .into_iter()
        .filter(|tab| matches!(tab.kind, TabKind::File { .. }))
        .collect()
}

async fn finished(bridge: &mut LspBridge, signal: &Notify, store: &mut EditorStore) -> PasteReply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            let Some(reply) = bridge.poll() else {
                signal.notified().await;
                continue;
            };
            match reply {
                Reply::ExplorerMovePrepare(event) => {
                    let documents = taide_native_app::explorer_source_missing::prepare_documents(
                        store,
                        &event.path,
                        &Default::default(),
                    );
                    let _result = event.completion.send(Ok(documents));
                }
                Reply::ExplorerMoved(event) => {
                    for draft in event.drafts.values() {
                        assert!(draft.mirror.source_missing);
                        assert_eq!(draft.mirror.content, "latest:disk body");
                    }
                    let result = taide_native_app::explorer_source_missing::commit_documents(
                        store,
                        &event,
                        &Default::default(),
                    );
                    let _result = event.completion.send(result);
                }
                Reply::RenamePrepare(event) => {
                    let result = taide_native_app::workspace_rename::prepare_documents(
                        store,
                        &event.from,
                        &HashMap::new(),
                    );
                    let _result = event.completion.send(result);
                }
                Reply::Renamed(event) => {
                    assert!(event.warnings.is_empty());
                    let result =
                        taide_native_app::workspace_rename::commit_documents(store, &event);
                    let _result = event.completion.send(result);
                }
                Reply::ExplorerPasted(event) => return event,
                _ => panic!("unexpected paste reply"),
            }
        }
    })
    .await
    .unwrap()
}

#[test]
fn 복사이름은_파일확장자_dotfile_폴더와_숫자충돌을_원본대로_보존한다() {
    for (desired, kind, taken, expected) in [
        ("文.tsx", TreeEntryKind::File, vec!["文.tsx"], "文 copy.tsx"),
        ("v1.2", TreeEntryKind::Directory, vec!["v1.2"], "v1.2 copy"),
        (
            ".gitignore",
            TreeEntryKind::File,
            vec![".gitignore"],
            ".gitignore copy",
        ),
        (
            "name.tar.gz",
            TreeEntryKind::File,
            vec!["name.tar.gz", "name.tar copy.gz", "name.tar copy 2.gz"],
            "name.tar copy 3.gz",
        ),
        ("source.txt", TreeEntryKind::File, Vec::new(), "source.txt"),
    ] {
        let taken = taken.into_iter().map(String::from).collect();
        assert_eq!(unique_name(desired, &taken, "copy", kind), expected);
    }
}

#[test]
fn 실제_프로젝트전환_copy는_열린source만_허용하고_회신owner를_보존한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir()
                    .join(format!("taide-native-paste-projects-{}", ProjectId::new())),
            );
            let source_root = directory.0.join("source");
            let target_root = directory.0.join("target");
            std::fs::create_dir_all(&source_root).unwrap();
            std::fs::create_dir_all(&target_root).unwrap();
            let source_root = std::fs::canonicalize(source_root).unwrap();
            let target_root = std::fs::canonicalize(target_root).unwrap();
            let source = source_root.join("跨项目.txt");
            std::fs::write(&source, "original cross-project body").unwrap();
            let source_project = ProjectId::new();
            let target_project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            for (project, root) in [
                (&source_project, &source_root),
                (&target_project, &target_root),
            ] {
                state.projects.write().insert(
                    project.clone(),
                    Project {
                        id: project.clone(),
                        root: root.to_str().unwrap().into(),
                        name: "synthetic clipboard project".into(),
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
            }
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let signal = Arc::new(Notify::new());
            let repaint = signal.clone();
            let mut bridge = LspBridge::connect(
                services.clone(),
                Default::default(),
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            let mut owners = taide_native_app::explorer_clipboard_owners::ClipboardOwners::new(
                eframe::egui::ViewportId::ROOT,
            );
            let owner = owners.mount(&taide_model::ids::ShellSlotId::new()).unwrap();
            let mut copy = request(&source, &target_root, Mode::Copy, TreeEntryKind::File);
            copy.owner = Some(owner);
            bridge
                .paste_entry(target_project.clone(), copy.clone())
                .unwrap();
            let event = finished(&mut bridge, &signal, &mut store).await;
            assert_eq!(event.request, copy);
            assert!(!event.clear_cut);
            let pasted = event
                .result
                .expect("another open project's source must remain authorized");
            assert_eq!(
                pasted.path,
                target_root.join("跨项目.txt").to_str().unwrap()
            );
            assert_eq!(
                std::fs::read_to_string(&pasted.path).unwrap(),
                "original cross-project body"
            );
            assert_eq!(
                std::fs::read_to_string(&source).unwrap(),
                "original cross-project body"
            );
            assert!(pasted.page.rows.iter().any(|row| row.path == pasted.path));
            assert!(file_tabs(&state.layouts.read()[&source_project]).is_empty());
            assert!(file_tabs(&state.layouts.read()[&target_project]).is_empty());
            state.projects.write().remove(&source_project);
            bridge
                .paste_entry(target_project.clone(), copy.clone())
                .unwrap();
            let event = finished(&mut bridge, &signal, &mut store).await;
            assert!(event.result.is_err());
            assert!(!event.clear_cut);
            assert_eq!(std::fs::read_dir(&target_root).unwrap().count(), 1);
            assert_eq!(
                std::fs::read_to_string(&source).unwrap(),
                "original cross-project body"
            );
            for failed in [
                request(
                    &target_root.join("跨项目.txt"),
                    &directory.0.join("outside"),
                    Mode::Copy,
                    TreeEntryKind::File,
                ),
                request(
                    Path::new("relative.txt"),
                    &target_root,
                    Mode::Copy,
                    TreeEntryKind::File,
                ),
                request(
                    &target_root.join("跨项目.txt"),
                    Path::new("relative"),
                    Mode::Copy,
                    TreeEntryKind::File,
                ),
            ] {
                bridge.paste_entry(target_project.clone(), failed).unwrap();
                assert!(
                    finished(&mut bridge, &signal, &mut store)
                        .await
                        .result
                        .is_err()
                );
            }
            assert!(!directory.0.join("outside").exists());
            tokio::time::timeout(TIMEOUT, bridge.disconnect())
                .await
                .unwrap()
                .unwrap();
            tokio::time::timeout(TIMEOUT, tasks.shutdown())
                .await
                .unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}

#[test]
fn 실제붙여넣기는_숨은충돌_8회제한_dirty이동_미러와_worker회수를_보존한다() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let directory = Directory(std::env::temp_dir().join(format!("taide-native-paste-{}", ProjectId::new())));
        let root = directory.0.join("root");
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::create_dir_all(root.join("full")).unwrap();
        std::fs::create_dir_all(root.join("v1.2")).unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let source = root.join("source 文.txt");
        std::fs::write(&source, "disk body").unwrap();
        std::fs::write(root.join("v1.2/child.txt"), "directory body").unwrap();
        std::fs::write(root.join("target/source 文.txt"), "collision body").unwrap();
        let mut taken = BTreeSet::new();
        for _ in 0..ATTEMPT_LIMIT {
            let name = unique_name("source 文.txt", &taken, "copy", TreeEntryKind::File);
            std::fs::write(root.join("full").join(&name), "untouched").unwrap();
            taken.insert(name);
        }
        let project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        state.projects.write().insert(project.clone(), Project {
            id: project.clone(), root: root.to_str().unwrap().into(), name: "synthetic paste".into(),
            capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default(),
        });
        state.layouts.write().insert(project.clone(), taide_layout::service::default_layout());
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT, max_views: VIEW_LIMIT, max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        }).unwrap();
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(services.clone(), Default::default(), Arc::new(move || repaint.notify_one())).unwrap();
        let copy = request(&source, &root.join("target"), Mode::Copy, TreeEntryKind::File);
        bridge.paste_entry(project.clone(), copy.clone()).unwrap();
        let event = finished(&mut bridge, &signal, &mut store).await;
        assert_eq!(event.request, copy);
        assert!(!event.clear_cut);
        let pasted = event.result.unwrap();
        assert_eq!(pasted.path, root.join("target/source 文 copy.txt").to_str().unwrap());
        assert_eq!(std::fs::read_to_string(&pasted.path).unwrap(), "disk body");
        assert_eq!(std::fs::read_to_string(root.join("target/source 文.txt")).unwrap(), "collision body");
        assert!(pasted.page.rows.iter().any(|row| row.path == pasted.path));
        assert!(file_tabs(&state.layouts.read()[&project]).is_empty());
        let mut directory_copy = request(&root.join("v1.2"), &root, Mode::Copy, TreeEntryKind::Directory);
        directory_copy.sibling_names.insert("v1.2".into());
        bridge.paste_entry(project.clone(), directory_copy).unwrap();
        let event = finished(&mut bridge, &signal, &mut store).await;
        assert_eq!(event.result.unwrap().path, root.join("v1.2 copy").to_str().unwrap());
        assert_eq!(std::fs::read_to_string(root.join("v1.2 copy/child.txt")).unwrap(), "directory body");
        bridge.paste_entry(project.clone(), request(&source, &root.join("full"), Mode::Copy, TreeEntryKind::File)).unwrap();
        let event = finished(&mut bridge, &signal, &mut store).await;
        assert!(matches!(event.result, Err(AppError::Localized(error)) if error.key == "error.file.destinationExists"));
        assert_eq!(std::fs::read_dir(root.join("full")).unwrap().count(), ATTEMPT_LIMIT);
        for name in taken {
            assert_eq!(std::fs::read_to_string(root.join("full").join(name)).unwrap(), "untouched");
        }
        for failed in [
            request(&root.join("missing.txt"), &root.join("target"), Mode::Copy, TreeEntryKind::File),
            request(&source, &directory.0.join("outside"), Mode::Copy, TreeEntryKind::File),
            request(Path::new("relative.txt"), &root, Mode::Copy, TreeEntryKind::File),
        ] {
            bridge.paste_entry(project.clone(), failed).unwrap();
            let event = finished(&mut bridge, &signal, &mut store).await;
            assert!(event.result.is_err());
            assert!(!event.clear_cut);
        }
        assert!(!directory.0.join("outside").exists());
        let layout = layout_actions::layout_open_tab(services.events.as_ref(), &state, project.clone(),
            TabKind::File { path: source.to_str().unwrap().into() }, "source 文.txt".into(), None, false).await.unwrap();
        let tab = file_tabs(&layout)[0].clone();
        let document = store.open_file(source.clone(), taide_file::service::open_file(&source, &[], false).unwrap()).unwrap();
        let view = store.attach_view(ViewKey { window: "main".into(), pane: PaneId::new(), tab: TabId::new() }, document).unwrap();
        replace_selections(&mut store, view, "unsaved:", None).unwrap();
        layout_actions::layout_set_dirty(services.events.as_ref(), &state, tab.id, true).await.unwrap();
        let cut = request(&source, &root.join("target"), Mode::Cut, TreeEntryKind::File);
        bridge.paste_entry(project.clone(), cut.clone()).unwrap();
        let event = finished(&mut bridge, &signal, &mut store).await;
        assert_eq!(event.request, cut);
        assert!(event.clear_cut);
        let pasted = event.result.unwrap();
        assert_eq!(pasted.path, root.join("target/source 文 copy 2.txt").to_str().unwrap());
        assert!(!source.exists());
        assert_eq!(std::fs::read_to_string(&pasted.path).unwrap(), "disk body");
        let snapshot = store.documents().snapshot(document).unwrap();
        assert_eq!(snapshot.key, DocumentKey::File(pasted.path.clone().into()));
        assert_eq!(snapshot.rope.to_string(), "unsaved:disk body");
        assert!(snapshot.dirty);
        let mirrors = taide_file::service::list_mirrors(&state.paths, &project).unwrap();
        assert!(mirrors.iter().any(|mirror| mirror.path == pasted.path && mirror.content == "unsaved:disk body"));
        assert_eq!(file_tabs(&state.layouts.read()[&project]).len(), 1);
        assert!(pasted.page.rows.iter().any(|row| row.path == pasted.path));
        tokio::time::timeout(TIMEOUT, bridge.disconnect()).await.unwrap().unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn 실제_프로젝트간_cut은_파일만_이동하고_기존탭의_최신초안을_보존한다() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let directory = Directory(std::env::temp_dir().join(format!("taide-native-cut-projects-{}", ProjectId::new())));
        let source_root = directory.0.join("source");
        let target_root = directory.0.join("target");
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::create_dir_all(&target_root).unwrap();
        let source_root = std::fs::canonicalize(source_root).unwrap();
        let target_root = std::fs::canonicalize(target_root).unwrap();
        let source = source_root.join("move.txt");
        let destination = target_root.join("move.txt");
        std::fs::write(&source, "disk body").unwrap();
        let source_project = ProjectId::new();
        let target_project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        for (project, root) in [(&source_project, &source_root), (&target_project, &target_root)] {
            state.projects.write().insert(project.clone(), Project {
                id: project.clone(), root: root.to_str().unwrap().into(), name: "synthetic cut project".into(),
                capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default(),
            });
            state.layouts.write().insert(project.clone(), taide_layout::service::default_layout());
        }
        let invalid_project = ProjectId::new();
        state.projects.write().insert(invalid_project.clone(), Project {
            id: invalid_project, root: String::new(), name: "synthetic invalid unrelated root".into(),
            capabilities: Vec::new(), root_missing: true, last_opened_at: 0.0, display: Default::default(),
        });
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state.clone(), tasks.clone(), Arc::new(RescanSink(state.clone())));
        let mut tabs = Vec::new();
        for project in [&source_project, &target_project] {
            let layout = layout_actions::layout_open_tab(services.events.as_ref(), &state, project.clone(),
                TabKind::File { path: source.to_str().unwrap().into() }, "move.txt".into(), None, false).await.unwrap();
            tabs.push(file_tabs(&layout)[0].clone());
        }
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT, max_views: VIEW_LIMIT, max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        }).unwrap();
        let document = store.open_file(source.clone(), taide_file::service::open_file(&source, &[], false).unwrap()).unwrap();
        let mut keys = Vec::new();
        for tab in &tabs {
            let key = ViewKey { window: "main".into(), pane: PaneId::new(), tab: tab.id.clone() };
            store.attach_view(key.clone(), document).unwrap();
            keys.push(key);
            layout_actions::layout_set_dirty(services.events.as_ref(), &state, tab.id.clone(), true).await.unwrap();
        }
        let source_view = store.views().find(&keys[0]).unwrap();
        replace_selections(&mut store, source_view, "latest:", None).unwrap();
        taide_file::service::mirror_dirty(&state.paths, &source_project, &source, source.to_str().unwrap(), "stale draft").unwrap();
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(services.clone(), Default::default(), Arc::new(move || repaint.notify_one())).unwrap();
        bridge.paste_entry(target_project.clone(), request(&source, &target_root, Mode::Cut, TreeEntryKind::File)).unwrap();
        let event = finished(&mut bridge, &signal, &mut store).await;
        let pasted = event.result.expect("GUI Cut must support moving between open project roots");
        assert!(event.clear_cut);
        assert_eq!(pasted.path, destination.to_str().unwrap());
        assert!(!source.exists());
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), "disk body");
        let source_layout = state.layouts.read()[&source_project].clone();
        let target_layout = state.layouts.read()[&target_project].clone();
        assert!(matches!(&file_tabs(&source_layout)[0].kind, TabKind::File { path } if path == source.to_str().unwrap()));
        assert!(matches!(&file_tabs(&target_layout)[0].kind, TabKind::File { path } if path == source.to_str().unwrap()));
        assert_eq!(file_tabs(&source_layout)[0].id, tabs[0].id);
        assert_eq!(file_tabs(&target_layout)[0].id, tabs[1].id);
        assert!(store.documents().snapshot(document).is_err());
        assert!(store.views().find(&keys[0]).is_none());
        assert!(store.views().find(&keys[1]).is_none());
        let source_mirrors = taide_file::service::list_mirrors(&state.paths, &source_project).unwrap();
        let mirror = source_mirrors.iter().find(|mirror| mirror.path == source.to_str().unwrap()).unwrap();
        assert_eq!(mirror.content, "latest:disk body");
        assert!(mirror.source_missing);
        let target_mirrors = taide_file::service::list_mirrors(&state.paths, &target_project).unwrap();
        assert!(target_mirrors.is_empty());
        assert!(pasted.page.rows.iter().any(|row| row.path == pasted.path));
        tokio::time::timeout(TIMEOUT, bridge.disconnect()).await.unwrap().unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn 중첩root_cut은_선택탭만_옮기고_내부project_초안을_보존한다() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let directory = Directory(std::env::temp_dir().join(format!("taide-native-cut-nested-{}", ProjectId::new())));
        std::fs::create_dir_all(directory.0.join("root/nested")).unwrap();
        let target_root = std::fs::canonicalize(directory.0.join("root")).unwrap();
        let source_root = target_root.join("nested");
        let source = source_root.join("move.txt");
        let destination = target_root.join("move.txt");
        std::fs::write(&source, "disk body").unwrap();
        let source_project = ProjectId::new();
        let target_project = ProjectId::new();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        for (project, root) in [(&source_project, &source_root), (&target_project, &target_root)] {
            state.projects.write().insert(project.clone(), Project {
                id: project.clone(), root: root.to_str().unwrap().into(), name: "synthetic nested cut".into(),
                capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default(),
            });
            state.layouts.write().insert(project.clone(), taide_layout::service::default_layout());
        }
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state.clone(), tasks.clone(), Arc::new(RescanSink(state.clone())));
        let mut tabs = Vec::new();
        for project in [&source_project, &target_project] {
            let layout = layout_actions::layout_open_tab(services.events.as_ref(), &state, project.clone(),
                TabKind::File { path: source.to_str().unwrap().into() }, "move.txt".into(), None, false).await.unwrap();
            tabs.push(file_tabs(&layout)[0].clone());
        }
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT, max_views: VIEW_LIMIT, max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        }).unwrap();
        let document = store.open_file(source.clone(), taide_file::service::open_file(&source, &[], false).unwrap()).unwrap();
        let mut views = Vec::new();
        for tab in &tabs {
            views.push(store.attach_view(ViewKey { window: "main".into(), pane: PaneId::new(), tab: tab.id.clone() }, document).unwrap());
            layout_actions::layout_set_dirty(services.events.as_ref(), &state, tab.id.clone(), true).await.unwrap();
        }
        replace_selections(&mut store, views[0], "latest:", None).unwrap();
        let lsp_rename = taide_lsp::native::protocol::lsp_types::RenameFile {
            old_uri: taide_lsp::service::workspace_folder_uri(source.to_str().unwrap()).parse().unwrap(),
            new_uri: taide_lsp::service::workspace_folder_uri(destination.to_str().unwrap()).parse().unwrap(),
            options: None, annotation_id: None,
        };
        let (sender, _receive) = tokio::sync::mpsc::channel(1);
        let no_repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
        assert!(taide_native_app::workspace_rename::apply(&services, lsp_rename, None, &sender, &no_repaint).await.is_err());
        assert!(source.exists() && !destination.exists());
        let signal = Arc::new(Notify::new());
        let repaint = signal.clone();
        let mut bridge = LspBridge::connect(services.clone(), Default::default(), Arc::new(move || repaint.notify_one())).unwrap();
        bridge.paste_entry(target_project.clone(), request(&source, &target_root, Mode::Cut, TreeEntryKind::File)).unwrap();
        let event = finished(&mut bridge, &signal, &mut store).await;
        let pasted = event.result.expect("GUI move must not inherit LSP root-transfer rejection");
        assert!(event.clear_cut);
        assert_eq!(pasted.path, destination.to_str().unwrap());
        assert!(!source.exists());
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), "disk body");
        assert!(matches!(&file_tabs(&state.layouts.read()[&source_project])[0].kind, TabKind::File { path } if path == source.to_str().unwrap()));
        assert!(matches!(&file_tabs(&state.layouts.read()[&target_project])[0].kind, TabKind::File { path } if path == destination.to_str().unwrap()));
        assert_eq!(file_tabs(&state.layouts.read()[&source_project])[0].id, tabs[0].id);
        assert_eq!(file_tabs(&state.layouts.read()[&target_project])[0].id, tabs[1].id);
        let snapshot = store.documents().snapshot(document).unwrap();
        assert_eq!(snapshot.key, DocumentKey::File(destination.clone()));
        assert_eq!(snapshot.rope.to_string(), "latest:disk body");
        assert!(snapshot.dirty);
        assert!(store.views().get(views[0]).is_none());
        assert!(store.views().get(views[1]).is_some());
        for (project, path, missing) in [(&source_project, &source, true), (&target_project, &destination, false)] {
            let mirrors = taide_file::service::list_mirrors(&state.paths, project).unwrap();
            let mirror = mirrors.iter().find(|mirror| mirror.path == path.to_str().unwrap()).unwrap();
            assert_eq!(mirror.content, "latest:disk body");
            assert_eq!(mirror.source_missing, missing);
        }
        tokio::time::timeout(TIMEOUT, bridge.disconnect()).await.unwrap().unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}
