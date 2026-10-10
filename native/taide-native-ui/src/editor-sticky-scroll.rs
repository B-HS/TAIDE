use std::borrow::Cow;
use std::sync::Arc;

use egui::{
    Color32, Context, CornerRadius, Event, FontId, Id, Key, PointerButton, Rect, Sense, Shadow, Ui,
    Vec2, pos2,
};
use taide_model::file::FileSizeTier;
use taide_native_editor::decoration::{
    Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness,
};
use taide_native_editor::document::{DocumentSnapshot, EditorError};
use taide_native_editor::folding::{
    FoldRegion, FoldingModel, MAX_FOLDING_REGIONS, indent_regions, language_regions,
};
use taide_native_editor::language_configuration::LanguageRules;
use taide_native_editor::sticky_model::{StickyLayout, StickyModel, StickyScope};
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::SelectionSet;

use crate::editor_geometry::{Row, RowLayout};
use crate::editor_gutter::{FoldControlFade, Gutter};
use crate::editor_paint::{Carets, Layers, frame_decorations};
use crate::editor_surface::{
    EditorAppearance, EditorDisplayOptions, FoldControl, FoldControlPainter,
};

const BORDER_WIDTH: f32 = 1.0;
const SHADOW_OFFSET: [i8; 2] = [0, 4];
const SHADOW_BLUR: u8 = 2;
const SHADOW_INSET: f32 = 2.0;
const COLLAPSED_ROTATION: f32 = 0.0;
const EXPANDED_ROTATION: f32 = std::f32::consts::FRAC_PI_2;
const COLLAPSED_LAST_LINE_OFFSET: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorStickyColors {
    pub background: Color32,
    pub border: Color32,
    pub hover: Color32,
    pub shadow: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickySetting {
    configured: bool,
    enabled: bool,
}

impl StickySetting {
    pub fn new(configured: bool) -> Self {
        Self {
            configured,
            enabled: configured,
        }
    }

    pub fn synchronize(&mut self, configured: bool) -> bool {
        if self.configured != configured {
            self.configured = configured;
            self.enabled = configured;
        }
        self.enabled
    }

    pub fn toggle(&mut self, configured: bool) {
        self.enabled = !self.synchronize(configured);
    }
}

#[derive(Clone)]
struct CachedModel {
    model: Arc<StickyModel>,
    regions: Option<Arc<[FoldRegion]>>,
    tab_size: u32,
    has_language_rules: bool,
}

#[derive(Clone)]
struct StickyRow {
    id: Id,
    row: Row,
    scope: StickyScope,
    rect: Rect,
    fold_end: Option<usize>,
    fold_zone: egui::Rangef,
    text_left: f32,
}

#[derive(Clone)]
struct PaintedState {
    document: DocumentSnapshot,
    rect: Rect,
    scroll: Vec2,
    font: FontId,
    line_height: f32,
    horizontal_padding: f32,
    line_numbers: bool,
    tab_size: u32,
    word_wrap: bool,
    folding: bool,
    folds: Vec<std::ops::Range<usize>>,
    bold_family: Option<egui::FontFamily>,
    supplied: Option<Arc<StickyModel>>,
}

#[derive(Default, Clone)]
pub(crate) struct StickyState {
    cache: Option<CachedModel>,
    shown: Vec<StickyRow>,
    painted: Option<PaintedState>,
    pressed: Option<usize>,
    fold_reveal: Option<(usize, usize)>,
    control_fade: FoldControlFade,
}

impl StickyState {
    pub(crate) fn contains_point(&self, point: egui::Pos2) -> bool {
        self.shown.iter().any(|row| row.rect.contains(point))
    }

    pub(crate) fn content_byte_at(&self, point: egui::Pos2) -> Option<usize> {
        self.shown
            .iter()
            .find(|row| row.rect.contains(point))
            .filter(|row| {
                point.x >= row.text_left
                    && point.x >= row.row.caret_rect(row.row.segment.bytes.start).left()
                    && point.x < row.row.caret_rect(row.row.segment.bytes.end).left()
            })
            .map(|row| row.row.byte_at(point))
    }

    pub(crate) fn definition_range_rects(&self, bytes: &std::ops::Range<usize>) -> Vec<Rect> {
        self.shown
            .iter()
            .filter_map(|row| {
                row.row.extent(bytes).map(|extent| {
                    Rect::from_x_y_ranges(
                        extent,
                        egui::Rangef::new(row.rect.top(), row.rect.bottom()),
                    )
                    .intersect(row.rect)
                })
            })
            .filter(|rect| rect.is_positive())
            .collect()
    }

    pub(crate) fn model(
        &mut self,
        document: &DocumentSnapshot,
        tab_size: u32,
        rules: Option<&dyn LanguageRules>,
        regions: Option<&Arc<[FoldRegion]>>,
        supplied: Option<&Arc<StickyModel>>,
    ) -> Arc<StickyModel> {
        if let Some(model) = supplied.filter(|model| model.describes(document)) {
            return Arc::clone(model);
        }
        if let Some(cache) = self.cache.as_ref().filter(|cache| {
            cache.model.describes(document)
                && cache.tab_size == tab_size
                && cache.has_language_rules == rules.is_some()
                && match (&cache.regions, regions) {
                    (Some(known), Some(current)) => Arc::ptr_eq(known, current),
                    (None, None) => true,
                    _ => false,
                }
        }) {
            return Arc::clone(&cache.model);
        }
        let calculated;
        let folds = if let Some(regions) = regions {
            regions.as_ref()
        } else {
            calculated = match rules {
                Some(rules) => {
                    language_regions(&document.rope, tab_size, MAX_FOLDING_REGIONS, rules)
                }
                None => indent_regions(&document.rope, tab_size, MAX_FOLDING_REGIONS),
            };
            &calculated
        };
        let model = Arc::new(StickyModel::from_folds(document, folds));
        self.cache = Some(CachedModel {
            model: Arc::clone(&model),
            regions: regions.cloned(),
            tab_size,
            has_language_rules: rules.is_some(),
        });
        model
    }

    pub(crate) fn clear(&mut self) {
        self.cache = None;
        self.shown.clear();
        self.painted = None;
        self.pressed = None;
        self.fold_reveal = None;
        self.control_fade = FoldControlFade::default();
    }

    pub(crate) fn focus_ids(&self) -> Vec<Id> {
        self.shown.iter().map(|row| row.id).collect()
    }

    pub(crate) fn prepare(
        &mut self,
        document: &DocumentSnapshot,
        rect: Rect,
        scroll: Vec2,
        appearance: &EditorAppearance,
        options: &EditorDisplayOptions,
        tab_size: u32,
        folds: &[std::ops::Range<usize>],
    ) -> bool {
        let valid = options.sticky_scroll
            && options.sticky_colors.is_some()
            && is_available(document)
            && self.painted.as_ref().is_some_and(|painted| {
                painted.document.id == document.id
                    && painted.document.revision == document.revision
                    && painted.document.metadata.language_id == document.metadata.language_id
                    && painted.rect == rect
                    && painted.scroll == scroll
                    && painted.font == appearance.font
                    && painted.line_height == appearance.line_height
                    && painted.horizontal_padding == appearance.horizontal_padding
                    && painted.line_numbers == appearance.line_numbers
                    && painted.tab_size == tab_size
                    && painted.word_wrap == options.word_wrap
                    && painted.folding == options.folding
                    && painted.folds == folds
                    && painted.bold_family == options.bold_family
                    && match (&painted.supplied, &options.sticky_model) {
                        (Some(known), Some(current)) => Arc::ptr_eq(known, current),
                        (None, None) => true,
                        _ => false,
                    }
            });
        if !valid {
            self.shown.clear();
            self.painted = None;
            self.pressed = None;
        }
        valid
    }

    pub(crate) fn pointer(
        &mut self,
        context: &Context,
        event: &Event,
        raw_index: usize,
    ) -> Option<StickyAction> {
        let Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers,
        } = event
        else {
            return None;
        };
        if *pressed {
            self.pressed = self.shown.iter().position(|row| {
                row.rect.contains(*pos)
                    && context.keyboard_focus_request_at(raw_index) == Some(row.id.accesskit_id())
            });
            return None;
        }
        let index = self.pressed.take()?;
        let row = self
            .shown
            .get(index)
            .filter(|row| row.rect.contains(*pos))?;
        if modifiers.ctrl || modifiers.command || modifiers.alt {
            return None;
        }
        if modifiers.shift {
            return self.painted.as_ref().map(|painted| StickyAction::Jump {
                byte: painted.document.rope.line_to_byte(row.scope.end_line),
                center: true,
            });
        }
        if row.fold_zone.contains(pos.x)
            && let Some(end) = row.fold_end
        {
            return Some(StickyAction::Fold {
                line: row.scope.start_line,
                end,
                index,
            });
        }
        (pos.x >= row.text_left).then(|| StickyAction::Jump {
            byte: row.row.byte_at(*pos),
            center: false,
        })
    }

    pub(crate) fn key(&self, event: &Event, index: usize) -> Option<StickyAction> {
        let Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } = event
        else {
            return None;
        };
        if !modifiers.is_none() {
            return None;
        }
        match key {
            Key::ArrowUp => Some(StickyAction::Focus(index.saturating_sub(1))),
            Key::ArrowDown => Some(StickyAction::Focus(
                (index + 1).min(self.shown.len().saturating_sub(1)),
            )),
            Key::Escape => Some(StickyAction::Exit),
            Key::Enter => self
                .shown
                .get(index)
                .zip(self.painted.as_ref())
                .map(|(row, painted)| StickyAction::Jump {
                    byte: painted.document.rope.line_to_byte(row.scope.start_line),
                    center: false,
                }),
            _ => None,
        }
    }

    pub(crate) fn focus(&self, ui: &Ui, index: usize) {
        if let Some(row) = self.shown.get(index) {
            ui.memory_mut(|memory| memory.request_focus(row.id));
        }
    }

    pub(crate) fn track_action(
        &self,
        store: &EditorStore,
        document: &DocumentSnapshot,
        action: StickyAction,
    ) -> Result<Option<StickyAction>, EditorError> {
        let Some(painted) = self.painted.as_ref().filter(|painted| {
            painted.document.id == document.id
                && painted.document.metadata.language_id == document.metadata.language_id
        }) else {
            return Ok(None);
        };
        if painted.document.revision == document.revision {
            return Ok(Some(action));
        }
        let marker = |byte| Decoration {
            bytes: byte..byte,
            kind: DecorationKind::Inline(InlineStyle::default()),
            stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
        };
        let track = |byte| -> Result<Option<usize>, EditorError> {
            let layer = DecorationLayer::new(painted.document.revision, 0, vec![marker(byte)]);
            Ok(layer
                .tracking(store.changes_since(document.id, painted.document.revision)?)
                .and_then(|tracked| tracked.items().first().map(|item| item.bytes.start)))
        };
        Ok(match action {
            StickyAction::Jump { byte, center } => {
                track(byte)?.map(|byte| StickyAction::Jump { byte, center })
            }
            StickyAction::Fold { line, end, index } => {
                track(painted.document.rope.line_to_byte(line))?
                    .zip(track(painted.document.rope.line_to_byte(end))?)
                    .map(|(line, end)| StickyAction::Fold {
                        line: document.rope.byte_to_line(line),
                        end: document.rope.byte_to_line(end),
                        index,
                    })
            }
            _ => Some(action),
        })
    }

    pub(crate) fn take_fold_reveal(&mut self) -> Option<(usize, usize)> {
        self.fold_reveal.take()
    }

    pub(crate) fn reveal_fold(&mut self, line: usize, index: usize) {
        self.fold_reveal = Some((line, index));
    }
}

