use super::*;

const SCREEN: egui::Vec2 = egui::vec2(640.0, 480.0);
const FIRST: Rect = Rect::from_min_max(egui::pos2(80.0, 100.0), egui::pos2(120.0, 120.0));
const SECOND: Rect = Rect::from_min_max(egui::pos2(240.0, 100.0), egui::pos2(280.0, 120.0));
const OPENED: f64 = 1.0;
const INPUT_TIME: f64 = 1.032;
const OUTSIDE: Pos2 = egui::pos2(440.0, 300.0);
const POINTER_EVENT_COUNT: usize = 2;

#[test]
fn tooltip_document_capture는_button_window_escape_취소를_존중하고_target_소비와_구분한다() {
    for mode in ["unbound", "window", "target", "prefix"] {
        let context = Context::default();
        let provider = Provider::default();
        let appearance = Appearance {
            background: Color32::BLACK,
            border: Color32::GRAY,
            foreground: Color32::WHITE,
        };
        let mut routes = crate::terminal_surface::Views::default();
        let mut actions = Vec::new();
        let overrides = serde_json::json!([
            {"actionId": "toggle-sidebar", "key": if mode == "prefix" { "Enter" } else { "Escape" }, "mods": []}
        ]).to_string();
        let mut render = |events, overrides: Option<&str>| {
            let mut result = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    events,
                    ..Default::default()
                },
                |ui| {
                    routes
                        .capture_window_keymap(ui.ctx(), overrides, &mut actions, false, |_| None)
                        .unwrap();
                    provider.begin_frame(ui.ctx());
                    if mode == "target" {
                        ui.input_mut(|input| {
                            input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
                        });
                    }
                    let button = ui.button("Synthetic action");
                    provider.show(
                        &button,
                        "Synthetic description",
                        egui::RectAlign::BOTTOM,
                        &appearance,
                    );
                    provider.finish_frame(ui.ctx());
                    let open = provider
                        .0
                        .lock()
                        .unwrap()
                        .get(&ViewportId::ROOT)
                        .unwrap()
                        .open
                        == Some(button.id);
                    let remaining = ui.input(|input| input.events.clone());
                    result = Some((button, open, remaining));
                },
            );
            output.textures_delta.clear();
            result.unwrap()
        };
        let (button, _, _) = render(Vec::new(), None);
        context.memory_mut(|memory| memory.request_focus(button.id));
        assert!(render(Vec::new(), None).1);
        let key = |key, pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let mut events = vec![key(egui::Key::Escape, true)];
        if mode == "prefix" {
            events.insert(0, key(egui::Key::Enter, true));
            events.push(key(egui::Key::Space, false));
        }
        let (button, open, remaining) = render(
            events,
            matches!(mode, "window" | "prefix").then_some(overrides.as_str()),
        );
        assert!(!button.clicked(), "mode={mode}");
        assert_eq!(open, mode == "window", "mode={mode}");
        assert!(
            !remaining.iter().any(|event| matches!(
                event,
                Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                }
            )),
            "mode={mode}: document or window must consume Escape"
        );
        if mode == "prefix" {
            assert!(
                remaining.iter().any(|event| matches!(
                    event,
                    Event::Key {
                        key: egui::Key::Space,
                        pressed: false,
                        ..
                    }
                )),
                "consumed raw indices must not delete the neighboring surviving keyup"
            );
        }
        let expected = usize::from(matches!(mode, "window" | "prefix"));
        assert_eq!(actions.len(), expected, "mode={mode}");
    }
}

#[test]
fn tooltip_actual_button은_실행된_key_click을_후속_event_소비와_구분한다() {
    for wrapped in [false, true] {
        let context = Context::default();
        let provider = Provider::default();
        let appearance = Appearance {
            background: Color32::BLACK,
            border: Color32::GRAY,
            foreground: Color32::WHITE,
        };
        let render = |events, consume: bool| {
            let mut result = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    events,
                    ..Default::default()
                },
                |ui| {
                    provider.begin_frame(ui.ctx());
                    let (button, trigger) = if wrapped {
                        wrap_button(ui, "Button click", false, |ui| ui.button("Action"))
                    } else {
                        let button = ui.button("Action");
                        let trigger = Trigger {
                            response: button.clone(),
                            label: "Button click".into(),
                            align: egui::RectAlign::BOTTOM,
                            focus_target: None,
                        };
                        (button, trigger)
                    };
                    if consume {
                        ui.input_mut(|input| {
                            input.events.retain(|event| {
                                !matches!(
                                    event,
                                    Event::Key {
                                        key: egui::Key::Space,
                                        pressed: false,
                                        ..
                                    }
                                )
                            })
                        });
                    }
                    provider.show_triggers(std::slice::from_ref(&trigger), &appearance);
                    provider.finish_frame(ui.ctx());
                    let open = provider
                        .0
                        .lock()
                        .unwrap()
                        .get(&ViewportId::ROOT)
                        .unwrap()
                        .open
                        == Some(trigger.response.id);
                    result = Some((button, open));
                },
            );
            output.textures_delta.clear();
            result.unwrap()
        };
        let (button, _) = render(Vec::new(), false);
        context.memory_mut(|memory| memory.request_focus(button.id));
        assert!(render(Vec::new(), false).1);
        let space = |pressed| Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let (button, open) = render(vec![space(true)], false);
        assert!(!button.clicked());
        assert!(open);
        let (button, open) = render(vec![space(false)], true);
        assert!(button.clicked());
        assert!(
            !open,
            "wrapped={wrapped}: completed default click must close despite later event consumption"
        );
    }
}

