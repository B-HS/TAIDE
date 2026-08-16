use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::settings::types::{
    Settings, DEFAULT_EDITOR_CURSOR_BLINKING, DEFAULT_EDITOR_CURSOR_STYLE, DEFAULT_EDITOR_RENDER_WHITESPACE, DEFAULT_TERMINAL_CURSOR_STYLE,
};
use crate::domain::theme::service as theme_service;
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::paths::AppPaths;

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
    pub editor_render_whitespace: Option<String>,
    pub editor_bracket_pair_colorization: Option<bool>,
    pub editor_font_ligatures: Option<bool>,
    pub editor_cursor_style: Option<String>,
    pub editor_cursor_blinking: Option<String>,
    pub editor_scroll_beyond_last_line: Option<bool>,
    pub editor_sticky_scroll_enabled: Option<bool>,
    pub terminal_scrollback: Option<u32>,
    pub terminal_cursor_style: Option<String>,
    pub terminal_cursor_blink: Option<bool>,
    pub enable_preview_tabs: Option<bool>,
    pub ai_auto_tab_enabled: Option<bool>,
    pub ai_provider: Option<String>,
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

/// Legacy → current key renames applied by [`migrate_legacy_ai_provider_keys`] — see that
/// function's doc comment for why this is a raw-JSON migration rather than `#[serde(alias)]`.
const LEGACY_AI_PROVIDER_KEY_RENAMES: &[(&str, &str)] = &[("aiAutoTabProvider", "aiProvider"), ("aiAutoTabModel", "aiModel")];

/// Renames pre-rename `aiAutoTabProvider`/`aiAutoTabModel` JSON keys (the `Settings`/
/// `SettingsPatch` field names before
/// `docs/acknowledge/2026-08-16-wave-g-ai-contract.md` §2-2/§3.1) to their current `aiProvider`/
/// `aiModel` names, in place, on an untyped JSON object — applied before deserializing into
/// `Settings`/`SettingsPatch` at every boundary a pre-rename value could still arrive from (a
/// hand-edited/legacy `settings.json` via [`load_settings`], a gist payload synced before the
/// rename via `sync::commands::sync_download`).
///
/// This is a `serde_json::Value` pre-pass rather than `#[serde(alias = "aiAutoTabProvider")]` on
/// the struct fields directly, because with the `specta` version this project pins, a field alias
/// makes `Settings`'/`SettingsPatch`'s Serialize and Deserialize shapes diverge — which splits
/// every TS consumer of those types (including ones with no relation to these two fields) into
/// `_Serialize`/`_Deserialize`/union type variants.
pub fn migrate_legacy_ai_provider_keys(object: &mut serde_json::Map<String, serde_json::Value>) {
    for (legacy_key, current_key) in LEGACY_AI_PROVIDER_KEY_RENAMES {
        if object.contains_key(*current_key) {
            continue;
        }
        if let Some(legacy_value) = object.remove(*legacy_key) {
            object.insert((*current_key).to_string(), legacy_value);
        }
    }
}

/// Applies [`migrate_legacy_ai_provider_keys`] to a raw JSON object before converting it into
/// `Settings` — the shared core of both [`read_settings_file`] (loading `settings.json` from disk)
/// and [`parse_settings_json`] (validating a hand-edited `AppFile` save), so a pre-rename or
/// otherwise legacy-shaped payload is migrated identically through either entry point.
fn settings_from_value(mut raw: serde_json::Value) -> AppResult<Settings> {
    if let Some(object) = raw.as_object_mut() {
        migrate_legacy_ai_provider_keys(object);
    }
    Ok(serde_json::from_value(raw)?)
}

/// Reads and type-checks `settings.json`, applying [`migrate_legacy_ai_provider_keys`] to its raw
/// JSON object first.
fn read_settings_file(path: &std::path::Path) -> AppResult<Option<Settings>> {
    let Some(raw) = persist::read_json::<serde_json::Value>(path)? else {
        return Ok(None);
    };
    Ok(Some(settings_from_value(raw)?))
}

