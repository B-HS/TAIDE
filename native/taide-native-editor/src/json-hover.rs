use std::ops::Range;

use crate::document::EditorError;
use crate::json_completion::{Environment, field, text};
use crate::json_parser::{self, Document, Kind, Value as NodeValue};
use crate::json_schema_document;
use crate::json_schema_evaluation::{self, SchemaView, Selection};
use crate::json_value::{Arena, Value};
use crate::json_value_text;

const LINE_FEED: u16 = 0x0a;
const CARRIAGE_RETURN: u16 = 0x0d;
const MARKDOWN_ESCAPED: &str = "\\`*_{}[]()#+-.!";
const PARAGRAPH_SEPARATOR: &str = "\n\n";

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub document: json_schema_document::Limits,
    pub max_text_bytes: usize,
    pub max_work: usize,
    pub max_depth: usize,
}

/// One `doHover` result: the markdown text Monaco renders and the byte range of the hovered node.
#[derive(Clone, Debug, PartialEq)]
pub struct Hover {
    pub contents: String,
    pub range: Range<usize>,
}

fn is_line_break(unit: u16) -> bool {
    unit == LINE_FEED || unit == CARRIAGE_RETURN
}

/// `toMarkdown`: doubles single line breaks between two non-break code units, then escapes
/// markdown syntax characters.
fn to_markdown(plain: &[u16]) -> String {
    let mut paragraphs = Vec::with_capacity(plain.len());
    let mut index = 0;
    while index < plain.len() {
        let unit = plain[index];
        let break_length = match plain.get(index + 1..) {
            Some([CARRIAGE_RETURN, LINE_FEED, ..]) => 2,
            Some([LINE_FEED, ..]) => 1,
            _ => 0,
        };
        if !is_line_break(unit)
            && break_length > 0
            && let Some(&next) = plain.get(index + 1 + break_length)
            && !is_line_break(next)
        {
            paragraphs.extend([unit, LINE_FEED, LINE_FEED, next]);
            index += break_length + 2;
            continue;
        }
        paragraphs.push(unit);
        index += 1;
    }
    let mut result = String::with_capacity(paragraphs.len());
    for character in String::from_utf16_lossy(&paragraphs).chars() {
        if MARKDOWN_ESCAPED.contains(character) {
            result.push('\\');
        }
        result.push(character);
    }
    result
}

fn code_block(content: &str) -> String {
    if content.contains('`') {
        return format!("`` {content} ``");
    }
    content.to_owned()
}

fn utf16_length(text: &str) -> usize {
    text.encode_utf16().count()
}

fn strictly_inside_brackets(input: &str, document: &Document, node: usize, byte: usize) -> bool {
    let current = &document.nodes[node];
    if !matches!(current.kind, Kind::Object | Kind::Array) {
        return false;
    }
    let offset = utf16_length(&input[current.bytes.start..byte]);
    let length = utf16_length(&input[current.bytes.clone()]);
    offset > 1 && offset + 1 < length
}

fn equals_node(arena: &Arena, value: usize, node: &NodeValue) -> bool {
    match (arena.get(value), node) {
        (Some(Value::String(left)), NodeValue::String(right)) => left == right,
        (Some(Value::Number(left)), NodeValue::Number(right)) => left == right,
        (Some(Value::Boolean(left)), NodeValue::Boolean(right)) => left == right,
        (Some(Value::Null), NodeValue::Null) => true,
        _ => false,
    }
}

fn entry(arena: &Arena, schema: usize, name: &str, index: Option<usize>) -> Option<Vec<u16>> {
    let Value::Array(entries) = arena.get(field(arena, schema, name)?)? else {
        return None;
    };
    match arena.get(*entries.get(index?)?)? {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        _ => None,
    }
}

