use std::num::NonZeroU64;

use taide_native_editor::syntax::{Token, TokenKind};
use taide_native_syntax::{
    GrammarTokenizer, SPAN_FIELDS, TextmateTokenizer, TokenizerLimits, UNSTYLED_STYLE_ID,
};

use crate::reference::{MAX_LINE_UTF16_LENGTH, THEME_IDS, theme_reference, tokenizer};

const LANGUAGE_ID: &str = "javascript";
const SLOW_LANGUAGE_ID: &str = "typescript";
const COMMENT_OPENING_LINE: &str = "/* open";
const COMMENT_CLOSING_LINE: &str = "still comment */ const a = 1";
const STATEMENT_LINE: &str = "const a = 1";
const REPEATED_STATEMENT: &str = "value = other + 1; ";
const REPEATED_STATEMENT_COUNT: usize = 1_000;
const BMP_CHARACTER: &str = "한";
const ASTRAL_CHARACTER: &str = "😀";
const ASTRAL_UTF16_UNITS: usize = 2;
const FILL: &str = "x";
const SHORTEST_LINE_TIME_LIMIT_MILLIS: u64 = 1;

fn javascript_tokenizer() -> TextmateTokenizer {
    tokenizer(
        &theme_reference(THEME_IDS[0]),
        &[LANGUAGE_ID],
        TokenizerLimits::default(),
    )
}

fn unstyled_line_start() -> Vec<Token> {
    vec![Token {
        start_byte: 0,
        kind: TokenKind::Other,
    }]
}

#[test]
fn 한도_길이_이상인_줄은_토큰화하지_않고_앞_줄의_상태를_그대로_넘긴다() {
    let mut tokenizer = javascript_tokenizer();
    let opened = tokenizer
        .try_tokenize_line(LANGUAGE_ID, COMMENT_OPENING_LINE, None)
        .unwrap();
    let comment_style = opened.spans[1];
    assert_ne!(comment_style, UNSTYLED_STYLE_ID);

    let skipped = tokenizer
        .try_tokenize_line(
            LANGUAGE_ID,
            &FILL.repeat(MAX_LINE_UTF16_LENGTH),
            Some(&opened.end_state),
        )
        .unwrap();
    assert_eq!(skipped.spans, [0, UNSTYLED_STYLE_ID]);
    assert_eq!(skipped.kinds, unstyled_line_start());
    assert!(!skipped.is_stopped_early);
    assert!(tokenizer.is_same_state(&skipped.end_state, &opened.end_state));

    let closed = tokenizer
        .try_tokenize_line(LANGUAGE_ID, COMMENT_CLOSING_LINE, Some(&skipped.end_state))
        .unwrap();
    assert_eq!(closed.spans[1], comment_style);
    assert!(closed.spans.len() > SPAN_FIELDS);
    assert!(!tokenizer.is_same_state(&closed.end_state, &opened.end_state));
}

#[test]
fn 한도는_바이트_수가_아니라_utf16_길이로_잰다() {
    let mut tokenizer = javascript_tokenizer();
    let opened = tokenizer
        .try_tokenize_line(LANGUAGE_ID, COMMENT_OPENING_LINE, None)
        .unwrap();
    let comment_style = opened.spans[1];

    let below_limit = BMP_CHARACTER.repeat(MAX_LINE_UTF16_LENGTH - 1);
    assert!(below_limit.len() > MAX_LINE_UTF16_LENGTH);
    let tokenized = tokenizer
        .try_tokenize_line(LANGUAGE_ID, &below_limit, Some(&opened.end_state))
        .unwrap();
    assert_eq!(tokenized.spans, [0, comment_style]);

    let at_limit = ASTRAL_CHARACTER.repeat(MAX_LINE_UTF16_LENGTH / ASTRAL_UTF16_UNITS);
    let skipped = tokenizer
        .try_tokenize_line(LANGUAGE_ID, &at_limit, Some(&opened.end_state))
        .unwrap();
    assert_eq!(skipped.spans, [0, UNSTYLED_STYLE_ID]);
}

#[test]
fn 첫_줄이_한도를_넘으면_다음_줄은_문서_처음과_같은_상태에서_토큰화된다() {
    let mut tokenizer = javascript_tokenizer();
    let skipped = tokenizer
        .try_tokenize_line(LANGUAGE_ID, &FILL.repeat(MAX_LINE_UTF16_LENGTH), None)
        .unwrap();
    assert_eq!(skipped.spans, [0, UNSTYLED_STYLE_ID]);
    let after = tokenizer
        .try_tokenize_line(LANGUAGE_ID, STATEMENT_LINE, Some(&skipped.end_state))
        .unwrap();
    let fresh = tokenizer
        .try_tokenize_line(LANGUAGE_ID, STATEMENT_LINE, None)
        .unwrap();
    assert_eq!(after.spans, fresh.spans);
    assert_eq!(after.kinds, fresh.kinds);
    assert!(tokenizer.is_same_state(&after.end_state, &fresh.end_state));
}

#[test]
fn 줄당_시간_한도를_넘기면_줄_중간에서_멈추고_멈춘_사실을_알린다() {
    let theme = theme_reference(THEME_IDS[0]);
    let line = REPEATED_STATEMENT.repeat(REPEATED_STATEMENT_COUNT);
    assert!(line.len() < MAX_LINE_UTF16_LENGTH);

    let mut limited = tokenizer(
        &theme,
        &[SLOW_LANGUAGE_ID],
        TokenizerLimits {
            line_time_limit_millis: NonZeroU64::new(SHORTEST_LINE_TIME_LIMIT_MILLIS),
            ..TokenizerLimits::default()
        },
    );
    let stopped = limited
        .try_tokenize_line(SLOW_LANGUAGE_ID, &line, None)
        .unwrap();
    assert!(stopped.is_stopped_early);
    assert_eq!(stopped.spans[0], 0);

    let mut unlimited = tokenizer(
        &theme,
        &[SLOW_LANGUAGE_ID],
        TokenizerLimits {
            line_time_limit_millis: None,
            ..TokenizerLimits::default()
        },
    );
    let complete = unlimited
        .try_tokenize_line(SLOW_LANGUAGE_ID, &line, None)
        .unwrap();
    assert!(!complete.is_stopped_early);
    assert!(complete.spans.len() > stopped.spans.len());

    let next = limited
        .tokenize_line(SLOW_LANGUAGE_ID, STATEMENT_LINE, Some(&stopped.end_state))
        .unwrap();
    assert_eq!(next.spans[0], 0);
}

#[test]
fn 싣지_않은_언어는_토큰화하지_않는다() {
    let mut tokenizer = javascript_tokenizer();
    assert!(
        tokenizer
            .tokenize_line(SLOW_LANGUAGE_ID, STATEMENT_LINE, None)
            .is_none()
    );
}
