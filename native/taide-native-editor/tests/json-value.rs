use std::collections::VecDeque;

use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_value::{Arena, Limits, Value, parse};

const LIMITS: Limits = Limits {
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
const SAMPLE_COUNT: usize = 8;
const MAX_EQUAL_WORK: usize = 64 * 1024;
const DEEP_NESTING: usize = 8192;

include!("fixtures/json-value-oracle.rs");

fn units(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe(arena: &Arena) -> String {
    let Some(root) = arena.root() else {
        return "undefined".into();
    };
    let mut pending = VecDeque::from([(root, "-1".to_owned(), "-".to_owned())]);
    let mut parts = Vec::new();
    while let Some((index, parent, edge)) = pending.pop_front() {
        let node = arena.get(index).unwrap();
        let current = parts.len();
        let mut content = String::new();
        let mut length = "-".to_owned();
        let kind = match node {
            Value::Null => "null",
            Value::Boolean(value) => {
                content = value.to_string();
                "boolean"
            }
            Value::Number(value) => {
                content = if value.is_nan() {
                    "nan".into()
                } else {
                    format!("{:016x}", value.to_bits())
                };
                "number"
            }
            Value::String(value) => {
                content = units(value);
                length = node.string_length().unwrap().to_string();
                "string"
            }
            Value::Array(items) => {
                for (index, &item) in items.iter().enumerate() {
                    pending.push_back((item, current.to_string(), format!("i{index}")))
                }
                "array"
            }
            Value::Object(members) => {
                for member in members {
                    pending.push_back((
                        member.value,
                        current.to_string(),
                        format!("k{}", units(&member.key)),
                    ))
                }
                "object"
            }
        };
        parts.push(format!(
            "{kind}:{parent}:{edge}:{content}:{length}:{};",
            node.is_truthy()
        ));
    }
    parts.join("")
}

#[test]
fn json_value는_실제get_node_value의_값_키순서와_논리utf16을_보존한다() {
    let mut mismatches = Vec::new();
    for &(input, raw, expected) in CASES {
        let actual = parse(input, LIMITS).map(|arena| describe(&arena));
        if raw != expected {
            assert!(input.contains("\\uD800") || input.contains("\\uDFFF"));
        }
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!(
                "{input:?}: actual={actual:?}, expected={expected:?}, source={raw:?}"
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
fn json_value의_명시비교는_실제equals의_숫자_객체키_배열과_동일객체를_보존한다() {
    for &(one, two, same_instance, expected) in EQUALITY_CASES {
        let one = parse(one, LIMITS).unwrap();
        let two_arena = parse(two, LIMITS).unwrap();
        let two = if same_instance { &one } else { &two_arena };
        assert_eq!(
            one.equals(
                one.root().unwrap(),
                two,
                two.root().unwrap(),
                MAX_EQUAL_WORK
            )
            .unwrap(),
            expected,
            "{one:?}/{two:?}/{same_instance}"
        );
    }
}

#[test]
fn json_value는_깊은값_멤버조회_노드_텍스트_비교예산을_지킨다() {
    let input = format!("{}0{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    let one = parse(&input, LIMITS).unwrap();
    let two = parse(&input, LIMITS).unwrap();
    assert_eq!(one.values().len(), DEEP_NESTING + 1);
    assert!(
        one.equals(
            one.root().unwrap(),
            &two,
            two.root().unwrap(),
            MAX_EQUAL_WORK
        )
        .unwrap()
    );
    assert!(matches!(
        one.equals(one.root().unwrap(), &two, two.root().unwrap(), DEEP_NESTING),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        parse(
            &input,
            Limits {
                max_values: DEEP_NESTING,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        parse(
            "{\"a\":\"b\"}",
            Limits {
                max_text_units: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        parse(
            "{}",
            Limits {
                max_values: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        parse(
            "",
            Limits {
                max_values: 0,
                max_text_units: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .root()
        .is_none()
    );
    let arena = parse(
        "{\"a\":[1],\"a\":[2],\"__proto__\":true,\"\\uD800\":\"\\uD800a\"}",
        LIMITS,
    )
    .unwrap();
    let root = arena.root().unwrap();
    let key = "a".encode_utf16().collect::<Vec<_>>();
    let array = arena.member(root, &key).unwrap();
    assert_eq!(
        arena.get(arena.item(array, 0).unwrap()),
        Some(&Value::Number(2.0))
    );
    assert!(arena.item(array, 1).is_none());
    assert!(arena.member(root, &[0xdfff]).is_none());
    let string = arena.get(arena.member(root, &[0xd800]).unwrap()).unwrap();
    assert_eq!(string, &Value::String(vec![0xd800, 0x61]));
    assert_eq!(string.string_length(), Some(2));
    assert!(matches!(
        arena.equals(root, &arena, root, 0),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        arena.equals(usize::MAX, &arena, root, MAX_EQUAL_WORK),
        Err(EditorError::InvalidBoundary)
    ));
    let document = json_parser::parse("{\"a\":1}", false, LIMITS.parser).unwrap();
    for parser in [
        json_parser::Limits {
            max_nodes: 0,
            ..LIMITS.parser
        },
        json_parser::Limits {
            max_depth: 1,
            ..LIMITS.parser
        },
    ] {
        assert!(matches!(
            Arena::from_document(&document, Limits { parser, ..LIMITS }),
            Err(EditorError::Capacity)
        ));
    }
}
