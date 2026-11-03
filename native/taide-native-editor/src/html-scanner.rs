use crate::json_parser::is_ecma_whitespace as is_js_whitespace;

const SCRIPT_TAG: &[u8] = b"script";
const STYLE_END: &[u8] = b"</style";
const DOCTYPE: &[u8] = b"!doctype";
const COMMENT_OPEN: &[u8] = b"!--";
const COMMENT_CLOSE: &[u8] = b"-->";
const SCRIPT_COMMENT_OPEN: &[u8] = b"<!--";
const SELF_CLOSE: &[u8] = b"/>";
const HTML_SCRIPT_TYPES: [&str; 2] = ["text/x-handlebars-template", "text/html"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    StartCommentTag,
    Comment,
    EndCommentTag,
    StartTagOpen,
    StartTagClose,
    StartTagSelfClose,
    StartTag,
    EndTagOpen,
    EndTagClose,
    EndTag,
    DelimiterAssign,
    AttributeName,
    AttributeValue,
    StartDoctypeTag,
    Doctype,
    EndDoctypeTag,
    Content,
    Whitespace,
    Unknown,
    Script,
    Styles,
    Eos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScannerState {
    WithinContent,
    AfterOpeningStartTag,
    AfterOpeningEndTag,
    WithinDoctype,
    WithinTag,
    WithinEndTag,
    WithinComment,
    WithinScriptContent,
    WithinStyleContent,
    AfterAttributeName,
    BeforeAttributeValue,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScriptDelimiter {
    CommentOpen,
    CommentClose,
    ScriptOpen,
    ScriptClose,
}

/// Incremental HTML tokenizer with the error tolerance of the VS Code HTML language service.
pub struct Scanner<'a> {
    source: &'a str,
    position: usize,
    state: ScannerState,
    emit_pseudo_close_tags: bool,
    token_offset: usize,
    token_type: TokenType,
    has_space_after_tag: bool,
    last_tag: String,
    last_attribute_name: Option<String>,
    last_type_value: Option<String>,
}

fn is_element_name_start(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':')
}

fn is_element_name_part(byte: u8) -> bool {
    is_element_name_start(byte) || matches!(byte, b'-' | b'.')
}

fn is_attribute_name_part(character: char) -> bool {
    !(is_js_whitespace(character)
        || matches!(character, '"' | '\'' | '>' | '<' | '/' | '=')
        || ('\u{0}'..='\u{f}').contains(&character)
        || ('\u{7f}'..='\u{9f}').contains(&character))
}

fn is_unquoted_value_part(character: char) -> bool {
    !(is_js_whitespace(character) || matches!(character, '"' | '\'' | '`' | '=' | '<' | '>'))
}

fn starts_with_ignore_ascii_case(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && bytes[..prefix.len()].eq_ignore_ascii_case(prefix)
}

impl<'a> Scanner<'a> {
    pub fn new(
        source: &'a str,
        initial_offset: usize,
        initial_state: ScannerState,
        emit_pseudo_close_tags: bool,
    ) -> Self {
        Self {
            source,
            position: initial_offset.min(source.len()),
            state: initial_state,
            emit_pseudo_close_tags,
            token_offset: 0,
            token_type: TokenType::Unknown,
            has_space_after_tag: false,
            last_tag: String::new(),
            last_attribute_name: None,
            last_type_value: None,
        }
    }

    pub fn token_type(&self) -> TokenType {
        self.token_type
    }

    pub fn token_offset(&self) -> usize {
        self.token_offset
    }

    pub fn token_end(&self) -> usize {
        self.position
    }

    pub fn token_length(&self) -> usize {
        self.position - self.token_offset
    }

