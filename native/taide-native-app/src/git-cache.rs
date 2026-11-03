use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use taide_model::app_event::AppEvent;
use taide_model::error::{AppError, AppResult};
use taide_model::git::{
    BlameLine, CommitFile, GitBranch, GitRemote, GitStashEntry, GitStatus, GutterHunk, LogEntry,
    TagInfo,
};
use taide_model::ids::ProjectId;

const STALE_AFTER: Duration = Duration::from_secs(60);
const DISCARD_AFTER: Duration = Duration::from_secs(10 * 60);
pub(crate) const MAX_REQUESTS_PER_FRAME: usize = 16;

/// Which invalidations reach a cached read (`git.query.ts` `isGitQueryScopeMutable`,
/// `ipc-sync-provider.tsx` `isGitWorktreeQueryForChangedPaths`).
pub(crate) trait CacheKey: Clone + Eq + Hash {
    /// Whether a repository change (status or refs event, rescan, successful mutation) reaches it.
    fn is_mutable(&self) -> bool;
    /// Whether a worktree change to these absolute paths reaches it.
    fn follows_worktree(&self, paths: &HashSet<String>) -> bool;
    fn stale_after(&self) -> Option<Duration>;
}

#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct Invalidation {
    repository: bool,
    paths: HashSet<String>,
}

impl Invalidation {
    pub(crate) fn repository() -> Self {
        Self {
            repository: true,
            paths: HashSet::new(),
        }
    }

    fn reaches(&self, key: &impl CacheKey) -> bool {
        (self.repository && key.is_mutable())
            || (!self.paths.is_empty() && key.follows_worktree(&self.paths))
    }
}

#[derive(Default)]
pub(crate) struct Changes {
    projects: Mutex<HashMap<ProjectId, Invalidation>>,
}

impl Changes {
    pub(crate) fn record(&self, event: &AppEvent) {
        let (project, paths) = match event {
            AppEvent::GitStatusChanged { project_id }
            | AppEvent::GitRefsChanged { project_id }
            | AppEvent::FsRescanRequired { project_id } => (project_id, None),
            AppEvent::FsChanged { project_id, change } => (project_id, Some(&change.paths)),
            _ => return,
        };
        let mut projects = self
            .projects
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let invalidation = projects.entry(project.clone()).or_default();
        match paths {
            Some(paths) => invalidation.paths.extend(paths.iter().cloned()),
            None => invalidation.repository = true,
        }
    }

    pub(crate) fn take(&self) -> HashMap<ProjectId, Invalidation> {
        std::mem::take(
            &mut *self
                .projects
                .lock()
                .unwrap_or_else(|error| error.into_inner()),
        )
    }
}

struct Entry<V> {
    value: Option<AppResult<V>>,
    fetching: bool,
    is_stale: bool,
    generation: u64,
    fetched_at: Option<Instant>,
    last_observed: Instant,
    is_observed: bool,
    was_observed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request<K> {
    pub project: ProjectId,
    pub key: K,
    pub generation: u64,
}

/// Per-project git reads with the original query client's rules (`query-client.ts`: 60 second
/// stale time, ten minute retention, no retry): a read is fetched while a surface observes it and
/// it is missing, invalidated, or found older than its stale time by a newly attached observer.
pub(crate) struct GitCache<K, V> {
    projects: HashMap<ProjectId, HashMap<K, Entry<V>>>,
}

impl<K, V> Default for GitCache<K, V> {
    fn default() -> Self {
        Self {
            projects: HashMap::new(),
        }
    }
}

impl<K: CacheKey, V> GitCache<K, V> {
    pub(crate) fn observe(&mut self, project: &ProjectId, key: K, now: Instant) {
        let entry = self
            .projects
            .entry(project.clone())
            .or_default()
            .entry(key)
            .or_insert_with(|| Entry {
                value: None,
                fetching: false,
                is_stale: false,
                generation: 0,
                fetched_at: None,
                last_observed: now,
                is_observed: false,
                was_observed: false,
            });
        entry.last_observed = now;
        entry.is_observed = true;
    }

    pub(crate) fn get(&self, project: &ProjectId, key: &K) -> Option<&AppResult<V>> {
        self.projects.get(project)?.get(key)?.value.as_ref()
    }

    /// When the read last answered, which changes exactly when its value does.
    pub(crate) fn fetched_at(&self, project: &ProjectId, key: &K) -> Option<Instant> {
        self.projects.get(project)?.get(key)?.fetched_at
    }

