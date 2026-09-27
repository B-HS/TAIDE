use std::sync::Mutex;

use taide_lib::domain::layout::service::{default_layout, finish_mutation};
use taide_lib::paths::AppPaths;
use taide_lib::state::AppState;
use taide_model::agent::ExternalOpenRequest;
use taide_model::app_event::AppEvent;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::flush::FlushScope;
use taide_model::ide::IdeStatus;
use taide_model::ids::ProjectId;
use taide_model::lsp::{LspInstallPhase, LspServerId, LspSessionStatus};
use taide_model::project::{Project, ProjectDisplay, WindowChrome};
use taide_model::remote::RemoteStatus;
use taide_model::settings::Settings;
use taide_model::sync::SyncStatus;
use taide_runtime::EventSink;

#[derive(Default)]
struct RecordingEventSink(Mutex<Vec<AppEvent>>);

impl EventSink for RecordingEventSink {
    fn publish(&self, event: AppEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[test]
fn layout_이벤트는_tauri_없는_port로_발행된다() {
    let sink = RecordingEventSink::default();
    let event = AppEvent::LayoutChanged {
        project_id: ProjectId::from("prj-event-sink".to_string()),
        revision: 1,
    };

    sink.publish(event.clone());

    assert_eq!(sink.0.lock().unwrap().as_slice(), &[event]);
}

#[test]
fn layout_변경_완료는_주입된_port에_현재_revision을_발행한다() {
    let sink = RecordingEventSink::default();
    let state = AppState::new(AppPaths::new(std::env::temp_dir()));
    let project_id = ProjectId::from("prj-layout-event-sink".to_string());
    let mut layout = default_layout();
    layout.revision += 1;

    let snapshot = finish_mutation(&sink, &state, &project_id, &mut layout);

    assert_eq!(snapshot.revision, layout.revision);
    assert!(state.dirty_layouts.read().contains(&project_id));
    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[AppEvent::LayoutChanged {
            project_id,
            revision: snapshot.revision,
        }]
    );
}

#[test]
fn 앱과_두_layout_발행_경로는_platform_adapter를_사용한다() {
    let app_source = include_str!("../src/lib.rs");
    let production_app_source = app_source.split_once("#[cfg(test)]\nmod tests").unwrap().0;
    let normalized_app_source = production_app_source.split_whitespace().collect::<Vec<_>>().join("");
    let layout_source = include_str!("../src/domain/layout/service.rs");
    let layout_actions = include_str!("../../crates/taide-runtime/src/layout_actions.rs");
    let adapter_source = include_str!("../src/platform/event_sink.rs");

    assert!(normalized_app_source.contains("layout_actions::layout_move_tab_to_window(&TauriEventSink(&app),&state,&windows,"));
    assert!(normalized_app_source
        .contains("layout_actions::return_auxiliary_window_tabs(&TauriEventSink(&app),&state,project_id,window_slot).await"));
    let move_action = layout_actions
        .split_once("pub async fn layout_move_tab_to_window<")
        .unwrap()
        .1
        .split_once("pub async fn return_auxiliary_window_tabs(")
        .unwrap()
        .0;
    let return_action = layout_actions
        .split_once("pub async fn return_auxiliary_window_tabs(")
        .unwrap()
        .1
        .split_once("pub async fn layout_get(")
        .unwrap()
        .0;
    assert!(move_action.contains("finish_mutation(sink, state, &project_id, layout)"));
    assert!(return_action.contains("sink.publish(AppEvent::LayoutChanged { project_id, revision })"));
    assert!(layout_source.contains("pub use taide_runtime::layout_actions::finish_mutation"));
    assert!(layout_actions.contains("pub fn finish_mutation(sink: &dyn EventSink"));
    assert!(layout_actions.contains("sink.publish(AppEvent::LayoutChanged"));
    assert!(adapter_source.contains("LayoutChanged { project_id, revision }.emit(self.0)"));
}

#[test]
fn git_status와_refs_이벤트는_같은_port에서_순서대로_발행된다() {
    let sink = RecordingEventSink::default();
    let project_id = ProjectId::from("prj-git-event-sink".to_string());

    sink.publish(AppEvent::GitStatusChanged {
        project_id: project_id.clone(),
    });
    sink.publish(AppEvent::GitRefsChanged {
        project_id: project_id.clone(),
    });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::GitStatusChanged {
                project_id: project_id.clone(),
            },
            AppEvent::GitRefsChanged { project_id },
        ]
    );
}

