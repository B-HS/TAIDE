use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui::{
    self, Context, Event, Key, Modifiers, PointerButton, RawInput, Rect, pos2, vec2,
};
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
use taide_model::layout::{Tab, TabKind, TabPathChange};
use taide_model::locale::ResolvedLocale;
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_model::tree::{TreeEntryKind, TreeRow, TreeRowPage};
use taide_native_app::bootstrap::services;
use taide_native_app::delete_dialog::{Confirmation, Output};
use taide_native_app::explorer::{Action, Explorer};
use taide_native_app::explorer_delete::{self, Request};
use taide_native_app::lsp::{LspBridge, Reply};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;
const TIMEOUT: Duration = Duration::from_secs(5);
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;

fn locale() -> ResolvedLocale {
    ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: [
            ("common.cancel", "Cancel"),
            ("explorer.delete", "Delete"),
            ("explorer.deleteConfirmTitle", "Delete {{name}}?"),
            (
                "explorer.deleteConfirmDescription",
                "Move {{name}} to Trash",
            ),
        ]
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect(),
    }
}

fn input(events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(WIDTH, HEIGHT))),
        events,
        ..Default::default()
    }
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn pointer(position: egui::Pos2, button: PointerButton, pressed: bool) -> Vec<Event> {
    vec![
        Event::PointerMoved(position),
        Event::PointerButton {
            pos: position,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        },
    ]
}

fn dialog(context: &Context, confirmation: &mut Confirmation, events: Vec<Event>) -> Output {
    let mut response = None;
    let mut output = context.run_ui(input(events), |_ui| {
        response = Some(confirmation.show(context, &locale()));
    });
    output.textures_delta.clear();
    response.unwrap()
}

