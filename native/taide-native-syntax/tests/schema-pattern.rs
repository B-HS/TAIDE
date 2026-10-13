use taide_native_editor::json_schema_associations::{
    Association, Limits, PatternCompiler, glob_source,
};
use taide_native_editor::json_schema_uri::Limits as UriLimits;
use taide_native_syntax::JsonSchemaPatternCompiler;

const LIMITS: Limits = Limits {
    uri: UriLimits {
        max_input_units: 64 * 1024,
        max_output_units: 256 * 1024,
        max_work: 1024 * 1024,
    },
    max_patterns: 1024,
    max_pattern_units: 64 * 1024,
    max_regex_units: 256 * 1024,
    max_uris: 1024,
    max_uri_units: 256 * 1024,
    max_work: 1024 * 1024,
};
const RADIX: u32 = 16;

include!("../../taide-native-editor/tests/fixtures/json-schema-associations-oracle.rs");

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
fn schema_pattern은_실제원본의_folder_include_exclude와_invalid해제에_일치한다() {
    let uris = vec![
        "urn:first#".encode_utf16().collect::<Vec<_>>(),
        "HTTP://HOST/schema#".encode_utf16().collect::<Vec<_>>(),
    ];
    for &(patterns, folder, resource, expected, expected_uris) in ASSOCIATION_CASES {
        let patterns = patterns
            .iter()
            .map(|pattern| units(pattern))
            .collect::<Vec<_>>();
        let folder = folder.map(units);
        let resource = units(resource);
        let association = Association::new(
            &patterns,
            folder.as_deref(),
            &uris,
            &JsonSchemaPatternCompiler,
            LIMITS,
        )
        .unwrap();
        assert_eq!(
            association.matches_resource(&resource).unwrap(),
            expected,
            "patterns={patterns:?},folder={folder:?},resource={resource:?}"
        );
        assert_eq!(
            association.uris(),
            &expected_uris
                .iter()
                .map(|uri| units(uri))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn schema_pattern은_실제legacy정규식의_문자단위와_컴파일유효성에_일치한다() {
    for &(input, _, is_valid) in GLOB_CASES {
        let input = units(input);
        let source = glob_source(&input, LIMITS.max_regex_units).unwrap();
        assert_eq!(
            JsonSchemaPatternCompiler.compile(&source).is_ok(),
            is_valid,
            "{input:?}/{source:?}"
        );
    }
    let source = "^.$".encode_utf16().collect::<Vec<_>>();
    let pattern = JsonSchemaPatternCompiler.compile(&source).unwrap();
    assert!(pattern.is_match(&[0xd800]));
    assert!(!pattern.is_match(&[0xd800, 0xdc00]));
    let source = "^..$".encode_utf16().collect::<Vec<_>>();
    assert!(
        JsonSchemaPatternCompiler
            .compile(&source)
            .unwrap()
            .is_match(&[0xd800, 0xdc00])
    );
}
