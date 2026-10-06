use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Event, FontId, ImeEvent, Rect, Sense, Stroke, Ui, pos2, vec2};
use taide_model::{
    error::{AppError, AppResult},
    ids::{PaneId, TabId},
    locale::ResolvedLocale,
    theme::ResolvedTheme,
};
use taide_native_terminal::{
    CellFlags, Color, Colors, Column, CursorShape, CursorStyle, GridDimensions, Line, Mode,
    NamedColor, Point, Rgb, Selection, SelectionGranularity, SelectionRange, SelectionStamp,
    SelectionType, Side, Size, TerminalCore, WindowSize,
    input::{InputAction, Key, Modifiers, MouseAction, MouseButton, MouseInput, NativeInput},
    session::Phase,
};
use taide_runtime::AppServices;

use crate::{
    command_registry::CommandContext,
    host::HostCommand,
    keymap::{Context as KeymapContext, Decision as KeymapDecision},
    terminal_dispatch::{EffectPorts, ObservePorts},
    terminal_host::{Hub, InputPreparation, InputResult, PendingInput, PreparedInput, Session},
    terminal_writer::Receipt,
};

const ANSI_KEYS: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "brightBlack",
    "brightRed",
    "brightGreen",
    "brightYellow",
    "brightBlue",
    "brightMagenta",
    "brightCyan",
    "brightWhite",
];
const ANSI_COUNT: usize = 16;
const CUBE_END: usize = 232;
const INDEXED_END: usize = 256;
const CUBE_SIDE: usize = 6;
const CUBE_BASE: usize = 55;
const CUBE_STEP: usize = 40;
const GRAY_BASE: usize = 8;
const GRAY_STEP: usize = 10;
const DIM_FACTOR: f32 = 0.66;
const LINE_HEIGHT: f32 = 1.0;
const MIN_FONT_SIZE: f32 = 6.0;
const MIN_COLUMNS: u16 = 3;
const MIN_ROWS: u16 = 2;
const INPUT_BYTES: usize = 64 * 1024;
const INPUT_RECEIPTS: usize = 64;
const QUEUED_INPUT_BYTES: usize = INPUT_RECEIPTS * (INPUT_BYTES + size_of::<PendingInput>());
const FOCUS_REPORT_BYTES: usize = INPUT_BYTES;
const FOCUS_QUEUE_BYTES: usize = FOCUS_REPORT_BYTES + INPUT_RECEIPTS * size_of::<PendingInput>();
const RECEIPT_POLL: Duration = Duration::from_millis(16);
const CURSOR_WIDTH: f32 = 1.0;
const PENDING_INPUT_UNITS: usize = 4096;
const STATUS_HEIGHT: f32 = 64.0;
const CURSOR_BLINK_INTERVAL: Duration = Duration::from_millis(600);
const SELECTION_HALF_CELL: f32 = 0.5;
const DOUBLE_CLICK_COUNT: u8 = 2;
const TRIPLE_CLICK_COUNT: u8 = 3;
const DRAG_SCROLL_THRESHOLD: f32 = 50.0;
const DRAG_SCROLL_MAX_SPEED: i32 = 15;
const DRAG_SCROLL_INTERVAL: Duration = Duration::from_millis(50);
const ROUND_HALF: f64 = 0.5;
const WHEEL_SCROLL_PIXELS: f64 = 50.0;
const WHEEL_POINT_NORMALIZER: f64 = 40.0;
const WHEEL_FAST_MULTIPLIER: f64 = 5.0;
const WHEEL_TRACKPAD_THRESHOLD: f32 = 50.0;
const WHEEL_TRACKPAD_MULTIPLIER: f64 = 0.3;
const MOUSE_BUTTON_COUNT: usize = 3;
const SPLIT_PANE_COUNT: f32 = 2.0;
const COMMAND_GUTTER_WIDTH: f32 = 2.0;
const COMMAND_HEX_LENGTH: usize = 7;
const MENU_SEARCH_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct Appearance {
    pub font: FontId,
    ansi: [Rgb; ANSI_COUNT],
    foreground: Rgb,
    background: Rgb,
    cursor: Rgb,
    selection: Color32,
    command_colors: taide_native_terminal::CommandColors,
}

impl Appearance {
    pub fn set_font_size(&mut self, size: u32) -> bool {
        let size = (size as f32).max(MIN_FONT_SIZE);
        if self.font.size == size {
            return false;
        }
        self.font.size = size;
        true
    }

    pub fn new(theme: &ResolvedTheme, font_size: u32) -> AppResult<Self> {
        let read = |key: &str| {
            let value = theme.terminal.get(key).ok_or_else(|| {
                AppError::Internal(format!("native terminal theme color is unavailable: {key}"))
            })?;
            crate::presentation::parse_color(value, key)
        };
        let rgb = |color: Color32| Rgb {
            r: color.r(),
            g: color.g(),
            b: color.b(),
        };
        let mut ansi = [Rgb { r: 0, g: 0, b: 0 }; ANSI_COUNT];
        for (entry, key) in ansi.iter_mut().zip(ANSI_KEYS) {
            *entry = rgb(read(key)?);
        }
        Ok(Self {
            font: FontId::monospace((font_size as f32).max(MIN_FONT_SIZE)),
            ansi,
            foreground: rgb(read("foreground")?),
            background: rgb(read("background")?),
            cursor: rgb(read("cursor")?),
            selection: read("selection")?,
            command_colors: taide_native_terminal::CommandColors {
                success: command_color(theme, "statusIndicator.success"),
                failure: command_color(theme, "statusIndicator.error"),
            },
        })
    }

    pub fn indexed(&self, index: usize) -> AppResult<Rgb> {
        if let Some(color) = self.ansi.get(index) {
            return Ok(*color);
        }
        if index < CUBE_END {
            let cube = index - ANSI_COUNT;
            let component = |index| {
                if index == 0 {
                    0
                } else {
                    (CUBE_BASE + index * CUBE_STEP) as u8
                }
            };
            return Ok(Rgb {
                r: component(cube / (CUBE_SIDE * CUBE_SIDE)),
                g: component(cube / CUBE_SIDE % CUBE_SIDE),
                b: component(cube % CUBE_SIDE),
            });
        }
        if index < INDEXED_END {
            let value = (GRAY_BASE + (index - CUBE_END) * GRAY_STEP) as u8;
            return Ok(Rgb {
                r: value,
                g: value,
                b: value,
            });
        }
        match index {
            value if value == NamedColor::Foreground as usize => Ok(self.foreground),
            value if value == NamedColor::Background as usize => Ok(self.background),
            value if value == NamedColor::Cursor as usize => Ok(self.cursor),
            value if value == NamedColor::BrightForeground as usize => Ok(self.foreground),
            value if value == NamedColor::DimForeground as usize => Ok(dim(self.foreground)),
            value
                if (NamedColor::DimBlack as usize..=NamedColor::DimWhite as usize)
                    .contains(&value) =>
            {
                Ok(dim(self.ansi[value - NamedColor::DimBlack as usize]))
            }
            _ => Err(AppError::InvalidArgument(
                "invalid native terminal color index".into(),
            )),
        }
    }

    fn resolve(&self, colors: &Colors, color: Color, flags: CellFlags) -> AppResult<Color32> {
        let resolved = match color {
            Color::Spec(rgb) => rgb,
            Color::Indexed(index) => {
                colors[usize::from(index)].unwrap_or(self.indexed(usize::from(index))?)
            }
            Color::Named(named) => {
                let index = named as usize;
                let named = if flags.contains(CellFlags::BOLD) && index < ANSI_COUNT / 2 {
                    named.to_bright()
                } else {
                    named
                };
                colors[named].unwrap_or(self.indexed(named as usize)?)
            }
        };
        let resolved = if flags.contains(CellFlags::DIM) {
            dim(resolved)
        } else {
            resolved
        };
        Ok(color32(resolved))
    }
}

fn dim(color: Rgb) -> Rgb {
    Rgb {
        r: (f32::from(color.r) * DIM_FACTOR) as u8,
        g: (f32::from(color.g) * DIM_FACTOR) as u8,
        b: (f32::from(color.b) * DIM_FACTOR) as u8,
    }
}

fn color32(color: Rgb) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

fn command_color(theme: &ResolvedTheme, key: &str) -> Option<Rgb> {
    let raw = theme.colors.get(key)?;
    let normalized = raw.get(..COMMAND_HEX_LENGTH)?;
    let color = crate::presentation::parse_color(normalized, key).ok()?;
    Some(Rgb {
        r: color.r(),
        g: color.g(),
        b: color.b(),
    })
}

#[derive(Default)]
struct View {
    session: String,
    paste_lifetime: Arc<()>,
    menu: Option<MenuSnapshot>,
    link_press: Option<crate::terminal_file_links::Link>,
    file_links: crate::terminal_file_links::Cache,
    offset: usize,
    selection: Option<Selection>,
    selection_stamp: Option<SelectionStamp>,
    is_all_selected: bool,
    gesture: Gesture,
    clicks: Clicks,
    drag: Drag,
    wheel: Wheel,
    mouse: Mouse,
    mouse_geometry: Option<MouseGeometry>,
    processed_frame: Option<u64>,
    processed_pass: Option<u64>,
    wheel_target: Option<WheelTarget>,
    captured_wheel: CapturedWheel,
    is_wheel_routed: bool,
    preedit: String,
    focused: bool,
    focus_id: Option<egui::Id>,
    focus_frame: Option<u64>,
    ordered_focus_frame: Option<u64>,
    input_order: Option<WireOrder>,
    outbox: Arc<Mutex<Outbox>>,
    pending: VecDeque<u16>,
    error: Option<String>,
    logged_error: Option<String>,
    blink: Blink,
}

impl View {
    fn input_capacity(&self) -> AppResult<usize> {
        self.outbox
            .lock()
            .map(|outbox| outbox.capacity())
            .map_err(|_| AppError::Internal("native terminal input queue lock poisoned".into()))
    }

    fn register_mouse(&mut self, ui: &Ui, response: &egui::Response, mode: Mode, running: bool) {
        let mode = mouse_mode(mode);
        if self.mouse.mode != mode {
            self.mouse = Mouse {
                mode,
                frame: ui.ctx().cumulative_frame_nr(),
                ..Default::default()
            };
            self.drag = Drag::default();
        }
        let mut target = WheelTarget::new(ui, response, running);
        target.mode = mode;
        if !target.enabled {
            self.mouse.packets.clear();
            self.mouse.buttons.fill(false);
            self.mouse.forced = false;
            self.captured_wheel.packets.clear();
        }
        self.wheel_target = Some(target);
    }

    fn clear_selection(&mut self) {
        self.selection = None;
        self.is_all_selected = false;
        self.gesture = Gesture::default();
        self.drag = Drag::default();
    }
}

#[derive(Default)]
struct Outbox {
    receipts: VecDeque<Receipt>,
    pending: VecDeque<PendingInput>,
    focus: VecDeque<PendingInput>,
    focus_bytes: usize,
    bytes: usize,
    error: Option<String>,
    staged: Vec<StagedInput>,
    staged_bytes: usize,
    staged_focus_bytes: usize,
}

impl Outbox {
    fn capacity(&self) -> usize {
        if !self.focus.is_empty() {
            return 0;
        }
        INPUT_RECEIPTS.saturating_sub(
            self.receipts.len()
                + self.pending.len()
                + self
                    .staged
                    .iter()
                    .filter(|input| !input.prepared.is_focus())
                    .count(),
        )
    }

    fn is_pending(&self) -> bool {
        !self.receipts.is_empty()
            || !self.pending.is_empty()
            || !self.focus.is_empty()
            || !self.staged.is_empty()
    }

    fn clear(&mut self) {
        self.receipts.clear();
        self.pending.clear();
        self.focus.clear();
        self.focus_bytes = 0;
        self.bytes = 0;
        self.staged.clear();
        self.staged_bytes = 0;
        self.staged_focus_bytes = 0;
    }

    fn submit(
        &mut self,
        session: &Session,
        services: &AppServices,
        input: NativeInput<'_>,
    ) -> AppResult<Option<InputAction>> {
        let is_focus = matches!(&input, NativeInput::Focus(_));
        let available = self.capacity();
        let focus_limit = FOCUS_REPORT_BYTES.min(session.input_payload_limit());
        let capacity = if is_focus {
            focus_limit
        } else {
            QUEUED_INPUT_BYTES
                .saturating_sub(self.bytes)
                .saturating_sub(size_of::<PendingInput>())
                .min(INPUT_BYTES)
        };
        let result = session.queue_input(
            services,
            input,
            capacity,
            available == 0 || !self.pending.is_empty(),
        )?;
        self.accept_result(result, is_focus, available, focus_limit)
    }

    fn accept_result(
        &mut self,
        result: InputResult,
        is_focus: bool,
        available: usize,
        focus_limit: usize,
    ) -> AppResult<Option<InputAction>> {
        match result {
            InputResult::Write(receipt) => self.receipts.push_back(receipt),
            InputResult::Local(action) => return Ok(Some(action)),
            InputResult::Pending(pending) => {
                if is_focus && available == 0 {
                    self.queue_focus(pending, focus_limit)?;
                    return Ok(None);
                }
                if available == 0 {
                    return Err(AppError::InvalidArgument(
                        "native terminal input receipt budget exceeded".into(),
                    ));
                }
                let weight = pending.retained_bytes().ok_or_else(|| {
                    AppError::InvalidArgument("native terminal queued input overflow".into())
                })?;
                if weight > QUEUED_INPUT_BYTES.saturating_sub(self.bytes) {
                    return Err(AppError::InvalidArgument(
                        "native terminal queued input budget exceeded".into(),
                    ));
                }
                self.bytes += weight;
                self.pending.push_back(pending);
            }
        }
        Ok(None)
    }

    fn stage(
        &mut self,
        session: &Session,
        input: NativeInput<'_>,
        order: WireOrder,
    ) -> AppResult<Option<InputAction>> {
        let is_focus = matches!(&input, NativeInput::Focus(_));
        let budget = if is_focus {
            FOCUS_QUEUE_BYTES.saturating_sub(self.focus_bytes + self.staged_focus_bytes)
        } else {
            QUEUED_INPUT_BYTES.saturating_sub(self.bytes + self.staged_bytes)
        };
        let capacity = budget
            .saturating_sub(size_of::<StagedInput>())
            .min(INPUT_BYTES)
            .min(session.input_payload_limit());
        let prepared = match session.prepare_input(input, capacity)? {
            InputPreparation::Local(action) => return Ok(Some(action)),
            InputPreparation::Prepared(prepared) => prepared,
        };
        let weight = prepared
            .payload_capacity()
            .checked_add(size_of::<StagedInput>())
            .ok_or_else(focus_budget)?;
        let exhausted = if is_focus {
            self.staged
                .iter()
                .filter(|packet| packet.prepared.is_focus())
                .count()
                >= INPUT_RECEIPTS
        } else {
            self.capacity() == 0
        };
        if exhausted || weight > budget {
            return Err(AppError::InvalidArgument(
                "native terminal staged input budget exceeded".into(),
            ));
        }
        let prepared = session.admit_prepared_input(prepared)?;
        if is_focus {
            self.staged_focus_bytes += weight;
        } else {
            self.staged_bytes += weight;
        }
        self.staged.push(StagedInput { order, prepared });
        Ok(None)
    }

    fn flush_staged(
        &mut self,
        session: &Session,
        services: &AppServices,
        viewport: egui::ViewportId,
        frame: u64,
    ) -> AppResult<()> {
        let mut ready = Vec::new();
        for packet in std::mem::take(&mut self.staged) {
            if packet.order.viewport != viewport || packet.order.event.frame > frame {
                self.staged.push(packet);
                continue;
            }
            let weight = packet.prepared.payload_capacity() + size_of::<StagedInput>();
            if packet.prepared.is_focus() {
                self.staged_focus_bytes -= weight;
            } else {
                self.staged_bytes -= weight;
            }
            ready.push(packet);
        }
        ready.sort_by_key(|packet| (packet.order.event, packet.order.phase));
        for packet in ready {
            let is_focus = packet.prepared.is_focus();
            let available = self.capacity();
            let focus_limit = FOCUS_REPORT_BYTES.min(session.input_payload_limit());
            let result = session.submit_prepared_input(
                services,
                packet.prepared,
                focus_limit,
                available == 0 || !self.pending.is_empty(),
            )?;
            self.accept_result(result, is_focus, available, focus_limit)?;
        }
        Ok(())
    }

    fn queue_focus(&mut self, pending: PendingInput, limit: usize) -> AppResult<()> {
        let weight = pending.retained_bytes().ok_or_else(focus_budget)?;
        let remaining = FOCUS_QUEUE_BYTES.saturating_sub(self.focus_bytes);
        if let Some(focus) = self.focus.back_mut()
            && focus.can_append_control(&pending)?
        {
            let previous = focus.retained_bytes().ok_or_else(focus_budget)?;
            let capacity = remaining
                .saturating_add(previous)
                .saturating_sub(size_of::<PendingInput>())
                .min(limit);
            focus.append_control(pending, capacity)?;
            self.focus_bytes =
                self.focus_bytes - previous + focus.retained_bytes().ok_or_else(focus_budget)?;
            return Ok(());
        }
        if self.focus.len() >= INPUT_RECEIPTS || weight > remaining {
            return Err(focus_budget());
        }
        self.focus_bytes += weight;
        self.focus.push_back(pending);
        Ok(())
    }

    fn poll(&mut self, session: &Session, services: &AppServices) {
        while let Some(receipt) = self.receipts.front_mut() {
            let Some(result) = receipt.try_wait() else {
                break;
            };
            self.receipts.pop_front();
            self.error = result.err().map(|error| error.to_string());
        }
        while self.receipts.len() < INPUT_RECEIPTS {
            let (pending, is_focus) = match self.pending.pop_front() {
                Some(pending) => (pending, false),
                None => match self.focus.pop_front() {
                    Some(focus) => (focus, true),
                    None => break,
                },
            };
            let Some(weight) = pending.retained_bytes() else {
                self.error = Some("native terminal queued input overflow".into());
                self.clear();
                break;
            };
            if is_focus {
                self.focus_bytes -= weight;
            } else {
                self.bytes -= weight;
            }
            match session.retry_input(services, pending) {
                Ok(InputResult::Write(receipt)) => self.receipts.push_back(receipt),
                Ok(InputResult::Pending(pending)) => {
                    if is_focus {
                        self.focus_bytes += weight;
                        self.focus.push_front(pending);
                    } else {
                        self.bytes += weight;
                        self.pending.push_front(pending);
                    }
                    break;
                }
                Ok(InputResult::Local(_)) => {
                    self.error = Some("native terminal retry returned a local action".into());
                    self.clear();
                    break;
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    self.clear();
                    break;
                }
            }
        }
    }
}

fn focus_budget() -> AppError {
    AppError::InvalidArgument("native terminal focus queue budget exceeded".into())
}

fn reconcile_view(core: &TerminalCore, view: &mut View) -> AppResult<()> {
    let current = core.selection_stamp()?;
    let Some(previous) = view.selection_stamp else {
        view.selection_stamp = Some(current);
        view.offset = view.offset.min(current.history);
        return Ok(());
    };
    if current == previous {
        view.offset = view.offset.min(current.history);
        if view.is_all_selected {
            view.selection = Some(full_selection(core)?);
        }
        return Ok(());
    }
    if current.buffer_epoch != previous.buffer_epoch {
        view.clear_selection();
        view.wheel = Wheel::default();
        view.mouse = Mouse::default();
        view.captured_wheel.packets.clear();
        view.offset = 0;
        view.selection_stamp = Some(current);
        return Ok(());
    }
    let delta = current.origin.checked_sub(previous.origin).ok_or_else(|| {
        AppError::InvalidArgument("native terminal view movement overflow".into())
    })?;
    if view.offset != 0 {
        view.offset = if delta >= 0 {
            view.offset
                .saturating_add(usize::try_from(delta).unwrap_or(usize::MAX))
        } else {
            view.offset
                .saturating_sub(usize::try_from(delta.unsigned_abs()).unwrap_or(usize::MAX))
        };
    }
    view.offset = view.offset.min(current.history);
    if current.input_epoch != previous.input_epoch {
        view.clear_selection();
        view.wheel.position = None;
        view.offset = 0;
    }
    if current.rows_epoch != previous.rows_epoch {
        view.clear_selection();
        view.wheel = Wheel::default();
        view.mouse = Mouse::default();
        view.captured_wheel.packets.clear();
    } else if view.is_all_selected {
        view.selection = Some(full_selection(core)?);
    } else if let Some(seed) = view.gesture.seed {
        view.gesture.seed = seed.rebase(delta, core.grid()?.topmost_line())?;
        view.selection = view
            .gesture
            .seed
            .map(|seed| seed.selection(current.columns))
            .transpose()?;
        if view.gesture.seed.is_none() {
            view.clear_selection();
        }
    } else if let Some(selection) = &view.selection {
        view.selection = core.rebase_selection(selection, previous)?;
        if view.selection.is_none() {
            view.gesture = Gesture::default();
            view.drag = Drag::default();
        }
    }
    view.selection_stamp = Some(current);
    Ok(())
}

#[derive(Default)]
struct Gesture {
    granularity: Option<SelectionGranularity>,
    seed: Option<Seed>,
}

#[derive(Default)]
struct Drag {
    due: Option<f64>,
    last: f64,
    amount: i32,
    has_end: bool,
}

#[derive(Default)]
struct Wheel {
    position: Option<WheelPosition>,
    partial: f64,
}

struct WheelPosition {
    offset: usize,
    history: usize,
    height: f32,
    pixels: f64,
}

struct WheelTarget {
    frame: u64,
    rect: Rect,
    layer: egui::LayerId,
    enabled: bool,
    mode: Mode,
}

impl WheelTarget {
    fn new(ui: &Ui, response: &egui::Response, accepts_wheel: bool) -> Self {
        let rect = response.rect.intersect(ui.clip_rect());
        let rect = ui
            .ctx()
            .layer_transform_to_global(response.layer_id)
            .map_or(rect, |transform| transform * rect);
        Self {
            frame: ui.ctx().cumulative_frame_nr(),
            rect,
            layer: response.layer_id,
            enabled: ui.is_enabled() && response.enabled() && accepts_wheel,
            mode: Mode::NONE,
        }
    }
}

#[derive(Default)]
struct CapturedWheel {
    packets: VecDeque<WheelPacket>,
}

struct WheelPacket {
    position: egui::Pos2,
    event: Event,
    order: EventOrder,
    sequence: u64,
    geometry: Option<MouseGeometry>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct EventOrder {
    frame: u64,
    index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WirePhase {
    InitialFocusLoss,
    InitialFocusGain,
    PointerFocusLoss,
    PointerFocusGain,
    CapturedInput,
    ContextMenuFocusLoss,
    ContextMenuFocusGain,
    FocusLoss,
    FocusGain,
    Input,
}

#[derive(Clone, Copy)]
struct WireOrder {
    viewport: egui::ViewportId,
    event: EventOrder,
    phase: WirePhase,
}

struct StagedInput {
    order: WireOrder,
    prepared: PreparedInput,
}

#[derive(Default)]
struct Mouse {
    mode: Mode,
    frame: u64,
    packets: VecDeque<MousePacket>,
    buttons: [bool; MOUSE_BUTTON_COUNT],
    forced: bool,
    last: Option<MouseInput>,
}

impl Mouse {
    fn button(&self) -> Option<MouseButton> {
        self.buttons
            .iter()
            .zip([MouseButton::Left, MouseButton::Middle, MouseButton::Right])
            .find_map(|(pressed, button)| pressed.then_some(button))
    }
}

struct MousePacket {
    position: egui::Pos2,
    action: Option<MouseAction>,
    event: Event,
    modifiers: egui::Modifiers,
    mode: Mode,
    order: EventOrder,
    sequence: u64,
    geometry: Option<MouseGeometry>,
}

#[derive(Clone, Copy)]
struct MouseGeometry {
    rect: Rect,
    global_rect: Rect,
    from_global: Option<egui::emath::TSTransform>,
    cell: egui::Vec2,
}

impl MouseGeometry {
    fn new(ui: &Ui, response: &egui::Response, cell: egui::Vec2) -> Self {
        Self {
            rect: response.rect,
            global_rect: WheelTarget::new(ui, response, true).rect,
            from_global: ui.ctx().layer_transform_from_global(response.layer_id),
            cell,
        }
    }
}

fn mouse_mode(mode: Mode) -> Mode {
    mode & (Mode::MOUSE_MODE | Mode::SGR_MOUSE | Mode::SGR_PIXEL_MOUSE)
}

fn tracks_wheel(mode: Mode) -> bool {
    mode.intersects(Mode::MOUSE_REPORT_CLICK | Mode::MOUSE_DRAG | Mode::MOUSE_MOTION)
}

fn force_selection(modifiers: egui::Modifiers) -> bool {
    !cfg!(target_os = "macos") && modifiers.shift
}

fn mouse_button(button: egui::PointerButton) -> Option<(usize, MouseButton)> {
    match button {
        egui::PointerButton::Primary => Some((0, MouseButton::Left)),
        egui::PointerButton::Middle => Some((1, MouseButton::Middle)),
        egui::PointerButton::Secondary => Some((MOUSE_BUTTON_COUNT - 1, MouseButton::Right)),
        _ => None,
    }
}

impl Wheel {
    fn consume(&mut self, core: &TerminalCore, height: f32, event: &Event) -> AppResult<f64> {
        let Event::MouseWheel {
            unit,
            delta,
            modifiers,
            ..
        } = event
        else {
            return Ok(0.0);
        };
        if !delta.is_finite() || !height.is_finite() || height <= 0.0 {
            return Err(AppError::InvalidArgument(
                "invalid native terminal wheel geometry".into(),
            ));
        }
        if modifiers.shift {
            return Ok(0.0);
        }
        let factor = if modifiers.alt || modifiers.ctrl {
            WHEEL_FAST_MULTIPLIER
        } else {
            1.0
        };
        let mut amount = f64::from(delta.y) * factor;
        match unit {
            egui::MouseWheelUnit::Point => {
                amount /= f64::from(height);
                if delta.y.abs() < WHEEL_TRACKPAD_THRESHOLD {
                    amount *= WHEEL_TRACKPAD_MULTIPLIER;
                }
                self.partial += amount;
                amount = self.partial.trunc();
                self.partial %= 1.0;
            }
            egui::MouseWheelUnit::Line => {}
            egui::MouseWheelUnit::Page => amount *= core.grid()?.screen_lines() as f64,
        }
        Ok(amount)
    }

    fn apply(
        &mut self,
        core: &TerminalCore,
        offset: &mut usize,
        height: f32,
        event: &Event,
    ) -> AppResult<Option<Key>> {
        let Event::MouseWheel {
            unit,
            delta,
            modifiers,
            ..
        } = event
        else {
            return Ok(None);
        };
        if !delta.is_finite() || !height.is_finite() || height <= 0.0 {
            return Err(AppError::InvalidArgument(
                "invalid native terminal wheel geometry".into(),
            ));
        }
        if !core.has_scrollback()? {
            let amount = self.consume(core, height, event)?;
            return Ok(match amount {
                value if value > 0.0 => Some(Key::ArrowUp),
                value if value < 0.0 => Some(Key::ArrowDown),
                _ => None,
            });
        }
        let history = core.grid()?.history_size();
        *offset = (*offset).min(history);
        let mut position = self
            .position
            .take()
            .filter(|position| {
                position.offset == *offset
                    && position.history == history
                    && position.height == height
            })
            .unwrap_or(WheelPosition {
                offset: *offset,
                history,
                height,
                pixels: (history - *offset) as f64 * f64::from(height),
            });
        let mut amount = f64::from(delta.y) * WHEEL_SCROLL_PIXELS;
        if *unit != egui::MouseWheelUnit::Line {
            amount /= WHEEL_POINT_NORMALIZER;
        }
        if modifiers.alt {
            amount *= WHEEL_FAST_MULTIPLIER;
        }
        let pixels = if amount > 0.0 {
            amount.ceil()
        } else {
            amount.floor()
        };
        position.pixels = (position.pixels - pixels).clamp(0.0, history as f64 * f64::from(height));
        let row = (position.pixels / f64::from(height) + ROUND_HALF).floor() as usize;
        *offset = history.saturating_sub(row);
        position.offset = *offset;
        self.position = Some(position);
        Ok(None)
    }
}

fn drag_amount(rect: Rect, pointer: egui::Pos2) -> i32 {
    let offset = if pointer.y < rect.top() {
        pointer.y - rect.top()
    } else if pointer.y > rect.bottom() {
        pointer.y - rect.bottom()
    } else {
        return 0;
    };
    let normalized = f64::from(offset.clamp(-DRAG_SCROLL_THRESHOLD, DRAG_SCROLL_THRESHOLD))
        / f64::from(DRAG_SCROLL_THRESHOLD);
    (normalized.signum() + (normalized * f64::from(DRAG_SCROLL_MAX_SPEED - 1) + ROUND_HALF).floor())
        as i32
}

fn drag_endpoint(core: &TerminalCore, view: &mut View, amount: i32, timed: bool) -> AppResult<()> {
    if view.is_all_selected {
        view.selection = Some(full_selection(core)?);
        return Ok(());
    }
    let grid = core.grid()?;
    let [_, (previous, side)] = view
        .selection
        .as_ref()
        .ok_or_else(|| {
            AppError::InvalidArgument("native terminal drag anchor is unavailable".into())
        })?
        .native_anchors();
    let (previous, side) = view
        .gesture
        .seed
        .and_then(|seed| seed.end)
        .map(|end| (end, Side::Left))
        .unwrap_or((previous, side));
    let column = if view
        .selection
        .as_ref()
        .is_some_and(|selection| selection.ty == SelectionType::Block)
    {
        previous
            .column
            .0
            .checked_add(usize::from(side == Side::Right))
            .ok_or_else(|| {
                AppError::InvalidArgument("native terminal drag column overflow".into())
            })?
    } else if amount > 0 {
        grid.columns()
    } else {
        0
    };
    let line = if timed && amount > 0 {
        Line((grid.screen_lines() as i32 - view.offset as i32).min(grid.bottommost_line().0))
    } else if timed && amount < 0 {
        Line(-(view.offset as i32))
    } else {
        previous.line
    };
    let end = Point::new(line, Column(column));
    if let Some(mut seed) = view.gesture.seed {
        seed.end = Some(end);
        view.selection = Some(seed.selection(grid.columns())?);
        view.gesture.seed = Some(seed);
    } else if let Some(selection) = &mut view.selection {
        selection.update(end, Side::Left);
    }
    Ok(())
}

fn drag_tick(ui: &Ui, core: &TerminalCore, view: &mut View, now: f64) -> AppResult<()> {
    let Some(due) = view.drag.due else {
        return Ok(());
    };
    if !now.is_finite() || !due.is_finite() || now < view.drag.last {
        view.drag = Drag::default();
        return Ok(());
    }
    view.drag.last = now;
    let interval = DRAG_SCROLL_INTERVAL.as_secs_f64();
    if now >= due {
        let next = due + (((now - due) / interval).floor() + 1.0) * interval;
        if !next.is_finite() || next <= now {
            view.drag = Drag::default();
            return Ok(());
        }
        view.drag.due = Some(next);
        if view.drag.has_end && view.drag.amount != 0 {
            let amount = view.drag.amount;
            view.offset = if amount > 0 {
                view.offset.saturating_sub(amount as usize)
            } else {
                view.offset.saturating_add(amount.unsigned_abs() as usize)
            }
            .min(core.grid()?.history_size());
            drag_endpoint(core, view, amount, true)?;
        }
    }
    if view.drag.has_end
        && view.drag.amount != 0
        && let Some(next) = view.drag.due
    {
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64((next - now).max(0.0)));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Seed {
    start: Point,
    length: usize,
    end: Option<Point>,
}

impl Seed {
    fn new(range: SelectionRange, columns: usize) -> AppResult<Self> {
        let length = usize::try_from(i64::from(range.end.line.0) - i64::from(range.start.line.0))
            .ok()
            .and_then(|rows| rows.checked_mul(columns))
            .and_then(|cells| cells.checked_add(range.end.column.0))
            .and_then(|cells| cells.checked_add(1))
            .and_then(|cells| cells.checked_sub(range.start.column.0))
            .ok_or_else(|| {
                AppError::InvalidArgument("native terminal selection length overflow".into())
            })?;
        Ok(Self {
            start: range.start,
            length,
            end: None,
        })
    }

    fn rebase(mut self, delta: i64, top: Line) -> AppResult<Option<Self>> {
        let moved = |point: Point| {
            i64::from(point.line.0)
                .checked_sub(delta)
                .and_then(|line| i32::try_from(line).ok())
                .map(|line| Point::new(Line(line), point.column))
                .ok_or_else(|| {
                    AppError::InvalidArgument("native terminal word movement overflow".into())
                })
        };
        self.start = moved(self.start)?;
        self.end = self.end.map(moved).transpose()?;
        if self.end.is_some_and(|end| end.line < top) {
            return Ok(None);
        }
        self.start.line = self.start.line.max(top);
        Ok(Some(self))
    }

    fn span_end(self, columns: usize, exclude_eol: bool) -> AppResult<Point> {
        let cells = self
            .start
            .column
            .0
            .checked_add(self.length)
            .ok_or_else(|| {
                AppError::InvalidArgument("native terminal selection length overflow".into())
            })?;
        if cells <= columns {
            return Ok(Point::new(self.start.line, Column(cells)));
        }
        let at_edge = exclude_eol && cells % columns == 0;
        let rows = cells / columns - usize::from(at_edge);
        let column = if at_edge { columns } else { cells % columns };
        let line = i64::try_from(rows)
            .ok()
            .and_then(|rows| i64::from(self.start.line.0).checked_add(rows))
            .and_then(|line| i32::try_from(line).ok())
            .ok_or_else(|| {
                AppError::InvalidArgument("native terminal word span overflow".into())
            })?;
        Ok(Point::new(Line(line), Column(column)))
    }

    fn selection(self, columns: usize) -> AppResult<Selection> {
        let reversed = self.end.is_some_and(|end| end < self.start);
        let start = if reversed {
            self.end.unwrap_or(self.start)
        } else {
            self.start
        };
        let end = match self.end {
            None => self.span_end(columns, true)?,
            Some(_) if reversed => self.span_end(columns, true)?,
            Some(end) if self.length > 0 && end.line == self.start.line => {
                let span = self.span_end(columns, false)?;
                if span.line == self.start.line {
                    span.max(end)
                } else {
                    span
                }
            }
            Some(end) => end,
        };
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Left);
        Ok(selection)
    }
}

#[derive(Default)]
struct Clicks {
    last: Option<(f64, egui::Pos2)>,
    prior: Option<f64>,
}

impl Clicks {
    fn count(&self, now: f64, position: egui::Pos2, options: egui::InputOptions) -> u8 {
        let Some((last, point)) = self.last else {
            return 1;
        };
        let elapsed = now - last;
        let close = position.distance_sq(point) < options.max_click_dist * options.max_click_dist;
        if !now.is_finite() || !close || elapsed < 0.0 {
            return 1;
        }
        if self.prior.is_some_and(|prior| {
            let elapsed = now - prior;
            elapsed >= 0.0
                && elapsed < options.max_double_click_delay * f64::from(DOUBLE_CLICK_COUNT)
        }) {
            return TRIPLE_CLICK_COUNT;
        }
        if elapsed < options.max_double_click_delay {
            return DOUBLE_CLICK_COUNT;
        }
        1
    }

    fn completed(&mut self, now: f64, position: egui::Pos2) {
        if !now.is_finite() || !position.is_finite() {
            *self = Self::default();
            return;
        }
        self.prior = self.last.map(|(time, _)| time);
        self.last = Some((now, position));
    }
}

#[derive(Default)]
struct Blink {
    key: Option<(Point, CursorStyle, bool)>,
    started: f64,
}

impl Blink {
    fn visible(
        &mut self,
        now: f64,
        point: Point,
        style: CursorStyle,
        focused: bool,
    ) -> (bool, Option<Duration>) {
        let key = Some((point, style, focused));
        if self.key != key || !now.is_finite() || !self.started.is_finite() || now < self.started {
            self.key = key;
            self.started = if now.is_finite() { now } else { 0.0 };
        }
        if !focused || !style.blinking || !now.is_finite() {
            return (true, None);
        }
        let elapsed = (now - self.started).max(0.0);
        let interval = CURSOR_BLINK_INTERVAL.as_secs_f64();
        let phase = elapsed % (interval * 2.0);
        let visible = phase < interval;
        let delay = interval - elapsed % interval;
        (visible, Some(Duration::from_secs_f64(delay)))
    }
}

fn application_keymap_decision(
    decision: &KeymapDecision,
    actions: &mut Vec<String>,
    has_focused_shell: bool,
    consume_unhandled_chord: bool,
    command_context: &CommandContext,
) -> bool {
    match decision {
        KeymapDecision::Dispatch(id) | KeymapDecision::ResolveChord(id)
            if crate::command_dispatch::accepts(id, has_focused_shell, command_context) =>
        {
            actions.push(id.clone());
            true
        }
        KeymapDecision::EnterChord | KeymapDecision::NoMatch => true,
        KeymapDecision::ResolveChord(_) => consume_unhandled_chord,
        _ => false,
    }
}

#[derive(Default)]
pub struct Views {
    palette: Arc<Mutex<Option<Appearance>>>,
    views: HashMap<(egui::ViewportId, PaneId, TabId), View>,
    keymaps: crate::keymap::Windows,
    command_context: CommandContext,
    outboxes: HashMap<String, Arc<Mutex<Outbox>>>,
    attaching: HashSet<TabId>,
    failed: HashMap<TabId, AppError>,
    attaching_geometry: HashMap<TabId, Arc<Mutex<WindowSize>>>,
    geometry: HashMap<String, Arc<Mutex<WindowSize>>>,
    resizing: HashSet<String>,
    raw_frame: Option<(egui::ViewportId, u64)>,
    pointer_sequence: u64,
}

pub struct Request<'surface> {
    pub pane: &'surface PaneId,
    pub tab: &'surface TabId,
    pub session_id: &'surface str,
    pub hub: &'surface Hub,
    pub services: &'surface AppServices,
    pub appearance: &'surface Appearance,
    pub locale: &'surface ResolvedLocale,
    pub request_focus: bool,
    pub commands: &'surface mut Vec<HostCommand>,
}

#[derive(Clone)]
pub struct PasteTarget {
    key: (egui::ViewportId, PaneId, TabId),
    session: String,
    lifetime: Weak<()>,
}

impl PasteTarget {
    pub(crate) fn viewport(&self) -> egui::ViewportId {
        self.key.0
    }
    pub(crate) fn matches(&self, source: &crate::terminal_tabs::MenuTarget) -> bool {
        self.key.1 == source.pane && self.key.2 == source.tab && self.session == source.session
    }

