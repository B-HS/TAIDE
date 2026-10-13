use std::ops::Range;

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Documentation, InsertTextFormat,
};

use crate::completion::Candidate;
use crate::document::{DocumentSnapshot, EditorError};
use crate::editing::line_content_range;
use crate::lsp::{LspRange, Position, TextEdit, byte_to_position, position_to_byte};
use crate::snippet_syntax::{Marker, ParseLimits, parse};

use super::{
    ConversionLimits, MarkupLimits, Syntax, expand_markup_with_random, expand_stylesheet, extract,
    markup_filters,
};

#[path = "emmet-completion-defaults.rs"]
mod defaults;

const CSS_PROPERTY_PREFIX_LENGTH: usize = 4;
const HEX_COLOR_LIMIT: usize = 6;
const SNIPPET_MARKER_LIMIT: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub item: CompletionItem,
    pub bytes: Range<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct CompletionLimits {
    pub parse: MarkupLimits,
    pub conversion: ConversionLimits,
    pub max_candidates: usize,
    pub max_bytes: usize,
}

fn is_operator(character: char) -> bool {
    matches!(character, '>' | '+' | '*' | '^')
}

fn parentheses_allowed(input: &str) -> bool {
    if !input.contains(['(', ')']) {
        return true;
    }
    for (index, character) in input.char_indices() {
        let rest = &input[index + character.len_utf8()..];
        if character == '{'
            && let Some(end) = rest.find('}')
            && !rest[..end].contains('{')
            && rest[..end].contains(['(', ')'])
            && rest[end + 1..].chars().next().is_none_or(is_operator)
        {
            return true;
        }
        if character == '(' {
            for (end, close) in rest.char_indices() {
                if matches!(close, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
                    break;
                }
                if close == ')' && rest[end + 1..].chars().next().is_some_and(is_operator) {
                    return true;
                }
            }
        }
        if is_operator(character) && rest.starts_with('(') {
            let line = rest
                .split(['\r', '\n', '\u{2028}', '\u{2029}'])
                .next()
                .unwrap_or_default();
            if line.contains(')') {
                return true;
            }
        }
        if character == '[' {
            let mut cursor = 0;
            while cursor < rest.len() {
                if rest[cursor..].starts_with("=\"") && cursor > 0 {
                    let value = &rest[cursor + "=\"".len()..];
                    let line = value
                        .split(['\r', '\n', '\u{2028}', '\u{2029}'])
                        .next()
                        .unwrap_or_default();
                    if line.contains("\"]") {
                        return true;
                    }
                }
                let next = rest[cursor..].chars().next().unwrap();
                if matches!(next, '[' | ']' | '(' | ')') {
                    break;
                }
                cursor += next.len_utf8();
            }
        }
    }
    false
}

fn valid(input: &str, syntax: Syntax) -> bool {
    let Some(first) = input.chars().next() else {
        return false;
    };
    if syntax == Syntax::Css {
        if let Some((prefix, _)) = input.split_once('#') {
            if prefix.is_empty() {
                return input.len() > 1
                    && input.len() <= HEX_COLOR_LIMIT + 1
                    && input[1..].bytes().all(|byte| byte.is_ascii_hexdigit());
            }
            if defaults::COMMON_TAGS.contains(&prefix) {
                return false;
            }
        }
        let first = input.strip_prefix('-').unwrap_or(input).chars().next();
        return first.is_some_and(|character| {
            character.is_ascii_alphabetic() || matches!(character, '!' | '@' | '#' | ',')
        });
    }
    if first == '!' {
        return input.chars().all(|character| character == '!');
    }
    if !parentheses_allowed(input) {
        return false;
    }
    let can_start = first.is_ascii_alphabetic()
        || matches!(first, '(' | '[' | '#' | '.' | ',')
        || first == '{' && syntax == Syntax::Html;
    can_start
        && input
            .chars()
            .any(|character| character.is_ascii_alphabetic() || matches!(character, '.' | ','))
}

fn whitespace(character: char) -> bool {
    character.is_whitespace() && character != '\u{85}' || character == '\u{feff}'
}

fn noise(input: &str, output: &str, syntax: Syntax) -> bool {
    if syntax == Syntax::Css {
        let start = input.len().saturating_sub(": ".len());
        let prefix = input
            .get(start..)
            .and_then(|suffix| suffix.find(':').map(|offset| &input[..start + offset]))
            .unwrap_or(input);
        return output == format!("{prefix}: ${{0}};")
            || output
                .chars()
                .filter(|character| !whitespace(*character))
                .collect::<String>()
                == format!(
                    "{};",
                    input
                        .chars()
                        .filter(|character| !whitespace(*character))
                        .collect::<String>()
                );
    }
    if defaults::COMMON_TAGS.contains(&input.to_ascii_lowercase().as_str())
        || defaults::MARKUP_KEYS.contains(&input)
    {
        return false;
    }
    if input.contains(['-', ':', ','])
        && !input.contains("--")
        && !input.contains("::")
        && !input.ends_with(':')
    {
        return false;
    }
    if input == "." {
        return false;
    }
    if let Some(prefix) = input.strip_suffix('.')
        && prefix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b',')
    {
        return prefix.is_empty() || !defaults::HTML_TAGS.contains(&prefix);
    }
    if syntax == Syntax::Jsx
        && input
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
        && input.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return false;
    }
    output.to_ascii_lowercase()
        == format!(
            "<{}>${{0}}</{}>",
            input.to_ascii_lowercase(),
            input.to_ascii_lowercase()
        )
}

