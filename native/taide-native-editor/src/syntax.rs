use crate::line_tokens::{LineTokens, TokenStyleTable};

const SPAN_FIELDS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Other,
    Comment,
    String,
    Regex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub start_byte: usize,
    pub kind: TokenKind,
}

#[derive(Debug, Clone)]
pub struct TokenLine {
    pub line: usize,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct SyntaxSnapshot {
    pub revision: u64,
    pub language_id: String,
    pub lines: Vec<TokenLine>,
}

impl SyntaxSnapshot {
    pub fn from_accurate_lines(
        revision: u64,
        language_id: String,
        tokens: &LineTokens,
        styles: &TokenStyleTable,
    ) -> Self {
        Self {
            revision,
            language_id,
            lines: (0..tokens.line_count())
                .take_while(|line| tokens.has_accurate_tokens(*line))
                .map(|line| {
                    let mut kinds: Vec<Token> = Vec::new();
                    for [start_byte, style_id] in tokens.spans(line).as_chunks::<SPAN_FIELDS>().0 {
                        let kind = styles.style(*style_id).kind;
                        if kinds.last().is_none_or(|previous| previous.kind != kind) {
                            kinds.push(Token {
                                start_byte: *start_byte as usize,
                                kind,
                            });
                        }
                    }
                    TokenLine {
                        line,
                        tokens: kinds,
                    }
                })
                .collect(),
        }
    }

    pub fn without_tokenizer(revision: u64, language_id: String, line_count: usize) -> Self {
        Self {
            revision,
            language_id,
            lines: (0..line_count)
                .map(|line| TokenLine {
                    line,
                    tokens: vec![Token {
                        start_byte: 0,
                        kind: TokenKind::Other,
                    }],
                })
                .collect(),
        }
    }

    pub fn token_at(&self, line: usize, byte: usize) -> Option<TokenKind> {
        let index = self
            .lines
            .binary_search_by_key(&line, |line| line.line)
            .ok()?;
        let tokens = &self.lines[index].tokens;
        let index = tokens
            .partition_point(|token| token.start_byte <= byte)
            .checked_sub(1)?;
        Some(tokens[index].kind)
    }
}
