use std::path::PathBuf;

use super::types::{AppFileTarget, AppInfo, PromptTemplateId};
use crate::domain::ai::prompt as ai_prompt;
use crate::domain::ai::types::{AiCommitMessagePromptTemplate, AiInlineEditPromptTemplate, AiPromptTemplate};
use crate::domain::settings::types::Settings;
use crate::error::{AppError, AppResult};
use crate::infra::persist;
use crate::paths::AppPaths;

pub const APP_NAME: &str = "TAIDE";

pub fn app_info() -> AppInfo {
    AppInfo {
        name: APP_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

/// The on-disk path an `AppFileTarget` resolves to — never handed to the frontend directly
/// (contract §3.3), only used internally to read/write the file.
pub fn app_file_path(paths: &AppPaths, target: AppFileTarget) -> PathBuf {
    match target {
        AppFileTarget::Settings => paths.settings_file(),
        AppFileTarget::Prompt { id } => paths.prompts_dir().join(format!("{}.json", id.as_str())),
    }
}

fn bundled_prompt_text(id: PromptTemplateId) -> String {
    let serialized = match id {
        PromptTemplateId::AutoTabDefault => serde_json::to_string_pretty(&ai_prompt::bundled_prompt_template()),
        PromptTemplateId::InlineEditDefault => serde_json::to_string_pretty(&ai_prompt::bundled_inline_edit_template()),
        PromptTemplateId::CommitMessageDefault => serde_json::to_string_pretty(&ai_prompt::bundled_commit_message_template()),
    };
    serialized.expect("bundled prompt templates are always serializable")
}

/// Reads an `AppFileTarget`'s current text — the on-disk override if one exists, otherwise the
/// live in-memory value (`Settings`) or the bundled default (a prompt template that was never
/// customized), mirroring the same bundled/override fallback `ai::prompt::load_template` already
/// applies when actually *using* a prompt. A missing `settings.json` falls back to `current`
/// (rather than `Settings::default()`) so a freshly-booted app that hasn't persisted a settings
/// file yet still shows the sanitized values already loaded into `AppState`.
pub fn read_app_file(paths: &AppPaths, target: AppFileTarget, current_settings: &Settings) -> AppResult<String> {
    let path = app_file_path(paths, target);
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match target {
            AppFileTarget::Settings => serde_json::to_string_pretty(current_settings).map_err(AppError::from),
            AppFileTarget::Prompt { id } => Ok(bundled_prompt_text(id)),
        },
        Err(error) => Err(AppError::from(error)),
    }
}

/// Validates `content` parses as the prompt template shape `id` expects — called before persisting
/// an `app_file_write` for a `Prompt` target so a malformed save is rejected with a clear error
/// instead of silently falling back to the bundled default the next time `ai::prompt::load_template`
/// reads it.
fn validate_prompt_json(id: PromptTemplateId, content: &str) -> AppResult<()> {
    let result = match id {
        PromptTemplateId::AutoTabDefault => serde_json::from_str::<AiPromptTemplate>(content).map(|_| ()),
        PromptTemplateId::InlineEditDefault => serde_json::from_str::<AiInlineEditPromptTemplate>(content).map(|_| ()),
        PromptTemplateId::CommitMessageDefault => serde_json::from_str::<AiCommitMessagePromptTemplate>(content).map(|_| ()),
    };
    result.map_err(|error| AppError::InvalidArgument(format!("프롬프트 템플릿 JSON이 올바르지 않습니다: {error}")))
}

/// Writes a `Prompt` target's override file after validating it parses as that prompt's shape —
/// invalid JSON is rejected and the existing override (or bundled default) is left untouched.
pub fn write_prompt_file(paths: &AppPaths, id: PromptTemplateId, content: &str) -> AppResult<()> {
    validate_prompt_json(id, content)?;
    persist::write_atomic(&app_file_path(paths, AppFileTarget::Prompt { id }), content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-app-file-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn settings_파일이_없으면_현재_메모리_값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("settings-missing"));
        let current = Settings {
            editor_font_size: 21,
            ..Settings::default()
        };

        let content = read_app_file(&paths, AppFileTarget::Settings, &current).expect("read");

        let parsed: Settings = serde_json::from_str(&content).expect("parse");
        assert_eq!(parsed.editor_font_size, 21);
    }

    #[test]
    fn settings_파일이_있으면_디스크_내용을_그대로_반환한다() {
        let paths = AppPaths::new(temp_data_dir("settings-present"));
        std::fs::create_dir_all(&paths.data_dir).expect("create dir");
        std::fs::write(paths.settings_file(), r#"{"version":1,"editorFontSize":30}"#).expect("write");

        let content = read_app_file(&paths, AppFileTarget::Settings, &Settings::default()).expect("read");

        assert!(content.contains("30"));
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 프롬프트_오버라이드가_없으면_번들_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("prompt-missing"));

        let content = read_app_file(
            &paths,
            AppFileTarget::Prompt {
                id: PromptTemplateId::AutoTabDefault,
            },
            &Settings::default(),
        )
        .expect("read");

        let parsed: AiPromptTemplate = serde_json::from_str(&content).expect("parse");
        assert_eq!(parsed, ai_prompt::bundled_prompt_template());
    }

    #[test]
    fn 유효한_프롬프트_오버라이드는_저장되고_그대로_읽힌다() {
        let paths = AppPaths::new(temp_data_dir("prompt-write"));
        let custom = AiInlineEditPromptTemplate {
            version: 2,
            system: "custom system".to_string(),
            user: "custom user {selection}".to_string(),
        };
        let content = serde_json::to_string_pretty(&custom).expect("serialize");

        write_prompt_file(&paths, PromptTemplateId::InlineEditDefault, &content).expect("write");
        let read_back = read_app_file(
            &paths,
            AppFileTarget::Prompt {
                id: PromptTemplateId::InlineEditDefault,
            },
            &Settings::default(),
        )
        .expect("read");

        let parsed: AiInlineEditPromptTemplate = serde_json::from_str(&read_back).expect("parse");
        assert_eq!(parsed, custom);
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 유효하지_않은_프롬프트_json은_저장이_거부된다() {
        let paths = AppPaths::new(temp_data_dir("prompt-invalid"));

        let result = write_prompt_file(&paths, PromptTemplateId::CommitMessageDefault, "{ not json");

        assert!(result.is_err());
        assert!(!paths
            .prompts_dir()
            .join(format!("{}.json", PromptTemplateId::CommitMessageDefault.as_str()))
            .exists());
    }
}
