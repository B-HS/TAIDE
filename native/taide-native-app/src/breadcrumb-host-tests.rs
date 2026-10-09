use super::*;
use eframe::egui::ViewportId;
use std::{future::Future, path::PathBuf, sync::Arc, task::Poll, time::Duration};
use taide_model::{
    app_event::AppEvent,
    error::AppErrorKind,
    file::{FileSizeTier, OpenedFile},
    ids::{PaneId, ProjectId, TabId},
    layout::{AuxWindowLayout, PaneNode, SplitDir},
    paths::AppPaths,
    project::Project,
};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const DEADLINE: Duration = Duration::from_secs(5);
const MAX_BYTES: usize = 4096;
const GENERATION: u64 = 7;
const AUX_SLOT: u32 = 4;

struct Events;

impl EventSink for Events {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    source: Source,
    target: String,
    target_tab: TabId,
    other_pane: PaneId,
    other_project: ProjectId,
}

impl Fixture {
    fn new(auxiliary: bool) -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-breadcrumb-host-{}", ProjectId::new()));
        let root = directory.join("project");
        let nested = root.join("src");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(nested.join("nested")).unwrap();
        let current = nested.join("current.rs");
        let other = nested.join("other.rs");
        std::fs::write(&current, "current").unwrap();
        std::fs::write(&other, "other\nline\n").unwrap();
        let path = current.canonicalize().unwrap().to_str().unwrap().to_owned();
        let target = other.canonicalize().unwrap().to_str().unwrap().to_owned();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: MAX_BYTES,
        })
        .unwrap();
        let document = store
            .open_file(
                path.clone().into(),
                OpenedFile {
                    path: path.clone(),
                    content: "current".into(),
                    byte_size: "current".len().try_into().unwrap(),
                    line_count: 1,
                    language_id: "rust".into(),
                    tier: FileSizeTier::Normal,
                    read_only: true,
                    encoding_lossy: false,
                    modified_ms: 0.0,
                    editor_config: Default::default(),
                },
            )
            .unwrap();
        let snapshot = store.documents().snapshot(document).unwrap();
        let source = Source {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
            path: path.clone(),
            document,
            revision: snapshot.revision,
            language: "rust".into(),
            generation: GENERATION,
        };
        let tab = |id, path, preview| Tab {
            id,
            kind: TabKind::File { path },
            title: "synthetic".into(),
            pinned: false,
            preview,
            dirty: false,
            view_state: Some("saved cursor beyond first line".into()),
        };
        let target_tab = TabId::new();
        let source_root = PaneNode::Leaf {
            id: source.pane.clone(),
            tabs: vec![
                tab(source.tab.clone(), path.clone(), false),
                tab(target_tab.clone(), target.clone(), false),
            ],
            active: Some(source.tab.clone()),
        };
        let other_pane = PaneId::new();
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Leaf {
            id: other_pane.clone(),
            tabs: vec![tab(TabId::new(), path.clone(), false)],
            active: None,
        };
        layout.focused_pane = other_pane.clone();
        if auxiliary {
            layout.auxiliary_windows.push(AuxWindowLayout {
                slot: AUX_SLOT,
                root: source_root,
                focused_pane: source.pane.clone(),
            });
        } else {
            layout.root = PaneNode::Split {
                id: PaneId::new(),
                dir: SplitDir::Horizontal,
                children: vec![source_root, layout.root],
                sizes: vec![0.5, 0.5],
            };
        }
        let other_project = ProjectId::new();
        let mut shared = taide_layout::service::default_layout();
        shared.root = PaneNode::Leaf {
            id: shared.focused_pane.clone(),
            tabs: vec![tab(TabId::new(), path.clone(), false)],
            active: None,
        };
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.settings.write().enable_preview_tabs = true;
        state.projects.write().insert(
            source.project.clone(),
            Project {
                id: source.project.clone(),
                root: root.canonicalize().unwrap().to_str().unwrap().into(),
                name: "synthetic".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        state.layouts.write().insert(source.project.clone(), layout);
        state.layouts.write().insert(other_project.clone(), shared);
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Events),
        );
        Self {
            directory,
            services,
            source,
            target,
            target_tab,
            other_pane,
            other_project,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn breadcrumb_실제_preview와_기존탭_1대1_reveal은_보조창과_다른_pane_프로젝트를_보존한다() {
    for auxiliary in [false, true] {
        let f = Fixture::new(auxiliary);
        let before = f.services.state.layouts.read().clone();
        let opened = open(&f.services, &f.source, f.target.clone(), ViewportId::ROOT)
            .await
            .unwrap();
        assert_eq!(opened.tab, f.target_tab);
        assert_eq!(opened.pane, f.source.pane);
        assert_eq!((opened.line, opened.column), (1.0, 1.0));
        assert_eq!(
            f.services.state.layouts.read()[&f.other_project],
            before[&f.other_project]
        );
        assert_eq!(
            taide_layout::service::find_leaf(&opened.layout.root, &f.other_pane),
            taide_layout::service::find_leaf(&before[&f.source.project].root, &f.other_pane)
        );
        assert!(crate::editor_problems::reply_is_current(
            &opened,
            &f.services.state.layouts.read()
        ));
        let mut reveals = crate::editor_reveal::Reveals::default();
        let now = std::time::Instant::now();
        assert!(reveals.queue(&opened, &f.services.state.layouts.read(), now));
        let position = reveals
            .consume(&opened.tab, &opened.path, ViewportId::ROOT, now)
            .unwrap();
        assert_eq!((position.line, position.column), (1.0, 1.0));
        let next = f.directory.join("project/src/new.rs");
        std::fs::write(&next, "new").unwrap();
        let mut source = f.source.clone();
        source.tab = opened.tab;
        source.path = opened.path;
        let next = open(
            &f.services,
            &source,
            next.to_str().unwrap().into(),
            ViewportId::ROOT,
        )
        .await
        .unwrap();
        let active = taide_layout::service::all_roots(&next.layout)
            .find_map(|root| taide_native_ui::snapshot::active_tab(root, &source.pane))
            .unwrap();
        assert!(active.preview);
        assert_eq!(active.id, next.tab);
        f.services
            .state
            .layouts
            .write()
            .get_mut(&source.project)
            .unwrap()
            .revision += 1;
        assert!(!crate::editor_problems::reply_is_current(
            &next,
            &f.services.state.layouts.read()
        ));
        tokio::time::timeout(DEADLINE, f.services.tasks.shutdown())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn breadcrumb_파일열기는_잠금대기중_바뀐_원본과_경계밖_파일_directory를_거절한다() {
    let f = Fixture::new(false);
    let guard = f.services.state.begin_mutation().await;
    let mut waiting = std::pin::pin!(open(
        &f.services,
        &f.source,
        f.target.clone(),
        ViewportId::ROOT
    ));
    assert!(matches!(
        std::future::poll_fn(|ctx| Poll::Ready(waiting.as_mut().poll(ctx))).await,
        Poll::Pending
    ));
    let mut layouts = f.services.state.layouts.write();
    let layout = layouts.get_mut(&f.source.project).unwrap();
    let PaneNode::Split { children, .. } = &mut layout.root else {
        panic!("split")
    };
    let PaneNode::Leaf { active, .. } = &mut children[0] else {
        panic!("leaf")
    };
    *active = None;
    let revision = layout.revision;
    drop(layouts);
    drop(guard);
    assert!(matches!(waiting.await, Err(AppError::NotFound(_))));
    assert_eq!(
        f.services.state.layouts.read()[&f.source.project].revision,
        revision
    );
    let outside = f.directory.join("outside.rs");
    std::fs::write(&outside, "outside").unwrap();
    assert_eq!(
        open(
            &f.services,
            &f.source,
            outside.to_str().unwrap().into(),
            ViewportId::ROOT
        )
        .await
        .err()
        .unwrap()
        .kind(),
        AppErrorKind::Forbidden
    );
    assert!(
        open(
            &f.services,
            &f.source,
            f.directory
                .join("project/src/nested")
                .to_str()
                .unwrap()
                .into(),
            ViewportId::ROOT
        )
        .await
        .is_err()
    );
    tokio::time::timeout(DEADLINE, f.services.tasks.shutdown())
        .await
        .unwrap();
}

#[tokio::test]
async fn breadcrumb_실제_host_tree_reveal은_현재파일의_형제를_공급하고_늦은_원본을_보존한다() {
    let f = Fixture::new(false);
    let mut bridge = crate::host::HostBridge::connect_with_clipboard_ports(
        f.services.clone(),
        Arc::new(|| {}),
        Arc::new(|_| Err(AppError::Forbidden("synthetic clipboard disabled".into()))),
        Arc::new(|| Err(AppError::Forbidden("synthetic clipboard disabled".into()))),
        None,
    )
    .unwrap();
    bridge
        .submit(crate::host::HostCommand::RevealBreadcrumbTree(
            f.source.clone(),
        ))
        .unwrap();
    let reply = tokio::time::timeout(DEADLINE, async {
        loop {
            if let Some(reply) = bridge.poll() {
                break reply;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let crate::host::HostReply::BreadcrumbTree { source, result } = reply else {
        panic!("breadcrumb tree reply")
    };
    assert_eq!(source, f.source);
    let page = result.unwrap();
    let siblings = crate::symbol_navigation::direct_children(
        &page.rows,
        std::path::Path::new(&source.path)
            .parent()
            .unwrap()
            .to_str()
            .unwrap(),
    );
    assert_eq!(siblings.len(), 3);
    assert!(siblings.iter().any(|row| row.path == f.target));
    f.services.state.layouts.write().remove(&f.source.project);
    assert!(
        !f.services
            .state
            .layouts
            .read()
            .get(&source.project)
            .is_some_and(|layout| source.is_active(layout))
    );
    tokio::time::timeout(DEADLINE, bridge.disconnect())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(DEADLINE, f.services.tasks.shutdown())
        .await
        .unwrap();
}
