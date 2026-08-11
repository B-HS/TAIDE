use crate::domain::ai::types::{AiPromptTemplate, AiPromptVars};
use crate::infra::persist;
use crate::paths::AppPaths;

pub const DEFAULT_PROMPT_TEMPLATE_ID: &str = "auto-tab-default";

const BUNDLED_AUTO_TAB_DEFAULT: &str = include_str!("../../../resources/prompts/auto-tab-default.json");

pub fn bundled_prompt_template() -> AiPromptTemplate {
    serde_json::from_str(BUNDLED_AUTO_TAB_DEFAULT).expect("bundled auto-tab-default.json must parse")
}

/// Loads the auto-tab prompt template, letting a user override at
/// `{app_data}/prompts/auto-tab-default.json` win over the bundled default (themes/locales
/// bundle+user-directory pattern). A missing or unparseable override file is ignored — the
/// override completely replaces the template rather than merging fields, since (unlike
/// theme/locale packs) there is no `extends` concept for prompt templates yet.
pub fn load_prompt_template(paths: &AppPaths) -> AiPromptTemplate {
    let override_path = paths.prompts_dir().join(format!("{DEFAULT_PROMPT_TEMPLATE_ID}.json"));
    match persist::read_json::<AiPromptTemplate>(&override_path) {
        Ok(Some(template)) => template,
        Ok(None) | Err(_) => bundled_prompt_template(),
    }
}

/// Reduces a (possibly absolute, possibly untitled-URI) editor path to just its final path
/// component before it ever reaches a prompt sent to an external provider (Codex, Ollama Cloud).
/// The model only needs a filename/extension for completion context — the full path would also
/// leak the local username and directory structure (`/Users/<name>/…`) in every request.
fn basename(file_path: &str) -> &str {
    let trimmed = file_path.trim_end_matches(['/', '\\']);
    match trimmed.rsplit(['/', '\\']).next() {
        Some(name) if !name.is_empty() => name,
        _ => file_path,
    }
}

pub fn render(template: &str, vars: &AiPromptVars) -> String {
    template
        .replace("{prefix}", vars.prefix)
        .replace("{suffix}", vars.suffix)
        .replace("{language}", vars.language)
        .replace("{filePath}", basename(vars.file_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_data_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("taide-ai-prompt-{name}-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn 번들_기본_템플릿은_파싱되고_버전을_가진다() {
        let template = bundled_prompt_template();
        assert_eq!(template.version, 1);
        assert!(template.fim.prompt.contains("{prefix}"));
        assert!(template.chat.user.contains("{suffix}"));
    }

    #[test]
    fn 오버라이드_파일이_없으면_번들_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("missing"));
        let template = load_prompt_template(&paths);
        assert_eq!(template, bundled_prompt_template());
    }

    #[test]
    fn 사용자_오버라이드가_있으면_번들_대신_사용된다() {
        let paths = AppPaths::new(temp_data_dir("override"));
        let custom = AiPromptTemplate {
            version: 2,
            fim: crate::domain::ai::types::AiFimPromptTemplate {
                prompt: "CUSTOM {prefix}".to_string(),
                suffix: "{suffix}".to_string(),
                stop: vec![],
            },
            chat: crate::domain::ai::types::AiChatPromptTemplate {
                system: "custom system".to_string(),
                user: "custom user".to_string(),
            },
        };
        persist::write_json(&paths.prompts_dir().join(format!("{DEFAULT_PROMPT_TEMPLATE_ID}.json")), &custom).expect("write override");

        let loaded = load_prompt_template(&paths);

        assert_eq!(loaded, custom);
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 파손된_오버라이드_파일은_무시되고_번들_기본값으로_폴백한다() {
        let paths = AppPaths::new(temp_data_dir("corrupt"));
        let override_path = paths.prompts_dir().join(format!("{DEFAULT_PROMPT_TEMPLATE_ID}.json"));
        std::fs::create_dir_all(override_path.parent().unwrap()).expect("create dir");
        std::fs::write(&override_path, b"{not json at all").expect("write corrupt override");

        let loaded = load_prompt_template(&paths);

        assert_eq!(loaded, bundled_prompt_template());
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn render_은_prefix_suffix_language_filepath_를_치환한다() {
        let rendered = render(
            "lang={language} file={filePath} prefix={prefix} suffix={suffix} again={prefix}",
            &AiPromptVars {
                prefix: "PRE",
                suffix: "SUF",
                language: "rust",
                file_path: "src/main.rs",
            },
        );
        assert_eq!(rendered, "lang=rust file=main.rs prefix=PRE suffix=SUF again=PRE");
    }

    #[test]
    fn render_은_절대경로에서_파일명만_남기고_디렉터리_구조는_제거한다() {
        let rendered = render(
            "file={filePath}",
            &AiPromptVars {
                prefix: "",
                suffix: "",
                language: "rust",
                file_path: "/Users/alice/projects/taide/src/main.rs",
            },
        );
        assert_eq!(rendered, "file=main.rs");
        assert!(!rendered.contains("alice"));
    }

    #[test]
    fn render_은_구분자가_없는_untitled_경로는_그대로_유지한다() {
        let rendered = render(
            "file={filePath}",
            &AiPromptVars {
                prefix: "",
                suffix: "",
                language: "rust",
                file_path: "untitled:tab-1",
            },
        );
        assert_eq!(rendered, "file=untitled:tab-1");
    }

    #[test]
    fn render_은_변수가_없는_템플릿을_그대로_반환한다() {
        let rendered = render(
            "no variables here",
            &AiPromptVars {
                prefix: "PRE",
                suffix: "SUF",
                language: "rust",
                file_path: "src/main.rs",
            },
        );
        assert_eq!(rendered, "no variables here");
    }
}
