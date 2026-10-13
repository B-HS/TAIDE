use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{ConversionLimits, MarkupLimits, Syntax, expand_markup};
use taide_native_editor::snippet_expansion::{Expansion, expand};
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
const SNIPPET_LIMITS: ParseLimits = ParseLimits {
    max_bytes: LIMITS.max_bytes,
    max_markers: LIMITS.max_nodes,
    max_nesting: LIMITS.max_nesting,
};

include!("fixtures/emmet-markup-boundaries-oracle.rs");

fn consume(output: &str) -> Expansion {
    let markers = parse(output, SNIPPET_LIMITS, |_, _| false).unwrap();
    expand(
        &markers,
        SNIPPET_LIMITS,
        |_| Ok(None),
        |_, _| Err(EditorError::Refused),
    )
    .unwrap()
}

fn corrected_text(syntax: Syntax, input: &str) -> Option<&'static str> {
    match input {
        r"p{\\\$cost}" => Some(r"<p>\$cost</p>"),
        r"p{\\\${1}}" => Some(r"<p>\${1}</p>"),
        r"p{\\\\${1}}" => Some(r"<p>\\</p>"),
        "p{${1:a{b}c}}" => Some("<p>a{b}c</p>"),
        r"p{${1:foo\}bar}}" => Some("<p>foo}bar</p>"),
        r"p{${1:foo\\$cost}}" => Some(r"<p>foo\$cost</p>"),
        r#"div[foo="${1:a{b}c}"]"# | r#"div[foo="${1:${2:a{b}c}}"]"# => {
            Some(r#"<div foo="a{b}c"></div>"#)
        }
        r#"div[foo="\${1}"]"# => Some(r#"<div foo="${1}"></div>"#),
        r#"div[foo="\\\${1}"]"# => Some(r#"<div foo="\${1}"></div>"#),
        r"div..foo\'bar" if syntax == Syntax::Jsx => {
            Some(r#"<div styleName={styles['foo\'bar']}></div>"#)
        }
        r"div..foo\\bar" if syntax == Syntax::Jsx => {
            Some(r#"<div styleName={styles['foo\\bar']}></div>"#)
        }
        r#"div[title='a"b']"# | r#"div[title="a\"b"]"# => Some(r#"<div title="a&quot;b"></div>"#),
        _ => None,
    }
}

#[test]
fn emmet출력의_문자_field_jsx경계는_원본과_소비결과를_보존한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(syntax, input, expected)| {
            let syntax = if syntax == "jsx" {
                Syntax::Jsx
            } else {
                Syntax::Html
            };
            let actual = expand_markup(input, syntax, PARSE_LIMITS, LIMITS)
                .ok()
                .map(|output| consume(&output).text);
            let corrected = corrected_text(syntax, input);
            let expected = corrected
                .map(str::to_owned)
                .or_else(|| expected.map(|output| consume(output).text));
            (actual != expected)
                .then(|| format!("{syntax:?} {input:?}: actual={actual:?}, expected={expected:?}"))
        })
        .collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn 속성이름의_field는_리터럴과_구분되며_실제_snippet인덱스로_전달된다() {
    let output = expand_markup(
        "div[${1:attr}=${2:value}]",
        Syntax::Html,
        PARSE_LIMITS,
        LIMITS,
    )
    .unwrap();
    assert_eq!(output, "<div ${1:attr}=\"${3:value}\">${0}</div>");
    let consumed = consume(&output);
    assert_eq!(consumed.text, "<div attr=\"value\"></div>");
    let indices = consumed
        .placeholders
        .iter()
        .map(|placeholder| placeholder.index.value())
        .collect::<Vec<_>>();
    let expected_indices = [1usize, 3, 0].map(|index| index as f64);
    assert_eq!(indices, expected_indices);
    let output = expand_markup("p{${1:cost$5}}", Syntax::Html, PARSE_LIMITS, LIMITS).unwrap();
    let consumed = consume(&output);
    assert_eq!(consumed.text, "<p>cost$5</p>");
    assert_eq!(consumed.placeholders.len(), 1);
}