/// Parses a hand-edited `settings.json` save (the `AppFile` tab's "Settings" target) into
/// `Settings`, applying the same legacy-key migration [`read_settings_file`] applies. Kept separate
/// from `read_settings_file` because it parses a `String` already in memory (an unsaved editor
/// buffer), not a file on disk, and reports a parse/schema failure as
/// [`AppError::InvalidArgument`] — a save-time error the caller should surface to the user and
/// reject, not the load-time "fall back to defaults and back up the corrupt file" behavior
/// [`load_settings`] uses for a `settings.json` the app itself can no longer make sense of.
pub fn parse_settings_json(content: &str) -> AppResult<Settings> {
    let raw: serde_json::Value = serde_json::from_str(content)
        .map_err(|error| AppError::InvalidArgument(format!("settings.json이 유효한 JSON이 아닙니다: {error}")))?;
    settings_from_value(raw).map_err(|error| AppError::InvalidArgument(format!("settings.json 스키마가 올바르지 않습니다: {error}")))
}

pub fn load_settings(paths: &AppPaths) -> Settings {
    let path = paths.settings_file();
    match read_settings_file(&path) {
        Ok(Some(settings)) => {
            let sanitized = sanitize(settings.clone());
            if sanitized != settings {
                let _ = persist::write_json(&path, &sanitized);
            }
            sanitized
        }
        Ok(None) => Settings::default(),
        Err(_) => {
            backup_corrupted(&path);
            let defaults = Settings::default();
            let _ = persist::write_json(&path, &defaults);
            defaults
        }
    }
}

pub fn save_settings(paths: &AppPaths, settings: &Settings) -> AppResult<()> {
    persist::write_json(&paths.settings_file(), settings)
}

fn backup_corrupted(path: &std::path::Path) {
    let backup_path = path.with_extension("json.bak");
    let _ = std::fs::rename(path, backup_path);
}

const EDITOR_TAB_SIZE_MIN: u32 = 1;
const EDITOR_TAB_SIZE_MAX: u32 = 8;
const DNS_HOSTNAME_MAX_LEN: usize = 253;
const DNS_LABEL_MAX_LEN: usize = 63;
const TERMINAL_SCROLLBACK_MIN: u32 = 100;
const TERMINAL_SCROLLBACK_MAX: u32 = 100_000;
const RESIZER_THICKNESS_MIN: u32 = 0;
const RESIZER_THICKNESS_MAX: u32 = 8;
const EDITOR_CURSOR_STYLES: &[&str] = &["line", "block", "underline"];
const EDITOR_CURSOR_BLINKING_STYLES: &[&str] = &["blink", "smooth", "phase", "expand", "solid"];
const EDITOR_RENDER_WHITESPACE_MODES: &[&str] = &["none", "boundary", "selection", "all"];
const TERMINAL_CURSOR_STYLES: &[&str] = &["bar", "block", "underline"];
const AI_PROVIDERS: &[&str] = &["ollamaCloud", "codex", "omlx"];

/// Mirrors the frontend's `SEARCH_HISTORY_LIMIT`
/// (`src/entities/search/search-history.ts`). The frontend already caps
/// `recent_searches` at this length before every `settings_update`, but this
/// field also arrives via [`apply_payload_settings`] (sync gist download),
/// which the frontend cap can't guard — a malformed or hand-edited gist
/// payload could otherwise store an unbounded array. Enforced here so every
/// entry point is covered, the same defense-in-depth as [`sanitize_allowed_hosts`].
const RECENT_SEARCHES_MAX: usize = 20;

fn sanitize_enum(value: String, allowed: &[&str], fallback: &str) -> String {
    if allowed.contains(&value.as_str()) {
        value
    } else {
        fallback.to_string()
    }
}

/// Unlike `sanitize_enum`, an out-of-list value here has no meaningful fallback string to fall
/// back to (there is no default AI provider) — it is dropped back to `None`, the same "not
/// configured" state as if it had never been set.
fn sanitize_optional_enum(value: Option<String>, allowed: &[&str]) -> Option<String> {
    value.filter(|v| allowed.contains(&v.as_str()))
}

fn sanitize_optional_url(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let authority = v.strip_prefix("http://").or_else(|| v.strip_prefix("https://"))?;
        let host = authority.split(['/', '?', '#']).next().unwrap_or("");
        if host.is_empty() {
            return None;
        }
        Some(v.trim_end_matches('/').to_string())
    })
}

