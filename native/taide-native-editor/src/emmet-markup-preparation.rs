use crate::document::EditorError;

use super::{
    ConversionLimits, ExpandedMarkupAttribute, ExpandedMarkupNode, MarkupLimits, MarkupValue,
    MarkupValueType, Syntax, convert_markup, parse_markup,
};

#[path = "emmet-markup-defaults.rs"]
mod defaults;

pub(crate) use defaults::{BOOLEAN_ATTRIBUTES, INLINE_ELEMENTS};

struct Resolver {
    syntax: Syntax,
    parse_limits: MarkupLimits,
    conversion_limits: ConversionLimits,
    nodes: usize,
    stack: Vec<&'static str>,
}

impl Resolver {
    fn resolve(
        &mut self,
        nodes: Vec<ExpandedMarkupNode>,
        depth: usize,
    ) -> Result<Vec<ExpandedMarkupNode>, EditorError> {
        if depth > self.conversion_limits.max_nesting {
            return Err(EditorError::Capacity);
        }
        let mut result = Vec::new();
        for mut node in nodes {
            if self.nodes >= self.conversion_limits.max_nodes {
                return Err(EditorError::Capacity);
            }
            self.nodes += 1;
            let snippet = node
                .name
                .as_deref()
                .and_then(|name| {
                    defaults::SNIPPETS
                        .iter()
                        .find(|(key, _)| *key == name)
                        .map(|(_, value)| *value)
                })
                .filter(|snippet| !self.stack.contains(snippet));
            let Some(snippet) = snippet else {
                node.children = self.resolve(node.children, depth + 1)?;
                result.push(node);
                continue;
            };
            let parsed = parse_markup(snippet, self.syntax, self.parse_limits)?;
            let converted = convert_markup(&parsed, defaults::VARIABLES, self.conversion_limits)?;
            self.stack.push(snippet);
            let resolved = self.resolve(converted, depth + 1);
            self.stack.pop();
            let mut resolved = resolved?;
            for top in &mut resolved {
                if let Some(attributes) = node.attributes.as_ref() {
                    top.attributes
                        .get_or_insert_with(Vec::new)
                        .extend(attributes.iter().cloned());
                }
                top.self_closing |= node.self_closing;
                if node.value.is_some() {
                    top.value = node.value.clone();
                }
                if node.repeat.is_some() {
                    top.repeat = node.repeat;
                }
            }
            let children = self.resolve(node.children, depth + 1)?;
            if let Some(last) = resolved.last_mut() {
                let mut deepest = last;
                while !deepest.children.is_empty() {
                    deepest = deepest.children.last_mut().ok_or(EditorError::Refused)?;
                }
                deepest.children.extend(children);
            }
            result.extend(resolved);
        }
        Ok(result)
    }
}

fn append(values: &mut Vec<MarkupValue>, value: MarkupValue) {
    if let (Some(MarkupValue::Text(previous)), MarkupValue::Text(next)) =
        (values.last_mut(), &value)
    {
        previous.push_str(next);
        return;
    }
    values.push(value);
}

fn merge_attributes(attributes: &mut Vec<ExpandedMarkupAttribute>) {
    let mut merged = Vec::<ExpandedMarkupAttribute>::new();
    for attribute in std::mem::take(attributes) {
        let previous = attribute
            .name
            .as_ref()
            .filter(|name| !name.is_empty())
            .and_then(|name| {
                merged
                    .iter_mut()
                    .find(|previous| previous.name.as_ref() == Some(name))
            });
        let Some(previous) = previous else {
            merged.push(attribute);
            continue;
        };
        if attribute.name.as_deref() == Some("class") {
            match (previous.value.as_mut(), attribute.value) {
                (Some(previous), Some(next)) => {
                    if !previous.is_empty() {
                        append(previous, MarkupValue::Text(" ".into()));
                    }
                    for value in next {
                        append(previous, value);
                    }
                }
                (None, next) => previous.value = next,
                _ => (),
            }
            continue;
        }
        previous.value = attribute.value;
        previous.is_implied |= attribute.is_implied;
        previous.is_boolean |= attribute.is_boolean;
        if previous.value_type != MarkupValueType::Expression {
            previous.value_type = attribute.value_type;
        }
    }
    *attributes = merged;
}