pub(crate) fn is_available(document: &DocumentSnapshot) -> bool {
    document.metadata.tier == FileSizeTier::Normal
}

pub(crate) struct StickyFrame<'a> {
    pub(crate) id: Id,
    pub(crate) layout: &'a StickyLayout,
    pub(crate) rows: &'a RowLayout<'a>,
    pub(crate) rect: Rect,
    pub(crate) text_rect: Rect,
    pub(crate) scroll: Vec2,
    pub(crate) gutter: Gutter,
    pub(crate) colors: EditorStickyColors,
    pub(crate) layers: &'a [Cow<'a, DecorationLayer>],
    pub(crate) regions: &'a [FoldRegion],
    pub(crate) folds: &'a [std::ops::Range<usize>],
    pub(crate) selection: &'a SelectionSet,
    pub(crate) options: &'a EditorDisplayOptions,
}

#[derive(Clone, Copy)]
pub(crate) enum StickyAction {
    Focus(usize),
    Exit,
    Jump {
        byte: usize,
        center: bool,
    },
    Fold {
        line: usize,
        end: usize,
        index: usize,
    },
}

pub(crate) fn paint(
    ui: &mut Ui,
    state: &mut StickyState,
    frame: StickyFrame<'_>,
    controls: &mut Option<&mut FoldControlPainter<'_>>,
) -> bool {
    state.shown.clear();
    if frame.layout.scopes.is_empty() {
        state.painted = None;
        return false;
    }
    let appearance = frame.rows.appearance;
    let height = frame.layout.height(appearance.line_height);
    let sticky_rect = Rect::from_min_max(
        frame.rect.min,
        pos2(frame.rect.right(), frame.rect.top() + height),
    );
    let painter = ui.painter().with_clip_rect(frame.rect);
    painter.add(
        Shadow {
            offset: SHADOW_OFFSET,
            blur: SHADOW_BLUR,
            spread: 0,
            color: frame.colors.shadow,
        }
        .as_shape(sticky_rect.shrink(SHADOW_INSET), CornerRadius::ZERO),
    );
    painter.rect_filled(sticky_rect, 0.0, frame.colors.background);
    let model = FoldingModel::new(frame.regions, frame.rows.document, frame.folds);
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let modifiers = ui.input(|input| input.modifiers);
    let mut toggle = false;
    let is_gutter_hovered = ui.is_enabled()
        && pointer.is_some_and(|pointer| {
            sticky_rect.contains(pointer) && pointer.x < frame.text_rect.left()
        })
        && frame.layout.scopes.iter().enumerate().any(|(index, _)| {
            ui.ctx()
                .read_response(frame.id.with(("sticky-line", index)))
                .is_some_and(|response| response.hovered())
        });
    let (control_opacity, is_fading) = state
        .control_fade
        .sample(is_gutter_hovered, ui.input(|input| input.time));
    if is_fading {
        ui.ctx().request_repaint();
    }
    state.painted = Some(PaintedState {
        document: frame.rows.document.clone(),
        rect: frame.rect,
        scroll: frame.scroll,
        font: appearance.font.clone(),
        line_height: appearance.line_height,
        horizontal_padding: appearance.horizontal_padding,
        line_numbers: appearance.line_numbers,
        tab_size: frame.rows.tab_size,
        word_wrap: frame.options.word_wrap,
        folding: frame.options.folding,
        folds: frame.folds.to_vec(),
        bold_family: frame.options.bold_family.clone(),
        supplied: frame.options.sticky_model.clone(),
    });
    for (index, scope) in frame.layout.scopes.iter().enumerate() {
        let mut top = frame.rect.top() + frame.layout.top(index, appearance.line_height);
        let band = Rect::from_min_max(
            pos2(
                frame.rect.left(),
                frame.rect.top() + index as f32 * appearance.line_height,
            ),
            pos2(frame.rect.right(), top + appearance.line_height),
        )
        .intersect(sticky_rect);
        if band.height() <= 0.0 {
            continue;
        }
        let response = ui.interact(band, frame.id.with(("sticky-line", index)), Sense::click());
        let hovered = response.hovered();
        let line = if hovered && modifiers.shift {
            scope.end_line
        } else {
            scope.start_line
        };
        if index + 1 == frame.layout.scopes.len() && model.header(line) == Some(true) {
            top += COLLAPSED_LAST_LINE_OFFSET;
        }
        let document = frame.rows.document;
        let bytes = document.rope.line_to_byte(line);
        let decorations = frame_decorations(document, frame.layers, line..line + 1)
            .into_iter()
            .filter(|decoration| {
                !matches!(
                    decoration.kind,
                    DecorationKind::Lane { .. } | DecorationKind::LineBackground(_)
                )
            })
            .collect::<Vec<_>>();
        let row = RowLayout {
            decorations: &decorations,
            ..*frame.rows
        }
        .row(
            frame.rows.display.row_of_byte(document, bytes),
            pos2(frame.text_rect.left() - frame.scroll.x, top),
        );
        ui.ctx().register_pointer_keyboard_focus(response.id);
        if let Some(label) = frame.options.sticky_toggle_label.as_deref() {
            response.context_menu(|ui| {
                let mut enabled = true;
                if ui.checkbox(&mut enabled, label).changed() {
                    toggle = true;
                    ui.close();
                }
            });
        }
        if response.has_focus() {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
        }
        if hovered {
            painter.rect_filled(band.intersect(frame.text_rect), 0.0, frame.colors.hover);
        }
        let row_painter = painter.with_clip_rect(band);
        let text_painter = row_painter.with_clip_rect(frame.text_rect.intersect(band));
        let layers = Layers {
            painter: &row_painter,
            text_painter: &text_painter,
            rect: frame.rect,
            text_rect: frame.text_rect,
            gutter: frame.gutter,
            appearance,
            decorations: &decorations,
        };
        layers.row_content(
            &row,
            &Carets {
                selections: frame.selection,
                head_rows: &[],
                primary_row: 0,
                primary_line: 0,
                focused: false,
            },
        );
        let control = frame
            .gutter
            .fold_control_rect(frame.rect.left(), &row, appearance);
        if let Some(collapsed) = model.header(line)
            && let Some(paint) = controls.as_deref_mut()
            && (collapsed || control_opacity > 0.0)
        {
            let clip = ui.clip_rect();
            ui.set_clip_rect(band.intersect(clip));
            paint(
                ui,
                FoldControl {
                    rect: control,
                    chevron_rotation: if collapsed {
                        COLLAPSED_ROTATION
                    } else {
                        EXPANDED_ROTATION
                    },
                    color: if collapsed {
                        ui.visuals().weak_text_color()
                    } else {
                        ui.visuals()
                            .weak_text_color()
                            .gamma_multiply(control_opacity)
                    },
                },
            );
            ui.set_clip_rect(clip);
        }
        let row = if line == scope.start_line {
            row
        } else {
            frame.rows.row(
                frame
                    .rows
                    .display
                    .row_of_byte(document, document.rope.line_to_byte(scope.start_line)),
                row.origin,
            )
        };
        state.shown.push(StickyRow {
            id: response.id,
            row,
            scope: *scope,
            rect: band,
            fold_end: frame
                .regions
                .iter()
                .find(|region| region.start_line == scope.start_line)
                .map(|region| region.end_line),
            fold_zone: frame.gutter.fold_click_zone(frame.rect.left()),
            text_left: frame.text_rect.left(),
        });
    }
    painter.rect_filled(
        Rect::from_min_max(
            pos2(sticky_rect.left(), sticky_rect.bottom()),
            pos2(sticky_rect.right(), sticky_rect.bottom() + BORDER_WIDTH),
        ),
        0.0,
        frame.colors.border,
    );
    toggle
}
