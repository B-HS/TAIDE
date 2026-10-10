use std::ops::Range;

use ferroni::api::Regex;
use ferroni::oniguruma::{ONIG_OPTION_CAPTURE_GROUP, ONIG_OPTION_IGNORECASE_IS_ASCII};

const SUPPORTED_FLAGS: &str = "gims";
const IGNORE_CASE_FLAG: char = 'i';
const MULTILINE_FLAG: char = 'm';
const DOT_ALL_FLAG: char = 's';
const DIGIT_ITEMS: &str = "0-9";
const WORD_ITEMS: &str = "A-Za-z0-9_";
const SPACE_ITEMS: &str =
    r"\t\n\x0B\f\r \x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}";
const LINE_TERMINATOR_ITEMS: &str = r"\n\r\x{2028}\x{2029}";
const ANY_CHARACTER: &str = r"[\x{0}-\x{10FFFF}]";
const NEVER_MATCHES: &str = "(?!)";
const INPUT_START: &str = r"\A";
const INPUT_END: &str = r"\z";
const BACKSPACE: &str = r"\x08";
const NULL_CHARACTER: &str = r"\x00";
const HEX_ESCAPE_DIGITS: usize = 2;
const UNICODE_ESCAPE_DIGITS: usize = 4;
const HEX_RADIX: u32 = 16;
const SURROGATES: Range<u32> = 0xD800..0xE000;
const FIRST_CAPTURE_GROUP: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsRegexError {
    UnsupportedFlag(char),
    UnsupportedSyntax(&'static str),
    Engine(String),
}

#[derive(Debug)]
pub struct JsRegex {
    regex: Regex,
}

impl JsRegex {
    pub fn new(source: &str, flags: &str) -> Result<Self, JsRegexError> {
        let translated = oniguruma_source(source, flags)?;
        let is_case_insensitive = flags.contains(IGNORE_CASE_FLAG);
        let mut builder = Regex::builder(&translated)
            .case_insensitive(is_case_insensitive)
            .option(ONIG_OPTION_CAPTURE_GROUP);
        if is_case_insensitive {
            builder = builder.option(ONIG_OPTION_IGNORECASE_IS_ASCII);
        }
        builder
            .build()
            .map(|regex| Self { regex })
            .map_err(|error| JsRegexError::Engine(error.to_string()))
    }

    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }

    pub fn captures_text_in_first_group(&self, text: &str) -> Option<bool> {
        self.regex.captures(text).map(|captures| {
            captures
                .get(FIRST_CAPTURE_GROUP)
                .is_some_and(|group| !group.is_empty())
        })
    }

    pub fn first_range(&self, text: &str) -> Option<Range<usize>> {
        self.regex.find(text).map(|found| found.range())
    }

    pub fn ranges(&self, text: &str) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        self.visit_ranges(text, |range| {
            if !range.is_empty() {
                ranges.push(range);
            }
            true
        });
        ranges
    }

    pub fn visit_ranges(&self, text: &str, mut visit: impl FnMut(Range<usize>) -> bool) {
        for found in self.regex.find_iter(text) {
            if !visit(found.range()) {
                break;
            }
        }
    }

    pub fn without_matches(&self, text: &str) -> String {
        let mut kept = String::with_capacity(text.len());
        let mut copied = 0;
        for range in self.ranges(text) {
            kept.push_str(&text[copied..range.start]);
            copied = range.end;
        }
        kept.push_str(&text[copied..]);
        kept
    }
}

