use super::*;
use crate::host::{HostBridge, HostCommand, HostReply};
use crate::theme_edit::Command;
use std::{sync::Arc, time::Duration};
use taide_model::{layout::Tab, paths::AppPaths};
use taide_runtime::TaskSupervisor;

struct NoopEventSink;

impl taide_runtime::EventSink for NoopEventSink {
    fn publish(&self, _: taide_model::app_event::AppEvent) {}
}

const SCREEN: [f32; 2] = [1000.0, 900.0];
const DEADLINE: Duration = Duration::from_secs(5);

#[test]
fn native_settings_catalog는_소유자와_mount_독립결과를_보존한다() {
    let context = egui::Context::default();
    let directory =
        std::env::temp_dir().join(format!("taide-native-settings-view-{}", ProjectId::new()));
    std::fs::create_dir_all(&directory).unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let mut layout = taide_layout::service::default_layout();
    layout.root = PaneNode::Leaf {
        id: owner.pane.clone(),
        tabs: vec![Tab {
            id: owner.tab.clone(),
            kind: TabKind::Settings,
            title: "Settings".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(owner.tab.clone()),
    };
    layout.focused_pane = owner.pane.clone();
    state
        .layouts
        .write()
        .insert(owner.project.clone(), layout.clone());
    assert!(owner.is_active(&state));
    let mut wrong = owner.clone();
    wrong.project = ProjectId::new();
    assert!(!wrong.is_active(&state));
    let settings = Settings::default();
    let locale = locale(&state, "en");
    let appearance = Appearance::new(
        &taide_runtime::theme_actions::theme_get(&state, "taide-dark".into()).unwrap(),
    )
    .unwrap();
    let mut views = Views::default();
    let request = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.0),
    )
    .0
    .load
    .unwrap();
    assert!(
        frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 0.1),
        )
        .0
        .load
        .is_none()
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services =
        crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(NoopEventSink));
    let ready = Arc::new(tokio::sync::Notify::new());
    let signal = ready.clone();
    let mut bridge = HostBridge::connect_with_clipboard_ports(
        services,
        Arc::new(move || signal.notify_one()),
        Arc::new(|_| panic!("unexpected clipboard write")),
        Arc::new(|| panic!("unexpected clipboard read")),
        None,
    )
    .unwrap();
    runtime.block_on(async {
        bridge
            .submit(HostCommand::ReadSettingsCatalog(request.clone()))
            .unwrap();
        let reply = tokio::time::timeout(DEADLINE, async {
            loop {
                if let Some(reply) = bridge.poll() {
                    break reply;
                }
                ready.notified().await;
            }
        })
        .await
        .unwrap();
        let HostReply::SettingsCatalog {
            request: completed,
            result,
        } = reply
        else {
            panic!("unexpected reply");
        };
        assert_eq!(completed, request);
        let catalog = result.unwrap();
        assert!(
            catalog
                .themes
                .as_ref()
                .unwrap()
                .iter()
                .any(|theme| theme.id == "taide-dark")
        );
        assert!(
            catalog
                .locales
                .as_ref()
                .unwrap()
                .iter()
                .any(|locale| locale.id == "en")
        );
        assert!(views.accept(&completed, Ok(catalog)));
        assert!(!views.accept(&completed, Err(AppError::Internal("duplicate".into()))));
        views.begin_frame();
        views.finish_frame();
        let next = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 0.2),
        )
        .0
        .load
        .unwrap();
        assert_ne!(next.mount(), completed.mount());
        assert!(!views.accept(&completed, Ok(Catalog::load(&state))));
        assert!(views.inspection().get(&owner).unwrap().pending.as_ref() == Some(&next));
        assert!(views.accept(
            &next,
            Ok(Catalog {
                themes: Err(AppError::Io("theme failure".into())),
                locales: Ok(vec![LocaleSummary {
                    id: "en".into(),
                    name: "English".into(),
                    builtin: true
                }])
            })
        ));
        let catalog = views
            .inspection()
            .get(&owner)
            .unwrap()
            .catalog
            .as_ref()
            .unwrap();
        assert!(catalog.themes.is_err() && catalog.locales.is_ok());
        assert!(
            frame(
                &mut views,
                &context,
                &owner,
                &settings,
                &locale,
                &appearance,
                (Vec::new(), 0.3),
            )
            .0
            .load
            .is_none()
        );
        state.layouts.write().remove(&owner.project);
        assert!(!next.is_active(&state));
        assert!(next.load(&state).is_err());
        views.clear();
        assert!(!views.accept(&next, Ok(Catalog::load(&state))));
        bridge.disconnect();
        tasks.shutdown().await;
    });
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_settings_view는_테마입력과_목차_scroll_번역을_연결한다() {
    let context = egui::Context::default();
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-settings-headless-{}", ProjectId::new())),
    ));
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let settings = Settings::default();
    let english = locale(&state, "en");
    let theme = taide_runtime::theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
    let appearance = Appearance::new(&theme).unwrap();
    let mut views = Views::default();
    let request = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &english,
        &appearance,
        (Vec::new(), 0.0),
    )
    .0
    .load
    .unwrap();
    let themes = vec![
        ThemeSummary {
            id: "taide-dark".into(),
            name: "Dark synthetic".into(),
            theme_type: ThemeType::Dark,
            builtin: true,
        },
        ThemeSummary {
            id: "taide-light".into(),
            name: "Light synthetic".into(),
            theme_type: ThemeType::Light,
            builtin: true,
        },
    ];
    assert!(views.accept(
        &request,
        Ok(Catalog {
            themes: Ok(themes),
            locales: Ok(vec![LocaleSummary {
                id: "en".into(),
                name: "English".into(),
                builtin: true
            }])
        })
    ));
    let (header, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &english,
        &appearance,
        (Vec::new(), 0.1),
    );
    let header_rect = header
        .traces
        .iter()
        .find(|trace| trace.field == "open-settings-file")
        .unwrap()
        .rect;
    assert_eq!(header_rect.height(), SETTINGS_FILE_HEIGHT);
    assert!(!header.open_settings_file);
    assert!(
        frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &english,
            &appearance,
            (click(header_rect.center()), 0.15),
        )
        .0
        .open_settings_file
    );
    let theme_rect = text_rect(&drawing, "Light synthetic", false);
    let output = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &english,
        &appearance,
        (click(theme_rect.center()), 0.2),
    )
    .0;
    assert!(
        output
            .changes
            .iter()
            .any(|change| matches!(change, Change::Theme(id) if id == "taide-light"))
    );
    let nav = text_rect(
        &drawing,
        &message(&english, "settings.interface", &[]),
        true,
    );
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &english,
        &appearance,
        (click(nav.center()), 0.3),
    );
    assert_eq!(
        views.inspection().get(&owner).unwrap().active,
        Section::Interface
    );
    let (_, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &english,
        &appearance,
        (Vec::new(), 1.0),
    );
    let resizer = text_rect(
        &drawing,
        &message(&english, "settings.resizerThickness", &[]),
        false,
    );
    assert!(resizer.top() >= 0.0 && resizer.bottom() < SCREEN[1]);
    let field = text_rect(
        &drawing,
        &message(&english, "settings.showSystemUsage", &[]),
        false,
    );
    let output = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &english,
        &appearance,
        (click(field.center()), 1.1),
    )
    .0;
    assert_eq!(output.changes.iter().filter(|change| matches!(change, Change::Switch(Switch::ShowSystemUsage, value) if *value != settings.show_system_usage)).count(), 1);
    let korean = locale(&state, "ko");
    let light = Appearance::new(
        &taide_runtime::theme_actions::theme_get(&state, "taide-light".into()).unwrap(),
    )
    .unwrap();
    let (_, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &korean,
        &light,
        (Vec::new(), 1.2),
    );
    text_rect(&drawing, &message(&korean, "settings.interface", &[]), true);
    assert_eq!(
        views.inspection().get(&owner).unwrap().active,
        Section::Interface
    );
    assert_eq!(
        views.inspection().get(&owner).unwrap().mount,
        request.mount()
    );
    assert_ne!(
        appearance.inspection_background(),
        light.inspection_background()
    );
}

