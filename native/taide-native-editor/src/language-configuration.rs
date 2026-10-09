use std::ops::Range;

use crate::document::DocumentSnapshot;
use crate::store::EditorStore;
use crate::syntax::{Token, TokenKind};

const QUOTE_CHARACTERS: [char; 3] = ['\'', '"', '`'];
const JS_WHITESPACE_RANGES: [(u32, u32); 10] = [
    (0x09, 0x0D),
    (0x20, 0x20),
    (0xA0, 0xA0),
    (0x1680, 0x1680),
    (0x2000, 0x200A),
    (0x2028, 0x2029),
    (0x202F, 0x202F),
    (0x205F, 0x205F),
    (0x3000, 0x3000),
    (0xFEFF, 0xFEFF),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentAction {
    None,
    Indent,
    IndentOutdent,
    Outdent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterAction {
    pub indent_action: IndentAction,
    pub append_text: Option<String>,
    pub remove_text: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndentMetadata {
    pub increases: bool,
    pub decreases: bool,
    pub indents_next_line: bool,
    pub is_unindented: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketPair {
    pub open: String,
    pub close: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoClosingPair {
    pub open: String,
    pub close: String,
    pub excluded_tokens: Vec<TokenKind>,
}

impl AutoClosingPair {
    pub fn allows(&self, kind: TokenKind) -> bool {
        kind == TokenKind::Other || !self.excluded_tokens.contains(&kind)
    }

    pub fn neutral_character(&self) -> Option<char> {
        ('0'..='9')
            .chain('a'..='z')
            .chain('A'..='Z')
            .find(|character| !self.open.contains(*character) && !self.close.contains(*character))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterPairs {
    pub brackets: Vec<BracketPair>,
    pub auto_closing_pairs: Vec<AutoClosingPair>,
    pub surrounding_pairs: Vec<BracketPair>,
    pub auto_close_before_quotes: String,
    pub auto_close_before_brackets: String,
    pub block_comment_start: Option<String>,
}

impl CharacterPairs {
    pub fn surrounding_close(&self, open: char) -> Option<&str> {
        let mut text = [0; 4];
        let open: &str = open.encode_utf8(&mut text);
        self.surrounding_pairs
            .iter()
            .rev()
            .find(|pair| pair.open == open)
            .map(|pair| pair.close.as_str())
    }

    pub fn is_opening_bracket(&self, text: &str) -> bool {
        self.brackets.iter().any(|pair| pair.open == text) && !self.is_closing_bracket(text)
    }

    pub fn is_closing_bracket(&self, text: &str) -> bool {
        self.brackets.iter().any(|pair| pair.close == text)
    }

    pub fn closes(&self, close: &str, open: &str) -> bool {
        self.brackets
            .iter()
            .any(|pair| pair.open == open && pair.close == close)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldMarker {
    Start,
    End,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentTokens {
    pub line: Option<String>,
    pub block: Option<(String, String)>,
}

pub trait LanguageRules {
    fn pairs(&self) -> &CharacterPairs;

    fn colorized_brackets(&self) -> Option<&[BracketPair]> {
        None
    }

    fn word_range(&self, _text: &str, _byte: usize) -> Option<Range<usize>> {
        None
    }

    fn comments(&self) -> Option<&CommentTokens> {
        None
    }

    fn enter_action(
        &self,
        previous_line: &str,
        before_enter: &str,
        after_enter: &str,
    ) -> Option<EnterAction>;

    fn indent_metadata(&self, line: &str) -> Option<IndentMetadata>;

    fn without_brackets(&self, text: &str) -> String;

    fn bracket_ranges(&self, line: &str) -> Vec<Range<usize>>;

    fn last_bracket(&self, text: &str) -> Option<Range<usize>>;

    fn is_off_side(&self) -> bool;

    fn fold_marker(&self, line: &str) -> Option<FoldMarker>;

    fn starts_marker_region(&self, line: &str) -> bool;
}

pub trait LineSyntax {
    fn follow_edits(&self, _store: &EditorStore) {}

    fn tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>>;

    fn accurate_tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        self.tokens(document, line)
    }

    fn kind_if_inserting(
        &self,
        document: &DocumentSnapshot,
        line: usize,
        byte_in_line: usize,
        character: char,
    ) -> TokenKind;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UntokenizedLines;

impl LineSyntax for UntokenizedLines {
    fn tokens(&self, document: &DocumentSnapshot, line: usize) -> Option<Vec<Token>> {
        let content = crate::editing::line_content_range(document, line);
        Some(if content.is_empty() {
            Vec::new()
        } else {
            vec![Token {
                start_byte: 0,
                kind: TokenKind::Other,
            }]
        })
    }

    fn kind_if_inserting(
        &self,
        _document: &DocumentSnapshot,
        _line: usize,
        _byte_in_line: usize,
        _character: char,
    ) -> TokenKind {
        TokenKind::Other
    }
}

#[derive(Clone, Copy)]
pub struct Language<'a> {
    pub rules: &'a dyn LanguageRules,
    pub syntax: &'a dyn LineSyntax,
}

pub fn is_quote(character: char) -> bool {
    QUOTE_CHARACTERS.contains(&character)
}

pub fn is_js_whitespace(character: char) -> bool {
    let code = u32::from(character);
    JS_WHITESPACE_RANGES
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&code))
}

pub fn token_kind_at(tokens: &[Token], byte: usize) -> TokenKind {
    tokens
        .partition_point(|token| token.start_byte <= byte)
        .checked_sub(1)
        .or_else(|| (!tokens.is_empty()).then_some(0))
        .map_or(TokenKind::Other, |index| tokens[index].kind)
}

pub fn without_brackets_outside_code(
    rules: &dyn LanguageRules,
    text: &str,
    tokens: &[Token],
    bytes: Range<usize>,
) -> String {
    let bytes = bytes.start.min(text.len())..bytes.end.min(text.len());
    if tokens.is_empty() {
        return text[bytes].into();
    }
    let mut processed = String::with_capacity(bytes.len());
    for (index, token) in tokens.iter().enumerate() {
        let token_end = tokens
            .get(index + 1)
            .map_or(text.len(), |next| next.start_byte.min(text.len()));
        let start = token.start_byte.max(bytes.start);
        let end = token_end.min(bytes.end);
        if start >= end || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
            continue;
        }
        if token.kind == TokenKind::Other {
            processed.push_str(&text[start..end]);
        } else {
            processed.push_str(&rules.without_brackets(&text[start..end]));
        }
    }
    processed
}
