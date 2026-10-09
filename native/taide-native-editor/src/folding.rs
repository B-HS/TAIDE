use std::borrow::Cow;
use std::iter::successors;
use std::ops::{Range, RangeInclusive};

use ropey::{Rope, RopeSlice};

use crate::change_journal::ChangeSet;
use crate::decoration::{Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness};
use crate::display_map::merged_line_ranges;
use crate::document::{DocumentSnapshot, EditorError};
use crate::editing::{line_text, rope_line_content_range};
use crate::language_configuration::{FoldMarker, LanguageRules, is_js_whitespace};
use crate::store::EditorStore;
use crate::view::{Selection, SelectionSet, ViewId, ViewState};

pub const MAX_FOLDING_REGIONS: usize = 5000;
pub(crate) const MAX_FOLDABLE_LINE: usize = 0xFF_FFFF;
const COUNTED_INDENT_LIMIT: usize = 1000;
const TRACKED_RANGE_Z_ORDER: u8 = 0;
const SINGLE_LEVEL: usize = 1;
const LINE_BREAK_CHARACTERS: [char; 2] = ['\n', '\r'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldRegion {
    pub start_line: usize,
    pub end_line: usize,
}

impl FoldRegion {
    fn is_contained_by(&self, other: &Self) -> bool {
        other.start_line <= self.start_line && other.end_line >= self.end_line
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldCommand {
    Fold,
    Unfold,
    ToggleFold,
    FoldRecursively,
    UnfoldRecursively,
    ToggleFoldRecursively,
    FoldAll,
    UnfoldAll,
    FoldAllExcept,
    UnfoldAllExcept,
    GotoParentFold,
    GotoPreviousFold,
    GotoNextFold,
    FoldAllBlockComments,
    FoldAllMarkerRegions,
    UnfoldAllMarkerRegions,
    CreateFromSelection,
    RemoveManualRanges,
    ToggleImports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldToggle {
    Region,
    Recursive,
    Surrounding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldClick {
    pub line: usize,
    pub is_on_control: bool,
    pub toggle: FoldToggle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenLevel {
    Sentinel,
    EndMarker,
    Indent(usize),
}

struct OpenIndent {
    level: OpenLevel,
    ends_above: usize,
    line: usize,
}

fn indent_level(line: RopeSlice<'_>, tab_size: usize) -> Option<usize> {
    let mut indent = 0;
    for byte in line.bytes() {
        match byte {
            b' ' => indent += 1,
            b'\t' => indent = indent - indent % tab_size + tab_size,
            b'\n' | b'\r' => return None,
            _ => return Some(indent),
        }
    }
    None
}

pub fn indent_regions(rope: &Rope, tab_size: u32, limit: usize) -> Vec<FoldRegion> {
    fold_regions(rope, tab_size, limit, None)
}

pub fn language_regions(
    rope: &Rope,
    tab_size: u32,
    limit: usize,
    rules: &dyn LanguageRules,
) -> Vec<FoldRegion> {
    fold_regions(rope, tab_size, limit, Some(rules))
}

fn fold_regions(
    rope: &Rope,
    tab_size: u32,
    limit: usize,
    rules: Option<&dyn LanguageRules>,
) -> Vec<FoldRegion> {
    let tab_size = (tab_size as usize).max(1);
    let line_count = rope.len_lines();
    let is_off_side = rules.is_some_and(LanguageRules::is_off_side);
    let mut found: Vec<(FoldRegion, usize)> = Vec::new();
    let mut indent_occurrences: Vec<usize> = Vec::new();
    let mut record = |start_line: usize, end_line: usize, indent: usize| {
        if end_line >= MAX_FOLDABLE_LINE {
            return;
        }
        let region = FoldRegion {
            start_line,
            end_line,
        };
        found.push((region, indent));
        if indent < COUNTED_INDENT_LIMIT {
            if indent_occurrences.len() <= indent {
                indent_occurrences.resize(indent + 1, 0);
            }
            indent_occurrences[indent] += 1;
        }
    };
    let mut open = vec![OpenIndent {
        level: OpenLevel::Sentinel,
        ends_above: line_count,
        line: line_count,
    }];
    let lines = (0..line_count)
        .rev()
        .zip(rope.lines_at(line_count).reversed());
    for (line, content) in lines {
        let Some(indent) = indent_level(content, tab_size) else {
            if is_off_side && let Some(previous) = open.last_mut() {
                previous.ends_above = line;
            }
            continue;
        };
        let marker = rules.and_then(|rules| {
            let text = Cow::from(content);
            rules.fold_marker(text.trim_end_matches(LINE_BREAK_CHARACTERS))
        });
        match marker {
            Some(FoldMarker::Start) => {
                let end_marker = open
                    .iter()
                    .rposition(|open| open.level == OpenLevel::EndMarker);
                if let Some(index) = end_marker {
                    open.truncate(index + 1);
                    let closed = &mut open[index];
                    record(line, closed.line, indent);
                    closed.line = line;
                    closed.level = OpenLevel::Indent(indent);
                    closed.ends_above = line;
                    continue;
                }
            }
            Some(FoldMarker::End) => {
                open.push(OpenIndent {
                    level: OpenLevel::EndMarker,
                    ends_above: line,
                    line,
                });
                continue;
            }
            None => {}
        }
        let is_deeper =
            |open: &OpenIndent| matches!(open.level, OpenLevel::Indent(open) if open > indent);
        if open.last().is_some_and(is_deeper) {
            while open.last().is_some_and(is_deeper) {
                open.pop();
            }
            let end_line = open.last().map_or(line, |outer| outer.ends_above - 1);
            if end_line > line {
                record(line, end_line, indent);
            }
        }
        match open.last_mut() {
            Some(outer) if outer.level == OpenLevel::Indent(indent) => outer.ends_above = line,
            _ => open.push(OpenIndent {
                level: OpenLevel::Indent(indent),
                ends_above: line,
                line,
            }),
        }
    }
    let found = found.into_iter().rev();
    if found.len() <= limit {
        return found.map(|(region, _)| region).collect();
    }
    let mut admitted = 0;
    let mut deepest_indent = indent_occurrences.len();
    for (indent, count) in indent_occurrences.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        if count + admitted > limit {
            deepest_indent = indent;
            break;
        }
        admitted += count;
    }
    found
        .filter(|(_, indent)| {
            if *indent != deepest_indent {
                return *indent < deepest_indent;
            }
            admitted += 1;
            admitted <= limit
        })
        .map(|(region, _)| region)
        .collect()
}

fn folded_regions(rope: &Rope, folds: &[Range<usize>]) -> Vec<FoldRegion> {
    let length = rope.len_bytes();
    let mut regions: Vec<FoldRegion> = folds
        .iter()
        .filter_map(|fold| {
            Some(FoldRegion {
                start_line: rope.byte_to_line(fold.start.min(length)).checked_sub(1)?,
                end_line: rope.byte_to_line(fold.end.min(length)),
            })
        })
        .collect();
    regions.sort_by_key(|region| (region.start_line, region.end_line));
    regions
}

fn selected_lines(rope: &Rope, selection: &SelectionSet) -> Vec<usize> {
    selection
        .selections
        .iter()
        .map(|selection| rope.byte_to_line(selection.anchor.min(selection.head)))
        .collect()
}

fn manual_regions(
    computed: &[FoldRegion],
    rope: &Rope,
    manual: &[Range<usize>],
) -> Vec<FoldRegion> {
    if manual.is_empty() {
        return computed.to_vec();
    }
    let manual = folded_regions(rope, manual);
    let mut combined = computed
        .iter()
        .copied()
        .filter(|region| {
            !manual.iter().any(|user| {
                region.start_line == user.start_line
                    || (region.start_line < user.start_line
                        && user.start_line <= region.end_line
                        && region.end_line < user.end_line)
                    || (user.start_line < region.start_line
                        && region.start_line <= user.end_line
                        && user.end_line < region.end_line)
            })
        })
        .chain(manual.iter().copied())
        .collect::<Vec<_>>();
    combined.sort_by_key(|region| region.start_line);
    combined
}

fn previous_regions(
    rope: &Rope,
    folds: &[Range<usize>],
    manual: &[Range<usize>],
) -> Vec<FoldRegion> {
    let manual = folded_regions(rope, manual);
    folded_regions(rope, folds)
        .into_iter()
        .filter(|region| {
            !manual.iter().any(|user| {
                (region.start_line < user.start_line
                    && user.start_line <= region.end_line
                    && region.end_line < user.end_line)
                    || (user.start_line < region.start_line
                        && region.start_line <= user.end_line
                        && user.end_line < region.end_line)
            })
        })
        .collect()
}

fn region_bytes(rope: &Rope, region: FoldRegion) -> Range<usize> {
    rope.line_to_byte(region.start_line + 1)..rope_line_content_range(rope, region.end_line).end
}

pub fn hidden_lines(rope: &Rope, folds: &[Range<usize>]) -> Vec<Range<usize>> {
    let length = rope.len_bytes();
    merged_line_ranges(
        folds.iter().map(|fold| {
            rope.byte_to_line(fold.start.min(length))..rope.byte_to_line(fold.end.min(length)) + 1
        }),
        rope.len_lines(),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldingModel {
    regions: Vec<FoldRegion>,
    collapsed: Vec<bool>,
    parents: Vec<Option<usize>>,
}

impl FoldingModel {
    pub fn new(
        regions: &[FoldRegion],
        document: &DocumentSnapshot,
        folds: &[Range<usize>],
    ) -> Self {
        Self::with_manual(regions, document, folds, &[])
    }

    pub fn with_manual(
        regions: &[FoldRegion],
        document: &DocumentSnapshot,
        folds: &[Range<usize>],
        manual: &[Range<usize>],
    ) -> Self {
        let regions = manual_regions(regions, &document.rope, manual);
        Self::merged(
            &regions,
            &previous_regions(&document.rope, folds, manual),
            document.rope.len_lines(),
            None,
        )
    }

    pub fn following_edit(
        regions: &[FoldRegion],
        document: &DocumentSnapshot,
        folds: &[Range<usize>],
        selection: &SelectionSet,
    ) -> Self {
        Self::following_edit_with_manual(regions, document, folds, &[], selection)
    }

    pub fn following_edit_with_manual(
        regions: &[FoldRegion],
        document: &DocumentSnapshot,
        folds: &[Range<usize>],
        manual: &[Range<usize>],
        selection: &SelectionSet,
    ) -> Self {
        let regions = manual_regions(regions, &document.rope, manual);
        Self::merged(
            &regions,
            &previous_regions(&document.rope, folds, manual),
            document.rope.len_lines(),
            Some(&selected_lines(&document.rope, selection)),
        )
    }

    fn merged(
        computed: &[FoldRegion],
        folded: &[FoldRegion],
        line_count: usize,
        selected: Option<&[usize]>,
    ) -> Self {
        let starts_inside = |lines: RangeInclusive<usize>| {
            selected.is_some_and(|selected| selected.iter().any(|line| lines.contains(line)))
        };
        let mut model = Self::default();
        let mut enclosing: Vec<FoldRegion> = Vec::new();
        let mut computed = computed.iter().copied().peekable();
        let mut folded = folded.iter().copied().peekable();
        loop {
            let next = computed.peek().copied();
            let previous = folded
                .peek()
                .copied()
                .filter(|previous| next.is_none_or(|next| next.start_line >= previous.start_line));
            let (region, is_collapsed) = match previous {
                Some(previous) => {
                    folded.next();
                    let stays_collapsed =
                        !starts_inside(previous.start_line + 1..=previous.end_line);
                    match next.filter(|next| next.start_line == previous.start_line) {
                        Some(next) => {
                            computed.next();
                            let keeps_extent = next.end_line == previous.end_line
                                || !starts_inside(next.start_line + 1..=next.end_line + 1);
                            (next, stays_collapsed && keeps_extent)
                        }
                        None if stays_collapsed => (previous, true),
                        None => continue,
                    }
                }
                None => match computed.next() {
                    Some(next) => (next, false),
                    None => break,
                },
            };
            while enclosing
                .last()
                .is_some_and(|outer| outer.end_line < region.start_line)
            {
                enclosing.pop();
            }
            let follows_last = model
                .regions
                .last()
                .is_none_or(|last| region.start_line > last.start_line);
            let nests = enclosing
                .last()
                .is_none_or(|outer| outer.end_line >= region.end_line);
            if region.end_line > region.start_line
                && region.end_line < line_count
                && follows_last
                && nests
            {
                model.regions.push(region);
                model.collapsed.push(is_collapsed);
                enclosing.push(region);
            }
        }
        let mut ancestors: Vec<usize> = Vec::new();
        for (index, region) in model.regions.iter().enumerate() {
            while ancestors
                .last()
                .is_some_and(|outer| !region.is_contained_by(&model.regions[*outer]))
            {
                ancestors.pop();
            }
            model.parents.push(ancestors.last().copied());
            ancestors.push(index);
        }
        model
    }

    pub fn regions(&self) -> &[FoldRegion] {
        &self.regions
    }

    pub fn is_collapsed(&self, index: usize) -> bool {
        self.collapsed[index]
    }

    pub fn header(&self, line: usize) -> Option<bool> {
        self.regions
            .binary_search_by_key(&line, |region| region.start_line)
            .ok()
            .map(|index| self.collapsed[index])
    }

    pub fn folds(&self, rope: &Rope) -> Vec<Range<usize>> {
        self.regions
            .iter()
            .zip(&self.collapsed)
            .filter(|(_, is_collapsed)| **is_collapsed)
            .map(|(region, _)| {
                rope.line_to_byte(region.start_line + 1)
                    ..rope_line_content_range(rope, region.end_line).end
            })
            .filter(|fold| fold.start <= fold.end)
            .collect()
    }

    fn innermost_at(&self, line: usize) -> Option<usize> {
        let first = self
            .regions
            .partition_point(|region| region.start_line <= line)
            .checked_sub(1)?;
        successors(Some(first), |index| self.parents[*index])
            .find(|index| self.regions[*index].end_line >= line)
    }

    fn enclosing_at(&self, line: usize) -> impl Iterator<Item = usize> + '_ {
        successors(self.innermost_at(line), |index| self.parents[*index])
    }

    fn inside(&self, outer: Option<usize>) -> Vec<(usize, usize)> {
        let first = outer.map_or(0, |outer| outer + 1);
        let last_line = outer.map_or(usize::MAX, |outer| self.regions[outer].end_line);
        let mut ancestors: Vec<usize> = Vec::new();
        self.regions[first..]
            .iter()
            .enumerate()
            .take_while(|(_, region)| region.start_line < last_line)
            .map(|(offset, region)| {
                while ancestors
                    .last()
                    .is_some_and(|ancestor| !region.is_contained_by(&self.regions[*ancestor]))
                {
                    ancestors.pop();
                }
                ancestors.push(first + offset);
                (first + offset, ancestors.len())
            })
            .collect()
    }

    fn toggle(&mut self, mut indices: Vec<usize>) -> bool {
        indices.sort_unstable();
        indices.dedup();
        for index in &indices {
            self.collapsed[*index] = !self.collapsed[*index];
        }
        !indices.is_empty()
    }

    fn differing_inside(&self, outer: Option<usize>, collapse: bool, levels: usize) -> Vec<usize> {
        self.inside(outer)
            .into_iter()
            .filter(|(index, level)| self.collapsed[*index] != collapse && *level < levels)
            .map(|(index, _)| index)
            .collect()
    }

    fn set_collapsed_down(&mut self, collapse: bool, levels: usize, lines: &[usize]) -> bool {
        let mut toggled = Vec::new();
        if lines.is_empty() {
            toggled = self.differing_inside(None, collapse, levels);
        }
        for line in lines {
            let Some(index) = self.innermost_at(*line) else {
                continue;
            };
            if self.collapsed[index] != collapse {
                toggled.push(index);
            }
            if levels > SINGLE_LEVEL {
                toggled.extend(self.differing_inside(Some(index), collapse, levels));
            }
        }
        self.toggle(toggled)
    }

    fn set_collapsed_up(&mut self, collapse: bool, lines: &[usize]) -> bool {
        let toggled = lines
            .iter()
            .filter_map(|line| {
                self.enclosing_at(*line)
                    .find(|index| self.collapsed[*index] != collapse)
            })
            .collect();
        self.toggle(toggled)
    }

    fn toggle_at(&mut self, levels: usize, lines: &[usize]) -> bool {
        let mut toggled = Vec::new();
        for line in lines {
            let Some(index) = self.innermost_at(*line) else {
                continue;
            };
            toggled.push(index);
            if levels > SINGLE_LEVEL {
                toggled.extend(self.differing_inside(Some(index), !self.collapsed[index], levels));
            }
        }
        self.toggle(toggled)
    }

    fn unrelated_to(&self, blocked: &[usize]) -> Vec<usize> {
        self.inside(None)
            .into_iter()
            .map(|(index, _)| index)
            .filter(|index| {
                let region = &self.regions[*index];
                blocked.iter().all(|blocked| {
                    let blocked = &self.regions[*blocked];
                    !blocked.is_contained_by(region) && !region.is_contained_by(blocked)
                })
            })
            .collect()
    }

    fn set_collapsed_for_rest(&mut self, collapse: bool, lines: &[usize]) -> bool {
        let blocked: Vec<usize> = lines
            .iter()
            .filter_map(|line| self.innermost_at(*line))
            .collect();
        let toggled = self
            .unrelated_to(&blocked)
            .into_iter()
            .filter(|index| self.collapsed[*index] != collapse)
            .collect();
        self.toggle(toggled)
    }

    fn header_at(&self, line: usize) -> Option<usize> {
        self.innermost_at(line)
            .filter(|index| self.regions[*index].start_line == line)
    }

    fn parent_fold_line(&self, line: usize) -> Option<usize> {
        let index = self.innermost_at(line)?;
        let start_line = self.regions[index].start_line;
        if line != start_line {
            return Some(start_line);
        }
        self.parents[index].map(|parent| self.regions[parent].start_line)
    }

    fn previous_fold_line(&self, line: usize) -> Option<usize> {
        let Some(index) = self.header_at(line) else {
            return self
                .regions
                .iter()
                .rev()
                .find(|region| region.start_line < line)
                .map(|region| region.start_line);
        };
        let parent = self.parents[index];
        let first_line = parent.map(|parent| self.regions[parent].start_line);
        self.regions[..index]
            .iter()
            .enumerate()
            .rev()
            .take_while(|(_, region)| first_line.is_none_or(|first| region.start_line > first))
            .find(|(sibling, _)| self.parents[*sibling] == parent)
            .map(|(_, region)| region.start_line)
    }

    fn next_fold_line(&self, line: usize) -> Option<usize> {
        let Some(index) = self.header_at(line) else {
            return self
                .regions
                .iter()
                .find(|region| region.start_line > line)
                .map(|region| region.start_line);
        };
        let parent = self.parents[index];
        let last_line = match parent {
            Some(parent) => self.regions[parent].end_line,
            None => self.regions.last()?.end_line,
        };
        self.regions
            .iter()
            .enumerate()
            .skip(index + 1)
            .take_while(|(_, region)| region.start_line < last_line)
            .find(|(sibling, _)| self.parents[*sibling] == parent)
            .map(|(_, region)| region.start_line)
    }

    pub fn run(&mut self, command: FoldCommand, lines: &[usize]) -> Option<usize> {
        let first = lines.first().copied();
        match command {
            FoldCommand::Fold => self.set_collapsed_up(true, lines),
            FoldCommand::Unfold => self.set_collapsed_down(false, SINGLE_LEVEL, lines),
            FoldCommand::ToggleFold => self.toggle_at(SINGLE_LEVEL, lines),
            FoldCommand::FoldRecursively => self.set_collapsed_down(true, usize::MAX, lines),
            FoldCommand::UnfoldRecursively => self.set_collapsed_down(false, usize::MAX, lines),
            FoldCommand::ToggleFoldRecursively => self.toggle_at(usize::MAX, lines),
            FoldCommand::FoldAll => self.set_collapsed_down(true, usize::MAX, &[]),
            FoldCommand::UnfoldAll => self.set_collapsed_down(false, usize::MAX, &[]),
            FoldCommand::FoldAllExcept => self.set_collapsed_for_rest(true, lines),
            FoldCommand::UnfoldAllExcept => self.set_collapsed_for_rest(false, lines),
            FoldCommand::GotoParentFold => return self.parent_fold_line(first?),
            FoldCommand::GotoPreviousFold => return self.previous_fold_line(first?),
            FoldCommand::GotoNextFold => return self.next_fold_line(first?),
            FoldCommand::FoldAllBlockComments
            | FoldCommand::FoldAllMarkerRegions
            | FoldCommand::UnfoldAllMarkerRegions
            | FoldCommand::CreateFromSelection
            | FoldCommand::RemoveManualRanges
            | FoldCommand::ToggleImports => false,
        };
        None
    }

    pub fn set_collapsed_where(
        &mut self,
        collapse: bool,
        matches_header_line: impl Fn(usize) -> bool,
    ) -> bool {
        let toggled = (0..self.regions.len())
            .filter(|index| {
                self.collapsed[*index] != collapse
                    && matches_header_line(self.regions[*index].start_line)
            })
            .collect();
        self.toggle(toggled)
    }

    pub fn click(&mut self, click: FoldClick) -> bool {
        let Some(index) = self.header_at(click.line) else {
            return false;
        };
        let is_collapsed = self.collapsed[index];
        if !click.is_on_control && !is_collapsed {
            return false;
        }
        let toggled = match click.toggle {
            FoldToggle::Surrounding => {
                let surrounding = self.unrelated_to(&[index]);
                let collapsed: Vec<usize> = surrounding
                    .iter()
                    .copied()
                    .filter(|index| self.collapsed[*index])
                    .collect();
                if collapsed.is_empty() {
                    surrounding
                } else {
                    collapsed
                }
            }
            FoldToggle::Recursive => {
                let mut inner: Vec<usize> = self
                    .inside(Some(index))
                    .into_iter()
                    .map(|(index, _)| index)
                    .filter(|index| self.collapsed[*index] == is_collapsed)
                    .collect();
                if is_collapsed || inner.is_empty() {
                    inner.push(index);
                }
                inner
            }
            FoldToggle::Region => vec![index],
        };
        self.toggle(toggled)
    }
}

pub(crate) fn tracked_folds(
    before: &Rope,
    after: &Rope,
    changes: &ChangeSet,
    folds: &[Range<usize>],
) -> Vec<Range<usize>> {
    let mut marks = folds
        .iter()
        .filter_map(|fold| {
            let header = before.byte_to_line(fold.start).checked_sub(1)?;
            Some((
                Decoration {
                    bytes: rope_line_content_range(before, header).end..fold.end,
                    kind: DecorationKind::Inline(InlineStyle::default()),
                    stickiness: Stickiness::AlwaysGrowsWhenTypingAtEdges,
                },
                fold.is_empty(),
            ))
        })
        .collect::<Vec<_>>();
    marks.sort_by_key(|(mark, _)| mark.bytes.start);
    let (marks, was_empty): (Vec<_>, Vec<_>) = marks.into_iter().unzip();
    let mut tracked = DecorationLayer::new(changes.revision_before, TRACKED_RANGE_Z_ORDER, marks);
    tracked.apply(changes);
    let length = after.len_bytes();
    let mut folds: Vec<Range<usize>> = tracked
        .items()
        .iter()
        .zip(was_empty)
        .filter_map(|(mark, was_empty)| {
            let header = after.byte_to_line(mark.bytes.start.min(length));
            let last = after.byte_to_line(mark.bytes.end.min(length));
            if last <= header {
                return None;
            }
            let fold = after.line_to_byte(header + 1)..rope_line_content_range(after, last).end;
            (fold.start < fold.end || was_empty).then_some(fold)
        })
        .collect();
    folds.sort_by_key(|fold| (fold.start, fold.end));
    folds.dedup();
    folds
}

fn shown_selection(rope: &Rope, hidden: &[Range<usize>], selection: &SelectionSet) -> SelectionSet {
    let shown = |offset: usize| {
        let line = rope.byte_to_line(offset);
        hidden
            .iter()
            .find(|lines| lines.contains(&line))
            .map_or(offset, |lines| {
                rope_line_content_range(rope, lines.start - 1).end
            })
    };
    SelectionSet {
        primary: selection.primary,
        selections: selection
            .selections
            .iter()
            .map(|selection| Selection {
                anchor: shown(selection.anchor),
                head: shown(selection.head),
            })
            .collect(),
    }
}

fn view_document(
    store: &EditorStore,
    view: ViewId,
) -> Result<(ViewState, DocumentSnapshot), EditorError> {
    let current = store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .clone();
    let document = store.documents().snapshot(current.document)?;
    Ok((current, document))
}

fn has_no_folds(store: &EditorStore, view: ViewId) -> Result<bool, EditorError> {
    Ok(store
        .views()
        .get(view)
        .ok_or(EditorError::NotFound)?
        .folds
        .is_empty())
}

fn store_folds(
    store: &mut EditorStore,
    current: ViewState,
    document: &DocumentSnapshot,
    folds: Vec<Range<usize>>,
    selection: SelectionSet,
) -> Result<bool, EditorError> {
    let rope = &document.rope;
    let hidden = hidden_lines(rope, &folds);
    let selection = if hidden == hidden_lines(rope, &current.folds) {
        selection
    } else {
        shown_selection(rope, &hidden, &selection)
    };
    if folds == current.folds && selection == current.selection {
        return Ok(false);
    }
    if selection != current.selection {
        store.break_undo_group(document.id)?;
        store.set_composition(current.id, None)?;
    }
    store.set_view_state(current.id, selection, current.scroll, folds)?;
    Ok(true)
}

pub fn reconcile_folds(
    store: &mut EditorStore,
    view: ViewId,
    regions: &[FoldRegion],
) -> Result<bool, EditorError> {
    if has_no_folds(store, view)? {
        return Ok(false);
    }
    let (current, document) = view_document(store, view)?;
    let folds = FoldingModel::following_edit_with_manual(
        regions,
        &document,
        &current.folds,
        &current.manual_folds,
        &current.selection,
    )
    .folds(&document.rope);
    let selection = current.selection.clone();
    store_folds(store, current, &document, folds, selection)
}

pub fn reveal_carets(store: &mut EditorStore, view: ViewId) -> Result<bool, EditorError> {
    if has_no_folds(store, view)? {
        return Ok(false);
    }
    let (current, document) = view_document(store, view)?;
    let rope = &document.rope;
    let anchors: Vec<usize> = current
        .selection
        .selections
        .iter()
        .map(|selection| rope.byte_to_line(selection.anchor))
        .collect();
    let folds: Vec<Range<usize>> = current
        .folds
        .iter()
        .filter(|fold| {
            let lines = rope.byte_to_line(fold.start)..=rope.byte_to_line(fold.end);
            !anchors.iter().any(|line| lines.contains(line))
        })
        .cloned()
        .collect();
    if folds.len() == current.folds.len() {
        return Ok(false);
    }
    let selection = current.selection.clone();
    store_folds(store, current, &document, folds, selection)
}

pub fn run_fold_command(
    store: &mut EditorStore,
    view: ViewId,
    regions: &[FoldRegion],
    command: FoldCommand,
) -> Result<bool, EditorError> {
    if command == FoldCommand::CreateFromSelection {
        return create_manual_ranges(store, view, regions);
    }
    if command == FoldCommand::RemoveManualRanges {
        return remove_manual_ranges(store, view, regions);
    }
    let (current, document) = view_document(store, view)?;
    let rope = &document.rope;
    let mut model =
        FoldingModel::with_manual(regions, &document, &current.folds, &current.manual_folds);
    let selection = match model.run(command, &selected_lines(rope, &current.selection)) {
        Some(line) => {
            let start = rope.line_to_byte(line);
            SelectionSet {
                primary: 0,
                selections: vec![Selection {
                    anchor: start,
                    head: start,
                }],
            }
        }
        None => current.selection.clone(),
    };
    let folds = model.folds(rope);
    store_folds(store, current, &document, folds, selection)
}

fn create_manual_ranges(
    store: &mut EditorStore,
    view: ViewId,
    regions: &[FoldRegion],
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let rope = &document.rope;
    let mut additions = Vec::new();
    let mut selection = current.selection.clone();
    for selected in &mut selection.selections {
        let start = selected.anchor.min(selected.head);
        let end = selected.anchor.max(selected.head);
        let start_line = rope.byte_to_line(start);
        let mut end_line = rope.byte_to_line(end);
        if end == rope.line_to_byte(end_line) {
            end_line = end_line.saturating_sub(1);
        }
        if end_line <= start_line {
            continue;
        }
        let region = FoldRegion {
            start_line,
            end_line,
        };
        additions.push(region);
        let byte = rope.line_to_byte(start_line);
        *selected = Selection {
            anchor: byte,
            head: byte,
        };
    }
    if additions.is_empty() {
        return Ok(false);
    }
    additions.sort_by_key(|region| region.start_line);
    let mut manual = folded_regions(rope, &current.manual_folds);
    manual.retain(|previous| {
        !additions.iter().any(|new| {
            previous.start_line == new.start_line
                || (previous.start_line <= new.end_line
                    && new.start_line <= previous.end_line
                    && !previous.is_contained_by(new)
                    && !new.is_contained_by(previous))
        })
    });
    manual.extend(additions.iter().copied());
    manual.sort_by_key(|region| region.start_line);
    let manual = FoldingModel::merged(&manual, &[], rope.len_lines(), None)
        .regions()
        .iter()
        .map(|region| region_bytes(rope, *region))
        .collect::<Vec<_>>();
    let mut collapsed = current.folds.clone();
    collapsed.extend(additions.iter().map(|region| region_bytes(rope, *region)));
    let model = FoldingModel::with_manual(regions, &document, &collapsed, &manual);
    store.set_manual_folds(view, manual)?;
    store_folds(store, current, &document, model.folds(rope), selection)?;
    Ok(true)
}

fn remove_manual_ranges(
    store: &mut EditorStore,
    view: ViewId,
    regions: &[FoldRegion],
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let rope = &document.rope;
    let selections = current
        .selection
        .selections
        .iter()
        .map(|selection| {
            rope.byte_to_line(selection.anchor.min(selection.head))
                ..=rope.byte_to_line(selection.anchor.max(selection.head))
        })
        .collect::<Vec<_>>();
    let intersects = |region: FoldRegion| {
        selections
            .iter()
            .any(|lines| *lines.start() <= region.end_line && region.start_line <= *lines.end())
    };
    let manual = folded_regions(rope, &current.manual_folds);
    let removed = manual
        .iter()
        .copied()
        .filter(|region| intersects(*region))
        .collect::<Vec<_>>();
    let retained = manual
        .iter()
        .copied()
        .filter(|region| !intersects(*region))
        .map(|region| region_bytes(rope, region))
        .collect::<Vec<_>>();
    let folds = folded_regions(rope, &current.folds)
        .into_iter()
        .filter(|region| {
            !removed
                .iter()
                .any(|user| user.start_line == region.start_line)
                && (regions.contains(region) || !intersects(*region))
        })
        .map(|region| region_bytes(rope, region))
        .collect::<Vec<_>>();
    let changed = retained != current.manual_folds || folds != current.folds;
    store.set_manual_folds(view, retained)?;
    let selection = current.selection.clone();
    store_folds(store, current, &document, folds, selection)?;
    Ok(changed)
}

pub fn run_syntax_fold_command(
    store: &mut EditorStore,
    view: ViewId,
    syntax: &crate::syntax_folding::SyntaxFolds,
    command: FoldCommand,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    if !syntax.describes(&document) {
        return Ok(false);
    }
    let kind = match command {
        FoldCommand::FoldAllBlockComments => "comment",
        FoldCommand::FoldAllMarkerRegions | FoldCommand::UnfoldAllMarkerRegions => "region",
        FoldCommand::ToggleImports => "imports",
        _ => return run_fold_command(store, view, syntax.regions(), command),
    };
    let mut model = FoldingModel::with_manual(
        syntax.regions(),
        &document,
        &current.folds,
        &current.manual_folds,
    );
    let manual = folded_regions(&document.rope, &current.manual_folds);
    for index in 0..model.regions.len() {
        let region = model.regions[index];
        if syntax.kind(region.start_line) == Some(kind)
            && !manual
                .iter()
                .any(|user| user.start_line == region.start_line)
        {
            model.collapsed[index] = match command {
                FoldCommand::ToggleImports => !model.collapsed[index],
                FoldCommand::UnfoldAllMarkerRegions => false,
                _ => true,
            };
        }
    }
    let selection = current.selection.clone();
    store_folds(
        store,
        current,
        &document,
        model.folds(&document.rope),
        selection,
    )
}

pub fn run_language_fold_command(
    store: &mut EditorStore,
    view: ViewId,
    regions: &[FoldRegion],
    command: FoldCommand,
    rules: &dyn LanguageRules,
) -> Result<bool, EditorError> {
    let collapse = match command {
        FoldCommand::FoldAllBlockComments | FoldCommand::FoldAllMarkerRegions => true,
        FoldCommand::UnfoldAllMarkerRegions => false,
        _ => return run_fold_command(store, view, regions, command),
    };
    let (current, document) = view_document(store, view)?;
    let mut model =
        FoldingModel::with_manual(regions, &document, &current.folds, &current.manual_folds);
    model.set_collapsed_where(collapse, |line| {
        let header = line_text(&document, line).text;
        if command != FoldCommand::FoldAllBlockComments {
            return rules.starts_marker_region(&header);
        }
        rules
            .pairs()
            .block_comment_start
            .as_deref()
            .is_some_and(|start| {
                header
                    .trim_start_matches(is_js_whitespace)
                    .starts_with(start)
            })
    });
    let folds = model.folds(&document.rope);
    let selection = current.selection.clone();
    store_folds(store, current, &document, folds, selection)
}

pub fn click_fold(
    store: &mut EditorStore,
    view: ViewId,
    regions: &[FoldRegion],
    click: FoldClick,
) -> Result<bool, EditorError> {
    let (current, document) = view_document(store, view)?;
    let mut model =
        FoldingModel::with_manual(regions, &document, &current.folds, &current.manual_folds);
    if !model.click(click) {
        return Ok(false);
    }
    let folds = model.folds(&document.rope);
    let selection = current.selection.clone();
    store_folds(store, current, &document, folds, selection)
}
