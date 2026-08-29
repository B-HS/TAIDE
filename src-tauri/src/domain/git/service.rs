use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::path::{Component, Path, PathBuf};

use git2::build::CheckoutBuilder;
use git2::Repository;
use serde::Serialize;
use specta::Type;

use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::language::{self, LanguageOverlay};

use super::types::{
    BlameLine, CommitFile, CommitOptions, ConflictSides, DiffMode, DiffSides, GitBranch, GitChangeKind, GitRemote, GitStashEntry,
    GitStatus, GutterHunk, HunkKind, LogEntry, RevertOutcome, StagedDiffText, StatusRow, TagCreateOptions, TagInfo,
};

const DEFAULT_STASH_MESSAGE: &str = "WIP";

/// Regular, non-executable file mode — see [`build_patch_text`]'s doc comment for why a synthetic
/// add/delete patch needs one at all.
const NEW_FILE_MODE: &str = "100644";

/// Upper bound on [`diff_staged_text`]'s returned patch text — keeps the AI commit-message request
/// payload (and its token cost) bounded regardless of how large the staged change is. Exceeding it
/// sets `truncated: true` on [`StagedDiffText`] rather than erroring, since a summary from a
/// partial diff is still useful.
const STAGED_DIFF_TEXT_MAX_BYTES: usize = 32 * 1024;

/// Lock files carry no information useful to an AI commit-message summary (their diffs are large,
/// mechanically generated, and never hand-written) — excluded from [`diff_staged_text`] the same
/// way binary deltas are, and listed in `skippedFiles` instead of their content.
const STAGED_DIFF_LOCK_FILE_NAMES: &[&str] = &["bun.lock", "Cargo.lock", "package-lock.json", "pnpm-lock.yaml", "yarn.lock"];

/// Exact basenames that conventionally hold secrets (SSH private keys) — a staged one must never
/// reach an external AI provider's request body, so it's excluded from [`diff_staged_text`] the
/// same way lock files are (security.md §1 "시크릿은 … 클라이언트 어디에도 노출하지 않는다").
const STAGED_DIFF_SECRET_FILE_NAMES: &[&str] = &["id_rsa", "id_dsa", "id_ecdsa", "id_ed25519", ".netrc", ".npmrc"];

/// Basename prefixes that conventionally hold secrets (`.env`, `.env.local`, `.env.production`, ...).
const STAGED_DIFF_SECRET_FILE_NAME_PREFIXES: &[&str] = &[".env"];

/// Basename extensions that conventionally hold secrets (private keys, certificates, keystores).
const STAGED_DIFF_SECRET_FILE_EXTENSIONS: &[&str] = &["pem", "key", "p12", "pfx", "jks", "keystore", "ppk", "der", "crt"];

/// Upper bound on the blob [`show_file`] will hand back. Deliberately the file domain's own
/// refuse-to-open tier, so "what the editor refuses to read from disk" and "what git will hand the
/// editor out of history" stay the same boundary — a commit-diff or file-history view must not become a
/// second door into a file `file_open` already refuses.
const SHOW_FILE_MAX_BYTES: u64 = crate::constants::REFUSED_FILE_BYTES;

/// How long a single `git` subprocess may run in [`run_git`] before it is killed. Generous on purpose:
/// the commands that reach here include `push`/`pull`/`fetch` against a slow remote and `commit` with
/// `add -A` over a large working tree, all of which can legitimately take minutes, and killing a `pull`
/// mid-integration is itself disruptive. The bound exists for the unbounded case the audit found (§2
/// M-7): a subprocess parked forever on an unreachable remote or a blocked credential prompt, which
/// previously pinned a blocking-pool thread — and, for `pull`, `AppState::begin_mutation`; for
/// `push`/`fetch`, [`super::commands::GitStore::push_fetch_lock`] — for the rest of the app's lifetime.
const GIT_COMMAND_TIMEOUT_SECS: u64 = 300;

/// How often [`run_command_with_timeout`] re-checks a still-running subprocess. Small enough that the
/// kill path is prompt and large enough that a normal short `git` call costs at most one extra sleep.
const GIT_COMMAND_POLL_INTERVAL_MS: u64 = 20;

/// How long [`run_command_with_timeout`] waits for the pipe readers to report end-of-file *after* the
/// subprocess has already exited. Short because by then only whatever is still sitting in the pipe
/// buffer remains — the reader threads have been draining concurrently for the command's whole run.
///
/// It exists because the exit-status deadline above does not cover this step: `git` can leave a
/// grandchild holding the write end of its stdout/stderr (an `ssh` ControlMaster/ControlPersist, a
/// background credential helper), and that keeps `read_to_end` from ever seeing EOF. Waiting for the
/// readers unconditionally there — the shape this function shipped with — turned a *successful*
/// `git push`/`pull` into the very indefinite hang the timeout exists to prevent, still holding
/// `AppState::begin_mutation` (for `pull`) or [`super::commands::GitStore::push_fetch_lock`]
/// (`push`/`fetch`). Timing out here just costs the tail of the output, which is exactly the trade the
/// kill path already makes by abandoning its readers.
const GIT_PIPE_DRAIN_TIMEOUT_SECS: u64 = 5;

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
    let repo = Repository::discover(root).map_err(|error| {
        AppError::localized(
            AppErrorKind::NotFound,
            "error.git.repositoryNotFound",
            format!("could not find a .git repository: {error}"),
        )
        .with_arg("detail", &error)
    })?;
    let workdir = repo.workdir().ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.bareUnsupported",
            "bare repositories are not supported",
        )
    })?;
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

/// Stages each path, taking the directory branch for an untracked-directory row. [`collect_status_rows`]
/// sets `recurse_untracked_dirs(false)`, so an untracked directory arrives as one `nested/` row rather
/// than one row per contained file — and `index.add_path` rejects a directory outright (libgit2 1.9.6's
/// `git_index_add_bypath` → "it is a directory"; the trailing-slash spelling fails even earlier, at path
/// validation). That made every such row permanently unstageable, and poisoned any multi-select it was
/// part of, since one failure aborts the whole loop before `index.write` (§4-A-6). `add_all` is the same
/// call `git add nested/` makes: it walks the directory and adds each contained file, skipping ignored
/// ones exactly as `IndexAddOption::DEFAULT` does elsewhere — so the row keeps meaning what the UI shows
/// it to mean ("stage everything under here"). Files and deletions take the unchanged
/// `add_path`/`remove_path` branches.
pub fn stage(repo_path: &Path, paths: &[String]) -> AppResult<()> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let mut index = repo.index().map_err(map_git_err)?;

    for raw in paths {
        let relative = to_repo_relative(&workdir, raw)?;
        let relative_path = Path::new(&relative);
        let absolute = workdir.join(relative_path);
        if absolute.is_dir() {
            index
                .add_all([without_trailing_separator(&relative)], git2::IndexAddOption::DEFAULT, None)
                .map_err(map_git_err)?;
        } else if absolute.exists() {
            index.add_path(relative_path).map_err(map_git_err)?;
        } else {
            index.remove_path(relative_path).map_err(map_git_err)?;
        }
    }

    index.write().map_err(map_git_err)
}

