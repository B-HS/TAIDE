use std::ops::Range;

use crate::document::EditorError;
use crate::snippet_syntax::Index;

use super::snippet_output::placeholder;
use super::stylesheet_format::{color, number};
use super::{
    ConversionLimits, MarkupLimits, StylesheetProperty, StylesheetToken, StylesheetValue,
    prepare_stylesheet,
};

const NUMERIC_DECIMALS: u32 = 4;
const UTF8_MAX_BYTES: usize = 4;

struct Field {
    digits: Range<usize>,
    index: usize,
    has_placeholder: bool,
}

struct Output {
    text: String,
    fields: Vec<Field>,
    max_bytes: usize,
    max_nesting: usize,
}

impl Output {
    fn push(&mut self, text: &str) -> Result<(), EditorError> {
        if self
            .text
            .len()
            .checked_add(text.len())
            .ok_or(EditorError::Capacity)?
            > self.max_bytes
        {
            return Err(EditorError::Capacity);
        }
        self.text.push_str(text);
        Ok(())
    }

    fn literal(&mut self, value: &str) -> Result<(), EditorError> {
        let value = value.replace("\r\n", "\n").replace('\r', "\n");
        for character in value.chars() {
            if matches!(character, '$' | '\\') {
                self.push("\\")?;
            }
            self.push(character.encode_utf8(&mut [0; UTF8_MAX_BYTES]))?;
        }
        Ok(())
    }

