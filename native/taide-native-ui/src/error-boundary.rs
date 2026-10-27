use std::backtrace::Backtrace;
use std::cell::Cell;
use std::panic::{AssertUnwindSafe, PanicHookInfo};
use std::sync::{Arc, Once};

use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Color32, FontId, Galley, Layout, Rect, RichText, Sense, TextFormat, Ui, UiBuilder, vec2,
};

const STATE_KEY: &str = "native-error-boundary";
const FALLBACK_KEY: &str = "native-error-boundary-fallback";
const ALERT_KEY: &str = "native-error-boundary-alert";
const STACK_KEY: &str = "native-error-boundary-stack";
const DISCARD_REASON: &str = "native error boundary replaced a panicked region";
const DESCRIPTION_KEY: &str = "errorBoundary.description";
const RETRY_KEY: &str = "common.retry";
const UNKNOWN_PANIC: &str = "Box<dyn Any>";
const GAP: f32 = 8.0;
const LABEL_FONT_SIZE: f32 = 14.0;
const LABEL_LINE_HEIGHT: f32 = 20.0;
const DESCRIPTION_FONT_SIZE: f32 = 12.0;
const DESCRIPTION_LINE_HEIGHT: f32 = 16.0;
const STACK_FONT_SIZE: f32 = 10.0;
const STACK_LINE_HEIGHT: f32 = STACK_FONT_SIZE * LABEL_LINE_HEIGHT / LABEL_FONT_SIZE;
const STACK_MAX_HEIGHT: f32 = 128.0;
const RETRY_FONT_SIZE: f32 = 12.0;
const RETRY_HEIGHT: f32 = 24.0;
const RETRY_PADDING_X: f32 = 8.0;
const RETRY_RADIUS: u8 = 6;
#[cfg(any(test, feature = "inspection"))]
const INJECTION_KEY: &str = "native-error-boundary-injection";

static HOOK: Once = Once::new();

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
    static CAUGHT: Cell<Option<String>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Region {
    Welcome,
    ActivityBar,
    StatusBar,
    SidebarPanel,
}

impl Region {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Welcome => "errorBoundary.welcome",
            Self::ActivityBar => "errorBoundary.sidebar",
            Self::StatusBar => "errorBoundary.statusBar",
            Self::SidebarPanel => "errorBoundary.sidebarPanel",
        }
    }

    fn is_row(self) -> bool {
        self == Self::StatusBar
    }

    fn is_unmounted_while_hidden(self) -> bool {
        self != Self::SidebarPanel
    }
}

#[derive(Clone, Copy)]
pub struct FallbackColors {
    pub background: Color32,
    pub foreground: Color32,
    pub muted: Color32,
}

pub enum Outcome<T> {
    Shown(T),
    Caught,
    Fallback,
}

#[derive(Clone, Default)]
struct State {
    generation: u64,
    frame: Option<u64>,
    failure: Option<Failure>,
}

#[derive(Clone)]
struct Failure {
    stack: Option<String>,
    is_focus_pending: bool,
}

pub fn show<T>(
    ui: &mut Ui,
    rect: Rect,
    region: Region,
    colors: FallbackColors,
    text: &dyn Fn(&str) -> String,
    contents: impl FnOnce(&mut Ui) -> T,
) -> Outcome<T> {
    show_instance(ui, rect, (region, egui::Id::NULL), colors, text, contents)
}

