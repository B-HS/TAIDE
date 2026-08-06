use serde::{Deserialize, Serialize};
use specta::Type;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_THEME_ID: &str = "taide-dark";
pub const DEFAULT_EDITOR_FONT_SIZE: u32 = 13;
pub const DEFAULT_TERMINAL_FONT_SIZE: u32 = 13;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub version: u32,
    #[serde(default = "default_theme_id")]
    pub theme_id: String,
    #[serde(default = "default_editor_font_size")]
    pub editor_font_size: u32,
    #[serde(default = "default_terminal_font_size")]
    pub terminal_font_size: u32,
    #[serde(default)]
    pub shell_override: Option<String>,
    #[serde(default)]
    pub follow_system_theme: bool,
}

fn default_theme_id() -> String {
    DEFAULT_THEME_ID.to_string()
}

fn default_editor_font_size() -> u32 {
    DEFAULT_EDITOR_FONT_SIZE
}

fn default_terminal_font_size() -> u32 {
    DEFAULT_TERMINAL_FONT_SIZE
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_SCHEMA_VERSION,
            theme_id: default_theme_id(),
            editor_font_size: default_editor_font_size(),
            terminal_font_size: default_terminal_font_size(),
            shell_override: None,
            follow_system_theme: false,
        }
    }
}