#[test]
fn tooltip_기본_key_close는_후속_ax_focus의_owner와_열림세대를_덮지_않는다() {
    for key in [egui::Key::Enter, egui::Key::Space] {
        for focus_last in [false, true] {
            for reverse in [false, true] {
                let context = Context::default();
                context.enable_accesskit();
                let provider = Provider::default();
                let ids = [Id::new("default-key-first"), Id::new("default-key-second")];
                let appearance = Appearance {
                    background: Color32::BLACK,
                    border: Color32::GRAY,
                    foreground: Color32::WHITE,
                };
                let render = |time, events| {
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(time),
                            events,
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                            ..Default::default()
                        },
                        |ui| {
                            provider.begin_frame(ui.ctx());
                            let mut triggers =
                                ids.into_iter().zip([FIRST, SECOND]).collect::<Vec<_>>();
                            if reverse {
                                triggers.reverse();
                            }
                            for (id, rect) in triggers {
                                let response = ui.interact(rect, id, egui::Sense::click());
                                provider.show(
                                    &response,
                                    "Default key reference",
                                    egui::RectAlign::TOP,
                                    &appearance,
                                );
                            }
                            provider.finish_frame(ui.ctx());
                        },
                    );
                    output.textures_delta.clear();
                    output
                };
                render(0.0, Vec::new());
                context.memory_mut(|memory| memory.request_focus(ids[0]));
                render(OPENED, Vec::new());
                let focus = Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: ids[1].accesskit_id(),
                    data: None,
                });
                let event = |pressed| Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                };
                let mut events = vec![event(true)];
                if focus_last {
                    events.push(focus);
                } else {
                    events.insert(0, focus);
                }
                if key == egui::Key::Space {
                    events.push(event(false));
                }
                let output = render(INPUT_TIME, events);
                let expected = focus_last.then_some(ids[1]);
                let owners = provider.0.lock().unwrap();
                assert_eq!(
                    owners.get(&ViewportId::ROOT).unwrap().open,
                    expected,
                    "key={key:?} focus_last={focus_last} reverse={reverse}"
                );
                for id in ids {
                    let node = output
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .find(|(node_id, _)| *node_id == id.accesskit_id())
                        .unwrap();
                    assert_eq!(!node.1.described_by().is_empty(), expected == Some(id));
                }
            }
        }
    }
}

#[test]
fn tooltip_신규_trigger와_같은_pass_space_pair도_소비된_press를_배제한다() {
    for registered in [false, true] {
        for key in [egui::Key::Enter, egui::Key::Space] {
            let context = Context::default();
            let provider = Provider::default();
            let id = Id::new("cancelled-new-tooltip-key");
            let render = |time, events, cancel: bool| {
                let mut open = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        provider.begin_frame(ui.ctx());
                        if cancel {
                            ui.input_mut(|input| {
                                assert!(input.consume_key(egui::Modifiers::NONE, key))
                            });
                        }
                        let response = ui.interact(FIRST, id, egui::Sense::click());
                        open = provider.is_open(&response);
                        provider.finish_frame(ui.ctx());
                    },
                );
                output.textures_delta.clear();
                open
            };
            if registered {
                render(0.0, Vec::new(), false);
                context.memory_mut(|memory| memory.request_focus(id));
                assert!(render(OPENED, Vec::new(), false));
            } else {
                context.memory_mut(|memory| memory.request_focus(id));
            }
            let event = |pressed| Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let events = if key == egui::Key::Space {
                vec![event(true), event(false)]
            } else {
                vec![event(true)]
            };
            assert!(
                render(INPUT_TIME, events, true),
                "registered={registered} key={key:?}"
            );
        }
    }
}

