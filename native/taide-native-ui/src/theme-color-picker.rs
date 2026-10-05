use egui::{self, Color32, Id, Key, PointerButton, Pos2, Rect, Response, Sense, Stroke, Ui};
use taide_model::{error::AppResult, locale::ResolvedLocale, theme::ResolvedTheme};

use crate::presentation::{color, message};

const SQUARE_SIZE: f32 = 176.0;
const HUE_HEIGHT: f32 = 14.0;
const POPUP_WIDTH: f32 = 208.0;
const POPUP_GAP: f32 = 12.0;
const SWATCH_SIZE: f32 = 16.0;
const TRIGGER_HEIGHT: f32 = 24.0;
const THUMB_RADIUS: f32 = 6.0;
const THUMB_STROKE: f32 = 2.0;
const INPUT_WIDTH: f32 = 176.0;
const HUE_DEGREES: f64 = 360.0;
const HUE_SEGMENT: f64 = 60.0;
const HUE_KEY_STEP: f64 = 1.0;
const SV_KEY_STEP: f64 = 0.05;
const RGB_MAX: f64 = 255.0;
const PERCENT_MAX: f64 = 100.0;
const HALF: f32 = 0.5;
const TRIGGER_PADDING: f32 = 6.0;
const TRIGGER_PADDING_COUNT: f32 = 3.0;
const TRIGGER_RADIUS: u8 = 2;
const BORDER_WIDTH: f32 = 1.0;
const VALUE_FONT: f32 = 11.0;
const SLIDER_SEGMENTS: usize = 6;
const HSV_GREEN_OFFSET: f64 = 2.0;
const HSV_BLUE_OFFSET: f64 = 4.0;
const HEX_CHANNEL_LENGTH: usize = 2;
const HEX_RADIX: u32 = 16;
const SHORT_HEX: usize = 3;
const RGB_HEX: usize = 6;
const RGBA_HEX: usize = 8;
const HUE_STOPS: [Color32; 7] = [
    Color32::from_rgb(255, 0, 0),
    Color32::from_rgb(255, 255, 0),
    Color32::from_rgb(0, 255, 0),
    Color32::from_rgb(0, 255, 255),
    Color32::from_rgb(0, 0, 255),
    Color32::from_rgb(255, 0, 255),
    Color32::from_rgb(255, 0, 0),
];

pub struct Appearance {
    background: Color32,
    foreground: Color32,
    muted: Color32,
    border: Color32,
    input: Color32,
    error: Color32,
    focus: Color32,
}

