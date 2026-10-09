use std::ops::Range;

use crate::change_journal::ChangesSince;
use crate::document::{DocumentId, DocumentSnapshot};
use crate::editing::line_text;
use crate::language_configuration::{BracketPair, LanguageRules};
use crate::line_breaks::{is_full_width_character, tab_columns};
use crate::line_tokens::{LineTokens, TokenStyleTable};
use crate::syntax::TokenKind;

const SPAN_FIELDS: usize = 2;
const FULL_WIDTH_COLUMNS: usize = 2;
const FIRST_SUPPLEMENTARY_CODE: u32 = 0x10000;

#[derive(Clone, Copy)]
pub struct BracketTokenData<'a> {
    pub lines: &'a LineTokens,
    pub styles: &'a TokenStyleTable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketInfo {
    pub bytes: Range<usize>,
    pub line: usize,
    pub column: usize,
    pub level: usize,
    pub invalid: bool,
    pub colorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketPairInfo {
    pub open: Range<usize>,
    pub close: Option<Range<usize>>,
    pub open_line: usize,
    pub close_line: usize,
    pub open_column: usize,
    pub close_column: usize,
    pub guide_column: usize,
    pub guide_level: usize,
    pub level: usize,
    pub has_text_before_close: bool,
    pub colorized: bool,
    pub end: usize,
    parent: Option<usize>,
    open_text: String,
    close_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketMatch {
    pub open: Range<usize>,
    pub close: Range<usize>,
    pub is_near: bool,
}

impl BracketPairInfo {
    pub fn strictly_contains(&self, byte: usize) -> bool {
        self.open.start < byte && byte < self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LineBracket {
    bytes: Range<usize>,
    column: usize,
    text: String,
}

#[derive(Debug, Clone)]
struct AnalyzedLine {
    length: usize,
    indent: Option<usize>,
    brackets: Vec<LineBracket>,
    spans: Option<Vec<u32>>,
    accurate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveIndentGuide {
    pub lines: RangeInclusiveLines,
    pub level: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeInclusiveLines {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Default)]
pub struct BracketModel {
    document: Option<DocumentId>,
    revision: u64,
    language_id: String,
    tab_size: u32,
    configured_pairs: Vec<BracketPair>,
    colorized_pairs: Vec<BracketPair>,
    off_side: bool,
    token_generation: Option<u64>,
    token_kinds: Vec<TokenKind>,
    lines: Vec<Option<AnalyzedLine>>,
    brackets: Vec<BracketInfo>,
    pairs: Vec<BracketPairInfo>,
    pair_interval_ends: Vec<usize>,
    pair_interval_size: usize,
    indent_levels: Vec<usize>,
    refresh_count: u64,
    analyzed_line_count: u64,
}

impl BracketModel {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn brackets(&self) -> &[BracketInfo] {
        &self.brackets
    }

    pub fn pairs(&self) -> &[BracketPairInfo] {
        &self.pairs
    }

    pub fn refresh_count(&self) -> u64 {
        self.refresh_count
    }

    pub fn analyzed_line_count(&self) -> u64 {
        self.analyzed_line_count
    }

    pub fn indent_level(&self, line: usize) -> usize {
        self.indent_levels.get(line).copied().unwrap_or_default()
    }

    pub fn brackets_in(&self, bytes: Range<usize>) -> &[BracketInfo] {
        let first = self
            .brackets
            .partition_point(|bracket| bracket.bytes.end <= bytes.start);
        let end = self
            .brackets
            .partition_point(|bracket| bracket.bytes.start < bytes.end);
        &self.brackets[first.min(end)..end]
    }

    pub fn pairs_in(&self, bytes: Range<usize>) -> Vec<&BracketPairInfo> {
        let end = self
            .pairs
            .partition_point(|pair| pair.open.start <= bytes.end);
        let mut result = Vec::new();
        if end == 0 {
            return result;
        }
        let mut stack = vec![(1, 0, self.pair_interval_size)];
        while let Some((node, first, after)) = stack.pop() {
            if first >= end || self.pair_interval_ends[node] < bytes.start {
                continue;
            }
            if after - first == 1 {
                result.push(&self.pairs[first]);
                continue;
            }
            let middle = (first + after) / 2;
            stack.push((node * 2 + 1, middle, after));
            stack.push((node * 2, first, middle));
        }
        result
    }

    pub fn active_pair(&self, byte: usize) -> Option<&BracketPairInfo> {
        self.pairs_in(byte..byte)
            .into_iter()
            .rev()
            .find(|pair| pair.strictly_contains(byte))
    }

    pub fn matching_brackets(&self, byte: usize) -> Option<BracketMatch> {
        let candidates = self.pairs_in(byte..byte);
        let near = candidates
            .iter()
            .filter_map(|pair| {
                let close = pair.close.as_ref()?;
                let range = if pair.open.start <= byte && byte <= pair.open.end {
                    &pair.open
                } else if close.start <= byte && byte <= close.end {
                    close
                } else {
                    return None;
                };
                Some((*pair, (range.start, range.end)))
            })
            .max_by_key(|(_, range)| *range)
            .map(|(pair, _)| pair);
        let pair = near.or_else(|| {
            candidates
                .into_iter()
                .rev()
                .find(|pair| pair.close.is_some() && pair.strictly_contains(byte))
        })?;
        Some(BracketMatch {
            open: pair.open.clone(),
            close: pair.close.clone()?,
            is_near: near.is_some(),
        })
    }

    pub fn active_indent(&self, line: usize, visible: Range<usize>) -> ActiveIndentGuide {
        let initial = self.indent_level(line);
        let mut result = ActiveIndentGuide {
            lines: RangeInclusiveLines {
                start: line,
                end: line,
            },
            level: initial,
        };
        let mut up = true;
        let mut down = true;
        let maximum = self.indent_levels.len().saturating_sub(1);
        for distance in 1..=self.indent_levels.len() {
            let above = line.checked_sub(distance);
            let below = line.checked_add(distance).filter(|below| *below <= maximum);
            if distance > 1 && above.is_none_or(|above| above < visible.start) {
                up = false;
            }
            if distance > 1 && below.is_none_or(|below| below >= visible.end) {
                down = false;
            }
            if distance == 1 {
                if let Some(below) = below
                    && self.indent_level(below) == initial + 1
                {
                    up = false;
                    result.lines = RangeInclusiveLines {
                        start: below,
                        end: below,
                    };
                    result.level = initial + 1;
                    continue;
                }
                if let Some(above) = above
                    && self.indent_level(above) == initial + 1
                {
                    down = false;
                    result.lines = RangeInclusiveLines {
                        start: above,
                        end: above,
                    };
                    result.level = initial + 1;
                    continue;
                }
                if initial == 0 {
                    return result;
                }
            }
            if up {
                match above {
                    Some(above) if self.indent_level(above) >= result.level => {
                        result.lines.start = above
                    }
                    _ => up = false,
                }
            }
            if down {
                match below {
                    Some(below) if self.indent_level(below) >= result.level => {
                        result.lines.end = below
                    }
                    _ => down = false,
                }
            }
            if !up && !down {
                break;
            }
        }
        result
    }

    pub(crate) fn is_current(
        &self,
        document: &DocumentSnapshot,
        rules: Option<&dyn LanguageRules>,
        tab_size: u32,
        tokens: Option<BracketTokenData<'_>>,
    ) -> bool {
        let (configured, colorized) = configured_brackets(rules);
        self.document == Some(document.id)
            && self.revision == document.revision
            && self.language_id == document.metadata.language_id
            && self.tab_size == tab_size.max(1)
            && self.configured_pairs == configured
            && self.colorized_pairs == colorized
            && self.off_side == rules.is_some_and(|rules| rules.is_off_side())
            && self.token_generation == tokens.map(|tokens| tokens.lines.generation())
            && self.token_kinds == token_kinds(tokens)
    }

    pub fn refresh(
        &mut self,
        document: &DocumentSnapshot,
        rules: Option<&dyn LanguageRules>,
        tab_size: u32,
        tokens: Option<BracketTokenData<'_>>,
        changes: ChangesSince<'_>,
    ) {
        let tab_size = tab_size.max(1);
        let (configured_pairs, colorized_pairs) = configured_brackets(rules);
        let off_side = rules.is_some_and(|rules| rules.is_off_side());
        let kinds = token_kinds(tokens);
        let generation = tokens.map(|tokens| tokens.lines.generation());
        let reset = self.document != Some(document.id)
            || self.language_id != document.metadata.language_id
            || self.tab_size != tab_size
            || self.configured_pairs != configured_pairs
            || self.colorized_pairs != colorized_pairs
            || self.off_side != off_side;
        if !reset
            && self.revision == document.revision
            && self.token_generation == generation
            && self.token_kinds == kinds
        {
            return;
        }
        let kinds_changed = self.token_kinds != kinds;
        if reset {
            self.lines = vec![None; document.rope.len_lines()];
        } else if self.revision != document.revision {
            self.apply_changes(document, changes);
        }
        self.document = Some(document.id);
        self.revision = document.revision;
        self.language_id = document.metadata.language_id.clone();
        self.tab_size = tab_size;
        self.configured_pairs = configured_pairs;
        self.colorized_pairs = colorized_pairs;
        self.off_side = off_side;
        self.token_generation = generation;
        self.token_kinds = kinds;
        for line in 0..self.lines.len() {
            let accurate = tokens.is_none_or(|tokens| tokens.lines.has_accurate_tokens(line));
            let spans = tokens.map(|tokens| tokens.lines.spans(line));
            if !kinds_changed
                && self.lines[line].as_ref().is_some_and(|known| {
                    known.accurate == accurate && known.spans.as_deref() == spans
                })
            {
                continue;
            }
            let content = line_text(document, line);
            let mut brackets = Vec::new();
            let indent = indentation(&content.text, tab_size);
            let mut column = 0;
            let mut offset = 0;
            if accurate && let Some(rules) = rules {
                for bytes in rules.bracket_ranges(&content.text) {
                    if bytes.is_empty()
                        || bytes.start < offset
                        || bytes.end > content.text.len()
                        || !content.text.is_char_boundary(bytes.start)
                        || !content.text.is_char_boundary(bytes.end)
                    {
                        continue;
                    }
                    column = visible_column(&content.text[offset..bytes.start], tab_size, column);
                    offset = bytes.start;
                    if !code_range(tokens, line, &bytes) {
                        continue;
                    }
                    brackets.push(LineBracket {
                        bytes: bytes.clone(),
                        column,
                        text: content.text[bytes].to_lowercase(),
                    });
                }
            }
            self.lines[line] = Some(AnalyzedLine {
                length: document.rope.line(line).len_bytes(),
                indent,
                brackets,
                spans: spans.map(<[u32]>::to_vec),
                accurate,
            });
            self.analyzed_line_count += 1;
        }
        self.rebuild(document.rope.len_bytes());
        self.refresh_count += 1;
    }

    fn apply_changes(&mut self, document: &DocumentSnapshot, changes: ChangesSince<'_>) {
        let ChangesSince::Tracked(changes) = changes else {
            self.lines = vec![None; document.rope.len_lines()];
            return;
        };
        for change in changes {
            if self.lines.len() != change.line_count_before {
                self.lines = vec![None; document.rope.len_lines()];
                return;
            }
            for span in change.spans.iter().rev() {
                let count = span.new_end.line - span.start.line + 1;
                self.lines.splice(
                    span.start.line..span.old_end.line + 1,
                    std::iter::repeat_n(None, count),
                );
            }
        }
        if self.lines.len() != document.rope.len_lines() {
            self.lines = vec![None; document.rope.len_lines()];
        }
    }

    fn rebuild(&mut self, document_end: usize) {
        self.brackets.clear();
        self.pairs.clear();
        self.pair_interval_ends.clear();
        self.resolve_indents();
        let minima = IndentMinima::new(&self.lines);
        let mut stack = Vec::<usize>::new();
        let mut associations = Vec::<Option<usize>>::new();
        let mut start = 0;
        for (line_number, line) in self.lines.iter().enumerate() {
            let Some(line) = line else { continue };
            for bracket in &line.brackets {
                let bytes = start + bracket.bytes.start..start + bracket.bytes.end;
                let is_close = self
                    .configured_pairs
                    .iter()
                    .any(|pair| pair.close.eq_ignore_ascii_case(&bracket.text));
                let matching = is_close
                    .then(|| {
                        stack.iter().rposition(|index| {
                            let open = &self.pairs[*index].open_text;
                            self.configured_pairs.iter().any(|pair| {
                                pair.open.eq_ignore_ascii_case(open)
                                    && pair.close.eq_ignore_ascii_case(&bracket.text)
                            })
                        })
                    })
                    .flatten();
                let association = if let Some(matching) = matching {
                    for incomplete in stack.drain(matching + 1..) {
                        self.pairs[incomplete].end = bytes.start;
                    }
                    let index = stack.pop().unwrap();
                    let pair = &mut self.pairs[index];
                    pair.close = Some(bytes.clone());
                    pair.end = bytes.end;
                    pair.close_text = Some(bracket.text.clone());
                    pair.close_line = line_number;
                    pair.close_column = bracket.column;
                    pair.has_text_before_close =
                        line.indent.is_some_and(|indent| indent < bracket.column);
                    pair.guide_column = pair
                        .open_column
                        .min(bracket.column)
                        .min(minima.query(pair.open_line + 1..line_number + 1));
                    Some(index)
                } else if !is_close
                    && self
                        .configured_pairs
                        .iter()
                        .any(|pair| pair.open.eq_ignore_ascii_case(&bracket.text))
                {
                    let index = self.pairs.len();
                    self.pairs.push(BracketPairInfo {
                        open: bytes.clone(),
                        close: None,
                        open_line: line_number,
                        close_line: line_number,
                        open_column: bracket.column,
                        close_column: bracket.column,
                        guide_column: bracket.column,
                        guide_level: stack.len(),
                        level: 0,
                        has_text_before_close: false,
                        colorized: true,
                        end: document_end,
                        parent: stack.last().copied(),
                        open_text: bracket.text.clone(),
                        close_text: None,
                    });
                    stack.push(index);
                    Some(index)
                } else {
                    None
                };
                self.brackets.push(BracketInfo {
                    bytes,
                    line: line_number,
                    column: bracket.column,
                    level: 0,
                    invalid: association.is_none(),
                    colorized: true,
                });
                associations.push(association);
            }
            start += line.length;
        }
        for index in 0..self.pairs.len() {
            let (parent, open, close) = {
                let pair = &self.pairs[index];
                (pair.parent, pair.open_text.clone(), pair.close_text.clone())
            };
            let level = parent.map_or(0, |parent| {
                self.pairs[parent].level + usize::from(self.pairs[parent].colorized)
            });
            let colorized = close.as_ref().is_none_or(|close| {
                self.colorized_pairs.iter().any(|pair| {
                    pair.open.eq_ignore_ascii_case(&open) && pair.close.eq_ignore_ascii_case(close)
                })
            });
            self.pairs[index].level = level;
            self.pairs[index].colorized = colorized;
        }
        self.pair_interval_size = self.pairs.len().max(1).next_power_of_two();
        self.pair_interval_ends = vec![0; self.pair_interval_size * 2];
        for (index, pair) in self.pairs.iter().enumerate() {
            self.pair_interval_ends[self.pair_interval_size + index] = pair.end;
        }
        for index in (1..self.pair_interval_size).rev() {
            self.pair_interval_ends[index] =
                self.pair_interval_ends[index * 2].max(self.pair_interval_ends[index * 2 + 1]);
        }
        let tokens_complete = self
            .lines
            .iter()
            .all(|line| line.as_ref().is_some_and(|line| line.accurate));
        for (bracket, association) in self.brackets.iter_mut().zip(associations) {
            if let Some(index) = association {
                let pair = &self.pairs[index];
                bracket.level = pair.level;
                bracket.colorized = pair.colorized;
                bracket.invalid =
                    pair.close.is_none() && (tokens_complete || pair.end < document_end);
            }
        }
    }

    fn resolve_indents(&mut self) {
        let mut below = vec![None; self.lines.len()];
        let mut next = None;
        for (index, line) in self.lines.iter().enumerate().rev() {
            below[index] = next;
            if let Some(indent) = line.as_ref().and_then(|line| line.indent) {
                next = Some(indent);
            }
        }
        let mut above = None;
        self.indent_levels.clear();
        for (index, line) in self.lines.iter().enumerate() {
            let indent = line.as_ref().and_then(|line| line.indent);
            let level = match (indent, above, below[index]) {
                (Some(indent), _, _) => indent.div_ceil(self.tab_size as usize),
                (None, Some(above), Some(below)) if above < below => {
                    1 + above / self.tab_size as usize
                }
                (None, Some(above), Some(below)) if above == below || self.off_side => {
                    below.div_ceil(self.tab_size as usize)
                }
                (None, Some(_), Some(below)) => 1 + below / self.tab_size as usize,
                _ => 0,
            };
            self.indent_levels.push(level);
            if indent.is_some() {
                above = indent;
            }
        }
    }
}

fn configured_brackets(rules: Option<&dyn LanguageRules>) -> (Vec<BracketPair>, Vec<BracketPair>) {
    let mut configured = rules.map_or_else(Vec::new, |rules| rules.pairs().brackets.clone());
    configured.retain(|pair| !pair.open.is_empty() && !pair.close.is_empty());
    let colorized = rules
        .and_then(|rules| rules.colorized_brackets())
        .map_or_else(
            || {
                configured
                    .iter()
                    .filter(|pair| !(pair.open == "<" && pair.close == ">"))
                    .cloned()
                    .collect::<Vec<_>>()
            },
            <[BracketPair]>::to_vec,
        );
    for pair in &colorized {
        if !configured.contains(pair) {
            configured.push(pair.clone());
        }
    }
    (configured, colorized)
}

fn token_kinds(tokens: Option<BracketTokenData<'_>>) -> Vec<TokenKind> {
    tokens.map_or_else(Vec::new, |tokens| {
        std::iter::once(tokens.styles.default_style().kind)
            .chain((0..tokens.styles.len()).map(|index| tokens.styles.style(index as u32).kind))
            .collect()
    })
}

fn code_range(tokens: Option<BracketTokenData<'_>>, line: usize, bytes: &Range<usize>) -> bool {
    let Some(tokens) = tokens else { return true };
    let spans = tokens.lines.spans(line).as_chunks::<SPAN_FIELDS>().0;
    let first = spans
        .partition_point(|[start, _]| (*start as usize) <= bytes.start)
        .saturating_sub(1);
    if spans.is_empty() {
        return tokens.styles.default_style().kind == TokenKind::Other;
    }
    spans[first..]
        .iter()
        .take_while(|[start, _]| (*start as usize) < bytes.end)
        .all(|[_, style]| tokens.styles.style(*style).kind == TokenKind::Other)
}

pub fn visible_column(text: &str, tab_size: u32, start: usize) -> usize {
    text.chars().fold(start, |column, character| {
        if character == '\t' {
            return column + tab_columns(column as f64, tab_size.max(1)) as usize;
        }
        column
            + if u32::from(character) >= FIRST_SUPPLEMENTARY_CODE
                || is_full_width_character(character)
            {
                FULL_WIDTH_COLUMNS
            } else {
                1
            }
    })
}

fn indentation(text: &str, tab_size: u32) -> Option<usize> {
    let first = text.find(|character| character != ' ' && character != '\t')?;
    Some(visible_column(&text[..first], tab_size, 0))
}

struct IndentMinima {
    size: usize,
    values: Vec<usize>,
}

impl IndentMinima {
    fn new(lines: &[Option<AnalyzedLine>]) -> Self {
        let size = lines.len().max(1).next_power_of_two();
        let mut values = vec![usize::MAX; size * 2];
        for (index, line) in lines.iter().enumerate() {
            values[size + index] = line
                .as_ref()
                .and_then(|line| line.indent)
                .unwrap_or(usize::MAX);
        }
        for index in (1..size).rev() {
            values[index] = values[index * 2].min(values[index * 2 + 1]);
        }
        Self { size, values }
    }

    fn query(&self, lines: Range<usize>) -> usize {
        let mut start = self.size + lines.start;
        let mut end = self.size + lines.end;
        let mut result = usize::MAX;
        while start < end {
            if start % 2 == 1 {
                result = result.min(self.values[start]);
                start += 1;
            }
            if end % 2 == 1 {
                end -= 1;
                result = result.min(self.values[end]);
            }
            start /= 2;
            end /= 2;
        }
        result
    }
}