#[test]
fn tooltip_소비된_enter와_space는_begin_frame_전후에도_click으로_닫지_않는다() {
    let mut failures = Vec::new();
    for consume_before_begin in [false, true] {
        for key in [egui::Key::Enter, egui::Key::Space] {
            for cancel_release in [false, true] {
                if key == egui::Key::Enter && cancel_release {
                    continue;
                }
                let context = Context::default();
                let provider = Provider::default();
                let id = Id::new("cancelled-tooltip-default-key");
                let render = |time, events, cancel: bool| {
                    let mut open = false;
                    let mut clicked = false;
                    let consume = |ui: &mut egui::Ui| {
                        if !cancel {
                            return;
                        }
                        ui.input_mut(|input| {
                            if cancel_release {
                                input.events.retain(|event| {
                                    !matches!(event, Event::Key { key: event_key, pressed: false, .. } if *event_key == key)
                                });
                            } else {
                                assert!(input.consume_key(egui::Modifiers::NONE, key));
                            }
                        });
                    };
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(time),
                            events,
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                            ..Default::default()
                        },
                        |ui| {
                            if consume_before_begin {
                                consume(ui);
                            }
                            provider.begin_frame(ui.ctx());
                            if !consume_before_begin {
                                consume(ui);
                            }
                            let response = ui.interact(FIRST, id, egui::Sense::click());
                            clicked = response.clicked();
                            open = provider.is_open(&response);
                            provider.finish_frame(ui.ctx());
                        },
                    );
                    output.textures_delta.clear();
                    (open, clicked)
                };
                render(0.0, Vec::new(), false);
                context.memory_mut(|memory| memory.request_focus(id));
                assert!(render(OPENED, Vec::new(), false).0);
                let event = |pressed| Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                };
                if cancel_release {
                    assert!(render(INPUT_TIME, vec![event(true)], false).0);
                }
                let (open, clicked) = render(INPUT_TIME, vec![event(!cancel_release)], true);
                if !open || clicked {
                    failures.push(format!("before={consume_before_begin} key={key:?} release={cancel_release} open={open} clicked={clicked}"));
                }
                if key == egui::Key::Space
                    && !cancel_release
                    && !render(INPUT_TIME, vec![event(false)], false).0
                {
                    failures.push(format!(
                        "before={consume_before_begin} cancelled Space press armed release"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn engine_외부_pointer_focus_해제는_뒤에_온_ax_focus와_설정을_보존한다() {
    use egui::SurrenderFocusOn;

    for mode in [
        SurrenderFocusOn::Presses,
        SurrenderFocusOn::Clicks,
        SurrenderFocusOn::Never,
    ] {
        for position in 0..=POINTER_EVENT_COUNT {
            let context = Context::default();
            context.enable_accesskit();
            context.options_mut(|options| options.input_options.surrender_focus_on = mode);
            let id = Id::new("ordered-engine-focus");
            let focus = Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: id.accesskit_id(),
                data: None,
            });
            let mut events = [true, false]
                .into_iter()
                .map(|pressed| Event::PointerButton {
                    pos: OUTSIDE,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                })
                .collect::<Vec<_>>();
            events.insert(position, focus);
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                    events,
                    ..Default::default()
                },
                |ui| {
                    ui.interact(SECOND, id, egui::Sense::click());
                },
            );
            output.textures_delta.clear();
            let pointer_boundary = match mode {
                SurrenderFocusOn::Presses => 0,
                SurrenderFocusOn::Clicks => 1,
                SurrenderFocusOn::Never => position,
            };
            let expected = mode == SurrenderFocusOn::Never || position > pointer_boundary;
            assert_eq!(
                context.memory(|memory| memory.has_focus(id)),
                expected,
                "mode={mode:?} position={position}"
            );
        }
    }
}