#[test]
fn 탐색기_삭제확인은_context와_정확한_단축키_취소초점_backdrop_escape_확정을_보존한다() {
    let context = Context::default();
    let project = ProjectId::new();
    let row = TreeRow {
        path: "/synthetic/delete.txt".into(),
        name: "delete.txt".into(),
        kind: TreeEntryKind::File,
        depth: 0,
        expanded: false,
        has_children: false,
    };
    let page = TreeRowPage {
        total: 1,
        rows: vec![row.clone()],
    };
    let mut explorer = Explorer::default();
    explorer.selected = Some(row.path.clone());
    let mut tree_frame = |events| {
        let mut response = None;
        let mut output = context.run_ui(input(events), |ui| {
            response = Some(explorer.show(ui, &project, &page, &locale()));
        });
        output.textures_delta.clear();
        response.unwrap()
    };
    let first = tree_frame(Vec::new());
    let tree_id = egui::Id::new(("native-tree-focus", &project));
    context.memory_mut(|memory| memory.request_focus(tree_id));
    assert!(
        tree_frame(vec![key(
            Key::Backspace,
            Modifiers::COMMAND | Modifiers::ALT
        )])
        .actions
        .is_empty()
    );
    assert_eq!(
        tree_frame(vec![key(Key::Backspace, Modifiers::COMMAND)]).actions,
        vec![Action::RequestDelete(row.clone())]
    );
    let position = first.rows[&row.path].rect.center();
    tree_frame(pointer(position, PointerButton::Secondary, true));
    tree_frame(pointer(position, PointerButton::Secondary, false));
    let menu = tree_frame(Vec::new());
    let delete_position = menu.menu["explorer.delete"].rect.center();
    tree_frame(pointer(delete_position, PointerButton::Primary, true));
    assert_eq!(
        tree_frame(pointer(delete_position, PointerButton::Primary, false)).actions,
        vec![Action::RequestDelete(row.clone())]
    );
    let request = Request {
        project,
        path: row.path,
        name: row.name,
    };
    let mut confirmation = Confirmation::new(request.clone());
    let first = dialog(&context, &mut confirmation, Vec::new());
    assert!(context.memory(|memory| memory.has_focus(first.cancel.id)));
    let outside = pos2(1.0, 1.0);
    assert_eq!(
        dialog(
            &context,
            &mut confirmation,
            pointer(outside, PointerButton::Primary, true)
        )
        .choice,
        None
    );
    assert_eq!(
        dialog(
            &context,
            &mut confirmation,
            pointer(outside, PointerButton::Primary, false)
        )
        .choice,
        None
    );
    assert_eq!(
        dialog(
            &context,
            &mut confirmation,
            vec![key(Key::Escape, Modifiers::NONE)]
        )
        .choice,
        Some(false)
    );
    let mut confirmation = Confirmation::new(request);
    let frame = dialog(&context, &mut confirmation, Vec::new());
    let confirm = frame.confirm.rect.center();
    dialog(
        &context,
        &mut confirmation,
        pointer(confirm, PointerButton::Primary, true),
    );
    assert_eq!(
        dialog(
            &context,
            &mut confirmation,
            pointer(confirm, PointerButton::Primary, false)
        )
        .choice,
        Some(true)
    );
}

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Directory(PathBuf);
impl Drop for Directory {
    fn drop(&mut self) {
        let _result = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn 삭제_ack는_늦은_revision을_폐기하지않고_확정한_dirty_pinned_tab의_기록을_정리한다() {
    let directory = Directory(std::env::temp_dir().join(format!(
        "taide-native-explorer-delete-ack-{}",
        ProjectId::new()
    )));
    std::fs::create_dir_all(&directory.0).unwrap();
    let path = directory.0.join("source.txt");
    std::fs::write(&path, "disk").unwrap();
    let path = std::fs::canonicalize(path).unwrap();
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store
        .open_file(
            path.clone(),
            taide_file::service::open_file(&path, &[], false).unwrap(),
        )
        .unwrap();
    let mut layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    let tab = taide_model::ids::TabId::new();
    taide_layout::service::open_tab(
        &mut layout,
        &pane,
        Tab {
            id: tab.clone(),
            kind: TabKind::File {
                path: path.to_str().unwrap().into(),
            },
            title: "source.txt".into(),
            pinned: true,
            preview: false,
            dirty: true,
            view_state: None,
        },
        false,
    )
    .unwrap();
    let view = store
        .attach_view(
            ViewKey {
                window: "main".into(),
                pane,
                tab,
            },
            document,
        )
        .unwrap();
    replace_selections(&mut store, view, "before:", None).unwrap();
    let documents = explorer_delete::prepare_documents(&store, &path, &HashSet::new());
    taide_layout::service::apply_tab_path_change(
        &mut layout,
        &TabPathChange::Deleted {
            path: path.to_str().unwrap().into(),
        },
    );
    assert_eq!(layout.closed_tabs.len(), 1);
    assert!(!layout.closed_tabs[0].tab.dirty);
    replace_selections(&mut store, view, "late:", None).unwrap();
    let (completion, _receive) = tokio::sync::oneshot::channel();
    let mut event = explorer_delete::Deleted {
        path: path.clone(),
        documents,
        layouts: HashMap::from([(ProjectId::new(), layout)]),
        drafts: HashMap::new(),
        completion,
    };
    assert!(explorer_delete::commit_documents(&mut store, &event, &HashSet::new()).is_err());
    assert_eq!(store.views().len(), 1);
    assert_eq!(
        store
            .documents()
            .snapshot(document)
            .unwrap()
            .rope
            .to_string(),
        "before:late:disk"
    );
    event.documents = explorer_delete::prepare_documents(&store, &path, &HashSet::new());
    assert_eq!(
        explorer_delete::commit_documents(&mut store, &event, &HashSet::new()).unwrap(),
        vec![document]
    );
    assert_eq!(store.views().len(), 0);
    assert!(store.documents().snapshot(document).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "disk");
}

#[test]
fn 삭제후_공유초안_save_as는_더좁은_root가_있어도_해당_프로젝트의_미러를_복구한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(std::env::temp_dir().join(format!(
                "taide-native-explorer-recovery-{}",
                ProjectId::new()
            )));
            std::fs::create_dir_all(directory.0.join("root/nested")).unwrap();
            let root = std::fs::canonicalize(directory.0.join("root")).unwrap();
            let nested = root.join("nested");
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            let projects = [ProjectId::new(), ProjectId::new()];
            for (id, path) in projects.iter().zip([&root, &nested]) {
                state.projects.write().insert(
                    id.clone(),
                    Project {
                        id: id.clone(),
                        root: path.to_str().unwrap().into(),
                        name: "synthetic survivor recovery".into(),
                        capabilities: Vec::new(),
                        root_missing: false,
                        last_opened_at: 0.0,
                        display: Default::default(),
                    },
                );
                state
                    .layouts
                    .write()
                    .insert(id.clone(), taide_layout::service::default_layout());
            }
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
            let source = nested.join("missing.txt");
            std::fs::write(&source, "disk").unwrap();
            let layout = taide_runtime::layout_actions::layout_open_tab(
                services.events.as_ref(),
                &state,
                projects[0].clone(),
                TabKind::File {
                    path: source.to_str().unwrap().into(),
                },
                "missing.txt".into(),
                None,
                false,
            )
            .await
            .unwrap();
            let tab = taide_native_app::tabs::tabs_in(&layout.root)
                .into_iter()
                .find(
                    |tab| matches!(&tab.kind, TabKind::File { path } if Path::new(path) == source),
                )
                .unwrap()
                .id
                .clone();
            std::fs::remove_file(&source).unwrap();
            taide_file::service::mirror_dirty(
                &state.paths,
                &projects[0],
                &source,
                source.to_str().unwrap(),
                "latest survivor draft",
            )
            .unwrap();
            let mirror = taide_file::service::list_mirrors(&state.paths, &projects[0])
                .unwrap()
                .remove(0);
            let draft = taide_native_app::missing_draft::MissingDraft {
                canonical: source.clone(),
                project: projects[0].clone(),
                mirror,
            };
            let target = root.join("recovered.txt");
            let saved = taide_native_app::missing_draft::save(
                &services,
                tab,
                draft,
                target.to_str().unwrap().into(),
            )
            .await
            .unwrap();
            assert_eq!(saved.source, source);
            assert_eq!(
                std::fs::read_to_string(&target).unwrap(),
                "latest survivor draft"
            );
            assert!(!source.exists());
            assert!(!taide_file::service::has_mirror(&state.paths, &projects[0], &source).unwrap());
            assert_eq!(tasks.tracked_count(), 0);
        });
}

