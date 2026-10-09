use eframe::egui::{self, Event, Key};

pub(crate) struct Fullscreen {
    applied: bool,
}

impl Fullscreen {
    pub(crate) fn new(zen: bool, enabled: bool) -> Self {
        Self {
            applied: zen && enabled,
        }
    }

    pub(crate) fn reconcile(&mut self, context: &egui::Context, zen: bool, enabled: bool) {
        let desired = zen && enabled;
        if self.applied == desired {
            return;
        }
        self.applied = desired;
        context.send_viewport_cmd(egui::ViewportCommand::Fullscreen(desired));
    }
}

pub(crate) fn escape(context: &egui::Context, zen: bool, enabled: bool, composing: bool) -> bool {
    if !zen
        || !enabled
        || composing
        || egui::Popup::is_any_open(context)
        || context.input(|input| {
            !input.raw.focused
                || input
                    .raw
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::Ime(_)))
        })
    {
        return false;
    }
    context.input_mut(|input| {
        let mut claimed = false;
        input.events.retain(|event| {
            if matches!(
                event,
                Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                }
            ) {
                claimed = true;
                return false;
            }
            true
        });
        claimed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use taide_model::{
        app_event::AppEvent,
        ids::{PaneId, ProjectId, TabId},
        paths::AppPaths,
    };
    use taide_native_editor::{
        store::{EditorLimits, EditorStore},
        view::{Selection, SelectionSet, ViewKey},
    };
    use taide_native_ui::{
        commands::{ShellIntent, ShellMutation},
        editor_surface::{EditorAppearance, NativeEditor},
        snapshot::ShellSnapshot,
    };
    use taide_runtime::{AppState, EventSink};

    const SCREEN: [f32; 2] = [640.0, 240.0];
    const FONT_SIZE: f32 = 14.0;
    const LINE_HEIGHT: f32 = 21.0;
    const PADDING: f32 = 8.0;
    const DOCUMENT_BYTES: usize = 1024;
    const SELECTED_BYTES: usize = 3;
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
    fn raw(events: Vec<Event>) -> egui::RawInput {
        egui::RawInput {
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(SCREEN[0], SCREEN[1]),
            )),
            ..Default::default()
        }
    }
    fn key(key: Key, command: bool) -> Event {
        Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                mac_cmd: command,
                command,
                ..Default::default()
            },
        }
    }
    fn finish(output: &mut egui::FullOutput) -> Vec<bool> {
        output.textures_delta.clear();
        output
            .viewport_output
            .values()
            .flat_map(|viewport| &viewport.commands)
            .filter_map(|command| {
                let egui::ViewportCommand::Fullscreen(value) = command else {
                    return None;
                };
                Some(*value)
            })
            .collect()
    }

    #[tokio::test]
    async fn zen_keymap은_window상태_남은_escape와_optin_fullscreen전환을_보존한다() {
        let directory =
            Directory(std::env::temp_dir().join(format!("taide-zen-keymap-{}", ProjectId::new())));
        std::fs::create_dir_all(&directory.0).unwrap();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        state.session.write().window_chrome.sidebar_rail_collapsed = true;
        let snapshot = ShellSnapshot::read(&state).await;
        assert!(snapshot.focused_project().is_none());
        assert!(crate::shell_keymap::supports("toggle-zen-mode"));
        let Some(ShellIntent::Mutate(mutation)) =
            crate::shell_keymap::action("toggle-zen-mode", &snapshot)
        else {
            panic!("expected window mutation")
        };
        taide_native_ui::commands::dispatch(&Sink, &state, mutation)
            .await
            .unwrap();
        assert!(state.session.read().window_chrome.zen);
        assert!(state.session.read().window_chrome.sidebar_rail_collapsed);
        let context = egui::Context::default();
        let mut views = crate::terminal_surface::Views::default();
        let mut actions = Vec::new();
        for event in [key(Key::K, true), key(Key::Z, false)] {
            let mut output = context.run_ui(raw(vec![event]), |ui| {
                let events = ui.input_mut(|input| std::mem::take(&mut input.events));
                for (index, event) in events.iter().enumerate() {
                    assert!(
                        views
                            .route_keymap(
                                crate::keymap::Route {
                                    context: ui.ctx(),
                                    event,
                                    index,
                                    scope: Default::default(),
                                    composing: false,
                                    overrides: None,
                                },
                                &mut actions,
                                false
                            )
                            .unwrap()
                    );
                }
            });
            finish(&mut output);
        }
        assert_eq!(actions, ["toggle-zen-mode"]);
        let mut editor_views = crate::terminal_surface::Views::default();
        let mut actions = Vec::new();
        for event in [key(Key::K, true), key(Key::Z, false)] {
            let mut output = context.run_ui(raw(vec![event]), |ui| {
                let events = ui.input_mut(|input| std::mem::take(&mut input.events));
                for (index, event) in events.iter().enumerate() {
                    editor_views
                        .route_keymap(
                            crate::keymap::Route {
                                context: ui.ctx(),
                                event,
                                index,
                                scope: crate::keymap::Context {
                                    editor: true,
                                    terminal: false,
                                },
                                composing: false,
                                overrides: None,
                            },
                            &mut actions,
                            false,
                        )
                        .unwrap();
                }
            });
            finish(&mut output);
        }
        assert!(actions.is_empty());
        let mut fullscreen = Fullscreen::new(false, false);
        for (zen, enabled, expected) in [
            (false, false, None),
            (true, false, None),
            (true, true, Some(true)),
            (true, true, None),
            (true, false, Some(false)),
            (true, true, Some(true)),
            (false, true, Some(false)),
        ] {
            let mut output = context.run_ui(raw(Vec::new()), |ui| {
                fullscreen.reconcile(ui.ctx(), zen, enabled)
            });
            assert_eq!(
                finish(&mut output),
                expected.into_iter().collect::<Vec<_>>()
            );
        }
        let mut restored = Fullscreen::new(true, true);
        let mut output = context.run_ui(raw(Vec::new()), |ui| {
            restored.reconcile(ui.ctx(), true, true)
        });
        assert!(finish(&mut output).is_empty());
        for (enabled, composing) in [(false, false), (true, true)] {
            let mut output = context.run_ui(raw(vec![key(Key::Escape, false)]), |ui| {
                assert!(!escape(ui.ctx(), true, enabled, composing))
            });
            finish(&mut output);
        }
        for events in [Vec::new(), vec![key(Key::Escape, false)]] {
            let mut output = context.run_ui(raw(events), |ui| {
                let modal =
                    egui::Modal::new(egui::Id::new("zen-fixture-modal")).show(ui.ctx(), |ui| {
                        ui.label("synthetic modal");
                    });
                if ui.input(|input| !input.events.is_empty()) {
                    assert!(modal.should_close());
                }
                assert!(!escape(ui.ctx(), true, true, false));
            });
            finish(&mut output);
        }
        let mut output = context.run_ui(raw(Vec::new()), |_| {});
        finish(&mut output);
        let tab = TabId::new();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let document = store
            .open_untitled(tab.clone(), "synthetic zen", "plaintext".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "zen-keymap".into(),
                    pane: PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        let editor = NativeEditor {
            appearance: EditorAppearance {
                font: egui::FontId::monospace(FONT_SIZE),
                line_height: LINE_HEIGHT,
                horizontal_padding: PADDING,
                background: egui::Color32::BLACK,
                foreground: egui::Color32::WHITE,
                muted: egui::Color32::GRAY,
                selection: egui::Color32::BLUE,
                cursor: egui::Color32::WHITE,
                current_line: egui::Color32::BLACK,
                line_numbers: false,
                indent: "\t".into(),
            },
        };
        let mut output = context.run_ui(raw(Vec::new()), |ui| {
            editor
                .show_with_keymap(ui, &mut store, view, true, |_, _, _| false)
                .unwrap();
        });
        finish(&mut output);
        let primary = Selection {
            anchor: 0,
            head: SELECTED_BYTES,
        };
        let current = store.views().get(view).unwrap().clone();
        store
            .set_view_state(
                view,
                SelectionSet {
                    primary: 1,
                    selections: vec![
                        Selection {
                            anchor: SELECTED_BYTES + 1,
                            head: SELECTED_BYTES + 1,
                        },
                        primary,
                    ],
                },
                current.scroll,
                current.folds,
            )
            .unwrap();
        for expected in [
            primary,
            Selection {
                anchor: SELECTED_BYTES,
                head: SELECTED_BYTES,
            },
        ] {
            let mut output = context.run_ui(raw(vec![key(Key::Escape, false)]), |ui| {
                editor
                    .show_with_keymap(ui, &mut store, view, true, |_, _, _| false)
                    .unwrap();
                assert!(!escape(ui.ctx(), true, true, false));
            });
            finish(&mut output);
            assert_eq!(
                store.views().get(view).unwrap().selection.selections,
                [expected]
            );
        }
        let mut exits = Vec::new();
        let mut output = context.run_ui(raw(vec![key(Key::Escape, false)]), |ui| {
            editor
                .show_with_keymap(ui, &mut store, view, true, |_, _, _| false)
                .unwrap();
            if escape(ui.ctx(), true, true, false) {
                exits.push(false);
            }
        });
        finish(&mut output);
        assert_eq!(exits, [false]);
        assert_eq!(store.documents().snapshot(document).unwrap().revision, 0);
        assert_eq!(
            store
                .documents()
                .snapshot(document)
                .unwrap()
                .rope
                .to_string(),
            "synthetic zen"
        );
        taide_native_ui::commands::dispatch(
            &Sink,
            &state,
            ShellMutation::SetWindowChrome(taide_model::project::WindowChromePatch {
                zen: Some(false),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
        assert!(!state.session.read().window_chrome.zen);
        assert!(state.session.read().window_chrome.sidebar_rail_collapsed);
    }

    #[test]
    fn zen_ime는_preedit_commit_frame의_escape를_전역으로_전달하지_않는다() {
        let context = egui::Context::default();
        for event in [
            Event::Ime(egui::ImeEvent::Preedit {
                text: "synthetic".into(),
                active_range_chars: None,
            }),
            Event::Ime(egui::ImeEvent::Commit(String::new())),
        ] {
            let mut output = context.run_ui(raw(vec![event, key(Key::Escape, false)]), |ui| {
                assert!(!escape(ui.ctx(), true, true, false));
            });
            finish(&mut output);
        }
    }

    #[test]
    fn zen_modal_exit은_직전_frame_modal의_메모리로_새_escape를_차단하지_않는다() {
        let context = egui::Context::default();
        let mut output = context.run_ui(raw(Vec::new()), |ui| {
            egui::Modal::new(egui::Id::new("zen-closing-modal")).show(ui.ctx(), |ui| {
                ui.label("synthetic closing modal");
            });
        });
        finish(&mut output);
        let mut claimed = false;
        let mut output = context.run_ui(raw(vec![key(Key::Escape, false)]), |ui| {
            assert!(ui.ctx().memory(|memory| memory.top_modal_layer().is_some()));
            claimed = escape(ui.ctx(), true, true, false);
        });
        finish(&mut output);
        assert!(claimed);
    }
}
