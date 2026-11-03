use std::ops::Range;

use crate::document::EditorError;
use crate::html_parser::{self, Document, Node};
use crate::html_scanner::{Scanner, ScannerState, TokenType};

const SELF_CLOSE: &[u8] = b"/>";
const END_TAG_OPEN_LENGTH: usize = 2;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub parser: html_parser::Limits,
    pub max_positions: usize,
    pub max_ranges: usize,
}

fn node_ranges(node: &Node, result: &mut Vec<Range<usize>>) {
    if let (Some(start_tag_end), Some(end_tag_start)) = (node.start_tag_end, node.end_tag_start)
        && start_tag_end < end_tag_start
    {
        result.push(start_tag_end..end_tag_start);
    }
    result.push(node.start..node.end);
}

fn attribute_ranges(
    input: &str,
    node: &Node,
    offset: usize,
    max_tokens: usize,
) -> Result<Vec<Range<usize>>, EditorError> {
    let text = &input[node.start..node.end];
    let relative = offset - node.start;
    let mut scanner = Scanner::new(text, 0, ScannerState::WithinContent, false);
    let mut result = Vec::<Range<usize>>::new();
    let mut inside_attribute = false;
    let mut attribute_start = 0usize;
    let mut tokens = 0usize;
    loop {
        let token = scanner.scan();
        if token == TokenType::Eos {
            break;
        }
        tokens += 1;
        if tokens > max_tokens {
            return Err(EditorError::Capacity);
        }
        let (start, end) = (scanner.token_offset(), scanner.token_end());
        match token {
            TokenType::AttributeName => {
                if relative < start {
                    inside_attribute = false;
                    continue;
                }
                if relative <= end {
                    result.insert(0, start..end);
                }
                inside_attribute = true;
                attribute_start = start;
            }
            TokenType::AttributeValue if inside_attribute => {
                if relative < start {
                    result.push(attribute_start..end);
                    continue;
                }
                if relative <= end {
                    result.insert(0, start..end);
                    let value = scanner.token_text().as_bytes();
                    let quoted = value
                        .first()
                        .zip(value.last())
                        .is_some_and(|(&first, &last)| {
                            first == last && matches!(first, b'"' | b'\'')
                        });
                    if quoted && relative > start && relative < end {
                        result.insert(0, start + 1..end - 1);
                    }
                    result.push(attribute_start..end);
                }
            }
            _ => {}
        }
    }
    Ok(result
        .into_iter()
        .map(|range| range.start + node.start..range.end + node.start)
        .collect())
}

fn applicable_ranges(
    input: &str,
    document: &Document,
    offset: usize,
    max_tokens: usize,
) -> Result<Vec<Range<usize>>, EditorError> {
    let index = document.find_node_at(offset);
    let node = &document.nodes[index];
    let mut result = Vec::new();
    let mut parent = node.parent;
    while let Some(current) = parent {
        node_ranges(&document.nodes[current], &mut result);
        parent = document.nodes[current].parent;
    }
    let with_attributes = |result: Vec<Range<usize>>| -> Result<Vec<Range<usize>>, EditorError> {
        let mut ranges = attribute_ranges(input, node, offset, max_tokens)?;
        ranges.extend(result);
        Ok(ranges)
    };
    let Some(start_tag_end) = node.start_tag_end else {
        return Ok(result);
    };
    let Some(end_tag_start) = node.end_tag_start else {
        if start_tag_end != node.end {
            return Ok(vec![node.start..node.end]);
        }
        let self_closing = input.as_bytes()[..start_tag_end].ends_with(SELF_CLOSE);
        let inner_end = start_tag_end - if self_closing { SELF_CLOSE.len() } else { 1 };
        result.insert(0, node.start + 1..inner_end);
        return with_attributes(result);
    };
    result.insert(0, node.start..node.end);
    if node.start < offset && offset < start_tag_end {
        result.insert(0, node.start + 1..start_tag_end - 1);
        return with_attributes(result);
    }
    if start_tag_end <= offset && offset <= end_tag_start {
        result.insert(0, start_tag_end..end_tag_start);
    } else if offset >= end_tag_start + END_TAG_OPEN_LENGTH {
        result.insert(0, end_tag_start + END_TAG_OPEN_LENGTH..node.end - 1);
    }
    Ok(result)
}

/// Computes the smart-select chain (innermost first) for each byte position like the HTML language service.
pub fn selection_ranges(
    input: &str,
    positions: &[usize],
    limits: Limits,
) -> Result<Vec<Vec<Range<usize>>>, EditorError> {
    if positions.len() > limits.max_positions {
        return Err(EditorError::Capacity);
    }
    if positions
        .iter()
        .any(|&byte| byte > input.len() || !input.is_char_boundary(byte))
    {
        return Err(EditorError::InvalidBoundary);
    }
    let document = html_parser::parse(input, limits.parser)?;
    let mut count = 0usize;
    let mut result = Vec::with_capacity(positions.len());
    for &offset in positions {
        let mut ranges = applicable_ranges(input, &document, offset, limits.parser.max_tokens)?;
        ranges.dedup();
        if ranges.is_empty() {
            ranges.push(offset..offset);
        }
        count = count
            .checked_add(ranges.len())
            .ok_or(EditorError::Capacity)?;
        if count > limits.max_ranges {
            return Err(EditorError::Capacity);
        }
        result.push(ranges);
    }
    Ok(result)
}
