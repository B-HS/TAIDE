use taide_native_editor::diagnostics::Severity;
use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser::{self, Document, Kind};
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_scalar::{self, Limits, Target, Validation};
use taide_native_editor::json_value::{self, Arena};
use taide_native_editor::json_value_text;

const VALUES: json_value::Limits = json_value::Limits {
    parser: json_parser::Limits {
        scanner: json_scanner::Limits {
            max_bytes: 64 * 1024,
            max_tokens: 128 * 1024,
            max_value_units: 64 * 1024,
        },
        max_nodes: 32 * 1024,
        max_depth: 16 * 1024,
        max_diagnostics: 32 * 1024,
        max_comments: 32 * 1024,
    },
    max_values: 16 * 1024,
    max_text_units: 64 * 1024,
};
const LIMITS: Limits = Limits {
    max_work: 2 * 1024 * 1024,
    max_diagnostics: 1024,
    max_message_units: 128 * 1024,
    max_enum_values: 16 * 1024,
    max_depth: 16 * 1024,
};
const TEXT: json_value_text::Limits = json_value_text::Limits {
    max_units: 128 * 1024,
    max_work: 2 * 1024 * 1024,
    max_depth: 16 * 1024,
};
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 8192;
const ERROR_SEVERITY: u8 = 1;
const WARNING_SEVERITY: u8 = 2;
const ENUM_CODE: u32 = 1;

include!("fixtures/json-schema-scalar-oracle.rs");

fn hex(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn target<'a>(document: &'a Document, values: &'a Arena, offset: usize) -> Target<'a> {
    let node = document
        .nodes
        .iter()
        .find(|node| node.bytes.start == offset && node.kind != Kind::Property)
        .unwrap();
    let parent = node.parent.map(|index| &document.nodes[index]);
    let root = values.root().unwrap();
    let value = if document
        .root
        .is_some_and(|root| std::ptr::eq(node, &document.nodes[root]))
    {
        root
    } else {
        values
            .member(root, &"target".encode_utf16().collect::<Vec<_>>())
            .unwrap()
    };
    Target {
        node,
        parent,
        values,
        value,
    }
}

fn describe(result: &Validation) -> String {
    result
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                Severity::Error => ERROR_SEVERITY,
                Severity::Warning => WARNING_SEVERITY,
                _ => panic!("Unexpected scalar severity"),
            };
            format!(
                "{}:{severity}:{}..{}:{}..{}:{};",
                diagnostic.code.map_or("-".into(), |code| code.to_string()),
                diagnostic.bytes.start,
                diagnostic.bytes.end,
                diagnostic.utf16.start,
                diagnostic.utf16.end,
                hex(&diagnostic.message)
            )
        })
        .collect()
}

#[test]
fn json_schema_scalar는_실제원본의_공통_숫자_문자열_진단범위_순서와_메시지를_보존한다() {
    let mut mismatches = Vec::new();
    let mut corrections = 0;
    for &(input, schema_input, offset, severity, raw, expected) in CASES {
        let document = json_parser::parse(input, false, VALUES.parser).unwrap();
        let values = Arena::from_document(&document, VALUES).unwrap();
        let schema = json_value::parse(schema_input, VALUES).unwrap();
        let severity = match severity {
            ERROR_SEVERITY => Severity::Error,
            WARNING_SEVERITY => Severity::Warning,
            _ => panic!("Unexpected fixture severity"),
        };
        let actual = json_schema_scalar::validate(
            target(&document, &values, offset),
            &schema,
            schema.root().unwrap(),
            severity,
            LIMITS,
        )
        .map(|result| describe(&result));
        if raw != expected {
            assert!(input.contains("\\uD800"));
            corrections += 1;
        }
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!(
                "{input:?}/{schema_input}: actual={actual:?}, expected={expected:?}, source={raw:?}"
            ));
        }
    }
    assert!(corrections > 0);
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(SAMPLE_COUNT)].join("\n")
    );
}

