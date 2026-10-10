use std::time::Duration;

use egui::EventFilter;
use egui::{
    Align2, Color32, Event, FontFamily, FontId, Id, Key, KeyboardShortcut, Layout, Margin,
    Modifiers, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, Vec2,
};
use taide_native_editor::find::FindPatternCompiler;
use taide_native_editor::language_configuration::LanguageRules;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::ViewId;

use crate::Instant;
use crate::editor_find::{EditorFind, FindCommand, FindError, FindFocus, OPTIONS_REVEAL_DELAY};

pub use crate::font_families::CODICON_FAMILY as ICON_FAMILY;
const INITIAL_WIDTH: f32 = 419.0;
const MIN_WIDTH: f32 = 170.0;
const NARROW_WIDTH: f32 = 257.0;
const COUNT_WIDTH: f32 = 69.0;
const EDITOR_MARGIN: f32 = 28.0;
const LEFT_MARGIN: f32 = 15.0;
const ROW_MARGIN: f32 = 4.0;
const INPUT_HEIGHT: f32 = 25.0;
const INPUT_MAX_HEIGHT: f32 = 118.0;
const INPUT_LINE_HEIGHT: f32 = 19.0;
const INPUT_FONT_SIZE: f32 = 13.0;
const COUNT_FONT_SIZE: f32 = 12.0;
const ICON_SIZE: f32 = 16.0;
const BUTTON_SIZE: f32 = 22.0;
const BUTTON_GAP: f32 = 3.0;
const TOGGLE_WIDTH: f32 = 18.0;
const RESIZE_WIDTH: f32 = 2.0;
const BORDER_WIDTH: f32 = 1.0;
const SEARCH_CONTROL_COUNT: f32 = 3.0;
const COLLAPSED_THRESHOLD: f32 = 50.0;
const DISABLED_OPACITY: f32 = 0.5;
const OPTIONS_TOP: f32 = 10.0;
const HISTORY_DELAY: Duration = Duration::from_millis(500);
const HISTORY_LIMIT: usize = 100;
const QUICK_INPUT_WIDTH_FACTOR: f32 = 0.62;
const QUICK_INPUT_MAX_WIDTH: f32 = 600.0;
const QUICK_INPUT_PADDING: i8 = 6;
const SHADOW_OFFSET: [i8; 2] = [0, 2];
const SHADOW_BLUR: u8 = 8;
const CENTER_DIVISOR: f32 = 2.0;
const INPUT_EVENT_FILTER: EventFilter = EventFilter {
    tab: true,
    escape: true,
    horizontal_arrows: true,
    vertical_arrows: true,
};
const CASE_ICON: char = '\u{eab1}';
const WORD_ICON: char = '\u{eb7e}';
const REGEX_ICON: char = '\u{eb38}';
const PRESERVE_ICON: char = '\u{eb2e}';
const SELECTION_ICON: char = '\u{eb85}';
const PREVIOUS_ICON: char = '\u{eaa1}';
const NEXT_ICON: char = '\u{ea9a}';
const REPLACE_ICON: char = '\u{eb3d}';
const REPLACE_ALL_ICON: char = '\u{eb3c}';
const CLOSE_ICON: char = '\u{ea76}';
const COLLAPSED_ICON: char = '\u{eab6}';
const EXPANDED_ICON: char = '\u{eab4}';

#[derive(Clone)]
pub struct FindAppearance {
    pub background: Color32,
    pub border: Color32,
    pub foreground: Color32,
    pub input_background: Color32,
    pub input_foreground: Color32,
    pub input_border: Color32,
    pub focus: Color32,
    pub active_background: Color32,
    pub active_foreground: Color32,
    pub active_border: Color32,
    pub hover: Color32,
    pub error: Color32,
    pub highlight: Color32,
    pub current_match: Color32,
    pub scope: Color32,
    pub shadow: Color32,
    pub quick_input_background: Color32,
    pub quick_input_foreground: Color32,
    pub placeholder: Color32,
    pub error_background: Color32,
    pub error_foreground: Color32,
}

#[derive(Default)]
pub struct FindHistory {
    search: Vec<String>,
    replacement: Vec<String>,
    pending: Option<(Instant, String, String)>,
}

impl FindHistory {
    pub fn search(&self) -> &[String] {
        &self.search
    }
    pub fn replacement(&self) -> &[String] {
        &self.replacement
    }

