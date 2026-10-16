use std::cell::Cell;

use taide_native_editor::diagnostics::{Marker, Severity};
use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_document::{self, Limits};
use taide_native_editor::json_schema_evaluation;
use taide_native_editor::json_schema_graph;
use taide_native_editor::json_schema_load;
use taide_native_editor::json_schema_policy::{self, Policy};
use taide_native_editor::json_schema_scalar;
use taide_native_editor::json_validation::{self, Level, Settings};
use taide_native_syntax::{JsonSchemaPatternCompiler, JsonSchemaValidationPatternCompiler};

include!("../../taide-native-editor/tests/fixtures/json-schema-service-limits.rs");
include!("../../taide-native-editor/tests/fixtures/json-schema-document-oracle.rs");

const DOCUMENT_LIMITS: Limits = Limits {
    service: LIMITS,
    graph: json_schema_graph::Limits {
        refs: REF_LIMITS,
        uri: URI_LIMITS,
        max_documents: 128,
        max_requests: 4096,
        max_uri_units: 1024 * 1024,
    },
    load: json_schema_load::Limits {
        scanner: VALUE_LIMITS.parser.scanner,
        uri: URI_LIMITS,
        max_values: VALUE_LIMITS.max_values,
        max_depth: VALUE_LIMITS.parser.max_depth,
        max_text_units: VALUE_LIMITS.max_text_units,
        max_work: 1024 * 1024,
        max_messages: 16,
        max_message_units: 1024 * 1024,
    },
    policy: json_schema_policy::Limits {
        syntax: json_validation::Limits {
            parser: VALUE_LIMITS.parser,
            max_diagnostics: 32 * 1024,
            max_message_bytes: 2 * 1024 * 1024,
        },
        evaluation: json_schema_evaluation::Limits {
            values: VALUE_LIMITS,
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
    },
    contributions: VALUE_LIMITS,
    max_steps: 64,
};
const SETTINGS: Settings = Settings {
    validate: true,
    allow_comments: true,
    comments: Some(Level::Error),
    trailing_commas: Some(Level::Error),
    schema_allows_comments: false,
    schema_allows_trailing_commas: false,
};
const POLICY: Policy = Policy {
    schema_validation: Some(Level::Warning),
    schema_request: Some(Level::Warning),
    schema_draft: None,
};
const LANGUAGE: &str = "json";
const SAMPLE_COUNT: usize = 5;

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
                _ => panic!("Unexpected document severity"),
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

fn run(
    uri: &str,
    input: &str,
    limits: Limits,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<Marker>, EditorError> {
    let contributions = json_schema_document::contributions(VALUE_LIMITS)?;
    json_schema_document::diagnostics(
        input,
        uri,
        LANGUAGE,
        SETTINGS,
        POLICY,
        &contributions,
        &JsonSchemaPatternCompiler,
        &JsonSchemaValidationPatternCompiler,
        limits,
        cancelled,
    )
}

#[test]
fn schema_document는_실제_monaco_worker의_기본설정_메타schema_요청불가_진단과_일치한다() {
    let mut failures = Vec::new();
    for &(uri, input, expected) in CASES {
        let actual = describe(input, &run(uri, input, DOCUMENT_LIMITS, &|| false).unwrap());
        if actual != expected {
            failures.push(format!(
                "{uri} | {input}\nexpected {expected}\nactual   {actual}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} document cases differ:\n{}",
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
fn schema_document는_취소_단계상한과_비활성설정을_지킨다() {
    let input = r#"{"$schema":"https://example.com/x.json"}"#;
    let calls = Cell::new(0usize);
    assert_eq!(
        run("file:///a.json", input, DOCUMENT_LIMITS, &|| {
            calls.set(calls.get() + 1);
            calls.get() > 1
        }),
        Err(EditorError::StaleRevision)
    );
    assert_eq!(
        run(
            "file:///a.json",
            input,
            Limits {
                max_steps: 1,
                ..DOCUMENT_LIMITS
            },
            &|| false
        ),
        Err(EditorError::Capacity)
    );
    let contributions = json_schema_document::contributions(VALUE_LIMITS).unwrap();
    assert_eq!(contributions.len(), 2);
    assert_eq!(
        json_schema_document::diagnostics(
            "{,",
            "file:///a.json",
            LANGUAGE,
            Settings {
                validate: false,
                ..SETTINGS
            },
            POLICY,
            &contributions,
            &JsonSchemaPatternCompiler,
            &JsonSchemaValidationPatternCompiler,
            DOCUMENT_LIMITS,
            &|| false,
        ),
        Ok(Vec::new())
    );
}
