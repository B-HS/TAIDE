use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{ConversionLimits, MarkupLimits, prepare_stylesheet};

const PARSE_LIMITS: MarkupLimits = MarkupLimits {
    max_bytes: 16 * 1024,
    max_nodes: 2048,
    max_nesting: 32,
};
const LIMITS: ConversionLimits = ConversionLimits {
    max_repeats: 7,
    max_bytes: 256 * 1024,
    max_nodes: 8192,
    max_nesting: 32,
};
const MISMATCH_SAMPLE_COUNT: usize = 8;
const RAW_BYTE_BUDGET: usize = 16;
const PROPERTIES_FOR_BUDGET: &str = "@ff+@ff";
const SINGLE_IMPORT_BYTE_BUDGET: usize = "@import url();".len();
const SINGLE_DISPLAY_NODE_BUDGET: usize = 2;

#[path = "fixtures/emmet-stylesheet-canonical.rs"]
mod canonical;

include!("fixtures/emmet-stylesheet-preparation-oracle.rs");

#[test]
fn stylesheet의_registry_keyword_fuzzy_단위와_기본값은_원본_준비ast와_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(input, expected)| {
            let actual = prepare_stylesheet(input, PARSE_LIMITS, LIMITS)
                .ok()
                .map(|properties| canonical::canonical(&properties));
            (actual.as_deref() != expected)
                .then(|| format!("{input:?}: actual={actual:?}, expected={expected:?}"))
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
fn stylesheet_준비는_raw확장과_토큰_깊이예산을_부분결과없이_거절한다() {
    for (input, limits) in [
        (
            PROPERTIES_FOR_BUDGET,
            ConversionLimits {
                max_bytes: RAW_BYTE_BUDGET,
                ..LIMITS
            },
        ),
        (
            "d",
            ConversionLimits {
                max_nodes: 1,
                ..LIMITS
            },
        ),
        (
            "lg(red,blue)",
            ConversionLimits {
                max_nesting: 0,
                ..LIMITS
            },
        ),
    ] {
        assert_eq!(
            prepare_stylesheet(input, PARSE_LIMITS, limits),
            Err(EditorError::Capacity),
            "{input}"
        );
    }
    assert!(prepare_stylesheet("d", PARSE_LIMITS, LIMITS).is_ok());
    let limits = ConversionLimits {
        max_bytes: SINGLE_IMPORT_BYTE_BUDGET,
        ..LIMITS
    };
    assert!(prepare_stylesheet("@i", PARSE_LIMITS, limits).is_ok());
    assert_eq!(
        prepare_stylesheet("@i+@i", PARSE_LIMITS, limits),
        Err(EditorError::Capacity)
    );
    let limits = ConversionLimits {
        max_nodes: SINGLE_DISPLAY_NODE_BUDGET,
        ..LIMITS
    };
    assert!(prepare_stylesheet("d", PARSE_LIMITS, limits).is_ok());
    assert_eq!(
        prepare_stylesheet("d+d", PARSE_LIMITS, limits),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        prepare_stylesheet(
            "m10",
            MarkupLimits {
                max_bytes: 1,
                ..PARSE_LIMITS
            },
            LIMITS
        ),
        Err(EditorError::Capacity)
    );
}
