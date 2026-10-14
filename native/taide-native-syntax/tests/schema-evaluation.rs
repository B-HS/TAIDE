use taide_native_editor::diagnostics::Severity;
use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_evaluation::{self, Draft, Limits, Validation};
use taide_native_editor::json_schema_refs;
use taide_native_editor::json_schema_scalar;
use taide_native_editor::json_value::{self, Arena, Value};
use taide_native_editor::json_value_text;
use taide_native_syntax::JsonSchemaValidationPatternCompiler;

const VALUES: json_value::Limits = json_value::Limits {
    parser: json_parser::Limits {
        scanner: json_scanner::Limits {
            max_bytes: 2 * 1024 * 1024,
            max_tokens: 4 * 1024 * 1024,
            max_value_units: 2 * 1024 * 1024,
        },
        max_nodes: 128 * 1024,
        max_depth: 64 * 1024,
        max_diagnostics: 32 * 1024,
        max_comments: 32 * 1024,
    },
    max_values: 128 * 1024,
    max_text_units: 4 * 1024 * 1024,
};
const LIMITS: Limits = Limits {
    values: VALUES,
    scalar: json_schema_scalar::Limits {
        max_work: 32 * 1024 * 1024,
        max_diagnostics: 8192,
        max_message_units: 4 * 1024 * 1024,
        max_enum_values: 32 * 1024,
        max_depth: 32 * 1024,
    },
    max_work: 32 * 1024 * 1024,
    max_frames: 8192,
    max_diagnostics: 8192,
    max_message_units: 4 * 1024 * 1024,
    max_matches: 128 * 1024,
    max_processed: 128 * 1024,
};
const TEXT: json_value_text::Limits = json_value_text::Limits {
    max_units: 4 * 1024 * 1024,
    max_work: 32 * 1024 * 1024,
    max_depth: 32 * 1024,
};
const SAMPLE_COUNT: usize = 5;
const DEEP_NESTING: usize = 2048;
const ERROR_SEVERITY: u8 = 1;
const WARNING_SEVERITY: u8 = 2;
const DIFFERENCE_CONTEXT: usize = 120;

include!("../../taide-native-editor/tests/fixtures/json-schema-evaluation-oracle.rs");

