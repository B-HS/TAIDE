use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;

use ignore::{WalkBuilder, WalkState};
use regex::{Regex, RegexBuilder};

use crate::constants::{self, REFUSED_FILE_BYTES};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::infra::persist;
use crate::infra::root_guard;

use super::types::{ReplaceSkipReason, SearchFileMatches, SearchLineMatch, SearchQuery, SEARCH_MATCH_LIMIT};

const BINARY_SNIFF_BYTES: usize = 8_000;
const PREVIEW_TRUNCATE_THRESHOLD_BYTES: usize = 200;
const PREVIEW_CONTEXT_BYTES: usize = 60;
const PREVIEW_ELLIPSIS: &str = "…";

/// How many lines one file's scan advances between two reads of the cancellation flag. A single
/// 40MB minified bundle is one `search_file` call with millions of lines, and before this the
/// cancellation flag was only ever checked *between* files — pressing Escape (or typing the next
/// character, which supersedes the run) could not interrupt it at all (audit §2 H-2). Checking
/// every line would put an atomic load in the hottest loop of the whole search for no benefit;
/// a 1024-line stride bounds the worst-case latency to a few milliseconds of scanning.
const CANCEL_CHECK_LINE_STRIDE: usize = 1_024;

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_whole_word_match(haystack: &str, start: usize, end: usize) -> bool {
    let before_ok = haystack[..start].chars().next_back().map(|c| !is_word_char(c)).unwrap_or(true);
    let after_ok = haystack[end..].chars().next().map(|c| !is_word_char(c)).unwrap_or(true);
    before_ok && after_ok
}

/// A literal (non-regex) needle, prepared **once per query** instead of once per line.
///
/// The previous shape rebuilt both sides of the comparison for every line it scanned:
/// `line.to_string()` (or `to_ascii_lowercase()`) plus a `query.text` clone — two heap allocations
/// per line, including the overwhelming majority of lines that contain no match at all (audit §2
/// M-4). Here the needle is prepared once — ASCII-lowercased up front when the query is
/// case-insensitive — and lines are searched in place, so scanning a line allocates nothing unless
/// it actually matches.
///
/// The case-insensitive branch's byte-wise comparison is exactly equivalent to the old
/// `String::find` on lowercased copies: `to_ascii_lowercase` is byte-length preserving and touches
/// only ASCII bytes, and a needle that is itself valid UTF-8 can never match starting on a UTF-8
/// continuation byte (`0x80..=0xBF` never equals the first byte of a valid encoded char), so every
/// reported offset stays on a char boundary.
#[derive(Debug, Clone)]
struct LiteralMatcher {
    needle: String,
    case_sensitive: bool,
    whole_word: bool,
}

impl LiteralMatcher {
    fn new(query: &SearchQuery) -> Self {
        let needle = if query.case_sensitive {
            query.text.clone()
        } else {
            query.text.to_ascii_lowercase()
        };

        Self {
            needle,
            case_sensitive: query.case_sensitive,
            whole_word: query.whole_word,
        }
    }

    fn equals_ignoring_ascii_case_at(&self, haystack: &[u8], start: usize) -> bool {
        haystack[start..start + self.needle.len()]
            .iter()
            .zip(self.needle.as_bytes())
            .all(|(actual, expected)| actual.to_ascii_lowercase() == *expected)
    }

    /// `from` is always a char boundary (0, or a previous match's end), so the case-sensitive
    /// branch can slice and delegate to `str::find` — the standard library's two-way search,
    /// which is what the old allocating path used and is far better tuned than a hand-rolled
    /// scan. The case-insensitive branch cannot delegate (there is no case-folding `find`), so it
    /// filters candidate starts on the first byte before doing the full comparison.
    fn find_from(&self, line: &str, from: usize) -> Option<usize> {
        if self.case_sensitive {
            return line[from..].find(self.needle.as_str()).map(|offset| from + offset);
        }

        let haystack = line.as_bytes();
        let first = *self.needle.as_bytes().first()?;
        let last_start = haystack.len().checked_sub(self.needle.len())?;

        (from..=last_start)
            .find(|&start| haystack[start].to_ascii_lowercase() == first && self.equals_ignoring_ascii_case_at(haystack, start))
    }

    fn find_matches(&self, line: &str) -> Vec<(u32, u32)> {
        if self.needle.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();
        let mut search_from = 0usize;

        while let Some(start) = self.find_from(line, search_from) {
            let end = start + self.needle.len();

            if !self.whole_word || is_whole_word_match(line, start, end) {
                matches.push((start as u32, end as u32));
            }

            search_from = end;
        }

        matches
    }
}

pub fn find_matches_in_line(line: &str, query: &SearchQuery) -> Vec<(u32, u32)> {
    LiteralMatcher::new(query).find_matches(line)
}

fn compile_regex(query: &SearchQuery) -> AppResult<Regex> {
    RegexBuilder::new(&query.text)
        .case_insensitive(!query.case_sensitive)
        .build()
        .map_err(|error| {
            AppError::localized(
                AppErrorKind::InvalidArgument,
                "error.search.invalidRegex",
                format!("invalid regular expression: {error}"),
            )
            .with_arg("detail", &error)
        })
}

fn find_regex_matches_in_line(line: &str, regex: &Regex, whole_word: bool) -> Vec<(u32, u32)> {
    regex
        .find_iter(line)
        .filter(|found| !whole_word || is_whole_word_match(line, found.start(), found.end()))
        .map(|found| (found.start() as u32, found.end() as u32))
        .collect()
}

enum MatchMode<'a> {
    Literal(&'a LiteralMatcher),
    Regex { regex: &'a Regex, whole_word: bool },
}

fn matches_for_line(line: &str, mode: &MatchMode<'_>) -> Vec<(u32, u32)> {
    match mode {
        MatchMode::Literal(matcher) => matcher.find_matches(line),
        MatchMode::Regex { regex, whole_word } => find_regex_matches_in_line(line, regex, *whole_word),
    }
}

/// Everything one `search_run`/`search_replace` pass needs prepared before it touches a single
/// file: the compiled regex (which already had to be built once, so its invalid-pattern error
/// surfaces before any I/O) and the literal needle. Built once per query and shared by every file
/// — and, for `search`, by every walker thread.
#[derive(Debug, Clone)]
pub struct CompiledQuery {
    regex: Option<Regex>,
    literal: LiteralMatcher,
}

impl CompiledQuery {
    fn mode(&self) -> MatchMode<'_> {
        match &self.regex {
            Some(regex) => MatchMode::Regex {
                regex,
                whole_word: self.literal.whole_word,
            },
            None => MatchMode::Literal(&self.literal),
        }
    }
}