pub fn oniguruma_source(source: &str, flags: &str) -> Result<String, JsRegexError> {
    if let Some(flag) = flags.chars().find(|flag| !SUPPORTED_FLAGS.contains(*flag)) {
        return Err(JsRegexError::UnsupportedFlag(flag));
    }
    let is_multiline = flags.contains(MULTILINE_FLAG);
    let dot_matches_all = flags.contains(DOT_ALL_FLAG);
    let characters: Vec<char> = source.chars().collect();
    let mut output = String::with_capacity(source.len() * 2);
    let mut index = 0;
    while let Some(&character) = characters.get(index) {
        match character {
            '\\' => index = escape(&characters, index, false, &mut output)?,
            '[' => index = class(&characters, index, &mut output)?,
            '^' if is_multiline => {
                output.push_str(&format!("(?:{INPUT_START}|(?<=[{LINE_TERMINATOR_ITEMS}]))"));
                index += 1;
            }
            '^' => {
                output.push_str(INPUT_START);
                index += 1;
            }
            '$' if is_multiline => {
                output.push_str(&format!("(?=[{LINE_TERMINATOR_ITEMS}]|{INPUT_END})"));
                index += 1;
            }
            '$' => {
                output.push_str(INPUT_END);
                index += 1;
            }
            '.' if dot_matches_all => {
                output.push_str(ANY_CHARACTER);
                index += 1;
            }
            '.' => {
                output.push_str(&format!("[^{LINE_TERMINATOR_ITEMS}]"));
                index += 1;
            }
            '(' => {
                if characters.get(index + 1) == Some(&'?')
                    && !is_supported_group(&characters[index + 2..])
                {
                    return Err(JsRegexError::UnsupportedSyntax("group modifier"));
                }
                output.push('(');
                index += 1;
            }
            '{' => match quantifier_end(&characters, index) {
                Some(end) => {
                    output.extend(&characters[index..end]);
                    index = end;
                }
                None => {
                    output.push_str(r"\{");
                    index += 1;
                }
            },
            '}' | ']' => {
                output.push('\\');
                output.push(character);
                index += 1;
            }
            _ => {
                output.push(character);
                index += 1;
            }
        }
    }
    Ok(output)
}

fn is_supported_group(after_question_mark: &[char]) -> bool {
    match after_question_mark {
        [':' | '=' | '!', ..] => true,
        ['<', '=' | '!', ..] => true,
        ['<', name, ..] => name.is_alphabetic() || matches!(name, '_' | '$'),
        _ => false,
    }
}

fn quantifier_end(characters: &[char], start: usize) -> Option<usize> {
    let digits_end = |from: usize| {
        from + characters[from..]
            .iter()
            .take_while(|character| character.is_ascii_digit())
            .count()
    };
    let minimum_end = digits_end(start + 1);
    if minimum_end == start + 1 {
        return None;
    }
    match characters.get(minimum_end) {
        Some('}') => Some(minimum_end + 1),
        Some(',') => {
            let maximum_end = digits_end(minimum_end + 1);
            (characters.get(maximum_end) == Some(&'}')).then_some(maximum_end + 1)
        }
        _ => None,
    }
}

fn is_class_escape(characters: &[char], backslash: usize) -> bool {
    characters.get(backslash) == Some(&'\\')
        && matches!(
            characters.get(backslash + 1),
            Some('d' | 'D' | 'w' | 'W' | 's' | 'S')
        )
}

fn hex_code(characters: &[char], start: usize, digits: usize) -> Option<u32> {
    let text: String = characters.get(start..start + digits)?.iter().collect();
    u32::from_str_radix(&text, HEX_RADIX).ok()
}

fn push_set(output: &mut String, items: &str, is_negated: bool, in_class: bool) {
    if in_class && !is_negated {
        output.push_str(items);
        return;
    }
    output.push('[');
    if is_negated {
        output.push('^');
    }
    output.push_str(items);
    output.push(']');
}

fn word_boundary(is_negated: bool) -> String {
    let (after_word, after_other) = if is_negated { ("=", "!") } else { ("!", "=") };
    format!(
        "(?:(?<=[{WORD_ITEMS}])(?{after_word}[{WORD_ITEMS}])|(?<![{WORD_ITEMS}])(?{after_other}[{WORD_ITEMS}]))"
    )
}