/// Drops the trailing separator an untracked-directory status row carries (`nested/` → `nested`),
/// which both libgit2's pathspec matcher and repo-relative path comparison want in the plain form.
/// `Components::as_path` trims it without hand-rolling a separator literal, and — because `Path`
/// equality compares components rather than raw bytes — comparing two of these also absorbs the
/// asymmetry [`to_repo_relative`] leaves behind: a relative input keeps the caller's trailing slash
/// while an absolute one loses it to `strip_prefix`'s own component normalization (§4-A-10).
fn without_trailing_separator(relative: &str) -> &Path {
    Path::new(relative).components().as_path()
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
    context.delete_all(&absolute_paths).map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.file.trashFailed",
            format!("failed to move to trash: {error}"),
        )
        .with_arg("detail", &error)
    })
}

#[cfg(not(target_os = "macos"))]
fn trash_untracked_files(workdir: &Path, relatives: &[String]) -> AppResult<()> {
    let absolute_paths: Vec<PathBuf> = relatives.iter().map(|relative| workdir.join(relative)).collect();
    trash::delete_all(&absolute_paths).map_err(|error| {
        AppError::localized(
            AppErrorKind::Internal,
            "error.file.trashFailed",
            format!("failed to move to trash: {error}"),
        )
        .with_arg("detail", &error)
    })
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
        rows.iter().any(|row| {
            without_trailing_separator(&row.path) == without_trailing_separator(relative) && row.unstaged == Some(GitChangeKind::Untracked)
        })
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

/// Rejects an over-limit blob from its object header instead of materializing it. `Odb::read_header`
/// reads only the header, so an over-limit blob is never decompressed into memory — where the previous
/// unbounded `find_blob` + `String::from_utf8_lossy` pair allocated the whole thing twice (once
/// decompressed, once as the lossy `String`) before anything could refuse it, for a value that then had
/// to be JSON-serialized across IPC as well (§2 M-1).
pub fn show_file(repo_path: &Path, rev: &str, path: &str) -> AppResult<String> {
    show_file_limited(repo_path, rev, path, SHOW_FILE_MAX_BYTES)
}

/// [`show_file`] with the size bound as a parameter, so tests can exercise the refusal against a
/// few-byte blob instead of materializing a [`SHOW_FILE_MAX_BYTES`]-sized one just to watch it be
/// rejected.
fn show_file_limited(repo_path: &Path, rev: &str, path: &str, max_bytes: u64) -> AppResult<String> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;

    let object = repo.revparse_single(rev).map_err(map_git_err)?;
    let commit = object.peel_to_commit().map_err(map_git_err)?;
    let tree = commit.tree().map_err(map_git_err)?;
    let entry = tree
        .get_path(Path::new(&relative))
        .map_err(|error| AppError::NotFound(format!("{relative}: {error}")))?;

    let odb = repo.odb().map_err(map_git_err)?;
    let (size, _) = odb.read_header(entry.id()).map_err(map_git_err)?;
    if size as u64 > max_bytes {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.blobTooLarge",
            format!("{relative}: blob is too large to show ({size} bytes)"),
        )
        .with_arg("path", &relative));
    }

    let blob = repo.find_blob(entry.id()).map_err(map_git_err)?;

    Ok(String::from_utf8_lossy(blob.content()).into_owned())
}

/// `before_path` names the row's pre-change path and drives the **original** (left) side only; the
/// modified (right) side is always `path`. A rename has different paths on the two sides — a staged
/// rename's HEAD entry lives at the old path, an unstaged one's index entry does — and reading both
/// sides at the new path made HEAD/index come back empty, rendering every renamed file as a whole-file
/// addition instead of its actual edit (§4-B B11). `None` keeps the pre-existing behaviour of using
/// `path` for both sides, which is correct for every non-rename row.
pub fn diff_file(
    repo_path: &Path,
    path: &str,
    mode: DiffMode,
    before_path: Option<&str>,
    language_overlays: &[LanguageOverlay],
) -> AppResult<DiffSides> {
    let repo = open_repo(repo_path)?;
    let workdir = repo_workdir(&repo)?;
    let relative = to_repo_relative(&workdir, path)?;
    let before_relative = match before_path {
        Some(raw) => to_repo_relative(&workdir, raw)?,
        None => relative.clone(),
    };

    let (original, modified) = match mode {
        DiffMode::WorkdirVsIndex => {
            let original = read_index_blob(&repo, &before_relative).unwrap_or_default();
            let modified = std::fs::read_to_string(workdir.join(&relative)).unwrap_or_default();
            (original, modified)
        }
        DiffMode::IndexVsHead => {
            let original = read_head_blob(&repo, &before_relative).unwrap_or_default();
            let modified = read_index_blob(&repo, &relative).unwrap_or_default();
            (original, modified)
        }
    };

    Ok(DiffSides {
        original,
        modified,
        language_id: language::language_id_for_path(Path::new(&relative), language_overlays),
    })
}

/// `true` when `delta` should be left out of [`diff_staged_text`]'s patch body — a lock file, a
/// secret-like file (by basename, on whichever side has a path — see [`is_secret_like_path`]), or
/// a binary delta on either side. Called from inside the [`git2::Diff::print`] callback, which both
/// skips emitting these deltas' lines and collects their paths into `skippedFiles` from the same
/// check — see [`diff_staged_text`]'s doc comment for why that has to happen during `print` rather
/// than a pass over `diff.deltas()` beforehand.
fn is_excluded_from_staged_diff(delta: &git2::DiffDelta) -> bool {
    let is_excluded_by_name = staged_diff_delta_path(delta).is_some_and(|path| is_lock_file_path(&path) || is_secret_like_path(&path));
    is_excluded_by_name || delta.new_file().is_binary() || delta.old_file().is_binary()
}

fn is_lock_file_path(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| STAGED_DIFF_LOCK_FILE_NAMES.contains(&name))
}

/// `true` when `path`'s basename matches one of [`STAGED_DIFF_SECRET_FILE_NAMES`] exactly, starts
/// with one of [`STAGED_DIFF_SECRET_FILE_NAME_PREFIXES`], or ends with one of
/// [`STAGED_DIFF_SECRET_FILE_EXTENSIONS`] — a conservative, extension/name-based heuristic (not a
/// content scan) for files that conventionally hold credentials.
fn is_secret_like_path(path: &str) -> bool {
    let Some(name) = Path::new(path).file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    STAGED_DIFF_SECRET_FILE_NAMES.contains(&name)
        || STAGED_DIFF_SECRET_FILE_NAME_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
        || STAGED_DIFF_SECRET_FILE_EXTENSIONS
            .iter()
            .any(|extension| Path::new(name).extension().and_then(|value| value.to_str()) == Some(extension))
}