pub fn compile_query(query: &SearchQuery) -> AppResult<CompiledQuery> {
    let regex = if query.regex { Some(compile_regex(query)?) } else { None };

    Ok(CompiledQuery {
        regex,
        literal: LiteralMatcher::new(query),
    })
}

/// Converts a byte offset within `text` (as produced by `str::find`/`Regex::find_iter`, always
/// char-boundary-aligned) to the number of UTF-16 code units that precede it — the unit Monaco's
/// `column`/`Position` and JS `String.slice` both use. Rust's own string indices are UTF-8 byte
/// offsets, so passing them straight through under-counts every match preceded by a multi-byte
/// character: ASCII text is off by nothing, but anything after non-ASCII (Korean, emoji, ...) lands
/// on the wrong column/highlight span in the editor. See
/// `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.3 (#23).
fn byte_offset_to_utf16_units(text: &str, byte_offset: usize) -> u32 {
    text[..byte_offset].chars().map(char::len_utf16).sum::<usize>() as u32
}

fn char_boundary_at_or_before(s: &str, byte_index: usize) -> usize {
    let mut index = byte_index.min(s.len());
    while index > 0 && !s.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn char_boundary_at_or_after(s: &str, byte_index: usize) -> usize {
    let mut index = byte_index.min(s.len());
    while index < s.len() && !s.is_char_boundary(index) {
        index += 1;
    }
    index
}

pub fn build_preview(line: &str, match_start: u32, match_end: u32) -> (String, u32, u32) {
    if line.len() <= PREVIEW_TRUNCATE_THRESHOLD_BYTES {
        return (line.to_string(), match_start, match_end);
    }

    let match_start = match_start as usize;
    let match_end = match_end as usize;

    let window_start = char_boundary_at_or_before(line, match_start.saturating_sub(PREVIEW_CONTEXT_BYTES));
    let window_end = char_boundary_at_or_after(line, (match_end + PREVIEW_CONTEXT_BYTES).min(line.len()));

    let prefix = if window_start > 0 { PREVIEW_ELLIPSIS } else { "" };
    let suffix = if window_end < line.len() { PREVIEW_ELLIPSIS } else { "" };

    let preview = format!("{prefix}{}{suffix}", &line[window_start..window_end]);
    let new_start = (match_start - window_start) + prefix.len();
    let new_end = (match_end - window_start) + prefix.len();

    (preview, new_start as u32, new_end as u32)
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

/// Reads a file that is about to be scanned for matches, refusing the two shapes that must never
/// be buffered whole.
///
/// Before this, both `search_file` and `replace_in_file` called `std::fs::read` — the *entire*
/// file, unconditionally — and only then looked at its first 8KB to decide it was binary and
/// should be dropped (audit §2 H-2 / §4-A-8). A single GB-sized log or database file in the
/// project therefore cost a full GB of resident memory per walker before being thrown away.
/// Here the size ceiling is checked from `metadata` (no read at all), and the binary sniff reads
/// only [`BINARY_SNIFF_BYTES`] before committing to the rest.
///
/// The ceiling is `constants::REFUSED_FILE_BYTES`, the same tier boundary `domain::file::service`
/// refuses to *open* a file at — so "searchable" and "openable" stay the same set: every file the
/// editor can show, search can scan, and nothing else. The reader is additionally capped at that
/// ceiling so a file growing between the `metadata` call and the read cannot reintroduce an
/// unbounded buffer.
fn read_scannable_bytes(path: &Path) -> Result<Vec<u8>, ReplaceSkipReason> {
    let metadata = std::fs::metadata(path).map_err(|_| ReplaceSkipReason::Unreadable)?;
    if metadata.len() >= REFUSED_FILE_BYTES {
        return Err(ReplaceSkipReason::TooLarge);
    }

    let file = std::fs::File::open(path).map_err(|_| ReplaceSkipReason::Unreadable)?;
    let mut reader = file.take(REFUSED_FILE_BYTES);

    let mut bytes = Vec::with_capacity(BINARY_SNIFF_BYTES);
    (&mut reader)
        .take(BINARY_SNIFF_BYTES as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ReplaceSkipReason::Unreadable)?;

    if is_binary(&bytes) {
        return Err(ReplaceSkipReason::Binary);
    }

    reader.read_to_end(&mut bytes).map_err(|_| ReplaceSkipReason::Unreadable)?;

    Ok(bytes)
}

pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_bytes = pattern.as_bytes();
    let text_bytes = text.as_bytes();

    let mut p = 0usize;
    let mut t = 0usize;
    let mut star_p: Option<usize> = None;
    let mut star_t = 0usize;

    while t < text_bytes.len() {
        if p < pattern_bytes.len() && pattern_bytes[p] == text_bytes[t] {
            p += 1;
            t += 1;
        } else if p < pattern_bytes.len() && pattern_bytes[p] == b'*' {
            star_p = Some(p);
            star_t = t;
            p += 1;
        } else if let Some(sp) = star_p {
            p = sp + 1;
            star_t += 1;
            t = star_t;
        } else {
            return false;
        }
    }

    while p < pattern_bytes.len() && pattern_bytes[p] == b'*' {
        p += 1;
    }

    p == pattern_bytes.len()
}

fn relative_path_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn passes_glob_filters(root: &Path, path: &Path, query: &SearchQuery) -> bool {
    let relative = relative_path_string(root, path);

    if let Some(include) = &query.include_glob {
        if !glob_match(include, &relative) {
            return false;
        }
    }

    if let Some(exclude) = &query.exclude_glob {
        if glob_match(exclude, &relative) {
            return false;
        }
    }

    true
}

fn context_before(lines: &[&str], line_index: usize, context: usize) -> Vec<String> {
    let start = line_index.saturating_sub(context);
    lines[start..line_index].iter().map(|line| (*line).to_string()).collect()
}

fn context_after(lines: &[&str], line_index: usize, context: usize) -> Vec<String> {
    let start = (line_index + 1).min(lines.len());
    let end = (line_index + 1 + context).min(lines.len());
    lines[start..end].iter().map(|line| (*line).to_string()).collect()
}

/// Collects up to `remaining` matches from one file, in ascending line order.
///
/// `cancelled` is polled every [`CANCEL_CHECK_LINE_STRIDE`] lines, not just once per file, so a
/// superseded or explicitly cancelled run stops inside a huge file instead of scanning it to the
/// end first. Whatever was collected before the flag flipped is discarded by the caller, which
/// checks the same flag.
fn search_file(path: &Path, query: &SearchQuery, mode: &MatchMode<'_>, remaining: u32, cancelled: &AtomicBool) -> Vec<SearchLineMatch> {
    if remaining == 0 {
        return Vec::new();
    }

    let Ok(bytes) = read_scannable_bytes(path) else {
        return Vec::new();
    };

    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<&str> = text.lines().collect();
    let context = query.context_lines as usize;
    let mut collected: Vec<SearchLineMatch> = Vec::new();

    'lines: for (line_index, line) in lines.iter().copied().enumerate() {
        if line_index % CANCEL_CHECK_LINE_STRIDE == 0 && cancelled.load(Ordering::Relaxed) {
            break 'lines;
        }

        for (match_start, match_end) in matches_for_line(line, mode) {
            if collected.len() as u32 >= remaining {
                break 'lines;
            }

            let (preview, preview_start, preview_end) = build_preview(line, match_start, match_end);
            let column = byte_offset_to_utf16_units(line, match_start as usize) + 1;
            let preview_match_start = byte_offset_to_utf16_units(&preview, preview_start as usize);
            let preview_match_end = byte_offset_to_utf16_units(&preview, preview_end as usize);
            collected.push(SearchLineMatch {
                line: (line_index + 1) as u32,
                column,
                preview,
                match_start: preview_match_start,
                match_end: preview_match_end,
                before: context_before(&lines, line_index, context),
                after: context_after(&lines, line_index, context),
            });
        }
    }

    collected
}