    fn record(&mut self, search: &str, replacement: &str, immediate: bool) {
        if immediate {
            self.commit(search, replacement);
            self.pending = None;
            return;
        }
        self.pending = Some((
            Instant::now() + HISTORY_DELAY,
            search.into(),
            replacement.into(),
        ));
    }

    fn commit(&mut self, search: &str, replacement: &str) {
        for (items, value) in [
            (&mut self.search, search),
            (&mut self.replacement, replacement),
        ] {
            if value.is_empty() {
                continue;
            }
            items.retain(|item| item != value);
            items.push(value.into());
            if items.len() > HISTORY_LIMIT {
                items.remove(0);
            }
        }
    }

    fn flush(&mut self, ui: &Ui) {
        if let Some((due, _, _)) = &self.pending {
            if *due <= Instant::now() {
                let (_, search, replacement) = self.pending.take().unwrap();
                self.commit(&search, &replacement);
            } else {
                ui.ctx()
                    .request_repaint_after(due.saturating_duration_since(Instant::now()));
            }
        }
    }
}

pub struct FindWidgetOutput {
    pub reserved_height: f32,
    pub request_editor_focus: bool,
    pub focused: bool,
}

pub fn editor_shortcut(event: &Event, is_mac: bool, visible: bool) -> Option<FindCommand> {
    let Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return None;
    };
    if visible && *key == Key::Escape && !modifiers.command && !modifiers.alt && !modifiers.ctrl {
        return Some(FindCommand::Close);
    }
    if visible
        && *key == Key::Enter
        && modifiers.alt
        && !modifiers.shift
        && !modifiers.ctrl
        && !modifiers.mac_cmd
    {
        return Some(FindCommand::SelectAll);
    }
    if visible && *key == Key::Enter && modifiers.command && modifiers.alt && !modifiers.shift {
        return Some(FindCommand::ReplaceAll);
    }
    if visible && *key == Key::Num1 && modifiers.command && modifiers.shift && !modifiers.alt {
        return Some(FindCommand::ReplaceOne);
    }
    if !modifiers.alt
        || modifiers.shift
        || (is_mac != modifiers.command)
        || is_mac && modifiers.ctrl
    {
        return None;
    }
    match key {
        Key::C => Some(FindCommand::ToggleCase),
        Key::W => Some(FindCommand::ToggleWholeWord),
        Key::R => Some(FindCommand::ToggleRegex),
        Key::L => Some(FindCommand::ToggleScope),
        Key::P => Some(FindCommand::TogglePreserveCase),
        _ => None,
    }
}

