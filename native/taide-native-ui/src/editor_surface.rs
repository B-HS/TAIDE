use std::sync::Arc;

use egui::text::CCursor;
use egui::{
    Color32, Event, FontId, Galley, ImeEvent, Key, Pos2, Rect, Response, Sense, Stroke, Ui, pos2,
    vec2,
};
use taide_native_editor::document::{DocumentSnapshot, Edit, EditorError, UndoGroup};
use taide_native_editor::editing::{
    Motion, line_content_range, move_selection, replace_selections, reveal_position, select_all,
    selected_text,
};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::store::{EditorStore, Transaction};
use taide_native_editor::view::{Composition, Selection, SelectionSet, ViewId};

const LINE_NUMBER_MIN_DIGITS: usize = 3;
const CURSOR_STROKE: f32 = 1.0;
const ROW_OVERSCAN: usize = 1;
const CENTER_DIVISOR: f32 = 2.0;
const PADDING_SIDES: f32 = 2.0;

type KeyboardInputRoute = (Vec<(Option<bool>, bool)>, (Option<bool>, bool));

#[derive(Clone)]
pub struct EditorAppearance {
    pub font: FontId,
    pub line_height: f32,
    pub horizontal_padding: f32,
    pub background: Color32,
    pub foreground: Color32,
    pub muted: Color32,
    pub selection: Color32,
    pub cursor: Color32,
    pub current_line: Color32,
    pub line_numbers: bool,
    pub indent: String,
}

#[derive(Default, Clone)]
struct InputState {
    ime_revision: Option<u64>,
}

pub struct EditorOutput {
    pub response: Response,
    pub save_requested: bool,
    pub changed: bool,
    pub rendered_lines: std::ops::Range<usize>,
    pub errors: Vec<EditorError>,
}

#[derive(Default)]
struct InputOutput {
    copied: Option<String>,
    errors: Vec<EditorError>,
}

struct Row {
    line: usize,
    start_char: usize,
    byte_range: std::ops::Range<usize>,
    origin: Pos2,
    galley: Arc<Galley>,
}

pub struct NativeEditor {
    pub appearance: EditorAppearance,
}

impl NativeEditor {
    pub fn with_indent(&self, options: IndentOptions) -> Self {
        let mut appearance = self.appearance.clone();
        appearance.indent = if options.insert_spaces {
            " ".repeat(options.tab_size as usize)
        } else {
            "\t".into()
        };
        Self { appearance }
    }

    pub fn reveal(
        &self,
        ui: &Ui,
        store: &mut EditorStore,
        view: ViewId,
        line: f64,
        column: f64,
    ) -> Result<(), EditorError> {
        let appearance = &self.appearance;
        if !appearance.line_height.is_finite()
            || appearance.line_height <= 0.0
            || !appearance.horizontal_padding.is_finite()
            || appearance.horizontal_padding < 0.0
        {
            return Err(EditorError::InvalidBoundary);
        }
        let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        if !rect.is_finite() || rect.width() <= 0.0 || rect.height() <= 0.0 {
            return Err(EditorError::InvalidBoundary);
        }
        let byte = reveal_position(store, view, line, column)?;
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        let line = document.rope.byte_to_line(byte);
        let range = line_content_range(&document, line);
        let start = document.rope.byte_to_char(range.start);
        let end = document.rope.byte_to_char(range.end);
        let galley = ui.painter().layout_no_wrap(
            document.rope.slice(start..end).to_string(),
            appearance.font.clone(),
            appearance.foreground,
        );
        let cursor = galley.pos_from_cursor(CCursor::new(document.rope.byte_to_char(byte) - start));
        let text_width = (rect.width() - gutter_width(ui, &document, appearance)).max(0.0);
        let mut scroll = current.scroll;
        if cursor.left() < scroll.x {
            scroll.x = cursor.left();
        }
        if cursor.right() + CURSOR_STROKE > scroll.x + text_width {
            scroll.x = (cursor.right() + CURSOR_STROKE - text_width).max(0.0);
        }
        let maximum =
            (document.rope.len_lines() as f32 * appearance.line_height - rect.height()).max(0.0);
        scroll.y = ((line as f32 + 1.0 / CENTER_DIVISOR) * appearance.line_height
            - rect.height() / CENTER_DIVISOR)
            .clamp(0.0, maximum);
        store.set_view_state(view, current.selection, scroll, current.folds)?;
        Ok(())
    }

