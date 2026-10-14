use std::collections::BTreeMap;
use std::sync::Arc;

use crate::document::EditorError;
use crate::json_parser::is_ecma_whitespace;
use crate::json_scanner::{self, Kind, Token};
use crate::json_schema_service::{Service, Ticket, Unresolved};
use crate::json_schema_uri;
use crate::json_value::{Arena, Member, Value, sort_members};

const BOM: char = '\u{feff}';
const HTTP_SCHEMA_PREFIX: &str = "http://json-schema.org/";
const HTTPS_SCHEMA_PREFIX: &str = "https://json-schema.org/";
const ERROR_SEPARATOR: &str = "Error: ";
const DOT: u16 = 0x2e;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub scanner: json_scanner::Limits,
    pub uri: json_schema_uri::Limits,
    pub max_values: usize,
    pub max_depth: usize,
    pub max_text_units: usize,
    pub max_work: usize,
    pub max_messages: usize,
    pub max_message_units: usize,
}

#[derive(Debug)]
pub struct Parsed {
    pub schema: Arena,
    pub first_error: Option<usize>,
}

pub enum Response<'a> {
    Unavailable,
    Text(&'a str),
    Failed(&'a [u16]),
}

struct Container<'a> {
    index: usize,
    is_object: bool,
    needs_comma: bool,
    positions: BTreeMap<&'a [u16], usize>,
}

struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    schema: Arena,
    frames: Vec<Container<'a>>,
    first_error: Option<usize>,
    text_units: usize,
    work: usize,
    limits: Limits,
}

