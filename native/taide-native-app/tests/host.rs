use std::sync::Arc;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::bootstrap::{LaunchConfig, services};
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{AppState, EventSink, TaskSupervisor, file_actions};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(3);
const DOCUMENT_LIMIT: usize = 2;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const EXTERNAL_DISK_SECONDS: u64 = 2;

struct Sink;
impl EventSink for Sink {
    fn publish(&self, _: AppEvent) {}
}

struct Fixture {
    dir: std::path::PathBuf,
    path: String,
    project: ProjectId,
    state: AppState,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("taide-native-app-host-{}", ProjectId::new()));
        let root = dir.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("editor.rs");
        std::fs::write(&path, "abc").unwrap();
        let state = AppState::new(AppPaths::new(dir.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: root.to_str().unwrap().into(),
                name: "synthetic native host".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        Self {
            dir,
            path: path.to_str().unwrap().into(),
            project,
            state,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).unwrap();
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

#[tokio::test]
async fn 실제_problems_open은_preview_tab과_reveal_경계_거절_호스트수명을_보존한다() {
    let fixture = Fixture::new();
    let path = std::fs::canonicalize(&fixture.path)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = services(fixture.state.clone(), tasks.clone(), Arc::new(Sink));
    let owner = Arc::downgrade(&services);
    fixture.state.settings.write().enable_preview_tabs = true;
    fixture.state.layouts.write().insert(
        fixture.project.clone(),
        taide_layout::service::default_layout(),
    );
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect_with_clipboard_ports(
        services.clone(),
        Arc::new(move || signal.notify_one()),
        Arc::new(|_| panic!("clipboard write is forbidden")),
        Arc::new(|| panic!("clipboard read is forbidden")),
        None,
    )
    .unwrap();
    let viewport = eframe::egui::ViewportId::ROOT;
    let mut previous_tab = None;
    for line in [3, 7] {
        host.submit(HostCommand::OpenProblem {
            project: fixture.project.clone(),
            path: path.clone(),
            line,
            column: 8,
            viewport,
        })
        .unwrap();
        let HostReply::ProblemOpened { result } = reply(&mut host, &ready).await else {
            panic!("expected Problems completion");
        };
        let opened = result.unwrap();
        assert_eq!(
            (
                &opened.project,
                &opened.path,
                opened.line,
                opened.column,
                opened.viewport
            ),
            (&fixture.project, &path, line as f64, 8.0, viewport)
        );
        if let Some(previous) = previous_tab.as_ref() {
            assert_eq!(&opened.tab, previous);
        }
        previous_tab = Some(opened.tab.clone());
        let taide_model::layout::PaneNode::Leaf { tabs, active, .. } = &opened.layout.root else {
            panic!("expected actual leaf");
        };
        assert!(
            tabs.iter()
                .any(|tab| Some(&tab.id) == active.as_ref() && tab.id == opened.tab && tab.preview)
        );
        let mut reveals = taide_native_app::editor_reveal::Reveals::default();
        let now = std::time::Instant::now();
        assert!(reveals.queue(&opened, &fixture.state.layouts.read(), now));
        let position = reveals.consume(&opened.tab, &path, viewport, now).unwrap();
        assert_eq!((position.line, position.column), (line as f64, 8.0));
    }
    let outside = fixture.dir.join("outside.rs");
    std::fs::write(&outside, "synthetic outside").unwrap();
    let before = fixture.state.layouts.read()[&fixture.project].clone();
    for (candidate, line) in [
        (outside.to_string_lossy().into_owned(), 1),
        (path.clone(), 0),
    ] {
        host.submit(HostCommand::OpenProblem {
            project: fixture.project.clone(),
            path: candidate,
            line,
            column: 1,
            viewport,
        })
        .unwrap();
        let HostReply::ProblemOpened { result } = reply(&mut host, &ready).await else {
            panic!("expected Problems rejection");
        };
        let error = result
            .err()
            .expect("invalid Problems request must not mutate the layout");
        let expected = if line == 0 {
            taide_model::error::AppErrorKind::InvalidArgument
        } else {
            taide_model::error::AppErrorKind::Forbidden
        };
        assert_eq!(error.kind(), expected);
        assert_eq!(fixture.state.layouts.read()[&fixture.project], before);
    }
    tokio::time::timeout(TIMEOUT, host.disconnect())
        .await
        .unwrap()
        .unwrap();
    fixture.state.begin_shutdown();
    services.lsp.shutdown();
    services.lsp.wait_for_idle().await;
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
    drop(services);
    assert!(owner.upgrade().is_none());
}

#[test]
fn 실제_host_옆으로열기는_빈그룹과_중복파일을_원본_단일mutation으로_처리한다() {
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = services(fixture.state.clone(), tasks.clone(), Arc::new(Sink));
    let mut layout = taide_layout::service::default_layout();
    layout.root = taide_model::layout::PaneNode::Leaf {
        id: layout.focused_pane.clone(),
        tabs: Vec::new(),
        active: None,
    };
    let source_pane = layout.focused_pane.clone();
    fixture
        .state
        .layouts
        .write()
        .insert(fixture.project.clone(), layout);
    let row = taide_model::tree::TreeRow {
        path: fixture.path.clone(),
        name: "editor.rs".into(),
        kind: taide_model::tree::TreeEntryKind::File,
        depth: 0,
        expanded: false,
        has_children: false,
    };
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut host = HostBridge::connect(services, Arc::new(move || signal.notify_one())).unwrap();
    runtime.block_on(async {
        let mut original_tab = None;
        for expected_count in [1, 2] {
            let before = fixture.state.layouts.read()[&fixture.project].clone();
            let request = taide_native_app::explorer::open_to_side_request(
                &fixture.project, &row, Some(&before), &taide_native_ui::shell::WindowScope::Main,
            ).unwrap();
            host.submit(HostCommand::OpenFileToSide(request)).unwrap();
            let after = tokio::time::timeout(TIMEOUT, async {
                loop {
                    if let Some(reply) = host.poll() {
                        if let HostReply::Failed(error) = reply { panic!("split failed: {error}"); }
                        panic!("unexpected split reply");
                    }
                    let after = fixture.state.layouts.read()[&fixture.project].clone();
                    if taide_native_app::tabs::tabs_in(&after.root).len() == expected_count { return after }
                    ready.notified().await;
                }
            }).await.unwrap();
            assert_eq!(after.revision, before.revision + 1);
            if expected_count == 1 {
                assert!(matches!(after.root, taide_model::layout::PaneNode::Leaf { .. }));
                assert!(taide_layout::service::find_leaf(&after.root, &source_pane).is_none());
            } else {
                assert!(matches!(after.root, taide_model::layout::PaneNode::Split { .. }));
                assert!(taide_layout::service::find_leaf(&after.root, &before.focused_pane).is_some());
            }
            let tabs = taide_native_app::tabs::tabs_in(&after.root);
            assert!(tabs.iter().all(|tab| !tab.preview && !tab.pinned && matches!(&tab.kind, taide_model::layout::TabKind::File { path } if path == &fixture.path)));
            if let Some(id) = &original_tab {
                assert!(tabs.iter().any(|tab| &tab.id == id));
                assert_ne!(tabs[0].id, tabs[1].id);
            } else { original_tab = Some(tabs[0].id.clone()); }
        }
        let before = fixture.state.layouts.read()[&fixture.project].clone();
        let mut request = taide_native_app::explorer::open_to_side_request(
            &fixture.project, &row, Some(&before), &taide_native_ui::shell::WindowScope::Main,
        ).unwrap();
        request.kind = taide_model::layout::TabKind::File { path: fixture.dir.join("outside.rs").to_str().unwrap().into() };
        host.submit(HostCommand::OpenFileToSide(request)).unwrap();
        assert!(matches!(reply(&mut host, &ready).await, HostReply::Failed(_)));
        assert_eq!(fixture.state.layouts.read()[&fixture.project], before);
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), "abc");
        tokio::time::timeout(TIMEOUT, host.disconnect()).await.unwrap().unwrap();
        tokio::time::timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
#[cfg(unix)]
fn native_save는_retarget된_symlink에_이전_문서의_초안을_쓰지_않는다() {
    let fixture = Fixture::new();
    let alias = fixture.dir.join("root/alias.rs");
    let other = fixture.dir.join("root/other.rs");
    std::os::unix::fs::symlink(&fixture.path, &alias).unwrap();
    std::fs::write(&other, "other untouched").unwrap();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let ready = Arc::new(Notify::new());
            let repaint = ready.clone();
            let mut bridge = HostBridge::connect(
                services(fixture.state.clone(), tasks.clone(), Arc::new(Sink)),
                Arc::new(move || repaint.notify_one()),
            )
            .unwrap();
            bridge
                .submit(HostCommand::OpenDocument(alias.to_str().unwrap().into()))
                .unwrap();
            let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected alias document");
            };
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let document = result.unwrap().commit(&mut store).unwrap();
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
            std::fs::remove_file(&alias).unwrap();
            std::os::unix::fs::symlink(&other, &alias).unwrap();
            bridge
                .submit(HostCommand::Save {
                    path: alias.to_str().unwrap().into(),
                    snapshot: store.save_snapshot(document).unwrap(),
                })
                .unwrap();
            let HostReply::Saved { result, .. } = reply(&mut bridge, &ready).await else {
                panic!("expected save result");
            };
            assert!(
                result.is_err(),
                "saving a replaced canonical identity must fail"
            );
            assert_eq!(std::fs::read_to_string(&other).unwrap(), "other untouched");
            assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), "abc");
            assert!(store.documents().snapshot(document).unwrap().dirty);
            bridge.disconnect().await.unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}

