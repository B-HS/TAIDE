use taide_model::theme::ThemeType;

use super::color::{delta_e76, hex_to_rgb, js_length, parse_hex_prefix, suffix, ALPHA_CHANNEL_MAX, HEX_ALPHA_LENGTH, OPAQUE_HEX_LENGTH};
use super::tables::{
    default_ansi_palette, list_active_selection_default, list_hover_default, list_inactive_selection_default, Category, Rule,
    COLOR_MAPPING, MATCH_HIGHLIGHT_FOREGROUND_CANDIDATES, TERMINAL_ANSI_TOKENS,
};
use super::Colors;

const SELF_REFERENCE_PREFIX: char = '@';
const TERMINAL_ANSI_CANDIDATE_PREFIX: &str = "terminal.ansi";
const TRANSPARENT: &str = "transparent";
const MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E: f64 = 2.3;
const SHORT_HEX_ALPHA_LENGTH: usize = 5;
const SHORT_ALPHA_CHANNEL_MAX: f64 = 15.0;
const BASE_ANSI_COUNT: usize = 8;
const BRIGHT_PREFIX: &str = "bright";

/// The 16 ANSI colors in [`TERMINAL_ANSI_TOKENS`] order.
pub type Ansi = Vec<(&'static str, String)>;

fn ansi_color<'a>(ansi: &'a Ansi, name: &str) -> Option<&'a str> {
    ansi.iter().find(|(token, _)| *token == name).map(|(_, value)| value.as_str())
}

/// A candidate is unusable when it is fully transparent or has the row background's exact RGB.
pub fn is_usable_list_background(candidate: Option<&str>, background: Option<&str>) -> bool {
    let Some(candidate) = candidate.filter(|candidate| !candidate.is_empty()) else {
        return false;
    };
    let alpha = if js_length(candidate) == HEX_ALPHA_LENGTH {
        parse_hex_prefix(suffix(candidate, OPAQUE_HEX_LENGTH))
    } else {
        Some(ALPHA_CHANNEL_MAX)
    };
    if alpha == Some(0.0) {
        return false;
    }
    let Some(background) = background.filter(|background| !background.is_empty()) else {
        return true;
    };
    match (hex_to_rgb(candidate), hex_to_rgb(background)) {
        (Some(candidate), Some(background)) => candidate != background,
        _ => true,
    }
}

/// A foreground candidate is usable only when fully opaque.
pub fn is_opaque_foreground_candidate(candidate: Option<&str>) -> bool {
    let Some(candidate) = candidate.filter(|candidate| !candidate.is_empty()) else {
        return false;
    };
    let length = js_length(candidate);
    if length == SHORT_HEX_ALPHA_LENGTH {
        return parse_hex_prefix(suffix(candidate, length - 1)) == Some(SHORT_ALPHA_CHANNEL_MAX);
    }
    if length != HEX_ALPHA_LENGTH {
        return true;
    }
    parse_hex_prefix(suffix(candidate, OPAQUE_HEX_LENGTH)) == Some(ALPHA_CHANNEL_MAX)
}

/// A match highlight must read as a different color from the body text; unparseable input passes.
pub fn is_distinct_from_body_foreground(candidate: &str, body_foreground: &str) -> bool {
    delta_e76(candidate, body_foreground).is_none_or(|distance| distance >= MATCH_HIGHLIGHT_MIN_DISTINCT_DELTA_E)
}

/// Resolves the 16 ANSI colors, falling back to VS Code's defaults for the missing ones.
pub fn resolve_ansi(vscode_colors: &Colors, theme_type: ThemeType) -> (Ansi, Vec<String>) {
    let defaults = default_ansi_palette(theme_type);
    let mut fallback_tokens = Vec::new();
    let ansi = TERMINAL_ANSI_TOKENS
        .iter()
        .zip(defaults)
        .enumerate()
        .map(|(index, (token, default))| {
            let base = TERMINAL_ANSI_TOKENS[index % BASE_ANSI_COUNT];
            let capitalized = capitalize(base);
            let source_key = if token.starts_with(BRIGHT_PREFIX) {
                format!("{TERMINAL_ANSI_CANDIDATE_PREFIX}Bright{capitalized}")
            } else {
                format!("{TERMINAL_ANSI_CANDIDATE_PREFIX}{capitalized}")
            };
            match vscode_colors.get(&source_key).filter(|value| !value.is_empty()) {
                Some(value) => (*token, value.clone()),
                None => {
                    fallback_tokens.push((*token).into());
                    (*token, default.into())
                }
            }
        })
        .collect();
    (ansi, fallback_tokens)
}