fn locale(state: &AppState, language: &str) -> ResolvedLocale {
    locale_actions::locale_get(state, language.into()).unwrap()
}

#[test]
fn native_settings_palette는_모든_builtin_테마에서_해석된다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-settings-palette-{}", ProjectId::new())),
    ));
    let themes = theme_actions::theme_list(&state).unwrap();
    let mut count = 0;
    for theme in themes.into_iter().filter(|theme| theme.builtin) {
        let resolved = theme_actions::theme_get(&state, theme.id.clone()).unwrap();
        Appearance::new(&resolved).unwrap_or_else(|error| panic!("{}: {error}", theme.id));
        count += 1;
    }
    assert!(count > 1);
}

#[test]
fn native_settings_language_keyboard는_arrow_down_재열기를_보존한다() {
    let context = egui::Context::default();
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-settings-input-{}", ProjectId::new())),
    ));
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let settings = Settings {
        language: "missing-synthetic".into(),
        ..Default::default()
    };
    let locale = locale(&state, "en");
    let appearance = Appearance::new(
        &taide_runtime::theme_actions::theme_get(&state, "taide-dark".into()).unwrap(),
    )
    .unwrap();
    let mut views = Views::default();
    let request = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.0),
    )
    .0
    .load
    .unwrap();
    views.accept(
        &request,
        Ok(Catalog {
            themes: Ok(Vec::new()),
            locales: Ok(vec![LocaleSummary {
                id: "en".into(),
                name: "English".into(),
                builtin: true,
            }]),
        }),
    );
    let (_, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.1),
    );
    let language = text_rect(&drawing, &message(&locale, "settings.language", &[]), true);
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (click(language.center()), 0.2),
    );
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 1.0),
    );
    let (_, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 1.05),
    );
    let trigger = text_rect(
        &drawing,
        &message(&locale, "settings.systemLanguage", &[]),
        false,
    )
    .center();
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (click(trigger), 1.1),
    );
    assert!(views.inspection()[&owner].language_open);
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (click(trigger), 1.5),
    );
    assert!(!views.inspection()[&owner].language_open);
    let previous_focus = context.memory(|memory| memory.focused());
    let (output, _) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (key(egui::Key::ArrowDown), 1.6),
    );
    assert!(
        views.inspection()[&owner].language_open,
        "before={previous_focus:?}, now={:?}, traces={:?}, keys={:?}",
        context.memory(|memory| memory.focused()),
        output.traces,
        context.input(|input| input.events.clone())
    );
    assert_eq!(views.inspection()[&owner].language_cursor, 0);
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (key(egui::Key::ArrowDown), 1.7),
    );
    let output = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (key(egui::Key::Enter), 1.8),
    )
    .0;
    assert!(
        output
            .changes
            .iter()
            .any(|change| matches!(change, Change::Language(id) if id == "en"))
    );
    assert!(!views.inspection()[&owner].language_open);
}