/// Claims `found.len()` matches (or as many as are left) out of the run-wide
/// [`SEARCH_MATCH_LIMIT`] budget shared by every walker thread, and truncates `found` to what it
/// actually got. Returns `false` when the budget is exhausted, which is the signal to quit the
/// walk entirely.
///
/// The compare-exchange loop is what keeps the limit exact under parallelism: a plain
/// `fetch_add` would let N threads that each read "9,999 used" all append their own file's
/// matches, overshooting the cap the frontend and `search_run`'s return value both rely on.
fn claim_match_budget(total: &AtomicU32, found: &mut Vec<SearchLineMatch>) -> bool {
    loop {
        let current = total.load(Ordering::Acquire);
        if current >= SEARCH_MATCH_LIMIT {
            return false;
        }

        let allowed = (SEARCH_MATCH_LIMIT - current).min(found.len() as u32);
        if total
            .compare_exchange_weak(current, current + allowed, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            found.truncate(allowed as usize);
            return true;
        }
    }
}

/// The walker configuration shared by [`search`]'s parallel walk and the sequential walks of
/// [`collect_project_files`]/[`list_project_files`] — one source of truth for *which* files any
/// search-domain traversal can ever see, whichever way it is driven.
///
/// `constants::IGNORED_DIR_NAMES` is pruned unconditionally (via `filter_entry`)
/// regardless of `respect_gitignore` — those directories (`.git`, `node_modules`, ...)
/// were never searchable before this feature and are not meant to become
/// searchable just because a project has no `.gitignore` or the toggle is off.
/// `respect_gitignore` only gates the `ignore` crate's own `.gitignore`/
/// `.git/info/exclude` recognition. Global git config excludes, parent-directory
/// ignore *rules*, hidden-file filtering, and `.ignore` files are deliberately
/// left unapplied in both modes — matching VS Code's own `search.useIgnoreFiles`
/// scope (repo-local git ignore rules only) and preserving the pre-existing
/// behavior of searching dotfiles that don't match `IGNORED_DIR_NAMES`.
/// `.parents(false)` above is what disables applying those ancestor rules to
/// the results (verified: no ancestor-only-ignored file ever appears). It
/// does *not* stop the `ignore` crate from opening and parsing each ancestor
/// directory's `.gitignore` on every call when `respect_gitignore` is on —
/// the crate ties that read to `git_ignore`/`git_exclude`, not to `parents`
/// (`ignore::dir::Ignore::add_parents`). Those reads never leave the
/// filesystem/never affect results, so this is a (tiny, per-search) I/O
/// cost rather than a correctness or sandboxing gap. See
/// `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4.
fn configure_walk(root: &Path, respect_gitignore: bool) -> WalkBuilder {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_global(false)
        .git_ignore(respect_gitignore)
        .git_exclude(respect_gitignore)
        .require_git(false)
        .filter_entry(|entry| {
            if entry.depth() == 0 {
                return true;
            }
            let is_dir = entry.file_type().map(|file_type| file_type.is_dir()).unwrap_or(false);
            if !is_dir {
                return true;
            }
            !constants::is_ignored_dir(&entry.file_name().to_string_lossy())
        });
    builder
}

fn build_walk(root: &Path, respect_gitignore: bool) -> ignore::Walk {
    configure_walk(root, respect_gitignore).build()
}

