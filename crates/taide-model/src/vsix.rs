use serde::{Deserialize, Serialize};
use specta::Type;

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
