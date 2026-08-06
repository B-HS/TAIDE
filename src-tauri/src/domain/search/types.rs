use serde::{Deserialize, Serialize};
use specta::Type;

pub const SEARCH_MATCH_LIMIT: u32 = 10_000;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub preview: String,
    pub match_start: u32,
    pub match_end: u32,
}