fn is_valid_dns_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= DNS_LABEL_MAX_LEN
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Accepts only a bare hostname or IPv4 literal for a `remote_allowed_hosts`
/// entry — no scheme, userinfo, path/query, whitespace, or port.
/// `remote::service::is_allowed_host` matches entries against the `Host`
/// header's hostname alone, and `remote::service::format_issue_link_url`
/// interpolates the first entry verbatim into `https://{host}/...`; anything
/// else either never matches (silently disabling the tunnel) or corrupts the
/// issued link (a scheme yields `https://https://...`, and
/// `real.example.com@attacker.example` would make a browser send the link's
/// one-time token to `attacker.example` as if `real.example.com` were
/// userinfo). See `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §6.
fn is_valid_allowed_host(value: &str) -> bool {
    !value.is_empty() && value.len() <= DNS_HOSTNAME_MAX_LEN && value.split('.').all(is_valid_dns_label)
}

/// Trims, lower-cases, and drops malformed entries from a
/// `remote_allowed_hosts` patch/disk value. Applied inside [`sanitize`] so
/// every entry point (patch or hand-edited `settings.json`) is covered, the
/// same defense-in-depth already used for `ai_omlx_base_url`
/// ([`sanitize_optional_url`]).
fn sanitize_allowed_hosts(hosts: Vec<String>) -> Vec<String> {
    hosts
        .into_iter()
        .map(|host| host.trim().to_ascii_lowercase())
        .filter(|host| is_valid_allowed_host(host))
        .collect()
}

fn sanitize_recent_searches(mut searches: Vec<String>) -> Vec<String> {
    searches.truncate(RECENT_SEARCHES_MAX);
    searches
}

fn merge_ai_omlx_base_url(patch_value: Option<&String>, existing: Option<&String>) -> Option<String> {
    match patch_value {
        None => existing.cloned(),
        Some(value) if value.is_empty() => None,
        Some(value) => sanitize_optional_url(Some(value.clone())).or_else(|| existing.cloned()),
    }
}

/// 숫자 범위·문자열 union 필드를 clamp/허용목록으로 보정한다. `Settings` 가 만들어지는 모든 출구
/// (`apply_patch` · 디스크 로드)에서 항상 거친다 — patch 든 손으로 편집한 settings.json 이든
/// 검증되지 않은 값이 Monaco/xterm 런타임까지 그대로 흘러가는 것을 막는다.
pub fn sanitize(settings: Settings) -> Settings {
    Settings {
        editor_tab_size: settings.editor_tab_size.clamp(EDITOR_TAB_SIZE_MIN, EDITOR_TAB_SIZE_MAX),
        terminal_scrollback: settings.terminal_scrollback.clamp(TERMINAL_SCROLLBACK_MIN, TERMINAL_SCROLLBACK_MAX),
        resizer_thickness: settings.resizer_thickness.clamp(RESIZER_THICKNESS_MIN, RESIZER_THICKNESS_MAX),
        editor_cursor_style: sanitize_enum(settings.editor_cursor_style, EDITOR_CURSOR_STYLES, DEFAULT_EDITOR_CURSOR_STYLE),
        editor_cursor_blinking: sanitize_enum(
            settings.editor_cursor_blinking,
            EDITOR_CURSOR_BLINKING_STYLES,
            DEFAULT_EDITOR_CURSOR_BLINKING,
        ),
        editor_render_whitespace: sanitize_enum(
            settings.editor_render_whitespace,
            EDITOR_RENDER_WHITESPACE_MODES,
            DEFAULT_EDITOR_RENDER_WHITESPACE,
        ),
        terminal_cursor_style: sanitize_enum(
            settings.terminal_cursor_style,
            TERMINAL_CURSOR_STYLES,
            DEFAULT_TERMINAL_CURSOR_STYLE,
        ),
        ai_provider: sanitize_optional_enum(settings.ai_provider, AI_PROVIDERS),
        ai_omlx_base_url: sanitize_optional_url(settings.ai_omlx_base_url),
        remote_allowed_hosts: sanitize_allowed_hosts(settings.remote_allowed_hosts),
        recent_searches: sanitize_recent_searches(settings.recent_searches),
        ..settings
    }
}

