use serde::de::DeserializeOwned;

use crate::domain::ai::types::{
    AiCommitMessagePromptTemplate, AiCommitMessagePromptVars, AiInlineEditPromptTemplate, AiInlineEditPromptVars, AiPromptTemplate,
    AiPromptVars,
};
use crate::infra::persist;
use crate::paths::AppPaths;

pub const AUTO_TAB_PROMPT_ID: &str = "auto-tab-default";
pub const INLINE_EDIT_PROMPT_ID: &str = "inline-edit-default";
pub const COMMIT_MESSAGE_PROMPT_ID: &str = "commit-message-default";

/// Selected code plus surrounding context sent to the model for Inline Edit is capped at this many
/// characters per side (`{prefix}`/`{suffix}`) — the selection itself and the instruction are never
/// truncated, only the extra context around them, so a huge file can't blow up the request payload
/// or token cost.
const INLINE_EDIT_CONTEXT_CHAR_LIMIT: usize = 2_000;

const BUNDLED_AUTO_TAB_DEFAULT: &str = include_str!("../../../resources/prompts/auto-tab-default.json");
const BUNDLED_INLINE_EDIT_DEFAULT: &str = include_str!("../../../resources/prompts/inline-edit-default.json");
const BUNDLED_COMMIT_MESSAGE_DEFAULT: &str = include_str!("../../../resources/prompts/commit-message-default.json");

pub fn bundled_prompt_template() -> AiPromptTemplate {
    serde_json::from_str(BUNDLED_AUTO_TAB_DEFAULT).expect("bundled auto-tab-default.json must parse")
}

pub fn bundled_inline_edit_template() -> AiInlineEditPromptTemplate {
    serde_json::from_str(BUNDLED_INLINE_EDIT_DEFAULT).expect("bundled inline-edit-default.json must parse")
}

pub fn bundled_commit_message_template() -> AiCommitMessagePromptTemplate {
    serde_json::from_str(BUNDLED_COMMIT_MESSAGE_DEFAULT).expect("bundled commit-message-default.json must parse")
}

/// Loads a prompt template by `id`, letting a user override at `{app_data}/prompts/{id}.json` win
/// over the bundled default (themes/locales bundle+user-directory pattern). A missing or
/// unparseable override file is ignored — the override completely replaces the template rather
/// than merging fields, since (unlike theme/locale packs) there is no `extends` concept for prompt
/// templates yet. Shared by all three prompt loaders below (auto-tab/inline-edit/commit-message) —
/// the file-read/override/fallback mechanics are identical, only the id and the bundled fallback
/// differ.
fn load_template<T: DeserializeOwned>(paths: &AppPaths, id: &str, bundled: fn() -> T) -> T {
    let override_path = paths.prompts_dir().join(format!("{id}.json"));
    match persist::read_json::<T>(&override_path) {
        Ok(Some(template)) => template,
        Ok(None) | Err(_) => bundled(),
    }
}

pub fn load_prompt_template(paths: &AppPaths) -> AiPromptTemplate {
    load_template(paths, AUTO_TAB_PROMPT_ID, bundled_prompt_template)
}

pub fn load_inline_edit_prompt_template(paths: &AppPaths) -> AiInlineEditPromptTemplate {
    load_template(paths, INLINE_EDIT_PROMPT_ID, bundled_inline_edit_template)
}

pub fn load_commit_message_prompt_template(paths: &AppPaths) -> AiCommitMessagePromptTemplate {
    load_template(paths, COMMIT_MESSAGE_PROMPT_ID, bundled_commit_message_template)
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

/// Keeps only the last [`INLINE_EDIT_CONTEXT_CHAR_LIMIT`] characters of `text` — used to cap
/// `{prefix}` context so a huge file above the selection doesn't balloon the Inline Edit request.
/// Walks `char_indices` (not byte offsets) so the cut always lands on a `char` boundary — slicing
/// mid multi-byte UTF-8 sequence panics. `text` shorter than the limit is returned unchanged.
fn clamp_prefix_context(text: &str) -> &str {
    match text.char_indices().rev().nth(INLINE_EDIT_CONTEXT_CHAR_LIMIT - 1) {
        Some((byte_start, _)) => &text[byte_start..],
        None => text,
    }
}

/// Keeps only the first [`INLINE_EDIT_CONTEXT_CHAR_LIMIT`] characters of `text` — the `{suffix}`
/// counterpart to [`clamp_prefix_context`].
fn clamp_suffix_context(text: &str) -> &str {
    match text.char_indices().nth(INLINE_EDIT_CONTEXT_CHAR_LIMIT) {
        Some((byte_end, _)) => &text[..byte_end],
        None => text,
    }
}

/// Single-pass placeholder substitution: scans `template` once, and at every position tries each
/// `(placeholder, value)` pair in order, copying the matched `value` into the output and advancing
/// past the placeholder *in the original template* — never re-scanning `value` itself for further
/// placeholder matches. This is deliberately not a chain of `str::replace` calls (the previous
/// implementation): `str::replace` operates on its *entire current input*, so if an earlier
/// substitution's value happens to contain literal placeholder text (e.g. a user's `{selection}`
/// containing the string `"{instruction}"`), a later `.replace("{instruction}", ...)` call in the
/// chain would match and expand that reinjected text too — letting diff/selection/instruction
/// content that merely *looks like* a placeholder warp the rendered prompt. Scanning the template
/// once and treating every substituted value as opaque output closes that off entirely.
fn render_placeholders(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = String::with_capacity(template.len());
    let mut rest = template;
    'scan: while !rest.is_empty() {
        for (placeholder, value) in replacements {
            if let Some(remainder) = rest.strip_prefix(placeholder) {
                result.push_str(value);
                rest = remainder;
                continue 'scan;
            }
        }
        let mut chars = rest.chars();
        if let Some(ch) = chars.next() {
            result.push(ch);
        }
        rest = chars.as_str();
    }
    result
}

