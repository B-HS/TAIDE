#![cfg(unix)]

use std::collections::HashSet;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use taide_model::app_event::AppEvent;
use taide_model::file::FileSizeTier;
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{AuxWindowLayout, PaneNode, SplitDir, Tab, TabKind};
use taide_model::paths::AppPaths;
use taide_model::project::{Project, ProjectRef, SessionShellState, ShellSlotTree};
use taide_native_app::bootstrap::services;
use taide_native_app::lsp::{LspBridge, Reply, visible_files};
use taide_native_editor::document::EditorError;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::lsp::apply_text_edits;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::ViewKey;
use taide_native_ui::shell::WindowScope;
use taide_native_ui::snapshot::ShellSnapshot;
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const TIMEOUT: Duration = Duration::from_secs(5);
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const EXECUTABLE_MODE: u32 = 0o700;
const TAB_SIZE: u32 = 4;
const AUXILIARY_SLOT: u32 = 1;
const RESIZER_THICKNESS: f32 = 4.0;

#[test]
fn 실제_shell의_visible_file만_활성_tab과_zen_slot_창_scope에_맞춰_선택한다() {
    let project = ProjectId::new();
    let other = ProjectId::new();
    let slot = ShellSlotId::new();
    let other_slot = ShellSlotId::new();
    let tab = |path: &str| Tab {
        id: TabId::new(),
        kind: TabKind::File { path: path.into() },
        title: path.into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    };
    let leaf = |tabs: Vec<Tab>| PaneNode::Leaf {
        id: PaneId::new(),
        active: tabs.first().map(|tab| tab.id.clone()),
        tabs,
    };
    let mut layout = taide_layout::service::default_layout();
    let mut settings = tab("not-a-file");
    settings.kind = TabKind::Settings;
    layout.root = PaneNode::Split {
        id: PaneId::new(),
        dir: SplitDir::Horizontal,
        children: vec![
            leaf(vec![tab("first.rs"), tab("inactive.rs")]),
            leaf(vec![settings, tab("behind-settings.rs")]),
        ],
        sizes: vec![1.0, 1.0],
    };
    layout.auxiliary_windows.push(AuxWindowLayout {
        slot: AUXILIARY_SLOT,
        root: leaf(vec![tab("auxiliary.rs")]),
        focused_pane: PaneId::new(),
    });
    let mut other_layout = taide_layout::service::default_layout();
    other_layout.root = leaf(vec![tab("other.rs")]);
    let mut snapshot = ShellSnapshot {
        projects: [project.clone(), other.clone()]
            .into_iter()
            .map(|id| ProjectRef {
                id,
                root: String::new(),
                name: String::new(),
                display: Default::default(),
                root_missing: false,
            })
            .collect(),
        groups: Vec::new(),
        shell: SessionShellState {
            tree: Some(ShellSlotTree::Split {
                dir: SplitDir::Horizontal,
                children: vec![
                    ShellSlotTree::Leaf {
                        slot_id: slot.clone(),
                        project_id: project.clone(),
                    },
                    ShellSlotTree::Leaf {
                        slot_id: other_slot,
                        project_id: other.clone(),
                    },
                ],
                sizes: vec![1.0, 1.0],
            }),
            focused: Some(slot),
            window_chrome: Default::default(),
        },
        layouts: std::collections::HashMap::from([
            (project.clone(), layout),
            (other.clone(), other_layout),
        ]),
        hide_status_in_zen: false,
        resizer_thickness: RESIZER_THICKNESS,
        welcome_on_empty_editor: true,
    };
    assert_eq!(
        visible_files(&snapshot, &WindowScope::Main),
        vec![(&project, "first.rs"), (&other, "other.rs")]
    );
    snapshot.shell.window_chrome.zen = true;
    assert_eq!(
        visible_files(&snapshot, &WindowScope::Main),
        vec![(&project, "first.rs")]
    );
    snapshot.shell.focused = None;
    assert!(visible_files(&snapshot, &WindowScope::Main).is_empty());
    assert_eq!(
        visible_files(
            &snapshot,
            &WindowScope::Auxiliary {
                project: project.clone(),
                slot: AUXILIARY_SLOT
            }
        ),
        vec![(&project, "auxiliary.rs")]
    );
    snapshot.projects.clear();
    snapshot.shell.window_chrome.zen = false;
    assert!(visible_files(&snapshot, &WindowScope::Main).is_empty());
}

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

