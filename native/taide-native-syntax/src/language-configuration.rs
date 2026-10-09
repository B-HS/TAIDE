use std::collections::HashMap;
use std::ops::Range;
use std::sync::OnceLock;

use serde::Deserialize;
use taide_native_editor::language_configuration::{
    AutoClosingPair, BracketPair, CharacterPairs, CommentTokens, EnterAction, FoldMarker,
    IndentAction, IndentMetadata, LanguageRules,
};
use taide_native_editor::syntax::TokenKind;

use crate::js_regex::{JsRegex, JsRegexError};

const MONACO_CONFIGURATIONS: &str = include_str!("../language-configurations/monaco.json");
const DEFAULT_ENTER_BRACKETS: [(&str, &str); 3] = [("(", ")"), ("{", "}"), ("[", "]")];
const AUTO_CLOSE_BEFORE_QUOTES: &str = ";:.,=}])> \n\t";
const AUTO_CLOSE_BEFORE_BRACKETS: &str = "'\"`;:.,=}])> \n\t";
const REGEX_SPECIAL_CHARACTERS: &str = r"\{}*+?|^$.[]()";
const IGNORE_CASE_FLAGS: &str = "i";
const CASE_SENSITIVE_FLAGS: &str = "";
const STRING_TOKEN: &str = "string";
const COMMENT_TOKEN: &str = "comment";
const REGEX_TOKEN: &str = "regex";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageConfigurationError {
    InvalidData(String),
    InvalidPattern {
        language_id: String,
        source: String,
        error: JsRegexError,
    },
}

