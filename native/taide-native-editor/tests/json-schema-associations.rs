use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_associations::{
    Association, Limits, Pattern, PatternCompiler, glob_source,
};
use taide_native_editor::json_schema_uri::Limits as UriLimits;

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
const MATCH_WORK_LIMIT: usize = 64;

include!("fixtures/json-schema-associations-oracle.rs");

struct MatchingPattern;
struct MatchingCompiler;

impl Pattern for MatchingPattern {
    fn is_match(&self, _: &[u16]) -> bool {
        true
    }
}

impl PatternCompiler for MatchingCompiler {
    fn compile(&self, _: &[u16]) -> Result<Box<dyn Pattern>, EditorError> {
        Ok(Box::new(MatchingPattern))
    }
}

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
fn json_schema_glob은_실제원본의_extended_globstar_변환에_일치한다() {
    assert!(!ASSOCIATION_CASES.is_empty());
    for &(input, expected, _) in GLOB_CASES {
        let input = units(input);
        assert_eq!(
            glob_source(&input, LIMITS.max_regex_units).unwrap(),
            units(expected),
            "{input:?}"
        );
    }
}

#[test]
fn json_schema_associations는_전체pattern_uri_작업예산과_순서를_지킨다() {
    let patterns = vec!["*.json".encode_utf16().collect::<Vec<_>>()];
    let uris = vec!["urn:first#".encode_utf16().collect::<Vec<_>>()];
    for limits in [
        Limits {
            max_patterns: 0,
            ..LIMITS
        },
        Limits {
            max_pattern_units: 0,
            ..LIMITS
        },
        Limits {
            max_regex_units: 0,
            ..LIMITS
        },
        Limits {
            max_uris: 0,
            ..LIMITS
        },
        Limits {
            max_uri_units: 0,
            ..LIMITS
        },
        Limits {
            max_work: 0,
            ..LIMITS
        },
    ] {
        assert!(matches!(
            Association::new(&patterns, None, &uris, &MatchingCompiler, limits),
            Err(EditorError::Capacity)
        ));
    }
    let resource = "file:///a.json".encode_utf16().collect::<Vec<_>>();
    let association = Association::new(&patterns, None, &uris, &MatchingCompiler, LIMITS).unwrap();
    assert!(association.matches_resource(&resource).unwrap());
    assert_eq!(
        association.uris(),
        &["urn:first".encode_utf16().collect::<Vec<_>>()]
    );
    let excluded = vec![
        patterns[0].clone(),
        "!a.json".encode_utf16().collect::<Vec<_>>(),
    ];
    let association = Association::new(&excluded, None, &uris, &MatchingCompiler, LIMITS).unwrap();
    assert!(!association.matches_resource(&resource).unwrap());
    let association = Association::new(
        &patterns,
        None,
        &uris,
        &MatchingCompiler,
        Limits {
            max_work: MATCH_WORK_LIMIT,
            ..LIMITS
        },
    )
    .unwrap();
    let resource = "file:///"
        .repeat(MATCH_WORK_LIMIT)
        .encode_utf16()
        .collect::<Vec<_>>();
    assert_eq!(
        association.matches_resource(&resource),
        Err(EditorError::Capacity)
    );
    assert!(matches!(
        Association::new(
            &patterns,
            None,
            &uris,
            &MatchingCompiler,
            Limits {
                uri: UriLimits {
                    max_input_units: 0,
                    ..LIMITS.uri
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert_eq!(glob_source(&[], 0), Err(EditorError::Capacity));
}
