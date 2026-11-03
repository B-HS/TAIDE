use taide_model::git::{GitChangeKind, StatusRow};
use taide_model::locale::ResolvedLocale;
use taide_native_ui::intents::git::{RowKey, Section};

pub(crate) const SECTIONS: [Section; 5] = [
    Section::Merge,
    Section::Staged,
    Section::Changes,
    Section::Stashes,
    Section::Graph,
];
pub(crate) const CHANGE_SECTIONS: [Section; 3] =
    [Section::Merge, Section::Staged, Section::Changes];

pub(crate) fn section_id(section: Section) -> &'static str {
    match section {
        Section::Merge => "merge",
        Section::Staged => "staged",
        Section::Changes => "changes",
        Section::Stashes => "stashes",
        Section::Graph => "graph",
    }
}

pub(crate) fn section_title(section: Section) -> &'static str {
    match section {
        Section::Merge => "git.mergeChanges",
        Section::Staged => "git.stagedChanges",
        Section::Changes => "git.changes",
        Section::Stashes => "git.stash",
        Section::Graph => "git.graph",
    }
}

fn index(section: Section) -> usize {
    match section {
        Section::Merge => 0,
        Section::Staged => 1,
        Section::Changes => 2,
        Section::Stashes => 3,
        Section::Graph => 4,
    }
}

/// The per-section collapse flags `Settings.git_sections_collapsed` stores as a list of ids
/// (`git-section.ts` `toGitSectionCollapsedMap`/`toGitSectionCollapsedIds`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Collapsed([bool; SECTIONS.len()]);

impl Collapsed {
    pub(crate) fn from_ids(ids: &[String]) -> Self {
        Self(SECTIONS.map(|section| ids.iter().any(|id| id == section_id(section))))
    }

    pub(crate) fn ids(&self) -> Vec<String> {
        SECTIONS
            .into_iter()
            .filter(|section| self.is_collapsed(*section))
            .map(|section| section_id(section).to_owned())
            .collect()
    }

    pub(crate) fn is_collapsed(&self, section: Section) -> bool {
        self.0[index(section)]
    }

    pub(crate) fn toggled(mut self, section: Section) -> Self {
        self.0[index(section)] = !self.0[index(section)];
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SectionState {
    pub count: usize,
    pub is_visible: bool,
    pub is_collapsed: bool,
}

/// One drawn change row: the group's status kind for it (`git-change-group.tsx`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChangeRow<'a> {
    pub row: &'a StatusRow,
    pub kind: GitChangeKind,
}

#[derive(Debug)]
pub(crate) struct Sections<'a> {
    pub merge: Vec<ChangeRow<'a>>,
    pub staged: Vec<ChangeRow<'a>>,
    pub changes: Vec<ChangeRow<'a>>,
    pub states: [SectionState; SECTIONS.len()],
    pub shows_no_changes: bool,
}

impl<'a> Sections<'a> {
    pub(crate) fn state(&self, section: Section) -> SectionState {
        self.states[index(section)]
    }

    pub(crate) fn rows(&self, section: Section) -> &[ChangeRow<'a>] {
        match section {
            Section::Merge => &self.merge,
            Section::Staged => &self.staged,
            Section::Changes => &self.changes,
            Section::Stashes | Section::Graph => &[],
        }
    }
}