#[derive(Debug, Deserialize)]
struct ConfigurationFile {
    languages: Vec<LanguageEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanguageEntry {
    language_id: String,
    configuration: Option<Configuration>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pattern {
    pub source: String,
    pub flags: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LineComment {
    Token(String),
    Detailed { comment: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Comments {
    line_comment: Option<LineComment>,
    block_comment: Option<(String, String)>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IndentationRules {
    increase_indent_pattern: Option<Pattern>,
    decrease_indent_pattern: Option<Pattern>,
    indent_next_line_pattern: Option<Pattern>,
    un_indented_line_pattern: Option<Pattern>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
enum IndentActionName {
    None,
    Indent,
    IndentOutdent,
    Outdent,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OnEnterAction {
    indent_action: IndentActionName,
    append_text: Option<String>,
    remove_text: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OnEnterRule {
    before_text: Pattern,
    after_text: Option<Pattern>,
    previous_line_text: Option<Pattern>,
    action: OnEnterAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PairEntry {
    open: String,
    close: String,
    not_in: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Markers {
    start: Pattern,
    end: Pattern,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Folding {
    off_side: Option<bool>,
    markers: Option<Markers>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Configuration {
    comments: Option<Comments>,
    brackets: Option<Vec<(String, String)>>,
    word_pattern: Option<Pattern>,
    indentation_rules: Option<IndentationRules>,
    on_enter_rules: Option<Vec<OnEnterRule>>,
    auto_closing_pairs: Option<Vec<PairEntry>>,
    surrounding_pairs: Option<Vec<PairEntry>>,
    auto_close_before: Option<String>,
    folding: Option<Folding>,
    colorized_bracket_pairs: Option<Vec<(String, String)>>,
}

#[derive(Debug)]
struct EnterRule {
    before: JsRegex,
    after: Option<JsRegex>,
    previous_line: Option<JsRegex>,
    action: EnterAction,
}

#[derive(Debug)]
struct EnterBracket {
    open: JsRegex,
    close: JsRegex,
}

#[derive(Debug)]
struct IndentationPatterns {
    increase: Option<JsRegex>,
    decrease: Option<JsRegex>,
    indent_next_line: Option<JsRegex>,
    unindented_line: Option<JsRegex>,
}

#[derive(Debug)]
enum MarkerPatterns {
    Combined(JsRegex),
    Separate { end: JsRegex },
}

#[derive(Debug)]
struct FoldMarkers {
    start: JsRegex,
    patterns: MarkerPatterns,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketPatternSources {
    pub enter_brackets: Vec<(String, String)>,
    pub brackets: Option<String>,
    pub tree_brackets: Option<String>,
    pub reversed_brackets: Option<String>,
}

#[derive(Debug)]
pub struct MonacoLanguage {
    pairs: CharacterPairs,
    colorized_brackets: Option<Vec<BracketPair>>,
    word_pattern: Option<Pattern>,
    word: JsRegex,
    line_comment: Option<String>,
    comments: CommentTokens,
    enter_rules: Vec<EnterRule>,
    enter_brackets: Vec<EnterBracket>,
    indentation: Option<IndentationPatterns>,
    brackets: Option<JsRegex>,
    tree_brackets: Option<JsRegex>,
    reversed_brackets: Option<JsRegex>,
    is_off_side: bool,
    fold_markers: Option<FoldMarkers>,
    sources: BracketPatternSources,
}

fn escape_regex_characters(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len() * 2);
    for character in text.chars() {
        if REGEX_SPECIAL_CHARACTERS.contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn is_js_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn is_word_or_space(character: char) -> bool {
    is_js_word_character(character) || character == ' '
}

fn enter_open_source(bracket: &str) -> String {
    let escaped = escape_regex_characters(bracket);
    let boundary = if escaped.chars().next().is_some_and(is_js_word_character) {
        r"\b"
    } else {
        ""
    };
    format!(r"{boundary}{escaped}\s*$")
}

fn enter_close_source(bracket: &str) -> String {
    let escaped = escape_regex_characters(bracket);
    let boundary = if escaped.chars().last().is_some_and(is_js_word_character) {
        r"\b"
    } else {
        ""
    };
    format!(r"^\s*{escaped}{boundary}")
}

fn rich_edit_piece(bracket: &str) -> String {
    let escaped = escape_regex_characters(bracket);
    if !bracket.is_empty() && bracket.chars().all(is_word_or_space) {
        format!(r"\b{escaped}\b")
    } else {
        escaped
    }
}

fn tree_piece(bracket: &str) -> String {
    let mut piece = escape_regex_characters(bracket);
    if bracket.chars().next().is_some_and(is_word_or_space) {
        piece = format!(r"\b{piece}");
    }
    if bracket.chars().last().is_some_and(is_word_or_space) {
        piece.push_str(r"\b");
    }
    piece
}

fn alternatives(pieces: &[String]) -> Option<String> {
    (!pieces.is_empty()).then(|| format!("({})", pieces.join(")|(")))
}

fn push_distinct(texts: &mut Vec<String>, text: &str) {
    if !texts.iter().any(|known| known == text) {
        texts.push(text.into());
    }
}

struct BracketTexts {
    opening: Vec<String>,
    closing: Vec<String>,
}

fn bracket_texts(configuration: &Configuration) -> BracketTexts {
    let mut texts = BracketTexts {
        opening: Vec::new(),
        closing: Vec::new(),
    };
    let pairs = configuration
        .brackets
        .iter()
        .chain(&configuration.colorized_bracket_pairs)
        .flatten()
        .filter(|(open, close)| !open.is_empty() && !close.is_empty());
    for (open, close) in pairs {
        push_distinct(&mut texts.opening, open);
        push_distinct(&mut texts.closing, close);
    }
    texts
}

fn utf16_ordered(texts: &mut [String]) {
    texts.sort_by(|left, right| left.encode_utf16().cmp(right.encode_utf16()));
}

fn fuzzy_groups(brackets: &[(String, String)]) -> Vec<(Vec<String>, Vec<String>)> {
    let pairs: Vec<(String, String)> = brackets
        .iter()
        .map(|(open, close)| (open.to_lowercase(), close.to_lowercase()))
        .collect();
    let mut group: Vec<usize> = (0..pairs.len()).collect();
    for first in 0..pairs.len() {
        for second in first + 1..pairs.len() {
            let (left, right) = (&pairs[first], &pairs[second]);
            let overlaps =
                left.0 == right.0 || left.0 == right.1 || left.1 == right.0 || left.1 == right.1;
            if !overlaps {
                continue;
            }
            let kept = group[first].min(group[second]);
            let merged = group[first].max(group[second]);
            for member in &mut group {
                if *member == merged {
                    *member = kept;
                }
            }
        }
    }
    (0..pairs.len())
        .filter_map(|index| {
            let members: Vec<&(String, String)> = pairs
                .iter()
                .zip(&group)
                .filter(|(_, member)| **member == index)
                .map(|(pair, _)| pair)
                .collect();
            (!members.is_empty()).then(|| {
                (
                    members.iter().map(|pair| pair.0.clone()).collect(),
                    members.iter().map(|pair| pair.1.clone()).collect(),
                )
            })
        })
        .collect()
}

fn reversed(text: &str) -> String {
    text.chars().rev().collect()
}

fn excluded_tokens(not_in: Option<&[String]>) -> Vec<TokenKind> {
    not_in
        .unwrap_or_default()
        .iter()
        .filter_map(|token| match token.as_str() {
            STRING_TOKEN => Some(TokenKind::String),
            COMMENT_TOKEN => Some(TokenKind::Comment),
            REGEX_TOKEN => Some(TokenKind::Regex),
            _ => None,
        })
        .collect()
}

struct Compiler<'a> {
    language_id: &'a str,
}

impl Compiler<'_> {
    fn regex(&self, source: &str, flags: &str) -> Result<JsRegex, LanguageConfigurationError> {
        JsRegex::new(source, flags).map_err(|error| LanguageConfigurationError::InvalidPattern {
            language_id: self.language_id.into(),
            source: source.into(),
            error,
        })
    }

    fn pattern(&self, pattern: &Pattern) -> Result<JsRegex, LanguageConfigurationError> {
        self.regex(&pattern.source, &pattern.flags)
    }

    fn optional(
        &self,
        pattern: Option<&Pattern>,
    ) -> Result<Option<JsRegex>, LanguageConfigurationError> {
        pattern.map(|pattern| self.pattern(pattern)).transpose()
    }

    fn optional_source(
        &self,
        source: Option<&String>,
    ) -> Result<Option<JsRegex>, LanguageConfigurationError> {
        source
            .map(|source| self.regex(source, IGNORE_CASE_FLAGS))
            .transpose()
    }
}

impl MonacoLanguage {
    fn compile(
        language_id: &str,
        configuration: &Configuration,
    ) -> Result<Self, LanguageConfigurationError> {
        let compiler = Compiler { language_id };
        let brackets = configuration.brackets.as_deref().unwrap_or_default();
        let bracket_pairs: Vec<BracketPair> = brackets
            .iter()
            .map(|(open, close)| BracketPair {
                open: open.clone(),
                close: close.clone(),
            })
            .collect();
        let auto_closing_pairs: Vec<AutoClosingPair> = match &configuration.auto_closing_pairs {
            Some(pairs) => pairs
                .iter()
                .map(|pair| AutoClosingPair {
                    open: pair.open.clone(),
                    close: pair.close.clone(),
                    excluded_tokens: excluded_tokens(pair.not_in.as_deref()),
                })
                .collect(),
            None => bracket_pairs
                .iter()
                .map(|pair| AutoClosingPair {
                    open: pair.open.clone(),
                    close: pair.close.clone(),
                    excluded_tokens: Vec::new(),
                })
                .collect(),
        };
        let surrounding_pairs = match &configuration.surrounding_pairs {
            Some(pairs) => pairs
                .iter()
                .map(|pair| BracketPair {
                    open: pair.open.clone(),
                    close: pair.close.clone(),
                })
                .collect(),
            None => auto_closing_pairs
                .iter()
                .map(|pair| BracketPair {
                    open: pair.open.clone(),
                    close: pair.close.clone(),
                })
                .collect(),
        };
        let comments = configuration.comments.as_ref();
        let pairs = CharacterPairs {
            brackets: bracket_pairs,
            auto_closing_pairs,
            surrounding_pairs,
            auto_close_before_quotes: configuration
                .auto_close_before
                .clone()
                .unwrap_or_else(|| AUTO_CLOSE_BEFORE_QUOTES.into()),
            auto_close_before_brackets: configuration
                .auto_close_before
                .clone()
                .unwrap_or_else(|| AUTO_CLOSE_BEFORE_BRACKETS.into()),
            block_comment_start: comments
                .and_then(|comments| comments.block_comment.as_ref())
                .map(|(start, _)| start.clone()),
        };
        let line_comment = comments
            .and_then(|comments| comments.line_comment.as_ref())
            .map(|line_comment| match line_comment {
                LineComment::Token(token) => token.clone(),
                LineComment::Detailed { comment, .. } => comment.clone(),
            });
        let comment_tokens = CommentTokens {
            line: line_comment.clone().filter(|token| !token.is_empty()),
            block: comments
                .and_then(|comments| comments.block_comment.clone())
                .filter(|(start, end)| !start.is_empty() && !end.is_empty()),
        };

        let has_enter_support = configuration.brackets.is_some()
            || configuration.indentation_rules.is_some()
            || configuration.on_enter_rules.is_some();
        let enter_bracket_sources: Vec<(String, String)> = if !has_enter_support {
            Vec::new()
        } else if configuration.brackets.is_some() {
            brackets
                .iter()
                .map(|(open, close)| (enter_open_source(open), enter_close_source(close)))
                .collect()
        } else {
            DEFAULT_ENTER_BRACKETS
                .iter()
                .map(|(open, close)| (enter_open_source(open), enter_close_source(close)))
                .collect()
        };
        let enter_brackets = enter_bracket_sources
            .iter()
            .map(|(open, close)| {
                Ok(EnterBracket {
                    open: compiler.regex(open, CASE_SENSITIVE_FLAGS)?,
                    close: compiler.regex(close, CASE_SENSITIVE_FLAGS)?,
                })
            })
            .collect::<Result<Vec<_>, LanguageConfigurationError>>()?;
        let enter_rules = configuration
            .on_enter_rules
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|rule| {
                Ok(EnterRule {
                    before: compiler.pattern(&rule.before_text)?,
                    after: compiler.optional(rule.after_text.as_ref())?,
                    previous_line: compiler.optional(rule.previous_line_text.as_ref())?,
                    action: EnterAction {
                        indent_action: match rule.action.indent_action {
                            IndentActionName::None => IndentAction::None,
                            IndentActionName::Indent => IndentAction::Indent,
                            IndentActionName::IndentOutdent => IndentAction::IndentOutdent,
                            IndentActionName::Outdent => IndentAction::Outdent,
                        },
                        append_text: rule.action.append_text.clone(),
                        remove_text: rule.action.remove_text,
                    },
                })
            })
            .collect::<Result<Vec<_>, LanguageConfigurationError>>()?;
        let indentation = configuration
            .indentation_rules
            .as_ref()
            .map(|rules| {
                Ok(IndentationPatterns {
                    increase: compiler.optional(rules.increase_indent_pattern.as_ref())?,
                    decrease: compiler.optional(rules.decrease_indent_pattern.as_ref())?,
                    indent_next_line: compiler.optional(rules.indent_next_line_pattern.as_ref())?,
                    unindented_line: compiler.optional(rules.un_indented_line_pattern.as_ref())?,
                })
            })
            .transpose()?;

        let texts = bracket_texts(configuration);
        let rich_edit_pieces: Vec<String> = texts
            .opening
            .iter()
            .chain(&texts.closing)
            .map(|text| rich_edit_piece(text))
            .collect();
        let mut sorted_texts = texts.opening.clone();
        for text in &texts.closing {
            push_distinct(&mut sorted_texts, text);
        }
        utf16_ordered(&mut sorted_texts);
        let tree_pieces: Vec<String> = sorted_texts
            .iter()
            .rev()
            .map(|text| tree_piece(text))
            .collect();
        let mut grouped_texts = Vec::new();
        for (opens, closes) in fuzzy_groups(brackets) {
            for text in opens.iter().chain(&closes) {
                push_distinct(&mut grouped_texts, text);
            }
        }
        let reversed_pieces: Vec<String> = grouped_texts
            .iter()
            .map(|text| rich_edit_piece(&reversed(text)))
            .collect();
        let sources = BracketPatternSources {
            enter_brackets: enter_bracket_sources,
            brackets: alternatives(&rich_edit_pieces),
            tree_brackets: (!tree_pieces.is_empty()).then(|| tree_pieces.join("|")),
            reversed_brackets: alternatives(&reversed_pieces),
        };

        let folding = configuration.folding.as_ref();
        let fold_markers = folding
            .and_then(|folding| folding.markers.as_ref())
            .map(|markers| {
                let patterns = if markers.start.flags == markers.end.flags {
                    MarkerPatterns::Combined(compiler.regex(
                        &format!("({})|(?:{})", markers.start.source, markers.end.source),
                        &markers.start.flags,
                    )?)
                } else {
                    MarkerPatterns::Separate {
                        end: compiler.pattern(&markers.end)?,
                    }
                };
                Ok(FoldMarkers {
                    start: compiler.pattern(&markers.start)?,
                    patterns,
                })
            })
            .transpose()?;
        let word = match &configuration.word_pattern {
            Some(pattern) => compiler.pattern(pattern)?,
            None => compiler.regex(
                r#"(-?\d*\.\d\w*)|([^`~!@#$%^&*()\-=+\[{}\]\\|;:'\",.<>/?\s]+)"#,
                "g",
            )?,
        };

        Ok(Self {
            pairs,
            colorized_brackets: configuration.colorized_bracket_pairs.as_ref().map(|pairs| {
                pairs
                    .iter()
                    .filter(|(open, close)| !open.is_empty() && !close.is_empty())
                    .map(|(open, close)| BracketPair {
                        open: open.clone(),
                        close: close.clone(),
                    })
                    .collect()
            }),
            word_pattern: configuration.word_pattern.clone(),
            word,
            line_comment,
            comments: comment_tokens,
            enter_rules,
            enter_brackets,
            indentation,
            brackets: compiler.optional_source(sources.brackets.as_ref())?,
            tree_brackets: compiler.optional_source(sources.tree_brackets.as_ref())?,
            reversed_brackets: compiler.optional_source(sources.reversed_brackets.as_ref())?,
            is_off_side: folding.is_some_and(|folding| folding.off_side == Some(true)),
            fold_markers,
            sources,
        })
    }

    pub fn word_pattern(&self) -> Option<&Pattern> {
        self.word_pattern.as_ref()
    }

    pub fn line_comment(&self) -> Option<&str> {
        self.line_comment.as_deref()
    }

    pub fn bracket_pattern_sources(&self) -> &BracketPatternSources {
        &self.sources
    }
}

impl LanguageRules for MonacoLanguage {
    fn pairs(&self) -> &CharacterPairs {
        &self.pairs
    }

    fn colorized_brackets(&self) -> Option<&[BracketPair]> {
        self.colorized_brackets.as_deref()
    }

    fn word_range(&self, text: &str, byte: usize) -> Option<Range<usize>> {
        const MAX_WORD_UTF16_LENGTH: usize = 1000;
        const WORD_RADIUS: usize = MAX_WORD_UTF16_LENGTH / 2;
        let utf16 = text[..byte].encode_utf16().count();
        let window = if text.encode_utf16().count() > MAX_WORD_UTF16_LENGTH {
            let byte_at = |target| {
                let mut units = 0;
                for (byte, character) in text.char_indices() {
                    if units >= target {
                        return byte;
                    }
                    units += character.len_utf16();
                }
                text.len()
            };
            byte_at((utf16 + 1).saturating_sub(WORD_RADIUS))..byte_at(utf16 + 1 + WORD_RADIUS)
        } else {
            0..text.len()
        };
        self.word
            .ranges(&text[window.clone()])
            .into_iter()
            .map(|range| range.start + window.start..range.end + window.start)
            .find(|range| range.start <= byte && byte <= range.end)
    }

    fn comments(&self) -> Option<&CommentTokens> {
        Some(&self.comments)
    }

    fn enter_action(
        &self,
        previous_line: &str,
        before_enter: &str,
        after_enter: &str,
    ) -> Option<EnterAction> {
        let matches = |pattern: &Option<JsRegex>, text: &str| {
            pattern
                .as_ref()
                .is_none_or(|pattern| pattern.is_match(text))
        };
        if let Some(rule) = self.enter_rules.iter().find(|rule| {
            rule.before.is_match(before_enter)
                && matches(&rule.after, after_enter)
                && matches(&rule.previous_line, previous_line)
        }) {
            return Some(rule.action.clone());
        }
        if before_enter.is_empty() {
            return None;
        }
        let bracket_action = |indent_action| EnterAction {
            indent_action,
            append_text: None,
            remove_text: None,
        };
        if !after_enter.is_empty()
            && self.enter_brackets.iter().any(|bracket| {
                bracket.open.is_match(before_enter) && bracket.close.is_match(after_enter)
            })
        {
            return Some(bracket_action(IndentAction::IndentOutdent));
        }
        self.enter_brackets
            .iter()
            .any(|bracket| bracket.open.is_match(before_enter))
            .then(|| bracket_action(IndentAction::Indent))
    }

    fn indent_metadata(&self, line: &str) -> Option<IndentMetadata> {
        let patterns = self.indentation.as_ref()?;
        let matches = |pattern: &Option<JsRegex>| {
            pattern
                .as_ref()
                .is_some_and(|pattern| pattern.is_match(line))
        };
        Some(IndentMetadata {
            increases: matches(&patterns.increase),
            decreases: matches(&patterns.decrease),
            indents_next_line: matches(&patterns.indent_next_line),
            is_unindented: matches(&patterns.unindented_line),
        })
    }

    fn without_brackets(&self, text: &str) -> String {
        match &self.brackets {
            Some(brackets) => brackets.without_matches(text),
            None => text.into(),
        }
    }

    fn bracket_ranges(&self, line: &str) -> Vec<Range<usize>> {
        self.tree_brackets
            .as_ref()
            .map(|brackets| brackets.ranges(line))
            .unwrap_or_default()
    }

    fn last_bracket(&self, text: &str) -> Option<Range<usize>> {
        let mirrored = reversed(text);
        let found = self.reversed_brackets.as_ref()?.first_range(&mirrored)?;
        Some(text.len() - found.end..text.len() - found.start)
    }

    fn is_off_side(&self) -> bool {
        self.is_off_side
    }

    fn fold_marker(&self, line: &str) -> Option<FoldMarker> {
        let markers = self.fold_markers.as_ref()?;
        match &markers.patterns {
            MarkerPatterns::Combined(pattern) => {
                pattern.captures_text_in_first_group(line).map(|is_start| {
                    if is_start {
                        FoldMarker::Start
                    } else {
                        FoldMarker::End
                    }
                })
            }
            MarkerPatterns::Separate { end } => {
                if markers.start.is_match(line) {
                    Some(FoldMarker::Start)
                } else {
                    end.is_match(line).then_some(FoldMarker::End)
                }
            }
        }
    }

    fn starts_marker_region(&self, line: &str) -> bool {
        self.fold_markers
            .as_ref()
            .is_some_and(|markers| markers.start.is_match(line))
    }
}

struct RegisteredLanguage {
    configuration: Configuration,
    compiled: OnceLock<Result<MonacoLanguage, LanguageConfigurationError>>,
}

type Registry = Result<HashMap<String, RegisteredLanguage>, LanguageConfigurationError>;

fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let file: ConfigurationFile = serde_json::from_str(MONACO_CONFIGURATIONS)
            .map_err(|error| LanguageConfigurationError::InvalidData(error.to_string()))?;
        Ok(file
            .languages
            .into_iter()
            .filter_map(|entry| {
                Some((
                    entry.language_id,
                    RegisteredLanguage {
                        configuration: entry.configuration?,
                        compiled: OnceLock::new(),
                    },
                ))
            })
            .collect())
    })
}

pub fn monaco_language_ids() -> Result<Vec<&'static str>, &'static LanguageConfigurationError> {
    let mut language_ids: Vec<&str> = registry().as_ref()?.keys().map(String::as_str).collect();
    language_ids.sort_unstable();
    Ok(language_ids)
}

pub fn monaco_language(
    language_id: &str,
) -> Result<Option<&'static MonacoLanguage>, &'static LanguageConfigurationError> {
    let Some(registered) = registry().as_ref()?.get(language_id) else {
        return Ok(None);
    };
    registered
        .compiled
        .get_or_init(|| MonacoLanguage::compile(language_id, &registered.configuration))
        .as_ref()
        .map(Some)
}
