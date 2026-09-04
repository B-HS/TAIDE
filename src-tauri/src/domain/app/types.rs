use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::ai::prompt;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub arch: String,
}

/// One duration slot's readout, milliseconds on the wire.
///
/// `infra::perf` accumulates nanoseconds in `u64`; specta forbids exporting 64-bit integers to
/// TypeScript (precision loss), so the conversion to `f64` milliseconds happens once in
/// `app::service::perf_snapshot` rather than at every reader.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PerfEntry {
    pub name: String,
    pub count: f64,
    pub total_ms: f64,
    pub max_ms: f64,
}

/// One accumulating counter's readout — bytes, chunks, or invocation counts, per the slot's
/// documented unit (`docs/debugging.md` §4.1). Counters carry no duration by design: they sit on
/// high-frequency paths where timing each occurrence would distort the measurement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PerfCounterEntry {
    pub name: String,
    pub total: f64,
}

/// `perf_snapshot`'s reply. `enabled` reports the `TAIDE_PERF` gate so a caller can tell "nothing
/// happened" from "instrumentation is off" — with the gate off every entry reads zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PerfSnapshot {
    pub enabled: bool,
    pub entries: Vec<PerfEntry>,
    pub counters: Vec<PerfCounterEntry>,
}

/// Closed set of prompt-template overrides editable through an `AppFile` tab — deliberately a real
/// enum (not a raw `String` id) so an invalid id can never reach `app::service::app_file_path` in
/// the first place; there is no runtime whitelist check to forget. Variant names are the
/// kebab-case-serialized wire form (`#[serde(rename_all = "kebab-case")]` below), matching
/// `ai::prompt`'s existing `*_PROMPT_ID` constants exactly — [`PromptTemplateId::as_str`] ties the
/// two together so the file name on disk and the id embedded in a tab never drift independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum PromptTemplateId {
    AutoTabDefault,
    InlineEditDefault,
    CommitMessageDefault,
}

impl PromptTemplateId {
    pub fn as_str(self) -> &'static str {
        match self {
            PromptTemplateId::AutoTabDefault => prompt::AUTO_TAB_PROMPT_ID,
            PromptTemplateId::InlineEditDefault => prompt::INLINE_EDIT_PROMPT_ID,
            PromptTemplateId::CommitMessageDefault => prompt::COMMIT_MESSAGE_PROMPT_ID,
        }
    }
}

/// Which app-owned config file an `AppFile` tab (or `app_file_read`/`app_file_write`) addresses.
/// `AppPaths`-derived on the Rust side only — neither variant carries a path, so the frontend and
/// the persisted layout JSON never see an absolute filesystem path for these tabs (contract §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppFileTarget {
    Settings,
    #[serde(rename_all = "camelCase")]
    Prompt {
        id: PromptTemplateId,
    },
}
