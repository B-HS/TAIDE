use eframe::egui::{self, Context, Event, Id, Pos2, Rect, Response};

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);

#[test]
fn button_escape는_원본처럼_focus를_유지하고_generic_focus_policy는_보존한다() {
    for native_button in [false, true] {
        let context = Context::default();
        let render = |events| {
            let mut response = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    events,
                    ..Default::default()
                },
                |ui| {
                    response = Some(if native_button {
                        ui.button("Native HTML-style button")
                    } else {
                        ui.interact(
                            Rect::from_min_size(Pos2::ZERO, SCREEN),
                            Id::new("generic-control"),
                            egui::Sense::click(),
                        )
                    });
                },
            );
            output.textures_delta.clear();
            response.unwrap()
        };
        let id = render(Vec::new()).id;
        context.memory_mut(|memory| memory.request_focus(id));
        let response = render(vec![key(egui::Key::Escape, true)]);
        assert_eq!(response.has_focus(), native_button);
        assert!(!response.clicked());
    }
}

#[test]
fn window_capture의_입력전_focus는_예약된_역방향_tab을_반영한다() {
    let context = Context::default();
    let mut routes = crate::terminal_surface::Views::default();
    let generic_id = Id::new("queued-generic-focus");
    let mut button_id = None;
    let mut actions = Vec::new();
    let overrides = serde_json::json!([
        {"actionId": "toggle-sidebar", "key": "Enter", "mods": []}
    ])
    .to_string();
    let mut render = |events, capture| {
        let mut clicked = false;
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                ..Default::default()
            },
            |ui| {
                if capture {
                    assert_eq!(ui.ctx().keyboard_focus_before_events(), button_id);
                    routes
                        .capture_window_keymap(
                            ui.ctx(),
                            Some(&overrides),
                            &mut actions,
                            false,
                            |_| None,
                        )
                        .unwrap();
                }
                let button = ui.button("Previous tab target");
                button_id = Some(button.id);
                clicked = button.clicked();
                ui.interact(
                    Rect::from_min_size(Pos2::ZERO, SCREEN),
                    generic_id,
                    egui::Sense::click(),
                );
            },
        );
        output.textures_delta.clear();
        clicked
    };
    render(Vec::new(), false);
    context.memory_mut(|memory| memory.request_focus(generic_id));
    render(Vec::new(), false);
    let mut previous = key(egui::Key::Tab, true);
    if let Event::Key { modifiers, .. } = &mut previous {
        *modifiers = egui::Modifiers::SHIFT;
    }
    render(vec![previous], false);
    assert!(!render(vec![key(egui::Key::Enter, true)], true));
    assert_eq!(actions, ["toggle-sidebar"]);
}

#[test]
fn app_window_capture는_ax_focus_뒤_button_default_action보다_먼저_소비한다() {
    for key_code in [egui::Key::Enter, egui::Key::Space] {
        for reverse in [false, true] {
            let context = Context::default();
            context.enable_accesskit();
            let mut routes = crate::terminal_surface::Views::default();
            let mut actions = Vec::new();
            let overrides = serde_json::json!([
                {"actionId": "toggle-sidebar", "key": key_code.name(), "mods": []}
            ])
            .to_string();
            let mut render = |events, capture| {
                let mut responses = Vec::new();
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        if capture {
                            routes
                                .capture_window_keymap(
                                    ui.ctx(),
                                    Some(&overrides),
                                    &mut actions,
                                    false,
                                    |_| None,
                                )
                                .unwrap();
                        }
                        let mut indices = [0, 1];
                        if reverse {
                            indices.reverse();
                        }
                        for index in indices {
                            responses.push((
                                index,
                                ui.scope_builder(
                                    egui::UiBuilder::new()
                                        .id(Id::new(("capture-ordered-button", index))),
                                    |ui| ui.button("Action"),
                                )
                                .inner,
                            ));
                        }
                    },
                );
                output.textures_delta.clear();
                responses.sort_by_key(|(index, _)| *index);
                responses
            };
            let initial = render(Vec::new(), false);
            context.memory_mut(|memory| memory.request_focus(initial[0].1.id));
            render(Vec::new(), false);
            let mut events = vec![
                Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: initial[1].1.id.accesskit_id(),
                    data: None,
                }),
                key(key_code, true),
            ];
            if key_code == egui::Key::Space {
                events.push(key(key_code, false));
            }
            let responses = render(events, true);
            assert!(!responses[0].1.clicked());
            assert!(
                !responses[1].1.clicked(),
                "key={key_code:?} reverse={reverse}"
            );
            assert_eq!(actions, ["toggle-sidebar"]);
        }
    }
}