    pub(crate) fn invalidate(&mut self, project: &ProjectId, invalidation: &Invalidation) {
        for (key, entry) in self.projects.get_mut(project).into_iter().flatten() {
            if invalidation.reaches(key) {
                entry.is_stale = true;
                entry.generation += 1;
            }
        }
    }

    pub(crate) fn retain(&mut self, open: &HashSet<ProjectId>) {
        self.projects.retain(|project, _| open.contains(project));
    }

    pub(crate) fn requests(&mut self, now: Instant) -> Vec<Request<K>> {
        let mut requests = Vec::new();
        for (project, entries) in &mut self.projects {
            entries.retain(|_, entry| {
                entry.fetching || now.duration_since(entry.last_observed) < DISCARD_AFTER
            });
            for (key, entry) in entries.iter_mut() {
                let is_attached = entry.is_observed && !entry.was_observed;
                entry.was_observed = entry.is_observed;
                if !std::mem::take(&mut entry.is_observed) || entry.fetching {
                    continue;
                }
                if is_attached
                    && key.stale_after().is_some_and(|stale_after| {
                        entry
                            .fetched_at
                            .is_none_or(|fetched| now.duration_since(fetched) >= stale_after)
                    })
                {
                    entry.is_stale = true;
                }
                if (entry.value.is_none() || entry.is_stale)
                    && requests.len() < MAX_REQUESTS_PER_FRAME
                {
                    requests.push(Request {
                        project: project.clone(),
                        key: key.clone(),
                        generation: entry.generation,
                    });
                }
            }
        }
        self.projects.retain(|_, entries| !entries.is_empty());
        requests
    }

    pub(crate) fn begin(&mut self, request: &Request<K>) {
        if let Some(entry) = self
            .projects
            .get_mut(&request.project)
            .and_then(|entries| entries.get_mut(&request.key))
        {
            entry.fetching = true;
            entry.is_stale = request.generation != entry.generation;
        }
    }

    pub(crate) fn finish(&mut self, request: &Request<K>, value: AppResult<V>, now: Instant) {
        let Some(entry) = self
            .projects
            .get_mut(&request.project)
            .and_then(|entries| entries.get_mut(&request.key))
        else {
            return;
        };
        entry.fetching = false;
        entry.value = Some(value);
        entry.fetched_at = Some(now);
        if request.generation != entry.generation {
            entry.is_stale = true;
        }
    }

    /// Reads `key` again even though it is fresh (`refetch()`): its next request carries a new
    /// generation, so an answer already in flight leaves it stale.
    pub(crate) fn refetch(&mut self, project: &ProjectId, key: &K) {
        if let Some(entry) = self
            .projects
            .get_mut(project)
            .and_then(|entries| entries.get_mut(key))
        {
            entry.is_stale = true;
            entry.generation += 1;
        }
    }

