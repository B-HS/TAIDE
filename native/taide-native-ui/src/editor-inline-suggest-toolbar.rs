use egui::text::CCursor;
use egui::{
    Align2, Area, Color32, Context, CornerRadius, FontId, Frame, Id, Order, Pos2, Rect, Sense,
    Stroke, Ui, Vec2, pos2, vec2,
};
use taide_native_editor::completion::Command;
use taide_native_editor::completion_preview_view::ViewData;
use taide_native_editor::document::DocumentSnapshot;

use crate::editor_geometry::EditorGeometry;
use crate::editor_markup::Colors;

const SHOW_DELAY: f64 = 0.300;
const HIDE_DELAY: f64 = 0.300;
const ALWAYS_SHOW_KEY: &str = "taide-inline-suggest-always-show-toolbar";
const STATE_KEY: &str = "inline-suggest-toolbar";
const PADDING: f32 = 4.0;
const ITEM_SPACING: f32 = 6.0;
const KEY_SPACING: f32 = 4.0;
const BORDER_WIDTH: f32 = 1.0;
const CORNER_RADIUS: u8 = 3;
const ICON_SIZE: f32 = 16.0;
const CHEVRON_RATIO: f32 = 0.2;
const CHEVRON_HEIGHT_RATIO: f32 = 0.3;
const CHECK_RISE_RATIO: f32 = 0.14;
const CHECK_KNEE_RATIO: f32 = 0.33;
const MORE_DOT_RADIUS: f32 = 1.2;
const MORE_DOT_SPACING: f32 = 4.0;
const CHECK_WIDTH: f32 = 16.0;
const MENU_ROW_PADDING: f32 = 4.0;
const DISABLED_OPACITY: f32 = 0.4;
const KEY_OPACITY: f32 = 0.6;
const FONT_SIZE: f32 = 12.0;
const KEY_FONT_SIZE: f32 = 8.0;

pub const PREVIOUS: &str = "Previous";
pub const NEXT: &str = "Next";
pub const ACCEPT: &str = "Accept";
pub const ACCEPT_WORD: &str = "Accept Word";
pub const ACCEPT_LINE: &str = "Accept Line";
pub const ALWAYS_SHOW_TOOLBAR: &str = "Always Show Toolbar";
pub const MORE_ACTIONS: &str = "More Actions...";
const ACCEPT_KEY: &str = "Tab";
const ACCEPT_WORD_MAC_KEY: &str = "\u{2318}\u{2192}";
const ACCEPT_WORD_KEY: &str = "Ctrl+RightArrow";

#[derive(Clone, Copy, Default)]
struct State {
    hovered_since: Option<f64>,
    left_at: Option<f64>,
    shown: bool,
    menu_open: bool,
    toolbar: Option<Rect>,
    menu: Option<Rect>,
    session: Option<(usize, f32)>,
}

pub struct Request<'a> {
    pub geometry: &'a EditorGeometry,
    pub document: &'a DocumentSnapshot,
    pub ghosts: &'a [ViewData],
    pub zones: &'a [Rect],
    pub colors: Colors,
    pub count: Option<usize>,
}

/// Returns whether the inline suggestion toolbar of the current window is pinned open.
pub fn is_always_shown(context: &Context) -> bool {
    let key = Id::new(ALWAYS_SHOW_KEY).with(context.viewport_id());
    context
        .data(|data| data.get_temp::<bool>(key))
        .unwrap_or(false)
}

/// Flips the window-scoped `editor.inlineSuggest.showToolbar` value between `always` and `onHover`.
pub fn toggle_always_shown(context: &Context) {
    let key = Id::new(ALWAYS_SHOW_KEY).with(context.viewport_id());
    let value = !is_always_shown(context);
    context.data_mut(|data| data.insert_temp(key, value));
}

pub(crate) fn contains(ui: &Ui, body: Id, position: Pos2) -> bool {
    let state = load(ui, body);
    state.shown
        && [state.toolbar, state.menu.filter(|_| state.menu_open)]
            .into_iter()
            .flatten()
            .any(|rect| rect.contains(position))
}

