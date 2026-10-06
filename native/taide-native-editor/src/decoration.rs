use std::borrow::Cow;
use std::cmp::Ordering;
use std::ops::Range;

use crate::change_journal::{ChangeSet, ChangeSpan, ChangesSince};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineKind {
    Straight,
    Squiggly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Underline {
    pub kind: UnderlineKind,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InlineStyle {
    pub foreground: Option<[u8; 4]>,
    pub background: Option<[u8; 4]>,
    pub underline: Option<Underline>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneMark {
    Bar,
    DeletedTriangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationKind {
    Inline(InlineStyle),
    LineBackground([u8; 4]),
    Lane { mark: LaneMark, color: [u8; 4] },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Stickiness {
    #[default]
    AlwaysGrowsWhenTypingAtEdges,
    NeverGrowsWhenTypingAtEdges,
    GrowsOnlyWhenTypingBefore,
    GrowsOnlyWhenTypingAfter,
}

impl Stickiness {
    fn start_sticks_to_previous_character(self) -> bool {
        matches!(
            self,
            Self::AlwaysGrowsWhenTypingAtEdges | Self::GrowsOnlyWhenTypingBefore
        )
    }

    fn end_sticks_to_previous_character(self) -> bool {
        matches!(
            self,
            Self::NeverGrowsWhenTypingAtEdges | Self::GrowsOnlyWhenTypingBefore
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoration {
    pub bytes: Range<usize>,
    pub kind: DecorationKind,
    pub stickiness: Stickiness,
}

impl Decoration {
    fn accept(&mut self, span: &ChangeSpan) {
        if self.bytes.end < span.start_byte {
            return;
        }
        let start = moved_marker(
            self.bytes.start,
            self.stickiness.start_sticks_to_previous_character(),
            span,
        );
        let end = moved_marker(
            self.bytes.end,
            self.stickiness.end_sticks_to_previous_character(),
            span,
        );
        self.bytes = start..end.max(start);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MarkerMove {
    MarkerDefined,
    ForceStay,
}

fn is_marker_before(
    marker: usize,
    sticks_to_previous_character: bool,
    boundary: usize,
    semantics: MarkerMove,
) -> bool {
    match marker.cmp(&boundary) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => semantics == MarkerMove::ForceStay || sticks_to_previous_character,
    }
}

fn moved_marker(marker: usize, sticks_to_previous_character: bool, span: &ChangeSpan) -> usize {
    let start = span.start_byte;
    let end = span.old_end_byte;
    let deleting = end - start;
    let inserting = span.new_end_byte - start;
    let common = deleting.min(inserting);
    let at_start = if deleting > 0 {
        MarkerMove::ForceStay
    } else {
        MarkerMove::MarkerDefined
    };
    if is_marker_before(marker, sticks_to_previous_character, start, at_start) {
        return marker;
    }
    let after_common = if deleting > inserting {
        MarkerMove::ForceStay
    } else {
        MarkerMove::MarkerDefined
    };
    if common > 0
        && is_marker_before(
            marker,
            sticks_to_previous_character,
            start + common,
            after_common,
        )
    {
        return marker;
    }
    if is_marker_before(
        marker,
        sticks_to_previous_character,
        end,
        MarkerMove::MarkerDefined,
    ) {
        return start + inserting;
    }
    marker - deleting + inserting
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecorationLayer {
    revision: u64,
    z_order: u8,
    items: Vec<Decoration>,
}

impl DecorationLayer {
    pub fn new(revision: u64, z_order: u8, mut items: Vec<Decoration>) -> Self {
        items.sort_by_key(|item| item.bytes.start);
        Self {
            revision,
            z_order,
            items,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn z_order(&self) -> u8 {
        self.z_order
    }

    pub fn items(&self) -> &[Decoration] {
        &self.items
    }

    pub fn intersecting(&self, bytes: Range<usize>) -> impl Iterator<Item = &Decoration> {
        let candidates = self
            .items
            .partition_point(|item| item.bytes.start <= bytes.end);
        self.items[..candidates]
            .iter()
            .filter(move |item| item.bytes.end >= bytes.start)
    }

    pub fn apply(&mut self, changes: &ChangeSet) -> bool {
        if changes.revision_before != self.revision {
            return false;
        }
        for span in changes.spans.iter().rev() {
            for item in &mut self.items {
                item.accept(span);
            }
        }
        self.revision = changes.revision_after;
        true
    }

    pub fn tracking(&self, changes: ChangesSince<'_>) -> Option<Cow<'_, Self>> {
        let ChangesSince::Tracked(changes) = changes else {
            return None;
        };
        let mut layer = Cow::Borrowed(self);
        for set in changes {
            if !layer.to_mut().apply(set) {
                return None;
            }
        }
        Some(layer)
    }
}
