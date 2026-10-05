use std::sync::{Mutex, atomic::AtomicBool};
use std::time::Instant;

use serde_json::Value;
use taide_git::store::StatusRead;
use taide_model::{
    agent::ExternalOpenRequest,
    file::{FsChange, FsChangeKind},
    flush::FlushScope,
    git::GitStatus,
    ids::ProjectId,
    lsp::{LspInstallPhase, LspServerId, LspSessionStatus},
    paths::AppPaths,
    project::Project,
    settings::Settings,
    sync::SyncStatus,
};
use taide_runtime::{AppState, TaskSupervisor};

use super::*;

const EXPECTED: &[(&str, &[&str])] = &[
    ("project:opened", &["project"]),
    ("project:closed", &["projectId"]),
    ("project:activated", &["projectId"]),
    ("project:list-changed", &["projects"]),
    ("project:groups-changed", &["groups"]),
    ("project:recent-cleared", &["removed", "skippedWithDrafts"]),
    ("session:shell-slots-changed", &["tree", "focused"]),
    ("session:window-chrome-changed", &["chrome"]),
    ("layout:changed", &["projectId", "revision"]),
    ("theme:changed", &["themeId"]),
    ("fs:changed", &["projectId", "change"]),
    ("fs:rescan-required", &["projectId"]),
    (
        "terminal:spawned",
        &["sessionId", "projectId", "cwd", "shell"],
    ),
    ("terminal:exited", &["sessionId", "code"]),
    ("terminal:cwd-changed", &["sessionId", "cwd"]),
    (
        "terminal:command-finished",
        &["sessionId", "cwd", "exitCode", "durationMs"],
    ),
    ("git:status-changed", &["projectId"]),
    ("git:refs-changed", &["projectId"]),
    (
        "lsp:session-status-changed",
        &["sessionId", "status", "lastError", "generation"],
    ),
    (
        "lsp:install-progress",
        &[
            "serverId",
            "phase",
            "receivedBytes",
            "totalBytes",
            "message",
        ],
    ),
    ("agent:state-changed", &["projectId", "agents"]),
    ("ide:status-changed", &["status"]),
    (
        "ide:diff-requested",
        &[
            "requestId",
            "projectId",
            "oldPath",
            "newPath",
            "newContents",
            "tabName",
        ],
    ),
    ("ide:save-requested", &["requestId", "projectId", "path"]),
    ("ide:close-tab-requested", &["tabName", "requestId"]),
    ("sync:state-changed", &["status"]),
    ("remote:state-changed", &["status"]),
    ("settings:changed", &["settings"]),
];
const CONTENT: &str = "synthetic \"quoted\"\n합성";
const BYTES: f64 = 512.0;
const DURATION: u32 = 321;

#[derive(Default)]
struct Sink {
    events: Mutex<Vec<AppEvent>>,
    git: Mutex<Option<GitStore>>,
    enabled: AtomicBool,
}

impl EventSink for Sink {
    fn publish(&self, event: AppEvent) {
        if self.enabled.load(Ordering::Acquire) {
            match &event {
                AppEvent::FsChanged { project_id, .. }
                | AppEvent::GitStatusChanged { project_id }
                | AppEvent::GitRefsChanged { project_id } => {
                    assert!(matches!(
                        self.git
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .read_status(project_id, Instant::now()),
                        StatusRead::Stale(_)
                    ));
                }
                _ => {}
            }
        }
        self.events.lock().unwrap().push(event);
    }
}

fn seed(git: &GitStore, project: &ProjectId) {
    git.invalidate_status(project);
    let StatusRead::Stale(pending) = git.read_status(project, Instant::now()) else {
        panic!("expected stale fixture cache");
    };
    git.finish_status(
        project,
        pending,
        &GitStatus {
            rows: Vec::new(),
            branch: None,
            ahead: 0,
            behind: 0,
            has_remote: false,
        },
    );
    assert!(matches!(
        git.read_status(project, Instant::now()),
        StatusRead::Fresh(_)
    ));
}

