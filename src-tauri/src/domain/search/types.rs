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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchReplaceResult {
    pub changed_files: u32,
    pub replaced_matches: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub path: String,
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
