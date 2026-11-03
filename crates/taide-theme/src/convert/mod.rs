use std::collections::BTreeMap;

use serde_json::Value;
use taide_model::theme::{SyntaxStyle, ThemeType, TokenColorRule, TokenColorSettings};

pub mod color;
pub mod contrast;
pub mod jsonc;
pub mod mapping;
pub mod state;
pub mod tables;

use color::{composite_over_background, is_hex_color, js_length, HEX_ALPHA_LENGTH};
use jsonc::expand_vscode_hex;
use tables::{COLOR_NAMESPACES, SYNTAX_SCOPE_CANDIDATES, TERMINAL_ANSI_TOKENS, TERMINAL_MIRRORED_TOKENS};

/// Color tokens keyed by their dotted name.
pub type Colors = BTreeMap<String, String>;

const FALLBACK_BACKGROUND: &str = "#000000";
const FALLBACK_FOREGROUND: &str = "#000000";
const SCOPE_SEPARATOR: char = ',';
const BOLD: &str = "bold";
const ITALIC: &str = "italic";

#[derive(Clone, Debug, Default, PartialEq)]
struct TokenRule {
    scopes: Vec<String>,
    foreground: Option<String>,
    background: Option<String>,
    font_style: Option<String>,
    is_bold: bool,
    is_italic: bool,
}

#[derive(Default)]
struct VscodeTheme {
    colors: Colors,
    token_colors: Vec<TokenRule>,
}

/// Result of converting one VS Code theme to the TAIDE token shape.
#[derive(Clone, Debug, PartialEq)]
pub struct ConversionResult {
    pub is_complete: bool,
    pub colors: Colors,
    pub syntax: BTreeMap<String, SyntaxStyle>,
    pub terminal: Colors,
    pub token_colors: Vec<TokenColorRule>,
    pub missing_colors: Vec<String>,
    pub missing_syntax: Vec<String>,
    pub missing_terminal: Vec<String>,
    pub safe_default_notices: Vec<String>,
    pub ansi_fallback_tokens: Vec<String>,
    pub repairs: Vec<String>,
    pub output_color_errors: Vec<String>,
    pub state_distinctness_errors: Vec<String>,
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_f64().is_some_and(|number| number != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn entries(value: Option<&Value>) -> Vec<(String, &Value)> {
    match value {
        Some(Value::Object(map)) => map.iter().map(|(key, value)| (key.clone(), value)).collect(),
        Some(Value::Array(items)) => items.iter().enumerate().map(|(index, value)| (index.to_string(), value)).collect(),
        _ => Vec::new(),
    }
}

fn read_token_rule(entry: &Value) -> Option<TokenRule> {
    if !matches!(entry, Value::Object(_) | Value::Array(_)) {
        return None;
    }
    let settings = entry.get("settings").filter(|settings| is_truthy(settings))?;
    let text = |name: &str| settings.get(name).and_then(Value::as_str);
    let scopes = match entry.get("scope") {
        Some(Value::String(scope)) => scope.split(SCOPE_SEPARATOR).map(str::to_owned).collect(),
        Some(Value::Array(scopes)) => scopes.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        _ => Vec::new(),
    }
    .into_iter()
    .map(|scope| scope.trim().to_owned())
    .filter(|scope| !scope.is_empty())
    .collect();
    let font_style = text("fontStyle").map(str::to_owned);
    let style = font_style.clone().unwrap_or_default();
    Some(TokenRule {
        scopes,
        foreground: text("foreground").map(expand_vscode_hex),
        background: text("background").map(expand_vscode_hex),
        is_bold: style.contains(BOLD),
        is_italic: style.contains(ITALIC),
        font_style,
    })
}

fn read_vscode_theme(raw: &Value) -> VscodeTheme {
    VscodeTheme {
        colors: entries(raw.get("colors"))
            .into_iter()
            .filter_map(|(key, value)| value.as_str().map(|value| (key, expand_vscode_hex(value))))
            .collect(),
        token_colors: match raw.get("tokenColors") {
            Some(Value::Array(rules)) => rules.iter().filter_map(read_token_rule).collect(),
            _ => Vec::new(),
        },
    }
}

fn merge_chain(raw_chain: &[Value]) -> VscodeTheme {
    raw_chain
        .iter()
        .map(read_vscode_theme)
        .fold(VscodeTheme::default(), |mut merged, theme| {
            merged.colors.extend(theme.colors);
            merged.token_colors.extend(theme.token_colors);
            merged
        })
}

fn find_best_rule<'a>(candidate_scope: &str, rules: &'a [TokenRule]) -> Option<&'a TokenRule> {
    let mut best: Option<(&TokenRule, usize)> = None;
    for rule in rules {
        for scope in &rule.scopes {
            let matches = candidate_scope == scope
                || candidate_scope
                    .strip_prefix(scope.as_str())
                    .is_some_and(|rest| rest.starts_with('.'));
            let length = js_length(scope);
            if matches && rule.foreground.as_ref().is_some_and(|fg| !fg.is_empty()) && best.is_none_or(|(_, best)| length >= best) {
                best = Some((rule, length));
            }
        }
    }
    best.map(|(rule, _)| rule)
}

