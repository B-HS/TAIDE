use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::path::{Component, Path, PathBuf};

use git2::build::CheckoutBuilder;
use git2::Repository;
use serde::Serialize;
use specta::Type;

use crate::error::{AppError, AppResult};

use super::types::{
    BlameLine, CommitFile, CommitOptions, ConflictSides, DiffMode, DiffSides, GitBranch, GitChangeKind, GitRemote, GitStashEntry,
    GitStatus, GutterHunk, HunkKind, LogEntry, RevertOutcome, StatusRow, TagCreateOptions, TagInfo,
};

const DEFAULT_STASH_MESSAGE: &str = "WIP";

/// Regular, non-executable file mode — see [`build_patch_text`]'s doc comment for why a synthetic
/// add/delete patch needs one at all.
const NEW_FILE_MODE: &str = "100644";

const LANGUAGE_ID_BY_EXTENSION: &[(&str, &str)] = &[
    ("ts", "typescript"),
    ("tsx", "typescriptreact"),
    ("js", "javascript"),
    ("jsx", "javascriptreact"),
    ("mjs", "javascript"),
    ("rs", "rust"),
    ("py", "python"),
    ("json", "json"),
    ("md", "markdown"),
    ("toml", "toml"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("css", "css"),
    ("html", "html"),
];
const DEFAULT_LANGUAGE_ID: &str = "plaintext";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AheadBehind {
    pub ahead: u32,
    pub behind: u32,
}

pub fn init(root: &Path) -> AppResult<()> {
    run_git(root, &["init"]).map(|_| ())
}

pub fn discover(root: &Path) -> AppResult<PathBuf> {
    let repo = Repository::discover(root).map_err(|error| AppError::NotFound(format!(".git 저장소를 찾을 수 없습니다: {error}")))?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| AppError::InvalidArgument("bare 저장소는 지원하지 않습니다".to_string()))?;
    Ok(workdir.to_path_buf())
}

pub fn status(repo_path: &Path) -> AppResult<GitStatus> {
    let repo = open_repo(repo_path)?;
    let rows = collect_status_rows(&repo)?;
    let branch = current_branch(&repo);
    let ahead_behind = ahead_behind_of(&repo);
    let has_remote = repo.remotes().map(|remotes| !remotes.is_empty()).unwrap_or(false);

    Ok(GitStatus {
        rows,
        branch,
        ahead: ahead_behind.ahead,
        behind: ahead_behind.behind,
        has_remote,
    })
}

pub fn ahead_behind(repo_path: &Path) -> AppResult<AheadBehind> {
    let repo = open_repo(repo_path)?;
    Ok(ahead_behind_of(&repo))
}

pub fn stage(repo_path: &Path, paths: &[String]) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let mut index = repo.index().map_err(map_git_err)?;

    for raw in paths {
        let relative = to_repo_relative(&workdir, raw)?;
        let relative_path = Path::new(&relative);
        if workdir.join(relative_path).exists() {
            index.add_path(relative_path).map_err(map_git_err)?;
        } else {
            index.remove_path(relative_path).map_err(map_git_err)?;
        }
    }

    index.write().map_err(map_git_err)
}

pub fn unstage(repo_path: &Path, paths: &[String]) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relatives = paths
        .iter()
        .map(|raw| to_repo_relative(&workdir, raw))
        .collect::<AppResult<Vec<_>>>()?;

    let result = match repo.head() {
        Ok(head) => match head.peel(git2::ObjectType::Commit) {
            Ok(target) => repo.reset_default(Some(&target), relatives.iter().map(String::as_str)),
            Err(error) => Err(error),
        },
        Err(_) => repo.reset_default(None, relatives.iter().map(String::as_str)),
    };
    result.map_err(map_git_err)
}

#[cfg(target_os = "macos")]
fn trash_untracked_files(workdir: &Path, relatives: &[String]) -> AppResult<()> {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    use trash::TrashContext;

    let absolute_paths: Vec<PathBuf> = relatives.iter().map(|relative| workdir.join(relative)).collect();
    let mut context = TrashContext::default();
    context.set_delete_method(DeleteMethod::NsFileManager);
    context
        .delete_all(&absolute_paths)
        .map_err(|error| AppError::Internal(format!("휴지통으로 이동하지 못했습니다: {error}")))
}

#[cfg(not(target_os = "macos"))]
fn trash_untracked_files(workdir: &Path, relatives: &[String]) -> AppResult<()> {
    let absolute_paths: Vec<PathBuf> = relatives.iter().map(|relative| workdir.join(relative)).collect();
    trash::delete_all(&absolute_paths).map_err(|error| AppError::Internal(format!("휴지통으로 이동하지 못했습니다: {error}")))
}

pub fn discard(repo_path: &Path, paths: &[String]) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relatives = paths
        .iter()
        .map(|raw| to_repo_relative(&workdir, raw))
        .collect::<AppResult<Vec<_>>>()?;

    let rows = collect_status_rows(&repo)?;
    let (untracked, tracked): (Vec<String>, Vec<String>) = relatives.into_iter().partition(|relative| {
        rows.iter()
            .any(|row| &row.path == relative && row.unstaged == Some(GitChangeKind::Untracked))
    });

    if !untracked.is_empty() {
        trash_untracked_files(&workdir, &untracked)?;
    }

    if tracked.is_empty() {
        return Ok(());
    }

    let mut checkout = CheckoutBuilder::new();
    checkout.force();
    checkout.remove_untracked(false);
    for relative in &tracked {
        checkout.path(relative.as_str());
    }

    repo.checkout_index(None, Some(&mut checkout)).map_err(map_git_err)
}

pub fn commit(repo_path: &Path, message: &str, opts: &CommitOptions) -> AppResult<String> {
    if opts.stage_all {
        run_git(repo_path, &["add", "-A"])?;
    }

    let mut args: Vec<&str> = vec!["commit", "-m", message];
    if opts.amend {
        args.push("--amend");
    }
    run_git(repo_path, &args)?;

    let oid = run_git(repo_path, &["rev-parse", "HEAD"])?;
    Ok(oid.trim().to_string())
}

pub fn push(repo_path: &Path) -> AppResult<()> {
    run_git(repo_path, &["push"]).map(|_| ())
}

pub fn pull(repo_path: &Path) -> AppResult<()> {
    run_git(repo_path, &["pull"]).map(|_| ())
}

pub fn fetch(repo_path: &Path) -> AppResult<()> {
    run_git(repo_path, &["fetch"]).map(|_| ())
}

pub fn undo_last_commit(repo_path: &Path) -> AppResult<()> {
    run_git(repo_path, &["reset", "--soft", "HEAD~1"]).map(|_| ())
}

pub fn current_user(repo_path: &Path) -> AppResult<Option<String>> {
    let repo = open_repo(repo_path)?;
    match repo.signature() {
        Ok(signature) => Ok(signature.name().ok().map(|name| name.to_string())),
        Err(_) => Ok(None),
    }
}

pub fn show_file(repo_path: &Path, rev: &str, path: &str) -> AppResult<String> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let object = repo.revparse_single(rev).map_err(map_git_err)?;
    let commit = object.peel_to_commit().map_err(map_git_err)?;
    let tree = commit.tree().map_err(map_git_err)?;
    let entry = tree
        .get_path(Path::new(&relative))
        .map_err(|error| AppError::NotFound(format!("{relative}: {error}")))?;
    let blob = repo.find_blob(entry.id()).map_err(map_git_err)?;

    Ok(String::from_utf8_lossy(blob.content()).into_owned())
}

pub fn diff_file(repo_path: &Path, path: &str, mode: DiffMode) -> AppResult<DiffSides> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let (original, modified) = match mode {
        DiffMode::WorkdirVsIndex => {
            let original = read_index_blob(&repo, &relative).unwrap_or_default();
            let modified = std::fs::read_to_string(workdir.join(&relative)).unwrap_or_default();
            (original, modified)
        }
        DiffMode::IndexVsHead => {
            let original = read_head_blob(&repo, &relative).unwrap_or_default();
            let modified = read_index_blob(&repo, &relative).unwrap_or_default();
            (original, modified)
        }
    };

    Ok(DiffSides {
        original,
        modified,
        language_id: language_id_for(Path::new(&relative)),
    })
}

pub fn log(repo_path: &Path, skip: usize, take: usize) -> AppResult<Vec<LogEntry>> {
    let repo = open_repo(repo_path)?;
    let mut walk = repo.revwalk().map_err(map_git_err)?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME).map_err(map_git_err)?;
    let _ = walk.push_glob("refs/heads/*");
    let _ = walk.push_head();

    let refs_by_oid = collect_refs_by_oid(&repo);

    let mut out = Vec::with_capacity(take);
    for oid_result in walk.skip(skip).take(take) {
        let oid = oid_result.map_err(map_git_err)?;
        let commit = repo.find_commit(oid).map_err(map_git_err)?;
        let author = commit.author();

        out.push(LogEntry {
            id: oid.to_string(),
            parents: (0..commit.parent_count())
                .filter_map(|index| commit.parent_id(index).ok().map(|parent| parent.to_string()))
                .collect(),
            summary: commit.summary().ok().flatten().unwrap_or_default().to_string(),
            author: author.name().unwrap_or_default().to_string(),
            time_unix: commit.time().seconds() as f64,
            refs: refs_by_oid.get(&oid).cloned().unwrap_or_default(),
        });
    }

    Ok(out)
}

pub fn gutter(repo_path: &Path, path: &str) -> AppResult<Vec<GutterHunk>> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let head_tree = head_tree_of(&repo);

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(&relative)
        .context_lines(0)
        .include_untracked(true)
        .show_untracked_content(true)
        .indent_heuristic(true);

    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))
        .map_err(map_git_err)?;

    let mut hunks: Vec<GutterHunk> = Vec::new();
    diff.foreach(
        &mut |_delta, _progress| true,
        None,
        Some(&mut |_delta, hunk| {
            let kind = hunk_kind(hunk.old_lines(), hunk.new_lines());
            let (start, end) = gutter_range(hunk.new_start(), hunk.new_lines());
            hunks.push(GutterHunk { kind, start, end });
            true
        }),
        None,
    )
    .map_err(map_git_err)?;

    Ok(hunks)
}

