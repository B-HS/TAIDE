use std::cmp::Ordering;
use std::collections::HashMap;

use crate::tokenizer::{
    FONT_STYLE_BOLD, FONT_STYLE_ITALIC, FONT_STYLE_STRIKETHROUGH, FONT_STYLE_UNDERLINE,
};

const SCOPE_SEPARATOR: char = '.';
const FONT_STYLE_SEGMENT_SEPARATOR: char = ' ';
const FONT_STYLE_SEGMENTS: [(&str, u32); 4] = [
    ("italic", FONT_STYLE_ITALIC),
    ("bold", FONT_STYLE_BOLD),
    ("underline", FONT_STYLE_UNDERLINE),
    ("strikethrough", FONT_STYLE_STRIKETHROUGH),
];
const DEFAULT_FOREGROUND: &str = "000000";
const DEFAULT_BACKGROUND: &str = "ffffff";
const COLOR_PREFIX: char = '#';
const OPAQUE_COLOR_DIGITS: usize = 6;
const TRANSLUCENT_COLOR_DIGITS: usize = 8;
const HEX_RADIX: u32 = 16;
const CHANNEL_DIGITS: usize = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MonacoTokenRule {
    pub(crate) token: String,
    pub(crate) foreground: Option<String>,
    pub(crate) background: Option<String>,
    pub(crate) font_style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IllegalTokenColor(pub(crate) String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatchedTokenStyle {
    pub(crate) font_style_bits: u32,
    pub(crate) foreground: [u8; 3],
}

#[derive(Debug, Clone)]
struct TrieElement {
    main_rule: MatchedTokenStyle,
    children: HashMap<String, TrieElement>,
}

impl TrieElement {
    fn new(main_rule: MatchedTokenStyle) -> Self {
        Self {
            main_rule,
            children: HashMap::new(),
        }
    }

    fn insert(&mut self, token: &str, font_style_bits: Option<u32>, foreground: Option<[u8; 3]>) {
        if token.is_empty() {
            if let Some(font_style_bits) = font_style_bits {
                self.main_rule.font_style_bits = font_style_bits;
            }
            if let Some(foreground) = foreground {
                self.main_rule.foreground = foreground;
            }
            return;
        }
        let (head, tail) = token.split_once(SCOPE_SEPARATOR).unwrap_or((token, ""));
        let inherited = self.main_rule;
        self.children
            .entry(head.to_owned())
            .or_insert_with(|| Self::new(inherited))
            .insert(tail, font_style_bits, foreground);
    }

    fn matched(&self, token: &str) -> MatchedTokenStyle {
        if token.is_empty() {
            return self.main_rule;
        }
        let (head, tail) = token.split_once(SCOPE_SEPARATOR).unwrap_or((token, ""));
        self.children
            .get(head)
            .map_or(self.main_rule, |child| child.matched(tail))
    }
}

pub(crate) struct MonacoTokenTheme {
    root: TrieElement,
}

impl MonacoTokenTheme {
    pub(crate) fn new(mut rules: Vec<MonacoTokenRule>) -> Result<Self, IllegalTokenColor> {
        rules.sort_by(|left, right| compare_utf16(&left.token, &right.token));
        let mut default_font_style_bits = 0;
        let mut default_foreground = DEFAULT_FOREGROUND.to_owned();
        let mut default_background = DEFAULT_BACKGROUND.to_owned();
        let default_rule_count = rules
            .iter()
            .take_while(|rule| rule.token.is_empty())
            .count();
        for rule in rules.drain(..default_rule_count) {
            if let Some(font_style) = &rule.font_style {
                default_font_style_bits = font_style_bits(font_style);
            }
            if let Some(foreground) = rule.foreground {
                default_foreground = foreground;
            }
            if let Some(background) = rule.background {
                default_background = background;
            }
        }
        let foreground = opaque_color(&default_foreground)?;
        opaque_color(&default_background)?;
        let mut root = TrieElement::new(MatchedTokenStyle {
            font_style_bits: default_font_style_bits,
            foreground,
        });
        for rule in &rules {
            let foreground = rule.foreground.as_deref().map(opaque_color).transpose()?;
            rule.background.as_deref().map(opaque_color).transpose()?;
            root.insert(
                &rule.token,
                rule.font_style.as_deref().map(font_style_bits),
                foreground,
            );
        }
        Ok(Self { root })
    }

    pub(crate) fn matched(&self, scope: &str) -> MatchedTokenStyle {
        self.root.matched(scope)
    }
}

fn compare_utf16(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn opaque_color(color: &str) -> Result<[u8; 3], IllegalTokenColor> {
    let illegal = || IllegalTokenColor(color.to_owned());
    let digits = color.strip_prefix(COLOR_PREFIX).unwrap_or(color);
    let is_legal = [OPAQUE_COLOR_DIGITS, TRANSLUCENT_COLOR_DIGITS].contains(&digits.len())
        && digits.bytes().all(|digit| digit.is_ascii_hexdigit());
    if !is_legal {
        return Err(illegal());
    }
    let mut channels = [0u8; 3];
    for (index, channel) in channels.iter_mut().enumerate() {
        let start = index * CHANNEL_DIGITS;
        *channel = u8::from_str_radix(&digits[start..start + CHANNEL_DIGITS], HEX_RADIX)
            .map_err(|_| illegal())?;
    }
    Ok(channels)
}

fn font_style_bits(font_style: &str) -> u32 {
    font_style
        .split(FONT_STYLE_SEGMENT_SEPARATOR)
        .filter_map(|segment| {
            FONT_STYLE_SEGMENTS
                .iter()
                .find(|(name, _)| *name == segment)
                .map(|(_, bit)| *bit)
        })
        .fold(0, |bits, bit| bits | bit)
}

#[cfg(test)]
#[path = "monaco-token-theme-tests.rs"]
mod tests;
