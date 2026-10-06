use std::fmt;

use taide_native_editor::syntax::Token;

use crate::textmate_tokenizer::EngineState;

pub const FONT_STYLE_ITALIC: u32 = 1;
pub const FONT_STYLE_BOLD: u32 = 2;
pub const FONT_STYLE_UNDERLINE: u32 = 4;
pub const FONT_STYLE_STRIKETHROUGH: u32 = 8;
pub const FONT_STYLE_VARIANTS: u32 = 16;
pub const UNSTYLED_STYLE_ID: u32 = 0;
pub const SPAN_FIELDS: usize = 2;

pub const fn style_id(color_index: u32, font_style_bits: u32) -> u32 {
    color_index * FONT_STYLE_VARIANTS + font_style_bits
}

pub const fn style_color_index(style_id: u32) -> u32 {
    style_id / FONT_STYLE_VARIANTS
}

pub const fn style_font_bits(style_id: u32) -> u32 {
    style_id % FONT_STYLE_VARIANTS
}

#[derive(Clone)]
pub struct LineState(pub(crate) EngineState);

impl LineState {
    pub fn is_same(&self, other: &Self) -> bool {
        self.0.is_same(&other.0)
    }
}

impl fmt::Debug for LineState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("LineState")
            .field(&format_args!("{}", self.0))
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct TokenizedLine {
    pub spans: Vec<u32>,
    pub kinds: Vec<Token>,
    pub end_state: LineState,
    pub is_stopped_early: bool,
}

pub trait GrammarTokenizer {
    fn tokenize_line(
        &mut self,
        language_id: &str,
        line: &str,
        previous: Option<&LineState>,
    ) -> Option<TokenizedLine>;

    fn is_same_state(&self, left: &LineState, right: &LineState) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxError {
    InvalidGrammar(String),
    InvalidTheme(String),
    UnknownLanguage(String),
    Tokenization(String),
}
