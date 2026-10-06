use std::time::{Duration, Instant};

pub const THEME_REAPPLY_DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Clone)]
pub struct LeadingTrailingDebounce {
    delay: Duration,
    deadline: Option<Instant>,
    has_trailing_call: bool,
}

impl LeadingTrailingDebounce {
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            deadline: None,
            has_trailing_call: false,
        }
    }

    pub fn trigger(&mut self, now: Instant) -> bool {
        let is_idle = self.deadline.is_none_or(|deadline| deadline <= now);
        self.has_trailing_call = !is_idle;
        self.deadline = Some(now + self.delay);
        is_idle
    }

    pub fn poll(&mut self, now: Instant) -> bool {
        if self.deadline.is_none_or(|deadline| deadline > now) {
            return false;
        }
        self.deadline = None;
        std::mem::take(&mut self.has_trailing_call)
    }

    pub fn trailing_delay(&self, now: Instant) -> Option<Duration> {
        self.deadline
            .filter(|_| self.has_trailing_call)
            .map(|deadline| deadline.saturating_duration_since(now))
    }
}

#[cfg(test)]
#[path = "leading-trailing-debounce-tests.rs"]
mod tests;