#[test]
fn app_window_keymap은_button_default_click보다_먼저_소비한다() {
    for key_code in [egui::Key::Enter, egui::Key::Space] {
        for split in [false, true] {
            let context = Context::default();
            let mut routes = crate::terminal_surface::Views::default();
            let mut actions = Vec::new();
            let overrides = serde_json::json!([
                {"actionId": "toggle-sidebar", "key": key_code.name(), "mods": []}
            ])
            .to_string();
            let mut render = |events, overrides: Option<&str>| {
                let mut clicked = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        let previous_actions = actions.len();
                        routes
                            .capture_window_keymap(ui.ctx(), overrides, &mut actions, false, |_| {
                                None
                            })
                            .unwrap();
                        clicked = ui.button("Synthetic app action").clicked();
                        routes
                            .route_window_keys(
                                ui.ctx(),
                                crate::keymap::Context::default(),
                                false,
                                overrides,
                                &mut actions,
                                false,
                            )
                            .unwrap();
                        if overrides.is_some() {
                            assert_eq!(actions.len(), previous_actions + 1);
                        }
                    },
                );
                output.textures_delta.clear();
                clicked
            };
            render(Vec::new(), None);
            let id = context.memory(|memory| memory.focused());
            assert!(id.is_none());
            let mut button_id = None;
            let mut output = context.run_ui(Default::default(), |ui| {
                button_id = Some(ui.button("Synthetic app action").id);
            });
            output.textures_delta.clear();
            context.memory_mut(|memory| memory.request_focus(button_id.unwrap()));
            render(Vec::new(), None);
            let mut events = vec![key(key_code, true)];
            if !split {
                events.push(key(key_code, false));
            }
            assert!(
                !render(events, Some(&overrides)),
                "{key_code:?} split={split}: canceled press"
            );
            if split {
                assert!(
                    !render(vec![key(key_code, false)], None),
                    "{key_code:?}: canceled press cannot arm a later release"
                );
            }
            assert_eq!(actions, ["toggle-sidebar"]);
        }
    }
}

#[test]
fn app_button_capture는_다른_focus_사건과_generic_target을_선점하지_않는다() {
    for mode in ["generic", "unbound", "pointer", "ax", "blur", "tab", "ime"] {
        let context = Context::default();
        context.enable_accesskit();
        let mut routes = crate::terminal_surface::Views::default();
        let mut actions = Vec::new();
        let overrides = serde_json::json!([
            {"actionId": "toggle-sidebar", "key": "Enter", "mods": []}
        ])
        .to_string();
        let mut id = None;
        let mut output = context.run_ui(Default::default(), |ui| {
            let response = if mode == "generic" {
                ui.interact(
                    Rect::from_min_size(Pos2::ZERO, SCREEN),
                    Id::new("synthetic-code-editor"),
                    egui::Sense::click(),
                )
            } else {
                ui.button("Synthetic registered button")
            };
            id = Some(response.id);
        });
        output.textures_delta.clear();
        let id = id.unwrap();
        context.memory_mut(|memory| memory.request_focus(id));
        let mut events = vec![key(egui::Key::Enter, true)];
        let other = match mode {
            "pointer" => Some(Event::PointerButton {
                pos: Pos2::ZERO,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }),
            "ax" => Some(Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: Id::new("unregistered-capture-target").accesskit_id(),
                    data: None,
                },
            )),
            "blur" => Some(Event::WindowFocused(false)),
            "tab" => Some(key(egui::Key::Tab, true)),
            "ime" => Some(Event::Ime(egui::ImeEvent::Preedit {
                text: "synthetic".into(),
                active_range_chars: None,
            })),
            _ => None,
        };
        if let Some(event) = other {
            events.insert(0, event);
        }
        let mut output = context.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                let before = ui.input(|input| input.events.clone());
                routes
                    .capture_window_keymap(
                        ui.ctx(),
                        (mode != "unbound").then_some(overrides.as_str()),
                        &mut actions,
                        false,
                        |_| None,
                    )
                    .unwrap();
                assert_eq!(
                    ui.input(|input| input.events.clone()),
                    before,
                    "mode={mode}"
                );
                assert!(actions.is_empty(), "mode={mode}");
            },
        );
        output.textures_delta.clear();
    }
}

