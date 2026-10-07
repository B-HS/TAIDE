use std::borrow::Cow;
use std::ops::Range;

use crate::decoration::{Decoration, DecorationKind, DecorationLayer, InlineStyle, Stickiness};
use crate::document::{DocumentId, DocumentSnapshot, EditorError};
use crate::editing::{CharacterClass, character_class, line_text, ordered, view_document};
use crate::language_configuration::{AutoClosingPair, Language, is_quote, token_kind_at};
use crate::store::EditorStore;
use crate::view::{Selection, ViewId};

const TRACKED_RANGE_Z_ORDER: u8 = 0;
const SINGLE_CHARACTER: usize = 1;
const ESCAPE_CHARACTER: char = '\\';
const ESCAPABLE_PREFIX_UTF16_LENGTH: usize = 2;
const WORD_ADJACENT_QUOTES: [char; 2] = ['\'', '"'];
const BLANK_SELECTION_CHARACTERS: [char; 4] = [' ', '\t', '\n', '\r'];

#[derive(Debug, Clone)]
struct AutoClosedAction {
    closing: DecorationLayer,
    enclosing: DecorationLayer,
}

fn tracked_ranges(revision: u64, ranges: Vec<Range<usize>>) -> DecorationLayer {
    DecorationLayer::new(
        revision,
        TRACKED_RANGE_Z_ORDER,
        ranges
            .into_iter()
            .map(|bytes| Decoration {
                bytes,
                kind: DecorationKind::Inline(InlineStyle::default()),
                stickiness: Stickiness::NeverGrowsWhenTypingAtEdges,
            })
            .collect(),
    )
}

impl AutoClosedAction {
    fn catch_up(&mut self, store: &EditorStore, document: &DocumentSnapshot) -> bool {
        for layer in [&mut self.closing, &mut self.enclosing] {
            let Ok(changes) = store.changes_since(document.id, layer.revision()) else {
                return false;
            };
            let Some(tracked) = layer.tracking(changes).map(Cow::into_owned) else {
                return false;
            };
            *layer = tracked;
        }
        true
    }

