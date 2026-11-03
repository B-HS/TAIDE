use std::ops::Range;

const MAX_ESCAPE_DIGITS: usize = 6;
const MIN_UNICODE_RANGE_DIGITS: usize = 1;
const MAX_UNICODE_RANGE_DIGITS: usize = 6;
const UTF16_UNIT_MASK: u32 = 0xffff;
const HEX_RADIX: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Css,
    Scss,
}

impl Dialect {
    /// Returns the stylesheet dialect served by the built-in CSS language service for a language id.
    pub fn from_language_id(language_id: &str) -> Option<Self> {
        match language_id {
            "css" => Some(Self::Css),
            "scss" => Some(Self::Scss),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ident,
    AtKeyword,
    String,
    BadString,
    UnquotedString,
    Hash,
    Num,
    Percentage,
    Dimension,
    UnicodeRange,
    Cdo,
    Cdc,
    Colon,
    SemiColon,
    CurlyL,
    CurlyR,
    ParenthesisL,
    ParenthesisR,
    BracketL,
    BracketR,
    Whitespace,
    Includes,
    Dashmatch,
    SubstringOperator,
    PrefixOperator,
    SuffixOperator,
    Delim,
    Ems,
    Exs,
    Length,
    Angle,
    Time,
    Freq,
    Exclamation,
    Resolution,
    Comma,
    Charset,
    Comment,
    Eof,
    ContainerQueryLength,
    VariableName,
    InterpolationFunction,
    EqualsOperator,
    NotEqualsOperator,
    GreaterEqualsOperator,
    SmallerEqualsOperator,
    Ellipsis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub bytes: Range<usize>,
    pub text: String,
}

pub struct Scanner<'a> {
    source: &'a str,
    position: usize,
    dialect: Dialect,
    pub ignore_comment: bool,
    pub ignore_whitespace: bool,
    pub in_url: bool,
}

fn unit_kind(unit: &str) -> Option<Kind> {
    Some(match unit {
        "em" => Kind::Ems,
        "ex" => Kind::Exs,
        "px" | "cm" | "mm" | "in" | "pt" | "pc" => Kind::Length,
        "deg" | "rad" | "grad" => Kind::Angle,
        "ms" | "s" => Kind::Time,
        "hz" | "khz" => Kind::Freq,
        "%" | "fr" => Kind::Percentage,
        "dpi" | "dpcm" => Kind::Resolution,
        "cqw" | "cqh" | "cqi" | "cqb" | "cqmin" | "cqmax" => Kind::ContainerQueryLength,
        _ => return None,
    })
}

fn is_hex_digit(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str, dialect: Dialect) -> Self {
        Self {
            source,
            position: 0,
            dialect,
            ignore_comment: true,
            ignore_whitespace: true,
            in_url: false,
        }
    }

    pub fn pos(&self) -> usize {
        self.position
    }

    pub fn go_back_to(&mut self, position: usize) {
        self.position = position;
    }

