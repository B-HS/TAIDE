use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser::{Document, Kind, Limits, Value, parse};
use taide_native_editor::json_scanner;

const LIMITS: Limits = Limits {
    scanner: json_scanner::Limits {
        max_bytes: 64 * 1024,
        max_tokens: 128 * 1024,
        max_value_units: 64 * 1024,
    },
    max_nodes: 32 * 1024,
    max_depth: 16 * 1024,
    max_diagnostics: 32 * 1024,
    max_comments: 32 * 1024,
};
const KINDS: &[Kind] = &[
    Kind::Object,
    Kind::Array,
    Kind::Property,
    Kind::String,
    Kind::Number,
    Kind::Boolean,
    Kind::Null,
];
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 8192;
const DEEP_OBJECT_NESTING: usize = 4096;
const EOF_WHITESPACE: usize = 4096;

include!("fixtures/json-parser-oracle.rs");

fn describe(document: &Document) -> String {
    let nodes = document
        .nodes
        .iter()
        .map(|node| {
            let kind = KINDS.iter().position(|kind| *kind == node.kind).unwrap();
            let parent = node
                .parent
                .map_or("-1".to_owned(), |parent| parent.to_string());
            let colon = node
                .colon_utf16
                .map_or("-1".to_owned(), |colon| colon.to_string());
            let value = match &node.value {
                Value::None | Value::Null => String::new(),
                Value::String(value) => value
                    .iter()
                    .map(|unit| format!("{unit:x}"))
                    .collect::<Vec<_>>()
                    .join(","),
                Value::Number(value) if value.is_nan() => "nan".to_owned(),
                Value::Number(value) => format!("{:016x}", value.to_bits()),
                Value::Boolean(value) => value.to_string(),
            };
            format!(
                "{kind}:{}..{}:{parent}:{colon}:{}:{value};",
                node.utf16.start,
                node.utf16.end,
                usize::from(node.is_integer)
            )
        })
        .collect::<String>();
    let errors = document
        .diagnostics
        .iter()
        .map(|error| {
            let severity = if error.is_warning { 2 } else { 1 };
            format!(
                "{}:{severity}:{}..{};",
                error.error.code(),
                error.utf16.start,
                error.utf16.end
            )
        })
        .collect::<String>();
    let comments = document
        .comments
        .iter()
        .map(|comment| format!("{}..{};", comment.utf16.start, comment.utf16.end))
        .collect::<String>();
    format!("{nodes}|{errors}|{comments}")
}