/// The path a [`StagedDiffText`]-facing message should show for `delta` — `new_file`'s path, or
/// `old_file`'s when the delta has no new side (a deletion).
fn staged_diff_delta_path(delta: &git2::DiffDelta) -> Option<String> {
    delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .map(|path| path.to_string_lossy().into_owned())
}

/// Finds the largest `cut <= bytes.len().min(max_bytes)` that doesn't land inside a multi-byte
/// UTF-8 sequence — used to hard-truncate [`diff_staged_text`]'s patch buffer at a `char` boundary
/// so `String::from_utf8_lossy` never has to paper over a chopped-off character with `U+FFFD`.
/// `max_bytes >= bytes.len()` (nothing to cut) returns `bytes.len()` immediately rather than
/// indexing `bytes[cut]` at that length — one past the last valid index, which would panic.
fn utf8_safe_truncate_len(bytes: &[u8], max_bytes: usize) -> usize {
    let mut cut = max_bytes.min(bytes.len());
    if cut >= bytes.len() {
        return bytes.len();
    }
    while cut > 0 && bytes[cut] & 0b1100_0000 == 0b1000_0000 {
        cut -= 1;
    }
    cut
}

/// Builds the unified diff text of staged changes (HEAD vs index) for AI commit-message
/// generation, via `git2`'s native `diff_tree_to_index` + `Diff::print` (see
/// `docs/acknowledge/2026-08-16-wave-g-ai-contract.md` §3.3) rather than shelling out to the `git`
/// CLI. `head_tree_of` returning `None` (no commits yet) diffs against libgit2's implicit empty
/// tree, so an initial commit's staged files show up as additions — the same behavior
/// [`ensure_clean_index`] already relies on. Binary deltas and lock files
/// ([`STAGED_DIFF_LOCK_FILE_NAMES`]) are left out of the patch body and listed in `skippedFiles`
/// instead; the patch body itself is capped at [`STAGED_DIFF_TEXT_MAX_BYTES`].
///
/// When the index has zero deltas against HEAD, this falls back to a HEAD↔workdir diff
/// (`diff_tree_to_workdir_with_index`, VS Code parity — see
/// `docs/acknowledge/2026-08-16-wave-h-keymap-contract.md` §3.4) so the AI commit-message flow
/// still has something to summarize when the user hasn't staged anything yet. The fallback diff
/// includes untracked files (`include_untracked`/`show_untracked_content`) since "workdir changes"
/// should mean the same thing here as it does for [`gutter`]'s full working-tree diff, not just
/// modifications to already-tracked paths. Which diff got used is reported back via
/// `used_fallback` so the caller can surface it (a toast, in the current UI) rather than silently
/// describing unstaged changes as if they were staged. The same exclusion/truncation pipeline
/// below runs unmodified over whichever [`git2::Diff`] was selected.
///
/// `skipped_files` is collected from *inside* the [`git2::Diff::print`] callback rather than a
/// separate pass over `diff.deltas()` beforehand — libgit2 only determines whether a delta's
/// content is binary while generating its patch (i.e. during this same `print` call), so a
/// delta's `is_binary()` flag reads as unset (`false`) on a fresh [`git2::Diff`] that hasn't been
/// printed/patched yet.
pub fn diff_staged_text(repo_path: &Path) -> AppResult<StagedDiffText> {
    let repo = open_repo(repo_path)?;
    let head_tree = head_tree_of(&repo);

    let staged_diff = repo.diff_tree_to_index(head_tree.as_ref(), None, None).map_err(map_git_err)?;
    let used_fallback = staged_diff.deltas().len() == 0;
    let diff = if used_fallback {
        let mut fallback_opts = git2::DiffOptions::new();
        // `recurse_untracked_dirs` defaults to off in libgit2 — without it, a brand-new untracked
        // *directory* collapses into a single opaque delta for the directory path instead of one
        // delta per file inside it, so its file contents never reach `Diff::print` at all.
        fallback_opts
            .include_untracked(true)
            .show_untracked_content(true)
            .recurse_untracked_dirs(true);
        repo.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut fallback_opts))
            .map_err(map_git_err)?
    } else {
        staged_diff
    };

    let mut buffer: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut skipped_files: Vec<String> = Vec::new();
    diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
        if is_excluded_from_staged_diff(&delta) {
            if let Some(path) = staged_diff_delta_path(&delta) {
                if !skipped_files.contains(&path) {
                    skipped_files.push(path);
                }
            }
            return true;
        }
        if buffer.len() >= STAGED_DIFF_TEXT_MAX_BYTES {
            truncated = true;
            return true;
        }
        if matches!(line.origin(), '+' | '-' | ' ') {
            buffer.push(line.origin() as u8);
        }
        buffer.extend_from_slice(line.content());
        true
    })
    .map_err(map_git_err)?;

    if buffer.len() > STAGED_DIFF_TEXT_MAX_BYTES {
        buffer.truncate(utf8_safe_truncate_len(&buffer, STAGED_DIFF_TEXT_MAX_BYTES));
        truncated = true;
    }

    let mut diff_text = String::from_utf8_lossy(&buffer).into_owned();
    append_staged_diff_notices(&mut diff_text, truncated, &skipped_files);

    Ok(StagedDiffText {
        diff_text,
        truncated,
        skipped_files,
        used_fallback,
    })
}

