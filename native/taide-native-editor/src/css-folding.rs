use crate::css_scanner::{Dialect, Kind, Scanner};
use crate::document::EditorError;
use crate::folding::FoldRegion;
use crate::json_parser::is_ecma_whitespace as is_css_whitespace;
use crate::syntax_folding::SyntaxFoldRange;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_depth: usize,
    pub max_ranges: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Delimiter {
    Brace,
    Comment,
}

struct Marker {
    line: usize,
    delimiter: Delimiter,
}

fn block_region(text: &str) -> Option<bool> {
    let rest = text
        .trim_start_matches(is_css_whitespace)
        .strip_prefix("/*")?
        .trim_start_matches(is_css_whitespace);
    let (is_start, rest) = match rest.strip_prefix("#endregion") {
        Some(rest) => (false, rest),
        None => (true, rest.strip_prefix("#region")?),
    };
    if rest
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    let label = rest[..rest.find("*/")?].trim_start_matches(is_css_whitespace);
    let trailing = label
        .find(['\n', '\r', '\u{2028}', '\u{2029}'])
        .map_or("", |index| &label[index..]);
    trailing.chars().all(is_css_whitespace).then_some(is_start)
}

fn line_region(text: &str) -> Option<bool> {
    let rest = text
        .trim_start_matches(is_css_whitespace)
        .strip_prefix("//")?
        .trim_start_matches(is_css_whitespace);
    let (is_start, rest) = match rest.strip_prefix("#endregion") {
        Some(rest) => (false, rest),
        None => (true, rest.strip_prefix("#region")?),
    };
    if rest
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    Some(is_start)
}

fn take_start(stack: &mut Vec<Marker>, delimiter: Delimiter) -> Option<Marker> {
    let index = stack
        .iter()
        .rposition(|marker| marker.delimiter == delimiter)?;
    Some(stack.remove(index))
}

pub fn folding_ranges(
    input: &str,
    dialect: Dialect,
    limits: Limits,
) -> Result<Vec<SyntaxFoldRange>, EditorError> {
    if input.len() > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    let mut starts = vec![0];
    let mut characters = input.char_indices().peekable();
    while let Some((byte, character)) = characters.next() {
        if character == '\n'
            || character == '\r'
                && !characters
                    .peek()
                    .is_some_and(|(_, character)| *character == '\n')
        {
            starts.push(byte + character.len_utf8());
        }
    }
    let line_at = |byte: usize| starts.partition_point(|&start| start <= byte) - 1;
    let mut scanner = Scanner::new(input, dialect);
    scanner.ignore_comment = false;
    let mut stack = Vec::<Marker>::new();
    let mut ranges = Vec::<SyntaxFoldRange>::new();
    let mut previous_end_line = None;
    let mut tokens = 0usize;
    loop {
        let token = scanner.scan();
        if token.kind == Kind::Eof {
            break;
        }
        tokens += 1;
        if tokens > limits.max_tokens {
            return Err(EditorError::Capacity);
        }
        let start_line = line_at(token.bytes.start);
        let end_line = line_at(token.bytes.end);
        let mut add = None;
        match token.kind {
            Kind::CurlyL | Kind::InterpolationFunction => {
                if stack.len() >= limits.max_depth {
                    return Err(EditorError::Capacity);
                }
                stack.push(Marker {
                    line: start_line,
                    delimiter: Delimiter::Brace,
                });
            }
            Kind::CurlyR => {
                if let Some(start) = take_start(&mut stack, Delimiter::Brace) {
                    let mut end = end_line;
                    if previous_end_line.is_some_and(|line| line != end) {
                        end -= 1;
                    }
                    if start.line != end {
                        add = Some((start.line, end, None));
                    }
                }
            }
            Kind::Comment => {
                let region = block_region(&token.text).or_else(|| {
                    (dialect == Dialect::Scss)
                        .then(|| line_region(&token.text))
                        .flatten()
                });
                match region {
                    Some(true) => {
                        if stack.len() >= limits.max_depth {
                            return Err(EditorError::Capacity);
                        }
                        stack.push(Marker {
                            line: start_line,
                            delimiter: Delimiter::Comment,
                        });
                    }
                    Some(false) => {
                        if let Some(start) = take_start(&mut stack, Delimiter::Comment)
                            && start.line != end_line
                        {
                            add = Some((start.line, end_line, Some("region")));
                        }
                    }
                    None => {
                        if start_line != end_line {
                            add = Some((start_line, end_line, Some("comment")));
                        }
                    }
                }
            }
            _ => {}
        }
        if let Some((start_line, end_line, kind)) = add {
            if ranges.len() >= limits.max_ranges {
                return Err(EditorError::Capacity);
            }
            ranges.push(SyntaxFoldRange {
                region: FoldRegion {
                    start_line,
                    end_line,
                },
                kind: kind.map(str::to_owned),
            });
        }
        previous_end_line = Some(end_line);
    }
    ranges.sort_by_key(|range| (range.region.start_line, range.region.end_line));
    let mut previous_end = None::<usize>;
    Ok(ranges
        .into_iter()
        .filter(|range| {
            let overlaps = previous_end.is_some_and(|previous| {
                range.region.start_line < previous && previous < range.region.end_line
            });
            if !overlaps {
                previous_end = Some(range.region.end_line);
            }
            !overlaps
        })
        .collect())
}
