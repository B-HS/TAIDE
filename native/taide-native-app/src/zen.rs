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

/// Original `ZEN_HINT_VISIBLE_DURATION_MS`.
pub(crate) const HINT_DURATION: std::time::Duration = std::time::Duration::from_millis(3000);
const HINT_TOP: f32 = 12.0;
const HINT_PADDING: egui::Vec2 = egui::vec2(12.0, 6.0);
const HINT_GAP: f32 = 8.0;
const HINT_FONT_SIZE: f32 = 12.0;
const HINT_LINE_HEIGHT: f32 = 16.0;
const HINT_RADIUS: u8 = 6;
const HINT_BORDER: f32 = 1.0;
const HINT_SHADOW_OFFSET: [i8; 2] = [0, 10];
const HINT_SHADOW_BLUR: u8 = 15;
const HINT_SHADOW_ALPHA: u8 = 26;

pub(crate) struct HintColors {
    background: egui::Color32,
    border: egui::Color32,
    foreground: egui::Color32,
    muted: egui::Color32,
}

impl HintColors {
    pub(crate) fn new(
        theme: &taide_model::theme::ResolvedTheme,
    ) -> taide_model::error::AppResult<Self> {
        let color = |key| crate::presentation::color(theme, key);
        Ok(Self {
            background: color("panel.background")?,
            border: color("app.border")?,
            foreground: color("app.foreground")?,
            muted: color("appSidebar.iconDefault")?,
        })
    }
}

/// The Zen entry banner. Each transition into Zen restarts one 3s visible period like the
/// original banner remounting; leaving Zen hides it immediately.
#[derive(Default)]
pub(crate) struct Hint {
    started: Option<std::time::Instant>,
    zen: bool,
}

impl Hint {
    pub(crate) fn show(
        &mut self,
        ui: &egui::Ui,
        rect: egui::Rect,
        zen: bool,
        now: std::time::Instant,
        colors: &HintColors,
        texts: (&str, &str),
    ) -> Option<egui::Rect> {
        if zen != self.zen {
            self.zen = zen;
            self.started = zen.then_some(now);
        }
        let started = self.started?;
        let elapsed = now.saturating_duration_since(started);
        if elapsed >= HINT_DURATION {
            self.started = None;
            return None;
        }
        ui.ctx().request_repaint_after(HINT_DURATION - elapsed);
        let layout = |text: &str, family: egui::FontFamily, color| {
            let mut job = egui::text::LayoutJob::single_section(
                text.to_owned(),
                egui::TextFormat {
                    font_id: egui::FontId::new(HINT_FONT_SIZE, family),
                    line_height: Some(HINT_LINE_HEIGHT),
                    color,
                    ..Default::default()
                },
            );
            job.wrap.max_rows = 1;
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let title = layout(
            texts.0,
            taide_native_ui::font_families::medium(ui),
            colors.foreground,
        );
        let exit = layout(texts.1, egui::FontFamily::Proportional, colors.muted);
        let content = egui::vec2(
            title.size().x + HINT_GAP + exit.size().x,
            title.size().y.max(exit.size().y),
        );
        let size = content + HINT_PADDING * 2.0 + egui::Vec2::splat(HINT_BORDER * 2.0);
        let banner = egui::Rect::from_min_size(
            egui::pos2(rect.center().x - size.x / 2.0, rect.top() + HINT_TOP),
            size,
        );
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("native-zen-hint"),
        ));
        painter.add(
            egui::epaint::Shadow {
                offset: HINT_SHADOW_OFFSET,
                blur: HINT_SHADOW_BLUR,
                spread: 0,
                color: egui::Color32::from_black_alpha(HINT_SHADOW_ALPHA),
            }
            .as_shape(banner, HINT_RADIUS),
        );
        painter.rect(
            banner,
            HINT_RADIUS,
            colors.background,
            egui::Stroke::new(HINT_BORDER, colors.border),
            egui::StrokeKind::Inside,
        );
        let origin = banner.min + HINT_PADDING + egui::Vec2::splat(HINT_BORDER);
        let title_width = title.size().x;
        painter.galley(origin, title, colors.foreground);
        painter.galley(
            origin + egui::vec2(title_width + HINT_GAP, 0.0),
            exit,
            colors.muted,
        );
        Some(banner)
    }
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

    #[test]
    fn zen_안내는_진입마다_3초동안_상단중앙에_보이고_종료하면_사라진다() {
        let app = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-zen-hint-{}", ProjectId::new())),
        ));
        let theme = taide_runtime::theme_actions::theme_get(&app, "taide-dark".into()).unwrap();
        let colors = HintColors::new(&theme).unwrap();
        let context = egui::Context::default();
        let mut hint = Hint::default();
        let start = std::time::Instant::now();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(SCREEN[0], SCREEN[1]));
        let mut run = |hint: &mut Hint, zen: bool, at: std::time::Instant| {
            let mut banner = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    banner = hint.show(
                        ui,
                        ui.max_rect(),
                        zen,
                        at,
                        &colors,
                        ("Zen Mode", "Press Esc to exit"),
                    )
                },
            );
            output.textures_delta.clear();
            let texts = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            (banner, texts, output.platform_output.cursor_icon)
        };
        let (banner, texts, _) = run(&mut hint, false, start);
        assert!(banner.is_none() && texts.is_empty());
        let (banner, texts, _) = run(&mut hint, true, start);
        let banner = banner.unwrap();
        assert_eq!(texts, ["Zen Mode", "Press Esc to exit"]);
        assert!((banner.center().x - SCREEN[0] / 2.0).abs() < 0.5);
        assert_eq!(banner.top(), HINT_TOP);
        let almost = start + HINT_DURATION - std::time::Duration::from_millis(1);
        assert!(run(&mut hint, true, almost).0.is_some());
        assert!(run(&mut hint, true, start + HINT_DURATION).0.is_none());
        assert!(run(&mut hint, true, start + HINT_DURATION * 2).0.is_none());
        let exit = start + HINT_DURATION * 3;
        assert!(run(&mut hint, false, exit).0.is_none());
        assert!(run(&mut hint, true, exit).0.is_some());
        assert!(run(&mut hint, false, exit).0.is_none());
    }
}