fn events(project: &ProjectId) -> Vec<AppEvent> {
    let project_data = Project {
        id: project.clone(),
        root: "/synthetic-project".into(),
        name: "synthetic".into(),
        capabilities: Vec::new(),
        root_missing: false,
        last_opened_at: 0.0,
        display: Default::default(),
    };
    vec![
        AppEvent::ProjectOpened {
            project: Box::new(project_data),
        },
        AppEvent::ProjectClosed {
            project_id: project.clone(),
        },
        AppEvent::ProjectActivated {
            project_id: Some(project.clone()),
        },
        AppEvent::ProjectListChanged {
            projects: Vec::new(),
        },
        AppEvent::ProjectGroupsChanged { groups: Vec::new() },
        AppEvent::ProjectRecentCleared {
            removed: 1,
            skipped_with_drafts: 0,
        },
        AppEvent::SessionShellSlotsChanged {
            tree: None,
            focused: None,
        },
        AppEvent::WindowChromeChanged {
            chrome: Default::default(),
        },
        AppEvent::LayoutChanged {
            project_id: project.clone(),
            revision: 1,
        },
        AppEvent::ThemeChanged {
            theme_id: "synthetic".into(),
        },
        AppEvent::FsChanged {
            project_id: project.clone(),
            change: FsChange {
                kind: FsChangeKind::Modified,
                paths: vec!["/synthetic-project/file".into()],
                from_app: true,
            },
        },
        AppEvent::FsRescanRequired {
            project_id: project.clone(),
        },
        AppEvent::TerminalSpawned {
            project_id: project.clone(),
            session_id: "synthetic".into(),
            cwd: "/synthetic-project".into(),
            shell: "synthetic".into(),
        },
        AppEvent::TerminalExited {
            session_id: "synthetic".into(),
            code: None,
        },
        AppEvent::TerminalCwdChanged {
            session_id: "synthetic".into(),
            cwd: "/synthetic-project".into(),
        },
        AppEvent::TerminalCommandFinished {
            session_id: "synthetic".into(),
            cwd: None,
            exit_code: Some(0),
            duration_ms: DURATION,
        },
        AppEvent::GitStatusChanged {
            project_id: project.clone(),
        },
        AppEvent::GitRefsChanged {
            project_id: project.clone(),
        },
        AppEvent::LspSessionStatusChanged {
            session_id: "synthetic".into(),
            status: LspSessionStatus::Crashed,
            last_error: Some("synthetic".into()),
            generation: 1,
        },
        AppEvent::LspInstallProgress {
            server_id: LspServerId::from("gopls"),
            phase: LspInstallPhase::Downloading,
            received_bytes: BYTES,
            total_bytes: None,
            message: None,
        },
        AppEvent::AgentStateChanged {
            project_id: project.clone(),
            agents: Vec::new(),
        },
        AppEvent::IdeStatusChanged {
            status: taide_ide::store::IdeStore::default().status(),
        },
        AppEvent::IdeDiffRequested {
            request_id: "synthetic".into(),
            project_id: project.clone(),
            old_path: "old".into(),
            new_path: "new".into(),
            new_contents: CONTENT.into(),
            tab_name: "synthetic".into(),
        },
        AppEvent::IdeSaveRequested {
            request_id: "synthetic".into(),
            project_id: project.clone(),
            path: "synthetic".into(),
        },
        AppEvent::IdeCloseTabRequested {
            tab_name: "synthetic".into(),
            request_id: None,
        },
        AppEvent::SyncStateChanged {
            status: SyncStatus {
                connected: false,
                has_gist: false,
                last_synced_at: None,
                remote_newer: None,
            },
        },
        AppEvent::RemoteStateChanged {
            status: RemoteStore::default().status(),
        },
        AppEvent::SettingsChanged {
            settings: Box::new(Settings::default()),
        },
    ]
}

