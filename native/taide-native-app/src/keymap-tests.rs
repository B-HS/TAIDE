#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG_COUNT: usize = 41;
    const BEFORE_TIMEOUT: Duration = Duration::from_millis(4999);

    fn key(key: &str, is_mac: bool) -> KeyEvent<'_> {
        KeyEvent {
            key,
            code: None,
            modifiers: Modifiers {
                meta: is_mac,
                control: !is_mac,
                ..Default::default()
            },
            repeat: false,
            composing: false,
        }
    }

    #[test]
    fn keymap_catalog와_override는_첫_우선순위_scope_chord제거와_legacy를_보존한다() {
        let mut map = Keymap::new().unwrap();
        assert_eq!(map.entries.len(), CATALOG_COUNT);
        let now = Instant::now();
        let terminal = Context {
            terminal: true,
            editor: false,
        };
        for is_mac in [true, false] {
            assert_eq!(
                map.decide(&key("ArrowUp", is_mac), terminal, is_mac, now),
                Decision::Dispatch("terminal-jump-to-previous-command".into())
            );
            assert_eq!(
                map.decide(&key("ArrowUp", is_mac), Context::default(), is_mac, now),
                Decision::None
            );
        }
        map.update(Some(r#"[{"actionId":"terminal-jump-to-previous-command","key":"j","mods":["mod"]},{"actionId":"terminal-jump-to-previous-command","key":"u","mods":["mod"]}]"#));
        assert_eq!(
            map.decide(&key("ArrowUp", true), terminal, true, now),
            Decision::None
        );
        assert_eq!(
            map.decide(&key("j", true), terminal, true, now),
            Decision::Dispatch("terminal-jump-to-previous-command".into())
        );
        assert_eq!(
            map.decide(&key("u", true), terminal, true, now),
            Decision::None
        );
        map.update(Some(
            r#"[{"actionId":"terminal-jump-to-previous-command","key":"p","mods":["mod"]}]"#,
        ));
        assert_eq!(
            map.decide(&key("p", true), terminal, true, now),
            Decision::Dispatch("quick-open".into())
        );
        map.update(Some(r#"[{"actionId":"keybindings.open","key":"j","mods":["mod"],"chord":{"key":1,"mods":[]}}]"#));
        assert_eq!(
            map.decide(&key("j", true), terminal, true, now),
            Decision::Dispatch("open-keybindings-editor".into())
        );
        map.update(Some("not json"));
        assert_eq!(
            map.decide(&key("j", true), terminal, true, now),
            Decision::None
        );
        map.update(None);
        assert_eq!(
            map.decide(&key("ArrowUp", true), terminal, true, now),
            Decision::Dispatch("terminal-jump-to-previous-command".into())
        );
    }

    #[test]
    fn keymap_chord는_형제_두단계_삼킴_repeat_ime_editor_유예와_timeout을_보존한다() {
        let mut map = Keymap::new().unwrap();
        let now = Instant::now();
        let context = Context::default();
        let terminal = Context {
            terminal: true,
            editor: false,
        };
        assert_eq!(
            map.decide(&key("k", true), terminal, true, now),
            Decision::None
        );
        assert_eq!(
            map.decide(&key("k", true), context, true, now),
            Decision::EnterChord
        );
        let mut repeat = key("k", true);
        repeat.repeat = true;
        assert_eq!(map.decide(&repeat, context, true, now), Decision::Ignore);
        let mut ime = key("s", false);
        ime.modifiers.control = false;
        ime.composing = true;
        assert_eq!(map.decide(&ime, context, true, now), Decision::Ignore);
        assert_eq!(
            map.decide(
                &key("ArrowRight", true),
                terminal,
                true,
                now + BEFORE_TIMEOUT
            ),
            Decision::ResolveChord("focus-group-right".into())
        );
        assert_eq!(
            map.decide(&key("k", true), context, true, now),
            Decision::EnterChord
        );
        assert_eq!(
            map.decide(&key("q", true), context, true, now),
            Decision::NoMatch
        );
        assert_eq!(
            map.decide(&key("k", true), context, true, now),
            Decision::EnterChord
        );
        assert_eq!(
            map.decide(&key("s", true), context, true, now + CHORD_TIMEOUT),
            Decision::Dispatch("save".into())
        );
        let editor = Context {
            editor: true,
            terminal: false,
        };
        assert_eq!(
            map.decide(&key("k", true), editor, true, now),
            Decision::ObserveEditorPrefix
        );
        assert_eq!(map.decide(&repeat, editor, true, now), Decision::Ignore);
        assert_eq!(
            map.decide(&key("p", true), editor, true, now),
            Decision::DeferToEditor
        );
        assert_eq!(
            map.decide(&key("p", true), editor, true, now),
            Decision::Dispatch("quick-open".into())
        );
    }

    #[test]
    fn keymap_physical키와_정확한_mod_ime_명령조합을_보존한다() {
        let mut map = Keymap::new().unwrap();
        let now = Instant::now();
        let mut event = key("π", true);
        event.code = Some("KeyP");
        assert_eq!(
            map.decide(&event, Context::default(), true, now),
            Decision::Dispatch("quick-open".into())
        );
        event.key = "j";
        assert_eq!(
            map.decide(&event, Context::default(), true, now),
            Decision::None
        );
        event.key = "p";
        event.modifiers.alt = true;
        assert_eq!(
            map.decide(&event, Context::default(), true, now),
            Decision::None
        );
        event.modifiers.alt = false;
        event.composing = true;
        assert_eq!(
            map.decide(&event, Context::default(), true, now),
            Decision::Dispatch("quick-open".into())
        );
        event.modifiers.meta = false;
        assert_eq!(
            map.decide(&event, Context::default(), true, now),
            Decision::Ignore
        );
    }

    #[test]
    fn keymap_egui_adapter는_arrow_문자_modifier와_keyup을_구분한다() {
        use eframe::egui::{Event, Key, Modifiers as EguiModifiers};
        let mut map = Keymap::new().unwrap();
        let now = Instant::now();
        let context = Context {
            terminal: true,
            editor: false,
        };
        let modifiers = if cfg!(target_os = "macos") {
            EguiModifiers::MAC_CMD | EguiModifiers::COMMAND
        } else {
            EguiModifiers::CTRL | EguiModifiers::COMMAND
        };
        let key = |key, pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        };
        assert_eq!(
            map.decide_egui(&key(Key::ArrowUp, true), context, false, now),
            Decision::Dispatch("terminal-jump-to-previous-command".into())
        );
        assert_eq!(
            map.decide_egui(&key(Key::ArrowUp, false), context, false, now),
            Decision::None
        );
        assert_eq!(dom_key(Key::Minus), "-");
        assert_eq!(dom_key(Key::Quote), "'");
        assert_eq!(dom_key(Key::Space), " ");
        assert_eq!(dom_code(Key::Num1), "Digit1");
        assert_eq!(dom_code(Key::OpenBracket), "BracketLeft");
        assert_eq!(dom_key(Key::ShiftLeft), "Shift");
        map.update(Some(r#"[{"actionId":"terminal-jump-to-previous-command","key":"k","mods":["mod"],"chord":{"key":"j","mods":["mod"]}}]"#));
        assert_eq!(
            map.decide_egui(&key(Key::K, true), context, false, now),
            Decision::EnterChord
        );
        assert_eq!(
            map.decide_egui(&key(Key::ShiftLeft, true), context, false, now),
            Decision::Ignore
        );
        assert_eq!(
            map.decide_egui(&key(Key::J, true), context, true, now),
            Decision::ResolveChord("terminal-jump-to-previous-command".into())
        );
    }

    #[test]
    fn window_keymap은_같은_raw_이벤트의_chord_결정을_두_listener에_공유한다() {
        use eframe::egui::{Event, Key, Modifiers as EguiModifiers};
        const SCREEN: [f32; 2] = [640.0, 240.0];
        let context = eframe::egui::Context::default();
        let mut windows = Windows::default();
        let modifiers = if cfg!(target_os = "macos") {
            EguiModifiers::MAC_CMD | EguiModifiers::COMMAND
        } else {
            EguiModifiers::CTRL | EguiModifiers::COMMAND
        };
        let event = |key| Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        };
        let mut frame = |event: Event, expected: Decision| {
            let mut output = context.run_ui(
                eframe::egui::RawInput {
                    screen_rect: Some(eframe::egui::Rect::from_min_size(
                        eframe::egui::Pos2::ZERO,
                        eframe::egui::vec2(SCREEN[0], SCREEN[1]),
                    )),
                    events: vec![event],
                    ..Default::default()
                },
                |ui| {
                    let event = ui.input(|input| input.events[0].clone());
                    let mut next = 0;
                    let index = event_index(ui.ctx(), &event, &mut next);
                    assert_eq!(index, 0);
                    for scope in [
                        Context::default(),
                        Context {
                            editor: true,
                            terminal: false,
                        },
                    ] {
                        windows
                            .route(
                                Route {
                                    context: ui.ctx(),
                                    event: &event,
                                    index,
                                    scope,
                                    composing: false,
                                    overrides: None,
                                },
                                |decision| {
                                    assert_eq!(decision, &expected);
                                    false
                                },
                            )
                            .unwrap();
                    }
                },
            );
            output.textures_delta.clear();
        };
        frame(event(Key::K), Decision::EnterChord);
        frame(event(Key::K), Decision::Ignore);
        frame(
            event(Key::ArrowRight),
            Decision::ResolveChord("focus-group-right".into()),
        );
        assert_eq!(windows.windows.len(), 1);
        windows.retain(|_| false);
        assert!(windows.windows.is_empty());
    }

    #[test]
    fn local_editor_입력은_ax_focus_전후_실제_view에_전달된다() {
        verify_local_editor_input(false);
    }

    #[test]
    fn local_editor는_ax_focus_왕복에서_확정하지_않은_조합을_취소한다() {
        verify_local_editor_input(true);
    }

    fn verify_local_editor_input(is_cancel: bool) {
        use eframe::egui::{self, Color32, Event, FontId};
        use taide_model::ids::{PaneId, TabId};
        use taide_native_editor::{
            store::{EditorLimits, EditorStore},
            view::ViewKey,
        };
        use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};
        const SCREEN: egui::Vec2 = egui::vec2(640.0, 240.0);
        const FONT: f32 = 14.0;
        const LINE: f32 = 20.0;
        const PADDING: f32 = 8.0;
        const LIMIT: usize = 1024;
        const HISTORY: usize = 8;
        for reverse in [false, true] {
            let mut store = EditorStore::new(EditorLimits {
                max_documents: 2,
                max_views: 2,
                max_document_bytes: LIMIT,
                max_undo_groups: HISTORY,
            })
            .unwrap();
            let mut views = Vec::new();
            for _ in 0..2 {
                let tab = TabId::new();
                let document = store
                    .open_untitled(tab.clone(), "", "plaintext".into())
                    .unwrap();
                views.push(
                    store
                        .attach_view(
                            ViewKey {
                                window: "main".into(),
                                pane: PaneId::new(),
                                tab,
                            },
                            document,
                        )
                        .unwrap(),
                );
            }
            let editor = NativeEditor {
                appearance: EditorAppearance {
                    font: FontId::monospace(FONT),
                    line_height: LINE,
                    horizontal_padding: PADDING,
                    background: Color32::BLACK,
                    foreground: Color32::WHITE,
                    muted: Color32::GRAY,
                    selection: Color32::GRAY,
                    cursor: Color32::WHITE,
                    current_line: Color32::BLACK,
                    line_numbers: true,
                    indent: "\t".into(),
                },
            };
            let context = egui::Context::default();
            context.enable_accesskit();
            let mut render = |events| {
                let mut ids = Vec::new();
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                        ..Default::default()
                    },
                    |ui| {
                        let mut order = [0, 1];
                        if reverse {
                            order.reverse();
                        }
                        for index in order {
                            let shown = ui
                                .scope_builder(
                                    egui::UiBuilder::new()
                                        .id(egui::Id::new(("local-event-owner", index))),
                                    |ui| {
                                        editor
                                            .show_with_input_route(
                                                ui,
                                                &mut store,
                                                views[index],
                                                false,
                                                |_, _, _| false,
                                                |response| {
                                                    response.ctx.keyboard_input_route(response.id)
                                                },
                                            )
                                            .unwrap()
                                    },
                                )
                                .inner;
                            assert!(shown.errors.is_empty());
                            ids.push((index, shown.response.id));
                        }
                    },
                );
                output.textures_delta.clear();
                ids.sort_by_key(|(index, _)| *index);
                ids
            };
            let ids = render(Vec::new());
            context.memory_mut(|memory| memory.request_focus(ids[0].1));
            render(Vec::new());
            render(vec![
                Event::Ime(egui::ImeEvent::Preedit {
                    text: "old-composition".into(),
                    active_range_chars: None,
                }),
                Event::Text("suppressed".into()),
            ]);
            let focus = |index: usize| {
                Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: ids[index].1.accesskit_id(),
                    data: None,
                })
            };
            let preedit = Event::Ime(egui::ImeEvent::Preedit {
                text: "new-composition".into(),
                active_range_chars: None,
            });
            let expected = if is_cancel {
                render(vec![
                    Event::Text("suppressed".into()),
                    focus(1),
                    preedit,
                    focus(0),
                    Event::Text("returned".into()),
                ]);
                ["returned", ""]
            } else {
                render(vec![
                    Event::Ime(egui::ImeEvent::Commit("before".into())),
                    focus(1),
                    preedit,
                    Event::Text("suppressed".into()),
                    Event::Ime(egui::ImeEvent::Commit("after".into())),
                ]);
                ["before", "after"]
            };
            for (index, expected) in expected.into_iter().enumerate() {
                assert!(
                    store
                        .views()
                        .get(views[index])
                        .unwrap()
                        .composition
                        .is_none()
                );
                let document = store.views().get(views[index]).unwrap().document;
                assert_eq!(
                    store
                        .documents()
                        .snapshot(document)
                        .unwrap()
                        .rope
                        .to_string(),
                    expected,
                    "reverse={reverse} view={index}"
                );
            }
        }
    }

    #[test]
    fn editor_group은_실제_native_editor_focus에서_기본_chord를_실행한다() {
        use eframe::egui::{self, Color32, Event, FontId, Key};
        use taide_model::ids::{PaneId, TabId};
        use taide_native_editor::{
            store::{EditorLimits, EditorStore},
            view::ViewKey,
        };
        use taide_native_ui::editor_surface::{EditorAppearance, NativeEditor};
        const SCREEN: [f32; 2] = [640.0, 240.0];
        const FONT: f32 = 14.0;
        const LINE: f32 = 20.0;
        const PADDING: f32 = 8.0;
        const LIMIT: usize = 1024;
        const HISTORY: usize = 8;
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_document_bytes: LIMIT,
            max_undo_groups: HISTORY,
        })
        .unwrap();
        let tab = TabId::new();
        let document = store
            .open_untitled(tab.clone(), "", "plaintext".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "main".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        let editor = NativeEditor {
            appearance: EditorAppearance {
                font: FontId::monospace(FONT),
                line_height: LINE,
                horizontal_padding: PADDING,
                background: Color32::BLACK,
                foreground: Color32::WHITE,
                muted: Color32::GRAY,
                selection: Color32::GRAY,
                cursor: Color32::WHITE,
                current_line: Color32::BLACK,
                line_numbers: true,
                indent: "\t".into(),
            },
        };
        let context = egui::Context::default();
        let mut views = crate::terminal_surface::Views::default();
        let command = if cfg!(target_os = "macos") {
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL | egui::Modifiers::COMMAND
        };
        let key = |key, modifiers, pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        };
        {
            let mut registered_editor = None;
            let mut draw = |events: Vec<Event>, overrides: Option<&str>| {
                let mut actions = Vec::new();
                let releases = events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Key {
                            key: pressed,
                            modifiers,
                            pressed: true,
                            ..
                        } => Some(key(*pressed, *modifiers, false)),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(SCREEN[0], SCREEN[1]),
                        )),
                        events: events.into_iter().chain(releases).collect(),
                        ..Default::default()
                    },
                    |ui| {
                        let composing = store
                            .views()
                            .get(view)
                            .is_some_and(|view| view.composition.is_some());
                        views
                            .capture_window_keymap(ui.ctx(), overrides, &mut actions, true, |id| {
                                (registered_editor == Some(id)).then_some(composing)
                            })
                            .unwrap();
                        let mut next = 0;
                        let output = editor
                            .show_with_keymap(ui, &mut store, view, true, |ui, event, composing| {
                                let index = event_index(ui.ctx(), event, &mut next);
                                views
                                    .route_keymap(
                                        Route {
                                            context: ui.ctx(),
                                            event,
                                            index,
                                            scope: Context {
                                                editor: true,
                                                terminal: false,
                                            },
                                            composing,
                                            overrides,
                                        },
                                        &mut actions,
                                        true,
                                    )
                                    .unwrap()
                            })
                            .unwrap();
                        registered_editor = Some(output.response.id);
                        assert!(output.response.has_focus());
                        assert!(!output.save_requested);
                        assert!(output.errors.is_empty());
                        if ui.ctx().current_pass_index() == 0 {
                            ui.ctx()
                                .request_discard("synthetic editor group multi-pass");
                        }
                    },
                );
                output.textures_delta.clear();
                actions
            };
            assert!(draw(vec![key(Key::K, command, true)], None).is_empty());
            assert_eq!(
                draw(vec![key(Key::ArrowRight, command, true)], None),
                ["focus-group-right"]
            );
        }
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            ""
        );
    }

    #[test]
    fn editor_group_override는_후등록우선_단일키소유_ime와_cache를_보존한다() {
        use eframe::egui::{self, Event, Key};
        const SCREEN: [f32; 2] = [640.0, 240.0];
        const AFTER_TIMEOUT: Duration = Duration::from_secs(6);
        let context = egui::Context::default();
        let mut windows = Windows::default();
        let command = if cfg!(target_os = "macos") {
            egui::Modifiers::MAC_CMD | egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::CTRL | egui::Modifiers::COMMAND
        };
        let mut frame =
            |key, modifiers, overrides, composing, twice, expected: Vec<&str>, consumed| {
                let event = |pressed| Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed,
                    repeat: false,
                    modifiers,
                };
                let mut actions = Vec::new();
                let mut output = context.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(SCREEN[0], SCREEN[1]),
                        )),
                        events: vec![event(true), event(false)],
                        ..Default::default()
                    },
                    |ui| {
                        let event = ui.input(|input| input.events[0].clone());
                        let listeners = if twice { 0..=1 } else { 0..=0 };
                        for _ in listeners {
                            let handled = windows
                                .route(
                                    Route {
                                        context: ui.ctx(),
                                        event: &event,
                                        index: 0,
                                        scope: Context {
                                            editor: true,
                                            terminal: false,
                                        },
                                        composing,
                                        overrides,
                                    },
                                    |decision| match decision {
                                        Decision::Dispatch(id) | Decision::ResolveChord(id)
                                            if crate::shell_keymap::supports(id) =>
                                        {
                                            actions.push(id.clone());
                                            true
                                        }
                                        Decision::EnterChord | Decision::NoMatch => true,
                                        _ => false,
                                    },
                                )
                                .unwrap();
                            assert_eq!(handled, consumed);
                        }
                    },
                );
                output.textures_delta.clear();
                assert_eq!(actions, expected);
            };
        let collision = Some(
            r#"[{"actionId":"focus-group-left","key":"j","mods":["mod"],"chord":{"key":"h","mods":["mod"]}},{"actionId":"focus-group-right","key":"j","mods":["mod"],"chord":{"key":"h","mods":["mod"]}}]"#,
        );
        frame(Key::J, command, collision, false, true, vec![], true);
        frame(
            Key::H,
            command,
            collision,
            false,
            true,
            vec!["focus-group-right"],
            true,
        );
        let single = Some(r#"[{"actionId":"focus-group-right","key":"h","mods":["mod"]}]"#);
        frame(
            Key::H,
            command,
            single,
            false,
            false,
            vec!["focus-group-right"],
            true,
        );
        let invalid = Some(
            r#"[{"actionId":"focus-group-right","key":"j","mods":["mod"],"chord":{"key":"Unknown","mods":[]}}]"#,
        );
        frame(Key::J, command, invalid, false, false, vec![], false);
        let bare = Some(
            r#"[{"actionId":"focus-group-right","key":"j","mods":["mod"],"chord":{"key":"h","mods":[]}}]"#,
        );
        frame(Key::J, command, bare, false, true, vec![], true);
        frame(Key::H, Default::default(), bare, true, false, vec![], false);
        frame(
            Key::H,
            Default::default(),
            bare,
            false,
            true,
            vec!["focus-group-right"],
            true,
        );
        let save = Some(
            r#"[{"actionId":"focus-group-right","key":"j","mods":["mod"],"chord":{"key":"s","mods":["mod"]}}]"#,
        );
        frame(Key::J, command, save, false, true, vec![], true);
        frame(Key::S, command, save, false, false, vec!["save"], true);
        let mut map = Keymap::new().unwrap();
        let now = Instant::now();
        assert_eq!(
            map.decide_editor(&key("k", true), true, now),
            Decision::EnterChord
        );
        let mut modifier = key("Shift", true);
        modifier.modifiers = Default::default();
        assert_eq!(map.decide_editor(&modifier, true, now), Decision::Ignore);
        assert_eq!(
            map.decide_editor(&key("ArrowRight", true), true, now + AFTER_TIMEOUT),
            Decision::None
        );
        assert_eq!(
            map.decide_editor(&key("k", false), false, now),
            Decision::EnterChord
        );
        assert_eq!(
            map.decide_editor(&key("ArrowRight", false), false, now),
            Decision::ResolveChord("focus-group-right".into())
        );
        assert_eq!(
            map.decide_editor(&key("k", true), true, now),
            Decision::EnterChord
        );
        let mut repeat = key("ArrowRight", true);
        repeat.repeat = true;
        assert_eq!(
            map.decide_editor(&repeat, true, now),
            Decision::ResolveChord("focus-group-right".into())
        );
        assert_eq!(EDITOR_GROUPS.len(), 7);
        assert!(!editor_key("F13"));
        assert!(!editor_key("ß"));
        windows.retain(|_| false);
        assert!(windows.windows.is_empty());
    }
}
