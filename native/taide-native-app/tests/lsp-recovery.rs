#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use taide_lsp::native::Phase;
use taide_model::app_event::AppEvent;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_model::paths::AppPaths;
use taide_model::project::Project;
use taide_native_app::lsp::status::Summary;
use taide_native_app::lsp::{LspBridge, Reply};
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(5);
const BYTE_LIMIT: usize = 1024;
const EXECUTABLE_MODE: u32 = 0o700;
const TAB_SIZE: u32 = 4;
const CONTENT: &str = "最新文😀 ";
const LIVE_PREFIX: &str = "live:";

struct Directory(PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Sink;

impl EventSink for Sink {
    fn publish(&self, _event: AppEvent) {}
}

async fn reply(bridge: &mut LspBridge, ready: &Notify, stage: &str) -> Reply {
    tokio::time::timeout(DEADLINE, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("production LspBridge timed out during {stage}"))
}

async fn summary(
    bridge: &LspBridge,
    ready: &Notify,
    project: &ProjectId,
    expected: Option<Summary>,
) {
    tokio::time::timeout(DEADLINE, async {
        while bridge.summary(project) != expected {
            ready.notified().await;
        }
    })
    .await
    .expect("production LspBridge status publication timed out");
}

#[tokio::test]
async fn 실제_editor_bridge는_child_crash후_같은_session의_문서를_replay하고_종료시_회수한다() {
    let directory = Directory(
        std::env::temp_dir().join(format!("taide-native-editor-recovery-{}", ProjectId::new())),
    );
    let root = directory.0.join("root");
    let bin = directory.0.join("bin");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&bin).unwrap();
    let mock = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples/native-lsp-mock");
    assert!(mock.is_file());
    let executable = bin.join("rust-analyzer");
    let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
    std::fs::write(
        &executable,
        format!("#!/bin/sh\nexec '{escaped}' --native-actions\n"),
    )
    .unwrap();
    std::fs::set_permissions(
        &executable,
        std::fs::Permissions::from_mode(EXECUTABLE_MODE),
    )
    .unwrap();
    let project = ProjectId::new();
    let state = AppState::new(AppPaths::new(directory.0.join("data")));
    state.projects.write().insert(
        project.clone(),
        Project {
            id: project.clone(),
            root: root.canonicalize().unwrap().to_str().unwrap().into(),
            name: "synthetic recovery".into(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: Default::default(),
        },
    );
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let services = taide_native_app::bootstrap::services(state, tasks.clone(), Arc::new(Sink));
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let mut bridge = LspBridge::connect(
        services.clone(),
        bin.as_os_str().to_owned(),
        Arc::new(move || signal.notify_one()),
    )
    .unwrap();
    let path = root.join("one.rs");
    std::fs::write(&path, CONTENT).unwrap();
    let path = path.canonicalize().unwrap();
    let file = taide_file::service::open_file(&path, &[], false).unwrap();
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 3,
        max_views: 2,
        max_undo_groups: 1,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let document = store.open_file(path, file).unwrap();
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
    bridge
        .sync(
            project.clone(),
            store.documents().snapshot(document).unwrap(),
        )
        .unwrap();
    let initial_owner = loop {
        match reply(&mut bridge, &ready, "initial diagnostics").await {
            Reply::Diagnostics { owner, .. } => break owner,
            Reply::Synced { .. } => {}
            Reply::Failed { error, .. } => panic!("initial session failed: {error}"),
            _ => panic!("unexpected initial reply"),
        }
    };
    let second_path = root.join("two.rs");
    std::fs::write(&second_path, CONTENT).unwrap();
    let second_path = second_path.canonicalize().unwrap();
    let second_file = taide_file::service::open_file(&second_path, &[], false).unwrap();
    let second_document = store.open_file(second_path, second_file).unwrap();
    bridge
        .sync(
            project.clone(),
            store.documents().snapshot(second_document).unwrap(),
        )
        .unwrap();
    summary(
        &bridge,
        &ready,
        &project,
        Some(Summary {
            running: 1,
            total: 1,
            has_crashed: false,
        }),
    )
    .await;
    assert_eq!(bridge.summary(&ProjectId::new()), None);
    loop {
        match reply(&mut bridge, &ready, "second document committed").await {
            Reply::Diagnostics {
                owner,
                document: current,
                ..
            } if current == second_document => {
                assert_eq!(owner, initial_owner);
                break;
            }
            Reply::Synced { .. } | Reply::Diagnostics { .. } => {}
            Reply::Failed { error, .. } => panic!("second document failed: {error}"),
            _ => panic!("unexpected second document reply"),
        }
    }
    let bindings = bridge.diagnostic_bindings().unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(
        bindings[&initial_owner],
        std::collections::HashSet::from([document, second_document])
    );
    let usage_labels = bridge.usage_labels();
    let initial_labels = usage_labels(&services);
    assert_eq!(initial_labels.len(), 1);
    assert_eq!(
        initial_labels.values().next().unwrap().0,
        taide_model::system::SystemUsageProcessKind::Lsp
    );
    let server_name = taide_lsp::manifest::servers()
        .into_iter()
        .find(|server| server.id.as_str() == "rustAnalyzer")
        .unwrap()
        .name;
    assert_eq!(
        initial_labels.values().next().unwrap().1,
        format!("{server_name} · synthetic recovery")
    );
    services.lsp.kill_all();
    loop {
        match reply(&mut bridge, &ready, "initial crash state").await {
            Reply::Synced { sessions, .. }
                if sessions.iter().any(|snapshot| {
                    snapshot.generation == 0 && snapshot.phase == Phase::Degraded
                }) =>
            {
                break;
            }
            Reply::Synced { .. } | Reply::Diagnostics { .. } => {}
            Reply::Failed { error, .. } => panic!("crash state failed: {error}"),
            _ => panic!("unexpected crash state reply"),
        }
    }
    summary(
        &bridge,
        &ready,
        &project,
        Some(Summary {
            running: 0,
            total: 1,
            has_crashed: true,
        }),
    )
    .await;
    replace_selections(&mut store, view, LIVE_PREFIX, None).unwrap();
    bridge
        .sync(
            project.clone(),
            store.documents().snapshot(document).unwrap(),
        )
        .unwrap();
    let recovered = loop {
        match reply(&mut bridge, &ready, "crash recovery").await {
            Reply::Synced { sessions, .. } => {
                if let Some(snapshot) = sessions
                    .into_iter()
                    .find(|snapshot| snapshot.generation == 1 && snapshot.phase == Phase::Running)
                {
                    break snapshot;
                }
            }
            Reply::Diagnostics { .. } => {}
            Reply::Failed { error, .. } => panic!("recovery failed: {error}"),
            _ => panic!("unexpected recovery reply"),
        }
    };
    assert!(recovered.pid.is_some());
    assert_eq!(recovered.pending, 0);
    let bindings = bridge.diagnostic_bindings().unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(
        bindings[&initial_owner],
        std::collections::HashSet::from([document, second_document])
    );
    summary(
        &bridge,
        &ready,
        &project,
        Some(Summary {
            running: 1,
            total: 1,
            has_crashed: false,
        }),
    )
    .await;
    let recovered_labels = usage_labels(&services);
    assert_eq!(recovered_labels.len(), 1);
    assert_eq!(
        recovered_labels.get(&recovered.pid.unwrap()),
        Some(&(
            taide_model::system::SystemUsageProcessKind::Lsp,
            format!("{server_name} · synthetic recovery")
        ))
    );
    bridge
        .format(
            store.documents().snapshot(document).unwrap(),
            taide_lsp::native::protocol::lsp_types::FormattingOptions {
                tab_size: TAB_SIZE,
                insert_spaces: true,
                ..Default::default()
            },
        )
        .unwrap();
    loop {
        match reply(&mut bridge, &ready, "replayed formatting").await {
            Reply::Formatted { snapshot, result } => {
                assert_eq!(snapshot.id, document);
                let edits = result.unwrap().unwrap();
                assert_eq!(edits.len(), 1);
                assert_eq!(
                    edits[0].new_text,
                    format!("formatted:{LIVE_PREFIX}{CONTENT}")
                );
                break;
            }
            Reply::Synced { .. } | Reply::Diagnostics { .. } => {}
            Reply::Failed { error, .. } => panic!("replayed formatting failed: {error}"),
            _ => panic!("unexpected formatting reply"),
        }
    }
    services.lsp.kill_all();
    loop {
        match reply(&mut bridge, &ready, "close during backoff").await {
            Reply::Synced { sessions, .. }
                if sessions.iter().any(|snapshot| {
                    snapshot.generation == 1 && snapshot.phase == Phase::Degraded
                }) =>
            {
                break;
            }
            Reply::Synced { .. } | Reply::Diagnostics { .. } => {}
            Reply::Failed { error, .. } => panic!("closing recovery failed: {error}"),
            _ => panic!("unexpected closing reply"),
        }
    }
    bridge.retain(&Default::default()).unwrap();
    bridge.retain_projects(&Default::default()).unwrap();
    tokio::time::timeout(DEADLINE, async {
        while tasks.tracked_count() != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    services.lsp.wait_for_idle().await;
    summary(&bridge, &ready, &project, None).await;
    assert!(usage_labels(&services).is_empty());
    assert!(bridge.diagnostic_bindings().unwrap().is_empty());
    bridge
        .sync(
            project.clone(),
            store.documents().snapshot(document).unwrap(),
        )
        .unwrap();
    let replacement_owner = loop {
        match reply(&mut bridge, &ready, "reacquired owner").await {
            Reply::Diagnostics {
                owner,
                document: current,
                ..
            } if current == document => break owner,
            Reply::Synced { .. } | Reply::Diagnostics { .. } => {}
            Reply::Failed { error, .. } => panic!("reacquired session failed: {error}"),
            _ => panic!("unexpected reacquired reply"),
        }
    };
    assert_ne!(replacement_owner, initial_owner);
    let workspace = root.join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = 'synthetic'\nversion = '0.0.0'\n",
    )
    .unwrap();
    let third_path = workspace.join("three.rs");
    std::fs::write(&third_path, CONTENT).unwrap();
    let third_path = third_path.canonicalize().unwrap();
    let third_file = taide_file::service::open_file(&third_path, &[], false).unwrap();
    let third_document = store.open_file(third_path, third_file).unwrap();
    bridge
        .sync(
            project.clone(),
            store.documents().snapshot(third_document).unwrap(),
        )
        .unwrap();
    let workspace_owner = loop {
        match reply(&mut bridge, &ready, "disjoint workspace owner").await {
            Reply::Diagnostics {
                owner,
                document: current,
                ..
            } if current == third_document => break owner,
            Reply::Synced { .. } | Reply::Diagnostics { .. } => {}
            Reply::Failed { error, .. } => panic!("workspace session failed: {error}"),
            _ => panic!("unexpected workspace reply"),
        }
    };
    assert_ne!(workspace_owner, replacement_owner);
    let bindings = bridge.diagnostic_bindings().unwrap();
    assert_eq!(bindings.len(), 2);
    assert_eq!(
        bindings[&replacement_owner],
        std::collections::HashSet::from([document])
    );
    assert_eq!(
        bindings[&workspace_owner],
        std::collections::HashSet::from([third_document])
    );
    tokio::time::timeout(DEADLINE, bridge.disconnect())
        .await
        .unwrap()
        .unwrap();
    services.state.begin_shutdown();
    services.lsp.shutdown();
    tokio::time::timeout(DEADLINE, services.lsp.wait_for_idle())
        .await
        .unwrap();
    tasks.shutdown().await;
    assert_eq!(tasks.tracked_count(), 0);
    assert!(usage_labels(&services).is_empty());
    let owner = Arc::downgrade(&services);
    drop(services);
    assert!(owner.upgrade().is_none());
}