/// Appends [`diff_staged_text`]'s truncation/skip facts as plain-text notices directly onto the
/// body handed to the AI model — `truncated`/`skipped_files` alone only reach the user (as a
/// toast), so without this the model has no way to know it's summarizing a partial diff, or that
/// binary/lock files changed at all, and would confidently describe an incomplete picture as
/// complete (contract §3.3 "절삭 사실 문자열 명시"). Filenames only, never file content — the same
/// exclusion [`is_excluded_from_staged_diff`] already enforces for the body itself.
fn append_staged_diff_notices(diff_text: &mut String, truncated: bool, skipped_files: &[String]) {
    if truncated {
        diff_text.push_str("\n\n[diff truncated at the size limit — later changes are not shown]\n");
    }
    if !skipped_files.is_empty() {
        diff_text.push_str("\n[files omitted from this diff — binary or lock file, content not shown]:\n");
        for path in skipped_files {
            diff_text.push_str("- ");
            diff_text.push_str(path);
            diff_text.push('\n');
        }
    }
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

    let patch = git2::Patch::from_diff(&diff, 0).map_err(map_git_err)?.ok_or_else(|| {
        AppError::localized(AppErrorKind::NotFound, "error.git.noChanges", format!("{relative}: no changes")).with_arg("path", &relative)
    })?;

    let mut target: Option<(u32, u32, u32, u32)> = None;
    for hunk_index in 0..patch.num_hunks() {
        let (hunk, _lines) = patch.hunk(hunk_index).map_err(map_git_err)?;
        let (start, end) = gutter_range(hunk.new_start(), hunk.new_lines());
        if start == hunk_start && end == hunk_end {
            target = Some((hunk.old_start(), hunk.old_lines(), hunk.new_start(), hunk.new_lines()));
            break;
        }
    }

    let (old_start, old_lines, new_start, new_lines) = target.ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.hunkNotFound",
            format!("{relative}: could not find hunk ({hunk_start}-{hunk_end})"),
        )
        .with_arg("path", &relative)
        .with_arg("hunkStart", hunk_start)
        .with_arg("hunkEnd", hunk_end)
    })?;

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
        return Err(AppError::localized(
            AppErrorKind::Internal,
            "error.git.hunkRangeOutOfFile",
            format!("{relative}: hunk range is out of the file's bounds"),
        )
        .with_arg("path", &relative));
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
    let conflict = index.conflict_get(Path::new(&relative)).map_err(|error| {
        AppError::localized(
            AppErrorKind::NotFound,
            "error.git.conflictInfoNotFound",
            format!("{relative}: could not find conflict info: {error}"),
        )
        .with_arg("path", &relative)
        .with_arg("detail", &error)
    })?;

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
    let patch = git2::Patch::from_diff(diff, 0).map_err(map_git_err)?.ok_or_else(|| {
        AppError::localized(AppErrorKind::NotFound, "error.git.noChanges", format!("{relative}: no changes")).with_arg("path", relative)
    })?;
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
        let selected = select_hunk_lines(&patch, hunk_index, line_count, anchor, 0..=u32::MAX)?.ok_or_else(|| {
            AppError::localized(AppErrorKind::NotFound, "error.git.noChanges", format!("{relative}: no changes")).with_arg("path", relative)
        })?;

        return Ok(build_patch_text(relative, &selected, sides));
    }

    Err(AppError::localized(
        AppErrorKind::InvalidArgument,
        "error.git.hunkNotFound",
        format!("{relative}: could not find hunk ({hunk_start}-{hunk_end})"),
    )
    .with_arg("path", relative)
    .with_arg("hunkStart", hunk_start)
    .with_arg("hunkEnd", hunk_end))
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
    let patch = git2::Patch::from_diff(diff, 0).map_err(map_git_err)?.ok_or_else(|| {
        AppError::localized(AppErrorKind::NotFound, "error.git.noChanges", format!("{relative}: no changes")).with_arg("path", relative)
    })?;
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
            return Err(AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.git.rangeSpansMultipleHunks",
                format!(
                    "{relative}: the requested range ({}-{}) spans multiple hunks",
                    line_range.start(),
                    line_range.end()
                ),
            )
            .with_arg("path", relative)
            .with_arg("start", line_range.start())
            .with_arg("end", line_range.end()));
        }

        let anchor = HunkAnchor {
            old_start: hunk.old_start(),
            new_start: hunk.new_start(),
            match_old_side,
        };
        let selected = select_hunk_lines(&patch, hunk_index, line_count, anchor, line_range.clone())?.ok_or_else(|| {
            AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.git.rangeHasNoChangedLines",
                format!(
                    "{relative}: the requested range ({}-{}) has no changed lines",
                    line_range.start(),
                    line_range.end()
                ),
            )
            .with_arg("path", relative)
            .with_arg("start", line_range.start())
            .with_arg("end", line_range.end())
        })?;

        return Ok(build_patch_text(relative, &selected, sides));
    }

    Err(AppError::localized(
        AppErrorKind::InvalidArgument,
        "error.git.rangeHunkNotFound",
        format!(
            "{relative}: could not find a hunk containing the requested range ({}-{})",
            line_range.start(),
            line_range.end()
        ),
    )
    .with_arg("path", relative)
    .with_arg("start", line_range.start())
    .with_arg("end", line_range.end()))
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
                return Err(AppError::localized(
                    AppErrorKind::InvalidArgument,
                    "error.git.noNewlineAtEofLineStageUnsupported",
                    "line-level staging is not supported for a hunk with no trailing newline",
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
    let workdir = repo_workdir(&repo)?;
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

        out.push(CommitFile {
            abs_path: workdir.join(&path).to_string_lossy().into_owned(),
            orig_abs_path: orig_path.as_ref().map(|orig| workdir.join(orig).to_string_lossy().into_owned()),
            path,
            orig_path,
            kind,
        });
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
        let conflicted_paths = conflicted_index_paths(&index)?;
        let workdir = repo_workdir(&repo)?;
        let conflicted_abs_paths = conflicted_paths
            .iter()
            .map(|path| workdir.join(path).to_string_lossy().into_owned())
            .collect();
        return Ok(RevertOutcome {
            conflicted: true,
            conflicted_paths,
            conflicted_abs_paths,
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
        conflicted_abs_paths: Vec::new(),
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
    let remote_branch = repo.find_branch(remote_ref, git2::BranchType::Remote).map_err(|error| {
        AppError::localized(
            AppErrorKind::NotFound,
            "error.git.remoteBranchNotFound",
            format!("{remote_ref}: could not find remote branch: {error}"),
        )
        .with_arg("remoteRef", remote_ref)
        .with_arg("detail", &error)
    })?;

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

    remote_ref.split_once('/').map(|(_, rest)| rest.to_string()).ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.remoteRefMalformed",
            format!("{remote_ref}: not a valid remote branch reference"),
        )
        .with_arg("remoteRef", remote_ref)
    })
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
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.cannotDeleteCheckedOutBranch",
            "cannot delete the currently checked-out branch",
        ));
    }

    if !force {
        let head_oid = repo
            .head()
            .map_err(map_git_err)?
            .target()
            .ok_or_else(|| AppError::localized(AppErrorKind::Internal, "error.git.headUnresolvable", "could not resolve HEAD"))?;
        let branch_oid = branch.get().target().ok_or_else(|| {
            AppError::localized(
                AppErrorKind::Internal,
                "error.git.branchTargetUnresolvable",
                "could not resolve branch target",
            )
        })?;
        let is_merged = head_oid == branch_oid || repo.graph_descendant_of(head_oid, branch_oid).unwrap_or(false);
        if !is_merged {
            return Err(AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.git.branchNotMerged",
                format!("branch '{name}' is not merged; use force to delete it anyway"),
            )
            .with_arg("name", name));
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
            AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.git.stashApplyConflict",
                format!("applying the stash conflicted: {error}"),
            )
            .with_arg("detail", &error)
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
            AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.git.checkoutConflict",
                format!("cannot check out: conflicts with uncommitted changes: {error}"),
            )
            .with_arg("detail", &error)
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
    Repository::open(repo_path).map_err(|error| {
        AppError::localized(
            AppErrorKind::NotFound,
            "error.git.repositoryOpenFailed",
            format!("could not open git repository: {error}"),
        )
        .with_arg("detail", &error)
    })
}

fn repo_workdir(repo: &Repository) -> AppResult<PathBuf> {
    repo.workdir().map(Path::to_path_buf).ok_or_else(|| {
        AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.bareUnsupported",
            "bare repositories are not supported",
        )
    })
}

