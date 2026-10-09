use std::ops::Range;
use std::sync::Arc;

use ropey::Rope;
use taide_model::ids::{PaneId, TabId};

use crate::display_map::DisplayMap;
use crate::document::{DocumentId, Edit, EditorError, UndoGroup, byte_to_char};

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
    pub fn normalized(self) -> Self {
        let mut entries: Vec<_> = self.selections.iter().copied().enumerate().collect();
        let mut last = entries.len().saturating_sub(1);
        let mut primary = self.primary;
        loop {
            let mut order: Vec<_> = (0..entries.len()).collect();
            order.sort_by_key(|index| {
                let s = entries[*index].1;
                (s.anchor.min(s.head), s.anchor.max(s.head))
            });
            let collision = order.windows(2).find_map(|pair| {
                let (a, b) = (entries[pair[0]].1, entries[pair[1]].1);
                let end = a.anchor.max(a.head);
                let start = b.anchor.min(b.head);
                let touches = a.anchor == a.head || b.anchor == b.head;
                (start < end || touches && start == end).then_some((pair[0], pair[1]))
            });
            let Some((a, b)) = collision else {
                break;
            };
            let (winner, loser) = if entries[a].0 < entries[b].0 {
                (a, b)
            } else {
                (b, a)
            };
            let (w, l) = (entries[winner], entries[loser]);
            let start = w.1.anchor.min(w.1.head).min(l.1.anchor.min(l.1.head));
            let end = w.1.anchor.max(w.1.head).max(l.1.anchor.max(l.1.head));
            let direction = if l.0 == last { l.1 } else { w.1 };
            entries[winner].1 = if direction.anchor <= direction.head {
                Selection {
                    anchor: start,
                    head: end,
                }
            } else {
                Selection {
                    anchor: end,
                    head: start,
                }
            };
            if primary == l.0 {
                primary = w.0;
            }
            if last == l.0 {
                last = w.0;
            }
            entries.remove(loser);
        }
        Self {
            primary: entries
                .iter()
                .position(|(index, _)| *index == primary)
                .unwrap_or(0),
            selections: entries.into_iter().map(|(_, s)| s).collect(),
        }
    }

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
pub struct SelectionReveal {
    pub bytes: Range<usize>,
    pub center_if_outside: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOperation {
    Other,
    DeletingLeft,
    DeletingRight,
    TypingOther,
    TypingFirstSpace,
    TypingConsecutiveSpace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditRun {
    pub operation: EditOperation,
    pub group: UndoGroup,
    pub revision: u64,
    pub selection: SelectionSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalColumns {
    pub revision: u64,
    pub leftover_visible_columns: Vec<isize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapAffinities {
    pub revision: u64,
    pub heads_at_row_end: Vec<bool>,
}

#[derive(Debug, Clone)]
pub struct ViewState {
    pub id: ViewId,
    pub key: ViewKey,
    pub document: DocumentId,
    pub selection: SelectionSet,
    pub scroll: ScrollPosition,
    pub folds: Vec<Range<usize>>,
    pub manual_folds: Vec<Range<usize>>,
    pub composition: Option<Composition>,
    pub edit_run: Option<EditRun>,
    pub goal_columns: Option<GoalColumns>,
    pub wrap_affinities: Option<WrapAffinities>,
    pub display: Option<Arc<DisplayMap>>,
    pub(crate) cursor_memory: crate::cursor_commands::CursorMemory,
    pub(crate) selection_reveal: Option<SelectionReveal>,
}

impl ViewState {
    pub fn head_at_row_end(&self, selection: usize, revision: u64) -> bool {
        self.wrap_affinities.as_ref().is_some_and(|affinities| {
            affinities.revision == revision
                && affinities
                    .heads_at_row_end
                    .get(selection)
                    .copied()
                    .unwrap_or(false)
        })
    }
}
