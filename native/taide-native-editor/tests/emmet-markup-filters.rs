use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{
    ConversionLimits, MarkupFilters, MarkupLimits, Syntax, expand_markup_with_filters,
    markup_filters, prepare_markup, prepare_markup_with_filters,
};

const PARSE_LIMITS: MarkupLimits = MarkupLimits {
    max_bytes: 4096,
    max_nodes: 256,
    max_nesting: 32,
};
const LIMITS: ConversionLimits = ConversionLimits {
    max_repeats: 1000,
    max_nodes: 2048,
    max_bytes: 256 * 1024,
    max_nesting: 32,
};
const BEM_BYTE_BUDGET: usize = "divclassabcdefghijklmnopqrstdivclass-abcdefghijkl".len();
const BEM_BUDGET_INPUT: &str = ".abcdefghijklmnopqrst>.-abcdefghijkl";

include!("fixtures/emmet-markup-filters-oracle.rs");

#[test]
fn emmet_markup의_bem_주석_trim접미사는_실제_원본_completion과_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(syntax, input, expected)| {
            let syntax = if syntax == "jsx" {
                Syntax::Jsx
            } else {
                Syntax::Html
            };
            let actual = expand_markup_with_filters(input, syntax, PARSE_LIMITS, LIMITS).ok();
            (actual.as_deref() != expected)
                .then(|| format!("{syntax:?} {input:?}: actual={actual:?}, expected={expected:?}"))
        })
        .collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn emmet_markup필터는_세접미사와_알려진_이름만_분리한다() {
    let (input, filters) = markup_filters("div|bem|c|t");
    assert_eq!(input, "div");
    assert!(filters.is_bem_enabled && filters.is_comment_enabled && filters.has_trim_filter);
    assert_eq!(markup_filters("div|c|c|c|c").0, "div|c");
    assert_eq!(markup_filters("div|unknown").0, "div|unknown");
}

#[test]
fn emmet_markup필터의_출력과_bem_byte예산은_부분결과를_반환하지않는다() {
    assert_eq!(
        expand_markup_with_filters(
            "div.block>.-element|bem|c",
            Syntax::Html,
            PARSE_LIMITS,
            ConversionLimits {
                max_bytes: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    let limits = ConversionLimits {
        max_bytes: BEM_BYTE_BUDGET,
        ..LIMITS
    };
    assert!(prepare_markup(BEM_BUDGET_INPUT, Syntax::Html, PARSE_LIMITS, limits).is_ok());
    assert_eq!(
        prepare_markup_with_filters(
            BEM_BUDGET_INPUT,
            Syntax::Html,
            PARSE_LIMITS,
            limits,
            MarkupFilters {
                is_bem_enabled: true,
                ..Default::default()
            },
        ),
        Err(EditorError::Capacity)
    );
}
