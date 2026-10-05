use std::cmp::Ordering;

use crate::document::EditorError;
pub use crate::snippet_normalization::{FinalTabstopOptions, RegexMetadata, parse_complete};

pub(crate) const MAX_STACK_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Index(f64);

impl Index {
    pub const FINAL: Self = Self(0.0);

    pub fn value(self) -> f64 {
        self.0
    }
}

impl Eq for Index {}

impl Ord for Index {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialOrd for Index {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Marker {
    Text(String),
    Placeholder {
        index: Index,
        children: Vec<Marker>,
        choices: Option<Vec<String>>,
        transform: Option<Transform>,
    },
    Variable {
        name: String,
        children: Vec<Marker>,
        transform: Option<Transform>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transform {
    pub pattern: String,
    pub options: String,
    pub format: Vec<FormatPart>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatPart {
    Text(String),
    Capture {
        index: Index,
        shorthand: Option<String>,
        if_value: Option<String>,
        else_value: Option<String>,
    },
}

#[derive(Clone, Copy)]
pub struct ParseLimits {
    pub max_bytes: usize,
    pub max_nesting: usize,
    pub max_markers: usize,
}

struct Parser<'a, F> {
    input: &'a str,
    position: usize,
    limits: ParseLimits,
    markers: usize,
    validate_transform: F,
}

enum Name<'a> {
    Index(Index, &'a str),
    Variable(&'a str),
}

pub fn parse(
    input: &str,
    limits: ParseLimits,
    validate_transform: impl FnMut(&str, &str) -> bool,
) -> Result<Vec<Marker>, EditorError> {
    if input.len() > limits.max_bytes
        || limits.max_nesting > MAX_STACK_DEPTH
        || limits.max_markers == 0
    {
        return Err(EditorError::Capacity);
    }
    let mut parser = Parser {
        input,
        position: 0,
        limits,
        markers: 0,
        validate_transform,
    };
    parser.children(false, 0).map(|(children, _)| children)
}

impl<F: FnMut(&str, &str) -> bool> Parser<'_, F> {
    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let next = self.peek()?;
        self.position += next.len_utf8();
        Some(next)
    }

    fn take(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            return true;
        }
        false
    }

    fn record(&mut self) -> Result<(), EditorError> {
        if self.markers >= self.limits.max_markers {
            return Err(EditorError::Capacity);
        }
        self.markers += 1;
        Ok(())
    }

    fn text(&mut self, children: &mut Vec<Marker>, value: &str) -> Result<(), EditorError> {
        if let Some(Marker::Text(last)) = children.last_mut() {
            last.push_str(value);
            return Ok(());
        }
        self.record()?;
        children.push(Marker::Text(value.into()));
        Ok(())
    }

    fn children(
        &mut self,
        until_close: bool,
        depth: usize,
    ) -> Result<(Vec<Marker>, bool), EditorError> {
        if depth > self.limits.max_nesting {
            return Err(EditorError::Capacity);
        }
        let mut children = Vec::new();
        while let Some(next) = self.peek() {
            if until_close && self.take('}') {
                return Ok((children, true));
            }
            if self.take('\\') {
                let escaped = match self.peek() {
                    Some('$' | '}' | '\\') => self.bump().unwrap(),
                    _ => '\\',
                };
                self.text(&mut children, &escaped.to_string())?;
                continue;
            }
            if next == '$'
                && let Some(markers) = self.dollar(depth)?
            {
                for marker in markers {
                    match marker {
                        Marker::Text(text) => self.text(&mut children, &text)?,
                        marker => children.push(marker),
                    }
                }
                continue;
            }
            let next = self.bump().unwrap();
            self.text(&mut children, &next.to_string())?;
        }
        Ok((children, false))
    }

    fn name(&mut self) -> Option<Name<'_>> {
        let start = self.position;
        let first = self.peek()?;
        if first.is_ascii_digit() {
            while self.peek().is_some_and(|value| value.is_ascii_digit()) {
                self.bump();
            }
            let digits = &self.input[start..self.position];
            let value = digits.parse::<f64>().ok()?;
            return Some(Name::Index(Index(value), digits));
        }
        if first == '_' || first.is_ascii_alphabetic() {
            while self
                .peek()
                .is_some_and(|value| value == '_' || value.is_ascii_alphanumeric())
            {
                self.bump();
            }
            return Some(Name::Variable(&self.input[start..self.position]));
        }
        None
    }

