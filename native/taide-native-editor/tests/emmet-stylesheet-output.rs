use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{
    ConversionLimits, MarkupLimits, StylesheetValue, expand_stylesheet, prepare_stylesheet,
    render_stylesheet,
};
use taide_native_editor::snippet_expansion::expand;
use taide_native_editor::snippet_syntax::{Marker, ParseLimits, parse};

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
const SNIPPET_LIMITS: ParseLimits = ParseLimits {
    max_bytes: LIMITS.max_bytes,
    max_markers: LIMITS.max_nodes,
    max_nesting: LIMITS.max_nesting,
};
const MISMATCH_SAMPLE_COUNT: usize = 8;
const MARGIN_OUTPUT_BYTES: usize = "margin: ${0};".len();

include!("fixtures/emmet-stylesheet-output-oracle.rs");

fn consume(output: &str) -> String {
    let markers = parse(output, SNIPPET_LIMITS, |_, _| false).unwrap();
    expand(
        &markers,
        SNIPPET_LIMITS,
        |_| Ok(None),
        |_, _| Err(EditorError::Refused),
    )
    .unwrap()
    .text
}

fn corrected_text(input: &str) -> Option<&'static str> {
    match input {
        "$var10" => Some("$var: 10px;"),
        "m${1:a{b}c}" => Some("margin: a{b}c;"),
        "m${1:foo\\}bar}" => Some("margin: foo}bar;"),
        "f:'a\\'b'" => Some("font: 'a\\'b';"),
        "m1000000000000000000000000000000" => Some("margin: 1e+30px;"),
        "m${foo:bar}" => Some("margin: bar;"),
        "f:'\\$cost'" => Some("font: '\\$cost';"),
        "f:'\\${1}'" => Some("font: '${1}';"),
        "f:'\\\\${1}'" => Some("font: '\\\\';"),
        _ => None,
    }
}

#[test]
fn stylesheet출력은_프로젝트_css_scss_snippet과_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(syntax, input, expected)| {
            let actual = expand_stylesheet(input, PARSE_LIMITS, LIMITS)
                .ok()
                .map(|output| consume(&output));
            let expected = if input == "m${}" {
                assert_eq!(expected, Some("margin: ${undefined};"));
                None
            } else {
                corrected_text(input)
                    .map(str::to_owned)
                    .or_else(|| expected.map(consume))
            };
            (actual != expected)
                .then(|| format!("{syntax}/{input:?}: actual={actual:?}, expected={expected:?}"))
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
fn stylesheet출력은_현재_native_snippet문법으로_소비된다() {
    for &(_, input, _) in CASES {
        if let Ok(output) = expand_stylesheet(input, PARSE_LIMITS, LIMITS) {
            assert!(
                parse(&output, SNIPPET_LIMITS, |_, _| false).is_ok(),
                "{input:?} {output:?}"
            );
        }
    }
}

#[test]
fn stylesheet_renderer는_출력과_함수깊이의_상한을_지킨다() {
    let properties = prepare_stylesheet("m", PARSE_LIMITS, LIMITS).unwrap();
    assert_eq!(
        render_stylesheet(&properties, MARGIN_OUTPUT_BYTES, LIMITS.max_nesting),
        Ok("margin: ${0};".into())
    );
    assert_eq!(
        render_stylesheet(&properties, MARGIN_OUTPUT_BYTES - 1, LIMITS.max_nesting),
        Err(EditorError::Capacity)
    );
    let properties = prepare_stylesheet("lg(red,blue)", PARSE_LIMITS, LIMITS).unwrap();
    assert_eq!(
        render_stylesheet(&properties, LIMITS.max_bytes, 0),
        Err(EditorError::Capacity)
    );
    let mut properties = prepare_stylesheet("m10", PARSE_LIMITS, LIMITS).unwrap();
    properties[0].values[0][0].value = StylesheetValue::Number {
        value: f64::INFINITY,
        raw: String::new(),
        unit: String::new(),
    };
    assert_eq!(
        render_stylesheet(&properties, LIMITS.max_bytes, LIMITS.max_nesting),
        Err(EditorError::Refused)
    );
}

#[test]
fn css_이름있는변수는_undefined로_바뀌지않으며_과학표기는_지수를_보존한다() {
    let output = expand_stylesheet("m${foo}+p${bar}", PARSE_LIMITS, LIMITS).unwrap();
    let markers = parse(&output, SNIPPET_LIMITS, |_, _| false).unwrap();
    let names = markers
        .iter()
        .filter_map(|marker| match marker {
            Marker::Variable { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["foo", "bar"]);
    assert_eq!(
        expand_stylesheet("m1000000000000000000000000000000", PARSE_LIMITS, LIMITS),
        Ok("margin: 1e+30px;".into())
    );
}

#[test]
fn css_따옴표안의_field와_escaped_field는_실제_snippet에서_구분된다() {
    let output = expand_stylesheet("f:'${1:FontName}'", PARSE_LIMITS, LIMITS).unwrap();
    let markers = parse(&output, SNIPPET_LIMITS, |_, _| false).unwrap();
    let result = expand(
        &markers,
        SNIPPET_LIMITS,
        |_| Ok(None),
        |_, _| Err(EditorError::Refused),
    )
    .unwrap();
    assert_eq!(result.text, "font: 'FontName';");
    assert_eq!(result.placeholders.len(), 1);
    let output = expand_stylesheet("f:'\\${1}'", PARSE_LIMITS, LIMITS).unwrap();
    let markers = parse(&output, SNIPPET_LIMITS, |_, _| false).unwrap();
    let result = expand(
        &markers,
        SNIPPET_LIMITS,
        |_| Ok(None),
        |_, _| Err(EditorError::Refused),
    )
    .unwrap();
    assert_eq!(result.text, "font: '${1}';");
    assert!(result.placeholders.is_empty());
}
