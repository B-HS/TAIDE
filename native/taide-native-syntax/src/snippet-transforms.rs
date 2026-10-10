use std::collections::BTreeMap;
use std::sync::LazyLock;

use regress::{Flags, Match, Regex};
use taide_native_editor::document::EditorError;
use taide_native_editor::snippet_syntax::{FormatPart, ParseLimits, RegexMetadata, Transform};

const REGEX_OPTIONS: [char; 8] = ['d', 'g', 'i', 'm', 's', 'u', 'v', 'y'];
const HIGH_SURROGATE: std::ops::RangeInclusive<u16> = 0xd800..=0xdbff;
const LOW_SURROGATE: std::ops::RangeInclusive<u16> = 0xdc00..=0xdfff;
const SURROGATE_WIDTH: usize = 2;

static WORDS: LazyLock<Regex> = LazyLock::new(|| Regex::with_flags(r"[\p{L}0-9]+", "u").unwrap());
static KEBAB_WORDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::with_flags(
        r"\p{Lu}{2,}(?=\p{Lu}\p{Ll}+[0-9]*|[\s_-]|$)|\p{Lu}?\p{Ll}+[0-9]*|\p{Lu}(?=\p{Lu}\p{Ll})|\p{Lu}(?=[\s_-]|$)|[0-9]+",
        "u",
    )
    .unwrap()
});
static SNAKE_BOUNDARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::with_flags(r"(\p{Ll})(\p{Lu})", "u").unwrap());
static SNAKE_SEPARATORS: LazyLock<Regex> =
    LazyLock::new(|| Regex::with_flags(r"[\s\-]+", "u").unwrap());
static KEBAB_SEPARATORS: LazyLock<Regex> =
    LazyLock::new(|| Regex::with_flags(r"[\s_-]+", "u").unwrap());

pub struct MonacoSnippetTransforms {
    limits: ParseLimits,
    compiled_bytes: usize,
    expressions: BTreeMap<(String, String), Expression>,
}

struct Expression {
    regex: Regex,
    ignore_case: bool,
    global: bool,
    sticky: bool,
    unicode: bool,
}

impl MonacoSnippetTransforms {
    pub fn new(limits: ParseLimits) -> Self {
        Self {
            limits,
            compiled_bytes: 0,
            expressions: BTreeMap::new(),
        }
    }

    pub fn compile(&mut self, source: &str, options: &str) -> Option<RegexMetadata> {
        let expression = self.expression(source, options).ok()?;
        Some(RegexMetadata {
            source: regex_source(source),
            ignore_case: expression.ignore_case,
            global: expression.global,
        })
    }

    pub fn evaluate(&mut self, transform: &Transform, value: &str) -> Result<String, EditorError> {
        let max_bytes = self.limits.max_bytes;
        if value.len() > max_bytes || transform.format.len() > self.limits.max_markers {
            return Err(EditorError::Capacity);
        }
        let format_bytes = transform.format.iter().try_fold(0usize, |bytes, part| {
            let values = match part {
                FormatPart::Text(value) => [Some(value), None, None],
                FormatPart::Capture {
                    shorthand,
                    if_value,
                    else_value,
                    ..
                } => [shorthand.as_ref(), if_value.as_ref(), else_value.as_ref()],
            };
            values
                .into_iter()
                .flatten()
                .try_fold(bytes, |bytes, value| {
                    bytes.checked_add(value.len()).ok_or(EditorError::Capacity)
                })
        })?;
        if format_bytes > max_bytes {
            return Err(EditorError::Capacity);
        }
        let expression = self.expression(&transform.pattern, &transform.options)?;
        let input = value.encode_utf16().collect::<Vec<_>>();
        let mut output = Vec::new();
        let mut start = 0;
        let mut last_end = 0;
        let mut matched = false;
        while start <= input.len() {
            let found = if expression.unicode {
                expression.regex.find_from_utf16(&input, start).next()
            } else {
                expression.regex.find_from_ucs2(&input, start).next()
            };
            let Some(found) = found else {
                break;
            };
            if expression.sticky && found.range.start != start {
                break;
            }
            matched = true;
            append(&mut output, &input[last_end..found.range.start], max_bytes)?;
            replace(
                &mut output,
                &transform.format,
                &input,
                Some(&found),
                max_bytes,
            )?;
            last_end = found.range.end;
            if !expression.global {
                break;
            }
            start = found.range.end;
            if found.range.is_empty() {
                start += if expression.unicode
                    && input
                        .get(start)
                        .is_some_and(|unit| HIGH_SURROGATE.contains(unit))
                    && input
                        .get(start + 1)
                        .is_some_and(|unit| LOW_SURROGATE.contains(unit))
                {
                    SURROGATE_WIDTH
                } else {
                    1
                };
            }
        }
        if !matched
            && transform.format.iter().any(|part| {
                matches!(part, FormatPart::Capture { else_value: Some(value), .. } if !value.is_empty())
            })
        {
            replace(&mut output, &transform.format, &input, None, max_bytes)?;
        } else {
            append(&mut output, &input[last_end..], max_bytes)?;
        }
        let output = String::from_utf16_lossy(&output);
        if output.len() > max_bytes {
            return Err(EditorError::Capacity);
        }
        Ok(output)
    }