    fn dollar(&mut self, depth: usize) -> Result<Option<Vec<Marker>>, EditorError> {
        let start = self.position;
        self.bump();
        let complex = self.take('{');
        let Some(name) = self.name() else {
            self.position = start;
            return Ok(None);
        };
        let (index, name, raw_name) = match name {
            Name::Index(index, raw) => (Some(index), None, raw.to_owned()),
            Name::Variable(name) => (None, Some(name.to_owned()), name.to_owned()),
        };
        let mut children = Vec::new();
        let mut choices = None;
        let mut transform = None;
        if complex {
            if self.take(':') {
                let (nested, closed) = self.children(true, depth + 1)?;
                if !closed {
                    let mut fallback = vec![Marker::Text(format!("${{{raw_name}:"))];
                    fallback.extend(nested);
                    return Ok(Some(fallback));
                }
                children = nested;
            } else if index.is_some_and(|index| index != Index::FINAL) && self.take('|') {
                let Some(parsed) = self.choices() else {
                    self.position = start;
                    return Ok(None);
                };
                self.record()?;
                choices = Some(parsed);
            } else if self.take('/') {
                let Some(parsed) = self.transform()? else {
                    self.position = start;
                    return Ok(None);
                };
                transform = Some(parsed);
            } else if !self.take('}') {
                self.position = start;
                return Ok(None);
            }
        }
        self.record()?;
        let marker = if let Some(index) = index {
            Marker::Placeholder {
                index,
                children,
                choices,
                transform,
            }
        } else {
            Marker::Variable {
                name: name.unwrap(),
                children,
                transform,
            }
        };
        Ok(Some(vec![marker]))
    }

    fn choices(&mut self) -> Option<Vec<String>> {
        let mut choices = Vec::new();
        loop {
            let mut text = String::new();
            loop {
                let next = self.peek()?;
                if next == ',' || next == '|' {
                    break;
                }
                self.bump();
                if next == '\\' && matches!(self.peek(), Some(',' | '|' | '\\')) {
                    text.push(self.bump()?);
                } else {
                    text.push(next);
                }
            }
            if text.is_empty() {
                return None;
            }
            choices.push(text);
            if self.take(',') {
                continue;
            }
            if self.take('|') && self.take('}') {
                return Some(choices);
            }
            return None;
        }
    }

    fn format_text(
        &mut self,
        format: &mut Vec<FormatPart>,
        value: char,
    ) -> Result<(), EditorError> {
        if let Some(FormatPart::Text(text)) = format.last_mut() {
            text.push(value);
            return Ok(());
        }
        self.record()?;
        format.push(FormatPart::Text(value.to_string()));
        Ok(())
    }

    fn transform(&mut self) -> Result<Option<Transform>, EditorError> {
        let mut pattern = String::new();
        loop {
            let Some(next) = self.bump() else {
                return Ok(None);
            };
            if next == '/' {
                break;
            }
            if next == '\\' && self.take('/') {
                pattern.push('/');
            } else {
                pattern.push(next);
            }
        }
        let mut format = Vec::new();
        loop {
            let Some(next) = self.peek() else {
                return Ok(None);
            };
            if self.take('/') {
                break;
            }
            if self.take('\\') {
                let escaped = match self.peek() {
                    Some('\\' | '/') => self.bump().unwrap(),
                    _ => '\\',
                };
                self.format_text(&mut format, escaped)?;
                continue;
            }
            if next == '$'
                && let Some(part) = self.capture()
            {
                self.record()?;
                format.push(part);
                continue;
            }
            let next = self.bump().unwrap();
            self.format_text(&mut format, next)?;
        }
        let start = self.position;
        loop {
            let Some(next) = self.bump() else {
                return Ok(None);
            };
            if next == '}' {
                break;
            }
        }
        let options = self.input[start..self.position - 1].to_owned();
        if !(self.validate_transform)(&pattern, &options) {
            return Ok(None);
        }
        Ok(Some(Transform {
            pattern,
            options,
            format,
        }))
    }

    fn capture(&mut self) -> Option<FormatPart> {
        let start = self.position;
        self.bump();
        let complex = self.take('{');
        let result = self.capture_inner(complex);
        if result.is_none() {
            self.position = start;
        }
        result
    }

    fn capture_inner(&mut self, complex: bool) -> Option<FormatPart> {
        let Name::Index(index, _) = self.name()? else {
            return None;
        };
        let mut shorthand = None;
        let mut if_value = None;
        let mut else_value = None;
        if complex && !self.take('}') {
            if !self.take(':') {
                return None;
            }
            if self.take('/') {
                let Name::Variable(name) = self.name()? else {
                    return None;
                };
                shorthand = Some(name.to_owned());
                if !self.take('}') {
                    return None;
                }
            } else if self.take('+') {
                if_value = Some(self.conditional('}')?);
            } else if self.take('?') {
                if_value = Some(self.conditional(':')?);
                else_value = Some(self.conditional('}')?);
            } else {
                self.take('-');
                else_value = Some(self.conditional('}')?);
            }
        }
        Some(FormatPart::Capture {
            index,
            shorthand,
            if_value,
            else_value,
        })
    }

    fn conditional(&mut self, delimiter: char) -> Option<String> {
        let mut text = String::new();
        loop {
            let next = self.bump()?;
            if next == delimiter {
                return (!text.is_empty()).then_some(text);
            }
            if next == '\\' {
                let escaped = self.bump()?;
                if !matches!(escaped, '$' | '}' | '\\') {
                    return None;
                }
                text.push(escaped);
            } else {
                text.push(next);
            }
        }
    }
}
