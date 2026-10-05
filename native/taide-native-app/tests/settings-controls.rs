use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use taide_model::settings::Settings;
use taide_model::{
    app_event::AppEvent,
    error::{AppError, AppErrorKind},
    ids::ProjectId,
    paths::AppPaths,
};
use taide_native_app::host::{HostBridge, HostCommand, HostReply};
use taide_native_app::settings_controls::{Change, Numeric, NumericDraft, Position, Switch};
use taide_runtime::{AppState, EventSink, TaskSupervisor};
use tokio::sync::Notify;

const DEADLINE: Duration = Duration::from_secs(5);
const EDITOR_FONT: u32 = 17;
const TERMINAL_FONT: u32 = 18;

#[test]
fn native_settings_controls는_단일필드와_numeric저장값을_보존한다() {
    let initial = Settings::default();
    for switch in Switch::ALL {
        let changed = !switch.value(&initial);
        let patch = Change::Switch(switch, changed).patch().unwrap();
        let json = serde_json::to_value(&patch).unwrap();
        assert_eq!(
            json.as_object()
                .unwrap()
                .values()
                .filter(|value| !value.is_null())
                .count(),
            1
        );
        let mut applied = serde_json::to_value(&initial).unwrap();
        for (field, value) in json.as_object().unwrap() {
            if !value.is_null() {
                applied[field] = value.clone();
            }
        }
        let applied: Settings = serde_json::from_value(applied).unwrap();
        assert_eq!(switch.value(&applied), changed);
        assert_eq!(applied.theme_id, initial.theme_id);
        assert_eq!(applied.language, initial.language);
        assert_eq!(applied.agent_hooks_enabled, initial.agent_hooks_enabled);
        assert_eq!(
            applied.ide_integration_enabled,
            initial.ide_integration_enabled
        );
    }
    for position in Position::ALL {
        let patch = Change::Position(position).patch().unwrap();
        assert_eq!(patch.toast_position.as_deref(), Some(position.value()));
    }
    let field = Numeric::ResizerThickness;
    assert_eq!(field.bounds(), (0, 8));
    assert!(field.commit(f64::NAN, 1).unwrap().is_none());
    assert!(field.commit(1.0, 1).unwrap().is_none());
    assert_eq!(
        field
            .commit(999.0, 1)
            .unwrap()
            .unwrap()
            .patch()
            .unwrap()
            .resizer_thickness,
        Some(8)
    );
    assert!(field.commit(1.5, 1).is_err());
    let mut draft = NumericDraft::new(1);
    draft.text = "999".into();
    let patch = draft.commit(field, 1).unwrap().unwrap().patch().unwrap();
    assert_eq!(patch.resizer_thickness, Some(8));
    assert_eq!(draft.text, "1");
    draft.sync(8);
    assert_eq!(draft.text, "8");
    draft.text = "3".into();
    draft.sync(8);
    assert_eq!(draft.text, "3");
    draft.sync(4);
    assert_eq!(draft.text, "4");
    for raw in [
        "+2", "2.", "2.e2", " 2", "2 ", "inf", "NaN", "0x2", "1e9999", "1e", "--1", "2.0.0",
    ] {
        draft.text = raw.into();
        assert!(draft.commit(field, 4).unwrap().is_none(), "{raw}");
        assert_eq!(draft.text, "4");
    }
    for raw in ["2", "2.0", "2e0", "20E-1", "2e+0"] {
        draft.text = raw.into();
        assert_eq!(
            draft
                .commit(field, 4)
                .unwrap()
                .unwrap()
                .patch()
                .unwrap()
                .resizer_thickness,
            Some(2)
        );
    }
    draft.text = ".5".into();
    assert!(draft.commit(field, 4).is_err());
    assert_eq!(draft.text, "4");
    draft.text = "invalid".into();
    assert!(draft.commit(field, 4).unwrap().is_none());
    assert_eq!(draft.text, "4");
    let debounce = Numeric::SearchOnTypeDebounce;
    assert_eq!(debounce.bounds(), (50, 2000));
    assert_eq!(
        debounce
            .commit(-1.0, 300)
            .unwrap()
            .unwrap()
            .patch()
            .unwrap()
            .search_on_type_debounce_ms,
        Some(50)
    );
    assert!(Change::Theme("taide-light".into()).patch().is_none());
}