    pub fn token_text(&self) -> &'a str {
        &self.source[self.token_offset..self.position]
    }

    pub fn state(&self) -> ScannerState {
        self.state
    }

    fn bytes(&self) -> &'a [u8] {
        self.source.as_bytes()
    }

    fn eos(&self) -> bool {
        self.position >= self.source.len()
    }

    fn peek(&self) -> u8 {
        self.bytes().get(self.position).copied().unwrap_or(0)
    }

    fn advance_char(&mut self) {
        self.position += self.source[self.position..]
            .chars()
            .next()
            .map_or(0, char::len_utf8);
    }

    fn advance_if_byte(&mut self, byte: u8) -> bool {
        if self.peek() == byte && !self.eos() {
            self.position += 1;
            return true;
        }
        false
    }

    fn advance_if_bytes(&mut self, bytes: &[u8]) -> bool {
        if self.bytes()[self.position..].starts_with(bytes) {
            self.position += bytes.len();
            return true;
        }
        false
    }

    fn advance_until_byte(&mut self, byte: u8) -> bool {
        match self.bytes()[self.position..]
            .iter()
            .position(|&value| value == byte)
        {
            Some(index) => {
                self.position += index;
                true
            }
            None => {
                self.position = self.source.len();
                false
            }
        }
    }

    fn advance_until_bytes(&mut self, bytes: &[u8]) -> bool {
        match self.bytes()[self.position..]
            .windows(bytes.len())
            .position(|window| window == bytes)
        {
            Some(index) => {
                self.position += index;
                true
            }
            None => {
                self.position = self.source.len();
                false
            }
        }
    }

    fn skip_whitespace(&mut self) -> bool {
        let start = self.position;
        while matches!(self.peek(), b' ' | b'\t' | b'\n' | 0x0c | b'\r') && !self.eos() {
            self.position += 1;
        }
        self.position > start
    }

    fn advance_while_char(&mut self, condition: impl Fn(char) -> bool) -> &'a str {
        let start = self.position;
        let length = self.source[start..]
            .char_indices()
            .find(|&(_, character)| !condition(character))
            .map_or(self.source.len() - start, |(index, _)| index);
        self.position += length;
        &self.source[start..self.position]
    }

    fn next_element_name(&mut self) -> String {
        let bytes = self.bytes();
        if !bytes
            .get(self.position)
            .is_some_and(|&byte| is_element_name_start(byte))
        {
            return String::new();
        }
        let start = self.position;
        self.position += 1;
        while bytes
            .get(self.position)
            .is_some_and(|&byte| is_element_name_part(byte))
        {
            self.position += 1;
        }
        self.source[start..self.position].to_ascii_lowercase()
    }

    fn find_script_delimiter(&self) -> Option<(usize, usize, ScriptDelimiter)> {
        let bytes = self.bytes();
        let mut index = self.position;
        while index < bytes.len() {
            match bytes[index] {
                b'<' => {
                    if bytes[index..].starts_with(SCRIPT_COMMENT_OPEN) {
                        return Some((
                            index,
                            index + SCRIPT_COMMENT_OPEN.len(),
                            ScriptDelimiter::CommentOpen,
                        ));
                    }
                    let mut cursor = index + 1;
                    let closing = bytes.get(cursor) == Some(&b'/');
                    if closing {
                        cursor += 1;
                    }
                    if starts_with_ignore_ascii_case(&bytes[cursor..], SCRIPT_TAG) {
                        cursor += SCRIPT_TAG.len();
                        cursor += self.source[cursor..]
                            .char_indices()
                            .find(|&(_, character)| !is_js_whitespace(character))
                            .map_or(self.source.len() - cursor, |(offset, _)| offset);
                        if bytes.get(cursor) == Some(&b'/') {
                            cursor += 1;
                        }
                        if bytes.get(cursor) == Some(&b'>') {
                            cursor += 1;
                        }
                        let kind = if closing {
                            ScriptDelimiter::ScriptClose
                        } else {
                            ScriptDelimiter::ScriptOpen
                        };
                        return Some((index, cursor, kind));
                    }
                }
                b'-' if bytes[index..].starts_with(COMMENT_CLOSE) => {
                    return Some((
                        index,
                        index + COMMENT_CLOSE.len(),
                        ScriptDelimiter::CommentClose,
                    ));
                }
                _ => {}
            }
            index += 1;
        }
        None
    }

    fn finish(&mut self, offset: usize, token_type: TokenType) -> TokenType {
        self.token_type = token_type;
        self.token_offset = offset;
        token_type
    }

    /// Scans the next token and returns its type.
    pub fn scan(&mut self) -> TokenType {
        let offset = self.position;
        let token = self.internal_scan();
        if token != TokenType::Eos
            && offset == self.position
            && !(self.emit_pseudo_close_tags
                && matches!(token, TokenType::StartTagClose | TokenType::EndTagClose))
        {
            self.advance_char();
            return self.finish(offset, TokenType::Unknown);
        }
        token
    }

    fn internal_scan(&mut self) -> TokenType {
        let offset = self.position;
        loop {
            if self.eos() {
                return self.finish(offset, TokenType::Eos);
            }
            match self.state {
                ScannerState::WithinComment => {
                    if self.advance_if_bytes(COMMENT_CLOSE) {
                        self.state = ScannerState::WithinContent;
                        return self.finish(offset, TokenType::EndCommentTag);
                    }
                    self.advance_until_bytes(COMMENT_CLOSE);
                    return self.finish(offset, TokenType::Comment);
                }
                ScannerState::WithinDoctype => {
                    if self.advance_if_byte(b'>') {
                        self.state = ScannerState::WithinContent;
                        return self.finish(offset, TokenType::EndDoctypeTag);
                    }
                    self.advance_until_byte(b'>');
                    return self.finish(offset, TokenType::Doctype);
                }
                ScannerState::WithinContent => {
                    if self.advance_if_byte(b'<') {
                        if !self.eos() && self.peek() == b'!' {
                            if self.advance_if_bytes(COMMENT_OPEN) {
                                self.state = ScannerState::WithinComment;
                                return self.finish(offset, TokenType::StartCommentTag);
                            }
                            if starts_with_ignore_ascii_case(
                                &self.bytes()[self.position..],
                                DOCTYPE,
                            ) {
                                self.position += DOCTYPE.len();
                                self.state = ScannerState::WithinDoctype;
                                return self.finish(offset, TokenType::StartDoctypeTag);
                            }
                        }
                        if self.advance_if_byte(b'/') {
                            self.state = ScannerState::AfterOpeningEndTag;
                            return self.finish(offset, TokenType::EndTagOpen);
                        }
                        self.state = ScannerState::AfterOpeningStartTag;
                        return self.finish(offset, TokenType::StartTagOpen);
                    }
                    self.advance_until_byte(b'<');
                    return self.finish(offset, TokenType::Content);
                }
                ScannerState::AfterOpeningEndTag => {
                    if !self.next_element_name().is_empty() {
                        self.state = ScannerState::WithinEndTag;
                        return self.finish(offset, TokenType::EndTag);
                    }
                    if self.skip_whitespace() {
                        return self.finish(offset, TokenType::Whitespace);
                    }
                    self.state = ScannerState::WithinEndTag;
                    self.advance_until_byte(b'>');
                    if offset < self.position {
                        return self.finish(offset, TokenType::Unknown);
                    }
                }
                ScannerState::WithinEndTag => {
                    if self.skip_whitespace() {
                        return self.finish(offset, TokenType::Whitespace);
                    }
                    if self.advance_if_byte(b'>') {
                        self.state = ScannerState::WithinContent;
                        return self.finish(offset, TokenType::EndTagClose);
                    }
                    if self.emit_pseudo_close_tags && self.peek() == b'<' {
                        self.state = ScannerState::WithinContent;
                        return self.finish(offset, TokenType::EndTagClose);
                    }
                    self.advance_char();
                    self.state = ScannerState::WithinContent;
                    return self.finish(offset, TokenType::Unknown);
                }
                ScannerState::AfterOpeningStartTag => {
                    self.last_tag = self.next_element_name();
                    self.last_type_value = None;
                    self.last_attribute_name = None;
                    if !self.last_tag.is_empty() {
                        self.has_space_after_tag = false;
                        self.state = ScannerState::WithinTag;
                        return self.finish(offset, TokenType::StartTag);
                    }
                    if self.skip_whitespace() {
                        return self.finish(offset, TokenType::Whitespace);
                    }
                    self.state = ScannerState::WithinTag;
                    self.advance_until_byte(b'>');
                    if offset < self.position {
                        return self.finish(offset, TokenType::Unknown);
                    }
                }
                ScannerState::WithinTag => {
                    if self.skip_whitespace() {
                        self.has_space_after_tag = true;
                        return self.finish(offset, TokenType::Whitespace);
                    }
                    if self.has_space_after_tag {
                        let name = self
                            .advance_while_char(is_attribute_name_part)
                            .to_lowercase();
                        let found = !name.is_empty();
                        self.last_attribute_name = Some(name);
                        if found {
                            self.state = ScannerState::AfterAttributeName;
                            self.has_space_after_tag = false;
                            return self.finish(offset, TokenType::AttributeName);
                        }
                    }
                    if self.advance_if_bytes(SELF_CLOSE) {
                        self.state = ScannerState::WithinContent;
                        return self.finish(offset, TokenType::StartTagSelfClose);
                    }
                    if self.advance_if_byte(b'>') {
                        self.state = if self.last_tag == "script" {
                            if self
                                .last_type_value
                                .as_deref()
                                .is_some_and(|value| HTML_SCRIPT_TYPES.contains(&value))
                            {
                                ScannerState::WithinContent
                            } else {
                                ScannerState::WithinScriptContent
                            }
                        } else if self.last_tag == "style" {
                            ScannerState::WithinStyleContent
                        } else {
                            ScannerState::WithinContent
                        };
                        return self.finish(offset, TokenType::StartTagClose);
                    }
                    if self.emit_pseudo_close_tags && self.peek() == b'<' {
                        self.state = ScannerState::WithinContent;
                        return self.finish(offset, TokenType::StartTagClose);
                    }
                    self.advance_char();
                    return self.finish(offset, TokenType::Unknown);
                }
                ScannerState::AfterAttributeName => {
                    if self.skip_whitespace() {
                        self.has_space_after_tag = true;
                        return self.finish(offset, TokenType::Whitespace);
                    }
                    if self.advance_if_byte(b'=') {
                        self.state = ScannerState::BeforeAttributeValue;
                        return self.finish(offset, TokenType::DelimiterAssign);
                    }
                    self.state = ScannerState::WithinTag;
                }
                ScannerState::BeforeAttributeValue => {
                    if self.skip_whitespace() {
                        return self.finish(offset, TokenType::Whitespace);
                    }
                    let mut value = self.advance_while_char(is_unquoted_value_part);
                    if !value.is_empty() {
                        if self.peek() == b'>' && self.bytes()[self.position - 1] == b'/' {
                            self.position -= 1;
                            value = &value[..value.len() - 1];
                        }
                        if self.last_attribute_name.as_deref() == Some("type") {
                            self.last_type_value = Some(value.to_owned());
                        }
                        if !value.is_empty() {
                            self.state = ScannerState::WithinTag;
                            self.has_space_after_tag = false;
                            return self.finish(offset, TokenType::AttributeValue);
                        }
                    }
                    let quote = self.peek();
                    if quote == b'\'' || quote == b'"' {
                        self.position += 1;
                        if self.advance_until_byte(quote) {
                            self.position += 1;
                        }
                        if self.last_attribute_name.as_deref() == Some("type") {
                            self.last_type_value = self
                                .source
                                .get(offset + 1..self.position.saturating_sub(1).max(offset + 1))
                                .map(str::to_owned);
                        }
                        self.state = ScannerState::WithinTag;
                        self.has_space_after_tag = false;
                        return self.finish(offset, TokenType::AttributeValue);
                    }
                    self.state = ScannerState::WithinTag;
                    self.has_space_after_tag = false;
                }
                ScannerState::WithinScriptContent => {
                    let mut script_state = 1u8;
                    while !self.eos() {
                        let Some((start, end, delimiter)) = self.find_script_delimiter() else {
                            self.position = self.source.len();
                            return self.finish(offset, TokenType::Script);
                        };
                        self.position = end;
                        match delimiter {
                            ScriptDelimiter::CommentOpen => {
                                if script_state == 1 {
                                    script_state = 2;
                                }
                            }
                            ScriptDelimiter::CommentClose => script_state = 1,
                            ScriptDelimiter::ScriptOpen => {
                                if script_state == 2 {
                                    script_state = 3;
                                }
                            }
                            ScriptDelimiter::ScriptClose => {
                                if script_state == 3 {
                                    script_state = 2;
                                } else {
                                    self.position = start;
                                    break;
                                }
                            }
                        }
                    }
                    self.state = ScannerState::WithinContent;
                    if offset < self.position {
                        return self.finish(offset, TokenType::Script);
                    }
                }
                ScannerState::WithinStyleContent => {
                    let bytes = self.bytes();
                    self.position = (self.position..bytes.len())
                        .find(|&index| starts_with_ignore_ascii_case(&bytes[index..], STYLE_END))
                        .unwrap_or(bytes.len());
                    self.state = ScannerState::WithinContent;
                    if offset < self.position {
                        return self.finish(offset, TokenType::Styles);
                    }
                }
            }
        }
    }
}
