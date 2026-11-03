use super::*;
use crate::modal::{self, Chrome, Composition, Layer, Transition};

const SMALL_WIDTH: f32 = 320.0;
const DEFAULT_WIDTH: f32 = 512.0;
const WIDE_BREAKPOINT: f32 = 640.0;
const VIEWPORT_MARGIN: f32 = 32.0;
const PADDING: i8 = 24;
const CONTENT_GAP: f32 = 16.0;
const ALERT_HEADER_GAP: f32 = 6.0;
const DIALOG_HEADER_GAP: f32 = 8.0;
const DIALOG_TITLE_SIZE: f32 = 18.0;
const DESCRIPTION_SIZE: f32 = 14.0;
const BUTTON_HEIGHT: f32 = 36.0;
const BUTTON_PADDING_X: f32 = 16.0;
const FOOTER_GAP: f32 = 8.0;
const CLOSE_INSET: f32 = 16.0;
const CLOSE_SIZE: f32 = 16.0;
const CLOSE_OPACITY: f32 = 0.7;
const SIDES: f32 = 2.0;

/// Focus and layer bookkeeping of one open Settings dialog.
pub(super) struct Focus {
    origin: Option<Id>,
    is_focus_pending: bool,
    layer: Option<(egui::Context, egui::LayerId)>,
    composition: Composition,
}

impl Focus {
    pub(super) fn open(origin: Id) -> Self {
        Self {
            origin: Some(origin),
            is_focus_pending: true,
            layer: None,
            composition: Composition::default(),
        }
    }

    /// Closes the dialog and returns the control to refocus once its layer stops being the top modal.
    pub(super) fn close(mut self) -> Option<Returning> {
        let layer = self.layer.as_ref().map(|(_, layer)| *layer)?;
        self.origin.take().map(|origin| Returning { origin, layer })
    }

    fn mount(&mut self, context: &egui::Context, layer: &Layer) {
        if self.layer.is_none() {
            self.layer = Some((context.clone(), layer.layer_id()));
        }
    }
}

/// Focus waiting to return to the control that opened a closed dialog.
#[derive(Clone, Copy, Debug)]
pub(super) struct Returning {
    origin: Id,
    layer: egui::LayerId,
}

/// Returns focus to the opener once the closed dialog no longer owns modal focus.
pub(super) fn settle(ui: &Ui, returning: &mut Option<Returning>) {
    let Some(pending) = *returning else {
        return;
    };
    if ui.memory(|memory| memory.top_modal_layer()) == Some(pending.layer) {
        ui.ctx().request_repaint();
        return;
    }
    *returning = None;
    ui.memory_mut(|memory| memory.request_focus(pending.origin));
}

impl Drop for Focus {
    fn drop(&mut self) {
        if let Some((context, layer)) = self.layer.take() {
            modal::unmount(&context, layer);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Size {
    Small,
    Default,
}

pub(super) struct Alert {
    pub(super) title: String,
    pub(super) description: String,
    pub(super) action: String,
    pub(super) is_destructive: bool,
    pub(super) size: Size,
    pub(super) alternative: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Choice {
    Cancel,
    Alternative,
    Confirm,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Style {
    Outline,
    Primary,
    Destructive,
}

fn layer(id: Id, appearance: &super::Appearance) -> Layer {
    Layer {
        id,
        chrome: Chrome {
            background: appearance.sections.modal,
            border: appearance.sections.modal_border,
            shadow: appearance.sections.shadow,
        },
        padding: PADDING,
        transition: Some(Transition::SETTLED),
        is_modal: true,
    }
}

fn content_width(context: &egui::Context, size: Size) -> f32 {
    let viewport = context.content_rect().width();
    let width = match size {
        Size::Small => SMALL_WIDTH.min(viewport),
        Size::Default if viewport >= WIDE_BREAKPOINT => DEFAULT_WIDTH,
        Size::Default => viewport - VIEWPORT_MARGIN,
    };
    (width - f32::from(PADDING) * SIDES - BORDER_WIDTH * SIDES).max(0.0)
}

pub(super) fn button(
    ui: &mut Ui,
    text: &str,
    style: Style,
    enabled: bool,
    width: Option<f32>,
    appearance: &super::Appearance,
) -> egui::Response {
    let (fill, foreground, stroke) = match style {
        Style::Outline => (
            appearance.background,
            appearance.foreground,
            Stroke::new(BORDER_WIDTH, appearance.border),
        ),
        Style::Primary => (
            appearance.primary,
            appearance.sections.primary_foreground,
            Stroke::NONE,
        ),
        Style::Destructive => (appearance.sections.error, Color32::WHITE, Stroke::NONE),
    };
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(BUTTON_PADDING_X, 0.0);
        ui.add_enabled(
            enabled,
            egui::Button::new(RichText::new(text).size(DESCRIPTION_SIZE).color(foreground))
                .fill(fill)
                .stroke(stroke)
                .corner_radius(ROW_RADIUS)
                .min_size(egui::vec2(width.unwrap_or_default(), BUTTON_HEIGHT)),
        )
    })
    .inner
}

fn header(ui: &mut Ui, title: &str, description: &str, gap: f32, appearance: &super::Appearance) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        ui.add(
            egui::Label::new(
                RichText::new(title)
                    .size(DIALOG_TITLE_SIZE)
                    .strong()
                    .color(appearance.foreground),
            )
            .wrap(),
        );
        ui.add(
            egui::Label::new(
                RichText::new(description)
                    .size(DESCRIPTION_SIZE)
                    .color(appearance.muted),
            )
            .wrap(),
        );
    });
}

