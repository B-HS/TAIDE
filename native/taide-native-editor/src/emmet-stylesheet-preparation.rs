use std::sync::OnceLock;

use crate::document::EditorError;

use super::{
    ConversionLimits, MarkupLimits, StylesheetProperty, StylesheetToken, StylesheetValue,
    parse_stylesheet,
};

#[path = "emmet-stylesheet-defaults.rs"]
mod defaults;

const DEFAULT_LIMITS: MarkupLimits = MarkupLimits {
    max_bytes: 64 * 1024,
    max_nodes: 2048,
    max_nesting: 32,
};
const RGB_SHORT_DIVISOR: u8 = 17;
const ALPHA_DECIMALS: usize = 8;
static SNIPPETS: OnceLock<Result<Vec<Snippet>, EditorError>> = OnceLock::new();

struct Snippet {
    key: &'static str,
    property: Option<&'static str>,
    raw: &'static str,
    values: Vec<Vec<Vec<StylesheetToken>>>,
    keywords: Vec<(String, StylesheetToken)>,
    dependencies: Vec<usize>,
}

fn is_space(character: char) -> bool {
    character.is_whitespace() && character != '\u{85}' || character == '\u{feff}'
}

fn synthetic(value: StylesheetValue) -> StylesheetToken {
    StylesheetToken {
        value,
        source: None,
    }
}

fn field(index: usize, name: String) -> StylesheetToken {
    synthetic(StylesheetValue::Field {
        index: Some(index),
        name,
        is_adjacent: false,
    })
}

fn property_definition(value: &'static str) -> Option<(&'static str, &'static str)> {
    let end = value
        .find(|character: char| !(character.is_ascii_lowercase() || character == '-'))
        .unwrap_or(value.len());
    if end == 0 {
        return None;
    }
    let (name, rest) = value.split_at(end);
    if rest.is_empty() {
        return Some((name, ""));
    }
    let value = rest
        .trim_start_matches(is_space)
        .strip_prefix(':')?
        .trim_start_matches(is_space)
        .trim_end_matches(';');
    if value.is_empty() || value.contains(['\n', '\r', ';']) {
        return None;
    }
    Some((name, value))
}

fn registry() -> Result<Vec<Snippet>, EditorError> {
    let mut snippets = Vec::<Snippet>::new();
    for &(key, raw) in defaults::SNIPPETS {
        let definition = property_definition(raw);
        let mut values = Vec::new();
        let mut keywords = Vec::<(String, StylesheetToken)>::new();
        if let Some((_, value)) = definition.filter(|(_, value)| !value.is_empty()) {
            for alternative in value.split('|') {
                let parsed =
                    parse_stylesheet(alternative.trim_matches(is_space), true, DEFAULT_LIMITS)?;
                let value = parsed
                    .into_iter()
                    .next()
                    .ok_or(EditorError::Refused)?
                    .values;
                for fragment in &value {
                    for token in fragment {
                        let keyword = match &token.value {
                            StylesheetValue::Literal(value) => Some((value.clone(), token.clone())),
                            StylesheetValue::Function { name, .. } => {
                                Some((name.clone(), token.clone()))
                            }
                            StylesheetValue::Field { name, .. }
                                if !name.trim_matches(is_space).is_empty() =>
                            {
                                let name = name.trim_matches(is_space).to_owned();
                                Some((name.clone(), synthetic(StylesheetValue::Literal(name))))
                            }
                            _ => None,
                        };
                        if let Some((name, token)) = keyword {
                            if let Some((_, previous)) =
                                keywords.iter_mut().find(|(key, _)| *key == name)
                            {
                                *previous = token;
                            } else {
                                keywords.push((name, token));
                            }
                        }
                    }
                }
                values.push(value);
            }
        }
        snippets.push(Snippet {
            key,
            property: definition.map(|(name, _)| name),
            raw,
            values,
            keywords,
            dependencies: Vec::new(),
        });
    }
    let mut stack = Vec::<usize>::new();
    for index in 0..snippets.len() {
        let Some(property) = snippets[index].property else {
            continue;
        };
        while let Some(&previous) = stack.last() {
            let parent = snippets[previous].property.ok_or(EditorError::Refused)?;
            if property
                .strip_prefix(parent)
                .is_some_and(|suffix| suffix.starts_with('-'))
            {
                snippets[previous].dependencies.push(index);
                stack.push(index);
                break;
            }
            stack.pop();
        }
        if stack.is_empty() {
            stack.push(index);
        }
    }
    Ok(snippets)
}

