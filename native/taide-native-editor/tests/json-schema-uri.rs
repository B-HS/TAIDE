use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_uri::{Limits, normalize_id, normalize_resource};

const LIMITS: Limits = Limits {
    max_input_units: 128 * 1024,
    max_output_units: 512 * 1024,
    max_work: 1024 * 1024,
};
const SAMPLE_COUNT: usize = 8;
const DEEP_PERCENT_COUNT: usize = 32 * 1024;

include!("fixtures/json-schema-uri-oracle.rs");

fn units(value: &str) -> Vec<u16> {
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .map(|unit| u16::from_str_radix(unit, 16).unwrap())
        .collect()
}

#[test]
fn json_schema_uri는_실제원본의_정규화와_resource_matching에_일치한다() {
    let mut mismatches = Vec::new();
    for &(input, id, matching) in CASES {
        let input = units(input);
        let id = units(id);
        let matching = units(matching);
        let actual_id = normalize_id(&input, LIMITS);
        let actual_resource = normalize_resource(&input, LIMITS);
        if actual_id.as_deref() != Ok(id.as_slice())
            || actual_resource.as_deref() != Ok(matching.as_slice())
        {
            mismatches.push(format!(
                "{input:?}: id={actual_id:?}/{id:?}, resource={actual_resource:?}/{matching:?}"
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
fn json_schema_uri는_깊은_invalid_percent를_재귀없이_처리한다() {
    let input = format!("file:///{}%41", "%FF".repeat(DEEP_PERCENT_COUNT));
    let expected = format!("file:///{}A", "%FF".repeat(DEEP_PERCENT_COUNT));
    let input = input.encode_utf16().collect::<Vec<_>>();
    let expected = expected.encode_utf16().collect::<Vec<_>>();
    assert_eq!(normalize_id(&input, LIMITS).unwrap(), expected);
    assert_eq!(normalize_resource(&input, LIMITS).unwrap(), expected);
    let input = "file://HOST/\u{10000}?q#part"
        .encode_utf16()
        .collect::<Vec<_>>();
    assert_eq!(
        normalize_resource(&input, LIMITS).unwrap(),
        "file://host/\u{10000}".encode_utf16().collect::<Vec<_>>()
    );
}

#[test]
fn json_schema_uri는_입력_출력_작업예산과_손실없는_문자경계를_지킨다() {
    let input = "file:///schema.json".encode_utf16().collect::<Vec<_>>();
    assert_eq!(
        normalize_id(
            &input,
            Limits {
                max_input_units: input.len() - 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        normalize_id(
            &input,
            Limits {
                max_output_units: input.len() - 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        normalize_resource(
            &input,
            Limits {
                max_work: input.len(),
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    let input = "file://HOST/%23%3F".encode_utf16().collect::<Vec<_>>();
    let result = normalize_id(&input, LIMITS).unwrap();
    assert_eq!(
        normalize_id(
            &input,
            Limits {
                max_input_units: input.len(),
                max_output_units: result.len(),
                ..LIMITS
            }
        )
        .unwrap(),
        result
    );
    assert_eq!(
        normalize_id(&[], LIMITS).unwrap(),
        "file:///".encode_utf16().collect::<Vec<_>>()
    );
}
