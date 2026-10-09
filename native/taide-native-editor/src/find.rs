use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::time::{Duration, Instant};

use ropey::Rope;

use crate::cursor_commands::word_range;
use crate::document::{DocumentSnapshot, byte_to_char};
use crate::editing::{WORD_SEPARATORS, rope_line_content_range};
use crate::language_configuration::LanguageRules;
use crate::view::SelectionSet;

pub const FIND_MATCH_LIMIT: usize = 19_999;
pub const FIND_SEARCH_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_SEED_UTF16_LENGTH: usize = 524_288;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FindSeedOptions {
    pub is_regex: bool,
    pub allow_multiline: bool,
    pub require_selection: bool,
}

pub fn seed_find_text(
    document: &DocumentSnapshot,
    selection: &SelectionSet,
    rules: Option<&dyn LanguageRules>,
    options: FindSeedOptions,
) -> Option<String> {
    let selected = selection.selections.get(selection.primary)?;
    let range = selected.anchor.min(selected.head)..selected.anchor.max(selected.head);
    validate_range(&document.rope, &range).ok()?;
    let range = if range.is_empty() {
        if options.require_selection {
            return None;
        }
        word_range(document, selected.head, rules)?
    } else {
        range
    };
    validate_range(&document.rope, &range).ok()?;
    let slice = document.rope.byte_slice(range);
    if slice
        .chars()
        .take(MAX_SEED_UTF16_LENGTH)
        .map(char::len_utf16)
        .sum::<usize>()
        >= MAX_SEED_UTF16_LENGTH
    {
        return None;
    }
    let text = slice.to_string();
    if !options.allow_multiline && text.contains(['\r', '\n']) {
        return None;
    }
    if options.is_regex {
        return Some(escape_find_literal(&text));
    }
    Some(text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindPatternError(pub String);

impl fmt::Display for FindPatternError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for FindPatternError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindPatternOptions {
    pub is_regex: bool,
    pub match_case: bool,
    pub multiline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindCaptures {
    pub groups: Vec<Option<Range<usize>>>,
}

pub trait FindPattern {
    fn captures_at(
        &self,
        text: &str,
        start: usize,
    ) -> Result<Option<FindCaptures>, FindPatternError>;
}

pub trait FindPatternCompiler {
    fn compile(
        &self,
        source: &str,
        options: FindPatternOptions,
    ) -> Result<Box<dyn FindPattern>, FindPatternError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindOptions {
    pub is_regex: bool,
    pub match_case: bool,
    pub whole_word: bool,
    pub word_separators: String,
}

impl Default for FindOptions {
    fn default() -> Self {
        Self {
            is_regex: false,
            match_case: false,
            whole_word: false,
            word_separators: WORD_SEPARATORS.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindMatch {
    pub range: Range<usize>,
    pub captures: Vec<Option<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FindResults {
    pub matches: Vec<FindMatch>,
    pub limit_reached: bool,
    pub timed_out: bool,
}

enum Pattern {
    Empty,
    Literal(String),
    Compiled(Box<dyn FindPattern>),
}

pub struct FindQuery {
    options: FindOptions,
    multiline: bool,
    pattern: Pattern,
}

impl FindQuery {
    pub fn new(
        source: &str,
        options: FindOptions,
        compiler: &dyn FindPatternCompiler,
    ) -> Result<Self, FindPatternError> {
        let multiline = if options.is_regex {
            is_multiline_regex_source(source)
        } else {
            source.contains('\n')
        };
        let pattern = if source.is_empty() {
            Pattern::Empty
        } else if !options.is_regex && options.match_case {
            Pattern::Literal(source.to_string())
        } else {
            Pattern::Compiled(compiler.compile(
                source,
                FindPatternOptions {
                    is_regex: options.is_regex,
                    match_case: options.match_case,
                    multiline,
                },
            )?)
        };
        Ok(Self {
            options,
            multiline,
            pattern,
        })
    }

    pub fn find_matches(
        &self,
        rope: &Rope,
        scopes: &[Range<usize>],
        limit: usize,
        timeout: Option<Duration>,
    ) -> Result<FindResults, FindPatternError> {
        let mut results = FindResults::default();
        if matches!(self.pattern, Pattern::Empty) || limit == 0 {
            return Ok(results);
        }
        let budget = SearchBudget::new(timeout);
        for scope in search_scopes(rope, scopes)? {
            if budget.expired() {
                results.timed_out = true;
                break;
            }
            if self.multiline {
                let text = NormalizedText::new(rope, scope.clone());
                self.collect(
                    &text.text,
                    0,
                    |offset| scope.start + text.original_offset(offset),
                    limit,
                    &budget,
                    &mut results,
                )?;
            } else {
                let first = rope.byte_to_line(scope.start);
                let last = rope.byte_to_line(scope.end);
                let mut line_start = rope.line_to_byte(first);
                for slice in rope.lines_at(first).take(last - first + 1) {
                    if budget.expired() {
                        results.timed_out = true;
                        break;
                    }
                    let line_bytes = slice.len_bytes();
                    let text = Cow::<str>::from(slice);
                    let content = text.strip_suffix('\n').unwrap_or(&text);
                    let content = content.strip_suffix('\r').unwrap_or(content);
                    let start = scope.start.saturating_sub(line_start).min(content.len());
                    let end = scope
                        .end
                        .saturating_sub(line_start)
                        .min(content.len())
                        .max(start);
                    self.collect(
                        &content[start..end],
                        0,
                        |offset| line_start + start + offset,
                        limit,
                        &budget,
                        &mut results,
                    )?;
                    if results.limit_reached || results.timed_out {
                        break;
                    }
                    line_start += line_bytes;
                }
            }
            if results.limit_reached || results.timed_out {
                break;
            }
        }
        Ok(results)
    }

    pub fn find_next(
        &self,
        rope: &Rope,
        start: usize,
        wrap: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<FindMatch>, FindPatternError> {
        validate_range(rope, &(start..start))?;
        if matches!(self.pattern, Pattern::Empty) {
            return Ok(None);
        }
        let budget = SearchBudget::new(timeout);
        let first = rope.byte_to_line(start);
        if self.multiline {
            let range = rope.line_to_byte(first)..rope.len_bytes();
            let text = NormalizedText::new(rope, range.clone());
            let offset = text.normalized_offset(start - range.start);
            let mut results = FindResults::default();
            self.collect(
                &text.text,
                offset,
                |offset| range.start + text.original_offset(offset),
                1,
                &budget,
                &mut results,
            )?;
            if results.timed_out {
                return Err(timeout_error());
            }
            if let Some(found) = results.matches.pop() {
                return Ok(Some(found));
            }
            if wrap && start != 0 {
                return self.find_next(rope, 0, false, budget.remaining());
            }
            return Ok(None);
        }
        let lines = rope.len_lines();
        let attempts = if wrap { lines + 1 } else { lines - first };
        for distance in 0..attempts {
            let line = (first + distance) % lines;
            let content = rope_line_content_range(rope, line);
            let slice = rope.byte_slice(content.clone());
            let text = Cow::<str>::from(slice);
            let offset = if distance == 0 {
                start.min(content.end) - content.start
            } else {
                0
            };
            let mut results = FindResults::default();
            self.collect(
                &text,
                offset,
                |offset| content.start + offset,
                1,
                &budget,
                &mut results,
            )?;
            if results.timed_out {
                return Err(timeout_error());
            }
            if let Some(found) = results.matches.pop() {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    pub fn find_previous(
        &self,
        rope: &Rope,
        start: usize,
        wrap: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<FindMatch>, FindPatternError> {
        validate_range(rope, &(start..start))?;
        if matches!(self.pattern, Pattern::Empty) {
            return Ok(None);
        }
        let budget = SearchBudget::new(timeout);
        if self.multiline {
            let results = self.find_matches(rope, &[], usize::MAX, budget.remaining())?;
            if results.timed_out {
                return Err(timeout_error());
            }
            let found = results
                .matches
                .iter()
                .rev()
                .find(|found| found.range.end <= start);
            return Ok(found
                .cloned()
                .or_else(|| wrap.then(|| results.matches.last().cloned()).flatten()));
        }
        let first = rope.byte_to_line(start);
        let lines = rope.len_lines();
        let attempts = if wrap { lines + 1 } else { first + 1 };
        for distance in 0..attempts {
            let line = (lines + first - distance % lines) % lines;
            let content = rope_line_content_range(rope, line);
            let slice = rope.byte_slice(content.clone());
            let text = Cow::<str>::from(slice);
            let mut results = FindResults::default();
            self.collect(
                &text,
                0,
                |offset| content.start + offset,
                usize::MAX,
                &budget,
                &mut results,
            )?;
            if results.timed_out {
                return Err(timeout_error());
            }
            if let Some(found) = results
                .matches
                .into_iter()
                .rev()
                .find(|found| distance != 0 || found.range.end <= start)
            {
                return Ok(Some(found));
            }
        }
        Ok(None)
    }

    fn collect(
        &self,
        text: &str,
        start: usize,
        original_offset: impl Fn(usize) -> usize,
        limit: usize,
        budget: &SearchBudget,
        results: &mut FindResults,
    ) -> Result<(), FindPatternError> {
        let mut offset = start;
        loop {
            if budget.expired() {
                results.timed_out = true;
                return Ok(());
            }
            let captures = match &self.pattern {
                Pattern::Empty => return Ok(()),
                Pattern::Literal(source) => text[offset..].find(source).map(|start| {
                    let start = offset + start;
                    FindCaptures {
                        groups: vec![Some(start..start + source.len())],
                    }
                }),
                Pattern::Compiled(pattern) => pattern.captures_at(text, offset)?,
            };
            let Some(captures) = captures else {
                return Ok(());
            };
            let Some(Some(range)) = captures.groups.first() else {
                return Err(FindPatternError("Missing full match capture".to_string()));
            };
            if range.start < offset
                || range.start > range.end
                || !text.is_char_boundary(range.start)
                || !text.is_char_boundary(range.end)
            {
                return Err(FindPatternError("Invalid match boundary".to_string()));
            }
            if !self.options.whole_word
                || is_whole_word_match(text, range.clone(), &self.options.word_separators)
            {
                let values = captures
                    .groups
                    .iter()
                    .map(|group| {
                        group
                            .as_ref()
                            .map(|range| {
                                text.get(range.clone()).map(str::to_string).ok_or_else(|| {
                                    FindPatternError("Invalid capture boundary".to_string())
                                })
                            })
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                results.matches.push(FindMatch {
                    range: original_offset(range.start)..original_offset(range.end),
                    captures: values,
                });
                if results.matches.len() >= limit {
                    results.limit_reached = true;
                    return Ok(());
                }
            }
            if range.end == text.len() {
                return Ok(());
            }
            offset = if range.is_empty() {
                range.end + text[range.end..].chars().next().unwrap().len_utf8()
            } else {
                range.end
            };
        }
    }
}

pub fn is_multiline_regex_source(source: &str) -> bool {
    let mut characters = source.chars();
    while let Some(character) = characters.next() {
        if character == '\n' {
            return true;
        }
        if character == '\\' && matches!(characters.next(), Some('n' | 'r' | 'W')) {
            return true;
        }
    }
    false
}

pub fn escape_find_literal(source: &str) -> String {
    let mut escaped = String::with_capacity(source.len());
    for character in source.chars() {
        if matches!(
            character,
            '\\' | '{' | '}' | '*' | '+' | '?' | '|' | '^' | '$' | '.' | '[' | ']' | '(' | ')'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

pub fn is_whole_word_match(text: &str, range: Range<usize>, separators: &str) -> bool {
    let separator = |character| matches!(character, ' ' | '\t') || separators.contains(character);
    let boundary = |character| matches!(character, '\r' | '\n') || separator(character);
    let before = text[..range.start].chars().next_back();
    let after = text[range.end..].chars().next();
    let first = text[range.clone()].chars().next();
    let last = text[range].chars().next_back();
    (before.is_none_or(boundary) || first.is_some_and(separator))
        && (after.is_none_or(boundary) || last.is_some_and(separator))
}

fn validate_range(rope: &Rope, range: &Range<usize>) -> Result<(), FindPatternError> {
    if range.start > range.end
        || byte_to_char(rope, range.start).is_err()
        || byte_to_char(rope, range.end).is_err()
    {
        return Err(FindPatternError("Invalid search boundary".to_string()));
    }
    Ok(())
}

fn search_scopes(
    rope: &Rope,
    scopes: &[Range<usize>],
) -> Result<Vec<Range<usize>>, FindPatternError> {
    if scopes.is_empty() {
        return Ok(vec![0..rope.len_bytes()]);
    }
    for scope in scopes {
        validate_range(rope, scope)?;
    }
    let mut ordered = scopes.to_vec();
    ordered.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ordered {
        if let Some(previous) = merged.last_mut()
            && (range.start < previous.end || range == *previous)
        {
            previous.end = previous.end.max(range.end);
            continue;
        }
        merged.push(range);
    }
    Ok(merged)
}

struct SearchBudget {
    started: Instant,
    timeout: Option<Duration>,
}

impl SearchBudget {
    fn new(timeout: Option<Duration>) -> Self {
        Self {
            started: Instant::now(),
            timeout,
        }
    }

    fn expired(&self) -> bool {
        self.timeout
            .is_some_and(|timeout| self.started.elapsed() >= timeout)
    }

    fn remaining(&self) -> Option<Duration> {
        self.timeout
            .map(|timeout| timeout.saturating_sub(self.started.elapsed()))
    }
}

fn timeout_error() -> FindPatternError {
    FindPatternError("Search exceeded its time limit".to_string())
}

struct NormalizedText {
    text: String,
    removed_at: Vec<usize>,
}

impl NormalizedText {
    fn new(rope: &Rope, range: Range<usize>) -> Self {
        let mut characters = rope.byte_slice(range).chars().peekable();
        let mut text = String::new();
        let mut removed_at = Vec::new();
        while let Some(character) = characters.next() {
            if character != '\r' {
                text.push(character);
                continue;
            }
            text.push('\n');
            if characters.peek() == Some(&'\n') {
                characters.next();
                removed_at.push(text.len());
            }
        }
        Self { text, removed_at }
    }

    fn original_offset(&self, offset: usize) -> usize {
        offset
            + self
                .removed_at
                .partition_point(|removed| *removed <= offset)
    }

    fn normalized_offset(&self, offset: usize) -> usize {
        let removed = self
            .removed_at
            .iter()
            .enumerate()
            .take_while(|(index, removed)| **removed + index < offset)
            .count();
        offset - removed
    }
}
