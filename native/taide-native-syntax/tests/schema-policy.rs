use taide_native_editor::diagnostics::{Marker, Severity};
use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_schema_evaluation::{self, Draft};
use taide_native_editor::json_schema_policy::{self, Limits, Policy};
use taide_native_editor::json_schema_refs::{self, Resolved};
use taide_native_editor::json_schema_scalar;
use taide_native_editor::json_validation::{self, Level, Settings};
use taide_native_editor::json_value::{self, Arena, Value};
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
    syntax: json_validation::Limits {
        parser: VALUES.parser,
        max_diagnostics: 32 * 1024,
        max_message_bytes: 2 * 1024 * 1024,
    },
    evaluation: json_schema_evaluation::Limits {
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
    },
    max_schema_work: 1024 * 1024,
};
const REFS: json_schema_refs::Limits = json_schema_refs::Limits {
    values: VALUES,
    max_work: 32 * 1024 * 1024,
    max_merges: 32 * 1024,
    max_text_units: 4 * 1024 * 1024,
    max_messages: 8192,
    max_message_units: 4 * 1024 * 1024,
};
const SCHEMA_URI: &str = "file:///schema.json";
const LANGUAGE: &str = "json";
const SAMPLE_COUNT: usize = 5;
const DEEP_ALL_OF: usize = 4096;

include!("../../taide-native-editor/tests/fixtures/json-schema-policy-oracle.rs");

fn hex(value: &str) -> String {
    value
        .encode_utf16()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe(input: &str, markers: &[Marker]) -> String {
    markers
        .iter()
        .map(|marker| {
            let message = &marker.message;
            let severity = match message.severity {
                Severity::Error => 1,
                Severity::Warning => 2,
                _ => panic!("Unexpected policy severity"),
            };
            let start = input[..marker.bytes.start].encode_utf16().count();
            let end = input[..marker.bytes.end].encode_utf16().count();
            format!(
                "{}:{severity}:{start}..{end}:{}:{};",
                message.code.as_deref().unwrap_or("-"),
                hex(&message.text),
                message.source.as_deref().map_or("-".to_owned(), hex)
            )
        })
        .collect()
}

fn text(arena: &Arena, name: &str) -> Option<String> {
    let key = name.encode_utf16().collect::<Vec<_>>();
    match arena.get(arena.member(arena.root()?, &key)?)? {
        Value::String(value) => Some(String::from_utf16(value).unwrap()),
        _ => None,
    }
}

fn flag(arena: &Arena, name: &str) -> Option<bool> {
    let key = name.encode_utf16().collect::<Vec<_>>();
    match arena.get(arena.member(arena.root()?, &key)?)? {
        Value::Boolean(value) => Some(*value),
        _ => None,
    }
}

fn level(value: Option<String>) -> Option<Level> {
    value.map(|value| match value.as_str() {
        "error" => Level::Error,
        "warning" => Level::Warning,
        "ignore" => Level::Ignore,
        other => panic!("Unexpected level {other}"),
    })
}

fn settings(configuration: &str, document: &str) -> (Settings, Policy) {
    let configuration = json_value::parse(configuration, VALUES).unwrap();
    let document = json_value::parse(document, VALUES).unwrap();
    let draft_key = "schemaDraft".encode_utf16().collect::<Vec<_>>();
    let schema_draft = document
        .member(document.root().unwrap(), &draft_key)
        .and_then(|index| match document.get(index) {
            Some(Value::Number(value)) => Some(match *value as u32 {
                3 => Draft::Draft3,
                4 => Draft::Draft4,
                6 => Draft::Draft6,
                7 => Draft::Draft7,
                19 => Draft::Draft2019,
                _ => Draft::Draft2020,
            }),
            _ => None,
        });
    (
        Settings {
            validate: flag(&configuration, "validate") != Some(false),
            allow_comments: flag(&configuration, "allowComments") == Some(true),
            comments: level(text(&document, "comments")),
            trailing_commas: level(text(&document, "trailingCommas")),
            schema_allows_comments: false,
            schema_allows_trailing_commas: false,
        },
        Policy {
            schema_validation: level(text(&document, "schemaValidation")),
            schema_request: level(text(&document, "schemaRequest")),
            schema_draft,
        },
    )
}

fn resolve(schema: &str) -> Option<Resolved> {
    (!schema.is_empty()).then(|| json_schema_refs::resolve_local(schema, SCHEMA_URI, REFS).unwrap())
}

#[test]
fn schema_policy는_실제원본의_스키마오류_경고_의미진단_주석쉼표정책과_중복제거를_보존한다() {
    let mut failures = Vec::new();
    for &(schema, input, configuration, document, expected) in CASES {
        let (settings, policy) = settings(configuration, document);
        let resolved = resolve(schema);
        let markers = json_schema_policy::diagnostics(
            input,
            LANGUAGE,
            settings,
            policy,
            resolved.as_ref(),
            &JsonSchemaValidationPatternCompiler,
            LIMITS,
        )
        .unwrap();
        let actual = describe(input, &markers);
        if actual != expected {
            failures.push(format!(
                "{schema} | {input:?} | {configuration} | {document}\nexpected {expected}\nactual   {actual}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} policy cases differ:\n{}",
        failures.len(),
        CASES.len(),
        failures
            .iter()
            .take(SAMPLE_COUNT)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn schema_policy는_허용탐색_예산과_순환_all_of와_진단예산을_검증한다() {
    let nested = format!(
        "{}{{\"allowComments\":true}}{}",
        "{\"allOf\":[".repeat(DEEP_ALL_OF),
        "]}".repeat(DEEP_ALL_OF)
    );
    let resolved = resolve(&nested).unwrap();
    assert_eq!(
        json_schema_policy::allows_comments(&resolved.schema, LIMITS.max_schema_work),
        Ok(Some(true))
    );
    assert_eq!(
        json_schema_policy::allows_comments(&resolved.schema, DEEP_ALL_OF),
        Err(EditorError::Capacity)
    );
    let cyclic = resolve(r##"{"allOf":[{"$ref":"#"}]}"##).unwrap();
    assert_eq!(
        json_schema_policy::allows_trailing_commas(&cyclic.schema, LIMITS.max_schema_work),
        Ok(None)
    );
    let (settings, policy) = settings(r#"{"validate":true}"#, "{}");
    let schema = resolve(r#"{"type":"string","minLength":2}"#).unwrap();
    let tight = Limits {
        syntax: json_validation::Limits {
            max_diagnostics: 1,
            ..LIMITS.syntax
        },
        ..LIMITS
    };
    assert_eq!(
        json_schema_policy::diagnostics(
            "[1,]",
            LANGUAGE,
            settings,
            policy,
            Some(&schema),
            &JsonSchemaValidationPatternCompiler,
            tight,
        ),
        Err(EditorError::Capacity)
    );
}