    pub fn show(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
    ) -> Result<EditorOutput, EditorError> {
        let mut save_requested = false;
        let mut output = self.show_with_keymap(ui, store, view, request_focus, |_, event, _| {
            if matches!(event, Event::Key { key: Key::S, pressed: true, modifiers, .. } if modifiers.command) {
                save_requested = true;
                return true;
            }
            false
        })?;
        output.save_requested = save_requested;
        Ok(output)
    }

    pub fn show_with_keymap(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
        keymap: impl FnMut(&Ui, &Event, bool) -> bool,
    ) -> Result<EditorOutput, EditorError> {
        self.show_with_input_route(ui, store, view, request_focus, keymap, |_| None)
    }

    #[doc = "Renders with caller-provided (ownership, focus-lost-before-event) entries and (final-focus, focus-lost-after-events), preserving default routing when absent."]
    pub fn show_with_input_route(
        &self,
        ui: &mut Ui,
        store: &mut EditorStore,
        view: ViewId,
        request_focus: bool,
        mut keymap: impl FnMut(&Ui, &Event, bool) -> bool,
        route: impl FnOnce(&Response) -> Option<KeyboardInputRoute>,
    ) -> Result<EditorOutput, EditorError> {
        let appearance = &self.appearance;
        if !appearance.line_height.is_finite()
            || appearance.line_height <= 0.0
            || !appearance.horizontal_padding.is_finite()
            || appearance.horizontal_padding < 0.0
        {
            return Err(EditorError::InvalidBoundary);
        }
        let id = ui.make_persistent_id(("native-code-editor", view));
        let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        let response = ui.interact(rect, id, Sense::click_and_drag());
        ui.allocate_rect(rect, Sense::hover());
        if ui.is_enabled() && (request_focus || response.clicked() || response.drag_started()) {
            response.request_focus();
        }
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let previous = store.documents().snapshot(current.document)?;
        let mut input_state = ui
            .ctx()
            .data_mut(|data| data.get_temp::<InputState>(id).unwrap_or_default());
        let mut output = InputOutput::default();
        let (ownership, (routed_focus, lost_after_events)) = route(&response).unwrap_or_default();
        let focused = routed_focus.unwrap_or_else(|| response.has_focus());
        let has_owned_input = ownership.iter().any(|(owned, _)| *owned == Some(true));
        if (response.has_focus() || has_owned_input) && ui.is_enabled() {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
            let events = ui.input_mut(|input| std::mem::take(&mut input.events));
            let mut remaining = Vec::new();
            for (index, event) in events.into_iter().enumerate() {
                let (owned, lost) = ownership.get(index).copied().unwrap_or_default();
                if lost {
                    input_state.ime_revision = None;
                    store.set_composition(view, None)?;
                }
                if !owned.unwrap_or_else(|| response.has_focus()) {
                    remaining.push(event);
                    continue;
                }
                let composing = store
                    .views()
                    .get(view)
                    .is_some_and(|view| view.composition.is_some());
                if keymap(ui, &event, composing) {
                    continue;
                }
                match self.input(store, view, &event, &mut input_state, &mut output) {
                    Ok(true) => {}
                    Ok(false) => remaining.push(event),
                    Err(error) => output.errors.push(error),
                }
            }
            ui.input_mut(|input| input.events = remaining);
        }
        if lost_after_events || !focused || !ui.is_enabled() {
            input_state.ime_revision = None;
            store.set_composition(view, None)?;
        }
        if let Some(text) = output.copied {
            ui.ctx().copy_text(text);
        }
        let document = store.documents().snapshot(current.document)?;
        let mut state = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let scroll_max =
            (document.rope.len_lines() as f32 * appearance.line_height - rect.height()).max(0.0);
        state.scroll.y = state.scroll.y.min(scroll_max);
        if document.revision != previous.revision || state.selection != current.selection {
            let head = state.selection.selections[state.selection.primary].head;
            let top = document.rope.byte_to_line(head) as f32 * appearance.line_height;
            if top < state.scroll.y {
                state.scroll.y = top;
            }
            if top + appearance.line_height > state.scroll.y + rect.height() {
                state.scroll.y = (top + appearance.line_height - rect.height()).max(0.0);
            }
        }
        if response.hovered() && ui.is_enabled() {
            let delta = ui.input_mut(|input| {
                let delta = input.smooth_scroll_delta;
                input.smooth_scroll_delta = egui::Vec2::ZERO;
                delta
            });
            state.scroll.x = (state.scroll.x - delta.x).max(0.0);
            state.scroll.y = (state.scroll.y - delta.y).clamp(
                0.0,
                (document.rope.len_lines() as f32 * appearance.line_height - rect.height())
                    .max(0.0),
            );
            store.set_view_state(
                view,
                state.selection.clone(),
                state.scroll.clone(),
                state.folds.clone(),
            )?;
        }
        store.set_view_state(
            view,
            state.selection.clone(),
            state.scroll.clone(),
            state.folds.clone(),
        )?;
        let painter = ui.painter().with_clip_rect(rect);
        let gutter = gutter_width(ui, &document, appearance);
        let text_rect = Rect::from_min_max(pos2(rect.left() + gutter, rect.top()), rect.max);
        let first = (state.scroll.y / appearance.line_height).floor() as usize;
        let end = (first + (rect.height() / appearance.line_height).ceil() as usize + ROW_OVERSCAN)
            .min(document.rope.len_lines());
        let mut rows = Vec::new();
        for line in first.min(end)..end {
            let byte_range = line_content_range(&document, line);
            let start_char = document.rope.byte_to_char(byte_range.start);
            let end_char = document.rope.byte_to_char(byte_range.end);
            let text = document.rope.slice(start_char..end_char).to_string();
            let galley =
                painter.layout_no_wrap(text, appearance.font.clone(), appearance.foreground);
            rows.push(Row {
                line,
                start_char,
                byte_range,
                origin: pos2(
                    text_rect.left() - state.scroll.x,
                    rect.top() + line as f32 * appearance.line_height - state.scroll.y,
                ),
                galley,
            });
        }
        if (response.clicked() || response.drag_started() || response.dragged())
            && let Some(pointer) = response.interact_pointer_pos()
            && !rows.is_empty()
        {
            let line = (((pointer.y - rect.top() + state.scroll.y) / appearance.line_height)
                .floor()
                .max(0.0) as usize)
                .clamp(rows[0].line, rows[rows.len() - 1].line);
            if let Some(row) = rows.iter().find(|row| row.line == line) {
                let cursor = row.galley.cursor_from_pos(pointer - row.origin);
                let head = document
                    .rope
                    .char_to_byte(row.start_char + cursor.index.0)
                    .min(row.byte_range.end);
                let extending = response.dragged() || ui.input(|input| input.modifiers.shift);
                let anchor = if extending {
                    state.selection.selections[state.selection.primary].anchor
                } else {
                    head
                };
                state.selection = SelectionSet {
                    primary: 0,
                    selections: vec![Selection { anchor, head }],
                };
                state.composition = None;
                input_state.ime_revision = None;
                store.set_composition(view, None)?;
                ui.memory_mut(|memory| memory.interrupt_ime());
                store.set_view_state(
                    view,
                    state.selection.clone(),
                    state.scroll.clone(),
                    state.folds.clone(),
                )?;
            }
        }
        ui.ctx().data_mut(|data| data.insert_temp(id, input_state));
        painter.rect_filled(rect, 0.0, appearance.background);
        let text_painter = painter.with_clip_rect(text_rect);
        let primary = state.selection.selections[state.selection.primary];
        let caret_line = document.rope.byte_to_line(primary.head);
        let mut caret_rect = None;
        for row in &rows {
            if row.line == caret_line {
                painter.rect_filled(
                    Rect::from_min_size(
                        pos2(rect.left(), row.origin.y),
                        vec2(rect.width(), appearance.line_height),
                    ),
                    0.0,
                    appearance.current_line,
                );
            }
            for selection in &state.selection.selections {
                let start = selection.anchor.min(selection.head);
                let end = selection.anchor.max(selection.head);
                if start < end && start <= row.byte_range.end && end > row.byte_range.start {
                    let left = cursor_rect(&document, row, start.max(row.byte_range.start));
                    let right = cursor_rect(&document, row, end.min(row.byte_range.end));
                    let right_x = if end > row.byte_range.end {
                        text_rect.right().max(right.left())
                    } else {
                        right.left()
                    };
                    text_painter.rect_filled(
                        Rect::from_min_max(
                            pos2(left.left(), row.origin.y),
                            pos2(right_x, row.origin.y + appearance.line_height),
                        ),
                        0.0,
                        appearance.selection,
                    );
                }
                if selection.anchor == selection.head
                    && document.rope.byte_to_line(selection.head) == row.line
                {
                    let cursor =
                        cursor_rect(&document, row, selection.head.min(row.byte_range.end));
                    if focused {
                        text_painter.line_segment(
                            [
                                cursor.min,
                                pos2(cursor.left(), row.origin.y + appearance.line_height),
                            ],
                            Stroke::new(CURSOR_STROKE, appearance.cursor),
                        );
                    }
                }
            }
            if row.line == caret_line {
                caret_rect = Some(cursor_rect(
                    &document,
                    row,
                    primary.head.min(row.byte_range.end),
                ));
            }
            text_painter.galley(row.origin, row.galley.clone(), appearance.foreground);
            if appearance.line_numbers {
                painter.text(
                    pos2(
                        rect.left() + gutter - appearance.horizontal_padding,
                        row.origin.y,
                    ),
                    egui::Align2::RIGHT_TOP,
                    (row.line + 1).to_string(),
                    appearance.font.clone(),
                    appearance.muted,
                );
            }
        }
        if focused
            && ui.is_enabled()
            && !document.metadata.read_only
            && let Some(cursor_rect) = caret_rect
        {
            if let Some(composition) = &state.composition {
                let galley = text_painter.layout_no_wrap(
                    composition.preedit.clone(),
                    appearance.font.clone(),
                    appearance.foreground,
                );
                let preedit_rect = Rect::from_min_size(cursor_rect.min, galley.size());
                text_painter.rect_filled(preedit_rect, 0.0, appearance.background);
                text_painter.galley(cursor_rect.min, galley, appearance.foreground);
                text_painter.line_segment(
                    [preedit_rect.left_bottom(), preedit_rect.right_bottom()],
                    Stroke::new(CURSOR_STROKE, appearance.cursor),
                );
            }
            let transform = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            ui.output_mut(|output| {
                output.mutable_text_under_cursor = response.hovered();
                output.ime = Some(egui::output::IMEOutput {
                    purpose: egui::IMEPurpose::Normal,
                    rect: transform * rect,
                    cursor_rect: transform * cursor_rect,
                    should_interrupt_composition: false,
                });
            });
        }
        Ok(EditorOutput {
            response,
            save_requested: false,
            changed: document.revision != previous.revision,
            rendered_lines: first.min(end)..end,
            errors: output.errors,
        })
    }

