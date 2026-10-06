use std::collections::{BTreeMap, HashMap, HashSet};

use taide_model::theme::{ResolvedTheme, SyntaxStyle, ThemeType, TokenColorRule};
use taide_native_editor::line_tokens::{TokenStyle, TokenStyleTable};

use crate::monaco_token_theme::{MonacoTokenRule, MonacoTokenTheme};
use crate::style_scopes::{normalize_color, normalize_font_style_text, standard_token_kind};
use crate::textmate_tokenizer::TextmateTokenizer;
use crate::theme_settings::{ThemeSetting, ThemeStyle};
use crate::tokenizer::{
    FONT_STYLE_BOLD, FONT_STYLE_ITALIC, FONT_STYLE_STRIKETHROUGH, FONT_STYLE_UNDERLINE,
};

const EDITOR_FOREGROUND_KEY: &str = "editor.foreground";
const EDITOR_BACKGROUND_KEY: &str = "editor.background";
const FALLBACK_EDITOR_FOREGROUND_DARK: &str = "#bbbbbb";
const FALLBACK_EDITOR_FOREGROUND_LIGHT: &str = "#333333";
const FALLBACK_EDITOR_BACKGROUND_DARK: &str = "#1e1e1e";
const FALLBACK_EDITOR_BACKGROUND_LIGHT: &str = "#fffffe";
const COLOR_PREFIX: char = '#';
const REPLACEMENT_COLOR_DIGITS: usize = 8;
const OPAQUE_COLOR_DIGITS: usize = 6;
const TRANSLUCENT_COLOR_DIGITS: usize = 8;
const OPAQUE_ALPHA: u8 = u8::MAX;
const BOLD_FONT_STYLE: &str = "bold";
const ITALIC_FONT_STYLE: &str = "italic";
const FONT_STYLE_SEPARATOR: &str = " ";
const SEMANTIC_SCOPE_PREFIX: &str = "taideSemantic";
const SCOPE_SEPARATOR: char = '.';

const SYNTAX_SCOPE_CANDIDATES: [(&str, &[&str]); 31] = [
    ("keyword", &["keyword.control", "keyword"]),
    ("storage", &["storage.type", "storage.modifier", "storage"]),
    ("operator", &["keyword.operator", "punctuation.separator"]),
    ("string", &["string.quoted", "string"]),
    ("number", &["constant.numeric"]),
    ("regexp", &["string.regexp"]),
    ("comment", &["comment.line", "comment"]),
    ("docComment", &["comment.block.documentation", "comment"]),
    ("function", &["entity.name.function", "support.function"]),
    (
        "method",
        &[
            "entity.name.function.member",
            "meta.function-call",
            "entity.name.function",
        ],
    ),
    (
        "variable",
        &["variable.other.readwrite", "variable.other", "variable"],
    ),
    ("parameter", &["variable.parameter"]),
    (
        "property",
        &[
            "variable.other.property",
            "support.type.property-name",
            "meta.object-literal.key",
        ],
    ),
    ("type", &["support.type", "entity.name.type"]),
    (
        "class",
        &[
            "entity.name.type.class",
            "entity.name.class",
            "support.class",
            "entity.name.type",
        ],
    ),
    (
        "interface",
        &["entity.name.type.interface", "entity.name.type"],
    ),
    ("enum", &["entity.name.type.enum", "entity.name.type"]),
    (
        "constant",
        &["constant.language", "variable.other.constant", "constant"],
    ),
    (
        "namespace",
        &["entity.name.namespace", "entity.name.type.module"],
    ),
    (
        "decorator",
        &[
            "meta.decorator",
            "entity.name.function.decorator",
            "punctuation.decorator",
        ],
    ),
    ("tag", &["entity.name.tag"]),
    ("attribute", &["entity.other.attribute-name"]),
    ("punctuation", &["punctuation.definition", "punctuation"]),
    ("invalid", &["invalid.illegal", "invalid"]),
    ("link", &["markup.underline.link", "string.other.link"]),
    ("markdownHeading", &["markup.heading"]),
    ("markdownEmphasis", &["markup.italic"]),
    ("markdownStrong", &["markup.bold"]),
    ("markdownCode", &["markup.inline.raw", "markup.raw"]),
    ("markdownQuote", &["markup.quote"]),
    (
        "markdownListMarker",
        &["punctuation.definition.list", "markup.list"],
    ),
];