#[tokio::test]
async fn 실제_bootstrap_relay는_원본28_wire_제외2와_git_before_forward_소유권을_보존한다() {
    let project = ProjectId::new();
    let sink = Arc::new(Sink::default());
    let directory = std::env::temp_dir().join(format!("taide-native-event-relay-{project}"));
    let assembly = crate::bootstrap::assemble(
        AppState::new(AppPaths::new(directory)),
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        sink.clone(),
    );
    let services = &assembly.services;
    *sink.git.lock().unwrap() = Some(services.git.clone());
    seed(&services.git, &project);
    services.events.publish(AppEvent::GitStatusChanged {
        project_id: project.clone(),
    });
    assert!(matches!(
        services.git.read_status(&project, Instant::now()),
        StatusRead::Fresh(_)
    ));
    sink.events.lock().unwrap().clear();
    let ports = crate::remote_git::Ports::new(assembly.git_events.clone());
    services
        .git
        .ensure_invalidation_listeners(|| (ports.install_status_invalidation)(services));
    sink.enabled.store(true, Ordering::Release);
    let mut receiver = services.remote.subscribe_events();
    let originals = events(&project);
    assert_eq!(originals.len(), EXPECTED.len());
    for (event, (name, keys)) in originals.iter().zip(EXPECTED) {
        seed(&services.git, &project);
        services.events.publish(event.clone());
        let raw = receiver.try_recv().unwrap();
        let envelope: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(envelope["t"], "event");
        assert_eq!(envelope["event"], *name);
        let payload: Value = serde_json::from_str(envelope["payload"].as_str().unwrap()).unwrap();
        let actual: Vec<_> = payload
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut expected = keys.to_vec();
        expected.sort_unstable();
        assert_eq!(actual, expected);
        if *name == "ide:diff-requested" {
            assert_eq!(payload["newContents"], CONTENT);
        }
        if *name == "lsp:install-progress" {
            assert_eq!(payload["receivedBytes"], BYTES);
            assert!(payload["totalBytes"].is_null());
        }
        if matches!(
            event,
            AppEvent::FsChanged { .. }
                | AppEvent::GitStatusChanged { .. }
                | AppEvent::GitRefsChanged { .. }
        ) {
            assert!(matches!(
                services.git.read_status(&project, Instant::now()),
                StatusRead::Stale(_)
            ));
        } else {
            assert!(matches!(
                services.git.read_status(&project, Instant::now()),
                StatusRead::Fresh(_)
            ));
        }
    }
    assert_eq!(*sink.events.lock().unwrap(), originals);
    let excluded = [
        AppEvent::AgentExternalOpen {
            request: ExternalOpenRequest {
                path: "synthetic".into(),
                wait_marker: None,
            },
        },
        AppEvent::HotExitFlushRequested {
            timeout_ms: 0.0,
            scope: FlushScope::All,
        },
    ];
    for event in &excluded {
        assert!(frame(event).is_none());
        services.events.publish(event.clone());
        assert!(matches!(
            receiver.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
    }
    let owner = Arc::downgrade(services);
    services.tasks.shutdown().await;
    assert_eq!(services.tasks.tracked_count(), 0);
    drop(ports);
    drop(assembly);
    assert!(owner.upgrade().is_none());

    let eager_sink = Arc::new(Sink::default());
    let eager = crate::bootstrap::services(
        AppState::new(AppPaths::new(std::env::temp_dir().join(format!(
            "taide-native-event-relay-eager-{}",
            ProjectId::new()
        )))),
        TaskSupervisor::new(tokio::runtime::Handle::current()),
        eager_sink.clone(),
    );
    *eager_sink.git.lock().unwrap() = Some(eager.git.clone());
    eager_sink.enabled.store(true, Ordering::Release);
    seed(&eager.git, &project);
    eager.events.publish(AppEvent::FsChanged {
        project_id: project,
        change: FsChange {
            kind: FsChangeKind::Modified,
            paths: Vec::new(),
            from_app: false,
        },
    });
    eager.tasks.shutdown().await;
    assert_eq!(eager.tasks.tracked_count(), 0);
}