fn implicit_tag(parent: Option<&str>) -> &'static str {
    match parent {
        Some("p") => "span",
        Some("ul" | "ol") => "li",
        Some("table" | "tbody" | "thead" | "tfoot") => "tr",
        Some("tr") => "td",
        Some("colgroup") => "col",
        Some("select" | "optgroup") => "option",
        Some("audio" | "video") => "source",
        Some("object") => "param",
        Some("map") => "area",
        Some(parent) if INLINE_ELEMENTS.contains(&parent) => "span",
        _ => "div",
    }
}

fn is_empty_attribute(attribute: &ExpandedMarkupAttribute) -> bool {
    match attribute.value.as_deref() {
        None => true,
        Some([MarkupValue::Field { name, .. }]) => name.is_empty(),
        _ => false,
    }
}

fn find_label_input(nodes: &mut [ExpandedMarkupNode]) -> Option<&mut ExpandedMarkupNode> {
    for node in nodes {
        if matches!(node.name.as_deref(), Some("input" | "textarea")) {
            return Some(node);
        }
        if let Some(found) = find_label_input(&mut node.children) {
            return Some(found);
        }
    }
    None
}

fn prepare(
    nodes: &mut [ExpandedMarkupNode],
    parent: Option<&str>,
    depth: usize,
    limits: ConversionLimits,
    bytes: &mut usize,
) -> Result<(), EditorError> {
    if depth > limits.max_nesting {
        return Err(EditorError::Capacity);
    }
    for node in nodes {
        if node.name.as_ref().is_none_or(|name| name.is_empty()) && node.attributes.is_some() {
            node.name = Some(implicit_tag(parent).into());
        }
        if let Some(attributes) = &mut node.attributes {
            merge_attributes(attributes);
        }
        if node.name.as_deref() == Some("label")
            && let Some(input) = find_label_input(&mut node.children)
        {
            if let Some(attributes) = &mut node.attributes {
                attributes.retain(|attribute| {
                    attribute.name.as_deref() != Some("for") || !is_empty_attribute(attribute)
                });
            }
            if let Some(attributes) = &mut input.attributes {
                attributes.retain(|attribute| {
                    attribute.name.as_deref() != Some("id") || !is_empty_attribute(attribute)
                });
            }
        }
        let value_bytes = |values: Option<&[MarkupValue]>| {
            values.map_or(0, |values| {
                values
                    .iter()
                    .map(|value| match value {
                        MarkupValue::Text(text) => text.len(),
                        MarkupValue::Field { name, .. } => name.len(),
                    })
                    .sum::<usize>()
            })
        };
        let content = node.name.as_ref().map_or(0, String::len)
            + value_bytes(node.value.as_deref())
            + node.attributes.as_ref().map_or(0, |attributes| {
                attributes
                    .iter()
                    .map(|attribute| {
                        attribute.name.as_ref().map_or(0, String::len)
                            + value_bytes(attribute.value.as_deref())
                    })
                    .sum::<usize>()
            });
        *bytes = bytes.checked_add(content).ok_or(EditorError::Capacity)?;
        if *bytes > limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        let normalized = node.name.as_ref().map(|name| name.to_ascii_lowercase());
        let next_parent = normalized.as_deref().filter(|name| !name.is_empty());
        prepare(&mut node.children, next_parent, depth + 1, limits, bytes)?;
    }
    Ok(())
}

pub fn prepare_markup(
    input: &str,
    syntax: Syntax,
    parse_limits: MarkupLimits,
    conversion_limits: ConversionLimits,
) -> Result<Vec<ExpandedMarkupNode>, EditorError> {
    let parsed = parse_markup(input, syntax, parse_limits)?;
    let converted = convert_markup(&parsed, defaults::VARIABLES, conversion_limits)?;
    let mut resolved = Resolver {
        syntax,
        parse_limits,
        conversion_limits,
        nodes: 0,
        stack: Vec::new(),
    }
    .resolve(converted, 0)?;
    prepare(&mut resolved, None, 0, conversion_limits, &mut 0)?;
    Ok(resolved)
}
