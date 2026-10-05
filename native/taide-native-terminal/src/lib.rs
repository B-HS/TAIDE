use std::sync::{Arc, Mutex};

use alacritty_terminal::{
    event::{Event, EventListener},
    grid::{Dimensions, Grid, Scroll},
    term::{Config, Osc52, RenderableContent, Term, TermDamage, TermMode, cell::Cell},
    vte::{
        Params,
        ansi::{Processor, StreamObserver},
    },
};
use taide_infra::terminal_scan::{MAX_TITLE_BYTES, ScanEvent, classify_osc_payload};
use taide_model::error::{AppError, AppResult};
use taide_native_retained::{Report, RetainedBytes, Visitor, measure};

pub mod input;
mod selection_bounds;
mod selection_text;
pub mod session;
mod stream;

pub use alacritty_terminal::event::{
    ColorQuery, Event as TerminalEvent, TextAreaSizeQuery, WindowSize,
};
pub use alacritty_terminal::grid::Dimensions as GridDimensions;
pub use alacritty_terminal::index::{Column, Line, Point, Side};
pub use alacritty_terminal::selection::{Selection, SelectionRange, SelectionType};
pub use alacritty_terminal::term::NativeSelectionStamp as SelectionStamp;
pub use alacritty_terminal::term::native_commands::{
    CommandBlock, CommandColors, CommandDecoration, MAX_COMMAND_BLOCKS,
};
pub use alacritty_terminal::term::{TermMode as Mode, cell::Flags as CellFlags, color::Colors};
pub use alacritty_terminal::vte::ansi::{Color, CursorShape, CursorStyle, NamedColor, Rgb};
pub use selection_bounds::SelectionGranularity;

const DEFAULT_GRID_BYTES: usize = 128 * 1024 * 1024;
const DEFAULT_FEED_BYTES: usize = 64 * 1024;
const DEFAULT_EFFECT_BYTES: usize = 64 * 1024;
const DEFAULT_EFFECT_COUNT: usize = 256;
const DEFAULT_RETAINED_VISITS: usize = 1_000_000;
const GRID_COUNT: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub columns: u16,
    pub rows: u16,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.screen_lines()
    }
    fn screen_lines(&self) -> usize {
        usize::from(self.rows)
    }
    fn columns(&self) -> usize {
        usize::from(self.columns)
    }
}

#[derive(Clone, Copy, RetainedBytes)]
pub struct Limits {
    pub grid_bytes: usize,
    pub feed_bytes: usize,
    pub effect_bytes: usize,
    pub effect_count: usize,
    pub retained_bytes: usize,
    pub retained_visits: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            grid_bytes: DEFAULT_GRID_BYTES,
            feed_bytes: DEFAULT_FEED_BYTES,
            effect_bytes: DEFAULT_EFFECT_BYTES,
            effect_count: DEFAULT_EFFECT_COUNT,
            retained_bytes: DEFAULT_GRID_BYTES,
            retained_visits: DEFAULT_RETAINED_VISITS,
        }
    }
}

pub enum Effect {
    Stream(ScanEvent),
    Terminal(Event),
}

impl RetainedBytes for Effect {
    fn visit<'value>(
        &'value self,
        visitor: &mut Visitor<'value, '_>,
    ) -> Result<(), taide_native_retained::Error> {
        match self {
            Self::Terminal(event) => visitor.push(event),
            Self::Stream(
                ScanEvent::Cwd(text)
                | ScanEvent::Title(text)
                | ScanEvent::AgentEvent(text)
                | ScanEvent::Notification9(text),
            ) => visitor.push(text),
            Self::Stream(ScanEvent::CommandMarker(marker)) => {
                match marker {
                    taide_infra::shell_integration::CommandMarker::OutputStart => {}
                    taide_infra::shell_integration::CommandMarker::Finished { exit_code } => {
                        visitor.push(exit_code)?
                    }
                }
                Ok(())
            }
        }
    }
}

#[derive(RetainedBytes)]
pub struct Outcome {
    pub effects: Vec<Effect>,
    pub text: String,
    pub overlap: String,
}

#[derive(RetainedBytes)]
struct Pending {
    effects: Vec<Effect>,
    bytes: usize,
    failed: bool,
    limits: Limits,
    stream: stream::NormalizedStream,
}

impl Pending {
    fn retire(&mut self) {
        self.effects = Vec::new();
        self.bytes = 0;
        self.stream = stream::NormalizedStream::default();
        self.failed = true;
    }
    fn push_stream(&mut self, effect: ScanEvent) {
        let bytes = match &effect {
            ScanEvent::Cwd(text)
            | ScanEvent::Title(text)
            | ScanEvent::AgentEvent(text)
            | ScanEvent::Notification9(text) => text.capacity(),
            ScanEvent::CommandMarker(_) => 0,
        };
        self.push(Effect::Stream(effect), bytes);
    }