fn snippet_text(markers: &[Marker], has_cursors: bool) -> String {
    markers
        .iter()
        .map(|marker| match marker {
            Marker::Text(text) => text.clone(),
            Marker::Placeholder { children, .. } => {
                if children.is_empty() && has_cursors {
                    "|".into()
                } else {
                    snippet_text(children, has_cursors)
                }
            }
            Marker::Variable { name, children, .. } => {
                if children.is_empty() {
                    name.clone()
                } else {
                    snippet_text(children, has_cursors)
                }
            }
        })
        .collect()
}

fn descriptions(output: &str, limits: CompletionLimits) -> Result<(String, String), EditorError> {
    let markers = parse(
        output,
        ParseLimits {
            max_bytes: limits.conversion.max_bytes,
            max_nesting: limits.conversion.max_nesting,
            max_markers: SNIPPET_MARKER_LIMIT,
        },
        |_, _| false,
    )?;
    Ok((snippet_text(&markers, false), snippet_text(&markers, true)))
}

fn expand(
    input: &str,
    syntax: Syntax,
    limits: CompletionLimits,
    random: &mut dyn FnMut() -> u32,
) -> Result<String, EditorError> {
    if syntax == Syntax::Css {
        let (input, _) = markup_filters(input);
        if "!important".starts_with(input) {
            return Ok("!important".into());
        }
        return expand_stylesheet(input, limits.parse, limits.conversion);
    }
    expand_markup_with_random(input, syntax, limits.parse, limits.conversion, random)
}

fn push(
    result: &mut Vec<Suggestion>,
    item: CompletionItem,
    bytes: &Range<usize>,
    limits: CompletionLimits,
    used: &mut usize,
) -> Result<(), EditorError> {
    if result.len() >= limits.max_candidates {
        return Err(EditorError::Capacity);
    }
    let size = item.label.len()
        + item.insert_text.as_ref().map_or(0, String::len)
        + item.detail.as_ref().map_or(0, String::len)
        + item.filter_text.as_ref().map_or(0, String::len)
        + item.sort_text.as_ref().map_or(0, String::len)
        + item.documentation.as_ref().map_or(0, |doc| match doc {
            Documentation::String(text) => text.len(),
            Documentation::MarkupContent(content) => content.value.len(),
        });
    *used = used.checked_add(size).ok_or(EditorError::Capacity)?;
    if *used > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    result.push(Suggestion {
        item,
        bytes: bytes.clone(),
    });
    Ok(())
}