#[test]
fn button_key의_ax_focus_배정은_최종_focus나_draw_순서에_몰리지_않는다() {
    for key_code in [egui::Key::Enter, egui::Key::Space] {
        for focus_last in [false, true] {
            for reverse in [false, true] {
                let context = Context::default();
                context.enable_accesskit();
                let render = |events| {
                    let mut responses = Vec::new();
                    let mut output = context.run_ui(
                        egui::RawInput {
                            events,
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                            ..Default::default()
                        },
                        |ui| {
                            let mut indices = [0, 1];
                            if reverse {
                                indices.reverse();
                            }
                            for index in indices {
                                let shown = ui.scope_builder(
                                    egui::UiBuilder::new()
                                        .id(Id::new(("ordered-button-key", index))),
                                    |ui| ui.button("Action"),
                                );
                                responses.push((index, shown.inner));
                            }
                        },
                    );
                    output.textures_delta.clear();
                    responses.sort_by_key(|(index, _)| *index);
                    responses
                };
                let initial = render(Vec::new());
                context.memory_mut(|memory| memory.request_focus(initial[0].1.id));
                render(Vec::new());
                let focus = Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: initial[1].1.id.accesskit_id(),
                    data: None,
                });
                let mut events = vec![key(key_code, true)];
                if focus_last {
                    events.push(focus);
                } else {
                    events.insert(0, focus);
                }
                if key_code == egui::Key::Space {
                    events.push(key(key_code, false));
                }
                let responses = render(events);
                let expected = if focus_last {
                    [key_code == egui::Key::Enter, false]
                } else {
                    [false, true]
                };
                assert_eq!(
                    [responses[0].1.clicked(), responses[1].1.clicked()],
                    expected,
                    "key={key_code:?} focus_last={focus_last} reverse={reverse}"
                );
            }
        }
    }
}

fn key(key: egui::Key, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

fn frame(context: &Context, events: Vec<Event>, cancel: Option<bool>, enabled: bool) -> Response {
    let mut response = None;
    let mut output = context.run_ui(egui::RawInput {
        events, screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)), ..Default::default()
    }, |ui| {
        if let Some(pressed) = cancel {
            ui.input_mut(|input| input.events.retain(|event| {
                !matches!(event, Event::Key { key: egui::Key::Space, pressed: actual, .. } if *actual == pressed)
            }));
        }
        response = Some(ui.add_enabled(enabled, egui::Button::new("Synthetic button")));
    });
    output.textures_delta.clear();
    response.unwrap()
}