#[test]
fn native_settings_controls_host는_저장과_theme이벤트_오류원본을_보존한다() {
    #[derive(Default)]
    struct Events(Mutex<Vec<AppEvent>>);
    impl EventSink for Events {
        fn publish(&self, event: AppEvent) {
            self.0.lock().unwrap().push(event);
        }
    }
    let directory = std::env::temp_dir().join(format!(
        "taide-native-settings-controls-{}",
        ProjectId::new()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let events = Arc::new(Events::default());
    let services =
        taide_native_app::bootstrap::services(state.clone(), tasks.clone(), events.clone());
    let ready = Arc::new(Notify::new());
    let signal = ready.clone();
    let reconciled = Arc::new(Mutex::new(Vec::new()));
    let applied = reconciled.clone();
    let apply_events = events.clone();
    let mut bridge = HostBridge::connect_with_settings_ports(
        services.clone(),
        Arc::new(move || signal.notify_one()),
        Arc::new(|_| panic!("unexpected synthetic clipboard write")),
        Arc::new(|| panic!("unexpected synthetic clipboard read")),
        None,
        Arc::new(move |services, current, updated| {
            let applied = applied.clone();
            let events = apply_events.clone();
            Box::pin(async move {
                assert_eq!(*services.state.settings.read(), updated);
                let stored: Settings = serde_json::from_slice(
                    &std::fs::read(services.state.paths.settings_file()).unwrap(),
                )
                .unwrap();
                assert_eq!(stored, updated);
                let before = events.0.lock().unwrap().len();
                tokio::task::yield_now().await;
                assert_eq!(events.0.lock().unwrap().len(), before);
                applied.lock().unwrap().push((current, updated));
            })
        }),
    )
    .unwrap();
    runtime.block_on(async {
        let initial = state.settings.read().clone();
        for switch in Switch::ALL {
            let before = state.settings.read().clone();
            let count = events.0.lock().unwrap().len();
            bridge.submit(HostCommand::UpdateSettings(Change::Switch(switch, !switch.value(&before)))).unwrap();
            tokio::time::timeout(DEADLINE, ready.notified()).await.unwrap();
            assert!(bridge.poll().is_none());
            assert_eq!(switch.value(&state.settings.read()), !switch.value(&before));
            assert_eq!(reconciled.lock().unwrap().last(), Some(&(before, state.settings.read().clone())));
            let emitted = events.0.lock().unwrap();
            assert_eq!(emitted.len(), count + 1);
            assert!(matches!(&emitted[count], AppEvent::SettingsChanged { settings } if settings.as_ref() == &*state.settings.read()));
        }
        bridge.submit(HostCommand::UpdateSettings(Change::Theme("taide-light".into()))).unwrap();
        tokio::time::timeout(DEADLINE, ready.notified()).await.unwrap();
        assert!(bridge.poll().is_none());
        assert_eq!(state.settings.read().theme_id, "taide-light");
        assert!(!state.settings.read().follow_system_theme);
        assert_eq!(reconciled.lock().unwrap().last().unwrap().1.theme_id, "taide-light");
        {
            let emitted = events.0.lock().unwrap();
            assert!(matches!(&emitted[emitted.len() - 2], AppEvent::SettingsChanged { .. }));
            assert!(matches!(&emitted[emitted.len() - 1], AppEvent::ThemeChanged { theme_id } if theme_id == "taide-light"));
        }
        for change in [Change::Language("ja".into()), Change::Position(Position::ALL[0]), Change::Numeric(Numeric::ResizerThickness, u32::MAX), Change::Numeric(Numeric::SearchOnTypeDebounce, 0)] {
            bridge.submit(HostCommand::UpdateSettings(change)).unwrap();
            tokio::time::timeout(DEADLINE, ready.notified()).await.unwrap();
            assert!(bridge.poll().is_none());
        }
        assert_eq!(state.settings.read().language, "ja");
        assert_eq!(state.settings.read().toast_position, "top-left");
        assert_eq!(state.settings.read().resizer_thickness, 8);
        assert_eq!(state.settings.read().search_on_type_debounce_ms, 50);
        assert_eq!(state.settings.read().agent_hooks_enabled, initial.agent_hooks_enabled);
        assert_eq!(state.settings.read().ide_integration_enabled, initial.ide_integration_enabled);
        for command in [
            HostCommand::SetEditorFontSize(EDITOR_FONT),
            HostCommand::SetTerminalFontSize(TERMINAL_FONT),
            HostCommand::SetKeymapOverrides(taide_native_app::keymap::catalog::Overrides::default()),
        ] {
            let before = state.settings.read().clone();
            let count = reconciled.lock().unwrap().len();
            bridge.submit(command).unwrap();
            tokio::time::timeout(DEADLINE, ready.notified()).await.unwrap();
            assert!(bridge.poll().is_none());
            let applied = reconciled.lock().unwrap();
            assert_eq!(applied.len(), count + 1);
            assert_eq!(applied.last(), Some(&(before, state.settings.read().clone())));
        }
        assert_eq!(state.settings.read().editor_font_size, EDITOR_FONT);
        assert_eq!(state.settings.read().terminal_font_size, TERMINAL_FONT);
        assert_eq!(state.settings.read().keymap_overrides.as_deref(), Some("[]"));
        let settings_events = events.0.lock().unwrap().iter().filter(|event| matches!(event, AppEvent::SettingsChanged { .. })).count();
        assert_eq!(reconciled.lock().unwrap().len(), settings_events);
        let stored: Settings = serde_json::from_slice(&std::fs::read(state.paths.settings_file()).unwrap()).unwrap();
        assert_eq!(stored, *state.settings.read());
        for change in [Change::Theme("../outside".into()), Change::Language("../outside".into())] {
            let before = state.settings.read().clone();
            let count = events.0.lock().unwrap().len();
            let reconcile_count = reconciled.lock().unwrap().len();
            bridge.submit(HostCommand::UpdateSettings(change)).unwrap();
            tokio::time::timeout(DEADLINE, ready.notified()).await.unwrap();
            assert!(matches!(bridge.poll(), Some(HostReply::SettingsFailed(error)) if error.kind() == AppErrorKind::InvalidArgument));
            assert_eq!(*state.settings.read(), before);
            assert_eq!(events.0.lock().unwrap().len(), count);
            assert_eq!(reconciled.lock().unwrap().len(), reconcile_count);
        }
        let before = state.settings.read().clone();
        let reconcile_count = reconciled.lock().unwrap().len();
        bridge.submit(HostCommand::UpdateSettings(Change::Theme("missing-native-theme".into()))).unwrap();
        tokio::time::timeout(DEADLINE, ready.notified()).await.unwrap();
        assert!(matches!(bridge.poll(), Some(HostReply::SettingsFailed(AppError::NotFound(_)))));
        assert_eq!(*state.settings.read(), before);
        assert_eq!(reconciled.lock().unwrap().len(), reconcile_count);
        let count = events.0.lock().unwrap().len();
        let settings_file = state.paths.settings_file();
        std::fs::rename(&settings_file, directory.join("previous-settings.json")).unwrap();
        std::fs::create_dir(&settings_file).unwrap();
        bridge.submit(HostCommand::UpdateSettings(Change::Language("ko".into()))).unwrap();
        tokio::time::timeout(DEADLINE, ready.notified()).await.unwrap();
        assert!(matches!(bridge.poll(), Some(HostReply::SettingsFailed(_))));
        assert_eq!(*state.settings.read(), before);
        assert_eq!(events.0.lock().unwrap().len(), count);
        assert_eq!(reconciled.lock().unwrap().len(), reconcile_count);
        tokio::time::timeout(DEADLINE, bridge.disconnect()).await.unwrap().unwrap();
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    });
    std::fs::remove_dir_all(directory).unwrap();
}