/// Walks `root` on every available core and streams each file's matches out as a single
/// [`SearchFileMatches`] batch.
///
/// The walk was sequential before (audit §2 H-1, and a standing contradiction with
/// `docs/architecture.md`'s "병렬 스캔"): one thread both listed directories and read every file,
/// so a project-wide search ran at single-disk-queue speed no matter how many cores were idle.
/// Three pieces of shared state make the parallel version behave like the sequential one did:
///
/// - `cancelled` is read by every worker before each file (and inside [`search_file`]'s line
///   loop), so a cancelled run stops everywhere, not just on the thread that noticed.
/// - the run-wide match budget is claimed atomically per file ([`claim_match_budget`]), so
///   [`SEARCH_MATCH_LIMIT`] stays an exact cap rather than a per-thread one.
/// - results reach `on_file_matches` through an mpsc channel drained on *this* thread, which
///   keeps the callback a plain `FnMut` (no `Send`/locking requirement on callers) and keeps
///   results streaming while the walk is still running — `WalkParallel::run` itself blocks until
///   the whole tree is done.
///
/// **Ordering contract**: matches within one file arrive in ascending source order, but the order
/// the *files* arrive in is unspecified and varies between runs. Callers that need a stable
/// display order must sort; the frontend deliberately does not (results appear as they are found,
/// like VS Code's).
pub fn search(
    root: &Path,
    query: &SearchQuery,
    cancelled: &AtomicBool,
    mut on_file_matches: impl FnMut(SearchFileMatches),
) -> AppResult<u32> {
    if query.text.is_empty() {
        return Ok(0);
    }

    let compiled = compile_query(query)?;
    let total = AtomicU32::new(0);

    std::thread::scope(|scope| {
        let (sender, receiver) = mpsc::channel::<SearchFileMatches>();
        let total = &total;
        let compiled = &compiled;

        scope.spawn(move || {
            configure_walk(root, query.respect_gitignore).build_parallel().run(|| {
                let sender = sender.clone();
                let mode = compiled.mode();
                Box::new(move |entry| {
                    if cancelled.load(Ordering::Relaxed) {
                        return WalkState::Quit;
                    }

                    let Ok(entry) = entry else {
                        return WalkState::Continue;
                    };
                    if !entry.file_type().map(|file_type| file_type.is_file()).unwrap_or(false) {
                        return WalkState::Continue;
                    }

                    let path = entry.path();
                    if !passes_glob_filters(root, path, query) {
                        return WalkState::Continue;
                    }

                    let claimed = total.load(Ordering::Acquire);
                    if claimed >= SEARCH_MATCH_LIMIT {
                        return WalkState::Quit;
                    }

                    let mut found = search_file(path, query, &mode, SEARCH_MATCH_LIMIT - claimed, cancelled);
                    if cancelled.load(Ordering::Relaxed) {
                        return WalkState::Quit;
                    }
                    if found.is_empty() {
                        return WalkState::Continue;
                    }
                    if !claim_match_budget(total, &mut found) {
                        return WalkState::Quit;
                    }

                    let batch = SearchFileMatches {
                        path: path.to_string_lossy().to_string(),
                        matches: found,
                    };
                    if sender.send(batch).is_err() {
                        return WalkState::Quit;
                    }

                    WalkState::Continue
                })
            });
        });

        for batch in receiver {
            on_file_matches(batch);
        }
    });

    Ok(total.load(Ordering::Acquire))
}

fn collect_project_files(root: &Path, query: &SearchQuery) -> Vec<PathBuf> {
    build_walk(root, query.respect_gitignore)
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|file_type| file_type.is_file()).unwrap_or(false))
        .map(|entry| entry.into_path())
        .filter(|path| passes_glob_filters(root, path, query))
        .collect()
}

/// Every file under `root` — the command palette's file quick-open index
/// (`docs/features/command-palette.md` §3: "트리 lazy 로딩에 의존하면 안 된다"; contract
/// `2026-08-25-d42-e2e-defects-contract.md` §3, item d). Reuses [`build_walk`] instead of a new
/// walker, so quick-open agrees with `search`/`search_replace` on exactly which directories never
/// appear (`constants::IGNORED_DIR_NAMES` — `.git`, `node_modules`, ...) rather than
/// re-implementing that list a second time.
///
/// Always called with `respect_gitignore: false` — unlike `search`/`collect_project_files`, there
/// is no per-call `SearchQuery` toggle to honor here, and this index must stay a superset of
/// `domain::tree`'s own listing (`tree::service` prunes only `IGNORED_DIR_NAMES`, never
/// `.gitignore`): a file the Explorer sidebar shows must always be quick-open-able too, so a
/// gitignored-but-tracked file (e.g. a generated file someone deliberately `git add -f`'d) doesn't
/// vanish from quick-open while still sitting in the tree.
pub fn list_project_files(root: &Path) -> Vec<PathBuf> {
    build_walk(root, false)
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|file_type| file_type.is_file()).unwrap_or(false))
        .map(|entry| entry.into_path())
        .collect()
}

fn strip_line_terminator(chunk: &str) -> &str {
    chunk.strip_suffix("\r\n").or_else(|| chunk.strip_suffix('\n')).unwrap_or(chunk)
}

fn apply_line_replacements(chunk: &str, matches: &[(u32, u32)], replacement: &str, output: &mut String) {
    let mut cursor = 0usize;

    for &(start, end) in matches {
        output.push_str(&chunk[cursor..start as usize]);
        output.push_str(replacement);
        cursor = end as usize;
    }

    output.push_str(&chunk[cursor..]);
}

/// What one file's replace pass actually did. Replaces the old `Option<u32>`, which collapsed
/// "nothing matched" and "this file could not be rewritten" into the same `None` and so let a
/// partially-failed replace report itself as a clean success (audit §4-B C10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplaceOutcome {
    Replaced(u32),
    NoMatch,
    Skipped(ReplaceSkipReason),
}

fn replace_in_file(path: &Path, mode: &MatchMode<'_>, replacement: &str) -> ReplaceOutcome {
    let bytes = match read_scannable_bytes(path) {
        Ok(bytes) => bytes,
        Err(reason) => return ReplaceOutcome::Skipped(reason),
    };

    let Ok(text) = std::str::from_utf8(&bytes) else {
        return ReplaceOutcome::Skipped(ReplaceSkipReason::NotUtf8);
    };

    let mut output = String::with_capacity(text.len());
    let mut file_matches = 0u32;

    for chunk in text.split_inclusive('\n') {
        let line = strip_line_terminator(chunk);
        let line_matches = matches_for_line(line, mode);

        if line_matches.is_empty() {
            output.push_str(chunk);
            continue;
        }

        file_matches += line_matches.len() as u32;
        apply_line_replacements(chunk, &line_matches, replacement, &mut output);
    }

    if file_matches == 0 {
        return ReplaceOutcome::NoMatch;
    }

    if persist::write_atomic_preserving_mode(path, output.as_bytes()).is_err() {
        return ReplaceOutcome::Skipped(ReplaceSkipReason::WriteFailed);
    }

    ReplaceOutcome::Replaced(file_matches)
}

/// Resolves `search_replace`'s target file list without touching disk otherwise — `paths` (an
/// explicit selection, e.g. "replace in these open tabs only") is validated against `root` the same
/// way a full-project replace's tree walk already confines itself, so both call shapes share one
/// root-escape guard.
pub fn resolve_replace_targets(root: &Path, query: &SearchQuery, paths: Option<&[PathBuf]>) -> Vec<PathBuf> {
    match paths {
        Some(explicit) => explicit
            .iter()
            .filter_map(|path| root_guard::ensure_within_root(root, path).ok())
            .collect(),
        None => collect_project_files(root, query),
    }
}