    fn expression(&mut self, source: &str, options: &str) -> Result<&Expression, EditorError> {
        if source.len() > self.limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        let canonical_options = validated_options(options)?;
        let key = (source.to_owned(), canonical_options);
        if !self.expressions.contains_key(&key) {
            let bytes = key
                .0
                .len()
                .checked_add(key.1.len())
                .ok_or(EditorError::Capacity)?;
            let compiled_bytes = self
                .compiled_bytes
                .checked_add(bytes)
                .ok_or(EditorError::Capacity)?;
            if compiled_bytes > self.limits.max_bytes
                || self.expressions.len() >= self.limits.max_markers
            {
                return Err(EditorError::Capacity);
            }
            let mut flags = Flags::from(key.1.as_str());
            flags.unicode |= flags.unicode_sets;
            let regex = if flags.unicode {
                Regex::with_flags(&key.0, flags)
            } else {
                Regex::from_unicode(key.0.encode_utf16().map(u32::from), flags)
            }
            .map_err(|_| EditorError::InvalidBoundary)?;
            self.expressions.insert(
                key.clone(),
                Expression {
                    regex,
                    ignore_case: flags.icase,
                    global: key.1.contains('g'),
                    sticky: key.1.contains('y'),
                    unicode: flags.unicode,
                },
            );
            self.compiled_bytes = compiled_bytes;
        }
        self.expressions
            .get(&key)
            .ok_or(EditorError::InvalidBoundary)
    }
}

fn validated_options(options: &str) -> Result<String, EditorError> {
    let mut seen = [false; REGEX_OPTIONS.len()];
    for option in options.chars() {
        let position = REGEX_OPTIONS
            .iter()
            .position(|allowed| *allowed == option)
            .ok_or(EditorError::InvalidBoundary)?;
        if seen[position] {
            return Err(EditorError::InvalidBoundary);
        }
        seen[position] = true;
    }
    if options.contains('u') && options.contains('v') {
        return Err(EditorError::InvalidBoundary);
    }
    Ok(REGEX_OPTIONS
        .into_iter()
        .zip(seen)
        .filter_map(|(option, present)| present.then_some(option))
        .collect())
}

fn regex_source(source: &str) -> String {
    if source.is_empty() {
        return "(?:)".to_owned();
    }
    let mut output = String::new();
    let mut escaped = false;
    for character in source.chars() {
        match character {
            '/' if !escaped => output.push_str(r"\/"),
            '\n' | '\r' | '\u{2028}' | '\u{2029}' => {
                if escaped {
                    output.pop();
                }
                output.push_str(match character {
                    '\n' => r"\n",
                    '\r' => r"\r",
                    '\u{2028}' => r"\u2028",
                    _ => r"\u2029",
                });
            }
            _ => output.push(character),
        }
        escaped = character == '\\' && !escaped;
    }
    output
}

fn append(output: &mut Vec<u16>, value: &[u16], limit: usize) -> Result<(), EditorError> {
    if output
        .len()
        .checked_add(value.len())
        .is_none_or(|length| length > limit)
    {
        return Err(EditorError::Capacity);
    }
    output.extend_from_slice(value);
    Ok(())
}

