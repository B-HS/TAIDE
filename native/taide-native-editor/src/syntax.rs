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