pub fn apply_patch(settings: &Settings, patch: &SettingsPatch) -> Settings {
    sanitize(Settings {
        version: settings.version,
        theme_id: patch.theme_id.clone().unwrap_or_else(|| settings.theme_id.clone()),
        editor_font_size: patch.editor_font_size.unwrap_or(settings.editor_font_size),
        terminal_font_size: patch.terminal_font_size.unwrap_or(settings.terminal_font_size),
        shell_override: patch.shell_override.clone().or_else(|| settings.shell_override.clone()),
        follow_system_theme: patch.follow_system_theme.unwrap_or(settings.follow_system_theme),
        language: patch.language.clone().unwrap_or_else(|| settings.language.clone()),
        toast_position: patch.toast_position.clone().unwrap_or_else(|| settings.toast_position.clone()),
        resizer_thickness: patch.resizer_thickness.unwrap_or(settings.resizer_thickness),
        editor_font_family: patch.editor_font_family.clone().or_else(|| settings.editor_font_family.clone()),
        terminal_font_family: patch.terminal_font_family.clone().or_else(|| settings.terminal_font_family.clone()),
        ui_font_family: patch.ui_font_family.clone().or_else(|| settings.ui_font_family.clone()),
        format_on_save: patch.format_on_save.unwrap_or(settings.format_on_save),
        auto_save_delay_ms: patch.auto_save_delay_ms.unwrap_or(settings.auto_save_delay_ms),
        keymap_overrides: patch.keymap_overrides.clone().or_else(|| settings.keymap_overrides.clone()),
        editor_minimap: patch.editor_minimap.unwrap_or(settings.editor_minimap),
        show_system_usage: patch.show_system_usage.unwrap_or(settings.show_system_usage),
        agent_status_badge_enabled: patch.agent_status_badge_enabled.unwrap_or(settings.agent_status_badge_enabled),
        agent_hooks_enabled: patch.agent_hooks_enabled.unwrap_or(settings.agent_hooks_enabled),
        ide_integration_enabled: patch.ide_integration_enabled.unwrap_or(settings.ide_integration_enabled),
        ide_auto_open_diff: patch.ide_auto_open_diff.unwrap_or(settings.ide_auto_open_diff),
        editor_word_wrap: patch.editor_word_wrap.unwrap_or(settings.editor_word_wrap),
        editor_line_numbers: patch.editor_line_numbers.unwrap_or(settings.editor_line_numbers),
        editor_tab_size: patch.editor_tab_size.unwrap_or(settings.editor_tab_size),
        editor_insert_spaces: patch.editor_insert_spaces.unwrap_or(settings.editor_insert_spaces),
        editor_detect_indentation: patch.editor_detect_indentation.unwrap_or(settings.editor_detect_indentation),
        editor_render_whitespace: patch
            .editor_render_whitespace
            .clone()
            .unwrap_or_else(|| settings.editor_render_whitespace.clone()),
        editor_bracket_pair_colorization: patch
            .editor_bracket_pair_colorization
            .unwrap_or(settings.editor_bracket_pair_colorization),
        editor_font_ligatures: patch.editor_font_ligatures.unwrap_or(settings.editor_font_ligatures),
        editor_cursor_style: patch
            .editor_cursor_style
            .clone()
            .unwrap_or_else(|| settings.editor_cursor_style.clone()),
        editor_cursor_blinking: patch
            .editor_cursor_blinking
            .clone()
            .unwrap_or_else(|| settings.editor_cursor_blinking.clone()),
        editor_scroll_beyond_last_line: patch
            .editor_scroll_beyond_last_line
            .unwrap_or(settings.editor_scroll_beyond_last_line),
        editor_sticky_scroll_enabled: patch.editor_sticky_scroll_enabled.unwrap_or(settings.editor_sticky_scroll_enabled),
        terminal_scrollback: patch.terminal_scrollback.unwrap_or(settings.terminal_scrollback),
        terminal_cursor_style: patch
            .terminal_cursor_style
            .clone()
            .unwrap_or_else(|| settings.terminal_cursor_style.clone()),
        terminal_cursor_blink: patch.terminal_cursor_blink.unwrap_or(settings.terminal_cursor_blink),
        enable_preview_tabs: patch.enable_preview_tabs.unwrap_or(settings.enable_preview_tabs),
        ai_auto_tab_enabled: patch.ai_auto_tab_enabled.unwrap_or(settings.ai_auto_tab_enabled),
        ai_provider: patch.ai_provider.clone().or_else(|| settings.ai_provider.clone()),
        ai_model: patch.ai_model.clone().or_else(|| settings.ai_model.clone()),
        ai_omlx_base_url: merge_ai_omlx_base_url(patch.ai_omlx_base_url.as_ref(), settings.ai_omlx_base_url.as_ref()),
        sync_gist_id: settings.sync_gist_id.clone(),
        sync_last_synced_at: settings.sync_last_synced_at.clone(),
        remote_access_enabled: patch.remote_access_enabled.unwrap_or(settings.remote_access_enabled),
        remote_password_only_login: patch.remote_password_only_login.unwrap_or(settings.remote_password_only_login),
        remote_allowed_hosts: patch
            .remote_allowed_hosts
            .clone()
            .unwrap_or_else(|| settings.remote_allowed_hosts.clone()),
        organize_imports_on_save: patch.organize_imports_on_save.unwrap_or(settings.organize_imports_on_save),
        fix_all_on_save: patch.fix_all_on_save.unwrap_or(settings.fix_all_on_save),
        editor_code_lens_enabled: patch.editor_code_lens_enabled.unwrap_or(settings.editor_code_lens_enabled),
        editor_semantic_highlighting: patch.editor_semantic_highlighting.unwrap_or(settings.editor_semantic_highlighting),
        editor_format_on_type: patch.editor_format_on_type.unwrap_or(settings.editor_format_on_type),
        editor_format_on_paste: patch.editor_format_on_paste.unwrap_or(settings.editor_format_on_paste),
        emmet_enabled: patch.emmet_enabled.unwrap_or(settings.emmet_enabled),
        recent_searches: patch.recent_searches.clone().unwrap_or_else(|| settings.recent_searches.clone()),
        zen_fullscreen: patch.zen_fullscreen.unwrap_or(settings.zen_fullscreen),
        zen_hide_status_bar: patch.zen_hide_status_bar.unwrap_or(settings.zen_hide_status_bar),
    })
}