#[test]
fn git_명령과_워처는_캐시_무효화_후_port에_발행한다() {
    let commands = include_str!("../../crates/taide-runtime/src/git_actions.rs");
    let watcher = include_str!("../src/domain/git/watch.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let status_command = commands.split_once("fn emit_status_changed(").unwrap().1;
    let refs_command = commands.split_once("fn emit_refs_changed(").unwrap().1;
    let watcher_callback = watcher.split_once("move |notification| {").unwrap().1;

    assert!(status_command.find("invalidate_status(project_id)").unwrap() < status_command.find("AppEvent::GitStatusChanged").unwrap());
    assert!(refs_command.find("invalidate_status(project_id)").unwrap() < refs_command.find("AppEvent::GitRefsChanged").unwrap());
    assert!(
        watcher_callback.find("invalidate_status(&emit_project)").unwrap() < watcher_callback.find("AppEvent::GitStatusChanged").unwrap()
    );
    assert!(watcher_callback.find("AppEvent::GitStatusChanged").unwrap() < watcher_callback.find("AppEvent::GitRefsChanged").unwrap());
    assert!(adapter.contains("GitStatusChanged { project_id }.emit(self.0)"));
    assert!(adapter.contains("GitRefsChanged { project_id }.emit(self.0)"));
}

#[test]
fn terminal_세션_이벤트_네_종은_같은_port에서_발행된다() {
    let sink = RecordingEventSink::default();
    let project_id = ProjectId::from("prj-terminal-event-sink".to_string());

    sink.publish(AppEvent::TerminalSpawned {
        session_id: "term-1".to_string(),
        project_id: project_id.clone(),
        cwd: "/repo".to_string(),
        shell: "/bin/sh".to_string(),
    });
    sink.publish(AppEvent::TerminalCwdChanged {
        session_id: "term-1".to_string(),
        cwd: "/repo/src".to_string(),
    });
    sink.publish(AppEvent::TerminalCommandFinished {
        session_id: "term-1".to_string(),
        cwd: Some("/repo/src".to_string()),
        exit_code: Some(0),
        duration_ms: 1,
    });
    sink.publish(AppEvent::TerminalExited {
        session_id: "term-1".to_string(),
        code: Some(0),
    });

    let recorded = sink.0.lock().unwrap();
    assert_eq!(recorded.len(), 4);
    assert!(matches!(&recorded[0], AppEvent::TerminalSpawned { project_id: recorded_project, .. } if recorded_project == &project_id));
    assert!(matches!(&recorded[1], AppEvent::TerminalCwdChanged { .. }));
    assert!(matches!(&recorded[2], AppEvent::TerminalCommandFinished { .. }));
    assert!(matches!(&recorded[3], AppEvent::TerminalExited { .. }));
}

#[test]
fn terminal_발행은_상태_갱신과_명령_측정_뒤에_수행된다() {
    let commands = include_str!("../src/domain/terminal/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let cwd = commands.split_once("fn report_cwd_change(").unwrap().1;
    let marker = commands.split_once("fn report_command_marker(").unwrap().1;
    let exit = commands.split_once("move |code| {").unwrap().1;
    let spawned = commands.split_once("let spawned = AppEvent::TerminalSpawned").unwrap().1;

    assert!(cwd.find("store.update_cwd(").unwrap() < cwd.find("AppEvent::TerminalCwdChanged").unwrap());
    assert!(marker.find("command_clock.record(").unwrap() < marker.find("AppEvent::TerminalCommandFinished").unwrap());
    assert!(exit.find("exit_metadata.mark_exited()").unwrap() < exit.find("AppEvent::TerminalExited").unwrap());
    assert!(spawned.find("store.insert(").unwrap() < spawned.find(".publish(spawned)").unwrap());
    assert!(adapter.contains("TerminalSpawned {") && adapter.contains("TerminalExited {") && adapter.contains("TerminalCwdChanged {"));
    assert!(adapter.contains("TerminalCommandFinished {"));
}

#[test]
fn 설정과_테마_이벤트는_같은_port에서_순서대로_발행된다() {
    let sink = RecordingEventSink::default();
    let settings = Settings::default();
    let theme_id = settings.theme_id.clone();

    sink.publish(AppEvent::SettingsChanged {
        settings: Box::new(settings.clone()),
    });
    sink.publish(AppEvent::ThemeChanged {
        theme_id: theme_id.clone(),
    });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::SettingsChanged {
                settings: Box::new(settings),
            },
            AppEvent::ThemeChanged { theme_id },
        ]
    );
}

#[test]
fn 설정_적용과_테마_변경은_상태_갱신_뒤_port로_발행된다() {
    let commands = include_str!("../src/domain/settings/commands.rs");
    let runtime = include_str!("../../crates/taide-runtime/src/settings_actions.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let apply = runtime.split_once("pub async fn apply_and_broadcast").unwrap().1;
    let theme = runtime.split_once("pub async fn settings_set_theme").unwrap().1;

    assert!(apply.find("service::save_settings(").unwrap() < apply.find("AppEvent::SettingsChanged").unwrap());
    assert!(apply.find("*state.settings.write()").unwrap() < apply.find("AppEvent::SettingsChanged").unwrap());
    assert!(
        apply.find("reconcile_integrations(current, updated.clone()).await").unwrap() < apply.find("AppEvent::SettingsChanged").unwrap()
    );
    assert!(theme.find("apply_and_broadcast(").unwrap() < theme.find("AppEvent::ThemeChanged").unwrap());
    assert!(commands.contains("SettingsToggleObservers>().apply(app, &current, &updated).await"));
    assert!(commands.contains("settings_actions::apply_and_broadcast("));
    assert!(commands.contains("settings_actions::settings_get("));
    assert!(commands.contains("settings_actions::settings_update("));
    assert!(commands.contains("settings_actions::settings_set_theme("));
    assert!(!commands.contains("service::save_settings("));
    assert!(!commands.contains("AppEvent::"));
    assert!(adapter.contains("SettingsChanged { settings: *settings }.emit(self.0)"));
    assert!(adapter.contains("ThemeChanged { theme_id }.emit(self.0)"));
}

#[test]
fn 동기화_상태_이벤트는_tauri_없는_port로_발행된다() {
    let sink = RecordingEventSink::default();
    let status = SyncStatus {
        connected: true,
        has_gist: true,
        last_synced_at: None,
        remote_newer: Some(false),
    };

    sink.publish(AppEvent::SyncStateChanged { status: status.clone() });

    assert_eq!(sink.0.lock().unwrap().as_slice(), &[AppEvent::SyncStateChanged { status }]);
}

#[test]
fn 동기화_성공_경로_네_곳은_상태_반영_뒤_port로_발행된다() {
    let commands = include_str!("../src/domain/sync/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let connect = commands.split_once("pub async fn sync_connect(").unwrap().1;
    let disconnect = commands.split_once("pub async fn sync_disconnect(").unwrap().1;
    let upload = commands.split_once("pub async fn sync_upload(").unwrap().1;
    let download = commands.split_once("pub async fn sync_download(").unwrap().1;

    assert_eq!(commands.matches(".publish(AppEvent::SyncStateChanged").count(), 4);
    assert!(connect.find("*state.settings.write()").unwrap() < connect.find("AppEvent::SyncStateChanged").unwrap());
    assert!(disconnect.find("*state.settings.write()").unwrap() < disconnect.find("AppEvent::SyncStateChanged").unwrap());
    assert!(upload.find("*state.settings.write()").unwrap() < upload.find("AppEvent::SyncStateChanged").unwrap());
    assert!(download.find("service::apply_locale_entries(").unwrap() < download.find("AppEvent::SyncStateChanged").unwrap());
    assert!(adapter.contains("SyncStateChanged { status }.emit(self.0)"));
}

#[test]
fn 원격_서버_상태는_같은_port에서_시작과_중지_순서로_발행된다() {
    let sink = RecordingEventSink::default();
    let running = RemoteStatus {
        running: true,
        port: 1,
        client_count: 0,
        password_configured: true,
    };

    sink.publish(AppEvent::RemoteStateChanged { status: running });
    sink.publish(AppEvent::RemoteStateChanged {
        status: RemoteStatus::default(),
    });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::RemoteStateChanged { status: running },
            AppEvent::RemoteStateChanged {
                status: RemoteStatus::default(),
            },
        ]
    );
}

#[test]
fn 원격_서버_시작과_중지는_수명주기_갱신_뒤_port로_발행된다() {
    let commands = include_str!("../src/domain/remote/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let start = commands.split_once("async fn bind_and_start(").unwrap().1;
    let stop = commands.split_once("pub fn stop_server(").unwrap().1;

    assert_eq!(commands.matches(".publish(AppEvent::RemoteStateChanged").count(), 2);
    assert!(start.find("remote.mark_started(").unwrap() < start.find("AppEvent::RemoteStateChanged").unwrap());
    assert!(stop.find("remote.take_shutdown_state()").unwrap() < stop.find("AppEvent::RemoteStateChanged").unwrap());
    assert!(stop.find("shutdown_tx.send(())").unwrap() < stop.find("AppEvent::RemoteStateChanged").unwrap());
    assert!(adapter.contains("RemoteStateChanged { status }.emit(self.0)"));
}

#[test]
fn 창_chrome_이벤트는_tauri_없는_port로_발행된다() {
    let sink = RecordingEventSink::default();
    let chrome = WindowChrome::default();

    sink.publish(AppEvent::WindowChromeChanged { chrome });

    assert_eq!(sink.0.lock().unwrap().as_slice(), &[AppEvent::WindowChromeChanged { chrome }]);
}

#[test]
fn 창_chrome_이벤트는_세션_저장과_guard_해제_뒤에_발행된다() {
    let commands = include_str!("../src/domain/project/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let set_chrome = commands.split_once("pub async fn session_set_window_chrome(").unwrap().1;

    assert!(set_chrome.find("*state.session.write() = session").unwrap() < set_chrome.find("AppEvent::WindowChromeChanged").unwrap());
    assert!(set_chrome.find("drop(_guard)").unwrap() < set_chrome.find("AppEvent::WindowChromeChanged").unwrap());
    assert!(adapter.contains("WindowChromeChanged { chrome }.emit(self.0)"));
}

#[test]
fn 프로젝트_목록_그룹_슬롯_snapshot은_같은_port에서_발행된다() {
    let sink = RecordingEventSink::default();

    sink.publish(AppEvent::ProjectListChanged { projects: Vec::new() });
    sink.publish(AppEvent::ProjectGroupsChanged { groups: Vec::new() });
    sink.publish(AppEvent::SessionShellSlotsChanged { tree: None, focused: None });

    let recorded = sink.0.lock().unwrap();
    assert!(matches!(&recorded[0], AppEvent::ProjectListChanged { projects } if projects.is_empty()));
    assert!(matches!(&recorded[1], AppEvent::ProjectGroupsChanged { groups } if groups.is_empty()));
    assert!(matches!(
        &recorded[2],
        AppEvent::SessionShellSlotsChanged { tree: None, focused: None }
    ));
}

#[test]
fn 프로젝트_목록_그룹_슬롯은_snapshot을_만든_뒤_port로_발행된다() {
    let commands = include_str!("../src/domain/project/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let list = commands.split_once("fn emit_list_changed(").unwrap().1;
    let slots = commands.split_once("fn emit_shell_slots_changed(").unwrap().1;
    let groups = commands.split_once("fn emit_groups_changed(").unwrap().1;

    assert!(list.find("service::list_projects(").unwrap() < list.find("AppEvent::ProjectListChanged").unwrap());
    assert!(slots.find("let session = state.session.read()").unwrap() < slots.find("AppEvent::SessionShellSlotsChanged").unwrap());
    assert!(slots.find("};").unwrap() < slots.find(".publish(payload)").unwrap());
    assert!(groups.find("service::list_groups(").unwrap() < groups.find("AppEvent::ProjectGroupsChanged").unwrap());
    assert!(adapter.contains("ProjectListChanged { projects }.emit(self.0)"));
    assert!(adapter.contains("ProjectGroupsChanged { groups }.emit(self.0)"));
    assert!(adapter.contains("SessionShellSlotsChanged { tree, focused }.emit(self.0)"));
}

#[test]
fn 프로젝트_수명주기와_최근_정리_이벤트는_같은_port에서_발행된다() {
    let sink = RecordingEventSink::default();
    let project_id = ProjectId::from("prj-project-event-sink".to_string());
    let project = Project {
        id: project_id.clone(),
        root: "/repo".to_string(),
        name: "repo".to_string(),
        capabilities: Vec::new(),
        root_missing: false,
        last_opened_at: 0.0,
        display: ProjectDisplay::default(),
    };

    sink.publish(AppEvent::ProjectOpened {
        project: Box::new(project.clone()),
    });
    sink.publish(AppEvent::ProjectActivated {
        project_id: Some(project_id.clone()),
    });
    sink.publish(AppEvent::ProjectClosed {
        project_id: project_id.clone(),
    });
    sink.publish(AppEvent::ProjectRecentCleared {
        removed: 1,
        skipped_with_drafts: 0,
    });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::ProjectOpened {
                project: Box::new(project),
            },
            AppEvent::ProjectActivated {
                project_id: Some(project_id.clone()),
            },
            AppEvent::ProjectClosed { project_id },
            AppEvent::ProjectRecentCleared {
                removed: 1,
                skipped_with_drafts: 0,
            },
        ]
    );
}

#[test]
fn 프로젝트_수명주기_발행은_기존_성공_경로와_순서를_유지한다() {
    let commands = include_str!("../src/domain/project/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let recent = commands.split_once("pub async fn project_forget_recent(").unwrap().1;
    let open = commands.split_once("pub async fn project_open(").unwrap().1;
    let close = commands.split_once("pub async fn project_close(").unwrap().1;

    assert_eq!(commands.matches(".publish(AppEvent::ProjectOpened").count(), 3);
    assert_eq!(commands.matches(".publish(AppEvent::ProjectActivated").count(), 7);
    assert_eq!(commands.matches(".publish(AppEvent::ProjectClosed").count(), 1);
    assert_eq!(commands.matches(".publish(AppEvent::ProjectRecentCleared").count(), 1);
    assert!(open.find("attach_project_capabilities(").unwrap() < open.find("AppEvent::ProjectOpened").unwrap());
    assert!(close.find("detach_all(").unwrap() < close.find("AppEvent::ProjectClosed").unwrap());
    assert!(close.find("AppEvent::ProjectClosed").unwrap() < close.find("AppEvent::ProjectActivated").unwrap());
    assert!(recent.find("emit_list_changed(").unwrap() < recent.find("AppEvent::ProjectRecentCleared").unwrap());
    assert!(adapter.contains("ProjectOpened { project: *project }.emit(self.0)"));
    assert!(adapter.contains("ProjectClosed { project_id }.emit(self.0)"));
    assert!(adapter.contains("ProjectActivated { project_id }.emit(self.0)"));
    let recent_adapter = adapter.split_once("let _ = ProjectRecentCleared {").unwrap().1;
    let recent_payload = recent_adapter.split_once(".emit(self.0)").unwrap().0;
    assert!(recent_payload.contains("removed,") && recent_payload.contains("skipped_with_drafts,"));
}

#[test]
fn 파일_변경과_재스캔_이벤트는_같은_port에서_발행된다() {
    let sink = RecordingEventSink::default();
    let project_id = ProjectId::from("prj-file-event-sink".to_string());
    let change = FsChange {
        kind: FsChangeKind::Modified,
        paths: vec!["/repo/src".to_string()],
        from_app: false,
    };

    sink.publish(AppEvent::FsChanged {
        project_id: project_id.clone(),
        change: change.clone(),
    });
    sink.publish(AppEvent::FsRescanRequired {
        project_id: project_id.clone(),
    });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::FsChanged {
                project_id: project_id.clone(),
                change,
            },
            AppEvent::FsRescanRequired { project_id },
        ]
    );
}

#[test]
fn 파일_watcher와_복원은_기존_순서로_port에_발행된다() {
    let watcher = include_str!("../src/domain/file/capability.rs");
    let project = include_str!("../src/domain/project/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let callback = watcher.split_once("move |notification| match notification {").unwrap().1;
    let attach = project.split_once("async fn attach_project_capabilities(").unwrap().1;
    let restore = project.split_once("pub(crate) fn restore_project_watchers(").unwrap().1;

    assert!(callback.find("WatchNotification::RescanRequired").unwrap() < callback.find("AppEvent::FsRescanRequired").unwrap());
    assert!(callback.find("resolve_from_app(").unwrap() < callback.find("AppEvent::FsChanged").unwrap());
    assert!(restore.find("drop(_guard)").unwrap() < restore.find("AppEvent::FsChanged").unwrap());
    assert!(restore.find("AppEvent::FsChanged").unwrap() < restore.find("AppEvent::GitStatusChanged").unwrap());
    assert!(attach.find("commit_attachments(").unwrap() < attach.find("AppEvent::GitStatusChanged").unwrap());
    assert!(adapter.contains("FsChanged { project_id, change }.emit(self.0)"));
    assert!(adapter.contains("FsRescanRequired { project_id }.emit(self.0)"));
}

#[test]
fn lsp_상태와_설치_진행은_같은_port에서_payload를_보존한다() {
    let sink = RecordingEventSink::default();
    let server_id = LspServerId("rust-analyzer".to_string());

    sink.publish(AppEvent::LspSessionStatusChanged {
        session_id: "lsp-1".to_string(),
        status: LspSessionStatus::Crashed,
        last_error: Some("exit".to_string()),
        generation: 1,
    });
    sink.publish(AppEvent::LspInstallProgress {
        server_id: server_id.clone(),
        phase: LspInstallPhase::Downloading,
        received_bytes: 1.0,
        total_bytes: Some(2.0),
        message: Some("download".to_string()),
    });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::LspSessionStatusChanged {
                session_id: "lsp-1".to_string(),
                status: LspSessionStatus::Crashed,
                last_error: Some("exit".to_string()),
                generation: 1,
            },
            AppEvent::LspInstallProgress {
                server_id,
                phase: LspInstallPhase::Downloading,
                received_bytes: 1.0,
                total_bytes: Some(2.0),
                message: Some("download".to_string()),
            },
        ]
    );
}

#[test]
fn lsp_helper는_snapshot과_byte_변환_뒤_adapter로_발행한다() {
    let commands = include_str!("../src/domain/lsp/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let status = commands.split_once("fn emit_status(").unwrap().1;
    let set_status = commands.split_once("fn set_status(").unwrap().1;
    let actions = include_str!("../../crates/taide-runtime/src/lsp_install_actions.rs");
    let install = actions.split_once("fn emit_install_progress(").unwrap().1;

    assert!(status.contains("AppEvent::LspSessionStatusChanged"));
    assert!(set_status.contains("emit_status(app, session_id, entry.lifecycle.set_status(status, last_error))"));
    assert!(install.contains("AppEvent::LspInstallProgress"));
    assert!(install.contains("received_bytes: received_bytes as f64"));
    assert!(install.contains("total_bytes: total_bytes.map(|value| value as f64)"));
    assert!(install.contains("events.publish(AppEvent::LspInstallProgress"));
    assert!(commands.contains("taide_runtime::lsp_install_toolchain::run_toolchain_install("));
    assert!(adapter.contains("LspSessionStatusChanged {") && adapter.contains("generation,"));
    assert!(adapter.contains("LspInstallProgress {") && adapter.contains("received_bytes,"));
}

#[test]
fn ide_상태와_요청_네_종은_같은_port에서_payload를_보존한다() {
    let sink = RecordingEventSink::default();
    let project_id = ProjectId::from("prj-ide-event-sink".to_string());
    let status = IdeStatus {
        running: true,
        port: 1,
        connected: true,
        client_count: 1,
    };

    sink.publish(AppEvent::IdeStatusChanged { status });
    sink.publish(AppEvent::IdeDiffRequested {
        request_id: "diff-1".to_string(),
        project_id: project_id.clone(),
        old_path: "/repo/old".to_string(),
        new_path: "/repo/new".to_string(),
        new_contents: "new contents".to_string(),
        tab_name: "Diff".to_string(),
    });
    sink.publish(AppEvent::IdeSaveRequested {
        request_id: "save-1".to_string(),
        project_id: project_id.clone(),
        path: "/repo/new".to_string(),
    });
    sink.publish(AppEvent::IdeCloseTabRequested {
        tab_name: "Diff".to_string(),
        request_id: Some("diff-1".to_string()),
    });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::IdeStatusChanged { status },
            AppEvent::IdeDiffRequested {
                request_id: "diff-1".to_string(),
                project_id: project_id.clone(),
                old_path: "/repo/old".to_string(),
                new_path: "/repo/new".to_string(),
                new_contents: "new contents".to_string(),
                tab_name: "Diff".to_string(),
            },
            AppEvent::IdeSaveRequested {
                request_id: "save-1".to_string(),
                project_id,
                path: "/repo/new".to_string(),
            },
            AppEvent::IdeCloseTabRequested {
                tab_name: "Diff".to_string(),
                request_id: Some("diff-1".to_string()),
            },
        ]
    );
}

#[test]
fn ide_명령과_mcp_server는_기존_조건_뒤에_port로_발행한다() {
    let commands = include_str!("../src/domain/ide/commands.rs");
    let server = include_str!("../src/domain/ide/server.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let stop = commands.split_once("pub fn stop_server(").unwrap().1;
    let start = commands.split_once("async fn bind_and_start(").unwrap().1;
    let diff = server.split_once("async fn tool_open_diff(").unwrap().1;
    let save = server.split_once("async fn tool_save_document(").unwrap().1;
    let close = server.split_once("async fn tool_close_tab(").unwrap().1;
    let close_all = server.split_once("async fn tool_close_all_diff_tabs(").unwrap().1;

    assert!(stop.find("ide.take_shutdown_state()").unwrap() < stop.find("AppEvent::IdeStatusChanged").unwrap());
    assert!(start.find("ide.mark_started(").unwrap() < start.find("AppEvent::IdeStatusChanged").unwrap());
    assert!(diff.find("insert_pending_diff(").unwrap() < diff.find("AppEvent::IdeDiffRequested").unwrap());
    assert!(save.find("insert_pending_save(").unwrap() < save.find("AppEvent::IdeSaveRequested").unwrap());
    assert!(close.find(".close_tab)(").unwrap() < close.find("AppEvent::IdeCloseTabRequested").unwrap());
    assert!(close_all.find(".close_tab)(").unwrap() < close_all.find("AppEvent::IdeCloseTabRequested").unwrap());
    assert!(adapter.contains("IdeStatusChanged { status }.emit(self.0)"));
    assert!(adapter.contains("IdeDiffRequested {") && adapter.contains("new_contents,"));
    assert!(adapter.contains("IdeSaveRequested {") && adapter.contains("request_id,"));
    assert!(adapter.contains("IdeCloseTabRequested { tab_name, request_id }.emit(self.0)"));
}

#[test]
fn agent_상태와_외부_열기는_같은_port에서_payload를_보존한다() {
    let sink = RecordingEventSink::default();
    let project_id = ProjectId::from("prj-agent-event-sink".to_string());
    let request = ExternalOpenRequest {
        path: "/repo/file".to_string(),
        wait_marker: Some("marker-1".to_string()),
    };

    sink.publish(AppEvent::AgentStateChanged {
        project_id: project_id.clone(),
        agents: Vec::new(),
    });
    sink.publish(AppEvent::AgentExternalOpen { request: request.clone() });

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &[
            AppEvent::AgentStateChanged {
                project_id,
                agents: Vec::new(),
            },
            AppEvent::AgentExternalOpen { request },
        ]
    );
}

#[test]
fn agent_diff와_외부_열기_queue는_기존_조건_뒤에_port로_발행한다() {
    let hooks = include_str!("../src/domain/agent/hooks.rs");
    let commands = include_str!("../src/domain/agent/commands.rs");
    let app = include_str!("../src/lib.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let hook = hooks
        .split_once("if let Some(changed) = agents.diff(&project_id, &updated) {")
        .unwrap()
        .1;
    let poll = commands
        .split_once("if let Some(changed) = agents.diff(&project_id, &detected) {")
        .unwrap()
        .1;
    let single_instance = app.split_once("tauri_plugin_single_instance::init(").unwrap().1;

    assert!(hook.contains("AppEvent::AgentStateChanged"));
    assert!(poll.contains("AppEvent::AgentStateChanged"));
    assert!(
        single_instance.find("queue_external_open(app_handle,").unwrap() < single_instance.find("AppEvent::AgentExternalOpen").unwrap()
    );
    assert!(commands.contains("queue_external_open(app_handle, request)"));
    assert!(adapter.contains("AgentStateChanged { project_id, agents }.emit(self.0)"));
    assert!(adapter.contains("AgentExternalOpen { request }.emit(self.0)"));
}

#[test]
fn hot_exit_요청은_세_scope를_같은_port에서_발행한다() {
    let sink = RecordingEventSink::default();
    let project_id = ProjectId::from("prj-hot-exit-event-sink".to_string());
    let scopes = [
        FlushScope::All,
        FlushScope::Window("editor-1".to_string()),
        FlushScope::Project(project_id),
    ];

    for scope in &scopes {
        sink.publish(AppEvent::HotExitFlushRequested {
            timeout_ms: 1.0,
            scope: scope.clone(),
        });
    }

    assert_eq!(
        sink.0.lock().unwrap().as_slice(),
        &scopes.map(|scope| AppEvent::HotExitFlushRequested { timeout_ms: 1.0, scope })
    );
}

#[test]
fn hot_exit_요청은_handshake_시작_뒤_발행하고_원격에_전파하지_않는다() {
    let window = include_str!("../src/domain/window/commands.rs");
    let project = include_str!("../src/domain/project/commands.rs");
    let adapter = include_str!("../src/platform/event_sink.rs");
    let auxiliary = window.split_once("fn handle_auxiliary_close_requested(").unwrap().1;
    let main = window.split_once("state.begin_hot_exit_flush(expected_windows)").unwrap().1;
    let project_flush = project.split_once("async fn await_project_flush(").unwrap().1;

    assert!(auxiliary.find("state.begin_flush(").unwrap() < auxiliary.find("AppEvent::HotExitFlushRequested").unwrap());
    assert!(main.contains("AppEvent::HotExitFlushRequested"));
    assert!(project_flush.find("state.begin_flush(").unwrap() < project_flush.find("AppEvent::HotExitFlushRequested").unwrap());
    assert!(window.contains("scope: FlushScope::All"));
    assert!(window.contains("scope: scope.clone()"));
    assert!(project.contains("scope: scope.clone()"));
    assert!(adapter.contains("HotExitFlushRequested { timeout_ms, scope }.emit(self.0)"));
}