#[test]
fn 실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(async {
        let directory = Directory(std::env::temp_dir().join(format!("taide-native-explorer-delete-{}", ProjectId::new())));
        let nested = directory.0.join("root/nested");
        std::fs::create_dir_all(&nested).unwrap();
        let root = std::fs::canonicalize(directory.0.join("root")).unwrap();
        let nested = std::fs::canonicalize(nested).unwrap();
        let projects = [ProjectId::new(), ProjectId::new()];
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        for (id, root) in projects.iter().zip([&root, &nested]) {
            state.projects.write().insert(id.clone(), Project { id: id.clone(), root: root.to_str().unwrap().into(), name: "synthetic explorer delete".into(), capabilities: Vec::new(), root_missing: false, last_opened_at: 0.0, display: Default::default() });
            state.layouts.write().insert(id.clone(), taide_layout::service::default_layout());
        }
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let services = services(state.clone(), tasks.clone(), Arc::new(Sink));
        let path = nested.join("delete-synthetic.txt");
        let other = nested.join("delete-synthetic.txt-other");
        std::fs::write(&path, "disk").unwrap();
        std::fs::write(&other, "keep").unwrap();
        let mut store = EditorStore::new(EditorLimits { max_documents: DOCUMENT_LIMIT, max_views: VIEW_LIMIT, max_undo_groups: HISTORY_LIMIT, max_document_bytes: BYTE_LIMIT }).unwrap();
        let document = store.open_file(path.clone(), taide_file::service::open_file(&path, &[], false).unwrap()).unwrap();
        let unrelated = store.open_file(other.clone(), taide_file::service::open_file(&other, &[], false).unwrap()).unwrap();
        let mut tabs = Vec::new();
        let mut views = Vec::new();
        for project in &projects {
            taide_runtime::layout_actions::layout_open_tab(services.events.as_ref(), &state, project.clone(), TabKind::File { path: path.to_str().unwrap().into() }, "delete-synthetic.txt".into(), None, false).await.unwrap();
            let layout = state.layouts.read()[project].clone();
            let tab = taide_native_app::tabs::tabs_in(&layout.root).into_iter().find(|tab| matches!(&tab.kind, TabKind::File { path: file } if Path::new(file) == path)).unwrap().id.clone();
            views.push(store.attach_view(ViewKey { window: project.to_string(), pane: layout.focused_pane, tab: tab.clone() }, document).unwrap());
            tabs.push(tab);
        }
        replace_selections(&mut store, views[0], "latest:", None).unwrap();
        taide_file::service::mirror_dirty(&state.paths, &projects[1], &path, path.to_str().unwrap(), "stale mirror").unwrap();
        let request = Request { project: projects[0].clone(), path: path.to_str().unwrap().into(), name: "delete-synthetic.txt".into() };
        let ready = Arc::new(tokio::sync::Notify::new());
        let notification = ready.clone();
        let mut bridge = LspBridge::connect(services.clone(), Default::default(), Arc::new(move || notification.notify_one())).unwrap();
        bridge.delete_entry(request.clone()).unwrap();
        let mut prepared = false;
        let mut committed = false;
        let mut activity = None;
        tokio::time::timeout(TIMEOUT, async {
            loop {
                while let Some(reply) = bridge.poll() {
                    match reply {
                        Reply::ExplorerDeletePrepare(event) => {
                            prepared = true;
                            activity = Some(event.activity.clone());
                            let documents = explorer_delete::prepare_documents(&store, &event.path, &HashSet::new());
                            assert!(documents.iter().any(|snapshot| snapshot.dirty));
                            assert!(taide_native_app::workspace_delete::prepare_documents(&store, &event.path, &HashSet::new()).is_err());
                            let _result = event.completion.send(Ok(documents));
                        }
                        Reply::ExplorerDeleted(event) => {
                            assert!(tokio::time::timeout(Duration::ZERO, state.begin_owned_mutation()).await.is_err());
                            assert!(!path.exists());
                            assert!(event.layouts[&projects[0]].closed_tabs.iter().all(|closed| !closed.tab.dirty));
                            assert!(!taide_native_app::tabs::tabs_in(&event.layouts[&projects[0]].root).iter().any(|tab| tab.id == tabs[0]));
                            assert!(taide_native_app::tabs::tabs_in(&event.layouts[&projects[1]].root).iter().any(|tab| tab.id == tabs[1]));
                            let draft = &event.drafts[path.to_str().unwrap()];
                            assert_eq!(draft.project, projects[1]);
                            assert!(draft.mirror.source_missing);
                            assert_eq!(draft.mirror.content, "latest:disk");
                            let mirrors = taide_file::service::list_mirrors(&state.paths, &projects[1]).unwrap();
                            assert_eq!(mirrors[0].content, "latest:disk");
                            assert!(mirrors[0].source_missing);
                            let result = explorer_delete::commit_documents(&mut store, &event, &HashSet::new()).unwrap();
                            assert_eq!(result, vec![document]);
                            assert!(store.documents().snapshot(document).is_err());
                            assert!(store.documents().snapshot(unrelated).is_ok());
                            assert_eq!(store.views().len(), 0);
                            committed = true;
                            let _result = event.completion.send(Ok(result));
                        }
                        Reply::ExplorerDeleteFinished { request: finished, result } => {
                            assert_eq!(finished, request);
                            assert_eq!(result.unwrap(), vec![document]);
                            return;
                        }
                        _ => panic!("unexpected explorer delete reply"),
                    }
                }
                ready.notified().await;
            }
        }).await.unwrap();
        assert!(prepared && committed);
        assert!(!activity.unwrap().is_active());
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "keep");
        let rejected = Request { project: projects[0].clone(), path: root.to_str().unwrap().into(), name: "root".into() };
        let (sender, _receiver) = tokio::sync::mpsc::channel(1);
        let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
        assert!(explorer_delete::apply(&services, rejected, &sender, &repaint).await.is_err());
        let outside = directory.0.join("outside.txt");
        std::fs::write(&outside, "outside").unwrap();
        state.authorize_cli_opened_path(&outside);
        assert!(explorer_delete::apply(&services, Request { project: projects[0].clone(), path: outside.to_str().unwrap().into(), name: "outside.txt".into() }, &sender, &repaint).await.is_err());
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "outside");
        bridge.disconnect().await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}