fn escape(
    characters: &[char],
    backslash: usize,
    in_class: bool,
    output: &mut String,
) -> Result<usize, JsRegexError> {
    let Some(&escaped) = characters.get(backslash + 1) else {
        return Err(JsRegexError::UnsupportedSyntax("trailing backslash"));
    };
    let after = backslash + 2;
    match escaped {
        'd' => push_set(output, DIGIT_ITEMS, false, in_class),
        'D' => push_set(output, DIGIT_ITEMS, true, in_class),
        'w' => push_set(output, WORD_ITEMS, false, in_class),
        'W' => push_set(output, WORD_ITEMS, true, in_class),
        's' => push_set(output, SPACE_ITEMS, false, in_class),
        'S' => push_set(output, SPACE_ITEMS, true, in_class),
        'b' if in_class => output.push_str(BACKSPACE),
        'b' => output.push_str(&word_boundary(false)),
        'B' if in_class => output.push('B'),
        'B' => output.push_str(&word_boundary(true)),
        't' | 'n' | 'v' | 'f' | 'r' => {
            output.push('\\');
            output.push(escaped);
        }
        '0' if !characters.get(after).is_some_and(char::is_ascii_digit) => {
            output.push_str(NULL_CHARACTER);
        }
        '0'..='9' if in_class => return Err(JsRegexError::UnsupportedSyntax("octal escape")),
        '0' => return Err(JsRegexError::UnsupportedSyntax("octal escape")),
        '1'..='9' => {
            let end = after
                + characters[after..]
                    .iter()
                    .take_while(|character| character.is_ascii_digit())
                    .count();
            output.extend(&characters[backslash..end]);
            return Ok(end);
        }
        'c' => {
            let Some(control) = characters
                .get(after)
                .filter(|control| control.is_ascii_alphabetic())
            else {
                return Err(JsRegexError::UnsupportedSyntax("control escape"));
            };
            output.push_str(r"\c");
            output.push(*control);
            return Ok(after + 1);
        }
        'x' => match hex_code(characters, after, HEX_ESCAPE_DIGITS) {
            Some(code) => {
                output.push_str(&format!(r"\x{{{code:X}}}"));
                return Ok(after + HEX_ESCAPE_DIGITS);
            }
            None => output.push('x'),
        },
        'u' => match hex_code(characters, after, UNICODE_ESCAPE_DIGITS) {
            Some(code) if SURROGATES.contains(&code) => {
                return Err(JsRegexError::UnsupportedSyntax("surrogate escape"));
            }
            Some(code) => {
                output.push_str(&format!(r"\x{{{code:X}}}"));
                return Ok(after + UNICODE_ESCAPE_DIGITS);
            }
            None => output.push('u'),
        },
        'k' if !in_class && characters.get(after) == Some(&'<') => {
            let Some(length) = characters[after..]
                .iter()
                .position(|character| *character == '>')
            else {
                return Err(JsRegexError::UnsupportedSyntax("named backreference"));
            };
            output.extend(&characters[backslash..=after + length]);
            return Ok(after + length + 1);
        }
        letter if letter.is_ascii_alphabetic() => output.push(letter),
        other => {
            output.push('\\');
            output.push(other);
        }
    }
    Ok(after)
}

fn class(characters: &[char], open: usize, output: &mut String) -> Result<usize, JsRegexError> {
    let mut index = open + 1;
    let is_negated = characters.get(index) == Some(&'^');
    if is_negated {
        index += 1;
    }
    let mut items = String::new();
    let mut follows_class_escape = false;
    loop {
        let Some(&character) = characters.get(index) else {
            return Err(JsRegexError::UnsupportedSyntax(
                "unterminated character class",
            ));
        };
        let is_escape = is_class_escape(characters, index);
        match character {
            ']' => break,
            '\\' => index = escape(characters, index, true, &mut items)?,
            '-' if follows_class_escape || is_class_escape(characters, index + 1) => {
                items.push_str(r"\-");
                index += 1;
            }
            '[' | '&' => {
                items.push('\\');
                items.push(character);
                index += 1;
            }
            _ => {
                items.push(character);
                index += 1;
            }
        }
        follows_class_escape = is_escape;
    }
    match (items.is_empty(), is_negated) {
        (true, false) => output.push_str(NEVER_MATCHES),
        (true, true) => output.push_str(ANY_CHARACTER),
        (false, _) => push_set(output, &items, is_negated, false),
    }
    Ok(index + 1)
}

#[cfg(test)]
#[path = "js-regex-tests.rs"]
mod tests;
