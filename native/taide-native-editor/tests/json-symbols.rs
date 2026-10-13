use taide_native_editor::document::EditorError;
use taide_native_editor::json_parser;
use taide_native_editor::json_scanner;
use taide_native_editor::json_symbols::{Limits, SymbolKind, Symbols, document_symbols};

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
    max_symbols: 16 * 1024,
    max_text_units: 4 * 1024 * 1024,
    result_limit: None,
};
const KIND_CODES: &[(SymbolKind, usize)] = &[
    (SymbolKind::MODULE, 2),
    (SymbolKind::FUNCTION, 12),
    (SymbolKind::VARIABLE, 13),
    (SymbolKind::STRING, 15),
    (SymbolKind::NUMBER, 16),
    (SymbolKind::BOOLEAN, 17),
    (SymbolKind::ARRAY, 18),
];
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 4096;

include!("fixtures/json-symbols-oracle.rs");

fn units(value: &[u16]) -> String {
    value
        .iter()
        .map(|unit| format!("{unit:x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn describe(symbols: &Symbols) -> String {
    let parts = symbols
        .items
        .iter()
        .map(|symbol| {
            let kind = KIND_CODES
                .iter()
                .find(|(kind, _)| *kind == symbol.kind)
                .unwrap()
                .1;
            let parent = symbol
                .parent
                .map_or("-1".to_owned(), |index| index.to_string());
            let detail = symbol.detail.as_deref().map_or("-".to_owned(), units);
            let container = symbol
                .container_name
                .as_deref()
                .map_or("-".to_owned(), units);
            format!(
                "{kind}:{parent}:{}..{}:{}..{}:string:{}:{detail}:{container};",
                symbol.utf16.start,
                symbol.utf16.end,
                symbol.selection_utf16.start,
                symbol.selection_utf16.end,
                units(&symbol.name)
            )
        })
        .collect::<String>();
    format!("{parts}|{}", symbols.limit_exceeded)
}

#[test]
fn json_symbols는_실제원본의_계층_평면_표시_제한과_keybindings에_일치한다() {
    let mut mismatches = Vec::new();
    for &(input, uri, hierarchical, result_limit, raw, expected) in CASES {
        let actual = document_symbols(
            input,
            uri,
            hierarchical,
            Limits {
                result_limit,
                ..LIMITS
            },
        )
        .map(|symbols| describe(&symbols));
        if actual.as_deref() != Ok(expected) {
            mismatches.push(format!("{input:?}/{uri}/{hierarchical}/{result_limit:?}: actual={actual:?}, expected={expected:?}, source={raw:?}"));
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
fn json_symbols는_모든원문범위_부모관계와_논리utf16값을_보존한다() {
    for &(input, uri, hierarchical, result_limit, _, _) in CASES {
        let symbols = document_symbols(
            input,
            uri,
            hierarchical,
            Limits {
                result_limit,
                ..LIMITS
            },
        )
        .unwrap();
        for (index, symbol) in symbols.items.iter().enumerate() {
            assert!(
                symbol.bytes.start <= symbol.selection_bytes.start
                    && symbol.selection_bytes.end <= symbol.bytes.end
            );
            for (bytes, utf16) in [
                (&symbol.bytes, &symbol.utf16),
                (&symbol.selection_bytes, &symbol.selection_utf16),
            ] {
                assert!(bytes.start <= bytes.end && bytes.end <= input.len());
                assert!(input.is_char_boundary(bytes.start) && input.is_char_boundary(bytes.end));
                assert_eq!(input[..bytes.start].encode_utf16().count(), utf16.start);
                assert_eq!(input[..bytes.end].encode_utf16().count(), utf16.end);
            }
            if let Some(parent) = symbol.parent {
                assert!(parent < index);
                assert!(
                    symbols.items[parent].bytes.start <= symbol.bytes.start
                        && symbol.bytes.end <= symbols.items[parent].bytes.end
                );
            }
        }
    }
    let symbols =
        document_symbols("{\"\\uD800\":\"\\uDFFF\"}", "file:///a.json", true, LIMITS).unwrap();
    assert_eq!(symbols.items[0].name, [0xd800]);
    assert_eq!(symbols.items[0].detail, Some(vec![0xdfff]));
}

#[test]
fn json_symbols는_깊은문서_표시값_전체심볼예산과_원본결과제한을_구분한다() {
    let input = format!("{}0{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    let symbols = document_symbols(&input, "file:///a.json", true, LIMITS).unwrap();
    assert_eq!(symbols.items.len(), DEEP_NESTING);
    assert!(!symbols.limit_exceeded);
    assert!(matches!(
        document_symbols(
            &input,
            "file:///a.json",
            true,
            Limits {
                max_symbols: DEEP_NESTING - 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    let limited = document_symbols(
        &input,
        "file:///a.json",
        true,
        Limits {
            result_limit: Some(1),
            ..LIMITS
        },
    )
    .unwrap();
    assert_eq!(limited.items.len(), 1);
    assert!(limited.limit_exceeded);
    assert!(matches!(
        document_symbols(
            "{\"a\":\"long\"}",
            "file:///a.json",
            true,
            Limits {
                max_text_units: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        document_symbols(
            "{\"a\":0}",
            "file:///a.json",
            false,
            Limits {
                max_text_units: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        document_symbols(
            "{\"a\":0}",
            "file:///a.json",
            true,
            Limits {
                parser: json_parser::Limits {
                    max_nodes: 0,
                    ..LIMITS.parser
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        document_symbols(
            "{}",
            "file:///a.json",
            true,
            Limits {
                max_symbols: 0,
                max_text_units: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .items
        .is_empty()
    );
}