/// Runs the language service `doHover` for one document at `byte` without hover contributions
/// or a schema request service. `None` matches both a `null` result and a hover whose only
/// content is empty, which Monaco does not show.
pub fn hover(
    input: &str,
    byte: usize,
    environment: Environment<'_>,
    limits: Limits,
) -> Result<Option<Hover>, EditorError> {
    if byte > input.len() || !input.is_char_boundary(byte) {
        return Err(EditorError::InvalidBoundary);
    }
    let document = json_parser::parse(input, true, limits.document.policy.syntax.parser)?;
    let Some(mut node) = document.node_at_byte(byte, false) else {
        return Ok(None);
    };
    if strictly_inside_brackets(input, &document, node, byte) {
        return Ok(None);
    }
    let range = document.nodes[node].bytes.clone();
    if document.nodes[node].kind == Kind::String
        && let Some(parent) = document.nodes[node].parent
        && document.nodes[parent].kind == Kind::Property
        && document.nodes[parent].children.first() == Some(&node)
    {
        let Some(&value) = document.nodes[parent].children.get(1) else {
            return Ok(None);
        };
        node = value;
    }
    let resolved = json_schema_document::document_schema(
        &document,
        environment.resource,
        environment.contributions,
        environment.associations,
        limits.document,
        environment.cancelled,
    )?
    .resolved;
    let Some(resolved) = resolved else {
        return Ok(None);
    };
    let schema = &resolved.schema;
    let Some(root) = schema.root() else {
        return Ok(None);
    };
    let matching = json_schema_evaluation::matching_schemas(
        &document,
        schema,
        root,
        environment.patterns,
        Selection {
            focus_byte: Some(document.nodes[node].bytes.start),
            exclude_node: None,
        },
        limits.document.policy.evaluation,
    )?;
    let mut title = None::<Vec<u16>>;
    let mut description = None::<String>;
    let mut enum_description = None::<String>;
    let mut enum_value = None::<String>;
    for matched in matching {
        if matched.node != node || matched.inverted || matched.view != SchemaView::Source {
            continue;
        }
        let current = matched.schema;
        if title.is_none() {
            title = text(schema, current, "title").map(<[u16]>::to_vec);
        }
        if description.is_none() {
            description = text(schema, current, "markdownDescription")
                .map(String::from_utf16_lossy)
                .or_else(|| text(schema, current, "description").map(to_markdown));
        }
        let Some(Value::Array(values)) =
            field(schema, current, "enum").and_then(|index| schema.get(index))
        else {
            continue;
        };
        let index = values
            .iter()
            .position(|value| equals_node(schema, *value, &document.nodes[node].value));
        let is_truthy = |name| {
            field(schema, current, name)
                .and_then(|index| schema.get(index))
                .is_some_and(Value::is_truthy)
        };
        if is_truthy("markdownEnumDescriptions") {
            enum_description = entry(schema, current, "markdownEnumDescriptions", index)
                .map(|value| String::from_utf16_lossy(&value));
        } else if is_truthy("enumDescriptions") {
            enum_description =
                entry(schema, current, "enumDescriptions", index).map(|value| to_markdown(&value));
        }
        if enum_description.is_some()
            && let Some(index) = index
        {
            enum_value = Some(match schema.get(values[index]) {
                Some(Value::String(value)) => String::from_utf16_lossy(value),
                _ => String::from_utf16_lossy(&json_value_text::stringify(
                    schema,
                    values[index],
                    json_value_text::Limits {
                        max_units: limits.max_text_bytes,
                        max_work: limits.max_work,
                        max_depth: limits.max_depth,
                    },
                )?),
            });
        }
    }
    let mut contents = title.map_or_else(String::new, |title| to_markdown(&title));
    for part in [
        description.filter(|description| !description.is_empty()),
        enum_description
            .zip(enum_value)
            .map(|(description, value)| format!("`{}`: {description}", code_block(&value))),
    ]
    .into_iter()
    .flatten()
    {
        if !contents.is_empty() {
            contents.push_str(PARAGRAPH_SEPARATOR);
        }
        contents.push_str(&part);
        if contents.len() > limits.max_text_bytes {
            return Err(EditorError::Capacity);
        }
    }
    if (environment.cancelled)() {
        return Err(EditorError::StaleRevision);
    }
    Ok((!contents.is_empty()).then_some(Hover { contents, range }))
}
