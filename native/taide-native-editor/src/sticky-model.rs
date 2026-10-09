use std::ops::Range;

use crate::document::{DocumentId, DocumentSnapshot};
use crate::folding::FoldRegion;

const MAX_STICKY_LINES: usize = 5;
const VIEWPORT_HEIGHT_FRACTION: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickyScope {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickyCandidate {
    pub scope: StickyScope,
    pub level: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StickyViewport {
    pub visible_lines: Range<usize>,
    pub scroll_top: f32,
    pub height: f32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StickyLayout {
    pub scopes: Vec<StickyScope>,
    pub last_relative_position: f32,
}

impl StickyLayout {
    pub fn height(&self, line_height: f32) -> f32 {
        (self.scopes.len() as f32 * line_height + self.last_relative_position).max(0.0)
    }

    pub fn top(&self, index: usize, line_height: f32) -> f32 {
        index as f32 * line_height
            + if index + 1 == self.scopes.len() {
                self.last_relative_position
            } else {
                0.0
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyModel {
    document: DocumentId,
    revision: u64,
    language_id: String,
    line_count: usize,
    candidates: Vec<StickyCandidate>,
    interval_size: usize,
    interval_ends: Vec<usize>,
}

impl StickyModel {
    pub fn new(document: &DocumentSnapshot, scopes: &[StickyScope]) -> Self {
        let line_count = document.rope.len_lines();
        let mut scopes = scopes.to_vec();
        scopes.retain(|scope| {
            scope.start_line < line_count
                && scope.end_line <= line_count
                && scope.end_line > scope.start_line + 1
        });
        scopes.sort_unstable_by(|left, right| {
            left.start_line
                .cmp(&right.start_line)
                .then_with(|| right.end_line.cmp(&left.end_line))
        });
        scopes.dedup_by_key(|scope| scope.start_line);
        let mut candidates = Vec::with_capacity(scopes.len());
        let mut parents = Vec::<StickyScope>::new();
        for scope in scopes {
            while parents.last().is_some_and(|parent| {
                scope.end_line > parent.end_line || scope.start_line >= parent.end_line
            }) {
                parents.pop();
            }
            candidates.push(StickyCandidate {
                scope,
                level: parents.len(),
            });
            parents.push(scope);
        }
        let interval_size = candidates.len().max(1).next_power_of_two();
        let mut interval_ends = vec![0; interval_size * 2];
        for (index, candidate) in candidates.iter().enumerate() {
            interval_ends[interval_size + index] = candidate.scope.end_line.saturating_add(1);
        }
        for index in (1..interval_size).rev() {
            interval_ends[index] = interval_ends[index * 2].max(interval_ends[index * 2 + 1]);
        }
        Self {
            document: document.id,
            revision: document.revision,
            language_id: document.metadata.language_id.clone(),
            line_count,
            candidates,
            interval_size,
            interval_ends,
        }
    }

    pub fn from_folds(document: &DocumentSnapshot, folds: &[FoldRegion]) -> Self {
        let scopes = folds
            .iter()
            .map(|fold| StickyScope {
                start_line: fold.start_line,
                end_line: fold.end_line.saturating_add(1),
            })
            .collect::<Vec<_>>();
        Self::new(document, &scopes)
    }

    pub fn describes(&self, document: &DocumentSnapshot) -> bool {
        self.document == document.id
            && self.revision == document.revision
            && self.language_id == document.metadata.language_id
            && self.line_count == document.rope.len_lines()
    }

    pub fn candidates(
        &self,
        visible: Range<usize>,
        hidden: &[Range<usize>],
    ) -> Vec<StickyCandidate> {
        if visible.is_empty() || self.candidates.is_empty() {
            return Vec::new();
        }
        let end = self
            .candidates
            .partition_point(|candidate| candidate.scope.start_line <= visible.end);
        let mut result = Vec::new();
        let mut stack = vec![(1, 0, self.interval_size)];
        while let Some((node, first, after)) = stack.pop() {
            if first >= end || self.interval_ends[node] < visible.start {
                continue;
            }
            if after - first == 1 {
                let candidate = self.candidates[first];
                let scope = candidate.scope;
                if !hidden.iter().any(|hidden| {
                    hidden.start <= scope.start_line
                        && scope.end_line <= hidden.end.saturating_add(1)
                }) {
                    result.push(candidate);
                }
                continue;
            }
            let middle = (first + after) / 2;
            stack.push((node * 2 + 1, middle, after));
            stack.push((node * 2, first, middle));
        }
        result
    }

    pub fn layout(
        &self,
        viewport: &StickyViewport,
        hidden: &[Range<usize>],
        mut top: impl FnMut(usize) -> f32,
        mut bottom: impl FnMut(usize) -> f32,
    ) -> StickyLayout {
        let mut result = StickyLayout::default();
        if !viewport.line_height.is_finite()
            || viewport.line_height <= 0.0
            || !viewport.height.is_finite()
            || viewport.height <= 0.0
            || !viewport.scroll_top.is_finite()
        {
            return result;
        }
        let maximum = MAX_STICKY_LINES.min(
            (viewport.height / viewport.line_height * VIEWPORT_HEIGHT_FRACTION).round() as usize,
        );
        if maximum == 0 {
            return result;
        }
        for candidate in self.candidates(viewport.visible_lines.clone(), hidden) {
            let scope = candidate.scope;
            let element_top = candidate.level as f32 * viewport.line_height;
            let body_bottom = bottom(scope.end_line - 1) - viewport.scroll_top;
            if element_top <= top(scope.start_line) - viewport.scroll_top
                || element_top > body_bottom
            {
                continue;
            }
            result.scopes.push(StickyScope {
                end_line: scope.end_line.min(self.line_count - 1),
                ..scope
            });
            let element_bottom = element_top + viewport.line_height;
            if element_bottom > body_bottom {
                result.last_relative_position = body_bottom - element_bottom;
            }
            if result.scopes.len() == maximum {
                break;
            }
        }
        result
    }
}
