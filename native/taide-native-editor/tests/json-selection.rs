use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_selection::{Limits, selection_ranges};

const LIMITS: Limits = Limits {
    parser: json_parser::Limits {
        scanner: json_scanner::Limits {
            max_bytes: 64 * 1024,
            max_tokens: 128 * 1024,
            max_value_units: 64 * 1024,
        },
        max_nodes: 32 * 1024,
        max_depth: 16 * 1024,
        max_diagnostics: 32 * 1024,
        max_comments: 32 * 1024,
    },
    max_positions: 1024,
    max_ranges: 32 * 1024,
};
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 8192;

include!("fixtures/json-selection-oracle.rs");

#[test]
fn json_selection은_원본의_내용_노드_쉼표_부모와_문자경계를_보존한다() {
    let mut mismatches = Vec::new();
    for &(input, position, raw, expected) in CASES {
        let actual = selection_ranges(input, &[position], LIMITS).map(|selections| {
            selections[0]
                .iter()
                .map(|range| format!("{}..{};", range.utf16.start, range.utf16.end))
                .collect::<String>()
        });
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!(
                "{input:?}/{position}: actual={actual:?}, expected={expected:?}, source={raw:?}"
            ));
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
fn json_selection은_다중위치의_원문범위와_독립적인_순서를_유지한다() {
    for &(input, position, _, _) in CASES {
        let positions = [position, input.len(), 0, position];
        let groups = selection_ranges(input, &positions, LIMITS).unwrap();
        assert_eq!(groups.len(), positions.len());
        assert_eq!(groups[0], groups[3]);
        for (group, cursor) in groups.iter().zip(positions) {
            for range in group {
                assert!(range.bytes.start <= cursor && cursor <= range.bytes.end);
                assert!(
                    input.is_char_boundary(range.bytes.start)
                        && input.is_char_boundary(range.bytes.end)
                );
                assert!(range.bytes.end <= input.len());
                assert_eq!(
                    input[..range.bytes.start].encode_utf16().count(),
                    range.utf16.start
                );
                assert_eq!(
                    input[..range.bytes.end].encode_utf16().count(),
                    range.utf16.end
                );
            }
        }
    }
}

#[test]
fn json_selection은_깊은문서와_전체위치_범위_파서예산을_부분결과없이_검증한다() {
    let input = format!("{}0{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    let groups = selection_ranges(&input, &[DEEP_NESTING], LIMITS).unwrap();
    assert_eq!(groups[0].len(), DEEP_NESTING * 2 + 1);
    assert!(matches!(
        selection_ranges(
            &input,
            &[DEEP_NESTING],
            Limits {
                max_ranges: DEEP_NESTING,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        selection_ranges(
            "{}",
            &[0, 1],
            Limits {
                max_positions: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    let expected_count = selection_ranges("{\"a\":1}", &[5], LIMITS).unwrap()[0].len();
    assert!(matches!(
        selection_ranges(
            "{\"a\":1}",
            &[5, 5],
            Limits {
                max_ranges: expected_count,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        selection_ranges("𐐀", &[1], LIMITS),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        selection_ranges("{}", &[3], LIMITS),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        selection_ranges(
            "{}",
            &[0],
            Limits {
                parser: json_parser::Limits {
                    max_nodes: 0,
                    ..LIMITS.parser
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        selection_ranges(
            "{}",
            &[0],
            Limits {
                max_ranges: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        selection_ranges(
            "",
            &[],
            Limits {
                max_ranges: 0,
                max_positions: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        selection_ranges("", &[0], LIMITS).unwrap()[0][0].bytes,
        0..0
    );
}