fn resolve_syntax(rules: &[TokenRule], editor_foreground: &str, editor_background: &str) -> BTreeMap<String, SyntaxStyle> {
    SYNTAX_SCOPE_CANDIDATES
        .iter()
        .map(|(token, candidates)| {
            let matched = candidates.iter().find_map(|candidate| find_best_rule(candidate, rules));
            let fg = matched
                .and_then(|rule| rule.foreground.clone())
                .unwrap_or_else(|| editor_foreground.into());
            let fg = if is_hex_color(&fg) && js_length(&fg) == HEX_ALPHA_LENGTH {
                composite_over_background(&fg, editor_background)
            } else {
                fg
            };
            (
                (*token).to_owned(),
                SyntaxStyle {
                    fg,
                    bold: matched.is_some_and(|rule| rule.is_bold),
                    italic: matched.is_some_and(|rule| rule.is_italic),
                },
            )
        })
        .collect()
}

fn token_colors(rules: Vec<TokenRule>) -> Vec<TokenColorRule> {
    rules
        .into_iter()
        .map(|rule| TokenColorRule {
            scope: rule.scopes,
            settings: TokenColorSettings {
                foreground: rule.foreground,
                background: rule.background,
                font_style: rule.font_style.filter(|style| !style.is_empty()),
            },
        })
        .filter(|rule| rule.settings.foreground.is_some() || rule.settings.background.is_some() || rule.settings.font_style.is_some())
        .collect()
}

/// Converts a parsed VS Code theme include chain (base first, most specific last) to TAIDE tokens.
pub fn convert_vscode_theme(raw_chain: &[Value], theme_type: ThemeType) -> ConversionResult {
    let theme = merge_chain(raw_chain);
    let (ansi, ansi_fallback_tokens) = mapping::resolve_ansi(&theme.colors, theme_type);
    let (mut colors, safe_default_notices) = mapping::resolve_colors(&theme.colors, theme_type, &ansi);
    let mut repairs = state::repair_state_distinctness(&mut colors);
    repairs.extend(contrast::repair_contrast_pairs(&mut colors, &theme.colors));
    let editor_background = colors
        .get("editor.background")
        .or_else(|| theme.colors.get("editor.background"))
        .map_or(FALLBACK_BACKGROUND, String::as_str)
        .to_owned();
    let editor_foreground = colors
        .get("editor.foreground")
        .or_else(|| theme.colors.get("editor.foreground"))
        .or_else(|| theme.colors.get("foreground"))
        .map_or(FALLBACK_FOREGROUND, String::as_str)
        .to_owned();
    let syntax = resolve_syntax(&theme.token_colors, &editor_foreground, &editor_background);
    let mut terminal = ansi.into_iter().map(|(token, value)| (token.to_owned(), value)).collect::<Colors>();
    for (terminal_key, color_key) in TERMINAL_MIRRORED_TOKENS {
        if let Some(value) = colors.get(color_key) {
            terminal.insert(terminal_key.into(), value.clone());
        }
    }
    let is_present = |map: &Colors, key: &str| map.get(key).is_some_and(|value| !value.is_empty());
    let missing_colors = COLOR_NAMESPACES
        .iter()
        .flat_map(|(namespace, tokens)| tokens.iter().map(move |token| format!("{namespace}.{token}")))
        .filter(|key| !is_present(&colors, key))
        .collect::<Vec<_>>();
    let missing_syntax = SYNTAX_SCOPE_CANDIDATES
        .iter()
        .map(|(token, _)| *token)
        .filter(|token| syntax.get(*token).is_none_or(|style| style.fg.is_empty()))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let missing_terminal = TERMINAL_ANSI_TOKENS
        .iter()
        .chain(TERMINAL_MIRRORED_TOKENS.iter().map(|(key, _)| key))
        .filter(|key| !is_present(&terminal, key))
        .map(|key| (*key).to_owned())
        .collect::<Vec<_>>();
    let output_color_errors = contrast::validate_output_colors(&colors);
    let state_distinctness_errors = state::validate_state_distinctness(&colors);
    ConversionResult {
        is_complete: missing_colors.is_empty() && missing_syntax.is_empty() && missing_terminal.is_empty(),
        token_colors: token_colors(theme.token_colors),
        colors,
        syntax,
        terminal,
        missing_colors,
        missing_syntax,
        missing_terminal,
        safe_default_notices,
        ansi_fallback_tokens,
        repairs,
        output_color_errors,
        state_distinctness_errors,
    }
}

#[cfg(test)]
mod tests;