#[test]
fn 실행_설정은_명시적_절대_data_dir만_허용한다() {
    for arguments in [
        vec![],
        vec!["--data-dir"],
        vec!["--data-dir", "/"],
        vec!["--data-dir", "relative"],
        vec!["--data-dir", "/tmp/../data"],
        vec!["--data-dir", "/tmp/data", "extra"],
    ] {
        assert!(LaunchConfig::parse(arguments.into_iter().map(String::from)).is_err());
    }
    assert_eq!(
        LaunchConfig::parse(["--data-dir".into(), "/tmp/native-test-data".into()])
            .unwrap()
            .data_dir,
        std::path::PathBuf::from("/tmp/native-test-data")
    );
}

#[test]
fn 실제_native_host는_열기_편집_저장_live_mirror_보호와_미소비_token_회수를_연결한다() {
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = services(fixture.state.clone(), tasks.clone(), Arc::new(Sink));
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let repaint: Arc<dyn Fn() + Send + Sync> = Arc::new(move || signal.notify_one());
    let mut bridge = HostBridge::connect(services.clone(), repaint.clone()).unwrap();
    runtime.block_on(async {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        bridge
            .submit(HostCommand::OpenDocument(fixture.path.clone()))
            .unwrap();
        let HostReply::Opened {
            path,
            project,
            result,
        } = reply(&mut bridge, &ready).await
        else {
            panic!("expected opened document")
        };
        assert_eq!(path, fixture.path);
        assert_eq!(project, Some(fixture.project.clone()));
        let document = result.unwrap().commit(&mut store).unwrap();
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
        replace_selections(&mut store, view, "한", None).unwrap();
        let snapshot = store.save_snapshot(document).unwrap();
        bridge
            .submit(HostCommand::Save {
                path: fixture.path.clone(),
                snapshot,
            })
            .unwrap();
        let HostReply::Saved { snapshot, result } = reply(&mut bridge, &ready).await else {
            panic!("expected saved document")
        };
        result.unwrap();
        assert!(store.mark_saved(snapshot, None).unwrap());
        assert_eq!(std::fs::read_to_string(&fixture.path).unwrap(), "한abc");
        file_actions::file_mirror_dirty(
            &fixture.state,
            &tasks,
            fixture.project.clone(),
            fixture.path.clone(),
            "draft".into(),
        )
        .await
        .unwrap();
        bridge
            .submit(HostCommand::OpenDocument(fixture.path.clone()))
            .unwrap();
        let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
            panic!("expected guarded document")
        };
        let admitted = result.unwrap().commit_with_notice(&mut store).unwrap();
        assert_eq!(admitted.document, document);
        assert_eq!(admitted.restored_conflict, None);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "한abc"
        );
        assert_eq!(
            file_actions::file_list_mirrors(&fixture.state, fixture.project.clone())
                .await
                .unwrap()[0]
                .content,
            "draft"
        );
        file_actions::file_clear_mirror(
            &fixture.state,
            fixture.project.clone(),
            fixture.path.clone(),
        )
        .await
        .unwrap();
        bridge
            .submit(HostCommand::OpenDocument(fixture.path.clone()))
            .unwrap();
        tokio::time::timeout(TIMEOUT, async {
            while bridge.pending_replies() == 0 {
                ready.notified().await;
            }
        })
        .await
        .unwrap();
        let worker = bridge.disconnect();
        tokio::time::timeout(TIMEOUT, worker)
            .await
            .unwrap()
            .unwrap();
        let guard = tokio::time::timeout(TIMEOUT, fixture.state.begin_mutation())
            .await
            .unwrap();
        drop(guard);
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn 실제_native_host_mirror_복원은_disk_baseline_conflict와_공유_draft를_구분한다() {
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = services(fixture.state.clone(), tasks.clone(), Arc::new(Sink));
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut bridge =
        HostBridge::connect(services.clone(), Arc::new(move || signal.notify_one())).unwrap();
    runtime.block_on(async {
        let disk = std::fs::File::open(&fixture.path).unwrap();
        disk.set_times(
            std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + Duration::from_secs(1)),
        )
        .unwrap();
        file_actions::file_mirror_dirty(
            &fixture.state,
            &tasks,
            fixture.project.clone(),
            fixture.path.clone(),
            "restored draft".into(),
        )
        .await
        .unwrap();
        std::fs::write(&fixture.path, "external disk").unwrap();
        disk.set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + Duration::from_secs(EXTERNAL_DISK_SECONDS)),
        )
        .unwrap();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: DOCUMENT_LIMIT,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: BYTE_LIMIT,
        })
        .unwrap();
        bridge
            .submit(HostCommand::OpenDocument(fixture.path.clone()))
            .unwrap();
        let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
            panic!("opened")
        };
        let admitted = result.unwrap().commit_with_notice(&mut store).unwrap();
        assert_eq!(admitted.restored_conflict, Some(true));
        let restored = store.documents().snapshot(admitted.document).unwrap();
        assert_eq!(restored.rope.to_string(), "restored draft");
        assert!(restored.dirty);
        assert_eq!(
            restored.metadata.disk_modified_ms,
            Some(EXTERNAL_DISK_SECONDS as f64 * taide_infra::clock::MS_PER_SECOND)
        );
        let view = store
            .attach_view(
                ViewKey {
                    window: "main".into(),
                    pane: PaneId::new(),
                    tab: TabId::new(),
                },
                admitted.document,
            )
            .unwrap();
        replace_selections(&mut store, view, "live ", None).unwrap();
        let live = store.documents().snapshot(admitted.document).unwrap().rope;
        let alias = std::path::Path::new(&fixture.path)
            .parent()
            .unwrap()
            .join(".")
            .join("editor.rs")
            .to_str()
            .unwrap()
            .to_owned();
        bridge.submit(HostCommand::OpenDocument(alias)).unwrap();
        let HostReply::Opened { result, .. } = reply(&mut bridge, &ready).await else {
            panic!("opened alias")
        };
        let duplicate = result.unwrap().commit_with_notice(&mut store).unwrap();
        assert_eq!(duplicate.document, admitted.document);
        assert_eq!(duplicate.restored_conflict, None);
        assert_eq!(
            store.documents().snapshot(admitted.document).unwrap().rope,
            live
        );
        let snapshot = store.save_snapshot(admitted.document).unwrap();
        bridge
            .submit(HostCommand::Save {
                path: fixture.path.clone(),
                snapshot,
            })
            .unwrap();
        let HostReply::Saved { snapshot, result } = reply(&mut bridge, &ready).await else {
            panic!("saved draft")
        };
        result.unwrap();
        assert!(store.mark_saved(snapshot, None).unwrap());
        assert_eq!(
            std::fs::read_to_string(&fixture.path).unwrap(),
            live.to_string()
        );
        assert!(
            file_actions::file_list_mirrors(&fixture.state, fixture.project.clone())
                .await
                .unwrap()
                .is_empty()
        );
        bridge.disconnect().await.unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    });
}

#[test]
fn 종료_layout_flush는_저장_실패를_숨기거나_dirty를_버리지_않는다() {
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services = services(fixture.state.clone(), tasks.clone(), Arc::new(Sink));
    fixture.state.layouts.write().insert(
        fixture.project.clone(),
        taide_layout::service::default_layout(),
    );
    fixture
        .state
        .dirty_layouts
        .write()
        .insert(fixture.project.clone());
    let data_dir = &fixture.state.paths.data_dir;
    std::fs::write(data_dir, "not a directory").unwrap();
    runtime.block_on(async {
        assert!(
            taide_native_app::host::flush_layouts(&services)
                .await
                .is_err()
        );
        assert!(
            fixture
                .state
                .dirty_layouts
                .read()
                .contains(&fixture.project)
        );
        assert_eq!(tasks.tracked_count(), 0);
    });
}