async fn reply(bridge: &mut LspBridge, ready: &Notify, stage: &str) -> Reply {
    tokio::time::timeout(TIMEOUT, async {
        loop {
            if let Some(reply) = bridge.poll() {
                return reply;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("{stage}: {error}"))
}

fn options() -> taide_lsp::native::protocol::lsp_types::FormattingOptions {
    taide_lsp::native::protocol::lsp_types::FormattingOptions {
        tab_size: TAB_SIZE,
        insert_spaces: true,
        ..Default::default()
    }
}

#[test]
fn 실제_저장은_fix_all_resolve_edit_command_server_apply_edit_imports_format_cleanup_순서로_진행한다()
 {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir()
                    .join(format!("taide-native-save-actions-{}", ProjectId::new())),
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
                    root: root.to_str().unwrap().into(),
                    name: "synthetic save actions".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state, tasks.clone(), Arc::new(Sink));
            let ready = Arc::new(Notify::new());
            let signal = ready.clone();
            let mut bridge = LspBridge::connect(
                services.clone(),
                bin.as_os_str().to_owned(),
                Arc::new(move || signal.notify_one()),
            )
            .unwrap();
            let signal = ready.clone();
            let mut host = taide_native_app::host::HostBridge::connect(
                services.clone(),
                Arc::new(move || signal.notify_one()),
            )
            .unwrap();
            let path = root.join("one.rs");
            std::fs::write(&path, "文😀 ").unwrap();
            let canonical = std::fs::canonicalize(&path).unwrap();
            let file = taide_file::service::open_file(&canonical, &[], false).unwrap();
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let document = store.open_file(canonical, file).unwrap();
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
            let mut synced = false;
            let mut diagnosed = false;
            while !synced || !diagnosed {
                match reply(&mut bridge, &ready, "initial documents").await {
                    Reply::Synced { .. } => synced = true,
                    Reply::Diagnostics { diagnostics, .. } => {
                        assert_eq!(diagnostics.diagnostics.len(), 1);
                        diagnosed = true;
                    }
                    Reply::Failed { error, .. } => panic!("initial LSP failure: {error}"),
                    _ => panic!("unexpected initial reply"),
                }
            }
            bridge
                .participate(
                    store.documents().snapshot(document).unwrap(),
                    Some(options()),
                    taide_native_app::lsp::SaveActionFlags {
                        fix_all: true,
                        organize_imports: true,
                    },
                )
                .unwrap();
            let mut applied = 0usize;
            loop {
                match reply(&mut bridge, &ready, "save participants").await {
                    Reply::DocumentQuery { path, completion } => {
                        let document = store
                            .documents()
                            .find(&taide_native_editor::document::DocumentKey::File(path))
                            .and_then(|document| store.documents().snapshot(document).ok());
                        let _result = completion.send(document);
                    }
                    Reply::WorkspaceEdit(event) => {
                        let outcome =
                            taide_native_app::lsp_workspace::apply_open(&mut store, &event);
                        assert_eq!(outcome.failure, None);
                        assert_eq!(outcome.changed, vec![document]);
                        applied += 1;
                        bridge
                            .sync(
                                project.clone(),
                                store.documents().snapshot(document).unwrap(),
                            )
                            .unwrap();
                        event.completion.send(true).unwrap();
                    }
                    Reply::Formatted { snapshot, result } => {
                        assert_eq!(applied, 3);
                        assert_eq!(
                            store
                                .documents()
                                .snapshot(document)
                                .unwrap()
                                .rope
                                .to_string(),
                            "imports:command:fixed:文😀 "
                        );
                        assert!(
                            apply_text_edits(
                                &mut store,
                                &snapshot,
                                Some(view),
                                result.unwrap().unwrap()
                            )
                            .unwrap()
                        );
                        break;
                    }
                    Reply::Synced { .. } | Reply::Diagnostics { .. } => {}
                    Reply::Failed { error, .. } => panic!("save participant failure: {error}"),
                    Reply::SyntaxFolding { .. }
                    | Reply::DocumentSymbols { .. }
                    | Reply::WorkspaceSymbols { .. } => {
                        panic!("unexpected symbol request")
                    }
                    Reply::DeletePrepare(_)
                    | Reply::ExplorerDeletePrepare(_)
                    | Reply::ExplorerMovePrepare(_)
                    | Reply::ExplorerMoved(_)
                    | Reply::ExplorerDeleted(_)
                    | Reply::ExplorerDeleteFinished { .. }
                    | Reply::ExplorerCreated(_)
                    | Reply::ExplorerRenamed(_)
                    | Reply::ExplorerPasted(_)
                    | Reply::Deleted(_)
                    | Reply::RenamePrepare(_)
                    | Reply::Renamed(_) => {
                        panic!("unexpected delete action")
                    }
                }
            }
            let requested = store.save_snapshot(document).unwrap();
            let prepared = taide_native_app::save::prepare(
                &mut store,
                requested,
                Some(view),
                taide_native_editor::save_cleanup::CleanupFlags {
                    trim_trailing_whitespace: false,
                    insert_final_newline: true,
                },
                false,
            )
            .unwrap()
            .unwrap();
            host.submit(taide_native_app::host::HostCommand::Save {
                path: path.to_str().unwrap().into(),
                snapshot: prepared.snapshot,
            })
            .unwrap();
            let saved = tokio::time::timeout(TIMEOUT, async {
                loop {
                    if let Some(reply) = host.poll() {
                        break reply;
                    }
                    ready.notified().await;
                }
            })
            .await
            .unwrap();
            let taide_native_app::host::HostReply::Saved { result, .. } = saved else {
                panic!("expected saved file")
            };
            result.unwrap();
            assert_eq!(
                std::fs::read_to_string(path).unwrap(),
                "formatted:imports:command:fixed:文😀 \n"
            );
            bridge
                .sync(project, store.documents().snapshot(document).unwrap())
                .unwrap();
            bridge.saved(document).unwrap();
            tokio::time::timeout(TIMEOUT, host.disconnect())
                .await
                .unwrap()
                .unwrap();
            tokio::time::timeout(TIMEOUT, bridge.disconnect())
                .await
                .unwrap()
                .unwrap();
            services.lsp.shutdown();
            tokio::time::timeout(TIMEOUT, services.lsp.wait_for_idle())
                .await
                .unwrap();
            tokio::time::timeout(TIMEOUT, tasks.shutdown())
                .await
                .unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}

#[test]
fn 실제_native_lsp_연결_정책은_대형_readonly_tier와_프로젝트_밖_문서를_제외한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir().join(format!("taide-native-lsp-policy-{}", ProjectId::new())),
            );
            let root = directory.0.join("root");
            std::fs::create_dir_all(&root).unwrap();
            let project = ProjectId::new();
            let state = AppState::new(AppPaths::new(directory.0.join("data")));
            state.projects.write().insert(
                project.clone(),
                Project {
                    id: project.clone(),
                    root: root.to_str().unwrap().into(),
                    name: "synthetic policy".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state, tasks.clone(), Arc::new(Sink));
            let ready = Arc::new(Notify::new());
            let signal = ready.clone();
            let mut bridge = LspBridge::connect(
                services.clone(),
                std::ffi::OsString::new(),
                Arc::new(move || signal.notify_one()),
            )
            .unwrap();
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let path = root.join("one.rs");
            std::fs::write(&path, "body").unwrap();
            let canonical = std::fs::canonicalize(path).unwrap();
            let file = taide_file::service::open_file(&canonical, &[], false).unwrap();
            let document = store.open_file(canonical, file).unwrap();
            for tier in [
                FileSizeTier::Large,
                FileSizeTier::ReadOnly,
                FileSizeTier::Refused,
            ] {
                let mut snapshot = store.documents().snapshot(document).unwrap();
                snapshot.metadata.tier = tier;
                bridge.sync(project.clone(), snapshot.clone()).unwrap();
                let Reply::Synced { sessions, .. } =
                    reply(&mut bridge, &ready, "excluded tier").await
                else {
                    panic!("expected no sessions")
                };
                assert!(sessions.is_empty());
                bridge.format(snapshot, options()).unwrap();
                let Reply::Formatted { result, .. } =
                    reply(&mut bridge, &ready, "excluded formatter").await
                else {
                    panic!("expected no formatter")
                };
                assert!(result.unwrap().is_none());
            }
            let path = directory.0.join("outside.rs");
            std::fs::write(&path, "outside").unwrap();
            let canonical = std::fs::canonicalize(path).unwrap();
            let file = taide_file::service::open_file(&canonical, &[], false).unwrap();
            let outside = store.open_file(canonical, file).unwrap();
            bridge
                .sync(project, store.documents().snapshot(outside).unwrap())
                .unwrap();
            let Reply::Failed { error, .. } = reply(&mut bridge, &ready, "outside document").await
            else {
                panic!("expected root refusal")
            };
            assert_eq!(error.kind(), taide_model::error::AppErrorKind::Forbidden);
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
fn 실제_native_app_lsp는_discovery_공유문서_포맷_late_edit_닫기와_종료를_연결한다() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let directory = Directory(
                std::env::temp_dir().join(format!("taide-native-app-lsp-{}", ProjectId::new())),
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
            assert!(
                mock.is_file(),
                "build the native-lsp-mock example before this integration test"
            );
            let executable = bin.join("rust-analyzer");
            let escaped = mock.to_str().unwrap().replace('\'', "'\\''");
            std::fs::write(
                &executable,
                format!("#!/bin/sh\nexec '{escaped}' --native-document\n"),
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
                    root: root.to_str().unwrap().into(),
                    name: "synthetic lsp".into(),
                    capabilities: Vec::new(),
                    root_missing: false,
                    last_opened_at: 0.0,
                    display: Default::default(),
                },
            );
            let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
            let services = services(state, tasks.clone(), Arc::new(Sink));
            let ready = Arc::new(Notify::new());
            let signal = ready.clone();
            let mut bridge = LspBridge::connect(
                services.clone(),
                bin.as_os_str().to_owned(),
                Arc::new(move || signal.notify_one()),
            )
            .unwrap();
            let mut store = EditorStore::new(EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            })
            .unwrap();
            let paths = [root.join("one.rs"), root.join("two.rs")];
            let mut documents = Vec::new();
            for path in paths {
                std::fs::write(&path, "文😀 ").unwrap();
                let canonical = std::fs::canonicalize(path).unwrap();
                let file = taide_file::service::open_file(&canonical, &[], false).unwrap();
                assert_eq!(file.language_id, "rust");
                documents.push(store.open_file(canonical, file).unwrap());
            }
            let view = store
                .attach_view(
                    ViewKey {
                        window: "main".into(),
                        pane: PaneId::new(),
                        tab: TabId::new(),
                    },
                    documents[0],
                )
                .unwrap();
            let first = store.documents().snapshot(documents[0]).unwrap();
            bridge.sync(project.clone(), first.clone()).unwrap();
            let Reply::Synced { sessions, .. } = reply(&mut bridge, &ready, "first sync").await
            else {
                panic!("expected synchronized document");
            };
            assert_eq!(sessions.len(), 1);
            bridge.sync(project.clone(), first.clone()).unwrap();
            bridge.format(first.clone(), options()).unwrap();
            let Reply::Formatted { snapshot, result } =
                reply(&mut bridge, &ready, "first format").await
            else {
                panic!("expected formatter");
            };
            assert_eq!(snapshot.revision, first.revision);
            assert!(
                apply_text_edits(&mut store, &snapshot, Some(view), result.unwrap().unwrap())
                    .unwrap()
            );
            assert_eq!(
                store
                    .documents()
                    .snapshot(first.id)
                    .unwrap()
                    .rope
                    .to_string(),
                "formatted:文😀 "
            );
            let mut second = store.documents().snapshot(documents[1]).unwrap();
            bridge.sync(project.clone(), second.clone()).unwrap();
            let Reply::Synced { sessions, .. } = reply(&mut bridge, &ready, "second sync").await
            else {
                panic!("expected second document");
            };
            let shared_pid = sessions[0].pid.unwrap();
            let changed = store.documents().snapshot(first.id).unwrap();
            bridge.sync(project.clone(), changed.clone()).unwrap();
            let Reply::Synced { sessions, .. } = reply(&mut bridge, &ready, "changed sync").await
            else {
                panic!("expected changed mirror");
            };
            assert_eq!(sessions[0].pid, Some(shared_pid));
            bridge.saved(first.id).unwrap();
            bridge.format(changed.clone(), options()).unwrap();
            replace_selections(&mut store, view, "late ", None).unwrap();
            let Reply::Formatted { snapshot, result } =
                reply(&mut bridge, &ready, "late format").await
            else {
                panic!("expected late formatter");
            };
            let before = store.documents().snapshot(first.id).unwrap();
            assert_eq!(
                apply_text_edits(&mut store, &snapshot, Some(view), result.unwrap().unwrap()),
                Err(EditorError::StaleRevision)
            );
            assert_eq!(
                store.documents().snapshot(first.id).unwrap().rope,
                before.rope
            );
            let taide_native_editor::document::DocumentKey::File(old_path) = &second.key else {
                panic!("expected file document");
            };
            let renamed = root.join("renamed.rs");
            std::fs::rename(old_path, &renamed).unwrap();
            let canonical = std::fs::canonicalize(&renamed).unwrap();
            let reopened = taide_file::service::open_file(&canonical, &[], false).unwrap();
            store
                .retarget_file(
                    &second,
                    canonical,
                    taide_native_editor::document::DocumentMetadata::from_opened(&reopened),
                )
                .unwrap();
            second = store.documents().snapshot(second.id).unwrap();
            bridge.sync(project.clone(), second.clone()).unwrap();
            let Reply::Synced { sessions, .. } = reply(&mut bridge, &ready, "renamed sync").await
            else {
                panic!("expected renamed document mirror");
            };
            assert_eq!(sessions[0].pid, Some(shared_pid));
            bridge.format(second.clone(), options()).unwrap();
            let Reply::Formatted { snapshot, result } =
                reply(&mut bridge, &ready, "renamed format").await
            else {
                panic!("expected renamed URI formatter");
            };
            assert_eq!(snapshot.key, second.key);
            assert_eq!(result.unwrap().unwrap()[0].new_text, "formatted:文😀 ");
            bridge.retain(&HashSet::from([second.id])).unwrap();
            bridge.format(second.clone(), options()).unwrap();
            let Reply::Formatted { result, .. } =
                reply(&mut bridge, &ready, "retained format").await
            else {
                panic!("expected retained document formatter");
            };
            assert_eq!(result.unwrap().unwrap()[0].new_text, "formatted:文😀 ");
            bridge.format(first, options()).unwrap();
            let Reply::Formatted { result, .. } = reply(&mut bridge, &ready, "closed format").await
            else {
                panic!("expected closed document no-op");
            };
            assert!(result.unwrap().is_none());
            let outside = ProjectId::new();
            bridge.sync(outside, second).unwrap();
            let Reply::Failed { .. } = reply(&mut bridge, &ready, "root refusal").await else {
                panic!("expected unopened project refusal");
            };
            tokio::time::timeout(TIMEOUT, bridge.disconnect())
                .await
                .unwrap()
                .unwrap();
            services.lsp.shutdown();
            tokio::time::timeout(TIMEOUT, services.lsp.wait_for_idle())
                .await
                .unwrap();
            tokio::time::timeout(TIMEOUT, tasks.shutdown())
                .await
                .unwrap();
            assert_eq!(tasks.tracked_count(), 0);
        });
}
