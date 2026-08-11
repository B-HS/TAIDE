use std::collections::HashMap;
use std::path::{Path, PathBuf};

use git2::build::CheckoutBuilder;
use git2::Repository;
use serde::Serialize;
use specta::Type;

use crate::error::{AppError, AppResult};

use super::types::{
    BlameLine, CommitOptions, DiffMode, DiffSides, GitBranch, GitChangeKind, GitRemote, GitStashEntry, GitStatus, GutterHunk, HunkKind,
    LogEntry, StatusRow,
};

const DEFAULT_STASH_MESSAGE: &str = "WIP";

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

    let head_tree = match repo.head() {
        Ok(head) => head.peel_to_commit().ok().and_then(|commit| commit.tree().ok()),
        Err(_) => None,
    };

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

    let head_tree = match repo.head() {
        Ok(head) => head.peel_to_commit().ok().and_then(|commit| commit.tree().ok()),
        Err(_) => None,
    };

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
    let candidate = Path::new(raw);
    if candidate.is_relative() {
        return Ok(raw.replace('\\', "/"));
    }

    let relative = candidate
        .strip_prefix(repo_root)
        .map_err(|_| AppError::InvalidArgument(format!("경로가 저장소 밖에 있습니다: {raw}")))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
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
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
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
}