#[test]
fn tooltip_pointer_down과_ax_focus는_같은_pass의_발생순서로_닫고_연다() {
    let mut failures = Vec::new();
    for focus_last in [false, true] {
        for point in [OUTSIDE, SECOND.center()] {
            for reverse in [false, true] {
                let context = Context::default();
                context.enable_accesskit();
                let provider = Provider::default();
                let ids = [Id::new("pointer-key-first"), Id::new("pointer-key-second")];
                let appearance = Appearance {
                    background: Color32::BLACK,
                    border: Color32::GRAY,
                    foreground: Color32::WHITE,
                };
                let render = |time, events| {
                    let mut output = context.run_ui(
                        egui::RawInput {
                            time: Some(time),
                            events,
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                            ..Default::default()
                        },
                        |ui| {
                            provider.begin_frame(ui.ctx());
                            let mut triggers =
                                ids.into_iter().zip([FIRST, SECOND]).collect::<Vec<_>>();
                            if reverse {
                                triggers.reverse();
                            }
                            for (id, rect) in triggers {
                                let response = ui.interact(rect, id, egui::Sense::click());
                                provider.show(
                                    &response,
                                    "Pointer key reference",
                                    egui::RectAlign::TOP,
                                    &appearance,
                                );
                            }
                            provider.finish_frame(ui.ctx());
                        },
                    );
                    output.textures_delta.clear();
                    output
                };
                render(0.0, Vec::new());
                let focus = Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: ids[1].accesskit_id(),
                    data: None,
                });
                let down = Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                };
                let up = Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                };
                let events = if focus_last {
                    vec![down, up, focus]
                } else {
                    vec![focus, down, up]
                };
                let output = render(INPUT_TIME, events);
                let owners = provider.0.lock().unwrap();
                let viewport = owners.get(&ViewportId::ROOT).unwrap();
                let expected = focus_last.then_some(ids[1]);
                if viewport.open != expected {
                    failures.push(format!("focus_last={focus_last} point={point:?} reverse={reverse} open={:?} expected={expected:?}", viewport.open));
                }
                for id in ids {
                    let node = output
                        .platform_output
                        .accesskit_update
                        .as_ref()
                        .unwrap()
                        .nodes
                        .iter()
                        .find(|(node_id, _)| *node_id == id.accesskit_id())
                        .unwrap();
                    if !node.1.described_by().is_empty() != (expected == Some(id)) {
                        failures.push(format!("focus_last={focus_last} point={point:?} reverse={reverse} stale AX for {id:?}"));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn tooltip_key는_source_focus_escape_순서와_같은_pass의_ax_timer_소비를_보존한다() {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/quality-assurance/assets/2026-10-04-tooltip-keys-source.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in source["results"].as_array().unwrap() {
        let order = case["order"].as_str().unwrap();
        let source_open = case["samples"].as_array().unwrap().last().unwrap()["triggers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|trigger| trigger["state"] != "closed")
            .map(|trigger| trigger["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            source_open,
            if order == "escape-focus" {
                vec!["second"]
            } else {
                Vec::new()
            }
        );
        for reverse in [false, true] {
            let context = Context::default();
            context.enable_accesskit();
            let provider = Provider::default();
            let ids = [Id::new("key-first"), Id::new("key-second")];
            let appearance = Appearance {
                background: Color32::BLACK,
                border: Color32::GRAY,
                foreground: Color32::WHITE,
            };
            let render = |time, events| {
                let mut escape_reached_consumer = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        provider.begin_frame(ui.ctx());
                        escape_reached_consumer =
                            ui.input(|input| input.key_pressed(egui::Key::Escape));
                        let mut triggers = ids.into_iter().zip([FIRST, SECOND]).collect::<Vec<_>>();
                        if reverse {
                            triggers.reverse();
                        }
                        for (id, rect) in triggers {
                            let response = ui.interact(rect, id, egui::Sense::click());
                            provider.show(
                                &response,
                                "Key reference",
                                egui::RectAlign::TOP,
                                &appearance,
                            );
                        }
                        provider.finish_frame(ui.ctx());
                    },
                );
                output.textures_delta.clear();
                (output, escape_reached_consumer)
            };
            render(0.0, Vec::new());
            if order != "initial-focus-escape" {
                context.memory_mut(|memory| memory.request_focus(ids[0]));
                render(OPENED, Vec::new());
            }
            let focus = Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: ids[1].accesskit_id(),
                data: None,
            });
            let escape = Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let events = if order == "escape-focus" {
                vec![escape, focus]
            } else {
                vec![focus, escape]
            };
            let (output, escape_reached_consumer) = render(INPUT_TIME, events);
            let owners = provider.0.lock().unwrap();
            let viewport = owners.get(&ViewportId::ROOT).unwrap();
            let expected_open = (order == "escape-focus").then_some(ids[1]);
            let expected_skip =
                (order != "escape-focus").then_some(INPUT_TIME + SKIP_DELAY_SECONDS);
            if viewport.open != expected_open
                || viewport.is_delayed
                || viewport.skip_until != expected_skip
                || escape_reached_consumer
            {
                failures.push(format!("order={order} reverse={reverse} open={:?} expected={expected_open:?} delayed={} skip={:?} expected_skip={expected_skip:?} escaped={escape_reached_consumer}",
                    viewport.open, viewport.is_delayed, viewport.skip_until));
            }
            for id in ids {
                let node = output
                    .platform_output
                    .accesskit_update
                    .as_ref()
                    .unwrap()
                    .nodes
                    .iter()
                    .find(|(node_id, _)| *node_id == id.accesskit_id())
                    .unwrap();
                if !node.1.described_by().is_empty() != (expected_open == Some(id)) {
                    failures.push(format!(
                        "order={order} reverse={reverse} stale AX for {id:?}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
