use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{
    ConversionLimits, MarkupLimits, MarkupValue, Syntax, convert_markup, parse_markup,
};

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
const VARIABLES: &[(&str, &str)] = &[("lang", "ko"), ("charset", "UTF-8"), ("locale", "ko-KR")];
const UPSTREAM_GROUP_CLOSING_INPUT: &str = "div[foo=(bar)]";

include!("fixtures/emmet-markup-conversion-oracle.rs");

#[path = "fixtures/emmet-markup-canonical.rs"]
mod comparison;

use comparison::canonical;

#[test]
fn emmet_markup의_반복_필드_변수_속성변환은_원본_conversion과_일치한다() {
    let mismatches = CASES.iter().filter_map(|&(syntax, with_variables, input, expected)| {
        let syntax = if syntax == "jsx" { Syntax::Jsx } else { Syntax::Html };
        let variables = if with_variables { VARIABLES } else { &[] };
        let actual = parse_markup(input, syntax, PARSE_LIMITS).and_then(|nodes| convert_markup(&nodes, variables, LIMITS)).ok().map(|nodes| canonical(&nodes));
        let corrected = expected.map(|expected| {
            if input == UPSTREAM_GROUP_CLOSING_INPUT {
                assert!(expected.contains("s5:(bar}"));
                return expected.replace("s5:(bar}", "s5:(bar)");
            }
            expected.into()
        });
        (actual != corrected).then(|| format!("{syntax:?} vars={with_variables} {input:?}: actual={actual:?}, expected={expected:?}"))
    }).collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn emmet_markup변환은_원본의_group닫힘_오탈자를_복제하지않는다() {
    let raw = parse_markup(UPSTREAM_GROUP_CLOSING_INPUT, Syntax::Html, PARSE_LIMITS).unwrap();
    let nodes = convert_markup(&raw, &[], LIMITS).unwrap();
    let attribute = &nodes[0].attributes.as_ref().unwrap()[0];
    assert_eq!(
        attribute.value.as_deref(),
        Some([MarkupValue::Text("(bar)".into())].as_slice())
    );
}

#[test]
fn emmet_markup변환의_노드_byte_깊이_상한은_중간결과를_반환하지않는다() {
    let nodes = parse_markup("div*3>span{hello}", Syntax::Html, PARSE_LIMITS).unwrap();
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
        ConversionLimits {
            max_repeats: 0,
            ..LIMITS
        },
    ] {
        assert_eq!(
            convert_markup(&nodes, &[], limits),
            Err(EditorError::Capacity)
        );
    }
}

#[test]
fn emmet_markup변환의_field이름은_실제_byte상한을_지킨다() {
    let fields = parse_markup("[${1}]", Syntax::Html, PARSE_LIMITS).unwrap();
    assert_eq!(
        convert_markup(
            &fields,
            &[],
            ConversionLimits {
                max_bytes: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
}
