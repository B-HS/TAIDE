use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};

use alacritty_terminal::vte::Params;
use alacritty_terminal::vte::ansi::StreamObserver;
use taide_infra::terminal_scan::{ScanEvent, ScanOutcome, classify_osc_payload};

use crate::input::{InputAction, InputError, NativeInput};
use crate::normalized_stream::NormalizedStream;
use crate::{AlacrittyProbe, Snapshot};

pub use crate::normalized_stream::MAX_PENDING_TEXT_BYTES;

pub const MAX_PENDING_EVENTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectOverflow;

impl Display for EffectOverflow {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("terminal stream effect capacity exceeded")
    }
}

impl std::error::Error for EffectOverflow {}

#[derive(Debug, Default)]
struct EffectBuffer {
    events: Vec<ScanEvent>,
    is_overflowed: bool,
    stream: NormalizedStream,
}

#[derive(Debug)]
struct Observer(Arc<Mutex<EffectBuffer>>);

impl StreamObserver for Observer {
    fn observe(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn observe_print(&mut self, character: char) {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stream
            .print(character);
    }

    fn observe_execute(&mut self, byte: u8) {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stream
            .execute(byte);
    }

    fn observe_csi(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        is_ignored: bool,
        action: char,
    ) {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .stream
            .csi(params, intermediates, is_ignored, action);
    }

    fn observe_payload(&mut self, payload: &[u8], _bell_terminated: bool) {
        let mut buffer = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if buffer.is_overflowed {
            return;
        }
        let Some(event) = std::str::from_utf8(payload)
            .ok()
            .and_then(classify_osc_payload)
        else {
            return;
        };
        if buffer.events.len() == MAX_PENDING_EVENTS {
            buffer.events.clear();
            buffer.is_overflowed = true;
            return;
        }
        buffer.events.push(event);
    }
}

pub struct OscEffectProbe {
    terminal: AlacrittyProbe,
    buffer: Arc<Mutex<EffectBuffer>>,
}

impl Default for OscEffectProbe {
    fn default() -> Self {
        let buffer = Arc::new(Mutex::new(EffectBuffer::default()));
        let mut terminal = AlacrittyProbe::new_with_payload_capture(false);
        terminal
            .parser
            .set_stream_observer(Some(Box::new(Observer(buffer.clone()))));
        Self { terminal, buffer }
    }
}

impl OscEffectProbe {
    pub fn advance(&mut self, bytes: &[u8]) -> Result<Vec<ScanEvent>, EffectOverflow> {
        Ok(self.advance_outcome(bytes)?.events)
    }

    pub fn advance_outcome(&mut self, bytes: &[u8]) -> Result<ScanOutcome, EffectOverflow> {
        self.terminal.advance(bytes);
        self.drain_outcome()
    }

    pub fn flush_sync(&mut self) -> Result<Vec<ScanEvent>, EffectOverflow> {
        Ok(self.flush_sync_outcome()?.events)
    }

    pub fn flush_sync_outcome(&mut self) -> Result<ScanOutcome, EffectOverflow> {
        self.terminal.parser.stop_sync(&mut self.terminal.term);
        self.drain_outcome()
    }

    pub fn snapshot(&self) -> Snapshot {
        self.terminal.snapshot()
    }

    pub fn encode_input(
        &self,
        input: NativeInput<'_>,
        capacity: usize,
    ) -> Result<InputAction, InputError> {
        self.terminal.encode_input(input, capacity)
    }

    fn drain_outcome(&self) -> Result<ScanOutcome, EffectOverflow> {
        let mut buffer = self
            .buffer
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if buffer.is_overflowed {
            return Err(EffectOverflow);
        }
        let Some((text, overlap)) = buffer.stream.take() else {
            buffer.events.clear();
            buffer.is_overflowed = true;
            return Err(EffectOverflow);
        };
        Ok(ScanOutcome {
            events: std::mem::take(&mut buffer.events),
            text,
            overlap,
        })
    }
}