    pub(crate) fn cancel_fetches(&mut self) {
        for entry in self
            .projects
            .values_mut()
            .flat_map(HashMap::values_mut)
            .filter(|entry| entry.fetching)
        {
            entry.fetching = false;
            entry.is_stale = true;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Status,
    Remotes,
    Branches,
    Stashes,
    Log,
    /// A diff tab's sides (`git_diff_file`): staged is index against HEAD, otherwise the working
    /// tree against the index.
    Diff {
        path: String,
        staged: bool,
        before_path: Option<String>,
    },
    /// A file-against-file compare tab's two file reads (`fileQueryOptions` twice).
    Compare {
        original: String,
        modified: String,
    },
    Tags,
    /// The files one commit changed (`gitCommitFilesQueryOptions`): keyed by an immutable rev, so
    /// no invalidation reaches it and it never goes stale.
    CommitFiles {
        rev: String,
    },
    /// One file of a commit against its first parent (`CommitFileDiff`, two `gitShowFileQueryOptions`
    /// reads): immutable revisions, so no invalidation reaches it and it never goes stale.
    CommitDiff {
        rev: String,
        parent: Option<String>,
        path: String,
        before_path: String,
    },
    /// The first page of one file's commits (`gitFileLogQueryOptions`).
    FileLog {
        path: String,
    },
    /// A file's disk text (`fileQueryOptions`), the original side of a Claude diff.
    File {
        path: String,
    },
    /// A file editor's HEAD-to-working-tree hunks (`gitGutterQueryOptions`).
    Gutter {
        path: String,
    },
    /// The repository user name blame compares authors with (`gitCurrentUserQueryOptions`): never
    /// stale by time.
    CurrentUser,
    /// One settled cursor line's blame (`gitBlameLineQueryOptions`).
    BlameLine {
        path: String,
        line: u32,
    },
    /// The whole-file blame overlay (`gitBlameOverlayQueryOptions`, `staleTime: 0`): `lines` is the
    /// line count when the overlay was turned on.
    BlameOverlay {
        path: String,
        lines: u32,
    },
    /// A conflicted file's ours and theirs texts with their line hunks, read on demand
    /// (`gitConflictSidesQueryOptions`); answered as a diff.
    ConflictSides {
        path: String,
    },
}

impl CacheKey for Key {
    fn is_mutable(&self) -> bool {
        match self {
            Self::Status
            | Self::Remotes
            | Self::Branches
            | Self::Stashes
            | Self::Log
            | Self::Diff { .. }
            | Self::Tags
            | Self::FileLog { .. }
            | Self::Gutter { .. }
            | Self::CurrentUser
            | Self::BlameLine { .. }
            | Self::BlameOverlay { .. }
            | Self::ConflictSides { .. } => true,
            Self::Compare { .. }
            | Self::CommitFiles { .. }
            | Self::CommitDiff { .. }
            | Self::File { .. } => false,
        }
    }

    fn follows_worktree(&self, paths: &HashSet<String>) -> bool {
        match self {
            Self::Status => true,
            Self::File { path } | Self::Gutter { path } => paths.contains(path),
            Self::Remotes
            | Self::Branches
            | Self::Stashes
            | Self::Log
            | Self::Tags
            | Self::CommitFiles { .. }
            | Self::CommitDiff { .. }
            | Self::FileLog { .. }
            | Self::CurrentUser
            | Self::BlameLine { .. }
            | Self::BlameOverlay { .. }
            | Self::ConflictSides { .. } => false,
            Self::Diff {
                path, before_path, ..
            } => {
                paths.contains(path)
                    || before_path
                        .as_ref()
                        .is_some_and(|before| paths.contains(before))
            }
            Self::Compare { original, modified } => {
                paths.contains(original) || paths.contains(modified)
            }
        }
    }

    fn stale_after(&self) -> Option<Duration> {
        match self {
            Self::Status
            | Self::Remotes
            | Self::Branches
            | Self::Stashes
            | Self::Log
            | Self::Diff { .. }
            | Self::Compare { .. }
            | Self::Tags
            | Self::FileLog { .. }
            | Self::File { .. }
            | Self::Gutter { .. }
            | Self::BlameLine { .. }
            | Self::ConflictSides { .. } => Some(STALE_AFTER),
            Self::BlameOverlay { .. } => Some(Duration::ZERO),
            Self::CommitFiles { .. } | Self::CommitDiff { .. } | Self::CurrentUser => None,
        }
    }
}

/// Both texts of a diff with the line hunks between them.
#[derive(Debug)]
pub struct Diff {
    pub sides: taide_model::git::DiffSides,
    pub hunks: Vec<taide_native_ui::diff_view::Hunk>,
}

#[derive(Debug)]
pub enum Value {
    Status(GitStatus),
    Remotes(Vec<GitRemote>),
    Branches(Vec<GitBranch>),
    Stashes(Vec<GitStashEntry>),
    Log(Vec<LogEntry>),
    Diff(Diff),
    Tags(Vec<TagInfo>),
    CommitFiles(Vec<CommitFile>),
    FileLog(Vec<LogEntry>),
    File(taide_model::file::OpenedFile),
    Gutter(Vec<GutterHunk>),
    CurrentUser(Option<String>),
    Blame(Vec<BlameLine>),
}

impl GitCache<Key, Value> {
    /// The project's last status answer, failures included: the supply the source control panel,
    /// the title bar branch and the explorer decorations read.
    pub(crate) fn status_result(
        &self,
        project: &ProjectId,
    ) -> Option<Result<&GitStatus, &AppError>> {
        Some(match self.get(project, &Key::Status)? {
            Ok(Value::Status(status)) => Ok(status),
            Ok(_) => return None,
            Err(error) => Err(error),
        })
    }

    /// The project's remotes; empty until read or when the read failed (`remotes = []`).
    pub(crate) fn remotes(&self, project: &ProjectId) -> &[GitRemote] {
        match self.get(project, &Key::Remotes) {
            Some(Ok(Value::Remotes(remotes))) => remotes,
            _ => &[],
        }
    }

    /// The project's local and remote branches; empty until read or when the read failed.
    pub(crate) fn branches(&self, project: &ProjectId) -> &[GitBranch] {
        match self.get(project, &Key::Branches) {
            Some(Ok(Value::Branches(branches))) => branches,
            _ => &[],
        }
    }

    /// The project's stash entries, newest first; empty until read or when the read failed.
    pub(crate) fn stashes(&self, project: &ProjectId) -> &[GitStashEntry] {
        match self.get(project, &Key::Stashes) {
            Some(Ok(Value::Stashes(stashes))) => stashes,
            _ => &[],
        }
    }

    /// The project's newest commits, first page; empty until read or when the read failed.
    pub(crate) fn log(&self, project: &ProjectId) -> &[LogEntry] {
        match self.get(project, &Key::Log) {
            Some(Ok(Value::Log(log))) => log,
            _ => &[],
        }
    }

    /// A diff or compare read's last answer, failures included; `None` until the first answer.
    pub(crate) fn diff(&self, project: &ProjectId, key: &Key) -> Option<Result<&Diff, &AppError>> {
        Some(match self.get(project, key)? {
            Ok(Value::Diff(diff)) => Ok(diff),
            Ok(_) => return None,
            Err(error) => Err(error),
        })
    }

    /// The project's tags; empty until read or when the read failed (`tags = []`).
    pub(crate) fn tags(&self, project: &ProjectId) -> &[TagInfo] {
        match self.get(project, &Key::Tags) {
            Some(Ok(Value::Tags(tags))) => tags,
            _ => &[],
        }
    }

    /// A commit's changed files, failures included; `None` until the first answer.
    pub(crate) fn commit_files(
        &self,
        project: &ProjectId,
        rev: &str,
    ) -> Option<Result<&[CommitFile], &AppError>> {
        Some(
            match self.get(
                project,
                &Key::CommitFiles {
                    rev: rev.to_owned(),
                },
            )? {
                Ok(Value::CommitFiles(files)) => Ok(files),
                Ok(_) => return None,
                Err(error) => Err(error),
            },
        )
    }

    /// One file's commits, failures included; `None` until the first answer.
    pub(crate) fn file_log(
        &self,
        project: &ProjectId,
        path: &str,
    ) -> Option<Result<&[LogEntry], &AppError>> {
        Some(
            match self.get(
                project,
                &Key::FileLog {
                    path: path.to_owned(),
                },
            )? {
                Ok(Value::FileLog(entries)) => Ok(entries),
                Ok(_) => return None,
                Err(error) => Err(error),
            },
        )
    }

    /// A file's last successful disk read; `None` until read or when the read failed.
    pub(crate) fn file(
        &self,
        project: &ProjectId,
        path: &str,
    ) -> Option<&taide_model::file::OpenedFile> {
        match self.get(
            project,
            &Key::File {
                path: path.to_owned(),
            },
        )? {
            Ok(Value::File(file)) => Some(file),
            _ => None,
        }
    }

    /// A file's working tree hunks; empty until read or when the read failed (`gutterHunks ?? []`).
    pub(crate) fn gutter(&self, project: &ProjectId, path: &str) -> &[GutterHunk] {
        match self.get(
            project,
            &Key::Gutter {
                path: path.to_owned(),
            },
        ) {
            Some(Ok(Value::Gutter(hunks))) => hunks,
            _ => &[],
        }
    }

    /// The repository user; `None` until read, when unset, or when the read failed.
    pub(crate) fn current_user(&self, project: &ProjectId) -> Option<&str> {
        match self.get(project, &Key::CurrentUser) {
            Some(Ok(Value::CurrentUser(user))) => user.as_deref(),
            _ => None,
        }
    }

    /// A blame read's last answer, failures included; `None` until the first answer.
    pub(crate) fn blame(
        &self,
        project: &ProjectId,
        key: &Key,
    ) -> Option<Result<&[BlameLine], &AppError>> {
        Some(match self.get(project, key)? {
            Ok(Value::Blame(lines)) => Ok(lines),
            Ok(_) => return None,
            Err(error) => Err(error),
        })
    }
}

#[cfg(test)]
#[path = "git-cache-tests.rs"]
mod tests;
