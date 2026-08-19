use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::ai::types::AiProviderId;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_THEME_ID: &str = "taide-dark";
pub const DEFAULT_EDITOR_FONT_SIZE: u32 = 13;
pub const DEFAULT_TERMINAL_FONT_SIZE: u32 = 13;
pub const DEFAULT_LANGUAGE: &str = "system";
pub const DEFAULT_TOAST_POSITION: &str = "bottom-right";
pub const DEFAULT_RESIZER_THICKNESS: u32 = 1;
pub const DEFAULT_EDITOR_TAB_SIZE: u32 = 4;
pub const DEFAULT_TERMINAL_SCROLLBACK: u32 = 10_000;

/// `Settings.editorRenderWhitespace` / `SettingsPatch.editorRenderWhitespace`'s value set — mirrors
/// `EditorRenderWhitespace` (`src/features/editor/code-editor.tsx`), the Monaco `renderWhitespace`
/// option's own literal union. Was a bare `Option<String>` (audit
/// `docs/quality-assurance/2026-08-18-architecture-audit.md` C6/T1-B), which forced 9 frontend `as`
/// casts and let a hand-edited `settings.json`/synced gist carry an arbitrary string. Narrowing to a
/// real enum makes specta emit the same `"none" | "boundary" | "selection" | "all"` union the
/// frontend already hand-declared, so those casts collapse to a plain type. A legacy/out-of-range
/// value is normalized back to the default *before* typed parse — see
/// [`crate::domain::settings::service::sanitize_legacy_settings_values`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum EditorRenderWhitespace {
    None,
    Boundary,
    #[default]
    Selection,
    All,
}

/// `Settings.editorCursorStyle` / `SettingsPatch.editorCursorStyle`'s value set — mirrors
/// `EditorCursorStyle` (`src/features/editor/code-editor.tsx`), Monaco's `cursorStyle` option. See
/// [`EditorRenderWhitespace`]'s doc comment for why this is a narrowed enum rather than `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum EditorCursorStyle {
    #[default]
    Line,
    Block,
    Underline,
}

/// `Settings.editorCursorBlinking` / `SettingsPatch.editorCursorBlinking`'s value set — mirrors
/// `EditorCursorBlinkingStyle` (`src/features/editor/code-editor.tsx`), Monaco's `cursorBlinking`
/// option. See [`EditorRenderWhitespace`]'s doc comment for why this is a narrowed enum rather than
/// `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum EditorCursorBlinking {
    #[default]
    Blink,
    Smooth,
    Phase,
    Expand,
    Solid,
}

