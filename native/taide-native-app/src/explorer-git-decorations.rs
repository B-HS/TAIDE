use std::collections::HashMap;

use taide_model::git::{GitChangeKind, GitStatus, StatusRow};
use taide_model::ids::ProjectId;

const PATH_SEPARATOR: char = '/';

/// A file tree row's git state (`file-tree-git-status.ts`); `ignored` is never derived.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Decoration {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

impl Decoration {
    pub const ALL: [Self; 6] = [
        Self::Added,
        Self::Modified,
        Self::Deleted,
        Self::Renamed,
        Self::Untracked,
        Self::Conflicted,
    ];

    fn of(kind: GitChangeKind) -> Self {
        match kind {
            GitChangeKind::Modified | GitChangeKind::TypeChange => Self::Modified,
            GitChangeKind::Added => Self::Added,
            GitChangeKind::Deleted => Self::Deleted,
            GitChangeKind::Renamed => Self::Renamed,
            GitChangeKind::Untracked => Self::Untracked,
            GitChangeKind::Conflicted => Self::Conflicted,
        }
    }

    fn priority(self) -> u8 {
        match self {
            Self::Conflicted => 6,
            Self::Added => 5,
            Self::Untracked => 4,
            Self::Renamed => 3,
            Self::Modified => 2,
            Self::Deleted => 1,
        }
    }

    /// The file row's right-edge badge letter (`file-tree-row.tsx` `GIT_STATUS_BADGE`).
    pub fn badge(self) -> &'static str {
        match self {
            Self::Added => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Untracked => "U",
            Self::Conflicted => "!",
        }
    }

    /// The badge's accessible name, the state name as the original `aria-label`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
            Self::Untracked => "untracked",
            Self::Conflicted => "conflicted",
        }
    }

    /// The row text color's theme key (`file-tree-row.tsx` `GIT_STATUS_TEXT_CLASS`).
    pub fn theme_key(self) -> &'static str {
        match self {
            Self::Added => "explorer.gitAdded",
            Self::Modified => "explorer.gitModified",
            Self::Deleted => "explorer.gitDeleted",
            Self::Renamed => "git.renamed",
            Self::Untracked => "explorer.gitUntracked",
            Self::Conflicted => "git.conflicted",
        }
    }
}

fn resolve(row: &StatusRow) -> Option<Decoration> {
    if row.is_conflicted {
        return Some(Decoration::Conflicted);
    }
    [row.staged, row.unstaged]
        .into_iter()
        .flatten()
        .map(Decoration::of)
        .reduce(|best, next| {
            if next.priority() > best.priority() {
                next
            } else {
                best
            }
        })
}

fn parent_dir(path: &str) -> &str {
    match path.rfind(PATH_SEPARATOR) {
        Some(index) if index > 0 => &path[..index],
        _ => "/",
    }
}

/// Per absolute path decorations: each changed file keeps its own state and every ancestor
/// directory strictly below `root` takes the highest-priority state among its descendants.
pub fn build(rows: &[StatusRow], root: Option<&str>) -> HashMap<String, Decoration> {
    let mut decorations = HashMap::new();
    let Some(root) = root else {
        return decorations;
    };
    let mut merge = |path: &str, decoration: Decoration| {
        if decorations
            .get(path)
            .is_some_and(|existing: &Decoration| existing.priority() >= decoration.priority())
        {
            return;
        }
        decorations.insert(path.to_owned(), decoration);
    };
    let prefix = format!("{root}{PATH_SEPARATOR}");
    for row in rows {
        let Some(decoration) = resolve(row) else {
            continue;
        };
        merge(&row.abs_path, decoration);
        let mut dir = parent_dir(&row.abs_path);
        while dir.starts_with(&prefix) {
            merge(dir, decoration);
            dir = parent_dir(dir);
        }
    }
    decorations
}

#[derive(Default)]
struct Entry {
    rows: Option<Vec<StatusRow>>,
    paths: HashMap<String, Decoration>,
}

/// The explorer's git decoration input. A status supplier calls [`GitDecorations::set`] for each
/// of [`GitDecorations::mounted_projects`]; the explorer only reads the derived per-path map.
#[derive(Default)]
pub struct GitDecorations {
    entries: HashMap<ProjectId, Entry>,
    mounted: HashMap<ProjectId, String>,
}

impl GitDecorations {
    /// Records `project`'s git status; `None` (an error or not a repository) clears its
    /// decorations. The map is rebuilt only when the status rows change.
    pub fn set(&mut self, project: &ProjectId, status: Option<&GitStatus>) {
        let Some(status) = status else {
            self.entries.remove(project);
            return;
        };
        let root = self.mounted.get(project).map(String::as_str);
        let entry = self.entries.entry(project.clone()).or_default();
        if entry.rows.as_ref() == Some(&status.rows) {
            return;
        }
        entry.paths = build(&status.rows, root);
        entry.rows = Some(status.rows.clone());
    }

    /// The projects whose explorer currently shows the file tree, in no particular order.
    pub fn mounted_projects(&self) -> Vec<ProjectId> {
        self.mounted.keys().cloned().collect()
    }

    pub fn get(&self, project: &ProjectId) -> Option<&HashMap<String, Decoration>> {
        self.entries.get(project).map(|entry| &entry.paths)
    }

    pub(crate) fn mount(&mut self, mounted: HashMap<ProjectId, String>) {
        for (project, entry) in &mut self.entries {
            let root = mounted.get(project);
            if root != self.mounted.get(project)
                && let Some(rows) = &entry.rows
            {
                entry.paths = build(rows, root.map(String::as_str));
            }
        }
        self.mounted = mounted;
    }

    pub(crate) fn forget(&mut self, project: &ProjectId) {
        self.entries.remove(project);
        self.mounted.remove(project);
    }
}

#[cfg(test)]
#[path = "explorer-git-decorations-tests.rs"]
mod tests;