    fn field(&mut self, index: Option<usize>, name: &str) -> Result<(), EditorError> {
        let Some(index) = index else {
            let (variable, default) = name
                .split_once(':')
                .map_or((name, None), |(variable, default)| {
                    (variable, Some(default))
                });
            let mut characters = variable.chars();
            if !characters
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
                || !characters
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                return Err(EditorError::Refused);
            }
            self.push("${")?;
            self.push(variable)?;
            self.push(":")?;
            placeholder(default.unwrap_or(variable), self.max_nesting, |text| {
                self.push(text)
            })?;
            self.push("}")?;
            return Ok(());
        };
        if index > 0 {
            Index::ordered(index)?;
        }
        self.push("${")?;
        let start = self.text.len();
        self.push(&index.to_string())?;
        let end = self.text.len();
        if !name.is_empty() {
            self.push(":")?;
            placeholder(name, self.max_nesting, |text| self.push(text))?;
        }
        self.push("}")?;
        self.fields.push(Field {
            digits: start..end,
            index,
            has_placeholder: !name.is_empty(),
        });
        Ok(())
    }

    fn string(&mut self, value: &str) -> Result<(), EditorError> {
        let mut position = 0;
        let mut text = String::new();
        while position < value.len() {
            let rest = &value[position..];
            if rest.starts_with('\\') {
                let length = rest.bytes().take_while(|byte| *byte == b'\\').count();
                if rest[length..].starts_with("${") {
                    let is_escaped = length % 2 != 0;
                    text.push_str(&rest[..length - usize::from(is_escaped)]);
                    position += length;
                    if is_escaped {
                        text.push_str("${");
                        position += "${".len();
                    }
                    continue;
                }
            }
            if rest.starts_with("${") {
                let start = position + "${".len();
                let mut cursor = start;
                let mut depth = 0usize;
                let mut closed = false;
                while let Some(character) = value[cursor..].chars().next() {
                    if character == '}' && depth == 0 {
                        closed = true;
                        break;
                    }
                    cursor += character.len_utf8();
                    if character == '\\'
                        && let Some(next) = value[cursor..].chars().next()
                    {
                        cursor += next.len_utf8();
                        continue;
                    }
                    if character == '{' {
                        depth += 1;
                        if depth > self.max_nesting {
                            return Err(EditorError::Capacity);
                        }
                    }
                    if character == '}' {
                        depth -= 1;
                    }
                }
                if closed {
                    let body = &value[start..cursor];
                    let (head, default) = body.split_once(':').map_or((body, ""), |parts| parts);
                    let index =
                        if !head.is_empty() && head.bytes().all(|byte| byte.is_ascii_digit()) {
                            Some(head.parse().map_err(|_| EditorError::Capacity)?)
                        } else {
                            None
                        };
                    if index.is_some()
                        || head.chars().next().is_some_and(|character| {
                            character.is_ascii_alphabetic() || character == '_'
                        })
                    {
                        self.literal(&text)?;
                        text.clear();
                        if let Some(index) = index {
                            self.field(Some(index), default)?;
                        } else {
                            self.field(None, body)?;
                        }
                        position = cursor + 1;
                        continue;
                    }
                }
            }
            let character = rest.chars().next().ok_or(EditorError::InvalidBoundary)?;
            text.push(character);
            position += character.len_utf8();
        }
        self.literal(&text)
    }

    fn token(&mut self, token: &StylesheetToken, depth: usize) -> Result<(), EditorError> {
        if depth > self.max_nesting {
            return Err(EditorError::Capacity);
        }
        match &token.value {
            StylesheetValue::Literal(value) | StylesheetValue::CustomProperty(value) => {
                self.literal(value)
            }
            StylesheetValue::Number { value, unit, .. } => {
                self.literal(&number(*value, NUMERIC_DECIMALS)?)?;
                self.literal(unit)
            }
            StylesheetValue::Color {
                red,
                green,
                blue,
                alpha,
                ..
            } => self.literal(&color(*red, *green, *blue, *alpha)?),
            StylesheetValue::String { value, quote } => {
                self.literal(&quote.to_string())?;
                self.string(value)?;
                self.literal(&quote.to_string())
            }
            StylesheetValue::Field { index, name, .. } => self.field(*index, name),
            StylesheetValue::Function { name, arguments } => {
                self.literal(name)?;
                self.literal("(")?;
                self.fragments(arguments, depth + 1)?;
                self.literal(")")
            }
        }
    }

    fn fragments(
        &mut self,
        fragments: &[Vec<StylesheetToken>],
        depth: usize,
    ) -> Result<(), EditorError> {
        for (index, fragment) in fragments.iter().enumerate() {
            if index > 0 {
                self.literal(", ")?;
            }
            for (index, token) in fragment.iter().enumerate() {
                if index > 0
                    && !matches!(
                        token.value,
                        StylesheetValue::Field {
                            is_adjacent: true,
                            ..
                        }
                    )
                {
                    self.literal(" ")?;
                }
                self.token(token, depth)?;
            }
        }
        Ok(())
    }

    fn properties(&mut self, properties: &[StylesheetProperty]) -> Result<(), EditorError> {
        for (index, property) in properties.iter().enumerate() {
            if index > 0 {
                self.literal("\n")?;
            }
            if let Some(name) = &property.name {
                self.literal(name)?;
                self.literal(": ")?;
                if property.values.is_empty() {
                    self.field(Some(0), "")?;
                } else {
                    self.fragments(&property.values, 0)?;
                }
                if property.is_important {
                    self.literal(" !important")?;
                }
                self.literal(";")?;
                continue;
            }
            for token in property.values.iter().flatten() {
                self.token(token, 0)?;
            }
            if property.is_important {
                if !property.values.is_empty() {
                    self.literal(" ")?;
                }
                self.literal("!important")?;
            }
        }
        Ok(())
    }

    fn finish(mut self) -> String {
        if self.fields.iter().any(|field| field.index == 0) {
            return self.text;
        }
        if let Some(largest) = self
            .fields
            .iter()
            .max_by_key(|field| field.index)
            .map(|field| field.index)
            && self
                .fields
                .iter()
                .find(|field| field.index == largest)
                .is_some_and(|field| !field.has_placeholder)
        {
            for field in self
                .fields
                .iter()
                .rev()
                .filter(|field| field.index == largest)
            {
                self.text.replace_range(field.digits.clone(), "0");
            }
        }
        self.text
    }
}

pub fn render_stylesheet(
    properties: &[StylesheetProperty],
    max_bytes: usize,
    max_nesting: usize,
) -> Result<String, EditorError> {
    let mut output = Output {
        text: String::new(),
        fields: Vec::new(),
        max_bytes,
        max_nesting,
    };
    output.properties(properties)?;
    Ok(output.finish())
}

pub fn expand_stylesheet(
    input: &str,
    parse_limits: MarkupLimits,
    limits: ConversionLimits,
) -> Result<String, EditorError> {
    let properties = prepare_stylesheet(input, parse_limits, limits)?;
    render_stylesheet(&properties, limits.max_bytes, limits.max_nesting)
}