#[test]
fn native_settings_numeric_position는_저장변경과_blur를_연결한다() {
    let context = egui::Context::default();
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-settings-number-{}", ProjectId::new())),
    ));
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let settings = Settings::default();
    let locale = locale(&state, "en");
    let appearance = Appearance::new(
        &taide_runtime::theme_actions::theme_get(&state, "taide-dark".into()).unwrap(),
    )
    .unwrap();
    let mut views = Views::default();
    let request = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.0),
    )
    .0
    .load
    .unwrap();
    views.accept(
        &request,
        Ok(Catalog {
            themes: Ok(Vec::new()),
            locales: Ok(Vec::new()),
        }),
    );
    let (_, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 2.0),
    );
    let interface = text_rect(&drawing, &message(&locale, "settings.interface", &[]), true);
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (click(interface.center()), 2.1),
    );
    views.inspection_mut().get_mut(&owner).unwrap().resizer.text = "999".into();
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 3.0),
    );
    let (_, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 3.05),
    );
    let number = text_rect(&drawing, "999", false);
    let (number_output, drawing) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (click(number.center()), 3.1),
    );
    let number_traces = number_output.traces;
    let mut shapes = drawing
        .shapes
        .iter()
        .map(|shape| &shape.shape)
        .collect::<Vec<_>>();
    let mut positions = Vec::new();
    while let Some(shape) = shapes.pop() {
        match shape {
            egui::Shape::Vec(children) => shapes.extend(children),
            egui::Shape::Rect(rect)
                if rect.rect.height() == POSITION_HEIGHT && rect.rect.width() < POSITION_WIDTH =>
            {
                positions.push(rect.rect)
            }
            _ => (),
        }
    }
    assert_eq!(positions.len(), Position::ALL.len());
    positions.sort_by(|a, b| {
        a.top()
            .total_cmp(&b.top())
            .then(a.left().total_cmp(&b.left()))
    });
    let output = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (click(positions.last().unwrap().center()), 3.2),
    )
    .0;
    assert!(output.changes.iter().any(
        |change| matches!(change, Change::Position(position) if position.value() == "bottom-right")
    ), "target={:?}, changes={:?}, focus={:?}", positions.last(), output.changes, context.memory(|memory| memory.focused()));
    assert!(
        output
            .changes
            .iter()
            .any(|change| matches!(change, Change::Numeric(Numeric::ResizerThickness, 8))),
        "number={number:?}, before={number_traces:?}, after={:?}, draft={:?}, changes={:?}",
        output.traces,
        views.inspection()[&owner].resizer.text,
        output.changes
    );
    assert_eq!(
        views.inspection()[&owner].resizer.text,
        settings.resizer_thickness.to_string()
    );
}

