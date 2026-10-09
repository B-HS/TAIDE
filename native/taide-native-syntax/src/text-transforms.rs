use ferroni::api::Regex;
use ferroni::oniguruma::ONIG_OPTION_CAPTURE_GROUP;
use taide_native_editor::line_commands::{TextCase, TextTransforms};

use crate::JsRegexError;

pub struct MonacoTextTransforms {
    title: Regex,
    case_boundary: Regex,
    single_letters: Regex,
    valid_start: Regex,
    capitals: Regex,
}

impl MonacoTextTransforms {
    pub fn new() -> Result<Self, JsRegexError> {
        let regex = |source| {
            Regex::builder(source)
                .option(ONIG_OPTION_CAPTURE_GROUP)
                .build()
                .map_err(|error| JsRegexError::Engine(error.to_string()))
        };
        Ok(Self {
            title: regex(
                r"((?:\A|(?<=[\n\r\x{2028}\x{2029}]))|[^\p{L}\p{N}']|(((?:\A|(?<=[\n\r\x{2028}\x{2029}]))|\P{L})'))\p{L}",
            )?,
            case_boundary: regex(r"(\p{Ll})(\p{Lu})")?,
            single_letters: regex(r"(\p{Lu}|\p{N})(\p{Lu})(\p{Ll})")?,
            valid_start: regex(r"\A(\p{Lu}[^\p{Lu}])")?,
            capitals: regex(
                r"(?:\A|(?<=[\n\r\x{2028}\x{2029}]))\p{Lu}+(?=[\n\r\x{2028}\x{2029}]|\z)",
            )?,
        })
    }
}

fn replace(regex: &Regex, text: &str, modify: impl Fn(&str) -> String) -> String {
    let mut result = String::with_capacity(text.len());
    let mut start = 0;
    for found in regex.find_iter(text) {
        let range = found.range();
        result.push_str(&text[start..range.start]);
        result.push_str(&modify(found.as_str()));
        start = range.end;
    }
    result.push_str(&text[start..]);
    result
}

fn separated(text: &str, separator: char) -> String {
    let first = text.chars().next().unwrap().len_utf8();
    format!("{}{separator}{}", &text[..first], &text[first..])
}

fn first_upper(text: &str) -> String {
    let Some(first) = text.chars().next() else {
        return String::new();
    };
    let prefix = if first.len_utf16() == 1 {
        first.to_uppercase().collect::<String>()
    } else {
        first.to_string()
    };
    format!("{prefix}{}", &text[first.len_utf8()..])
}

impl TextTransforms for MonacoTextTransforms {
    fn transform(&self, case: TextCase, text: &str) -> String {
        match case {
            TextCase::Upper => text.to_uppercase(),
            TextCase::Lower => text.to_lowercase(),
            TextCase::Title => replace(&self.title, &text.to_lowercase(), str::to_uppercase),
            TextCase::Snake | TextCase::Kebab => {
                let separator = if case == TextCase::Snake { '_' } else { '-' };
                let mut text = text.to_string();
                if case == TextCase::Kebab {
                    let mut units = text.encode_utf16().collect::<Vec<_>>();
                    let mut index = 0;
                    let non_space = |unit| {
                        char::from_u32(u32::from(unit)).is_none_or(|character| {
                            !taide_native_editor::language_configuration::is_js_whitespace(
                                character,
                            )
                        })
                    };
                    while index + 2 < units.len() {
                        if non_space(units[index])
                            && units[index + 1] == u16::from(b'_')
                            && non_space(units[index + 2])
                        {
                            units[index + 1] = u16::from(b'-');
                            index += 3;
                        } else {
                            index += 1;
                        }
                    }
                    text = String::from_utf16(&units).unwrap();
                }
                let text = replace(&self.case_boundary, &text, |matched| {
                    separated(matched, separator)
                });
                replace(&self.single_letters, &text, |matched| {
                    separated(matched, separator)
                })
                .to_lowercase()
            }
            TextCase::Camel => {
                let multiline = text.contains(['\r', '\n']);
                let mut words = text
                    .split(|character| {
                        matches!(character, '_' | '-')
                            || (!multiline
                                && taide_native_editor::language_configuration::is_js_whitespace(
                                    character,
                                ))
                    })
                    .filter(|word| !word.is_empty());
                let first = if text.starts_with(|character| {
                    matches!(character, '_' | '-')
                        || (!multiline
                            && taide_native_editor::language_configuration::is_js_whitespace(
                                character,
                            ))
                }) {
                    String::new()
                } else {
                    words
                        .next()
                        .map(|word| replace(&self.valid_start, word, str::to_lowercase))
                        .unwrap_or_default()
                };
                let mut result = first;
                for word in words {
                    result.push_str(&first_upper(word));
                }
                result
            }
            TextCase::Pascal => text
                .split_inclusive('.')
                .flat_map(|part| part.split(['_', ' ', '\t', '-']))
                .map(|word| {
                    let normalized = first_upper(word);
                    if normalized.encode_utf16().count() > 1 && self.capitals.is_match(&normalized)
                    {
                        let first = normalized.chars().next().unwrap().len_utf8();
                        format!(
                            "{}{}",
                            &normalized[..first],
                            normalized[first..].to_lowercase()
                        )
                    } else {
                        normalized
                    }
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "text-transforms-tests.rs"]
mod tests;
