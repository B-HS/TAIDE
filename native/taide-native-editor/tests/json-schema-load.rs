use std::collections::VecDeque;

use taide_native_editor::document::EditorError;
use taide_native_editor::json_schema_load::{self, Limits, Response};
use taide_native_editor::json_schema_uri;
use taide_native_editor::json_value::{Arena, Value};

const LIMITS: Limits = Limits {
    scanner: taide_native_editor::json_scanner::Limits {
        max_bytes: 128 * 1024,
        max_tokens: 128 * 1024,
        max_value_units: 128 * 1024,
    },
    uri: json_schema_uri::Limits {
        max_input_units: 64 * 1024,
        max_output_units: 256 * 1024,
        max_work: 1024 * 1024,
    },
    max_values: 32 * 1024,
    max_depth: 16 * 1024,
    max_text_units: 128 * 1024,
    max_work: 1024 * 1024,
    max_messages: 16,
    max_message_units: 256 * 1024,
};
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 8192;
const RADIX: u32 = 16;

enum ResponseInput {
    Text(&'static str),
    Failed(&'static str),
    Unavailable,
}
include!("fixtures/json-schema-load-oracle.rs");

fn units(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}
fn hex(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe(arena: &Arena) -> String {
    let Some(root) = arena.root() else {
        return "undefined".to_owned();
    };
    let mut queue = VecDeque::from([(root, "-1".to_owned(), "-".to_owned())]);
    let mut result = Vec::new();
    while let Some((index, parent, edge)) = queue.pop_front() {
        let cursor = result.len();
        let mut content = String::new();
        let kind = match arena.get(index).unwrap() {
            Value::Null => "null",
            Value::Boolean(value) => {
                content = value.to_string();
                "boolean"
            }
            Value::Number(value) => {
                content = format!("{:016x}", value.to_bits());
                "number"
            }
            Value::String(value) => {
                content = hex(value);
                "string"
            }
            Value::Object(members) => {
                for member in members {
                    queue.push_back((
                        member.value,
                        cursor.to_string(),
                        format!("k{}", hex(&member.key)),
                    ));
                }
                "object"
            }
            Value::Array(items) => {
                for (position, &item) in items.iter().enumerate() {
                    queue.push_back((item, cursor.to_string(), format!("i{position}")));
                }
                "array"
            }
        };
        result.push(format!("{kind}:{parent}:{edge}:{content};"));
    }
    result.join("")
}

#[test]
fn json_schema_load는_실제_jsonc의_값과_첫오류위치에_일치한다() {
    let mut mismatches = Vec::new();
    for &(input, raw, expected) in RAW_CASES {
        let parsed = json_schema_load::parse_content(input, LIMITS).unwrap();
        let offset = parsed
            .first_error
            .map_or("-".to_owned(), |offset| offset.to_string());
        let actual = format!("{}|{offset}", describe(&parsed.schema));
        if actual != expected {
            mismatches.push(format!("{input:?}: {actual}/{expected}, raw={raw}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(SAMPLE_COUNT)].join("\n")
    );
}

mod cache {
    use super::{LIMITS as LOAD_LIMITS, units};
    use taide_native_editor::document::EditorError;
    use taide_native_editor::json_schema_associations::{Pattern, PatternCompiler};
    use taide_native_editor::json_schema_load::{self, Response};
    use taide_native_editor::json_schema_refs::resolve_arena;
    use taide_native_editor::json_schema_service::{LoadState, ResolveState, Service};

    include!("fixtures/json-schema-service-limits.rs");

    struct UnusedCompiler;
    impl PatternCompiler for UnusedCompiler {
        fn compile(&self, _: &[u16]) -> Result<Box<dyn Pattern>, EditorError> {
            Err(EditorError::InvalidBoundary)
        }
    }

    #[test]
    fn json_schema_response는_실제cache와_요청소유_만료에_연결된다() {
        let mut service = Service::new(&UnusedCompiler, LIMITS);
        let uri = units("http://json-schema.org/draft-07/schema");
        let LoadState::Start(ticket) = service.begin_load(&uri).unwrap() else {
            panic!("missing request");
        };
        assert_eq!(
            json_schema_load::request_url(service.request_uri(&ticket).unwrap(), URI_LIMITS)
                .unwrap(),
            units("https://json-schema.org/draft-07/schema")
        );
        json_schema_load::publish_response(
            &mut service,
            &ticket,
            Response::Text("\u{feff} {\"type\":\"string\"}"),
            false,
            LOAD_LIMITS,
        )
        .unwrap();
        let LoadState::Ready(first) = service.begin_load(&uri).unwrap() else {
            panic!("missing cached response");
        };
        assert_eq!(first.errors.len(), 1);
        let ResolveState::Start {
            ticket: resolve_ticket,
            source: LoadState::Ready(source),
        } = service.begin_resolve(&uri).unwrap()
        else {
            panic!("missing cached source");
        };
        assert!(std::sync::Arc::ptr_eq(&first, &source));
        let local = resolve_arena(
            (*source.schema).clone(),
            "http://json-schema.org/draft-07/schema",
            REF_LIMITS,
        )
        .unwrap();
        assert!(local.external.is_empty());
        assert!(service.on_resource_change(&uri).unwrap());
        assert_eq!(
            json_schema_load::publish_response(
                &mut service,
                &ticket,
                Response::Text("{}"),
                false,
                LOAD_LIMITS
            ),
            Err(EditorError::StaleRevision)
        );
        assert_eq!(
            service.publish_resolved(&resolve_ticket, local, &[]),
            Err(EditorError::StaleRevision)
        );
        let LoadState::Start(fresh) = service.begin_load(&uri).unwrap() else {
            panic!("missing fresh request");
        };
        json_schema_load::publish_response(
            &mut service,
            &fresh,
            Response::Text("false"),
            false,
            LOAD_LIMITS,
        )
        .unwrap();
        let LoadState::Ready(source) = service.begin_load(&uri).unwrap() else {
            panic!("missing boolean schema");
        };
        assert!(matches!(
            source.schema.get(source.schema.root().unwrap()),
            Some(taide_native_editor::json_value::Value::Boolean(false))
        ));
    }
}

#[test]
fn json_schema_load는_실제응답_오류와_파일표시에_일치한다() {
    let mut mismatches = Vec::new();
    for &(uri, ref response, raw, expected) in LOAD_CASES {
        let uri_units = units(uri);
        let error = if let ResponseInput::Failed(error) = response {
            units(error)
        } else {
            Vec::new()
        };
        let response = match response {
            ResponseInput::Text(input) => Response::Text(input),
            ResponseInput::Failed(_) => Response::Failed(&error),
            ResponseInput::Unavailable => Response::Unavailable,
        };
        let result = json_schema_load::decode(&uri_units, response, false, LIMITS).unwrap();
        let errors = result
            .errors
            .iter()
            .map(|error| hex(error))
            .collect::<Vec<_>>()
            .join(";");
        let actual = format!("{}|{errors}", describe(&result.schema));
        if actual != expected {
            mismatches.push(format!("{uri:?}: {actual}/{expected}, raw={raw}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} mismatches: {}",
        mismatches.len(),
        mismatches[..mismatches.len().min(SAMPLE_COUNT)].join("\n")
    );
    for &(uri, is_windows, expected) in DISPLAY_CASES {
        let expected = expected
            .split(',')
            .map(|unit| u16::from_str_radix(unit, RADIX).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            json_schema_uri::display_string(&units(uri), is_windows, LIMITS.uri).unwrap(),
            expected
        );
    }
}

#[test]
fn json_schema_load는_명시stack과_입력_깊이_값_문자_작업_메시지예산을_지킨다() {
    let input = format!("{}0{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    assert!(
        json_schema_load::parse_content(&input, LIMITS)
            .unwrap()
            .first_error
            .is_none()
    );
    for limits in [
        Limits {
            max_depth: DEEP_NESTING - 1,
            ..LIMITS
        },
        Limits {
            max_values: 0,
            ..LIMITS
        },
        Limits {
            max_work: 0,
            ..LIMITS
        },
    ] {
        assert!(matches!(
            json_schema_load::parse_content(&input, limits),
            Err(EditorError::Capacity)
        ));
    }
    assert!(matches!(
        json_schema_load::parse_content(
            "{\"a\":\"b\"}",
            Limits {
                max_text_units: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        json_schema_load::parse_content(
            "{}",
            Limits {
                scanner: taide_native_editor::json_scanner::Limits {
                    max_bytes: 0,
                    ..LIMITS.scanner
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    let uri = units("http://json-schema.org/draft-07/schema");
    assert_eq!(
        json_schema_load::request_url(&uri, LIMITS.uri).unwrap(),
        units("https://json-schema.org/draft-07/schema")
    );
    assert!(matches!(
        json_schema_load::decode(
            &uri,
            Response::Unavailable,
            false,
            Limits {
                max_messages: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        json_schema_load::decode(
            &uri,
            Response::Failed(&units("Error: owned")),
            false,
            Limits {
                max_message_units: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        json_schema_load::decode(
            &uri,
            Response::Text("{}"),
            false,
            Limits {
                max_messages: 0,
                max_message_units: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .errors
        .is_empty()
    );
    let bounded = json_schema_load::decode(
        &uri,
        Response::Unavailable,
        false,
        Limits {
            uri: json_schema_uri::Limits {
                max_input_units: uri.len(),
                max_output_units: uri.len(),
                ..LIMITS.uri
            },
            ..LIMITS
        },
    )
    .unwrap();
    assert_eq!(bounded.errors.len(), 1);
}
