use crate::document::EditorError;
use crate::folding::FoldRegion;
use crate::json_parser::is_ecma_whitespace;
use crate::json_scanner::{self, Error, Kind, Token};
use crate::syntax_folding::SyntaxFoldRange;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub scanner: json_scanner::Limits,
    pub max_work_bytes: usize,
    pub max_depth: usize,
    pub max_ranges: usize,
    pub result_limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folding {
    pub ranges: Vec<SyntaxFoldRange>,
    pub limit_exceeded: bool,
}

struct Budget {
    limits: Limits,
    work_bytes: usize,
    tokens: usize,
    value_units: usize,
}

impl Budget {
    fn scan(&mut self, input: &str, offset: usize) -> Result<Vec<Token>, EditorError> {
        self.work_bytes = self
            .work_bytes
            .checked_add(input.len() - offset)
            .ok_or(EditorError::Capacity)?;
        if self.work_bytes > self.limits.max_work_bytes {
            return Err(EditorError::Capacity);
        }
        let limits = json_scanner::Limits {
            max_tokens: self.limits.scanner.max_tokens - self.tokens,
            max_value_units: self.limits.scanner.max_value_units - self.value_units,
            ..self.limits.scanner
        };
        let tokens = json_scanner::scan(&input[offset..], false, limits)?;
        self.tokens += tokens.len();
        self.value_units += tokens.iter().map(|token| token.value.len()).sum::<usize>();
        Ok(tokens)
    }
}

fn region(text: &str) -> Option<bool> {
    let directive = text
        .strip_prefix("//")?
        .trim_start_matches(is_ecma_whitespace)
        .strip_prefix('#')?;
    for (keyword, is_start) in [("region", true), ("endregion", false)] {
        let Some(rest) = directive.strip_prefix(keyword) else {
            continue;
        };
        if rest
            .chars()
            .next()
            .is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_')
        {
            return Some(is_start);
        }
    }
    None
}

pub fn folding_ranges(input: &str, limits: Limits) -> Result<Folding, EditorError> {
    if input.len() > limits.scanner.max_bytes {
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
    let line_at = |byte| starts.partition_point(|&start| start <= byte) - 1;
    let mut budget = Budget {
        limits,
        work_bytes: 0,
        tokens: 0,
        value_units: 0,
    };
    let mut tokens = budget.scan(input, 0)?.into_iter();
    let mut origin = 0usize;
    let mut stack = Vec::<SyntaxFoldRange>::new();
    let mut ranges = Vec::new();
    let mut levels = Vec::new();
    let mut previous_start = None;
    while let Some(token) = tokens.next() {
        if token.kind == Kind::End {
            break;
        }
        let start_byte = origin + token.bytes.start;
        let end_byte = origin + token.bytes.end;
        let start_line = line_at(start_byte);
        let closing = match token.kind {
            Kind::CloseBrace => Some("object"),
            Kind::CloseBracket => Some("array"),
            _ => None,
        };
        let mut add = None;
        if matches!(token.kind, Kind::OpenBrace | Kind::OpenBracket) {
            if stack.len() >= limits.max_depth {
                return Err(EditorError::Capacity);
            }
            stack.push(SyntaxFoldRange {
                region: FoldRegion {
                    start_line,
                    end_line: start_line,
                },
                kind: Some(
                    if token.kind == Kind::OpenBrace {
                        "object"
                    } else {
                        "array"
                    }
                    .to_owned(),
                ),
            });
        } else if let Some(kind) = closing {
            if stack
                .last()
                .is_some_and(|range| range.kind.as_deref() == Some(kind))
            {
                let mut range = stack.pop().unwrap();
                if start_line > range.region.start_line + 1
                    && previous_start != Some(range.region.start_line)
                {
                    range.region.end_line = start_line - 1;
                    add = Some(range);
                }
            }
        } else if token.kind == Kind::BlockComment {
            if token.error == Some(Error::EndOfComment) && start_line + 1 < starts.len() {
                origin = starts[start_line + 1];
                tokens = budget.scan(input, origin)?.into_iter();
                continue;
            }
            let end_line = line_at(end_byte);
            if start_line < end_line {
                add = Some(SyntaxFoldRange {
                    region: FoldRegion {
                        start_line,
                        end_line,
                    },
                    kind: Some("comment".to_owned()),
                });
            }
        } else if token.kind == Kind::LineComment {
            match region(&input[start_byte..end_byte]) {
                Some(true) => {
                    if stack.len() >= limits.max_depth {
                        return Err(EditorError::Capacity);
                    }
                    stack.push(SyntaxFoldRange {
                        region: FoldRegion {
                            start_line,
                            end_line: start_line,
                        },
                        kind: Some("region".to_owned()),
                    });
                }
                Some(false) => {
                    if let Some(index) = stack
                        .iter()
                        .rposition(|range| range.kind.as_deref() == Some("region"))
                    {
                        let mut range = stack[index].clone();
                        stack.truncate(index);
                        if start_line > range.region.start_line
                            && previous_start != Some(range.region.start_line)
                        {
                            range.region.end_line = start_line;
                            add = Some(range);
                        }
                    }
                }
                None => {}
            }
        }
        if let Some(range) = add {
            if ranges.len() >= limits.max_ranges {
                return Err(EditorError::Capacity);
            }
            previous_start = Some(range.region.start_line);
            ranges.push(range);
            levels.push(stack.len());
        }
    }
    let Some(limit) = limits.result_limit.filter(|&limit| ranges.len() > limit) else {
        return Ok(Folding {
            ranges,
            limit_exceeded: false,
        });
    };
    let mut counts = vec![0usize; levels.iter().copied().max().unwrap_or(0) + 1];
    for &level in &levels {
        counts[level] += 1;
    }
    let mut entries = 0usize;
    let mut threshold = 0usize;
    for (level, count) in counts.into_iter().enumerate() {
        if count + entries > limit {
            threshold = level;
            break;
        }
        entries += count;
    }
    let mut remaining = limit - entries;
    let ranges = ranges
        .into_iter()
        .zip(levels)
        .filter_map(|(range, level)| {
            if level < threshold {
                return Some(range);
            }
            if level == threshold && remaining > 0 {
                remaining -= 1;
                return Some(range);
            }
            None
        })
        .collect();
    Ok(Folding {
        ranges,
        limit_exceeded: true,
    })
}