fn focus_first(ui: &Ui, focus: &mut Focus, order: &[Id]) {
    if !focus.is_focus_pending || ui.is_sizing_pass() {
        return;
    }
    if let Some(first) = order.first() {
        ui.memory_mut(|memory| memory.request_focus_with_filter(*first, modal::FOCUS_FILTER));
        focus.is_focus_pending = false;
    }
}

/// Draws a Radix-style alert dialog: only Escape or a button closes it, and the action closes it at
/// once.
pub(super) fn alert(
    ui: &Ui,
    id: Id,
    focus: &mut Focus,
    alert: &Alert,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    _output: &mut Output,
) -> Option<Choice> {
    let context = ui.ctx().clone();
    let layer = layer(id, appearance);
    focus.mount(&context, &layer);
    let width = content_width(&context, alert.size);
    let enabled = ui.is_enabled();
    let mut order = Vec::new();
    let mut choice = None;
    let response = layer.show(&context, |ui| {
        ui.set_width(width);
        ui.visuals_mut().override_text_color = Some(appearance.foreground);
        ui.spacing_mut().item_spacing = egui::vec2(FOOTER_GAP, CONTENT_GAP);
        if !enabled {
            ui.disable();
        }
        ui.with_layout(
            egui::Layout::top_down(if alert.size == Size::Small {
                Align::Center
            } else {
                Align::Min
            }),
            |ui| {
                header(
                    ui,
                    &alert.title,
                    &alert.description,
                    ALERT_HEADER_GAP,
                    appearance,
                )
            },
        );
        let cancel_text = alert
            .alternative
            .clone()
            .unwrap_or_else(|| message(locale, "common.cancel", &[]));
        let action_style = if alert.is_destructive {
            Style::Destructive
        } else {
            Style::Primary
        };
        let (cancel, confirm) = if alert.size == Size::Small {
            let column = ((ui.available_width() - FOOTER_GAP) * HALF).max(0.0);
            ui.horizontal(|ui| {
                let cancel = button(
                    ui,
                    &cancel_text,
                    Style::Outline,
                    true,
                    Some(column),
                    appearance,
                );
                let confirm = button(
                    ui,
                    &alert.action,
                    action_style,
                    true,
                    Some(column),
                    appearance,
                );
                (cancel, confirm)
            })
            .inner
        } else {
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), BUTTON_HEIGHT),
                egui::Layout::right_to_left(Align::Center),
                |ui| {
                    let confirm = button(ui, &alert.action, action_style, true, None, appearance);
                    let cancel = button(ui, &cancel_text, Style::Outline, true, None, appearance);
                    (cancel, confirm)
                },
            )
            .inner
        };
        #[cfg(any(test, feature = "inspection"))]
        {
            _output
                .traces
                .push(Trace::capture("settings.dialog.cancel", &cancel));
            _output
                .traces
                .push(Trace::capture("settings.dialog.confirm", &confirm));
        }
        order = vec![cancel.id, confirm.id];
        focus_first(ui, focus, &order);
        if cancel.clicked() {
            choice = Some(if alert.alternative.is_some() {
                Choice::Alternative
            } else {
                Choice::Cancel
            });
        }
        if confirm.clicked() {
            choice = Some(Choice::Confirm);
        }
    });
    if enabled && response.is_top_modal && !response.any_popup_open {
        modal::trap_focus(&context, &order);
    }
    if enabled && modal::takes_escape(&context, &response) {
        choice = Some(Choice::Cancel);
    }
    choice
}

