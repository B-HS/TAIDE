use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{
    ConversionLimits, MarkupLimits, Syntax, expand_markup_with_random,
};
use taide_native_editor::snippet_syntax::{ParseLimits, parse};

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
const MULTIPLIER: u32 = 1_664_525;
const INCREMENT: u32 = 1_013_904_223;
const STALLED_BYTE_BUDGET: usize = 1024;
const SMALL_BYTE_BUDGET: usize = 128;
const LARGE_WORD_COUNT: &str = "lorem999999999999999999999999999999";
const MISMATCH_SAMPLE_COUNT: usize = 8;
const SNIPPET_LIMITS: ParseLimits = ParseLimits {
    max_bytes: LIMITS.max_bytes,
    max_markers: LIMITS.max_nodes,
    max_nesting: LIMITS.max_nesting,
};

include!("fixtures/emmet-markup-lorem-oracle.rs");

#[test]
fn lorem의_세언어_단어수와_반복_필터출력은_동일난수의_원본과_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(syntax, seed, input, expected)| {
            let syntax = if syntax == "jsx" {
                Syntax::Jsx
            } else {
                Syntax::Html
            };
            let mut state = seed;
            let mut random = || {
                state = state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
                state
            };
            let actual =
                expand_markup_with_random(input, syntax, PARSE_LIMITS, LIMITS, &mut random).ok();
            if let Some(output) = &actual {
                assert!(
                    parse(output, SNIPPET_LIMITS, |_, _| false).is_ok(),
                    "invalid snippet: {input:?} {output:?}"
                );
            }
            (actual.as_deref() != expected).then(|| {
                format!("{syntax:?}/{seed} {input:?}: actual={actual:?}, expected={expected:?}")
            })
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
fn lorem이아닌_태그는_난수를_소비하지않는다() {
    let mut random = || panic!("unexpected random draw");
    assert_eq!(
        expand_markup_with_random("div>p", Syntax::Html, PARSE_LIMITS, LIMITS, &mut random),
        Ok("<div>\n\t<p>${0}</p>\n</div>".into())
    );
}

#[test]
fn lorem의_초과생성과_정체난수는_부분결과없이_제한된다() {
    let mut draws = 0;
    let mut stalled = || {
        draws += 1;
        0
    };
    assert_eq!(
        expand_markup_with_random(
            "lorem100",
            Syntax::Html,
            PARSE_LIMITS,
            ConversionLimits {
                max_bytes: STALLED_BYTE_BUDGET,
                ..LIMITS
            },
            &mut stalled
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(draws, STALLED_BYTE_BUDGET);
    let mut random = || 0;
    for input in [LARGE_WORD_COUNT, "lorem100", "p>loremru8*7"] {
        assert_eq!(
            expand_markup_with_random(
                input,
                Syntax::Html,
                PARSE_LIMITS,
                ConversionLimits {
                    max_bytes: SMALL_BYTE_BUDGET,
                    ..LIMITS
                },
                &mut random
            ),
            Err(EditorError::Capacity),
            "{input}"
        );
    }
}
