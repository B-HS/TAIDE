use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{ConversionLimits, MarkupLimits, Syntax, prepare_markup};

const PARSE_LIMITS: MarkupLimits = MarkupLimits {
    max_bytes: 4096,
    max_nodes: 256,
    max_nesting: 32,
};
const LIMITS: ConversionLimits = ConversionLimits {
    max_repeats: 7,
    max_nodes: 2048,
    max_bytes: 256 * 1024,
    max_nesting: 32,
};
const UPSTREAM_GROUP_CLOSING_INPUT: &str = "div[foo=(bar)]";

#[path = "fixtures/emmet-markup-canonical.rs"]
mod comparison;

include!("fixtures/emmet-markup-preparation-oracle.rs");

#[test]
fn emmet_markup의_기본스니펫_alias_태그_속성준비는_원본_ast와_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(syntax, input, expected)| {
            let syntax = if syntax == "jsx" {
                Syntax::Jsx
            } else {
                Syntax::Html
            };
            let actual = prepare_markup(input, syntax, PARSE_LIMITS, LIMITS)
                .ok()
                .map(|nodes| comparison::canonical(&nodes));
            let corrected = expected.map(|expected| {
                if input == UPSTREAM_GROUP_CLOSING_INPUT {
                    assert!(expected.contains("s5:(bar}"));
                    return expected.replace("s5:(bar}", "s5:(bar)");
                }
                expected.into()
            });
            (actual != corrected)
                .then(|| format!("{syntax:?} {input:?}: actual={actual:?}, expected={expected:?}"))
        })
        .collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn emmet_markup준비는_alias_확장의_node_byte_depth예산을_지킨다() {
    for limits in [
        ConversionLimits {
            max_nodes: 1,
            ..LIMITS
        },
        ConversionLimits {
            max_bytes: 1,
            ..LIMITS
        },
        ConversionLimits {
            max_nesting: 0,
            ..LIMITS
        },
    ] {
        assert_eq!(
            prepare_markup("input:checkbox", Syntax::Html, PARSE_LIMITS, limits),
            Err(EditorError::Capacity)
        );
    }
}