fn replace(
    output: &mut Vec<u16>,
    format: &[FormatPart],
    input: &[u16],
    found: Option<&Match>,
    limit: usize,
) -> Result<(), EditorError> {
    for part in format {
        let value = match part {
            FormatPart::Text(value) => value.encode_utf16().collect(),
            FormatPart::Capture {
                index,
                shorthand,
                if_value,
                else_value,
            } => {
                let group = found
                    .and_then(|found| {
                        let value = index.value();
                        if value > found.captures.len() as f64
                            || value < 0.0
                            || value.fract() != 0.0
                        {
                            return None;
                        }
                        found.group(value as usize).map(|range| &input[range])
                    })
                    .unwrap_or(&[]);
                resolve_format(
                    group,
                    shorthand.as_deref(),
                    if_value.as_deref(),
                    else_value.as_deref(),
                )
            }
        };
        append(output, &value, limit)?;
    }
    Ok(())
}

fn resolve_format(
    value: &[u16],
    shorthand: Option<&str>,
    if_value: Option<&str>,
    else_value: Option<&str>,
) -> Vec<u16> {
    if let Some(shorthand) = shorthand {
        let text = String::from_utf16_lossy(value);
        let transformed = match shorthand {
            "upcase" => Some(text.to_uppercase()),
            "downcase" => Some(text.to_lowercase()),
            "capitalize" => return first_case(value, true),
            "pascalcase" => return word_case(&text, false),
            "camelcase" => return word_case(&text, true),
            "snakecase" => Some(snake_case(&text)),
            "kebabcase" => Some(kebab_case(&text)),
            _ => None,
        };
        if let Some(transformed) = transformed {
            return transformed.encode_utf16().collect();
        }
    }
    match (value.is_empty(), if_value, else_value) {
        (false, Some(value), _) | (true, _, Some(value)) => value.encode_utf16().collect(),
        _ => value.to_vec(),
    }
}

fn first_case(value: &[u16], uppercase: bool) -> Vec<u16> {
    let Some(first) = value
        .first()
        .and_then(|unit| char::from_u32(u32::from(*unit)))
    else {
        return value.to_vec();
    };
    let first = if uppercase {
        first.to_uppercase().collect::<String>()
    } else {
        first.to_lowercase().collect()
    };
    first
        .encode_utf16()
        .chain(value[1..].iter().copied())
        .collect()
}

fn word_case(text: &str, camel: bool) -> Vec<u16> {
    let words = WORDS.find_iter(text).collect::<Vec<_>>();
    if words.is_empty() {
        return text.encode_utf16().collect();
    }
    words
        .into_iter()
        .enumerate()
        .flat_map(|(index, word)| {
            first_case(
                &text[word.range].encode_utf16().collect::<Vec<_>>(),
                !camel || index != 0,
            )
        })
        .collect()
}

fn snake_case(text: &str) -> String {
    let mut output = String::new();
    let mut end = 0;
    for found in SNAKE_BOUNDARY.find_iter(text) {
        output.push_str(&text[end..found.range.start]);
        output.push_str(&text[found.group(1).unwrap()]);
        output.push('_');
        output.push_str(&text[found.group(2).unwrap()]);
        end = found.range.end;
    }
    output.push_str(&text[end..]);
    SNAKE_SEPARATORS.replace_all(&output, "_").to_lowercase()
}

fn kebab_case(text: &str) -> String {
    if WORDS.find(text).is_none() {
        return text.to_owned();
    }
    let cleaned = text.trim_matches(is_js_whitespace).trim_matches('_');
    let words = KEBAB_WORDS
        .find_iter(cleaned)
        .map(|found| cleaned[found.range].to_lowercase())
        .collect::<Vec<_>>();
    if !words.is_empty() {
        return words.join("-");
    }
    KEBAB_SEPARATORS
        .replace_all(cleaned, "-")
        .trim_matches('-')
        .to_lowercase()
}

fn is_js_whitespace(character: char) -> bool {
    matches!(character, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
