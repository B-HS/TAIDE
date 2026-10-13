use std::ops::Range;

use crate::document::EditorError;

const HEX_DIGITS: usize = 4;
const HEX_RADIX: u32 = 16;
const UTF16_SCALAR_UNIT_CAPACITY: usize = 2;
const CONTROL_END: char = '\u{1f}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Comma,
    Colon,
    Null,
    True,
    False,
    String,
    Number,
    LineComment,
    BlockComment,
    LineBreak,
    Whitespace,
    Unknown,
    End,
}

impl Kind {
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::LineComment | Self::BlockComment | Self::LineBreak | Self::Whitespace
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    EndOfComment,
    EndOfString,
    EndOfNumber,
    InvalidUnicode,
    InvalidEscape,
    InvalidCharacter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub error: Option<Error>,
    pub bytes: Range<usize>,
    pub utf16: Range<usize>,
    pub line: usize,
    pub character: usize,
    pub value: Vec<u16>,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_value_units: usize,
}

struct Scanner<'a> {
    input: &'a str,
    byte: usize,
    utf16: usize,
    line: usize,
    line_start: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<char> {
        self.input[self.byte..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.byte += character.len_utf8();
        self.utf16 += character.len_utf16();
        if character == '\n' || character == '\r' && self.peek() != Some('\n') {
            self.line += 1;
            self.line_start = self.utf16;
        }
        Some(character)
    }

    fn newline(&mut self) {
        let first = self.advance();
        if first == Some('\r') && self.peek() == Some('\n') {
            self.advance();
        }
    }

    fn string(&mut self, budget: usize) -> (Vec<u16>, Option<Error>) {
        let mut value = Vec::new();
        let mut error = None;
        loop {
            if value.len() > budget {
                return (value, None);
            }
            let Some(character) = self.peek() else {
                return (value, Some(Error::EndOfString));
            };
            if character == '"' {
                self.advance();
                return (value, error);
            }
            if matches!(character, '\r' | '\n') {
                return (value, Some(Error::EndOfString));
            }
            self.advance();
            if character != '\\' {
                if character <= CONTROL_END {
                    error = Some(Error::InvalidCharacter);
                }
                value.extend(
                    character
                        .encode_utf16(&mut [0; UTF16_SCALAR_UNIT_CAPACITY])
                        .iter()
                        .copied(),
                );
                continue;
            }
            let Some(escaped) = self.advance() else {
                return (value, Some(Error::EndOfString));
            };
            let decoded = match escaped {
                '"' | '\\' | '/' => Some(escaped),
                'b' => Some('\u{8}'),
                'f' => Some('\u{c}'),
                'n' => Some('\n'),
                'r' => Some('\r'),
                't' => Some('\t'),
                'u' => {
                    let mut hex = String::new();
                    for _ in 0..HEX_DIGITS {
                        let Some(digit) = self.peek().filter(char::is_ascii_hexdigit) else {
                            break;
                        };
                        self.advance();
                        hex.push(digit);
                    }
                    if hex.len() == HEX_DIGITS {
                        value.push(u16::from_str_radix(&hex, HEX_RADIX).unwrap());
                    } else {
                        error = Some(Error::InvalidUnicode);
                    }
                    continue;
                }
                _ => None,
            };
            if let Some(decoded) = decoded {
                value.extend(
                    decoded
                        .encode_utf16(&mut [0; UTF16_SCALAR_UNIT_CAPACITY])
                        .iter()
                        .copied(),
                );
            } else {
                error = Some(Error::InvalidEscape);
            }
        }
    }

    fn value(&self, range: Range<usize>, budget: usize) -> Vec<u16> {
        self.input[range]
            .encode_utf16()
            .take(budget.saturating_add(1))
            .collect()
    }

    fn number(&mut self, start: usize, budget: usize) -> (Vec<u16>, Option<Error>) {
        if self.advance() != Some('0') {
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
        }
        if self.peek() == Some('.') {
            self.advance();
            if !self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                return (
                    self.value(start..self.byte, budget),
                    Some(Error::EndOfNumber),
                );
            }
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
        }
        let mut end = self.byte;
        if self
            .peek()
            .is_some_and(|character| matches!(character, 'e' | 'E'))
        {
            self.advance();
            if self
                .peek()
                .is_some_and(|character| matches!(character, '+' | '-'))
            {
                self.advance();
            }
            if !self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                return (self.value(start..end, budget), Some(Error::EndOfNumber));
            }
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.advance();
            }
            end = self.byte;
        }
        (self.value(start..end, budget), None)
    }

    fn next(&mut self, budget: usize) -> Token {
        let start = self.byte;
        let utf16_start = self.utf16;
        let line = self.line;
        let character = utf16_start - self.line_start;
        let mut value = Vec::new();
        let mut error = None;
        let kind = match self.peek() {
            None => Kind::End,
            Some(' ' | '\t') => {
                while self
                    .peek()
                    .is_some_and(|character| matches!(character, ' ' | '\t'))
                {
                    self.advance();
                }
                value = self.value(start..self.byte, budget);
                Kind::Whitespace
            }
            Some('\r' | '\n') => {
                self.newline();
                value = self.value(start..self.byte, budget);
                Kind::LineBreak
            }
            Some('"') => {
                self.advance();
                (value, error) = self.string(budget);
                Kind::String
            }
            Some('/') if self.input[start..].starts_with("//") => {
                while self
                    .peek()
                    .is_some_and(|character| !matches!(character, '\r' | '\n'))
                {
                    self.advance();
                }
                value = self.value(start..self.byte, budget);
                Kind::LineComment
            }
            Some('/') if self.input[start..].starts_with("/*") => {
                self.advance();
                self.advance();
                loop {
                    if self.input[self.byte..].starts_with("*/") {
                        self.advance();
                        self.advance();
                        break;
                    }
                    match self.peek() {
                        None => {
                            error = Some(Error::EndOfComment);
                            break;
                        }
                        Some('\r' | '\n') => self.newline(),
                        Some(_) => {
                            self.advance();
                        }
                    }
                }
                value = self.value(start..self.byte, budget);
                Kind::BlockComment
            }
            Some('-') => {
                self.advance();
                if self
                    .peek()
                    .is_some_and(|character| character.is_ascii_digit())
                {
                    (value, error) = self.number(start, budget);
                    Kind::Number
                } else {
                    value.push(u16::from(b'-'));
                    Kind::Unknown
                }
            }
            Some(character) if character.is_ascii_digit() => {
                (value, error) = self.number(start, budget);
                Kind::Number
            }
            Some(character) if matches!(character, '{' | '}' | '[' | ']' | ',' | ':') => {
                self.advance();
                match character {
                    '{' => Kind::OpenBrace,
                    '}' => Kind::CloseBrace,
                    '[' => Kind::OpenBracket,
                    ']' => Kind::CloseBracket,
                    ',' => Kind::Comma,
                    _ => Kind::Colon,
                }
            }
            Some(_) => {
                while self.peek().is_some_and(|character| {
                    !matches!(
                        character,
                        ' ' | '\t' | '\r' | '\n' | '{' | '}' | '[' | ']' | '"' | ':' | ',' | '/'
                    )
                }) {
                    self.advance();
                }
                if self.byte == start {
                    self.advance();
                }
                let raw = &self.input[start..self.byte];
                value = self.value(start..self.byte, budget);
                match raw {
                    "true" => Kind::True,
                    "false" => Kind::False,
                    "null" => Kind::Null,
                    _ => Kind::Unknown,
                }
            }
        };
        Token {
            kind,
            error,
            bytes: start..self.byte,
            utf16: utf16_start..self.utf16,
            line,
            character,
            value,
        }
    }
}

pub fn scan(input: &str, ignore_trivia: bool, limits: Limits) -> Result<Vec<Token>, EditorError> {
    if input.len() > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    let mut scanner = Scanner {
        input,
        byte: 0,
        utf16: 0,
        line: 0,
        line_start: 0,
    };
    let mut tokens = Vec::new();
    let mut scanned = 0usize;
    let mut value_units = 0usize;
    loop {
        let token = scanner.next(limits.max_value_units - value_units);
        scanned = scanned.checked_add(1).ok_or(EditorError::Capacity)?;
        value_units = value_units
            .checked_add(token.value.len())
            .ok_or(EditorError::Capacity)?;
        if scanned > limits.max_tokens || value_units > limits.max_value_units {
            return Err(EditorError::Capacity);
        }
        let end = token.kind == Kind::End;
        if !ignore_trivia || !token.kind.is_trivia() {
            tokens.push(token);
        }
        if end {
            return Ok(tokens);
        }
    }
}