    pub(crate) fn is_alive(&self) -> bool {
        self.lifetime.strong_count() > 0
    }
}

struct MenuSnapshot {
    popup: egui::Id,
    position: egui::Pos2,
    target: crate::terminal_tabs::MenuTarget,
    paste: PasteTarget,
    can_copy: bool,
    horizontal: bool,
    vertical: bool,
    split_items: Vec<egui::Id>,
    split_focus_pending: bool,
    root_items: Vec<egui::Id>,
    root_search: MenuSearch,
    split_search: MenuSearch,
}

#[derive(Default)]
struct MenuSearch {
    query: String,
    expires_at: f64,
    frame: Option<u64>,
    before_events: Option<(String, f64)>,
    suppressed_space: HashSet<usize>,
}

#[derive(Clone, Copy)]
enum MenuAction {
    Copy,
    Paste,
    SelectAll,
    Clear,
    Split(taide_model::layout::DropEdge),
    New,
    Kill,
}

struct MenuRequest<'menu> {
    response: &'menu egui::Response,
    opening: Option<egui::Pos2>,
    session: &'menu Session,
    services: &'menu AppServices,
    locale: &'menu ResolvedLocale,
    commands: &'menu mut Vec<HostCommand>,
    pane: &'menu PaneId,
    tab: &'menu TabId,
}

fn menu_item(response: egui::Response) -> egui::Response {
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(egui::accesskit::Role::MenuItem);
    });
    response
}

fn menu_separator(ui: &mut Ui) {
    let separator = ui.separator();
    separator.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::Other));
    ui.ctx().accesskit_node_builder(separator.id, |node| {
        node.set_role(egui::accesskit::Role::Splitter);
        node.set_orientation(egui::accesskit::Orientation::Horizontal);
    });
}

fn consume_menu_key(ui: &Ui, keys: &[egui::Key]) -> bool {
    ui.input_mut(|input| {
        let key = input.events.iter().find_map(|event| {
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
                && keys.contains(key)
            {
                return Some((*modifiers, *key));
            }
            None
        });
        key.is_some_and(|(modifiers, key)| input.consume_key(modifiers, key))
    })
}

fn prepare_menu_search(ui: &Ui, owner: egui::Id, items: &[egui::Id], search: &mut MenuSearch) {
    search.suppressed_space.clear();
    let frame = ui.ctx().cumulative_frame_nr();
    if search.frame == Some(frame) {
        if let Some((query, expires_at)) = &search.before_events {
            search.query.clone_from(query);
            search.expires_at = *expires_at;
        }
    } else {
        search.frame = Some(frame);
        search.before_events = Some((search.query.clone(), search.expires_at));
    }
    let time = ui.input(|input| input.time);
    if time >= search.expires_at {
        search.query.clear();
    }
    if !ui.is_enabled() || !ui.memory(|memory| memory.allows_interaction(ui.layer_id())) {
        return;
    }
    if !search.query.is_empty() {
        ui.ctx()
            .request_repaint_after(Duration::from_secs_f64((search.expires_at - time).max(0.0)));
    }
    let (raw, events) = ui.input(|input| (input.raw.events.clone(), input.events.clone()));
    let mut next = 0;
    let indices: Vec<_> = events
        .iter()
        .map(|event| egui::Context::raw_event_index(&raw, event, &mut next))
        .collect();
    let target = |node| {
        std::iter::once(owner)
            .chain(items.iter().copied())
            .find(|id| id.accesskit_id() == node)
    };
    let mut focused = ui.ctx().keyboard_focus_before_events();
    let mut is_searching = !search.query.is_empty();
    let mut character_key = None;
    for (index, event) in raw.iter().enumerate() {
        if let Some(node) = ui.ctx().keyboard_focus_request_at(index) {
            focused = target(node);
        }
        let preceding_key = character_key.take();
        if indices.contains(&index) {
            let owns_input = focused.is_some_and(|id| id == owner || items.contains(&id));
            if let Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } = event
            {
                if owns_input && is_searching && *key == egui::Key::Space {
                    search.suppressed_space.insert(index);
                }
                if *pressed {
                    character_key = Some(*modifiers);
                }
            }
            if let Event::Text(text) = event
                && let Some(modifiers) = preceding_key
                && !modifiers.ctrl
                && !modifiers.alt
                && !modifiers.command
                && !modifiers.mac_cmd
                && text.encode_utf16().count() == 1
                && owns_input
            {
                is_searching = true;
            }
            if matches!(event, Event::WindowFocused(false)) {
                focused = None;
            }
        }
        if let Some(node) = ui.ctx().keyboard_focus_after_request_at(index) {
            focused = target(node);
        }
    }
    ui.input_mut(|input| {
        let mut next = 0;
        input.events.retain(|event| {
            let index = egui::Context::raw_event_index(&input.raw.events, event, &mut next);
            !search.suppressed_space.contains(&index)
        });
    });
}

fn navigate_menu(
    ui: &Ui,
    owner: egui::Id,
    items: &[(egui::Id, bool, String)],
    search: &mut MenuSearch,
) -> Option<egui::Id> {
    if !ui.is_enabled() || !ui.memory(|memory| memory.allows_interaction(ui.layer_id())) {
        return None;
    }
    let enabled: Vec<_> = items
        .iter()
        .filter_map(|(id, enabled, _)| enabled.then_some(*id))
        .collect();
    let (raw, events) = ui.input(|input| (input.raw.events.clone(), input.events.clone()));
    let mut next = 0;
    let indices: Vec<_> = events
        .iter()
        .map(|event| egui::Context::raw_event_index(&raw, event, &mut next))
        .collect();
    let mut focused = ui.ctx().keyboard_focus_before_events();
    let target = |node| {
        std::iter::once(owner)
            .chain(enabled.iter().copied())
            .find(|id| id.accesskit_id() == node)
    };
    let mut consumed = HashSet::new();
    let mut scheduled_focus = None;
    let mut selected_item = None;
    let mut is_navigation = false;
    let time = ui.input(|input| input.time);
    let mut character_key = None;
    for (index, event) in raw.iter().enumerate() {
        if let Some(node) = ui.ctx().keyboard_focus_request_at(index) {
            focused = target(node);
        }
        let preceding_key = character_key.take();
        if let Event::Key {
            pressed: true,
            modifiers,
            ..
        } = event
            && (indices.contains(&index) || search.suppressed_space.contains(&index))
        {
            character_key = Some(*modifiers);
        }
        if let Some(position) = indices.iter().position(|candidate| *candidate == index) {
            if let Event::Text(text) = event
                && let Some(modifiers) = preceding_key
                && !modifiers.ctrl
                && !modifiers.alt
                && !modifiers.command
                && !modifiers.mac_cmd
                && text.encode_utf16().count() == 1
                && focused.is_some_and(|id| id == owner || enabled.contains(&id))
            {
                search.query.push_str(text);
                search.expires_at = time + MENU_SEARCH_TIMEOUT.as_secs_f64();
                ui.ctx().request_repaint_after(MENU_SEARCH_TIMEOUT);
                let first = search.query.chars().next().unwrap();
                let repeated = search.query.chars().all(|character| character == first);
                let query = if repeated {
                    first.to_string()
                } else {
                    search.query.clone()
                };
                let current = focused.and_then(|id| items.iter().find(|(item, _, _)| *item == id));
                let current_label = current.map(|(_, _, label)| label.trim());
                let start = current_label
                    .filter(|label| !label.is_empty())
                    .and_then(|label| {
                        enabled.iter().position(|id| {
                            items
                                .iter()
                                .find(|(item, _, _)| item == id)
                                .is_some_and(|(_, _, text)| text.trim() == label)
                        })
                    })
                    .unwrap_or_default();
                let exclude_current = query.encode_utf16().count() == 1;
                let query = query.to_lowercase();
                let matched = enabled
                    .iter()
                    .cycle()
                    .skip(start)
                    .take(enabled.len())
                    .find(|id| {
                        let label = items
                            .iter()
                            .find(|(item, _, _)| item == *id)
                            .unwrap()
                            .2
                            .trim();
                        (!exclude_current || Some(label) != current_label)
                            && label.to_lowercase().starts_with(&query)
                    });
                if let Some(id) = matched
                    && items
                        .iter()
                        .find(|(item, _, _)| item == id)
                        .map(|(_, _, label)| label.trim())
                        != current_label
                {
                    scheduled_focus = Some(*id);
                    is_navigation = true;
                }
                consumed.insert(position);
            }
            if let Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                let is_owner = focused == Some(owner);
                let item =
                    focused.and_then(|id| enabled.iter().position(|candidate| *candidate == id));
                if is_owner || item.is_some() {
                    if item.is_some() && matches!(key, egui::Key::Enter | egui::Key::Space) {
                        selected_item = focused;
                        consumed.insert(position);
                        is_navigation = true;
                    }
                    if *key == egui::Key::Tab {
                        consumed.insert(position);
                        is_navigation = true;
                    }
                    if matches!(key, egui::Key::ArrowLeft | egui::Key::ArrowRight) {
                        is_navigation = true;
                    }
                    if is_owner || !modifiers.any() {
                        let selected = match key {
                            egui::Key::Home | egui::Key::PageUp => enabled.first().copied(),
                            egui::Key::End | egui::Key::PageDown => enabled.last().copied(),
                            egui::Key::ArrowDown => item.map_or_else(
                                || enabled.first().copied(),
                                |index| enabled.get(index + 1).copied(),
                            ),
                            egui::Key::ArrowUp => item.map_or_else(
                                || enabled.last().copied(),
                                |index| {
                                    index
                                        .checked_sub(1)
                                        .and_then(|index| enabled.get(index).copied())
                                },
                            ),
                            _ => None,
                        };
                        if matches!(
                            key,
                            egui::Key::Home
                                | egui::Key::PageUp
                                | egui::Key::End
                                | egui::Key::PageDown
                                | egui::Key::ArrowDown
                                | egui::Key::ArrowUp
                        ) {
                            consumed.insert(position);
                            is_navigation = true;
                            if is_owner {
                                focused = selected.or(Some(owner));
                            } else if let Some(id) = selected {
                                scheduled_focus = Some(id);
                            }
                        }
                    }
                }
            }
            if matches!(event, Event::WindowFocused(false)) {
                focused = None;
                scheduled_focus = None;
            }
        }
        if let Some(node) = ui.ctx().keyboard_focus_after_request_at(index) {
            focused = target(node);
        }
    }
    if !is_navigation {
        ui.memory_mut(|memory| {
            if let Some(id) = memory
                .focused()
                .filter(|id| *id == owner || enabled.contains(id))
            {
                memory.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                );
            }
        });
        return None;
    }
    ui.input_mut(|input| {
        let mut index = 0;
        input.events.retain(|_| {
            let keep = !consumed.contains(&index);
            index += 1;
            keep
        });
    });
    ui.memory_mut(|memory| {
        memory.move_focus(egui::FocusDirection::None);
        if let Some(id) = scheduled_focus
            .or(focused)
            .filter(|id| *id == owner || enabled.contains(id))
        {
            memory.request_focus(id);
            memory.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        }
    });
    selected_item
}

fn terminal_context_menu(ui: &Ui, view: &mut View, request: MenuRequest<'_>) -> AppResult<()> {
    let MenuRequest {
        response,
        opening,
        session,
        services,
        locale,
        commands,
        pane,
        tab,
    } = request;
    if !ui.is_enabled() {
        let popup = egui::Popup::context_menu(response);
        view.menu = None;
        egui::Popup::close_id(ui.ctx(), popup.get_id());
        return Ok(());
    }
    let mut popup = egui::Popup::context_menu(response)
        .open_memory(if opening.is_some() {
            Some(egui::SetOpenCommand::Bool(true))
        } else {
            response
                .clicked()
                .then_some(egui::SetOpenCommand::Bool(false))
        })
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    if let Some(position) = opening.or_else(|| view.menu.as_ref().map(|menu| menu.position)) {
        popup = popup.at_position(position);
    }
    if view.menu.is_none() && (opening.is_some() || popup.is_open()) && ui.is_enabled() {
        let Some(position) = opening.or_else(|| popup.get_anchor_rect().map(|rect| rect.min))
        else {
            return Ok(());
        };
        let target = crate::terminal_tabs::MenuTarget {
            project: session.metadata().project_id().clone(),
            pane: pane.clone(),
            tab: tab.clone(),
            session: view.session.clone(),
        };
        let valid = services
            .state
            .layouts
            .read()
            .get(&target.project)
            .is_some_and(|layout| crate::terminal_tabs::menu_tab(layout, &target).is_some());
        if !valid {
            egui::Popup::close_id(ui.ctx(), popup.get_id());
            return Ok(());
        }
        let can_copy = session.snapshot(|state| {
            reconcile_view(state.core, view)?;
            view.selection
                .as_ref()
                .map(|selection| state.core.selection_text(selection, INPUT_BYTES))
                .transpose()
                .map(|text| text.flatten().is_some_and(|text| !text.is_empty()))
        })??;
        let required = taide_native_ui::split::MIN_PANE_SIZE * SPLIT_PANE_COUNT
            + services.state.settings.read().resizer_thickness as f32;
        view.menu = Some(MenuSnapshot {
            popup: popup.get_id(),
            position,
            target,
            paste: PasteTarget {
                key: (ui.ctx().viewport_id(), pane.clone(), tab.clone()),
                session: view.session.clone(),
                lifetime: Arc::downgrade(&view.paste_lifetime),
            },
            can_copy,
            horizontal: response.rect.width().is_finite()
                && response.rect.width() > 0.0
                && response.rect.width() >= required,
            vertical: response.rect.height().is_finite()
                && response.rect.height() > 0.0
                && response.rect.height() >= required,
            split_items: Vec::new(),
            split_focus_pending: false,
            root_items: Vec::new(),
            root_search: MenuSearch::default(),
            split_search: MenuSearch::default(),
        });
    }
    let Some(menu) = &mut view.menu else {
        return Ok(());
    };
    let valid = services
        .state
        .layouts
        .read()
        .get(&menu.target.project)
        .is_some_and(|layout| crate::terminal_tabs::menu_tab(layout, &menu.target).is_some());
    if !valid {
        egui::Popup::close_id(ui.ctx(), popup.get_id());
        view.menu = None;
        return Ok(());
    }
    let label = |key| crate::presentation::message(locale, key, &[]);
    let target = menu.target.clone();
    let paste = menu.paste.clone();
    let menu_owner = response.id.with("context-menu-keyboard-owner");
    let mut action = None;
    popup.show(|menu_ui| {
        let owner = menu_ui.interact(
            menu_ui.max_rect(),
            menu_owner,
            Sense::focusable_noninteractive(),
        );
        owner.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::Other));
        menu_ui.ctx().accesskit_node_builder(menu_owner, |node| {
            node.set_role(egui::accesskit::Role::Menu);
            node.add_action(egui::accesskit::Action::Focus);
        });
        if opening.is_some()
            && menu_ui
                .ctx()
                .keyboard_input_route(menu_owner)
                .is_none_or(|(_, (focused, _))| focused == Some(true))
        {
            owner.request_focus();
        }
        menu_ui.scope_builder(
            egui::UiBuilder::new().accessibility_parent(menu_owner),
            |menu_ui| {
                prepare_menu_search(menu_ui, menu_owner, &menu.root_items, &mut menu.root_search);
                let mut menu_items = Vec::new();
                let mut choices = Vec::new();
                for (key, enabled, choice) in [
                    ("terminal.copy", menu.can_copy, MenuAction::Copy),
                    ("terminal.paste", true, MenuAction::Paste),
                    ("terminal.selectAll", true, MenuAction::SelectAll),
                ] {
                    let text = label(key);
                    let item =
                        menu_item(menu_ui.add_enabled(enabled, egui::Button::new(text.clone())));
                    menu_items.push((item.id, item.enabled(), text));
                    choices.push((item.id, choice));
                    if item.clicked() {
                        action = Some(choice);
                        menu_ui.close();
                    }
                }
                menu_separator(menu_ui);
                let clear = menu_item(menu_ui.button(label("terminal.clear")));
                menu_items.push((clear.id, clear.enabled(), label("terminal.clear")));
                choices.push((clear.id, MenuAction::Clear));
                if clear.clicked() {
                    action = Some(MenuAction::Clear);
                    menu_ui.close();
                }
                menu_separator(menu_ui);
                let split_id = menu_ui.next_auto_id();
                let split_owner = split_id.with("submenu-keyboard-owner");
                let focus = menu_ui.ctx().keyboard_focus_before_events();
                let open_from_keyboard = focus == Some(split_id)
                    && menu_ui.is_enabled()
                    && consume_menu_key(
                        menu_ui,
                        &[egui::Key::ArrowRight, egui::Key::Enter, egui::Key::Space],
                    );
                let close_left = focus
                    .is_some_and(|id| id == split_owner || menu.split_items.contains(&id))
                    && menu_ui.is_enabled()
                    && consume_menu_key(menu_ui, &[egui::Key::ArrowLeft]);
                let mut open_override = None;
                if open_from_keyboard {
                    open_override = Some(true);
                    menu.split_focus_pending = true;
                    menu_ui.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                }
                if close_left {
                    open_override = Some(false);
                    menu.split_focus_pending = false;
                    menu_ui.memory_mut(|memory| {
                        memory.move_focus(egui::FocusDirection::None);
                        memory.request_focus(split_id);
                    });
                }
                let mut split_button =
                    egui::containers::menu::SubMenuButton::new(label("tab.split"));
                split_button.sub_menu = split_button.sub_menu.open_override(open_override);
                let (split, submenu) = split_button.ui(menu_ui, |submenu| {
                    use taide_model::layout::DropEdge;
                    prepare_menu_search(
                        submenu,
                        split_owner,
                        &menu.split_items,
                        &mut menu.split_search,
                    );
                    let owner = submenu.interact(
                        submenu.max_rect(),
                        split_owner,
                        Sense::focusable_noninteractive(),
                    );
                    owner.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::Other));
                    submenu.ctx().accesskit_node_builder(split_owner, |node| {
                        node.set_role(egui::accesskit::Role::Menu);
                        node.set_labelled_by([split_id.accesskit_id()]);
                    });
                    let mut items = Vec::new();
                    let mut choices = Vec::new();
                    submenu.scope_builder(
                        egui::UiBuilder::new().accessibility_parent(split_owner),
                        |submenu| {
                            for (edge, key, enabled) in [
                                (DropEdge::Left, "editorArea.splitLeft", menu.horizontal),
                                (DropEdge::Right, "editorArea.splitRight", menu.horizontal),
                                (DropEdge::Top, "editorArea.splitTop", menu.vertical),
                                (DropEdge::Bottom, "editorArea.splitBottom", menu.vertical),
                            ] {
                                let text = label(key);
                                let item = menu_item(
                                    submenu.add_enabled(enabled, egui::Button::new(text.clone())),
                                );
                                if menu.split_focus_pending && item.enabled() {
                                    item.request_focus();
                                    menu.split_focus_pending = false;
                                }
                                items.push((item.id, item.enabled(), text));
                                choices.push((item.id, MenuAction::Split(edge)));
                                if item.clicked() {
                                    action = Some(MenuAction::Split(edge));
                                    submenu.close();
                                }
                            }
                        },
                    );
                    if menu.split_focus_pending && owner.enabled() {
                        owner.request_focus();
                        menu.split_focus_pending = false;
                    }
                    if let Some(selected) =
                        navigate_menu(submenu, split_owner, &items, &mut menu.split_search)
                        && let Some((_, choice)) = choices.iter().find(|(id, _)| *id == selected)
                    {
                        action = Some(*choice);
                        submenu.close();
                    }
                    menu.split_items = items.iter().map(|(id, _, _)| *id).collect();
                });
                if submenu.is_none() {
                    menu.split_items.clear();
                    menu.split_focus_pending = false;
                    menu.split_search = MenuSearch::default();
                }
                let split = menu_item(split);
                menu_items.push((split.id, split.enabled(), label("tab.split")));
                split.ctx.accesskit_node_builder(split.id, |node| {
                    node.set_label(label("tab.split"));
                    node.set_has_popup(egui::accesskit::HasPopup::Menu);
                    node.set_expanded(submenu.is_some());
                    if submenu.is_some() {
                        node.set_controls([split_owner.accesskit_id()]);
                    }
                });
                let new = menu_item(menu_ui.button(label("tab.newTerminal")));
                menu_items.push((new.id, new.enabled(), label("tab.newTerminal")));
                choices.push((new.id, MenuAction::New));
                if new.clicked() {
                    action = Some(MenuAction::New);
                    menu_ui.close();
                }
                menu_separator(menu_ui);
                let kill = menu_item(menu_ui.button(label("terminal.kill")));
                menu_items.push((kill.id, kill.enabled(), label("terminal.kill")));
                choices.push((kill.id, MenuAction::Kill));
                if kill.clicked() {
                    action = Some(MenuAction::Kill);
                    menu_ui.close();
                }
                if let Some(selected) =
                    navigate_menu(menu_ui, menu_owner, &menu_items, &mut menu.root_search)
                    && let Some((_, choice)) = choices.iter().find(|(id, _)| *id == selected)
                {
                    action = Some(*choice);
                    menu_ui.close();
                }
                menu.root_items = menu_items.iter().map(|(id, _, _)| *id).collect();
            },
        );
    });
    if let Some(action) = action {
        match action {
            MenuAction::Copy => {
                if let Some(text) = session.snapshot(|state| {
                    reconcile_view(state.core, view)?;
                    view.selection
                        .as_ref()
                        .map(|selection| state.core.selection_text(selection, INPUT_BYTES))
                        .transpose()
                        .map(Option::flatten)
                })?? {
                    commands.push(HostCommand::CopyTerminalSelection(text));
                }
            }
            MenuAction::Paste => commands.push(HostCommand::ReadTerminalClipboard(paste)),
            MenuAction::SelectAll => {
                view.selection = Some(session.snapshot(|state| full_selection(state.core))??);
                view.is_all_selected = true;
                view.gesture = Gesture::default();
            }
            MenuAction::Clear => {
                session.clear_current_row()?;
                session.snapshot(|state| reconcile_view(state.core, view))??;
                view.offset = 0;
            }
            MenuAction::Split(edge) => commands.push(HostCommand::TerminalMenu {
                target,
                operation: crate::terminal_tabs::MenuOperation::Split(edge),
                title: label("terminal.title"),
            }),
            MenuAction::New => commands.push(HostCommand::TerminalMenu {
                target,
                operation: crate::terminal_tabs::MenuOperation::New,
                title: label("terminal.title"),
            }),
            MenuAction::Kill => commands.push(HostCommand::TerminalMenu {
                target,
                operation: crate::terminal_tabs::MenuOperation::Kill,
                title: String::new(),
            }),
        }
    }
    if view.menu.is_some() && !response.context_menu_opened() {
        view.menu = None;
        response.request_focus();
    }
    Ok(())
}

