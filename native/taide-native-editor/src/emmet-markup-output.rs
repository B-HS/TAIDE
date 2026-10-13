use std::ops::Range;

use crate::document::EditorError;

use super::markup_filters::apply_markup_filters;
use super::markup_preparation::{BOOLEAN_ATTRIBUTES, INLINE_ELEMENTS};
use super::{
    ConversionLimits, ExpandedMarkupAttribute, ExpandedMarkupNode, MarkupLimits, MarkupValue,
    MarkupValueType, Syntax, markup_filters, prepare_markup, prepare_markup_with_filters,
    prepare_markup_with_random,
};

const INDENT: &str = "\t";
const UTF8_MAX_BYTES: usize = 4;
const RESERVED_PROPERTIES: &[&str] = &[
    "for",
    "while",
    "of",
    "async",
    "await",
    "const",
    "let",
    "var",
    "continue",
    "break",
    "debugger",
    "do",
    "export",
    "import",
    "in",
    "instanceof",
    "new",
    "return",
    "switch",
    "this",
    "throw",
    "try",
    "catch",
    "typeof",
    "void",
    "with",
    "yield",
];

struct Field {
    digits: Range<usize>,
    index: usize,
    has_placeholder: bool,
}

struct Output {
    text: String,
    syntax: Syntax,
    level: usize,
    line: usize,
    field: usize,
    fields: Vec<Field>,
    max_bytes: usize,
    max_nesting: usize,
    is_comment_enabled: bool,
}

fn is_snippet(node: &ExpandedMarkupNode) -> bool {
    node.name.as_ref().is_none_or(|name| name.is_empty()) && node.attributes.is_none()
}

fn is_inline(node: &ExpandedMarkupNode) -> bool {
    if let Some(name) = node.name.as_ref().filter(|name| !name.is_empty()) {
        return INLINE_ELEMENTS.contains(&name.to_ascii_lowercase().as_str());
    }
    node.value.is_some() && node.attributes.is_none()
}

fn has_newline(values: &[MarkupValue]) -> bool {
    values
        .iter()
        .any(|value| matches!(value, MarkupValue::Text(text) if text.contains(['\n', '\r'])))
}

fn starts_with_block(values: &[MarkupValue]) -> bool {
    let Some(MarkupValue::Text(text)) = values.first() else {
        return false;
    };
    let Some(rest) = text.strip_prefix('<') else {
        return false;
    };
    let name = rest
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    !name.is_empty() && !INLINE_ELEMENTS.contains(&name.to_ascii_lowercase().as_str())
}

fn should_format(
    index: usize,
    items: &[ExpandedMarkupNode],
    parent: Option<&ExpandedMarkupNode>,
) -> bool {
    let node = &items[index];
    if index == 0 && parent.is_none() {
        return false;
    }
    if parent.is_some_and(is_snippet) && items.len() == 1 {
        return false;
    }
    if is_snippet(node) {
        let adjacent = index
            .checked_sub(1)
            .and_then(|index| items.get(index))
            .is_some_and(is_snippet)
            || items.get(index + 1).is_some_and(is_snippet);
        let values = node.value.as_deref().unwrap_or_default();
        if adjacent
            || has_newline(values)
            || !node.children.is_empty()
                && values
                    .iter()
                    .any(|value| matches!(value, MarkupValue::Field { .. }))
        {
            return true;
        }
    }
    if !is_inline(node) {
        return true;
    }
    if index == 0 && items.iter().any(|node| !is_inline(node)) {
        return true;
    }
    if index > 0 && !is_inline(&items[index - 1]) {
        return true;
    }
    node.children
        .iter()
        .enumerate()
        .any(|(index, _)| should_format(index, &node.children, parent))
}

impl Output {
    fn push(&mut self, value: &str) -> Result<(), EditorError> {
        if self
            .text
            .len()
            .checked_add(value.len())
            .is_none_or(|bytes| bytes > self.max_bytes)
        {
            return Err(EditorError::Capacity);
        }
        self.text.push_str(value);
        Ok(())
    }

    fn newline(&mut self, level: usize) -> Result<(), EditorError> {
        self.push("\n")?;
        self.line += 1;
        if level > self.max_nesting {
            return Err(EditorError::Capacity);
        }
        for _ in 0..level {
            self.push(INDENT)?;
        }
        Ok(())
    }

    fn literal(&mut self, value: &str) -> Result<(), EditorError> {
        let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
        for (index, line) in normalized.split('\n').enumerate() {
            if index > 0 {
                self.newline(self.level)?;
            }
            for character in line.chars() {
                if character == '$' && !self.text.ends_with('\\') {
                    self.push("\\")?;
                }
                self.push(character.encode_utf8(&mut [0; UTF8_MAX_BYTES]))?;
            }
        }
        Ok(())
    }

