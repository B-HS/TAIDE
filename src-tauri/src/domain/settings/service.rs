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