fn score(abbreviation: &str, candidate: &str, is_partial: bool) -> f64 {
    let abbreviation = abbreviation.to_ascii_lowercase();
    let candidate = candidate.to_ascii_lowercase();
    if abbreviation == candidate {
        return 1.0;
    }
    let left = abbreviation.as_bytes();
    let right = candidate.as_bytes();
    if left.first().is_none()
        || left.first() != right.first()
        || !is_partial && left.len() > right.len()
    {
        return 0.0;
    }
    let minimum = left.len().min(right.len());
    let maximum = left.len().max(right.len());
    let mut i = 1;
    let mut j = 1;
    let mut points = maximum;
    while i < left.len() {
        let mut found = false;
        let mut is_acronym = false;
        while j < right.len() {
            if left[i] == right[j] {
                found = true;
                points += maximum - if is_acronym { i } else { j };
                break;
            }
            is_acronym = right[j] == b'-';
            j += 1;
        }
        if !found {
            if !is_partial {
                return 0.0;
            }
            break;
        }
        i += 1;
    }
    let sum = |value: usize| (value as f64) * ((value as f64) + 1.0) / 2.0;
    (points as f64) * ((i as f64) / (maximum as f64)) / (sum(maximum) - sum(maximum - minimum))
}

fn best_match<'a>(
    abbreviation: &str,
    candidates: impl Iterator<Item = (usize, &'a str)>,
    is_partial: bool,
    min_score: f64,
) -> Option<usize> {
    let mut found = None;
    let mut largest = 0.0;
    for (index, candidate) in candidates {
        let value = score(abbreviation, candidate, is_partial);
        if value == 1.0 {
            return Some(index);
        }
        if value > 0.0 && value >= largest {
            found = Some(index);
            largest = value;
        }
    }
    found.filter(|_| largest >= min_score)
}

fn keyword(value: &str, snippet: &Snippet, snippets: &[Snippet]) -> Option<StylesheetToken> {
    for current in
        std::iter::once(snippet).chain(snippet.dependencies.iter().map(|index| &snippets[*index]))
    {
        if let Some(index) = best_match(
            value,
            current
                .keywords
                .iter()
                .enumerate()
                .map(|(index, (key, _))| (index, key.as_str())),
            false,
            0.0,
        ) {
            return Some(current.keywords[index].1.clone());
        }
    }
    best_match(
        value,
        defaults::KEYWORDS.iter().copied().enumerate(),
        false,
        0.0,
    )
    .map(|index| synthetic(StylesheetValue::Literal(defaults::KEYWORDS[index].into())))
}