/// `git-sections.ts` `buildGitSections`: a conflicted row only joins Merge, a row with both index
/// and worktree changes joins Staged and Changes, an empty section is not drawn, and a collapsed
/// section keeps its count.
pub(crate) fn build_sections<'a>(
    rows: &'a [StatusRow],
    stash_count: usize,
    graph_count: usize,
    collapsed: Collapsed,
) -> Sections<'a> {
    let merge: Vec<_> = rows
        .iter()
        .filter(|row| row.is_conflicted)
        .map(|row| ChangeRow {
            row,
            kind: GitChangeKind::Conflicted,
        })
        .collect();
    let group = |pick: fn(&StatusRow) -> Option<GitChangeKind>| -> Vec<_> {
        rows.iter()
            .filter(|row| !row.is_conflicted)
            .filter_map(|row| pick(row).map(|kind| ChangeRow { row, kind }))
            .collect()
    };
    let staged = group(|row| row.staged);
    let changes = group(|row| row.unstaged);
    let counts = [
        merge.len(),
        staged.len(),
        changes.len(),
        stash_count,
        graph_count,
    ];
    let states = SECTIONS.map(|section| SectionState {
        count: counts[index(section)],
        is_visible: counts[index(section)] > 0,
        is_collapsed: collapsed.is_collapsed(section),
    });
    Sections {
        shows_no_changes: merge.is_empty() && staged.is_empty() && changes.is_empty(),
        merge,
        staged,
        changes,
        states,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListRow {
    Header(Section),
    Row(Section, usize),
}

/// The three change groups as one list for the virtualized change pane: a visible group's header,
/// then its rows unless it is collapsed (`change-row-navigation.ts` `buildGitChangeListRows`).
pub(crate) fn list_rows(sections: &Sections<'_>) -> Vec<ListRow> {
    CHANGE_SECTIONS
        .into_iter()
        .filter(|section| sections.state(*section).is_visible)
        .flat_map(|section| {
            let rows = if sections.state(section).is_collapsed {
                0
            } else {
                sections.rows(section).len()
            };
            std::iter::once(ListRow::Header(section))
                .chain((0..rows).map(move |row| ListRow::Row(section, row)))
        })
        .collect()
}

/// The repository path a list item stands for, when it is a change row.
pub(crate) fn row_key(sections: &Sections<'_>, item: ListRow) -> Option<RowKey> {
    match item {
        ListRow::Header(_) => None,
        ListRow::Row(section, row) => Some(RowKey {
            section,
            path: sections.rows(section)[row].row.path.clone(),
        }),
    }
}

/// Header positions in a [`list_rows`] result, ascending (`gitChangeListHeaderIndexes`).
pub(crate) fn header_indexes(list: &[ListRow]) -> Vec<usize> {
    list.iter()
        .enumerate()
        .filter(|(_, item)| matches!(item, ListRow::Header(_)))
        .map(|(index, _)| index)
        .collect()
}

/// The header pinned for a window starting at `first_visible`: the last one at or above it
/// (`resolveStickyHeaderIndex`).
pub(crate) fn sticky_header_index(headers: &[usize], first_visible: usize) -> Option<usize> {
    headers
        .iter()
        .copied()
        .take_while(|index| *index <= first_visible)
        .last()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Direction {
    Down,
    Up,
}

/// A stop in the panel's roving focus order: an item of the scrolled pane or the graph header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    Item(usize),
    Graph,
}

/// `resolveNextChangeRowIndex`: enters at the top or bottom from outside and clamps at both ends.
pub(crate) fn next_item_index(
    direction: Direction,
    active: Option<usize>,
    count: usize,
) -> Option<usize> {
    let last = count.checked_sub(1)?;
    Some(match (direction, active) {
        (Direction::Down, None) => 0,
        (Direction::Up, None) => last,
        (Direction::Down, Some(active)) => (active + 1).min(last),
        (Direction::Up, Some(active)) => active.saturating_sub(1).min(last),
    })
}

/// `resolveGitPanelFocusTarget`: the same sequence continued across the graph pane's header.
pub(crate) fn focus_target(
    direction: Direction,
    origin: Option<FocusTarget>,
    item_count: usize,
    has_graph_pane: bool,
) -> Option<FocusTarget> {
    let active = match origin {
        Some(FocusTarget::Graph) => {
            return match direction {
                Direction::Down => None,
                Direction::Up => item_count.checked_sub(1).map(FocusTarget::Item),
            };
        }
        Some(FocusTarget::Item(index)) => Some(index),
        None => None,
    };
    if has_graph_pane
        && ((active.is_none() && direction == Direction::Up)
            || (item_count == 0 && direction == Direction::Down))
    {
        return Some(FocusTarget::Graph);
    }
    let index = next_item_index(direction, active, item_count)?;
    if has_graph_pane && direction == Direction::Down && active == Some(index) {
        return Some(FocusTarget::Graph);
    }
    Some(FocusTarget::Item(index))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Selection<T> {
    pub selected: Vec<T>,
    pub anchor: T,
}

/// `list-selection.ts` `resolveListSelection`: a plain click selects one, Shift selects the
/// anchor's range (added to the selection with Cmd/Ctrl), Cmd/Ctrl toggles and moves the anchor.
/// Entries no longer listed drop out.
pub(crate) fn resolve_selection<T: Clone + PartialEq>(
    ordered: &[T],
    selected: &[T],
    anchor: Option<&T>,
    clicked: &T,
    shift: bool,
    additive: bool,
) -> Selection<T> {
    let visible: Vec<T> = ordered
        .iter()
        .filter(|item| selected.contains(item))
        .cloned()
        .collect();
    let position = |item: &T| ordered.iter().position(|ordered| ordered == item);
    let Some(clicked_index) = position(clicked) else {
        return Selection {
            selected: visible,
            anchor: clicked.clone(),
        };
    };
    if let Some((anchor, anchor_index)) =
        anchor.and_then(|anchor| position(anchor).map(|index| (anchor, index)))
        && shift
    {
        let range = &ordered[anchor_index.min(clicked_index)..=anchor_index.max(clicked_index)];
        let combined = if additive { visible } else { Vec::new() };
        return Selection {
            selected: range.iter().fold(combined, |mut items, item| {
                if !items.contains(item) {
                    items.push(item.clone());
                }
                items
            }),
            anchor: anchor.clone(),
        };
    }
    if additive {
        let mut next: Vec<T> = visible.into_iter().filter(|item| item != clicked).collect();
        if !selected.contains(clicked) {
            next.push(clicked.clone());
        }
        return Selection {
            selected: next,
            anchor: clicked.clone(),
        };
    }
    Selection {
        selected: vec![clicked.clone()],
        anchor: clicked.clone(),
    }
}

/// A change row command, shared by the row buttons and its context menu (`git-change-group.tsx`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowAction {
    OpenFile,
    OpenChanges,
    Stage,
    Unstage,
    Discard,
    CopyPath,
    Reveal,
}

impl RowAction {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::OpenFile => "git.openFile",
            Self::OpenChanges => "git.openChanges",
            Self::Stage => "git.stageChanges",
            Self::Unstage => "git.unstageChanges",
            Self::Discard => "git.discard",
            Self::CopyPath => "explorer.copyPath",
            Self::Reveal => "explorer.reveal",
        }
    }
}

