use std::ops::Range;
use std::time::Duration;

use crate::document::{DocumentSnapshot, Edit, EditorError, UndoGroup};
use crate::editing::{SEPARATE_STEP, apply_step, normalize_line_breaks};
use crate::find::{FindPatternError, FindQuery};
use crate::store::{EditorStore, Transaction};
use crate::view::ViewId;

const DECIMAL_RADIX: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplacePiece {
    Static(String),
    Capture { index: usize, case_ops: Vec<char> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacePattern {
    pieces: Vec<ReplacePiece>,
}

impl ReplacePattern {
    pub fn new(source: &str, is_regex: bool) -> Self {
        if !is_regex {
            return Self {
                pieces: vec![ReplacePiece::Static(source.to_string())],
            };
        }
        let mut characters = source.chars().peekable();
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut case_ops = Vec::new();
        while let Some(character) = characters.next() {
            if character == '\\' {
                let Some(next) = characters.next() else {
                    literal.push(character);
                    break;
                };
                match next {
                    '\\' => literal.push('\\'),
                    'n' => literal.push('\n'),
                    't' => literal.push('\t'),
                    'u' | 'U' | 'l' | 'L' => case_ops.push(next),
                    _ => {
                        literal.push(character);
                        literal.push(next);
                    }
                }
                continue;
            }
            if character != '$' {
                literal.push(character);
                continue;
            }
            let Some(next) = characters.next() else {
                literal.push(character);
                break;
            };
            if next == '$' {
                literal.push('$');
                continue;
            }
            let index = match next {
                '0' | '&' => 0,
                '1'..='9' => {
                    let mut index = next.to_digit(DECIMAL_RADIX as u32).unwrap() as usize;
                    if let Some(digit) = characters.peek().copied().filter(char::is_ascii_digit) {
                        characters.next();
                        index = index * DECIMAL_RADIX
                            + digit.to_digit(DECIMAL_RADIX as u32).unwrap() as usize;
                    }
                    index
                }
                _ => {
                    literal.push(character);
                    literal.push(next);
                    continue;
                }
            };
            if !literal.is_empty() {
                pieces.push(ReplacePiece::Static(std::mem::take(&mut literal)));
            }
            pieces.push(ReplacePiece::Capture {
                index,
                case_ops: std::mem::take(&mut case_ops),
            });
        }
        if !literal.is_empty() {
            pieces.push(ReplacePiece::Static(literal));
        }
        Self { pieces }
    }

    pub fn has_captures(&self) -> bool {
        self.pieces
            .iter()
            .any(|piece| matches!(piece, ReplacePiece::Capture { .. }))
    }

    pub fn build(&self, captures: &[Option<String>], preserve_case: bool) -> String {
        if !self.has_captures() {
            let value = self
                .pieces
                .iter()
                .filter_map(|piece| match piece {
                    ReplacePiece::Static(value) => Some(value.as_str()),
                    ReplacePiece::Capture { .. } => None,
                })
                .collect::<String>();
            return if preserve_case {
                preserve_replacement_case(
                    captures
                        .first()
                        .and_then(Option::as_deref)
                        .unwrap_or_default(),
                    &value,
                )
            } else {
                value
            };
        }
        self.pieces
            .iter()
            .map(|piece| match piece {
                ReplacePiece::Static(value) => value.clone(),
                ReplacePiece::Capture { index, case_ops } => {
                    let value = substitute_capture(*index, captures);
                    apply_case_ops(&value, case_ops)
                }
            })
            .collect()
    }
}

fn substitute_capture(mut index: usize, captures: &[Option<String>]) -> String {
    if index == 0 {
        return captures
            .first()
            .and_then(Option::as_deref)
            .unwrap_or_default()
            .to_string();
    }
    let mut remainder = String::new();
    while index > 0 {
        if index < captures.len() {
            return format!(
                "{}{remainder}",
                captures[index].as_deref().unwrap_or_default()
            );
        }
        remainder.insert_str(0, &(index % DECIMAL_RADIX).to_string());
        index /= DECIMAL_RADIX;
    }
    format!("${remainder}")
}

fn apply_case_ops(value: &str, operations: &[char]) -> String {
    if operations.is_empty() {
        return value.to_string();
    }
    let mut units = Vec::new();
    let mut operation = 0;
    for unit in value.encode_utf16() {
        let Some(current) = operations.get(operation) else {
            units.push(unit);
            continue;
        };
        if let Some(character) = char::from_u32(u32::from(unit)) {
            let transformed = match current {
                'U' | 'u' => character.to_uppercase().collect::<String>(),
                'L' | 'l' => character.to_lowercase().collect::<String>(),
                _ => character.to_string(),
            };
            units.extend(transformed.encode_utf16());
        } else {
            units.push(unit);
        }
        if matches!(current, 'u' | 'l') {
            operation += 1;
        }
    }
    String::from_utf16_lossy(&units)
}

pub fn preserve_replacement_case(matched: &str, pattern: &str) -> String {
    if matched.is_empty() {
        return pattern.to_string();
    }
    let same_parts = |separator| {
        matched.contains(separator)
            && pattern.contains(separator)
            && matched.split(separator).count() == pattern.split(separator).count()
    };
    let hyphens = same_parts('-');
    let underscores = same_parts('_');
    if hyphens != underscores {
        let separator = if hyphens { '-' } else { '_' };
        return matched
            .split(separator)
            .zip(pattern.split(separator))
            .map(|(matched, pattern)| preserve_replacement_case(matched, pattern))
            .collect::<Vec<_>>()
            .join(&separator.to_string());
    }
    if matched.to_uppercase() == matched {
        return pattern.to_uppercase();
    }
    if matched.to_lowercase() == matched {
        return pattern.to_lowercase();
    }
    let first = matched.chars().next().unwrap();
    if first.len_utf16() != 1 {
        return pattern.to_string();
    }
    let first_text = first.to_string();
    let upper = first_text.to_lowercase() != first_text;
    let lower = first_text.to_uppercase() != first_text;
    if !upper && !lower {
        return pattern.to_string();
    }
    let Some(first) = pattern.chars().next() else {
        return pattern.to_string();
    };
    let prefix = if first.len_utf16() != 1 {
        first.to_string()
    } else if upper {
        first.to_uppercase().collect::<String>()
    } else {
        first.to_lowercase().collect::<String>()
    };
    format!("{prefix}{}", &pattern[first.len_utf8()..])
}

pub fn replacement_edits(
    query: &FindQuery,
    document: &DocumentSnapshot,
    scopes: &[Range<usize>],
    pattern: &ReplacePattern,
    preserve_case: bool,
    timeout: Option<Duration>,
) -> Result<Vec<Edit>, FindPatternError> {
    let results = query.find_matches(&document.rope, scopes, usize::MAX, timeout)?;
    if results.timed_out {
        return Err(FindPatternError(
            "Search exceeded its time limit".to_string(),
        ));
    }
    Ok(results
        .matches
        .into_iter()
        .map(|found| Edit {
            bytes: found.range,
            text: normalize_line_breaks(
                &pattern.build(&found.captures, preserve_case),
                document.metadata.line_ending.as_str(),
            ),
        })
        .collect())
}

pub fn apply_replacement_edits(
    store: &mut EditorStore,
    view: ViewId,
    revision: u64,
    edits: Vec<Edit>,
) -> Result<bool, EditorError> {
    let current = store.views().get(view).ok_or(EditorError::NotFound)?;
    let document = store.documents().snapshot(current.document)?;
    if document.revision != revision {
        return Err(EditorError::StaleRevision);
    }
    if document.metadata.read_only {
        return Err(EditorError::ReadOnly);
    }
    if edits.is_empty() {
        return Ok(false);
    }
    apply_step(
        store,
        view,
        Some(Transaction {
            revision,
            edits,
            group: UndoGroup(revision),
            origin: Some(view),
            selection_after: None,
        }),
        SEPARATE_STEP,
    )
}
