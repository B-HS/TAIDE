use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use taide_infra::watcher::WatchNotification;
use taide_model::app_event::AppEvent;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::ids::{ProjectId, TabId};
use taide_model::layout::{PaneNode, TabKind};
use taide_model::paths::AppPaths;
use taide_model::project::CapabilityKind;
use taide_native_app::bootstrap::services;
use taide_native_app::events::TreeChanges;
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::projects::{NativeProjects, git_invalidation};
use taide_runtime::{AppState, EventSink, ExitDrain, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(5);
const LARGE_TREE_FILES: usize = 300;
const DUPLICATE_PATHS: usize = 140;
const OVERFLOW_DIRECTORIES: usize = 140;

#[derive(Default)]
struct Events(Mutex<Vec<AppEvent>>, Notify);
impl EventSink for Events {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
        self.1.notify_one();
    }
}

static OS_WATCH_REGISTRATION: Mutex<()> = Mutex::new(());

fn exclusive_os_watch_registration() -> std::sync::MutexGuard<'static, ()> {
    OS_WATCH_REGISTRATION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("taide-native-projects-{}", ProjectId::new()));
        std::fs::create_dir_all(path.join("root/.git/refs/heads")).unwrap();
        std::fs::create_dir_all(path.join("data")).unwrap();
        for index in 0..LARGE_TREE_FILES {
            std::fs::write(path.join(format!("root/file-{index}.rs")), "abc").unwrap();
        }
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

async fn reply(bridge: &mut HostBridge, signal: &Notify) -> HostReply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            signal.notified().await;
        }
    })
    .await
    .unwrap()
}

fn file_tab(state: &AppState, project: &ProjectId, path: &str) -> TabId {
    let layouts = state.layouts.read();
    let PaneNode::Leaf { tabs, .. } = &layouts[project].root else {
        panic!("synthetic root pane")
    };
    tabs.iter()
        .find(|tab| matches!(&tab.kind, TabKind::File { path: tab_path } if tab_path == path))
        .unwrap()
        .id
        .clone()
}