impl Views {
    pub fn remote_effects(&self, context: &egui::Context) -> crate::remote_terminal::Effects {
        let palette = self.palette.clone();
        let context = context.clone();
        Arc::new(move |_, _| {
            let command_colors = palette
                .lock()
                .map_err(|_| AppError::Internal("native terminal palette lock poisoned".into()))?
                .as_ref()
                .ok_or_else(|| AppError::Internal("native terminal palette is unavailable".into()))?
                .command_colors;
            let context = context.clone();
            Ok(ObservePorts {
                command_colors,
                updated: Arc::new(move || context.request_repaint()),
                event: Arc::new(|_| Ok(())),
                stream: Arc::new(|_| Ok(())),
            })
        })
    }

    pub(crate) fn set_palette(&mut self, appearance: &Appearance) -> AppResult<()> {
        let mut palette = self
            .palette
            .lock()
            .map_err(|_| AppError::Internal("native terminal palette lock poisoned".into()))?;
        *palette = Some(appearance.clone());
        Ok(())
    }

    pub fn paste_target(
        &self,
        viewport: egui::ViewportId,
        pane: &PaneId,
        tab: &TabId,
    ) -> Option<PasteTarget> {
        let key = (viewport, pane.clone(), tab.clone());
        let view = self.views.get(&key)?;
        if view.session.is_empty() {
            return None;
        }
        Some(PasteTarget {
            key,
            session: view.session.clone(),
            lifetime: Arc::downgrade(&view.paste_lifetime),
        })
    }

    pub fn accept_file_links(
        &mut self,
        target: &PasteTarget,
        request: &crate::terminal_file_links::Request,
        result: AppResult<Vec<Option<String>>>,
    ) -> bool {
        let Some(lifetime) = target.lifetime.upgrade() else {
            return false;
        };
        let Some(view) = self.views.get_mut(&target.key) else {
            return false;
        };
        if view.session != target.session || !Arc::ptr_eq(&lifetime, &view.paste_lifetime) {
            return false;
        }
        view.file_links.complete(request, result)
    }

    pub fn cancel_file_links(
        &mut self,
        target: &PasteTarget,
        request: &crate::terminal_file_links::Request,
    ) {
        if let Some(view) = self.views.get_mut(&target.key)
            && view.session == target.session
            && target
                .lifetime
                .upgrade()
                .is_some_and(|lifetime| Arc::ptr_eq(&lifetime, &view.paste_lifetime))
        {
            view.file_links.cancel(request);
        }
    }

    pub fn accept_paste(
        &mut self,
        target: &PasteTarget,
        text: &str,
        hub: &Hub,
        services: &AppServices,
    ) -> AppResult<bool> {
        if services.state.is_shutting_down() {
            return Ok(false);
        }
        let Some(lifetime) = target.lifetime.upgrade() else {
            return Ok(false);
        };
        let Some(view) = self.views.get_mut(&target.key) else {
            return Ok(false);
        };
        if view.session != target.session || !Arc::ptr_eq(&lifetime, &view.paste_lifetime) {
            return Ok(false);
        }
        let Some(session) = hub.get(&target.session) else {
            return Ok(false);
        };
        let layouts = services.state.layouts.read();
        let Some(layout) = layouts.get(session.metadata().project_id()) else {
            return Ok(false);
        };
        let active = std::iter::once(&layout.root)
            .chain(layout.auxiliary_windows.iter().map(|window| &window.root))
            .filter_map(|root| taide_layout::service::find_leaf(root, &target.key.1))
            .any(|leaf| matches!(leaf, taide_model::layout::PaneNode::Leaf { tabs, active, .. }
                if active.as_ref() == Some(&target.key.2)
                    && tabs.iter().any(|tab| tab.id == target.key.2
                        && matches!(&tab.kind, taide_model::layout::TabKind::Terminal { session_id, .. }
                            if session_id == &target.session))));
        drop(layouts);
        if !active || !session.snapshot(|state| state.phase == Phase::Running)? {
            return Ok(false);
        }
        view.outbox
            .lock()
            .map_err(|_| AppError::Internal("native terminal input queue lock poisoned".into()))?
            .submit(&session, services, NativeInput::Paste(text))?;
        view.offset = 0;
        Ok(true)
    }

    fn capture_mouse(
        &mut self,
        context: &egui::Context,
        order: EventOrder,
        position: egui::Pos2,
        event: &Event,
        modifiers: egui::Modifiers,
        is_pointer_down: bool,
    ) -> bool {
        let viewport = context.viewport_id();
        let frame = order.frame;
        let is_wheel = matches!(event, Event::MouseWheel { .. });
        let is_capture = matches!(
            event,
            Event::PointerMoved(_) | Event::PointerButton { pressed: false, .. }
        );
        let has_capture = is_capture
            && self.views.iter().any(|((owner, _, _), view)| {
                *owner == viewport
                    && (view.mouse.button().is_some() || view.mouse.forced)
                    && view
                        .wheel_target
                        .as_ref()
                        .is_some_and(|target| target.enabled)
            });
        let layer = context.layer_id_at(position);
        let mut candidates = self.views.iter_mut().filter(|((owner, _, _), view)| {
            *owner == viewport
                && view.wheel_target.as_ref().is_some_and(|target| {
                    target.enabled
                        && target.mode.intersects(Mode::MOUSE_MODE)
                        && ((!is_wheel
                            && has_capture
                            && (view.mouse.button().is_some() || view.mouse.forced))
                            || (!has_capture
                                && target.frame.checked_add(1) == Some(frame)
                                && target.rect.contains(position)
                                && layer.is_none_or(|layer| layer == target.layer)))
                })
        });
        let Some((_, view)) = candidates.next() else {
            return false;
        };
        if candidates.next().is_some() {
            return false;
        }
        let mode = view.wheel_target.as_ref().unwrap().mode;
        let previous_buttons = view.mouse.buttons;
        let (action, modifiers) = match event {
            Event::PointerButton {
                button,
                pressed,
                modifiers,
                ..
            } => {
                let Some((index, button)) = mouse_button(*button) else {
                    return false;
                };
                if *pressed && force_selection(*modifiers) {
                    if button == MouseButton::Left {
                        view.mouse.forced = true;
                    }
                    return false;
                }
                if button == MouseButton::Left && !pressed && view.mouse.forced {
                    view.mouse.forced = false;
                    return false;
                }
                if !pressed && mode.contains(Mode::MOUSE_X10) {
                    return false;
                }
                if !pressed && !view.mouse.buttons[index] {
                    return false;
                }
                if !mode.contains(Mode::MOUSE_X10) {
                    view.mouse.buttons[index] = *pressed;
                }
                (
                    Some(if *pressed {
                        MouseAction::Press(button)
                    } else {
                        MouseAction::Release(button)
                    }),
                    *modifiers,
                )
            }
            Event::PointerMoved(_) => {
                if view.mouse.forced || mode.contains(Mode::MOUSE_X10) {
                    return false;
                }
                let button = view.mouse.button();
                if button.is_none() && is_pointer_down {
                    return false;
                }
                if !mode.contains(Mode::MOUSE_MOTION)
                    && (!mode.contains(Mode::MOUSE_DRAG) || button.is_none())
                {
                    return false;
                }
                (Some(MouseAction::Move(button)), modifiers)
            }
            Event::MouseWheel {
                delta, modifiers, ..
            } if delta.y != 0.0 && tracks_wheel(mode) => (None, *modifiers),
            _ => return false,
        };
        let reserved_releases = view
            .mouse
            .buttons
            .iter()
            .filter(|pressed| **pressed)
            .count();
        if view.mouse.packets.len() + reserved_releases >= INPUT_RECEIPTS {
            view.mouse.buttons = previous_buttons;
            view.error = Some("native terminal captured mouse event budget exceeded".into());
        } else {
            let Some(sequence) = self.pointer_sequence.checked_add(1) else {
                view.mouse.buttons = previous_buttons;
                view.error = Some("native terminal mouse sequence overflow".into());
                return is_wheel;
            };
            self.pointer_sequence = sequence;
            view.mouse.packets.push_back(MousePacket {
                position,
                action,
                event: event.clone(),
                modifiers,
                mode,
                order,
                sequence,
                geometry: view.mouse_geometry,
            });
        }
        is_wheel
    }

    fn view_mut(&mut self, ui: &Ui, pane: &PaneId, tab: &TabId) -> &mut View {
        let viewport = ui.ctx().viewport_id();
        let frame = ui.ctx().cumulative_frame_nr();
        let view = self
            .views
            .entry((viewport, pane.clone(), tab.clone()))
            .or_default();
        view.is_wheel_routed = self.raw_frame == Some((viewport, frame));
        view
    }

    fn is_shown_elsewhere(
        &self,
        context: &egui::Context,
        pane: &PaneId,
        tab: &TabId,
        session: &str,
    ) -> bool {
        let viewport = context.viewport_id();
        self.views
            .iter()
            .any(|((owner, shown_pane, shown_tab), view)| {
                (*owner != viewport || shown_pane != pane || shown_tab != tab)
                    && view.session == session
                    && view.processed_pass.is_some_and(|pass| {
                        pass.saturating_add(1) >= context.cumulative_pass_nr_for(*owner)
                    })
            })
    }

    pub fn raw_input(&mut self, context: &egui::Context, input: &mut egui::RawInput) {
        self.raw_input_with_replay(context, input, 0);
    }

    pub fn raw_input_with_replay(
        &mut self,
        context: &egui::Context,
        input: &mut egui::RawInput,
        replayed_events: usize,
    ) {
        let viewport = input.viewport_id;
        if context.viewport_id() != viewport || replayed_events > input.events.len() {
            return;
        }
        let mut pointer = context.input_for(viewport, |input| input.pointer.hover_pos());
        let frame = context.cumulative_frame_nr_for(viewport);
        for ((owner, _, _), view) in &mut self.views {
            if *owner == viewport && view.mouse.frame != frame {
                view.mouse.frame = frame;
            }
        }
        self.raw_frame = Some((viewport, frame));
        let mut modifiers = context.input_for(viewport, |input| input.modifiers);
        let mut buttons = context.input_for(viewport, |input| {
            [
                egui::PointerButton::Primary,
                egui::PointerButton::Middle,
                egui::PointerButton::Secondary,
            ]
            .map(|button| input.pointer.button_down(button))
        });
        let mut event_index = 0;
        let mut retained_index = 0;
        input.events.retain(|event| {
            let is_replayed = event_index < replayed_events;
            event_index += 1;
            let order = EventOrder {
                frame,
                index: retained_index,
            };
            retained_index += 1;
            if let Event::PointerButton {
                button, pressed, ..
            } = event
                && let Some((index, _)) = mouse_button(*button)
            {
                buttons[index] = *pressed;
            }
            match event {
                Event::ModifiersChanged(value)
                | Event::Key {
                    modifiers: value, ..
                }
                | Event::PointerButton {
                    modifiers: value, ..
                }
                | Event::MouseWheel {
                    modifiers: value, ..
                } => modifiers = *value,
                _ => {}
            }
            match event {
                Event::PointerMoved(position) | Event::PointerButton { pos: position, .. } => {
                    pointer = Some(*position)
                }
                Event::PointerGone => {
                    pointer = None;
                    for ((owner, _, _), view) in &mut self.views {
                        if !is_replayed
                            && *owner == viewport
                            && !buttons.iter().any(|pressed| *pressed)
                        {
                            view.mouse.buttons.fill(false);
                            view.mouse.forced = false;
                        }
                    }
                }
                _ => {}
            }
            if is_replayed {
                return true;
            }
            if let Some(position) = pointer
                && self.capture_mouse(
                    context,
                    order,
                    position,
                    event,
                    modifiers,
                    buttons.iter().any(|pressed| *pressed),
                )
            {
                retained_index -= 1;
                return false;
            }
            match event {
                Event::MouseWheel { delta, .. }
                    if delta.y != 0.0 && delta.x.abs() <= delta.y.abs() =>
                {
                    let Some(position) = pointer else {
                        return true;
                    };
                    let layer = context.layer_id_at(position);
                    let mut candidates = self.views.iter_mut().filter(|((owner, _, _), view)| {
                        *owner == viewport
                            && view.wheel_target.as_ref().is_some_and(|target| {
                                target.enabled
                                    && !tracks_wheel(target.mode)
                                    && target.frame.checked_add(1) == Some(frame)
                                    && target.rect.contains(position)
                                    && layer.is_none_or(|layer| layer == target.layer)
                            })
                    });
                    let Some((_, view)) = candidates.next() else {
                        return true;
                    };
                    if candidates.next().is_some() {
                        return true;
                    }
                    if view.captured_wheel.packets.len() >= INPUT_RECEIPTS {
                        view.error =
                            Some("native terminal captured wheel event budget exceeded".into());
                    } else {
                        let Some(sequence) = self.pointer_sequence.checked_add(1) else {
                            view.error = Some("native terminal pointer sequence overflow".into());
                            retained_index -= 1;
                            return false;
                        };
                        self.pointer_sequence = sequence;
                        view.captured_wheel.packets.push_back(WheelPacket {
                            position,
                            event: event.clone(),
                            order,
                            sequence,
                            geometry: view.mouse_geometry,
                        });
                    }
                    retained_index -= 1;
                    return false;
                }
                _ => {}
            }
            true
        });
    }

    pub fn attached(
        &mut self,
        tab: TabId,
        result: &AppResult<String>,
        hub: &Hub,
        services: &AppServices,
    ) {
        if !self.attaching.remove(&tab) {
            return;
        }
        let geometry = self.attaching_geometry.remove(&tab);
        match result {
            Ok(session) => {
                if let Some(geometry) = geometry {
                    self.geometry.entry(session.clone()).or_insert(geometry);
                }
                self.failed.remove(&tab);
                if let Some(session) = hub.get(session) {
                    let outbox = self
                        .outboxes
                        .entry(session.id().into())
                        .or_default()
                        .clone();
                    for ((_, _, owner), view) in &mut self.views {
                        if owner != &tab {
                            continue;
                        }
                        let pending = std::mem::take(&mut view.pending);
                        *view = View {
                            session: session.id().into(),
                            pending,
                            outbox: outbox.clone(),
                            ..Default::default()
                        };
                        if !view.pending.is_empty() {
                            let units = view.pending.drain(..).collect::<Vec<_>>();
                            let pending = String::from_utf16_lossy(&units);
                            submit_input(
                                view,
                                &session,
                                services,
                                NativeInput::CommittedText(&pending),
                            );
                        }
                    }
                }
            }
            Err(error) => {
                self.failed.insert(tab.clone(), error.clone());
                for ((_, _, owner), view) in &mut self.views {
                    if owner == &tab {
                        view.pending.clear();
                        view.preedit.clear();
                    }
                }
            }
        }
    }

    pub fn resized(&mut self, session: &str) {
        self.resizing.remove(session);
    }

    pub fn submission_failed(&mut self) {
        self.attaching.clear();
        self.attaching_geometry.clear();
        self.resizing.clear();
        for view in self.views.values_mut() {
            view.pending.clear();
            view.preedit.clear();
        }
    }

    pub fn retain(&mut self, tabs: &HashSet<TabId>, sessions: &HashSet<String>) {
        self.views.retain(|(_, _, tab), _| tabs.contains(tab));
        self.attaching.retain(|tab| tabs.contains(tab));
        self.failed.retain(|tab, _| tabs.contains(tab));
        self.attaching_geometry.retain(|tab, _| tabs.contains(tab));
        self.geometry
            .retain(|session, _| sessions.contains(session));
        self.resizing.retain(|session| sessions.contains(session));
        self.outboxes
            .retain(|session, _| sessions.contains(session));
    }

    pub fn flush_inputs(
        &mut self,
        hub: &Hub,
        services: &AppServices,
        context: &egui::Context,
    ) -> AppResult<()> {
        self.flush_staged_inputs(hub, services, context, false)?;
        self.release_focus(hub, services, context, false);
        for (id, outbox) in &self.outboxes {
            let mut outbox = outbox.lock().map_err(|_| {
                AppError::Internal("native terminal input queue lock poisoned".into())
            })?;
            if let Some(session) = hub.get(id) {
                outbox.poll(&session, services);
            } else {
                outbox.clear();
            }
            if outbox.is_pending() {
                context.request_repaint_after(RECEIPT_POLL);
            }
        }
        self.flush_captured_pointer(hub, services, context, None)
    }

    pub fn finish_frame(
        &mut self,
        hub: &Hub,
        services: &AppServices,
        context: &egui::Context,
    ) -> AppResult<()> {
        self.flush_captured_pointer(
            hub,
            services,
            context,
            Some((
                context.viewport_id(),
                EventOrder {
                    frame: context.cumulative_frame_nr(),
                    index: usize::MAX,
                },
            )),
        )?;
        self.flush_staged_inputs(hub, services, context, true)?;
        self.release_focus(hub, services, context, true);
        Ok(())
    }

    fn flush_staged_inputs(
        &mut self,
        hub: &Hub,
        services: &AppServices,
        context: &egui::Context,
        finished: bool,
    ) -> AppResult<()> {
        let frame = context.cumulative_frame_nr();
        let through = if finished {
            Some(frame)
        } else {
            frame.checked_sub(1)
        };
        let Some(through) = through else {
            return Ok(());
        };
        for (id, outbox) in &self.outboxes {
            let mut outbox = outbox.lock().map_err(|_| {
                AppError::Internal("native terminal input queue lock poisoned".into())
            })?;
            if let Some(session) = hub.get(id) {
                if let Err(error) =
                    outbox.flush_staged(&session, services, context.viewport_id(), through)
                {
                    outbox.error = Some(error.to_string());
                    return Err(error);
                }
            } else {
                outbox.clear();
            }
        }
        Ok(())
    }

    fn ordered_focus_batch(&self, context: &egui::Context) -> bool {
        let mut target = context
            .keyboard_focus_before_events()
            .map(|id| id.accesskit_id());
        let events = context.input(|input| input.raw.events.clone());
        let mut has_request = false;
        for (index, event) in events.iter().enumerate() {
            let request = context.keyboard_focus_request_at(index);
            if let Some(node) = request {
                target = Some(node);
                has_request = true;
            }
            if is_wire_barrier(event)
                || (matches!(event, Event::PointerButton { pressed: true, .. })
                    && request.is_none())
            {
                return false;
            }
            if is_navigation(event)
                && !target.is_some_and(|node| context.keyboard_navigation_is_locked(node, event))
            {
                return false;
            }
            if matches!(event, Event::WindowFocused(false)) {
                target = None;
            }
            if let Some(node) = context.keyboard_focus_after_request_at(index) {
                target = Some(node);
                has_request = true;
            }
        }
        has_request
    }

    fn release_focus(
        &mut self,
        hub: &Hub,
        services: &AppServices,
        context: &egui::Context,
        finished: bool,
    ) {
        let viewport = context.viewport_id();
        let frame = context.cumulative_frame_nr();
        let (window_focused, live) = context.input(|input| {
            (
                input.raw.focused,
                input.raw.viewports.keys().copied().collect::<HashSet<_>>(),
            )
        });
        let focused_id = context.memory(|memory| memory.focused());
        let has_ordered_batch = !finished && self.ordered_focus_batch(context);
        for ((owner, _, _), view) in &mut self.views {
            let closed = *owner != viewport && !live.contains(owner);
            let rendered = view.focus_frame.is_some_and(|processed| {
                if finished {
                    processed == frame
                } else {
                    processed == frame || processed.checked_add(1) == Some(frame)
                }
            });
            if (closed || (*owner == viewport && !rendered))
                && Arc::weak_count(&view.paste_lifetime) > 0
            {
                view.paste_lifetime = Arc::new(());
            }
            if closed || (*owner == viewport && !rendered) {
                if *owner == viewport
                    && let Some(menu) = &view.menu
                {
                    egui::Popup::close_id(context, menu.popup);
                }
                view.menu = None;
                view.link_press = None;
                view.file_links = Default::default();
            }
            if closed
                || (*owner == viewport
                    && !has_ordered_batch
                    && (!window_focused || view.focus_id != focused_id || !rendered))
            {
                change_focus(view, hub, services, false);
                view.preedit.clear();
            }
        }
        self.views
            .retain(|(owner, _, _), _| *owner == viewport || live.contains(owner));
        self.keymaps
            .retain(|owner| owner == viewport || live.contains(&owner));
    }

    fn focus_view(
        &mut self,
        key: (egui::ViewportId, PaneId, TabId),
        id: egui::Id,
        context: &egui::Context,
        focused: bool,
        hub: &Hub,
        services: &AppServices,
    ) {
        if self.ordered_focus_batch(context) {
            if focused {
                for (other, view) in &mut self.views {
                    if other.0 != context.viewport_id() {
                        change_focus(view, hub, services, false);
                        view.preedit.clear();
                    }
                }
            }
            let view = self.views.entry(key).or_default();
            view.focus_id = Some(id);
            let frame = context.cumulative_frame_nr();
            view.focus_frame = Some(frame);
            if view.ordered_focus_frame == Some(frame) {
                return;
            }
            view.ordered_focus_frame = Some(frame);
            let mut window_focused = context.input(|input| input.raw.focused);
            let initial = window_focused && context.keyboard_focus_before_events() == Some(id);
            stage_focus(
                view,
                hub,
                services,
                initial,
                WireOrder {
                    viewport: context.viewport_id(),
                    event: EventOrder { frame, index: 0 },
                    phase: if initial {
                        WirePhase::InitialFocusGain
                    } else {
                        WirePhase::InitialFocusLoss
                    },
                },
            );
            let events = context.input(|input| input.raw.events.clone());
            for (index, event) in events.into_iter().enumerate() {
                let next = if let Some(node) = context.keyboard_focus_request_at(index) {
                    Some(window_focused && node == id.accesskit_id())
                } else {
                    match event {
                        Event::WindowFocused(false) => {
                            window_focused = false;
                            Some(false)
                        }
                        _ => None,
                    }
                };
                if let Some(next) = next {
                    stage_focus(
                        view,
                        hub,
                        services,
                        next,
                        WireOrder {
                            viewport: context.viewport_id(),
                            event: EventOrder { frame, index },
                            phase: if matches!(event, Event::PointerButton { .. }) {
                                if next {
                                    WirePhase::PointerFocusGain
                                } else {
                                    WirePhase::PointerFocusLoss
                                }
                            } else if next {
                                WirePhase::FocusGain
                            } else {
                                WirePhase::FocusLoss
                            },
                        },
                    );
                }
                if let Some(node) = context.keyboard_focus_after_request_at(index) {
                    let next = window_focused && node == id.accesskit_id();
                    stage_focus(
                        view,
                        hub,
                        services,
                        next,
                        WireOrder {
                            viewport: context.viewport_id(),
                            event: EventOrder { frame, index },
                            phase: if next {
                                WirePhase::ContextMenuFocusGain
                            } else {
                                WirePhase::ContextMenuFocusLoss
                            },
                        },
                    );
                }
            }
            return;
        }
        if focused {
            for (other, view) in &mut self.views {
                if *other != key {
                    if other.0 == context.viewport_id()
                        && view.focus_id.is_some_and(|id| {
                            context
                                .keyboard_input_route(id)
                                .is_some_and(|(ownership, _)| {
                                    ownership.iter().any(|(owned, _)| *owned == Some(true))
                                })
                        })
                    {
                        continue;
                    }
                    change_focus(view, hub, services, false);
                    view.preedit.clear();
                }
            }
        }
        let view = self.views.entry(key).or_default();
        view.focus_id = Some(id);
        view.focus_frame = Some(context.cumulative_frame_nr());
        view.ordered_focus_frame = None;
        if !focused
            && context
                .keyboard_input_route(id)
                .is_some_and(|(ownership, _)| {
                    ownership.iter().any(|(owned, _)| *owned == Some(true))
                })
        {
            return;
        }
        change_focus(view, hub, services, focused);
    }

    fn flush_captured_pointer(
        &mut self,
        hub: &Hub,
        services: &AppServices,
        context: &egui::Context,
        through: Option<(egui::ViewportId, EventOrder)>,
    ) -> AppResult<()> {
        let ordered = self.ordered_focus_batch(context);
        loop {
            let mut next = None;
            for (key, view) in &mut self.views {
                let mouse = view
                    .mouse
                    .packets
                    .front()
                    .map(|packet| (packet.sequence, packet.order, false));
                let wheel = view
                    .captured_wheel
                    .packets
                    .front()
                    .map(|packet| (packet.sequence, packet.order, true));
                let Some((sequence, order, is_wheel)) =
                    mouse.into_iter().chain(wheel).min_by_key(|packet| packet.0)
                else {
                    continue;
                };
                let Some(session) = hub.get(&view.session) else {
                    view.mouse = Mouse::default();
                    view.captured_wheel.packets.clear();
                    continue;
                };
                let has_reached_event = through
                    .is_some_and(|(viewport, through)| viewport == key.0 && order <= through);
                let is_processed = view
                    .processed_frame
                    .is_some_and(|frame| order.frame <= frame);
                let completed_frame = if key.0 == context.viewport_id() {
                    context.cumulative_frame_nr()
                } else {
                    view.mouse.frame.max(order.frame)
                };
                if !has_reached_event && !is_processed && order.frame >= completed_frame {
                    continue;
                }
                context.request_repaint_after(RECEIPT_POLL);
                if view.input_capacity()? == 0
                    && (!is_wheel
                        || !session.snapshot(|snapshot| snapshot.core.has_scrollback())??)
                {
                    continue;
                }
                if next
                    .as_ref()
                    .is_none_or(|(_, _, previous, _, _)| sequence < *previous)
                {
                    next = Some((key.clone(), session, sequence, order, is_wheel));
                }
            }
            let Some((key, session, _, through, is_wheel)) = next else {
                break;
            };
            let view = self.views.get_mut(&key).unwrap();
            let previous_order = view.input_order;
            if ordered
                && key.0 == context.viewport_id()
                && through.frame == context.cumulative_frame_nr()
            {
                view.input_order = Some(WireOrder {
                    viewport: key.0,
                    event: through,
                    phase: WirePhase::CapturedInput,
                });
            }
            match session.snapshot(|snapshot| {
                if snapshot.phase != Phase::Running {
                    view.mouse = Mouse::default();
                    view.captured_wheel.packets.clear();
                    return Ok(Vec::new());
                }
                reconcile_view(snapshot.core, view)?;
                if is_wheel {
                    return Ok(take_wheel(snapshot.core, view, Some(through), 1, None)?
                        .into_iter()
                        .map(|key| NativeInput::Key {
                            key,
                            modifiers: Default::default(),
                        })
                        .collect());
                }
                Ok(take_mouse(snapshot.core, view, Some(through), 1)?
                    .into_iter()
                    .map(NativeInput::Mouse)
                    .collect())
            }) {
                Ok(Ok(inputs)) => {
                    for input in inputs {
                        submit_input(view, &session, services, input);
                    }
                }
                Ok(Err(error)) | Err(error) => {
                    view.mouse = Mouse::default();
                    view.captured_wheel.packets.clear();
                    view.error = Some(error.to_string());
                }
            }
            view.input_order = previous_order;
        }
        Ok(())
    }

    pub fn cancel_inputs(&mut self) {
        self.keymaps.clear();
        for view in self.views.values_mut() {
            view.paste_lifetime = Arc::new(());
            view.menu = None;
            view.link_press = None;
            view.file_links = Default::default();
        }
        for view in self.views.values_mut() {
            view.mouse = Mouse::default();
            view.captured_wheel.packets.clear();
        }
        for outbox in self.outboxes.values() {
            outbox
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clear();
        }
    }

    pub fn show(&mut self, ui: &mut Ui, request: Request<'_>) -> AppResult<egui::Response> {
        self.show_with_keymap(ui, request, |_| false)
    }

