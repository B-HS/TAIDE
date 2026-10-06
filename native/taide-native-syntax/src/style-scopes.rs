use std::collections::HashMap;

use taide_native_editor::syntax::TokenKind;

use crate::theme_settings::ThemeSetting;
use crate::tokenizer::{
    FONT_STYLE_BOLD, FONT_STYLE_ITALIC, FONT_STYLE_STRIKETHROUGH, FONT_STYLE_UNDERLINE,
};

const COLOR_PREFIX: char = '#';
const SHORT_COLOR_LENGTHS: [usize; 2] = [3, 4];
const COLOR_STYLE_SEPARATOR: char = '|';
const FONT_STYLE_SEPARATOR: &str = " ";
const FONT_STYLE_LIST_SEPARATOR: char = ',';
const FONT_STYLE_NAMES: [(u32, &str); 4] = [
    (FONT_STYLE_ITALIC, "italic"),
    (FONT_STYLE_BOLD, "bold"),
    (FONT_STYLE_UNDERLINE, "underline"),
    (FONT_STYLE_STRIKETHROUGH, "strikethrough"),
];
const STRIKETHROUGH_ALIAS: &str = "line-through";
const STRIKETHROUGH_NAME: &str = "strikethrough";
const STANDARD_TOKEN_WORDS: [(&str, TokenKind); 4] = [
    ("comment", TokenKind::Comment),
    ("string", TokenKind::String),
    ("regex", TokenKind::Regex),
    ("regexp", TokenKind::Regex),
];
const UNMATCHED_SCOPE: &str = "";

pub(crate) struct StyleScopes {
    scope_by_color_style: HashMap<String, String>,
}

impl StyleScopes {
    pub(crate) fn from_theme(settings: &[ThemeSetting]) -> Self {
        let mut scope_by_color_style = HashMap::new();
        for setting in settings {
            let Some(first_scope) = setting.scope.as_ref().and_then(|scopes| scopes.first()) else {
                continue;
            };
            let Some(style) = setting.settings.as_ref() else {
                continue;
            };
            let Some(color) = style
                .foreground
                .as_deref()
                .and_then(normalize_color)
                .as_deref()
                .and_then(normalize_color)
            else {
                continue;
            };
            let font_style = normalize_font_style_text(style.font_style.as_deref().unwrap_or(""));
            scope_by_color_style
                .entry(color_style_key(&color, &font_style))
                .or_insert_with(|| first_scope.clone());
        }
        Self {
            scope_by_color_style,
        }
    }

    pub(crate) fn scope(&self, color: &str, font_style_bits: u32) -> &str {
        normalize_color(color)
            .and_then(|color| {
                self.scope_by_color_style
                    .get(&color_style_key(&color, &font_style_names(font_style_bits)))
            })
            .map_or(UNMATCHED_SCOPE, String::as_str)
    }
}

pub(crate) fn standard_token_kind(scope: &str) -> TokenKind {
    let bytes = scope.as_bytes();
    (0..bytes.len())
        .filter(|start| *start == 0 || !is_word_byte(bytes[*start - 1]))
        .find_map(|start| {
            STANDARD_TOKEN_WORDS
                .iter()
                .find(|(word, _)| {
                    bytes[start..].starts_with(word.as_bytes())
                        && bytes
                            .get(start + word.len())
                            .is_none_or(|next| !is_word_byte(*next))
                })
                .map(|(_, kind)| *kind)
        })
        .unwrap_or(TokenKind::Other)
}

pub(crate) fn normalize_color(color: &str) -> Option<String> {
    if color.is_empty() {
        return None;
    }
    let lowered = color
        .strip_prefix(COLOR_PREFIX)
        .unwrap_or(color)
        .to_lowercase();
    if SHORT_COLOR_LENGTHS.contains(&lowered.encode_utf16().count()) {
        return Some(
            lowered
                .chars()
                .flat_map(|character| [character, character])
                .collect(),
        );
    }
    Some(lowered)
}

pub(crate) fn normalize_font_style_text(font_style: &str) -> String {
    let requested: Vec<String> = font_style
        .split(|character| character == FONT_STYLE_LIST_SEPARATOR || is_js_whitespace(character))
        .map(str::to_lowercase)
        .map(|name| {
            if name == STRIKETHROUGH_ALIAS {
                STRIKETHROUGH_NAME.to_owned()
            } else {
                name
            }
        })
        .collect();
    FONT_STYLE_NAMES
        .iter()
        .filter(|(_, name)| requested.iter().any(|candidate| candidate == name))
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(FONT_STYLE_SEPARATOR)
}

fn font_style_names(font_style_bits: u32) -> String {
    FONT_STYLE_NAMES
        .iter()
        .filter(|(bit, _)| font_style_bits & bit != 0)
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(FONT_STYLE_SEPARATOR)
}

fn color_style_key(color: &str, font_style: &str) -> String {
    if font_style.is_empty() {
        return color.to_owned();
    }
    format!("{color}{COLOR_STYLE_SEPARATOR}{font_style}")
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

#[cfg(test)]
#[path = "style-scopes-tests.rs"]
mod tests;
