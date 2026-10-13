use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{
    ConversionLimits, MarkupLimits, MarkupValue, Syntax, expand_markup, prepare_markup,
    render_markup,
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
const UPSTREAM_GROUP_CLOSING_INPUT: &str = "div[foo=(bar)]";
const UPSTREAM_LITERAL_FIELD_INPUT: &str = "p{\\${1}}";
const UPSTREAM_QUOTED_ATTRIBUTE_INPUT: &str = "div[title=\"a\\\"b\"]";
const SNIPPET_LIMITS: taide_native_editor::snippet_syntax::ParseLimits =
    taide_native_editor::snippet_syntax::ParseLimits {
        max_bytes: LIMITS.max_bytes,
        max_nesting: LIMITS.max_nesting,
        max_markers: LIMITS.max_nodes,
    };

include!("fixtures/emmet-markup-output-oracle.rs");

#[test]
fn emmet_markup의_html_jsx_출력은_원본_snippet_필드_들여쓰기와_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(syntax, input, expected)| {
            let syntax = if syntax == "jsx" {
                Syntax::Jsx
            } else {
                Syntax::Html
            };
            let actual = expand_markup(input, syntax, PARSE_LIMITS, LIMITS).ok();
            let corrected = expected.map(|expected| {
                if input == UPSTREAM_GROUP_CLOSING_INPUT {
                    assert!(expected.contains("(bar}"));
                    return expected.replace("(bar}", "(bar)");
                }
                if input == UPSTREAM_LITERAL_FIELD_INPUT {
                    assert_eq!(expected, "<p>${0}</p>");
                    return "<p>\\${1}</p>".into();
                }
                if input == UPSTREAM_QUOTED_ATTRIBUTE_INPUT {
                    assert_eq!(expected, "<div title=\"a\"b\">${0}</div>");
                    return "<div title=\"a&quot;b\">${0}</div>".into();
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
fn emmet_markup의_escaped_field는_실제_snippet소비에서_본문문자로_보존된다() {
    let output = expand_markup(
        UPSTREAM_LITERAL_FIELD_INPUT,
        Syntax::Html,
        PARSE_LIMITS,
        LIMITS,
    )
    .unwrap();
    let markers =
        taide_native_editor::snippet_syntax::parse(&output, SNIPPET_LIMITS, |_, _| false).unwrap();
    let expanded = taide_native_editor::snippet_expansion::expand(
        &markers,
        SNIPPET_LIMITS,
        |_| Ok(None),
        |_, _| Err(EditorError::Refused),
    )
    .unwrap();
    assert_eq!(expanded.text, "<p>${1}</p>");
    assert!(expanded.placeholders.is_empty());
}

#[test]
fn emmet_markup출력은_현재_native_snippet문법으로_소비된다() {
    for &(syntax, input, _) in CASES {
        let syntax = if syntax == "jsx" {
            Syntax::Jsx
        } else {
            Syntax::Html
        };
        let Ok(output) = expand_markup(input, syntax, PARSE_LIMITS, LIMITS) else {
            continue;
        };
        assert!(
            taide_native_editor::snippet_syntax::parse(&output, SNIPPET_LIMITS, |_, _| false)
                .is_ok(),
            "invalid native snippet: {input:?} {output:?}"
        );
    }
}

#[test]
fn emmet_markup출력은_예산초과와_css를_거절한다() {
    let nodes = prepare_markup("div>span", Syntax::Html, PARSE_LIMITS, LIMITS).unwrap();
    assert_eq!(
        render_markup(&nodes, Syntax::Html, 1, LIMITS.max_nesting),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        render_markup(&nodes, Syntax::Html, LIMITS.max_bytes, 0),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        render_markup(&nodes, Syntax::Css, LIMITS.max_bytes, LIMITS.max_nesting),
        Err(EditorError::Refused)
    );
    let mut fields = nodes;
    fields[0].value = Some(vec![MarkupValue::Field {
        index: usize::MAX,
        name: String::new(),
    }]);
    fields[0].children.clear();
    assert_eq!(
        render_markup(&fields, Syntax::Html, LIMITS.max_bytes, LIMITS.max_nesting),
        Err(EditorError::Capacity)
    );
}
