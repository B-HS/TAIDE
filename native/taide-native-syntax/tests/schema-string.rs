use taide_native_editor::diagnostics::Severity;
use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_scalar::{Limits, Target, Validation, validate_with_patterns};
use taide_native_editor::json_schema_string::PatternCompiler;
use taide_native_editor::json_value::{self, Arena};
use taide_native_syntax::JsonSchemaValidationPatternCompiler;

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
const SAMPLE_COUNT: usize = 8;
const ERROR_SEVERITY: u8 = 1;
const WARNING_SEVERITY: u8 = 2;
const INVALID_PATTERN: &str = "[";
const LONG_PATTERN_UNITS: usize = 4096;
const PATTERN_WORK_LIMIT: usize = 2048;

include!("../../taide-native-editor/tests/fixtures/json-schema-string-oracle.rs");

fn evaluate(input: &str, schema: &str, limits: Limits) -> Result<Validation, EditorError> {
    let document = json_parser::parse(input, false, VALUES.parser)?;
    let values = Arena::from_document(&document, VALUES)?;
    let schema = json_value::parse(schema, VALUES)?;
    let node = &document.nodes[document.root.ok_or(EditorError::InvalidIdentity)?];
    validate_with_patterns(
        Target {
            node,
            parent: None,
            values: &values,
            value: values.root().ok_or(EditorError::InvalidIdentity)?,
        },
        &schema,
        schema.root().ok_or(EditorError::InvalidIdentity)?,
        Severity::Warning,
        limits,
        &JsonSchemaValidationPatternCompiler,
    )
}

fn describe(result: &Validation) -> String {
    result
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                Severity::Error => ERROR_SEVERITY,
                Severity::Warning => WARNING_SEVERITY,
                _ => panic!("Unexpected string severity"),
            };
            let message = diagnostic
                .message
                .iter()
                .map(|unit| format!("{unit:x}"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{}:{severity}:{}..{}:{}..{}:{message};",
                diagnostic.code.map_or("-".into(), |code| code.to_string()),
                diagnostic.bytes.start,
                diagnostic.bytes.end,
                diagnostic.utf16.start,
                diagnostic.utf16.end
            )
        })
        .collect()
}

#[test]
fn schema_string은_실제원본의_pattern_형식_메시지우선순위와_공통진단순서를_보존한다() {
    let mut mismatches = Vec::new();
    for &(input, schema, expected) in CASES {
        let actual = evaluate(input, schema, LIMITS).map(|result| describe(&result));
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!(
                "{input:?}/{schema}: actual={actual:?}, expected={expected}"
            ));
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
fn schema_validation_pattern은_unicode와_legacy의_서로게이트와_컴파일실패를_구분한다() {
    let compiler = JsonSchemaValidationPatternCompiler;
    let source = "^.$".encode_utf16().collect::<Vec<_>>();
    let value = "𐐀".encode_utf16().collect::<Vec<_>>();
    assert!(
        compiler
            .compile(&source, false, true)
            .unwrap()
            .unwrap()
            .is_match(&value)
    );
    assert!(
        !compiler
            .compile(&source, false, false)
            .unwrap()
            .unwrap()
            .is_match(&value)
    );
    let invalid = INVALID_PATTERN.encode_utf16().collect::<Vec<_>>();
    assert!(compiler.compile(&invalid, false, true).unwrap().is_none());
    let fallback = "\\a".encode_utf16().collect::<Vec<_>>();
    assert!(compiler.compile(&fallback, false, true).unwrap().is_none());
    assert!(
        compiler
            .compile(&fallback, false, false)
            .unwrap()
            .unwrap()
            .is_match(&[u16::from(b'a')])
    );
}

#[test]
fn schema_string은_패턴_출력_진단의_공유예산을_지킨다() {
    let schema = "{\"pattern\":\"[\",\"format\":\"uri\"}";
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
    ] {
        assert!(
            matches!(evaluate("\"\"", schema, limits), Err(EditorError::Capacity)),
            "{limits:?}"
        );
    }
    let pattern_schema = format!("{{\"pattern\":\"{}\"}}", "x".repeat(LONG_PATTERN_UNITS));
    json_value::parse(&pattern_schema, VALUES).unwrap();
    assert!(matches!(
        evaluate(
            "\"a\"",
            &pattern_schema,
            Limits {
                max_work: PATTERN_WORK_LIMIT,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
}