impl Appearance {
    pub fn new(theme: &ResolvedTheme) -> AppResult<Self> {
        Ok(Self {
            background: color(theme, "popover.background")?,
            foreground: color(theme, "app.foreground")?,
            muted: color(theme, "appSidebar.iconDefault")?,
            border: color(theme, "app.border")?,
            input: color(theme, "panel.inputBackground")?,
            error: color(theme, "statusIndicator.error")?,
            focus: color(theme, "app.focusBorder")?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Hsv {
    h: f64,
    s: f64,
    v: f64,
}

impl Hsv {
    fn from_color(value: &str) -> Option<Self> {
        let hex = value.trim().strip_prefix('#')?;
        if ![SHORT_HEX, RGB_HEX, RGBA_HEX].contains(&hex.len())
            || !hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        let mut channels = [0.0; 3];
        for (index, channel) in channels.iter_mut().enumerate() {
            let raw = if hex.len() == SHORT_HEX {
                hex[index..index + 1].repeat(HEX_CHANNEL_LENGTH)
            } else {
                hex[index * HEX_CHANNEL_LENGTH..index * HEX_CHANNEL_LENGTH + HEX_CHANNEL_LENGTH]
                    .into()
            };
            *channel = f64::from(u8::from_str_radix(&raw, HEX_RADIX).ok()?) / RGB_MAX;
        }
        let [r, g, b] = channels;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            HUE_SEGMENT * (((g - b) / delta) % SLIDER_SEGMENTS as f64)
        } else if max == g {
            HUE_SEGMENT * ((b - r) / delta + HSV_GREEN_OFFSET)
        } else {
            HUE_SEGMENT * ((r - g) / delta + HSV_BLUE_OFFSET)
        };
        Some(Self {
            h: (h + HUE_DEGREES) % HUE_DEGREES,
            s: if max == 0.0 { 0.0 } else { delta / max },
            v: max,
        })
    }

    fn rgb(self) -> [u8; 3] {
        let c = self.v * self.s;
        let x = c * (1.0 - ((self.h / HUE_SEGMENT) % HSV_GREEN_OFFSET - 1.0).abs());
        let m = self.v - c;
        let channels = match ((self.h / HUE_SEGMENT).floor() as usize) % SLIDER_SEGMENTS {
            0 => [c, x, 0.0],
            1 => [x, c, 0.0],
            2 => [0.0, c, x],
            3 => [0.0, x, c],
            4 => [x, 0.0, c],
            _ => [c, 0.0, x],
        };
        channels.map(|channel| ((channel + m) * RGB_MAX).clamp(0.0, RGB_MAX).round() as u8)
    }

    fn hex(self) -> String {
        let [r, g, b] = self.rgb();
        format!("#{r:02x}{g:02x}{b:02x}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slider {
    Square,
    Hue,
}

pub struct Picker {
    stored: String,
    hex_input: String,
    hex_error: bool,
    open: bool,
    drag: Option<(Slider, Hsv)>,
    tooltip: Option<crate::tooltip_trigger::Trigger>,
    #[cfg(any(test, feature = "inspection"))]
    traces: [Option<(Id, Rect)>; 4],
}

impl Picker {
    pub fn take_tooltip(&mut self) -> Option<crate::tooltip_trigger::Trigger> {
        self.tooltip.take()
    }

    #[cfg(any(test, feature = "inspection"))]
    pub fn input_traces(&self) -> &[Option<(Id, Rect)>; 4] {
        &self.traces
    }

    pub fn trigger_width(ui: &Ui, value: &str, locale: &ResolvedLocale) -> f32 {
        let label = if value.trim().eq_ignore_ascii_case("transparent") {
            message(locale, "themeEditor.transparentLabel", &[])
        } else {
            value.into()
        };
        ui.painter()
            .layout_no_wrap(label, egui::FontId::monospace(VALUE_FONT), Color32::WHITE)
            .size()
            .x
            + SWATCH_SIZE
            + TRIGGER_PADDING * TRIGGER_PADDING_COUNT
    }

    pub fn new(value: String) -> Self {
        Self {
            hex_input: value.clone(),
            stored: value,
            hex_error: false,
            open: false,
            drag: None,
            tooltip: None,
            #[cfg(any(test, feature = "inspection"))]
            traces: [None; 4],
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        id: Id,
        value: &str,
        locale: &ResolvedLocale,
        appearance: &Appearance,
    ) -> Option<String> {
        if self.stored != value {
            self.stored = value.into();
            self.hex_input = value.into();
        }
        let mut changed = None;
        ui.push_id(id, |ui| {
            let label = if value.trim().eq_ignore_ascii_case("transparent") {
                message(locale, "themeEditor.transparentLabel", &[])
            } else {
                value.into()
            };
            let title = message(locale, "themeEditor.pickColor", &[]);
            let font = egui::FontId::monospace(VALUE_FONT);
            let galley = ui
                .painter()
                .layout_no_wrap(label, font, appearance.foreground);
            let response = ui.allocate_response(
                egui::vec2(
                    galley.size().x + SWATCH_SIZE + TRIGGER_PADDING * TRIGGER_PADDING_COUNT,
                    TRIGGER_HEIGHT,
                ),
                Sense::click(),
            );
            #[cfg(any(test, feature = "inspection"))]
            {
                self.traces[0] = Some((response.id, response.rect));
            }
            ui.painter().rect_stroke(
                response.rect,
                TRIGGER_RADIUS,
                Stroke::new(BORDER_WIDTH, appearance.border),
                egui::StrokeKind::Inside,
            );
            let swatch = Rect::from_min_size(
                Pos2::new(
                    response.rect.left() + TRIGGER_PADDING,
                    response.rect.center().y - SWATCH_SIZE * HALF,
                ),
                egui::vec2(SWATCH_SIZE, SWATCH_SIZE),
            );
            let swatch_color = crate::presentation::parse_color(value, "theme picker swatch")
                .unwrap_or(appearance.input);
            ui.painter().rect_filled(swatch, 1, swatch_color);
            ui.painter().rect_stroke(
                swatch,
                1,
                Stroke::new(BORDER_WIDTH, appearance.border),
                egui::StrokeKind::Inside,
            );
            ui.painter().galley(
                Pos2::new(
                    swatch.right() + TRIGGER_PADDING,
                    response.rect.center().y - galley.size().y * HALF,
                ),
                galley,
                appearance.foreground,
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &title)
            });
            self.tooltip = Some(crate::tooltip_trigger::Trigger {
                response: response.clone(),
                label: title,
                align: egui::RectAlign::BOTTOM,
                focus_target: None,
            });
            let was_open = self.open;
            if response.clicked() {
                response.request_focus();
                self.open = !self.open;
                if self.open {
                    self.hex_input = self.stored.clone();
                }
            }
            let mut open = self.open;
            egui::Popup::from_response(&response)
                .open_bool(&mut open)
                .width(POPUP_WIDTH)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .frame(
                    egui::Frame::popup(ui.style())
                        .fill(appearance.background)
                        .stroke(Stroke::new(BORDER_WIDTH, appearance.border)),
                )
                .show(|ui| {
                    ui.visuals_mut().override_text_color = Some(appearance.foreground);
                    ui.spacing_mut().item_spacing.y = POPUP_GAP;
                    if ui.input(|input| {
                        !input.focused
                            || input
                                .events
                                .iter()
                                .any(|event| matches!(event, egui::Event::PointerGone))
                    }) {
                        self.drag = None;
                    }
                    let square = ui.allocate_response(
                        egui::vec2(SQUARE_SIZE, SQUARE_SIZE),
                        Sense::click_and_drag(),
                    );
                    #[cfg(any(test, feature = "inspection"))]
                    {
                        self.traces[1] = Some((square.id, square.rect));
                    }
                    self.slider(ui, &square, Slider::Square, locale, &mut changed);
                    let active = self.active();
                    let [r, g, b] = Hsv {
                        h: active.h,
                        s: 1.0,
                        v: 1.0,
                    }
                    .rgb();
                    ui.painter()
                        .rect_filled(square.rect, 0, Color32::from_rgb(r, g, b));
                    gradient(
                        ui,
                        square.rect,
                        [
                            Color32::WHITE,
                            Color32::TRANSPARENT,
                            Color32::TRANSPARENT,
                            Color32::WHITE,
                        ],
                    );
                    gradient(
                        ui,
                        square.rect,
                        [
                            Color32::TRANSPARENT,
                            Color32::TRANSPARENT,
                            Color32::BLACK,
                            Color32::BLACK,
                        ],
                    );
                    let thumb = Pos2::new(
                        square.rect.left() + active.s as f32 * square.rect.width(),
                        square.rect.top() + (1.0 - active.v) as f32 * square.rect.height(),
                    );
                    ui.painter().circle_stroke(
                        thumb,
                        THUMB_RADIUS,
                        Stroke::new(THUMB_STROKE, appearance.border),
                    );
                    if square.has_focus() {
                        ui.painter().rect_stroke(
                            square.rect,
                            0,
                            Stroke::new(BORDER_WIDTH, appearance.focus),
                            egui::StrokeKind::Outside,
                        );
                    }
                    let hue = ui.allocate_response(
                        egui::vec2(SQUARE_SIZE, HUE_HEIGHT),
                        Sense::click_and_drag(),
                    );
                    #[cfg(any(test, feature = "inspection"))]
                    {
                        self.traces[2] = Some((hue.id, hue.rect));
                    }
                    self.slider(ui, &hue, Slider::Hue, locale, &mut changed);
                    for (index, stops) in HUE_STOPS.windows(HEX_CHANNEL_LENGTH).enumerate() {
                        let left = hue.rect.left()
                            + index as f32 * hue.rect.width() / (HUE_STOPS.len() - 1) as f32;
                        let right = hue.rect.left()
                            + (index + 1) as f32 * hue.rect.width() / (HUE_STOPS.len() - 1) as f32;
                        gradient(
                            ui,
                            Rect::from_min_max(
                                Pos2::new(left, hue.rect.top()),
                                Pos2::new(right, hue.rect.bottom()),
                            ),
                            [stops[0], stops[1], stops[1], stops[0]],
                        );
                    }
                    let active = self.active();
                    ui.painter().circle_stroke(
                        Pos2::new(
                            hue.rect.left() + (active.h / HUE_DEGREES) as f32 * hue.rect.width(),
                            hue.rect.top() + HUE_HEIGHT * HALF,
                        ),
                        HUE_HEIGHT * HALF,
                        Stroke::new(THUMB_STROKE, appearance.border),
                    );
                    if hue.has_focus() {
                        ui.painter().rect_stroke(
                            hue.rect,
                            0,
                            Stroke::new(BORDER_WIDTH, appearance.focus),
                            egui::StrokeKind::Outside,
                        );
                    }
                    ui.label(
                        egui::RichText::new(message(
                            locale,
                            "themeEditor.colorValuePlaceholder",
                            &[],
                        ))
                        .color(appearance.muted),
                    );
                    let mut input = egui::TextEdit::singleline(&mut self.hex_input)
                        .id_salt((&self.stored, "hex"))
                        .desired_width(INPUT_WIDTH)
                        .return_key(None)
                        .hint_text("#rrggbb")
                        .show(ui);
                    #[cfg(any(test, feature = "inspection"))]
                    {
                        self.traces[3] = Some((input.response.id, input.response.rect));
                    }
                    if !was_open {
                        input.response.request_focus();
                        input
                            .state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(self.hex_input.chars().count()),
                            )));
                        input.state.store(ui.ctx(), input.response.id);
                    }
                    if input.response.lost_focus() {
                        let raw = self.hex_input.trim();
                        if raw.eq_ignore_ascii_case("transparent") {
                            self.hex_error = false;
                            changed = Some("transparent".into());
                        } else if Hsv::from_color(raw).is_some() {
                            self.hex_error = false;
                            changed = Some(raw.to_ascii_lowercase());
                        } else {
                            self.hex_error = true;
                        }
                    }
                    if self.hex_error
                        || crate::presentation::parse_color(value, "theme picker value").is_err()
                    {
                        ui.colored_label(
                            appearance.error,
                            message(locale, "themeEditor.invalidColor", &[]),
                        );
                    }
                });
            self.open = open;
            if !open {
                self.drag = None;
            }
        });
        changed
    }

    fn active(&self) -> Hsv {
        self.drag
            .map(|(_, hsv)| hsv)
            .or_else(|| Hsv::from_color(&self.stored))
            .unwrap_or(Hsv {
                h: 0.0,
                s: 1.0,
                v: 1.0,
            })
    }

    fn slider(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        slider: Slider,
        locale: &ResolvedLocale,
        changed: &mut Option<String>,
    ) {
        let active = self.active();
        let label = match slider {
            Slider::Square => "themeEditor.saturationValueSliderLabel",
            Slider::Hue => "themeEditor.hueSliderLabel",
        };
        let value = match slider {
            Slider::Square => (active.v * PERCENT_MAX).round(),
            Slider::Hue => active.h.round(),
        };
        response.widget_info(|| {
            egui::WidgetInfo::slider(ui.is_enabled(), value, message(locale, label, &[]))
        });
        if !ui.is_enabled() || !ui.input(|input| input.focused) {
            return;
        }
        if response.is_pointer_button_down_on() && ui.input(|input| input.pointer.primary_down()) {
            response.request_focus();
            if let Some(position) = ui.input(|input| input.pointer.interact_pos()) {
                self.sample(slider, response.rect, position);
            }
        }
        let released = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    ..
                } => Some(*pos),
                _ => None,
            })
        });
        if self.drag.is_some_and(|(owner, _)| owner == slider)
            && let Some(position) = released
        {
            self.sample(slider, response.rect, position);
            *changed = self.drag.take().map(|(_, hsv)| hsv.hex());
        }
        if !response.has_focus() {
            return;
        }
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        let mut hsv = self.active();
        let mut stepped = false;
        ui.input_mut(|input| {
            for key in [
                Key::Home,
                Key::End,
                Key::ArrowLeft,
                Key::ArrowRight,
                Key::ArrowUp,
                Key::ArrowDown,
            ] {
                if !input.consume_key(egui::Modifiers::NONE, key) {
                    continue;
                }
                stepped = true;
                match (slider, key) {
                    (Slider::Square, Key::Home) => hsv.v = 0.0,
                    (Slider::Square, Key::End) => hsv.v = 1.0,
                    (Slider::Square, Key::ArrowLeft) => hsv.s = (hsv.s - SV_KEY_STEP).max(0.0),
                    (Slider::Square, Key::ArrowRight) => hsv.s = (hsv.s + SV_KEY_STEP).min(1.0),
                    (Slider::Square, Key::ArrowUp) => hsv.v = (hsv.v + SV_KEY_STEP).min(1.0),
                    (Slider::Square, Key::ArrowDown) => hsv.v = (hsv.v - SV_KEY_STEP).max(0.0),
                    (Slider::Hue, Key::Home) => hsv.h = 0.0,
                    (Slider::Hue, Key::End) => hsv.h = HUE_DEGREES,
                    (Slider::Hue, Key::ArrowLeft | Key::ArrowDown) => {
                        hsv.h = (hsv.h - HUE_KEY_STEP).max(0.0)
                    }
                    (Slider::Hue, Key::ArrowRight | Key::ArrowUp) => {
                        hsv.h = (hsv.h + HUE_KEY_STEP).min(HUE_DEGREES)
                    }
                    _ => {}
                }
            }
        });
        if stepped {
            *changed = Some(hsv.hex());
        }
    }

    fn sample(&mut self, slider: Slider, rect: Rect, position: Pos2) {
        if self.drag.is_some_and(|(owner, _)| owner != slider) {
            return;
        }
        let mut hsv = self.active();
        let x = f64::from(((position.x - rect.left()) / rect.width()).clamp(0.0, 1.0));
        match slider {
            Slider::Square => {
                hsv.s = x;
                hsv.v =
                    f64::from((1.0 - (position.y - rect.top()) / rect.height()).clamp(0.0, 1.0));
            }
            Slider::Hue => hsv.h = x * HUE_DEGREES,
        }
        self.drag = Some((slider, hsv));
    }
}

fn gradient(ui: &Ui, rect: Rect, colors: [Color32; 4]) {
    let mut mesh = egui::epaint::Mesh::default();
    for (pos, color) in [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ]
    .into_iter()
    .zip(colors)
    {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
}

#[cfg(all(test, feature = "native-host"))]
#[path = "../../taide-native-app/src/theme-color-picker-tests.rs"]
mod tests;