    fn push(&mut self, effect: Effect, payload: usize) {
        if self.failed {
            return;
        }
        let bytes = self
            .bytes
            .checked_add(payload)
            .and_then(|bytes| bytes.checked_add(size_of::<Effect>()));
        let Some(bytes) = bytes.filter(|bytes| {
            *bytes <= self.limits.effect_bytes && self.effects.len() < self.limits.effect_count
        }) else {
            self.retire();
            return;
        };
        self.effects.push(effect);
        self.bytes = bytes;
    }
}

#[derive(Clone, RetainedBytes)]
struct Listener(Arc<Mutex<Pending>>);

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        if matches!(
            event,
            Event::ClipboardLoad(..)
                | Event::ClipboardStore(..)
                | Event::MouseCursorDirty
                | Event::Wakeup
                | Event::CursorBlinkingChange
        ) {
            return;
        }
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if let Event::Title(title) = &event {
            if title.len() <= MAX_TITLE_BYTES
                && let Some(effect) = classify_osc_payload(&format!("2;{title}"))
            {
                pending.push_stream(effect);
            }
            return;
        }
        let payload = match &event {
            Event::PtyWrite(bytes) => Some(bytes.capacity()),
            Event::NativeColorRequest(_, query) => query
                .prefix
                .capacity()
                .checked_add(query.terminator.capacity()),
            _ => Some(0),
        };
        let Some(payload) = payload else {
            pending.retire();
            return;
        };
        pending.push(Effect::Terminal(event), payload);
    }
}

#[derive(RetainedBytes)]
struct Observer(Arc<Mutex<Pending>>);

impl std::fmt::Debug for Observer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TerminalObserver")
    }
}

impl StreamObserver for Observer {
    fn visit_retained<'value>(
        &'value self,
        visitor: &mut Visitor<'value, '_>,
    ) -> Result<(), taide_native_retained::Error> {
        visitor.push(self)
    }

    fn observe(&mut self, _: &[&[u8]], _: bool) {}

    fn observe_print(&mut self, character: char) {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if !pending.failed {
            pending.stream.print(character);
        }
    }

    fn observe_execute(&mut self, byte: u8) {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if !pending.failed {
            pending.stream.execute(byte);
        }
    }

    fn observe_csi(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        is_ignored: bool,
        action: char,
    ) {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if !pending.failed {
            pending
                .stream
                .csi(params, intermediates, is_ignored, action);
        }
    }

    fn observe_payload(&mut self, payload: &[u8], _: bool) {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if pending.failed {
            return;
        }
        let Some(effect) = std::str::from_utf8(payload)
            .ok()
            .and_then(classify_osc_payload)
        else {
            return;
        };
        if !matches!(effect, ScanEvent::Title(_)) {
            pending.push_stream(effect);
        }
    }
}

#[derive(RetainedBytes)]
pub struct TerminalCore {
    terminal: Option<Term<Listener>>,
    parser: Processor,
    pending: Arc<Mutex<Pending>>,
    history: usize,
    limits: Limits,
}

fn invalid(message: &str) -> AppError {
    AppError::InvalidArgument(message.into())
}

fn admit(size: Size, history: usize, limits: Limits) -> AppResult<()> {
    if size.columns() < alacritty_terminal::term::MIN_COLUMNS
        || size.screen_lines() < alacritty_terminal::term::MIN_SCREEN_LINES
        || limits.feed_bytes == 0
        || limits.effect_bytes == 0
        || limits.effect_count == 0
        || limits.retained_bytes == 0
        || limits.retained_visits == 0
    {
        return Err(invalid("terminal size or effect limits are invalid"));
    }
    let cells = size
        .screen_lines()
        .checked_mul(GRID_COUNT)
        .and_then(|rows| rows.checked_add(history))
        .and_then(|rows| rows.checked_mul(size.columns()))
        .and_then(|cells| cells.checked_mul(size_of::<Cell>()));
    if cells.is_none_or(|bytes| bytes > limits.grid_bytes) {
        return Err(invalid("terminal grid exceeds its admission budget"));
    }
    Ok(())
}

impl TerminalCore {
    pub fn new(size: Size, history: usize, limits: Limits) -> AppResult<Self> {
        admit(size, history, limits)?;
        let pending = Arc::new(Mutex::new(Pending {
            effects: Vec::new(),
            bytes: 0,
            failed: false,
            limits,
            stream: stream::NormalizedStream::default(),
        }));
        let config = Config {
            scrolling_history: history,
            osc52: Osc52::Disabled,
            ..Default::default()
        };
        let terminal = Term::new(config, &size, Listener(pending.clone()));
        let mut parser = Processor::new();
        parser.set_stream_observer(Some(Box::new(Observer(pending.clone()))));
        let core = Self {
            terminal: Some(terminal),
            parser,
            pending,
            history,
            limits,
        };
        core.retained()?;
        Ok(core)
    }