    fn is_valid(&self, document: &DocumentSnapshot, selections: &[Range<usize>]) -> bool {
        let rope = &document.rope;
        let length = rope.len_bytes();
        let mut enclosing: Vec<Range<usize>> = self
            .enclosing
            .items()
            .iter()
            .map(|item| item.bytes.clone())
            .collect();
        let spans_lines = |range: &Range<usize>| {
            rope.byte_to_line(range.start.min(length)) != rope.byte_to_line(range.end.min(length))
        };
        if enclosing.iter().any(spans_lines) {
            return false;
        }
        enclosing.sort_by_key(|range| (range.start, range.end));
        selections.iter().enumerate().all(|(index, selection)| {
            enclosing
                .get(index)
                .is_some_and(|range| range.start < selection.start && selection.end < range.end)
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct AutoClosedPairs {
    document: Option<DocumentId>,
    actions: Vec<AutoClosedAction>,
}

impl AutoClosedPairs {
    pub fn follow(&mut self, store: &EditorStore, view: ViewId) -> Result<(), EditorError> {
        let (current, document) = view_document(store, view)?;
        if self.document != Some(document.id) {
            self.actions.clear();
            self.document = Some(document.id);
        }
        if self.actions.is_empty() {
            return Ok(());
        }
        let mut selections: Vec<Range<usize>> =
            current.selection.selections.iter().map(ordered).collect();
        selections.sort_by_key(|range| (range.start, range.end));
        self.actions.retain_mut(|action| {
            action.catch_up(store, &document) && action.is_valid(&document, &selections)
        });
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub(crate) fn record(
        &mut self,
        document: &DocumentSnapshot,
        closing: Vec<Range<usize>>,
        enclosing: Vec<Range<usize>>,
    ) {
        if self.document != Some(document.id) {
            self.actions.clear();
            self.document = Some(document.id);
        }
        self.actions.push(AutoClosedAction {
            closing: tracked_ranges(document.revision, closing),
            enclosing: tracked_ranges(document.revision, enclosing),
        });
    }

    fn closes_at(&self, offset: usize) -> bool {
        self.actions.iter().any(|action| {
            action
                .closing
                .items()
                .iter()
                .any(|item| item.bytes.start == offset)
        })
    }
}

struct Caret {
    line: usize,
    before: String,
    after: String,
}

fn caret(document: &DocumentSnapshot, selection: &Selection) -> Option<Caret> {
    if selection.anchor != selection.head {
        return None;
    }
    let line = document.rope.byte_to_line(selection.head);
    let content = line_text(document, line);
    let column = (selection.head - content.start).min(content.text.len());
    let (before, after) = content.text.split_at(column);
    Some(Caret {
        line,
        before: before.into(),
        after: after.into(),
    })
}

fn carets(document: &DocumentSnapshot, selections: &[Selection]) -> Option<Vec<Caret>> {
    selections
        .iter()
        .map(|selection| caret(document, selection))
        .collect()
}

fn is_single_character(text: &str) -> bool {
    text.chars().count() == SINGLE_CHARACTER
}

fn longer<'a>(
    best: Option<&'a AutoClosingPair>,
    candidate: &'a AutoClosingPair,
) -> Option<&'a AutoClosingPair> {
    if best.is_some_and(|best| candidate.open.len() <= best.open.len()) {
        best
    } else {
        Some(candidate)
    }
}

pub(crate) fn closing_text(
    document: &DocumentSnapshot,
    selections: &[Selection],
    character: char,
    language: Language<'_>,
) -> Option<String> {
    let pairs = language.rules.pairs();
    let opens_pair = |candidate: &&AutoClosingPair| candidate.open.ends_with(character);
    if !pairs
        .auto_closing_pairs
        .iter()
        .any(|pair| opens_pair(&pair))
    {
        return None;
    }
    let carets = carets(document, selections)?;
    let pair = pairs
        .auto_closing_pairs
        .iter()
        .filter(opens_pair)
        .filter(|candidate| {
            let prefix = &candidate.open[..candidate.open.len() - character.len_utf8()];
            carets.iter().all(|caret| caret.before.ends_with(prefix))
        })
        .fold(None, longer)?;
    let auto_close_before = if is_quote(character) {
        &pairs.auto_close_before_quotes
    } else {
        &pairs.auto_close_before_brackets
    };
    let contained_close = if is_single_character(&pair.open) || pair.open.is_empty() {
        ""
    } else {
        pairs
            .auto_closing_pairs
            .iter()
            .filter(|candidate| candidate.close.chars().last() == pair.close.chars().last())
            .filter(|candidate| {
                candidate.open != pair.open
                    && pair.open.contains(&candidate.open)
                    && pair.close.ends_with(&candidate.close)
            })
            .fold(None, longer)
            .map_or("", |candidate| candidate.close.as_str())
    };
    let mut is_contained_pair_present = true;
    for caret in &carets {
        if !caret.after.starts_with(contained_close) {
            is_contained_pair_present = false;
        }
        if let Some(next) = caret.after.chars().next() {
            let starts_with = |text: &str| !text.is_empty() && caret.after.starts_with(text);
            let is_before_opening = pairs
                .auto_closing_pairs
                .iter()
                .any(|pair| starts_with(&pair.open));
            let is_before_closing = pairs
                .auto_closing_pairs
                .iter()
                .any(|pair| starts_with(&pair.close));
            if (is_before_opening || !is_before_closing) && !auto_close_before.contains(next) {
                return None;
            }
        }
        if is_single_character(&pair.open)
            && WORD_ADJACENT_QUOTES.contains(&character)
            && caret
                .before
                .chars()
                .last()
                .is_some_and(|previous| character_class(previous) == CharacterClass::Regular)
        {
            return None;
        }
        let tokens = language.syntax.tokens(document, caret.line)?;
        if !tokens.is_empty() {
            let kind = token_kind_at(&tokens, caret.before.len().saturating_sub(1));
            if !pair.allows(kind) {
                return None;
            }
        }
        if let Some(neutral) = pair.neutral_character() {
            let kind = language.syntax.kind_if_inserting(
                document,
                caret.line,
                caret.before.len(),
                neutral,
            );
            if !pair.allows(kind) {
                return None;
            }
        }
    }
    Some(if is_contained_pair_present {
        pair.close[..pair.close.len() - contained_close.len()].into()
    } else {
        pair.close.clone()
    })
}

pub(crate) fn overtypes(
    document: &DocumentSnapshot,
    selections: &[Selection],
    character: char,
    language: Language<'_>,
    auto_closed: &AutoClosedPairs,
) -> bool {
    let closes_single_pair = language
        .rules
        .pairs()
        .auto_closing_pairs
        .iter()
        .any(|pair| pair.close.chars().eq([character]) && is_single_character(&pair.open));
    closes_single_pair
        && selections.iter().all(|selection| {
            caret(document, selection).is_some_and(|caret| {
                let is_escaped_quote = is_quote(character)
                    && caret.before.ends_with(ESCAPE_CHARACTER)
                    && caret.before.encode_utf16().count() >= ESCAPABLE_PREFIX_UTF16_LENGTH;
                caret.after.starts_with(character)
                    && !is_escaped_quote
                    && auto_closed.closes_at(selection.head)
            })
        })
}

pub(crate) fn pair_deletions(
    document: &DocumentSnapshot,
    selections: &[Selection],
    language: Language<'_>,
    auto_closed: &AutoClosedPairs,
) -> Option<Vec<Range<usize>>> {
    let pairs = language.rules.pairs();
    selections
        .iter()
        .map(|selection| {
            let caret = caret(document, selection)?;
            let previous = caret.before.chars().last()?;
            let next = caret.after.chars().next()?;
            let is_pair = pairs
                .auto_closing_pairs
                .iter()
                .any(|pair| pair.open.chars().eq([previous]) && pair.close.chars().eq([next]));
            (is_pair && auto_closed.closes_at(selection.head))
                .then(|| selection.head - previous.len_utf8()..selection.head + next.len_utf8())
        })
        .collect()
}

pub(crate) fn surrounding_close<'a>(
    document: &DocumentSnapshot,
    selections: &[Selection],
    character: char,
    language: Language<'a>,
) -> Option<&'a str> {
    let close = language.rules.pairs().surrounding_close(character)?;
    let surrounds = selections.iter().all(|selection| {
        let range = ordered(selection);
        if range.is_empty() {
            return false;
        }
        let text = document.rope.byte_slice(range).to_string();
        let is_blank = text
            .chars()
            .all(|selected| BLANK_SELECTION_CHARACTERS.contains(&selected));
        let is_quote_over_quote =
            is_quote(character) && is_single_character(&text) && text.chars().all(is_quote);
        !is_blank && !is_quote_over_quote
    });
    surrounds.then_some(close)
}