#[test]
fn native_settings_open_host는_선택pane와_중복_aux_거절을_보존한다() {
    let directory = std::env::temp_dir().join(format!("taide-settings-open-{}", ProjectId::new()));
    std::fs::create_dir_all(&directory).unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    let project = ProjectId::new();
    let mut layout = taide_layout::service::default_layout();
    let pane = layout.focused_pane.clone();
    let aux_pane = PaneId::new();
    layout
        .auxiliary_windows
        .push(taide_model::layout::AuxWindowLayout {
            slot: 1,
            root: PaneNode::Leaf {
                id: aux_pane.clone(),
                tabs: Vec::new(),
                active: None,
            },
            focused_pane: aux_pane.clone(),
        });
    state.layouts.write().insert(project.clone(), layout);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services =
        crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(NoopEventSink));
    let ready = Arc::new(tokio::sync::Notify::new());
    let signal = ready.clone();
    let mut bridge = HostBridge::connect_with_clipboard_ports(
        services,
        Arc::new(move || signal.notify_one()),
        Arc::new(|_| panic!("clipboard write")),
        Arc::new(|| panic!("clipboard read")),
        None,
    )
    .unwrap();
    runtime.block_on(async {
        for target in [&pane, &pane, &aux_pane] {
            bridge
                .submit(HostCommand::OpenSettings {
                    project: project.clone(),
                    pane: target.clone(),
                    title: "Synthetic settings".into(),
                })
                .unwrap();
            tokio::time::timeout(DEADLINE, ready.notified())
                .await
                .unwrap();
            assert!(bridge.poll().is_none());
        }
        let opened = state.layouts.read()[&project].clone();
        for root in taide_layout::service::all_roots(&opened) {
            let settings = crate::tabs::tabs_in(root)
                .into_iter()
                .filter(|tab| matches!(tab.kind, TabKind::Settings))
                .collect::<Vec<_>>();
            assert_eq!(settings.len(), 1);
            assert!(!settings[0].preview && !settings[0].dirty);
        }
        bridge
            .submit(HostCommand::OpenSettings {
                project: project.clone(),
                pane: PaneId::new(),
                title: "Invalid pane".into(),
            })
            .unwrap();
        tokio::time::timeout(DEADLINE, ready.notified())
            .await
            .unwrap();
        assert!(matches!(bridge.poll(), Some(HostReply::Failed(_))));
        assert_eq!(state.layouts.read()[&project], opened);
        bridge.disconnect();
        tasks.shutdown().await;
    });
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_settings_theme는_실제_create_host와_목록갱신_닫힘을_연결한다() {
    let directory =
        std::env::temp_dir().join(format!("taide-native-settings-theme-{}", ProjectId::new()));
    std::fs::create_dir_all(&directory).unwrap();
    let state = AppState::new(AppPaths::new(directory.clone()));
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let mut layout = taide_layout::service::default_layout();
    layout.root = PaneNode::Leaf {
        id: owner.pane.clone(),
        tabs: vec![Tab {
            id: owner.tab.clone(),
            kind: TabKind::Settings,
            title: "Settings".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(owner.tab.clone()),
    };
    layout.focused_pane = owner.pane.clone();
    state.layouts.write().insert(owner.project.clone(), layout);
    let context = egui::Context::default();
    let settings = Settings::default();
    let locale = locale(&state, "en");
    let appearance =
        Appearance::new(&theme_actions::theme_get(&state, "taide-dark".into()).unwrap()).unwrap();
    let mut views = Views::default();
    let stale = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.0),
    )
    .0
    .load
    .unwrap();
    views.observe_theme_revision(1);
    let request = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.1),
    )
    .0
    .load
    .unwrap();
    assert_ne!(stale, request);
    assert!(!views.accept(&stale, Ok(Catalog::load(&state))));
    let mut catalog = Catalog::load(&state);
    catalog
        .themes
        .as_mut()
        .unwrap()
        .retain(|theme| matches!(theme.id.as_str(), "taide-dark" | "taide-light"));
    assert!(views.accept(&request, Ok(catalog)));
    let first = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.2),
    )
    .0;
    let create = first
        .traces
        .iter()
        .find(|trace| trace.field == "create-theme")
        .unwrap()
        .rect;
    let clicked = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (click(create.center()), 0.3),
    )
    .0;
    assert!(clicked.error.is_none());
    assert!(clicked.changes.is_empty());
    assert!(views.inspection()[&owner].editor.is_some());
    let command = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.4),
    )
    .0
    .themes
    .pop()
    .unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let tasks = TaskSupervisor::new(runtime.handle().clone());
    let services =
        crate::bootstrap::services(state.clone(), tasks.clone(), Arc::new(NoopEventSink));
    let ready = Arc::new(tokio::sync::Notify::new());
    let wake = ready.clone();
    let mut bridge = HostBridge::connect_with_clipboard_ports(
        services,
        Arc::new(move || wake.notify_one()),
        Arc::new(|_| panic!("clipboard write")),
        Arc::new(|| panic!("clipboard read")),
        None,
    )
    .unwrap();
    runtime.block_on(async {
        bridge.submit(HostCommand::ThemeEdit(command)).unwrap();
        let reply = next_theme_reply(&mut bridge, &ready).await;
        assert!(views.accept_theme(reply).is_none());
        let loaded = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 0.5),
        );
        assert!(loaded.0.error.is_none());
        assert!(views.preview(egui::ViewportId::ROOT, &state).is_some());
        assert!(
            views
                .preview(egui::ViewportId::from_hash_of("another-window"), &state)
                .is_none()
        );
        let id = views
            .preview(egui::ViewportId::ROOT, &state)
            .unwrap()
            .id
            .clone();
        let save = text_rect(&loaded.1, "Save", false);
        let mut output = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (click(save.center()), 0.6),
        )
        .0;
        assert_eq!(output.themes.len(), 1);
        let command = output.themes.pop().unwrap();
        bridge.submit(HostCommand::ThemeEdit(command)).unwrap();
        assert!(
            views
                .accept_theme(next_theme_reply(&mut bridge, &ready).await)
                .is_none()
        );
        assert!(views.inspection()[&owner].editor.is_none());
        assert!(views.preview(egui::ViewportId::ROOT, &state).is_none());
        let request = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 0.7),
        )
        .0
        .load
        .unwrap();
        assert_ne!(request, stale);
        assert!(views.accept(&request, request.load(&state)));
        assert!(
            views.inspection()[&owner]
                .catalog
                .as_ref()
                .unwrap()
                .themes
                .as_ref()
                .unwrap()
                .iter()
                .any(|theme| theme.id == id && !theme.builtin)
        );
        views
            .inspection_mut()
            .get_mut(&owner)
            .unwrap()
            .catalog
            .as_mut()
            .unwrap()
            .themes
            .as_mut()
            .unwrap()
            .retain(|theme| {
                theme.id == id || matches!(theme.id.as_str(), "taide-dark" | "taide-light")
            });
        let listing = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 0.8),
        )
        .0;
        let edit = listing
            .traces
            .iter()
            .find(|trace| trace.field == "edit-custom-theme")
            .unwrap()
            .rect;
        assert!(edit.right() <= SCREEN[0] - f32::from(PAGE_X));
        assert!(
            listing
                .traces
                .iter()
                .any(|trace| trace.field == "duplicate-custom-theme")
        );
        let clicked = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (click(edit.center()), 0.9),
        )
        .0;
        assert!(clicked.changes.is_empty());
        assert!(views.inspection()[&owner].editor.is_some());
        let command = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 1.0),
        )
        .0
        .themes
        .pop()
        .unwrap();
        bridge.submit(HostCommand::ThemeEdit(command)).unwrap();
        assert!(
            views
                .accept_theme(next_theme_reply(&mut bridge, &ready).await)
                .is_none()
        );
        let loaded = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 1.1),
        );
        let delete = text_rect(&loaded.1, "Delete Theme", false);
        frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (click(delete.center()), 1.2),
        );
        let modal = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (Vec::new(), 1.3),
        );
        let confirm = text_rect(&modal.1, "Delete Theme", false);
        let mut output = frame(
            &mut views,
            &context,
            &owner,
            &settings,
            &locale,
            &appearance,
            (click(confirm.center()), 1.4),
        )
        .0;
        assert_eq!(output.themes.len(), 1);
        bridge
            .submit(HostCommand::ThemeEdit(output.themes.pop().unwrap()))
            .unwrap();
        assert!(
            views
                .accept_theme(next_theme_reply(&mut bridge, &ready).await)
                .is_none()
        );
        assert!(views.inspection()[&owner].editor.is_none());
        assert!(views.preview(egui::ViewportId::ROOT, &state).is_none());
        assert!(
            !theme_actions::theme_list(&state)
                .unwrap()
                .iter()
                .any(|theme| theme.id == id)
        );
        bridge.disconnect().await.unwrap();
        tasks.shutdown().await;
    });
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn native_settings_preview는_창과_활성_owner_미저장값_수명을_보존한다() {
    let state = AppState::new(AppPaths::new(
        std::env::temp_dir().join(format!("taide-native-theme-preview-{}", ProjectId::new())),
    ));
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let mut layout = taide_layout::service::default_layout();
    layout.root = PaneNode::Leaf {
        id: owner.pane.clone(),
        tabs: vec![Tab {
            id: owner.tab.clone(),
            kind: TabKind::Settings,
            title: "Settings".into(),
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        }],
        active: Some(owner.tab.clone()),
    };
    layout.focused_pane = owner.pane.clone();
    state
        .layouts
        .write()
        .insert(owner.project.clone(), layout.clone());
    let context = egui::Context::default();
    let settings = Settings::default();
    let locale = locale(&state, "en");
    let theme = theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
    let appearance = Appearance::new(&theme).unwrap();
    let mut views = Views::default();
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.0),
    );
    views.inspection_mut().get_mut(&owner).unwrap().editor = Some(
        Editor::new(
            owner.clone(),
            "taide-dark".into(),
            Mode::Create,
            "Preview copy".into(),
        )
        .unwrap(),
    );
    let Command::Load(request) = frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.1),
    )
    .0
    .themes
    .pop()
    .unwrap() else {
        panic!("expected load");
    };
    let mut draft =
        crate::theme_draft::Draft::load(&state, "taide-dark", Mode::Create, "Preview copy".into())
            .unwrap();
    draft
        .set_color(
            crate::theme_draft::ColorDomain::Colors,
            "app.background",
            "#abc".into(),
        )
        .unwrap();
    draft
        .set_color(
            crate::theme_draft::ColorDomain::Terminal,
            "background",
            "transparent".into(),
        )
        .unwrap();
    assert!(
        views
            .accept_theme(crate::theme_edit::Reply::Loaded {
                request,
                result: Ok(Box::new(draft))
            })
            .is_none()
    );
    frame(
        &mut views,
        &context,
        &owner,
        &settings,
        &locale,
        &appearance,
        (Vec::new(), 0.2),
    );
    let preview = views.preview(egui::ViewportId::ROOT, &state).unwrap();
    let palette = crate::presentation_refresh::Appearances::new(preview, &settings).unwrap();
    assert_eq!(palette.shell.background, Color32::from_rgb(170, 187, 204));
    assert_eq!(preview.terminal["background"], "transparent");
    assert_eq!(state.settings.read().theme_id, settings.theme_id);
    assert_eq!(
        theme_actions::theme_get(&state, "taide-dark".into())
            .unwrap()
            .colors,
        theme.colors
    );
    assert!(
        views
            .preview(egui::ViewportId::from_hash_of("another-window"), &state)
            .is_none()
    );
    if let PaneNode::Leaf { active, .. } =
        &mut state.layouts.write().get_mut(&owner.project).unwrap().root
    {
        *active = None;
    }
    assert!(views.preview(egui::ViewportId::ROOT, &state).is_none());
    state.layouts.write().insert(owner.project.clone(), layout);
    assert!(views.preview(egui::ViewportId::ROOT, &state).is_some());
    views.begin_frame();
    views.finish_frame();
    assert!(views.preview(egui::ViewportId::ROOT, &state).is_none());
}