fn hunk_kind(old_lines: u32, new_lines: u32) -> HunkKind {
    if old_lines == 0 {
        HunkKind::Added
    } else if new_lines == 0 {
        HunkKind::Deleted
    } else {
        HunkKind::Modified
    }
}

fn gutter_range(new_start: u32, new_lines: u32) -> (u32, u32) {
    if new_lines == 0 {
        let marker = new_start.max(1);
        (marker, marker)
    } else {
        (new_start, new_start + new_lines - 1)
    }
}

pub fn discard_hunk(repo_path: &Path, path: &str, hunk_start: u32, hunk_end: u32) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let head_tree = head_tree_of(&repo);

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(&relative)
        .context_lines(0)
        .include_untracked(true)
        .show_untracked_content(true)
        .indent_heuristic(true);

    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))
        .map_err(map_git_err)?;

    let patch = git2::Patch::from_diff(&diff, 0)
        .map_err(map_git_err)?
        .ok_or_else(|| AppError::NotFound(format!("{relative}: 변경 사항이 없습니다")))?;

    let mut target: Option<(u32, u32, u32, u32)> = None;
    for hunk_index in 0..patch.num_hunks() {
        let (hunk, _lines) = patch.hunk(hunk_index).map_err(map_git_err)?;
        let (start, end) = gutter_range(hunk.new_start(), hunk.new_lines());
        if start == hunk_start && end == hunk_end {
            target = Some((hunk.old_start(), hunk.old_lines(), hunk.new_start(), hunk.new_lines()));
            break;
        }
    }

    let (old_start, old_lines, new_start, new_lines) =
        target.ok_or_else(|| AppError::InvalidArgument(format!("{relative}: 지정한 hunk({hunk_start}-{hunk_end})를 찾을 수 없습니다")))?;

    let old_content = read_head_blob(&repo, &relative).unwrap_or_default();
    let new_content = std::fs::read_to_string(workdir.join(&relative))?;

    let old_lines_vec: Vec<&str> = old_content.split_inclusive('\n').collect();
    let new_lines_vec: Vec<&str> = new_content.split_inclusive('\n').collect();

    let new_range_start = if new_lines == 0 {
        new_start as usize
    } else {
        (new_start - 1) as usize
    };
    let new_range_end = new_range_start + new_lines as usize;
    let old_range_start = if old_lines == 0 {
        old_start as usize
    } else {
        (old_start - 1) as usize
    };
    let old_range_end = old_range_start + old_lines as usize;

    if new_range_end > new_lines_vec.len() || old_range_end > old_lines_vec.len() {
        return Err(AppError::Internal(format!("{relative}: hunk 범위가 파일 길이를 벗어났습니다")));
    }

    let mut rebuilt = String::new();
    rebuilt.push_str(&new_lines_vec[..new_range_start].concat());
    rebuilt.push_str(&old_lines_vec[old_range_start..old_range_end].concat());
    rebuilt.push_str(&new_lines_vec[new_range_end..].concat());

    std::fs::write(workdir.join(&relative), rebuilt).map_err(AppError::from)
}

/// Reads the three sides of an unresolved merge conflict for `path` from the
/// repository index (`git2::Index::conflict_get`'s ancestor/our/their
/// stages), plus the file's current on-disk content. A side is `None` when
/// that stage has no entry — e.g. the ancestor is absent for an add/add
/// conflict, or "ours"/"theirs" is absent for a delete/modify conflict.
pub fn conflict_sides(repo_path: &Path, path: &str) -> AppResult<ConflictSides> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let index = repo.index().map_err(map_git_err)?;
    let conflict = index
        .conflict_get(Path::new(&relative))
        .map_err(|error| AppError::NotFound(format!("{relative}: 충돌 정보를 찾을 수 없습니다: {error}")))?;

    let workdir_content = std::fs::read_to_string(workdir.join(&relative)).unwrap_or_default();

    Ok(ConflictSides {
        base: conflict.ancestor.and_then(|entry| blob_content(&repo, entry.id)),
        ours: conflict.our.and_then(|entry| blob_content(&repo, entry.id)),
        theirs: conflict.their.and_then(|entry| blob_content(&repo, entry.id)),
        workdir: workdir_content,
    })
}

/// Resolves a merge conflict at `path` by writing `content` to the working
/// tree and re-adding the path to the index, which clears all of its
/// conflict stages and replaces them with a single stage-0 entry.
pub fn resolve_conflict(repo_path: &Path, path: &str, content: &str) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    std::fs::write(workdir.join(&relative), content)?;

    let mut index = repo.index().map_err(map_git_err)?;
    index.add_path(Path::new(&relative)).map_err(map_git_err)?;
    index.write().map_err(map_git_err)
}

pub fn stage_hunk(repo_path: &Path, path: &str, hunk_start: u32, hunk_end: u32) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(&relative)
        .context_lines(0)
        .include_untracked(true)
        .show_untracked_content(true)
        .indent_heuristic(true);

    let diff = repo.diff_index_to_workdir(None, Some(&mut opts)).map_err(map_git_err)?;
    let patch_text = build_whole_hunk_patch(&diff, &relative, hunk_start, hunk_end, false)?;

    apply_patch_text(&repo, &patch_text)
}

/// Unstages a single hunk by diffing HEAD against the index with
/// [`git2::DiffOptions::reverse`], which swaps the delta's old/new sides so
/// that `new_file` ends up as HEAD's content — the same mechanism
/// `git2::Repository::reset_default` relies on for whole-file unstage (see
/// libgit2's `reset.c`). The selected hunk is then reconstructed and applied
/// to [`git2::ApplyLocation::Index`] (see [`build_patch_text`]), which moves
/// just that hunk's index content back to what HEAD has.
pub fn unstage_hunk(repo_path: &Path, path: &str, hunk_start: u32, hunk_end: u32) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let head_tree = head_tree_of(&repo);

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(&relative).context_lines(0).indent_heuristic(true).reverse(true);

    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))
        .map_err(map_git_err)?;
    let patch_text = build_whole_hunk_patch(&diff, &relative, hunk_start, hunk_end, true)?;

    apply_patch_text(&repo, &patch_text)
}

pub fn stage_lines(repo_path: &Path, path: &str, line_start: u32, line_end: u32) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(&relative)
        .context_lines(0)
        .include_untracked(true)
        .show_untracked_content(true)
        .indent_heuristic(true);

    let diff = repo.diff_index_to_workdir(None, Some(&mut opts)).map_err(map_git_err)?;
    let patch_text = build_partial_patch(&diff, &relative, line_start..=line_end, false)?;

    apply_patch_text(&repo, &patch_text)
}

pub fn unstage_lines(repo_path: &Path, path: &str, line_start: u32, line_end: u32) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let head_tree = head_tree_of(&repo);

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(&relative).context_lines(0).indent_heuristic(true).reverse(true);

    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))
        .map_err(map_git_err)?;
    let patch_text = build_partial_patch(&diff, &relative, line_start..=line_end, true)?;

    apply_patch_text(&repo, &patch_text)
}

fn apply_patch_text(repo: &Repository, patch_text: &[u8]) -> AppResult<()> {
    let synthetic = git2::Diff::from_buffer(patch_text).map_err(map_git_err)?;
    repo.apply(&synthetic, git2::ApplyLocation::Index, None).map_err(map_git_err)
}

/// Builds a synthetic single-hunk patch that reproduces the hunk whose
/// displayed range equals `(hunk_start, hunk_end)` verbatim (every changed
/// line kept), matched with [`gutter_range`] on the same side
/// `git_gutter`/`git_discard_hunk` already key their hunk boundaries on —
/// see [`build_partial_patch`] for the shared matching/anchoring machinery
/// this reuses by simply selecting every line in the hunk.
fn build_whole_hunk_patch(diff: &git2::Diff, relative: &str, hunk_start: u32, hunk_end: u32, match_old_side: bool) -> AppResult<Vec<u8>> {
    let patch = git2::Patch::from_diff(diff, 0)
        .map_err(map_git_err)?
        .ok_or_else(|| AppError::NotFound(format!("{relative}: 변경 사항이 없습니다")))?;
    let sides = patch_file_sides(&patch);

    for hunk_index in 0..patch.num_hunks() {
        let (hunk, line_count) = patch.hunk(hunk_index).map_err(map_git_err)?;
        let (start, end) = if match_old_side {
            gutter_range(hunk.old_start(), hunk.old_lines())
        } else {
            gutter_range(hunk.new_start(), hunk.new_lines())
        };
        if start != hunk_start || end != hunk_end {
            continue;
        }

        let anchor = HunkAnchor {
            old_start: hunk.old_start(),
            new_start: hunk.new_start(),
            match_old_side,
        };
        let selected = select_hunk_lines(&patch, hunk_index, line_count, anchor, 0..=u32::MAX)?
            .ok_or_else(|| AppError::NotFound(format!("{relative}: 변경 사항이 없습니다")))?;

        return Ok(build_patch_text(relative, &selected, sides));
    }

    Err(AppError::InvalidArgument(format!(
        "{relative}: 지정한 hunk({hunk_start}-{hunk_end})를 찾을 수 없습니다"
    )))
}

/// Builds a minimal single-hunk unified-diff patch for `relative`, keeping
/// only the diff lines whose position on the matching side (see
/// [`build_whole_hunk_patch`]) falls within `line_range`.
///
/// The target hunk is the one whose displayed range overlaps the requested
/// line range; a request spanning more than one hunk is rejected. Within
/// that hunk, unselected additions are dropped and unselected removals are
/// turned back into context lines — the standard "stage selected lines"
/// transform — so the reconstructed patch only touches the lines the caller
/// asked for.
fn build_partial_patch(diff: &git2::Diff, relative: &str, line_range: RangeInclusive<u32>, match_old_side: bool) -> AppResult<Vec<u8>> {
    let patch = git2::Patch::from_diff(diff, 0)
        .map_err(map_git_err)?
        .ok_or_else(|| AppError::NotFound(format!("{relative}: 변경 사항이 없습니다")))?;
    let sides = patch_file_sides(&patch);

    for hunk_index in 0..patch.num_hunks() {
        let (hunk, line_count) = patch.hunk(hunk_index).map_err(map_git_err)?;
        let (range_start, range_end) = if match_old_side {
            gutter_range(hunk.old_start(), hunk.old_lines())
        } else {
            gutter_range(hunk.new_start(), hunk.new_lines())
        };
        if *line_range.end() < range_start || range_end < *line_range.start() {
            continue;
        }

        if hunk_overlaps_another(&patch, hunk_index, match_old_side, &line_range)? {
            return Err(AppError::InvalidArgument(format!(
                "{relative}: 지정한 범위({}-{})가 여러 hunk에 걸쳐 있습니다",
                line_range.start(),
                line_range.end()
            )));
        }

        let anchor = HunkAnchor {
            old_start: hunk.old_start(),
            new_start: hunk.new_start(),
            match_old_side,
        };
        let selected = select_hunk_lines(&patch, hunk_index, line_count, anchor, line_range.clone())?.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "{relative}: 지정한 범위({}-{})에 변경된 라인이 없습니다",
                line_range.start(),
                line_range.end()
            ))
        })?;

        return Ok(build_patch_text(relative, &selected, sides));
    }

    Err(AppError::InvalidArgument(format!(
        "{relative}: 지정한 범위({}-{})를 포함하는 hunk를 찾을 수 없습니다",
        line_range.start(),
        line_range.end()
    )))
}

