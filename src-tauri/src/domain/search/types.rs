use serde::{Deserialize, Serialize};
use specta::Type;

pub const SEARCH_MATCH_LIMIT: u32 = 10_000;

/// Default number of lines shown before/after a match when a caller omits
/// `contextLines` — zero preserves the pre-existing (context-free) preview
/// behavior for every caller that predates this field (the quick search
/// panel), so only callers that explicitly opt in (Search Editor) pay for
/// the extra lines. See `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4.
pub const SEARCH_CONTEXT_LINES: u32 = 0;

fn default_search_context_lines() -> u32 {
    SEARCH_CONTEXT_LINES
}

/// `.gitignore` recognition defaults to on — matching VS Code's
/// `search.useIgnoreFiles` default — so callers that omit the field (again,
/// every caller that predates this field) pick up gitignore-aware search for
/// free. `constants::IGNORED_DIR_NAMES` is applied regardless of this flag
/// (see `search::service::build_walk`).
fn default_respect_gitignore() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub text: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub whole_word: bool,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub include_glob: Option<String>,
    #[serde(default)]
    pub exclude_glob: Option<String>,
    #[serde(default = "default_search_context_lines")]
    pub context_lines: u32,
    #[serde(default = "default_respect_gitignore")]
    pub respect_gitignore: bool,
}

/// How many individual skipped files `search_replace` names in its result before it stops
/// listing them (the running `skipped_count` keeps counting past this point). A project-wide
/// replace can legitimately walk past thousands of files it must refuse — every binary asset,
/// every file above `constants::REFUSED_FILE_BYTES` — and shipping all of them back would turn a
/// "3 files skipped" notice into a multi-megabyte payload. The cap only bounds the *listing*, so
/// the frontend can still say "N skipped" truthfully and show the first few paths.
pub const REPLACE_SKIP_REPORT_LIMIT: usize = 50;

/// Why one file `search_replace` was asked to rewrite came back unchanged even though it was in
/// the resolved target set. Before this existed the whole class was a silent `None` return, so a
/// replace that touched nothing (unreadable file, non-UTF-8 encoding, oversized file) reported the
/// same "0 files changed" as a replace whose query simply had no matches — audit §4-B C10.
/// A file that was read fine and just had no match is *not* a skip and never appears here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ReplaceSkipReason {
    /// At or above `constants::REFUSED_FILE_BYTES` — the same ceiling `domain::file` refuses to
    /// open a file at, so replace never buffers a file the editor itself would not load.
    TooLarge,
    /// A NUL byte inside the first `BINARY_SNIFF_BYTES` — rewriting it would corrupt it.
    Binary,
    /// Not valid UTF-8. Replace (unlike search) requires strict UTF-8, since a lossy decode would
    /// write `U+FFFD` back over the original bytes.
    NotUtf8,
    /// `metadata`/`open`/`read` failed — permissions, a file deleted mid-walk, an I/O error.
    Unreadable,
    /// Matches were found and rewritten in memory, but the atomic write back to disk failed.
    WriteFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceSkippedFile {
    pub path: String,
    pub reason: ReplaceSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchReplaceResult {
    pub changed_files: u32,
    pub replaced_matches: u32,
    /// The first [`REPLACE_SKIP_REPORT_LIMIT`] skipped files, in target-walk order.
    pub skipped: Vec<ReplaceSkippedFile>,
    /// Every skipped file, including the ones past the listing cap — `skipped.len()` when nothing
    /// was truncated.
    pub skipped_count: u32,
}

/// One match inside a file. Carries no `path`: matches reach the frontend grouped per file inside
/// [`SearchFileMatches`], so the path is sent once per file instead of once per match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchLineMatch {
    pub line: u32,
    /// 1-based, in UTF-16 code units — Monaco `Position.column`'s own convention, so the frontend
    /// can hand this straight to `reveal-registry.ts` without converting. Computed from the UTF-8
    /// byte offset `search::service::search_file` matches at via
    /// `search::service::byte_offset_to_utf16_units`.
    pub column: u32,
    pub preview: String,
    /// 0-based UTF-16 code unit offsets into `preview` — `preview` is consumed as a JS string via
    /// `preview.slice(matchStart, matchEnd)`, which indexes by UTF-16 code unit, not UTF-8 byte.
    pub match_start: u32,
    pub match_end: u32,
    /// Up to `SearchQuery::context_lines` lines immediately before the match
    /// line, in file order. Empty when `context_lines` is `0`.
    pub before: Vec<String>,
    /// Up to `SearchQuery::context_lines` lines immediately after the match
    /// line, in file order. Empty when `context_lines` is `0`.
    pub after: Vec<String>,
}

/// Every match `search_run` found in one file — the unit its `onMatch` channel actually carries.
/// One channel message per *file*, not per match (audit §2 M-9): a 10,000-match run used to cost
/// 10,000 IPC messages, each re-serializing the same `path`, and made the frontend re-group the
/// whole accumulated list on every one of them (§1-3).
///
/// `matches` is always in ascending source order (line, then column) — the order the file was
/// scanned in — so the frontend's context-line de-duplication can rely on it. The order the
/// *files* themselves arrive in is deliberately unspecified: the walk is parallel
/// (`search::service::search`), so file arrival order varies run to run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchFileMatches {
    pub path: String,
    pub matches: Vec<SearchLineMatch>,
}
