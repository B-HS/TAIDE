use std::ops::Range;

use ropey::Rope;
use taide_model::ids::{PaneId, TabId};

use crate::document::{DocumentId, Edit, EditorError, byte_to_char};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(pub(crate) u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewKey {
    pub window: String,
    pub pane: PaneId,
    pub tab: TabId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSet {
    pub primary: usize,
    pub selections: Vec<Selection>,
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self {
            primary: 0,
            selections: vec![Selection { anchor: 0, head: 0 }],
        }
    }
}

impl SelectionSet {
    pub(crate) fn validate(&self, rope: &Rope) -> Result<(), EditorError> {
        if self.selections.is_empty() || self.primary >= self.selections.len() {
            return Err(EditorError::InvalidBoundary);
        }
        for selection in &self.selections {
            byte_to_char(rope, selection.anchor)?;
            byte_to_char(rope, selection.head)?;
        }
        Ok(())
    }

    pub(crate) fn mapped(&self, edits: &[Edit]) -> Self {
        Self {
            primary: self.primary,
            selections: self
                .selections
                .iter()
                .map(|selection| Selection {
                    anchor: map_offset(selection.anchor, edits),
                    head: map_offset(selection.head, edits),
                })
                .collect(),
        }
    }

    pub(crate) fn clamped(&self, rope: &Rope) -> Self {
        let boundary = |offset: usize| {
            let offset = offset.min(rope.len_bytes());
            rope.char_to_byte(rope.byte_to_char(offset))
        };
        Self {
            primary: self.primary,
            selections: self
                .selections
                .iter()
                .map(|selection| Selection {
                    anchor: boundary(selection.anchor),
                    head: boundary(selection.head),
                })
                .collect(),
        }
    }
}

fn map_offset(offset: usize, edits: &[Edit]) -> usize {
    let mut removed = 0;
    let mut inserted = 0;
    for edit in edits {
        if offset < edit.bytes.start {
            break;
        }
        if offset < edit.bytes.end {
            return edit.bytes.start - removed + inserted + edit.text.len();
        }
        removed += edit.bytes.end - edit.bytes.start;
        inserted += edit.text.len();
    }
    offset - removed + inserted
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScrollPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composition {
    pub revision: u64,
    pub replace: Range<usize>,
    pub preedit: String,
}

#[derive(Debug, Clone)]
pub struct ViewState {
    pub id: ViewId,
    pub key: ViewKey,
    pub document: DocumentId,
    pub selection: SelectionSet,
    pub scroll: ScrollPosition,
    pub folds: Vec<Range<usize>>,
    pub composition: Option<Composition>,
}
