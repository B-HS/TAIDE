use taide_native_editor::change_journal::{ChangeSet, ChangeSpan};
use taide_native_editor::line_tokens::LineTokens;

use crate::tokenizer::{LineState, TokenizedLine};

pub const MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH: usize = 20 * 1024 * 1024;
pub const MAX_TOKENIZED_DOCUMENT_LINES: usize = 300 * 1000;
const KNOWN_END_STATE_WINDOW: usize = 1024;

pub fn is_too_large_for_tokenization(utf16_length: usize, line_count: usize) -> bool {
    utf16_length > MAX_TOKENIZED_DOCUMENT_UTF16_LENGTH || line_count > MAX_TOKENIZED_DOCUMENT_LINES
}

#[derive(Debug, Clone)]
pub struct TokenizationPlan {
    pub start_line: usize,
    pub start_state: Option<LineState>,
    pub first_known_line: usize,
    pub known_end_states: Vec<Option<LineState>>,
}

impl TokenizationPlan {
    pub fn known_end_state(&self, line: usize) -> Option<&LineState> {
        self.known_end_states
            .get(line.checked_sub(self.first_known_line)?)?
            .as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct DocumentTokens {
    tokens: LineTokens,
    end_states: Vec<Option<LineState>>,
}

impl DocumentTokens {
    pub fn new(line_count: usize) -> Self {
        Self {
            tokens: LineTokens::new(line_count),
            end_states: vec![None; line_count],
        }
    }

    pub fn reset(&mut self, line_count: usize) {
        *self = Self::new(line_count);
    }

    pub fn tokens(&self) -> &LineTokens {
        &self.tokens
    }

    pub fn apply(&mut self, changes: &ChangeSet) -> bool {
        let is_applied = self.tokens.apply(changes)
            && changes
                .spans
                .iter()
                .rev()
                .all(|span| self.move_end_states(span))
            && self.end_states.len() == self.tokens.line_count();
        if !is_applied {
            self.reset(changes.line_count_after);
        }
        is_applied
    }

    fn move_end_states(&mut self, span: &ChangeSpan) -> bool {
        let first = span.start.line;
        let Some(dropped) = span.old_end.line.checked_sub(first) else {
            return false;
        };
        let Some(inserted) = span.new_end.line.checked_sub(first) else {
            return false;
        };
        if first + dropped >= self.end_states.len() {
            return false;
        }
        self.end_states
            .splice(first..first + dropped, std::iter::repeat_n(None, inserted));
        true
    }

    pub fn plan(&mut self) -> Option<TokenizationPlan> {
        let invalid = self.tokens.invalid_ranges().first()?.clone();
        let start_state = match invalid.start.checked_sub(1) {
            None => None,
            Some(previous) => match self.end_states.get(previous).cloned().flatten() {
                Some(state) => Some(state),
                None => {
                    self.reset(self.tokens.line_count());
                    return self.plan();
                }
            },
        };
        let first_known_line = invalid.end.saturating_sub(1);
        let line_count = self.tokens.line_count();
        let known_end = line_count.min(first_known_line + KNOWN_END_STATE_WINDOW);
        let known_end_states = (first_known_line..known_end)
            .map(|line| {
                let is_next_line_settled = line + 1 >= line_count || self.tokens.is_valid(line + 1);
                self.end_states[line]
                    .clone()
                    .filter(|_| is_next_line_settled)
            })
            .collect();
        Some(TokenizationPlan {
            start_line: invalid.start,
            start_state,
            first_known_line,
            known_end_states,
        })
    }

    pub fn accept(&mut self, first_line: usize, lines: Vec<TokenizedLine>) {
        for (offset, tokenized) in lines.into_iter().enumerate() {
            let Some(stored) = self.end_states.get_mut(first_line + offset) else {
                return;
            };
            let has_end_state_changed = stored
                .as_ref()
                .is_none_or(|previous| !previous.is_same(&tokenized.end_state));
            if has_end_state_changed {
                *stored = Some(tokenized.end_state);
            }
            self.tokens
                .set_line(first_line + offset, tokenized.spans, has_end_state_changed);
        }
    }

    pub fn is_awaiting(&self, line: usize) -> bool {
        self.tokens.first_invalid_line() == Some(line)
    }
}

#[cfg(test)]
#[path = "document-tokens-tests.rs"]
mod tests;