/// `Settings.terminalCursorStyle` / `SettingsPatch.terminalCursorStyle`'s value set — mirrors
/// `TerminalCursorStyle` (`src/features/terminal/terminal-view.tsx`), xterm.js's `cursorStyle`
/// option. See [`EditorRenderWhitespace`]'s doc comment for why this is a narrowed enum rather than
/// `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum TerminalCursorStyle {
    #[default]
    Bar,
    Block,
    Underline,
}

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
    #[serde(default)]
    pub editor_render_whitespace: EditorRenderWhitespace,
    #[serde(default = "default_true")]
    pub editor_bracket_pair_colorization: bool,
    #[serde(default)]
    pub editor_font_ligatures: bool,
    #[serde(default)]
    pub editor_cursor_style: EditorCursorStyle,
    #[serde(default)]
    pub editor_cursor_blinking: EditorCursorBlinking,
    #[serde(default = "default_true")]
    pub editor_scroll_beyond_last_line: bool,
    #[serde(default = "default_true")]
    pub editor_sticky_scroll_enabled: bool,
    #[serde(default = "default_terminal_scrollback")]
    pub terminal_scrollback: u32,
    #[serde(default)]
    pub terminal_cursor_style: TerminalCursorStyle,
    #[serde(default = "default_true")]
    pub terminal_cursor_blink: bool,
    #[serde(default = "default_true")]
    pub enable_preview_tabs: bool,
    #[serde(default)]
    pub ai_auto_tab_enabled: bool,
    /// Renamed from `ai_auto_tab_provider` — this field now backs every AI feature's default
    /// provider (auto-tab, Inline Edit, AI commit messages), not just auto-tab, so the name was
    /// generalized. A pre-rename `settings.json`/synced gist payload (`aiAutoTabProvider`) is
    /// migrated to this field name by
    /// [`crate::domain::settings::service::migrate_legacy_ai_provider_keys`] before
    /// deserialization — deliberately *not* a `#[serde(alias = ...)]` on this field, since with
    /// the `specta` version this project pins, an alias makes `Settings`' Serialize and
    /// Deserialize shapes diverge, splitting every TS consumer of `Settings` (including ones
    /// unrelated to this field) into `_Serialize`/`_Deserialize`/union type variants. See
    /// `docs/acknowledge/2026-08-16-wave-g-ai-contract.md` §2-2/§3.1.
    ///
    /// `Option<AiProviderId>` (not a locally-declared enum) reuses the same type the `ai_*`
    /// commands already validate `provider` arguments against (`domain::ai::types::AiProviderId`),
    /// instead of `Option<String>` plus the hand-maintained `AI_PROVIDERS` allow-list `sanitize()`
    /// used to check it against — see [`EditorRenderWhitespace`]'s doc comment for why an
    /// out-of-range value is normalized before typed parse rather than accepted here as `None`.
    #[serde(default)]
    pub ai_provider: Option<AiProviderId>,
    /// Renamed from `ai_auto_tab_model` — see [`Settings::ai_provider`]'s doc comment.
    #[serde(default)]
    pub ai_model: Option<String>,
    #[serde(default)]
    pub ai_omlx_base_url: Option<String>,
    #[serde(default)]
    pub sync_gist_id: Option<String>,
    #[serde(default)]
    pub sync_last_synced_at: Option<String>,
    #[serde(default)]
    pub remote_access_enabled: bool,
    #[serde(default)]
    pub remote_password_only_login: bool,
    #[serde(default)]
    pub remote_allowed_hosts: Vec<String>,
    #[serde(default)]
    pub organize_imports_on_save: bool,
    #[serde(default)]
    pub fix_all_on_save: bool,
    #[serde(default = "default_true")]
    pub editor_code_lens_enabled: bool,
    #[serde(default = "default_true")]
    pub editor_semantic_highlighting: bool,
    #[serde(default)]
    pub editor_format_on_type: bool,
    #[serde(default)]
    pub editor_format_on_paste: bool,
    #[serde(default = "default_true")]
    pub emmet_enabled: bool,
    /// Most-recent-first search terms, newest at index `0`. The cap
    /// (currently 20) and dedup/prepend logic are the frontend's
    /// responsibility (`entities/search`) — this field is a plain passthrough,
    /// same as `remote_allowed_hosts`' list shape but without server-side
    /// validation, since search history carries no security weight. See
    /// `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.5.
    #[serde(default)]
    pub recent_searches: Vec<String>,
    /// When `true`, entering Zen mode also fullscreens the main window
    /// (`window::commands::window_set_fullscreen`). Defaults to `false` — Zen mode's chrome hiding
    /// is opt-out (`zen_hide_status_bar`), but fullscreen is opt-in, since it also affects the OS
    /// window itself, not just TAIDE's own chrome. See
    /// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.2.
    #[serde(default)]
    pub zen_fullscreen: bool,
    /// Whether Zen mode hides the status bar in addition to the sidebar/tab bar it always hides.
    /// Defaults to `true`.
    #[serde(default = "default_true")]
    pub zen_hide_status_bar: bool,
}

/// A partial `Settings` update — every field optional, merged over the current value by
/// `service::apply_patch` (`None` = leave unchanged). Also the shape a synced gist stores its
/// nested `settings` object as (`sync::types::SyncPayload`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub theme_id: Option<String>,
    pub editor_font_size: Option<u32>,
    pub terminal_font_size: Option<u32>,
    pub shell_override: Option<String>,
    pub follow_system_theme: Option<bool>,
    pub language: Option<String>,
    pub toast_position: Option<String>,
    pub resizer_thickness: Option<u32>,
    pub editor_font_family: Option<String>,
    pub terminal_font_family: Option<String>,
    pub ui_font_family: Option<String>,
    pub format_on_save: Option<bool>,
    pub auto_save_delay_ms: Option<u32>,
    pub keymap_overrides: Option<String>,
    pub editor_minimap: Option<bool>,
    pub show_system_usage: Option<bool>,
    pub agent_status_badge_enabled: Option<bool>,
    pub agent_hooks_enabled: Option<bool>,
    pub ide_integration_enabled: Option<bool>,
    pub ide_auto_open_diff: Option<bool>,
    pub editor_word_wrap: Option<bool>,
    pub editor_line_numbers: Option<bool>,
    pub editor_tab_size: Option<u32>,
    pub editor_insert_spaces: Option<bool>,
    pub editor_detect_indentation: Option<bool>,
    pub editor_render_whitespace: Option<EditorRenderWhitespace>,
    pub editor_bracket_pair_colorization: Option<bool>,
    pub editor_font_ligatures: Option<bool>,
    pub editor_cursor_style: Option<EditorCursorStyle>,
    pub editor_cursor_blinking: Option<EditorCursorBlinking>,
    pub editor_scroll_beyond_last_line: Option<bool>,
    pub editor_sticky_scroll_enabled: Option<bool>,
    pub terminal_scrollback: Option<u32>,
    pub terminal_cursor_style: Option<TerminalCursorStyle>,
    pub terminal_cursor_blink: Option<bool>,
    pub enable_preview_tabs: Option<bool>,
    pub ai_auto_tab_enabled: Option<bool>,
    pub ai_provider: Option<AiProviderId>,
    pub ai_model: Option<String>,
    pub ai_omlx_base_url: Option<String>,
    pub remote_access_enabled: Option<bool>,
    pub remote_password_only_login: Option<bool>,
    pub remote_allowed_hosts: Option<Vec<String>>,
    pub organize_imports_on_save: Option<bool>,
    pub fix_all_on_save: Option<bool>,
    pub editor_code_lens_enabled: Option<bool>,
    pub editor_semantic_highlighting: Option<bool>,
    pub editor_format_on_type: Option<bool>,
    pub editor_format_on_paste: Option<bool>,
    pub emmet_enabled: Option<bool>,
    pub recent_searches: Option<Vec<String>>,
    pub zen_fullscreen: Option<bool>,
    pub zen_hide_status_bar: Option<bool>,
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

