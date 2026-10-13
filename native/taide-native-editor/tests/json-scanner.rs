use taide_native_editor::document::EditorError;
use taide_native_editor::json_scanner::{Error, Kind, Limits, Token, scan};

const LIMITS: Limits = Limits {
    max_bytes: 64 * 1024,
    max_tokens: 128 * 1024,
    max_value_units: 64 * 1024,
};
const KINDS: &[Kind] = &[
    Kind::OpenBrace,
    Kind::CloseBrace,
    Kind::OpenBracket,
    Kind::CloseBracket,
    Kind::Comma,
    Kind::Colon,
    Kind::Null,
    Kind::True,
    Kind::False,
    Kind::String,
    Kind::Number,
    Kind::LineComment,
    Kind::BlockComment,
    Kind::LineBreak,
    Kind::Whitespace,
    Kind::Unknown,
    Kind::End,
];
const ERRORS: &[Error] = &[
    Error::EndOfComment,
    Error::EndOfString,
    Error::EndOfNumber,
    Error::InvalidUnicode,
    Error::InvalidEscape,
    Error::InvalidCharacter,
];
const SAMPLE_COUNT: usize = 8;

include!("fixtures/json-scanner-oracle.rs");

fn describe(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| {
            let kind = KINDS.iter().position(|kind| *kind == token.kind).unwrap() + 1;
            let error = token.error.map_or(0, |error| {
                ERRORS.iter().position(|current| *current == error).unwrap() + 1
            });
            let value = token
                .value
                .iter()
                .map(|value| format!("{value:x}"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{kind}:{}..{}:{}:{}:{error}:{value};",
                token.utf16.start, token.utf16.end, token.line, token.character
            )
        })
        .collect()
}

#[test]
fn json_scanner는_실제원본의_값_오류_위치와_명시적인_범위수정에_일치한다() {
    let mut mismatches = Vec::new();
    for &(input, ignore_trivia, raw, expected) in CASES {
        let actual = scan(input, ignore_trivia, LIMITS).map(|tokens| describe(&tokens));
        if raw != expected {
            assert!(
                input.contains("//")
                    || input.contains("/*")
                    || input.contains("\\\n")
                    || input.contains("\\\r")
            );
        }
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!("{input:?}/{ignore_trivia}: actual={actual:?}, expected={expected:?}, source={raw:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(SAMPLE_COUNT)].join("\n")
    );
}

#[test]
fn json_scanner의_원본byte범위는_utf8경계와_utf16좌표를_정확히_보존한다() {
    for &(input, ignore_trivia, _, _) in CASES {
        let tokens = scan(input, ignore_trivia, LIMITS).unwrap();
        for token in tokens {
            assert!(token.bytes.start <= token.bytes.end && token.bytes.end <= input.len());
            assert!(
                input.is_char_boundary(token.bytes.start)
                    && input.is_char_boundary(token.bytes.end)
            );
            assert_eq!(
                input[..token.bytes.start].encode_utf16().count(),
                token.utf16.start
            );
            assert_eq!(
                input[..token.bytes.end].encode_utf16().count(),
                token.utf16.end
            );
        }
    }
    let tokens = scan("\"\\uD800\\uDC00\\uDFFF\"", false, LIMITS).unwrap();
    assert_eq!(tokens[0].value, [0xd800, 0xdc00, 0xdfff]);
}

#[test]
fn json_scanner는_trivia를_제외해도_전체토큰과_논리값_예산을_부분결과없이_지킨다() {
    for ignore in [false, true] {
        assert_eq!(
            scan(
                "{}",
                ignore,
                Limits {
                    max_bytes: 1,
                    ..LIMITS
                }
            ),
            Err(EditorError::Capacity)
        );
        assert_eq!(
            scan(
                " \n",
                ignore,
                Limits {
                    max_tokens: 1,
                    ..LIMITS
                }
            ),
            Err(EditorError::Capacity)
        );
        assert_eq!(
            scan(
                "\"ab\" \"cd\"",
                ignore,
                Limits {
                    max_value_units: 3,
                    ..LIMITS
                }
            ),
            Err(EditorError::Capacity)
        );
        assert_eq!(
            scan(
                "\"\\uD800\"",
                ignore,
                Limits {
                    max_value_units: 0,
                    ..LIMITS
                }
            ),
            Err(EditorError::Capacity)
        );
        assert_eq!(
            scan(
                "",
                ignore,
                Limits {
                    max_tokens: 0,
                    ..LIMITS
                }
            ),
            Err(EditorError::Capacity)
        );
    }
}
