use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::sync::Arc;

use ferriki_textmate::{
    EncodedTokenAttributes, Grammar, GrammarConfiguration, RawTheme, RawThemeScope,
    RawThemeSetting, RawThemeStyle, StateStack, SyncRegistry, parse_raw_grammar,
};
use taide_native_editor::syntax::{Token, TokenKind};

use crate::style_scopes::{StyleScopes, standard_token_kind};
use crate::theme_settings::ThemeSetting;
use crate::tokenizer::{
    FONT_STYLE_VARIANTS, GrammarTokenizer, LineState, SyntaxError, TokenizedLine,
    UNSTYLED_STYLE_ID, style_color_index, style_id,
};
use crate::utf16_offsets::{Utf16ByteCursor, utf16_len};

const GRAMMAR_FILE_NAME: &str = "grammar.tmLanguage.json";
const ROOT_LANGUAGE_ID: u32 = 1;
const ANY_BALANCED_BRACKET_SELECTOR: &str = "*";
const DEFAULT_MAX_LINE_UTF16_LENGTH: usize = 20_000;
const DEFAULT_LINE_TIME_LIMIT_MILLIS: u64 = 500;
const UNLIMITED_LINE_TIME: u64 = 0;
const ENGINE_TOKEN_FIELDS: usize = 2;

#[derive(Clone)]
pub(crate) struct EngineState {
    stack: Arc<StateStack>,
}

impl EngineState {
    pub(crate) fn initial() -> Self {
        Self {
            stack: StateStack::null(),
        }
    }

    pub(crate) fn is_same(&self, other: &Self) -> bool {
        self.stack.equals(&other.stack)
    }
}

impl fmt::Display for EngineState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.stack, formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageGrammar<'a> {
    pub language_id: &'a str,
    pub scope_name: &'a str,
}

#[derive(Debug, Clone, Default)]
pub struct GrammarSet<'a> {
    pub grammar_sources: Vec<&'a str>,
    pub languages: Vec<LanguageGrammar<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenizerLimits {
    pub max_line_utf16_length: usize,
    pub line_time_limit_millis: Option<NonZeroU64>,
}

impl Default for TokenizerLimits {
    fn default() -> Self {
        Self {
            max_line_utf16_length: DEFAULT_MAX_LINE_UTF16_LENGTH,
            line_time_limit_millis: NonZeroU64::new(DEFAULT_LINE_TIME_LIMIT_MILLIS),
        }
    }
}

pub struct TextmateTokenizer {
    registry: SyncRegistry,
    scope_name_by_language_id: BTreeMap<String, String>,
    grammar_by_scope_name: BTreeMap<String, Rc<Grammar>>,
    color_map: Vec<String>,
    monaco_scope_by_style_id: Vec<String>,
    kind_by_style_id: Vec<TokenKind>,
    limits: TokenizerLimits,
}

impl TextmateTokenizer {
    pub fn new(
        grammars: &GrammarSet<'_>,
        theme: &[ThemeSetting],
        limits: TokenizerLimits,
    ) -> Result<Self, SyntaxError> {
        let mut registry = SyncRegistry::new(Some(raw_theme(theme)), None)
            .map_err(|error| SyntaxError::InvalidTheme(error.to_string()))?;
        for source in &grammars.grammar_sources {
            let grammar = parse_raw_grammar(source, Some(GRAMMAR_FILE_NAME))
                .map_err(|error| SyntaxError::InvalidGrammar(error.to_string()))?;
            registry.add_grammar(grammar, Vec::new());
        }
        let color_map = registry.get_color_map();
        let style_scopes = StyleScopes::from_theme(theme);
        let monaco_scope_by_style_id: Vec<String> = color_map
            .iter()
            .flat_map(|color| {
                (0..FONT_STYLE_VARIANTS)
                    .map(|font_style_bits| style_scopes.scope(color, font_style_bits).to_owned())
            })
            .collect();
        let kind_by_style_id = monaco_scope_by_style_id
            .iter()
            .map(|scope| standard_token_kind(scope))
            .collect();
        Ok(Self {
            registry,
            scope_name_by_language_id: grammars
                .languages
                .iter()
                .map(|language| {
                    (
                        language.language_id.to_owned(),
                        language.scope_name.to_owned(),
                    )
                })
                .collect(),
            grammar_by_scope_name: BTreeMap::new(),
            color_map,
            monaco_scope_by_style_id,
            kind_by_style_id,
            limits,
        })
    }

    pub fn color_map(&self) -> &[String] {
        &self.color_map
    }

    pub fn style_color(&self, style_id: u32) -> Option<&str> {
        self.color_map
            .get(style_color_index(style_id) as usize)
            .map(String::as_str)
    }

    pub fn style_count(&self) -> u32 {
        u32::try_from(self.monaco_scope_by_style_id.len()).unwrap_or(u32::MAX)
    }

