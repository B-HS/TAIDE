use parking_lot::Mutex;
use taide_model::terminal::PtyAttachResult;

use crate::service::ScrollbackRing;

type OutputSink = Box<dyn Fn(&[u8]) -> bool + Send + Sync>;

/// Resets terminal styling before each scrollback replay.
pub const TERMINAL_REPLAY_PREAMBLE: &[u8] = b"\x1b[0m";

struct TerminalOutputState {
    scrollback: ScrollbackRing,
    subscribers: Vec<(u32, OutputSink)>,
    next_subscription_id: u32,
}

/// Serializes scrollback recording, replay, and live subscriber delivery.
pub struct TerminalSessionOutput(Mutex<TerminalOutputState>);

impl TerminalSessionOutput {
    pub fn new(scrollback_capacity: usize) -> Self {
        Self(Mutex::new(TerminalOutputState {
            scrollback: ScrollbackRing::new(scrollback_capacity),
            subscribers: Vec::new(),
            next_subscription_id: 0,
        }))
    }

    pub fn append_and_broadcast(&self, bytes: &[u8]) {
        let mut state = self.0.lock();
        state.scrollback.append(bytes);
        state.subscribers.retain(|(_, sink)| sink(bytes));
    }

    pub fn attach(&self, sink: impl Fn(&[u8]) -> bool + Send + Sync + 'static) -> PtyAttachResult {
        let mut state = self.0.lock();
        let (front, back) = state.scrollback.as_slices();
        let mut replay_bytes = TERMINAL_REPLAY_PREAMBLE.len();
        let _ = sink(TERMINAL_REPLAY_PREAMBLE);

        for half in [front, back] {
            if half.is_empty() {
                continue;
            }
            replay_bytes += half.len();
            let _ = sink(half);
        }

        let subscription_id = state.next_subscription_id;
        state.next_subscription_id = subscription_id.wrapping_add(1);
        state.subscribers.push((subscription_id, Box::new(sink)));

        PtyAttachResult { subscription_id, replay_bytes: u32::try_from(replay_bytes).unwrap_or(u32::MAX) }
    }

    pub fn detach(&self, subscription_id: u32) {
        self.0.lock().subscribers.retain(|(id, _)| *id != subscription_id);
    }
}
