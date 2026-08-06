use serde::{Deserialize, Serialize};
use specta::Type;

pub const PLUGIN_MANIFEST_VERSION: u32 = 1;
pub const PLUGIN_MANIFEST_FILE: &str = "taide-plugin.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginLanguageContribution {
    pub id: String,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub grammar: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginLspContribution {
    pub language_id: String,
    pub id: String,
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub detect: Vec<String>,
    #[serde(default)]
    pub shareable: bool,
    #[serde(default)]
    pub install_instructions: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributions {
    #[serde(default)]
    pub languages: Vec<PluginLanguageContribution>,
    #[serde(default)]
    pub lsp: Vec<PluginLspContribution>,
    #[serde(default)]
    pub themes: Vec<PluginThemeContribution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginThemeContribution {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub contributes: PluginContributions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub root: String,
    pub enabled: bool,
    #[serde(default)]
    pub error: Option<String>,
}
