use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

/// A VS Code snippet field that accepts either a single line or an array of
/// lines (`prefix`, `body`, `description` in the upstream schema all share
/// this shape). `#[serde(untagged)]` picks whichever variant matches the JSON
/// value on the wire, so callers don't need a discriminant field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(untagged)]
pub enum SnippetStringOrList {
    Single(String),
    Multiple(Vec<String>),
}

/// One named snippet entry, matching VS Code's `<languageId>.json` /
/// `*.code-snippets` schema. `scope` only has meaning inside a
/// `.code-snippets` file (a comma-separated `languageId` list); it is kept as
/// a plain optional field here and interpreted by the frontend completion
/// provider (`docs/acknowledge/2026-08-15-wave-f-editor-presentation-contract.md`
/// §3.3). `isFileTemplate`/`include`/`exclude` from the upstream schema are
/// intentionally not modeled — unknown JSON fields are dropped silently by
/// serde's default (non-`deny_unknown_fields`) behavior, matching the
/// contract's "1차 무시" decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnippetEntry {
    pub prefix: SnippetStringOrList,
    pub body: SnippetStringOrList,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<SnippetStringOrList>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// The on-disk shape of a single snippet file: JSON object keyed by snippet
/// name.
pub type SnippetMap = BTreeMap<String, SnippetEntry>;

/// One snippet file plus its parsed contents, as returned by `snippet_list`.
/// The frontend's completion provider derives the file's `languageId` from
/// `file_name` itself (`<languageId>.json` vs. global `*.code-snippets`) —
/// see the contract §3.3 — so no separate language field is carried here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnippetFile {
    pub file_name: String,
    pub snippets: SnippetMap,
}