    fn input(
        &self,
        store: &mut EditorStore,
        view: ViewId,
        event: &Event,
        state: &mut InputState,
        output: &mut InputOutput,
    ) -> Result<bool, EditorError> {
        let current = store
            .views()
            .get(view)
            .ok_or(EditorError::NotFound)?
            .clone();
        let document = store.documents().snapshot(current.document)?;
        match event {
            Event::Copy => {
                output.copied = Some(selected_text(store, view)?);
            }
            Event::Cut => {
                output.copied = Some(selected_text(store, view)?);
                if !document.metadata.read_only {
                    replace_selections(store, view, "", None)?;
                }
            }
            Event::Paste(text) => {
                state.ime_revision = None;
                replace_selections(store, view, text, None)?;
            }
            Event::Text(text) if current.composition.is_none() => {
                replace_selections(store, view, text, None)?;
            }
            Event::Text(_) => {}
            Event::Ime(ImeEvent::Preedit { text, .. }) => {
                if document.metadata.read_only {
                    return Err(EditorError::ReadOnly);
                }
                if state
                    .ime_revision
                    .is_some_and(|revision| revision != document.revision)
                {
                    state.ime_revision = None;
                    store.set_composition(view, None)?;
                    return Err(EditorError::StaleRevision);
                }
                let primary = current.selection.selections[current.selection.primary];
                let replace = current
                    .composition
                    .map(|composition| composition.replace)
                    .unwrap_or(primary.anchor.min(primary.head)..primary.anchor.max(primary.head));
                state.ime_revision = Some(document.revision);
                if text.is_empty() {
                    store.set_composition(view, None)?;
                } else {
                    store.set_composition(
                        view,
                        Some(Composition {
                            revision: document.revision,
                            replace,
                            preedit: text.clone(),
                        }),
                    )?;
                }
            }
            Event::Ime(ImeEvent::Commit(text)) => {
                let revision = state.ime_revision.take();
                if revision.is_some_and(|revision| revision != document.revision) {
                    store.set_composition(view, None)?;
                    return Err(EditorError::StaleRevision);
                }
                if !text.is_empty() {
                    if let Some(composition) = current.composition {
                        replace_range(store, view, &document, composition.replace, text)?;
                    } else {
                        replace_selections(store, view, text, None)?;
                    }
                }
                store.set_composition(view, None)?;
            }
            Event::Ime(ImeEvent::DeleteSurrounding {
                before_chars,
                after_chars,
            }) => {
                let primary = current.selection.selections[current.selection.primary];
                let scalar = document.rope.byte_to_char(primary.head);
                let start = document
                    .rope
                    .char_to_byte(scalar.saturating_sub(*before_chars));
                let end = document.rope.char_to_byte(
                    scalar
                        .saturating_add(*after_chars)
                        .min(document.rope.len_chars()),
                );
                replace_range(store, view, &document, start..end, "")?;
            }
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if modifiers.command {
                    match key {
                        Key::A => select_all(store, view)?,
                        Key::Z if modifiers.shift => {
                            store.redo(document.id)?;
                        }
                        Key::Z => {
                            store.undo(document.id)?;
                        }
                        Key::Y => {
                            store.redo(document.id)?;
                        }
                        Key::Home => {
                            move_selection(store, view, Motion::DocumentStart, modifiers.shift)?
                        }
                        Key::End => {
                            move_selection(store, view, Motion::DocumentEnd, modifiers.shift)?
                        }
                        _ => return Ok(false),
                    }
                } else if modifiers.alt
                    || modifiers.ctrl
                    || modifiers.mac_cmd
                    || current.composition.is_some()
                {
                    return Ok(false);
                } else {
                    let motion = match key {
                        Key::ArrowLeft => Some(Motion::Left),
                        Key::ArrowRight => Some(Motion::Right),
                        Key::ArrowUp => Some(Motion::Up),
                        Key::ArrowDown => Some(Motion::Down),
                        Key::Home => Some(Motion::LineStart),
                        Key::End => Some(Motion::LineEnd),
                        _ => None,
                    };
                    if let Some(motion) = motion {
                        move_selection(store, view, motion, modifiers.shift)?;
                    } else {
                        match key {
                            Key::Backspace => {
                                replace_selections(store, view, "", Some(false))?;
                            }
                            Key::Delete => {
                                replace_selections(store, view, "", Some(true))?;
                            }
                            Key::Enter => {
                                let first_line = document.rope.line(0);
                                let line_break = if first_line.len_chars() > 1
                                    && first_line.char(first_line.len_chars() - 2) == '\r'
                                {
                                    "\r\n"
                                } else {
                                    "\n"
                                };
                                replace_selections(store, view, line_break, None)?;
                            }
                            Key::Tab if !modifiers.shift => {
                                replace_selections(store, view, &self.appearance.indent, None)?;
                            }
                            Key::Escape => {
                                let primary =
                                    current.selection.selections[current.selection.primary];
                                if current.selection.selections.len() == 1
                                    && primary.anchor == primary.head
                                {
                                    return Ok(false);
                                }
                                let selection = if current.selection.selections.len() > 1 {
                                    primary
                                } else {
                                    Selection {
                                        anchor: primary.head,
                                        head: primary.head,
                                    }
                                };
                                store.break_undo_group(document.id)?;
                                store.set_view_state(
                                    view,
                                    SelectionSet {
                                        primary: 0,
                                        selections: vec![selection],
                                    },
                                    current.scroll,
                                    current.folds,
                                )?;
                            }
                            _ => return Ok(false),
                        }
                    }
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

fn gutter_width(ui: &Ui, document: &DocumentSnapshot, appearance: &EditorAppearance) -> f32 {
    if !appearance.line_numbers {
        return appearance.horizontal_padding;
    }
    let digits = document
        .rope
        .len_lines()
        .to_string()
        .len()
        .max(LINE_NUMBER_MIN_DIGITS);
    let width = ui
        .painter()
        .layout_no_wrap(
            "0".repeat(digits),
            appearance.font.clone(),
            appearance.muted,
        )
        .size()
        .x;
    width + appearance.horizontal_padding * PADDING_SIDES
}

fn cursor_rect(document: &DocumentSnapshot, row: &Row, byte: usize) -> Rect {
    row.galley
        .pos_from_cursor(CCursor::new(
            document.rope.byte_to_char(byte) - row.start_char,
        ))
        .translate(row.origin.to_vec2())
}

fn replace_range(
    store: &mut EditorStore,
    view: ViewId,
    document: &DocumentSnapshot,
    bytes: std::ops::Range<usize>,
    text: &str,
) -> Result<(), EditorError> {
    let head = bytes.start + text.len();
    store.apply(
        document.id,
        Transaction {
            revision: document.revision,
            group: UndoGroup(document.revision),
            origin: Some(view),
            edits: vec![Edit {
                bytes,
                text: text.into(),
            }],
            selection_after: Some(SelectionSet {
                primary: 0,
                selections: vec![Selection { anchor: head, head }],
            }),
        },
    )?;
    Ok(())
}