/// Like [`show`] for a region that appears once per `instance` in a window (one sidebar panel
/// per shell slot), each with its own failure state.
pub fn show_instance<T>(
    ui: &mut Ui,
    rect: Rect,
    (region, instance): (Region, egui::Id),
    colors: FallbackColors,
    text: &dyn Fn(&str) -> String,
    contents: impl FnOnce(&mut Ui) -> T,
) -> Outcome<T> {
    let id = egui::Id::new((STATE_KEY, region, instance, ui.ctx().viewport_id()));
    let frame = ui.ctx().cumulative_frame_nr();
    let mut state = ui
        .data(|data| data.get_temp::<State>(id))
        .unwrap_or_default();
    if region.is_unmounted_while_hidden()
        && state
            .frame
            .is_some_and(|drawn| drawn.saturating_add(1) < frame)
    {
        state = State {
            generation: state.generation + 1,
            ..State::default()
        };
    }
    state.frame = Some(frame);
    let outcome = if state.failure.is_some() {
        Outcome::Fallback
    } else {
        let caught = ui
            .scope_builder(
                UiBuilder::new()
                    .id_salt((region, instance, state.generation))
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    capture(|| {
                        let value = contents(ui);
                        #[cfg(any(test, feature = "inspection"))]
                        injected_panic_point(ui.ctx(), region, instance);
                        value
                    })
                },
            )
            .inner;
        match caught {
            Ok(value) => Outcome::Shown(value),
            Err(failure) => {
                state.failure = Some(failure);
                ui.ctx().request_discard(DISCARD_REASON);
                Outcome::Caught
            }
        }
    };
    if let Some(failure) = state.failure.as_mut()
        && fallback(ui, rect, region, failure, colors, text)
    {
        state.failure = None;
        state.generation += 1;
        ui.ctx().request_repaint();
    }
    ui.data_mut(|data| data.insert_temp(id, state));
    outcome
}

fn capture<T>(run: impl FnOnce() -> T) -> Result<T, Failure> {
    if cfg!(debug_assertions) {
        HOOK.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                if DEPTH.with(Cell::get) > 0 {
                    CAUGHT.with(|caught| caught.set(Some(stack(info))));
                }
                previous(info);
            }));
        });
    }
    DEPTH.with(|depth| depth.set(depth.get() + 1));
    let result = std::panic::catch_unwind(AssertUnwindSafe(run));
    DEPTH.with(|depth| depth.set(depth.get() - 1));
    let stack = CAUGHT.with(Cell::take);
    result.map_err(|_| Failure {
        stack,
        is_focus_pending: true,
    })
}

fn stack(info: &PanicHookInfo<'_>) -> String {
    let location = info.location().map(ToString::to_string).unwrap_or_default();
    format!(
        "panicked at {location}:\n{}\n{}",
        info.payload_as_str().unwrap_or(UNKNOWN_PANIC),
        Backtrace::force_capture()
    )
}

fn layout(
    ui: &Ui,
    text: String,
    format: TextFormat,
    halign: Align,
    wrap_width: f32,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text, format);
    job.halign = halign;
    job.wrap = TextWrapping::wrap_at_width(wrap_width);
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

fn format(font_id: FontId, line_height: f32, color: Color32) -> TextFormat {
    TextFormat {
        font_id,
        line_height: Some(line_height),
        color,
        ..Default::default()
    }
}

