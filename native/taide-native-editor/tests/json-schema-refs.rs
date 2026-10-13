use std::collections::{BTreeMap, VecDeque};

use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_refs::{Limits, Resolved, resolve_local};
use taide_native_editor::json_value::{self, Value};

const LIMITS: Limits = Limits {
    values: json_value::Limits {
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
    },
    max_work: 1024 * 1024,
    max_merges: 16 * 1024,
    max_text_units: 1024 * 1024,
    max_messages: 16 * 1024,
    max_message_units: 1024 * 1024,
};
const URI: &str = "file:///schema.json";
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 1024;

include!("fixtures/json-schema-refs-oracle.rs");

fn units(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe(result: &Resolved) -> String {
    let arena = &result.schema;
    let mut pending = VecDeque::from([(arena.root().unwrap(), "-1".to_owned(), "-".to_owned())]);
    let mut seen = BTreeMap::new();
    let mut parts = Vec::new();
    while let Some((index, parent, edge)) = pending.pop_front() {
        let value = arena.get(index).unwrap();
        let cursor = parts.len();
        if matches!(value, Value::Array(_) | Value::Object(_)) {
            if let Some(first) = seen.get(&index) {
                parts.push(format!("ref:{parent}:{edge}:{first};"));
                continue;
            }
            seen.insert(index, cursor);
        }
        let mut content = String::new();
        let kind = match value {
            Value::Null => "null",
            Value::Boolean(value) => {
                content = value.to_string();
                "boolean"
            }
            Value::Number(value) => {
                content = value.to_string();
                "number"
            }
            Value::String(value) => {
                content = units(value);
                "string"
            }
            Value::Array(items) => {
                for (position, &item) in items.iter().enumerate() {
                    pending.push_back((item, cursor.to_string(), format!("i{position}")))
                }
                "array"
            }
            Value::Object(members) => {
                for member in members {
                    pending.push_back((
                        member.value,
                        cursor.to_string(),
                        format!("k{}", units(&member.key)),
                    ))
                }
                "object"
            }
        };
        parts.push(format!("{kind}:{parent}:{edge}:{content};"));
    }
    let errors = result
        .errors
        .iter()
        .map(|error| units(error))
        .collect::<Vec<_>>()
        .join(";");
    let warnings = if result.unsupported.is_empty() {
        String::new()
    } else {
        units(&format!("The schema uses meta-schema features ({}) that are not yet supported by the validator.",result.unsupported.join(", ")).encode_utf16().collect::<Vec<_>>())
    };
    let draft = result.schema_draft.as_deref().map_or("-".to_owned(), units);
    format!("{}|{errors}|{warnings}|{draft}", parts.join(""))
}

#[test]
fn json_schema_refs는_실제원본의_local참조_anchor_merge_순환과_진단에_일치한다() {
    let mut mismatches = Vec::new();
    for &(input, raw, expected) in CASES {
        let actual = resolve_local(input, URI, LIMITS).map(|result| describe(&result));
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!(
                "{input}: actual={actual:?}, expected={expected:?}, source={raw:?}"
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
fn json_schema_refs는_외부참조를_실제대상과_논리fragment로_대기시킨다() {
    let input =
        "{\"properties\":{\"a\":{\"$ref\":\"other.json#/definitions/A\",\"title\":\"keep\"}}}";
    let result = resolve_local(input, URI, LIMITS).unwrap();
    assert_eq!(result.external.len(), 1);
    assert_eq!(
        result.external[0].uri,
        "other.json".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(
        result.external[0].fragment,
        Some("/definitions/A".encode_utf16().collect::<Vec<_>>())
    );
    let target = result.external[0].target;
    assert!(
        result
            .schema
            .member(target, &"$ref".encode_utf16().collect::<Vec<_>>())
            .is_none()
    );
    assert!(
        result
            .schema
            .member(target, &"title".encode_utf16().collect::<Vec<_>>())
            .is_some()
    );
    assert!(result.errors.is_empty());
}

#[test]
fn json_schema_refs는_깊은graph와_탐색_merge_문자_메시지_노드예산을_지킨다() {
    let input = format!(
        "{}{{\"type\":\"string\"}}{}",
        "{\"items\":".repeat(DEEP_NESTING),
        "}".repeat(DEEP_NESTING)
    );
    assert!(
        resolve_local(&input, URI, LIMITS)
            .unwrap()
            .errors
            .is_empty()
    );
    assert!(matches!(
        resolve_local(
            &input,
            URI,
            Limits {
                max_work: DEEP_NESTING,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    let reference = "{\"definitions\":{\"A\":{\"type\":\"string\"}},\"$ref\":\"#/definitions/A\"}";
    assert!(matches!(
        resolve_local(
            reference,
            URI,
            Limits {
                max_merges: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        resolve_local(
            reference,
            URI,
            Limits {
                max_text_units: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        resolve_local(
            "{\"$ref\":\"#missing\"}",
            URI,
            Limits {
                max_messages: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        resolve_local(
            "{\"$ref\":\"#missing\"}",
            URI,
            Limits {
                max_message_units: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        resolve_local(
            "{}",
            URI,
            Limits {
                values: json_value::Limits {
                    max_values: 0,
                    ..LIMITS.values
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        resolve_local(
            "{}",
            URI,
            Limits {
                max_merges: 0,
                max_text_units: 0,
                max_messages: 0,
                max_message_units: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .errors
        .is_empty()
    );
    assert!(matches!(
        resolve_local(
            reference,
            URI,
            Limits {
                max_work: 2,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    let false_reference = "{\"definitions\":{\"A\":false},\"$ref\":\"#/definitions/A\"}";
    let initial_values = json_value::parse(false_reference, LIMITS.values)
        .unwrap()
        .values()
        .len();
    assert!(matches!(
        resolve_local(
            false_reference,
            URI,
            Limits {
                values: json_value::Limits {
                    max_values: initial_values,
                    ..LIMITS.values
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
}
