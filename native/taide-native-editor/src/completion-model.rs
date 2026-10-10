use lsp_types::CompletionItemKind;

use crate::completion::Candidate;
use crate::completion_filter::{Score, ScoreOptions, Scorer};
use crate::document::{DocumentSnapshot, EditorError, byte_to_char};

const GRACEFUL_SOURCE_LIMIT: usize = 2000;
const DEFAULT_SCORE: i32 = -100;
const MONACO_KIND_ORDER: [CompletionItemKind; 25] = [
    CompletionItemKind::METHOD,
    CompletionItemKind::FUNCTION,
    CompletionItemKind::CONSTRUCTOR,
    CompletionItemKind::FIELD,
    CompletionItemKind::VARIABLE,
    CompletionItemKind::CLASS,
    CompletionItemKind::STRUCT,
    CompletionItemKind::INTERFACE,
    CompletionItemKind::MODULE,
    CompletionItemKind::PROPERTY,
    CompletionItemKind::EVENT,
    CompletionItemKind::OPERATOR,
    CompletionItemKind::UNIT,
    CompletionItemKind::VALUE,
    CompletionItemKind::CONSTANT,
    CompletionItemKind::ENUM,
    CompletionItemKind::ENUM_MEMBER,
    CompletionItemKind::KEYWORD,
    CompletionItemKind::TEXT,
    CompletionItemKind::COLOR,
    CompletionItemKind::FILE,
    CompletionItemKind::REFERENCE,
    CompletionItemKind::FOLDER,
    CompletionItemKind::TYPE_PARAMETER,
    CompletionItemKind::SNIPPET,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked {
    pub candidate: usize,
    pub score: Score,
}

struct Item {
    candidate: Candidate,
    overwrite_before: usize,
    label: Vec<u16>,
    sort: Vec<u16>,
    kind: usize,
}

pub struct Model {
    items: Vec<Item>,
    ranked: Vec<Ranked>,
    scorer: Scorer,
    leading: Option<String>,
    delta: isize,
}

impl Model {
    pub fn new(
        document: &DocumentSnapshot,
        candidates: Vec<Candidate>,
    ) -> Result<Self, EditorError> {
        let has_sort_text = candidates.iter().any(|candidate| {
            candidate
                .item
                .sort_text
                .as_deref()
                .is_some_and(|text| !text.is_empty())
        });
        let mut items = candidates
            .into_iter()
            .map(|candidate| {
                if candidate.document != document.id || candidate.revision != document.revision {
                    return Err(EditorError::StaleRevision);
                }
                byte_to_char(&document.rope, candidate.insert.start)?;
                byte_to_char(&document.rope, candidate.requested_byte)?;
                if candidate.insert.start > candidate.requested_byte {
                    return Err(EditorError::InvalidBoundary);
                }
                let overwrite_before = document
                    .rope
                    .byte_slice(candidate.insert.start..candidate.requested_byte)
                    .len_utf16_cu();
                let sort_text = if has_sort_text {
                    candidate
                        .item
                        .sort_text
                        .as_deref()
                        .filter(|text| !text.is_empty())
                        .unwrap_or(&candidate.item.label)
                        .to_lowercase()
                } else {
                    candidate.item.label.clone()
                };
                Ok(Item {
                    label: candidate.item.label.encode_utf16().collect(),
                    sort: sort_text.encode_utf16().collect(),
                    kind: MONACO_KIND_ORDER
                        .iter()
                        .position(|kind| Some(*kind) == candidate.item.kind)
                        .or_else(|| {
                            MONACO_KIND_ORDER
                                .iter()
                                .position(|kind| *kind == CompletionItemKind::TEXT)
                        })
                        .unwrap(),
                    candidate,
                    overwrite_before,
                })
            })
            .collect::<Result<Vec<_>, EditorError>>()?;
        items.sort_by(|left, right| {
            left.sort
                .cmp(&right.sort)
                .then_with(|| left.label.cmp(&right.label))
                .then(left.kind.cmp(&right.kind))
        });
        Ok(Self {
            items,
            ranked: Vec::new(),
            scorer: Scorer::default(),
            leading: None,
            delta: 0,
        })
    }

    pub fn candidate(&self, index: usize) -> Option<&Candidate> {
        self.items.get(index).map(|item| &item.candidate)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn filter(&mut self, leading: &str, delta: isize) -> &[Ranked] {
        if self.leading.as_deref() == Some(leading) && self.delta == delta {
            return &self.ranked;
        }
        let source = if self.leading.is_some() && self.delta < delta {
            self.ranked
                .iter()
                .map(|item| item.candidate)
                .collect::<Vec<_>>()
        } else {
            (0..self.items.len()).collect()
        };
        let graceful = source.len() <= GRACEFUL_SOURCE_LIMIT;
        let leading_units = leading.encode_utf16().collect::<Vec<_>>();
        let mut ranked = Vec::new();
        let mut previous_word = None;
        let mut word = String::new();
        for index in source {
            let item = &self.items[index];
            let word_len = item.overwrite_before.saturating_add_signed(delta);
            if previous_word != Some(word_len) {
                let Ok(pattern) = String::from_utf16(
                    &leading_units[leading_units.len().saturating_sub(word_len)..],
                ) else {
                    continue;
                };
                word = pattern;
                previous_word = Some(word_len);
            }
            let word_pos = word
                .encode_utf16()
                .take(item.overwrite_before)
                .take_while(|character| {
                    *character == u16::from(b' ') || *character == u16::from(b'\t')
                })
                .count();
            let score = if word_len == 0 || word_pos >= word_len {
                Some(Score {
                    value: DEFAULT_SCORE,
                    word_start: 0,
                    positions: Vec::new(),
                })
            } else {
                let filter = item
                    .candidate
                    .item
                    .filter_text
                    .as_deref()
                    .unwrap_or(&item.candidate.item.label);
                let score = if graceful {
                    self.scorer
                        .graceful(&word, word_pos, filter, 0, ScoreOptions::default())
                } else {
                    self.scorer
                        .score(&word, word_pos, filter, 0, ScoreOptions::default())
                };
                score.map(|score| {
                    if filter.to_lowercase() == item.candidate.item.label.to_lowercase() {
                        return score;
                    }
                    let mut highlighted =
                        self.scorer
                            .any(&word, word_pos, &item.candidate.item.label, 0);
                    highlighted.value = score.value;
                    highlighted
                })
            };
            if let Some(score) = score {
                ranked.push(Ranked {
                    candidate: index,
                    score,
                });
            }
        }
        ranked.sort_by(|left, right| right.score.value.cmp(&left.score.value));
        self.ranked = ranked;
        self.leading = Some(leading.to_owned());
        self.delta = delta;
        &self.ranked
    }
}