pub fn set_theme(paths: &AppPaths, settings: &Settings, theme_id: &str) -> AppResult<Settings> {
    if !theme_service::theme_exists(paths, theme_id) {
        return Err(AppError::NotFound(format!("theme not found: {theme_id}")));
    }

    let updated = Settings {
        theme_id: theme_id.to_string(),
        ..settings.clone()
    };
    save_settings(paths, &updated)?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-settings-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn patch의_none_필드는_기존값을_보존한다() {
        let settings = Settings {
            editor_font_size: 20,
            ..Settings::default()
        };
        let patch = SettingsPatch {
            terminal_font_size: Some(16),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.editor_font_size, 20);
        assert_eq!(updated.terminal_font_size, 16);
        assert_eq!(updated.theme_id, settings.theme_id);
        assert_eq!(updated.follow_system_theme, settings.follow_system_theme);
    }

    #[test]
    fn patch의_some_필드는_값을_덮어쓴다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            theme_id: Some("taide-light".to_string()),
            follow_system_theme: Some(true),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.theme_id, "taide-light");
        assert!(updated.follow_system_theme);
    }

    #[test]
    fn patch로_저장시_포맷과_자동저장_지연을_설정한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            format_on_save: Some(true),
            auto_save_delay_ms: Some(1_500),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.format_on_save);
        assert_eq!(updated.auto_save_delay_ms, 1_500);
    }

    #[test]
    fn patch로_미니맵과_리소스_표시_agent_ide_설정을_변경한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            editor_minimap: Some(false),
            show_system_usage: Some(false),
            agent_status_badge_enabled: Some(false),
            agent_hooks_enabled: Some(true),
            ide_integration_enabled: Some(false),
            ide_auto_open_diff: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(!updated.editor_minimap);
        assert!(!updated.show_system_usage);
        assert!(!updated.agent_status_badge_enabled);
        assert!(updated.agent_hooks_enabled);
        assert!(!updated.ide_integration_enabled);
        assert!(!updated.ide_auto_open_diff);
    }

    #[test]
    fn patch로_키맵_오버라이드_json을_설정한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            keymap_overrides: Some("[]".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.keymap_overrides, Some("[]".to_string()));
    }

    #[test]
    fn patch로_에디터와_터미널_신규_설정_15필드를_변경한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            editor_word_wrap: Some(true),
            editor_line_numbers: Some(false),
            editor_tab_size: Some(2),
            editor_insert_spaces: Some(false),
            editor_detect_indentation: Some(false),
            editor_render_whitespace: Some("all".to_string()),
            editor_bracket_pair_colorization: Some(false),
            editor_font_ligatures: Some(true),
            editor_cursor_style: Some("block".to_string()),
            editor_cursor_blinking: Some("smooth".to_string()),
            editor_scroll_beyond_last_line: Some(false),
            terminal_scrollback: Some(5_000),
            terminal_cursor_style: Some("underline".to_string()),
            terminal_cursor_blink: Some(false),
            enable_preview_tabs: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.editor_word_wrap);
        assert!(!updated.editor_line_numbers);
        assert_eq!(updated.editor_tab_size, 2);
        assert!(!updated.editor_insert_spaces);
        assert!(!updated.editor_detect_indentation);
        assert_eq!(updated.editor_render_whitespace, "all");
        assert!(!updated.editor_bracket_pair_colorization);
        assert!(updated.editor_font_ligatures);
        assert_eq!(updated.editor_cursor_style, "block");
        assert_eq!(updated.editor_cursor_blinking, "smooth");
        assert!(!updated.editor_scroll_beyond_last_line);
        assert_eq!(updated.terminal_scrollback, 5_000);
        assert_eq!(updated.terminal_cursor_style, "underline");
        assert!(!updated.terminal_cursor_blink);
        assert!(!updated.enable_preview_tabs);
    }

    #[test]
    fn 범위를_벗어난_숫자와_허용목록_밖의_문자열은_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            editor_tab_size: Some(100),
            terminal_scrollback: Some(1),
            resizer_thickness: Some(999),
            editor_cursor_style: Some("invalid".to_string()),
            editor_cursor_blinking: Some("invalid".to_string()),
            editor_render_whitespace: Some("invalid".to_string()),
            terminal_cursor_style: Some("invalid".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.editor_tab_size, 8);
        assert_eq!(updated.terminal_scrollback, 100);
        assert_eq!(updated.resizer_thickness, 8);
        assert_eq!(updated.editor_cursor_style, DEFAULT_EDITOR_CURSOR_STYLE);
        assert_eq!(updated.editor_cursor_blinking, DEFAULT_EDITOR_CURSOR_BLINKING);
        assert_eq!(updated.editor_render_whitespace, DEFAULT_EDITOR_RENDER_WHITESPACE);
        assert_eq!(updated.terminal_cursor_style, DEFAULT_TERMINAL_CURSOR_STYLE);
    }

    #[test]
    fn patch로_auto_tab_설정을_변경한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_auto_tab_enabled: Some(true),
            ai_provider: Some("ollamaCloud".to_string()),
            ai_model: Some("qwen2.5-coder".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.ai_auto_tab_enabled);
        assert_eq!(updated.ai_provider, Some("ollamaCloud".to_string()));
        assert_eq!(updated.ai_model, Some("qwen2.5-coder".to_string()));
    }

    #[test]
    fn 허용목록_밖의_ai_provider는_none으로_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_provider: Some("anthropic".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_provider, None);
    }

    #[test]
    fn 구_필드명_객체는_마이그레이션_후_ai_provider_키로_파싱된다() {
        let mut value: serde_json::Value = serde_json::from_str(r#"{"aiAutoTabProvider":"ollamaCloud","aiAutoTabModel":"qwen2.5-coder"}"#)
            .expect("구 필드명 JSON이 파싱되어야 함");
        migrate_legacy_ai_provider_keys(value.as_object_mut().expect("object"));

        let patch: SettingsPatch = serde_json::from_value(value).expect("마이그레이션된 JSON이 SettingsPatch로 파싱되어야 함");

        assert_eq!(patch.ai_provider, Some("ollamaCloud".to_string()));
        assert_eq!(patch.ai_model, Some("qwen2.5-coder".to_string()));
    }

    #[test]
    fn 신_필드명이_이미_있으면_구_필드명_마이그레이션은_덮어쓰지_않는다() {
        let mut value: serde_json::Value =
            serde_json::from_str(r#"{"aiAutoTabProvider":"codex","aiProvider":"omlx"}"#).expect("JSON이 파싱되어야 함");
        migrate_legacy_ai_provider_keys(value.as_object_mut().expect("object"));

        let patch: SettingsPatch = serde_json::from_value(value).expect("마이그레이션된 JSON이 SettingsPatch로 파싱되어야 함");

        assert_eq!(patch.ai_provider, Some("omlx".to_string()));
    }

    #[test]
    fn 구_필드명_ai_auto_tab_provider로_저장된_settings_json도_ai_provider로_읽힌다() {
        let paths = AppPaths::new(temp_data_dir("legacy-ai-provider-field"));
        let path = paths.settings_file();
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dir");
        std::fs::write(
            &path,
            br#"{"version":1,"aiAutoTabProvider":"codex","aiAutoTabModel":"gpt-5.6-sol"}"#,
        )
        .expect("write legacy settings file");

        let settings = load_settings(&paths);

        assert_eq!(settings.ai_provider, Some("codex".to_string()));
        assert_eq!(settings.ai_model, Some("gpt-5.6-sol".to_string()));

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn patch로_omlx_base_url을_설정하면_trailing_slash가_제거된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("http://localhost:8000/".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, Some("http://localhost:8000".to_string()));
    }

    #[test]
    fn http_https로_시작하지_않는_omlx_base_url은_none으로_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("ftp://localhost:8000".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, None);
    }

    #[test]
    fn 스킴만_있고_host가_없는_omlx_base_url은_none으로_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("http://".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, None);
    }

    #[test]
    fn 빈_문자열_패치는_기존_omlx_base_url을_해제한다() {
        let settings = Settings {
            ai_omlx_base_url: Some("http://localhost:8000".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            ai_omlx_base_url: Some(String::new()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, None);
    }

    #[test]
    fn 유효하지_않은_omlx_base_url_패치는_기존_값을_보존한다() {
        let settings = Settings {
            ai_omlx_base_url: Some("http://localhost:8000".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            ai_omlx_base_url: Some("localhost:8123".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, Some("http://localhost:8000".to_string()));
    }

    #[test]
    fn patch에_omlx_base_url이_없으면_기존_값을_유지한다() {
        let settings = Settings {
            ai_omlx_base_url: Some("http://localhost:8000".to_string()),
            ..Settings::default()
        };
        let patch = SettingsPatch {
            editor_font_size: Some(16),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_omlx_base_url, Some("http://localhost:8000".to_string()));
    }

    #[test]
    fn patch로_원격_접속_활성화를_변경한다() {
        let settings = Settings::default();
        assert!(!settings.remote_access_enabled);

        let patch = SettingsPatch {
            remote_access_enabled: Some(true),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_access_enabled);
    }

    #[test]
    fn patch로_원격_비밀번호만_로그인_허용을_변경한다() {
        let settings = Settings::default();
        assert!(!settings.remote_password_only_login);

        let patch = SettingsPatch {
            remote_password_only_login: Some(true),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_password_only_login);
    }

    #[test]
    fn patch로_원격_허용_호스트_목록을_변경한다() {
        let settings = Settings::default();
        assert!(settings.remote_allowed_hosts.is_empty());

        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["tunnel.example.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.remote_allowed_hosts, vec!["tunnel.example.com".to_string()]);
    }

    #[test]
    fn 스킴이_포함된_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["https://tunnel.example.com".to_string(), "tunnel.example.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.remote_allowed_hosts, vec!["tunnel.example.com".to_string()]);
    }

    #[test]
    fn userinfo가_포함된_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["real.example.com@attacker.example".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(
            updated.remote_allowed_hosts.is_empty(),
            "userinfo 형태의 허용 호스트가 링크 토큰 탈취 경로로 저장되면 안 된다"
        );
    }

    #[test]
    fn 포트나_공백이_포함된_허용_호스트는_걸러진다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["tunnel.example.com:8080".to_string(), "tunnel example.com".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.remote_allowed_hosts.is_empty());
    }

    #[test]
    fn 유효한_호스트명과_ipv4는_소문자로_정규화되어_저장된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            remote_allowed_hosts: Some(vec!["Tunnel.Example.Com".to_string(), " 192.168.1.5 ".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(
            updated.remote_allowed_hosts,
            vec!["tunnel.example.com".to_string(), "192.168.1.5".to_string()]
        );
    }

    #[test]
    fn patch로_organize_imports_fix_all_code_lens_설정을_변경한다() {
        let settings = Settings::default();
        assert!(!settings.organize_imports_on_save);
        assert!(!settings.fix_all_on_save);
        assert!(settings.editor_code_lens_enabled);

        let patch = SettingsPatch {
            organize_imports_on_save: Some(true),
            fix_all_on_save: Some(true),
            editor_code_lens_enabled: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.organize_imports_on_save);
        assert!(updated.fix_all_on_save);
        assert!(!updated.editor_code_lens_enabled);
    }

    #[test]
    fn patch로_semantic_하이라이팅_포매팅_emmet_설정을_변경한다() {
        let settings = Settings::default();
        assert!(settings.editor_semantic_highlighting);
        assert!(!settings.editor_format_on_type);
        assert!(!settings.editor_format_on_paste);
        assert!(settings.emmet_enabled);

        let patch = SettingsPatch {
            editor_semantic_highlighting: Some(false),
            editor_format_on_type: Some(true),
            editor_format_on_paste: Some(true),
            emmet_enabled: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(!updated.editor_semantic_highlighting);
        assert!(updated.editor_format_on_type);
        assert!(updated.editor_format_on_paste);
        assert!(!updated.emmet_enabled);
    }

    #[test]
    fn patch로_recent_searches를_변경한다() {
        let settings = Settings::default();
        assert!(settings.recent_searches.is_empty());

        let patch = SettingsPatch {
            recent_searches: Some(vec!["needle".to_string(), "haystack".to_string()]),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.recent_searches, vec!["needle".to_string(), "haystack".to_string()]);
    }

    #[test]
    fn patch에_recent_searches가_없으면_기존_값을_유지한다() {
        let settings = Settings {
            recent_searches: vec!["needle".to_string()],
            ..Settings::default()
        };

        let updated = apply_patch(&settings, &SettingsPatch::default());

        assert_eq!(updated.recent_searches, vec!["needle".to_string()]);
    }

    #[test]
    fn recent_searches는_상한을_초과하면_잘려나간다() {
        let settings = Settings::default();
        let oversized: Vec<String> = (0..(RECENT_SEARCHES_MAX + 10)).map(|index| format!("term-{index}")).collect();
        let patch = SettingsPatch {
            recent_searches: Some(oversized.clone()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.recent_searches.len(), RECENT_SEARCHES_MAX);
        assert_eq!(updated.recent_searches, oversized[..RECENT_SEARCHES_MAX]);
    }

    #[test]
    fn resizer_thickness는_0을_허용한다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            resizer_thickness: Some(0),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.resizer_thickness, 0);
    }

    #[test]
    fn 없는_settings_파일은_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("missing"));
        let settings = load_settings(&paths);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn 파손된_settings_파일은_bak로_밀리고_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("corrupt"));
        let path = paths.settings_file();
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dir");
        std::fs::write(&path, b"{not json at all").expect("write corrupt file");

        let settings = load_settings(&paths);

        assert_eq!(settings, Settings::default());
        assert!(path.with_extension("json.bak").exists());

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 손으로_편집된_settings_파일의_잘못된_값은_로드시_보정되어_재저장된다() {
        let paths = AppPaths::new(temp_data_dir("hand-edited"));
        let path = paths.settings_file();
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dir");
        std::fs::write(
            &path,
            br#"{"version":1,"terminalCursorStyle":"banana","editorTabSize":0,"terminalScrollback":99999999}"#,
        )
        .expect("write settings file");

        let settings = load_settings(&paths);

        assert_eq!(settings.terminal_cursor_style, DEFAULT_TERMINAL_CURSOR_STYLE);
        assert_eq!(settings.editor_tab_size, EDITOR_TAB_SIZE_MIN);
        assert_eq!(settings.terminal_scrollback, TERMINAL_SCROLLBACK_MAX);

        let reloaded = persist::read_json::<Settings>(&path).expect("reread").expect("some");
        assert_eq!(reloaded, settings);

        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn patch로_zen_설정을_변경한다() {
        let settings = Settings::default();
        assert!(!settings.zen_fullscreen);
        assert!(settings.zen_hide_status_bar);

        let patch = SettingsPatch {
            zen_fullscreen: Some(true),
            zen_hide_status_bar: Some(false),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.zen_fullscreen);
        assert!(!updated.zen_hide_status_bar);
    }

    #[test]
    fn set_theme는_존재하는_테마만_허용한다() {
        let paths = AppPaths::new(temp_data_dir("set-theme"));
        std::fs::create_dir_all(paths.themes_dir()).expect("create themes dir");
        let settings = Settings::default();

        let ok = set_theme(&paths, &settings, theme_service::BUILTIN_LIGHT_ID);
        assert!(ok.is_ok());
        assert_eq!(ok.unwrap().theme_id, theme_service::BUILTIN_LIGHT_ID);

        let err = set_theme(&paths, &settings, "does-not-exist");
        assert!(matches!(err, Err(AppError::NotFound(_))));

        std::fs::remove_dir_all(paths.data_dir).ok();
    }
}
