use crate::document::EditorError;
use crate::folding::FoldRegion;
use crate::html_parser::is_void_element;
use crate::html_scanner::{Scanner, ScannerState, TokenType};
use crate::json_parser::is_ecma_whitespace as is_js_whitespace;
use crate::syntax_folding::SyntaxFoldRange;

const REGION_START: &str = "#region";
const REGION_END: &str = "endregion";

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_depth: usize,
    pub max_ranges: usize,
}

struct Open {
    line: usize,
    tag: Option<String>,
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn ends_word(text: &str, end: usize) -> bool {
    !text.as_bytes().get(end).copied().is_some_and(is_word_byte)
}

fn comment_region(text: &str) -> Option<bool> {
    let trimmed = text.trim_start_matches(is_js_whitespace);
    let marker = text.len() - trimmed.len() + REGION_START.len();
    if trimmed.starts_with(REGION_START) && ends_word(text, marker) {
        return Some(true);
    }
    text.match_indices(REGION_END)
        .any(|(index, _)| ends_word(text, index + REGION_END.len()))
        .then_some(false)
}

/// Computes HTML folding ranges (elements, `<!-- #region -->` markers, multi-line comments) like the HTML language service.
pub fn folding_ranges(input: &str, limits: Limits) -> Result<Vec<SyntaxFoldRange>, EditorError> {
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
    let mut scanner = Scanner::new(input, 0, ScannerState::WithinContent, false);
    let mut stack = Vec::<Open>::new();
    let mut ranges = Vec::<SyntaxFoldRange>::new();
    let mut last_tag = None::<String>;
    let mut previous_start = None::<usize>;
    let mut tokens = 0usize;
    loop {
        let token = scanner.scan();
        if token == TokenType::Eos {
            break;
        }
        tokens += 1;
        if tokens > limits.max_tokens {
            return Err(EditorError::Capacity);
        }
        let mut add = None;
        match token {
            TokenType::StartTag => {
                if stack.len() >= limits.max_depth {
                    return Err(EditorError::Capacity);
                }
                let tag = scanner.token_text().to_owned();
                stack.push(Open {
                    line: line_at(scanner.token_offset()),
                    tag: Some(tag.clone()),
                });
                last_tag = Some(tag);
            }
            TokenType::EndTag => last_tag = Some(scanner.token_text().to_owned()),
            TokenType::StartTagClose | TokenType::EndTagClose | TokenType::StartTagSelfClose => {
                if token == TokenType::StartTagClose
                    && !last_tag.as_deref().is_some_and(is_void_element)
                {
                    continue;
                }
                if let Some(index) = stack
                    .iter()
                    .rposition(|open| open.tag.is_some() && open.tag == last_tag)
                {
                    let start_line = stack[index].line;
                    stack.truncate(index);
                    let end_line = line_at(scanner.token_offset()).checked_sub(1);
                    if let Some(end_line) = end_line
                        && end_line > start_line
                        && previous_start != Some(start_line)
                    {
                        add = Some((start_line, end_line, None));
                    }
                }
            }
            TokenType::Comment => {
                let start_line = line_at(scanner.token_offset());
                match comment_region(scanner.token_text()) {
                    Some(true) => {
                        if stack.len() >= limits.max_depth {
                            return Err(EditorError::Capacity);
                        }
                        stack.push(Open {
                            line: start_line,
                            tag: None,
                        });
                    }
                    Some(false) => {
                        if let Some(index) = stack.iter().rposition(|open| open.tag.is_none()) {
                            let region_start = stack[index].line;
                            stack.truncate(index);
                            if start_line > region_start && previous_start != Some(region_start) {
                                add = Some((region_start, start_line, Some("region")));
                            }
                        }
                    }
                    None => {
                        let end_line = line_at(scanner.token_end());
                        if start_line < end_line {
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
            previous_start = Some(start_line);
        }
    }
    ranges.sort_by_key(|range| range.region.start_line);
    Ok(ranges)
}
