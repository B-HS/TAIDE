use serde::Deserialize;
use taide_native_editor::document::EditorError;
use taide_native_editor::snippet_expansion::expand;
use taide_native_editor::snippet_syntax::{
    FinalTabstopOptions, Marker, ParseLimits, Transform, parse_complete,
};
use taide_native_syntax::MonacoSnippetTransforms;

const BYTE_LIMIT: usize = 64 * 1024;
const MARKER_LIMIT: usize = 128;
const NESTING_LIMIT: usize = 16;
const REFERENCE: &str = include_str!("fixtures/snippet-transforms-reference.json");

#[derive(Deserialize)]
struct Reference {
    cases: Vec<Case>,
    metadata: Vec<MetadataCase>,
}

#[derive(Deserialize)]
struct Case {
    snippet: String,
    value: String,
    expected: String,
}

#[derive(Deserialize)]
struct MetadataCase {
    source: String,
    options: String,
    expected: Option<Metadata>,
}

#[derive(Deserialize)]
struct Metadata {
    source: String,
    ignore_case: bool,
    global: bool,
}

fn limits() -> ParseLimits {
    ParseLimits {
        max_bytes: BYTE_LIMIT,
        max_markers: MARKER_LIMIT,
        max_nesting: NESTING_LIMIT,
    }
}

#[test]
fn 스니펫_변환은_실제_monaco의_캡처_조건_대소문자_utf16과_일치한다() {
    let reference: Reference = serde_json::from_str(REFERENCE).unwrap();
    let mut compiler = MonacoSnippetTransforms::new(limits());
    for (index, case) in reference.cases.into_iter().enumerate() {
        let markers = parse_complete(
            &case.snippet,
            limits(),
            FinalTabstopOptions {
                insert: false,
                enforce: false,
            },
            |source, options| compiler.compile(source, options),
        )
        .unwrap();
        let Marker::Variable {
            transform: Some(transform),
            ..
        } = &markers[0]
        else {
            panic!("not parsed {index}: {}", case.snippet);
        };
        assert_eq!(
            compiler.evaluate(transform, &case.value).unwrap(),
            case.expected,
            "case {index}: {} input {:?}",
            case.snippet,
            case.value
        );
    }
}

#[test]
fn 스니펫_정규식은_js의_검증과_소스_메타데이터를_보존한다() {
    let reference: Reference = serde_json::from_str(REFERENCE).unwrap();
    for (index, case) in reference.metadata.into_iter().enumerate() {
        let mut compiler = MonacoSnippetTransforms::new(limits());
        let metadata = compiler.compile(&case.source, &case.options);
        assert_eq!(
            metadata.is_some(),
            case.expected.is_some(),
            "metadata {index}: {:?} {:?}",
            case.source,
            case.options
        );
        if let (Some(actual), Some(expected)) = (metadata, case.expected) {
            assert_eq!(actual.source, expected.source, "source {index}");
            assert_eq!(
                actual.ignore_case, expected.ignore_case,
                "ignore_case {index}"
            );
            assert_eq!(actual.global, expected.global, "global {index}");
        }
    }
}

#[test]
fn 복제한_변수의_멀티라인_변환도_실제_엔진으로_동일하게_확장된다() {
    let mut compiler = MonacoSnippetTransforms::new(limits());
    let markers = parse_complete(
        "${1:${name/(.*)/${1:/upcase}/s}} $1",
        limits(),
        FinalTabstopOptions {
            insert: true,
            enforce: false,
        },
        |source, flags| compiler.compile(source, flags),
    )
    .unwrap();
    let expansion = expand(
        &markers,
        limits(),
        |_| Ok(Some("first\nsecond".to_owned())),
        |transform, value| compiler.evaluate(transform, value),
    )
    .unwrap();
    assert_eq!(expansion.text, "FIRST\nSECOND FIRST\nSECOND");
}

#[test]
fn 변환은_결과와_컴파일_캐시의_한도를_넘으면_거절한다() {
    let small_limits = ParseLimits {
        max_bytes: 8,
        max_markers: 1,
        max_nesting: 1,
    };
    let mut compiler = MonacoSnippetTransforms::new(small_limits);
    assert!(compiler.compile("(.)", "").is_some());
    assert!(compiler.compile("(.)", "").is_some());
    assert!(compiler.compile("(.)", "i").is_none());
    assert!(compiler.compile("source-too-long", "").is_none());
    let markers = parse_complete(
        "${name/(.)/$1$1/g}",
        limits(),
        FinalTabstopOptions {
            insert: false,
            enforce: false,
        },
        |_, _| {
            Some(taide_native_editor::snippet_syntax::RegexMetadata {
                source: "(.)".to_owned(),
                ignore_case: false,
                global: true,
            })
        },
    )
    .unwrap();
    let Marker::Variable {
        transform: Some(transform),
        ..
    } = &markers[0]
    else {
        panic!("variable");
    };
    let mut compiler = MonacoSnippetTransforms::new(ParseLimits {
        max_markers: MARKER_LIMIT,
        ..small_limits
    });
    assert_eq!(
        compiler.evaluate(transform, "abcdef").unwrap_err(),
        EditorError::Capacity
    );
    let oversized = Transform {
        pattern: ".".to_owned(),
        options: "".to_owned(),
        format: vec![],
    };
    assert_eq!(
        compiler
            .evaluate(&oversized, "\u{1f642}\u{1f642}\u{1f642}")
            .unwrap_err(),
        EditorError::Capacity
    );
}