fn hex(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn evaluate(
    input: &str,
    schema: &str,
    limits: Limits,
    draft: Option<Draft>,
) -> Result<Validation, EditorError> {
    let document = json_parser::parse(input, false, VALUES.parser)?;
    let schema = json_value::parse(schema, VALUES)?;
    json_schema_evaluation::validate(
        &document,
        &schema,
        schema.root().ok_or(EditorError::InvalidIdentity)?,
        &JsonSchemaValidationPatternCompiler,
        Severity::Warning,
        draft,
        limits,
    )
}

fn describe(document: &json_parser::Document, schema: &Arena, result: &Validation) -> String {
    let diagnostics = result
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                Severity::Error => ERROR_SEVERITY,
                Severity::Warning => WARNING_SEVERITY,
                _ => panic!("Unexpected evaluation severity"),
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
        .collect::<String>();
    let processed = result
        .processed_properties
        .iter()
        .map(|name| hex(name))
        .collect::<Vec<_>>()
        .join("/");
    let enums = result.enum_values.as_ref().map_or("-".into(), |values| {
        values
            .iter()
            .map(|&value| hex(&json_value_text::stringify(schema, value, TEXT).unwrap()))
            .collect::<Vec<_>>()
            .join("/")
    });
    let metadata = format!(
        "{}:{}:{}:{}:{enums}:{processed}",
        result.properties_matches,
        result.properties_value_matches,
        result.primary_value_matches,
        result.enum_value_match
    );
    let schemas = result
        .schemas
        .iter()
        .filter(|matching| matches!(schema.get(matching.schema), Some(Value::Object(_))))
        .map(|matching| {
            let node = &document.nodes[matching.node];
            format!(
                "{}..{}:{}:{};",
                node.utf16.start,
                node.utf16.end,
                matching.inverted,
                hex(&json_value_text::stringify(schema, matching.schema, TEXT).unwrap())
            )
        })
        .collect::<String>();
    format!("{diagnostics}|{metadata}|{schemas}")
}

#[test]
fn schema_evaluation은_실제원본의_구조_조합_draft_진단과_매칭점수를_보존한다() {
    let mut mismatches = Vec::new();
    let mut corrections = 0;
    for (case_index, &(input, schema_input, raw, expected)) in CASES.iter().enumerate() {
        let document = json_parser::parse(input, false, VALUES.parser).unwrap();
        let schema = json_value::parse(schema_input, VALUES).unwrap();
        let actual = json_schema_evaluation::validate(
            &document,
            &schema,
            schema.root().unwrap(),
            &JsonSchemaValidationPatternCompiler,
            Severity::Warning,
            None,
            LIMITS,
        )
        .map(|result| describe(&document, &schema, &result));
        if raw != expected {
            assert!(input.contains("\\uD800"));
            corrections += 1;
        }
        if actual.as_deref() != Ok(expected) {
            let difference = match actual {
                Ok(actual) => {
                    let index = actual
                        .bytes()
                        .zip(expected.bytes())
                        .position(|(one, two)| one != two)
                        .unwrap_or(actual.len().min(expected.len()));
                    let start = index.saturating_sub(DIFFERENCE_CONTEXT);
                    format!(
                        "at {index}: actual={:?}, expected={:?}",
                        &actual[start.min(actual.len())
                            ..(index + DIFFERENCE_CONTEXT).min(actual.len())],
                        &expected[start.min(expected.len())
                            ..(index + DIFFERENCE_CONTEXT).min(expected.len())]
                    )
                }
                Err(error) => format!("error={error:?}"),
            };
            mismatches.push(format!(
                "case{case_index} {input:?}/{schema_input}: {difference}"
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
fn schema_evaluation은_깊은문서와_schema를_명시스택으로_평가한다() {
    let input = format!(
        "{}0{}",
        "{\"a\":".repeat(DEEP_NESTING),
        "}".repeat(DEEP_NESTING)
    );
    let schema = format!(
        "{}{{\"type\":\"string\"}}{}",
        "{\"properties\":{\"a\":".repeat(DEEP_NESTING),
        "}}".repeat(DEEP_NESTING)
    );
    let result = evaluate(&input, &schema, LIMITS, None).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.schemas.len(), DEEP_NESTING + 1);
    assert_eq!(
        result.diagnostics[0].bytes,
        DEEP_NESTING * "{\"a\":".len()..DEEP_NESTING * "{\"a\":".len() + 1
    );
    assert!(matches!(
        evaluate(
            &input,
            &schema,
            Limits {
                max_frames: DEEP_NESTING,
                ..LIMITS
            },
            None
        ),
        Err(EditorError::Capacity)
    ));
}

#[test]
fn schema_evaluation은_분기에서_버린오류도_전체작업_자료_메시지_진단_예산에_포함한다() {
    for limits in [
        Limits {
            max_work: 1,
            ..LIMITS
        },
        Limits {
            max_frames: 1,
            ..LIMITS
        },
        Limits {
            max_diagnostics: 0,
            ..LIMITS
        },
        Limits {
            max_message_units: 1,
            ..LIMITS
        },
        Limits {
            max_matches: 0,
            ..LIMITS
        },
    ] {
        assert!(
            matches!(
                evaluate("0", "{\"allOf\":[{\"type\":\"string\"}]}", limits, None),
                Err(EditorError::Capacity)
            ),
            "{limits:?}"
        );
    }
    assert!(matches!(
        evaluate(
            "{\"a\":1}",
            "{\"properties\":{\"a\":true}}",
            Limits {
                max_processed: 0,
                ..LIMITS
            },
            None
        ),
        Err(EditorError::Capacity)
    ));
    let alternatives = format!(
        "{{\"anyOf\":[{}]}}",
        vec!["{\"const\":0}"; SAMPLE_COUNT].join(",")
    );
    assert!(matches!(
        evaluate(
            "1",
            &alternatives,
            Limits {
                max_diagnostics: 1,
                ..LIMITS
            },
            None
        ),
        Err(EditorError::Capacity)
    ));
}

#[test]
fn schema_evaluation은_명시draft_우선순위와_불완전입력_원문을_보존한다() {
    let schema = "{\"$schema\":\"http://json-schema.org/draft-07/schema#\",\"items\":[false]}";
    assert_eq!(
        evaluate("[0]", schema, LIMITS, None)
            .unwrap()
            .diagnostics
            .len(),
        1
    );
    assert!(
        evaluate("[0]", schema, LIMITS, Some(Draft::Draft2020))
            .unwrap()
            .diagnostics
            .is_empty()
    );
    assert!(
        evaluate("", "{}", LIMITS, None)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    for input in ["{\"a\":}", "{\"a\":0,\"a\":}"] {
        let result = evaluate(input, "{\"required\":[\"a\"]}", LIMITS, None).unwrap();
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].bytes, 0..1);
    }
}

#[test]
fn schema_evaluation은_실제해결graph의_진행하는재귀를_허용하고_진행없는순환을_거절한다() {
    let refs = json_schema_refs::Limits {
        values: VALUES,
        max_work: LIMITS.max_work,
        max_merges: 32 * 1024,
        max_text_units: LIMITS.max_message_units,
        max_messages: LIMITS.max_diagnostics,
        max_message_units: LIMITS.max_message_units,
    };
    for (input, schema_input, cyclic, errors) in [
        (
            "{\"value\":1,\"child\":{\"value\":\"bad\"}}",
            "{\"properties\":{\"value\":{\"type\":\"number\"},\"child\":{\"$ref\":\"#\"}},\"required\":[\"value\"]}",
            false,
            1,
        ),
        (
            "{\"a\":1,\"b\":\"bad\"}",
            "{\"$defs\":{\"number\":{\"type\":\"number\"}},\"properties\":{\"a\":{\"$ref\":\"#/$defs/number\"},\"b\":{\"$ref\":\"#/$defs/number\"}}}",
            false,
            1,
        ),
        ("0", "{\"allOf\":[{\"$ref\":\"#\"}]}", true, 0),
    ] {
        let resolved =
            json_schema_refs::resolve_local(schema_input, "file:///evaluation-schema.json", refs)
                .unwrap();
        assert!(resolved.errors.is_empty());
        let document = json_parser::parse(input, false, VALUES.parser).unwrap();
        let result = json_schema_evaluation::validate(
            &document,
            &resolved.schema,
            resolved.schema.root().unwrap(),
            &JsonSchemaValidationPatternCompiler,
            Severity::Warning,
            None,
            LIMITS,
        );
        if cyclic {
            assert!(matches!(result, Err(EditorError::Capacity)));
        } else {
            assert_eq!(result.unwrap().diagnostics.len(), errors);
        }
    }
}
