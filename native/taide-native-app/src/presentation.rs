pub use taide_native_ui::presentation::{color, editor_appearance, message, shell_colors};
pub(crate) use taide_native_ui::presentation::{
    next_editor_font_size, parse_color, update_editor_font_size,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{HostBridge, HostCommand, HostReply};
    use eframe::egui::{self, Color32, Event, FontId, Key};
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
        time::Duration,
    };
    use taide_model::{
        app_event::AppEvent,
        ids::{ProjectId, TabId},
        paths::AppPaths,
        settings::Settings,
    };
    use taide_native_editor::{
        store::{EditorLimits, EditorStore},
        view::ViewKey,
    };
    use taide_native_ui::presentation::{
        EDITOR_LINE_HEIGHT_FACTOR, EDITOR_PADDING, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE,
    };
    use taide_native_ui::{
        commands::ShellIntent,
        editor_surface::{EditorAppearance, NativeEditor},
        snapshot::ShellSnapshot,
    };
    use taide_runtime::{AppState, EventSink, TaskSupervisor};
    use tokio::sync::Notify;

    const DEADLINE: Duration = Duration::from_secs(3);
    const DOCUMENT_BYTES: usize = 1024;
    const SCREEN: [f32; 2] = [640.0, 240.0];
    const TEXT: &str = "synthetic font";

    #[derive(Default)]
    struct Sink(Mutex<Vec<Settings>>);
    impl EventSink for Sink {
        fn publish(&self, event: AppEvent) {
            if let AppEvent::SettingsChanged { settings } = event {
                self.0.lock().unwrap().push(*settings);
            }
        }
    }
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[tokio::test]
    async fn keybinding_host는_실제_settings파일_이벤트와_실패시_이전값을_보존한다() {
        use crate::keymap::catalog::{Binding, Overrides};
        use serde_json::json;

        let directory = Directory(
            std::env::temp_dir().join(format!("taide-keybinding-host-{}", ProjectId::new())),
        );
        std::fs::create_dir_all(&directory.0).unwrap();
        let state = AppState::new(AppPaths::new(directory.0.join("data")));
        let initial = state.settings.read().clone();
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let events = Arc::new(Sink::default());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), events.clone());
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge = HostBridge::connect_with_clipboard_ports(
            services,
            Arc::new(move || signal.notify_one()),
            Arc::new(|_| panic!("clipboard write is not allowed")),
            Arc::new(|| panic!("clipboard read is not allowed")),
            None,
        )
        .unwrap();
        let binding =
            Binding::parse(&json!({"key":"j","mods":["mod"],"chord":{"key":"s","mods":[]}}))
                .unwrap();
        let original = Overrides::parse(Some(
            r#"[{"actionId":"future.action","key":"r","mods":["ctrl"],"future":{"preserve":true}}]"#,
        ));
        let assigned = original.assign("save", &binding);
        let unbound = assigned.unbind("save");
        let reset = unbound.reset("save");
        for overrides in [assigned, unbound, reset, Overrides::default()] {
            let json = overrides.json().unwrap();
            bridge
                .submit(HostCommand::SetKeymapOverrides(overrides))
                .unwrap();
            tokio::time::timeout(DEADLINE, ready.notified())
                .await
                .unwrap();
            assert!(bridge.poll().is_none());
            let mut expected = initial.clone();
            expected.keymap_overrides = Some(json);
            assert_eq!(*state.settings.read(), expected);
            let persisted: Settings =
                serde_json::from_slice(&std::fs::read(state.paths.settings_file()).unwrap())
                    .unwrap();
            assert_eq!(persisted, expected);
            assert_eq!(events.0.lock().unwrap().last(), Some(&expected));
        }
        let before = state.settings.read().clone();
        let count = events.0.lock().unwrap().len();
        let settings_file = state.paths.settings_file();
        std::fs::rename(&settings_file, directory.0.join("previous-settings.json")).unwrap();
        std::fs::create_dir(&settings_file).unwrap();
        bridge
            .submit(HostCommand::SetKeymapOverrides(
                original.assign("save", &binding),
            ))
            .unwrap();
        tokio::time::timeout(DEADLINE, ready.notified())
            .await
            .unwrap();
        assert!(matches!(bridge.poll(), Some(HostReply::SettingsFailed(_))));
        assert_eq!(*state.settings.read(), before);
        assert_eq!(events.0.lock().unwrap().len(), count);
        tokio::time::timeout(DEADLINE, bridge.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[tokio::test]
    async fn font_keymap은_전역단계_저장실패_이벤트와_editor표시만_변경한다() {
        let directory =
            Directory(std::env::temp_dir().join(format!("taide-font-keymap-{}", ProjectId::new())));
        std::fs::create_dir_all(&directory.0).unwrap();
        let paths = AppPaths::new(directory.0.join("data"));
        let state = AppState::new(paths);
        let initial = state.settings.read().clone();
        let snapshot = ShellSnapshot::read(&state).await;
        for (id, increase) in [("font-size-up", true), ("font-size-down", false)] {
            assert!(crate::shell_keymap::supports(id));
            assert!(
                matches!(crate::shell_keymap::action(id, &snapshot), Some(ShellIntent::ChangeEditorFontSize { increase: value }) if value == increase)
            );
        }
        assert_eq!(
            next_editor_font_size(MAX_CODE_FONT_SIZE, true),
            MAX_CODE_FONT_SIZE
        );
        assert_eq!(
            next_editor_font_size(MIN_CODE_FONT_SIZE, false),
            MIN_CODE_FONT_SIZE
        );
        assert_eq!(next_editor_font_size(u32::MAX, true), MAX_CODE_FONT_SIZE);
        assert_eq!(next_editor_font_size(0, false), MIN_CODE_FONT_SIZE);
        let context = egui::Context::default();
        let mut views = crate::terminal_surface::Views::default();
        for (key, id) in [
            (Key::Equals, "font-size-up"),
            (Key::Minus, "font-size-down"),
        ] {
            let mut actions = Vec::new();
            let mut output = context.run_ui(
                egui::RawInput {
                    events: vec![Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers {
                            mac_cmd: true,
                            command: true,
                            ..Default::default()
                        },
                    }],
                    ..Default::default()
                },
                |ui| {
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
                },
            );
            output.textures_delta.clear();
            assert_eq!(actions, [id]);
        }
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let events = Arc::new(Sink::default());
        let services = crate::bootstrap::services(state.clone(), tasks.clone(), events.clone());
        let ready = Arc::new(Notify::new());
        let signal = ready.clone();
        let mut bridge = HostBridge::connect_with_clipboard_ports(
            services,
            Arc::new(move || signal.notify_one()),
            Arc::new(|_| panic!("clipboard write is not allowed")),
            Arc::new(|| panic!("clipboard read is not allowed")),
            None,
        )
        .unwrap();
        for size in [
            next_editor_font_size(initial.editor_font_size, true),
            initial.editor_font_size,
            MAX_CODE_FONT_SIZE,
            MIN_CODE_FONT_SIZE,
        ] {
            bridge.submit(HostCommand::SetEditorFontSize(size)).unwrap();
            tokio::time::timeout(DEADLINE, ready.notified())
                .await
                .unwrap();
            assert!(bridge.poll().is_none());
            let current = state.settings.read().clone();
            let mut expected = initial.clone();
            expected.editor_font_size = size;
            assert_eq!(current, expected);
            let persisted: Settings =
                serde_json::from_slice(&std::fs::read(state.paths.settings_file()).unwrap())
                    .unwrap();
            assert_eq!(persisted, expected);
            assert_eq!(events.0.lock().unwrap().last(), Some(&expected));
        }
        let tab = TabId::new();
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: 1,
            max_undo_groups: 1,
            max_document_bytes: DOCUMENT_BYTES,
        })
        .unwrap();
        let document = store
            .open_untitled(tab.clone(), TEXT, "plaintext".into())
            .unwrap();
        let view = store
            .attach_view(
                ViewKey {
                    window: "font-keymap".into(),
                    pane: taide_model::ids::PaneId::new(),
                    tab,
                },
                document,
            )
            .unwrap();
        let revision = store.documents().snapshot(document).unwrap().revision;
        let selection = store.views().get(view).unwrap().selection.clone();
        let mut editor = NativeEditor {
            appearance: EditorAppearance {
                font: FontId::monospace(initial.editor_font_size as f32),
                line_height: initial.editor_font_size as f32 * EDITOR_LINE_HEIGHT_FACTOR,
                horizontal_padding: EDITOR_PADDING,
                background: Color32::BLACK,
                foreground: Color32::WHITE,
                muted: Color32::GRAY,
                selection: Color32::BLUE,
                cursor: Color32::WHITE,
                current_line: Color32::BLACK,
                line_numbers: false,
                indent: "\t".into(),
            },
        };
        let mut heights = Vec::new();
        for size in [MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE] {
            assert!(update_editor_font_size(&mut editor.appearance, size));
            assert!(!update_editor_font_size(&mut editor.appearance, size));
            assert_eq!(
                editor.appearance.line_height,
                size as f32 * EDITOR_LINE_HEIGHT_FACTOR
            );
            assert_eq!(editor.appearance.font.family, egui::FontFamily::Monospace);
            assert_eq!(editor.appearance.foreground, Color32::WHITE);
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(SCREEN[0], SCREEN[1]),
                    )),
                    ..Default::default()
                },
                |ui| {
                    editor
                        .show_with_keymap(ui, &mut store, view, false, |_, _, _| false)
                        .unwrap();
                },
            );
            output.textures_delta.clear();
            let height = output
                .shapes
                .iter()
                .find_map(|shape| {
                    let egui::Shape::Text(text) = &shape.shape else {
                        return None;
                    };
                    (text.galley.text() == TEXT).then(|| text.galley.size().y)
                })
                .expect("expected native editor text shape");
            heights.push(height);
            assert_eq!(
                store.documents().snapshot(document).unwrap().revision,
                revision
            );
            assert_eq!(
                store
                    .documents()
                    .snapshot(document)
                    .unwrap()
                    .rope
                    .to_string(),
                TEXT
            );
            assert_eq!(store.views().get(view).unwrap().selection, selection);
        }
        assert!(heights[0] > heights[1]);
        let before = state.settings.read().clone();
        let count = events.0.lock().unwrap().len();
        let settings_file = state.paths.settings_file();
        std::fs::rename(&settings_file, directory.0.join("previous-settings.json")).unwrap();
        std::fs::create_dir(&settings_file).unwrap();
        bridge
            .submit(HostCommand::SetEditorFontSize(MAX_CODE_FONT_SIZE))
            .unwrap();
        tokio::time::timeout(DEADLINE, ready.notified())
            .await
            .unwrap();
        assert!(matches!(bridge.poll(), Some(HostReply::SettingsFailed(_))));
        assert_eq!(*state.settings.read(), before);
        assert_eq!(events.0.lock().unwrap().len(), count);
        tokio::time::timeout(DEADLINE, bridge.disconnect())
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(DEADLINE, tasks.shutdown())
            .await
            .unwrap();
        assert_eq!(tasks.tracked_count(), 0);
    }
}
