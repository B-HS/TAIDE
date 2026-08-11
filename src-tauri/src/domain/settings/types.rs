use serde::{Deserialize, Serialize};
use specta::Type;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_THEME_ID: &str = "taide-dark";
pub const DEFAULT_EDITOR_FONT_SIZE: u32 = 13;
pub const DEFAULT_TERMINAL_FONT_SIZE: u32 = 13;
pub const DEFAULT_LANGUAGE: &str = "system";
pub const DEFAULT_TOAST_POSITION: &str = "bottom-right";
pub const DEFAULT_RESIZER_THICKNESS: u32 = 1;
pub const DEFAULT_EDITOR_TAB_SIZE: u32 = 4;
pub const DEFAULT_EDITOR_CURSOR_STYLE: &str = "line";
pub const DEFAULT_EDITOR_CURSOR_BLINKING: &str = "blink";
pub const DEFAULT_EDITOR_RENDER_WHITESPACE: &str = "selection";
pub const DEFAULT_TERMINAL_SCROLLBACK: u32 = 10_000;
pub const DEFAULT_TERMINAL_CURSOR_STYLE: &str = "bar";

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
    #[serde(default)]
    pub editor_word_wrap: bool,
    #[serde(default = "default_true")]
    pub editor_line_numbers: bool,
    #[serde(default = "default_editor_tab_size")]
    pub editor_tab_size: u32,
    #[serde(default = "default_true")]
    pub editor_insert_spaces: bool,
    #[serde(default = "default_true")]
    pub editor_detect_indentation: bool,
    #[serde(default = "default_editor_render_whitespace")]
    pub editor_render_whitespace: String,
    #[serde(default = "default_true")]
    pub editor_bracket_pair_colorization: bool,
    #[serde(default)]
    pub editor_font_ligatures: bool,
    #[serde(default = "default_editor_cursor_style")]
    pub editor_cursor_style: String,
    #[serde(default = "default_editor_cursor_blinking")]
    pub editor_cursor_blinking: String,
    #[serde(default = "default_true")]
    pub editor_scroll_beyond_last_line: bool,
    #[serde(default = "default_terminal_scrollback")]
    pub terminal_scrollback: u32,
    #[serde(default = "default_terminal_cursor_style")]
    pub terminal_cursor_style: String,
    #[serde(default = "default_true")]
    pub terminal_cursor_blink: bool,
    #[serde(default = "default_true")]
    pub enable_preview_tabs: bool,
    #[serde(default)]
    pub ai_auto_tab_enabled: bool,
    #[serde(default)]
    pub ai_auto_tab_provider: Option<String>,
    #[serde(default)]
    pub ai_auto_tab_model: Option<String>,
    #[serde(default)]
    pub sync_gist_id: Option<String>,
    #[serde(default)]
    pub sync_last_synced_at: Option<String>,
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

fn default_editor_tab_size() -> u32 {
    DEFAULT_EDITOR_TAB_SIZE
}

fn default_editor_render_whitespace() -> String {
    DEFAULT_EDITOR_RENDER_WHITESPACE.to_string()
}

fn default_editor_cursor_style() -> String {
    DEFAULT_EDITOR_CURSOR_STYLE.to_string()
}

fn default_editor_cursor_blinking() -> String {
    DEFAULT_EDITOR_CURSOR_BLINKING.to_string()
}

fn default_terminal_scrollback() -> u32 {
    DEFAULT_TERMINAL_SCROLLBACK
}

fn default_terminal_cursor_style() -> String {
    DEFAULT_TERMINAL_CURSOR_STYLE.to_string()
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
            editor_word_wrap: false,
            editor_line_numbers: default_true(),
            editor_tab_size: default_editor_tab_size(),
            editor_insert_spaces: default_true(),
            editor_detect_indentation: default_true(),
            editor_render_whitespace: default_editor_render_whitespace(),
            editor_bracket_pair_colorization: default_true(),
            editor_font_ligatures: false,
            editor_cursor_style: default_editor_cursor_style(),
            editor_cursor_blinking: default_editor_cursor_blinking(),
            editor_scroll_beyond_last_line: default_true(),
            terminal_scrollback: default_terminal_scrollback(),
            terminal_cursor_style: default_terminal_cursor_style(),
            terminal_cursor_blink: default_true(),
            enable_preview_tabs: default_true(),
            ai_auto_tab_enabled: false,
            ai_auto_tab_provider: None,
            ai_auto_tab_model: None,
            sync_gist_id: None,
            sync_last_synced_at: None,
        }
    }
}
