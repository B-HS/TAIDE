use crate::document::EditorError;

use super::{
    ConversionLimits, ExpandedMarkupNode, MarkupLimits, MarkupValue, Syntax, prepare_markup,
};

const MAX_FILTERS: usize = 3;
const ELEMENT_SEPARATOR: &str = "__";
const MODIFIER_SEPARATOR: &str = "_";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MarkupFilters {
    pub is_bem_enabled: bool,
    pub is_comment_enabled: bool,
    pub has_trim_filter: bool,
}

pub fn markup_filters(input: &str) -> (&str, MarkupFilters) {
    let mut input = input;
    let mut filters = MarkupFilters::default();
    for _ in 0..MAX_FILTERS {
        if let Some(rest) = input.strip_suffix("|bem") {
            filters.is_bem_enabled = true;
            input = rest;
            continue;
        }
        if let Some(rest) = input.strip_suffix("|c") {
            filters.is_comment_enabled = true;
            input = rest;
            continue;
        }
        if let Some(rest) = input.strip_suffix("|t") {
            filters.has_trim_filter = true;
            input = rest;
            continue;
        }
        break;
    }
    (input, filters)
}

fn notation(value: &str, prefix: char, modifiers: bool) -> Option<(usize, &str, &str)> {
    let count = value
        .chars()
        .take_while(|character| *character == prefix)
        .count();
    if count == 0 {
        return None;
    }
    let start = &value[count..];
    if !start
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }
    let end = start
        .bytes()
        .take_while(|character| {
            character.is_ascii_alphanumeric()
                || *character == b'-'
                || modifiers && *character == b'_'
        })
        .count();
    Some((count, &start[..end], &start[end..]))
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        if !value.is_empty() && !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn find_block(values: &[String]) -> Option<String> {
    for prefers_hyphen in [true, false] {
        for value in values {
            if notation(value, '-', false).is_some() || notation(value, '_', true).is_some() {
                break;
            }
            let mut characters = value.chars();
            if !characters
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic())
            {
                continue;
            }
            if !prefers_hyphen || characters.next() == Some('-') {
                return Some(value.clone());
            }
        }
    }
    None
}

fn block_name(path: &[Option<String>], depth: usize) -> &str {
    let mut index = path
        .len()
        .saturating_sub(depth)
        .min(path.len().saturating_sub(1));
    loop {
        if let Some(name) = path.get(index).and_then(Option::as_deref) {
            return name;
        }
        if index == 0 {
            return "";
        }
        index -= 1;
    }
}

fn bem(
    nodes: &mut [ExpandedMarkupNode],
    path: &mut Vec<Option<String>>,
    depth: usize,
    limits: ConversionLimits,
    bytes: &mut usize,
) -> Result<(), EditorError> {
    if depth > limits.max_nesting {
        return Err(EditorError::Capacity);
    }
    for node in nodes {
        let class = node.attributes.as_mut().and_then(|attributes| {
            attributes
                .iter_mut()
                .find(|attribute| attribute.name.as_deref() == Some("class"))
        });
        let text = class
            .as_ref()
            .and_then(|attribute| attribute.value.as_ref())
            .map(|values| {
                values
                    .iter()
                    .map(|value| match value {
                        MarkupValue::Text(text) => text.as_str(),
                        MarkupValue::Field { name, .. } => name.as_str(),
                    })
                    .collect::<String>()
            })
            .unwrap_or_default();
        let mut values = Vec::new();
        for value in text.split(|character: char| {
            character.is_whitespace() && character != '\u{85}' || character == '\u{feff}'
        }) {
            if let Some(index) = value
                .find('_')
                .filter(|index| *index > 0 && !value.starts_with('-'))
            {
                values.push(value[..index].into());
                values.push(value[index..].into());
            } else {
                values.push(value.into());
            }
        }
        let values = unique(values);
        path.push(find_block(&values));
        let mut expanded = Vec::new();
        for original in values {
            let mut remaining = original.as_str();
            let mut prefix = String::new();
            if let Some((count, element, rest)) = notation(remaining, '-', false) {
                prefix = format!("{}{ELEMENT_SEPARATOR}{element}", block_name(path, count));
                expanded.push(prefix.clone());
                remaining = rest;
            }
            if let Some((count, modifier, rest)) = notation(remaining, '_', true) {
                if prefix.is_empty() {
                    prefix = block_name(path, count).into();
                    expanded.push(prefix.clone());
                }
                expanded.push(format!("{prefix}{MODIFIER_SEPARATOR}{modifier}"));
                remaining = rest;
            }
            if remaining == original {
                expanded.push(original);
            }
        }
        let expanded = unique(expanded);
        if !expanded.is_empty()
            && let Some(class) = class
        {
            let text = expanded.join(" ");
            *bytes = bytes.checked_add(text.len()).ok_or(EditorError::Capacity)?;
            if *bytes > limits.max_bytes {
                return Err(EditorError::Capacity);
            }
            class.value = Some(vec![MarkupValue::Text(text)]);
        }
        bem(&mut node.children, path, depth + 1, limits, bytes)?;
        path.pop();
    }
    Ok(())
}

pub fn prepare_markup_with_filters(
    input: &str,
    syntax: Syntax,
    parse_limits: MarkupLimits,
    conversion_limits: ConversionLimits,
    filters: MarkupFilters,
) -> Result<Vec<ExpandedMarkupNode>, EditorError> {
    let mut nodes = prepare_markup(input, syntax, parse_limits, conversion_limits)?;
    if filters.is_bem_enabled {
        bem(&mut nodes, &mut Vec::new(), 0, conversion_limits, &mut 0)?;
    }
    Ok(nodes)
}