#[test]
fn json_parser는_실제언어서비스의_ast_회복_진단과_주석에_일치한다() {
    let mut mismatches = Vec::new();
    for &(input, collect_comments, raw, expected) in CASES {
        let actual = parse(input, collect_comments, LIMITS).map(|document| describe(&document));
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!("{input:?}/{collect_comments}: actual={actual:?}, expected={expected:?}, source={raw:?}"));
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
fn json_parser는_원문경계_부모자식과_논리utf16값을_보존한다() {
    for &(input, collect_comments, _, _) in CASES {
        let document = parse(input, collect_comments, LIMITS).unwrap();
        if let Some(root) = document.root {
            assert_eq!(root, 0);
        } else {
            assert!(document.nodes.is_empty());
        }
        for (index, node) in document.nodes.iter().enumerate() {
            assert!(node.bytes.start <= node.bytes.end && node.bytes.end <= input.len());
            assert!(
                input.is_char_boundary(node.bytes.start) && input.is_char_boundary(node.bytes.end)
            );
            assert_eq!(
                input[..node.bytes.start].encode_utf16().count(),
                node.utf16.start
            );
            assert_eq!(
                input[..node.bytes.end].encode_utf16().count(),
                node.utf16.end
            );
            if let Some(parent) = node.parent {
                assert!(parent < index);
                assert!(document.nodes[parent].children.contains(&index));
            }
            for &child in &node.children {
                assert_eq!(document.nodes[child].parent, Some(index));
            }
        }
        for diagnostic in &document.diagnostics {
            assert!(
                input.is_char_boundary(diagnostic.bytes.start)
                    && input.is_char_boundary(diagnostic.bytes.end)
            );
            assert!(
                diagnostic.bytes.start <= diagnostic.bytes.end
                    && diagnostic.bytes.end <= input.len()
            );
            assert_eq!(
                input[..diagnostic.bytes.start].encode_utf16().count(),
                diagnostic.utf16.start
            );
            assert_eq!(
                input[..diagnostic.bytes.end].encode_utf16().count(),
                diagnostic.utf16.end
            );
        }
    }
    let document = parse("[\"\\uD800\\uDFFF\"]", true, LIMITS).unwrap();
    assert_eq!(document.nodes[1].value, Value::String(vec![0xd800, 0xdfff]));
}

#[test]
fn json_parser는_명시스택과_노드_깊이_진단_주석예산을_부분결과없이_지킨다() {
    let input = format!("{}0{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    let document = parse(&input, true, LIMITS).unwrap();
    assert_eq!(document.nodes.len(), DEEP_NESTING + 1);
    assert_eq!(
        document.node_at_utf16(DEEP_NESTING, false),
        Some(DEEP_NESTING)
    );
    let object_input = format!(
        "{}0{}",
        "{\"a\":".repeat(DEEP_OBJECT_NESTING),
        "}".repeat(DEEP_OBJECT_NESTING)
    );
    let object_document = parse(&object_input, true, LIMITS).unwrap();
    assert_eq!(object_document.nodes.len(), DEEP_OBJECT_NESTING * 3 + 1);
    let incomplete = format!(
        "{}0{}",
        "[".repeat(DEEP_NESTING),
        " ".repeat(EOF_WHITESPACE)
    );
    let incomplete_document = parse(&incomplete, true, LIMITS).unwrap();
    assert_eq!(incomplete_document.nodes.len(), DEEP_NESTING + 1);
    assert_eq!(incomplete_document.diagnostics.len(), 1);
    assert_eq!(
        incomplete_document.diagnostics[0].bytes,
        DEEP_NESTING..DEEP_NESTING + 1
    );
    for limits in [
        Limits {
            max_depth: DEEP_NESTING,
            ..LIMITS
        },
        Limits {
            max_nodes: DEEP_NESTING,
            ..LIMITS
        },
    ] {
        assert!(matches!(
            parse(&input, true, limits),
            Err(EditorError::Capacity)
        ));
    }
    assert!(matches!(
        parse(
            "{\"a\":1}",
            true,
            Limits {
                max_nodes: 3,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        parse(
            "{\"a\":1}",
            true,
            Limits {
                max_depth: 2,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        parse(
            "{a:1}",
            true,
            Limits {
                max_diagnostics: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        parse(
            "//a\n{}",
            true,
            Limits {
                max_comments: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        parse(
            "//a\n{}",
            false,
            Limits {
                max_comments: 0,
                ..LIMITS
            }
        )
        .is_ok()
    );
    assert!(
        parse(
            " ",
            true,
            Limits {
                max_nodes: 0,
                max_depth: 0,
                max_diagnostics: 0,
                max_comments: 0,
                ..LIMITS
            }
        )
        .is_ok()
    );
    assert!(matches!(
        parse(
            "0",
            true,
            Limits {
                max_nodes: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
}

#[test]
fn json_parser의_가장안쪽노드_조회는_원본의_좌우경계와_utf16을_보존한다() {
    for &(input, offset, include_right_bound, expected) in QUERY_CASES {
        let document = parse(input, true, LIMITS).unwrap();
        assert_eq!(
            document.node_at_utf16(offset, include_right_bound),
            expected,
            "{input:?}/{offset}/{include_right_bound}"
        );
        let byte = input
            .char_indices()
            .map(|(byte, _)| byte)
            .chain([input.len()])
            .find(|&byte| input[..byte].encode_utf16().count() == offset);
        if let Some(byte) = byte {
            assert_eq!(
                document.node_at_byte(byte, include_right_bound),
                expected,
                "{input:?}/{byte}/{include_right_bound}"
            );
        }
    }
}