impl<'a> Parser<'a> {
    fn current(&self) -> &'a Token {
        &self.tokens[self.cursor]
    }

    fn error(&mut self) {
        if self.first_error.is_none() {
            self.first_error = Some(self.current().utf16.start);
        }
    }

    fn charge(&mut self, count: usize) -> Result<(), EditorError> {
        self.work = self.work.checked_add(count).ok_or(EditorError::Capacity)?;
        if self.work > self.limits.max_work {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn significant(&mut self) -> Result<(), EditorError> {
        loop {
            self.charge(1)?;
            let token = self.current();
            if token.error.is_some() || token.kind == Kind::Unknown {
                self.error();
            }
            if !token.kind.is_trivia() && token.kind != Kind::Unknown {
                return Ok(());
            }
            self.cursor += 1;
        }
    }

    fn next(&mut self) -> Result<(), EditorError> {
        if self.current().kind != Kind::End {
            self.cursor += 1;
        }
        self.significant()
    }

    fn account(&mut self, count: usize) -> Result<(), EditorError> {
        self.text_units = self
            .text_units
            .checked_add(count)
            .ok_or(EditorError::Capacity)?;
        if self.text_units > self.limits.max_text_units {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn start_value(&mut self) -> Result<Option<usize>, EditorError> {
        let token = self.current();
        let value = match token.kind {
            Kind::OpenBrace => Value::Object(Vec::new()),
            Kind::OpenBracket => Value::Array(Vec::new()),
            Kind::String => {
                self.account(token.value.len())?;
                Value::String(token.value.clone())
            }
            Kind::Number => {
                let number = String::from_utf16(&token.value)
                    .map_err(|_| EditorError::InvalidBoundary)?
                    .parse::<f64>()
                    .unwrap_or(f64::NAN);
                if number.is_nan() {
                    self.error();
                    Value::Number(0.0)
                } else {
                    Value::Number(number)
                }
            }
            Kind::Null => Value::Null,
            Kind::True => Value::Boolean(true),
            Kind::False => Value::Boolean(false),
            _ => return Ok(None),
        };
        if self.schema.nodes.len() >= self.limits.max_values {
            return Err(EditorError::Capacity);
        }
        if self.frames.len() > self.limits.max_depth {
            return Err(EditorError::Capacity);
        }
        let index = self.schema.nodes.len();
        self.schema.nodes.push(value);
        if matches!(token.kind, Kind::OpenBrace | Kind::OpenBracket) {
            self.frames.push(Container {
                index,
                is_object: token.kind == Kind::OpenBrace,
                needs_comma: false,
                positions: BTreeMap::new(),
            });
        }
        self.next()?;
        Ok(Some(index))
    }

    fn recover(&mut self, closing: Kind) -> Result<(), EditorError> {
        while !matches!(self.current().kind, Kind::End | Kind::Comma)
            && self.current().kind != closing
        {
            self.next()?;
        }
        Ok(())
    }

    fn run(mut self) -> Result<Parsed, EditorError> {
        self.significant()?;
        self.schema.root = self.start_value()?;
        if self.schema.root.is_none() {
            self.error();
        }
        while !self.frames.is_empty() {
            self.charge(1)?;
            let frame_index = self.frames.len() - 1;
            let frame = &self.frames[frame_index];
            let closing = if frame.is_object {
                Kind::CloseBrace
            } else {
                Kind::CloseBracket
            };
            if self.current().kind == closing || self.current().kind == Kind::End {
                let frame = self.frames.pop().ok_or(EditorError::InvalidBoundary)?;
                if let Value::Object(members) = &mut self.schema.nodes[frame.index] {
                    sort_members(members);
                }
                if self.current().kind == Kind::End {
                    self.error();
                } else {
                    self.next()?;
                }
                continue;
            }
            let needs_comma = frame.needs_comma;
            let is_object = frame.is_object;
            if self.current().kind == Kind::Comma {
                if !needs_comma {
                    self.error();
                }
                self.next()?;
            } else if needs_comma {
                self.error();
            }
            let key = if is_object {
                if self.current().kind != Kind::String {
                    self.error();
                    self.recover(closing)?;
                    self.frames[frame_index].needs_comma = true;
                    continue;
                }
                let key = self.current().value.as_slice();
                self.next()?;
                if self.current().kind != Kind::Colon {
                    self.error();
                    self.recover(closing)?;
                    self.frames[frame_index].needs_comma = true;
                    continue;
                }
                self.next()?;
                Some(key)
            } else {
                None
            };
            let value = self.start_value()?;
            self.frames[frame_index].needs_comma = true;
            let Some(value) = value else {
                self.error();
                self.recover(closing)?;
                continue;
            };
            let parent = self.frames[frame_index].index;
            if let Some(key) = key {
                let position = self.frames[frame_index].positions.get(key).copied();
                if let Some(position) = position {
                    let Value::Object(members) = &mut self.schema.nodes[parent] else {
                        return Err(EditorError::InvalidBoundary);
                    };
                    members[position].value = value;
                } else {
                    self.account(key.len())?;
                    let Value::Object(members) = &mut self.schema.nodes[parent] else {
                        return Err(EditorError::InvalidBoundary);
                    };
                    self.frames[frame_index]
                        .positions
                        .insert(key, members.len());
                    members.push(Member {
                        key: key.to_vec(),
                        value,
                    });
                }
            } else {
                let Value::Array(items) = &mut self.schema.nodes[parent] else {
                    return Err(EditorError::InvalidBoundary);
                };
                items.push(value);
            }
        }
        if self.schema.root.is_some() && self.current().kind != Kind::End {
            self.error();
        }
        Ok(Parsed {
            schema: self.schema,
            first_error: self.first_error,
        })
    }
}

pub fn parse_content(input: &str, limits: Limits) -> Result<Parsed, EditorError> {
    let tokens = json_scanner::scan(input, false, limits.scanner)?;
    Parser {
        tokens: &tokens,
        cursor: 0,
        schema: Arena {
            root: None,
            nodes: Vec::new(),
        },
        frames: Vec::new(),
        first_error: None,
        text_units: 0,
        work: 0,
        limits,
    }
    .run()
}

pub fn request_url(uri: &[u16], limits: json_schema_uri::Limits) -> Result<Vec<u16>, EditorError> {
    if uri.len() > limits.max_input_units || uri.len() > limits.max_work {
        return Err(EditorError::Capacity);
    }
    let prefix = HTTP_SCHEMA_PREFIX.encode_utf16().collect::<Vec<_>>();
    let result = if let Some(rest) = uri.strip_prefix(prefix.as_slice()) {
        HTTPS_SCHEMA_PREFIX
            .encode_utf16()
            .chain(rest.iter().copied())
            .collect::<Vec<_>>()
    } else {
        uri.to_vec()
    };
    if result.len() > limits.max_output_units {
        return Err(EditorError::Capacity);
    }
    Ok(result)
}

pub fn publish_response(
    service: &mut Service<'_>,
    ticket: &Ticket,
    response: Response<'_>,
    is_windows: bool,
    limits: Limits,
) -> Result<(), EditorError> {
    let unresolved = decode(service.request_uri(ticket)?, response, is_windows, limits)?;
    service.publish_load(ticket, unresolved)
}

pub fn decode(
    uri: &[u16],
    response: Response<'_>,
    is_windows: bool,
    limits: Limits,
) -> Result<Unresolved, EditorError> {
    let uri = if matches!(&response, Response::Unavailable) {
        if uri.len() > limits.uri.max_input_units || uri.len() > limits.uri.max_output_units {
            return Err(EditorError::Capacity);
        }
        uri.to_vec()
    } else {
        request_url(uri, limits.uri)?
    };
    let display = json_schema_uri::display_string(&uri, is_windows, limits.uri)?;
    let mut result = Unresolved {
        schema: Arc::new(Arena {
            root: Some(0),
            nodes: vec![Value::Object(Vec::new())],
        }),
        errors: Vec::new(),
    };
    let mut message_units = 0usize;
    let mut message = |prefix: &str, tail: &[u16]| -> Result<(), EditorError> {
        let count = prefix
            .encode_utf16()
            .count()
            .checked_add(display.len())
            .and_then(|count| count.checked_add(tail.len()))
            .ok_or(EditorError::Capacity)?;
        message_units = message_units
            .checked_add(count)
            .ok_or(EditorError::Capacity)?;
        if result.errors.len() >= limits.max_messages || message_units > limits.max_message_units {
            return Err(EditorError::Capacity);
        }
        result.errors.push(
            prefix
                .encode_utf16()
                .chain(display.iter().copied())
                .chain(tail.iter().copied())
                .collect(),
        );
        Ok(())
    };
    match response {
        Response::Unavailable => message(
            "Unable to load schema from '",
            &"'. No schema request service available"
                .encode_utf16()
                .collect::<Vec<_>>(),
        )?,
        Response::Failed(error) => {
            if error.len() > limits.max_text_units || error.len() > limits.max_work {
                return Err(EditorError::Capacity);
            }
            let separator = ERROR_SEPARATOR.encode_utf16().collect::<Vec<_>>();
            let start = error
                .windows(separator.len())
                .position(|part| part == separator)
                .map_or(0, |position| position + separator.len());
            let error = &error[start..];
            let end = error
                .windows(separator.len())
                .position(|part| part == separator)
                .unwrap_or(error.len());
            let error = &error[..end];
            let error = error.strip_suffix(&[DOT]).unwrap_or(error);
            let required = "': ."
                .encode_utf16()
                .count()
                .checked_add(error.len())
                .and_then(|count| count.checked_add(display.len()))
                .and_then(|count| {
                    count.checked_add("Unable to load schema from '".encode_utf16().count())
                })
                .ok_or(EditorError::Capacity)?;
            if required > limits.max_message_units || limits.max_messages == 0 {
                return Err(EditorError::Capacity);
            }
            let tail = "': "
                .encode_utf16()
                .chain(error.iter().copied())
                .chain([DOT])
                .collect::<Vec<_>>();
            message("Unable to load schema from '", &tail)?;
        }
        Response::Text("") => message(
            "Unable to load schema from '",
            &"': No content.".encode_utf16().collect::<Vec<_>>(),
        )?,
        Response::Text(input) => {
            if input.len() > limits.scanner.max_bytes {
                return Err(EditorError::Capacity);
            }
            let input = if input.starts_with(BOM) {
                message(
                    "Problem reading content from '",
                    &"': UTF-8 with BOM detected, only UTF 8 is allowed."
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                )?;
                input.trim_start_matches(is_ecma_whitespace)
            } else {
                input
            };
            let parsed = parse_content(input, limits)?;
            if let Some(offset) = parsed.first_error {
                message(
                    "Unable to parse content from '",
                    &format!("': Parse error at offset {offset}.")
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                )?;
            }
            let is_schema = parsed
                .schema
                .root()
                .and_then(|root| parsed.schema.get(root))
                .is_some_and(|root| matches!(root, Value::Object(_) | Value::Boolean(_)));
            if is_schema {
                result.schema = Arc::new(parsed.schema);
            } else if parsed.first_error.is_none() {
                message(
                    "Unable to load schema from '",
                    &"': Schema must be an object or boolean."
                        .encode_utf16()
                        .collect::<Vec<_>>(),
                )?;
            }
        }
    }
    if result.schema.values().len() > limits.max_values {
        return Err(EditorError::Capacity);
    }
    Ok(result)
}