/// Whether any hunk *after* `current_index` also overlaps `line_range` — hunks come out of
/// [`git2::Patch`] in ascending file order, so a range that reaches a second hunk always shows up
/// as an overlap with one further along, never an earlier one already passed over.
fn hunk_overlaps_another(
    patch: &git2::Patch,
    current_index: usize,
    match_old_side: bool,
    line_range: &RangeInclusive<u32>,
) -> AppResult<bool> {
    for hunk_index in (current_index + 1)..patch.num_hunks() {
        let (hunk, _) = patch.hunk(hunk_index).map_err(map_git_err)?;
        let (range_start, range_end) = if match_old_side {
            gutter_range(hunk.old_start(), hunk.old_lines())
        } else {
            gutter_range(hunk.new_start(), hunk.new_lines())
        };
        if range_start > *line_range.end() {
            break;
        }
        if *line_range.end() >= range_start && range_end >= *line_range.start() {
            return Ok(true);
        }
    }
    Ok(false)
}

struct SelectedHunk {
    old_start: u32,
    old_count: u32,
    new_count: u32,
    body: Vec<u8>,
}

#[derive(Clone, Copy)]
struct HunkAnchor {
    old_start: u32,
    new_start: u32,
    match_old_side: bool,
}

/// Walks every line of one hunk, computing each line's "position" as the
/// running line counter on the matching side (context and same-side lines
/// use their own counter; a line absent from the matching side inherits the
/// counter's current, un-incremented value — the position it would occupy
/// if kept, i.e. a pure deletion is anchored to the line right after it).
/// Lines outside `line_range` are dropped (additions) or turned into
/// context (removals); the hunk header counts are recomputed from what's
/// left. Returns `None` when nothing in range was actually a change.
///
/// Lines at the end of a file with no trailing newline are rejected —
/// reconstructing the unified-diff "\ No newline at end of file" marker
/// correctly is out of scope for this partial-line transform.
fn select_hunk_lines(
    patch: &git2::Patch,
    hunk_index: usize,
    line_count: usize,
    anchor: HunkAnchor,
    line_range: RangeInclusive<u32>,
) -> AppResult<Option<SelectedHunk>> {
    let HunkAnchor {
        old_start,
        new_start,
        match_old_side,
    } = anchor;
    let mut old_cursor = old_start;
    let mut new_cursor = new_start;
    let mut old_count = 0u32;
    let mut new_count = 0u32;
    let mut body = Vec::new();
    let mut selected_any = false;

    for line_index in 0..line_count {
        let line = patch.line_in_hunk(hunk_index, line_index).map_err(map_git_err)?;
        let position = if match_old_side { old_cursor } else { new_cursor };

        match line.origin() {
            ' ' => {
                write_patch_line(&mut body, ' ', line.content());
                old_count += 1;
                new_count += 1;
                old_cursor += 1;
                new_cursor += 1;
            }
            '+' => {
                if line_range.contains(&position) {
                    write_patch_line(&mut body, '+', line.content());
                    new_count += 1;
                    selected_any = true;
                }
                new_cursor += 1;
            }
            '-' => {
                if line_range.contains(&position) {
                    write_patch_line(&mut body, '-', line.content());
                    old_count += 1;
                    selected_any = true;
                } else {
                    write_patch_line(&mut body, ' ', line.content());
                    old_count += 1;
                    new_count += 1;
                }
                old_cursor += 1;
            }
            _ => {
                return Err(AppError::InvalidArgument(
                    "파일 끝에 개행이 없는 hunk는 라인 단위 스테이지를 지원하지 않습니다".to_string(),
                ));
            }
        }
    }

    if !selected_any {
        return Ok(None);
    }

    Ok(Some(SelectedHunk {
        old_start,
        old_count,
        new_count,
        body,
    }))
}

fn write_patch_line(body: &mut Vec<u8>, prefix: char, content: &[u8]) {
    body.push(prefix as u8);
    body.extend_from_slice(content);
}

/// Whether the file this patch's diff describes exists on the diff's `old`/`new` side, read once
/// from the delta shared by every hunk in a single-file [`git2::Patch`]. Feeds the `---`/`+++`
/// headers [`build_patch_text`] writes — a side that doesn't exist must be marked `/dev/null`, or
/// `git2::Repository::apply` misreads the delta as a same-path modification and tries (and fails)
/// to read a preimage/postimage that was never there. See [`build_patch_text`] for the concrete
/// failure this prevents (staging a hunk in a brand-new untracked file).
#[derive(Clone, Copy)]
struct PatchFileSides {
    old_exists: bool,
    new_exists: bool,
}

fn patch_file_sides(patch: &git2::Patch) -> PatchFileSides {
    let delta = patch.delta();
    PatchFileSides {
        old_exists: delta.old_file().exists(),
        new_exists: delta.new_file().exists(),
    }
}

/// Writes the hunk's header using `old_start` for *both* the `-` and `+`
/// positions (offset by one when the preimage is empty), rather than the
/// hunk's real `new_start`.
///
/// This looks wrong but is required: `git_apply` locates a hunk by seeking
/// to `new_start - 1` in whatever it's patching (`libgit2`'s `apply.c`,
/// `apply_hunk`, unconditionally reads `hunk->hunk.new_start` regardless of
/// [`git2::ApplyLocation`]) and then verifies the preimage there. Every
/// diff this module builds is patched with [`git2::ApplyLocation::Index`],
/// where that target is always the *current index* — which is what the
/// diff's `old` side represents (`stage_hunk`/`stage_lines` diff straight
/// from the index; `unstage_hunk`/`unstage_lines` diff HEAD against the
/// index with `reverse(true)`, which swaps old/new so `old` is the index
/// again). So the seek position `git_apply` computes must land on
/// `old_start`'s position, not the hunk's real (and, for an add/delete-only
/// hunk with zero context, numerically different) `new_start`. The one
/// exception is a hunk with an empty preimage (a pure insertion,
/// `old_count == 0`): there's nothing to verify, and the correct splice
/// point is one line further along than `old_start` marks (GNU diff's
/// zero-count convention points `old_start` at the line *before* the
/// insertion) — `+ 1` reproduces that.
///
/// The `---`/`+++` paths themselves fall back to `/dev/null` when [`PatchFileSides`] says that
/// side has no file — e.g. staging a hunk in a file that doesn't exist in the index yet
/// (untracked). That alone isn't enough, though: `libgit2`'s patch parser (`patch_parse.c`,
/// `check_filenames`/`check_header_names`) only *accepts* a `/dev/null` path when it already
/// believes the delta is an add or a delete, which it learns solely from an explicit `new file
/// mode`/`deleted file mode` header line — never inferred from the hunk's `-0,0`/`+0,0` line
/// counts. Without that header line, the parser still expects `---`/`+++` to echo the `diff --git
/// a/{relative} b/{relative}` line's paths verbatim and rejects a lone `/dev/null` with
/// `"mismatched old/new path names"`. [`NEW_FILE_MODE`] is the (assumed non-executable) mode
/// written into the new index entry a `new file mode` line produces — this module has no access
/// to the workdir file's real permission bits at this point, so a hunk/line stage of an untracked
/// *executable* script loses its exec bit; staging the whole file (which reads the real mode via
/// `git2::Index::add_path`) does not have this limitation.
fn build_patch_text(relative: &str, hunk: &SelectedHunk, sides: PatchFileSides) -> Vec<u8> {
    let new_start = hunk.old_start + u32::from(hunk.old_count == 0);
    let old_path = if sides.old_exists {
        format!("a/{relative}")
    } else {
        "/dev/null".to_string()
    };
    let new_path = if sides.new_exists {
        format!("b/{relative}")
    } else {
        "/dev/null".to_string()
    };
    let mut text = Vec::new();
    text.extend_from_slice(format!("diff --git a/{relative} b/{relative}\n").as_bytes());
    if !sides.old_exists {
        text.extend_from_slice(format!("new file mode {NEW_FILE_MODE}\n").as_bytes());
    } else if !sides.new_exists {
        text.extend_from_slice(format!("deleted file mode {NEW_FILE_MODE}\n").as_bytes());
    }
    text.extend_from_slice(format!("--- {old_path}\n").as_bytes());
    text.extend_from_slice(format!("+++ {new_path}\n").as_bytes());
    text.extend_from_slice(format!("@@ -{},{} +{new_start},{} @@\n", hunk.old_start, hunk.old_count, hunk.new_count).as_bytes());
    text.extend_from_slice(&hunk.body);
    text
}

pub fn commit_files(repo_path: &Path, rev: &str) -> AppResult<Vec<CommitFile>> {
    let repo = open_repo(repo_path)?;
    let object = repo.revparse_single(rev).map_err(map_git_err)?;
    let commit = object.peel_to_commit().map_err(map_git_err)?;
    let tree = commit.tree().map_err(map_git_err)?;
    let parent_tree = first_parent_tree(&commit)?;

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(map_git_err)?;

    let mut out = Vec::new();
    for delta in diff.deltas() {
        let Some(kind) = delta_change_kind(delta.status()) else { continue };
        let new_path = delta.new_file().path().map(|p| p.to_string_lossy().into_owned());
        let old_path = delta.old_file().path().map(|p| p.to_string_lossy().into_owned());
        let path = new_path.clone().or_else(|| old_path.clone()).unwrap_or_default();
        let orig_path = old_path.filter(|orig| Some(orig) != new_path.as_ref());

        out.push(CommitFile { path, orig_path, kind });
    }

    Ok(out)
}

