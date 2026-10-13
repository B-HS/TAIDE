use crate::document::EditorError;

use super::{MarkupAttribute, MarkupNode, Repeat};

#[derive(Debug, Clone, Copy)]
pub struct ConversionLimits {
    pub max_repeats: usize,
    pub max_nodes: usize,
    pub max_bytes: usize,
    pub max_nesting: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkupValue {
    Text(String),
    Field { index: usize, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkupValueType {
    Raw,
    SingleQuote,
    DoubleQuote,
    Expression,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedMarkupAttribute {
    pub name: Option<String>,
    pub value: Option<Vec<MarkupValue>>,
    pub value_type: MarkupValueType,
    pub is_boolean: bool,
    pub is_implied: bool,
    pub is_multiple: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkupRepeat {
    pub count: usize,
    pub index: usize,
    pub is_implicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedMarkupNode {
    pub name: Option<String>,
    pub value: Option<Vec<MarkupValue>>,
    pub attributes: Option<Vec<ExpandedMarkupAttribute>>,
    pub children: Vec<Self>,
    pub repeat: Option<MarkupRepeat>,
    pub self_closing: bool,
}

struct Converter<'a> {
    variables: &'a [(&'a str, &'a str)],
    limits: ConversionLimits,
    repeaters: Vec<MarkupRepeat>,
    repeat_guard: usize,
    nodes: usize,
    bytes: usize,
    inserted: bool,
}

impl Converter<'_> {
    fn charge(&mut self, bytes: usize) -> Result<(), EditorError> {
        self.bytes = self.bytes.checked_add(bytes).ok_or(EditorError::Capacity)?;
        if self.bytes > self.limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn value(&mut self, raw: &str, fields: bool) -> Result<Vec<MarkupValue>, EditorError> {
        let mut values = Vec::new();
        let mut text = String::new();
        let mut cursor = 0;
        while cursor < raw.len() {
            let character = raw[cursor..]
                .chars()
                .next()
                .ok_or(EditorError::InvalidBoundary)?;
            cursor += character.len_utf8();
            if character == '\\' {
                if let Some(character) = raw[cursor..].chars().next() {
                    text.push(character);
                    cursor += character.len_utf8();
                }
                continue;
            }
            if character != '$' {
                text.push(character);
                continue;
            }
            if fields && raw[cursor..].starts_with('{') {
                cursor += 1;
                let start = cursor;
                let mut depth = 0;
                let mut closed = false;
                while let Some(character) = raw[cursor..].chars().next() {
                    cursor += character.len_utf8();
                    match character {
                        '{' => {
                            depth += 1;
                            if depth > self.limits.max_nesting {
                                return Err(EditorError::Capacity);
                            }
                        }
                        '}' if depth == 0 => {
                            closed = true;
                            break;
                        }
                        '}' => depth -= 1,
                        _ => (),
                    }
                }
                if !closed {
                    return Err(EditorError::Refused);
                }
                let field = &raw[start..cursor - 1];
                let index_end = field.bytes().take_while(u8::is_ascii_digit).count();
                if index_end > 0 {
                    let suffix = &field[index_end..];
                    let name = if suffix.is_empty() {
                        String::new()
                    } else {
                        suffix.strip_prefix(':').ok_or(EditorError::Refused)?.into()
                    };
                    let index = field[..index_end]
                        .parse()
                        .map_err(|_| EditorError::Capacity)?;
                    if !text.is_empty() {
                        self.charge(text.len())?;
                        values.push(MarkupValue::Text(std::mem::take(&mut text)));
                    }
                    self.charge(name.len())?;
                    values.push(MarkupValue::Field { index, name });
                } else if !field.is_empty() {
                    if !field
                        .chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_alphabetic())
                    {
                        return Err(EditorError::Refused);
                    }
                    text.push_str(
                        self.variables
                            .iter()
                            .find(|(name, _)| *name == field)
                            .map_or(field, |(_, value)| *value),
                    );
                }
                continue;
            }
            if raw[cursor..].starts_with('#') {
                cursor += 1;
                self.inserted = true;
                continue;
            }
            let mut size = 1;
            while raw[cursor..].starts_with('$') {
                cursor += 1;
                size += 1;
            }
            let mut parent = 0;
            let mut reverse = false;
            let mut base = 1;
            if raw[cursor..].starts_with('@') {
                cursor += 1;
                while raw[cursor..].starts_with('^') {
                    cursor += 1;
                    parent += 1;
                }
                if raw[cursor..].starts_with('-') {
                    cursor += 1;
                    reverse = true;
                }
                let start = cursor;
                while raw[cursor..]
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit())
                {
                    cursor += 1;
                }
                if cursor != start {
                    base = raw[start..cursor]
                        .parse::<usize>()
                        .map_err(|_| EditorError::Capacity)?;
                }
            }
            let mut number = 1;
            if let Some(repeat) = self.repeaters.last() {
                let index = if reverse {
                    repeat.count - repeat.index - 1
                } else {
                    repeat.index
                };
                number = base.checked_add(index).ok_or(EditorError::Capacity)?;
                let last = self.repeaters.len() - 1;
                let ancestor = last.saturating_sub(parent);
                if parent > 0 && ancestor != last {
                    let offset = repeat
                        .count
                        .checked_mul(self.repeaters[ancestor].index)
                        .ok_or(EditorError::Capacity)?;
                    number = number.checked_add(offset).ok_or(EditorError::Capacity)?;
                }
            }
            if size > self.limits.max_bytes {
                return Err(EditorError::Capacity);
            }
            text.push_str(&format!("{number:0size$}"));
        }
        if !text.is_empty() {
            self.charge(text.len())?;
            values.push(MarkupValue::Text(text));
        }
        Ok(values)
    }

    fn name(&mut self, raw: &str, fields: bool) -> Result<String, EditorError> {
        let values = self.value(raw, fields)?;
        let mut output = String::new();
        for value in values {
            match value {
                MarkupValue::Text(text) => output.push_str(&text),
                MarkupValue::Field { index, name } => {
                    let field = if name.is_empty() {
                        format!("${{{index}}}")
                    } else {
                        format!("${{{index}:{name}}}")
                    };
                    self.charge(field.len() - name.len())?;
                    output.push_str(&field);
                }
            }
        }
        Ok(output)
    }

    fn attribute(&mut self, raw: &MarkupAttribute) -> Result<ExpandedMarkupAttribute, EditorError> {
        let mut name = raw
            .name
            .as_deref()
            .map(|name| self.name(name, true))
            .transpose()?;
        let is_implied = name.as_ref().is_some_and(|name| name.starts_with('!'));
        let is_boolean = name.as_ref().is_some_and(|name| name.ends_with('.'));
        if let Some(name) = &mut name {
            if is_boolean {
                name.pop();
            }
            if is_implied {
                name.remove(0);
            }
        }
        let mut value_type = if raw.is_expression {
            MarkupValueType::Expression
        } else {
            MarkupValueType::Raw
        };
        let value = raw
            .value
            .as_deref()
            .map(|value| {
                let mut value = value;
                if let Some(quote @ ('\'' | '"')) = value.chars().next() {
                    value_type = if quote == '\'' {
                        MarkupValueType::SingleQuote
                    } else {
                        MarkupValueType::DoubleQuote
                    };
                    value = &value[quote.len_utf8()..];
                    if value.ends_with(quote) {
                        value = &value[..value.len() - quote.len_utf8()];
                    }
                } else if !raw.is_expression && value.starts_with('{') {
                    value_type = MarkupValueType::Expression;
                    value = &value[1..];
                    if value.ends_with('}') {
                        value = &value[..value.len() - 1];
                    }
                }
                self.value(value, true)
            })
            .transpose()?;
        Ok(ExpandedMarkupAttribute {
            name,
            value,
            value_type,
            is_boolean,
            is_implied,
            is_multiple: raw.is_multiple,
        })
    }

    fn one(
        &mut self,
        raw: &MarkupNode,
        depth: usize,
        repeat: Option<MarkupRepeat>,
    ) -> Result<Vec<ExpandedMarkupNode>, EditorError> {
        if depth > self.limits.max_nesting {
            return Err(EditorError::Capacity);
        }
        if raw.is_group {
            let mut children = self.convert(&raw.children, depth + 1)?;
            for child in &mut children {
                if child.repeat.is_none() {
                    child.repeat = repeat;
                }
            }
            return Ok(children);
        }
        if self.nodes >= self.limits.max_nodes {
            return Err(EditorError::Capacity);
        }
        self.nodes += 1;
        let name = raw
            .name
            .as_deref()
            .map(|name| self.name(name, false))
            .transpose()?;
        let value = raw
            .text
            .as_deref()
            .map(|text| self.value(text, true))
            .transpose()?;
        let mut children = self.convert(&raw.children, depth + 1)?;
        let attributes = raw
            .has_attributes
            .then(|| {
                raw.attributes
                    .iter()
                    .map(|attribute| self.attribute(attribute))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        let text_only = name.as_ref().is_none_or(|name| name.is_empty())
            && !raw.has_attributes
            && value.as_ref().is_some_and(|values| {
                !values
                    .iter()
                    .any(|value| matches!(value, MarkupValue::Field { .. }))
            });
        let node = ExpandedMarkupNode {
            name,
            value,
            attributes,
            children: if text_only {
                Vec::new()
            } else {
                std::mem::take(&mut children)
            },
            repeat,
            self_closing: raw.self_closing,
        };
        let mut result = vec![node];
        result.extend(children);
        Ok(result)
    }

    fn convert(
        &mut self,
        raw: &[MarkupNode],
        depth: usize,
    ) -> Result<Vec<ExpandedMarkupNode>, EditorError> {
        let mut result = Vec::new();
        for node in raw {
            let Some(repeat) = node.repeat else {
                result.extend(self.one(node, depth, None)?);
                continue;
            };
            let count = match repeat {
                Repeat::Count(count) => count.max(1),
                Repeat::Implicit => 1,
            };
            for index in 0..count {
                let repeat = MarkupRepeat {
                    count,
                    index,
                    is_implicit: repeat == Repeat::Implicit,
                };
                self.repeaters.push(repeat);
                let nodes = self.one(node, depth, Some(repeat));
                self.repeaters.pop();
                let mut nodes = nodes?;
                if repeat.is_implicit
                    && !self.inserted
                    && let Some(last) = nodes.last_mut()
                {
                    let mut deepest = last;
                    while !deepest.children.is_empty() {
                        deepest = deepest.children.last_mut().ok_or(EditorError::Refused)?;
                    }
                    let values = deepest.value.get_or_insert_with(Vec::new);
                    if !matches!(values.last(), Some(MarkupValue::Text(_))) {
                        values.push(MarkupValue::Text(String::new()));
                    }
                }
                result.extend(nodes);
                self.repeat_guard = self.repeat_guard.saturating_sub(1);
                if self.repeat_guard == 0 {
                    break;
                }
            }
            if repeat == Repeat::Implicit {
                self.inserted = true;
            }
        }
        Ok(result)
    }
}

pub fn convert_markup(
    nodes: &[MarkupNode],
    variables: &[(&str, &str)],
    limits: ConversionLimits,
) -> Result<Vec<ExpandedMarkupNode>, EditorError> {
    if limits.max_repeats == 0 && !nodes.is_empty() {
        return Err(EditorError::Capacity);
    }
    Converter {
        variables,
        limits,
        repeaters: Vec::new(),
        repeat_guard: limits.max_repeats,
        nodes: 0,
        bytes: 0,
        inserted: false,
    }
    .convert(nodes, 0)
}