async fn next_theme_reply(
    bridge: &mut HostBridge,
    ready: &tokio::sync::Notify,
) -> crate::theme_edit::Reply {
    let reply = tokio::time::timeout(DEADLINE, async {
        loop {
            if let Some(reply) = bridge.poll() {
                break reply;
            }
            ready.notified().await;
        }
    })
    .await
    .unwrap();
    let HostReply::ThemeEdit(reply) = reply else {
        panic!("expected theme edit reply");
    };
    reply
}

fn key(key: egui::Key) -> Vec<egui::Event> {
    vec![egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]
}

fn frame(
    views: &mut Views,
    context: &egui::Context,
    owner: &Owner,
    settings: &Settings,
    locale: &ResolvedLocale,
    appearance: &Appearance,
    input: (Vec<egui::Event>, f64),
) -> (Output, egui::FullOutput) {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(SCREEN[0], SCREEN[1]),
        )),
        events: input.0,
        time: Some(input.1),
        ..Default::default()
    };
    let mut output = Output::default();
    views.begin_frame();
    let mut drawing = context.run_ui(input, |ui| {
        output = views.show(ui, owner.clone(), settings, locale, appearance);
    });
    views.finish_frame();
    drawing.textures_delta.clear();
    (output, drawing)
}

fn click(position: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(position),
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn text_rect(drawing: &egui::FullOutput, value: &str, is_nav: bool) -> Rect {
    let mut pending = drawing
        .shapes
        .iter()
        .map(|shape| &shape.shape)
        .collect::<Vec<_>>();
    while let Some(shape) = pending.pop() {
        match shape {
            egui::Shape::Vec(shapes) => pending.extend(shapes),
            egui::Shape::Text(text)
                if text.galley.job.text == value && (text.pos.x < TOC_WIDTH) == is_nav =>
            {
                return text.galley.rect.translate(text.pos.to_vec2());
            }
            _ => (),
        }
    }
    panic!("text not rendered: {value}, nav: {is_nav}");
}