/// The group's row context menu in order; `None` is a separator (`buildContextMenuEntries`).
pub(crate) fn row_menu(section: Section) -> &'static [Option<RowAction>] {
    match section {
        Section::Merge => &[
            Some(RowAction::OpenFile),
            Some(RowAction::OpenChanges),
            None,
            Some(RowAction::CopyPath),
            Some(RowAction::Reveal),
        ],
        Section::Staged => &[
            Some(RowAction::OpenFile),
            Some(RowAction::OpenChanges),
            Some(RowAction::Unstage),
            None,
            Some(RowAction::CopyPath),
            Some(RowAction::Reveal),
        ],
        Section::Changes => &[
            Some(RowAction::OpenFile),
            Some(RowAction::OpenChanges),
            Some(RowAction::Stage),
            Some(RowAction::Discard),
            None,
            Some(RowAction::CopyPath),
            Some(RowAction::Reveal),
        ],
        Section::Stashes | Section::Graph => &[],
    }
}

/// The discard confirmation text: the path itself for one file, otherwise the file count
/// (`git-panel.tsx` discard `AlertDialog`).
pub(crate) fn discard_description(locale: &ResolvedLocale, paths: &[String]) -> String {
    let target = match paths {
        [path] => path.clone(),
        _ => crate::presentation::message(
            locale,
            "git.fileCount",
            &[("count", &paths.len().to_string())],
        ),
    };
    crate::presentation::message(locale, "git.discardDescription", &[("target", &target)])
}

/// `status-row-item.tsx` `STATUS_LETTER`.
pub(crate) fn status_letter(kind: GitChangeKind) -> &'static str {
    match kind {
        GitChangeKind::Modified => "M",
        GitChangeKind::Added => "A",
        GitChangeKind::Deleted => "D",
        GitChangeKind::Renamed => "R",
        GitChangeKind::Untracked => "U",
        GitChangeKind::TypeChange => "T",
        GitChangeKind::Conflicted => "!",
    }
}

/// `status-row-item.tsx` `STATUS_TEXT_CLASS`: a type change reads as modified.
pub(crate) fn status_color_key(kind: GitChangeKind) -> &'static str {
    match kind {
        GitChangeKind::Modified | GitChangeKind::TypeChange => "git.modified",
        GitChangeKind::Added => "git.added",
        GitChangeKind::Deleted => "git.deleted",
        GitChangeKind::Renamed => "git.renamed",
        GitChangeKind::Untracked => "git.untracked",
        GitChangeKind::Conflicted => "git.conflicted",
    }
}

/// The row's tooltip: `{orig} → {path}` for a rename, otherwise the repository path.
pub(crate) fn row_tooltip(row: &StatusRow) -> String {
    match &row.orig_path {
        Some(orig) => format!("{orig} → {}", row.path),
        None => row.path.clone(),
    }
}

/// The file name and, when the path has one, its directory (`status-row-item.tsx`).
pub(crate) fn split_path(path: &str) -> (&str, Option<&str>) {
    match path.rsplit_once('/') {
        Some((directory, name)) => (name, Some(directory)),
        None => (path, None),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommitGate {
    BlockedByConflicts,
    ConfirmStageAll,
    Commit,
}

/// `commit-gate.ts` `resolveCommitGate`: an unresolved conflict blocks committing, and with
/// nothing staged but something unstaged the commit stages everything after a confirmation.
pub(crate) fn commit_gate(rows: &[StatusRow]) -> CommitGate {
    if rows.iter().any(|row| row.is_conflicted) {
        return CommitGate::BlockedByConflicts;
    }
    if !rows.iter().any(|row| row.staged.is_some()) && rows.iter().any(|row| row.unstaged.is_some())
    {
        return CommitGate::ConfirmStageAll;
    }
    CommitGate::Commit
}

#[cfg(test)]
#[path = "git-scm-model-tests.rs"]
mod tests;