fn fallback(
    ui: &mut Ui,
    rect: Rect,
    region: Region,
    failure: &mut Failure,
    colors: FallbackColors,
    text: &dyn Fn(&str) -> String,
) -> bool {
    ui.scope_builder(
        UiBuilder::new()
            .id_salt((FALLBACK_KEY, region))
            .max_rect(rect),
        |ui| {
            ui.set_clip_rect(rect.intersect(ui.clip_rect()));
            ui.painter().rect_filled(rect, 0.0, colors.background);
            let name = text(region.label_key());
            let explanation = text(DESCRIPTION_KEY);
            let alert = ui.interact(
                rect,
                ui.id().with(ALERT_KEY),
                Sense::focusable_noninteractive(),
            );
            ui.ctx().accesskit_node_builder(alert.id, |node| {
                node.set_role(egui::accesskit::Role::Alert);
                node.set_label(name.clone());
                node.set_description(explanation.clone());
            });
            if failure.is_focus_pending {
                alert.request_focus();
                failure.is_focus_pending = ui.ctx().will_discard();
            }
            let wrap_width = if region.is_row() {
                f32::INFINITY
            } else {
                rect.width()
            };
            let label = layout(
                ui,
                name,
                format(
                    FontId::proportional(LABEL_FONT_SIZE),
                    LABEL_LINE_HEIGHT,
                    colors.foreground,
                ),
                Align::Center,
                wrap_width,
            );
            let description = layout(
                ui,
                explanation,
                format(
                    FontId::proportional(DESCRIPTION_FONT_SIZE),
                    DESCRIPTION_LINE_HEIGHT,
                    colors.muted,
                ),
                Align::Center,
                wrap_width,
            );
            let stack = failure
                .stack
                .clone()
                .filter(|_| !region.is_row())
                .map(|stack| {
                    layout(
                        ui,
                        stack,
                        format(
                            FontId::monospace(STACK_FONT_SIZE),
                            STACK_LINE_HEIGHT,
                            ui.visuals().error_fg_color,
                        ),
                        Align::Min,
                        f32::INFINITY,
                    )
                });
            let retry_font = FontId::new(RETRY_FONT_SIZE, crate::font_families::medium(ui));
            let retry = text(RETRY_KEY);
            let retry_width = ui
                .painter()
                .layout_no_wrap(retry.clone(), retry_font.clone(), colors.foreground)
                .size()
                .x
                + RETRY_PADDING_X * 2.0;
            let content = if region.is_row() {
                vec2(
                    label.size().x + description.size().x + retry_width + GAP * 2.0,
                    RETRY_HEIGHT.max(label.size().y),
                )
            } else {
                let stack_height = stack
                    .as_ref()
                    .map_or(0.0, |stack| stack.size().y.min(STACK_MAX_HEIGHT) + GAP);
                vec2(
                    rect.width(),
                    label.size().y + description.size().y + stack_height + RETRY_HEIGHT + GAP * 2.0,
                )
            };
            let arrangement = if region.is_row() {
                Layout::left_to_right(Align::Center)
            } else {
                Layout::top_down(Align::Center)
            };
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(Rect::from_center_size(rect.center(), content))
                    .layout(arrangement),
                |ui| {
                    ui.spacing_mut().item_spacing = vec2(GAP, GAP);
                    ui.spacing_mut().button_padding = vec2(RETRY_PADDING_X, 0.0);
                    ui.add(egui::Label::new(label).selectable(false));
                    ui.add(egui::Label::new(description).selectable(false));
                    if let Some(stack) = stack {
                        egui::ScrollArea::both()
                            .id_salt(STACK_KEY)
                            .max_height(STACK_MAX_HEIGHT)
                            .max_width(rect.width())
                            .show(ui, |ui| ui.add(egui::Label::new(stack).selectable(false)));
                    }
                    ui.add(
                        egui::Button::new(RichText::new(retry).font(retry_font))
                            .min_size(vec2(0.0, RETRY_HEIGHT))
                            .corner_radius(RETRY_RADIUS),
                    )
                    .clicked()
                },
            )
            .inner
        },
    )
    .inner
}

#[cfg(any(test, feature = "inspection"))]
pub fn inject_panic(context: &egui::Context, region: Region) {
    inject_instance_panic(context, region, egui::Id::NULL);
}

#[cfg(any(test, feature = "inspection"))]
pub fn inject_instance_panic(context: &egui::Context, region: Region, instance: egui::Id) {
    context
        .data_mut(|data| data.insert_temp(egui::Id::new((INJECTION_KEY, region, instance)), true));
}

#[cfg(any(test, feature = "inspection"))]
fn injected_panic_point(context: &egui::Context, region: Region, instance: egui::Id) {
    if context
        .data_mut(|data| data.remove_temp::<bool>(egui::Id::new((INJECTION_KEY, region, instance))))
        .unwrap_or(false)
    {
        panic!("synthetic native error boundary panic in {region:?}");
    }
}