pub fn show(
    ui: &mut Ui,
    find: &mut EditorFind,
    history: &mut FindHistory,
    store: &mut EditorStore,
    view: ViewId,
    compiler: &dyn FindPatternCompiler,
    rules: Option<&dyn LanguageRules>,
    appearance: &FindAppearance,
    mut keymap: impl FnMut(&Ui, &Event, bool) -> bool,
) -> Result<FindWidgetOutput, FindError> {
    history.flush(ui);
    let mut request_editor_focus = ui.is_enabled() && find.focus == Some(FindFocus::Editor);
    if request_editor_focus {
        find.focus = None;
    }
    if !find.visible {
        find.input_focused = false;
        if let Some(due) = find.options_until {
            if due > Instant::now() {
                let bounds = ui.available_rect_before_wrap().intersect(ui.clip_rect());
                let size = Vec2::new(
                    BUTTON_SIZE * SEARCH_CONTROL_COUNT + ROW_MARGIN * 2.0,
                    BUTTON_SIZE + ROW_MARGIN * 2.0,
                );
                let rect = Rect::from_min_size(
                    egui::pos2(
                        bounds.right() - EDITOR_MARGIN - size.x,
                        bounds.top() + OPTIONS_TOP,
                    ),
                    size,
                );
                ui.painter().add(
                    egui::epaint::Shadow {
                        offset: SHADOW_OFFSET,
                        blur: SHADOW_BLUR,
                        spread: 0,
                        color: appearance.shadow,
                    }
                    .as_shape(rect, ui.visuals().window_corner_radius),
                );
                ui.painter().rect_filled(
                    rect,
                    ui.visuals().window_corner_radius,
                    appearance.background,
                );
                let mut hovered = false;
                for (index, (glyph, label, active, command)) in [
                    (
                        CASE_ICON,
                        "Match Case",
                        find.options.match_case,
                        FindCommand::ToggleCase,
                    ),
                    (
                        WORD_ICON,
                        "Match Whole Word",
                        find.options.whole_word,
                        FindCommand::ToggleWholeWord,
                    ),
                    (
                        REGEX_ICON,
                        "Use Regular Expression",
                        find.options.is_regex,
                        FindCommand::ToggleRegex,
                    ),
                ]
                .into_iter()
                .enumerate()
                {
                    let button_rect = Rect::from_min_size(
                        rect.min + Vec2::new(ROW_MARGIN + index as f32 * BUTTON_SIZE, ROW_MARGIN),
                        Vec2::splat(BUTTON_SIZE),
                    );
                    let response = icon_button(
                        ui,
                        button_rect,
                        ui.make_persistent_id(("find-options", view, index)),
                        glyph,
                        label,
                        active,
                        true,
                        appearance,
                    );
                    hovered |= response.hovered();
                    if response.clicked() {
                        find.execute(command, store, view, compiler, rules)?;
                    }
                }
                if hovered {
                    find.options_until = Some(Instant::now() + OPTIONS_REVEAL_DELAY);
                }
                ui.ctx()
                    .request_repaint_after(due.saturating_duration_since(Instant::now()));
            } else {
                find.options_until = None;
            }
        }
        return Ok(FindWidgetOutput {
            reserved_height: 0.0,
            request_editor_focus,
            focused: false,
        });
    }
    find.refresh(store, view, compiler)?;
    let bounds = ui.available_rect_before_wrap().intersect(ui.clip_rect());
    let id = ui.make_persistent_id(("native-find-widget", view));
    let find_id = id.with("find");
    let replace_id = id.with("replace");
    let button_id = |name: &str| id.with(name);
    let read_only = store
        .documents()
        .snapshot(
            store
                .views()
                .get(view)
                .ok_or(taide_native_editor::document::EditorError::NotFound)?
                .document,
        )?
        .metadata
        .read_only;
    let max_width = (bounds.width() - EDITOR_MARGIN - LEFT_MARGIN).max(0.0);
    let collapsed =
        bounds.width() + COLLAPSED_THRESHOLD < INITIAL_WIDTH + EDITOR_MARGIN - COUNT_WIDTH;
    let narrow = bounds.width() < INITIAL_WIDTH + EDITOR_MARGIN - COUNT_WIDTH;
    let reduced = bounds.width() < INITIAL_WIDTH + EDITOR_MARGIN;
    let width = find
        .width
        .unwrap_or(INITIAL_WIDTH)
        .min(max_width)
        .min(if collapsed {
            MIN_WIDTH
        } else if narrow {
            NARROW_WIDTH
        } else {
            f32::INFINITY
        });
    let find_height = input_height(&find.search);
    let replace_height = input_height(&find.replacement);
    let height = ROW_MARGIN
        + find_height
        + ROW_MARGIN
        + if find.replace_visible {
            replace_height + ROW_MARGIN
        } else {
            0.0
        };
    let rect = Rect::from_min_size(
        egui::pos2(
            bounds.right() - EDITOR_MARGIN - width,
            bounds.top() + ROW_MARGIN,
        ),
        Vec2::new(width, height),
    );
    let mut widget = ui.new_child(
        UiBuilder::new()
            .id_salt(id)
            .max_rect(rect)
            .layout(Layout::top_down(egui::Align::Min)),
    );
    ui.painter().add(
        egui::epaint::Shadow {
            offset: SHADOW_OFFSET,
            blur: SHADOW_BLUR,
            spread: 0,
            color: appearance.shadow,
        }
        .as_shape(rect, ui.visuals().window_corner_radius),
    );
    widget.set_clip_rect(rect.intersect(bounds));
    widget.painter().rect(
        rect,
        widget.visuals().window_corner_radius,
        appearance.background,
        Stroke::new(BORDER_WIDTH, appearance.border),
        StrokeKind::Inside,
    );
    let resize_rect = Rect::from_min_size(rect.min, Vec2::new(RESIZE_WIDTH, rect.height()));
    let resize = widget.interact(resize_rect, button_id("resize"), Sense::drag());
    if resize.dragged() {
        find.width =
            Some((width - resize.drag_delta().x).clamp(MIN_WIDTH.min(max_width), max_width));
    }
    if resize.double_clicked() {
        find.width = Some(if width > INITIAL_WIDTH {
            INITIAL_WIDTH.min(max_width)
        } else {
            max_width
        });
    }
    let input_left = rect.left() + TOGGLE_WIDTH + ROW_MARGIN;
    let close_left = rect.right() - ROW_MARGIN - BUTTON_SIZE;
    let action_width = if collapsed {
        BUTTON_SIZE
    } else {
        (BUTTON_SIZE + BUTTON_GAP) * SEARCH_CONTROL_COUNT + if reduced { 0.0 } else { COUNT_WIDTH }
    };
    let input_right = (close_left - BUTTON_GAP - action_width).max(input_left);
    let controls_width = if collapsed {
        0.0
    } else {
        (BUTTON_SIZE + BORDER_WIDTH) * SEARCH_CONTROL_COUNT
    };
    let input_rect = Rect::from_min_max(
        egui::pos2(input_left, rect.top() + ROW_MARGIN),
        egui::pos2(input_right, rect.top() + ROW_MARGIN + find_height),
    );
    let replace_rect = input_rect.translate(Vec2::new(0.0, find_height + ROW_MARGIN));
    let mut ids = vec![find_id];
    if find.replace_visible {
        ids.push(replace_id);
    }
    if !collapsed {
        ids.extend([button_id("case"), button_id("word"), button_id("regex")]);
    }
    if find.replace_visible {
        ids.push(button_id("preserve"));
    }
    if !collapsed && !find.results().matches.is_empty() {
        ids.extend([button_id("previous"), button_id("next")]);
    }
    ids.extend([button_id("scope"), button_id("close")]);
    if find.replace_visible && !collapsed && !find.results().matches.is_empty() {
        ids.extend([button_id("replace-one"), button_id("replace-all")]);
    }
    let focused = ui.memory(|memory| memory.focused());
    ids.extend([button_id("toggle"), button_id("resize")]);
    let owns_focus = widget.is_enabled() && focused.is_some_and(|focus| ids.contains(&focus));
    if owns_focus {
        for event in ui.input(|input| input.events.clone()) {
            match event {
                Event::Ime(egui::ImeEvent::Preedit { text, .. }) => {
                    find.composing = !text.is_empty()
                }
                Event::Ime(egui::ImeEvent::Commit(_)) => find.composing = false,
                _ => {}
            }
        }
    } else {
        find.composing = false;
    }
    let composing = find.composing;
    let mut commands = Vec::new();
    let is_mac = ui.ctx().os().is_mac();
    let mut tab_focus = None;
    let cursor = focused
        .and_then(|id| TextEdit::load_state(ui.ctx(), id))
        .and_then(|state| state.cursor.char_range())
        .map(|range| usize::from(range.primary.index));
    if owns_focus && !composing {
        let events = widget.input_mut(|input| std::mem::take(&mut input.events));
        let remaining = events
            .into_iter()
            .filter(|event| !keymap(&widget, event, composing))
            .collect();
        widget.input_mut(|input| input.events = remaining);
        let focus = focused.unwrap();
        widget.input_mut(|input| {
            input.events.retain(|event| {
                let Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                else {
                    return true;
                };
                if *key == Key::Escape && !modifiers.command && !modifiers.alt && !modifiers.ctrl {
                    commands.push(FindCommand::Close);
                    return false;
                }
                if *key == Key::Tab && !modifiers.command && !modifiers.alt && !modifiers.ctrl {
                    let index = ids.iter().position(|id| *id == focus).unwrap_or(0);
                    let next = if modifiers.shift {
                        (index + ids.len() - 1) % ids.len()
                    } else {
                        (index + 1) % ids.len()
                    };
                    tab_focus = Some(ids[next]);
                    return false;
                }
                if *key == Key::ArrowDown && modifiers.command && !modifiers.alt && !modifiers.shift
                {
                    request_editor_focus = true;
                    return false;
                }
                if *key == Key::Enter
                    && !modifiers.ctrl
                    && !modifiers.mac_cmd
                    && !modifiers.command
                    && (modifiers.alt
                        || focus == find_id
                        || focus == replace_id && !modifiers.shift)
                {
                    commands.push(if modifiers.alt {
                        FindCommand::SelectAll
                    } else if focus == replace_id {
                        FindCommand::ReplaceOne
                    } else if modifiers.shift {
                        FindCommand::Previous
                    } else {
                        FindCommand::Next
                    });
                    return false;
                }
                if *key == Key::Enter
                    && modifiers.command
                    && (modifiers.alt || focus == replace_id && is_mac)
                {
                    commands.push(FindCommand::ReplaceAll);
                    return false;
                }
                if *key == Key::Num1 && modifiers.command && modifiers.shift && !modifiers.alt {
                    commands.push(FindCommand::ReplaceOne);
                    return false;
                }
                if modifiers.alt && !modifiers.shift && (is_mac == modifiers.command) {
                    let command = match key {
                        Key::C => Some(FindCommand::ToggleCase),
                        Key::W => Some(FindCommand::ToggleWholeWord),
                        Key::R => Some(FindCommand::ToggleRegex),
                        Key::L => Some(FindCommand::ToggleScope),
                        Key::P => Some(FindCommand::TogglePreserveCase),
                        _ => None,
                    };
                    if let Some(command) = command {
                        commands.push(command);
                        return false;
                    }
                }
                if (*key == Key::ArrowUp || *key == Key::ArrowDown)
                    && *modifiers == Modifiers::NONE
                    && (focus == find_id || focus == replace_id)
                {
                    let (text, items, draft) = if focus == find_id {
                        (
                            &mut find.search,
                            &history.search,
                            &mut find.search_history_draft,
                        )
                    } else {
                        (
                            &mut find.replacement,
                            &history.replacement,
                            &mut find.replacement_history_draft,
                        )
                    };
                    let boundary = cursor.is_some_and(|cursor| {
                        if *key == Key::ArrowUp {
                            !text.chars().take(cursor).any(|character| character == '\n')
                        } else {
                            !text.chars().skip(cursor).any(|character| character == '\n')
                        }
                    });
                    if boundary && !items.is_empty() {
                        let index = items.iter().position(|value| value == text);
                        if index.is_none() && *key == Key::ArrowUp {
                            *draft = Some(text.clone());
                        }
                        let next = if *key == Key::ArrowUp {
                            index.map_or(items.len() - 1, |index| index.saturating_sub(1))
                        } else {
                            index.map_or(items.len(), |index| index + 1)
                        };
                        *text = items
                            .get(next)
                            .cloned()
                            .unwrap_or_else(|| draft.take().unwrap_or_default());
                        return false;
                    }
                }
                true
            });
        });
    }
    if let Some(focus) = tab_focus {
        widget.memory_mut(|memory| memory.request_focus_with_filter(focus, INPUT_EVENT_FILTER));
    }
    if !read_only {
        let toggle_rect = Rect::from_min_size(rect.min, Vec2::new(TOGGLE_WIDTH, height));
        if icon_button(
            &mut widget,
            toggle_rect,
            button_id("toggle"),
            if find.replace_visible {
                EXPANDED_ICON
            } else {
                COLLAPSED_ICON
            },
            "Toggle Replace",
            false,
            true,
            appearance,
        )
        .clicked()
        {
            find.replace_visible = !find.replace_visible;
        }
    }
    let pending_focus = if widget.is_enabled() {
        find.focus.take()
    } else {
        None
    };
    let search_changed = text_input(
        &mut widget,
        input_rect,
        find_id,
        &mut find.search,
        "Find",
        controls_width,
        pending_focus == Some(FindFocus::Find),
        appearance,
    );
    let mut replacement_changed = false;
    if find.replace_visible {
        replacement_changed = text_input(
            &mut widget,
            Rect::from_min_size(
                replace_rect.min,
                Vec2::new(replace_rect.width(), replace_height),
            ),
            replace_id,
            &mut find.replacement,
            "Replace",
            BUTTON_SIZE + BORDER_WIDTH,
            pending_focus == Some(FindFocus::Replace),
            appearance,
        );
    }
    if !collapsed {
        for (index, (name, glyph, label, active, command)) in [
            (
                "case",
                CASE_ICON,
                "Match Case",
                find.options.match_case,
                FindCommand::ToggleCase,
            ),
            (
                "word",
                WORD_ICON,
                "Match Whole Word",
                find.options.whole_word,
                FindCommand::ToggleWholeWord,
            ),
            (
                "regex",
                REGEX_ICON,
                "Use Regular Expression",
                find.options.is_regex,
                FindCommand::ToggleRegex,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let control = Rect::from_min_size(
                egui::pos2(
                    input_rect.right() - controls_width
                        + index as f32 * (BUTTON_SIZE + BORDER_WIDTH),
                    input_rect.top() + BORDER_WIDTH,
                ),
                Vec2::splat(BUTTON_SIZE),
            );
            if icon_button(
                &mut widget,
                control,
                button_id(name),
                glyph,
                label,
                active,
                true,
                appearance,
            )
            .clicked()
            {
                commands.push(command);
            }
        }
    }
    if find.replace_visible {
        let control = Rect::from_min_size(
            egui::pos2(
                replace_rect.right() - BUTTON_SIZE - BORDER_WIDTH,
                replace_rect.top() + BORDER_WIDTH,
            ),
            Vec2::splat(BUTTON_SIZE),
        );
        if icon_button(
            &mut widget,
            control,
            button_id("preserve"),
            PRESERVE_ICON,
            "Preserve Case",
            find.preserve_case,
            true,
            appearance,
        )
        .clicked()
        {
            commands.push(FindCommand::TogglePreserveCase);
        }
    }
    let selection = store.views().get(view).unwrap().selection.clone();
    let primary = selection.selections[selection.primary];
    let position = find.results().matches.iter().position(|found| {
        found.range == (primary.anchor.min(primary.head)..primary.anchor.max(primary.head))
    });
    let count = if find.results().matches.is_empty() {
        "No results".to_string()
    } else {
        format!(
            "{} of {}{}",
            position.map_or_else(|| "?".into(), |index| (index + 1).to_string()),
            find.results().matches.len(),
            if find.results().limit_reached {
                "+"
            } else {
                ""
            }
        )
    };
    let mut action_x = input_rect.right() + BUTTON_GAP;
    if !reduced {
        let count_rect = Rect::from_min_size(
            egui::pos2(action_x, input_rect.top()),
            Vec2::new(COUNT_WIDTH, INPUT_HEIGHT),
        );
        widget.painter().text(
            count_rect.center(),
            Align2::CENTER_CENTER,
            &count,
            FontId::proportional(COUNT_FONT_SIZE),
            if find.results().matches.is_empty() && !find.search.is_empty() {
                appearance.error
            } else {
                appearance.foreground
            },
        );
        action_x += COUNT_WIDTH;
    }
    let has_matches = !find.results().matches.is_empty() && find.error.is_none();
    if !collapsed {
        for (name, glyph, label, command) in [
            (
                "previous",
                PREVIOUS_ICON,
                "Previous Match",
                FindCommand::Previous,
            ),
            ("next", NEXT_ICON, "Next Match", FindCommand::Next),
        ] {
            if icon_button(
                &mut widget,
                Rect::from_min_size(
                    egui::pos2(action_x, input_rect.top() + BORDER_WIDTH),
                    Vec2::splat(BUTTON_SIZE),
                ),
                button_id(name),
                glyph,
                label,
                false,
                has_matches,
                appearance,
            )
            .clicked()
            {
                commands.push(command);
            }
            action_x += BUTTON_SIZE + BUTTON_GAP;
        }
    }
    let scope_rect = Rect::from_min_size(
        egui::pos2(action_x, input_rect.top() + BORDER_WIDTH),
        Vec2::splat(BUTTON_SIZE),
    );
    if icon_button(
        &mut widget,
        scope_rect,
        button_id("scope"),
        SELECTION_ICON,
        "Find in Selection",
        !find.scopes().is_empty(),
        true,
        appearance,
    )
    .clicked()
    {
        commands.push(FindCommand::ToggleScope);
    }
    let close_rect = Rect::from_min_size(
        egui::pos2(close_left, input_rect.top() + BORDER_WIDTH),
        Vec2::splat(BUTTON_SIZE),
    );
    if icon_button(
        &mut widget,
        close_rect,
        button_id("close"),
        CLOSE_ICON,
        "Close",
        false,
        true,
        appearance,
    )
    .clicked()
    {
        commands.push(FindCommand::Close);
    }
    if find.replace_visible && !collapsed {
        for (index, (name, glyph, label, command)) in [
            (
                "replace-one",
                REPLACE_ICON,
                "Replace",
                FindCommand::ReplaceOne,
            ),
            (
                "replace-all",
                REPLACE_ALL_ICON,
                "Replace All",
                FindCommand::ReplaceAll,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let button_rect = Rect::from_min_size(
                egui::pos2(
                    input_rect.right() + BUTTON_GAP + index as f32 * (BUTTON_SIZE + BUTTON_GAP),
                    replace_rect.top() + BORDER_WIDTH,
                ),
                Vec2::splat(BUTTON_SIZE),
            );
            if icon_button(
                &mut widget,
                button_rect,
                button_id(name),
                glyph,
                label,
                false,
                has_matches && !read_only,
                appearance,
            )
            .clicked()
            {
                commands.push(command);
            }
        }
    }
    if search_changed || replacement_changed {
        history.record(&find.search, &find.replacement, false);
    }
    let query_changed = find.refresh(store, view, compiler)?;
    if (search_changed || query_changed) && find.error.is_none() {
        find.navigate(store, view, true, true)?;
    }
    for command in commands {
        history.record(&find.search, &find.replacement, true);
        if let Err(error) = find.execute(command, store, view, compiler, rules) {
            find.error = Some(error.to_string());
        }
    }
    if let Some(message) = &find.error {
        widget.painter().rect_stroke(
            input_rect,
            BORDER_WIDTH,
            Stroke::new(BORDER_WIDTH, appearance.error),
            StrokeKind::Inside,
        );
        widget
            .interact(input_rect, button_id("error"), Sense::hover())
            .on_hover_text(message);
        if widget.memory(|memory| memory.has_focus(find_id)) {
            egui::Area::new(button_id("error-message"))
                .order(egui::Order::Tooltip)
                .fixed_pos(input_rect.left_bottom())
                .show(ui.ctx(), |ui| {
                    egui::Frame::NONE
                        .fill(appearance.error_background)
                        .stroke(Stroke::new(BORDER_WIDTH, appearance.error))
                        .inner_margin(QUICK_INPUT_PADDING)
                        .show(ui, |ui| {
                            ui.set_max_width(input_rect.width());
                            ui.colored_label(appearance.error_foreground, message);
                        });
                });
        }
    }
    if ui.is_enabled()
        && let Some(mut number) = find.match_number.take()
    {
        let escape = ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        let width = (bounds.width() * QUICK_INPUT_WIDTH_FACTOR).min(QUICK_INPUT_MAX_WIDTH);
        let popup = egui::Area::new(button_id("match-number"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(
                bounds.center().x - width / CENTER_DIVISOR,
                bounds.top(),
            ))
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(appearance.quick_input_background)
                    .stroke(Stroke::new(BORDER_WIDTH, appearance.border))
                    .shadow(egui::epaint::Shadow {
                        offset: SHADOW_OFFSET,
                        blur: SHADOW_BLUR,
                        spread: 0,
                        color: appearance.shadow,
                    })
                    .corner_radius(ui.visuals().window_corner_radius)
                    .inner_margin(QUICK_INPUT_PADDING)
                    .show(ui, |ui| {
                        let count = find.results().matches.len();
                        let placeholder = format!(
                            "Type a number to go to a specific match (between 1 and {count})"
                        );
                        let response = ui.add(
                            TextEdit::singleline(&mut number)
                                .id(button_id("match-number-input"))
                                .event_filter(INPUT_EVENT_FILTER)
                                .hint_text(placeholder)
                                .desired_width(width)
                                .text_color(appearance.quick_input_foreground),
                        );
                        if find.match_focus {
                            ui.memory_mut(|memory| {
                                memory.request_focus_with_filter(response.id, INPUT_EVENT_FILTER)
                            });
                            find.match_focus = false;
                        }
                        let index = number.parse::<isize>().ok().and_then(|number| {
                            if number > 0 {
                                usize::try_from(number)
                                    .ok()
                                    .filter(|number| *number <= count)
                            } else if number < 0 {
                                count
                                    .checked_sub(number.unsigned_abs())
                                    .map(|index| index + 1)
                            } else {
                                None
                            }
                        });
                        if !number.is_empty() && index.is_none() {
                            ui.colored_label(
                                appearance.error,
                                format!("Please type a number between 1 and {count}"),
                            );
                        }
                        let accepted =
                            ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Enter));
                        (response.changed(), accepted, index)
                    })
                    .inner
            });
        let (changed, accepted, index) = popup.inner;
        if changed && let Some(index) = index {
            find.preview_match(store, view, index)?;
        }
        let clicked_outside = ui.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|position| !popup.response.rect.contains(position))
        });
        if escape || clicked_outside {
            find.focus = Some(FindFocus::Find);
        } else if accepted && let Some(index) = index {
            find.go_to_match(store, view, index)?;
        } else {
            find.match_number = Some(number);
        }
    }
    request_editor_focus |= ui.is_enabled() && find.focus == Some(FindFocus::Editor);
    if request_editor_focus {
        find.focus = None;
    }
    let focused = widget.is_enabled()
        && widget.memory(|memory| memory.focused().is_some_and(|focus| ids.contains(&focus)));
    find.input_focused = widget.is_enabled() && widget.memory(|memory| memory.has_focus(find_id));
    Ok(FindWidgetOutput {
        reserved_height: if find.visible {
            height + ROW_MARGIN
        } else {
            0.0
        },
        request_editor_focus,
        focused,
    })
}