    pub(crate) fn capture_window_keymap(
        &mut self,
        context: &egui::Context,
        overrides: Option<&str>,
        actions: &mut Vec<String>,
        has_focused_shell: bool,
        editor_composition: impl Fn(egui::Id) -> Option<bool>,
    ) -> AppResult<()> {
        if !context.input(|input| input.focused) {
            return Ok(());
        }
        let events = context.input_mut(|input| std::mem::take(&mut input.events));
        let mut focused = context.keyboard_focus_before_events();
        let mut next = 0;
        let mut remaining = Vec::new();
        let mut failure = None;
        for event in events {
            let index = crate::keymap::event_index(context, &event, &mut next);
            if let Some(node) = context.keyboard_focus_request_at(index) {
                focused = context.previous_keyboard_target(node);
            }
            let scope = focused.and_then(|id| {
                self.keyboard_target_scope(context, id)
                    .or_else(|| {
                        editor_composition(id).map(|composing| {
                            (
                                KeymapContext {
                                    terminal: false,
                                    editor: true,
                                },
                                composing,
                            )
                        })
                    })
                    .or_else(|| {
                        context
                            .button_had_default_keys(id)
                            .then_some((KeymapContext::default(), false))
                    })
                    .or_else(|| {
                        context
                            .is_context_menu_keyboard_owner(id)
                            .then_some((KeymapContext::default(), false))
                    })
            });
            let handled = match scope {
                Some((scope, composing)) => self.keymaps.route(
                    crate::keymap::Route {
                        context,
                        event: &event,
                        index,
                        scope,
                        composing,
                        overrides,
                    },
                    |decision| {
                        application_keymap_decision(
                            decision,
                            actions,
                            has_focused_shell,
                            !scope.terminal && !scope.editor,
                            &self.command_context,
                        )
                    },
                ),
                None => Ok(false),
            };
            let consumed = match handled {
                Ok(handled) => handled,
                Err(error) => {
                    failure = Some(error);
                    true
                }
            };
            if !consumed {
                let is_locked_navigation = is_navigation(&event)
                    && focused.is_some_and(|id| {
                        context.keyboard_navigation_is_locked(id.accesskit_id(), &event)
                    });
                if matches!(
                    &event,
                    Event::PointerButton { pressed: true, .. }
                        | Event::Touch { .. }
                        | Event::WindowFocused(_)
                        | Event::Ime(_)
                        | Event::Key {
                            key: egui::Key::Tab
                                | egui::Key::ArrowUp
                                | egui::Key::ArrowDown
                                | egui::Key::ArrowLeft
                                | egui::Key::ArrowRight,
                            pressed: true,
                            ..
                        }
                        | Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                            action: egui::accesskit::Action::Click,
                            ..
                        })
                ) && !is_locked_navigation
                    && context.keyboard_focus_request_at(index).is_none()
                {
                    focused = None;
                }
                remaining.push(event);
            }
            if let Some(node) = context.keyboard_focus_after_request_at(index) {
                focused = context.previous_keyboard_target(node);
            }
        }
        context.input_mut(|input| input.events = remaining);
        failure.map_or(Ok(()), Err)
    }

    fn keyboard_target_scope(
        &self,
        context: &egui::Context,
        id: egui::Id,
    ) -> Option<(KeymapContext, bool)> {
        let frame = context.cumulative_frame_nr();
        self.views.iter().find_map(|((viewport, _, _), view)| {
            (*viewport == context.viewport_id()
                && view.focus_id == Some(id)
                && view.focus_frame.is_some_and(|previous| {
                    previous == frame || previous.checked_add(1) == Some(frame)
                }))
            .then_some((
                KeymapContext {
                    terminal: true,
                    editor: false,
                },
                !view.preedit.is_empty(),
            ))
        })
    }

    pub(crate) fn route_window_keys(
        &mut self,
        context: &egui::Context,
        scope: KeymapContext,
        composing: bool,
        overrides: Option<&str>,
        actions: &mut Vec<String>,
        has_focused_shell: bool,
    ) -> AppResult<()> {
        let events = context.input_mut(|input| std::mem::take(&mut input.events));
        let mut next = 0;
        let mut remaining = Vec::new();
        let mut failure = None;
        for event in events {
            let index = crate::keymap::event_index(context, &event, &mut next);
            match self.route_keymap(
                crate::keymap::Route {
                    context,
                    event: &event,
                    index,
                    scope,
                    composing,
                    overrides,
                },
                actions,
                has_focused_shell,
            ) {
                Ok(true) => (),
                Ok(false) => remaining.push(event),
                Err(error) => failure = Some(error),
            }
        }
        context.input_mut(|input| input.events = remaining);
        failure.map_or(Ok(()), Err)
    }

    pub(crate) fn route_keymap(
        &mut self,
        request: crate::keymap::Route<'_>,
        actions: &mut Vec<String>,
        has_focused_shell: bool,
    ) -> AppResult<bool> {
        self.keymaps.route(request, |decision| {
            application_keymap_decision(
                decision,
                actions,
                has_focused_shell,
                true,
                &self.command_context,
            )
        })
    }

    pub(crate) fn set_command_context(&mut self, context: CommandContext) {
        self.command_context = context;
    }

    pub(crate) fn has_keyboard_focus(&self, context: &egui::Context) -> bool {
        let id = context.memory(|memory| memory.focused());
        self.views.iter().any(|((viewport, _, _), view)| {
            *viewport == context.viewport_id()
                && view.focused
                && view.focus_id == id
                && view.focus_frame == Some(context.cumulative_frame_nr())
        })
    }

    pub(crate) fn clear_keymap_chord(&mut self, context: &egui::Context) {
        self.keymaps.clear_chord(context.viewport_id());
    }

    pub(crate) fn chord_status(
        &self,
        context: &egui::Context,
        now: Instant,
    ) -> crate::keymap::ChordStatus {
        self.keymaps
            .chord_status(context, now, cfg!(target_os = "macos"))
    }

    pub fn show_with_keymap(
        &mut self,
        ui: &mut Ui,
        request: Request<'_>,
        mut handler: impl FnMut(&str) -> bool,
    ) -> AppResult<egui::Response> {
        let Request {
            pane,
            tab,
            session_id,
            hub,
            services,
            appearance,
            locale,
            request_focus,
            commands,
        } = request;
        {
            let mut palette = self
                .palette
                .lock()
                .map_err(|_| AppError::Internal("native terminal palette lock poisoned".into()))?;
            if palette.is_none() {
                *palette = Some(appearance.clone());
            }
        }
        self.flush_inputs(hub, services, ui.ctx())?;
        let id = ui.make_persistent_id(("native-terminal", pane, tab));
        let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        let response = ui.interact(rect, id, Sense::click_and_drag());
        ui.allocate_rect(rect, Sense::hover());
        let pointer_focus = (response.clicked() || response.drag_started())
            && ui
                .ctx()
                .keyboard_input_route(id)
                .is_none_or(|(_, (focused, _))| focused.unwrap_or(true));
        if ui.is_enabled() && (request_focus || pointer_focus) {
            response.request_focus();
        }
        let cell = ui.fonts_mut(|fonts| {
            vec2(
                fonts.glyph_width(&appearance.font, 'M'),
                fonts.row_height(&appearance.font) * LINE_HEIGHT,
            )
        });
        let has_scrollback = services.state.settings.read().terminal_scrollback != 0;
        let Some(size) = measured(rect, cell, has_scrollback) else {
            return Ok(response);
        };
        let window = WindowSize {
            num_cols: size.columns,
            num_lines: size.rows,
            cell_width: cell.x.round().clamp(1.0, f32::from(u16::MAX)) as u16,
            cell_height: cell.y.round().clamp(1.0, f32::from(u16::MAX)) as u16,
        };
        let restart_label = crate::presentation::message(locale, "terminal.restart", &[]);
        let (starting_ownership, (starting_focus, starting_lost_after_events)) =
            ui.ctx().keyboard_input_route(id).unwrap_or_default();
        let starting_focused = starting_focus.unwrap_or_else(|| response.has_focus());
        let has_starting_input = starting_ownership
            .iter()
            .any(|(owned, _)| *owned == Some(true));
        let Some(session) = hub.get(session_id) else {
            if !ui.is_enabled() {
                return Ok(response);
            }
            if let Some(error) = self.failed.get(tab) {
                let message = crate::toast::describe_error(locale, error);
                if !status(ui, rect, id, appearance, &message, &restart_label) {
                    return Ok(response);
                }
                self.failed.remove(tab);
            }
            if self.attaching.insert(tab.clone()) {
                let geometry = Arc::new(Mutex::new(window));
                self.attaching_geometry
                    .insert(tab.clone(), geometry.clone());
                commands.push(HostCommand::AttachTerminal {
                    tab: tab.clone(),
                    size,
                    ports: effect_ports(ui, appearance, geometry, self.palette.clone()),
                });
            }
            if self.attaching.contains(tab) && (starting_focused || has_starting_input) {
                self.focus_view(
                    (ui.ctx().viewport_id(), pane.clone(), tab.clone()),
                    id,
                    ui.ctx(),
                    starting_focused && ui.input(|input| input.focused),
                    hub,
                    services,
                );
                let view = self.view_mut(ui, pane, tab);
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
                    let (owned, lost) = starting_ownership.get(index).copied().unwrap_or_default();
                    if lost {
                        view.preedit.clear();
                    }
                    if !owned.unwrap_or(starting_focused) || !buffer_starting_input(view, &event) {
                        remaining.push(event);
                    }
                }
                ui.input_mut(|input| input.events = remaining);
                if starting_lost_after_events {
                    view.preedit.clear();
                }
                if !starting_focused {
                    view.focused = false;
                    view.preedit.clear();
                }
            }
            return Ok(response);
        };
        let phase = session.snapshot(|snapshot| snapshot.phase)?;
        if self.attaching.contains(tab) {
            if (starting_focused || has_starting_input) && ui.is_enabled() {
                self.focus_view(
                    (ui.ctx().viewport_id(), pane.clone(), tab.clone()),
                    id,
                    ui.ctx(),
                    starting_focused && ui.input(|input| input.focused),
                    hub,
                    services,
                );
                let view = self.view_mut(ui, pane, tab);
                let events = ui.input_mut(|input| std::mem::take(&mut input.events));
                let mut remaining = Vec::new();
                for (index, event) in events.into_iter().enumerate() {
                    let (owned, lost) = starting_ownership.get(index).copied().unwrap_or_default();
                    if lost {
                        view.preedit.clear();
                    }
                    if !owned.unwrap_or(starting_focused) || !buffer_starting_input(view, &event) {
                        remaining.push(event);
                    }
                }
                ui.input_mut(|input| input.events = remaining);
                if starting_lost_after_events {
                    view.preedit.clear();
                }
                if !starting_focused {
                    change_focus(view, hub, services, false);
                    view.preedit.clear();
                }
            }
            return Ok(response);
        }
        session.configure_command_colors(appearance.command_colors)?;
        if let Some(message) = ended_message(locale, self.failed.get(tab), phase) {
            if status(ui, rect, id, appearance, &message, &restart_label)
                && self.attaching.insert(tab.clone())
            {
                self.failed.remove(tab);
                for ((_, _, owner), view) in &mut self.views {
                    if owner == tab {
                        *view = View::default();
                    }
                }
                let geometry = Arc::new(Mutex::new(window));
                self.attaching_geometry
                    .insert(tab.clone(), geometry.clone());
                commands.push(HostCommand::RestartTerminal {
                    tab: tab.clone(),
                    size,
                    ports: effect_ports(ui, appearance, geometry, self.palette.clone()),
                });
            }
            return Ok(response);
        }
        let (running, current_size, mode) = session.snapshot(|snapshot| {
            let grid = snapshot.core.grid()?;
            Ok::<_, AppError>((
                snapshot.phase == Phase::Running,
                Size {
                    columns: grid.columns() as u16,
                    rows: grid.screen_lines() as u16,
                },
                snapshot.core.mode()?,
            ))
        })??;
        let owns_size =
            response.has_focus() || !self.is_shown_elsewhere(ui.ctx(), pane, tab, session_id);
        if running && owns_size && ui.is_enabled() {
            if let Some(geometry) = self.geometry.get(session_id) {
                *geometry.lock().map_err(|_| {
                    AppError::Internal("native terminal geometry lock poisoned".into())
                })? = window;
            }
            if size != current_size && self.resizing.insert(session_id.into()) {
                commands.push(HostCommand::ResizeTerminal {
                    session: session_id.into(),
                    size,
                });
            }
        }
        let outbox = self.outboxes.entry(session_id.into()).or_default().clone();
        let view = self.view_mut(ui, pane, tab);
        if view.session != session_id {
            let pending = std::mem::take(&mut view.pending);
            *view = View {
                session: session_id.into(),
                pending,
                is_wheel_routed: view.is_wheel_routed,
                ..Default::default()
            };
        }
        view.outbox = outbox;
        session.snapshot(|snapshot| reconcile_view(snapshot.core, view))??;
        view.register_mouse(ui, &response, mode, running);
        if running && ui.is_enabled() && response.enabled() {
            ui.ctx().register_pointer_keyboard_focus(id);
            ui.ctx()
                .register_context_menu_keyboard_focus(id, id.with("context-menu-keyboard-owner"));
        }
        view.mouse_geometry = Some(MouseGeometry::new(ui, &response, cell));
        {
            let outbox = view.outbox.lock().map_err(|_| {
                AppError::Internal("native terminal input queue lock poisoned".into())
            })?;
            if outbox.is_pending() {
                ui.ctx().request_repaint_after(RECEIPT_POLL);
            }
        }
        let (_, (routed_focus, _)) = ui.ctx().keyboard_input_route(id).unwrap_or_default();
        let focused = routed_focus.unwrap_or_else(|| response.has_focus())
            && ui.is_enabled()
            && ui.input(|input| input.focused);
        self.focus_view(
            (ui.ctx().viewport_id(), pane.clone(), tab.clone()),
            id,
            ui.ctx(),
            focused,
            hub,
            services,
        );
        let menu_was_open =
            self.view_mut(ui, pane, tab).menu.is_some() || response.context_menu_opened();
        let menu_open = if !menu_was_open && ui.is_enabled() && running {
            let transform = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            let global_rect = transform * response.rect;
            let events = ui.input(|input| input.raw.events.clone());
            events.iter().enumerate().find_map(|(index, event)| {
                let Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    ..
                } = event
                else {
                    return None;
                };
                (global_rect.contains(*pos)
                    && ui.ctx().keyboard_focus_request_at(index) == Some(id.accesskit_id()))
                .then_some((index, *pos))
            })
        } else {
            None
        };
        let menu_open_index = menu_open.map(|(index, _)| index);
        let menu_active = menu_was_open || menu_open.is_some();
        let (ownership, (_, lost_after_events)) =
            ui.ctx().keyboard_input_route(id).unwrap_or_default();
        let has_owned_input = ownership.iter().any(|(owned, _)| *owned == Some(true));
        if (focused || has_owned_input)
            && ui.is_enabled()
            && running
            && (!menu_active || menu_open_index.is_some())
        {
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
            let frame = ui.ctx().cumulative_frame_nr();
            let events = ui.input_mut(|input| std::mem::take(&mut input.events));
            let mut next = 0;
            let mut remaining = Vec::new();
            for (index, event) in events.into_iter().enumerate() {
                let order = EventOrder {
                    frame,
                    index: crate::keymap::event_index(ui.ctx(), &event, &mut next),
                };
                let (owned, lost) = ownership.get(index).copied().unwrap_or_default();
                if lost {
                    self.view_mut(ui, pane, tab).preedit.clear();
                }
                if !owned.unwrap_or(focused)
                    || (menu_active && menu_open_index.is_none_or(|opening| order.index >= opening))
                {
                    remaining.push(event);
                    continue;
                }
                self.flush_captured_pointer(
                    hub,
                    services,
                    ui.ctx(),
                    Some((ui.ctx().viewport_id(), order)),
                )?;
                let view = self.view_mut(ui, pane, tab);
                dispatch_pointer_input(view, ui, &response, &session, services, cell, Some(order));
                let composing = !view.preedit.is_empty();
                let mut decision = KeymapDecision::None;
                let mut jump = false;
                let claimed = self.keymaps.route(
                    crate::keymap::Route {
                        context: ui.ctx(),
                        event: &event,
                        index: order.index,
                        scope: KeymapContext {
                            terminal: true,
                            editor: false,
                        },
                        composing,
                        overrides: services.state.settings.read().keymap_overrides.as_deref(),
                    },
                    |result| {
                        decision = result.clone();
                        match result {
                            KeymapDecision::Dispatch(id) | KeymapDecision::ResolveChord(id) => {
                                jump = matches!(
                                    id.as_str(),
                                    "terminal-jump-to-previous-command"
                                        | "terminal-jump-to-next-command"
                                );
                                jump || handler(id)
                                    || matches!(result, KeymapDecision::ResolveChord(_))
                            }
                            KeymapDecision::EnterChord | KeymapDecision::NoMatch => true,
                            _ => false,
                        }
                    },
                )?;
                let view = self.view_mut(ui, pane, tab);
                if view.ordered_focus_frame == Some(frame) {
                    view.input_order = Some(WireOrder {
                        viewport: ui.ctx().viewport_id(),
                        event: order,
                        phase: WirePhase::Input,
                    });
                }
                if (claimed && !jump)
                    || handle_input(
                        view,
                        ui,
                        &session,
                        services,
                        &event,
                        current_size.rows,
                        &decision,
                    )
                {
                    view.blink.started = ui.input(|input| input.time);
                } else {
                    remaining.push(event);
                }
                self.view_mut(ui, pane, tab).input_order = None;
            }
            ui.input_mut(|input| input.events = remaining);
        }
        terminal_context_menu(
            ui,
            self.view_mut(ui, pane, tab),
            MenuRequest {
                response: &response,
                opening: menu_open.map(|(_, position)| position),
                session: &session,
                services,
                locale,
                commands,
                pane,
                tab,
            },
        )?;
        if lost_after_events {
            self.view_mut(ui, pane, tab).preedit.clear();
        }
        if !focused {
            let view = self.view_mut(ui, pane, tab);
            change_focus(view, hub, services, false);
            view.preedit.clear();
        }
        self.flush_captured_pointer(
            hub,
            services,
            ui.ctx(),
            Some((
                ui.ctx().viewport_id(),
                EventOrder {
                    frame: ui.ctx().cumulative_frame_nr(),
                    index: usize::MAX,
                },
            )),
        )?;
        let view = self.view_mut(ui, pane, tab);
        if running && focused {
            dispatch_pointer_input(view, ui, &response, &session, services, cell, None);
        } else if !running {
            view.mouse.packets.clear();
            view.captured_wheel.packets.clear();
        } else {
            dispatch_wheel_input(view, ui, &response, &session, services, cell, None);
        }
        view.processed_frame = Some(ui.ctx().cumulative_frame_nr());
        view.processed_pass = Some(ui.ctx().cumulative_pass_nr());
        if view
            .outbox
            .lock()
            .map_err(|_| AppError::Internal("native terminal input queue lock poisoned".into()))?
            .is_pending()
            || !view.mouse.packets.is_empty()
        {
            ui.ctx().request_repaint_after(RECEIPT_POLL);
        }
        match session
            .snapshot(|snapshot| select_pointer(ui, &response, snapshot.core, rect, cell, view))
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) | Err(error) => view.error = Some(error.to_string()),
        }
        session.snapshot(|snapshot| {
            paint(ui, rect, cell, snapshot.core, view, appearance, focused)?;
            let result = show_external_link(
                ui,
                &response,
                snapshot.core,
                view,
                ExternalLinkRequest {
                    cell,
                    appearance,
                    session: &session,
                    services,
                    pane,
                    tab,
                    commands,
                    menu_active,
                },
            );
            if let Err(error) = result {
                view.error = Some(error.to_string());
            }
            Ok::<_, AppError>(())
        })??;
        let queue_error = view
            .outbox
            .lock()
            .map_err(|_| AppError::Internal("native terminal input queue lock poisoned".into()))?
            .error
            .clone();
        let failure = view.error.clone().or(queue_error);
        if failure != view.logged_error {
            if let Some(error) = &failure {
                log::warn!("native terminal view failed: {error}");
            }
            view.logged_error = failure;
        }
        Ok(response)
    }
}

fn change_focus(view: &mut View, hub: &Hub, services: &AppServices, focused: bool) {
    if view.focused == focused {
        return;
    }
    view.focused = focused;
    view.preedit.clear();
    let Some(session) = hub.get(&view.session) else {
        return;
    };
    match session.snapshot(|snapshot| snapshot.phase == Phase::Running) {
        Ok(true) => {
            submit_input(view, &session, services, NativeInput::Focus(focused));
        }
        Ok(false) => {}
        Err(error) => view.error = Some(error.to_string()),
    }
}

fn is_wire_barrier(event: &Event) -> bool {
    matches!(
        event,
        Event::Touch { .. }
            | Event::WindowFocused(true)
            | Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                ..
            })
    )
}

fn is_navigation(event: &Event) -> bool {
    matches!(
        event,
        Event::Key {
            key: egui::Key::Tab
                | egui::Key::ArrowUp
                | egui::Key::ArrowDown
                | egui::Key::ArrowLeft
                | egui::Key::ArrowRight,
            pressed: true,
            ..
        }
    )
}

fn stage_focus(
    view: &mut View,
    hub: &Hub,
    services: &AppServices,
    focused: bool,
    order: WireOrder,
) {
    if view.focused == focused {
        return;
    }
    view.focused = focused;
    let Some(session) = hub.get(&view.session) else {
        return;
    };
    view.input_order = Some(order);
    submit_input(view, &session, services, NativeInput::Focus(focused));
    view.input_order = None;
}

fn dispatch_pointer_input(
    view: &mut View,
    ui: &Ui,
    response: &egui::Response,
    session: &Session,
    services: &AppServices,
    cell: egui::Vec2,
    through: Option<EventOrder>,
) {
    match session
        .snapshot(|snapshot| process_mouse(ui, response, snapshot.core, cell, view, through))
    {
        Ok(Ok(inputs)) => {
            for input in inputs {
                submit_input(view, session, services, NativeInput::Mouse(input));
            }
        }
        Ok(Err(error)) | Err(error) => view.error = Some(error.to_string()),
    }
    dispatch_wheel_input(view, ui, response, session, services, cell, through);
}

fn dispatch_wheel_input(
    view: &mut View,
    ui: &Ui,
    response: &egui::Response,
    session: &Session,
    services: &AppServices,
    cell: egui::Vec2,
    through: Option<EventOrder>,
) {
    match session
        .snapshot(|snapshot| scroll_wheel(ui, response, snapshot.core, cell, view, through))
    {
        Ok(Ok(keys)) => {
            for key in keys {
                submit_input(
                    view,
                    session,
                    services,
                    NativeInput::Key {
                        key,
                        modifiers: Default::default(),
                    },
                );
            }
        }
        Ok(Err(error)) | Err(error) => view.error = Some(error.to_string()),
    }
}

fn process_mouse(
    ui: &Ui,
    response: &egui::Response,
    core: &TerminalCore,
    cell: egui::Vec2,
    view: &mut View,
    through: Option<EventOrder>,
) -> AppResult<Vec<MouseInput>> {
    view.mouse_geometry = Some(MouseGeometry::new(ui, response, cell));
    if !ui.is_enabled() || !response.enabled() || !ui.input(|input| input.focused) {
        view.mouse.packets.clear();
        view.mouse.buttons.fill(false);
        view.mouse.forced = false;
        return Ok(Vec::new());
    }
    let frame = ui.ctx().cumulative_frame_nr();
    if view.mouse.frame != frame {
        view.mouse.packets.clear();
        return Ok(Vec::new());
    }
    let inputs = take_mouse(core, view, through, INPUT_RECEIPTS)?;
    if !view.mouse.packets.is_empty() {
        ui.ctx().request_repaint_after(RECEIPT_POLL);
    }
    Ok(inputs)
}

fn take_mouse(
    core: &TerminalCore,
    view: &mut View,
    through: Option<EventOrder>,
    limit: usize,
) -> AppResult<Vec<MouseInput>> {
    let mode = mouse_mode(core.mode()?);
    if !mode.intersects(Mode::MOUSE_MODE) {
        view.mouse = Mouse::default();
        return Ok(Vec::new());
    }
    let grid = core.grid()?;
    let capacity = view.input_capacity()?.min(limit);
    let mut inputs = Vec::new();
    while inputs.len() < capacity
        && view
            .mouse
            .packets
            .front()
            .is_some_and(|packet| through.is_none_or(|through| packet.order <= through))
    {
        let Some(packet) = view.mouse.packets.pop_front() else {
            break;
        };
        if packet.mode != mode {
            continue;
        }
        let Some(geometry) = packet.geometry else {
            return Err(AppError::InvalidArgument(
                "native terminal captured mouse geometry unavailable".into(),
            ));
        };
        let cell = geometry.cell;
        let action = if let Some(action) = packet.action {
            if matches!(action, MouseAction::Press(_))
                && !geometry.global_rect.contains(packet.position)
            {
                continue;
            }
            action
        } else {
            let amount = view.wheel.consume(core, cell.y, &packet.event)?;
            if amount == 0.0 {
                continue;
            }
            if amount > 0.0 {
                MouseAction::WheelUp
            } else {
                MouseAction::WheelDown
            }
        };
        let position = geometry
            .from_global
            .map_or(packet.position, |transform| transform * packet.position);
        if !position.is_finite() || !cell.is_finite() || cell.x <= 0.0 || cell.y <= 0.0 {
            return Err(AppError::InvalidArgument(
                "invalid native terminal mouse geometry".into(),
            ));
        }
        let x = (position.x - geometry.rect.left())
            .clamp(0.0, (grid.columns() as f32 * cell.x - 1.0).max(0.0));
        let y = (position.y - geometry.rect.top())
            .clamp(0.0, (grid.screen_lines() as f32 * cell.y - 1.0).max(0.0));
        let pixels = mode
            .contains(Mode::SGR_PIXEL_MOUSE)
            .then_some((x.floor() as u32, y.floor() as u32));
        let input = MouseInput {
            column: ((x / cell.x).floor() as usize).min(grid.columns() - 1) as u16,
            row: ((y / cell.y).floor() as usize).min(grid.screen_lines() - 1) as u16,
            pixels,
            action,
            modifiers: Modifiers {
                shift: packet.modifiers.shift,
                alt: packet.modifiers.alt,
                control: packet.modifiers.ctrl,
                command: packet.modifiers.mac_cmd,
            },
        };
        if matches!(action, MouseAction::Move(_)) && view.mouse.last == Some(input) {
            continue;
        }
        match core.encode_input(NativeInput::Mouse(input), INPUT_BYTES) {
            Ok(InputAction::Write(_)) => {
                inputs.push(input);
            }
            Ok(_) => {}
            Err(error) => {
                return Err(AppError::InvalidArgument(format!(
                    "native terminal mouse rejected: {error:?}"
                )));
            }
        }
        view.mouse.last = Some(input);
    }
    Ok(inputs)
}

fn take_wheel(
    core: &TerminalCore,
    view: &mut View,
    through: Option<EventOrder>,
    limit: usize,
    fallback: Option<MouseGeometry>,
) -> AppResult<Vec<Key>> {
    if tracks_wheel(core.mode()?) {
        view.captured_wheel.packets.clear();
        return Ok(Vec::new());
    }
    let capacity = view.input_capacity()?.min(limit);
    let mut keys = Vec::new();
    let mut processed = 0;
    while processed < limit
        && (core.has_scrollback()? || keys.len() < capacity)
        && view
            .captured_wheel
            .packets
            .front()
            .is_some_and(|packet| through.is_none_or(|through| packet.order <= through))
    {
        let packet = view.captured_wheel.packets.pop_front().unwrap();
        processed += 1;
        let geometry = packet.geometry.or(fallback).ok_or_else(|| {
            AppError::InvalidArgument("native terminal captured wheel geometry unavailable".into())
        })?;
        if geometry.global_rect.contains(packet.position) {
            apply_wheel(
                core,
                geometry.cell,
                view,
                &packet.event,
                &mut keys,
                capacity,
            )?;
        }
    }
    Ok(keys)
}

fn scroll_wheel(
    ui: &Ui,
    response: &egui::Response,
    core: &TerminalCore,
    cell: egui::Vec2,
    view: &mut View,
    through: Option<EventOrder>,
) -> AppResult<Vec<Key>> {
    if !ui.is_enabled() || tracks_wheel(core.mode()?) {
        view.captured_wheel.packets.clear();
        return Ok(Vec::new());
    }
    reconcile_view(core, view)?;
    let capacity = view.input_capacity()?;
    let mut keys = take_wheel(
        core,
        view,
        through,
        INPUT_RECEIPTS,
        Some(MouseGeometry::new(ui, response, cell)),
    )?;
    if !view.captured_wheel.packets.is_empty() {
        ui.ctx().request_repaint_after(RECEIPT_POLL);
    }
    if view.is_wheel_routed || !response.hovered() {
        return Ok(keys);
    }
    ui.input_mut(|input| {
        let mut error = None;
        input.events.retain(|event| {
            if error.is_some() {
                return true;
            }
            let Event::MouseWheel { delta, .. } = event else {
                return true;
            };
            if delta.y == 0.0 || delta.x.abs() > delta.y.abs() {
                return true;
            }
            if let Err(failure) = apply_wheel(core, cell, view, event, &mut keys, capacity) {
                error = Some(failure);
                return true;
            }
            false
        });
        input.smooth_scroll_delta.y = 0.0;
        match error {
            Some(error) => Err(error),
            None => Ok(keys),
        }
    })
}

fn apply_wheel(
    core: &TerminalCore,
    cell: egui::Vec2,
    view: &mut View,
    event: &Event,
    keys: &mut Vec<Key>,
    capacity: usize,
) -> AppResult<()> {
    if let Some(key) = view.wheel.apply(core, &mut view.offset, cell.y, event)? {
        if keys.len() >= capacity {
            return Err(AppError::InvalidArgument(
                "native terminal wheel input receipt budget exceeded".into(),
            ));
        }
        keys.push(key);
    }
    Ok(())
}

fn measured(rect: Rect, cell: egui::Vec2, has_scrollback: bool) -> Option<Size> {
    if !rect.is_finite() || !cell.is_finite() || cell.x <= 0.0 || cell.y <= 0.0 {
        return None;
    }
    let scrollbar_width = if has_scrollback {
        crate::terminal_ruler::WIDTH as f32
    } else {
        0.0
    };
    let columns = ((rect.width() - scrollbar_width) / cell.x).floor();
    let rows = (rect.height() / cell.y).floor();
    if columns < f32::from(MIN_COLUMNS)
        || rows < f32::from(MIN_ROWS)
        || columns > f32::from(u16::MAX)
        || rows > f32::from(u16::MAX)
    {
        return None;
    }
    Some(Size {
        columns: columns as u16,
        rows: rows as u16,
    })
}

fn submit_input(
    view: &mut View,
    session: &Session,
    services: &AppServices,
    input: NativeInput<'_>,
) -> Option<InputAction> {
    let is_focus = matches!(&input, NativeInput::Focus(_));
    let result = view
        .outbox
        .lock()
        .map_err(|_| AppError::Internal("native terminal input queue lock poisoned".into()))
        .and_then(|mut outbox| match view.input_order {
            Some(order) => outbox.stage(session, input, order),
            None => outbox.submit(session, services, input),
        });
    match result {
        Ok(None) => {
            if !is_focus {
                view.offset = 0;
                view.error = None;
            }
            None
        }
        Ok(action) => action,
        Err(error) => {
            view.error = Some(error.to_string());
            None
        }
    }
}

enum InputEvent<'event> {
    Send(NativeInput<'event>),
    Handled,
    Unhandled,
}

fn input_event<'event>(view: &mut View, event: &'event Event) -> InputEvent<'event> {
    let input = match event {
        Event::Ime(ImeEvent::Preedit { text, .. }) => {
            if text.len() > INPUT_BYTES {
                view.error = Some("native terminal preedit budget exceeded".into());
                view.preedit.clear();
            } else {
                view.preedit.clone_from(text);
            }
            return InputEvent::Handled;
        }
        Event::Ime(ImeEvent::Commit(text)) => {
            view.preedit.clear();
            NativeInput::CommittedText(text)
        }
        Event::Text(text) if view.preedit.is_empty() => NativeInput::CommittedText(text),
        Event::Text(_) => return InputEvent::Handled,
        Event::Paste(text) => NativeInput::Paste(text),
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } if view.preedit.is_empty() => {
            let Some(key) = key_input(*key, *modifiers) else {
                return InputEvent::Unhandled;
            };
            NativeInput::Key {
                key,
                modifiers: Modifiers {
                    shift: modifiers.shift,
                    alt: modifiers.alt,
                    control: modifiers.ctrl,
                    command: modifiers.mac_cmd,
                },
            }
        }
        Event::Key { .. } if !view.preedit.is_empty() => return InputEvent::Handled,
        _ => return InputEvent::Unhandled,
    };
    InputEvent::Send(input)
}

fn buffer_starting_input(view: &mut View, event: &Event) -> bool {
    let input = match input_event(view, event) {
        InputEvent::Send(input) => input,
        InputEvent::Handled => return true,
        InputEvent::Unhandled => return false,
    };
    match taide_native_terminal::input::encode_input(Mode::default(), input, INPUT_BYTES) {
        Ok(InputAction::Write(bytes)) => match std::str::from_utf8(&bytes) {
            Ok(text) => {
                for unit in text.encode_utf16() {
                    if view.pending.len() == PENDING_INPUT_UNITS {
                        view.pending.pop_front();
                    }
                    view.pending.push_back(unit);
                }
            }
            Err(error) => view.error = Some(error.to_string()),
        },
        Ok(_) => {}
        Err(error) => {
            view.error = Some(format!("native terminal pending input rejected: {error:?}"))
        }
    }
    true
}