    pub fn substring(&self, bytes: Range<usize>) -> &'a str {
        &self.source[bytes]
    }

    fn peek(&self, offset: usize) -> u8 {
        self.source
            .as_bytes()
            .get(self.position + offset)
            .copied()
            .unwrap_or(0)
    }

    fn eos(&self) -> bool {
        self.position >= self.source.len()
    }

    fn current_char(&self) -> Option<char> {
        self.source[self.position..].chars().next()
    }

    fn advance_char(&mut self, result: &mut String) {
        if let Some(character) = self.current_char() {
            self.position += character.len_utf8();
            result.push(character);
        }
    }

    fn advance_if(&mut self, byte: u8) -> bool {
        if self.peek(0) == byte && !self.eos() {
            self.position += 1;
            return true;
        }
        false
    }

    fn advance_if_all(&mut self, bytes: &[u8]) -> bool {
        if self.source.as_bytes()[self.position.min(self.source.len())..].starts_with(bytes) {
            self.position += bytes.len();
            return true;
        }
        false
    }

    fn advance_while(&mut self, condition: impl Fn(u8) -> bool) -> usize {
        let start = self.position;
        while self.position < self.source.len() && condition(self.source.as_bytes()[self.position])
        {
            self.position += 1;
        }
        self.position - start
    }

    fn finish(&self, offset: usize, kind: Kind, text: Option<String>) -> Token {
        let text = text
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| self.source[offset..self.position].to_owned());
        Token {
            kind,
            bytes: offset..self.position,
            text,
        }
    }

    pub fn scan_unquoted_string(&mut self) -> Option<Token> {
        let offset = self.position;
        let mut content = String::new();
        if self.unquoted_string(&mut content) {
            return Some(self.finish(offset, Kind::UnquotedString, Some(content)));
        }
        None
    }

    pub fn scan(&mut self) -> Token {
        if let Some(token) = self.trivia() {
            return token;
        }
        let offset = self.position;
        if self.eos() {
            return self.finish(offset, Kind::Eof, None);
        }
        self.scan_next(offset)
    }

    pub fn try_scan_unicode(&mut self) -> Option<Token> {
        let offset = self.position;
        if !self.eos() && self.unicode_range() {
            return Some(self.finish(offset, Kind::UnicodeRange, None));
        }
        self.position = offset;
        None
    }

    fn scan_next(&mut self, offset: usize) -> Token {
        if self.dialect == Dialect::Scss
            && let Some(token) = self.scan_scss(offset)
        {
            return token;
        }
        if self.advance_if_all(b"<!--") {
            return self.finish(offset, Kind::Cdo, None);
        }
        if self.advance_if_all(b"-->") {
            return self.finish(offset, Kind::Cdc, None);
        }
        let mut content = String::new();
        if self.ident(&mut content) {
            return self.finish(offset, Kind::Ident, Some(content));
        }
        if self.advance_if(b'@') {
            let mut content = String::from("@");
            if self.name(&mut content) {
                if content == "@charset" {
                    return self.finish(offset, Kind::Charset, Some(content));
                }
                return self.finish(offset, Kind::AtKeyword, Some(content));
            }
            return self.finish(offset, Kind::Delim, None);
        }
        if self.advance_if(b'#') {
            let mut content = String::from("#");
            if self.name(&mut content) {
                return self.finish(offset, Kind::Hash, Some(content));
            }
            return self.finish(offset, Kind::Delim, None);
        }
        if self.advance_if(b'!') {
            return self.finish(offset, Kind::Exclamation, None);
        }
        if self.number() {
            let position = self.position;
            let mut content = self.source[offset..position].to_owned();
            if self.advance_if(b'%') {
                return self.finish(offset, Kind::Percentage, None);
            }
            if self.ident(&mut content) {
                let unit = self.source[position..self.position].to_lowercase();
                let kind = unit_kind(&unit).unwrap_or(Kind::Dimension);
                return self.finish(offset, kind, Some(content));
            }
            return self.finish(offset, Kind::Num, None);
        }
        let mut content = String::new();
        if let Some(kind) = self.string(&mut content) {
            return self.finish(offset, kind, Some(content));
        }
        let single = match self.peek(0) {
            b';' => Some(Kind::SemiColon),
            b':' => Some(Kind::Colon),
            b'{' => Some(Kind::CurlyL),
            b'}' => Some(Kind::CurlyR),
            b']' => Some(Kind::BracketR),
            b'[' => Some(Kind::BracketL),
            b'(' => Some(Kind::ParenthesisL),
            b')' => Some(Kind::ParenthesisR),
            b',' => Some(Kind::Comma),
            _ => None,
        };
        if let Some(kind) = single {
            self.position += 1;
            return self.finish(offset, kind, None);
        }
        if self.peek(1) == b'=' {
            let operator = match self.peek(0) {
                b'~' => Some(Kind::Includes),
                b'|' => Some(Kind::Dashmatch),
                b'*' => Some(Kind::SubstringOperator),
                b'^' => Some(Kind::PrefixOperator),
                b'$' => Some(Kind::SuffixOperator),
                _ => None,
            };
            if let Some(kind) = operator {
                self.position += 2;
                return self.finish(offset, kind, None);
            }
        }
        let mut ignored = String::new();
        self.advance_char(&mut ignored);
        self.finish(offset, Kind::Delim, None)
    }

    fn scan_scss(&mut self, offset: usize) -> Option<Token> {
        if self.advance_if(b'$') {
            let mut content = String::from("$");
            if self.ident(&mut content) {
                return Some(self.finish(offset, Kind::VariableName, Some(content)));
            }
            self.position = offset;
        }
        if self.advance_if_all(b"#{") {
            return Some(self.finish(offset, Kind::InterpolationFunction, None));
        }
        if self.advance_if_all(b"==") {
            return Some(self.finish(offset, Kind::EqualsOperator, None));
        }
        if self.advance_if_all(b"!=") {
            return Some(self.finish(offset, Kind::NotEqualsOperator, None));
        }
        if self.advance_if(b'<') {
            if self.advance_if(b'=') {
                return Some(self.finish(offset, Kind::SmallerEqualsOperator, None));
            }
            return Some(self.finish(offset, Kind::Delim, None));
        }
        if self.advance_if(b'>') {
            if self.advance_if(b'=') {
                return Some(self.finish(offset, Kind::GreaterEqualsOperator, None));
            }
            return Some(self.finish(offset, Kind::Delim, None));
        }
        if self.advance_if_all(b"...") {
            return Some(self.finish(offset, Kind::Ellipsis, None));
        }
        None
    }

    fn trivia(&mut self) -> Option<Token> {
        loop {
            let offset = self.position;
            if self.whitespace() {
                if !self.ignore_whitespace {
                    return Some(self.finish(offset, Kind::Whitespace, None));
                }
            } else if self.comment() {
                if !self.ignore_comment {
                    return Some(self.finish(offset, Kind::Comment, None));
                }
            } else {
                return None;
            }
        }
    }

    fn comment(&mut self) -> bool {
        if self.advance_if_all(b"/*") {
            let bytes = self.source.as_bytes();
            let mut hot = false;
            while self.position < bytes.len() {
                let byte = bytes[self.position];
                if hot && byte == b'/' {
                    self.position += 1;
                    return true;
                }
                hot = byte == b'*';
                self.position += 1;
            }
            return true;
        }
        if self.dialect == Dialect::Scss && !self.in_url && self.advance_if_all(b"//") {
            self.advance_while(|byte| !matches!(byte, b'\n' | b'\r' | 0x0c));
            return true;
        }
        false
    }

    fn number(&mut self) -> bool {
        let leading = usize::from(self.peek(0) == b'.');
        if self.peek(leading).is_ascii_digit() {
            self.position += leading + 1;
            self.advance_while(|byte| byte.is_ascii_digit() || leading == 0 && byte == b'.');
            return true;
        }
        false
    }

    fn newline(&mut self, result: &mut String) -> bool {
        let byte = self.peek(0);
        if matches!(byte, b'\r' | 0x0c | b'\n') {
            self.position += 1;
            result.push(char::from(byte));
            if byte == b'\r' && self.advance_if(b'\n') {
                result.push('\n');
            }
            return true;
        }
        false
    }

    fn escape(&mut self, result: &mut String, include_newlines: bool) -> bool {
        if self.peek(0) != b'\\' {
            return false;
        }
        self.position += 1;
        let start = self.position;
        let mut digits = 0;
        while digits < MAX_ESCAPE_DIGITS && is_hex_digit(self.peek(0)) && !self.eos() {
            self.position += 1;
            digits += 1;
        }
        if digits > 0 {
            let value = u32::from_str_radix(&self.source[start..self.position], HEX_RADIX)
                .unwrap_or_default()
                & UTF16_UNIT_MASK;
            if value != 0 {
                result.push(char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER));
            }
            if matches!(self.peek(0), b' ' | b'\t') && !self.eos() {
                self.position += 1;
            } else {
                self.newline(&mut String::new());
            }
            return true;
        }
        let byte = self.peek(0);
        if !matches!(byte, b'\r' | 0x0c | b'\n') {
            if self.eos() {
                result.push('\0');
                self.position = self.source.len();
                return true;
            }
            self.advance_char(result);
            return true;
        }
        include_newlines && self.newline(result)
    }

    fn string_char(&mut self, close: u8, result: &mut String) -> bool {
        let byte = self.peek(0);
        if byte != 0 && byte != close && !matches!(byte, b'\\' | b'\r' | 0x0c | b'\n') {
            self.advance_char(result);
            return true;
        }
        false
    }

    fn string(&mut self, result: &mut String) -> Option<Kind> {
        let close = self.peek(0);
        if close != b'\'' && close != b'"' {
            return None;
        }
        self.position += 1;
        result.push(char::from(close));
        while self.string_char(close, result) || self.escape(result, true) {}
        if self.peek(0) == close {
            self.position += 1;
            result.push(char::from(close));
            return Some(Kind::String);
        }
        Some(Kind::BadString)
    }

    fn unquoted_char(&mut self, result: &mut String) -> bool {
        let byte = self.peek(0);
        if byte != 0
            && !matches!(
                byte,
                b'\\' | b'\'' | b'"' | b'(' | b')' | b' ' | b'\t' | b'\n' | 0x0c | b'\r'
            )
        {
            self.advance_char(result);
            return true;
        }
        false
    }

    fn unquoted_string(&mut self, result: &mut String) -> bool {
        let mut has_content = false;
        while self.unquoted_char(result) || self.escape(result, false) {
            has_content = true;
        }
        has_content
    }

    fn whitespace(&mut self) -> bool {
        self.advance_while(|byte| matches!(byte, b' ' | b'\t' | b'\n' | 0x0c | b'\r')) > 0
    }

    fn name(&mut self, result: &mut String) -> bool {
        let mut matched = false;
        while self.ident_char(result) || self.escape(result, false) {
            matched = true;
        }
        matched
    }

    pub(crate) fn ident(&mut self, result: &mut String) -> bool {
        let position = self.position;
        let length = result.len();
        if self.minus(result) {
            if self.minus(result) || self.ident_first_char(result) || self.escape(result, false) {
                while self.ident_char(result) || self.escape(result, false) {}
                return true;
            }
        } else if self.ident_first_char(result) || self.escape(result, false) {
            while self.ident_char(result) || self.escape(result, false) {}
            return true;
        }
        self.position = position;
        result.truncate(length);
        false
    }

    fn ident_first_char(&mut self, result: &mut String) -> bool {
        let byte = self.peek(0);
        if byte == b'_' || byte.is_ascii_alphabetic() || byte >= 0x80 {
            self.advance_char(result);
            return true;
        }
        false
    }

    fn minus(&mut self, result: &mut String) -> bool {
        if self.peek(0) == b'-' {
            self.position += 1;
            result.push('-');
            return true;
        }
        false
    }

    fn ident_char(&mut self, result: &mut String) -> bool {
        let byte = self.peek(0);
        if byte == b'_' || byte == b'-' || byte.is_ascii_alphanumeric() || byte >= 0x80 {
            self.advance_char(result);
            return true;
        }
        false
    }

    fn unicode_range(&mut self) -> bool {
        if !self.advance_if(b'+') {
            return false;
        }
        let code_points =
            self.advance_while(is_hex_digit) + self.advance_while(|byte| byte == b'?');
        if !(MIN_UNICODE_RANGE_DIGITS..=MAX_UNICODE_RANGE_DIGITS).contains(&code_points) {
            return false;
        }
        if self.advance_if(b'-') {
            let digits = self.advance_while(is_hex_digit);
            return (MIN_UNICODE_RANGE_DIGITS..=MAX_UNICODE_RANGE_DIGITS).contains(&digits);
        }
        true
    }
}