fn input_height(text: &str) -> f32 {
    (text.split('\n').count() as f32 * INPUT_LINE_HEIGHT + ROW_MARGIN)
        .clamp(INPUT_HEIGHT, INPUT_MAX_HEIGHT)
}

fn text_input(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    text: &mut String,
    label: &str,
    controls: f32,
    focus: bool,
    appearance: &FindAppearance,
) -> bool {
    ui.painter().rect(
        rect,
        BORDER_WIDTH,
        appearance.input_background,
        Stroke::new(BORDER_WIDTH, appearance.input_border),
        StrokeKind::Inside,
    );
    if focus {
        ui.memory_mut(|memory| memory.request_focus_with_filter(id, INPUT_EVENT_FILTER));
        let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(text.chars().count()),
            )));
        state.store(ui.ctx(), id);
    }
    let mut child = ui.new_child(UiBuilder::new().id_salt(id).max_rect(Rect::from_min_max(
        rect.min,
        egui::pos2((rect.right() - controls).max(rect.left()), rect.bottom()),
    )));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    let newline = Modifiers::CTRL;
    let output = TextEdit::multiline(text)
        .id(id)
        .hint_text(egui::RichText::new(label).color(appearance.placeholder))
        .font(FontId::proportional(INPUT_FONT_SIZE))
        .text_color(appearance.input_foreground)
        .background_color(Color32::TRANSPARENT)
        .frame(egui::Frame::NONE)
        .margin(Margin::symmetric(ROW_MARGIN as i8, BORDER_WIDTH as i8))
        .desired_rows(1)
        .desired_width((rect.width() - controls).max(0.0))
        .min_size(Vec2::new((rect.width() - controls).max(0.0), rect.height()))
        .event_filter(INPUT_EVENT_FILTER)
        .return_key(KeyboardShortcut::new(newline, Key::Enter))
        .show(&mut child);
    output.response.widget_info(|| {
        egui::WidgetInfo::text_edit(ui.is_enabled(), text.as_str(), text.as_str(), label)
    });
    if output.response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            BORDER_WIDTH,
            Stroke::new(BORDER_WIDTH, appearance.focus),
            StrokeKind::Inside,
        );
    }
    output.response.changed()
}

fn icon_button(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    glyph: char,
    label: &str,
    active: bool,
    enabled: bool,
    appearance: &FindAppearance,
) -> Response {
    let response = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let fill = if active {
        appearance.active_background
    } else if response.hovered() && enabled {
        appearance.hover
    } else {
        Color32::TRANSPARENT
    };
    let border = if active {
        appearance.active_border
    } else if response.has_focus() {
        appearance.focus
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        ui.visuals().widgets.inactive.corner_radius,
        fill,
        Stroke::new(BORDER_WIDTH, border),
        StrokeKind::Inside,
    );
    let color = if !enabled {
        appearance.foreground.gamma_multiply(DISABLED_OPACITY)
    } else if active {
        appearance.active_foreground
    } else {
        appearance.foreground
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::new(ICON_SIZE, FontFamily::Name(ICON_FAMILY.into())),
        color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled && ui.is_enabled(), label)
    });
    response.on_hover_text(label)
}
