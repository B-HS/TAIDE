use std::ops::Range;

use crate::document::EditorError;

use super::MarkupLimits;

const RGB_COMPONENT_BYTES: usize = 2;
const SHORT_RGB_COMPONENTS: usize = 3;

#[derive(Debug, Clone, PartialEq)]
pub enum StylesheetValue {
    Literal(String),
    CustomProperty(String),
    Number {
        value: f64,
        raw: String,
        unit: String,
    },
    Color {
        red: u8,
        green: u8,
        blue: u8,
        alpha: f64,
        raw: String,
    },
    String {
        value: String,
        quote: char,
    },
    Field {
        index: Option<usize>,
        name: String,
        is_adjacent: bool,
    },
    Function {
        name: String,
        arguments: Vec<Vec<StylesheetToken>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StylesheetToken {
    pub value: StylesheetValue,
    pub source: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StylesheetProperty {
    pub name: Option<String>,
    pub values: Vec<Vec<StylesheetToken>>,
    pub is_important: bool,
}

#[derive(Clone)]
enum Lexeme {
    Value(StylesheetValue),
    Open,
    Close,
    Operator(char),
    Whitespace,
}

struct Token {
    value: Lexeme,
    start: usize,
    end: usize,
}

struct Lexer<'a> {
    input: &'a str,
    position: usize,
    limits: MarkupLimits,
}

fn is_word(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

fn is_literal(character: char) -> bool {
    is_word(character) || matches!(character, '%' | '/')
}

fn is_keyword(character: char) -> bool {
    is_word(character) || character.is_ascii_digit() || character == '-'
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn take(&mut self, expected: char) -> bool {
        if self.peek() != Some(expected) {
            return false;
        }
        self.position += expected.len_utf8();
        true
    }

    fn consume(&mut self, predicate: impl Fn(char) -> bool) {
        while let Some(character) = self.peek().filter(|character| predicate(*character)) {
            self.position += character.len_utf8();
        }
    }

    fn field(&mut self) -> Result<Option<StylesheetValue>, EditorError> {
        if !self.input[self.position..].starts_with("${") {
            return Ok(None);
        }
        self.position += "${".len();
        let start = self.position;
        self.consume(|character| character.is_ascii_digit());
        let index = if self.position > start {
            Some(
                self.input[start..self.position]
                    .parse()
                    .map_err(|_| EditorError::Capacity)?,
            )
        } else {
            None
        };
        let has_name = if index.is_some() {
            self.take(':')
        } else {
            self.peek()
                .is_some_and(|character| character.is_ascii_alphabetic())
        };
        let start = self.position;
        if has_name {
            let mut depth = 0usize;
            while let Some(character) = self.peek() {
                if character == '\\' {
                    self.position += character.len_utf8();
                    if let Some(next) = self.peek() {
                        self.position += next.len_utf8();
                    }
                    continue;
                }
                if character == '}' && depth == 0 {
                    break;
                }
                self.position += character.len_utf8();
                if character == '{' {
                    depth = depth.checked_add(1).ok_or(EditorError::Capacity)?;
                    if depth > self.limits.max_nesting {
                        return Err(EditorError::Capacity);
                    }
                }
                if character == '}' {
                    depth -= 1;
                }
            }
        }
        let name = self.input[start..self.position].to_owned();
        if !self.take('}') {
            return Err(EditorError::Refused);
        }
        Ok(Some(StylesheetValue::Field {
            index,
            name,
            is_adjacent: false,
        }))
    }

    fn number(&mut self) -> Result<Option<StylesheetValue>, EditorError> {
        let start = self.position;
        self.take('-');
        let after_negative = self.position;
        self.consume(|character| character.is_ascii_digit());
        let has_integer = self.position > after_negative;
        let before_decimal = self.position;
        if self.take('.') {
            let after_decimal = self.position;
            self.consume(|character| character.is_ascii_digit());
            if !has_integer && self.position == after_decimal {
                self.position = before_decimal;
            }
        }
        if self.position == after_negative {
            self.position = start;
            return Ok(None);
        }
        let raw = self.input[start..self.position].to_owned();
        let value = raw.parse::<f64>().map_err(|_| EditorError::Refused)?;
        if !value.is_finite() {
            return Err(EditorError::Capacity);
        }
        let unit_start = self.position;
        if !self.take('%') {
            self.consume(is_word);
        }
        let unit = self.input[unit_start..self.position].to_owned();
        Ok(Some(StylesheetValue::Number { value, raw, unit }))
    }

    fn color(&mut self) -> Result<Option<StylesheetValue>, EditorError> {
        if !self.take('#') {
            return Ok(None);
        }
        let start = self.position;
        self.consume(|character| character.is_ascii_hexdigit());
        let mut color = self.input[start..self.position].to_owned();
        let is_transparent = color.is_empty() && self.take('t');
        if is_transparent {
            color = "0".into();
        }
        let mut alpha = if is_transparent { 0.0 } else { 1.0 };
        let has_alpha = self.take('.');
        if has_alpha {
            let start = self.position;
            self.consume(|character| character.is_ascii_digit());
            alpha = if self.position == start {
                1.0
            } else {
                format!(".{}", &self.input[start..self.position])
                    .parse()
                    .map_err(|_| EditorError::Refused)?
            };
        }
        if color.is_empty() && !has_alpha && self.position < self.input.len() {
            return Ok(Some(StylesheetValue::Literal("#".into())));
        }
        let rgb = match color.len() {
            0 => "000000".into(),
            1 => color.repeat(RGB_COMPONENT_BYTES * SHORT_RGB_COMPONENTS),
            RGB_COMPONENT_BYTES => color.repeat(SHORT_RGB_COMPONENTS),
            SHORT_RGB_COMPONENTS => color
                .chars()
                .flat_map(|character| [character, character])
                .collect::<String>(),
            _ => color.repeat(RGB_COMPONENT_BYTES),
        };
        let component = |index| {
            let start = index * RGB_COMPONENT_BYTES;
            u8::from_str_radix(&rgb[start..start + RGB_COMPONENT_BYTES], 16)
                .map_err(|_| EditorError::Refused)
        };
        Ok(Some(StylesheetValue::Color {
            red: component(0)?,
            green: component(1)?,
            blue: component(2)?,
            alpha,
            raw: self.input[start..self.position].to_owned(),
        }))
    }

    fn token(&mut self, short: bool) -> Result<Option<Lexeme>, EditorError> {
        if let Some(value) = self.field()? {
            return Ok(Some(Lexeme::Value(value)));
        }
        if self.input[self.position..].starts_with("--") {
            let start = self.position;
            self.position += "--".len();
            self.consume(is_keyword);
            return Ok(Some(Lexeme::Value(StylesheetValue::CustomProperty(
                self.input[start..self.position].into(),
            ))));
        }
        if let Some(value) = self.number()? {
            return Ok(Some(Lexeme::Value(value)));
        }
        if let Some(value) = self.color()? {
            return Ok(Some(Lexeme::Value(value)));
        }
        let Some(character) = self.peek() else {
            return Ok(None);
        };
        if matches!(character, '\'' | '"') {
            self.position += character.len_utf8();
            let start = self.position;
            while let Some(next) = self.peek() {
                if next == character {
                    break;
                }
                self.position += next.len_utf8();
                if next == '\\'
                    && let Some(escaped) = self.peek()
                {
                    self.position += escaped.len_utf8();
                }
            }
            let value = self.input[start..self.position].to_owned();
            self.take(character);
            return Ok(Some(Lexeme::Value(StylesheetValue::String {
                value,
                quote: character,
            })));
        }
        if matches!(character, '(' | ')') {
            self.position += 1;
            return Ok(Some(if character == '(' {
                Lexeme::Open
            } else {
                Lexeme::Close
            }));
        }
        if matches!(character, '+' | '!' | ',' | ':' | '-') {
            self.position += 1;
            return Ok(Some(Lexeme::Operator(character)));
        }
        if matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{a0}') {
            self.consume(|character| matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{a0}'));
            return Ok(Some(Lexeme::Whitespace));
        }
        let start = self.position;
        if matches!(character, '@' | '$') {
            self.position += 1;
            if start == 0 {
                self.consume(is_literal);
            } else {
                self.consume(is_keyword);
            }
        } else if is_word(character) {
            self.position += 1;
            if short {
                self.consume(is_literal);
            } else {
                self.consume(is_keyword);
            }
        } else {
            self.take('.');
            self.consume(is_literal);
        }
        if self.position == start {
            return Ok(None);
        }
        Ok(Some(Lexeme::Value(StylesheetValue::Literal(
            self.input[start..self.position].into(),
        ))))
    }

    fn tokens(mut self, is_value: bool) -> Result<Vec<Token>, EditorError> {
        let mut tokens = Vec::<Token>::new();
        let mut depth = 0usize;
        let mut should_consume_operator = false;
        while self.position < self.input.len() {
            if tokens.len() >= self.limits.max_nodes {
                return Err(EditorError::Capacity);
            }
            let start = self.position;
            let value = if should_consume_operator
                && self
                    .peek()
                    .is_some_and(|character| matches!(character, '+' | '!' | ',' | ':' | '-'))
            {
                let operator = self.peek().ok_or(EditorError::Refused)?;
                self.position += 1;
                Lexeme::Operator(operator)
            } else {
                self.token(depth == 0 && !is_value)?
                    .ok_or(EditorError::Refused)?
            };
            should_consume_operator = matches!(
                &value,
                Lexeme::Value(StylesheetValue::Color { .. })
            ) || matches!(&value, Lexeme::Value(StylesheetValue::Number { unit, .. }) if unit.is_empty());
            if matches!(value, Lexeme::Open) {
                if depth == 0 {
                    let mut merge_start = None;
                    let mut merge_end = None;
                    while tokens.last().is_some_and(|token| {
                        matches!(
                            token.value,
                            Lexeme::Value(
                                StylesheetValue::Literal(_) | StylesheetValue::Number { .. }
                            )
                        )
                    }) {
                        let token = tokens.pop().ok_or(EditorError::Refused)?;
                        merge_end.get_or_insert(token.end);
                        merge_start = Some(token.start);
                    }
                    if let (Some(start), Some(end)) = (merge_start, merge_end) {
                        tokens.push(Token {
                            value: Lexeme::Value(StylesheetValue::Literal(
                                self.input[start..end].into(),
                            )),
                            start,
                            end,
                        });
                    }
                }
                depth = depth.checked_add(1).ok_or(EditorError::Capacity)?;
                if depth > self.limits.max_nesting {
                    return Err(EditorError::Capacity);
                }
            }
            if matches!(value, Lexeme::Close) {
                depth = depth.checked_sub(1).ok_or(EditorError::Refused)?;
            }
            tokens.push(Token {
                value,
                start,
                end: self.position,
            });
        }
        Ok(tokens)
    }
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    max_nesting: usize,
}

impl Parser {
    fn take(&mut self, predicate: impl Fn(&Lexeme) -> bool) -> bool {
        if !self
            .tokens
            .get(self.position)
            .is_some_and(|token| predicate(&token.value))
        {
            return false;
        }
        self.position += 1;
        true
    }

    fn fragment(
        &mut self,
        in_argument: bool,
        depth: usize,
    ) -> Result<Option<Vec<StylesheetToken>>, EditorError> {
        if depth > self.max_nesting {
            return Err(EditorError::Capacity);
        }
        let mut values = Vec::new();
        let mut previous_end = None;
        while let Some(token) = self.tokens.get(self.position) {
            if let Lexeme::Value(value) = &token.value {
                let mut value = value.clone();
                if let StylesheetValue::Field { is_adjacent, .. } = &mut value {
                    *is_adjacent = previous_end == Some(token.start);
                }
                previous_end = Some(token.end);
                let mut source = Some(token.start..token.end);
                self.position += 1;
                if let StylesheetValue::Literal(name) = &value
                    && self.take(|token| matches!(token, Lexeme::Open))
                {
                    let mut arguments = Vec::new();
                    while self.position < self.tokens.len()
                        && !self.take(|token| matches!(token, Lexeme::Close))
                    {
                        if let Some(fragment) = self.fragment(true, depth + 1)? {
                            arguments.push(fragment);
                        } else if !self.take(|token| {
                            matches!(token, Lexeme::Whitespace | Lexeme::Operator(','))
                        }) {
                            return Err(EditorError::Refused);
                        }
                    }
                    value = StylesheetValue::Function {
                        name: name.clone(),
                        arguments,
                    };
                    previous_end = None;
                    source = None;
                }
                values.push(StylesheetToken { value, source });
                continue;
            }
            if self.take(|token| {
                matches!(token, Lexeme::Operator(':' | '-'))
                    || in_argument && matches!(token, Lexeme::Whitespace)
            }) {
                continue;
            }
            break;
        }
        Ok((!values.is_empty()).then_some(values))
    }

    fn properties(mut self, is_value: bool) -> Result<Vec<StylesheetProperty>, EditorError> {
        let mut properties = Vec::new();
        while self.position < self.tokens.len() {
            let mut name = None;
            if !is_value
                && let Some(Token {
                    value: Lexeme::Value(StylesheetValue::Literal(literal)),
                    ..
                }) = self.tokens.get(self.position)
                && !self
                    .tokens
                    .get(self.position + 1)
                    .is_some_and(|token| matches!(token.value, Lexeme::Open))
            {
                name = Some(literal.clone());
                self.position += 1;
                self.take(|token| matches!(token, Lexeme::Operator(':' | '-')));
            }
            if is_value {
                self.take(|token| matches!(token, Lexeme::Whitespace));
            }
            let mut values = Vec::new();
            let mut is_important = false;
            while self.position < self.tokens.len() {
                if self.take(|token| matches!(token, Lexeme::Operator('!'))) {
                    is_important = true;
                } else if let Some(fragment) = self.fragment(is_value, 0)? {
                    values.push(fragment);
                } else if !self.take(|token| matches!(token, Lexeme::Operator(','))) {
                    break;
                }
            }
            if name.is_some() || !values.is_empty() || is_important {
                properties.push(StylesheetProperty {
                    name,
                    values,
                    is_important,
                });
            } else if !self.take(|token| matches!(token, Lexeme::Operator('+'))) {
                return Err(EditorError::Refused);
            }
        }
        Ok(properties)
    }
}

pub fn parse_stylesheet(
    input: &str,
    is_value: bool,
    limits: MarkupLimits,
) -> Result<Vec<StylesheetProperty>, EditorError> {
    if input.len() > limits.max_bytes || limits.max_nodes == 0 {
        return Err(EditorError::Capacity);
    }
    let tokens = Lexer {
        input,
        position: 0,
        limits,
    }
    .tokens(is_value)?;
    let tokens = if input.is_ascii() {
        tokens
    } else {
        let mut offsets = Vec::with_capacity(input.len() + 1);
        let mut utf16 = 0usize;
        for character in input.chars() {
            offsets.extend(std::iter::repeat(utf16).take(character.len_utf8()));
            utf16 += character.len_utf16();
        }
        offsets.push(utf16);
        tokens
            .into_iter()
            .map(|token| Token {
                value: token.value,
                start: offsets[token.start],
                end: offsets[token.end],
            })
            .collect()
    };
    Parser {
        tokens,
        position: 0,
        max_nesting: limits.max_nesting,
    }
    .properties(is_value)
}