fn handle_input(
    view: &mut View,
    ui: &Ui,
    session: &Session,
    services: &AppServices,
    event: &Event,
    rows: u16,
    decision: &KeymapDecision,
) -> bool {
    if let KeymapDecision::Dispatch(id) | KeymapDecision::ResolveChord(id) = decision
        && matches!(
            id.as_str(),
            "terminal-jump-to-previous-command" | "terminal-jump-to-next-command"
        )
    {
        match session.snapshot(|snapshot| {
            let history = snapshot.core.grid()?.history_size();
            let current = history.saturating_sub(view.offset);
            let lines = snapshot.core.command_start_lines()?;
            let target = if id == "terminal-jump-to-previous-command" {
                lines.into_iter().filter(|line| *line < current).max()
            } else {
                lines.into_iter().filter(|line| *line > current).min()
            };
            Ok::<_, AppError>(target.map(|line| history.saturating_sub(line.min(history))))
        }) {
            Ok(Ok(Some(offset))) => view.offset = offset,
            Ok(Ok(None)) => (),
            Ok(Err(error)) | Err(error) => view.error = Some(error.to_string()),
        }
        return true;
    }
    if matches!(
        decision,
        KeymapDecision::EnterChord | KeymapDecision::NoMatch | KeymapDecision::ResolveChord(_)
    ) {
        return true;
    }
    if matches!(event, Event::Copy) {
        match session.snapshot(|snapshot| {
            reconcile_view(snapshot.core, view)?;
            let Some(selection) = &view.selection else {
                return Ok(None);
            };
            snapshot.core.selection_text(selection, INPUT_BYTES)
        }) {
            Ok(Ok(Some(text))) => ui.ctx().copy_text(text),
            Ok(Ok(None)) => {}
            Ok(Err(error)) | Err(error) => view.error = Some(error.to_string()),
        }
        return true;
    }
    let input = match input_event(view, event) {
        InputEvent::Send(input) => input,
        InputEvent::Handled => return true,
        InputEvent::Unhandled => return false,
    };
    match submit_input(view, session, services, input) {
        Some(InputAction::PageUp) => view.offset = view.offset.saturating_add(usize::from(rows)),
        Some(InputAction::PageDown) => view.offset = view.offset.saturating_sub(usize::from(rows)),
        Some(InputAction::SelectAll) => {
            match session.snapshot(|snapshot| {
                reconcile_view(snapshot.core, view)?;
                full_selection(snapshot.core)
            }) {
                Ok(Ok(selection)) => {
                    view.selection = Some(selection);
                    view.is_all_selected = true;
                    view.gesture = Gesture::default();
                }
                Ok(Err(error)) | Err(error) => view.error = Some(error.to_string()),
            }
        }
        _ => {}
    }
    true
}

fn effect_ports(
    ui: &Ui,
    appearance: &Appearance,
    geometry: Arc<Mutex<WindowSize>>,
    palette: Arc<Mutex<Option<Appearance>>>,
) -> EffectPorts {
    let context = ui.ctx().clone();
    EffectPorts {
        command_colors: appearance.command_colors,
        updated: Arc::new(move || context.request_repaint()),
        color: Arc::new(move |index| {
            palette
                .lock()
                .map_err(|_| AppError::Internal("native terminal palette lock poisoned".into()))?
                .as_ref()
                .ok_or_else(|| AppError::Internal("native terminal palette is unavailable".into()))?
                .indexed(index)
        }),
        geometry: Arc::new(move || {
            geometry
                .lock()
                .map(|geometry| *geometry)
                .map_err(|_| AppError::Internal("native terminal geometry lock poisoned".into()))
        }),
        event: Arc::new(|_| Ok(())),
        stream: Arc::new(|_| Ok(())),
    }
}

fn ended_message(
    locale: &ResolvedLocale,
    attach_failure: Option<&AppError>,
    phase: Phase,
) -> Option<String> {
    if let Some(error) = attach_failure {
        return Some(crate::toast::describe_error(locale, error));
    }
    let exit_code = match phase {
        Phase::Exited(code) => code,
        Phase::Failed(_) => None,
        Phase::Running | Phase::Draining(_) => return None,
    };
    let mut message = crate::presentation::message(locale, "terminal.processExited", &[]);
    if let Some(code) = exit_code {
        message.push_str(&format!(" ({code})"));
    }
    Some(message)
}

fn status(
    ui: &mut Ui,
    rect: Rect,
    id: egui::Id,
    appearance: &Appearance,
    message: &str,
    restart: &str,
) -> bool {
    ui.painter()
        .rect_filled(rect, 0.0, color32(appearance.background));
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .id_salt(id.with("status")),
        |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space((rect.height() - STATUS_HEIGHT).max(0.0) / 2.0);
                ui.colored_label(color32(appearance.foreground), message);
                ui.button(restart).clicked()
            })
            .inner
        },
    )
    .inner
}

fn key_input(key: egui::Key, modifiers: egui::Modifiers) -> Option<Key> {
    let key = match key {
        egui::Key::ArrowUp => Key::ArrowUp,
        egui::Key::ArrowDown => Key::ArrowDown,
        egui::Key::ArrowLeft => Key::ArrowLeft,
        egui::Key::ArrowRight => Key::ArrowRight,
        egui::Key::Home => Key::Home,
        egui::Key::End => Key::End,
        egui::Key::Insert => Key::Insert,
        egui::Key::Delete => Key::Delete,
        egui::Key::PageUp => Key::PageUp,
        egui::Key::PageDown => Key::PageDown,
        egui::Key::Tab => Key::Tab,
        egui::Key::Backspace => Key::Backspace,
        egui::Key::Enter => Key::Enter,
        egui::Key::Escape => Key::Escape,
        egui::Key::F1 => Key::Function(1),
        egui::Key::F2 => Key::Function(2),
        egui::Key::F3 => Key::Function(3),
        egui::Key::F4 => Key::Function(4),
        egui::Key::F5 => Key::Function(5),
        egui::Key::F6 => Key::Function(6),
        egui::Key::F7 => Key::Function(7),
        egui::Key::F8 => Key::Function(8),
        egui::Key::F9 => Key::Function(9),
        egui::Key::F10 => Key::Function(10),
        egui::Key::F11 => Key::Function(11),
        egui::Key::F12 => Key::Function(12),
        _ if modifiers.ctrl || modifiers.mac_cmd => {
            let name = key.symbol_or_name().to_lowercase();
            let mut chars = name.chars();
            let character = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            Key::Character(character)
        }
        _ => return None,
    };
    Some(key)
}

fn selection_anchor(
    core: &TerminalCore,
    rect: Rect,
    cell: egui::Vec2,
    offset: usize,
    pointer: egui::Pos2,
) -> AppResult<(Point, Side)> {
    pointer_anchor(core, rect, cell, offset, pointer, true)
}

fn pointer_anchor(
    core: &TerminalCore,
    rect: Rect,
    cell: egui::Vec2,
    offset: usize,
    pointer: egui::Pos2,
    boundary: bool,
) -> AppResult<(Point, Side)> {
    if !pointer.is_finite() || !cell.is_finite() || cell.x <= 0.0 || cell.y <= 0.0 {
        return Err(AppError::InvalidArgument(
            "invalid native terminal selection geometry".into(),
        ));
    }
    let grid = core.grid()?;
    let row = ((pointer.y - rect.top()) / cell.y)
        .ceil()
        .clamp(1.0, grid.screen_lines() as f32) as i32
        - 1;
    let line = Line(
        (row - offset.min(grid.total_lines().saturating_sub(grid.screen_lines())) as i32)
            .max(grid.topmost_line().0),
    );
    let precision = if boundary { SELECTION_HALF_CELL } else { 1.0 };
    let maximum = if boundary {
        grid.columns()
    } else {
        grid.columns() - 1
    };
    let mut column = (((pointer.x - rect.left()) / cell.x - precision).ceil())
        .clamp(0.0, maximum as f32) as usize;
    if column < grid.columns()
        && grid[Point::new(line, Column(column))]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    {
        if boundary {
            column += 1;
        } else {
            column = column.saturating_sub(1);
        }
    }
    if column == grid.columns() {
        return Ok((Point::new(line, grid.last_column()), Side::Right));
    }
    Ok((Point::new(line, Column(column)), Side::Left))
}

fn full_selection(core: &TerminalCore) -> AppResult<Selection> {
    let grid = core.grid()?;
    let mut selection = Selection::new(
        SelectionType::Simple,
        Point::new(grid.topmost_line(), Column(0)),
        Side::Left,
    );
    selection.update(
        Point::new(grid.bottommost_line(), grid.last_column()),
        Side::Right,
    );
    Ok(selection)
}

fn extend_selection(
    core: &TerminalCore,
    rect: Rect,
    cell: egui::Vec2,
    view: &mut View,
    pointer: egui::Pos2,
    incremental: bool,
) -> AppResult<()> {
    if let (Some(granularity), Some(mut seed)) = (view.gesture.granularity, view.gesture.seed) {
        let columns = core.grid()?.columns();
        let end = if incremental {
            let (point, side) = selection_anchor(core, rect, cell, view.offset, pointer)?;
            Point::new(
                point.line,
                Column(point.column.0 + usize::from(side == Side::Right)),
            )
        } else {
            let (point, _) = pointer_anchor(core, rect, cell, view.offset, pointer, false)?;
            match granularity {
                SelectionGranularity::Line => Point::new(
                    point.line,
                    Column(if point.line < seed.start.line {
                        0
                    } else {
                        columns
                    }),
                ),
                SelectionGranularity::Word => {
                    let target = core.selection_at(point, granularity)?;
                    if point < seed.start {
                        target.start
                    } else {
                        Point::new(target.end.line, target.end.column + 1)
                    }
                }
            }
        };
        seed.end = Some(end);
        view.selection = Some(seed.selection(columns)?);
        view.gesture.seed = Some(seed);
        return Ok(());
    }
    let (point, side) = selection_anchor(core, rect, cell, view.offset, pointer)?;
    if let Some(selection) = &mut view.selection {
        selection.update(point, side);
    }
    Ok(())
}

fn select_pointer(
    ui: &Ui,
    response: &egui::Response,
    core: &TerminalCore,
    rect: Rect,
    cell: egui::Vec2,
    view: &mut View,
) -> AppResult<()> {
    if !ui.is_enabled() {
        view.drag = Drag::default();
        return Ok(());
    }
    reconcile_view(core, view)?;
    let now = ui.input(|input| input.time);
    let is_mouse = core.mode()?.intersects(Mode::MOUSE_MODE);
    let forced_press = ui.input(|input| input.events.iter().any(|event| matches!(event, Event::PointerButton { button: egui::PointerButton::Primary, pressed: true, modifiers, .. } if force_selection(*modifiers))));
    if is_mouse && !view.mouse.forced && !forced_press && view.drag.due.is_none() {
        return Ok(());
    }
    let mut pressed = false;
    if response.is_pointer_button_down_on() || response.clicked() {
        let press = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                } => Some((*pos, *modifiers)),
                _ => None,
            })
        });
        if let Some((position, modifiers)) = press {
            pressed = true;
            if !is_mouse && modifiers.shift && view.selection.is_some() {
                extend_selection(core, rect, cell, view, position, true)?;
            } else {
                let options = ui.ctx().options(|options| options.input_options);
                let count = view.clicks.count(now, position, options);
                let granularity = match count {
                    DOUBLE_CLICK_COUNT => Some(SelectionGranularity::Word),
                    TRIPLE_CLICK_COUNT => Some(SelectionGranularity::Line),
                    _ => None,
                };
                view.gesture = Gesture::default();
                if let Some(granularity) = granularity {
                    let (point, _) =
                        pointer_anchor(core, rect, cell, view.offset, position, false)?;
                    let seed = Seed::new(
                        core.selection_at(point, granularity)?,
                        core.grid()?.columns(),
                    )?;
                    view.selection = Some(seed.selection(core.grid()?.columns())?);
                    view.gesture = Gesture {
                        granularity: Some(granularity),
                        seed: Some(seed),
                    };
                } else {
                    view.is_all_selected = false;
                    let (point, side) = selection_anchor(core, rect, cell, view.offset, position)?;
                    let kind = if modifiers.alt {
                        SelectionType::Block
                    } else {
                        SelectionType::Simple
                    };
                    view.selection = Some(Selection::new(kind, point, side));
                }
            }
            view.drag = Drag {
                due: Some(now + DRAG_SCROLL_INTERVAL.as_secs_f64()),
                last: now,
                ..Default::default()
            };
        }
    }
    let stopped = response.drag_stopped_by(egui::PointerButton::Primary);
    let moved = !pressed
        && view.drag.due.is_some()
        && ui.input(|input| {
            (input.pointer.primary_down() || stopped)
                && input
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::PointerMoved(_)))
        });
    if view.drag.due.is_some()
        && (response.drag_started_by(egui::PointerButton::Primary) || moved)
        && let Some(position) = response.interact_pointer_pos()
    {
        extend_selection(core, rect, cell, view, position, false)?;
        view.drag.has_end = true;
        let canvas = Rect::from_min_size(
            rect.min,
            vec2(rect.width(), core.grid()?.screen_lines() as f32 * cell.y),
        );
        view.drag.amount = drag_amount(canvas, position);
        if view.drag.amount != 0 {
            drag_endpoint(core, view, view.drag.amount, false)?;
        }
    }
    if !ui.input(|input| input.pointer.primary_down()) {
        view.drag = Drag::default();
    } else {
        drag_tick(ui, core, view, now)?;
    }
    if response.clicked()
        && let Some(position) = response.interact_pointer_pos()
    {
        view.clicks.completed(now, position);
    }
    Ok(())
}

struct ExternalLinkRequest<'request> {
    cell: egui::Vec2,
    appearance: &'request Appearance,
    session: &'request Session,
    services: &'request AppServices,
    pane: &'request PaneId,
    tab: &'request TabId,
    commands: &'request mut Vec<HostCommand>,
    menu_active: bool,
}

fn show_external_link(
    ui: &Ui,
    response: &egui::Response,
    core: &TerminalCore,
    view: &mut View,
    request: ExternalLinkRequest<'_>,
) -> AppResult<()> {
    if !ui.is_enabled() || request.menu_active || request.services.state.is_shutting_down() {
        view.link_press = None;
        return Ok(());
    }
    let offset = view.offset;
    let grid = core.grid()?;
    let canvas = Rect::from_min_size(
        response.rect.min,
        vec2(
            grid.columns() as f32 * request.cell.x,
            grid.screen_lines() as f32 * request.cell.y,
        ),
    )
    .intersect(response.rect);
    let source = crate::terminal_tabs::MenuTarget {
        project: request.session.metadata().project_id().clone(),
        pane: request.pane.clone(),
        tab: request.tab.clone(),
        session: view.session.clone(),
    };
    let cwd = Some(request.session.metadata().cwd())
        .filter(|cwd| !cwd.is_empty())
        .or_else(|| request.services.terminal.cwd(&source.session))
        .or_else(|| {
            request
                .services
                .state
                .layouts
                .read()
                .get(&source.project)
                .and_then(|layout| crate::terminal_tabs::menu_tab(layout, &source))
                .and_then(|tab| match &tab.kind {
                    taide_model::layout::TabKind::Terminal { cwd, .. } => cwd.clone(),
                    _ => None,
                })
        })
        .unwrap_or_default();
    let owner = PasteTarget {
        key: (
            ui.ctx().viewport_id(),
            request.pane.clone(),
            request.tab.clone(),
        ),
        session: view.session.clone(),
        lifetime: Arc::downgrade(&view.paste_lifetime),
    };
    let position = ui.input(|input| input.pointer.hover_pos());
    if response.contains_pointer()
        && let Some(position) = position
        && canvas.contains(position)
    {
        let (point, _) =
            pointer_anchor(core, response.rect, request.cell, offset, position, false)?;
        if crate::terminal_links::external_at(core, point)?.is_none() {
            let row = crate::terminal_links::read_row(core, point.line)?;
            if let Some(resolution) = view.file_links.request(&cwd, &row)? {
                request
                    .commands
                    .push(HostCommand::ResolveTerminalFileLinks {
                        owner: owner.clone(),
                        source: source.clone(),
                        request: resolution,
                    });
            }
        }
    }
    let link_at = |position| {
        if !canvas.contains(position) {
            return Ok(None);
        }
        pointer_anchor(core, response.rect, request.cell, offset, position, false)
            .and_then(|(point, _)| view.file_links.at(core, &cwd, point))
    };
    let current = if response.contains_pointer() {
        position.map(link_at).transpose()?.flatten()
    } else {
        None
    };
    let events = ui.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                Event::PointerButton {
                    pos,
                    pressed,
                    modifiers,
                    ..
                } => Some((*pos, *pressed, *modifiers)),
                _ => None,
            })
            .collect::<Vec<_>>()
    });
    for (position, pressed, modifiers) in events {
        let event_link = if response.rect.contains(position)
            && ui
                .ctx()
                .layer_id_at(position)
                .is_none_or(|layer| layer == ui.layer_id())
        {
            link_at(position)?
        } else {
            None
        };
        if pressed {
            view.link_press = event_link;
            continue;
        }
        let previous = view.link_press.take();
        if !crate::terminal_links::should_activate(modifiers, cfg!(target_os = "macos")) {
            continue;
        }
        let Some(link) = event_link else { continue };
        if previous.as_ref() != Some(&link) {
            continue;
        }
        let active = request
            .services
            .state
            .layouts
            .read()
            .get(&source.project)
            .is_some_and(|layout| crate::terminal_tabs::menu_tab(layout, &source).is_some());
        if !active {
            continue;
        }
        let command = match link {
            crate::terminal_file_links::Link::External(link) => HostCommand::OpenTerminalUrl {
                owner: owner.clone(),
                source: source.clone(),
                uri: link.uri,
            },
            crate::terminal_file_links::Link::File(link) => HostCommand::OpenTerminalFile {
                owner: owner.clone(),
                source: source.clone(),
                link,
            },
        };
        request.commands.push(command);
    }
    let Some(link) = current else { return Ok(()) };
    let range = link.range();
    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    let grid = core.grid()?;
    let painter = ui.painter().with_clip_rect(response.rect);
    for row in 0..grid.screen_lines() {
        let line = Line(row as i32 - view.offset as i32);
        if line < range.start.line || line > range.end.line {
            continue;
        }
        let start = if line == range.start.line {
            range.start.column.0
        } else {
            0
        };
        let end = if line == range.end.line {
            range.end.column.0 + 1
        } else {
            grid.columns()
        };
        let origin = response.rect.min
            + vec2(
                start as f32 * request.cell.x,
                (row + 1) as f32 * request.cell.y,
            );
        let color = &grid[Point::new(line, Column(start))];
        painter.line_segment(
            [
                origin,
                origin + vec2((end - start) as f32 * request.cell.x, 0.0),
            ],
            Stroke::new(
                CURSOR_WIDTH,
                request
                    .appearance
                    .resolve(core.content()?.colors, color.fg, color.flags)?,
            ),
        );
    }
    Ok(())
}

