use std::collections::BTreeMap;
use std::ops::Range;

use crate::document::EditorError;
use crate::json_scanner::{self, Error as ScanError, Kind as TokenKind, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Object,
    Array,
    Property,
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    String(Vec<u16>),
    Number(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: Kind,
    pub bytes: Range<usize>,
    pub utf16: Range<usize>,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub value: Value,
    pub colon_byte: Option<usize>,
    pub colon_utf16: Option<usize>,
    pub is_integer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    ExpectedValue,
    ExpectedEnd,
    InvalidNumber,
    EndOfComment,
    EndOfString,
    EndOfNumber,
    InvalidUnicode,
    InvalidEscape,
    InvalidCharacter,
    PropertyExpected,
    CommaExpected,
    ColonExpected,
    ValueExpected,
    ArrayEndExpected,
    ObjectEndExpected,
    TrailingComma,
    DuplicateKey,
    KeyMustBeQuoted,
}

impl Error {
    pub fn code(self) -> u32 {
        match self {
            Self::ExpectedValue | Self::ExpectedEnd | Self::InvalidNumber => 0,
            Self::EndOfComment => 257,
            Self::EndOfString => 258,
            Self::EndOfNumber => 259,
            Self::InvalidUnicode => 260,
            Self::InvalidEscape => 261,
            Self::InvalidCharacter => 262,
            Self::PropertyExpected => 513,
            Self::CommaExpected => 514,
            Self::ColonExpected => 515,
            Self::ValueExpected => 516,
            Self::ArrayEndExpected => 517,
            Self::ObjectEndExpected => 518,
            Self::TrailingComma => 519,
            Self::DuplicateKey => 520,
            Self::KeyMustBeQuoted => 528,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub error: Error,
    pub bytes: Range<usize>,
    pub utf16: Range<usize>,
    pub is_warning: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    pub bytes: Range<usize>,
    pub utf16: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub root: Option<usize>,
    pub nodes: Vec<Node>,
    pub diagnostics: Vec<Diagnostic>,
    pub comments: Vec<Comment>,
}

impl Document {
    pub fn node_at_utf16(&self, offset: usize, include_right_bound: bool) -> Option<usize> {
        self.node_at(offset, include_right_bound, |node| &node.utf16)
    }

    pub fn node_at_byte(&self, offset: usize, include_right_bound: bool) -> Option<usize> {
        self.node_at(offset, include_right_bound, |node| &node.bytes)
    }

    fn node_at(
        &self,
        offset: usize,
        include_right_bound: bool,
        range: impl Fn(&Node) -> &Range<usize>,
    ) -> Option<usize> {
        let contains = |node: &Node| {
            let range = range(node);
            range.contains(&offset) || include_right_bound && offset == range.end
        };
        let mut current = self.root?;
        if !contains(&self.nodes[current]) {
            return None;
        }
        loop {
            let child = self.nodes[current]
                .children
                .iter()
                .take_while(|&&child| range(&self.nodes[child]).start <= offset)
                .find(|&&child| contains(&self.nodes[child]));
            let Some(&child) = child else {
                return Some(current);
            };
            current = child;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub scanner: json_scanner::Limits,
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_diagnostics: usize,
    pub max_comments: usize,
}

struct Container {
    node: usize,
    depth: usize,
    needs_comma: bool,
    keys: BTreeMap<Vec<u16>, Option<usize>>,
}

enum Task {
    Value {
        parent: Option<usize>,
        depth: usize,
    },
    Container(Container),
    ArrayResume(Container),
    ObjectResume {
        container: Container,
        property: usize,
    },
    PropertyResume(usize),
}

struct Parser<'a> {
    input: &'a str,
    tokens: Vec<Token>,
    cursor: usize,
    document: Document,
    limits: Limits,
    collect_comments: bool,
    last_problem: Option<usize>,
    eof_error_range: Option<(Range<usize>, Range<usize>)>,
}

const COMMENT_KEY: &[u16] = &[u16::from_be_bytes([0, b'/']); 2];

pub(crate) fn is_ecma_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
            | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

impl Parser<'_> {
    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn report(
        &mut self,
        error: Error,
        bytes: Range<usize>,
        utf16: Range<usize>,
        is_warning: bool,
    ) -> Result<(), EditorError> {
        if self.last_problem == Some(utf16.start) {
            return Ok(());
        }
        if self.document.diagnostics.len() >= self.limits.max_diagnostics {
            return Err(EditorError::Capacity);
        }
        self.last_problem = Some(utf16.start);
        self.document.diagnostics.push(Diagnostic {
            error,
            bytes,
            utf16,
            is_warning,
        });
        Ok(())
    }

    fn error(&mut self, error: Error) -> Result<(), EditorError> {
        let token = self.current();
        let mut bytes = token.bytes.clone();
        let mut utf16 = token.utf16.clone();
        if bytes.is_empty() && bytes.start > 0 {
            if let Some((bytes, utf16)) = &self.eof_error_range {
                return self.report(error, bytes.clone(), utf16.clone(), false);
            }
            for (byte, character) in self.input[..bytes.start].char_indices().rev() {
                bytes = byte..byte + character.len_utf8();
                utf16.start -= character.len_utf16();
                utf16.end = utf16.start + character.len_utf16();
                if !is_ecma_whitespace(character) || byte == 0 {
                    break;
                }
            }
            self.eof_error_range = Some((bytes.clone(), utf16.clone()));
        }
        self.report(error, bytes, utf16, false)
    }

    fn scan_error(&mut self) -> Result<(), EditorError> {
        let error = match self.current().error {
            None => return Ok(()),
            Some(ScanError::EndOfComment) => Error::EndOfComment,
            Some(ScanError::EndOfString) => Error::EndOfString,
            Some(ScanError::EndOfNumber) => Error::EndOfNumber,
            Some(ScanError::InvalidUnicode) => Error::InvalidUnicode,
            Some(ScanError::InvalidEscape) => Error::InvalidEscape,
            Some(ScanError::InvalidCharacter) => Error::InvalidCharacter,
        };
        self.error(error)
    }

    fn significant(&mut self) -> Result<(), EditorError> {
        loop {
            self.scan_error()?;
            let token = self.current();
            if !token.kind.is_trivia() {
                return Ok(());
            }
            if self.collect_comments
                && matches!(token.kind, TokenKind::LineComment | TokenKind::BlockComment)
            {
                if self.document.comments.len() >= self.limits.max_comments {
                    return Err(EditorError::Capacity);
                }
                self.document.comments.push(Comment {
                    bytes: token.bytes.clone(),
                    utf16: token.utf16.clone(),
                });
            }
            self.cursor += 1;
        }
    }

    fn next(&mut self) -> Result<(), EditorError> {
        if self.current().kind != TokenKind::End {
            self.cursor += 1;
        }
        self.significant()
    }

    fn add_node(
        &mut self,
        kind: Kind,
        parent: Option<usize>,
        depth: usize,
        value: Value,
    ) -> Result<usize, EditorError> {
        if self.document.nodes.len() >= self.limits.max_nodes || depth > self.limits.max_depth {
            return Err(EditorError::Capacity);
        }
        let token = self.current();
        let node = Node {
            kind,
            parent,
            bytes: token.bytes.clone(),
            utf16: token.utf16.clone(),
            children: Vec::new(),
            value,
            colon_byte: None,
            colon_utf16: None,
            is_integer: true,
        };
        let index = self.document.nodes.len();
        self.document.nodes.push(node);
        Ok(index)
    }

    fn finalize(&mut self, node: usize) {
        let byte_end = self.current().bytes.end;
        let utf16_end = self.current().utf16.end;
        self.document.nodes[node].bytes.end = byte_end;
        self.document.nodes[node].utf16.end = utf16_end;
    }

    fn recover(&mut self, closing: TokenKind) -> Result<(), EditorError> {
        while !matches!(self.current().kind, TokenKind::End | TokenKind::Comma)
            && self.current().kind != closing
        {
            self.next()?;
        }
        Ok(())
    }

    fn property_key(&mut self, container: &mut Container) -> Result<Option<usize>, EditorError> {
        let token_kind = self.current().kind;
        if !matches!(token_kind, TokenKind::String | TokenKind::Unknown) {
            return Ok(None);
        }
        let property = self.add_node(
            Kind::Property,
            Some(container.node),
            container.depth + 1,
            Value::None,
        )?;
        if token_kind == TokenKind::Unknown {
            self.error(Error::KeyMustBeQuoted)?;
        }
        let value = std::mem::take(&mut self.tokens[self.cursor].value);
        let key = self.add_node(
            Kind::String,
            Some(property),
            container.depth + 2,
            Value::String(value),
        )?;
        self.document.nodes[property].children.push(key);
        self.next()?;
        let Value::String(key_value) = &self.document.nodes[key].value else {
            unreachable!()
        };
        if key_value != COMMENT_KEY {
            let seen = container.keys.get(key_value).copied();
            let key_value = key_value.clone();
            if let Some(first) = seen {
                let key_node = &self.document.nodes[key];
                self.report(
                    Error::DuplicateKey,
                    key_node.bytes.clone(),
                    key_node.utf16.clone(),
                    true,
                )?;
                if let Some(first) = first {
                    let first_key = self.document.nodes[first].children[0];
                    let first_key = &self.document.nodes[first_key];
                    self.report(
                        Error::DuplicateKey,
                        first_key.bytes.clone(),
                        first_key.utf16.clone(),
                        true,
                    )?;
                }
                container.keys.insert(key_value, None);
            } else {
                container.keys.insert(key_value, Some(property));
            }
        }
        if self.current().kind == TokenKind::Colon {
            self.document.nodes[property].colon_byte = Some(self.current().bytes.start);
            self.document.nodes[property].colon_utf16 = Some(self.current().utf16.start);
            self.next()?;
        } else {
            self.error(Error::ColonExpected)?;
            if self.current().kind == TokenKind::String {
                let key_end = self.document.nodes[key].bytes.end;
                let next_start = self.current().bytes.start;
                if self.input[key_end..next_start].contains(['\r', '\n']) {
                    self.document.nodes[property].bytes.end = key_end;
                    self.document.nodes[property].utf16.end = self.document.nodes[key].utf16.end;
                    container.needs_comma = true;
                    self.document.nodes[container.node].children.push(property);
                    return Ok(None);
                }
            }
        }
        Ok(Some(property))
    }

    fn run(&mut self) -> Result<Option<usize>, EditorError> {
        self.significant()?;
        if self.current().kind == TokenKind::End {
            return Ok(None);
        }
        let mut tasks = vec![Task::Value {
            parent: None,
            depth: 1,
        }];
        let mut result = None;
        while let Some(task) = tasks.pop() {
            match task {
                Task::Value { parent, depth } => {
                    result = None;
                    let token_kind = self.current().kind;
                    let (kind, value) = match token_kind {
                        TokenKind::OpenBrace => (Kind::Object, Value::None),
                        TokenKind::OpenBracket => (Kind::Array, Value::None),
                        TokenKind::String => (
                            Kind::String,
                            Value::String(std::mem::take(&mut self.tokens[self.cursor].value)),
                        ),
                        TokenKind::Number => (Kind::Number, Value::Number(f64::NAN)),
                        TokenKind::True => (Kind::Boolean, Value::Boolean(true)),
                        TokenKind::False => (Kind::Boolean, Value::Boolean(false)),
                        TokenKind::Null => (Kind::Null, Value::Null),
                        _ => continue,
                    };
                    let node = self.add_node(kind, parent, depth, value)?;
                    if kind == Kind::Number && self.current().error.is_none() {
                        let raw = &self.input[self.current().bytes.clone()];
                        let Ok(value) = raw.parse::<f64>() else {
                            self.error(Error::InvalidNumber)?;
                            result = Some(node);
                            continue;
                        };
                        self.document.nodes[node].value = Value::Number(value);
                        self.document.nodes[node].is_integer = !raw.contains('.');
                    }
                    self.next()?;
                    if matches!(kind, Kind::Object | Kind::Array) {
                        tasks.push(Task::Container(Container {
                            node,
                            depth,
                            needs_comma: false,
                            keys: BTreeMap::new(),
                        }));
                    } else {
                        result = Some(node);
                    }
                }
                Task::Container(mut container) => {
                    let is_array = self.document.nodes[container.node].kind == Kind::Array;
                    let closing = if is_array {
                        TokenKind::CloseBracket
                    } else {
                        TokenKind::CloseBrace
                    };
                    if matches!(self.current().kind, TokenKind::End)
                        || self.current().kind == closing
                    {
                        self.finalize(container.node);
                        if self.current().kind == closing {
                            self.next()?;
                        } else {
                            self.error(if is_array {
                                Error::ArrayEndExpected
                            } else {
                                Error::ObjectEndExpected
                            })?;
                        }
                        result = Some(container.node);
                        continue;
                    }
                    if self.current().kind == TokenKind::Comma {
                        if !container.needs_comma {
                            self.error(if is_array {
                                Error::ValueExpected
                            } else {
                                Error::PropertyExpected
                            })?;
                        }
                        let comma_bytes = self.current().bytes.clone();
                        let comma_utf16 = self.current().utf16.clone();
                        self.next()?;
                        if self.current().kind == closing {
                            if container.needs_comma {
                                self.report(Error::TrailingComma, comma_bytes, comma_utf16, false)?;
                            }
                            tasks.push(Task::Container(container));
                            continue;
                        }
                    } else if container.needs_comma {
                        self.error(Error::CommaExpected)?;
                    }
                    if is_array {
                        let parent = Some(container.node);
                        let depth = container.depth + 1;
                        tasks.push(Task::ArrayResume(container));
                        tasks.push(Task::Value { parent, depth });
                    } else {
                        let previous_children = self.document.nodes[container.node].children.len();
                        match self.property_key(&mut container)? {
                            Some(property) => {
                                let depth = container.depth + 2;
                                tasks.push(Task::ObjectResume {
                                    container,
                                    property,
                                });
                                tasks.push(Task::PropertyResume(property));
                                tasks.push(Task::Value {
                                    parent: Some(property),
                                    depth,
                                });
                            }
                            None => {
                                if self.document.nodes[container.node].children.len()
                                    == previous_children
                                {
                                    self.error(Error::PropertyExpected)?;
                                    self.recover(closing)?;
                                }
                                container.needs_comma = true;
                                tasks.push(Task::Container(container));
                            }
                        }
                    }
                }
                Task::ArrayResume(mut container) => {
                    if let Some(value) = result.take() {
                        self.document.nodes[container.node].children.push(value);
                    } else {
                        self.error(Error::ValueExpected)?;
                        self.recover(TokenKind::CloseBracket)?;
                    }
                    container.needs_comma = true;
                    tasks.push(Task::Container(container));
                }
                Task::PropertyResume(property) => {
                    if let Some(value) = result.take() {
                        self.document.nodes[property].bytes.end =
                            self.document.nodes[value].bytes.end;
                        self.document.nodes[property].utf16.end =
                            self.document.nodes[value].utf16.end;
                        self.document.nodes[property].children.push(value);
                    } else {
                        self.error(Error::ValueExpected)?;
                        self.finalize(property);
                        self.recover(TokenKind::CloseBrace)?;
                    }
                }
                Task::ObjectResume {
                    mut container,
                    property,
                } => {
                    self.document.nodes[container.node].children.push(property);
                    container.needs_comma = true;
                    tasks.push(Task::Container(container));
                }
            }
        }
        if result.is_none() {
            self.error(Error::ExpectedValue)?;
        } else if self.current().kind != TokenKind::End {
            self.error(Error::ExpectedEnd)?;
        }
        Ok(result)
    }
}

pub fn parse(input: &str, collect_comments: bool, limits: Limits) -> Result<Document, EditorError> {
    let tokens = json_scanner::scan(input, false, limits.scanner)?;
    let mut parser = Parser {
        input,
        tokens,
        cursor: 0,
        document: Document {
            root: None,
            nodes: Vec::new(),
            diagnostics: Vec::new(),
            comments: Vec::new(),
        },
        limits,
        collect_comments,
        last_problem: None,
        eof_error_range: None,
    };
    parser.document.root = parser.run()?;
    Ok(parser.document)
}
