use std::ops::Range;

use crate::color::{DocumentColor, Hsla, Rgba, parse_hex};
use crate::document::EditorError;
use crate::json_parser::is_ecma_whitespace as is_js_whitespace;

const CHANNEL_MAX: f64 = 255.0;
const PERCENT: f64 = 100.0;
const MAX_CHANNEL: u32 = 255;
const MAX_HUE: u32 = 360;
const MAX_PERCENT_DIGITS: usize = 2;
const MAX_CHANNEL_DIGITS: usize = 3;
const HEX_LENGTHS: [usize; 4] = [3, 4, 6, 8];
const FUNCTIONS: [(&str, Function); 4] = [
    ("rgba(", Function::Rgba),
    ("rgb(", Function::Rgb),
    ("hsla(", Function::Hsla),
    ("hsl(", Function::Hsl),
];

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_colors: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorInformation {
    pub bytes: Range<usize>,
    pub color: DocumentColor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Function {
    Rgb,
    Rgba,
    Hsl,
    Hsla,
}

fn is_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

struct Cursor<'a> {
    rest: &'a str,
}

impl<'a> Cursor<'a> {
    fn whitespace(&mut self) -> usize {
        let trimmed = self.rest.trim_start_matches(is_js_whitespace);
        let count = self.rest.len() - trimmed.len();
        self.rest = trimmed;
        count
    }

    fn eat(&mut self, character: char) -> bool {
        match self.rest.strip_prefix(character) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    fn token(&mut self) -> &'a str {
        let end = self
            .rest
            .find(|character: char| !character.is_ascii_digit() && character != '.')
            .unwrap_or(self.rest.len());
        let (token, rest) = self.rest.split_at(end);
        self.rest = rest;
        token
    }

    fn separator(&mut self, allow_slash: bool) -> bool {
        let leading = self.whitespace();
        if self.eat(',') || allow_slash && self.eat('/') {
            self.whitespace();
            return true;
        }
        leading > 0
    }

    fn close(&mut self) -> bool {
        self.whitespace();
        self.eat(')') && self.rest.is_empty()
    }
}

fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn channel(token: &str) -> Option<f64> {
    let valid = digits(token)
        && token.len() <= MAX_CHANNEL_DIGITS
        && (token.len() == 1 || !token.starts_with('0'))
        && token.parse::<u32>().is_ok_and(|value| value <= MAX_CHANNEL);
    valid.then(|| token.parse::<f64>().ok()).flatten()
}

fn alpha(token: &str, trailing_zeros: bool) -> Option<f64> {
    let valid = match token.split_once('.') {
        None => matches!(token, "0" | "1"),
        Some(("0", fraction)) if digits(fraction) => true,
        Some(("", fraction)) => digits(fraction),
        Some(("0" | "1", fraction)) => {
            fraction.is_empty() || trailing_zeros && fraction.bytes().all(|byte| byte == b'0')
        }
        _ => false,
    };
    valid.then(|| token.parse::<f64>().ok()).flatten()
}

fn hue(token: &str) -> Option<f64> {
    let (integer, fraction) = match token.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (token, None),
    };
    let valid = digits(integer)
        && (integer.len() == 1 || !integer.starts_with('0'))
        && integer.parse::<u32>().is_ok_and(|value| value <= MAX_HUE)
        && fraction.is_none_or(digits);
    valid.then(|| token.parse::<f64>().ok()).flatten()
}

fn percentage(token: &str) -> Option<f64> {
    let valid = match token.split_once('.') {
        _ if token == "100" => true,
        Some(("100", fraction)) => {
            !fraction.is_empty() && fraction.bytes().all(|byte| byte == b'0')
        }
        Some((integer, fraction)) => {
            digits(integer)
                && integer.len() <= MAX_PERCENT_DIGITS
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
        }
        None => digits(token) && token.len() <= MAX_PERCENT_DIGITS,
    };
    valid.then(|| token.parse::<f64>().ok()).flatten()
}

