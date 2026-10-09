use std::ops::Range;

use crate::document::DocumentSnapshot;
use crate::editing::line_text;
use crate::language_configuration::{Language, token_kind_at};
use crate::syntax::TokenKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedBrackets {
    pub open: Range<usize>,
    pub close: Range<usize>,
}

pub fn bracket_pairs(document: &DocumentSnapshot, language: Language<'_>) -> Vec<MatchedBrackets> {
    let mut stack = Vec::<(Range<usize>, String)>::new();
    let mut pairs = Vec::new();
    for line in 0..document.rope.len_lines() {
        let content = line_text(document, line);
        let Some(tokens) = language.syntax.tokens(document, line) else {
            continue;
        };
        for range in language.rules.bracket_ranges(&content.text) {
            if token_kind_at(&tokens, range.start) != TokenKind::Other {
                continue;
            }
            let text = &content.text[range.clone()];
            let absolute = content.start + range.start..content.start + range.end;
            let closes = |open: &str| {
                language.rules.pairs().brackets.iter().any(|pair| {
                    pair.open.eq_ignore_ascii_case(open) && pair.close.eq_ignore_ascii_case(text)
                })
            };
            if let Some(index) = stack.iter().rposition(|(_, open)| closes(open)) {
                let (open, _) = stack.remove(index);
                stack.truncate(index);
                pairs.push(MatchedBrackets {
                    open,
                    close: absolute,
                });
            } else if language
                .rules
                .pairs()
                .brackets
                .iter()
                .any(|pair| pair.open.eq_ignore_ascii_case(text))
            {
                stack.push((absolute, text.into()));
            }
        }
    }
    pairs.sort_by_key(|pair| (pair.open.start, std::cmp::Reverse(pair.close.end)));
    pairs
}

pub fn matching_brackets(pairs: &[MatchedBrackets], byte: usize) -> Option<&MatchedBrackets> {
    pairs
        .iter()
        .filter(|pair| contains(&pair.open, byte) || contains(&pair.close, byte))
        .max_by_key(|pair| {
            if contains(&pair.close, byte) {
                pair.close.start
            } else {
                pair.open.start
            }
        })
}

pub fn next_bracket(
    document: &DocumentSnapshot,
    language: Language<'_>,
    byte: usize,
    pairs: &[MatchedBrackets],
) -> Option<Range<usize>> {
    for line in document.rope.byte_to_line(byte)..document.rope.len_lines() {
        let content = line_text(document, line);
        let Some(tokens) = language.syntax.tokens(document, line) else {
            continue;
        };
        for range in language.rules.bracket_ranges(&content.text) {
            if content.start + range.end > byte
                && token_kind_at(&tokens, range.start) == TokenKind::Other
            {
                let absolute = content.start + range.start..content.start + range.end;
                let is_open = language
                    .rules
                    .pairs()
                    .brackets
                    .iter()
                    .any(|pair| pair.open.eq_ignore_ascii_case(&content.text[range.clone()]));
                if is_open || pairs.iter().any(|pair| pair.close == absolute) {
                    return Some(absolute);
                }
            }
        }
    }
    None
}

pub fn enclosing_brackets(pairs: &[MatchedBrackets], byte: usize) -> Option<&MatchedBrackets> {
    pairs
        .iter()
        .filter(|pair| pair.open.start < byte && byte < pair.close.end)
        .max_by_key(|pair| pair.open.start)
}

pub fn contains(range: &Range<usize>, byte: usize) -> bool {
    range.start <= byte && byte <= range.end
}
