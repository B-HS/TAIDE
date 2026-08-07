use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::settings::types::Settings;
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
}

pub fn load_settings(paths: &AppPaths) -> Settings {
    let path = paths.settings_file();
    match persist::read_json::<Settings>(&path) {
        Ok(Some(settings)) => settings,
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

pub fn apply_patch(settings: &Settings, patch: &SettingsPatch) -> Settings {
    Settings {
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
    }
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
