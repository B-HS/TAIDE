use std::ops::Range;

use crate::diagnostics::{Marker, Severity};
use crate::document::DocumentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Next,
    Previous,
    NextInFiles,
    PreviousInFiles,
    Close,
}

impl Command {
    pub fn from_action(action: &str) -> Option<Self> {
        match action {
            "editor.action.marker.next" => Some(Self::Next),
            "editor.action.marker.prev" => Some(Self::Previous),
            "editor.action.marker.nextInFiles" => Some(Self::NextInFiles),
            "editor.action.marker.prevInFiles" => Some(Self::PreviousInFiles),
            _ => None,
        }
    }

    pub fn all_files(self) -> bool {
        matches!(self, Self::NextInFiles | Self::PreviousInFiles)
    }

    pub fn forward(self) -> bool {
        matches!(self, Self::Next | Self::NextInFiles)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub resource: String,
    pub document: DocumentId,
    pub marker: Marker,
    pub initial_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinate {
    pub problem: Problem,
    pub index: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Navigation {
    entries: Vec<Problem>,
    selected: Option<usize>,
}

impl Navigation {
    pub fn update(&mut self, mut entries: Vec<Problem>) {
        entries.retain(|entry| entry.marker.message.severity != Severity::Hint);
        entries.sort_by(|left, right| {
            left.resource
                .cmp(&right.resource)
                .then(
                    left.marker
                        .message
                        .severity
                        .cmp(&right.marker.message.severity),
                )
                .then(left.marker.bytes.start.cmp(&right.marker.bytes.start))
                .then(left.marker.bytes.end.cmp(&right.marker.bytes.end))
        });
        if self.entries != entries {
            self.entries = entries;
            self.selected = None;
        }
    }

    pub fn selected(&self) -> Option<Coordinate> {
        let index = self.selected?;
        Some(Coordinate {
            problem: self.entries.get(index)?.clone(),
            index: index + 1,
            total: self.entries.len(),
        })
    }

    pub fn select(&mut self, problem: &Problem) -> Option<Coordinate> {
        self.selected = self.entries.iter().position(|entry| {
            entry.resource == problem.resource
                && entry.document == problem.document
                && entry.marker == problem.marker
        });
        self.selected()
    }

    pub fn reset(&mut self) {
        self.selected = None;
    }

    pub fn follow_cursor(&mut self, document: DocumentId, cursor: usize) {
        if self.selected().is_some_and(|selected| {
            selected.problem.document != document
                || cursor < selected.problem.marker.bytes.start
                || cursor > selected.problem.marker.bytes.end
        }) {
            self.reset();
        }
    }

    pub fn navigate(&mut self, resource: &str, cursor: usize, forward: bool) -> Option<Coordinate> {
        let count = self.entries.len();
        if count == 0 {
            return None;
        }
        let index = if let Some(index) = self.selected {
            if forward {
                (index + 1) % count
            } else {
                (index + count - 1) % count
            }
        } else {
            let first = self
                .entries
                .partition_point(|entry| entry.resource.as_str() < resource);
            let end = self
                .entries
                .partition_point(|entry| entry.resource.as_str() <= resource);
            if first == end {
                if forward {
                    first % count
                } else {
                    (first + count - 1) % count
                }
            } else if let Some(index) = (first..end).find(|&index| {
                self.entries[index].initial_range.start <= cursor
                    && cursor <= self.entries[index].initial_range.end
                    || cursor <= self.entries[index].marker.bytes.start
            }) {
                let contains = self.entries[index].initial_range.start <= cursor
                    && cursor <= self.entries[index].initial_range.end;
                if forward || contains {
                    index
                } else {
                    (index + count - 1) % count
                }
            } else if forward {
                end % count
            } else {
                end - 1
            }
        };
        self.selected = Some(index);
        self.selected()
    }
}