    pub fn advance(&mut self, bytes: &[u8]) -> AppResult<Vec<Effect>> {
        Ok(self.advance_outcome(bytes)?.effects)
    }

    pub fn encode_input(
        &self,
        input: input::NativeInput<'_>,
        capacity: usize,
    ) -> Result<input::InputAction, input::InputError> {
        let mode = self.mode().map_err(|_| input::InputError::Retired)?;
        if let input::NativeInput::Mouse(mouse) = &input {
            let grid = self.grid().map_err(|_| input::InputError::Retired)?;
            if usize::from(mouse.column) >= grid.columns()
                || usize::from(mouse.row) >= grid.screen_lines()
            {
                return Ok(input::InputAction::Ignore);
            }
            let mut mouse_mode = input::MouseMode::from_mode(mode);
            if mode.contains(TermMode::MOUSE_X10) {
                mouse_mode.protocol = input::MouseProtocol::X10;
            }
            if mode.contains(TermMode::SGR_PIXEL_MOUSE) {
                mouse_mode.encoding = input::MouseEncoding::SgrPixels;
            }
            return input::encode_mouse(mouse_mode, *mouse, capacity);
        }
        input::encode_input(mode, input, capacity)
    }

    pub fn advance_outcome(&mut self, bytes: &[u8]) -> AppResult<Outcome> {
        if bytes.len() > self.limits.feed_bytes {
            return Err(invalid("terminal feed exceeds its chunk limit"));
        }
        let terminal = self
            .terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?;
        self.parser.advance(terminal, bytes);
        self.drain()
    }

    pub fn flush_sync(&mut self) -> AppResult<Vec<Effect>> {
        Ok(self.flush_sync_outcome()?.effects)
    }

    pub fn flush_sync_outcome(&mut self) -> AppResult<Outcome> {
        let terminal = self
            .terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?;
        self.parser.stop_sync(terminal);
        self.drain()
    }

    pub fn sync_deadline(&self) -> Option<std::time::Instant> {
        self.parser.sync_timeout().sync_timeout()
    }

    fn drain(&mut self) -> AppResult<Outcome> {
        self.enforce_retained()?;
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if pending.failed {
            self.terminal = None;
            return Err(invalid("terminal effect budget overflow retired the core"));
        }
        let Some((text, overlap)) = pending.stream.take() else {
            pending.retire();
            self.terminal = None;
            return Err(invalid(
                "terminal normalized text overflow retired the core",
            ));
        };
        pending.bytes = 0;
        let outcome = Outcome {
            effects: std::mem::take(&mut pending.effects),
            text,
            overlap,
        };
        drop(pending);
        let admission = self.retained().and_then(|report| {
            let remaining = taide_native_retained::Limits {
                bytes: self.limits.retained_bytes.saturating_sub(report.bytes),
                visits: self.limits.retained_visits.saturating_sub(report.visits),
            };
            measure(&outcome, remaining)
                .map_err(|_| invalid("terminal outcome exceeds its retained handoff budget"))
        });
        if let Err(error) = admission {
            self.retire();
            return Err(error);
        }
        Ok(outcome)
    }

    pub fn resize(&mut self, size: Size) -> AppResult<()> {
        self.resize_terminal(size)?;
        self.enforce_retained()
    }

    pub fn clear_current_row(&mut self) -> AppResult<bool> {
        let changed = self
            .terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .native_clear_current_row();
        self.enforce_retained()?;
        Ok(changed)
    }

    pub fn validate_resize(&self, size: Size) -> AppResult<()> {
        admit(size, self.history, self.limits)
    }

    pub fn resize_outcome(&mut self, size: Size) -> AppResult<Outcome> {
        self.resize_terminal(size)?;
        self.drain()
    }

    fn resize_terminal(&mut self, size: Size) -> AppResult<()> {
        self.validate_resize(size)?;
        self.terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .resize(size);
        Ok(())
    }

    pub fn retained(&self) -> AppResult<Report> {
        measure(
            self,
            taide_native_retained::Limits {
                bytes: self.limits.retained_bytes,
                visits: self.limits.retained_visits,
            },
        )
        .map_err(|_| invalid("terminal retained payload cannot be admitted"))
    }

    fn enforce_retained(&mut self) -> AppResult<()> {
        if let Some(terminal) = self.terminal.as_mut() {
            terminal.native_prune_commands();
        }
        if self.selection_stamp().is_err() {
            self.retire();
            return Err(invalid(
                "terminal selection position overflow retired the core",
            ));
        }
        if let Err(error) = self.retained() {
            self.retire();
            return Err(error);
        }
        Ok(())
    }

