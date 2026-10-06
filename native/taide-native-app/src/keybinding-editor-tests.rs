#[cfg(test)]
mod tests {
    use super::*;
    use crate::{host::HostCommand, keymap::Decision, terminal_surface::Views};
    use taide_model::{ids::ProjectId, paths::AppPaths};
    use taide_runtime::AppState;

    const SCREEN: [f32; 2] = [1000.0, 800.0];
    const ROW_FOCUS_CONTROL_LIMIT: usize = 4;
    const HEADER_FOCUS_CONTROL_COUNT: usize = 5;
    const MIN_ROW_FOCUS_CONTROL_COUNT: usize = 2;
    const INPUT_FRAME_TIME: f64 = 1.0;
    const SETTLEMENT_DELAY: f64 = 0.5;
    const BACKGROUND_FOCUS_SIDE: f32 = 8.0;
    const SETTLED_FRAME_COUNT: usize = 3;

    fn assert_focused_control_visible(context: &egui::Context) -> Id {
        let focused = context
            .memory(|memory| memory.focused())
            .expect("focus escaped modal");
        let (recorded, rect, interact_rect) = context
            .data(|data| {
                data.get_temp::<Option<(Id, egui::Rect, egui::Rect)>>(Id::new(
                    "keybinding-current-focus",
                ))
            })
            .flatten()
            .expect("focused control disappeared from current pass");
        assert_eq!(recorded, focused);
        assert!(
            interact_rect.contains_rect(rect),
            "focused control is clipped: {rect:?} / {interact_rect:?}"
        );
        focused
    }

    #[test]
    fn native_presentation_refresh는_열린_keybindings의_입력과_그림을_보존한다() {
        let context = egui::Context::default();
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        let english = locale("en");
        editor.open(&context);
        frame(&mut editor, &context, &english, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &english, raw(Vec::new()), None, true);
        editor.query = "font-size-up".into();
        editor.capture.start_row("font-size-up".into());
        editor.focus_next = true;
        let previous = editor.previous_focus;
        let state = AppState::new(AppPaths::new(std::env::temp_dir().join(format!(
            "taide-presentation-keybindings-{}",
            ProjectId::new()
        ))));
        let light =
            taide_runtime::theme_actions::theme_get(&state, "vscode-light-modern".into()).unwrap();
        let background = color(&light, "modal.background").unwrap();
        editor.set_appearance(Appearance::new(&light).unwrap());
        assert!(editor.is_open() && editor.is_capturing());
        assert_eq!(editor.query, "font-size-up");
        assert_eq!(editor.previous_focus, previous);
        let translated = locale("ko");
        let mut settled = raw(Vec::new());
        settled.time = Some(1.0);
        let (_, output) = frame(&mut editor, &context, &translated, settled, None, true);
        text_position(
            &output,
            &message(&translated, "settings.keymapEditorTitle", &[]),
        );
        let mut shapes = output
            .shapes
            .iter()
            .map(|shape| &shape.shape)
            .collect::<Vec<_>>();
        let mut has_background = false;
        while let Some(shape) = shapes.pop() {
            match shape {
                egui::Shape::Rect(rect) if rect.fill == background => has_background = true,
                egui::Shape::Vec(children) => shapes.extend(children),
                _ => (),
            }
        }
        assert!(has_background);
        assert!(editor.is_open() && editor.is_capturing());
        assert_eq!(editor.query, "font-size-up");
        assert_eq!(editor.previous_focus, previous);
    }

    fn locale(id: &str) -> ResolvedLocale {
        let json = match id {
            "en" => include_str!("../../../crates/taide-locale/resources/locales/en.json"),
            "ko" => include_str!("../../../crates/taide-locale/resources/locales/ko.json"),
            "ja" => include_str!("../../../crates/taide-locale/resources/locales/ja.json"),
            _ => unreachable!(),
        };
        ResolvedLocale {
            id: id.into(),
            name: id.into(),
            messages: serde_json::from_str(json).unwrap(),
            warnings: Vec::new(),
        }
    }