fn first_parent_tree<'repo>(commit: &git2::Commit<'repo>) -> AppResult<Option<git2::Tree<'repo>>> {
    if commit.parent_count() == 0 {
        return Ok(None);
    }
    let parent = commit.parent(0).map_err(map_git_err)?;
    Ok(Some(parent.tree().map_err(map_git_err)?))
}

fn delta_change_kind(status: git2::Delta) -> Option<GitChangeKind> {
    match status {
        git2::Delta::Added => Some(GitChangeKind::Added),
        git2::Delta::Deleted => Some(GitChangeKind::Deleted),
        git2::Delta::Modified => Some(GitChangeKind::Modified),
        git2::Delta::Renamed => Some(GitChangeKind::Renamed),
        git2::Delta::Copied => Some(GitChangeKind::Added),
        git2::Delta::Typechange => Some(GitChangeKind::TypeChange),
        _ => None,
    }
}

/// Lists the commits touching `path`, newest first. Each candidate commit is
/// diffed against its first parent with a pathspec filter (no `--follow`
/// rename tracking); `skip`/`take` paginate over the filtered result, not
/// over the raw revwalk.
pub fn file_log(repo_path: &Path, path: &str, skip: usize, take: usize) -> AppResult<Vec<LogEntry>> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let mut walk = repo.revwalk().map_err(map_git_err)?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME).map_err(map_git_err)?;
    let _ = walk.push_glob("refs/heads/*");
    let _ = walk.push_head();

    let refs_by_oid = collect_refs_by_oid(&repo);

    let mut out = Vec::with_capacity(take);
    let mut skipped = 0usize;

    for oid_result in walk {
        let oid = oid_result.map_err(map_git_err)?;
        let commit = repo.find_commit(oid).map_err(map_git_err)?;

        if !commit_touches_path(&repo, &commit, &relative)? {
            continue;
        }
        if skipped < skip {
            skipped += 1;
            continue;
        }
        if out.len() >= take {
            break;
        }

        let author = commit.author();
        out.push(LogEntry {
            id: oid.to_string(),
            parents: (0..commit.parent_count())
                .filter_map(|index| commit.parent_id(index).ok().map(|parent| parent.to_string()))
                .collect(),
            summary: commit.summary().ok().flatten().unwrap_or_default().to_string(),
            author: author.name().unwrap_or_default().to_string(),
            time_unix: commit.time().seconds() as f64,
            refs: refs_by_oid.get(&oid).cloned().unwrap_or_default(),
        });
    }

    Ok(out)
}

fn commit_touches_path(repo: &Repository, commit: &git2::Commit, relative: &str) -> AppResult<bool> {
    let tree = commit.tree().map_err(map_git_err)?;
    let parent_tree = first_parent_tree(commit)?;

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(relative);
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))
        .map_err(map_git_err)?;

    Ok(diff.deltas().len() > 0)
}

/// Reverts `rev` via `git2::Repository::revert`, which stages the result
/// without committing (mirrors `git revert --no-commit`). When the result is
/// conflict-free, this then creates the revert commit itself — reusing the
/// message `git_revert` already wrote to `MERGE_MSG` — and cleans up the
/// revert state files, so a single call matches plain `git revert`'s default
/// (auto-commit) behavior. When conflicts remain, the revert state is left
/// in place for the conflict-resolution commands
/// (`conflict_sides`/`resolve_conflict`) to pick up, and the caller must
/// finish with `git_commit` afterward.
///
/// Requires a clean index (no staged changes relative to HEAD) before starting, matching plain
/// `git revert`'s refusal to run against a dirty index — `git2::Repository::revert` has no such
/// guard on its own, and the auto-commit path below calls `index.write_tree()`, which captures
/// the *entire* current index. Without this check, unrelated staged work already sitting in the
/// index would be silently folded into the revert commit.
pub fn revert_commit(repo_path: &Path, rev: &str) -> AppResult<RevertOutcome> {
    let repo = open_repo(repo_path)?;
    let object = repo.revparse_single(rev).map_err(map_git_err)?;
    let commit = object.peel_to_commit().map_err(map_git_err)?;

    ensure_clean_index(&repo)?;

    repo.revert(&commit, None).map_err(map_git_err)?;

    let mut index = repo.index().map_err(map_git_err)?;
    if index.has_conflicts() {
        return Ok(RevertOutcome {
            conflicted: true,
            conflicted_paths: conflicted_index_paths(&index)?,
        });
    }

    let tree_oid = index.write_tree().map_err(map_git_err)?;
    let tree = repo.find_tree(tree_oid).map_err(map_git_err)?;
    let head_commit = repo.head().map_err(map_git_err)?.peel_to_commit().map_err(map_git_err)?;
    let message = repo
        .message()
        .unwrap_or_else(|_| format!("Revert \"{}\"\n", commit.summary().ok().flatten().unwrap_or_default()));
    let signature = repo
        .signature()
        .or_else(|_| git2::Signature::now("TAIDE", "taide@local"))
        .map_err(map_git_err)?;

    repo.commit(Some("HEAD"), &signature, &signature, &message, &tree, &[&head_commit])
        .map_err(map_git_err)?;
    repo.cleanup_state().map_err(map_git_err)?;

    Ok(RevertOutcome {
        conflicted: false,
        conflicted_paths: Vec::new(),
    })
}

/// Distinct paths across every conflict stage in `index`, in first-seen order — each
/// [`git2::IndexConflict`] carries up to three stages (ancestor/our/their) of the *same* path, so
/// this collapses them to one entry per conflicted file.
fn conflicted_index_paths(index: &git2::Index) -> AppResult<Vec<String>> {
    let mut paths = Vec::new();
    for conflict in index.conflicts().map_err(map_git_err)? {
        let conflict = conflict.map_err(map_git_err)?;
        let Some(path) = conflict
            .ancestor
            .or(conflict.our)
            .or(conflict.their)
            .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
        else {
            continue;
        };
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub fn tags(repo_path: &Path) -> AppResult<Vec<TagInfo>> {
    let repo = open_repo(repo_path)?;
    let names = repo.tag_names(None).map_err(map_git_err)?;

    let mut out = Vec::new();
    for name in names.iter().flatten().flatten() {
        let Ok(reference) = repo.find_reference(&format!("refs/tags/{name}")) else {
            continue;
        };
        let Some(target_oid) = reference.target() else { continue };

        out.push(match repo.find_tag(target_oid) {
            Ok(tag) => TagInfo {
                name: name.to_string(),
                target: tag.target_id().to_string(),
                message: tag.message().ok().flatten().map(str::to_string),
                annotated: true,
            },
            Err(_) => TagInfo {
                name: name.to_string(),
                target: target_oid.to_string(),
                message: None,
                annotated: false,
            },
        });
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Creates a tag. An annotated tag is created only when both `opts.annotated`
/// is requested and a non-empty `opts.message` is given — an annotated tag
/// without a message would need an interactive editor, so that combination
/// silently falls back to a lightweight tag instead, matching how the
/// desktop app has no editor prompt to fall back on.
pub fn tag_create(repo_path: &Path, name: &str, target: &str, opts: &TagCreateOptions) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let object = repo.revparse_single(target).map_err(map_git_err)?;
    let message = opts.message.as_deref().filter(|value| !value.trim().is_empty());

    match (opts.annotated, message) {
        (true, Some(message)) => {
            let signature = repo
                .signature()
                .or_else(|_| git2::Signature::now("TAIDE", "taide@local"))
                .map_err(map_git_err)?;
            repo.tag(name, &object, &signature, message, false).map_err(map_git_err)?;
        }
        _ => {
            repo.tag_lightweight(name, &object, false).map_err(map_git_err)?;
        }
    }

    Ok(())
}

pub fn tag_delete(repo_path: &Path, name: &str) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    repo.tag_delete(name).map_err(map_git_err)
}

/// Checks out a remote-tracking branch (e.g. `origin/foo`) by creating (or
/// reusing) a local branch of the same short name that tracks it, then
/// checking that local branch out — fixing the previous behavior where
/// checking out a remote ref left HEAD detached. A pre-existing local branch
/// of that name is checked out as-is rather than treated as an error.
pub fn checkout_remote_branch(repo_path: &Path, remote_ref: &str) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let remote_branch = repo
        .find_branch(remote_ref, git2::BranchType::Remote)
        .map_err(|error| AppError::NotFound(format!("{remote_ref}: 원격 브랜치를 찾을 수 없습니다: {error}")))?;

    let local_name = local_branch_name_for(&repo, remote_ref)?;

    if repo.find_branch(&local_name, git2::BranchType::Local).is_err() {
        let target_commit = remote_branch.get().peel_to_commit().map_err(map_git_err)?;
        let mut local_branch = repo.branch(&local_name, &target_commit, false).map_err(map_git_err)?;
        local_branch.set_upstream(Some(remote_ref)).map_err(map_git_err)?;
    }

    checkout_ref(&repo, &format!("refs/heads/{local_name}"))
}

fn local_branch_name_for(repo: &Repository, remote_ref: &str) -> AppResult<String> {
    if let Ok(names) = repo.remotes() {
        for name in names.iter().flatten().flatten() {
            if let Some(stripped) = remote_ref.strip_prefix(&format!("{name}/")) {
                return Ok(stripped.to_string());
            }
        }
    }

    remote_ref
        .split_once('/')
        .map(|(_, rest)| rest.to_string())
        .ok_or_else(|| AppError::InvalidArgument(format!("{remote_ref}: 원격 브랜치 이름 형식이 아닙니다")))
}

pub fn blame_range(repo_path: &Path, path: &str, from: u32, to: u32) -> AppResult<Vec<BlameLine>> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let mut opts = git2::BlameOptions::new();
    opts.first_parent(true)
        .use_mailmap(true)
        .ignore_whitespace(true)
        .min_line(from as usize)
        .max_line(to as usize);

    let blame = repo.blame_file(Path::new(&relative), Some(&mut opts)).map_err(map_git_err)?;

    let mut out = Vec::new();
    for hunk in blame.iter() {
        let oid = hunk.final_commit_id();
        let is_uncommitted = oid.is_zero();
        let signature = hunk.final_signature();
        let author = signature.as_ref().and_then(|sig| sig.name().ok()).unwrap_or_default().to_string();
        let time_unix = signature.as_ref().map(|sig| sig.when().seconds()).unwrap_or(0) as f64;
        let summary = if is_uncommitted {
            String::new()
        } else {
            repo.find_commit(oid)
                .ok()
                .and_then(|commit| commit.summary().ok().flatten().map(str::to_string))
                .unwrap_or_default()
        };

        let start = hunk.final_start_line() as u32;
        for offset in 0..hunk.lines_in_hunk() as u32 {
            out.push(BlameLine {
                line: start + offset,
                commit_id: oid.to_string(),
                author: author.clone(),
                time_unix,
                summary: summary.clone(),
                is_uncommitted,
            });
        }
    }

    Ok(out)
}