const SEMANTIC_TOKEN_THEME_TARGETS: [&str; 21] = [
    "namespace",
    "type",
    "class",
    "enum",
    "interface",
    "parameter",
    "variable",
    "property",
    "constant",
    "function",
    "method",
    "keyword",
    "comment",
    "string",
    "number",
    "regexp",
    "operator",
    "decorator",
    "storage",
    "invalid",
    "punctuation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenThemeError {
    InvalidSyntaxColor(String),
    IllegalTokenColor(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenTheme {
    settings: Vec<ThemeSetting>,
    editor_foreground: String,
    editor_background: String,
}

impl TokenTheme {
    pub fn from_resolved(theme: &ResolvedTheme) -> Result<Self, TokenThemeError> {
        let rules = token_color_rules(theme)?;
        let (settings, editor_foreground, editor_background) = normalized(theme, rules);
        let token_theme = Self {
            settings,
            editor_foreground,
            editor_background,
        };
        token_theme.monaco_theme()?;
        Ok(token_theme)
    }

    pub fn settings(&self) -> &[ThemeSetting] {
        &self.settings
    }

    pub fn style_table(
        &self,
        tokenizer: &TextmateTokenizer,
    ) -> Result<TokenStyleTable, TokenThemeError> {
        let monaco = self.monaco_theme()?;
        let style = |scope: &str| {
            let matched = monaco.matched(scope);
            let [red, green, blue] = matched.foreground;
            TokenStyle {
                foreground: [red, green, blue, OPAQUE_ALPHA],
                is_italic: matched.font_style_bits & FONT_STYLE_ITALIC != 0,
                is_bold: matched.font_style_bits & FONT_STYLE_BOLD != 0,
                is_underlined: matched.font_style_bits & FONT_STYLE_UNDERLINE != 0,
                is_struck_through: matched.font_style_bits & FONT_STYLE_STRIKETHROUGH != 0,
                kind: standard_token_kind(scope),
            }
        };
        Ok(TokenStyleTable::new(
            style(""),
            (0..tokenizer.style_count())
                .map(|style_id| style(tokenizer.monaco_scope(style_id)))
                .collect(),
        ))
    }

    fn monaco_theme(&self) -> Result<MonacoTokenTheme, TokenThemeError> {
        let default_rule = MonacoTokenRule {
            token: String::new(),
            foreground: Some(self.editor_foreground.clone()),
            background: Some(self.editor_background.clone()),
            font_style: None,
        };
        let rules = self
            .settings
            .iter()
            .filter_map(|setting| {
                let style = setting.settings.as_ref()?;
                let has_style = [&style.foreground, &style.background, &style.font_style]
                    .into_iter()
                    .any(|value| value.as_deref().is_some_and(|value| !value.is_empty()));
                has_style.then_some((setting.scope.as_deref().unwrap_or(&[]), style))
            })
            .flat_map(|(scopes, style)| {
                scopes.iter().map(move |scope| MonacoTokenRule {
                    token: scope.clone(),
                    foreground: style.foreground.as_deref().and_then(normalize_color),
                    background: style.background.as_deref().and_then(normalize_color),
                    font_style: Some(normalize_font_style_text(
                        style.font_style.as_deref().unwrap_or(""),
                    )),
                })
            });
        MonacoTokenTheme::new(std::iter::once(default_rule).chain(rules).collect())
            .map_err(|error| TokenThemeError::IllegalTokenColor(error.0))
    }
}

fn is_theme_color(color: &str) -> bool {
    color.strip_prefix(COLOR_PREFIX).is_some_and(|digits| {
        [OPAQUE_COLOR_DIGITS, TRANSLUCENT_COLOR_DIGITS].contains(&digits.len())
            && digits.bytes().all(|digit| digit.is_ascii_hexdigit())
    })
}

fn syntax_rule(scope: Vec<String>, style: &SyntaxStyle) -> Result<ThemeSetting, TokenThemeError> {
    if !is_theme_color(&style.fg) {
        return Err(TokenThemeError::InvalidSyntaxColor(style.fg.clone()));
    }
    let font_style = [
        (style.bold, BOLD_FONT_STYLE),
        (style.italic, ITALIC_FONT_STYLE),
    ]
    .into_iter()
    .filter_map(|(is_enabled, name)| is_enabled.then_some(name))
    .collect::<Vec<_>>()
    .join(FONT_STYLE_SEPARATOR);
    Ok(ThemeSetting {
        scope: Some(scope),
        settings: Some(ThemeStyle {
            foreground: Some(style.fg.clone()),
            background: None,
            font_style: (!font_style.is_empty()).then_some(font_style),
        }),
    })
}

fn claimed_scope_rules(
    is_included: impl Fn(&str) -> bool,
    syntax: &BTreeMap<String, SyntaxStyle>,
) -> Result<Vec<ThemeSetting>, TokenThemeError> {
    let mut claimed: HashSet<&str> = HashSet::new();
    let mut rules = Vec::new();
    for (token, candidates) in SYNTAX_SCOPE_CANDIDATES {
        if !is_included(token) {
            continue;
        }
        let Some(style) = syntax.get(token) else {
            continue;
        };
        let scope: Vec<&str> = candidates
            .iter()
            .copied()
            .filter(|candidate| !claimed.contains(candidate))
            .collect();
        if scope.is_empty() {
            continue;
        }
        claimed.extend(&scope);
        rules.push(syntax_rule(
            scope.into_iter().map(str::to_owned).collect(),
            style,
        )?);
    }
    Ok(rules)
}

fn raw_rule(rule: &TokenColorRule) -> ThemeSetting {
    let kept = |value: &Option<String>| value.clone().filter(|value| !value.is_empty());
    ThemeSetting {
        scope: Some(rule.scope.clone()),
        settings: Some(ThemeStyle {
            foreground: kept(&rule.settings.foreground),
            background: kept(&rule.settings.background),
            font_style: kept(&rule.settings.font_style),
        }),
    }
}

fn token_color_rules(theme: &ResolvedTheme) -> Result<Vec<ThemeSetting>, TokenThemeError> {
    let mut rules = match &theme.token_colors {
        Some(token_colors) => token_colors.iter().map(raw_rule).collect(),
        None => claimed_scope_rules(|_| true, &theme.syntax)?,
    };
    rules.extend(claimed_scope_rules(
        |token| {
            theme
                .syntax_overrides
                .iter()
                .any(|overridden| overridden == token)
        },
        &theme.syntax,
    )?);
    for token in SEMANTIC_TOKEN_THEME_TARGETS {
        let Some(style) = theme.syntax.get(token) else {
            continue;
        };
        rules.push(syntax_rule(
            vec![format!("{SEMANTIC_SCOPE_PREFIX}{SCOPE_SEPARATOR}{token}")],
            style,
        )?);
    }
    Ok(rules)
}

#[derive(Default)]
struct ColorReplacements {
    replacement_by_color: HashMap<Option<String>, String>,
}

impl ColorReplacements {
    fn replacement(&mut self, color: Option<&str>) -> String {
        let next = self.replacement_by_color.len() + 1;
        self.replacement_by_color
            .entry(color.map(str::to_owned))
            .or_insert_with(|| {
                format!(
                    "{COLOR_PREFIX}{next:0width$x}",
                    width = REPLACEMENT_COLOR_DIGITS
                )
            })
            .clone()
    }

    fn replaced(&mut self, color: Option<String>) -> Option<String> {
        match color {
            Some(color) if !color.is_empty() && !color.starts_with(COLOR_PREFIX) => {
                Some(self.replacement(Some(&color)))
            }
            other => other,
        }
    }

    fn editor_color(&mut self, color: Option<&str>) -> String {
        match color {
            Some(color) if color.starts_with(COLOR_PREFIX) => color.to_owned(),
            other => self.replacement(other),
        }
    }
}

fn normalized(
    theme: &ResolvedTheme,
    rules: Vec<ThemeSetting>,
) -> (Vec<ThemeSetting>, String, String) {
    let editor_color = |key: &str| theme.colors.get(key).map(String::as_str);
    let is_light = theme.theme_type == ThemeType::Light;
    let foreground = editor_color(EDITOR_FOREGROUND_KEY)
        .filter(|color| !color.is_empty())
        .unwrap_or(if is_light {
            FALLBACK_EDITOR_FOREGROUND_LIGHT
        } else {
            FALLBACK_EDITOR_FOREGROUND_DARK
        });
    let background = editor_color(EDITOR_BACKGROUND_KEY)
        .filter(|color| !color.is_empty())
        .unwrap_or(if is_light {
            FALLBACK_EDITOR_BACKGROUND_LIGHT
        } else {
            FALLBACK_EDITOR_BACKGROUND_DARK
        });
    let global = ThemeSetting {
        scope: None,
        settings: Some(ThemeStyle {
            foreground: Some(foreground.to_owned()),
            background: Some(background.to_owned()),
            font_style: None,
        }),
    };
    let mut replacements = ColorReplacements::default();
    let settings = std::iter::once(global)
        .chain(rules)
        .map(|setting| ThemeSetting {
            scope: setting.scope,
            settings: setting.settings.map(|style| {
                let foreground = replacements.replaced(style.foreground);
                let background = replacements.replaced(style.background);
                ThemeStyle {
                    foreground,
                    background,
                    font_style: style.font_style,
                }
            }),
        })
        .collect();
    let background = replacements.editor_color(editor_color(EDITOR_BACKGROUND_KEY));
    let foreground = replacements.editor_color(editor_color(EDITOR_FOREGROUND_KEY));
    (
        settings,
        monaco_color(&foreground),
        monaco_color(&background),
    )
}

fn monaco_color(color: &str) -> String {
    format!(
        "{COLOR_PREFIX}{}",
        normalize_color(color).unwrap_or_default()
    )
}

#[cfg(test)]
#[path = "token-theme-tests.rs"]
mod tests;
