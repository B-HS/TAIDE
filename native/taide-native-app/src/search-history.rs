pub(crate) const SEARCH_HISTORY_LIMIT: usize = 20;

/// The history with `term` moved to the front, or `None` when nothing changes (a blank term or
/// one already at the front).
pub(crate) fn add_recent_term(history: &[String], term: &str) -> Option<Vec<String>> {
    let trimmed = term.trim();
    if trimmed.is_empty() || history.first().is_some_and(|first| first == trimmed) {
        return None;
    }
    Some(
        std::iter::once(trimmed.to_owned())
            .chain(history.iter().filter(|entry| *entry != trimmed).cloned())
            .take(SEARCH_HISTORY_LIMIT)
            .collect(),
    )
}

/// Recent-search writes in flight, so a term recorded before the previous write settled builds
/// on that write instead of the settled settings value.
#[derive(Debug, Default)]
pub(crate) struct Queue {
    pending: Option<Vec<String>>,
    in_flight: u32,
}

impl Queue {
    /// The list to persist for `term`, or `None` when the term is already at the front.
    pub(crate) fn enqueue(&mut self, settled: &[String], term: &str) -> Option<Vec<String>> {
        let next = add_recent_term(self.pending.as_deref().unwrap_or(settled), term)?;
        self.pending = Some(next.clone());
        self.in_flight += 1;
        Some(next)
    }

    /// Marks one write finished; the settled value is the base again once none are left.
    pub(crate) fn settle(&mut self) {
        self.in_flight = self.in_flight.saturating_sub(1);
        if self.in_flight == 0 {
            self.pending = None;
        }
    }

    /// The list the history menu shows: the last write still in flight, else `settled`.
    pub(crate) fn list<'a>(&'a self, settled: &'a [String]) -> &'a [String] {
        self.pending.as_deref().unwrap_or(settled)
    }
}

#[cfg(test)]
#[path = "search-history-tests.rs"]
mod tests;