pub fn render(template: &str, vars: &AiPromptVars) -> String {
    render_placeholders(
        template,
        &[
            ("{prefix}", vars.prefix),
            ("{suffix}", vars.suffix),
            ("{language}", vars.language),
            ("{filePath}", basename(vars.file_path)),
        ],
    )
}

pub fn render_inline_edit(template: &str, vars: &AiInlineEditPromptVars) -> String {
    let prefix = clamp_prefix_context(vars.prefix);
    let suffix = clamp_suffix_context(vars.suffix);
    render_placeholders(
        template,
        &[
            ("{selection}", vars.selection),
            ("{instruction}", vars.instruction),
            ("{language}", vars.language),
            ("{filePath}", basename(vars.file_path)),
            ("{prefix}", prefix),
            ("{suffix}", suffix),
        ],
    )
}

pub fn render_commit_message(template: &str, vars: &AiCommitMessagePromptVars) -> String {
    render_placeholders(template, &[("{diff}", vars.diff), ("{recentCommits}", vars.recent_commits)])
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
        persist::write_json(&paths.prompts_dir().join(format!("{AUTO_TAB_PROMPT_ID}.json")), &custom).expect("write override");

        let loaded = load_prompt_template(&paths);

        assert_eq!(loaded, custom);
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 파손된_오버라이드_파일은_무시되고_번들_기본값으로_폴백한다() {
        let paths = AppPaths::new(temp_data_dir("corrupt"));
        let override_path = paths.prompts_dir().join(format!("{AUTO_TAB_PROMPT_ID}.json"));
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

    /// A value substituted early in the placeholder list (here, `{selection}`) that happens to
    /// contain literal placeholder-looking text (`"{instruction}"`) must not be expanded by a
    /// later substitution in the same render call — see [`render_placeholders`]'s doc comment.
    #[test]
    fn 인라인_편집_render_은_치환된_값_안의_플레이스홀더_형태_문자열을_재확장하지_않는다() {
        let rendered = render_inline_edit(
            "sel={selection} instr={instruction}",
            &AiInlineEditPromptVars {
                selection: "const template = \"{instruction}\"",
                instruction: "rename x to y",
                language: "typescript",
                file_path: "src/main.ts",
                prefix: "",
                suffix: "",
            },
        );
        assert_eq!(rendered, "sel=const template = \"{instruction}\" instr=rename x to y");
    }

    #[test]
    fn 커밋_메시지_render_은_diff_안의_recent_commits_플레이스홀더_형태_문자열을_재확장하지_않는다() {
        let rendered = render_commit_message(
            "recent={recentCommits} diff={diff}",
            &AiCommitMessagePromptVars {
                diff: "+ literal text: {recentCommits}",
                recent_commits: "abc1234 fix: bug",
            },
        );
        assert_eq!(rendered, "recent=abc1234 fix: bug diff=+ literal text: {recentCommits}");
    }

    #[test]
    fn 인라인_편집_번들_기본_템플릿은_파싱되고_버전을_가진다() {
        let template = bundled_inline_edit_template();
        assert_eq!(template.version, 1);
        assert!(template.user.contains("{selection}"));
        assert!(template.user.contains("{instruction}"));
    }

    #[test]
    fn 인라인_편집_오버라이드가_없으면_번들_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("inline-edit-missing"));
        let template = load_inline_edit_prompt_template(&paths);
        assert_eq!(template, bundled_inline_edit_template());
    }

    #[test]
    fn 인라인_편집_오버라이드가_있으면_번들_대신_사용된다() {
        let paths = AppPaths::new(temp_data_dir("inline-edit-override"));
        let custom = AiInlineEditPromptTemplate {
            version: 2,
            system: "custom system".to_string(),
            user: "custom user {selection}".to_string(),
        };
        persist::write_json(&paths.prompts_dir().join(format!("{INLINE_EDIT_PROMPT_ID}.json")), &custom).expect("write override");

        let loaded = load_inline_edit_prompt_template(&paths);

        assert_eq!(loaded, custom);
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 인라인_편집_render_은_모든_변수를_치환한다() {
        let rendered = render_inline_edit(
            "sel={selection} instr={instruction} lang={language} file={filePath} pre={prefix} suf={suffix}",
            &AiInlineEditPromptVars {
                selection: "const x = 1",
                instruction: "rename x to y",
                language: "typescript",
                file_path: "/Users/alice/app/src/main.ts",
                prefix: "PRE",
                suffix: "SUF",
            },
        );
        assert_eq!(
            rendered,
            "sel=const x = 1 instr=rename x to y lang=typescript file=main.ts pre=PRE suf=SUF"
        );
    }

    #[test]
    fn 인라인_편집_render_은_prefix_컨텍스트를_상한_문자수로_자른다() {
        let long_prefix: String = "a".repeat(INLINE_EDIT_CONTEXT_CHAR_LIMIT + 500);
        let rendered = render_inline_edit(
            "{prefix}",
            &AiInlineEditPromptVars {
                selection: "",
                instruction: "",
                language: "",
                file_path: "",
                prefix: &long_prefix,
                suffix: "",
            },
        );
        assert_eq!(rendered.chars().count(), INLINE_EDIT_CONTEXT_CHAR_LIMIT);
    }

    #[test]
    fn 인라인_편집_render_은_suffix_컨텍스트를_상한_문자수로_자른다() {
        let long_suffix: String = "b".repeat(INLINE_EDIT_CONTEXT_CHAR_LIMIT + 500);
        let rendered = render_inline_edit(
            "{suffix}",
            &AiInlineEditPromptVars {
                selection: "",
                instruction: "",
                language: "",
                file_path: "",
                prefix: "",
                suffix: &long_suffix,
            },
        );
        assert_eq!(rendered.chars().count(), INLINE_EDIT_CONTEXT_CHAR_LIMIT);
    }

    #[test]
    fn 인라인_편집_render_은_상한_이내의_컨텍스트는_그대로_유지한다() {
        let rendered = render_inline_edit(
            "{prefix}|{suffix}",
            &AiInlineEditPromptVars {
                selection: "",
                instruction: "",
                language: "",
                file_path: "",
                prefix: "short prefix",
                suffix: "short suffix",
            },
        );
        assert_eq!(rendered, "short prefix|short suffix");
    }

    #[test]
    fn 인라인_편집_render_은_멀티바이트_경계에서_안전하게_자른다() {
        let long_prefix: String = "가".repeat(INLINE_EDIT_CONTEXT_CHAR_LIMIT + 10);
        let rendered = render_inline_edit(
            "{prefix}",
            &AiInlineEditPromptVars {
                selection: "",
                instruction: "",
                language: "",
                file_path: "",
                prefix: &long_prefix,
                suffix: "",
            },
        );
        assert_eq!(rendered.chars().count(), INLINE_EDIT_CONTEXT_CHAR_LIMIT);
        assert!(!rendered.contains('\u{FFFD}'));
    }

    #[test]
    fn 커밋_메시지_번들_기본_템플릿은_파싱되고_버전을_가진다() {
        let template = bundled_commit_message_template();
        assert_eq!(template.version, 1);
        assert!(template.user.contains("{diff}"));
        assert!(template.user.contains("{recentCommits}"));
    }

    #[test]
    fn 커밋_메시지_오버라이드가_없으면_번들_기본값을_반환한다() {
        let paths = AppPaths::new(temp_data_dir("commit-message-missing"));
        let template = load_commit_message_prompt_template(&paths);
        assert_eq!(template, bundled_commit_message_template());
    }

    #[test]
    fn 커밋_메시지_오버라이드가_있으면_번들_대신_사용된다() {
        let paths = AppPaths::new(temp_data_dir("commit-message-override"));
        let custom = AiCommitMessagePromptTemplate {
            version: 2,
            system: "custom system".to_string(),
            user: "custom user {diff}".to_string(),
        };
        persist::write_json(&paths.prompts_dir().join(format!("{COMMIT_MESSAGE_PROMPT_ID}.json")), &custom).expect("write override");

        let loaded = load_commit_message_prompt_template(&paths);

        assert_eq!(loaded, custom);
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 커밋_메시지_파손된_오버라이드_파일은_무시되고_번들_기본값으로_폴백한다() {
        let paths = AppPaths::new(temp_data_dir("commit-message-corrupt"));
        let override_path = paths.prompts_dir().join(format!("{COMMIT_MESSAGE_PROMPT_ID}.json"));
        std::fs::create_dir_all(override_path.parent().unwrap()).expect("create dir");
        std::fs::write(&override_path, b"{not json at all").expect("write corrupt override");

        let loaded = load_commit_message_prompt_template(&paths);

        assert_eq!(loaded, bundled_commit_message_template());
        std::fs::remove_dir_all(paths.data_dir).ok();
    }

    #[test]
    fn 커밋_메시지_render_은_diff와_recent_commits를_치환한다() {
        let rendered = render_commit_message(
            "diff={diff} recent={recentCommits}",
            &AiCommitMessagePromptVars {
                diff: "+added line",
                recent_commits: "fix: previous bug",
            },
        );
        assert_eq!(rendered, "diff=+added line recent=fix: previous bug");
    }
}
