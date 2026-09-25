use std::time::Instant;

use parking_lot::Mutex;
use taide_infra::shell_integration::CommandMarker;

/// The measured result of a shell command bounded by OSC 133 markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedCommand {
    pub exit_code: Option<i32>,
    pub duration_ms: u32,
}

/// Records the latest command start and consumes it when the matching finish arrives.
#[derive(Default)]
pub struct TerminalCommandClock(Mutex<Option<Instant>>);

impl TerminalCommandClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, marker: CommandMarker, now: Instant) -> Option<TimedCommand> {
        let exit_code = match marker {
            CommandMarker::OutputStart => {
                *self.0.lock() = Some(now);
                return None;
            }
            CommandMarker::Finished { exit_code } => exit_code,
        };

        let started = self.0.lock().take()?;

        Some(TimedCommand { exit_code, duration_ms: u32::try_from(now.saturating_duration_since(started).as_millis()).unwrap_or(u32::MAX) })
    }
}
