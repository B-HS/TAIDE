use serde::{Deserialize, Serialize};
use specta::Type;

pub const VSIX_EXTENSION_ROOT: &str = "extension";
pub const VSIX_MANIFEST_ENTRY: &str = "extension/package.json";
pub const VSIX_ENTRY_MAX_BYTES: u64 = 2 * 1024 * 1024;
pub const VSIX_INCLUDE_CHAIN_MAX_DEPTH: usize = 5;
pub const VSIX_MAX_THEME_CONTRIBUTIONS: usize = 64;
pub const VSIX_TOTAL_MAX_EXTRACTED_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VsixExtensionInfo {
    pub name: String,
    pub display_name: String,
    pub publisher: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VsixThemeIncludeEntry {
    pub path: String,
    pub raw_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VsixExtractedTheme {
    pub label: String,
    pub ui_theme: String,
    pub raw_json: String,
    pub include_chain: Vec<VsixThemeIncludeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VsixThemeExtractionResult {
    pub extension: VsixExtensionInfo,
    pub themes: Vec<VsixExtractedTheme>,
}