#[test]
fn 실제_project_open_restore_file_tab_전체_tree와_exit가_같은_소유권을_쓴다() {
    let _os_watch_registration = exclusive_os_watch_registration();
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let events = Arc::new(Events::default());
    let services = services(state.clone(), tasks.clone(), events.clone());
    let projects = NativeProjects::new(services.clone());
    let signal = Arc::new(Notify::new());
    let ready = signal.clone();
    let mut bridge =
        HostBridge::connect(services.clone(), Arc::new(move || ready.notify_one())).unwrap();
    runtime.block_on(async {
        let root = fixture.0.join("root").to_str().unwrap().to_owned();
        bridge
            .submit(HostCommand::OpenProject(root.clone()))
            .unwrap();
        let project = loop {
            let project = state.projects.read().values().next().cloned();
            if let Some(project) = project {
                break project;
            }
            tokio::time::timeout(TIMEOUT, signal.notified())
                .await
                .unwrap();
            if let Some(HostReply::Failed(error)) = bridge.poll() {
                panic!("{error}")
            }
        };
        let id = project.id;
        bridge
            .submit(HostCommand::TreeRows {
                project: id.clone(),
                offset: 0,
            })
            .unwrap();
        let HostReply::Tree { result, .. } = reply(&mut bridge, &signal).await else {
            panic!("tree reply")
        };
        let page = result.unwrap();
        assert!(page.rows.len() >= LARGE_TREE_FILES);
        assert_eq!(page.rows.len(), page.total as usize);
        assert_eq!(
            project.capabilities,
            vec![CapabilityKind::Git, CapabilityKind::Terminal]
        );
        assert!(state.watchers.read().contains_key(&id));
        assert!(state.git_watchers.read().contains_key(&id));
        assert!(state.layouts.read().contains_key(&id));
        let first = fixture
            .0
            .join("root/file-0.rs")
            .to_str()
            .unwrap()
            .to_owned();
        let second = fixture
            .0
            .join("root/file-1.rs")
            .to_str()
            .unwrap()
            .to_owned();
        for _ in 0..2 {
            bridge
                .submit(HostCommand::OpenFileTab {
                    project: id.clone(),
                    pane: None,
                    path: first.clone(),
                    preview: true,
                })
                .unwrap();
        }
        bridge
            .submit(HostCommand::TreeRows {
                project: id.clone(),
                offset: 0,
            })
            .unwrap();
        assert!(matches!(
            reply(&mut bridge, &signal).await,
            HostReply::Tree { result: Ok(_), .. }
        ));
        let tab = file_tab(&state, &id, &first);
        let tab_count = match &state.layouts.read()[&id].root {
            PaneNode::Leaf { tabs, .. } => tabs.len(),
            _ => panic!("pane"),
        };
        bridge
            .submit(HostCommand::SetDirty {
                tab: tab.clone(),
                dirty: true,
            })
            .unwrap();
        bridge
            .submit(HostCommand::OpenFileTab {
                project: id.clone(),
                pane: None,
                path: second.clone(),
                preview: true,
            })
            .unwrap();
        bridge
            .submit(HostCommand::TreeRows {
                project: id.clone(),
                offset: 0,
            })
            .unwrap();
        assert!(matches!(
            reply(&mut bridge, &signal).await,
            HostReply::Tree { result: Ok(_), .. }
        ));
        assert_eq!(file_tab(&state, &id, &first), tab);
        assert_ne!(file_tab(&state, &id, &second), tab);
        {
            let layouts = state.layouts.read();
            let PaneNode::Leaf { tabs, .. } = &layouts[&id].root else {
                panic!("pane")
            };
            assert_eq!(tabs.len(), tab_count + 1);
            assert!(
                tabs.iter()
                    .find(|candidate| candidate.id == tab)
                    .unwrap()
                    .dirty
            );
        }
        let revision = state.layouts.read()[&id].revision;
        projects.restore_watchers().await.unwrap();
        assert_eq!(state.layouts.read()[&id].revision, revision);
        assert_eq!(state.watchers.read().len(), 1);
        assert_eq!(state.git_watchers.read().len(), 1);
        assert_eq!(projects.open(root).await.unwrap().id, id);
        let added = fixture.0.join("root/added.rs");
        std::fs::write(&added, "new").unwrap();
        bridge
            .submit(HostCommand::RefreshTree {
                project: id.clone(),
                dirs: None,
            })
            .unwrap();
        let HostReply::TreeSynced { result, errors, .. } = reply(&mut bridge, &signal).await else {
            panic!("rescan reply")
        };
        assert!(errors.is_empty());
        assert!(
            result
                .unwrap()
                .rows
                .iter()
                .any(|row| row.name == "added.rs")
        );
        let outside = fixture.0.join("outside.rs");
        std::fs::write(&outside, "denied").unwrap();
        bridge
            .submit(HostCommand::OpenFileTab {
                project: id.clone(),
                pane: None,
                path: outside.to_str().unwrap().into(),
                preview: false,
            })
            .unwrap();
        assert!(matches!(
            reply(&mut bridge, &signal).await,
            HostReply::Failed(_)
        ));
        bridge.disconnect().await.unwrap();
        let drain = ExitDrain::new(services.ai_requests.clone()).with_state(state.clone());
        tokio::time::timeout(
            TIMEOUT,
            drain.wait_for_direct_exit(
                tasks.clone(),
                services.lsp_install.clone(),
                services.lsp.clone(),
                services.terminal.clone(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(state.watchers.read().is_empty());
        assert!(state.git_watchers.read().is_empty());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(
            events
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|event| matches!(event, AppEvent::ProjectOpened { .. }))
        );
    });
}

#[test]
fn 실제_os_watcher는_파일과_git_변경을_전달하고_종료까지_join한다() {
    let _os_watch_registration = exclusive_os_watch_registration();
    let fixture = Fixture::new();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let state = AppState::new(AppPaths::new(fixture.0.join("data")));
    let events = Arc::new(Events::default());
    let services = services(state.clone(), tasks.clone(), events.clone());
    runtime.block_on(async {
        let projects = NativeProjects::new(services.clone());
        let project = projects.open(fixture.0.join("root").to_str().unwrap().into()).await.unwrap();
        events.0.lock().unwrap().clear();
        std::fs::write(fixture.0.join("root/new-from-outside.rs"), "external").unwrap();
        std::fs::write(fixture.0.join("root/.git/index"), "synthetic index change").unwrap();
        let received = tokio::time::timeout(TIMEOUT, async {
            loop {
                let ready = {
                    let captured = events.0.lock().unwrap();
                    let file = captured.iter().any(|event| matches!(event, AppEvent::FsChanged { project_id, change }
                        if project_id == &project.id && !change.from_app && change.paths.iter().any(|path| path.ends_with("/new-from-outside.rs"))));
                    let git = captured.iter().any(|event| matches!(event, AppEvent::GitStatusChanged { project_id } if project_id == &project.id));
                    file && git
                };
                if ready { break }
                events.1.notified().await;
            }
        }).await;
        let observations = {
            let captured = events.0.lock().unwrap();
            let files = captured.iter().filter(|event| matches!(event, AppEvent::FsChanged { .. })).count();
            let git = captured.iter().filter(|event| matches!(event, AppEvent::GitStatusChanged { .. })).count();
            (files, git, captured.len())
        };
        let drain = ExitDrain::new(services.ai_requests.clone()).with_state(state.clone());
        tokio::time::timeout(TIMEOUT, drain.wait_for_direct_exit(tasks.clone(), services.lsp_install.clone(), services.lsp.clone(), services.terminal.clone())).await.unwrap().unwrap();
        assert!(state.watchers.read().is_empty());
        assert!(state.git_watchers.read().is_empty());
        assert_eq!(tasks.tracked_count(), 0);
        assert!(received.is_ok(), "watcher delivery timeout: fs={}, git={}, total={}", observations.0, observations.1, observations.2);
    });
}

#[test]
fn 파일_이벤트는_중복과_self_echo를_합치고_포화는_rescan으로_보존한다() {
    let changes = TreeChanges::default();
    let project = ProjectId::new();
    let mut paths = vec!["/root/first/file.rs".into(); DUPLICATE_PATHS];
    paths.push("/root/last/file.rs".into());
    changes.record(&AppEvent::FsChanged {
        project_id: project.clone(),
        change: FsChange {
            paths,
            kind: FsChangeKind::Created,
            from_app: true,
        },
    });
    assert_eq!(
        changes.take([]).remove(&project).unwrap().unwrap(),
        BTreeSet::from(["/root/first".into(), "/root/last".into()])
    );
    changes.record(&AppEvent::FsChanged {
        project_id: project.clone(),
        change: FsChange {
            paths: vec!["/root/file.rs".into()],
            kind: FsChangeKind::Modified,
            from_app: true,
        },
    });
    assert!(changes.take([]).is_empty());
    changes.record(&AppEvent::FsChanged {
        project_id: project.clone(),
        change: FsChange {
            paths: (0..OVERFLOW_DIRECTORIES)
                .map(|index| format!("/root/dir-{index}/file.rs"))
                .collect(),
            kind: FsChangeKind::Renamed,
            from_app: false,
        },
    });
    assert!(changes.take([]).remove(&project).unwrap().is_none());
    let tick = WatchNotification::Changes(vec![FsChange {
        paths: vec![
            "/root/.git/index".into(),
            "/root/.git/refs/heads/main".into(),
            "/root/.git/objects/a/b".into(),
        ],
        kind: FsChangeKind::Modified,
        from_app: false,
    }]);
    assert_eq!(git_invalidation(&tick), (true, true));
    assert_eq!(
        git_invalidation(&WatchNotification::RescanRequired),
        (true, true)
    );
    assert_eq!(
        git_invalidation(&WatchNotification::Changes(Vec::new())),
        (false, false)
    );
}

#[cfg(unix)]
#[tokio::test]
async fn project_닫기는_해당_project의_terminal_hub_entry와_admission만_회수한다() {
    use taide_model::error::AppError;
    use taide_model::project::Project;
    use taide_model::terminal::PtySpawnOptions;
    use taide_native_app::terminal_dispatch::EffectPorts;
    use taide_native_app::terminal_host::{Hub, Limits};
    use taide_native_app::{terminal_frames, terminal_writer};
    use taide_native_terminal::session::Phase;
    use taide_native_terminal::{Rgb, WindowSize};
    use taide_runtime::project_actions::ProjectLifecyclePort;

    const SESSIONS: usize = 2;
    const QUEUE_BYTES: usize = 256 * 1024;
    const QUEUE_COUNT: usize = 64;
    const QUEUE_VISITS: usize = 4096;
    const COLUMNS: u16 = 80;
    const ROWS: u16 = 24;
    const HISTORY: usize = 128;
    const RETIRE_POLL: Duration = Duration::from_millis(5);
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let state = AppState::new(AppPaths::new(std::env::temp_dir().join(format!(
        "taide-native-project-terminals-{}",
        ProjectId::new()
    ))));
    let closed_project = ProjectId::new();
    let kept_project = ProjectId::new();
    for project in [&closed_project, &kept_project] {
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project.clone(),
                root: env!("CARGO_MANIFEST_DIR").into(),
                name: "synthetic project terminal".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
    }
    let services = services(state, tasks.clone(), Arc::new(Events::default()));
    let hub = Arc::new(
        Hub::new(
            services.clone(),
            Limits {
                sessions: SESSIONS,
                core: Default::default(),
                frames: terminal_frames::Limits {
                    bytes: QUEUE_BYTES,
                    count: QUEUE_COUNT,
                    visits: QUEUE_VISITS,
                },
                writer: terminal_writer::Limits {
                    bytes: QUEUE_BYTES,
                    count: QUEUE_COUNT,
                },
            },
        )
        .unwrap(),
    );
    let opts = |project: &ProjectId| PtySpawnOptions {
        project_id: project.clone(),
        cwd: env!("CARGO_MANIFEST_DIR").into(),
        shell: Some("/bin/cat".into()),
        cols: COLUMNS,
        rows: ROWS,
        scrollback_bytes: None,
    };
    let ports = || EffectPorts {
        command_colors: Default::default(),
        updated: Arc::new(|| {}),
        color: Arc::new(|_| Ok(Rgb { r: 0, g: 0, b: 0 })),
        geometry: Arc::new(|| {
            Ok(WindowSize {
                num_cols: COLUMNS,
                num_lines: ROWS,
                cell_width: 1,
                cell_height: 1,
            })
        }),
        event: Arc::new(|_| Ok(())),
        stream: Arc::new(|_| Ok(())),
    };
    let closed = hub
        .spawn(
            opts(&closed_project),
            HISTORY,
            async { Vec::new() },
            ports(),
        )
        .await
        .unwrap();
    let kept = hub
        .spawn(opts(&kept_project), HISTORY, async { Vec::new() }, ports())
        .await
        .unwrap();
    assert!(matches!(
        hub.spawn(
            opts(&ProjectId::new()),
            HISTORY,
            async { Vec::new() },
            ports()
        )
        .await,
        Err(AppError::InvalidArgument(_))
    ));
    let retired = Arc::downgrade(&hub.get(&closed).unwrap());
    NativeProjects::new(services.clone())
        .with_terminals(hub.clone())
        .detach_all(&closed_project);
    assert!(hub.get(&closed).is_none());
    assert!(
        services
            .terminal
            .sessions_for_project(&closed_project)
            .is_empty()
    );
    assert_eq!(
        hub.get(&kept)
            .unwrap()
            .snapshot(|state| state.phase)
            .unwrap(),
        Phase::Running
    );
    tokio::time::timeout(TIMEOUT, async {
        while retired.upgrade().is_some() {
            tokio::time::sleep(RETIRE_POLL).await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        hub.spawn(
            opts(&ProjectId::new()),
            HISTORY,
            async { Vec::new() },
            ports()
        )
        .await,
        Err(AppError::NotFound(_))
    ));
    tokio::time::timeout(TIMEOUT, hub.close(&kept))
        .await
        .unwrap()
        .unwrap();
    services.terminal.shutdown();
    tokio::time::timeout(TIMEOUT, services.terminal.wait_for_idle())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(TIMEOUT, tasks.shutdown())
        .await
        .unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}
