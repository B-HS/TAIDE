use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{MarkupLimits, parse_stylesheet};

const LIMITS: MarkupLimits = MarkupLimits {
    max_bytes: 16 * 1024,
    max_nodes: 2048,
    max_nesting: 32,
};
const MISMATCH_SAMPLE_COUNT: usize = 8;

include!("fixtures/emmet-stylesheet-parser-oracle.rs");

#[path = "fixtures/emmet-stylesheet-canonical.rs"]
mod canonical;

use canonical::canonical;

#[test]
fn stylesheet구조는_원본기본snippet과_value모드_ast에_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(is_value, input, expected)| {
            let actual = parse_stylesheet(input, is_value, LIMITS)
                .ok()
                .map(|properties| canonical(&properties));
            let corrected = match (is_value, input) {
                (false, "f:'a\\'b'") => {
                    assert_eq!(expected, Some("1:fi0[[ssingle:2:a\\l1:bssingle:0:]]"));
                    Some("1:fi0[[ssingle:4:a\\'b]]")
                }
                (true, "f:'a\\'b'") => {
                    assert_eq!(expected, Some("-i0[[l1:fssingle:2:a\\l1:bssingle:0:]]"));
                    Some("-i0[[l1:fssingle:4:a\\'b]]")
                }
                (false, "m${1:foo\\}bar}") => {
                    assert_eq!(expected, None);
                    Some("1:mi0[[f1:0:8:foo\\}bar]]")
                }
                (true, "m${1:foo\\}bar}") => {
                    assert_eq!(expected, None);
                    Some("-i0[[l1:mf1:1:8:foo\\}bar]]")
                }
                _ => expected,
            };
            (actual.as_deref() != corrected)
                .then(|| format!("{is_value}/{input:?}: actual={actual:?}, expected={corrected:?}"))
        })
        .collect::<Vec<_>>();
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(MISMATCH_SAMPLE_COUNT)].join("\n")
    );
}

#[test]
fn stylesheet구조는_입력_token_깊이_예산과_비유한숫자를_거절한다() {
    for (input, limits) in [
        (
            "m10",
            MarkupLimits {
                max_bytes: 1,
                ..LIMITS
            },
        ),
        (
            "m10",
            MarkupLimits {
                max_nodes: 1,
                ..LIMITS
            },
        ),
        (
            "foo(bar(1))",
            MarkupLimits {
                max_nesting: 1,
                ..LIMITS
            },
        ),
        (
            "m${1:a{b}}",
            MarkupLimits {
                max_nesting: 0,
                ..LIMITS
            },
        ),
    ] {
        assert_eq!(
            parse_stylesheet(input, false, limits),
            Err(EditorError::Capacity),
            "{input}"
        );
    }
    let overflow = format!("m{}", "9".repeat(LIMITS.max_nodes));
    assert_eq!(
        parse_stylesheet(&overflow, false, LIMITS),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        parse_stylesheet("foo)", false, LIMITS),
        Err(EditorError::Refused)
    );
}

#[test]
fn stylesheet토큰의_원본범위는_utf16이며_함수는_합성값으로_구분된다() {
    let properties = parse_stylesheet("m\"\u{1f642}\"${1}", false, LIMITS).unwrap();
    let tokens = &properties[0].values[0];
    assert_eq!(tokens[0].source, Some(1..5));
    assert_eq!(tokens[1].source, Some(5..9));
    let properties = parse_stylesheet("foo(1)${1}", false, LIMITS).unwrap();
    assert_eq!(properties[0].values[0][0].source, None);
}
