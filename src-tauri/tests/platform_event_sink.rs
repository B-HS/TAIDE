use std::sync::Mutex;

use taide_lib::domain::layout::service::{default_layout, finish_mutation};
use taide_lib::paths::AppPaths;
use taide_lib::state::AppState;
use taide_model::app_event::AppEvent;
use taide_model::ids::ProjectId;
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
    let adapter_source = include_str!("../src/platform/event_sink.rs");

    assert!(normalized_app_source.contains("TauriEventSink(&app).publish(AppEvent::LayoutChanged"));
    assert!(normalized_app_source.contains("layout_service::finish_mutation(&TauriEventSink(&app)"));
    assert!(layout_source.contains("pub fn finish_mutation(sink: &dyn EventSink"));
    assert!(layout_source.contains("sink.publish(AppEvent::LayoutChanged"));
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
    let commands = include_str!("../src/domain/git/commands.rs");
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
    let adapter = include_str!("../src/platform/event_sink.rs");
    let apply = commands.split_once("pub async fn apply_and_broadcast(").unwrap().1;
    let theme = commands.split_once("pub async fn settings_set_theme(").unwrap().1;

    assert!(apply.find("service::save_settings(").unwrap() < apply.find("AppEvent::SettingsChanged").unwrap());
    assert!(apply.find("*state.settings.write()").unwrap() < apply.find("AppEvent::SettingsChanged").unwrap());
    assert!(apply.find("SettingsToggleObservers>().apply(").unwrap() < apply.find("AppEvent::SettingsChanged").unwrap());
    assert!(theme.find("apply_and_broadcast(").unwrap() < theme.find("AppEvent::ThemeChanged").unwrap());
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
