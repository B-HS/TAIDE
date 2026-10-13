use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{
    MarkupLimits, StylesheetProperty, StylesheetValue, parse_stylesheet,
};

const LIMITS: MarkupLimits = MarkupLimits {
    max_bytes: 16 * 1024,
    max_nodes: 2048,
    max_nesting: 32,
};
const MISMATCH_SAMPLE_COUNT: usize = 8;

include!("fixtures/emmet-stylesheet-parser-oracle.rs");

fn part(value: Option<&str>) -> String {
    value.map_or_else(|| "-".into(), |value| format!("{}:{value}", value.len()))
}

fn number(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    value.to_string()
}

fn values(fragments: &[Vec<StylesheetValue>]) -> String {
    fragments
        .iter()
        .map(|fragment| {
            let text = fragment
                .iter()
                .map(|value| match value {
                    StylesheetValue::Literal(value) => format!("l{}", part(Some(value))),
                    StylesheetValue::CustomProperty(value) => format!("c{}", part(Some(value))),
                    StylesheetValue::Number { value, raw, unit } => format!(
                        "n{}:{}{}",
                        number(*value),
                        part(Some(raw)),
                        part(Some(unit))
                    ),
                    StylesheetValue::Color {
                        red,
                        green,
                        blue,
                        alpha,
                        raw,
                    } => format!(
                        "r{red}:{green}:{blue}:{}:{}",
                        number(*alpha),
                        part(Some(raw))
                    ),
                    StylesheetValue::String { value, quote } => {
                        let quote = if *quote == '\'' { "single" } else { "double" };
                        format!("s{quote}:{}", part(Some(value)))
                    }
                    StylesheetValue::Field { index, name } => format!(
                        "f{}:{}",
                        index.map_or_else(|| "-".into(), |index| index.to_string()),
                        part(Some(name))
                    ),
                    StylesheetValue::Function { name, arguments } => {
                        format!("x{}[{}]", part(Some(name)), values(arguments))
                    }
                })
                .collect::<String>();
            format!("[{text}]")
        })
        .collect()
}

fn canonical(properties: &[StylesheetProperty]) -> String {
    properties
        .iter()
        .map(|property| {
            format!(
                "{}i{}[{}]",
                part(property.name.as_deref()),
                usize::from(property.is_important),
                values(&property.values)
            )
        })
        .collect()
}

#[test]
fn stylesheet구조는_원본기본snippet과_value모드_ast에_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(is_value, input, expected)| {
            let actual = parse_stylesheet(input, is_value, LIMITS)
                .ok()
                .map(|properties| canonical(&properties));
            (actual.as_deref() != expected)
                .then(|| format!("{is_value}/{input:?}: actual={actual:?}, expected={expected:?}"))
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