fn parameters(function: Function, text: &str) -> Option<DocumentColor> {
    let mut cursor = Cursor { rest: text };
    if !cursor.eat('(') {
        return None;
    }
    cursor.whitespace();
    match function {
        Function::Rgb | Function::Rgba => {
            let red = channel(cursor.token())?;
            cursor.separator(false).then_some(())?;
            let green = channel(cursor.token())?;
            cursor.separator(false).then_some(())?;
            let blue = channel(cursor.token())?;
            let alpha = if function == Function::Rgba {
                cursor.separator(true).then_some(())?;
                alpha(cursor.token(), false)?
            } else {
                1.0
            };
            cursor.close().then_some(DocumentColor {
                red: red / CHANNEL_MAX,
                green: green / CHANNEL_MAX,
                blue: blue / CHANNEL_MAX,
                alpha,
            })
        }
        Function::Hsl | Function::Hsla => {
            let hue = hue(cursor.token())?;
            cursor.separator(false).then_some(())?;
            let saturation = percentage(cursor.token())?;
            cursor.eat('%').then_some(())?;
            cursor.separator(false).then_some(())?;
            let lightness = percentage(cursor.token())?;
            cursor.eat('%').then_some(())?;
            let alpha = if function == Function::Hsla {
                cursor.separator(true).then_some(())?;
                alpha(cursor.token(), true)?
            } else {
                1.0
            };
            cursor.close().then(|| {
                Rgba::from_hsla(Hsla::new(
                    hue,
                    saturation / PERCENT,
                    lightness / PERCENT,
                    alpha,
                ))
                .to_document()
            })
        }
    }
}

fn function_match(line: &str, start: usize) -> Option<(usize, Option<DocumentColor>)> {
    if line[..start].chars().next_back().is_some_and(is_word) {
        return None;
    }
    let rest = &line[start..];
    let (name, function) = FUNCTIONS.iter().find(|(name, _)| rest.starts_with(name))?;
    let open = name.len() - 1;
    let body = &rest[open + 1..];
    let length = body
        .find(|character: char| {
            !(character.is_ascii_digit()
                || is_js_whitespace(character)
                || matches!(character, ',' | '.' | '%' | '/'))
        })
        .filter(|&end| body[end..].starts_with(')'))?;
    let end = open + 1 + length + 1;
    Some((end, parameters(*function, &rest[open..end])))
}

fn hex_match(line: &str, start: usize) -> Option<(usize, Option<DocumentColor>)> {
    let digits = &line[start + 1..];
    let length = digits
        .find(|character: char| !character.is_ascii_hexdigit())
        .unwrap_or(digits.len());
    if !HEX_LENGTHS.contains(&length) || digits[length..].chars().next().is_some_and(is_word) {
        return None;
    }
    let end = 1 + length;
    Some((
        end,
        parse_hex(&line[start..start + end]).map(Rgba::to_document),
    ))
}

fn line_colors(
    line: &str,
    origin: usize,
    colors: &mut Vec<ColorInformation>,
    limits: Limits,
) -> Result<(), EditorError> {
    let mut start = 0;
    while start < line.len() {
        let character = line[start..].chars().next().unwrap_or_default();
        let found = match character {
            'r' | 'h' => function_match(line, start),
            '#' if start == 0
                || line[..start].chars().next_back().is_some_and(|previous| {
                    matches!(previous, '\'' | '"') || is_js_whitespace(previous)
                }) =>
            {
                hex_match(line, start)
            }
            _ => None,
        };
        let Some((length, color)) = found else {
            start += character.len_utf8();
            continue;
        };
        if let Some(color) = color {
            if colors.len() >= limits.max_colors {
                return Err(EditorError::Capacity);
            }
            colors.push(ColorInformation {
                bytes: origin + start..origin + start + length,
                color,
            });
        }
        start += length;
    }
    Ok(())
}

/// Finds `#hex`, `rgb[a]()` and `hsl[a]()` colors like Monaco's default document color provider.
pub fn document_colors(input: &str, limits: Limits) -> Result<Vec<ColorInformation>, EditorError> {
    if input.len() > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    let mut colors = Vec::new();
    let mut origin = 0;
    let bytes = input.as_bytes();
    while origin <= input.len() {
        let end = bytes[origin..]
            .iter()
            .position(|&byte| matches!(byte, b'\r' | b'\n'))
            .map_or(input.len(), |index| origin + index);
        line_colors(&input[origin..end], origin, &mut colors, limits)?;
        if end == input.len() {
            break;
        }
        origin = end
            + if input[end..].starts_with("\r\n") {
                2
            } else {
                1
            };
    }
    Ok(colors)
}