fn default_terminal_scrollback() -> u32 {
    DEFAULT_TERMINAL_SCROLLBACK
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
            editor_render_whitespace: EditorRenderWhitespace::default(),
            editor_bracket_pair_colorization: default_true(),
            editor_font_ligatures: false,
            editor_cursor_style: EditorCursorStyle::default(),
            editor_cursor_blinking: EditorCursorBlinking::default(),
            editor_scroll_beyond_last_line: default_true(),
            editor_sticky_scroll_enabled: default_true(),
            terminal_scrollback: default_terminal_scrollback(),
            terminal_cursor_style: TerminalCursorStyle::default(),
            terminal_cursor_blink: default_true(),
            enable_preview_tabs: default_true(),
            ai_auto_tab_enabled: false,
            ai_provider: None,
            ai_model: None,
            ai_omlx_base_url: None,
            sync_gist_id: None,
            sync_last_synced_at: None,
            remote_access_enabled: false,
            remote_password_only_login: false,
            remote_allowed_hosts: Vec::new(),
            organize_imports_on_save: false,
            fix_all_on_save: false,
            editor_code_lens_enabled: default_true(),
            editor_semantic_highlighting: default_true(),
            editor_format_on_type: false,
            editor_format_on_paste: false,
            emmet_enabled: default_true(),
            recent_searches: Vec::new(),
            zen_fullscreen: false,
            zen_hide_status_bar: default_true(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use regex::Regex;

    use super::*;

    /// `R5#3` — the field set `Settings` actually serializes (read off `Default`'s real serde
    /// output, so a future `#[serde(rename = ...)]` override is honored rather than guessed from
    /// the Rust identifier) must match the field set the generated TS binding exposes. Every field
    /// here is `#[serde(default)]`, so a field added to one side and forgotten on the other never
    /// fails to deserialize — it just silently stops round-tripping. This test is the parity check
    /// that gap has been missing.
    ///
    /// Only the TS side is text-sliced (the Rust side above is real serde output, not source
    /// parsing): the block between `export type Settings = {` and its first following `};` — safe
    /// only because `Settings` is a flat field bag with no nested `{ ... }` object-typed field to
    /// contain an earlier `};` of its own. A future field whose TS type is an inline object literal
    /// would truncate this slice at that nested closer, silently dropping every field declared after
    /// it from `ts_fields` (`rust_fields` would then look like it grew fields the TS side lacks, not
    /// the reverse). See `docs/acknowledge/2026-08-18-audit-t1-batch1-contract.md` §1 T1-E.
    #[test]
    fn settings_필드_집합은_rust와_bindings_ts에서_일치한다() {
        let serialized = serde_json::to_value(Settings::default()).expect("Settings 직렬화 성공");
        let rust_fields: BTreeSet<String> = serialized
            .as_object()
            .expect("Settings 는 JSON 객체로 직렬화된다")
            .keys()
            .cloned()
            .collect();

        let bindings_source = include_str!("../../../../src/shared/api/bindings.ts");
        let start = bindings_source
            .find("export type Settings = {")
            .expect("bindings.ts 에서 Settings 타입 시작을 찾을 수 없습니다")
            + "export type Settings = {".len();
        let end = bindings_source[start..]
            .find("};")
            .expect("bindings.ts 에서 Settings 타입 끝을 찾을 수 없습니다");
        let block = &bindings_source[start..start + end];
        let pattern = Regex::new(r"(?m)^\s*([a-zA-Z_][a-zA-Z0-9_]*)\??:\s").expect("유효한 정규식");
        let ts_fields: BTreeSet<String> = pattern.captures_iter(block).map(|capture| capture[1].to_string()).collect();

        assert_eq!(
            rust_fields, ts_fields,
            "Rust Settings 필드 집합과 bindings.ts 의 Settings 타입 필드 집합이 다릅니다"
        );
    }
}