    fn retire(&mut self) {
        self.terminal = None;
        self.parser = Processor::new();
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .retire();
    }

    pub fn content(&self) -> AppResult<RenderableContent<'_>> {
        Ok(self
            .terminal
            .as_ref()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .renderable_content())
    }

    pub fn cursor_style(&self) -> AppResult<CursorStyle> {
        Ok(self
            .terminal
            .as_ref()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .cursor_style())
    }

    pub fn set_default_cursor_style(&mut self, style: CursorStyle) -> AppResult<()> {
        self.terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .set_default_cursor_style(style);
        Ok(())
    }

    fn observe_focus(&mut self, focused: bool) -> Result<(), input::InputError> {
        self.terminal
            .as_mut()
            .ok_or(input::InputError::Retired)?
            .is_focused = focused;
        Ok(())
    }

    pub fn grid(&self) -> AppResult<&Grid<Cell>> {
        Ok(self
            .terminal
            .as_ref()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .grid())
    }

    pub fn command_blocks(&self) -> AppResult<Vec<CommandBlock>> {
        Ok(self
            .terminal
            .as_ref()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .native_command_blocks())
    }

    pub fn configure_command_colors(&mut self, colors: CommandColors) -> AppResult<()> {
        self.terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .native_set_command_colors(colors);
        Ok(())
    }

    pub fn command_start_lines(&self) -> AppResult<Vec<usize>> {
        Ok(self
            .terminal
            .as_ref()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .native_command_start_lines())
    }

    pub fn command_decorations(
        &self,
        range: std::ops::Range<Line>,
    ) -> AppResult<Vec<CommandDecoration>> {
        Ok(self
            .terminal
            .as_ref()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .native_command_decorations(range))
    }

    pub fn selection_range(&self, selection: &Selection) -> AppResult<Option<SelectionRange>> {
        selection_text::range(self.grid()?, selection)
    }

    pub fn selection_at(
        &self,
        point: Point,
        granularity: SelectionGranularity,
    ) -> AppResult<SelectionRange> {
        selection_bounds::bounds(
            self.grid()?,
            point,
            granularity,
            self.limits.retained_visits,
        )
    }

    pub fn selection_stamp(&self) -> AppResult<SelectionStamp> {
        self.terminal
            .as_ref()
            .and_then(|terminal| terminal.native_selection_stamp())
            .ok_or_else(|| invalid("terminal selection state is unavailable"))
    }

    pub fn record_user_input(&mut self) -> AppResult<()> {
        let recorded = self
            .terminal
            .as_mut()
            .is_some_and(Term::native_record_user_input);
        if recorded {
            return Ok(());
        }
        self.retire();
        Err(invalid("terminal user input sequence is exhausted"))
    }

    pub fn rebase_selection(
        &self,
        selection: &Selection,
        previous: SelectionStamp,
    ) -> AppResult<Option<Selection>> {
        let current = self.selection_stamp()?;
        if current.buffer_epoch != previous.buffer_epoch
            || current.rows_epoch != previous.rows_epoch
            || current.input_epoch != previous.input_epoch
        {
            return Ok(None);
        }
        let delta = current
            .origin
            .checked_sub(previous.origin)
            .ok_or_else(|| invalid("terminal selection movement overflow"))?;
        Ok(selection
            .clone()
            .native_rebase(delta, self.grid()?.topmost_line()))
    }

    pub fn selection_text(
        &self,
        selection: &Selection,
        capacity: usize,
    ) -> AppResult<Option<String>> {
        let Some(range) = selection_text::slice(self.grid()?, selection)? else {
            return Ok(None);
        };
        if capacity > self.limits.retained_bytes {
            return Err(invalid(
                "terminal selection byte limit exceeds the core budget",
            ));
        }
        selection_text::text(self.grid()?, range, capacity, self.limits.retained_visits).map(Some)
    }

    pub fn mode(&self) -> AppResult<TermMode> {
        Ok(*self
            .terminal
            .as_ref()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .mode())
    }

    pub fn has_scrollback(&self) -> AppResult<bool> {
        Ok(!self.mode()?.contains(TermMode::ALT_SCREEN) && self.history != 0)
    }

    pub fn scroll(&mut self, scroll: Scroll) -> AppResult<()> {
        self.terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .scroll_display(scroll);
        Ok(())
    }

    pub fn damage(&mut self) -> AppResult<TermDamage<'_>> {
        Ok(self
            .terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .damage())
    }

    pub fn reset_damage(&mut self) -> AppResult<()> {
        self.terminal
            .as_mut()
            .ok_or_else(|| invalid("terminal core is retired"))?
            .reset_damage();
        Ok(())
    }
}