pub fn remotes(repo_path: &Path) -> AppResult<Vec<GitRemote>> {
    let repo = open_repo(repo_path)?;
    let names = repo.remotes().map_err(map_git_err)?;

    let mut out = Vec::new();
    for name in names.iter().flatten().flatten() {
        let Ok(remote) = repo.find_remote(name) else { continue };
        let Ok(url) = remote.url() else { continue };
        out.push(GitRemote {
            name: name.to_string(),
            url: url.to_string(),
        });
    }

    Ok(out)
}

pub fn branches(repo_path: &Path) -> AppResult<Vec<GitBranch>> {
    let repo = open_repo(repo_path)?;
    let branches = repo.branches(None).map_err(map_git_err)?;

    let mut out = Vec::new();
    for branch_result in branches {
        let (branch, branch_type) = branch_result.map_err(map_git_err)?;
        let Some(name) = branch.name().map_err(map_git_err)? else {
            continue;
        };
        if branch_type == git2::BranchType::Remote && name.ends_with("/HEAD") {
            continue;
        }

        let is_remote = branch_type == git2::BranchType::Remote;
        let is_head = !is_remote && branch.is_head();
        let upstream = if is_remote {
            None
        } else {
            branch
                .upstream()
                .ok()
                .and_then(|upstream| upstream.name().ok().flatten().map(str::to_string))
        };

        out.push(GitBranch {
            name: name.to_string(),
            is_head,
            is_remote,
            upstream,
        });
    }

    Ok(out)
}

pub fn branch_create(repo_path: &Path, name: &str, checkout: bool) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let head_commit = repo.head().map_err(map_git_err)?.peel_to_commit().map_err(map_git_err)?;
    repo.branch(name, &head_commit, false).map_err(map_git_err)?;

    if checkout {
        checkout_ref(&repo, &format!("refs/heads/{name}"))?;
    }

    Ok(())
}

pub fn branch_checkout(repo_path: &Path, name: &str) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    checkout_ref(&repo, name)
}

pub fn branch_delete(repo_path: &Path, name: &str, force: bool) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let mut branch = repo.find_branch(name, git2::BranchType::Local).map_err(map_git_err)?;

    if branch.is_head() {
        return Err(AppError::InvalidArgument("현재 체크아웃된 브랜치는 삭제할 수 없습니다".to_string()));
    }

    if !force {
        let head_oid = repo
            .head()
            .map_err(map_git_err)?
            .target()
            .ok_or_else(|| AppError::Internal("HEAD 를 확인할 수 없습니다".to_string()))?;
        let branch_oid = branch
            .get()
            .target()
            .ok_or_else(|| AppError::Internal("브랜치 대상을 확인할 수 없습니다".to_string()))?;
        let is_merged = head_oid == branch_oid || repo.graph_descendant_of(head_oid, branch_oid).unwrap_or(false);
        if !is_merged {
            return Err(AppError::InvalidArgument(format!(
                "'{name}' 브랜치는 병합되지 않았습니다. 강제 삭제하려면 force 를 사용하세요"
            )));
        }
    }

    branch.delete().map_err(map_git_err)
}

pub fn stash_list(repo_path: &Path) -> AppResult<Vec<GitStashEntry>> {
    let mut repo = open_repo(repo_path)?;
    let mut out = Vec::new();
    repo.stash_foreach(|index, message, _oid| {
        out.push(GitStashEntry {
            index: index as u32,
            message: message.to_string(),
        });
        true
    })
    .map_err(map_git_err)?;
    Ok(out)
}

pub fn stash_push(repo_path: &Path, message: Option<&str>) -> AppResult<()> {
    let mut repo = open_repo(repo_path)?;
    let signature = repo
        .signature()
        .or_else(|_| git2::Signature::now("TAIDE", "taide@local"))
        .map_err(map_git_err)?;
    repo.stash_save(
        &signature,
        message.unwrap_or(DEFAULT_STASH_MESSAGE),
        Some(git2::StashFlags::INCLUDE_UNTRACKED),
    )
    .map_err(map_git_err)?;
    Ok(())
}

pub fn stash_apply(repo_path: &Path, index: u32) -> AppResult<()> {
    let mut repo = open_repo(repo_path)?;
    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    let mut apply_opts = git2::StashApplyOptions::new();
    apply_opts.checkout_options(checkout);

    repo.stash_apply(index as usize, Some(&mut apply_opts)).map_err(|error| {
        if error.code() == git2::ErrorCode::Conflict {
            AppError::InvalidArgument(format!("스태시를 적용하는 중 충돌이 발생했습니다: {error}"))
        } else {
            map_git_err(error)
        }
    })
}

pub fn stash_drop(repo_path: &Path, index: u32) -> AppResult<()> {
    let mut repo = open_repo(repo_path)?;
    repo.stash_drop(index as usize).map_err(map_git_err)
}

fn checkout_ref(repo: &Repository, refname: &str) -> AppResult<()> {
    let (object, reference) = repo.revparse_ext(refname).map_err(map_git_err)?;

    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    repo.checkout_tree(&object, Some(&mut checkout)).map_err(|error| {
        if error.code() == git2::ErrorCode::Conflict {
            AppError::InvalidArgument(format!("커밋되지 않은 변경 사항과 충돌하여 체크아웃할 수 없습니다: {error}"))
        } else {
            map_git_err(error)
        }
    })?;

    match reference {
        Some(reference) if reference.is_branch() => {
            let branch_ref = reference.name().map_err(map_git_err)?;
            repo.set_head(branch_ref).map_err(map_git_err)
        }
        _ => repo.set_head_detached(object.id()).map_err(map_git_err),
    }
}

fn open_repo(repo_path: &Path) -> AppResult<Repository> {
    Repository::open(repo_path).map_err(|error| AppError::NotFound(format!("git 저장소를 열 수 없습니다: {error}")))
}

fn repo_workdir(repo: &Repository) -> AppResult<PathBuf> {
    repo.workdir()
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::InvalidArgument("bare 저장소는 지원하지 않습니다".to_string()))
}

fn to_repo_relative(repo_root: &Path, raw: &str) -> AppResult<String> {
    let normalized = raw.replace('\\', "/");
    let candidate = Path::new(&normalized);
    let relative = if candidate.is_relative() {
        candidate.to_path_buf()
    } else {
        candidate
            .strip_prefix(repo_root)
            .map_err(|_| AppError::InvalidArgument(format!("경로가 저장소 밖에 있습니다: {raw}")))?
            .to_path_buf()
    };

    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        return Err(AppError::InvalidArgument(format!("경로가 저장소 밖에 있습니다: {raw}")));
    }

    Ok(relative.to_string_lossy().to_string())
}

fn collect_status_rows(repo: &Repository) -> AppResult<Vec<StatusRow>> {
    let mut opts = git2::StatusOptions::new();
    opts.show(git2::StatusShow::IndexAndWorkdir)
        .include_untracked(true)
        .recurse_untracked_dirs(false)
        .include_ignored(false)
        .include_unmodified(false)
        .exclude_submodules(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .update_index(true);

    let statuses = repo.statuses(Some(&mut opts)).map_err(map_git_err)?;

    let mut rows = Vec::with_capacity(statuses.len());
    for entry in statuses.iter() {
        let flags = entry.status();
        if flags.contains(git2::Status::IGNORED) {
            continue;
        }

        let Ok(path) = entry.path() else { continue };

        let orig_path = entry
            .head_to_index()
            .or_else(|| entry.index_to_workdir())
            .and_then(|delta| delta.old_file().path().map(|p| p.to_string_lossy().into_owned()))
            .filter(|orig| orig != path);

        rows.push(StatusRow {
            path: path.to_string(),
            orig_path,
            staged: staged_change_kind(flags),
            unstaged: unstaged_change_kind(flags),
            is_conflicted: flags.contains(git2::Status::CONFLICTED),
        });
    }

    Ok(rows)
}

fn staged_change_kind(flags: git2::Status) -> Option<GitChangeKind> {
    if flags.contains(git2::Status::INDEX_NEW) {
        Some(GitChangeKind::Added)
    } else if flags.contains(git2::Status::INDEX_MODIFIED) {
        Some(GitChangeKind::Modified)
    } else if flags.contains(git2::Status::INDEX_DELETED) {
        Some(GitChangeKind::Deleted)
    } else if flags.contains(git2::Status::INDEX_RENAMED) {
        Some(GitChangeKind::Renamed)
    } else if flags.contains(git2::Status::INDEX_TYPECHANGE) {
        Some(GitChangeKind::TypeChange)
    } else {
        None
    }
}

fn unstaged_change_kind(flags: git2::Status) -> Option<GitChangeKind> {
    if flags.contains(git2::Status::CONFLICTED) {
        Some(GitChangeKind::Conflicted)
    } else if flags.contains(git2::Status::WT_NEW) {
        Some(GitChangeKind::Untracked)
    } else if flags.contains(git2::Status::WT_MODIFIED) {
        Some(GitChangeKind::Modified)
    } else if flags.contains(git2::Status::WT_DELETED) {
        Some(GitChangeKind::Deleted)
    } else if flags.contains(git2::Status::WT_RENAMED) {
        Some(GitChangeKind::Renamed)
    } else if flags.contains(git2::Status::WT_TYPECHANGE) {
        Some(GitChangeKind::TypeChange)
    } else {
        None
    }
}

fn current_branch(repo: &Repository) -> Option<String> {
    match repo.head() {
        Ok(head) => head.shorthand().ok().map(str::to_string),
        Err(_) => repo
            .find_reference("HEAD")
            .ok()
            .and_then(|reference| reference.symbolic_target().ok().flatten().map(str::to_string))
            .map(|target| target.trim_start_matches("refs/heads/").to_string()),
    }
}

fn ahead_behind_of(repo: &Repository) -> AheadBehind {
    let Ok(head) = repo.head() else {
        return AheadBehind { ahead: 0, behind: 0 };
    };
    let Some(local) = head.target() else {
        return AheadBehind { ahead: 0, behind: 0 };
    };
    let branch = git2::Branch::wrap(head);
    let Ok(upstream) = branch.upstream() else {
        return AheadBehind { ahead: 0, behind: 0 };
    };
    let Some(upstream_oid) = upstream.get().target() else {
        return AheadBehind { ahead: 0, behind: 0 };
    };

    match repo.graph_ahead_behind(local, upstream_oid) {
        Ok((ahead, behind)) => AheadBehind {
            ahead: ahead as u32,
            behind: behind as u32,
        },
        Err(_) => AheadBehind { ahead: 0, behind: 0 },
    }
}

fn collect_refs_by_oid(repo: &Repository) -> HashMap<git2::Oid, Vec<String>> {
    let mut map: HashMap<git2::Oid, Vec<String>> = HashMap::new();

    if let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) {
        for branch_result in branches.flatten() {
            let (branch, _) = branch_result;
            if let (Ok(Some(name)), Some(target)) = (branch.name(), branch.get().target()) {
                map.entry(target).or_default().push(name.to_string());
            }
        }
    }

    if let Ok(tags) = repo.tag_names(None) {
        for tag in tags.iter().flatten().flatten() {
            if let Some(target) = repo
                .find_reference(&format!("refs/tags/{tag}"))
                .ok()
                .and_then(|reference| reference.target())
            {
                map.entry(target).or_default().push(tag.to_string());
            }
        }
    }

    map
}