    fn theme() -> ResolvedTheme {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-keybinding-ui-{}", ProjectId::new())),
        ));
        taide_runtime::theme_actions::theme_get(&state, "vscode-dark-modern".into()).unwrap()
    }

    #[test]
    fn keybinding_theme는_모든_builtin의_정본색상키로_초기화된다() {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-keybinding-theme-{}", ProjectId::new())),
        ));
        let themes = taide_runtime::theme_actions::theme_list(&state).unwrap();
        assert!(!themes.is_empty());
        for theme in themes.into_iter().filter(|theme| theme.builtin) {
            let resolved = taide_runtime::theme_actions::theme_get(&state, theme.id).unwrap();
            Appearance::new(&resolved).unwrap();
            color(&resolved, "statusIndicator.error").unwrap();
            color(&resolved, "statusIndicator.warning").unwrap();
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
                command,
                mac_cmd: command,
                ..Default::default()
            },
        }
    }

    fn frame(
        editor: &mut Editor,
        context: &egui::Context,
        locale: &ResolvedLocale,
        raw: egui::RawInput,
        overrides: Option<&str>,
        enabled: bool,
    ) -> (Output, egui::FullOutput) {
        let mut result = Output::default();
        let mut drawing = context.run_ui(raw, |ui| {
            let output = editor.show(ui.ctx(), locale, overrides, enabled).unwrap();
            result.saves.extend(output.saves);
            result.warnings.extend(output.warnings);
            result.started_capture |= output.started_capture;
            result.tooltip_triggers.extend(output.tooltip_triggers);
            let current_focus = context
                .memory(|memory| memory.focused())
                .and_then(|id| context.read_response(id))
                .map(|response| (response.id, response.rect, response.interact_rect));
            context.data_mut(|data| {
                data.insert_temp(Id::new("keybinding-current-focus"), current_focus)
            });
        });
        drawing.textures_delta.clear();
        (result, drawing)
    }

    fn text_position(output: &egui::FullOutput, value: &str) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                let egui::Shape::Text(text) = &shape.shape else {
                    return None;
                };
                (text.galley.text() == value).then(|| text.pos + text.galley.size() * 0.5)
            })
            .unwrap_or_else(|| panic!("missing rendered text: {value}"))
    }

    fn icon_position(output: &egui::FullOutput, texture: egui::TextureId) -> egui::Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.texture_id == texture => {
                    Some(mesh.calc_bounds().center())
                }
                egui::Shape::Rect(rect)
                    if rect
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == texture) =>
                {
                    Some(rect.rect.center())
                }
                _ => None,
            })
            .expect("expected rendered keybinding icon")
    }

    fn click(
        editor: &mut Editor,
        context: &egui::Context,
        locale: &ResolvedLocale,
        position: egui::Pos2,
        overrides: Option<&str>,
    ) -> Output {
        let pointer = |pressed| {
            vec![
                Event::PointerMoved(position),
                Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        let (first, _) = frame(editor, context, locale, raw(pointer(true)), overrides, true);
        let (mut second, _) = frame(
            editor,
            context,
            locale,
            raw(pointer(false)),
            overrides,
            true,
        );
        assert!(first.saves.is_empty());
        second.started_capture |= first.started_capture;
        second
    }

    #[test]
    fn keybinding_controls는_검색glyph_pill_헤더와_필터입력을_연결한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "font-size-up".into();
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let mut settled = raw(Vec::new());
        settled.time = Some(1.0);
        let (_, drawing) = frame(&mut editor, &context, &locale, settled, None, true);
        let texture = editor.icons.texture_id(Icon::Keyboard).unwrap();
        let glyph = drawing
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect
                        .brush
                        .as_ref()
                        .is_some_and(|brush| brush.fill_texture_id == texture) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(glyph.size(), egui::Vec2::splat(14.0));
        let header = [
            "settings.keymapCommandColumn",
            "settings.keymapKeyColumn",
            "settings.keymapSourceColumn",
        ]
        .map(|key| text_position(&drawing, &message(&locale, key, &[]).to_uppercase()));
        assert!(header[0].x < header[1].x && header[1].x < header[2].x);
        assert_eq!(header[0].y, header[1].y);
        assert_eq!(header[1].y, header[2].y);
        let count = editor.rows.iter().filter(|row| row.is_unassigned()).count();
        let position = text_position(
            &drawing,
            &format!(
                "{} ({count})",
                message(&locale, "settings.keymapUnassignedFilter", &[])
            ),
        );
        let pill = drawing
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect)
                    if rect.rect.contains(position)
                        && rect.stroke.color == editor.appearance.border =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(pill.rect.height(), 24.0);
        assert!(pill.corner_radius.nw >= 12);
        click(&mut editor, &context, &locale, position, None);
        assert!(editor.unassigned_only);
        let (_, drawing) = frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        text_position(&drawing, &message(&locale, "settings.keymapNoResults", &[]));
    }

    #[test]
    fn keybinding_row_grid는_열과_버튼그룹의_원본_간격을_보존한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "font-size-up".into();
        let overrides = r#"[{"actionId":"font-size-up","key":"x","mods":["mod"]}]"#;
        frame(
            &mut editor,
            &context,
            &locale,
            raw(Vec::new()),
            Some(overrides),
            true,
        );
        let (_, drawing) = frame(
            &mut editor,
            &context,
            &locale,
            raw(Vec::new()),
            Some(overrides),
            true,
        );
        let reset = icon_position(&drawing, editor.icons.texture_id(Icon::Reset).unwrap());
        let unbind = icon_position(&drawing, editor.icons.texture_id(Icon::Unbind).unwrap());
        assert_eq!(unbind.x - reset.x, 28.0);
        assert_eq!(unbind.y, reset.y);
        let row = editor
            .rows
            .iter()
            .find(|row| row.id == "font-size-up")
            .unwrap();
        let assigned = row.binding.label(true);
        let binding = drawing
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == assigned => Some(text),
                _ => None,
            })
            .unwrap();
        let source = drawing
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.text() == message(&locale, "settings.keymapSourceUser", &[]) =>
                {
                    Some(text)
                }
                _ => None,
            })
            .unwrap();
        assert!((source.pos.x - binding.pos.x - binding.galley.size().x - 12.0).abs() <= 1.0);
    }

    #[test]
    fn keybinding_modal_frame은_테마_scrim과_원본_그림자를_사용한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut theme = theme();
        theme.colors.insert("app.shadow".into(), "#4080c080".into());
        let mut editor = Editor::new(&theme, "en-US", true).unwrap();
        editor.open(&context);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let mut settled = raw(Vec::new());
        settled.time = Some(1.0);
        let (_, drawing) = frame(&mut editor, &context, &locale, settled, None, true);
        let shadow = color(&theme, "app.shadow").unwrap();
        let mut shapes = drawing
            .shapes
            .iter()
            .map(|shape| &shape.shape)
            .collect::<Vec<_>>();
        let mut rects = Vec::new();
        while let Some(shape) = shapes.pop() {
            match shape {
                egui::Shape::Rect(rect) => rects.push(rect),
                egui::Shape::Vec(children) => shapes.extend(children),
                _ => (),
            }
        }
        assert!(
            rects
                .iter()
                .any(|rect| rect.rect.size() == egui::vec2(SCREEN[0], SCREEN[1])
                    && rect.fill == shadow.gamma_multiply(0.5))
        );
        assert!(
            rects
                .iter()
                .any(|rect| rect.blur_width == 24.0 && rect.fill == shadow)
        );
        let panel = rects
            .iter()
            .find(|rect| {
                rect.fill == editor.appearance.background
                    && rect.stroke.color == editor.appearance.modal_border
            })
            .unwrap();
        assert_eq!(panel.rect.size(), egui::vec2(768.0, SCREEN[1] * 0.7));
        let close = icon_position(&drawing, editor.icons.texture_id(Icon::Close).unwrap());
        assert_eq!(
            close,
            egui::pos2(panel.rect.right() - 24.0, panel.rect.top() + 24.0)
        );
    }

    #[test]
    fn keybinding_modal_frame은_query와_마지막_close_사이에서_tab을_순환한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let query = editor.search_focus.unwrap();
        let close = editor.close_focus.unwrap();
        assert_eq!(context.memory(|memory| memory.focused()), Some(query));
        let mut backward = key(Key::Tab, false);
        if let Event::Key { modifiers, .. } = &mut backward {
            modifiers.shift = true;
        }
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![backward]),
            None,
            true,
        );
        assert_eq!(context.memory(|memory| memory.focused()), Some(close));
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Tab, false)]),
            None,
            true,
        );
        assert_eq!(context.memory(|memory| memory.focused()), Some(query));
        let (_, drawing) = frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let position = icon_position(&drawing, editor.icons.texture_id(Icon::Close).unwrap());
        click(&mut editor, &context, &locale, position, None);
        assert!(!editor.is_open());
    }

    #[test]
    fn keybinding_modal_input은_popup의_escape를_먼저_소비하지_않는다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        egui::Popup::open_id(&context, Id::new("synthetic-keybinding-popup"));
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Escape, false)]),
            None,
            true,
        );
        assert!(editor.is_open());
        assert!(
            context.input(|input| input.events.iter().any(|event| matches!(
                event,
                Event::Key {
                    key: Key::Escape,
                    pressed: true,
                    ..
                }
            )))
        );
        egui::Popup::close_all(&context);
    }

    #[test]
    fn keybinding_modal_tab은_화면밖_행을_표시하고_끝까지_순환한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "focus-group".into();
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let search = editor.search_focus.unwrap();
        let close = editor.close_focus.unwrap();
        let mut visited = Vec::new();
        let rows = editor
            .visible_rows(&locale, &ConflictIndex::new(&editor.rows, true))
            .len();
        assert!(rows > 10);
        for index in 0..rows * 4 + 10 {
            let mut input = raw(vec![key(Key::Tab, false)]);
            input.time = Some(index as f64 + 1.0);
            frame(&mut editor, &context, &locale, input, None, true);
            let mut input = raw(Vec::new());
            input.time = Some(index as f64 + 1.5);
            frame(&mut editor, &context, &locale, input, None, true);
            let focused = context
                .memory(|memory| memory.focused())
                .expect("focus escaped modal");
            if focused == search {
                break;
            }
            let (recorded, rect, interact_rect) = context
                .data(|data| {
                    data.get_temp::<Option<(Id, egui::Rect, egui::Rect)>>(Id::new(
                        "keybinding-current-focus",
                    ))
                })
                .flatten()
                .expect("focused widget disappeared");
            assert_eq!(recorded, focused);
            assert!(
                interact_rect.contains(rect.center()),
                "focused control is clipped: {rect:?} / {interact_rect:?}",
            );
            assert!(
                !visited.contains(&focused),
                "tab did not complete a single ordered cycle"
            );
            visited.push(focused);
        }
        assert_eq!(visited.last(), Some(&close));
        assert!(visited.len() >= rows * 2);
        assert_eq!(context.memory(|memory| memory.focused()), Some(search));
    }

    #[test]
    fn keybinding_modal_shift_tab은_화면밖_행을_노출하고_역순으로_순환한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "focus-group".into();
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let search = editor.search_focus.unwrap();
        let close = editor.close_focus.unwrap();
        let rows = editor
            .visible_rows(&locale, &ConflictIndex::new(&editor.rows, true))
            .len();
        let mut visited = Vec::new();
        for index in 0..rows * ROW_FOCUS_CONTROL_LIMIT + HEADER_FOCUS_CONTROL_COUNT {
            let mut tab = key(Key::Tab, false);
            if let Event::Key { modifiers, .. } = &mut tab {
                modifiers.shift = true;
            }
            let time = index as f64 + INPUT_FRAME_TIME;
            let mut input = raw(vec![tab]);
            input.time = Some(time);
            frame(&mut editor, &context, &locale, input, None, true);
            let mut settled = raw(Vec::new());
            settled.time = Some(time + SETTLEMENT_DELAY);
            frame(&mut editor, &context, &locale, settled, None, true);
            let focused = assert_focused_control_visible(&context);
            if focused == search {
                break;
            }
            assert!(
                !visited.contains(&focused),
                "reverse Tab repeated a control before wrapping"
            );
            visited.push(focused);
        }
        assert_eq!(visited.first(), Some(&close));
        assert!(visited.len() >= rows * MIN_ROW_FOCUS_CONTROL_COUNT);
        assert_eq!(context.memory(|memory| memory.focused()), Some(search));
    }

    #[test]
    fn keybinding_화면밖_capture의_autofocus와_blur는_가시영역과_검색복귀를_보존한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "focus-group".into();
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let rows = editor.visible_rows(&locale, &ConflictIndex::new(&editor.rows, true));
        let row = editor.rows[*rows.last().unwrap()].id.clone();
        let search = editor.search_focus.unwrap();
        editor.capture.start_row(row.clone());
        editor.focus_next = true;
        let mut input = raw(Vec::new());
        input.time = Some(INPUT_FRAME_TIME);
        let (output, _) = frame(&mut editor, &context, &locale, input, None, true);
        assert!(output.saves.is_empty());
        let mut settled = raw(Vec::new());
        settled.time = Some(INPUT_FRAME_TIME + SETTLEMENT_DELAY);
        frame(&mut editor, &context, &locale, settled, None, true);
        assert_eq!(
            Some(assert_focused_control_visible(&context)),
            editor.capture_focus
        );
        assert!(matches!(&editor.capture.target, Some(Target::Row { id, .. }) if *id == row));
        context.memory_mut(|memory| memory.request_focus(search));
        let (output, _) = frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        assert!(output.saves.is_empty());
        assert!(!editor.is_capturing());
        assert_eq!(assert_focused_control_visible(&context), search);
    }

    #[test]
    fn keybinding_제거된_행_focus는_modal로_회수하고_다음_tab을_보존한다() {
        for shift in [false, true] {
            let context = egui::Context::default();
            let locale = locale("en");
            let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
            editor.open(&context);
            editor.query = "font-size-up".into();
            frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
            frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
            let search = editor.search_focus.unwrap();
            let row_control = editor.focus_order[HEADER_FOCUS_CONTROL_COUNT - 1];
            context.memory_mut(|memory| memory.request_focus(row_control));
            frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
            assert_eq!(assert_focused_control_visible(&context), row_control);
            editor.query = "synthetic-no-matching-row".into();
            frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
            let container = Id::new(("native-keybinding-focus-container", context.viewport_id()));
            assert_eq!(context.memory(|memory| memory.focused()), Some(container));
            let mut tab = key(Key::Tab, false);
            if let Event::Key { modifiers, .. } = &mut tab {
                modifiers.shift = shift;
            }
            let (output, _) = frame(&mut editor, &context, &locale, raw(vec![tab]), None, true);
            assert!(output.saves.is_empty());
            let expected = if shift { container } else { search };
            assert_eq!(context.memory(|memory| memory.focused()), Some(expected));
            assert!(editor.is_open());
        }
    }

    #[test]
    fn keybinding_필터변경은_남은_행의_focus_id와_현재_tab_대상을_보존한다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "focus-group".into();
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let rows = editor.visible_rows(&locale, &ConflictIndex::new(&editor.rows, true));
        let row = editor.rows[*rows.last().unwrap()].id.clone();
        let focused = editor
            .focus_order
            .iter()
            .find(|id| {
                editor
                    .focus_rows
                    .get(id)
                    .is_some_and(|control| control.row == row)
            })
            .copied()
            .unwrap();
        context.memory_mut(|memory| memory.request_focus(focused));
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        editor.query = row.clone();
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        assert_eq!(assert_focused_control_visible(&context), focused);
        assert_eq!(editor.focus_rows.get(&focused).unwrap().row, row);
        let filter = editor.focus_order[HEADER_FOCUS_CONTROL_COUNT - 2];
        let close = editor.close_focus.unwrap();
        context.memory_mut(|memory| memory.request_focus(filter));
        editor.query = "synthetic-no-matching-row".into();
        let (output, _) = frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Tab, false)]),
            None,
            true,
        );
        assert!(output.saves.is_empty());
        assert_eq!(context.memory(|memory| memory.focused()), Some(close));
        assert!(editor.focus_rows.is_empty());
        assert!(!editor.focus_order.contains(&focused));
    }

    #[test]
    fn keybinding_override변경은_남은_unbind_focus와_제거된_reset_회수를_보존한다() {
        const OVERRIDES: &str = r#"[{"actionId":"font-size-up","key":"x","mods":["mod"]}]"#;
        for key in ["settings.keymapUnbind", "settings.keymapResetOne"] {
            let context = egui::Context::default();
            let locale = locale("en");
            let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
            editor.open(&context);
            editor.query = "font-size-up".into();
            frame(
                &mut editor,
                &context,
                &locale,
                raw(Vec::new()),
                Some(OVERRIDES),
                true,
            );
            let (output, _) = frame(
                &mut editor,
                &context,
                &locale,
                raw(Vec::new()),
                Some(OVERRIDES),
                true,
            );
            let focused = output
                .tooltip_triggers
                .iter()
                .find(|(label, _)| *label == key)
                .unwrap()
                .1
                .id;
            context.memory_mut(|memory| memory.request_focus(focused));
            frame(
                &mut editor,
                &context,
                &locale,
                raw(Vec::new()),
                Some(OVERRIDES),
                true,
            );
            let (output, _) = frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
            assert!(output.saves.is_empty());
            if key == "settings.keymapUnbind" {
                let unbind = output
                    .tooltip_triggers
                    .iter()
                    .find(|(label, _)| *label == key)
                    .unwrap()
                    .1
                    .id;
                assert_eq!(
                    unbind, focused,
                    "surviving unbind acquired a different identity"
                );
                assert_eq!(context.memory(|memory| memory.focused()), Some(focused));
            } else {
                assert_eq!(
                    context.memory(|memory| memory.focused()),
                    Some(Id::new((
                        "native-keybinding-focus-container",
                        context.viewport_id()
                    )))
                );
            }
        }
    }

    #[test]
    fn keybinding_새_reset의_tab_배정은_현재_override_모델을_사용한다() {
        const OVERRIDES: &str = r#"[{"actionId":"font-size-up","key":"x","mods":["mod"]}]"#;
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "font-size-up".into();
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let change = editor.focus_order[HEADER_FOCUS_CONTROL_COUNT - 1];
        context.memory_mut(|memory| memory.request_focus(change));
        let (output, _) = frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Tab, false)]),
            Some(OVERRIDES),
            true,
        );
        let reset = output
            .tooltip_triggers
            .iter()
            .find(|(label, _)| *label == "settings.keymapResetOne")
            .unwrap()
            .1
            .id;
        assert_eq!(context.memory(|memory| memory.focused()), Some(reset));
        assert_eq!(assert_focused_control_visible(&context), reset);
        assert!(output.saves.is_empty());
    }

    #[test]
    fn keybinding_modal_input은_캡처중_tab과_화살표를_focus이동으로_넘기지_않는다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        editor.open(&context);
        editor.query = "font-size-up".into();
        editor.capture.start_row("font-size-up".into());
        editor.focus_next = true;
        editor.search_focus_next = false;
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::K, true)]),
            None,
            true,
        );
        let (output, _) = frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Tab, false)]),
            None,
            true,
        );
        assert_eq!(output.saves.len(), 1);
        assert!(output.saves[0].json().unwrap().contains("\"key\":\"Tab\""));
        editor.capture.toggle_search();
        editor.query.clear();
        editor.focus_next = true;
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::ArrowDown, false)]),
            None,
            true,
        );
        assert!(matches!(editor.capture.target, Some(Target::Search)));
        assert_eq!(
            editor.capture.searched_key.as_ref().unwrap().key(),
            "ArrowDown"
        );
    }

    #[test]
    fn keybinding_escape로_닫힌_뒤_이어지는_프레임에도_이전_포커스가_유지된다() {
        let context = egui::Context::default();
        let locale = locale("en");
        let mut editor = Editor::new(&theme(), "en-US", true).unwrap();
        let background = Id::new("synthetic-background-focus");
        let frame_over_background = |editor: &mut Editor, events: Vec<Event>| {
            let mut drawing = context.run_ui(raw(events), |ui| {
                ui.interact(
                    egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::Vec2::splat(BACKGROUND_FOCUS_SIDE),
                    ),
                    background,
                    egui::Sense::focusable_noninteractive(),
                );
                editor.show(ui.ctx(), &locale, None, true).unwrap();
            });
            drawing.textures_delta.clear();
            context.memory(|memory| memory.focused())
        };
        context.memory_mut(|memory| memory.request_focus(background));
        assert_eq!(
            frame_over_background(&mut editor, Vec::new()),
            Some(background)
        );
        editor.open(&context);
        frame_over_background(&mut editor, Vec::new());
        let focused = frame_over_background(&mut editor, Vec::new());
        assert!(focused.is_some() && focused == editor.search_focus);
        assert_eq!(
            frame_over_background(&mut editor, vec![key(Key::Escape, false)]),
            Some(background)
        );
        assert!(!editor.is_open());
        for _ in 0..SETTLED_FRAME_COUNT {
            assert_eq!(
                frame_over_background(&mut editor, Vec::new()),
                Some(background),
                "닫는 프레임에 되돌린 포커스가 이후 프레임에도 남는다"
            );
        }
    }

    #[test]
    fn keybinding_editor는_실제modal_버튼_focus_캡처_저장_검색과_창chord를_연결한다() {
        let theme = theme();
        let locale = locale("en");
        let context = egui::Context::default();
        let previous_focus = Id::new("synthetic-editor-focus");
        context.memory_mut(|memory| memory.request_focus(previous_focus));
        let mut editor = Editor::new(&theme, "en-US", true).unwrap();
        let now = Instant::now();
        editor.observe_closed(
            Context {
                editor: true,
                terminal: false,
            },
            now,
        );
        assert!(editor.context_keys.is_empty());
        editor.open(&context);
        assert!(editor.context_keys.is_empty());
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let (_, drawing) = frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![Event::Text("font-size-up".into())]),
            None,
            true,
        );
        assert_eq!(editor.query, "font-size-up");
        let change = text_position(&drawing, &message(&locale, "settings.keymapChange", &[]));
        assert!(context.content_rect().contains(change));
        let capture = click(&mut editor, &context, &locale, change, None);
        assert!(capture.started_capture && editor.is_capturing());
        let (output, _) = frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        assert!(output.saves.is_empty());
        assert_eq!(
            context.memory(|memory| memory.focused()),
            editor.capture_focus
        );
        let (output, _) = frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::X, false), Event::Text("x".into())]),
            None,
            true,
        );
        assert_eq!(
            output.warnings,
            [message(&locale, "settings.keymapModifierRequired", &[])]
        );
        assert_eq!(editor.query, "font-size-up");
        let (output, _) = frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::K, true)]),
            None,
            true,
        );
        assert!(output.saves.is_empty());
        let (_, drawing) = frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let confirm = text_position(
            &drawing,
            &message(&locale, "settings.keymapChordConfirmSingle", &[]),
        );
        let output = click(&mut editor, &context, &locale, confirm, None);
        assert_eq!(output.saves.len(), 1);
        let encoded = output.saves[0].json().unwrap();
        assert!(encoded.contains("font-size-up") && encoded.contains("\"key\":\"k\""));
        let _command = HostCommand::SetKeymapOverrides(output.saves[0].clone());
        assert!(!editor.is_capturing());
        let (_, drawing) = frame(
            &mut editor,
            &context,
            &locale,
            raw(Vec::new()),
            Some(&encoded),
            true,
        );
        let reset = icon_position(&drawing, editor.icons.texture_id(Icon::Reset).unwrap());
        let output = click(&mut editor, &context, &locale, reset, Some(&encoded));
        assert_eq!(output.saves.len(), 1);
        assert_eq!(output.saves[0].json().unwrap(), "[]");
        let (_, drawing) = frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        let search = text_position(
            &drawing,
            &message(&locale, "settings.keymapSearchByKey", &[]),
        );
        let output = click(&mut editor, &context, &locale, search, None);
        assert!(output.started_capture && editor.query.is_empty());
        frame(&mut editor, &context, &locale, raw(Vec::new()), None, true);
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::S, true)]),
            None,
            true,
        );
        assert!(editor.capture.searched_key.is_some());
        assert!(
            editor
                .visible_rows(&locale, &ConflictIndex::new(&editor.rows, true))
                .iter()
                .any(|index| editor.rows[*index].id == "save")
        );
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Escape, false)]),
            None,
            true,
        );
        assert!(
            editor.is_open() && !editor.is_capturing() && editor.capture.searched_key.is_none()
        );
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![
                Event::Ime(egui::ImeEvent::Preedit {
                    text: "가".into(),
                    active_range_chars: None,
                }),
                key(Key::Escape, false),
            ]),
            None,
            true,
        );
        assert!(editor.is_open());
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![Event::Ime(egui::ImeEvent::Preedit {
                text: String::new(),
                active_range_chars: None,
            })]),
            None,
            true,
        );
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Escape, false)]),
            None,
            false,
        );
        assert!(editor.is_open());
        frame(
            &mut editor,
            &context,
            &locale,
            raw(vec![key(Key::Escape, false)]),
            None,
            true,
        );
        assert!(!editor.is_open() && editor.query.is_empty());
        assert_eq!(
            context.memory(|memory| memory.focused()),
            Some(previous_focus)
        );
        editor.observe_closed(
            Context {
                editor: true,
                terminal: false,
            },
            now,
        );
        editor.observe_closed(
            Context {
                editor: false,
                terminal: true,
            },
            now + CONTEXT_POLL / 2,
        );
        assert_eq!(editor.context_keys, ["editorTextFocus"]);
        editor.observe_closed(
            Context {
                editor: false,
                terminal: true,
            },
            now + CONTEXT_POLL,
        );
        assert_eq!(editor.context_keys, ["terminalFocus"]);
        editor.open(&context);
        editor.observe_closed(Context::default(), now + CONTEXT_POLL * 2);
        assert_eq!(editor.context_keys, ["terminalFocus"]);

        let key_context = egui::Context::default();
        let mut views = Views::default();
        let mut actions = Vec::new();
        for event in [key(Key::K, true), key(Key::S, true)] {
            let mut drawing = key_context.run_ui(raw(vec![event]), |ui| {
                let events = ui.input_mut(|input| std::mem::take(&mut input.events));
                for (index, event) in events.iter().enumerate() {
                    assert!(
                        views
                            .route_keymap(
                                crate::keymap::Route {
                                    context: ui.ctx(),
                                    event,
                                    index,
                                    scope: Context::default(),
                                    composing: false,
                                    overrides: None
                                },
                                &mut actions,
                                false
                            )
                            .unwrap()
                    );
                }
            });
            drawing.textures_delta.clear();
        }
        assert_eq!(actions, ["open-keybindings-editor"]);
        assert!(crate::shell_keymap::supports(&actions[0]));
        let mut map = crate::keymap::Keymap::new().unwrap();
        assert_eq!(
            map.decide_egui(&key(Key::K, true), Context::default(), false, now),
            Decision::EnterChord
        );
        let mut maps = crate::keymap::Windows::default();
        let mut drawing = key_context.run_ui(raw(vec![key(Key::K, true)]), |ui| {
            let event = key(Key::K, true);
            assert!(
                maps.route(
                    crate::keymap::Route {
                        context: ui.ctx(),
                        event: &event,
                        index: 0,
                        scope: Context::default(),
                        composing: false,
                        overrides: None
                    },
                    |decision| matches!(decision, Decision::EnterChord)
                )
                .unwrap()
            );
            maps.clear_chord(ui.ctx().viewport_id());
            assert!(
                !maps
                    .route(
                        crate::keymap::Route {
                            context: ui.ctx(),
                            event: &key(Key::Z, false),
                            index: 1,
                            scope: Context::default(),
                            composing: false,
                            overrides: None
                        },
                        |decision| matches!(decision, Decision::ResolveChord(_))
                    )
                    .unwrap()
            );
        });
        drawing.textures_delta.clear();
        for id in ["ko", "ja"] {
            let translated = self::locale(id);
            let (_, drawing) = frame(
                &mut editor,
                &context,
                &translated,
                raw(Vec::new()),
                None,
                true,
            );
            text_position(
                &drawing,
                &message(&translated, "settings.keymapEditorTitle", &[]),
            );
        }
    }
}
