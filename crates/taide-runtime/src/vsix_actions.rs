use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use taide_model::error::AppResult;
use taide_model::plugin::LoadedPlugin;
use taide_model::theme::{Theme, ThemeType};
use taide_model::vsix::{VsixExtensionInfo, VsixExtractedTheme, VsixThemeCandidate, VsixThemeExtractionResult, VsixThemeImportFailure};
use taide_theme::convert::{convert_vscode_theme, jsonc::parse_jsonc};
use taide_vsix::service;

use crate::plugin_install_worker::{run_install, stage_plugin};
use crate::{AppState, TaskSupervisor};

const THEME_SCHEMA_VERSION: u32 = 1;
const FALLBACK_THEME_ID_SLUG: &str = "imported-theme";
const FALLBACK_THEME_LABEL_SLUG: &str = "theme";
const DEDUPE_SUFFIX_START: u32 = 2;
const LIGHT_UI_THEMES: [&str; 2] = ["vs", "hc-light"];

/// Lowercases a name and joins its ASCII letter and digit runs with `-`.
pub fn slugify_theme_id(name: &str) -> String {
    let mut slug = String::new();
    for character in name.trim().to_lowercase().chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            slug.push(character);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

fn extension_slug(extension: &VsixExtensionInfo) -> String {
    let identity = [extension.publisher.as_str(), extension.name.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = slugify_theme_id(&identity);
    if slug.is_empty() {
        slugify_theme_id(&extension.display_name)
    } else {
        slug
    }
}

fn candidate_id(extension: &VsixExtensionInfo, label: &str, is_multiple: bool, used: &HashSet<String>) -> String {
    let base = Some(extension_slug(extension))
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| FALLBACK_THEME_ID_SLUG.into());
    let with_label = if is_multiple {
        let label = Some(slugify_theme_id(label))
            .filter(|slug| !slug.is_empty())
            .unwrap_or_else(|| FALLBACK_THEME_LABEL_SLUG.into());
        format!("{base}-{label}")
    } else {
        base
    };
    if !used.contains(&with_label) {
        return with_label;
    }
    let mut suffix = DEDUPE_SUFFIX_START;
    while used.contains(&format!("{with_label}-{suffix}")) {
        suffix += 1;
    }
    format!("{with_label}-{suffix}")
}

fn raw_chain(extracted: &VsixExtractedTheme) -> Option<Vec<serde_json::Value>> {
    extracted
        .include_chain
        .iter()
        .rev()
        .map(|entry| entry.raw_json.as_str())
        .chain([extracted.raw_json.as_str()])
        .map(|source| parse_jsonc(source).ok())
        .collect()
}

/// Converts each extracted VSIX theme to an import candidate with a stable, collision-aware id.
pub fn vsix_theme_candidates(result: &VsixThemeExtractionResult, existing_ids: &[String]) -> Vec<VsixThemeCandidate> {
    let mut used = HashSet::new();
    let is_multiple = result.themes.len() > 1;
    result
        .themes
        .iter()
        .enumerate()
        .map(|(index, extracted)| {
            let id = candidate_id(&result.extension, &extracted.label, is_multiple, &used);
            used.insert(id.clone());
            let theme_type = if LIGHT_UI_THEMES.contains(&extracted.ui_theme.as_str()) {
                ThemeType::Light
            } else {
                ThemeType::Dark
            };
            let conversion = raw_chain(extracted).map(|chain| convert_vscode_theme(&chain, theme_type));
            let failure = match &conversion {
                None => Some(VsixThemeImportFailure::Parse),
                Some(conversion) if !conversion.is_complete => Some(VsixThemeImportFailure::Incomplete),
                Some(conversion) if !conversion.output_color_errors.is_empty() => Some(VsixThemeImportFailure::Contrast),
                Some(_) => None,
            };
            let warning_count = conversion.as_ref().map_or(0, |conversion| {
                conversion.safe_default_notices.len()
                    + conversion.ansi_fallback_tokens.len()
                    + conversion.repairs.len()
                    + conversion.state_distinctness_errors.len()
            });
            let source = [result.extension.display_name.as_str(), result.extension.version.as_str()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let theme = conversion.filter(|_| failure.is_none()).map(|conversion| Theme {
                version: THEME_SCHEMA_VERSION,
                id: id.clone(),
                name: extracted.label.clone(),
                theme_type,
                extends: None,
                palette: BTreeMap::new(),
                colors: conversion.colors,
                syntax: conversion.syntax,
                terminal: conversion.terminal,
                token_colors: Some(conversion.token_colors),
                author: Some(result.extension.publisher.clone()).filter(|publisher| !publisher.is_empty()),
                license: None,
                source: Some(source).filter(|source| !source.is_empty()),
            });
            VsixThemeCandidate {
                key: format!("{index}-{}", extracted.label),
                label: extracted.label.clone(),
                id_collides: existing_ids.contains(&id),
                id,
                theme_type,
                warning_count,
                theme,
                failure,
            }
        })
        .collect()
}

/// Applies the shared vsix extract themes policy.
pub async fn vsix_extract_themes(vsix_path: String) -> AppResult<VsixThemeExtractionResult> {
    service::extract_themes(Path::new(&vsix_path))
}

/// Applies the shared vsix import plugin policy.
pub async fn vsix_import_plugin(
    state: &AppState,
    tasks: &TaskSupervisor,
    commit_staged_import: impl FnOnce(&Path, &str) -> AppResult<LoadedPlugin> + Send + 'static,
    vsix_path: String,
) -> AppResult<LoadedPlugin> {
    let state = state.clone();
    let worker_tasks = tasks.clone();
    run_install(tasks, "vsix-import", async move {
        let plugins_dir = state.paths.plugins_dir();
        let vsix_path = PathBuf::from(&vsix_path);
        let staged = stage_plugin(&worker_tasks, move || service::stage_vsix_import(&plugins_dir, &vsix_path)).await?;
        let _guard = state.begin_mutation().await;
        commit_staged_import(&staged.path, &staged.id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use taide_model::theme::{TokenColorRule, TokenColorSettings};
    use taide_model::vsix::VsixThemeIncludeEntry;

    use super::*;

    const EDITOR_BACKGROUND: &str = "#1e1e1e";
    const BODY_TEXT: &str = "#d4d4d4";

    fn minimal_json() -> String {
        json!({ "colors": { "editor.background": EDITOR_BACKGROUND, "editor.foreground": BODY_TEXT, "foreground": BODY_TEXT } }).to_string()
    }

    fn extracted(label: &str, ui_theme: &str, raw_json: String) -> VsixExtractedTheme {
        VsixExtractedTheme {
            label: label.into(),
            ui_theme: ui_theme.into(),
            raw_json,
            include_chain: Vec::new(),
        }
    }

    fn result(themes: Vec<VsixExtractedTheme>) -> VsixThemeExtractionResult {
        VsixThemeExtractionResult {
            extension: VsixExtensionInfo {
                name: "my-cool-theme".into(),
                display_name: "My Cool Theme".into(),
                publisher: "someone".into(),
                version: "1.0.0".into(),
            },
            themes,
        }
    }

    fn single() -> VsixThemeExtractionResult {
        result(vec![extracted("My Cool Theme", "vs-dark", minimal_json())])
    }

    #[test]
    fn 단일_테마는_publisher_name_slug_id를_받고_기존_id면_충돌로_표시한다() {
        let [candidate] = <[_; 1]>::try_from(vsix_theme_candidates(&single(), &[])).unwrap();
        assert_eq!(candidate.id, "someone-my-cool-theme");
        assert_eq!(candidate.key, "0-My Cool Theme");
        assert_eq!(candidate.theme_type, ThemeType::Dark);
        assert_eq!(candidate.failure, None);
        let theme = candidate.theme.unwrap();
        assert_eq!(theme.id, "someone-my-cool-theme");
        assert_eq!(theme.author.as_deref(), Some("someone"));
        assert_eq!(theme.source.as_deref(), Some("My Cool Theme 1.0.0"));
        assert!(vsix_theme_candidates(&single(), &["someone-my-cool-theme".into()])[0].id_collides);
    }

    #[test]
    fn id는_nls_표시명과_무관하고_여러_테마는_라벨을_붙이며_같은_id는_번호를_붙인다() {
        let mut nls = single();
        nls.extension = VsixExtensionInfo {
            name: "github-theme".into(),
            display_name: "%displayName%".into(),
            publisher: "github".into(),
            version: "1.0.0".into(),
        };
        assert_eq!(vsix_theme_candidates(&nls, &[])[0].id, "github-github-theme");
        let candidates = vsix_theme_candidates(
            &result(vec![
                extracted("My Cool Theme", "vs-dark", minimal_json()),
                extracted("My Cool Theme Light", "vs", minimal_json()),
                extracted("My Cool Theme", "hc-light", minimal_json()),
            ]),
            &[],
        );
        assert_eq!(
            candidates.iter().map(|candidate| candidate.id.as_str()).collect::<Vec<_>>(),
            [
                "someone-my-cool-theme-my-cool-theme",
                "someone-my-cool-theme-my-cool-theme-light",
                "someone-my-cool-theme-my-cool-theme-2",
            ]
        );
        assert_eq!(candidates[1].theme_type, ThemeType::Light);
        assert_eq!(candidates[2].theme_type, ThemeType::Light);
    }

    #[test]
    fn 파싱_실패는_parse이고_token_colors와_include_체인은_테마에_실린다() {
        let broken = vsix_theme_candidates(&result(vec![extracted("Broken", "vs-dark", "{ not json".into())]), &[]);
        assert_eq!(broken[0].failure, Some(VsixThemeImportFailure::Parse));
        assert!(broken[0].theme.is_none());
        let tokens = json!({
            "colors": { "editor.background": EDITOR_BACKGROUND, "editor.foreground": BODY_TEXT, "foreground": BODY_TEXT },
            "tokenColors": [{ "scope": "comment", "settings": { "foreground": "#6a9955", "fontStyle": "italic" } }],
        });
        let candidate = &vsix_theme_candidates(&result(vec![extracted("Tokens", "vs-dark", tokens.to_string())]), &[])[0];
        assert!(candidate
            .theme
            .as_ref()
            .unwrap()
            .token_colors
            .as_ref()
            .unwrap()
            .contains(&TokenColorRule {
                scope: vec!["comment".into()],
                settings: TokenColorSettings {
                    foreground: Some("#6a9955".into()),
                    background: None,
                    font_style: Some("italic".into()),
                },
            }));
        let base = json!({ "colors": { "editor.background": "#000000", "editor.foreground": BODY_TEXT, "foreground": BODY_TEXT } });
        let leaf = json!({ "colors": { "editor.background": "#222222" }, "include": "./base.json" });
        let mut derived = extracted("Derived", "vs-dark", leaf.to_string());
        derived.include_chain = vec![VsixThemeIncludeEntry {
            path: "base.json".into(),
            raw_json: base.to_string(),
        }];
        let candidate = &vsix_theme_candidates(&result(vec![derived]), &[])[0];
        assert_eq!(candidate.theme.as_ref().unwrap().colors["editor.background"], "#222222");
    }

    #[test]
    fn 수리_뒤_남은_상태색_구별성_위반은_경고_수에_더해진다() {
        let terminal = |selection: &str, cursor: &str| {
            json!({ "colors": {
                "editor.background": EDITOR_BACKGROUND,
                "editor.foreground": BODY_TEXT,
                "foreground": BODY_TEXT,
                "terminal.background": BODY_TEXT,
                "terminal.selectionBackground": selection,
                "terminalCursor.foreground": cursor,
            } })
        };
        let candidate = |selection: &str, cursor: &str| {
            vsix_theme_candidates(
                &result(vec![extracted("Terminal", "vs-dark", terminal(selection, cursor).to_string())]),
                &[],
            )
            .remove(0)
        };
        let residual = |selection: &str, cursor: &str| {
            convert_vscode_theme(&[terminal(selection, cursor)], ThemeType::Dark)
                .state_distinctness_errors
                .len()
        };
        let distinct = candidate(EDITOR_BACKGROUND, EDITOR_BACKGROUND);
        let collapsed = candidate(BODY_TEXT, BODY_TEXT);
        let added = residual(BODY_TEXT, BODY_TEXT) - residual(EDITOR_BACKGROUND, EDITOR_BACKGROUND);
        assert!(added > 0);
        assert_eq!(distinct.failure, None);
        assert_eq!(collapsed.failure, None);
        assert_eq!(collapsed.warning_count - distinct.warning_count, added);
    }
}