pub(crate) fn color(red: u8, green: u8, blue: u8, alpha: f64) -> String {
    if red == 0 && green == 0 && blue == 0 && alpha == 0.0 {
        return "transparent".into();
    }
    if alpha == 1.0 {
        if [red, green, blue]
            .iter()
            .all(|component| component % RGB_SHORT_DIVISOR == 0)
        {
            return format!(
                "#{:x}{:x}{:x}",
                red / RGB_SHORT_DIVISOR,
                green / RGB_SHORT_DIVISOR,
                blue / RGB_SHORT_DIVISOR
            );
        }
        return format!("#{red:02x}{green:02x}{blue:02x}");
    }
    let alpha = format!("{alpha:.ALPHA_DECIMALS$}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned();
    format!("rgba({red}, {green}, {blue}, {alpha})")
}

fn has_field(values: &[Vec<StylesheetToken>]) -> bool {
    values.iter().flatten().any(|token| match &token.value {
        StylesheetValue::Field { .. } => true,
        StylesheetValue::Function { arguments, .. } => has_field(arguments),
        _ => false,
    })
}

fn wrap(
    fragment: &[StylesheetToken],
    index: &mut usize,
) -> Result<Vec<StylesheetToken>, EditorError> {
    let mut result = Vec::new();
    for token in fragment {
        let name = match &token.value {
            StylesheetValue::Literal(value) => Some(value.clone()),
            StylesheetValue::Number { value, unit, .. } => Some(format!("{value}{unit}")),
            StylesheetValue::Color {
                red,
                green,
                blue,
                alpha,
                ..
            } => Some(color(*red, *green, *blue, *alpha)),
            StylesheetValue::String { value, quote } => Some(format!("{quote}{value}{quote}")),
            StylesheetValue::Function { name, .. } => Some(name.clone()),
            _ => None,
        };
        if let Some(name) = name {
            result.push(field(*index, name));
            *index = index.checked_add(1).ok_or(EditorError::Capacity)?;
            if let StylesheetValue::Function { arguments, .. } = &token.value {
                result.push(synthetic(StylesheetValue::Literal("(".into())));
                for (argument_index, argument) in arguments.iter().enumerate() {
                    result.extend(wrap(argument, index)?);
                    if argument_index + 1 < arguments.len() {
                        result.push(synthetic(StylesheetValue::Literal(", ".into())));
                    }
                }
                result.push(synthetic(StylesheetValue::Literal(")".into())));
            }
        } else {
            result.push(token.clone());
        }
    }
    Ok(result)
}

fn raw_snippet(node: &mut StylesheetProperty, snippet: &Snippet) -> Result<(), EditorError> {
    let mut donated = node.values.first().into_iter().flatten().cloned();
    let mut output = Vec::new();
    let mut offset = 0;
    let mut search = 0;
    while let Some(position) = snippet.raw[search..].find("${") {
        let start = search + position;
        let Some(end) = snippet.raw[start..].find('}').map(|end| start + end) else {
            break;
        };
        search = end + 1;
        let body = &snippet.raw[start + "${".len()..end];
        let (digits, name) = body
            .split_once(':')
            .map_or((body, None), |(digits, name)| (digits, Some(name)));
        if digits.is_empty()
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
            || name.is_some_and(str::is_empty)
        {
            continue;
        }
        if start > offset {
            output.push(synthetic(StylesheetValue::Literal(
                snippet.raw[offset..start].into(),
            )));
        }
        let token = if let Some(token) = donated.next() {
            token
        } else {
            field(
                digits.parse().map_err(|_| EditorError::Capacity)?,
                name.unwrap_or_default().into(),
            )
        };
        output.push(token);
        offset = end + 1;
    }
    if offset < snippet.raw.len() {
        output.push(synthetic(StylesheetValue::Literal(
            snippet.raw[offset..].into(),
        )));
    }
    node.name = None;
    node.values = vec![output];
    Ok(())
}

fn resolve(node: &mut StylesheetProperty, snippets: &[Snippet]) -> Result<(), EditorError> {
    let gradient = match node.values.as_slice() {
        [fragment] => match fragment.as_slice() {
            [
                StylesheetToken {
                    value: StylesheetValue::Function { name, arguments },
                    ..
                },
            ] if name == "lg" => Some(arguments.clone()),
            _ => None,
        },
        _ => None,
    };
    if gradient.is_some() || node.name.as_deref() == Some("lg") {
        node.name = Some("background-image".into());
        node.values = vec![vec![synthetic(StylesheetValue::Function {
            name: "linear-gradient".into(),
            arguments: gradient.unwrap_or_else(|| vec![vec![field(0, String::new())]]),
        })]];
    } else if let Some(abbreviation) = node.name.as_deref()
        && let Some(index) = best_match(
            abbreviation,
            snippets
                .iter()
                .enumerate()
                .map(|(index, snippet)| (index, snippet.key)),
            true,
            defaults::FUZZY_MIN_SCORE,
        )
    {
        let snippet = &snippets[index];
        if let Some(property) = snippet.property {
            let mut position = 0;
            let mut unmatched = "";
            for (index, character) in abbreviation.char_indices() {
                if let Some(found) = snippet.key[position..].find(character) {
                    position += found + character.len_utf8();
                } else {
                    unmatched = &abbreviation[index..];
                    break;
                }
            }
            if !unmatched.is_empty() {
                if !node.values.is_empty() {
                    resolve_numeric(node);
                    return Ok(());
                }
                let Some(value) = keyword(unmatched, snippet, snippets) else {
                    resolve_numeric(node);
                    return Ok(());
                };
                node.values.push(vec![value]);
            }
            node.name = Some(property.into());
            if node.values.is_empty() {
                if let Some(default) = snippet.values.first() {
                    node.values = if snippet.values.len() == 1 || has_field(default) {
                        default.clone()
                    } else {
                        default
                            .iter()
                            .map(|fragment| wrap(fragment, &mut 1))
                            .collect::<Result<_, _>>()?
                    };
                }
            } else {
                for token in node.values.iter_mut().flatten() {
                    match &mut token.value {
                        StylesheetValue::Literal(value) => {
                            if let Some(replacement) = keyword(value, snippet, snippets) {
                                *token = replacement;
                            }
                        }
                        StylesheetValue::Function { name, arguments } => {
                            if let Some(StylesheetToken {
                                value:
                                    StylesheetValue::Function {
                                        name: resolved,
                                        arguments: default,
                                    },
                                source,
                            }) = keyword(name, snippet, snippets)
                            {
                                arguments.extend(default.into_iter().skip(arguments.len()));
                                *name = resolved;
                                token.source = source;
                            }
                        }
                        _ => (),
                    }
                }
            }
        } else {
            raw_snippet(node, snippet)?;
        }
    }
    resolve_numeric(node);
    Ok(())
}

fn resolve_numeric(node: &mut StylesheetProperty) {
    if let Some(name) = &node.name {
        for token in node.values.iter_mut().flatten() {
            if let StylesheetValue::Number { value, raw, unit } = &mut token.value {
                if !unit.is_empty() {
                    if let Some((_, alias)) =
                        defaults::UNIT_ALIASES.iter().find(|(key, _)| *key == unit)
                    {
                        *unit = (*alias).into();
                    }
                } else if *value != 0.0 && !defaults::UNITLESS.contains(&name.as_str()) {
                    *unit = if raw.contains('.') {
                        defaults::FLOAT_UNIT.into()
                    } else {
                        defaults::INTEGER_UNIT.into()
                    };
                }
            }
        }
    }
}

fn inspect(
    values: &mut [Vec<StylesheetToken>],
    depth: usize,
    limits: ConversionLimits,
    nodes: &mut usize,
    bytes: &mut usize,
) -> Result<(), EditorError> {
    if depth > limits.max_nesting {
        return Err(EditorError::Capacity);
    }
    for fragment in values {
        let mut previous_end = None;
        for (index, token) in fragment.iter_mut().enumerate() {
            *nodes = nodes.checked_add(1).ok_or(EditorError::Capacity)?;
            if *nodes > limits.max_nodes {
                return Err(EditorError::Capacity);
            }
            let cost = match &mut token.value {
                StylesheetValue::Literal(value)
                | StylesheetValue::CustomProperty(value)
                | StylesheetValue::String { value, .. } => value.len(),
                StylesheetValue::Number { raw, unit, .. } => raw
                    .len()
                    .checked_add(unit.len())
                    .ok_or(EditorError::Capacity)?,
                StylesheetValue::Color { raw, .. } => raw.len(),
                StylesheetValue::Field {
                    name, is_adjacent, ..
                } => {
                    *is_adjacent = index > 0
                        && token.source.as_ref().map(|source| source.start) == previous_end;
                    name.len()
                }
                StylesheetValue::Function { name, arguments } => {
                    inspect(arguments, depth + 1, limits, nodes, bytes)?;
                    name.len()
                }
            };
            previous_end = token.source.as_ref().map(|source| source.end);
            *bytes = bytes.checked_add(cost).ok_or(EditorError::Capacity)?;
            if *bytes > limits.max_bytes {
                return Err(EditorError::Capacity);
            }
        }
    }
    Ok(())
}

pub fn prepare_stylesheet(
    input: &str,
    parse_limits: MarkupLimits,
    limits: ConversionLimits,
) -> Result<Vec<StylesheetProperty>, EditorError> {
    let mut properties = parse_stylesheet(input, false, parse_limits)?;
    let snippets = SNIPPETS
        .get_or_init(registry)
        .as_ref()
        .map_err(Clone::clone)?;
    let mut nodes = 0usize;
    let mut bytes = 0usize;
    for node in &mut properties {
        resolve(node, snippets)?;
        nodes = nodes.checked_add(1).ok_or(EditorError::Capacity)?;
        bytes = bytes
            .checked_add(node.name.as_ref().map_or(0, String::len))
            .ok_or(EditorError::Capacity)?;
        if nodes > limits.max_nodes || bytes > limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        inspect(&mut node.values, 0, limits, &mut nodes, &mut bytes)?;
    }
    Ok(properties)
}