/// Applies one replace pass to a single file — the unit `search_replace`'s command handler calls
/// once per file so it can reacquire `AppState::begin_mutation`'s lock between files instead of
/// holding it for a whole project-wide replace (see that command's doc comment).
pub fn replace_one_file(path: &Path, compiled: &CompiledQuery, replacement: &str) -> ReplaceOutcome {
    replace_in_file(path, &compiled.mode(), replacement)
}

#[cfg(test)]
mod tests {
    use super::super::types::{ReplaceSkippedFile, SearchReplaceResult, REPLACE_SKIP_REPORT_LIMIT};
    use super::*;

    fn query(text: &str) -> SearchQuery {
        SearchQuery {
            text: text.to_string(),
            case_sensitive: false,
            whole_word: false,
            regex: false,
            include_glob: None,
            exclude_glob: None,
            context_lines: 0,
            respect_gitignore: true,
        }
    }

    /// Flattens [`search`]'s per-file batches back into one `(path, match)` list so assertions can
    /// stay written against individual matches. Sorted by path (then source order within a file),
    /// because the parallel walk deliberately leaves the order files arrive in unspecified — a
    /// test that asserted arrival order would be flaky by construction.
    fn collect_search(root: &Path, query: &SearchQuery, cancelled: &AtomicBool) -> (u32, Vec<(String, SearchLineMatch)>) {
        let mut flat = Vec::new();
        let total = search(root, query, cancelled, |batch| {
            let SearchFileMatches { path, matches } = batch;
            for item in matches {
                flat.push((path.clone(), item));
            }
        })
        .unwrap();

        flat.sort_by(|left, right| left.0.cmp(&right.0));
        (total, flat)
    }

    #[test]
    fn 쿼리가_비어있으면_빈_결과를_반환한다() {
        let matches = find_matches_in_line("hello world", &query(""));
        assert!(matches.is_empty());
    }

    #[test]
    fn 대소문자를_구분하지_않으면_모든_대소문자_조합을_찾는다() {
        let matches = find_matches_in_line("Hello hello HELLO", &query("hello"));
        assert_eq!(matches, vec![(0, 5), (6, 11), (12, 17)]);
    }

    #[test]
    fn 대소문자_구분_옵션이면_정확히_일치하는_경우만_찾는다() {
        let mut q = query("hello");
        q.case_sensitive = true;
        let matches = find_matches_in_line("Hello hello HELLO", &q);
        assert_eq!(matches, vec![(6, 11)]);
    }

    #[test]
    fn 단어_단위_옵션이면_부분_단어는_제외한다() {
        let mut q = query("cat");
        q.whole_word = true;
        let matches = find_matches_in_line("cat concatenate cat_food cat", &q);
        assert_eq!(matches, vec![(0, 3), (25, 28)]);
    }

    #[test]
    fn 겹치지_않는_연속_매치를_모두_찾는다() {
        let matches = find_matches_in_line("aaaa", &query("aa"));
        assert_eq!(matches, vec![(0, 2), (2, 4)]);
    }

    #[test]
    fn 짧은_줄은_그대로_미리보기로_반환한다() {
        let (preview, start, end) = build_preview("short line with match", 12, 17);
        assert_eq!(preview, "short line with match");
        assert_eq!((start, end), (12, 17));
    }

    #[test]
    fn 긴_줄은_매치_주변만_잘라내고_말줄임표를_붙인다() {
        let padding = "x".repeat(300);
        let line = format!("{padding}needle{padding}");
        let match_start = padding.len() as u32;
        let match_end = match_start + "needle".len() as u32;

        let (preview, new_start, new_end) = build_preview(&line, match_start, match_end);

        assert!(preview.starts_with('…'));
        assert!(preview.ends_with('…'));
        assert_eq!(&preview[new_start as usize..new_end as usize], "needle");
        assert!(preview.len() < line.len());
    }