fn to_repo_relative(repo_root: &Path, raw: &str) -> AppResult<String> {
    let normalized = raw.replace('\\', "/");
    let candidate = Path::new(&normalized);
    let relative = if candidate.is_relative() {
        candidate.to_path_buf()
    } else {
        candidate
            .strip_prefix(repo_root)
            .map_err(|_| {
                AppError::localized(
                    AppErrorKind::InvalidArgument,
                    "error.git.pathOutsideRepository",
                    format!("path is outside the repository: {raw}"),
                )
                .with_arg("path", raw)
            })?
            .to_path_buf()
    };

    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
    {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.pathOutsideRepository",
            format!("path is outside the repository: {raw}"),
        )
        .with_arg("path", raw));
    }

    Ok(relative.to_string_lossy().to_string())
}

/// Deliberately does **not** set `StatusOptions::update_index` (audit R4#11, C11 axis B): that
/// flag makes a status *query* write `.git/index` back to disk — libgit2 persists refreshed stat
/// info for racily-clean entries — from a command path that takes no mutation guard, racing every
/// guarded command that writes the index (`stage`, `commit`, ...). Per the git2 doc the flag is
/// purely a stat-cache optimization ("results in less work being done on subsequent calls");
/// dropping it leaves the returned status byte-identical and only costs re-hashing entries whose
/// stat info is stale in the index on each call — bounded by the number of such entries, and any
/// index-writing git operation (a commit, a stage, terminal `git status`) refreshes them anyway.
fn collect_status_rows(repo: &Repository) -> AppResult<Vec<StatusRow>> {
    let workdir = repo_workdir(repo)?;
    let mut opts = git2::StatusOptions::new();
    opts.show(git2::StatusShow::IndexAndWorkdir)
        .include_untracked(true)
        .recurse_untracked_dirs(false)
        .include_ignored(false)
        .include_unmodified(false)
        .exclude_submodules(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

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
            abs_path: workdir.join(path).to_string_lossy().into_owned(),
            orig_abs_path: orig_path.as_ref().map(|orig| workdir.join(orig).to_string_lossy().into_owned()),
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
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.git.indexNotClean",
            "cannot run: the index has staged changes. Commit or unstage them first.",
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

/// Classifies a `git2::Error` into a locale key without ever producing `NotFound`/`InvalidArgument`
/// — those two kinds are the only ones TS consumers branch on (`commit-file-diff.tsx`,
/// `workspace-edit-applier.ts`), and a git2 `ErrorCode::NotFound` (e.g. a bad `revparse_single`
/// rev) promoted to that kind would make those screens silently swallow the error instead of
/// showing it. See `docs/acknowledge/2026-08-24-d34-apperror-campaign-contract.md` §3.1 축1.
fn map_git_err(error: git2::Error) -> AppError {
    let detail = error.message().to_string();
    let (kind, key, fallback) = match error.code() {
        git2::ErrorCode::NotFound => (AppErrorKind::Internal, "error.git.objectNotFound", "git object not found"),
        git2::ErrorCode::Exists => (AppErrorKind::Internal, "error.git.alreadyExists", "git object already exists"),
        git2::ErrorCode::Conflict | git2::ErrorCode::MergeConflict => {
            (AppErrorKind::Internal, "error.git.conflict", "git operation conflicted")
        }
        git2::ErrorCode::Auth | git2::ErrorCode::Certificate => {
            (AppErrorKind::Forbidden, "error.git.authFailed", "git authentication failed")
        }
        git2::ErrorCode::Locked => (AppErrorKind::Internal, "error.git.locked", "git reference is locked"),
        git2::ErrorCode::UnbornBranch => (
            AppErrorKind::Internal,
            "error.git.unbornBranch",
            "the current branch has no commits yet",
        ),
        git2::ErrorCode::Uncommitted => (
            AppErrorKind::Internal,
            "error.git.uncommittedChanges",
            "uncommitted changes are blocking this operation",
        ),
        _ => match error.class() {
            git2::ErrorClass::Net | git2::ErrorClass::Http | git2::ErrorClass::Ssh => {
                (AppErrorKind::Internal, "error.git.network", "git network operation failed")
            }
            _ => (AppErrorKind::Internal, "error.git.operationFailed", "git operation failed"),
        },
    };
    AppError::localized(kind, key, format!("{fallback}: {detail}")).with_arg("detail", &detail)
}

/// Runs `command` to completion, killing it once it outlives `timeout` and reporting that as
/// `Ok(None)`. Replaces `Command::output()`, which waits forever: the stdout/stderr pipes are drained by
/// their own threads (a `git push`/`pull` writes more progress output than a pipe buffer holds, so a
/// caller that only polls the exit status would deadlock the child against a full pipe), and the exit
/// status is polled meanwhile. Stdin is null and both outputs are piped — byte-for-byte the
/// configuration `output()` applies — so a killed subprocess is the only observable difference.
///
/// On the kill path the reader threads are deliberately **not** joined: `kill` reaches the `git` process
/// only, and a grandchild it left holding the write end (an `ssh`, a credential helper) would keep the
/// read blocked, turning the timeout into the very hang it exists to prevent. The detached threads end on
/// their own when the last writer closes, and their buffers are dropped unread — nothing on this path
/// wants the output.
fn run_command_with_timeout(
    command: &mut std::process::Command,
    timeout: std::time::Duration,
) -> std::io::Result<Option<std::process::Output>> {
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().map(read_pipe_on_thread);
    let stderr = child.stderr.take().map(read_pipe_on_thread);

    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if std::time::Instant::now() >= deadline {
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(GIT_COMMAND_POLL_INTERVAL_MS));
    };

    let Some(status) = status else {
        child.kill()?;
        child.wait()?;
        return Ok(None);
    };

    let drain_deadline = std::time::Instant::now() + std::time::Duration::from_secs(GIT_PIPE_DRAIN_TIMEOUT_SECS);
    let collect = |reader: Option<std::sync::mpsc::Receiver<Vec<u8>>>| {
        reader
            .and_then(|receiver| {
                receiver
                    .recv_timeout(drain_deadline.saturating_duration_since(std::time::Instant::now()))
                    .ok()
            })
            .unwrap_or_default()
    };
    Ok(Some(std::process::Output {
        status,
        stdout: collect(stdout),
        stderr: collect(stderr),
    }))
}

/// Drains one child pipe to end-of-file on its own thread, handing the bytes back over a channel. A
/// read error yields whatever was read so far rather than propagating — the caller's contract is the
/// subprocess's exit status, and a truncated stderr is still better diagnostics than none.
///
/// A channel rather than a `JoinHandle` because the collecting side must be able to *give up*: see
/// [`GIT_PIPE_DRAIN_TIMEOUT_SECS`]. A `JoinHandle` can only be waited on forever.
fn read_pipe_on_thread(mut pipe: impl std::io::Read + Send + 'static) -> std::sync::mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = pipe.read_to_end(&mut buffer);
        let _ = sender.send(buffer);
    });
    receiver
}

