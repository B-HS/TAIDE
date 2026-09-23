use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use specta::Type;

pub const THEME_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ThemeType {
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyntaxStyle {
    pub fg: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TokenColorSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TokenColorRule {
    pub scope: Vec<String>,
    pub settings: TokenColorSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub version: u32,
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: ThemeType,
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default)]
    pub palette: BTreeMap<String, String>,
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
    #[serde(default)]
    pub syntax: BTreeMap<String, SyntaxStyle>,
    #[serde(default)]
    pub terminal: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_colors: Option<Vec<TokenColorRule>>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: ThemeType,
    pub builtin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTheme {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: ThemeType,
    pub colors: BTreeMap<String, String>,
    pub syntax: BTreeMap<String, SyntaxStyle>,
    pub terminal: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_colors: Option<Vec<TokenColorRule>>,
    #[serde(default)]
    pub syntax_overrides: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}
