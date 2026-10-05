use lsp_types::{DocumentFilter, Uri};
use regex::{Regex, RegexBuilder};

use super::{DocumentMirror, Failure};

const MAX_PATTERN_BYTES: usize = 4096;
const MAX_PATTERN_DEPTH: usize = 32;
const MAX_COMPILED_PATTERN_BYTES: usize = 64 * 1024;

pub(crate) struct Filter {
    language: Option<String>,
    scheme: Option<String>,
    pattern: Option<Regex>,
}

impl Filter {
    pub(crate) fn compile(filter: DocumentFilter) -> Result<Self, Failure> {
        if [&filter.language, &filter.scheme, &filter.pattern]
            .iter()
            .all(|field| field.is_none())
            || [&filter.language, &filter.scheme, &filter.pattern]
                .iter()
                .any(|field| field.as_ref().is_some_and(String::is_empty))
        {
            return Err(Failure::MalformedResponse);
        }
        let pattern = filter
            .pattern
            .map(|pattern| {
                if pattern.len() > MAX_PATTERN_BYTES {
                    return Err(Failure::Capacity);
                }
                let chars = pattern.chars().collect::<Vec<_>>();
                let mut offset = 0;
                let source = sequence(&chars, &mut offset, 0, false)?;
                RegexBuilder::new(&format!("\\A(?:{source})\\z"))
                    .unicode(true)
                    .dot_matches_new_line(true)
                    .size_limit(MAX_COMPILED_PATTERN_BYTES)
                    .build()
                    .map_err(|_| Failure::MalformedResponse)
            })
            .transpose()?;
        Ok(Self {
            language: filter.language,
            scheme: filter.scheme,
            pattern,
        })
    }

    pub(crate) fn matches(&self, document: &DocumentMirror) -> bool {
        if self
            .language
            .as_ref()
            .is_some_and(|language| language != "*" && language != &document.language_id)
        {
            return false;
        }
        let Ok(uri) = document.uri.parse::<Uri>() else {
            return false;
        };
        let Some(scheme) = uri.scheme() else {
            return false;
        };
        if self.scheme.as_ref().is_some_and(|expected| {
            expected != "*" && !expected.eq_ignore_ascii_case(scheme.as_str())
        }) {
            return false;
        }
        let Ok(path) = uri.path().as_estr().decode().into_string() else {
            return false;
        };
        !path.contains('\0')
            && self
                .pattern
                .as_ref()
                .is_none_or(|pattern| pattern.is_match(&path))
    }
}

fn sequence(
    chars: &[char],
    offset: &mut usize,
    depth: usize,
    is_group: bool,
) -> Result<String, Failure> {
    if depth > MAX_PATTERN_DEPTH {
        return Err(Failure::Capacity);
    }
    let mut result = String::new();
    while let Some(character) = chars.get(*offset).copied() {
        if is_group && matches!(character, ',' | '}') {
            break;
        }
        *offset += 1;
        match character {
            '*' if chars.get(*offset) == Some(&'*') => {
                *offset += 1;
                if chars.get(*offset) == Some(&'/') {
                    *offset += 1;
                    result.push_str("(?:.*/)?");
                } else {
                    result.push_str(".*");
                }
            }
            '*' => result.push_str("[^/]*"),
            '?' => result.push_str("[^/]"),
            '{' => {
                result.push_str("(?:");
                loop {
                    result.push_str(&sequence(chars, offset, depth + 1, true)?);
                    match chars.get(*offset) {
                        Some(',') => {
                            *offset += 1;
                            result.push('|');
                        }
                        Some('}') => {
                            *offset += 1;
                            break;
                        }
                        _ => return Err(Failure::MalformedResponse),
                    }
                }
                result.push(')');
            }
            '[' => {
                let mut class = String::new();
                if chars.get(*offset) == Some(&'!') {
                    class.push('^');
                    *offset += 1;
                }
                let start = *offset;
                loop {
                    match chars.get(*offset).copied() {
                        Some(']') if *offset > start => {
                            *offset += 1;
                            break;
                        }
                        Some('-') => {
                            *offset += 1;
                            class.push('-');
                        }
                        Some(value) => {
                            *offset += 1;
                            class.push_str(&regex::escape(&value.to_string()));
                        }
                        None => return Err(Failure::MalformedResponse),
                    }
                }
                result.push_str(&format!("[[{class}]&&[^/]]"));
            }
            '\\' => {
                let escaped = chars.get(*offset).ok_or(Failure::MalformedResponse)?;
                *offset += 1;
                result.push_str(&regex::escape(&escaped.to_string()));
            }
            '}' => return Err(Failure::MalformedResponse),
            literal => result.push_str(&regex::escape(&literal.to_string())),
        }
    }
    Ok(result)
}