fn run_git(repo_path: &Path, args: &[&str]) -> AppResult<String> {
    let timeout = std::time::Duration::from_secs(GIT_COMMAND_TIMEOUT_SECS);
    let output =
        run_command_with_timeout(std::process::Command::new("git").current_dir(repo_path).args(args), timeout).map_err(|error| {
            AppError::localized(
                AppErrorKind::Internal,
                "error.git.spawnFailed",
                format!("failed to run git: {error}"),
            )
            .with_arg("detail", &error)
        })?;

    let Some(output) = output else {
        let command = args.join(" ");
        let seconds = GIT_COMMAND_TIMEOUT_SECS;
        return Err(AppError::localized(
            AppErrorKind::Internal,
            "error.git.commandTimedOut",
            format!("git {command} did not finish within {seconds}s and was stopped"),
        )
        .with_arg("command", &command)
        .with_arg("seconds", seconds));
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let command = args.join(" ");
        return Err(AppError::localized(
            AppErrorKind::Internal,
            "error.git.commandFailed",
            format!("git {command} failed: {stderr}"),
        )
        .with_arg("command", &command)
        .with_arg("detail", &stderr));
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
    fn status의_abs_path는_저장소_루트_기준_절대경로다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "hello");

        let result = status(repo.path()).expect("status");

        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].path, "a.txt");
        let expected_root = std::fs::canonicalize(repo.path()).unwrap_or_else(|_| repo.path().to_path_buf());
        assert_eq!(PathBuf::from(&result.rows[0].abs_path), expected_root.join("a.txt"));
    }

    #[test]
    fn status_조회는_인덱스_파일을_다시_쓰지_않는다() {
        const STAT_STALE_MTIME_SKEW_SECS: u64 = 2;

        let repo = TestRepo::new();
        repo.write_file("a.txt", "one");
        repo.commit_all("first");
        let file_path = repo.path().join("a.txt");
        let mtime_at_commit = std::fs::metadata(&file_path).expect("metadata").modified().expect("mtime");
        repo.write_file("a.txt", "one");
        std::fs::OpenOptions::new()
            .write(true)
            .open(&file_path)
            .expect("open")
            .set_modified(mtime_at_commit + std::time::Duration::from_secs(STAT_STALE_MTIME_SKEW_SECS))
            .expect("set mtime");
        let rewritten_mtime = std::fs::metadata(&file_path).expect("metadata").modified().expect("mtime");
        assert_ne!(
            mtime_at_commit, rewritten_mtime,
            "재기록된 파일의 mtime 이 인덱스가 기록한 시점과 달라야 stat-stale 전제가 성립한다 — 이 전제 없이는 update_index 를 복원해도 테스트가 공허하게 통과한다"
        );
        let index_path = repo.path().join(".git").join("index");
        let index_before = std::fs::read(&index_path).expect("index read");

        let first = status(repo.path()).expect("status");
        let index_after = std::fs::read(&index_path).expect("index read");
        let second = status(repo.path()).expect("status");

        assert!(first.rows.is_empty(), "동일 내용 재기록은 변경으로 보고되면 안 된다");
        assert_eq!(index_before, index_after, "status 조회가 .git/index 를 기록하면 안 된다 (R4#11)");
        assert_eq!(first.rows, second.rows, "stat 캐시 미갱신이 상태 판독 정확도를 바꾸면 안 된다");
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

    /// §4-A-6 재현: `collect_status_rows` 의 `recurse_untracked_dirs(false)` 는 미추적 디렉토리를
    /// `nested/` **단일 행**으로 접어 내려보내는데, 그 행을 그대로 stage 하면 `index.add_path` 가
    /// 디렉토리에 `GIT_EDIRECTORY` 로 실패했다 — UI 가 보여준 행이 상시 stage 불가였다는 뜻이다.
    #[test]
    fn 미추적_디렉토리_행을_stage하면_하위_파일이_인덱스에_들어간다() {
        let repo = TestRepo::new();
        repo.write_file("nested/a.txt", "hello");
        repo.write_file("nested/deep/b.txt", "world");

        let rows = status(repo.path()).expect("status").rows;
        assert_eq!(rows.len(), 1, "미추적 디렉토리는 단일 행으로 접혀야 이 재현이 성립한다");
        assert_eq!(rows[0].path, "nested/");

        stage(repo.path(), &[rows[0].path.clone()]).expect("stage");

        let staged: Vec<String> = status(repo.path())
            .expect("status")
            .rows
            .iter()
            .filter(|row| row.staged == Some(GitChangeKind::Added))
            .map(|row| row.path.clone())
            .collect();
        assert_eq!(staged, vec!["nested/a.txt".to_string(), "nested/deep/b.txt".to_string()]);
    }

    /// 같은 §4-A-6 을 절대경로 축으로 재현한다 — 프론트는 `StatusRow::abs_path` 를 넘기는 경로도
    /// 있고, 그쪽은 `to_repo_relative` 의 `strip_prefix` 가 트레일링 슬래시를 지워 `nested` 로
    /// 도착한다. 슬래시 유무와 무관하게 디렉토리면 `add_all` 로 가야 한다.
    #[test]
    fn 미추적_디렉토리_행은_절대경로로_stage해도_하위_파일이_인덱스에_들어간다() {
        let repo = TestRepo::new();
        repo.write_file("nested/a.txt", "hello");

        let rows = status(repo.path()).expect("status").rows;
        stage(repo.path(), &[rows[0].abs_path.clone()]).expect("stage");

        let result = status(repo.path()).expect("status");
        assert_eq!(result.rows[0].path, "nested/a.txt");
        assert_eq!(result.rows[0].staged, Some(GitChangeKind::Added));
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

    /// §4-A-10 재현: 상태 행의 `path` 는 `nested/`(트레일링 슬래시)인데 절대경로로 도착한 같은
    /// 행은 `to_repo_relative` 의 `strip_prefix` 가 슬래시를 지워 `nested` 가 된다 — 문자열 비교로
    /// untracked 판정을 하면 어긋나 tracked 로 분류되고, `checkout_index` 는 미추적 디렉토리에
    /// 아무 일도 하지 않아 조용한 no-op 가 됐다.
    #[test]
    fn 미추적_디렉토리_행은_절대경로로_discard해도_워킹트리에서_사라진다() {
        let repo = TestRepo::new();
        repo.write_file("nested/a.txt", "hello");

        let rows = status(repo.path()).expect("status").rows;
        assert_eq!(rows[0].path, "nested/");

        discard(repo.path(), &[rows[0].abs_path.clone()]).expect("discard");

        assert!(!repo.path().join("nested").exists());
    }

    /// §2 M-1: 상한을 넘는 블롭은 `Odb::read_header` 가 읽은 헤더만으로 거절되어야 하고, 상한과
    /// 같은 크기는 그대로 통과해야 한다(경계 포함).
    #[test]
    fn show_file은_상한을_넘는_블롭을_거부하고_상한_이하는_돌려준다() {
        let repo = TestRepo::new();
        repo.write_file("blob.txt", "0123456789");
        repo.commit_all("init");

        let error = show_file_limited(repo.path(), "HEAD", "blob.txt", 9).expect_err("상한 초과 블롭은 거부되어야 한다");
        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);

        let content = show_file_limited(repo.path(), "HEAD", "blob.txt", 10).expect("상한과 같은 크기는 통과해야 한다");
        assert_eq!(content, "0123456789");
    }

    /// §4-B B11 재현: 스테이지된 개명은 HEAD 쪽 blob 이 **원경로**에 있는데 양쪽을 새 경로로 읽어
    /// 원본이 비어버렸다 — 편집이 아니라 파일 전체 추가로 보였다.
    #[test]
    fn 스테이지된_개명의_diff는_원경로를_주면_head_blob을_원본으로_쓴다() {
        let repo = TestRepo::new();
        repo.write_file("old.txt", "line1\nline2\nline3\nline4\n");
        repo.commit_all("init");
        std::fs::rename(repo.path().join("old.txt"), repo.path().join("new.txt")).unwrap();
        repo.write_file("new.txt", "line1\nline2\nline3\nline4 changed\n");
        stage(repo.path(), &["old.txt".to_string(), "new.txt".to_string()]).expect("stage");

        let without_before = diff_file(repo.path(), "new.txt", DiffMode::IndexVsHead, None, &[]).expect("diff");
        assert_eq!(
            without_before.original, "",
            "원경로를 주지 않으면 HEAD 측이 비어 파일 전체 추가로 보인다 — 이것이 B11 의 증상이다"
        );

        let with_before = diff_file(repo.path(), "new.txt", DiffMode::IndexVsHead, Some("old.txt"), &[]).expect("diff");
        assert_eq!(with_before.original, "line1\nline2\nline3\nline4\n");
        assert_eq!(with_before.modified, "line1\nline2\nline3\nline4 changed\n");
    }

    /// 개명이 아직 스테이지되지 않은 축 — 인덱스 항목이 원경로에 남아 있으므로 워킹트리 비교도
    /// 같은 `before_path` 축을 그대로 쓴다.
    #[test]
    fn 스테이지되지_않은_개명의_diff는_원경로의_인덱스_blob을_원본으로_쓴다() {
        let repo = TestRepo::new();
        repo.write_file("old.txt", "line1\nline2\nline3\nline4\n");
        repo.commit_all("init");
        std::fs::rename(repo.path().join("old.txt"), repo.path().join("new.txt")).unwrap();
        repo.write_file("new.txt", "line1\nline2\nline3\nline4 changed\n");

        let sides = diff_file(repo.path(), "new.txt", DiffMode::WorkdirVsIndex, Some("old.txt"), &[]).expect("diff");

        assert_eq!(sides.original, "line1\nline2\nline3\nline4\n");
        assert_eq!(sides.modified, "line1\nline2\nline3\nline4 changed\n");
    }

    /// §2 M-7: 상한을 넘긴 서브프로세스는 죽고 `None` 으로 보고되어야 한다 — 이전에는
    /// `Command::output()` 이 끝날 때까지 무한정 기다렸다.
    #[cfg(unix)]
    #[test]
    fn 상한을_넘긴_서브프로세스는_강제_종료되고_none을_돌려준다() {
        const SLEEP_SECONDS: u64 = 30;
        const TIMEOUT_MS: u64 = 200;

        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(format!("sleep {SLEEP_SECONDS}"));
        let started = std::time::Instant::now();

        let result = run_command_with_timeout(&mut command, std::time::Duration::from_millis(TIMEOUT_MS)).expect("spawn");

        assert!(result.is_none(), "상한을 넘긴 프로세스는 출력 대신 None 으로 보고되어야 한다");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(SLEEP_SECONDS),
            "타임아웃 경로가 자식의 자연 종료를 기다리면 안 된다"
        );
    }

    #[cfg(unix)]
    #[test]
    fn 상한_안에_끝난_서브프로세스는_stdout과_stderr를_그대로_돌려준다() {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg("printf hello; printf oops >&2");

        let output = run_command_with_timeout(&mut command, std::time::Duration::from_secs(10))
            .expect("spawn")
            .expect("상한 안에 끝난 프로세스는 출력이 있어야 한다");

        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), "hello");
        assert_eq!(String::from_utf8_lossy(&output.stderr), "oops");
    }

    /// 파이프 버퍼(보통 64KiB)보다 큰 출력을 내는 자식 — 종료 상태만 폴링하고 파이프를 비우지
    /// 않으면 자식이 가득 찬 파이프에 막혀 교착한다. `git push`/`pull` 의 진행 출력이 정확히 이
    /// 모양이라, 리더 스레드 없이 폴링만 하는 구현은 여기서 죽는다.
    #[cfg(unix)]
    #[test]
    fn 파이프_버퍼보다_큰_출력도_교착_없이_수집된다() {
        const OUTPUT_BYTES: usize = 512 * 1024;

        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(format!("head -c {OUTPUT_BYTES} /dev/zero | tr '\\0' 'x'"));

        let output = run_command_with_timeout(&mut command, std::time::Duration::from_secs(30))
            .expect("spawn")
            .expect("상한 안에 끝난 프로세스는 출력이 있어야 한다");

        assert_eq!(output.stdout.len(), OUTPUT_BYTES);
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
        assert_eq!(staged.modified, "line1\nCHANGED2\nline3\nCHANGED4\nline5\n");

        unstage_hunk(repo.path(), "a.txt", second_start, second_end).expect("unstage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
        assert_eq!(staged.modified, "line1\nline3\n");

        unstage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("unstage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
        assert_eq!(staged.modified, "line1\nNEW\nline2\n");

        unstage_hunk(repo.path(), "a.txt", hunks[0].start, hunks[0].end).expect("unstage_hunk");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "new.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "new.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
        assert_eq!(staged.modified, "line1\nNEWA\nNEWB\nline2\n");

        unstage_lines(repo.path(), "a.txt", 3, 3).expect("unstage_lines");

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let staged = diff_file(repo.path(), "a.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file");
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

        let a = files.iter().find(|file| file.path == "a.txt").expect("a.txt in diff");
        let expected_root = std::fs::canonicalize(repo.path()).unwrap_or_else(|_| repo.path().to_path_buf());
        assert_eq!(PathBuf::from(&a.abs_path), expected_root.join("a.txt"));
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
    fn diff_staged_text는_스테이지된_변경사항의_통합_diff를_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED\n");
        stage(repo.path(), &["a.txt".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.diff_text.contains("a.txt"));
        assert!(result.diff_text.contains("-line2"));
        assert!(result.diff_text.contains("+CHANGED"));
        assert!(!result.truncated);
        assert!(result.skipped_files.is_empty());
        assert!(!result.used_fallback);
    }

    #[test]
    fn diff_staged_text는_최초_커밋_이전에도_스테이지된_추가_파일을_diff로_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "new file content\n");
        stage(repo.path(), &["a.txt".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.diff_text.contains("a.txt"));
        assert!(result.diff_text.contains("+new file content"));
        assert!(!result.used_fallback);
    }

    #[test]
    fn diff_staged_text는_스테이지된_변경이_없으면_빈_diff를_반환한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.commit_all("init");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.diff_text.is_empty());
        assert!(!result.truncated);
        assert!(result.skipped_files.is_empty());
    }

    #[test]
    fn diff_staged_text는_스테이지된_변경이_없고_워킹트리에_변경이_있으면_워킹트리_diff로_폴백한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\nline2\n");
        repo.commit_all("init");
        repo.write_file("a.txt", "line1\nCHANGED\n");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.used_fallback);
        assert!(result.diff_text.contains("a.txt"));
        assert!(result.diff_text.contains("-line2"));
        assert!(result.diff_text.contains("+CHANGED"));
    }

    #[test]
    fn diff_staged_text는_폴백_시_untracked_파일도_추가로_포함한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\n");
        repo.commit_all("init");
        repo.write_file("new.txt", "brand new content\n");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.used_fallback);
        assert!(result.diff_text.contains("new.txt"));
        assert!(result.diff_text.contains("+brand new content"));
    }

    #[test]
    fn diff_staged_text는_폴백_시_untracked_하위_디렉토리_안의_파일_내용도_포함한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\n");
        repo.commit_all("init");
        repo.write_file("feature/nested.txt", "content inside a new untracked directory\n");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.used_fallback);
        assert!(result.diff_text.contains("feature/nested.txt"));
        assert!(result.diff_text.contains("+content inside a new untracked directory"));
    }

    #[test]
    fn diff_staged_text는_최초_커밋_이전_상태에서도_워킹트리_diff로_폴백한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line1\n");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.used_fallback);
        assert!(result.diff_text.contains("a.txt"));
        assert!(result.diff_text.contains("+line1"));
    }

    #[test]
    fn diff_staged_text는_lock_파일을_본문에서_제외하되_파일명_안내는_남긴다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.commit_all("init");
        repo.write_file("a.txt", "a2");
        repo.write_file("bun.lock", "lockfile content that should never reach the AI prompt");
        stage(repo.path(), &["a.txt".to_string(), "bun.lock".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.diff_text.contains("a.txt"));
        assert!(!result.diff_text.contains("lockfile content"));
        assert!(result.diff_text.contains("bun.lock"));
        assert_eq!(result.skipped_files, vec!["bun.lock".to_string()]);
    }

    #[test]
    fn diff_staged_text는_바이너리_파일을_본문에서_제외하되_파일명_안내는_남긴다() {
        let repo = TestRepo::new();
        let binary_path = repo.path().join("image.png");
        std::fs::write(&binary_path, [0u8, 1, 2, 3, 0, 255, 254]).expect("write binary file");
        stage(repo.path(), &["image.png".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.diff_text.contains("image.png"));
        assert_eq!(result.skipped_files, vec!["image.png".to_string()]);
    }

    #[test]
    fn diff_staged_text는_env_파일을_본문에서_제외하고_skipped_files에_기록한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "a");
        repo.commit_all("init");
        repo.write_file("a.txt", "a2");
        repo.write_file(".env", "some placeholder credential value for the test");
        stage(repo.path(), &["a.txt".to_string(), ".env".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(!result.diff_text.contains("placeholder credential value"));
        assert_eq!(result.skipped_files, vec![".env".to_string()]);
    }

    #[test]
    fn diff_staged_text는_ssh_개인키_파일명을_본문에서_제외하고_skipped_files에_기록한다() {
        let repo = TestRepo::new();
        repo.write_file("id_rsa", "not a real key, just placeholder key material for the test");
        stage(repo.path(), &["id_rsa".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(!result.diff_text.contains("placeholder key material"));
        assert_eq!(result.skipped_files, vec!["id_rsa".to_string()]);
    }

    #[test]
    fn diff_staged_text는_jks_키스토어와_netrc를_본문에서_제외한다() {
        let repo = TestRepo::new();
        repo.write_file("release.jks", "not a real keystore, placeholder binary-ish content");
        repo.write_file(".netrc", "machine example.com login user password placeholder-secret");
        stage(repo.path(), &["release.jks".to_string(), ".netrc".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(!result.diff_text.contains("placeholder-secret"));
        assert!(result.skipped_files.contains(&"release.jks".to_string()));
        assert!(result.skipped_files.contains(&".netrc".to_string()));
    }

    #[test]
    fn diff_staged_text는_상한_바이트를_넘으면_잘라내고_절삭_사실을_본문에_명시한다() {
        let repo = TestRepo::new();
        repo.write_file("a.txt", "line\n".repeat(STAGED_DIFF_TEXT_MAX_BYTES).as_str());
        stage(repo.path(), &["a.txt".to_string()]).expect("stage");

        let result = diff_staged_text(repo.path()).expect("diff_staged_text");

        assert!(result.truncated);
        assert!(result.diff_text.contains("truncated"));
    }

    #[test]
    fn is_secret_like_path는_정확한_파일명을_매칭한다() {
        assert!(is_secret_like_path("id_rsa"));
        assert!(is_secret_like_path("nested/dir/id_ed25519"));
        assert!(!is_secret_like_path("id_rsa.pub"));
    }

    #[test]
    fn is_secret_like_path는_env_접두사_파일명을_매칭한다() {
        assert!(is_secret_like_path(".env"));
        assert!(is_secret_like_path(".env.local"));
        assert!(is_secret_like_path("config/.env.production"));
        assert!(!is_secret_like_path("environment.ts"));
    }

    #[test]
    fn is_secret_like_path는_시크릿_확장자를_매칭한다() {
        assert!(is_secret_like_path("server.pem"));
        assert!(is_secret_like_path("cert.key"));
        assert!(is_secret_like_path("keystore.p12"));
        assert!(!is_secret_like_path("notes.keys"));
    }

    #[test]
    fn is_secret_like_path는_일반_파일은_매칭하지_않는다() {
        assert!(!is_secret_like_path("src/main.rs"));
        assert!(!is_secret_like_path("README.md"));
    }

    #[test]
    fn utf8_safe_truncate_len은_상한이_길이보다_크거나_같으면_전체_길이를_반환한다() {
        let bytes = "abc".as_bytes();
        assert_eq!(utf8_safe_truncate_len(bytes, 3), 3);
        assert_eq!(utf8_safe_truncate_len(bytes, 10), 3);
        assert_eq!(utf8_safe_truncate_len(&[], 5), 0);
    }

    #[test]
    fn utf8_safe_truncate_len은_멀티바이트_경계에서_안전하게_잘라낸다() {
        let bytes = "가".as_bytes();
        assert_eq!(bytes.len(), 3);
        assert_eq!(utf8_safe_truncate_len(bytes, 1), 0);
        assert_eq!(utf8_safe_truncate_len(bytes, 2), 0);
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
        let expected_root = std::fs::canonicalize(repo.path()).unwrap_or_else(|_| repo.path().to_path_buf());
        assert_eq!(
            outcome.conflicted_abs_paths,
            vec![expected_root.join("a.txt").to_string_lossy().into_owned()]
        );
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
        let staged = diff_file(repo.path(), "b.txt", DiffMode::IndexVsHead, None, &[]).expect("diff_file b.txt");
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