    #[test]
    fn 글롭_패턴은_와일드카드로_임의의_문자열에_매칭한다() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("src/*.ts", "src/index.ts"));
        assert!(!glob_match("*.rs", "main.ts"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact.txt", "exact.txt"));
        assert!(!glob_match("exact.txt", "not-exact.txt"));
    }

    #[test]
    fn nul_바이트가_있으면_바이너리로_판별한다() {
        assert!(is_binary(&[0x41, 0x42, 0x00, 0x43]));
        assert!(!is_binary(&[0x41, 0x42, 0x43]));
    }

    #[test]
    fn 유효한_정규식으로_매치를_찾는다() {
        let regex = RegexBuilder::new("n.+dle").case_insensitive(false).build().unwrap();
        let matches = find_regex_matches_in_line("a needle in a haystack", &regex, false);
        assert_eq!(matches, vec![(2, 8)]);
    }

    #[test]
    fn 정규식_매치도_단어_경계_옵션을_적용한다() {
        let regex = RegexBuilder::new("cat").case_insensitive(true).build().unwrap();
        let matches = find_regex_matches_in_line("cat concatenate cat_food cat", &regex, true);
        assert_eq!(matches, vec![(0, 3), (25, 28)]);
    }

    #[test]
    fn case_sensitive_옵션이_정규식_대소문자_구분에_반영된다() {
        let mut q = query("HELLO");
        q.regex = true;
        q.case_sensitive = true;
        let regex = compile_regex(&q).unwrap();
        assert!(!regex.is_match("hello"));
        assert!(regex.is_match("HELLO"));
    }

    #[test]
    fn case_sensitive가_아니면_정규식도_대소문자를_구분하지_않는다() {
        let mut q = query("HELLO");
        q.regex = true;
        let regex = compile_regex(&q).unwrap();
        assert!(regex.is_match("hello"));
    }

    #[test]
    fn 잘못된_정규식은_패닉_대신_에러를_반환한다() {
        let mut q = query("(unterminated");
        q.regex = true;
        let cancelled = AtomicBool::new(false);
        let result = search(Path::new("."), &q, &cancelled, |_| {});
        assert_eq!(result.unwrap_err().kind(), AppErrorKind::InvalidArgument);
    }

    #[test]
    fn 리터럴_검색은_정규식_경로와_무관하게_그대로_동작한다() {
        let matches = find_matches_in_line("aaaa", &query("aa"));
        assert_eq!(matches, vec![(0, 2), (2, 4)]);
    }

    struct Fixture {
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.root).ok();
        }
    }

    fn build_fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("taide-search-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("src").join("main.rs"), "fn main() {\n    println!(\"needle here\");\n}\n").unwrap();
        std::fs::write(root.join("node_modules").join("ignored.rs"), "needle").unwrap();
        std::fs::write(root.join("binary.bin"), [0x00u8, 0x01, 0x02, b'n', b'e', b'e', b'd', b'l', b'e']).unwrap();
        Fixture { root }
    }

    #[test]
    fn 무시_디렉토리와_바이너리_파일은_검색에서_제외된다() {
        let fixture = build_fixture();
        let cancelled = AtomicBool::new(false);

        let (total, results) = collect_search(&fixture.root, &query("needle"), &cancelled);

        assert_eq!(total, 1);
        assert_eq!(results.len(), 1);
        assert!(results[0].0.ends_with("main.rs"));
        assert_eq!(results[0].1.line, 2);
    }

    #[test]
    fn ascii_매치의_컬럼은_1부터_시작하는_utf16_코드유닛이다() {
        let fixture = build_fixture();
        let cancelled = AtomicBool::new(false);

        let (_, results) = collect_search(&fixture.root, &query("needle"), &cancelled);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].1.column, 15,
            "'    println!(\"' 는 14바이트(=14 UTF-16 코드유닛)이므로 1-based 컬럼은 15여야 한다"
        );
    }

    fn build_korean_fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("taide-search-korean-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("korean.txt"), "한글 needle 뒤\n").unwrap();
        Fixture { root }
    }

    #[test]
    fn 한글_뒤_매치는_바이트가_아닌_utf16_코드유닛_기준으로_컬럼을_계산한다() {
        let fixture = build_korean_fixture();
        let cancelled = AtomicBool::new(false);

        let (_, results) = collect_search(&fixture.root, &query("needle"), &cancelled);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].1.column, 4,
            "'한글 '은 UTF-16 코드유닛 3개(한·글·공백)이므로 1-based 컬럼은 4여야 한다 (바이트 기준이면 8이 되어 어긋난다: 한/글 각 3바이트+공백 1바이트=7, +1=8)"
        );
    }

    #[test]
    fn 파일_트리에서_정규식_검색이_동작한다() {
        let fixture = build_fixture();
        let mut q = query("n.+dle");
        q.regex = true;
        let cancelled = AtomicBool::new(false);

        let (total, results) = collect_search(&fixture.root, &q, &cancelled);

        assert_eq!(total, 1);
        assert!(results[0].0.ends_with("main.rs"));
        assert_eq!(results[0].1.line, 2);
    }

    #[test]
    fn 취소_플래그가_설정되면_검색을_중단한다() {
        let fixture = build_fixture();
        let cancelled = AtomicBool::new(true);

        let (total, results) = collect_search(&fixture.root, &query("needle"), &cancelled);

        assert_eq!(total, 0);
        assert!(results.is_empty());
    }

    #[test]
    fn context_before는_시작_경계에서_잘린다() {
        let lines = ["a", "b", "c"];
        assert_eq!(context_before(&lines, 0, 2), Vec::<String>::new());
        assert_eq!(context_before(&lines, 2, 2), vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn context_after는_끝_경계에서_잘린다() {
        let lines = ["a", "b", "c"];
        assert_eq!(context_after(&lines, 2, 2), Vec::<String>::new());
        assert_eq!(context_after(&lines, 0, 2), vec!["b".to_string(), "c".to_string()]);
    }

    #[test]
    fn 컨텍스트_줄이_0이면_전후_줄이_비어있다() {
        let fixture = build_fixture();
        let cancelled = AtomicBool::new(false);

        let (_, results) = collect_search(&fixture.root, &query("needle"), &cancelled);

        assert!(results[0].1.before.is_empty());
        assert!(results[0].1.after.is_empty());
    }

    #[test]
    fn 컨텍스트_줄_옵션은_전후_줄을_채우고_파일_경계에서_잘린다() {
        let fixture = build_fixture();
        let mut q = query("needle");
        q.context_lines = 2;
        let cancelled = AtomicBool::new(false);

        let (_, results) = collect_search(&fixture.root, &q, &cancelled);

        assert_eq!(results[0].1.before, vec!["fn main() {".to_string()]);
        assert_eq!(results[0].1.after, vec!["}".to_string()]);
    }

    fn build_gitignore_fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("taide-search-gitignore-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("ignored-by-git")).unwrap();
        std::fs::write(root.join(".gitignore"), "ignored-by-git/\n").unwrap();
        std::fs::write(root.join("kept.rs"), "needle").unwrap();
        std::fs::write(root.join("ignored-by-git").join("skip.rs"), "needle").unwrap();
        Fixture { root }
    }

    #[test]
    fn respect_gitignore_기본값은_gitignore된_파일을_제외한다() {
        let fixture = build_gitignore_fixture();
        let cancelled = AtomicBool::new(false);

        let (total, results) = collect_search(&fixture.root, &query("needle"), &cancelled);

        assert_eq!(total, 1);
        assert!(results[0].0.ends_with("kept.rs"));
    }

    #[test]
    fn respect_gitignore가_꺼지면_gitignore된_파일도_포함한다() {
        let fixture = build_gitignore_fixture();
        let mut q = query("needle");
        q.respect_gitignore = false;
        let cancelled = AtomicBool::new(false);

        let (total, _) = collect_search(&fixture.root, &q, &cancelled);

        assert_eq!(total, 2);
    }

    #[test]
    fn respect_gitignore가_꺼져도_ignored_dir_names는_항상_제외된다() {
        let fixture = build_fixture();
        let mut q = query("needle");
        q.respect_gitignore = false;
        let cancelled = AtomicBool::new(false);

        let (total, results) = collect_search(&fixture.root, &q, &cancelled);

        assert_eq!(total, 1);
        assert!(results[0].0.ends_with("main.rs"));
    }

    /// Reproduces the d-42 quick-open gap (contract §3, item d) at the layer that actually owns the
    /// bug: before this function existed, the palette's only file source was `tree_rows`, whose
    /// `TreeStore` only ever holds entries for directories a caller already `tree_toggle`'d open —
    /// `src/main.rs` here stands in for a file sitting in a folder the sidebar tree has never been
    /// expanded into, which `tree_rows` alone can never surface no matter how the palette filters
    /// it. A full walk finds it unconditionally.
    #[test]
    fn 트리에서_한_번도_확장되지_않은_하위_폴더의_파일도_찾는다() {
        let fixture = build_fixture();

        let files = list_project_files(&fixture.root);

        assert!(files.iter().any(|path| path.ends_with("src/main.rs")), "found: {files:?}");
    }

    #[test]
    fn ignored_dir_names_디렉토리는_gitignore_설정과_무관하게_항상_제외된다() {
        let fixture = build_fixture();

        let files = list_project_files(&fixture.root);

        assert!(!files
            .iter()
            .any(|path| path.components().any(|component| component.as_os_str() == "node_modules")));
    }

    #[test]
    fn 바이너리_파일도_목록에_포함된다_검색과_달리_내용을_읽지_않는다() {
        let fixture = build_fixture();

        let files = list_project_files(&fixture.root);

        assert!(files.iter().any(|path| path.ends_with("binary.bin")));
    }

    /// `respect_gitignore: false` is deliberate (see `list_project_files`'s doc comment) — a file
    /// still visible in the Explorer tree (which never applies `.gitignore`) must stay
    /// quick-open-able, unlike a `search`/`search_replace` call with its default `respect_gitignore:
    /// true`.
    #[test]
    fn gitignore된_파일도_트리와_동일하게_포함한다() {
        let fixture = build_gitignore_fixture();

        let files = list_project_files(&fixture.root);

        assert!(files.iter().any(|path| path.ends_with("ignored-by-git/skip.rs")));
        assert!(files.iter().any(|path| path.ends_with("kept.rs")));
    }

    fn replace_temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("taide-replace-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// Mirrors `search_replace`'s command-layer orchestration (`resolve_replace_targets` once,
    /// then `replace_one_file` per target) — the decomposition production actually calls, since
    /// there is no longer a single `replace` entry point to test against.
    fn replace(root: &Path, query: &SearchQuery, replacement: &str, paths: Option<&[PathBuf]>) -> AppResult<SearchReplaceResult> {
        let compiled = compile_query(query)?;
        let target_files = resolve_replace_targets(root, query, paths);

        let mut changed_files = 0u32;
        let mut replaced_matches = 0u32;
        let mut skipped: Vec<ReplaceSkippedFile> = Vec::new();
        let mut skipped_count = 0u32;

        for path in &target_files {
            match replace_one_file(path, &compiled, replacement) {
                ReplaceOutcome::Replaced(count) => {
                    changed_files += 1;
                    replaced_matches += count;
                }
                ReplaceOutcome::NoMatch => {}
                ReplaceOutcome::Skipped(reason) => {
                    skipped_count += 1;
                    if skipped.len() < REPLACE_SKIP_REPORT_LIMIT {
                        skipped.push(ReplaceSkippedFile {
                            path: path.to_string_lossy().to_string(),
                            reason,
                        });
                    }
                }
            }
        }

        Ok(SearchReplaceResult {
            changed_files,
            replaced_matches,
            skipped,
            skipped_count,
        })
    }

    #[test]
    fn 단순_치환은_모든_일치를_바꾸고_파일에_반영한다() {
        let root = replace_temp_root("simple");
        let file = root.join("a.txt");
        std::fs::write(&file, "needle one\nneedle two\n").unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 2);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "found one\nfound two\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 정규식_치환이_동작한다() {
        let root = replace_temp_root("regex");
        let file = root.join("a.txt");
        std::fs::write(&file, "cat1 cat2 dog\n").unwrap();

        let mut q = query("cat[0-9]");
        q.regex = true;
        let result = replace(&root, &q, "pet", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 2);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "pet pet dog\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn case_sensitive_옵션이_치환_대상을_구분한다() {
        let root = replace_temp_root("case-sensitive");
        let file = root.join("a.txt");
        std::fs::write(&file, "Cat cat CAT\n").unwrap();

        let mut q = query("cat");
        q.case_sensitive = true;
        let result = replace(&root, &q, "dog", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 1);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "Cat dog CAT\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 일치하는_항목이_없으면_0을_반환하고_파일을_바꾸지_않는다() {
        let root = replace_temp_root("no-match");
        let file = root.join("a.txt");
        std::fs::write(&file, "no match here\n").unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.changed_files, 0);
        assert_eq!(result.replaced_matches, 0);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "no match here\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 글롭_필터가_치환_대상_파일을_제한한다() {
        let root = replace_temp_root("glob");
        let included = root.join("keep.rs");
        let excluded = root.join("skip.txt");
        std::fs::write(&included, "needle\n").unwrap();
        std::fs::write(&excluded, "needle\n").unwrap();

        let mut q = query("needle");
        q.include_glob = Some("*.rs".to_string());
        let result = replace(&root, &q, "found", None).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 1);
        assert_eq!(std::fs::read_to_string(&included).unwrap(), "found\n");
        assert_eq!(std::fs::read_to_string(&excluded).unwrap(), "needle\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn paths_가_주어지면_해당_파일만_치환한다() {
        let root = replace_temp_root("explicit-paths");
        let target = root.join("target.txt");
        let other = root.join("other.txt");
        std::fs::write(&target, "needle\n").unwrap();
        std::fs::write(&other, "needle\n").unwrap();

        let result = replace(&root, &query("needle"), "found", Some(std::slice::from_ref(&target))).unwrap();

        assert_eq!(result.changed_files, 1);
        assert_eq!(result.replaced_matches, 1);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "found\n");
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "needle\n");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 바이너리_파일은_치환하지_않는다() {
        let root = replace_temp_root("binary");
        let file = root.join("binary.bin");
        std::fs::write(&file, [0x00u8, 0x01, b'n', b'e', b'e', b'd', b'l', b'e']).unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.changed_files, 0);
        assert_eq!(result.replaced_matches, 0);

        std::fs::remove_dir_all(&root).ok();
    }

    /// Creates a file whose *metadata* reports `len` bytes without writing that many — `set_len`
    /// leaves a sparse file on every filesystem TAIDE targets, so the size-ceiling tests below cost
    /// no disk and no time.
    fn write_sparse_file(path: &Path, len: u64) {
        let file = std::fs::File::create(path).unwrap();
        file.set_len(len).unwrap();
    }

    #[test]
    fn 크기_상한_이상인_파일은_읽지_않고_거절한다() {
        let root = replace_temp_root("size-cap");
        let file = root.join("huge.log");
        write_sparse_file(&file, REFUSED_FILE_BYTES);

        let outcome = read_scannable_bytes(&file);

        assert_eq!(
            outcome.unwrap_err(),
            ReplaceSkipReason::TooLarge,
            "상한 판정은 내용 읽기(바이너리 sniff)보다 먼저 일어나야 한다 — 전량 read 후 판별하면 GB 급 파일이 통째로 메모리에 올라온다"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 크기_상한_이상인_파일은_검색_대상에서_제외된다() {
        let root = replace_temp_root("size-cap-search");
        write_sparse_file(&root.join("huge.log"), REFUSED_FILE_BYTES);
        std::fs::write(root.join("small.txt"), "needle\n").unwrap();
        let cancelled = AtomicBool::new(false);

        let (total, results) = collect_search(&root, &query("needle"), &cancelled);

        assert_eq!(total, 1);
        assert!(results[0].0.ends_with("small.txt"));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 크기_상한_이상인_파일의_치환은_스킵_사유와_함께_보고된다() {
        let root = replace_temp_root("size-cap-replace");
        let huge = root.join("huge.log");
        write_sparse_file(&huge, REFUSED_FILE_BYTES);

        let result = replace(&root, &query("needle"), "found", Some(std::slice::from_ref(&huge))).unwrap();

        assert_eq!(result.changed_files, 0);
        assert_eq!(result.skipped_count, 1);
        assert_eq!(result.skipped[0].reason, ReplaceSkipReason::TooLarge);
        assert!(result.skipped[0].path.ends_with("huge.log"));

        std::fs::remove_dir_all(&root).ok();
    }

    /// C10 회귀: 치환이 실패한 파일이 "매치 없음"과 구분되지 않으면, 전부 거절된 치환도
    /// "0개 변경"으로만 보고돼 사용자가 실패를 알 수 없다.
    #[test]
    fn 치환_스킵은_사유별로_구분해_보고한다() {
        let root = replace_temp_root("skip-reasons");
        let binary = root.join("binary.bin");
        let invalid_utf8 = root.join("euc-kr.txt");
        std::fs::write(&binary, [0x00u8, b'n', b'e', b'e', b'd', b'l', b'e']).unwrap();
        std::fs::write(&invalid_utf8, [0xB0u8, 0xA1, b'n', b'e', b'e', b'd', b'l', b'e']).unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.changed_files, 0);
        assert_eq!(result.skipped_count, 2);
        let reason_for = |suffix: &str| {
            result
                .skipped
                .iter()
                .find(|entry| entry.path.ends_with(suffix))
                .map(|entry| entry.reason)
        };
        assert_eq!(reason_for("binary.bin"), Some(ReplaceSkipReason::Binary));
        assert_eq!(reason_for("euc-kr.txt"), Some(ReplaceSkipReason::NotUtf8));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 매치가_없는_파일은_스킵으로_세지_않는다() {
        let root = replace_temp_root("no-match-not-skip");
        std::fs::write(root.join("a.txt"), "no match here\n").unwrap();

        let result = replace(&root, &query("needle"), "found", None).unwrap();

        assert_eq!(result.skipped_count, 0);
        assert!(result.skipped.is_empty());

        std::fs::remove_dir_all(&root).ok();
    }

    /// H-2 회귀: 예전 `search_file` 은 취소 플래그를 전혀 읽지 않아, 파일 하나가 수백만 줄이면
    /// 이미 취소된 검색도 그 파일을 끝까지 스캔했다.
    #[test]
    fn 파일_내_라인_루프도_취소_플래그를_확인한다() {
        let root = replace_temp_root("cancel-in-file");
        let file = root.join("many-lines.txt");
        std::fs::write(&file, "needle\n".repeat(4_000)).unwrap();
        let cancelled = AtomicBool::new(true);
        let matcher = LiteralMatcher::new(&query("needle"));

        let found = search_file(
            &file,
            &query("needle"),
            &MatchMode::Literal(&matcher),
            SEARCH_MATCH_LIMIT,
            &cancelled,
        );

        assert!(found.is_empty(), "취소된 검색은 파일 첫 줄에서 즉시 중단해야 한다");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 매치는_파일_단위_배치로_전달되고_파일_내_순서를_보존한다() {
        let root = replace_temp_root("batching");
        std::fs::write(root.join("a.txt"), "needle\nx\nneedle\nneedle\n").unwrap();
        std::fs::write(root.join("b.txt"), "needle\n").unwrap();
        let cancelled = AtomicBool::new(false);
        let mut batches = Vec::new();

        let total = search(&root, &query("needle"), &cancelled, |batch| batches.push(batch)).unwrap();

        assert_eq!(total, 4);
        assert_eq!(batches.len(), 2, "파일 하나당 배치 하나로 전달돼야 한다");
        let a = batches.iter().find(|batch| batch.path.ends_with("a.txt")).unwrap();
        assert_eq!(a.matches.iter().map(|item| item.line).collect::<Vec<_>>(), vec![1, 3, 4]);

        std::fs::remove_dir_all(&root).ok();
    }

    /// 병렬 워커가 각자 상한을 세면 총합이 상한을 넘는다 — 예산은 원자적으로 배분돼야 한다.
    #[test]
    fn 병렬_스캔에서도_매치_상한을_정확히_지킨다() {
        let root = replace_temp_root("match-limit");
        let per_file = SEARCH_MATCH_LIMIT as usize * 2 / 3;
        for name in ["a.txt", "b.txt", "c.txt"] {
            std::fs::write(root.join(name), "needle\n".repeat(per_file)).unwrap();
        }
        let cancelled = AtomicBool::new(false);

        let (total, results) = collect_search(&root, &query("needle"), &cancelled);

        assert_eq!(total, SEARCH_MATCH_LIMIT);
        assert_eq!(results.len() as u32, SEARCH_MATCH_LIMIT);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn 대소문자_무시_매칭은_ascii만_접는다_비ascii는_그대로_비교한다() {
        assert_eq!(find_matches_in_line("Ünïcode ünïcode", &query("Ünïcode")), vec![(0, 9)]);
        assert_eq!(find_matches_in_line("ABC abc", &query("abc")), vec![(0, 3), (4, 7)]);
    }

    #[test]
    fn 멀티바이트_문자_뒤_매치도_바이트_경계를_어긋나지_않는다() {
        let matches = find_matches_in_line("한글 needle 뒤", &query("needle"));
        assert_eq!(matches, vec![(7, 13)], "'한글 '은 7바이트이므로 매치는 바이트 7에서 시작한다");
    }
}
