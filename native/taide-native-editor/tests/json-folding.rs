use taide_native_editor::document::EditorError;
use taide_native_editor::json_folding::{Folding, Limits, folding_ranges};
use taide_native_editor::json_scanner;

const LIMITS: Limits = Limits {
    scanner: json_scanner::Limits {
        max_bytes: 64 * 1024,
        max_tokens: 128 * 1024,
        max_value_units: 128 * 1024,
    },
    max_work_bytes: 4 * 1024 * 1024,
    max_depth: 16 * 1024,
    max_ranges: 16 * 1024,
    result_limit: None,
};
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 4096;
const SOURCE_FOLDING_LEVEL_LIMIT: usize = 30;

include!("fixtures/json-folding-oracle.rs");

fn describe(folding: &Folding) -> String {
    let ranges = folding
        .ranges
        .iter()
        .map(|range| {
            format!(
                "{}..{}:{};",
                range.region.start_line,
                range.region.end_line,
                range.kind.as_deref().unwrap()
            )
        })
        .collect::<String>();
    format!("{ranges}|{}", folding.limit_exceeded)
}

#[test]
fn json_folding은_실제원본의_줄_주석_region_회복과_결과제한에_일치한다() {
    let mut mismatches = Vec::new();
    for &(input, result_limit, raw, expected) in CASES {
        let actual = folding_ranges(
            input,
            Limits {
                result_limit,
                ..LIMITS
            },
        )
        .map(|folding| describe(&folding));
        if raw != expected {
            assert!(
                input.contains("endregion")
                    || input.matches("{\n").count() >= SOURCE_FOLDING_LEVEL_LIMIT
                    || input.matches("[\n").count() >= SOURCE_FOLDING_LEVEL_LIMIT,
                "Unexpected source correction: {input:?}/{result_limit:?}: {raw}/{expected}"
            );
        }
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!("{input:?}/{result_limit:?}: actual={actual:?}, expected={expected:?}, source={raw:?}"));
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
fn json_folding은_실제문서_줄경계_닫힌줄과_중복시작을_보존한다() {
    for &(input, result_limit, _, _) in CASES {
        let folding = folding_ranges(
            input,
            Limits {
                result_limit,
                ..LIMITS
            },
        )
        .unwrap();
        let mut line_count = 1;
        let mut characters = input.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\n' || character == '\r' && characters.peek() != Some(&'\n') {
                line_count += 1;
            }
        }
        for range in &folding.ranges {
            assert!(
                range.region.start_line < range.region.end_line
                    && range.region.end_line < line_count
            );
        }
        if let Some(limit) = result_limit {
            assert!(folding.ranges.len() <= limit);
        }
    }
    let folding = folding_ranges("{\n0\n}", LIMITS).unwrap();
    assert_eq!(folding.ranges[0].region.start_line, 0);
    assert_eq!(folding.ranges[0].region.end_line, 1);
    let folding = folding_ranges(
        "// #region\n// ordinary endregion text\n0\n// #endregion",
        LIMITS,
    )
    .unwrap();
    assert_eq!(folding.ranges[0].region.end_line, 3);
}

#[test]
fn json_folding은_명시스택과_재탐색을_포함한_전체작업예산을_지킨다() {
    let input = format!(
        "{}0\n{}",
        "[\n".repeat(DEEP_NESTING),
        "]\n".repeat(DEEP_NESTING)
    );
    assert_eq!(
        folding_ranges(&input, LIMITS).unwrap().ranges.len(),
        DEEP_NESTING
    );
    assert!(matches!(
        folding_ranges(
            &input,
            Limits {
                max_depth: DEEP_NESTING - 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        folding_ranges(
            &input,
            Limits {
                max_ranges: DEEP_NESTING - 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    let incomplete = "/*\n/*\n[\n0\n]\n";
    assert_eq!(folding_ranges(incomplete, LIMITS).unwrap().ranges.len(), 1);
    assert!(matches!(
        folding_ranges(
            incomplete,
            Limits {
                max_work_bytes: incomplete.len(),
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        folding_ranges(
            incomplete,
            Limits {
                scanner: json_scanner::Limits {
                    max_tokens: 2,
                    ..LIMITS.scanner
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        folding_ranges(
            incomplete,
            Limits {
                scanner: json_scanner::Limits {
                    max_value_units: incomplete.encode_utf16().count(),
                    ..LIMITS.scanner
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        folding_ranges(
            "{}",
            Limits {
                scanner: json_scanner::Limits {
                    max_bytes: 1,
                    ..LIMITS.scanner
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        folding_ranges(
            "{}",
            Limits {
                max_work_bytes: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        folding_ranges(
            "",
            Limits {
                max_work_bytes: 0,
                max_ranges: 0,
                max_depth: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .ranges
        .is_empty()
    );
}