/// Draws a Radix-style dialog with a corner close button; Escape outside an IME composition and a
/// click outside close it. Returns the content result and whether the dialog closes.
pub(super) fn dialog<R>(
    ui: &Ui,
    id: Id,
    focus: &mut Focus,
    title: &str,
    description: &str,
    locale: &ResolvedLocale,
    appearance: &super::Appearance,
    icons: &Icons,
    output: &mut Output,
    content: impl FnOnce(&mut Ui, &mut Vec<Id>, &mut Output) -> R,
) -> (R, bool) {
    let context = ui.ctx().clone();
    let is_composing = ui.input(|input| focus.composition.observe(&input.events));
    let layer = layer(id, appearance);
    focus.mount(&context, &layer);
    let width = content_width(&context, Size::Default);
    let enabled = ui.is_enabled();
    let mut order = Vec::new();
    let mut close = false;
    let response = layer.show(&context, |ui| {
        ui.set_width(width);
        ui.visuals_mut().override_text_color = Some(appearance.foreground);
        ui.spacing_mut().item_spacing = egui::vec2(FOOTER_GAP, CONTENT_GAP);
        if !enabled {
            ui.disable();
        }
        let top = ui.max_rect();
        header(ui, title, description, DIALOG_HEADER_GAP, appearance);
        let result = content(ui, &mut order, output);
        let footer = ui
            .allocate_ui_with_layout(
                egui::vec2(ui.available_width(), BUTTON_HEIGHT),
                egui::Layout::right_to_left(Align::Center),
                |ui| {
                    button(
                        ui,
                        &message(locale, "common.close", &[]),
                        Style::Outline,
                        true,
                        None,
                        appearance,
                    )
                },
            )
            .inner;
        let corner = Rect::from_min_size(
            egui::pos2(
                top.right() + f32::from(PADDING) - CLOSE_INSET - CLOSE_SIZE,
                top.top() - f32::from(PADDING) + CLOSE_INSET,
            ),
            egui::Vec2::splat(CLOSE_SIZE),
        );
        let label = message(locale, "common.close", &[]);
        let corner_close = ui
            .scope(|ui| {
                ui.set_opacity(CLOSE_OPACITY);
                ui.put(
                    corner,
                    egui::Button::opt_image_and_text(
                        icons
                            .image(Icon::Close, appearance.foreground)
                            .map(|image| image.fit_to_exact_size(egui::Vec2::splat(CLOSE_SIZE))),
                        None,
                    )
                    .frame(false),
                )
            })
            .inner;
        corner_close.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, corner_close.enabled(), &label)
        });
        #[cfg(any(test, feature = "inspection"))]
        {
            output
                .traces
                .push(Trace::capture("settings.dialog.close", &footer));
            output
                .traces
                .push(Trace::capture("settings.dialog.corner", &corner_close));
        }
        order.push(footer.id);
        order.push(corner_close.id);
        focus_first(ui, focus, &order);
        close = footer.clicked() || corner_close.clicked();
        result
    });
    if enabled && response.is_top_modal && !response.any_popup_open {
        modal::trap_focus(&context, &order);
    }
    if enabled && !is_composing && modal::takes_escape(&context, &response) {
        close = true;
    }
    if enabled && response.is_top_modal && response.backdrop_response.clicked() {
        close = true;
    }
    (response.inner, close)
}