#[test]
fn json_value_text는_실제json_stringify의_숫자_객체키_제어문자와_논리utf16을_보존한다() {
    let mut mismatches = Vec::new();
    for &(input, expected) in STRINGIFY_CASES {
        let values = json_value::parse(input, VALUES).unwrap();
        let actual = json_value_text::stringify(&values, values.root().unwrap(), TEXT)
            .map(|text| hex(&text));
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!("{input:?}: actual={actual:?}, expected={expected}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(SAMPLE_COUNT)].join("\n")
    );
}

#[test]
fn json_schema_scalar는_enum_const의_일치메타데이터와_논리메시지를_보존한다() {
    let document = json_parser::parse("0", false, VALUES.parser).unwrap();
    let values = Arena::from_document(&document, VALUES).unwrap();
    for (input, expected_match, expected_diagnostics) in [
        ("{\"enum\":[0],\"const\":1}", false, 1),
        ("{\"enum\":[1],\"const\":0}", true, 1),
        ("{\"enum\":[0]}", true, 0),
        ("{\"enum\":[]}", false, 1),
    ] {
        let schema = json_value::parse(input, VALUES).unwrap();
        let result = json_schema_scalar::validate(
            target(&document, &values, 0),
            &schema,
            schema.root().unwrap(),
            Severity::Error,
            LIMITS,
        )
        .unwrap();
        assert_eq!(result.enum_value_match, expected_match, "{input}");
        assert_eq!(result.diagnostics.len(), expected_diagnostics, "{input}");
        let entries = result.enum_values.unwrap();
        if let Some(value) = schema.member(
            schema.root().unwrap(),
            &"const".encode_utf16().collect::<Vec<_>>(),
        ) {
            assert_eq!(entries, vec![value]);
        }
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code == Some(ENUM_CODE))
        );
    }
}

#[test]
fn json_schema_scalar는_전체비교_메시지_자료_진단_예산과_잘못된형식을_거절한다() {
    let document = json_parser::parse("\"aaaaaaaa\"", false, VALUES.parser).unwrap();
    let values = Arena::from_document(&document, VALUES).unwrap();
    let schema = json_value::parse(
        "{\"type\":\"number\",\"enum\":[\"bbbbbbbb\",\"cccccccc\"],\"deprecated\":true}",
        VALUES,
    )
    .unwrap();
    for limits in [
        Limits {
            max_work: 1,
            ..LIMITS
        },
        Limits {
            max_message_units: 1,
            ..LIMITS
        },
        Limits {
            max_diagnostics: 1,
            ..LIMITS
        },
        Limits {
            max_enum_values: 1,
            ..LIMITS
        },
    ] {
        assert!(
            matches!(
                json_schema_scalar::validate(
                    target(&document, &values, 0),
                    &schema,
                    schema.root().unwrap(),
                    Severity::Error,
                    limits
                ),
                Err(EditorError::Capacity)
            ),
            "{limits:?}"
        );
    }
    for input in [
        "{\"type\":[1]}",
        "{\"type\":true}",
        "{\"type\":\"number\",\"errorMessage\":{}}",
        "{\"deprecationMessage\":true}",
    ] {
        let malformed = json_value::parse(input, VALUES).unwrap();
        assert!(
            matches!(
                json_schema_scalar::validate(
                    target(&document, &values, 0),
                    &malformed,
                    malformed.root().unwrap(),
                    Severity::Warning,
                    LIMITS
                ),
                Err(EditorError::InvalidBoundary)
            ),
            "{input}"
        );
    }
    let repeated = format!(
        "{{\"enum\":[{}],\"errorMessage\":\"short\"}}",
        vec!["\"aaaaaaab\""; SAMPLE_COUNT].join(",")
    );
    let repeated = json_value::parse(&repeated, VALUES).unwrap();
    assert!(matches!(
        json_schema_scalar::validate(
            target(&document, &values, 0),
            &repeated,
            repeated.root().unwrap(),
            Severity::Warning,
            Limits {
                max_work: 120,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
}

#[test]
fn json_value_text는_깊은값에서_스택을_쓰지않고_출력_깊이와_작업예산을_지킨다() {
    let input = format!("{}0{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    let arena = json_value::parse(&input, VALUES).unwrap();
    let value = arena.root().unwrap();
    assert_eq!(
        String::from_utf16(&json_value_text::stringify(&arena, value, TEXT).unwrap()).unwrap(),
        input
    );
    for limits in [
        json_value_text::Limits {
            max_depth: DEEP_NESTING,
            ..TEXT
        },
        json_value_text::Limits {
            max_work: DEEP_NESTING,
            ..TEXT
        },
        json_value_text::Limits {
            max_units: input.len() - 1,
            ..TEXT
        },
    ] {
        assert!(
            matches!(
                json_value_text::stringify(&arena, value, limits),
                Err(EditorError::Capacity)
            ),
            "{limits:?}"
        );
    }
    let quoted = json_value::parse("\"a\"", VALUES).unwrap();
    assert_eq!(
        json_value_text::stringify(
            &quoted,
            quoted.root().unwrap(),
            json_value_text::Limits {
                max_units: 3,
                ..TEXT
            }
        )
        .unwrap(),
        "\"a\"".encode_utf16().collect::<Vec<_>>()
    );
    assert!(matches!(
        json_value_text::stringify(
            &quoted,
            quoted.root().unwrap(),
            json_value_text::Limits {
                max_units: 2,
                ..TEXT
            }
        ),
        Err(EditorError::Capacity)
    ));
}
