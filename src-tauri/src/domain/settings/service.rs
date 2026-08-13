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
    pub ai_auto_tab_provider: Option<String>,
    pub ai_auto_tab_model: Option<String>,
    pub ai_omlx_base_url: Option<String>,
    pub remote_access_enabled: Option<bool>,
}

pub fn load_settings(paths: &AppPaths) -> Settings {
    let path = paths.settings_file();
    match persist::read_json::<Settings>(&path) {
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
const TERMINAL_SCROLLBACK_MIN: u32 = 100;
const TERMINAL_SCROLLBACK_MAX: u32 = 100_000;
const RESIZER_THICKNESS_MIN: u32 = 0;
const RESIZER_THICKNESS_MAX: u32 = 8;
const EDITOR_CURSOR_STYLES: &[&str] = &["line", "block", "underline"];
const EDITOR_CURSOR_BLINKING_STYLES: &[&str] = &["blink", "smooth", "phase", "expand", "solid"];
const EDITOR_RENDER_WHITESPACE_MODES: &[&str] = &["none", "boundary", "selection", "all"];
const TERMINAL_CURSOR_STYLES: &[&str] = &["bar", "block", "underline"];
const AI_AUTO_TAB_PROVIDERS: &[&str] = &["ollamaCloud", "codex", "omlx"];

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
fn sanitize(settings: Settings) -> Settings {
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
        ai_auto_tab_provider: sanitize_optional_enum(settings.ai_auto_tab_provider, AI_AUTO_TAB_PROVIDERS),
        ai_omlx_base_url: sanitize_optional_url(settings.ai_omlx_base_url),
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
        ai_auto_tab_provider: patch.ai_auto_tab_provider.clone().or_else(|| settings.ai_auto_tab_provider.clone()),
        ai_auto_tab_model: patch.ai_auto_tab_model.clone().or_else(|| settings.ai_auto_tab_model.clone()),
        ai_omlx_base_url: merge_ai_omlx_base_url(patch.ai_omlx_base_url.as_ref(), settings.ai_omlx_base_url.as_ref()),
        sync_gist_id: settings.sync_gist_id.clone(),
        sync_last_synced_at: settings.sync_last_synced_at.clone(),
        remote_access_enabled: patch.remote_access_enabled.unwrap_or(settings.remote_access_enabled),
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
            ai_auto_tab_provider: Some("ollamaCloud".to_string()),
            ai_auto_tab_model: Some("qwen2.5-coder".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert!(updated.ai_auto_tab_enabled);
        assert_eq!(updated.ai_auto_tab_provider, Some("ollamaCloud".to_string()));
        assert_eq!(updated.ai_auto_tab_model, Some("qwen2.5-coder".to_string()));
    }

    #[test]
    fn 허용목록_밖의_auto_tab_provider는_none으로_보정된다() {
        let settings = Settings::default();
        let patch = SettingsPatch {
            ai_auto_tab_provider: Some("anthropic".to_string()),
            ..SettingsPatch::default()
        };

        let updated = apply_patch(&settings, &patch);

        assert_eq!(updated.ai_auto_tab_provider, None);
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