pub fn complete_line(
    line: &str,
    cursor: usize,
    syntax: Syntax,
    limits: CompletionLimits,
    random: &mut dyn FnMut() -> u32,
) -> Result<Vec<Suggestion>, EditorError> {
    if cursor > line.len() || !line.is_char_boundary(cursor) {
        return Err(EditorError::InvalidBoundary);
    }
    let prefix = &line[..cursor];
    let (stem, _) = markup_filters(prefix);
    let filter_bytes = prefix.len() - stem.len();
    let Some(extraction) = extract(line, stem.len(), syntax, limits.parse.max_bytes) else {
        return Ok(Vec::new());
    };
    let input = extraction.abbreviation;
    let bytes = extraction.bytes.start
        ..extraction
            .bytes
            .end
            .checked_add(filter_bytes)
            .ok_or(EditorError::Capacity)?;
    if bytes.end > line.len()
        || !line.is_char_boundary(bytes.end)
        || bytes.start > cursor
        || bytes.end < cursor
    {
        return Err(EditorError::InvalidBoundary);
    }
    let word_start = prefix
        .char_indices()
        .rev()
        .find(|(_, character)| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | ',' | ':' | '-' | '.'))
        })
        .map_or(0, |(index, character)| index + character.len_utf8());
    if syntax != Syntax::Css
        && &prefix[word_start..] == input
        && prefix.ends_with(&format!("<{input}"))
    {
        return Ok(Vec::new());
    }
    let expansion_input = format!("{input}{}", &prefix[stem.len()..]);
    let mut result = Vec::new();
    let mut used = 0usize;
    if valid(input, syntax)
        && let Ok(output) = expand(&expansion_input, syntax, limits, random)
        && !output.is_empty()
        && !noise(input, &output, syntax)
    {
        let (label, doc) = descriptions(&output, limits)?;
        let can_show = if syntax == Syntax::Css {
            if input.len() > CSS_PROPERTY_PREFIX_LENGTH
                && defaults::CSS_PROPERTIES
                    .iter()
                    .any(|property| property.starts_with(input))
            {
                return Ok(Vec::new());
            }
            input.bytes().any(|byte| byte.is_ascii_digit())
                || input.contains("${")
                || subsequence(input, &label)
        } else {
            true
        };
        if can_show {
            let item = CompletionItem {
                label: if syntax == Syntax::Css {
                    label
                } else {
                    expansion_input.clone()
                },
                detail: Some("Emmet abbreviation".into()),
                documentation: Some(Documentation::String(doc)),
                kind: Some(CompletionItemKind::PROPERTY),
                insert_text: Some(output),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                filter_text: (syntax == Syntax::Css).then(|| input.into()),
                ..Default::default()
            };
            push(&mut result, item, &bytes, limits, &mut used)?;
        }
    }
    if syntax == Syntax::Css {
        return Ok(result);
    }
    let mut extra_prefix = input;
    if let Some(index) = input.rfind(['>', '+']) {
        let suffix = &input[index + 1..];
        if !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'-'))
        {
            extra_prefix = suffix;
        }
    }
    let mut snippets = Vec::new();
    for (keys, is_snippet) in [
        (defaults::COMMON_TAGS, false),
        (defaults::MARKUP_KEYS, true),
    ] {
        for key in keys {
            let lowercase = extra_prefix.to_ascii_lowercase();
            if extra_prefix.is_empty()
                || !key.starts_with(&lowercase)
                || *key == lowercase
                || is_snippet && defaults::COMMON_TAGS.contains(key)
            {
                continue;
            }
            let current = format!(
                "{}{}{}",
                input,
                &key[extra_prefix.len()..],
                &prefix[stem.len()..]
            );
            let Ok(output) = expand(&current, syntax, limits, random) else {
                continue;
            };
            if output.is_empty() {
                continue;
            }
            let (_, doc) = descriptions(&output, limits)?;
            let item = CompletionItem {
                label: format!("{}{}", extra_prefix, &key[extra_prefix.len()..]),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some("Emmet Abbreviation".into()),
                documentation: Some(Documentation::String(doc)),
                insert_text: Some(output),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            };
            if is_snippet {
                push(&mut snippets, item, &bytes, limits, &mut used)?;
            } else {
                push(&mut result, item, &bytes, limits, &mut used)?;
            }
        }
    }
    if !snippets.is_empty()
        && extra_prefix != input
        && result.first().is_some_and(|suggestion| {
            suggestion.item.detail.as_deref() == Some("Emmet abbreviation")
        })
    {
        result[0].item.sort_text = Some(format!("0{}", result[0].item.label));
        for suggestion in &mut snippets {
            suggestion.item.filter_text = Some(input.into());
            suggestion.item.sort_text = Some(format!("9{input}"));
        }
    }
    if result
        .len()
        .checked_add(snippets.len())
        .ok_or(EditorError::Capacity)?
        > limits.max_candidates
    {
        return Err(EditorError::Capacity);
    }
    result.extend(snippets);
    if syntax == Syntax::Html
        && result.len() >= 2
        && input.contains(':')
        && result.first().is_some_and(|suggestion| {
            suggestion.item.insert_text.as_deref() == Some(&format!("<{input}>${{0}}</{input}>"))
        })
    {
        result.retain(|suggestion| suggestion.item.label != input);
    }
    Ok(result)
}

fn subsequence(input: &str, output: &str) -> bool {
    let output = output.to_ascii_lowercase();
    let mut chars = output.chars();
    input
        .to_ascii_lowercase()
        .chars()
        .all(|character| chars.by_ref().any(|current| current == character))
}

pub fn complete_document(
    document: &DocumentSnapshot,
    position: Position,
    limits: CompletionLimits,
    random: &mut dyn FnMut() -> u32,
) -> Result<Vec<Candidate>, EditorError> {
    let Some(syntax) = Syntax::for_language(&document.metadata.language_id) else {
        return Ok(Vec::new());
    };
    let byte = position_to_byte(document, position)?;
    if byte_to_position(document, byte)? != position {
        return Err(EditorError::InvalidBoundary);
    }
    let line = line_content_range(document, position.line as usize);
    if line.len() > limits.parse.max_bytes {
        return Ok(Vec::new());
    }
    let text = document.rope.byte_slice(line.clone()).to_string();
    complete_line(&text, byte - line.start, syntax, limits, random)?
        .into_iter()
        .map(|suggestion| {
            let range = LspRange {
                start: byte_to_position(document, line.start + suggestion.bytes.start)?,
                end: byte_to_position(document, line.start + suggestion.bytes.end)?,
            };
            let mut item = suggestion.item;
            item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: item.insert_text.clone().ok_or(EditorError::Refused)?,
            }));
            Candidate::new(document, position, range, item).ok_or(EditorError::InvalidBoundary)
        })
        .collect()
}