    pub fn monaco_scope(&self, style_id: u32) -> &str {
        self.monaco_scope_by_style_id
            .get(style_id as usize)
            .map_or("", String::as_str)
    }

    pub fn token_kind(&self, style_id: u32) -> TokenKind {
        self.kind_by_style_id
            .get(style_id as usize)
            .copied()
            .unwrap_or(TokenKind::Other)
    }

    pub fn try_tokenize_line(
        &mut self,
        language_id: &str,
        line: &str,
        previous: Option<&LineState>,
    ) -> Result<TokenizedLine, SyntaxError> {
        let grammar = self.grammar(language_id)?;
        let previous_stack = previous.map(|state| Arc::clone(&state.0.stack));
        if self.exceeds_line_length(line) {
            return Ok(TokenizedLine {
                spans: vec![0, UNSTYLED_STYLE_ID],
                kinds: vec![Token {
                    start_byte: 0,
                    kind: self.token_kind(UNSTYLED_STYLE_ID),
                }],
                end_state: LineState(EngineState {
                    stack: previous_stack.unwrap_or_else(StateStack::null),
                }),
                is_stopped_early: false,
            });
        }
        let time_limit = self
            .limits
            .line_time_limit_millis
            .map_or(UNLIMITED_LINE_TIME, NonZeroU64::get);
        let result = grammar
            .tokenize_line2(line, previous_stack, time_limit)
            .map_err(|error| SyntaxError::Tokenization(error.to_string()))?;
        let mut cursor = Utf16ByteCursor::new(line);
        let mut spans = Vec::with_capacity(result.tokens.len());
        let mut kinds: Vec<Token> = Vec::new();
        for [utf16_start, metadata] in result.tokens.as_chunks::<ENGINE_TOKEN_FIELDS>().0 {
            let attributes = EncodedTokenAttributes::new(*metadata);
            let style = style_id(
                attributes.foreground(),
                attributes.font_style().bits().cast_unsigned(),
            );
            let start_byte = cursor.byte_offset(*utf16_start as usize);
            if spans.last() == Some(&style) {
                continue;
            }
            spans.push(u32::try_from(start_byte).unwrap_or(u32::MAX));
            spans.push(style);
            let kind = self.token_kind(style);
            if kinds.last().map(|token| token.kind) != Some(kind) {
                kinds.push(Token { start_byte, kind });
            }
        }
        Ok(TokenizedLine {
            spans,
            kinds,
            end_state: LineState(EngineState {
                stack: result.rule_stack,
            }),
            is_stopped_early: result.stopped_early,
        })
    }

    fn exceeds_line_length(&self, line: &str) -> bool {
        line.len() >= self.limits.max_line_utf16_length
            && utf16_len(line) >= self.limits.max_line_utf16_length
    }

    fn grammar(&mut self, language_id: &str) -> Result<Rc<Grammar>, SyntaxError> {
        let scope_name = self
            .scope_name_by_language_id
            .get(language_id)
            .ok_or_else(|| SyntaxError::UnknownLanguage(language_id.to_owned()))?;
        if let Some(grammar) = self.grammar_by_scope_name.get(scope_name) {
            return Ok(Rc::clone(grammar));
        }
        let configuration = GrammarConfiguration::default()
            .with_initial_language_id(ROOT_LANGUAGE_ID)
            .with_balanced_bracket_selectors(Some(vec![ANY_BALANCED_BRACKET_SELECTOR.to_owned()]));
        let grammar = self
            .registry
            .grammar_for_scope_name(scope_name, configuration)
            .map_err(|error| SyntaxError::InvalidTheme(error.to_string()))?
            .ok_or_else(|| SyntaxError::UnknownLanguage(language_id.to_owned()))?;
        self.grammar_by_scope_name
            .insert(scope_name.clone(), Rc::clone(&grammar));
        Ok(grammar)
    }
}

impl GrammarTokenizer for TextmateTokenizer {
    fn tokenize_line(
        &mut self,
        language_id: &str,
        line: &str,
        previous: Option<&LineState>,
    ) -> Option<TokenizedLine> {
        self.try_tokenize_line(language_id, line, previous).ok()
    }

    fn is_same_state(&self, left: &LineState, right: &LineState) -> bool {
        left.is_same(right)
    }
}

fn raw_theme(theme: &[ThemeSetting]) -> RawTheme {
    RawTheme::default().with_settings(
        theme
            .iter()
            .map(|setting| {
                RawThemeSetting::default()
                    .with_scope(setting.scope.clone().map(RawThemeScope::Array))
                    .with_settings(setting.settings.as_ref().map(|style| {
                        RawThemeStyle::default()
                            .with_foreground(style.foreground.clone())
                            .with_background(style.background.clone())
                            .with_font_style(style.font_style.clone())
                    }))
            })
            .collect(),
    )
}