fn load(ui: &Ui, body: Id) -> State {
    ui.ctx()
        .data(|data| data.get_temp::<State>(body.with(STATE_KEY)))
        .unwrap_or_default()
}

fn store(ui: &Ui, body: Id, state: State) {
    ui.ctx()
        .data_mut(|data| data.insert_temp(body.with(STATE_KEY), state));
}

fn injected_rect(geometry: &EditorGeometry, byte: usize, extends: bool) -> Option<Rect> {
    let row = geometry.rows.iter().find(|row| {
        let bytes = &row.segment.bytes;
        row.origin.y + geometry.line_height > geometry.rect.top()
            && row.origin.y < geometry.rect.bottom()
            && bytes.start <= byte
            && (byte < bytes.end
                || byte == bytes.end
                    && (!row.wraps || row.text.injection_anchor(byte - bytes.start)))
    })?;
    let local = u32::try_from(byte - row.segment.bytes.start).ok()?;
    let text = &row.text;
    let mut injected =
        (text.indent_chars..text.model_bytes.len().saturating_sub(1)).filter(|&index| {
            text.injected[index] && text.hints[index].is_none() && text.model_bytes[index] == local
        });
    let first = injected.next()?;
    let last = injected.last().unwrap_or(first);
    let origin = row.text_origin();
    let left = origin.x + row.galley.pos_from_cursor(CCursor::new(first)).left();
    let right = if extends {
        geometry.content_rect.right()
    } else {
        origin.x + row.galley.pos_from_cursor(CCursor::new(last + 1)).left()
    };
    Some(Rect::from_min_max(
        pos2(left, row.origin.y),
        pos2(right.max(left), row.origin.y + geometry.line_height),
    ))
}

fn hover_rects(request: &Request<'_>) -> Vec<Rect> {
    let mut rects = request.zones.to_vec();
    for ghost in request.ghosts {
        let line_end =
            taide_native_editor::editing::line_content_range(request.document, ghost.line).end;
        rects.extend(
            ghost.inline.iter().filter_map(|part| {
                injected_rect(request.geometry, part.byte, part.byte == line_end)
            }),
        );
    }
    rects
}

fn text_width(ui: &Ui, text: &str, font: &FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.into(), font.clone(), Color32::PLACEHOLDER)
        .size()
        .x
}

fn interact(ui: &mut Ui, id: Id, size: Vec2, enabled: bool) -> (Rect, bool) {
    let (_, rect) = ui.allocate_space(size);
    let response = ui.interact(
        rect,
        id,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    ui.ctx()
        .register_pointer_preserves_keyboard_focus(response.id);
    (rect, enabled && response.clicked())
}

fn chevron(ui: &mut Ui, id: Id, right: bool, enabled: bool, colors: Colors) -> bool {
    let (rect, clicked) = interact(ui, id, Vec2::splat(ICON_SIZE), enabled);
    let center = rect.center();
    let width = ICON_SIZE * CHEVRON_RATIO;
    let height = ICON_SIZE * CHEVRON_HEIGHT_RATIO;
    let sign = if right { 1.0 } else { -1.0 };
    let color = if enabled {
        colors.foreground
    } else {
        colors.foreground.gamma_multiply(DISABLED_OPACITY)
    };
    ui.painter().add(egui::Shape::line(
        vec![
            pos2(center.x - sign * width, center.y - height),
            pos2(center.x + sign * width, center.y),
            pos2(center.x - sign * width, center.y + height),
        ],
        Stroke::new(BORDER_WIDTH, color),
    ));
    clicked
}

fn labeled(ui: &mut Ui, id: Id, label: &str, key: Option<&str>, colors: Colors) -> bool {
    let font = FontId::proportional(FONT_SIZE);
    let key_font = FontId::proportional(KEY_FONT_SIZE);
    let label_width = text_width(ui, label, &font);
    let key_width = key.map_or(0.0, |key| KEY_SPACING + text_width(ui, key, &key_font));
    let (rect, clicked) = interact(ui, id, vec2(label_width + key_width, ICON_SIZE), true);
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        label,
        font,
        colors.foreground,
    );
    if let Some(key) = key {
        ui.painter().text(
            pos2(rect.left() + label_width + KEY_SPACING, rect.center().y),
            Align2::LEFT_CENTER,
            key,
            key_font,
            colors.foreground.gamma_multiply(KEY_OPACITY),
        );
    }
    clicked
}

