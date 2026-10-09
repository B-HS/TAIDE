use super::*;
use eframe::egui::ViewportId;
use std::{future::Future, path::PathBuf, sync::Arc, task::Poll, time::Duration};
use taide_model::{
    app_event::AppEvent,
    error::AppErrorKind,
    ids::{ShellSlotId, TabId},
    layout::{AuxWindowLayout, SplitDir},
    paths::AppPaths,
    project::{Project, ShellSlotTree},
};
use taide_runtime::{AppState, EventSink, TaskSupervisor};

const DEADLINE: Duration = Duration::from_secs(5);
const AUX_SLOT: u32 = 4;
const STALE_CASES: usize = 5;

struct Events;
impl EventSink for Events {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    directory: PathBuf,
    services: Arc<AppServices>,
    project: ProjectId,
    focused: PaneId,
    other: PaneId,
    target: String,
    target_tab: TabId,
    scope: WindowScope,
}

impl Fixture {
    fn new(auxiliary: bool) -> Self {
        let directory =
            std::env::temp_dir().join(format!("taide-workspace-symbol-host-{}", ProjectId::new()));
        let root = directory.join("project");
        std::fs::create_dir_all(&root).unwrap();
        let current = root.join("current.rs");
        let target = root.join("target.rs");
        std::fs::write(&current, "current").unwrap();
        std::fs::write(&target, "class\n  \u{1f600}method\nend").unwrap();
        let current = current.canonicalize().unwrap().to_str().unwrap().to_owned();
        let target = target.canonicalize().unwrap().to_str().unwrap().to_owned();
        let project = ProjectId::new();
        let focused = PaneId::new();
        let other = PaneId::new();
        let target_tab = TabId::new();
        let leaf = |pane, path, tab: TabId, pinned| PaneNode::Leaf {
            id: pane,
            tabs: vec![Tab {
                id: tab.clone(),
                kind: TabKind::File { path },
                title: "synthetic".into(),
                pinned,
                preview: false,
                dirty: false,
                view_state: Some("preserved state".into()),
            }],
            active: Some(tab),
        };
        let tree = PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![
                leaf(focused.clone(), current, TabId::new(), false),
                leaf(other.clone(), target.clone(), target_tab.clone(), true),
            ],
            sizes: vec![0.5, 0.5],
        };
        let mut layout = taide_layout::service::default_layout();
        let scope = if auxiliary {
            layout.auxiliary_windows.push(AuxWindowLayout {
                slot: AUX_SLOT,
                root: tree,
                focused_pane: focused.clone(),
            });
            WindowScope::Auxiliary {
                project: project.clone(),
                slot: AUX_SLOT,
            }
        } else {
            layout.root = tree;
            layout.focused_pane = focused.clone();
            WindowScope::Main
        };
        let state = AppState::new(AppPaths::new(directory.join("data")));
        state.settings.write().enable_preview_tabs = true;
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.canonicalize().unwrap().to_str().unwrap().into(),
                name: "synthetic".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        state.layouts.write().insert(project.clone(), layout);
        state.session.write().shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: ShellSlotId::new(),
            project_id: project.clone(),
        });
        let services = crate::bootstrap::services(
            state,
            TaskSupervisor::new(tokio::runtime::Handle::current()),
            Arc::new(Events),
        );
        Self {
            directory,
            services,
            project,
            focused,
            other,
            target,
            target_tab,
            scope,
        }
    }

    fn request(&self, path: String) -> Request {
        let layouts = self.services.state.layouts.read();
        let layout = &layouts[&self.project];
        let (_, focused) =
            crate::symbol_sidebar::window_tree(&self.project, layout, &self.scope).unwrap();
        Request {
            project: self.project.clone(),
            pane: destination(layout, &self.project, &self.scope, &path).unwrap(),
            focused: focused.clone(),
            revision: layout.revision,
            scope: self.scope.clone(),
            path,
            line: 2,
            column: 5,
            viewport: ViewportId::ROOT,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.services.tasks.stop_all();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn workspace_host는_현재창의_기존_고정탭과_preview_및_utf16_reveal을_보존한다() {
    for auxiliary in [false, true] {
        let f = Fixture::new(auxiliary);
        let before = f.services.state.layouts.read()[&f.project].clone();
        let request = f.request(f.target.clone());
        assert_eq!(request.pane, f.other);
        let opened = open(&f.services, request).await.unwrap();
        assert_eq!(opened.tab, f.target_tab);
        assert_eq!((opened.line, opened.column), (2.0, 5.0));
        let (root, focused) =
            crate::symbol_sidebar::window_tree(&f.project, &opened.layout, &f.scope).unwrap();
        assert_eq!(focused, &f.other);
        let active = taide_native_ui::snapshot::active_tab(root, &f.other).unwrap();
        let (before_root, _) =
            crate::symbol_sidebar::window_tree(&f.project, &before, &f.scope).unwrap();
        assert_eq!(
            taide_layout::service::find_leaf(root, &f.focused),
            taide_layout::service::find_leaf(before_root, &f.focused)
        );
        assert!(active.pinned && !active.preview);
        assert_eq!(active.view_state.as_deref(), Some("preserved state"));
        if auxiliary {
            assert_eq!(opened.layout.root, before.root);
        }
        let mut reveals = crate::editor_reveal::Reveals::default();
        let now = std::time::Instant::now();
        assert!(reveals.queue(&opened, &f.services.state.layouts.read(), now));
        let position = reveals
            .consume(&opened.tab, &opened.path, ViewportId::ROOT, now)
            .unwrap();
        assert_eq!((position.line, position.column), (2.0, 5.0));
        let next = f.directory.join("project/preview.rs");
        std::fs::write(&next, "preview").unwrap();
        let next = next.to_str().unwrap().to_owned();
        let request = f.request(next.clone());
        assert_eq!(request.pane, f.other);
        let preview = open(&f.services, request).await.unwrap();
        let (root, _) =
            crate::symbol_sidebar::window_tree(&f.project, &preview.layout, &f.scope).unwrap();
        assert!(
            taide_native_ui::snapshot::active_tab(root, &f.other)
                .unwrap()
                .preview
        );
        assert!(crate::editor_problems::reply_is_current(
            &preview,
            &f.services.state.layouts.read()
        ));
        f.services
            .state
            .layouts
            .write()
            .get_mut(&f.project)
            .unwrap()
            .revision += 1;
        assert!(!crate::editor_problems::reply_is_current(
            &preview,
            &f.services.state.layouts.read()
        ));
        tokio::time::timeout(DEADLINE, f.services.tasks.shutdown())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn workspace_host는_잠금대기후_바뀐_pane_세대_프로젝트_창_슬롯과_경계밖_파일을_거절한다() {
    for change in 0..STALE_CASES {
        let f = Fixture::new(change == 2);
        let request = f.request(f.target.clone());
        let guard = f.services.state.begin_mutation().await;
        let mut waiting = std::pin::pin!(open(&f.services, request));
        assert!(matches!(
            std::future::poll_fn(|ctx| Poll::Ready(waiting.as_mut().poll(ctx))).await,
            Poll::Pending
        ));
        match change {
            0 => {
                f.services
                    .state
                    .layouts
                    .write()
                    .get_mut(&f.project)
                    .unwrap()
                    .revision += 1
            }
            1 => {
                f.services
                    .state
                    .layouts
                    .write()
                    .get_mut(&f.project)
                    .unwrap()
                    .focused_pane = f.other.clone()
            }
            2 => f
                .services
                .state
                .layouts
                .write()
                .get_mut(&f.project)
                .unwrap()
                .auxiliary_windows
                .clear(),
            3 => {
                f.services.state.projects.write().remove(&f.project);
            }
            _ => f.services.state.session.write().shell_slots = None,
        }
        let before = f.services.state.layouts.read().clone();
        drop(guard);
        assert!(matches!(waiting.await, Err(AppError::NotFound(_))));
        assert_eq!(*f.services.state.layouts.read(), before);
    }
    let f = Fixture::new(false);
    let outside = f.directory.join("outside.rs");
    std::fs::write(&outside, "outside").unwrap();
    assert_eq!(
        open(&f.services, f.request(outside.to_str().unwrap().into()))
            .await
            .err()
            .unwrap()
            .kind(),
        AppErrorKind::Forbidden
    );
    assert!(
        open(
            &f.services,
            f.request(f.directory.join("project").to_str().unwrap().into())
        )
        .await
        .is_err()
    );
    let mut request = f.request(f.target.clone());
    request.column = 0;
    assert!(open(&f.services, request).await.is_err());
    tokio::time::timeout(DEADLINE, f.services.tasks.shutdown())
        .await
        .unwrap();
}

#[tokio::test]
async fn workspace_실제_host_bridge는_검색파일을_열고_현재_응답만_reveal한다() {
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
        .submit(crate::host::HostCommand::OpenWorkspaceSymbol(
            f.request(f.target.clone()),
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
    let crate::host::HostReply::WorkspaceSymbolOpened { result } = reply else {
        panic!("workspace symbol reply")
    };
    let opened = result.unwrap();
    assert_eq!(opened.tab, f.target_tab);
    assert!(crate::editor_problems::reply_is_current(
        &opened,
        &f.services.state.layouts.read()
    ));
    f.services.state.layouts.write().remove(&f.project);
    assert!(!crate::editor_problems::reply_is_current(
        &opened,
        &f.services.state.layouts.read()
    ));
    tokio::time::timeout(DEADLINE, bridge.disconnect())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(DEADLINE, f.services.tasks.shutdown())
        .await
        .unwrap();
}