#[test]
fn button_space는_원본처럼_press_repeat에_누르고_짝있는_release에만_click한다() {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-05-button-default-keys-source.json"
    ))
    .unwrap();
    let samples = &source["results"][0];
    assert_eq!(samples["mode"], "space");
    assert_eq!(
        [
            &samples["down"]["clicks"],
            &samples["repeat"]["clicks"],
            &samples["up"]["clicks"]
        ],
        [
            &serde_json::json!(0),
            &serde_json::json!(0),
            &serde_json::json!(1)
        ]
    );
    let context = Context::default();
    let id = frame(&context, Vec::new(), None, true).id;
    context.memory_mut(|memory| memory.request_focus(id));
    assert!(!frame(&context, vec![key(egui::Key::Space, false)], None, true).clicked());
    assert!(
        !frame(&context, vec![key(egui::Key::Space, true)], None, true).clicked(),
        "Space keydown cannot click a native HTML button"
    );
    assert!(
        !frame(&context, vec![key(egui::Key::Space, true)], None, true).clicked(),
        "held Space repeat cannot click"
    );
    assert!(frame(&context, vec![key(egui::Key::Space, false)], None, true).clicked());
    assert!(!frame(&context, vec![key(egui::Key::Space, false)], None, true).clicked());
}

#[test]
fn button_space는_소비된_press_release와_blur_disabled에서_활성화하지_않는다() {
    for mode in ["cancel-down", "cancel-up", "blur", "disabled"] {
        let context = Context::default();
        let id = frame(&context, Vec::new(), None, true).id;
        context.memory_mut(|memory| memory.request_focus(id));
        let cancel = (mode == "cancel-down").then_some(true);
        assert!(
            !frame(&context, vec![key(egui::Key::Space, true)], cancel, true).clicked(),
            "{mode}: press"
        );
        if mode == "blur" {
            context.memory_mut(|memory| memory.request_focus(Id::new("another-control")));
        }
        let cancel = (mode == "cancel-up").then_some(false);
        assert!(
            !frame(
                &context,
                vec![key(egui::Key::Space, false)],
                cancel,
                mode != "disabled"
            )
            .clicked(),
            "{mode}: release"
        );
    }
}

#[test]
fn button_enter_ax와_generic_menu의_press_정책은_변경하지_않는다() {
    let context = Context::default();
    context.enable_accesskit();
    let id = frame(&context, Vec::new(), None, true).id;
    context.memory_mut(|memory| memory.request_focus(id));
    assert!(frame(&context, vec![key(egui::Key::Enter, true)], None, true).clicked());
    assert!(frame(&context, vec![key(egui::Key::Enter, true)], None, true).clicked());
    assert!(!frame(&context, vec![key(egui::Key::Enter, false)], None, true).clicked());
    assert!(
        frame(
            &context,
            vec![Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: id.accesskit_id(),
                    data: None,
                }
            )],
            None,
            true
        )
        .clicked()
    );
    for menu in [false, true] {
        let context = Context::default();
        let render = |events| {
            let mut shown = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    events,
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    ..Default::default()
                },
                |ui| {
                    shown = Some(if menu {
                        egui::containers::menu::MenuButton::new("Synthetic menu")
                            .ui(ui, |ui| {
                                ui.label("Item");
                            })
                            .0
                    } else {
                        ui.interact(
                            Rect::from_min_size(egui::pos2(80.0, 100.0), egui::vec2(40.0, 20.0)),
                            Id::new("generic-action"),
                            egui::Sense::click(),
                        )
                    });
                },
            );
            output.textures_delta.clear();
            shown.unwrap()
        };
        let id = render(Vec::new()).id;
        context.memory_mut(|memory| memory.request_focus(id));
        assert!(
            render(vec![key(egui::Key::Space, true)]).clicked(),
            "menu={menu}"
        );
    }
}

#[test]
fn button_space_armed는_unmount_후_같은_id에_이어서_활성화하지_않는다() {
    let context = Context::default();
    let id = frame(&context, Vec::new(), None, true).id;
    context.memory_mut(|memory| memory.request_focus(id));
    assert!(!frame(&context, vec![key(egui::Key::Space, true)], None, true).clicked());
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            ..Default::default()
        },
        |_ui| {},
    );
    output.textures_delta.clear();
    context.memory_mut(|memory| memory.request_focus(id));
    let response = frame(&context, vec![key(egui::Key::Space, false)], None, true);
    assert_eq!(response.id, id);
    assert!(!response.clicked());
}