fn more(ui: &mut Ui, id: Id, colors: Colors) -> bool {
    let (rect, clicked) = interact(ui, id, Vec2::splat(ICON_SIZE), true);
    for offset in [-MORE_DOT_SPACING, 0.0, MORE_DOT_SPACING] {
        ui.painter().circle_filled(
            rect.center() + vec2(offset, 0.0),
            MORE_DOT_RADIUS,
            colors.foreground,
        );
    }
    clicked
}

fn menu_row(ui: &mut Ui, id: Id, label: &str, checked: bool, width: f32, colors: Colors) -> bool {
    let font = FontId::proportional(FONT_SIZE);
    let (rect, clicked) = interact(ui, id, vec2(width, ICON_SIZE + MENU_ROW_PADDING), true);
    if checked {
        let center = pos2(rect.left() + CHECK_WIDTH / 2.0, rect.center().y);
        let half = ICON_SIZE * CHEVRON_RATIO;
        let rise = ICON_SIZE * CHECK_RISE_RATIO;
        ui.painter().add(egui::Shape::line(
            vec![
                pos2(center.x - half, center.y),
                pos2(center.x - half * CHECK_KNEE_RATIO, center.y + rise),
                pos2(center.x + half, center.y - rise),
            ],
            Stroke::new(BORDER_WIDTH, colors.foreground),
        ));
    }
    ui.painter().text(
        pos2(rect.left() + CHECK_WIDTH, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        colors.foreground,
    );
    clicked
}

fn frame(colors: Colors) -> Frame {
    Frame::new()
        .fill(colors.background)
        .stroke(Stroke::new(BORDER_WIDTH, colors.border))
        .corner_radius(CornerRadius::same(CORNER_RADIUS))
        .inner_margin(PADDING)
}

pub(crate) fn show(ui: &Ui, body: Id, request: Request<'_>) -> Option<Command> {
    let mut state = load(ui, body);
    let now = ui.input(|input| input.time);
    let always = is_always_shown(ui.ctx());
    let anchor = request.ghosts.iter().find_map(|ghost| {
        let byte = ghost.inline.first().map_or_else(
            || taide_native_editor::editing::line_content_range(request.document, ghost.line).end,
            |part| part.byte,
        );
        request
            .geometry
            .caret_rect(byte)
            .map(|caret| (ghost.line, caret))
    });
    let Some((line, caret)) = anchor else {
        store(ui, body, State::default());
        return None;
    };
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let over_ghost = pointer.is_some_and(|position| {
        hover_rects(&request)
            .iter()
            .any(|rect| rect.contains(position))
    });
    let over_toolbar = pointer.is_some_and(|position| {
        state.shown
            && [state.toolbar, state.menu.filter(|_| state.menu_open)]
                .into_iter()
                .flatten()
                .any(|rect| rect.contains(position))
    });
    if always {
        state.shown = true;
        state.hovered_since = None;
        state.left_at = None;
    } else if over_ghost || over_toolbar {
        state.left_at = None;
        if !state.shown {
            let since = *state.hovered_since.get_or_insert(now);
            if now - since >= SHOW_DELAY {
                state.shown = true;
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(
                        SHOW_DELAY - (now - since),
                    ));
            }
        }
    } else {
        state.hovered_since = None;
        if state.shown {
            let left = *state.left_at.get_or_insert(now);
            if now - left >= HIDE_DELAY {
                state.shown = false;
                state.menu_open = false;
                state.left_at = None;
            } else {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(
                        HIDE_DELAY - (now - left),
                    ));
            }
        }
    }
    if !state.shown {
        state.toolbar = None;
        state.menu = None;
        state.session = None;
        store(ui, body, state);
        return None;
    }
    let left = match state.session {
        Some((session_line, left)) if always && session_line == line => left.min(caret.left()),
        _ => caret.left(),
    };
    state.session = Some((line, left));
    let height = state
        .toolbar
        .map_or(ICON_SIZE + PADDING * 2.0, |rect| rect.height());
    let above = caret.top() - height >= request.geometry.rect.top();
    let (pivot, position) = if above {
        (Align2::LEFT_BOTTOM, pos2(left, caret.top()))
    } else {
        (Align2::LEFT_TOP, pos2(left, caret.bottom()))
    };
    let colors = request.colors;
    let cycle = request.count.is_some_and(|count| count > 1);
    let is_mac = ui.ctx().os().is_mac();
    let mut command = None;
    let mut toggle_menu = false;
    let toolbar = Area::new(body.with(STATE_KEY).with("toolbar"))
        .order(Order::Foreground)
        .pivot(pivot)
        .fixed_pos(position)
        .show(ui.ctx(), |ui| {
            frame(colors).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = ITEM_SPACING;
                    if chevron(ui, body.with(PREVIOUS), false, cycle, colors) {
                        command = Some(Command::ShowPreviousInline);
                    }
                    if let Some(count) = request.count {
                        ui.label(
                            egui::RichText::new(format!("1/{count}"))
                                .size(FONT_SIZE)
                                .color(colors.foreground),
                        );
                    }
                    if chevron(ui, body.with(NEXT), true, cycle, colors) {
                        command = Some(Command::ShowNextInline);
                    }
                    if labeled(ui, body.with(ACCEPT), ACCEPT, Some(ACCEPT_KEY), colors) {
                        command = Some(Command::CommitInline);
                    }
                    let word_key = if is_mac {
                        ACCEPT_WORD_MAC_KEY
                    } else {
                        ACCEPT_WORD_KEY
                    };
                    if labeled(
                        ui,
                        body.with(ACCEPT_WORD),
                        ACCEPT_WORD,
                        Some(word_key),
                        colors,
                    ) {
                        command = Some(Command::AcceptNextWord);
                    }
                    toggle_menu = more(ui, body.with(MORE_ACTIONS), colors);
                });
            });
        })
        .response
        .rect;
    state.toolbar = Some(toolbar);
    if toggle_menu {
        state.menu_open = !state.menu_open;
    }
    let mut toggle_always = false;
    if state.menu_open {
        let font = FontId::proportional(FONT_SIZE);
        let width = CHECK_WIDTH
            + [ACCEPT_LINE, ALWAYS_SHOW_TOOLBAR]
                .into_iter()
                .map(|label| text_width(ui, label, &font))
                .fold(0.0, f32::max);
        let menu = Area::new(body.with(STATE_KEY).with("menu"))
            .order(Order::Foreground)
            .pivot(Align2::RIGHT_TOP)
            .fixed_pos(toolbar.right_bottom())
            .show(ui.ctx(), |ui| {
                frame(colors).show(ui, |ui| {
                    ui.vertical(|ui| {
                        if menu_row(
                            ui,
                            body.with(ACCEPT_LINE),
                            ACCEPT_LINE,
                            false,
                            width,
                            colors,
                        ) {
                            command = Some(Command::AcceptNextLine);
                        }
                        toggle_always = menu_row(
                            ui,
                            body.with(ALWAYS_SHOW_TOOLBAR),
                            ALWAYS_SHOW_TOOLBAR,
                            always,
                            width,
                            colors,
                        );
                    });
                });
            })
            .response
            .rect;
        state.menu = Some(menu);
    } else {
        state.menu = None;
    }
    if toggle_always {
        toggle_always_shown(ui.ctx());
    }
    if command.is_some() || toggle_always {
        state.menu_open = false;
    }
    store(ui, body, state);
    command
}

#[cfg(feature = "inspection")]
pub fn inspect(context: &Context, body: Id) -> Option<(Rect, Option<Rect>)> {
    let state = context
        .data(|data| data.get_temp::<State>(body.with(STATE_KEY)))
        .unwrap_or_default();
    state
        .shown
        .then_some(state.toolbar)
        .flatten()
        .map(|toolbar| (toolbar, state.menu.filter(|_| state.menu_open)))
}