fn read_index_blob(repo: &Repository, relative: &str) -> Option<String> {
    let index = repo.index().ok()?;
    let entry = index.get_path(Path::new(relative), 0)?;
    let blob = repo.find_blob(entry.id).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

fn head_tree_of(repo: &Repository) -> Option<git2::Tree<'_>> {
    repo.head().ok()?.peel_to_commit().ok()?.tree().ok()
}

/// Rejects the call when the index already holds staged changes relative to HEAD. Used by
/// porcelain commands (currently just [`revert_commit`]) that auto-commit whatever the index
/// contains once they finish — running one against a dirty index would silently fold unrelated
/// staged work into the resulting commit.
fn ensure_clean_index(repo: &Repository) -> AppResult<()> {
    let head_tree = head_tree_of(repo);
    let diff = repo.diff_tree_to_index(head_tree.as_ref(), None, None).map_err(map_git_err)?;
    if diff.deltas().len() > 0 {
        return Err(AppError::InvalidArgument(
            "인덱스에 스테이지된 변경 사항이 있어 실행할 수 없습니다. 먼저 커밋하거나 스테이지를 해제하세요.".to_string(),
        ));
    }
    Ok(())
}

fn blob_content(repo: &Repository, oid: git2::Oid) -> Option<String> {
    let blob = repo.find_blob(oid).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

fn read_head_blob(repo: &Repository, relative: &str) -> Option<String> {
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    let tree = commit.tree().ok()?;
    let entry = tree.get_path(Path::new(relative)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

fn language_id_for(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_lowercase)
        .and_then(|extension| {
            LANGUAGE_ID_BY_EXTENSION
                .iter()
                .find(|(key, _)| *key == extension)
                .map(|(_, id)| *id)
        })
        .unwrap_or(DEFAULT_LANGUAGE_ID)
        .to_string()
}

fn map_git_err(error: git2::Error) -> AppError {
    AppError::Internal(error.to_string())
}

fn run_git(repo_path: &Path, args: &[&str]) -> AppResult<String> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(args)
        .output()
        .map_err(|error| AppError::Internal(format!("git 실행에 실패했습니다: {error}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let command = args.join(" ");
        return Err(AppError::Internal(format!("git {command} 실패: {stderr}")));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRepo {
        dir: PathBuf,
    }

    impl TestRepo {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("taide-git-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            Repository::init(&dir).unwrap();
            Self { dir }
        }

        fn path(&self) -> &Path {
            &self.dir
        }

        fn write_file(&self, relative: &str, content: &str) {
            let full = self.dir.join(relative);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        }

        fn commit_all(&self, message: &str) -> String {
            let repo = Repository::open(&self.dir).unwrap();
            let mut index = repo.index().unwrap();
            index.add_all(["*"], git2::IndexAddOption::DEFAULT, None).unwrap();
            index.write().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let signature = git2::Signature::now("Test User", "test@example.com").unwrap();

            let parents: Vec<git2::Commit> = match repo.head() {
                Ok(head) => vec![head.peel_to_commit().unwrap()],
                Err(_) => vec![],
            };
            let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

            let oid = repo
                .commit(Some("HEAD"), &signature, &signature, message, &tree, &parent_refs)
                .unwrap();
            oid.to_string()
        }

        /// Commits whatever is already staged in the index as-is, without
        /// re-adding every file the way [`TestRepo::commit_all`] does — lets
        /// a test stage adds/modifies/deletes precisely (e.g. via
        /// [`stage`]) before committing.
        fn commit_staged(&self, message: &str) -> String {
            let repo = Repository::open(&self.dir).unwrap();
            let mut index = repo.index().unwrap();
            let tree_oid = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let signature = git2::Signature::now("Test User", "test@example.com").unwrap();

            let parents: Vec<git2::Commit> = match repo.head() {
                Ok(head) => vec![head.peel_to_commit().unwrap()],
                Err(_) => vec![],
            };
            let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

            let oid = repo
                .commit(Some("HEAD"), &signature, &signature, message, &tree, &parent_refs)
                .unwrap();
            oid.to_string()
        }
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn to_repo_relative는_상대_경로의_상위_탐색을_거부한다() {
        let root = Path::new("/tmp/repo");

        assert!(to_repo_relative(root, "../../etc/passwd").is_err());
        assert!(to_repo_relative(root, "sub/../../escape").is_err());
        assert!(to_repo_relative(root, "..\\..\\windows").is_err());
        assert!(to_repo_relative(root, "/etc/passwd").is_err());
        assert_eq!(to_repo_relative(root, "src/main.rs").unwrap(), "src/main.rs");
        assert_eq!(to_repo_relative(root, "/tmp/repo/src/main.rs").unwrap(), "src/main.rs");
    }

    #[test]
    fn init은_빈_디렉토리에_git_저장소를_생성한다() {
        let dir = std::env::temp_dir().join(format!("taide-git-init-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        init(&dir).expect("init");

        assert!(dir.join(".git").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn init은_이미_초기화된_저장소에도_에러_없이_동작한다() {
        let repo = TestRepo::new();

        let result = init(repo.path());

        assert!(result.is_ok());
        assert!(repo.path().join(".git").exists());
    }

    #[test]
    fn 초기_저장소에_파일을_추가하면_untracked로_표시된다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");

        let result = status(repo.path()).expect("status");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].unstaged, Some(GitChangeKind::Untracked));
        assert_eq!(result.rows[0].staged, None);
    }

    #[test]
    fn stage_후에는_staged가_added로_바뀐다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");

        stage(repo.path(), &["a.txt".to_string()]).expect("stage");

        let result = status(repo.path()).expect("status");
        assert_eq!(result.rows[0].staged, Some(GitChangeKind::Added));
        assert_eq!(result.rows[0].unstaged, None);
    }

    #[test]
    fn 커밋_후에는_상태가_비어있다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");

        let result = status(repo.path()).expect("status");
        assert!(result.rows.is_empty());
    }

    #[test]
    fn 커밋된_파일을_수정하면_unstaged가_modified다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        repo.write_file("a.txt", "hello world");

        let result = status(repo.path()).expect("status");
        assert_eq!(result.rows[0].unstaged, Some(GitChangeKind::Modified));
    }

    #[test]
    fn staged와_unstaged가_동시에_존재할_수_있다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        repo.write_file("a.txt", "staged content");
        stage(repo.path(), &["a.txt".to_string()]).expect("stage");
        repo.write_file("a.txt", "workdir content");

        let result = status(repo.path()).expect("status");
        assert_eq!(result.rows[0].staged, Some(GitChangeKind::Modified));
        assert_eq!(result.rows[0].unstaged, Some(GitChangeKind::Modified));
    }

    #[test]
    fn discard는_워킹트리_변경을_되돌린다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        repo.write_file("a.txt", "modified");

        discard(repo.path(), &["a.txt".to_string()]).expect("discard");

        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn untracked_파일_discard는_휴지통으로_이동해_워킹트리에서_사라진다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");

        discard(repo.path(), &["a.txt".to_string()]).expect("discard");

        assert!(!repo.path().join("a.txt").exists());
    }

    #[test]
    fn tracked와_untracked를_함께_discard해도_각각_처리된다() {
        let repo = TestRepo::new();
        repo.write_file("tracked.txt", "hello");
        repo.commit_all("init");
        repo.write_file("tracked.txt", "modified");
        repo.write_file("untracked.txt", "new");

        discard(repo.path(), &["tracked.txt".to_string(), "untracked.txt".to_string()]).expect("discard");

        assert_eq!(std::fs::read_to_string(repo.path().join("tracked.txt")).unwrap(), "hello");
        assert!(!repo.path().join("untracked.txt").exists());
    }

    #[test]
    fn gutter는_수정된_라인의_hunk를_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED\nline3\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");

        assert!(!hunks.is_empty());
        assert!(hunks.iter().any(|hunk| hunk.kind == HunkKind::Modified));
    }

    #[test]
    fn log는_커밋의_부모_관계를_채운다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "1");
        let first = repo.commit_all("first");
        repo.write_file("a.txt", "2");
        let second = repo.commit_all("second");

        let entries = log(repo.path(), 0, 10).expect("log");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, second);
        assert_eq!(entries[0].parents, vec![first.clone()]);
        assert!(entries[1].parents.is_empty());
        assert_eq!(entries[1].id, first);
    }

    #[test]
    fn branches는_현재_브랜치를_is_head로_표시한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");

        let result = branches(repo.path()).expect("branches");

        assert_eq!(result.len(), 1);
        assert!(result[0].is_head);
        assert!(!result[0].is_remote);
    }

    #[test]
    fn branch_create는_새_브랜치를_만들되_체크아웃하지_않는다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");

        branch_create(repo.path(), "feature", false).expect("branch_create");

        let result = branches(repo.path()).expect("branches");
        let feature = result.iter().find(|branch| branch.name == "feature").expect("feature branch");
        assert!(!feature.is_head);
        let current = current_branch(&open_repo(repo.path()).unwrap()).unwrap();
        assert_ne!(current, "feature");
    }

    #[test]
    fn branch_create_checkout_true면_생성_후_체크아웃한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");

        branch_create(repo.path(), "feature", true).expect("branch_create");

        let current = current_branch(&open_repo(repo.path()).unwrap()).unwrap();
        assert_eq!(current, "feature");
    }

    #[test]
    fn branch_checkout은_지정한_브랜치로_head를_옮긴다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        branch_create(repo.path(), "feature", false).expect("branch_create");

        branch_checkout(repo.path(), "feature").expect("branch_checkout");

        let current = current_branch(&open_repo(repo.path()).unwrap()).unwrap();
        assert_eq!(current, "feature");
    }

    #[test]
    fn branch_checkout은_충돌하는_미커밋_변경이_있으면_실패한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        branch_create(repo.path(), "feature", true).expect("branch_create");
        repo.write_file("a.txt", "feature change");
        repo.commit_all("feature commit");
        branch_checkout(repo.path(), "master")
            .or_else(|_| branch_checkout(repo.path(), "main"))
            .expect("checkout back");
        repo.write_file("a.txt", "conflicting uncommitted change");

        let result = branch_checkout(repo.path(), "feature");

        assert!(result.is_err());
    }

    #[test]
    fn branch_delete는_현재_브랜치를_삭제할_수_없다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        let current = current_branch(&open_repo(repo.path()).unwrap()).unwrap();

        let result = branch_delete(repo.path(), &current, false);

        assert!(result.is_err());
    }

    #[test]
    fn branch_delete는_병합되지_않은_브랜치는_force_없이_삭제할_수_없다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        branch_create(repo.path(), "feature", true).expect("branch_create");
        repo.write_file("a.txt", "feature change");
        repo.commit_all("feature commit");
        branch_checkout(repo.path(), "master")
            .or_else(|_| branch_checkout(repo.path(), "main"))
            .expect("checkout back to base");

        let result = branch_delete(repo.path(), "feature", false);

        assert!(result.is_err());
    }

    #[test]
    fn branch_delete는_force면_병합되지_않아도_삭제한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        branch_create(repo.path(), "feature", true).expect("branch_create");
        repo.write_file("a.txt", "feature change");
        repo.commit_all("feature commit");
        branch_checkout(repo.path(), "master")
            .or_else(|_| branch_checkout(repo.path(), "main"))
            .expect("checkout back to base");

        branch_delete(repo.path(), "feature", true).expect("branch_delete force");

        let result = branches(repo.path()).expect("branches");
        assert!(!result.iter().any(|branch| branch.name == "feature"));
    }

    #[test]
    fn stash_push_후_워킹트리가_head_상태로_복원된다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        repo.write_file("a.txt", "dirty change");

        stash_push(repo.path(), Some("wip work")).expect("stash_push");

        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "hello");
        let list = stash_list(repo.path()).expect("stash_list");
        assert_eq!(list.len(), 1);
        assert!(list[0].message.contains("wip work"));
    }

    #[test]
    fn stash_apply는_스태시된_변경을_워킹트리에_되돌린다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        repo.write_file("a.txt", "dirty change");
        stash_push(repo.path(), None).expect("stash_push");

        stash_apply(repo.path(), 0).expect("stash_apply");

        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "dirty change");
    }

    #[test]
    fn stash_drop은_목록에서_스태시를_제거한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");
        repo.commit_all("init");
        repo.write_file("a.txt", "dirty change");
        stash_push(repo.path(), None).expect("stash_push");

        stash_drop(repo.path(), 0).expect("stash_drop");

        let list = stash_list(repo.path()).expect("stash_list");
        assert!(list.is_empty());
    }

    #[test]
    fn discard_hunk는_지정한_hunk만_되돌리고_나머지는_유지한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\nline4\nline5\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED2\nline3\nCHANGED4\nline5\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks.len(), 2);
        let (first_start, first_end) = (hunks[0].start, hunks[0].end);

        discard_hunk(repo.path(), "a.txt", first_start, first_end).expect("discard_hunk");

        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "line1\nline2\nline3\nCHANGED4\nline5\n");
    }

    #[test]
    fn discard_hunk는_추가된_라인_hunk를_제거한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nNEW\nline2\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].kind, HunkKind::Added);

        discard_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("discard_hunk");

        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "line1\nline2\n");
    }

    #[test]
    fn discard_hunk는_삭제된_라인_hunk를_복원한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nline3\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].kind, HunkKind::Deleted);

        discard_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("discard_hunk");

        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "line1\nline2\nline3\n");
    }

    #[test]
    fn discard_hunk는_존재하지_않는_hunk면_에러를_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED\n");

        let result = discard_hunk(repo.path(), "a.txt", 999, 999);

        assert!(result.is_err());
    }

    fn merge_feature_into_base(repo: &TestRepo) -> String {
        repo.write_file("a.txt", "base\n");
        repo.commit_all("base");
        let base_branch = current_branch(&open_repo(repo.path()).unwrap()).unwrap();

        branch_create(repo.path(), "feature", true).expect("branch_create");
        repo.write_file("a.txt", "feature\n");
        repo.commit_all("feature change");

        branch_checkout(repo.path(), &base_branch).expect("checkout back");
        repo.write_file("a.txt", "main\n");
        repo.commit_all("main change");

        let git_repo = Repository::open(repo.path()).unwrap();
        let feature_branch = git_repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let feature_commit = feature_branch.get().peel_to_commit().unwrap();
        let annotated = git_repo.find_annotated_commit(feature_commit.id()).unwrap();
        git_repo.merge(&[&annotated], None, None).expect("merge");
        assert!(git_repo.index().unwrap().has_conflicts());

        base_branch
    }

    #[test]
    fn conflict_sides는_충돌의_세_버전과_workdir_내용을_반환한다() {
        let repo = TestRepo::new();
        merge_feature_into_base(&repo);

        let sides = conflict_sides(repo.path(), "a.txt").expect("conflict_sides");

        assert_eq!(sides.base.as_deref(), Some("base\n"));
        assert_eq!(sides.ours.as_deref(), Some("main\n"));
        assert_eq!(sides.theirs.as_deref(), Some("feature\n"));
        assert!(sides.workdir.contains("main") && sides.workdir.contains("feature"));
    }

    #[test]
    fn resolve_conflict는_workdir을_쓰고_index_충돌을_해소한다() {
        let repo = TestRepo::new();
        merge_feature_into_base(&repo);

        resolve_conflict(repo.path(), "a.txt", "resolved\n").expect("resolve_conflict");

        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "resolved\n");

        let result = status(repo.path()).expect("status");
        let row = result.rows.iter().find(|row| row.path == "a.txt").expect("row");
        assert!(!row.is_conflicted);
        assert_eq!(row.staged, Some(GitChangeKind::Modified));
        assert_eq!(row.unstaged, None);
    }

    #[test]
    fn stage_hunk는_선택한_hunk만_인덱스에_반영한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\nline4\nline5\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED2\nline3\nCHANGED4\nline5\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks.len(), 2);
        let (first_start, first_end) = (hunks[0].start, hunks[0].end);

        stage_hunk(repo.path(), "a.txt", first_start, first_end).expect("stage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nCHANGED2\nline3\nline4\nline5\n");

        let result = status(repo.path()).expect("status");
        let row = result.rows.iter().find(|row| row.path == "a.txt").expect("row");
        assert_eq!(row.staged, Some(GitChangeKind::Modified));
        assert_eq!(row.unstaged, Some(GitChangeKind::Modified));
    }

    #[test]
    fn unstage_hunk는_스테이지된_hunk_하나만_되돌린다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\nline4\nline5\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED2\nline3\nCHANGED4\nline5\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks.len(), 2);
        let (first_start, first_end) = (hunks[0].start, hunks[0].end);
        let (second_start, second_end) = (hunks[1].start, hunks[1].end);

        stage_hunk(repo.path(), "a.txt", first_start, first_end).expect("stage first");
        stage_hunk(repo.path(), "a.txt", second_start, second_end).expect("stage second");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nCHANGED2\nline3\nCHANGED4\nline5\n");

        unstage_hunk(repo.path(), "a.txt", second_start, second_end).expect("unstage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nCHANGED2\nline3\nline4\nline5\n");
    }

    #[test]
    fn stage_hunk는_존재하지_않는_hunk면_에러를_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED\n");

        let result = stage_hunk(repo.path(), "a.txt", 999, 999);

        assert!(result.is_err());
    }

    #[test]
    fn stage_hunk는_순수_삭제_hunk도_인덱스에_반영한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nline3\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks[0].kind, HunkKind::Deleted);

        stage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("stage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nline3\n");
    }

    #[test]
    fn unstage_hunk는_순수_삭제_hunk도_되돌린다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nline3\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        stage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("stage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nline3\n");

        unstage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("unstage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nline2\nline3\n");
    }

    #[test]
    fn stage_hunk는_순수_추가_hunk도_인덱스에_반영한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nNEW\nline2\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks[0].kind, HunkKind::Added);

        stage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("stage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nNEW\nline2\n");

        unstage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("unstage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nline2\n");
    }

    #[test]
    fn stage_hunk는_미추적_파일의_hunk도_인덱스에_반영한다() {
        let repo = TestRepo::new();
        repo.write_file("tracked.txt", "line1\n");
        repo.commit_all("init");
        repo.write_file("new.txt", "hello\nworld\n");

        let hunks = gutter(repo.path(), "new.txt").expect("gutter");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].kind, HunkKind::Added);

        stage_hunk(repo.path(), "new.txt", hunks[0].start, hunks[0].end).expect("stage_hunk");

        let staged = diff_file(repo.path(), "new.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "hello\nworld\n");

        let result = status(repo.path()).expect("status");
        let row = result.rows.iter().find(|row| row.path == "new.txt").expect("row");
        assert_eq!(row.staged, Some(GitChangeKind::Added));

        unstage_hunk(repo.path(), "new.txt", hunks[0].start, hunks[0].end).expect("unstage_hunk");

        let result = status(repo.path()).expect("status");
        let row = result.rows.iter().find(|row| row.path == "new.txt").expect("row");
        assert_eq!(row.staged, None);
        assert_eq!(row.unstaged, Some(GitChangeKind::Untracked));
    }

    #[test]
    fn stage_lines는_미추적_파일의_선택한_라인만_인덱스에_반영한다() {
        let repo = TestRepo::new();
        repo.write_file("tracked.txt", "line1\n");
        repo.commit_all("init");
        repo.write_file("new.txt", "hello\nworld\n");

        stage_lines(repo.path(), "new.txt", 1, 1).expect("stage_lines");

        let staged = diff_file(repo.path(), "new.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "hello\n");

        let workdir = std::fs::read_to_string(repo.path().join("new.txt")).unwrap();
        assert_eq!(workdir, "hello\nworld\n");
    }

    #[test]
    fn stage_lines는_선택한_추가_라인만_인덱스에_반영한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nNEWA\nNEWB\nline2\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].kind, HunkKind::Added);
        assert_eq!(hunks[0].start, 2);
        assert_eq!(hunks[0].end, 3);

        stage_lines(repo.path(), "a.txt", 2, 2).expect("stage_lines");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nNEWA\nline2\n");

        let workdir = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(workdir, "line1\nNEWA\nNEWB\nline2\n");
    }

    #[test]
    fn unstage_lines는_스테이지된_라인_일부만_되돌린다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nNEWA\nNEWB\nline2\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        stage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("stage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nNEWA\nNEWB\nline2\n");

        unstage_lines(repo.path(), "a.txt", 3, 3).expect("unstage_lines");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nNEWA\nline2\n");
    }

    #[test]
    fn stage_lines는_hunk_범위를_벗어나면_에러를_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nNEWA\nNEWB\nline2\n");

        let result = stage_lines(repo.path(), "a.txt", 100, 100);

        assert!(result.is_err());
    }

    #[test]
    fn stage_lines는_여러_hunk에_걸친_범위면_에러를_반환하고_아무것도_반영하지_않는다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\nline3\nline4\nline5\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED2\nline3\nCHANGED4\nline5\n");

        let hunks = gutter(repo.path(), "a.txt").expect("gutter");
        assert_eq!(hunks.len(), 2);

        let result = stage_lines(repo.path(), "a.txt", hunks[0].start, hunks[1].end);

        assert!(result.is_err());

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead).expect("diff_file");
        assert_eq!(staged.modified, "line1\nline2\nline3\nline4\nline5\n");
    }

    #[test]
    fn commit_files는_초기_커밋의_모든_파일을_added로_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.write_file("b.txt", "b");
        let first = repo.commit_all("init");

        let files = commit_files(repo.path(), &first).expect("commit_files");

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|file| file.kind == GitChangeKind::Added));
        assert!(files.iter().any(|file| file.path == "a.txt"));
        assert!(files.iter().any(|file| file.path == "b.txt"));
    }

    #[test]
    fn commit_files는_수정_삭제_추가를_구분한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.write_file("b.txt", "b");
        repo.commit_all("init");

        repo.write_file("a.txt", "a2");
        std::fs::remove_file(repo.path().join("b.txt")).unwrap();
        repo.write_file("c.txt", "c");
        stage(repo.path(), &["a.txt".to_string(), "b.txt".to_string(), "c.txt".to_string()]).expect("stage");
        let second = repo.commit_staged("second");

        let files = commit_files(repo.path(), &second).expect("commit_files");

        let kind_of = |path: &str| files.iter().find(|file| file.path == path).map(|file| file.kind);
        assert_eq!(kind_of("a.txt"), Some(GitChangeKind::Modified));
        assert_eq!(kind_of("b.txt"), Some(GitChangeKind::Deleted));
        assert_eq!(kind_of("c.txt"), Some(GitChangeKind::Added));
    }

    #[test]
    fn file_log는_해당_파일을_건드린_커밋만_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "1");
        repo.write_file("x.txt", "x");
        let first = repo.commit_all("first touches a and x");

        repo.write_file("x.txt", "x2");
        repo.commit_all("second touches only x");

        repo.write_file("a.txt", "2");
        let third = repo.commit_all("third touches only a");

        let entries = file_log(repo.path(), "a.txt", 0, 10).expect("file_log");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, third);
        assert_eq!(entries[1].id, first);
    }

    #[test]
    fn file_log는_skip과_take로_페이지네이션한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "1");
        let first = repo.commit_all("first");
        repo.write_file("a.txt", "2");
        repo.commit_all("second");

        let entries = file_log(repo.path(), "a.txt", 1, 10).expect("file_log");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, first);
    }

    #[test]
    fn revert_commit는_충돌없이_되돌리면_새_커밋을_만든다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\n");
        repo.commit_all("first");
        repo.write_file("a.txt", "line1\nline2\n");
        let second = repo.commit_all("second");

        let outcome = revert_commit(repo.path(), &second).expect("revert_commit");

        assert!(!outcome.conflicted);
        let content = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert_eq!(content, "line1\n");

        let entries = log(repo.path(), 0, 10).expect("log");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].parents, vec![second]);
    }

    #[test]
    fn revert_commit는_충돌이_생기면_conflicted를_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "A\n");
        repo.commit_all("first");
        repo.write_file("a.txt", "B\n");
        let second = repo.commit_all("second");
        repo.write_file("a.txt", "C\n");
        repo.commit_all("third");

        let outcome = revert_commit(repo.path(), &second).expect("revert_commit");

        assert!(outcome.conflicted);
        assert_eq!(outcome.conflicted_paths, vec!["a.txt".to_string()]);
        let result = status(repo.path()).expect("status");
        assert!(result.rows.iter().any(|row| row.is_conflicted));
    }

    #[test]
    fn revert_commit는_인덱스에_무관한_staged_변경이_있으면_거부한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\n");
        repo.commit_all("first");
        repo.write_file("a.txt", "line1\nline2\n");
        let second = repo.commit_all("second");
        repo.write_file("b.txt", "unrelated staged work\n");
        stage_hunk(repo.path(), "b.txt", 1, 1).expect("stage_hunk b.txt");

        let result = revert_commit(repo.path(), &second);

        assert!(result.is_err());
        let staged = diff_file(repo.path(), "b.txt", DiffMode::IndexVsHead).expect("diff_file b.txt");
        assert_eq!(staged.modified, "unrelated staged work\n");
        let entries = log(repo.path(), 0, 10).expect("log");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn tag_create는_annotated와_lightweight_태그를_만든다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        let first = repo.commit_all("init");

        tag_create(
            repo.path(),
            "v1",
            &first,
            &TagCreateOptions {
                message: Some("release".to_string()),
                annotated: true,
            },
        )
        .expect("tag_create annotated");
        tag_create(
            repo.path(),
            "v1-lw",
            "HEAD",
            &TagCreateOptions {
                message: None,
                annotated: true,
            },
        )
        .expect("tag_create lightweight fallback");

        let all_tags = tags(repo.path()).expect("tags");

        let annotated_tag = all_tags.iter().find(|tag| tag.name == "v1").expect("v1");
        assert!(annotated_tag.annotated);
        assert_eq!(annotated_tag.message.as_deref(), Some("release"));
        assert_eq!(annotated_tag.target, first);

        let lightweight_tag = all_tags.iter().find(|tag| tag.name == "v1-lw").expect("v1-lw");
        assert!(!lightweight_tag.annotated);
        assert_eq!(lightweight_tag.message, None);
        assert_eq!(lightweight_tag.target, first);
    }

    #[test]
    fn tag_delete는_목록에서_태그를_제거한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.commit_all("init");
        tag_create(
            repo.path(),
            "v1",
            "HEAD",
            &TagCreateOptions {
                message: None,
                annotated: false,
            },
        )
        .expect("tag_create");

        tag_delete(repo.path(), "v1").expect("tag_delete");

        let all_tags = tags(repo.path()).expect("tags");
        assert!(!all_tags.iter().any(|tag| tag.name == "v1"));
    }

    #[test]
    fn checkout_remote_branch는_로컬_추적_브랜치를_만들고_체크아웃한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.commit_all("init");

        let git_repo = Repository::open(repo.path()).unwrap();
        git_repo.remote("origin", "https://example.invalid/repo.git").expect("remote");
        let head_oid = git_repo.head().unwrap().target().unwrap();
        git_repo
            .reference("refs/remotes/origin/feature", head_oid, true, "test setup")
            .expect("create remote ref");
        drop(git_repo);

        checkout_remote_branch(repo.path(), "origin/feature").expect("checkout_remote_branch");

        let current = current_branch(&open_repo(repo.path()).unwrap()).unwrap();
        assert_eq!(current, "feature");

        let all_branches = branches(repo.path()).expect("branches");
        let local = all_branches
            .iter()
            .find(|branch| branch.name == "feature" && !branch.is_remote)
            .expect("local feature branch");
        assert_eq!(local.upstream.as_deref(), Some("origin/feature"));
    }

    #[test]
    fn checkout_remote_branch는_동명_로컬_브랜치가_있으면_그대로_체크아웃한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.commit_all("init");

        branch_create(repo.path(), "feature", false).expect("branch_create");

        let git_repo = Repository::open(repo.path()).unwrap();
        git_repo.remote("origin", "https://example.invalid/repo.git").expect("remote");
        let head_oid = git_repo.head().unwrap().target().unwrap();
        git_repo
            .reference("refs/remotes/origin/feature", head_oid, true, "test setup")
            .expect("create remote ref");
        drop(git_repo);

        checkout_remote_branch(repo.path(), "origin/feature").expect("checkout_remote_branch");

        let current = current_branch(&open_repo(repo.path()).unwrap()).unwrap();
        assert_eq!(current, "feature");
    }
}
