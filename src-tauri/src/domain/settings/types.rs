use serde::{Deserialize, Serialize};
use specta::Type;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_THEME_ID: &str = "taide-dark";
pub const DEFAULT_EDITOR_FONT_SIZE: u32 = 13;
pub const DEFAULT_TERMINAL_FONT_SIZE: u32 = 13;
pub const DEFAULT_LANGUAGE: &str = "system";
pub const DEFAULT_TOAST_POSITION: &str = "bottom-right";
pub const DEFAULT_RESIZER_THICKNESS: u32 = 1;

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
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_toast_position")]
    pub toast_position: String,
    #[serde(default = "default_resizer_thickness")]
    pub resizer_thickness: u32,
    #[serde(default)]
    pub editor_font_family: Option<String>,
    #[serde(default)]
    pub terminal_font_family: Option<String>,
    #[serde(default)]
    pub ui_font_family: Option<String>,
    #[serde(default)]
    pub format_on_save: bool,
    #[serde(default)]
    pub auto_save_delay_ms: u32,
    #[serde(default)]
    pub keymap_overrides: Option<String>,
    #[serde(default = "default_true")]
    pub editor_minimap: bool,
    #[serde(default = "default_true")]
    pub show_system_usage: bool,
    #[serde(default = "default_true")]
    pub agent_status_badge_enabled: bool,
    #[serde(default)]
    pub agent_hooks_enabled: bool,
    #[serde(default = "default_true")]
    pub ide_integration_enabled: bool,
    #[serde(default = "default_true")]
    pub ide_auto_open_diff: bool,
}

fn default_theme_id() -> String {
    DEFAULT_THEME_ID.to_string()
}

fn default_language() -> String {
    DEFAULT_LANGUAGE.to_string()
}

fn default_toast_position() -> String {
    DEFAULT_TOAST_POSITION.to_string()
}

fn default_resizer_thickness() -> u32 {
    DEFAULT_RESIZER_THICKNESS
}

fn default_editor_font_size() -> u32 {
    DEFAULT_EDITOR_FONT_SIZE
}

fn default_true() -> bool {
    true
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
            language: default_language(),
            toast_position: default_toast_position(),
            resizer_thickness: default_resizer_thickness(),
            editor_font_family: None,
            terminal_font_family: None,
            ui_font_family: None,
            format_on_save: false,
            auto_save_delay_ms: 0,
            keymap_overrides: None,
            editor_minimap: default_true(),
            show_system_usage: default_true(),
            agent_status_badge_enabled: default_true(),
            agent_hooks_enabled: false,
            ide_integration_enabled: default_true(),
            ide_auto_open_diff: default_true(),
        }
    }
}
