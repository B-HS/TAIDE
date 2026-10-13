use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser::{self, Limits as ParserLimits};
use taide_native_editor::json_scanner::Limits as ScannerLimits;
use taide_native_editor::json_schema_uri::{Limits, resolve_relative_path, schema_property};

const LIMITS: Limits = Limits {
    max_input_units: 128 * 1024,
    max_output_units: 512 * 1024,
    max_work: 2 * 1024 * 1024,
};
const PARSER_LIMITS: ParserLimits = ParserLimits {
    scanner: ScannerLimits {
        max_bytes: 128 * 1024,
        max_tokens: 128 * 1024,
        max_value_units: 128 * 1024,
    },
    max_nodes: 64 * 1024,
    max_depth: 16 * 1024,
    max_diagnostics: 64 * 1024,
    max_comments: 64 * 1024,
};
const SAMPLE_COUNT: usize = 8;
const RADIX: u32 = 16;
const DEEP_PARENT_COUNT: usize = 8192;

include!("fixtures/json-schema-context-oracle.rs");

fn units(value: &str) -> Vec<u16> {
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .map(|unit| u16::from_str_radix(unit, RADIX).unwrap())
        .collect()
}

#[test]
fn json_schema_context는_실제worker의_상대경로와_uri_encoding에_일치한다() {
    let mut mismatches = Vec::new();
    for &(resource, relative, expected) in CONTEXT_CASES {
        let resource = units(resource);
        let relative = units(relative);
        let expected = expected.map(units).ok_or(EditorError::InvalidBoundary);
        let actual = resolve_relative_path(&relative, &resource, LIMITS);
        if actual != expected {
            mismatches.push(format!(
                "{relative:?}/{resource:?}: {actual:?}/{expected:?}"
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
fn json_schema_context는_실제worker의_첫_schema속성_선택에_일치한다() {
    let mut mismatches = Vec::new();
    for &(resource, input, expected) in PROPERTY_CASES {
        let resource = units(resource);
        let document = json_parser::parse(input, true, PARSER_LIMITS).unwrap();
        let expected = expected
            .map(|result| result.map(units))
            .ok_or(EditorError::InvalidBoundary);
        let actual = schema_property(&document, &resource, LIMITS);
        if actual != expected {
            mismatches.push(format!("{input}/{resource:?}: {actual:?}/{expected:?}"));
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
fn json_schema_context는_입력_출력_작업예산과_깊은부모경로를_지킨다() {
    let resource = "file:///root/file.json".encode_utf16().collect::<Vec<_>>();
    let relative = format!("{}schema.json", "../".repeat(DEEP_PARENT_COUNT));
    let relative = relative.encode_utf16().collect::<Vec<_>>();
    assert_eq!(
        resolve_relative_path(&relative, &resource, LIMITS).unwrap(),
        "file:///schema.json".encode_utf16().collect::<Vec<_>>()
    );
    let relative = "漢字/schema.json".encode_utf16().collect::<Vec<_>>();
    let result = resolve_relative_path(&relative, &resource, LIMITS).unwrap();
    assert_eq!(
        resolve_relative_path(
            &relative,
            &resource,
            Limits {
                max_input_units: relative.len() + resource.len() - 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        resolve_relative_path(
            &relative,
            &resource,
            Limits {
                max_output_units: result.len() - 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        resolve_relative_path(
            &relative,
            &resource,
            Limits {
                max_work: relative.len() + resource.len(),
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    let document = json_parser::parse("{\"$schema\":\"a.json\"}", true, PARSER_LIMITS).unwrap();
    assert_eq!(
        schema_property(
            &document,
            &resource,
            Limits {
                max_work: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    let document = json_parser::parse(
        "{\"$schema\":\"https://host/schema.json\"}",
        true,
        PARSER_LIMITS,
    )
    .unwrap();
    assert_eq!(
        schema_property(
            &document,
            &resource,
            Limits {
                max_output_units: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    let document = json_parser::parse("{}", true, PARSER_LIMITS).unwrap();
    assert_eq!(
        schema_property(
            &document,
            &resource,
            Limits {
                max_work: 0,
                max_output_units: 0,
                ..LIMITS
            }
        )
        .unwrap(),
        None
    );
    let mut malformed = document;
    malformed.root = Some(malformed.nodes.len());
    assert_eq!(
        schema_property(&malformed, &resource, LIMITS),
        Err(EditorError::InvalidBoundary)
    );
}