    fn tokens(&mut self, values: &[MarkupValue]) -> Result<(), EditorError> {
        let mut largest = None::<usize>;
        for value in values {
            match value {
                MarkupValue::Text(text) => self.literal(text)?,
                MarkupValue::Field { index, name } => {
                    let number = self
                        .field
                        .checked_add(*index)
                        .ok_or(EditorError::Capacity)?;
                    self.push("${")?;
                    let start = self.text.len();
                    self.push(&number.to_string())?;
                    let end = self.text.len();
                    if !name.is_empty() {
                        self.push(":")?;
                        self.push(name)?;
                    }
                    self.push("}")?;
                    self.fields.push(Field {
                        digits: start..end,
                        index: number,
                        has_placeholder: !name.is_empty(),
                    });
                    largest = Some(largest.map_or(*index, |largest| largest.max(*index)));
                }
            }
        }
        if let Some(largest) = largest {
            self.field = self
                .field
                .checked_add(largest)
                .and_then(|index| index.checked_add(1))
                .ok_or(EditorError::Capacity)?;
        }
        Ok(())
    }

    fn caret(&mut self) -> Result<(), EditorError> {
        self.tokens(&[MarkupValue::Field {
            index: 0,
            name: String::new(),
        }])
    }

    fn attribute(&mut self, attribute: &ExpandedMarkupAttribute) -> Result<(), EditorError> {
        let Some(original) = attribute.name.as_deref().filter(|name| !name.is_empty()) else {
            return Ok(());
        };
        if attribute.is_implied
            && attribute.value_type == MarkupValueType::Raw
            && attribute.value.as_ref().is_none_or(Vec::is_empty)
        {
            return Ok(());
        }
        let jsx = self.syntax == Syntax::Jsx;
        let name = match (jsx, attribute.is_multiple, original) {
            (true, true, "class") => "styleName",
            (true, _, "class") => "className",
            (true, _, "for") => "htmlFor",
            _ => original,
        };
        let mut expression =
            attribute.value_type == MarkupValueType::Expression || jsx && attribute.is_multiple;
        let mut value = attribute.value.clone();
        if jsx
            && attribute.is_multiple
            && original == "class"
            && let Some([MarkupValue::Text(text)]) = value.as_deref()
        {
            let mut characters = text.chars();
            let is_property = characters.next().is_some_and(|character| {
                character.is_ascii_alphabetic() || matches!(character, '_' | '$')
            }) && characters.all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '$')
            }) && !RESERVED_PROPERTIES.contains(&text.as_str());
            let text = if is_property {
                format!("styles.{text}")
            } else {
                format!("styles['{text}']")
            };
            value = Some(vec![MarkupValue::Text(text)]);
            expression = true;
        }
        if value.is_none()
            && (attribute.is_boolean
                || BOOLEAN_ATTRIBUTES.contains(&original.to_ascii_lowercase().as_str()))
        {
            value = Some(vec![MarkupValue::Text(name.into())]);
        }
        self.literal(" ")?;
        self.literal(name)?;
        self.literal("=")?;
        self.literal(if expression { "{" } else { "\"" })?;
        if let Some(value) = value.as_deref() {
            self.tokens(value)?;
        } else {
            self.caret()?;
        }
        self.literal(if expression { "}" } else { "\"" })?;
        Ok(())
    }

    fn snippet(&mut self, node: &ExpandedMarkupNode, depth: usize) -> Result<bool, EditorError> {
        if node.children.is_empty() {
            return Ok(false);
        }
        let Some(values) = node.value.as_deref() else {
            return Ok(false);
        };
        let Some(index) = values
            .iter()
            .position(|value| matches!(value, MarkupValue::Field { .. }))
        else {
            return Ok(false);
        };
        self.tokens(&values[..index])?;
        let line = self.line;
        self.nodes(&node.children, Some(node), depth + 1)?;
        let rest = &values[index + 1..];
        if line != self.line
            && let Some(MarkupValue::Text(text)) = rest.first()
        {
            self.literal(text.trim_start())?;
            self.tokens(&rest[1..])?;
        } else {
            self.tokens(rest)?;
        }
        Ok(true)
    }

    fn comment(&mut self, node: &ExpandedMarkupNode) -> Result<(), EditorError> {
        if !self.is_comment_enabled {
            return Ok(());
        }
        let Some(attributes) = node.attributes.as_ref() else {
            return Ok(());
        };
        if !attributes
            .iter()
            .any(|attribute| matches!(attribute.name.as_deref(), Some("id" | "class")))
        {
            return Ok(());
        }
        self.literal("\n<!-- /")?;
        for (name, prefix) in [("id", "#"), ("class", ".")] {
            if let Some(values) = attributes.iter().rev().find_map(|attribute| {
                attribute
                    .name
                    .as_deref()
                    .filter(|attribute| attribute.eq_ignore_ascii_case(name))
                    .and_then(|_| attribute.value.as_deref())
            }) {
                self.literal(prefix)?;
                self.tokens(values)?;
            }
        }
        self.literal(" -->")
    }

    fn nodes(
        &mut self,
        nodes: &[ExpandedMarkupNode],
        parent: Option<&ExpandedMarkupNode>,
        depth: usize,
    ) -> Result<(), EditorError> {
        if depth > self.max_nesting {
            return Err(EditorError::Capacity);
        }
        for (index, node) in nodes.iter().enumerate() {
            let format = should_format(index, nodes, parent);
            let offset = usize::from(parent.is_some_and(|parent| {
                !is_snippet(parent) && parent.name.as_deref() != Some("html")
            }));
            self.level += offset;
            if format {
                self.newline(self.level)?;
            }
            if let Some(name) = node.name.as_deref().filter(|name| !name.is_empty()) {
                self.literal("<")?;
                self.literal(name)?;
                if let Some(attributes) = &node.attributes {
                    for attribute in attributes {
                        self.attribute(attribute)?;
                    }
                }
                self.literal(">")?;
                if !(node.self_closing && node.children.is_empty() && node.value.is_none()) {
                    if !self.snippet(node, depth)? {
                        if let Some(values) = node.value.as_deref() {
                            let inner = has_newline(values) || starts_with_block(values);
                            if inner {
                                self.level += 1;
                                self.newline(self.level)?;
                            }
                            self.tokens(values)?;
                            if inner {
                                self.level -= 1;
                                self.newline(self.level)?;
                            }
                        }
                        self.nodes(&node.children, Some(node), depth + 1)?;
                        if node.value.is_none() && node.children.is_empty() {
                            if name == "body" {
                                self.level += 1;
                                self.newline(self.level)?;
                            }
                            self.caret()?;
                            if name == "body" {
                                self.level -= 1;
                                self.newline(self.level)?;
                            }
                        }
                    }
                    self.literal("</")?;
                    self.literal(name)?;
                    self.literal(">")?;
                    self.comment(node)?;
                }
            } else if !self.snippet(node, depth)?
                && let Some(values) = node.value.as_deref()
            {
                self.tokens(values)?;
                self.nodes(&node.children, Some(node), depth + 1)?;
            }
            if format
                && index + 1 == nodes.len()
                && let Some(parent) = parent
            {
                self.newline(self.level.saturating_sub(usize::from(!is_snippet(parent))))?;
            }
            self.level -= offset;
        }
        Ok(())
    }

    fn finish(mut self) -> String {
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

fn render(
    nodes: &[ExpandedMarkupNode],
    syntax: Syntax,
    max_bytes: usize,
    max_nesting: usize,
    is_comment_enabled: bool,
) -> Result<String, EditorError> {
    if syntax == Syntax::Css {
        return Err(EditorError::Refused);
    }
    let mut output = Output {
        text: String::new(),
        syntax,
        level: 0,
        line: 0,
        field: 1,
        fields: Vec::new(),
        max_bytes,
        max_nesting,
        is_comment_enabled,
    };
    output.nodes(nodes, None, 0)?;
    Ok(output.finish())
}

pub fn render_markup(
    nodes: &[ExpandedMarkupNode],
    syntax: Syntax,
    max_bytes: usize,
    max_nesting: usize,
) -> Result<String, EditorError> {
    render(nodes, syntax, max_bytes, max_nesting, false)
}

pub fn expand_markup(
    input: &str,
    syntax: Syntax,
    parse_limits: MarkupLimits,
    conversion_limits: ConversionLimits,
) -> Result<String, EditorError> {
    let nodes = prepare_markup(input, syntax, parse_limits, conversion_limits)?;
    render_markup(
        &nodes,
        syntax,
        conversion_limits.max_bytes,
        conversion_limits.max_nesting,
    )
}

pub fn expand_markup_with_filters(
    input: &str,
    syntax: Syntax,
    parse_limits: MarkupLimits,
    conversion_limits: ConversionLimits,
) -> Result<String, EditorError> {
    let (input, filters) = markup_filters(input);
    let nodes =
        prepare_markup_with_filters(input, syntax, parse_limits, conversion_limits, filters)?;
    render(
        &nodes,
        syntax,
        conversion_limits.max_bytes,
        conversion_limits.max_nesting,
        filters.is_comment_enabled,
    )
}

pub fn expand_markup_with_random(
    input: &str,
    syntax: Syntax,
    parse_limits: MarkupLimits,
    conversion_limits: ConversionLimits,
    random: &mut dyn FnMut() -> u32,
) -> Result<String, EditorError> {
    let (input, filters) = markup_filters(input);
    let mut nodes =
        prepare_markup_with_random(input, syntax, parse_limits, conversion_limits, random)?;
    apply_markup_filters(&mut nodes, conversion_limits, filters)?;
    render(
        &nodes,
        syntax,
        conversion_limits.max_bytes,
        conversion_limits.max_nesting,
        filters.is_comment_enabled,
    )
}