fn capitalize(value: &str) -> String {
    let mut characters = value.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

fn ansi_name_from_candidate(candidate: &str) -> Option<&'static str> {
    let rest = candidate.strip_prefix(TERMINAL_ANSI_CANDIDATE_PREFIX)?;
    let mut characters = rest.chars();
    let name = characters
        .next()
        .map(|first| first.to_lowercase().chain(characters).collect::<String>())?;
    TERMINAL_ANSI_TOKENS.iter().find(|token| **token == name).copied()
}

struct Context<'a> {
    vscode_colors: &'a Colors,
    resolved: Colors,
    ansi: &'a Ansi,
    theme_type: ThemeType,
}

impl Context<'_> {
    fn candidate(&self, candidate: &str) -> Option<String> {
        if let Some(key) = candidate.strip_prefix(SELF_REFERENCE_PREFIX) {
            return self.resolved.get(key).cloned();
        }
        match ansi_name_from_candidate(candidate) {
            Some(name) => ansi_color(self.ansi, name).map(str::to_owned),
            None => self.vscode_colors.get(candidate).cloned(),
        }
    }

    fn vscode(&self, key: &str) -> Option<&str> {
        self.vscode_colors.get(key).map(String::as_str)
    }

    fn resolved(&self, key: &str) -> Option<&str> {
        self.resolved.get(key).map(String::as_str)
    }

    fn family_fallback(&self, category: Category) -> Option<String> {
        category
            .family_fallback_keys()
            .iter()
            .find_map(|key| self.vscode_colors.get(*key).cloned())
    }

    fn derive(&self, rule: Rule) -> Option<String> {
        let usable_or = |candidate: Option<&str>, background: Option<&str>, default: &str| {
            Some(if is_usable_list_background(candidate, background) {
                candidate.unwrap_or_default().to_owned()
            } else {
                default.to_owned()
            })
        };
        match rule {
            Rule::Chain(_) => None,
            Rule::Ansi(name) => ansi_color(self.ansi, name).map(str::to_owned),
            Rule::Transparent => Some(TRANSPARENT.into()),
            Rule::ListHover(background) => usable_or(
                self.vscode("list.hoverBackground"),
                self.resolved(background),
                list_hover_default(self.theme_type),
            ),
            Rule::ListSelection(background) => usable_or(
                self.vscode("list.activeSelectionBackground"),
                self.resolved(background),
                list_active_selection_default(self.theme_type),
            ),
            Rule::ExplorerFocused => {
                let background = self.resolved("explorer.background");
                Some(
                    [self.vscode("list.focusBackground"), self.vscode("list.inactiveSelectionBackground")]
                        .into_iter()
                        .find(|candidate| is_usable_list_background(*candidate, background))
                        .flatten()
                        .unwrap_or(list_inactive_selection_default(self.theme_type))
                        .to_owned(),
                )
            }
            Rule::ListActive => {
                let candidate = self.vscode("list.activeSelectionBackground");
                let is_usable = is_usable_list_background(candidate, self.resolved("list.background"))
                    && is_usable_list_background(candidate, self.resolved("list.hoverBackground"));
                Some(if is_usable {
                    candidate.unwrap_or_default().to_owned()
                } else {
                    list_active_selection_default(self.theme_type).to_owned()
                })
            }
            Rule::MatchHighlight => {
                let body = self.resolved("app.foreground").unwrap_or_default();
                MATCH_HIGHLIGHT_FOREGROUND_CANDIDATES
                    .iter()
                    .map(|key| self.vscode(key))
                    .filter(|candidate| is_opaque_foreground_candidate(*candidate))
                    .flatten()
                    .find(|candidate| is_distinct_from_body_foreground(candidate, body))
                    .map(str::to_owned)
            }
            Rule::RefBranch => self
                .vscode("gitDecoration.modifiedResourceForeground")
                .or_else(|| ansi_color(self.ansi, "blue"))
                .map(str::to_owned),
        }
    }

    fn entry(&self, category: Category, rule: Rule) -> Option<String> {
        let Rule::Chain(candidates) = rule else {
            return self.derive(rule).or_else(|| self.family_fallback(category));
        };
        candidates
            .iter()
            .filter_map(|candidate| self.candidate(candidate))
            .find(|value| !value.is_empty())
            .or_else(|| self.family_fallback(category))
    }
}

/// Resolves every TAIDE color token and lists the ones that fell back to a safe default.
pub fn resolve_colors(vscode_colors: &Colors, theme_type: ThemeType, ansi: &Ansi) -> (Colors, Vec<String>) {
    let mut context = Context {
        vscode_colors,
        resolved: Colors::new(),
        ansi,
        theme_type,
    };
    let mut notices = Vec::new();
    for (key, category, rule) in COLOR_MAPPING {
        let resolved = context.entry(*category, *rule).unwrap_or_else(|| {
            notices.push(format!("{key} (category: {})", category.label()));
            category.safe_default(theme_type).into()
        });
        context.resolved.insert((*key).into(), resolved);
    }
    (context.resolved, notices)
}