fn paint(
    ui: &Ui,
    rect: Rect,
    cell_size: egui::Vec2,
    core: &TerminalCore,
    view: &mut View,
    appearance: &Appearance,
    focused: bool,
) -> AppResult<()> {
    reconcile_view(core, view)?;
    let grid = core.grid()?;
    let content = core.content()?;
    let selection = view
        .selection
        .as_ref()
        .map(|selection| core.selection_range(selection))
        .transpose()?
        .flatten();
    let history = grid.total_lines().saturating_sub(grid.screen_lines());
    view.offset = view.offset.min(history);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(
        rect,
        0.0,
        appearance.resolve(
            content.colors,
            Color::Named(NamedColor::Background),
            CellFlags::empty(),
        )?,
    );
    for background_pass in [true, false] {
        for row in 0..grid
            .screen_lines()
            .min((rect.height() / cell_size.y).ceil() as usize)
        {
            let line = Line(row as i32 - view.offset as i32);
            for column in 0..grid
                .columns()
                .min((rect.width() / cell_size.x).ceil() as usize)
            {
                let point = Point::new(line, Column(column));
                let cell = &grid[point];
                let origin = rect.min + vec2(column as f32 * cell_size.x, row as f32 * cell_size.y);
                let width = if cell.flags.contains(CellFlags::WIDE_CHAR) {
                    cell_size.x * 2.0
                } else {
                    cell_size.x
                };
                let cell_rect = Rect::from_min_size(origin, vec2(width, cell_size.y));
                let mut foreground = appearance.resolve(content.colors, cell.fg, cell.flags)?;
                let mut background =
                    appearance.resolve(content.colors, cell.bg, CellFlags::empty())?;
                if cell.flags.contains(CellFlags::INVERSE) {
                    std::mem::swap(&mut foreground, &mut background);
                }
                if background_pass {
                    painter.rect_filled(cell_rect, 0.0, background);
                    if let Some(selection) = selection
                        && (selection.contains(point)
                            || (cell.flags.contains(CellFlags::WIDE_CHAR)
                                && selection.contains(Point::new(point.line, point.column + 1)))
                            || (cell.flags.contains(CellFlags::WIDE_CHAR_SPACER)
                                && point.column.0 > 0
                                && selection.contains(Point::new(point.line, point.column - 1))))
                    {
                        painter.rect_filled(cell_rect, 0.0, appearance.selection);
                    }
                    continue;
                }
                if cell.flags.intersects(
                    CellFlags::WIDE_CHAR_SPACER
                        | CellFlags::LEADING_WIDE_CHAR_SPACER
                        | CellFlags::HIDDEN,
                ) {
                    continue;
                }
                let mut text = cell.c.to_string();
                if let Some(extra) = cell.zerowidth() {
                    text.extend(extra);
                }
                painter.text(
                    origin,
                    egui::Align2::LEFT_TOP,
                    text,
                    appearance.font.clone(),
                    foreground,
                );
                if cell.flags.intersects(CellFlags::ALL_UNDERLINES) {
                    painter.line_segment(
                        [cell_rect.left_bottom(), cell_rect.right_bottom()],
                        Stroke::new(CURSOR_WIDTH, foreground),
                    );
                }
                if cell.flags.contains(CellFlags::STRIKEOUT) {
                    painter.line_segment(
                        [
                            pos2(cell_rect.left(), cell_rect.center().y),
                            pos2(cell_rect.right(), cell_rect.center().y),
                        ],
                        Stroke::new(CURSOR_WIDTH, foreground),
                    );
                }
            }
        }
    }
    let first = Line(-(view.offset as i32));
    let last = Line(first.0 + grid.screen_lines() as i32);
    for decoration in core.command_decorations(first..last)? {
        let origin = rect.min + vec2(0.0, (decoration.line.0 - first.0) as f32 * cell_size.y);
        painter.rect_filled(
            Rect::from_min_size(origin, vec2(COMMAND_GUTTER_WIDTH, cell_size.y)),
            0.0,
            color32(decoration.color),
        );
    }
    let cursor = content.cursor;
    if view.offset == 0 {
        let (visible, next_blink) = view.blink.visible(
            ui.input(|input| input.time),
            cursor.point,
            core.cursor_style()?,
            focused,
        );
        if cursor.shape != CursorShape::Hidden
            && let Some(delay) = next_blink
        {
            ui.ctx().request_repaint_after(delay);
        }
        let origin = rect.min
            + vec2(
                cursor.point.column.0 as f32 * cell_size.x,
                cursor.point.line.0 as f32 * cell_size.y,
            );
        let cursor_rect = Rect::from_min_size(origin, cell_size);
        let color = appearance.resolve(
            content.colors,
            Color::Named(NamedColor::Cursor),
            CellFlags::empty(),
        )?;
        if cursor.shape == CursorShape::Hidden || !visible {
        } else if !focused || cursor.shape == CursorShape::HollowBlock {
            painter.rect_stroke(
                cursor_rect,
                0.0,
                Stroke::new(CURSOR_WIDTH, color),
                egui::StrokeKind::Inside,
            );
        } else {
            match cursor.shape {
                CursorShape::Beam => {
                    painter.line_segment(
                        [cursor_rect.left_top(), cursor_rect.left_bottom()],
                        Stroke::new(CURSOR_WIDTH, color),
                    );
                }
                CursorShape::Underline => {
                    painter.line_segment(
                        [cursor_rect.left_bottom(), cursor_rect.right_bottom()],
                        Stroke::new(CURSOR_WIDTH, color),
                    );
                }
                _ => {
                    painter.rect_filled(cursor_rect, 0.0, color);
                    let cell = &grid[cursor.point];
                    if !cell.flags.contains(CellFlags::HIDDEN) {
                        let mut text = cell.c.to_string();
                        if let Some(extra) = cell.zerowidth() {
                            text.extend(extra);
                        }
                        let foreground = appearance.resolve(
                            content.colors,
                            Color::Named(NamedColor::Background),
                            CellFlags::empty(),
                        )?;
                        painter.text(
                            origin,
                            egui::Align2::LEFT_TOP,
                            text,
                            appearance.font.clone(),
                            foreground,
                        );
                    }
                }
            }
        }
        if focused {
            if !view.preedit.is_empty() {
                painter.text(
                    origin,
                    egui::Align2::LEFT_TOP,
                    &view.preedit,
                    appearance.font.clone(),
                    color32(appearance.foreground),
                );
            }
            let transform = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            ui.output_mut(|output| {
                output.ime = Some(egui::output::IMEOutput {
                    purpose: egui::IMEPurpose::Normal,
                    rect: transform * rect,
                    cursor_rect: transform * cursor_rect,
                    should_interrupt_composition: false,
                })
            });
        }
    }
    crate::terminal_ruler::paint(
        ui,
        Rect::from_min_size(
            rect.min,
            vec2(
                rect.width(),
                (grid.screen_lines() as f32 * cell_size.y).min(rect.height()),
            ),
        ),
        core,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "terminal-input-tests.rs"]
mod input_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn window_capture는_secondary_press_뒤_메뉴_owner와_후행_ax를_보존한다() {
        for late_ax in [false, true] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let terminal = egui::Id::new("menu-window-terminal");
            let menu = terminal.with("context-menu-keyboard-owner");
            let pane = PaneId::new();
            let tab = TabId::new();
            let mut routes = Views::default();
            let mut actions = Vec::new();
            let overrides = serde_json::json!([
                {"actionId": "focus-group-1", "key": "Enter", "mods": []},
                {"actionId": "terminal-jump-to-previous-command", "key": "Enter", "mods": []}
            ])
            .to_string();
            let rect =
                Rect::from_min_size(pos2(SCREEN[0] / 2.0, 0.0), vec2(SCREEN[0] / 2.0, SCREEN[1]));
            let mut render = |events, capture| {
                let mut button = None;
                let mut clicked = false;
                let mut remaining = Vec::new();
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(SCREEN[0], SCREEN[1]),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        if capture {
                            assert_eq!(
                                context.keyboard_focus_request_at(2),
                                Some(terminal.accesskit_id())
                            );
                            assert_eq!(
                                context.keyboard_focus_after_request_at(2),
                                Some(menu.accesskit_id())
                            );
                            assert_eq!(
                                context.previous_keyboard_target(menu.accesskit_id()),
                                Some(menu)
                            );
                            assert!(context.is_context_menu_keyboard_owner(menu));
                            routes
                                .capture_window_keymap(
                                    &context,
                                    Some(&overrides),
                                    &mut actions,
                                    true,
                                    |_| None,
                                )
                                .unwrap();
                            remaining = ui.input(|input| input.events.clone());
                        } else {
                            assert!(!context.is_context_menu_keyboard_owner(menu));
                        }
                        let response = ui.button("Menu scope prefix action");
                        button = Some(response.id);
                        clicked = response.clicked();
                        ui.interact(rect, terminal, Sense::click_and_drag());
                        context.register_pointer_keyboard_focus(terminal);
                        context.register_context_menu_keyboard_focus(terminal, menu);
                        if capture {
                            ui.interact(
                                Rect::from_min_size(rect.min, vec2(1.0, 1.0)),
                                menu,
                                Sense::focusable_noninteractive(),
                            );
                        }
                        let view = routes.view_mut(ui, &pane, &tab);
                        view.focus_id = Some(terminal);
                        view.focus_frame = Some(context.cumulative_frame_nr());
                    },
                );
                output.textures_delta.clear();
                (button.unwrap(), clicked, remaining)
            };
            let (button, _, _) = render(Vec::new(), false);
            context.memory_mut(|memory| memory.request_focus(button));
            render(Vec::new(), false);
            let key = |pressed| Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let mut events = vec![
                key(true),
                key(false),
                Event::PointerButton {
                    pos: rect.min + vec2(1.0, 1.0),
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                key(true),
                key(false),
            ];
            if late_ax {
                events.push(Event::AccessKitActionRequest(
                    egui::accesskit::ActionRequest {
                        action: egui::accesskit::Action::Focus,
                        target_tree: egui::accesskit::TreeId::ROOT,
                        target_node: button.accesskit_id(),
                        data: None,
                    },
                ));
            }
            let (_, clicked, remaining) = render(events, true);
            assert!(!clicked);
            assert_eq!(
                remaining
                    .iter()
                    .filter(|event| matches!(event, Event::Key { pressed: true, .. }))
                    .count(),
                0
            );
            assert_eq!(actions, ["focus-group-1", "focus-group-1"]);
            assert_eq!(
                context.memory(|memory| memory.focused()),
                Some(if late_ax { button } else { menu })
            );
        }
    }

    #[test]
    fn window_capture는_pointer와_ax_focus의_사건별_scope를_보존한다() {
        for pointer_last in [false, true] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let terminal = egui::Id::new("pointer-window-terminal");
            let pane = PaneId::new();
            let tab = TabId::new();
            let mut routes = Views::default();
            let mut actions = Vec::new();
            let overrides = serde_json::json!([
                {"actionId": "focus-group-1", "key": "Enter", "mods": []},
                {"actionId": "terminal-jump-to-previous-command", "key": "Enter", "mods": []}
            ])
            .to_string();
            let rect =
                Rect::from_min_size(pos2(SCREEN[0] / 2.0, 0.0), vec2(SCREEN[0] / 2.0, SCREEN[1]));
            let mut render = |events, capture| {
                let mut remaining = Vec::new();
                let mut button = None;
                let mut clicked = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(SCREEN[0], SCREEN[1]),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        if capture {
                            routes
                                .capture_window_keymap(
                                    &context,
                                    Some(&overrides),
                                    &mut actions,
                                    true,
                                    |_| None,
                                )
                                .unwrap();
                            remaining = ui.input(|input| input.events.clone());
                        }
                        let response = ui.button("Pointer scope action");
                        button = Some(response.id);
                        clicked = response.clicked();
                        ui.interact(rect, terminal, Sense::click_and_drag());
                        context.register_pointer_keyboard_focus(terminal);
                        let view = routes.view_mut(ui, &pane, &tab);
                        view.focus_id = Some(terminal);
                        view.focus_frame = Some(context.cumulative_frame_nr());
                    },
                );
                output.textures_delta.clear();
                (remaining, button.unwrap(), clicked)
            };
            let (_, button, _) = render(Vec::new(), false);
            context.memory_mut(|memory| memory.request_focus(button));
            render(Vec::new(), false);
            let key = |pressed| Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let pointer = |pressed| Event::PointerButton {
                pos: rect.min + vec2(1.0, 1.0),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            let focus = Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: button.accesskit_id(),
                data: None,
            });
            let events = if pointer_last {
                vec![
                    focus,
                    key(true),
                    key(false),
                    pointer(true),
                    key(true),
                    key(false),
                    pointer(false),
                ]
            } else {
                vec![
                    pointer(true),
                    key(true),
                    key(false),
                    pointer(false),
                    focus,
                    key(true),
                    key(false),
                ]
            };
            let (remaining, _, clicked) = render(events, true);
            assert!(
                !clicked,
                "pointer_last={pointer_last}: captured button Enter executed"
            );
            let keys: Vec<_> = remaining
                .into_iter()
                .filter(|event| matches!(event, Event::Key { .. }))
                .collect();
            let expected = if pointer_last {
                [key(false), key(true), key(false)]
            } else {
                [key(true), key(false), key(false)]
            };
            assert_eq!(keys, expected);
            assert_eq!(actions, ["focus-group-1"]);
            assert_eq!(
                context.memory(|memory| memory.focused()),
                Some(if pointer_last { terminal } else { button })
            );
        }
    }

    #[test]
    fn pointer_focus요청은_실제_enabled_hit과_선언에만_배정한다() {
        for (declared, enabled, covered) in [
            (true, true, false),
            (false, true, false),
            (true, false, false),
            (true, true, true),
        ] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let source = egui::Id::new("pointer-focus-source");
            let target = egui::Id::new("pointer-focus-hit-target");
            let rect =
                Rect::from_min_size(pos2(SCREEN[0] / 2.0, 0.0), vec2(SCREEN[0] / 2.0, SCREEN[1]));
            let render = |events, inspect| {
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(SCREEN[0], SCREEN[1]),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        if inspect {
                            let allowed = declared && enabled && !covered;
                            assert_eq!(
                                context.keyboard_focus_request_at(1),
                                allowed.then_some(target.accesskit_id()),
                                "declared={declared} enabled={enabled} covered={covered}"
                            );
                            assert_eq!(context.keyboard_focus_request_at(3), None);
                            assert_eq!(context.keyboard_focus_request_at(usize::MAX), None);
                            let first = context.keyboard_input_route(source);
                            let second = context.keyboard_input_route(target);
                            if allowed {
                                let (first, (final_first, _)) = first.unwrap();
                                let (second, (final_second, _)) = second.unwrap();
                                assert_eq!(
                                    [first[0].0, first[2].0, first[4].0],
                                    [Some(true), Some(false), Some(false)]
                                );
                                assert_eq!(
                                    [second[0].0, second[2].0, second[4].0],
                                    [Some(false), Some(true), Some(true)]
                                );
                                assert_eq!((final_first, final_second), (Some(false), Some(true)));
                            } else {
                                assert!(first.is_none() && second.is_none());
                            }
                        }
                        ui.interact(
                            Rect::from_min_size(egui::Pos2::ZERO, vec2(SCREEN[0] / 2.0, SCREEN[1])),
                            source,
                            Sense::click(),
                        );
                        ui.add_enabled_ui(enabled, |ui| {
                            ui.interact(rect, target, Sense::click_and_drag());
                            if declared {
                                context.register_pointer_keyboard_focus(target);
                            }
                        });
                        if covered {
                            egui::Area::new(egui::Id::new("pointer-focus-cover"))
                                .order(egui::Order::Foreground)
                                .fixed_pos(rect.min)
                                .fade_in(false)
                                .show(&context, |ui| {
                                    ui.interact(
                                        rect,
                                        egui::Id::new("pointer-focus-cover-click"),
                                        Sense::click(),
                                    );
                                    ui.allocate_rect(rect, Sense::hover());
                                });
                        }
                    },
                );
                output.textures_delta.clear();
            };
            render(Vec::new(), false);
            context.memory_mut(|memory| memory.request_focus(source));
            render(Vec::new(), false);
            render(Vec::new(), false);
            let pointer = |pressed| Event::PointerButton {
                pos: rect.min + vec2(1.0, 1.0),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            render(
                vec![
                    Event::Text("before".into()),
                    pointer(true),
                    Event::Text("after".into()),
                    pointer(false),
                    Event::Text("release".into()),
                ],
                true,
            );
        }
    }

    #[test]
    fn pointer_focus는_이전_button_enter를_소급_취소하지_않는다() {
        for reverse in [false, true] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let target = egui::Id::new("declared-pointer-focus-target");
            let render = |events| {
                let mut ids = Vec::new();
                let mut clicked = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(SCREEN[0], SCREEN[1]),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        let mut order = [0, 1];
                        if reverse {
                            order.reverse();
                        }
                        for index in order {
                            let rect = Rect::from_min_size(
                                pos2(index as f32 * SCREEN[0] / 2.0, 0.0),
                                vec2(SCREEN[0] / 2.0, SCREEN[1]),
                            );
                            let response = ui
                                .scope_builder(
                                    egui::UiBuilder::new()
                                        .id(egui::Id::new(("pointer-focus-default-key", index)))
                                        .max_rect(rect),
                                    |ui| {
                                        if index == 0 {
                                            let response = ui.button("Button prefix");
                                            clicked = response.clicked();
                                            return response;
                                        }
                                        let response =
                                            ui.interact(rect, target, Sense::click_and_drag());
                                        ui.ctx().register_pointer_keyboard_focus(target);
                                        response
                                    },
                                )
                                .inner;
                            ids.push((index, response.id, response.rect));
                        }
                    },
                );
                output.textures_delta.clear();
                ids.sort_by_key(|(index, _, _)| *index);
                (ids, clicked)
            };
            let (ids, _) = render(Vec::new());
            context.memory_mut(|memory| memory.request_focus(ids[0].1));
            render(Vec::new());
            let (_, clicked) = render(vec![
                Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                Event::PointerButton {
                    pos: ids[1].2.min + vec2(1.0, 1.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
            assert!(
                clicked,
                "reverse={reverse}: earlier button Enter was cancelled"
            );
            assert_eq!(context.memory(|memory| memory.focused()), Some(target));
        }
    }

    #[test]
    fn window_capture는_잠긴_탐색키_뒤_terminal_scope를_유지한다() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let terminal = egui::Id::new("locked-navigation-terminal");
        let pane = PaneId::new();
        let tab = TabId::new();
        let mut routes = Views::default();
        let mut actions = Vec::new();
        let overrides = serde_json::json!([
            {"actionId": "focus-group-1", "key": "Enter", "mods": []},
            {"actionId": "terminal-jump-to-previous-command", "key": "Enter", "mods": []}
        ])
        .to_string();
        let key = |key, pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let mut render = |events, capture| {
            let mut remaining = Vec::new();
            let mut button = None;
            let mut output = context.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    if capture {
                        routes
                            .capture_window_keymap(
                                ui.ctx(),
                                Some(&overrides),
                                &mut actions,
                                true,
                                |_| None,
                            )
                            .unwrap();
                        remaining = ui.input(|input| input.events.clone());
                    }
                    ui.interact(
                        Rect::from_min_size(egui::Pos2::ZERO, vec2(1.0, 1.0)),
                        terminal,
                        Sense::click(),
                    );
                    button = Some(ui.button("Navigation scope target").id);
                    context.memory_mut(|memory| {
                        memory.set_focus_lock_filter(
                            terminal,
                            egui::EventFilter {
                                tab: true,
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: true,
                            },
                        )
                    });
                    let view = routes.view_mut(ui, &pane, &tab);
                    view.focus_id = Some(terminal);
                    view.focus_frame = Some(ui.ctx().cumulative_frame_nr());
                },
            );
            output.textures_delta.clear();
            (remaining, button.unwrap())
        };
        let (_, button) = render(Vec::new(), false);
        context.memory_mut(|memory| memory.request_focus(terminal));
        render(Vec::new(), false);
        render(Vec::new(), false);
        let events = vec![
            key(egui::Key::Tab, true),
            key(egui::Key::ArrowLeft, true),
            key(egui::Key::Enter, true),
            key(egui::Key::Enter, false),
            Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: button.accesskit_id(),
                data: None,
            }),
            key(egui::Key::Enter, true),
        ];
        let (remaining, _) = render(events.clone(), true);
        assert_eq!(remaining, events[..events.len() - 1]);
        assert_eq!(actions, ["focus-group-1"]);
    }

    #[test]
    fn 탐색키_owner는_입력시작_filter와_실제_등록을_검사한다() {
        for (locked, enabled, registered) in [
            (true, true, true),
            (false, true, true),
            (true, false, true),
            (true, true, false),
        ] {
            let context = egui::Context::default();
            let id = egui::Id::new("navigation-filter-owner");
            let other = egui::Id::new("unregistered-navigation-owner");
            let render = |enabled, registered, inspect| {
                let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                    if inspect {
                        for key in [
                            egui::Key::Tab,
                            egui::Key::ArrowLeft,
                            egui::Key::ArrowRight,
                            egui::Key::ArrowUp,
                            egui::Key::ArrowDown,
                        ] {
                            let mut event = Event::Key {
                                key,
                                physical_key: None,
                                pressed: true,
                                repeat: false,
                                modifiers: egui::Modifiers::NONE,
                            };
                            assert_eq!(
                                context.keyboard_navigation_is_locked(id.accesskit_id(), &event),
                                locked && enabled && registered
                            );
                            assert!(
                                !context
                                    .keyboard_navigation_is_locked(other.accesskit_id(), &event)
                            );
                            if let Event::Key { pressed, .. } = &mut event {
                                *pressed = false;
                            }
                            assert!(
                                !context.keyboard_navigation_is_locked(id.accesskit_id(), &event)
                            );
                        }
                        assert!(!context.keyboard_navigation_is_locked(
                            id.accesskit_id(),
                            &Event::Text("x".into())
                        ));
                    }
                    if registered {
                        ui.add_enabled_ui(enabled, |ui| {
                            ui.interact(
                                Rect::from_min_size(egui::Pos2::ZERO, vec2(1.0, 1.0)),
                                id,
                                Sense::click(),
                            );
                        });
                    }
                    context.memory_mut(|memory| {
                        memory.set_focus_lock_filter(
                            id,
                            egui::EventFilter {
                                tab: locked,
                                horizontal_arrows: locked,
                                vertical_arrows: locked,
                                escape: locked,
                            },
                        )
                    });
                });
                output.textures_delta.clear();
            };
            render(true, true, false);
            context.memory_mut(|memory| memory.request_focus(id));
            render(true, true, false);
            render(enabled, registered, false);
            render(enabled, registered, true);
        }
    }

    #[test]
    fn window_capture는_사건별_terminal_editor_button_scope와_로컬_chord를_보존한다() {
        for local_chord in [false, true] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let mut routes = Views::default();
            let pane = PaneId::new();
            let tab = TabId::new();
            let editor_id = egui::Id::new("ordered-keymap-editor");
            let terminal_id = egui::Id::new("ordered-keymap-terminal");
            let mut button_id = None;
            let mut actions = Vec::new();
            let overrides = serde_json::json!([
                {"actionId": "focus-group-1", "key": "Enter", "mods": []},
                {"actionId": "terminal-jump-to-previous-command", "key": "Enter", "mods": [],
                    "chord": if local_chord { serde_json::json!({"key": "Space", "mods": []}) }
                        else { serde_json::Value::Null }}
            ])
            .to_string();
            let key = |key, pressed| Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            };
            let focus = |id: egui::Id| {
                Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Focus,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: id.accesskit_id(),
                    data: None,
                })
            };
            let mut render = |events: Vec<Event>, capture, composing| {
                let mut remaining = Vec::new();
                let mut clicked = false;
                let mut resolved = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(
                            egui::Pos2::ZERO,
                            vec2(SCREEN[0], SCREEN[1]),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        if capture {
                            routes
                                .capture_window_keymap(
                                    ui.ctx(),
                                    Some(&overrides),
                                    &mut actions,
                                    true,
                                    |id| (id == editor_id).then_some(composing),
                                )
                                .unwrap();
                            remaining = ui.input(|input| input.events.clone());
                            if local_chord {
                                let mut next = 0;
                                for event in &remaining {
                                    let index =
                                        crate::keymap::event_index(ui.ctx(), event, &mut next);
                                    routes
                                        .keymaps
                                        .route(
                                            crate::keymap::Route {
                                                context: ui.ctx(),
                                                event,
                                                index,
                                                scope: KeymapContext {
                                                    terminal: true,
                                                    editor: false,
                                                },
                                                composing: false,
                                                overrides: Some(&overrides),
                                            },
                                            |decision| {
                                                if let KeymapDecision::ResolveChord(id) = decision {
                                                    assert_eq!(
                                                        id,
                                                        "terminal-jump-to-previous-command"
                                                    );
                                                    resolved = true;
                                                    return true;
                                                }
                                                false
                                            },
                                        )
                                        .unwrap();
                                }
                            }
                        }
                        let button = ui.button("Scope target");
                        button_id = Some(button.id);
                        clicked = button.clicked();
                        for id in [terminal_id, editor_id] {
                            ui.allocate_rect(
                                Rect::from_min_size(egui::Pos2::ZERO, vec2(1.0, 1.0)),
                                Sense::hover(),
                            );
                            ui.interact(
                                Rect::from_min_size(egui::Pos2::ZERO, vec2(1.0, 1.0)),
                                id,
                                Sense::click(),
                            );
                        }
                        let view = routes.view_mut(ui, &pane, &tab);
                        view.focus_id = Some(terminal_id);
                        view.focus_frame = Some(ui.ctx().cumulative_frame_nr());
                    },
                );
                output.textures_delta.clear();
                (remaining, clicked, resolved)
            };
            render(Vec::new(), false, false);
            context.memory_mut(|memory| memory.request_focus(terminal_id));
            render(Vec::new(), false, false);
            if local_chord {
                let (remaining, _, resolved) = render(
                    vec![key(egui::Key::Enter, true), key(egui::Key::Space, true)],
                    true,
                    false,
                );
                assert_eq!(remaining.len(), 1);
                assert!(resolved);
            } else {
                let (remaining, clicked, _) = render(
                    vec![
                        key(egui::Key::Enter, true),
                        focus(editor_id),
                        key(egui::Key::Enter, true),
                    ],
                    true,
                    false,
                );
                assert!(remaining.iter().any(|event| matches!(
                    event,
                    Event::Key {
                        key: egui::Key::Enter,
                        pressed: true,
                        ..
                    }
                )));
                assert!(!clicked);
                let id = context.memory(|memory| memory.focused()).unwrap();
                assert_eq!(id, editor_id);
                let (remaining, clicked, _) = render(
                    vec![key(egui::Key::Enter, false), key(egui::Key::Enter, true)],
                    true,
                    true,
                );
                assert_eq!(remaining.len(), 2);
                assert!(!clicked);
            }
            assert_eq!(actions.len(), usize::from(!local_chord));
            if !local_chord {
                assert_eq!(actions, ["focus-group-1"]);
                let id = button_id.unwrap();
                let mut clicked = false;
                let mut output = context.run_ui(
                    egui::RawInput {
                        events: vec![
                            focus(id),
                            key(egui::Key::Enter, false),
                            key(egui::Key::Enter, true),
                        ],
                        ..Default::default()
                    },
                    |ui| {
                        routes
                            .capture_window_keymap(
                                ui.ctx(),
                                Some(&overrides),
                                &mut actions,
                                true,
                                |id| (id == editor_id).then_some(true),
                            )
                            .unwrap();
                        clicked = ui.button("Scope target").clicked();
                    },
                );
                output.textures_delta.clear();
                assert!(!clicked);
                assert_eq!(actions, ["focus-group-1", "focus-group-1"]);
            }
        }
    }

    use super::*;

    fn inclusive_selection(start: Point, end: Point) -> Selection {
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Right);
        selection.include_all();
        selection
    }

    fn selected_text(core: &TerminalCore, (start, end): (Point, Point)) -> AppResult<String> {
        Ok(core
            .selection_text(&inclusive_selection(start, end), INPUT_BYTES)?
            .unwrap_or_default())
    }

    const COLUMNS: u16 = 12;
    const ROWS: u16 = 4;
    const HISTORY: usize = 8;
    const FONT_SIZE: f32 = 13.0;
    const CELL: [f32; 2] = [8.0, 16.0];
    const SCREEN: [f32; 2] = [160.0, 80.0];
    const EDGE_FRACTION: f32 = 0.1;

    #[test]
    fn ruler_fit는_scrollback_설정에만_14px를_예약한다() {
        const FIT_SCREEN: [f32; 2] = [640.0, 320.0];
        const WITH_RULER_COLUMNS: u16 = 78;
        const WITHOUT_RULER_COLUMNS: u16 = 80;
        const FIT_ROWS: u16 = 20;
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(FIT_SCREEN[0], FIT_SCREEN[1]));
        let cell = vec2(CELL[0], CELL[1]);
        assert_eq!(
            measured(rect, cell, true),
            Some(Size {
                columns: WITH_RULER_COLUMNS,
                rows: FIT_ROWS
            })
        );
        assert_eq!(
            measured(rect, cell, false),
            Some(Size {
                columns: WITHOUT_RULER_COLUMNS,
                rows: FIT_ROWS
            })
        );
        assert!(measured(Rect::ZERO, cell, true).is_none());
        assert!(measured(rect, vec2(f32::NAN, cell.y), true).is_none());
    }

    #[test]
    fn command_gutter는_실제_shape의_두_pixel_색과_alt_숨김을_보존한다() {
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let success = Rgb { r: 1, g: 2, b: 3 };
        let failure = Rgb { r: 4, g: 5, b: 6 };
        core.configure_command_colors(taide_native_terminal::CommandColors {
            success: Some(success),
            failure: Some(failure),
        })
        .unwrap();
        core.advance(b"\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07\r\n\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;1\x07\x1b[?25l").unwrap();
        let context = egui::Context::default();
        let mut view = View::default();
        let render = |core: &TerminalCore, view: &mut View| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        pos2(0.0, 0.0),
                        vec2(SCREEN[0], SCREEN[1]),
                    )),
                    ..Default::default()
                },
                |ui| {
                    paint(
                        ui,
                        ui.available_rect_before_wrap(),
                        vec2(CELL[0], CELL[1]),
                        core,
                        view,
                        &appearance(),
                        false,
                    )
                    .unwrap();
                },
            );
            output.textures_delta.clear();
            output
                .shapes
                .into_iter()
                .filter_map(|shape| match shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.rect.width() == COMMAND_GUTTER_WIDTH
                            && [color32(success), color32(failure)].contains(&rect.fill) =>
                    {
                        Some((rect.rect, rect.fill))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let painted = render(&core, &mut view);
        assert_eq!(painted.len(), 2);
        assert_eq!(painted[0].0.size(), vec2(COMMAND_GUTTER_WIDTH, CELL[1]));
        assert_eq!(painted[1].0.top() - painted[0].0.top(), CELL[1]);
        assert_eq!(
            painted.iter().map(|(_, color)| *color).collect::<Vec<_>>(),
            [color32(success), color32(failure)]
        );
        core.advance(b"\x1b[?1049h").unwrap();
        assert!(render(&core, &mut view).is_empty());
        core.advance(b"\x1b[?1049l").unwrap();
        assert_eq!(render(&core, &mut view), painted);
    }

    #[test]
    fn mouse_disabled는_숨김_capture와_달리_대기와_held를_해제한다() {
        let context = egui::Context::default();
        let mut views = Views::default();
        let key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
        let mode = Mode::MOUSE_REPORT_CLICK | Mode::SGR_MOUSE;
        let frame = |views: &mut Views, enabled| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response = ui.interact(rect, ui.id(), Sense::click_and_drag());
                        views
                            .view_mut(ui, &key.1, &key.2)
                            .register_mouse(ui, &response, mode, true);
                    });
                },
            );
            output.textures_delta.clear();
        };
        frame(&mut views, true);
        let mut raw = egui::RawInput {
            events: vec![Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
            ..Default::default()
        };
        views.raw_input(&context, &mut raw);
        assert_eq!(views.views[&key].mouse.packets.len(), 1);
        assert_eq!(views.views[&key].mouse.button(), Some(MouseButton::Left));
        frame(&mut views, false);
        assert!(views.views[&key].mouse.packets.is_empty());
        assert!(views.views[&key].mouse.button().is_none());
        assert!(!views.views[&key].mouse.forced);
    }

    #[test]
    fn mouse_adapter는_raw_순서_capture_release_cell_pixel과_multi_pass를_보존한다() {
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        core.advance(b"\x1b[?1003h\x1b[?1006h").unwrap();
        let context = egui::Context::default();
        let mut views = Views::default();
        let key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
        let frame = |views: &mut Views, core: &TerminalCore, events| {
            let mut input = egui::RawInput {
                screen_rect: Some(rect),
                events,
                ..Default::default()
            };
            views.raw_input(&context, &mut input);
            let remaining_wheel = input
                .events
                .iter()
                .filter(|event| matches!(event, Event::MouseWheel { .. }))
                .count();
            let mut bytes = Vec::new();
            let mut output = context.run_ui(input, |ui| {
                if ui.ctx().current_pass_index() == 0 {
                    ui.ctx().request_discard("synthetic mouse multi-pass");
                }
                let response = ui.interact(
                    rect,
                    ui.make_persistent_id("synthetic-mouse"),
                    Sense::click_and_drag(),
                );
                let view = views.view_mut(ui, &key.1, &key.2);
                reconcile_view(core, view).unwrap();
                view.register_mouse(ui, &response, core.mode().unwrap(), true);
                for input in
                    process_mouse(ui, &response, core, vec2(CELL[0], CELL[1]), view, None).unwrap()
                {
                    let InputAction::Write(written) = core
                        .encode_input(NativeInput::Mouse(input), INPUT_BYTES)
                        .unwrap()
                    else {
                        panic!("mouse input was not written")
                    };
                    bytes.push(String::from_utf8(written).unwrap());
                }
                select_pointer(ui, &response, core, rect, vec2(CELL[0], CELL[1]), view).unwrap();
                assert!(
                    scroll_wheel(ui, &response, core, vec2(CELL[0], CELL[1]), view, None)
                        .unwrap()
                        .is_empty()
                );
            });
            output.textures_delta.clear();
            (bytes, remaining_wheel)
        };
        assert!(frame(&mut views, &core, Vec::new()).0.is_empty());
        let position = pos2(20.0, 20.0);
        let moved = pos2(28.0, 20.0);
        let outside = pos2(SCREEN[0] + CELL[0], SCREEN[1] + CELL[1]);
        let button = |position, pressed, modifiers| Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        let mut pending = egui::RawInput {
            events: vec![
                button(position, true, Default::default()),
                button(position, false, Default::default()),
            ],
            ..Default::default()
        };
        views.raw_input(&context, &mut pending);
        assert_eq!(views.views[&key].mouse.packets.len(), 2);
        let _ = context.run_logic(&pending, |_| {});
        let replayed_events = pending.events.len();
        views.raw_input_with_replay(&context, &mut pending, replayed_events);
        assert_eq!(views.views[&key].mouse.packets.len(), 2);
        assert_eq!(
            frame(&mut views, &core, Vec::new()).0,
            ["\x1b[<0;3;2M", "\x1b[<0;3;2m"]
        );
        let control = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        let (bytes, remaining_wheel) = frame(
            &mut views,
            &core,
            vec![
                button(position, true, control),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: vec2(0.0, 1.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: Default::default(),
                },
                Event::PointerMoved(moved),
                button(outside, false, Default::default()),
                Event::PointerMoved(outside),
            ],
        );
        assert_eq!(remaining_wheel, 0);
        assert_eq!(
            bytes,
            [
                "\x1b[<16;3;2M",
                "\x1b[<64;3;2M",
                "\x1b[<32;4;2M",
                "\x1b[<0;12;4m"
            ]
        );
        assert!(views.views[&key].selection.is_none());
        assert!(views.views[&key].mouse.button().is_none());
        assert_eq!(
            frame(
                &mut views,
                &core,
                vec![
                    Event::PointerMoved(moved),
                    Event::PointerMoved(pos2(moved.x + EDGE_FRACTION, moved.y + EDGE_FRACTION))
                ]
            )
            .0,
            ["\x1b[<35;4;2M"]
        );
        assert!(
            frame(
                &mut views,
                &core,
                vec![button(position, false, Default::default())]
            )
            .0
            .is_empty()
        );
        assert!(
            frame(
                &mut views,
                &core,
                vec![
                    button(outside, true, Default::default()),
                    Event::PointerMoved(position),
                    button(position, false, Default::default())
                ]
            )
            .0
            .is_empty()
        );
        assert_eq!(
            frame(
                &mut views,
                &core,
                vec![
                    button(position, true, Default::default()),
                    Event::PointerGone
                ]
            )
            .0,
            ["\x1b[<0;3;2M"]
        );
        assert_eq!(views.views[&key].mouse.button(), Some(MouseButton::Left));
        assert_eq!(
            frame(
                &mut views,
                &core,
                vec![button(outside, false, Default::default())]
            )
            .0,
            ["\x1b[<0;12;4m"]
        );
        core.advance(b"\x1b[?1016h").unwrap();
        assert!(frame(&mut views, &core, Vec::new()).0.is_empty());
        assert_eq!(
            frame(
                &mut views,
                &core,
                vec![
                    Event::PointerMoved(moved),
                    Event::PointerMoved(pos2(moved.x + EDGE_FRACTION, moved.y + EDGE_FRACTION)),
                    Event::PointerMoved(pos2(moved.x + 1.0, moved.y))
                ]
            )
            .0,
            ["\x1b[<35;28;20M", "\x1b[<35;29;20M"]
        );
        let (bytes, _) = frame(
            &mut views,
            &core,
            vec![
                Event::ModifiersChanged(control),
                Event::PointerMoved(position),
            ],
        );
        assert_eq!(bytes, ["\x1b[<51;20;20M"]);
        let wheel = |unit, amount| Event::MouseWheel {
            unit,
            delta: vec2(0.0, amount),
            phase: egui::TouchPhase::Move,
            modifiers: Default::default(),
        };
        const PARTIAL_POINTS: usize = 5;
        const POINT_AMOUNT: f32 = 10.0;
        assert!(
            frame(
                &mut views,
                &core,
                (0..PARTIAL_POINTS)
                    .map(|_| wheel(egui::MouseWheelUnit::Point, POINT_AMOUNT))
                    .collect()
            )
            .0
            .is_empty()
        );
        core.record_user_input().unwrap();
        assert_eq!(
            frame(
                &mut views,
                &core,
                vec![wheel(egui::MouseWheelUnit::Point, POINT_AMOUNT)]
            )
            .0,
            ["\x1b[<64;20;20M"]
        );
        core.advance(b"\x1b[?9h\x1b[?1016lone\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven\r\neight\r\nnine").unwrap();
        assert!(frame(&mut views, &core, Vec::new()).0.is_empty());
        assert!(
            frame(
                &mut views,
                &core,
                vec![wheel(egui::MouseWheelUnit::Line, 1.0)]
            )
            .0
            .is_empty()
        );
        assert!(views.views[&key].offset > 0);
    }

    #[tokio::test]
    async fn mouse_queue는_상한에서도_release와_다음_frame의_미전송_입력을_보존한다() {
        const FREE_RECEIPTS: usize = 2;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        core.advance(b"\x1b[?1003h\x1b[?1016h").unwrap();
        let context = egui::Context::default();
        let mut views = Views::default();
        let key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
        let frame = |views: &mut Views| {
            let mut input = egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            };
            views.raw_input(&context, &mut input);
            let mut inputs = Vec::new();
            let mut output = context.run_ui(input, |ui| {
                let response = ui.interact(rect, ui.id(), Sense::click_and_drag());
                let view = views.view_mut(ui, &key.1, &key.2);
                reconcile_view(&core, view).unwrap();
                view.register_mouse(ui, &response, core.mode().unwrap(), true);
                inputs.extend(
                    process_mouse(ui, &response, &core, vec2(CELL[0], CELL[1]), view, None)
                        .unwrap(),
                );
            });
            output.textures_delta.clear();
            inputs
        };
        assert!(frame(&mut views).is_empty());
        let position = pos2(1.0, 1.0);
        let button = |pressed| Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let mut input = egui::RawInput::default();
        input.events.push(button(true));
        input
            .events
            .extend((0..INPUT_RECEIPTS).map(|index| Event::PointerMoved(pos2(index as f32, 1.0))));
        input.events.push(button(false));
        views.raw_input(&context, &mut input);
        let view = views.views.get_mut(&key).unwrap();
        assert_eq!(view.mouse.packets.len(), INPUT_RECEIPTS);
        assert!(matches!(
            view.mouse.packets.back().unwrap().action,
            Some(MouseAction::Release(MouseButton::Left))
        ));
        assert!(view.mouse.button().is_none());
        let tasks = taide_runtime::TaskSupervisor::new(tokio::runtime::Handle::current());
        let (writer, worker) = crate::terminal_writer::Writer::start(
            &tasks,
            crate::terminal_writer::Limits {
                bytes: INPUT_BYTES,
                count: INPUT_RECEIPTS,
            },
            |_| Ok(()),
        )
        .unwrap();
        for _ in FREE_RECEIPTS..INPUT_RECEIPTS {
            view.outbox
                .lock()
                .unwrap()
                .receipts
                .push_back(writer.submit(vec![0]).unwrap());
        }
        let first = frame(&mut views);
        assert_eq!(first.len(), FREE_RECEIPTS);
        assert!(matches!(
            first[0].action,
            MouseAction::Press(MouseButton::Left)
        ));
        let view = views.views.get_mut(&key).unwrap();
        assert_eq!(view.mouse.packets.len(), INPUT_RECEIPTS - FREE_RECEIPTS);
        let receipts = std::mem::take(&mut view.outbox.lock().unwrap().receipts);
        for receipt in receipts {
            receipt.wait().await.unwrap();
        }
        let remaining = frame(&mut views);
        assert_eq!(remaining.len(), INPUT_RECEIPTS - FREE_RECEIPTS);
        assert!(matches!(
            remaining.last().unwrap().action,
            MouseAction::Release(MouseButton::Left)
        ));
        assert!(views.views[&key].mouse.packets.is_empty());
        writer.close();
        worker.await.unwrap();
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[test]
    fn wheel는_작은_point를_매_frame_한_행으로_올리지_않는다() {
        const START: f64 = 30.0;
        const SMALL_POINT: f32 = 1.0;
        const OUTPUT_ROWS: usize = 18;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let output = (0..OUTPUT_ROWS)
            .map(|row| format!("row{row}"))
            .collect::<Vec<_>>()
            .join("\r\n");
        core.advance(output.as_bytes()).unwrap();
        let context = egui::Context::default();
        let cell = vec2(CELL[0], CELL[1]);
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
        let mut view = View::default();
        let frame = |time, events, view: &mut View| {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    events,
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    let response = ui.interact(
                        rect,
                        ui.make_persistent_id("synthetic-wheel"),
                        Sense::click_and_drag(),
                    );
                    scroll_wheel(ui, &response, &core, cell, view, None).unwrap();
                },
            );
            output.textures_delta.clear();
        };
        frame(START - 1.0, Vec::new(), &mut view);
        frame(
            START,
            vec![
                Event::PointerMoved(rect.center()),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, SMALL_POINT),
                    phase: egui::TouchPhase::Move,
                    modifiers: Default::default(),
                },
            ],
            &mut view,
        );
        assert_eq!(view.offset, 0);
    }

    #[test]
    fn wheel_epoch는_buffer_왕복_reset_rows_변경_전의_미승인_packet을_전송하지_않는다() {
        const EMPTY_HISTORY: usize = 0;
        let boundaries = [
            (Some(b"\x1b[?1049h\x1b[?1049l".as_slice()), None),
            (Some(b"\x1bc".as_slice()), None),
            (
                None,
                Some(Size {
                    columns: COLUMNS,
                    rows: ROWS + 1,
                }),
            ),
        ];
        for (bytes, size) in boundaries {
            let mut core = TerminalCore::new(
                Size {
                    columns: COLUMNS,
                    rows: ROWS,
                },
                EMPTY_HISTORY,
                Default::default(),
            )
            .unwrap();
            let context = egui::Context::default();
            let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
            let key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
            let mut views = Views::default();
            let mut rendered = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    let response = ui.interact(rect, ui.id(), Sense::click_and_drag());
                    let view = views.view_mut(ui, &key.1, &key.2);
                    reconcile_view(&core, view).unwrap();
                    view.register_mouse(ui, &response, core.mode().unwrap(), true);
                    view.mouse_geometry =
                        Some(MouseGeometry::new(ui, &response, vec2(CELL[0], CELL[1])));
                },
            );
            rendered.textures_delta.clear();
            let mut raw = egui::RawInput {
                events: vec![
                    Event::PointerMoved(rect.center()),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: vec2(0.0, 1.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            };
            views.raw_input(&context, &mut raw);
            let view = views.views.get_mut(&key).unwrap();
            assert_eq!(view.captured_wheel.packets.len(), 1);
            let previous = core.selection_stamp().unwrap();
            if let Some(bytes) = bytes {
                core.advance(bytes).unwrap();
                assert_ne!(
                    core.selection_stamp().unwrap().buffer_epoch,
                    previous.buffer_epoch
                );
            }
            if let Some(size) = size {
                core.resize(size).unwrap();
                assert_ne!(
                    core.selection_stamp().unwrap().rows_epoch,
                    previous.rows_epoch
                );
            }
            assert!(!core.mode().unwrap().contains(Mode::ALT_SCREEN));
            reconcile_view(&core, view).unwrap();
            let keys = take_wheel(&core, view, None, 1, None).unwrap();
            assert!(keys.is_empty(), "stale wheel produced {} keys", keys.len());
            assert!(view.captured_wheel.packets.is_empty());
        }
    }

    #[tokio::test]
    async fn wheel_hidden은_포화_중_원래_geometry로_local_scroll을_처리하고_취소한다() {
        const OUTPUT_ROWS: usize = 18;
        const EXPECTED_OFFSET: usize = 3;
        const MOVED_TOP: f32 = 200.0;
        const WHEEL_COUNT: usize = 2;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let output = (0..OUTPUT_ROWS)
            .map(|row| format!("row{row}"))
            .collect::<Vec<_>>()
            .join("\r\n");
        core.advance(output.as_bytes()).unwrap();
        let context = egui::Context::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
        let key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
        let mut views = Views::default();
        let mut rendered = context.run_ui(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ui| {
                let response = ui.interact(rect, ui.id(), Sense::click_and_drag());
                let view = views.view_mut(ui, &key.1, &key.2);
                view.register_mouse(ui, &response, core.mode().unwrap(), true);
                view.mouse_geometry =
                    Some(MouseGeometry::new(ui, &response, vec2(CELL[0], CELL[1])));
            },
        );
        rendered.textures_delta.clear();
        let mut raw = egui::RawInput {
            events: std::iter::once(Event::PointerMoved(rect.center()))
                .chain((0..WHEEL_COUNT).map(|_| Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: vec2(0.0, 1.0),
                    phase: egui::TouchPhase::Move,
                    modifiers: Default::default(),
                }))
                .collect(),
            ..Default::default()
        };
        views.raw_input(&context, &mut raw);
        assert_eq!(raw.events.len(), 1);
        for _ in 0..WHEEL_COUNT {
            let mut hidden = context.run_ui(egui::RawInput::default(), |ui| {
                ui.label("another tab");
            });
            hidden.textures_delta.clear();
            views.raw_input(&context, &mut egui::RawInput::default());
        }
        let tasks = taide_runtime::TaskSupervisor::new(tokio::runtime::Handle::current());
        let (writer, worker) = crate::terminal_writer::Writer::start(
            &tasks,
            crate::terminal_writer::Limits {
                bytes: INPUT_BYTES,
                count: INPUT_RECEIPTS,
            },
            |_| Ok(()),
        )
        .unwrap();
        let view = views.views.get_mut(&key).unwrap();
        for _ in 0..INPUT_RECEIPTS {
            view.outbox
                .lock()
                .unwrap()
                .receipts
                .push_back(writer.submit(vec![0]).unwrap());
        }
        assert_eq!(view.input_capacity().unwrap(), 0);
        assert_eq!(view.captured_wheel.packets.len(), WHEEL_COUNT);
        view.mouse_geometry.as_mut().unwrap().global_rect = rect.translate(vec2(0.0, MOVED_TOP));
        reconcile_view(&core, view).unwrap();
        assert!(take_wheel(&core, view, None, 1, None).unwrap().is_empty());
        assert_eq!(view.offset, EXPECTED_OFFSET);
        assert_eq!(view.captured_wheel.packets.len(), WHEEL_COUNT - 1);
        assert_eq!(core.grid().unwrap().display_offset(), 0);
        let receipts = std::mem::take(&mut view.outbox.lock().unwrap().receipts);
        views.cancel_inputs();
        assert!(views.views[&key].captured_wheel.packets.is_empty());
        for receipt in receipts {
            receipt.wait().await.unwrap();
        }
        writer.close();
        worker.await.unwrap();
        tasks.shutdown().await;
        assert_eq!(tasks.tracked_count(), 0);
    }

    #[test]
    fn wheel_policy는_누적_상한_소유권과_live_mode_방향키를_보존한다() {
        const START: f64 = 40.0;
        const STEP: f64 = 0.01;
        const POINT_EVENTS: usize = 5;
        const PARTIAL_EVENTS: usize = 4;
        const OUTPUT_ROWS: usize = 18;
        const LARGE_DELTA: f32 = 1024.0;
        const LINE_OFFSET: usize = 3;
        let size = Size {
            columns: COLUMNS,
            rows: ROWS,
        };
        let mut core = TerminalCore::new(size, HISTORY, Default::default()).unwrap();
        let output = (0..OUTPUT_ROWS)
            .map(|row| format!("row{row}"))
            .collect::<Vec<_>>()
            .join("\r\n");
        core.advance(output.as_bytes()).unwrap();
        let empty = TerminalCore::new(size, HISTORY, Default::default()).unwrap();
        assert!(empty.has_scrollback().unwrap());
        let zero = TerminalCore::new(size, 0, Default::default()).unwrap();
        assert!(!zero.has_scrollback().unwrap());
        let context = egui::Context::default();
        let cell = vec2(CELL[0], CELL[1]);
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
        let mut now = START;
        let mut frame = |core: &TerminalCore, events, view: &mut View, enabled| {
            now += STEP;
            let mut keys = Ok(Vec::new());
            let mut remaining = 0;
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(now),
                    events,
                    screen_rect: Some(rect),
                    ..Default::default()
                },
                |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response = ui.interact(
                            rect,
                            ui.make_persistent_id("synthetic-wheel-policy"),
                            Sense::click_and_drag(),
                        );
                        keys = scroll_wheel(ui, &response, core, cell, view, None);
                        remaining = ui.input(|input| {
                            input
                                .events
                                .iter()
                                .filter(|event| matches!(event, Event::MouseWheel { .. }))
                                .count()
                        });
                    });
                },
            );
            output.textures_delta.clear();
            (keys, remaining)
        };
        let wheel = |unit, y, modifiers| Event::MouseWheel {
            unit,
            delta: vec2(0.0, y),
            phase: egui::TouchPhase::Move,
            modifiers,
        };
        let mut view = View::default();
        frame(&core, Vec::new(), &mut view, true).0.unwrap();
        for index in 0..POINT_EVENTS {
            let result = frame(
                &core,
                vec![
                    Event::PointerMoved(rect.center()),
                    wheel(egui::MouseWheelUnit::Point, 1.0, Default::default()),
                ],
                &mut view,
                true,
            );
            assert!(result.0.unwrap().is_empty());
            assert_eq!(result.1, 0);
            assert_eq!(view.offset, usize::from(index == POINT_EVENTS - 1));
        }
        frame(&core, Vec::new(), &mut view, true).0.unwrap();
        assert_eq!(view.offset, 1);
        assert_eq!(core.grid().unwrap().display_offset(), 0);
        let other = View::default();
        assert_eq!(other.offset, 0);
        view = View::default();
        frame(
            &core,
            vec![wheel(egui::MouseWheelUnit::Line, 1.0, Default::default())],
            &mut view,
            true,
        )
        .0
        .unwrap();
        assert_eq!(view.offset, LINE_OFFSET);
        frame(
            &core,
            vec![wheel(
                egui::MouseWheelUnit::Point,
                LARGE_DELTA,
                Default::default(),
            )],
            &mut view,
            true,
        )
        .0
        .unwrap();
        assert_eq!(view.offset, HISTORY);
        frame(
            &core,
            vec![wheel(
                egui::MouseWheelUnit::Point,
                -LARGE_DELTA,
                Default::default(),
            )],
            &mut view,
            true,
        )
        .0
        .unwrap();
        assert_eq!(view.offset, 0);
        let alt = egui::Modifiers {
            alt: true,
            ..Default::default()
        };
        frame(
            &core,
            vec![wheel(egui::MouseWheelUnit::Line, 1.0, alt)],
            &mut view,
            true,
        )
        .0
        .unwrap();
        assert_eq!(view.offset, HISTORY);
        view = View::default();
        let ctrl = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        frame(
            &core,
            vec![wheel(egui::MouseWheelUnit::Line, 1.0, ctrl)],
            &mut view,
            true,
        )
        .0
        .unwrap();
        assert_eq!(view.offset, LINE_OFFSET);
        view = View::default();
        frame(
            &core,
            vec![wheel(egui::MouseWheelUnit::Page, 1.0, Default::default())],
            &mut view,
            true,
        )
        .0
        .unwrap();
        assert_eq!(view.offset, 0);
        let horizontal = Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(LARGE_DELTA, 0.0),
            phase: egui::TouchPhase::Move,
            modifiers: Default::default(),
        };
        let result = frame(&core, vec![horizontal], &mut view, true);
        assert!(result.0.unwrap().is_empty());
        assert_eq!(result.1, 1);
        assert_eq!(view.offset, 0);
        let result = frame(
            &core,
            vec![wheel(egui::MouseWheelUnit::Line, 1.0, Default::default())],
            &mut view,
            false,
        );
        assert!(result.0.unwrap().is_empty());
        assert_eq!(result.1, 1);
        assert_eq!(view.offset, 0);
        let result = frame(
            &core,
            vec![
                Event::PointerMoved(pos2(-1.0, -1.0)),
                wheel(egui::MouseWheelUnit::Line, 1.0, Default::default()),
            ],
            &mut view,
            true,
        );
        assert!(result.0.unwrap().is_empty());
        assert_eq!(result.1, 1);
        assert_eq!(view.offset, 0);
        view = View::default();
        let result = frame(
            &empty,
            vec![
                Event::PointerMoved(rect.center()),
                wheel(egui::MouseWheelUnit::Line, 1.0, Default::default()),
            ],
            &mut view,
            true,
        );
        assert!(result.0.unwrap().is_empty());
        assert_eq!(view.offset, 0);
        view = View::default();
        assert!(matches!(
            frame(
                &zero,
                vec![wheel(
                    egui::MouseWheelUnit::Line,
                    LARGE_DELTA,
                    Default::default()
                )],
                &mut view,
                true
            )
            .0
            .unwrap()
            .as_slice(),
            [Key::ArrowUp]
        ));
        core.advance(b"\x1b[?1049h").unwrap();
        assert!(!core.has_scrollback().unwrap());
        view = View::default();
        for index in 0..PARTIAL_EVENTS {
            let keys = frame(
                &core,
                vec![wheel(
                    egui::MouseWheelUnit::Point,
                    cell.y,
                    Default::default(),
                )],
                &mut view,
                true,
            )
            .0
            .unwrap();
            assert_eq!(keys.len(), usize::from(index == PARTIAL_EVENTS - 1));
            if let Some(key) = keys.first() {
                assert!(matches!(key, Key::ArrowUp));
            }
        }
        let keys = frame(
            &core,
            vec![
                wheel(egui::MouseWheelUnit::Line, LARGE_DELTA, Default::default()),
                wheel(egui::MouseWheelUnit::Page, -1.0, Default::default()),
            ],
            &mut view,
            true,
        )
        .0
        .unwrap();
        assert!(matches!(keys.as_slice(), [Key::ArrowUp, Key::ArrowDown]));
        assert!(
            matches!(core.encode_input(NativeInput::Key { key: keys[0], modifiers: Default::default() }, INPUT_BYTES).unwrap(), InputAction::Write(bytes) if bytes == b"\x1b[A")
        );
        core.advance(b"\x1b[?1h").unwrap();
        assert!(
            matches!(core.encode_input(NativeInput::Key { key: keys[1], modifiers: Default::default() }, INPUT_BYTES).unwrap(), InputAction::Write(bytes) if bytes == b"\x1bOB")
        );
        view.wheel = Wheel::default();
        assert!(matches!(
            frame(
                &core,
                vec![wheel(egui::MouseWheelUnit::Point, cell.y, alt)],
                &mut view,
                true
            )
            .0
            .unwrap()
            .as_slice(),
            [Key::ArrowUp]
        ));
        view.wheel = Wheel::default();
        assert!(matches!(
            frame(
                &core,
                vec![wheel(egui::MouseWheelUnit::Point, -cell.y, ctrl)],
                &mut view,
                true
            )
            .0
            .unwrap()
            .as_slice(),
            [Key::ArrowDown]
        ));
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        assert!(
            frame(
                &core,
                vec![wheel(egui::MouseWheelUnit::Line, LARGE_DELTA, shift)],
                &mut view,
                true
            )
            .0
            .unwrap()
            .is_empty()
        );
        core.advance(b"\x1b[?1000h").unwrap();
        let result = frame(
            &core,
            vec![wheel(egui::MouseWheelUnit::Line, 1.0, Default::default())],
            &mut view,
            true,
        );
        assert!(result.0.unwrap().is_empty());
        assert_eq!(result.1, 1);
        core.advance(b"\x1b[?1000l").unwrap();
        let result = frame(
            &core,
            (0..=INPUT_RECEIPTS)
                .map(|_| wheel(egui::MouseWheelUnit::Line, 1.0, Default::default()))
                .collect(),
            &mut view,
            true,
        );
        assert!(result.0.is_err());
        assert_eq!(result.1, 1);
        let partial_before_input = view.wheel.partial;
        assert_ne!(partial_before_input, 0.0);
        core.record_user_input().unwrap();
        frame(&core, Vec::new(), &mut view, true).0.unwrap();
        assert_eq!(view.wheel.partial, partial_before_input);
        assert!(view.wheel.position.is_none());
        let invalid = wheel(egui::MouseWheelUnit::Point, f32::NAN, Default::default());
        assert!(
            view.wheel
                .apply(&core, &mut view.offset, cell.y, &invalid)
                .is_err()
        );
        assert_eq!(core.grid().unwrap().display_offset(), 0);
    }

    #[test]
    fn wheel_capture는_logic_disabled_hidden_viewport와_budget을_보존한다() {
        const WIDTH: f32 = 320.0;
        const HEIGHT: f32 = 160.0;
        const TARGET_SIZE: f32 = 100.0;
        const SECOND_LEFT: f32 = 160.0;
        let context = egui::Context::default();
        let core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let first = Rect::from_min_size(pos2(0.0, 0.0), vec2(TARGET_SIZE, TARGET_SIZE));
        let second = Rect::from_min_size(pos2(SECOND_LEFT, 0.0), vec2(TARGET_SIZE, TARGET_SIZE));
        let first_key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
        let second_key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
        let mut views = Views::default();
        let frame = |views: &mut Views, enabled, hidden: bool| {
            let expected_route = views.raw_frame.is_some_and(|(viewport, frame)| {
                viewport == egui::ViewportId::ROOT && frame == context.cumulative_frame_nr()
            });
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(WIDTH, HEIGHT))),
                    ..Default::default()
                },
                |ui| {
                    if !hidden {
                        ui.add_enabled_ui(enabled, |ui| {
                            let response = ui.interact(
                                first,
                                ui.make_persistent_id("first-capture"),
                                Sense::click_and_drag(),
                            );
                            let view = views.view_mut(ui, &first_key.1, &first_key.2);
                            assert_eq!(view.is_wheel_routed, expected_route);
                            view.wheel_target = Some(WheelTarget::new(ui, &response, true));
                            scroll_wheel(ui, &response, &core, vec2(CELL[0], CELL[1]), view, None)
                                .unwrap();
                        });
                    }
                    let response = ui.interact(
                        second,
                        ui.make_persistent_id("second-capture"),
                        Sense::click_and_drag(),
                    );
                    let view = views.view_mut(ui, &second_key.1, &second_key.2);
                    assert_eq!(view.is_wheel_routed, expected_route);
                    view.wheel_target = Some(WheelTarget::new(ui, &response, true));
                    scroll_wheel(ui, &response, &core, vec2(CELL[0], CELL[1]), view, None).unwrap();
                    if ui.ctx().current_pass_index() == 0 {
                        ui.ctx()
                            .request_discard("synthetic wheel ownership multi-pass");
                    }
                },
            );
            output.textures_delta.clear();
        };
        let wheel = || Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: vec2(0.0, 1.0),
            phase: egui::TouchPhase::Move,
            modifiers: Default::default(),
        };
        let raw = |position| egui::RawInput {
            events: vec![Event::PointerMoved(position), wheel()],
            ..Default::default()
        };
        frame(&mut views, true, false);
        let mut input = raw(first.center());
        views.raw_input(&context, &mut input);
        assert_eq!(views.views[&first_key].captured_wheel.packets.len(), 1);
        assert!(views.views[&second_key].captured_wheel.packets.is_empty());
        assert_eq!(input.events.len(), 1);
        drop(context.run_logic(&input, |_| {}));
        input.events.push(wheel());
        views.raw_input(&context, &mut input);
        assert_eq!(views.views[&first_key].captured_wheel.packets.len(), 2);
        assert_eq!(input.events.len(), 1);
        frame(&mut views, false, false);
        assert!(views.views[&first_key].captured_wheel.packets.is_empty());
        let mut input = raw(first.center());
        views.raw_input(&context, &mut input);
        assert_eq!(input.events.len(), 2);
        assert!(views.views[&first_key].captured_wheel.packets.is_empty());
        let mut input = raw(second.center());
        views.raw_input(&context, &mut input);
        assert_eq!(input.events.len(), 1);
        assert_eq!(views.views[&second_key].captured_wheel.packets.len(), 1);
        frame(&mut views, true, true);
        let mut input = raw(first.center());
        views.raw_input(&context, &mut input);
        assert_eq!(input.events.len(), 2);
        assert!(views.views[&first_key].captured_wheel.packets.is_empty());
        let mut input = raw(second.center());
        input.viewport_id = egui::ViewportId::from_hash_of("synthetic-other-viewport");
        views.raw_input(&context, &mut input);
        assert_eq!(input.events.len(), 2);
        assert!(views.views[&second_key].captured_wheel.packets.is_empty());
        let mut input = egui::RawInput {
            events: std::iter::once(Event::PointerMoved(second.center()))
                .chain((0..=INPUT_RECEIPTS).map(|_| wheel()))
                .collect(),
            ..Default::default()
        };
        views.raw_input(&context, &mut input);
        assert_eq!(input.events.len(), 1);
        assert_eq!(
            views.views[&second_key].captured_wheel.packets.len(),
            INPUT_RECEIPTS
        );
        assert!(views.views[&second_key].captured_wheel.packets.capacity() <= INPUT_RECEIPTS);
        assert!(views.views[&second_key].error.is_some());
        views.retain(&HashSet::from([second_key.2.clone()]), &HashSet::new());
        assert!(!views.views.contains_key(&first_key));
        frame(&mut views, true, true);
        let target = views.views[&second_key].wheel_target.as_ref().unwrap();
        views.views.insert(
            first_key.clone(),
            View {
                wheel_target: Some(WheelTarget {
                    frame: target.frame,
                    rect: target.rect,
                    layer: target.layer,
                    enabled: true,
                    mode: Mode::NONE,
                }),
                ..Default::default()
            },
        );
        let mut input = raw(second.center());
        views.raw_input(&context, &mut input);
        assert_eq!(input.events.len(), 2);
        assert!(views.views[&first_key].captured_wheel.packets.is_empty());
        assert!(views.views[&second_key].captured_wheel.packets.is_empty());
    }

    #[test]
    fn wheel_owner는_terminal에서_다른_scroll_area로_smoothing_tail을_넘기지_않는다() {
        const START: f64 = 60.0;
        const STEP: f64 = 0.016;
        const SCREEN_HEIGHT: f32 = 240.0;
        const OTHER_TOP: f32 = 100.0;
        const OTHER_HEIGHT: f32 = 80.0;
        const INITIAL_OFFSET: f32 = 100.0;
        const CONTENT_HEIGHT: f32 = 512.0;
        let core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let context = egui::Context::default();
        let cell = vec2(CELL[0], CELL[1]);
        let terminal = Rect::from_min_size(pos2(0.0, 0.0), vec2(SCREEN[0], SCREEN[1]));
        let other = Rect::from_min_size(pos2(0.0, OTHER_TOP), vec2(SCREEN[0], OTHER_HEIGHT));
        let mut views = Views::default();
        let key = (egui::ViewportId::ROOT, PaneId::new(), TabId::new());
        let mut frame = |time, events| {
            let mut offset = INITIAL_OFFSET;
            let mut input = egui::RawInput {
                time: Some(time),
                events,
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN_HEIGHT),
                )),
                ..Default::default()
            };
            views.raw_input(&context, &mut input);
            let mut output = context.run_ui(input, |ui| {
                let response = ui.interact(
                    terminal,
                    ui.make_persistent_id("synthetic-wheel-owner"),
                    Sense::click_and_drag(),
                );
                let view = views.view_mut(ui, &key.1, &key.2);
                view.wheel_target = Some(WheelTarget::new(ui, &response, true));
                scroll_wheel(ui, &response, &core, cell, view, None).unwrap();
                offset = ui
                    .scope_builder(egui::UiBuilder::new().max_rect(other), |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("synthetic-other-scroll-area")
                            .auto_shrink([false, false])
                            .vertical_scroll_offset(INITIAL_OFFSET)
                            .show(ui, |ui| ui.set_min_height(CONTENT_HEIGHT))
                            .state
                            .offset
                            .y
                    })
                    .inner;
            });
            output.textures_delta.clear();
            offset
        };
        frame(START, Vec::new());
        assert_eq!(
            frame(
                START + STEP,
                vec![
                    Event::PointerMoved(terminal.center()),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: vec2(0.0, 1.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: Default::default()
                    }
                ]
            ),
            INITIAL_OFFSET
        );
        assert_eq!(
            frame(
                START + STEP + STEP,
                vec![Event::PointerMoved(other.center())]
            ),
            INITIAL_OFFSET
        );
    }

    #[test]
    fn selection_autoscroll은_실제_down_move_timer와_history_상한을_보존한다() {
        const START: f64 = 10.0;
        const MOVE: f64 = 10.02;
        const BEFORE: f64 = 10.049;
        const TICK: f64 = 10.051;
        const VIEW_TOP: f32 = 80.0;
        const OUTSIDE: f32 = 50.0;
        const SCREEN_SIZE: f32 = 256.0;
        const OUTPUT_ROWS: usize = 18;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let output = (0..OUTPUT_ROWS)
            .map(|row| format!("row{row}"))
            .collect::<Vec<_>>()
            .join("\r\n");
        core.advance(output.as_bytes()).unwrap();
        let context = egui::Context::default();
        let cell = vec2(CELL[0], CELL[1]);
        let rect = Rect::from_min_size(
            pos2(CELL[0], VIEW_TOP),
            cell * vec2(f32::from(COLUMNS), f32::from(ROWS)),
        );
        let mut view = View::default();
        let frame = |now, events, view: &mut View| {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(now),
                    events,
                    screen_rect: Some(Rect::from_min_size(
                        pos2(0.0, 0.0),
                        vec2(SCREEN_SIZE, SCREEN_SIZE),
                    )),
                    ..Default::default()
                },
                |ui| {
                    let response = ui.interact(
                        rect,
                        ui.make_persistent_id("synthetic-autoscroll"),
                        Sense::click_and_drag(),
                    );
                    select_pointer(ui, &response, &core, rect, cell, view).unwrap();
                },
            );
            output.textures_delta.clear();
        };
        frame(START - 1.0, Vec::new(), &mut view);
        let start = rect.min + cell * EDGE_FRACTION;
        frame(
            START,
            vec![
                Event::PointerMoved(start),
                Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            &mut view,
        );
        let above = pos2(start.x, rect.top() - OUTSIDE);
        frame(MOVE, vec![Event::PointerMoved(above)], &mut view);
        frame(BEFORE, Vec::new(), &mut view);
        assert_eq!(view.offset, 0);
        frame(TICK, Vec::new(), &mut view);
        assert_eq!(view.offset, HISTORY);
        assert_eq!(core.grid().unwrap().display_offset(), 0);
        assert!(
            core.selection_text(view.selection.as_ref().unwrap(), INPUT_BYTES)
                .unwrap()
                .unwrap()
                .starts_with("row6\n")
        );
    }

    #[test]
    fn selection_lifetime은_출력_scroll_trim과_독립_view_위치를_보존한다() {
        const RETAINED_ROWS: usize = 2;
        const FIRST_END: usize = 4;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            RETAINED_ROWS,
            Default::default(),
        )
        .unwrap();
        core.advance(b"first\r\nsecond\r\nthird\r\nfourth").unwrap();
        let context = egui::Context::default();
        let mut view = View {
            selection: Some(inclusive_selection(
                Point::new(Line(0), Column(0)),
                Point::new(Line(0), Column(FIRST_END)),
            )),
            ..Default::default()
        };
        let frame = |core: &TerminalCore, view: &mut View| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        pos2(0.0, 0.0),
                        vec2(SCREEN[0], SCREEN[1]),
                    )),
                    ..Default::default()
                },
                |ui| {
                    paint(
                        ui,
                        ui.available_rect_before_wrap(),
                        vec2(CELL[0], CELL[1]),
                        core,
                        view,
                        &appearance(),
                        false,
                    )
                    .unwrap();
                },
            );
            output.textures_delta.clear();
            view.selection
                .as_ref()
                .and_then(|selection| core.selection_text(selection, INPUT_BYTES).unwrap())
                .unwrap_or_default()
        };
        assert_eq!(frame(&core, &mut view), "first");
        core.advance(b"\r\nfifth").unwrap();
        assert_eq!(frame(&core, &mut view), "first");
        assert_eq!(view.offset, 0);
        view.offset = 1;
        core.advance(b"\r\nsixth").unwrap();
        assert_eq!(frame(&core, &mut view), "first");
        assert_eq!(view.offset, RETAINED_ROWS);
        core.advance(b"\r\nseventh").unwrap();
        assert_eq!(frame(&core, &mut view), "");
        assert!(view.selection.is_none());
    }

    #[test]
    fn selection_drag_policy는_down_column_seed_release_disabled와_epoch를_정리한다() {
        const START: f64 = 20.0;
        const MOVE: f64 = 20.02;
        const TICK: f64 = 20.051;
        const VIEW_TOP: f32 = 80.0;
        const SCREEN_SIZE: f32 = 256.0;
        const OUTPUT_ROWS: usize = 18;
        const SCROLLED: usize = 4;
        const COLUMN: usize = 2;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let output = (0..OUTPUT_ROWS)
            .map(|row| format!("row{row}"))
            .collect::<Vec<_>>()
            .join("\r\n");
        core.advance(output.as_bytes()).unwrap();
        let context = egui::Context::default();
        let cell = vec2(CELL[0], CELL[1]);
        let rect = Rect::from_min_size(
            pos2(CELL[0], VIEW_TOP),
            cell * vec2(f32::from(COLUMNS), f32::from(ROWS)),
        );
        let mut view = View {
            offset: SCROLLED,
            ..Default::default()
        };
        let frame = |core: &TerminalCore, now, events, view: &mut View, enabled| {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(now),
                    events,
                    screen_rect: Some(Rect::from_min_size(
                        pos2(0.0, 0.0),
                        vec2(SCREEN_SIZE, SCREEN_SIZE),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.add_enabled_ui(enabled, |ui| {
                        let response = ui.interact(
                            rect,
                            ui.make_persistent_id("synthetic-drag-policy"),
                            Sense::click_and_drag(),
                        );
                        select_pointer(ui, &response, core, rect, cell, view).unwrap();
                    });
                },
            );
            output.textures_delta.clear();
        };
        let button = |pos, pressed, modifiers| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        let start = rect.min + cell * EDGE_FRACTION;
        let below = pos2(start.x, rect.bottom() + EDGE_FRACTION);
        frame(&core, START - 1.0, Vec::new(), &mut view, true);
        frame(
            &core,
            START,
            vec![
                Event::PointerMoved(start),
                button(start, true, Default::default()),
            ],
            &mut view,
            true,
        );
        frame(
            &core,
            MOVE,
            vec![Event::PointerMoved(below)],
            &mut view,
            true,
        );
        frame(&core, TICK, Vec::new(), &mut view, true);
        assert_eq!(view.offset, SCROLLED - 1);
        let [_, (end, side)] = view.selection.as_ref().unwrap().native_anchors();
        assert_eq!(
            end,
            Point::new(
                Line(i32::from(ROWS) - (SCROLLED as i32 - 1)),
                Column(usize::from(COLUMNS))
            )
        );
        assert_eq!(side, Side::Left);
        frame(
            &core,
            TICK + 1.0,
            vec![button(below, false, Default::default())],
            &mut view,
            true,
        );
        assert!(view.drag.due.is_none());
        assert_eq!(
            view.selection.as_ref().unwrap().native_anchors()[1],
            (end, side)
        );
        frame(&core, TICK + 2.0, Vec::new(), &mut view, true);
        assert_eq!(view.offset, SCROLLED - 1);

        for (index, kind) in [SelectionType::Simple, SelectionType::Block]
            .into_iter()
            .enumerate()
        {
            let mut selected = Selection::new(kind, Point::new(Line(-1), Column(0)), Side::Left);
            selected.update(Point::new(Line(-1), Column(COLUMN)), Side::Left);
            view.selection = Some(selected);
            view.offset = SCROLLED;
            view.gesture = Gesture::default();
            drag_endpoint(&core, &mut view, 1, true).unwrap();
            let [_, (end, _)] = view.selection.as_ref().unwrap().native_anchors();
            let expected = if index == 0 {
                usize::from(COLUMNS)
            } else {
                COLUMN
            };
            assert_eq!(end, Point::new(Line(0), Column(expected)));
        }
        view.selection = Some(full_selection(&core).unwrap());
        view.is_all_selected = true;
        drag_endpoint(&core, &mut view, -1, true).unwrap();
        assert_eq!(view.selection, Some(full_selection(&core).unwrap()));
        view.is_all_selected = false;
        let seed = Seed {
            start: Point::new(Line(-1), Column(0)),
            length: usize::from(COLUMNS),
            end: None,
        };
        view.selection = Some(seed.selection(usize::from(COLUMNS)).unwrap());
        view.gesture = Gesture {
            granularity: Some(SelectionGranularity::Line),
            seed: Some(seed),
        };
        view.gesture.seed.as_mut().unwrap().end = Some(Point::new(Line(-2), Column(COLUMN)));
        drag_endpoint(&core, &mut view, -1, false).unwrap();
        assert_eq!(
            view.gesture.seed.unwrap().end,
            Some(Point::new(Line(-2), Column(0)))
        );
        drag_endpoint(&core, &mut view, -1, true).unwrap();
        assert_eq!(
            view.gesture.seed.unwrap().end,
            Some(Point::new(Line(-(SCROLLED as i32)), Column(0)))
        );
        view.drag = Drag {
            due: Some(TICK + 3.0),
            last: TICK + 2.0,
            amount: 1,
            has_end: true,
        };
        frame(&core, TICK + 3.0, Vec::new(), &mut view, false);
        assert!(view.drag.due.is_none());
        assert_eq!(view.offset, SCROLLED);
        view.drag = Drag {
            due: Some(TICK + 4.0),
            last: TICK + 3.0,
            amount: 1,
            has_end: true,
        };
        core.advance(b"\x1b[?1049h\x1b[?1049l").unwrap();
        frame(&core, TICK + 4.0, Vec::new(), &mut view, true);
        assert!(view.drag.due.is_none());
        assert!(view.selection.is_none());
        assert_eq!(view.offset, 0);
        assert_eq!(core.grid().unwrap().display_offset(), 0);
    }

    #[test]
    fn selection_pointer는_down_word_line_drag_shift와_독립_view를_보존한다() {
        const START: f64 = 10.0;
        const CLICK_STEP: f64 = 0.1;
        const RELEASE_STEP: f64 = 0.01;
        const GROUP_STEP: f64 = 10.0;
        const FIRST_WORD: usize = 1;
        const FIRST_WORD_END: usize = 3;
        const NEXT_WORD: usize = 5;
        const SHIFT_END: usize = 7;
        const NEXT_LINE: usize = 1;
        const WRAPPED_LINE: usize = 3;
        const EDGE_FRACTION: f32 = 0.1;
        const SURFACE_ROWS: u16 = 8;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: SURFACE_ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        core.advance(b"one two\r\nline2\r\nabcdefghijklmnop\r\nlast")
            .unwrap();
        let context = egui::Context::default();
        let cell = vec2(CELL[0], CELL[1]);
        let rect = Rect::from_min_size(
            pos2(0.0, 0.0),
            cell * vec2(f32::from(COLUMNS), f32::from(SURFACE_ROWS)),
        );
        let position = |column: usize, row: usize| {
            rect.min
                + cell
                    * vec2(
                        column as f32 + SELECTION_HALF_CELL,
                        row as f32 + SELECTION_HALF_CELL,
                    )
        };
        let button = |pos, pressed, modifiers| Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        let frame = |time, events, view: &mut View| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(rect),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    let response = ui.interact(
                        rect,
                        ui.make_persistent_id("synthetic-selection"),
                        Sense::click_and_drag(),
                    );
                    select_pointer(ui, &response, &core, rect, cell, view).unwrap();
                    paint(ui, rect, cell, &core, view, &appearance(), false).unwrap();
                },
            );
            output.textures_delta.clear();
            view.selection
                .as_ref()
                .and_then(|selection| core.selection_text(selection, INPUT_BYTES).unwrap())
                .unwrap_or_default()
        };
        let point = position(FIRST_WORD, 0);
        let next = position(NEXT_WORD, 0);
        let mut words = View::default();
        drop(frame(START - CLICK_STEP, Vec::new(), &mut words));
        assert_eq!(
            frame(
                START,
                vec![
                    Event::PointerMoved(point),
                    button(point, true, Default::default())
                ],
                &mut words
            ),
            ""
        );
        assert_eq!(
            frame(
                START + RELEASE_STEP,
                vec![button(point, false, Default::default())],
                &mut words
            ),
            ""
        );
        assert_eq!(
            frame(
                START + CLICK_STEP,
                vec![button(point, true, Default::default())],
                &mut words
            ),
            "one"
        );
        assert_eq!(
            frame(
                START + CLICK_STEP + RELEASE_STEP,
                vec![Event::PointerMoved(next)],
                &mut words
            ),
            "one two"
        );
        assert_eq!(
            frame(
                START + CLICK_STEP + RELEASE_STEP * f64::from(DOUBLE_CLICK_COUNT),
                vec![button(next, false, Default::default())],
                &mut words
            ),
            "one two"
        );
        let saved = words.selection.clone();
        let mut lines = View::default();
        let group = START + GROUP_STEP;
        for count in 0..usize::from(TRIPLE_CLICK_COUNT) {
            let now = group + count as f64 * CLICK_STEP;
            let expected = match count {
                0 => "",
                1 => "one",
                _ => "one two",
            };
            assert_eq!(
                frame(
                    now,
                    vec![
                        Event::PointerMoved(point),
                        button(point, true, Default::default())
                    ],
                    &mut lines
                ),
                expected
            );
            if count + 1 < usize::from(TRIPLE_CLICK_COUNT) {
                assert_eq!(
                    frame(
                        now + RELEASE_STEP,
                        vec![button(point, false, Default::default())],
                        &mut lines
                    ),
                    expected
                );
            }
        }
        let dragged = group + f64::from(TRIPLE_CLICK_COUNT) * CLICK_STEP;
        let line = position(0, NEXT_LINE);
        assert_eq!(
            frame(dragged, vec![Event::PointerMoved(line)], &mut lines),
            "one two\nline2"
        );
        assert_eq!(
            frame(
                dragged + RELEASE_STEP,
                vec![button(line, false, Default::default())],
                &mut lines
            ),
            "one two\nline2"
        );
        let wrapped = position(0, WRAPPED_LINE);
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        assert_eq!(
            frame(
                dragged + CLICK_STEP,
                vec![Event::PointerMoved(wrapped), button(wrapped, true, shift)],
                &mut lines
            ),
            "one two\nline2\nabcdefghijkl"
        );
        drop(frame(
            dragged + CLICK_STEP + RELEASE_STEP,
            vec![button(wrapped, false, shift)],
            &mut lines,
        ));
        assert_eq!(words.selection, saved);
        let mut normal = View::default();
        let plain = group + GROUP_STEP;
        let start = rect.min + cell * EDGE_FRACTION;
        let end = rect.min + cell * vec2(FIRST_WORD_END as f32 + EDGE_FRACTION, EDGE_FRACTION);
        drop(frame(
            plain,
            vec![
                Event::PointerMoved(start),
                button(start, true, Default::default()),
            ],
            &mut normal,
        ));
        assert_eq!(
            frame(
                plain + CLICK_STEP,
                vec![Event::PointerMoved(end)],
                &mut normal
            ),
            "one"
        );
        drop(frame(
            plain + CLICK_STEP + RELEASE_STEP,
            vec![button(end, false, Default::default())],
            &mut normal,
        ));
        let shift_end = rect.min + cell * vec2(SHIFT_END as f32 + EDGE_FRACTION, EDGE_FRACTION);
        assert_eq!(
            frame(
                plain + CLICK_STEP * f64::from(DOUBLE_CLICK_COUNT),
                vec![
                    Event::PointerMoved(shift_end),
                    button(shift_end, true, shift)
                ],
                &mut normal
            ),
            "one two"
        );
        drop(frame(
            plain + CLICK_STEP * f64::from(DOUBLE_CLICK_COUNT) + RELEASE_STEP,
            vec![button(shift_end, false, shift)],
            &mut normal,
        ));
        assert_eq!(core.grid().unwrap().display_offset(), 0);
    }

    #[test]
    fn selection_owner는_focus_local_reject를_보존하고_user_input으로_모든_view를_정리한다() {
        use taide_native_terminal::session::SharedTerminal;
        let terminal = SharedTerminal::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        terminal
            .advance(b"first\r\nsecond\r\nthird\r\nfourth\r\nfifth\x1b[?1004h")
            .unwrap();
        let selected = inclusive_selection(
            Point::new(Line(-1), Column(0)),
            Point::new(Line(-1), Column(4)),
        );
        let mut first = View {
            offset: 1,
            selection: Some(selected.clone()),
            ..Default::default()
        };
        let mut second = View {
            offset: 1,
            selection: Some(selected),
            ..Default::default()
        };
        let reconcile = |view: &mut View| {
            terminal
                .snapshot(|snapshot| reconcile_view(snapshot.core, view))
                .unwrap()
                .unwrap();
        };
        reconcile(&mut first);
        reconcile(&mut second);
        assert!(matches!(
            terminal.encode_input(NativeInput::Focus(true), INPUT_BYTES),
            Ok(InputAction::Write(_))
        ));
        assert_eq!(
            terminal.encode_input(
                NativeInput::Key {
                    key: Key::Character('a'),
                    modifiers: Modifiers {
                        command: true,
                        ..Default::default()
                    }
                },
                INPUT_BYTES
            ),
            Ok(InputAction::SelectAll)
        );
        assert!(
            terminal
                .encode_input(NativeInput::CommittedText("too long"), 1)
                .is_err()
        );
        reconcile(&mut first);
        reconcile(&mut second);
        assert!(first.selection.is_some() && second.selection.is_some());
        assert_eq!(first.offset, 1);
        assert_eq!(second.offset, 1);
        assert!(matches!(
            terminal.encode_input(NativeInput::CommittedText("typed"), INPUT_BYTES),
            Ok(InputAction::Write(_))
        ));
        reconcile(&mut first);
        reconcile(&mut second);
        assert!(first.selection.is_none() && second.selection.is_none());
        assert_eq!(first.offset, 0);
        assert_eq!(second.offset, 0);
    }

    #[test]
    fn selection_all은_출력과_trim_후에도_현재_전체_버퍼를_선택한다() {
        const RETAINED_ROWS: usize = 1;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            RETAINED_ROWS,
            Default::default(),
        )
        .unwrap();
        core.advance(b"first\r\nsecond\r\nthird\r\nfourth").unwrap();
        let mut view = View {
            selection: Some(full_selection(&core).unwrap()),
            is_all_selected: true,
            ..Default::default()
        };
        reconcile_view(&core, &mut view).unwrap();
        core.advance(b"\r\nfifth\r\nsixth").unwrap();
        reconcile_view(&core, &mut view).unwrap();
        let selected = core
            .selection_text(view.selection.as_ref().unwrap(), INPUT_BYTES)
            .unwrap()
            .unwrap();
        assert_eq!(selected, "second\nthird\nfourth\nfifth\nsixth");
        core.advance(b"\x1b[3J").unwrap();
        reconcile_view(&core, &mut view).unwrap();
        assert_eq!(
            core.selection_text(view.selection.as_ref().unwrap(), INPUT_BYTES)
                .unwrap()
                .unwrap(),
            "third\nfourth\nfifth\nsixth"
        );
        assert!(view.is_all_selected);
        core.advance(b"\x1bc").unwrap();
        reconcile_view(&core, &mut view).unwrap();
        assert!(!view.is_all_selected);
        assert!(view.selection.is_none());
    }

    #[test]
    fn selection_seed는_reflow_trim_후에도_원본_word_최소_셀_길이를_유지한다() {
        const RETAINED_ROWS: usize = 2;
        const NARROW_COLUMNS: u16 = 6;
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            RETAINED_ROWS,
            Default::default(),
        )
        .unwrap();
        core.advance(b"abcdefghijklmnopqrstuvwx\r\nline2\r\nline3\r\nline4")
            .unwrap();
        let seed = core
            .selection_at(Point::new(Line(-1), Column(0)), SelectionGranularity::Word)
            .unwrap();
        let seed = Seed::new(seed, usize::from(COLUMNS)).unwrap();
        let mut view = View {
            selection: Some(seed.selection(usize::from(COLUMNS)).unwrap()),
            gesture: Gesture {
                granularity: Some(SelectionGranularity::Word),
                seed: Some(seed),
            },
            ..Default::default()
        };
        reconcile_view(&core, &mut view).unwrap();
        core.resize(Size {
            columns: NARROW_COLUMNS,
            rows: ROWS,
        })
        .unwrap();
        reconcile_view(&core, &mut view).unwrap();
        assert_eq!(
            core.selection_text(view.selection.as_ref().unwrap(), INPUT_BYTES)
                .unwrap()
                .unwrap(),
            "ghijklmnopqrstuvwx\nline2"
        );
    }

    #[test]
    fn selection_model은_원본_정방향_역방향_끝점과_trim_길이를_보존한다() {
        const MINIMUM: usize = 4;
        const LONG_MINIMUM: usize = 24;
        let core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let seed = Seed {
            start: Point::new(Line(1), Column(1)),
            length: MINIMUM,
            end: None,
        };
        let range = |seed: Seed| {
            core.selection_range(&seed.selection(usize::from(COLUMNS)).unwrap())
                .unwrap()
                .unwrap()
        };
        assert_eq!(
            range(seed),
            SelectionRange::new(
                Point::new(Line(1), Column(1)),
                Point::new(Line(1), Column(MINIMUM)),
                false
            )
        );
        assert_eq!(
            range(Seed {
                end: Some(Point::new(Line(0), Column(0))),
                ..seed
            }),
            SelectionRange::new(
                Point::new(Line(0), Column(0)),
                Point::new(Line(1), Column(MINIMUM)),
                false
            )
        );
        assert_eq!(
            range(Seed {
                end: Some(Point::new(Line(1), Column(2))),
                ..seed
            }),
            range(seed)
        );
        assert_eq!(
            range(Seed {
                end: Some(Point::new(Line(1), Column(8))),
                ..seed
            })
            .end,
            Point::new(Line(1), Column(7))
        );
        assert_eq!(
            range(Seed {
                end: Some(Point::new(Line(2), Column(0))),
                ..seed
            })
            .end,
            Point::new(Line(1), Column(usize::from(COLUMNS) - 1))
        );
        let long = Seed {
            start: Point::new(Line(0), Column(0)),
            length: LONG_MINIMUM,
            end: None,
        };
        assert_eq!(
            range(long).end,
            Point::new(Line(1), Column(usize::from(COLUMNS) - 1))
        );
        assert_eq!(
            range(Seed {
                end: Some(Point::new(Line(0), Column(1))),
                ..long
            }),
            range(long)
        );
        let trimmed = long.rebase(i64::from(ROWS), Line(0)).unwrap().unwrap();
        assert_eq!(trimmed.start, long.start);
        assert_eq!(trimmed.length, long.length);
        assert!(
            Seed {
                end: Some(Point::new(Line(0), Column(1))),
                ..long
            }
            .rebase(i64::from(ROWS), Line(0))
            .unwrap()
            .is_none()
        );
    }

    fn appearance() -> Appearance {
        Appearance {
            command_colors: Default::default(),
            font: FontId::monospace(FONT_SIZE),
            ansi: [Rgb { r: 1, g: 2, b: 3 }; ANSI_COUNT],
            foreground: Rgb {
                r: 240,
                g: 240,
                b: 240,
            },
            background: Rgb { r: 0, g: 0, b: 0 },
            cursor: Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            selection: Color32::BLUE,
        }
    }

    #[test]
    fn native_presentation_refresh_palette는_기존_actor_callback과_창별수명을_보존한다() {
        let context = egui::Context::default();
        let initial = appearance();
        let mut views = Views::default();
        views.set_palette(&initial).unwrap();
        let geometry = Arc::new(Mutex::new(WindowSize {
            num_lines: 1,
            num_cols: 1,
            cell_width: 1,
            cell_height: 1,
        }));
        let mut ports = Vec::new();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            for _ in 0..2 {
                ports.push(effect_ports(
                    ui,
                    &initial,
                    geometry.clone(),
                    views.palette.clone(),
                ));
            }
        });
        output.textures_delta.clear();
        let foreground = NamedColor::Foreground as usize;
        for port in &ports {
            assert_eq!((port.color)(foreground).unwrap(), initial.foreground);
        }
        let mut current = initial.clone();
        current.foreground = Rgb { r: 3, g: 4, b: 5 };
        current.ansi[0] = Rgb { r: 6, g: 7, b: 8 };
        views.set_palette(&current).unwrap();
        for port in &ports {
            assert_eq!((port.color)(foreground).unwrap(), current.foreground);
            assert_eq!((port.color)(0).unwrap(), current.ansi[0]);
            assert!((port.color)(INDEXED_END + ANSI_COUNT).is_err());
        }
        let mut other = Views::default();
        other.set_palette(&initial).unwrap();
        assert_eq!(
            other.palette.lock().unwrap().as_ref().unwrap().foreground,
            initial.foreground
        );
        drop(views);
        for port in &ports {
            assert_eq!((port.color)(foreground).unwrap(), current.foreground);
        }
        assert_eq!(initial.foreground, appearance().foreground);
    }

    #[test]
    fn cursor_blink은_시간_focus_위치_override와_폰트_크기_변경을_보존한다() {
        const START: f64 = 10.0;
        const STEP: f64 = 0.6;
        const MIDDLE: f64 = 0.3;
        const LARGE_FONT: u32 = 24;
        let point = Point::new(Line(0), Column(0));
        let mut style = CursorStyle {
            shape: CursorShape::Beam,
            blinking: true,
        };
        let mut blink = Blink::default();
        assert_eq!(
            blink.visible(START, point, style, true),
            (true, Some(CURSOR_BLINK_INTERVAL))
        );
        assert!(!blink.visible(START + STEP + MIDDLE, point, style, true).0);
        assert!(
            blink
                .visible(START + STEP * 2.0 + MIDDLE, point, style, true)
                .0
        );
        assert_eq!(
            blink.visible(START + STEP * 2.0 + MIDDLE, point, style, false),
            (true, None)
        );
        assert!(
            blink
                .visible(START + STEP * 2.0 + MIDDLE, point, style, true)
                .0
        );
        style.blinking = false;
        assert_eq!(
            blink.visible(START + STEP * 4.0, point, style, true),
            (true, None)
        );
        style.blinking = true;
        assert!(blink.visible(START + STEP * 4.0, point, style, true).0);
        assert!(
            !blink
                .visible(START + STEP * 4.0 + STEP + MIDDLE, point, style, true)
                .0
        );
        assert!(
            blink
                .visible(
                    START + STEP * 4.0 + STEP + MIDDLE,
                    Point::new(Line(0), Column(1)),
                    style,
                    true
                )
                .0
        );
        assert_eq!(blink.visible(f64::NAN, point, style, true), (true, None));
        assert!(blink.visible(START, point, style, true).0);
        assert!(blink.visible(0.0, point, style, true).0);
        let mut appearance = appearance();
        let family = appearance.font.family.clone();
        assert!(!appearance.set_font_size(FONT_SIZE as u32));
        assert!(appearance.set_font_size(LARGE_FONT));
        assert_eq!(appearance.font.size, LARGE_FONT as f32);
        assert_eq!(appearance.font.family, family);
        assert!(appearance.set_font_size(0));
        assert_eq!(appearance.font.size, MIN_FONT_SIZE);
    }

    #[test]
    fn selection_copy는_출력_공백을_보존하고_미출력_blank_tab과_nbsp를_xterm처럼_처리한다() {
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        core.advance("abc   \r\nA\tB\r\n한e\u{301}\u{a0}Z".as_bytes())
            .unwrap();
        assert_eq!(
            selected_text(
                &core,
                (
                    Point::new(Line(0), Column(0)),
                    Point::new(Line(0), Column(usize::from(COLUMNS) - 1))
                )
            )
            .unwrap(),
            "abc   "
        );
        assert_eq!(
            selected_text(
                &core,
                (
                    Point::new(Line(1), Column(0)),
                    Point::new(Line(1), Column(usize::from(COLUMNS) - 1))
                )
            )
            .unwrap(),
            "A       B"
        );
        assert_eq!(
            selected_text(
                &core,
                (
                    Point::new(Line(2), Column(0)),
                    Point::new(Line(2), Column(usize::from(COLUMNS) - 1))
                )
            )
            .unwrap(),
            "한e\u{301} Z"
        );
        let cell = vec2(CELL[0], CELL[1]);
        let rect = Rect::from_min_size(
            pos2(0.0, 0.0),
            cell * vec2(f32::from(COLUMNS), f32::from(ROWS)),
        );
        let anchor = |x: f32, y: f32| {
            selection_anchor(&core, rect, cell, 0, pos2(x * cell.x, y * cell.y)).unwrap()
        };
        assert_eq!(
            anchor(0.0, 0.0),
            (Point::new(Line(0), Column(0)), Side::Left)
        );
        assert_eq!(
            anchor(SELECTION_HALF_CELL, 1.0),
            (Point::new(Line(0), Column(0)), Side::Left)
        );
        assert_eq!(
            anchor(1.0, 1.0),
            (Point::new(Line(0), Column(1)), Side::Left)
        );
        assert_eq!(
            anchor(1.0, 3.0),
            (Point::new(Line(2), Column(2)), Side::Left)
        );
        assert_eq!(
            anchor(f32::from(COLUMNS), f32::from(ROWS)),
            (
                Point::new(Line(i32::from(ROWS) - 1), Column(usize::from(COLUMNS) - 1)),
                Side::Right
            )
        );
        let (start, start_side) = anchor(0.0, 1.0);
        let (end, end_side) = anchor(3.0, 1.0);
        let mut selected = Selection::new(SelectionType::Simple, start, start_side);
        selected.update(end, end_side);
        assert_eq!(
            core.selection_text(&selected, INPUT_BYTES)
                .unwrap()
                .unwrap(),
            "abc"
        );
        let mut column = Selection::new(
            SelectionType::Block,
            Point::new(Line(0), Column(0)),
            Side::Left,
        );
        column.update(Point::new(Line(1), Column(1)), Side::Left);
        assert_eq!(
            core.selection_text(&column, INPUT_BYTES).unwrap().unwrap(),
            "a\nA"
        );
        assert!(core.selection_text(&selected, 1).is_err());
    }

    #[test]
    fn pending_input은_원본의_utf16_tail과_preedit_paste_키_경계를_보존한다() {
        let mut view = View::default();
        let text = format!("𐐀{}", "a".repeat(PENDING_INPUT_UNITS - 1));
        assert!(buffer_starting_input(&mut view, &Event::Text(text)));
        assert_eq!(view.pending.len(), PENDING_INPUT_UNITS);
        let units = view.pending.iter().copied().collect::<Vec<_>>();
        assert_eq!(
            String::from_utf16_lossy(&units),
            format!("\u{fffd}{}", "a".repeat(PENDING_INPUT_UNITS - 1))
        );
        view.pending.clear();
        assert!(buffer_starting_input(
            &mut view,
            &Event::Ime(ImeEvent::Preedit {
                text: "한".into(),
                active_range_chars: None
            })
        ));
        assert!(buffer_starting_input(
            &mut view,
            &Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default()
            }
        ));
        assert!(view.pending.is_empty());
        assert!(buffer_starting_input(
            &mut view,
            &Event::Ime(ImeEvent::Commit("한".into()))
        ));
        assert!(buffer_starting_input(
            &mut view,
            &Event::Paste("\r\n日\n".into())
        ));
        assert!(buffer_starting_input(
            &mut view,
            &Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    shift: true,
                    ..Default::default()
                }
            }
        ));
        let units = view.pending.iter().copied().collect::<Vec<_>>();
        assert_eq!(String::from_utf16_lossy(&units), "한\r日\r\n");
    }

    #[test]
    fn sync_중_새_ui_frame은_마지막_공개_화면을_보존한다() {
        let appearance = appearance();
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        let mut view = View::default();
        let context = egui::Context::default();
        let frame = |core: &TerminalCore, view: &mut View| {
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        pos2(0.0, 0.0),
                        vec2(SCREEN[0], SCREEN[1]),
                    )),
                    ..Default::default()
                },
                |ui| {
                    paint(
                        ui,
                        ui.available_rect_before_wrap(),
                        vec2(CELL[0], CELL[1]),
                        core,
                        view,
                        &appearance,
                        false,
                    )
                    .unwrap();
                },
            );
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.job.text.as_str()),
                    _ => None,
                })
                .collect::<String>();
            output.textures_delta.clear();
            text
        };
        core.advance(b"old\x1b[?25l").unwrap();
        assert!(frame(&core, &mut view).contains("old"));
        core.advance(b"\x1b[?2026h\x1b[Hnew").unwrap();
        assert!(core.sync_deadline().is_some());
        let held = frame(&core, &mut view);
        assert!(held.contains("old"), "unpublished screen leaked: {held}");
        assert!(!held.contains("new"));
        core.flush_sync().unwrap();
        let flushed = frame(&core, &mut view);
        assert!(flushed.contains("new"));
        assert!(!flushed.contains("old"));
    }

    #[test]
    fn borrowed_grid의_palette_wide_nfd와_독립_scroll_선택을_그린다() {
        let appearance = appearance();
        assert_eq!(appearance.indexed(16).unwrap(), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(
            appearance.indexed(231).unwrap(),
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(appearance.indexed(232).unwrap(), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(
            appearance.indexed(255).unwrap(),
            Rgb {
                r: 238,
                g: 238,
                b: 238
            }
        );
        assert!(appearance.indexed(269).is_err());
        assert!(measured(Rect::ZERO, vec2(CELL[0], CELL[1]), true).is_none());
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        core.advance(
            "\x1b]4;1;#aabbcc\x07\x1b[31m한e\u{301}\x1b[0m\r\nsecond\r\nthird\r\nfourth\r\nfifth"
                .as_bytes(),
        )
        .unwrap();
        let mut view = View {
            offset: 1,
            selection: Some(inclusive_selection(
                Point::new(Line(-1), Column(0)),
                Point::new(Line(-1), Column(2)),
            )),
            ..Default::default()
        };
        assert_eq!(
            core.selection_text(view.selection.as_ref().unwrap(), INPUT_BYTES)
                .unwrap()
                .unwrap(),
            "한e\u{301}"
        );
        let context = egui::Context::default();
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                ..Default::default()
            },
            |ui| {
                let rect = ui.available_rect_before_wrap();
                paint(
                    ui,
                    rect,
                    vec2(CELL[0], CELL[1]),
                    &core,
                    &mut view,
                    &appearance,
                    true,
                )
                .unwrap();
            },
        );
        let mut saw_selection = false;
        let mut saw_wide = false;
        let mut saw_nfd = false;
        for shape in &output.shapes {
            if let egui::Shape::Rect(rect) = &shape.shape {
                saw_selection |= rect.fill == appearance.selection;
            }
            if let egui::Shape::Text(text) = &shape.shape {
                if text.galley.job.text == "한" {
                    assert_eq!(text.fallback_color, Color32::from_rgb(0xaa, 0xbb, 0xcc));
                    saw_wide = true;
                }
                saw_nfd |= text.galley.job.text == "e\u{301}";
            }
        }
        assert!(saw_selection && saw_wide && saw_nfd);
        assert_eq!(core.grid().unwrap().display_offset(), 0);
        assert_eq!(view.offset, 1);
        output.textures_delta.clear();
    }

    #[test]
    fn 종료_화면_문구는_로컬라이즈된_실패와_종료_문구만_사용한다() {
        let locale = ResolvedLocale {
            id: "en".into(),
            name: "English".into(),
            warnings: Vec::new(),
            messages: std::collections::BTreeMap::from([
                ("terminal.processExited".into(), "Process exited".into()),
                ("test.failure".into(), "Cannot spawn {{target}}".into()),
            ]),
        };
        let spawn = AppError::localized(
            taide_model::error::AppErrorKind::Io,
            "test.failure",
            "fallback",
        )
        .with_arg("target", "synthetic-shell");
        assert_eq!(
            ended_message(&locale, Some(&spawn), Phase::Running).as_deref(),
            Some("Cannot spawn synthetic-shell")
        );
        assert_eq!(
            ended_message(&locale, None, Phase::Exited(Some(3))).as_deref(),
            Some("Process exited (3)")
        );
        assert_eq!(
            ended_message(&locale, None, Phase::Exited(None)).as_deref(),
            Some("Process exited")
        );
        for failure in [
            taide_native_terminal::session::Failure::Parser,
            taide_native_terminal::session::Failure::Delivery,
            taide_native_terminal::session::Failure::Spawn,
        ] {
            assert_eq!(
                ended_message(&locale, None, Phase::Failed(failure)).as_deref(),
                Some("Process exited")
            );
        }
        assert_eq!(ended_message(&locale, None, Phase::Running), None);
        assert_eq!(ended_message(&locale, None, Phase::Draining(Some(0))), None);
    }
}
