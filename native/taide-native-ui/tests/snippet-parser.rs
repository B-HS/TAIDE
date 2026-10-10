use std::process::Command;

use serde_json::{Value, json};
use taide_model::ids::TabId;
use taide_native_editor::{
    document::{EditorError, LineEnding},
    indent::IndentOptions,
    snippet_expansion::expand,
    snippet_syntax::{
        FinalTabstopOptions, FormatPart, Marker, ParseLimits, RegexMetadata, Transform, parse,
        parse_complete,
    },
    snippet_variables::{OvertypedText, SelectionVariables, clipboard_value},
    snippet_whitespace::{WhitespaceContext, adjust_whitespace},
    store::{EditorLimits, EditorStore},
    view::{Selection, SelectionSet},
};

const MAX_BYTES: usize = 1024 * 1024;
const MAX_NESTING: usize = 64;
const MAX_MARKERS: usize = 4096;
const CASE_COUNT: usize = 34;
const REFUSED_BYTES: usize = 2;
const REFUSED_DEPTH: usize = 3;
const REFUSED_MARKERS: usize = 2;
const COMPLETE_CASE_COUNT: usize = 212;
const NORMALIZED_BYTE_BUDGET: usize = 32;
const NORMALIZED_MARKER_BUDGET: usize = 7;
const VARIABLE_CASE_COUNT: usize = 69;
const EXPANSION_BYTE_BUDGET: usize = 48;
const EXPANSION_VALUE_BYTES: usize = 20;
const WHITESPACE_CASE_COUNT: usize = 90;
const WHITESPACE_BYTE_BUDGET: usize = 16;
const OVERSIZED_TAB_SIZE: u32 = 1024;
const SELECTION_VARIABLE_CASE_COUNT: usize = 216;
const CLIPBOARD_VARIABLE_CASE_COUNT: usize = 14;
const VARIABLE_VALUE_BUDGET: usize = 8;

fn limits() -> ParseLimits {
    ParseLimits {
        max_bytes: MAX_BYTES,
        max_nesting: MAX_NESTING,
        max_markers: MAX_MARKERS,
    }
}

fn format(part: &FormatPart) -> Value {
    match part {
        FormatPart::Text(text) => json!({ "kind": "text", "text": text }),
        FormatPart::Capture {
            index,
            shorthand,
            if_value,
            else_value,
        } => json!({
            "kind": "capture", "index": index.value(), "shorthand": shorthand,
            "ifValue": if_value, "elseValue": else_value,
        }),
    }
}

fn transform(transform: &Option<Transform>) -> Value {
    match transform {
        None => Value::Null,
        Some(transform) => json!({
            "pattern": transform.pattern,
            "options": transform.options,
            "format": transform.format.iter().map(format).collect::<Vec<_>>(),
        }),
    }
}

fn marker(marker: &Marker) -> Value {
    match marker {
        Marker::Text(text) => json!({ "kind": "text", "text": text }),
        Marker::Placeholder {
            index,
            children,
            choices,
            transform: modification,
        } => json!({
            "kind": "placeholder", "index": index.value(),
            "children": children.iter().map(self::marker).collect::<Vec<_>>(),
            "choices": choices, "transform": transform(modification),
        }),
        Marker::Variable {
            name,
            children,
            transform: modification,
        } => json!({
            "kind": "variable", "name": name,
            "children": children.iter().map(self::marker).collect::<Vec<_>>(),
            "transform": transform(modification),
        }),
    }
}

fn number_values(value: Value) -> Value {
    match value {
        Value::Number(number) => json!(number.as_f64().unwrap()),
        Value::Array(values) => Value::Array(values.into_iter().map(number_values).collect()),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, number_values(value)))
                .collect(),
        ),
        value => value,
    }
}

fn regex_metadata(attempts: &[Value], pattern: &str, options: &str) -> Option<RegexMetadata> {
    attempts.iter().find_map(|attempt| {
        if attempt["pattern"].as_str() != Some(pattern)
            || attempt["options"].as_str() != Some(options)
            || attempt["accepted"].as_bool() != Some(true)
        {
            return None;
        }
        Some(RegexMetadata {
            source: attempt["source"].as_str().unwrap().into(),
            ignore_case: attempt["ignoreCase"].as_bool().unwrap(),
            global: attempt["global"].as_bool().unwrap(),
        })
    })
}

#[test]
fn snippet_variables는_실제_rope_선택과_원본_커서_문맥_clipboard분배를_보존한다() {
    let bun = std::env::var("TAIDE_M8_ORACLE_BUN").unwrap_or_else(|_| "bun".into());
    let oracle = Command::new(bun)
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snippet-variable-context-oracle.mjs"
        ))
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let cases: Value = serde_json::from_slice(&oracle.stdout).unwrap();
    let selections = cases["selectionCases"].as_array().unwrap();
    assert_eq!(selections.len(), SELECTION_VARIABLE_CASE_COUNT);
    let mut store = EditorStore::new(EditorLimits {
        max_documents: SELECTION_VARIABLE_CASE_COUNT,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: MAX_BYTES,
    })
    .unwrap();
    for case in selections {
        let context = &case["context"];
        let text = context["text"].as_str().unwrap();
        let document = store
            .open_untitled(TabId::new(), text, "plaintext".into())
            .unwrap();
        let snapshot = store.documents().snapshot(document).unwrap();
        let cursor = usize::try_from(context["cursor"].as_u64().unwrap()).unwrap();
        let count = usize::try_from(context["count"].as_u64().unwrap()).unwrap();
        let selection = Selection {
            anchor: usize::try_from(context["anchor"].as_u64().unwrap()).unwrap(),
            head: usize::try_from(context["head"].as_u64().unwrap()).unwrap(),
        };
        let selections = SelectionSet {
            primary: cursor,
            selections: vec![selection; count],
        };
        let resolver = SelectionVariables::new(&snapshot, &selections, cursor, MAX_BYTES).unwrap();
        let value = resolver
            .resolve(
                taide_native_editor::snippet_expansion::VariableContext {
                    name: case["name"].as_str().unwrap(),
                    preceding_text_line: case["precedingTextLine"].as_str(),
                },
                context["overtyped"].as_object().map(|_| OvertypedText {
                    value: context["overtyped"]["value"].as_str().unwrap(),
                    multiline: context["overtyped"]["multiline"].as_bool().unwrap(),
                }),
                |document, byte| {
                    assert_eq!(document.id, snapshot.id);
                    assert_eq!(byte, selection.head);
                    Ok(context["word"].as_str().map(String::from))
                },
            )
            .unwrap_or_else(|error| panic!("{case}: {error:?}"));
        assert_eq!(json!(value), case["expected"], "{case}");
        let after = store.documents().snapshot(document).unwrap();
        assert_eq!(snapshot.rope, after.rope);
        assert_eq!(snapshot.revision, after.revision);
        assert_eq!(snapshot.dirty, after.dirty);
        assert!(!store.undo(document).unwrap());
    }
    let clipboards = cases["clipboardCases"].as_array().unwrap();
    assert_eq!(clipboards.len(), CLIPBOARD_VARIABLE_CASE_COUNT);
    for case in clipboards {
        let value = clipboard_value(
            case["text"].as_str(),
            usize::try_from(case["cursor"].as_u64().unwrap()).unwrap(),
            usize::try_from(case["count"].as_u64().unwrap()).unwrap(),
            case["spread"].as_bool().unwrap(),
            MAX_BYTES,
        )
        .unwrap();
        assert_eq!(json!(value), case["expected"], "{case}");
    }
}

#[test]
fn snippet_variables는_경계_값예산과_언어_단어_실패를_거절한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: MAX_BYTES,
    })
    .unwrap();
    let document = store
        .open_untitled(TabId::new(), "한\r\nlast", "plaintext".into())
        .unwrap();
    let snapshot = store.documents().snapshot(document).unwrap();
    let selection = |anchor, head| SelectionSet {
        primary: 0,
        selections: vec![Selection { anchor, head }],
    };
    for selections in [
        selection(1, 1),
        selection(0, MAX_BYTES),
        selection("한\r".len(), "한\r".len()),
    ] {
        assert!(matches!(
            SelectionVariables::new(&snapshot, &selections, 0, MAX_BYTES),
            Err(EditorError::InvalidBoundary)
        ));
    }
    assert!(matches!(
        SelectionVariables::new(&snapshot, &selection(0, 0), 1, MAX_BYTES),
        Err(EditorError::InvalidBoundary)
    ));
    let resolver =
        SelectionVariables::new(&snapshot, &selection(0, 0), 0, VARIABLE_VALUE_BUDGET).unwrap();
    let context =
        |name, preceding_text_line| taide_native_editor::snippet_expansion::VariableContext {
            name,
            preceding_text_line,
        };
    assert!(matches!(
        resolver.resolve(context("TM_CURRENT_WORD", None), None, |_, _| Err(
            EditorError::Refused
        )),
        Err(EditorError::Refused)
    ));
    assert!(matches!(
        resolver.resolve(context("TM_CURRENT_WORD", None), None, |_, _| Ok(Some(
            "x".repeat(VARIABLE_VALUE_BUDGET + 1)
        ))),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        resolver.resolve(
            context("TM_SELECTED_TEXT", Some("\t\t\t\t\t\t\t\t")),
            Some(OvertypedText {
                value: "a\nb",
                multiline: true
            }),
            |_, _| Ok(None)
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        clipboard_value(Some("x"), 0, 0, true, MAX_BYTES),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        clipboard_value(Some("x"), 1, 1, true, MAX_BYTES),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        clipboard_value(Some("oversized"), 0, 1, true, VARIABLE_VALUE_BUDGET),
        Err(EditorError::Capacity)
    ));
    assert_eq!(
        store.documents().snapshot(document).unwrap().rope,
        snapshot.rope
    );
    assert!(!store.undo(document).unwrap());
}

#[test]
fn snippet_whitespace는_원본_삽입순서_들여쓰기와_eol_ast_범위를_보존한다() {
    let bun = std::env::var("TAIDE_M8_ORACLE_BUN").unwrap_or_else(|_| "bun".into());
    let oracle = Command::new(bun)
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snippet-parser-oracle.mjs"
        ))
        .arg("whitespace")
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let cases: Value = serde_json::from_slice(&oracle.stdout).unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), WHITESPACE_CASE_COUNT);
    for case in cases {
        let input = case["input"].as_str().unwrap();
        let attempts = case["attempts"].as_array().unwrap();
        let parsed = parse_complete(
            input,
            limits(),
            FinalTabstopOptions {
                insert: true,
                enforce: false,
            },
            |pattern, options| regex_metadata(attempts, pattern, options),
        )
        .unwrap();
        let context = &case["whitespace"];
        let adjusted = adjust_whitespace(
            &parsed,
            WhitespaceContext {
                line: context["line"].as_str().unwrap(),
                byte_column: context["byteColumn"].as_u64().unwrap().try_into().unwrap(),
                indent: IndentOptions {
                    tab_size: context["indentSize"].as_u64().unwrap().try_into().unwrap(),
                    insert_spaces: context["insertSpaces"].as_bool().unwrap(),
                },
                line_ending: match context["eol"].as_str().unwrap() {
                    "\n" => LineEnding::Lf,
                    "\r\n" => LineEnding::CrLf,
                    value => panic!("invalid oracle eol: {value:?}"),
                },
                adjust_indentation: context["adjust"].as_bool().unwrap(),
            },
            limits(),
        )
        .unwrap_or_else(|error| panic!("{input:?} {context}: {error:?}"));
        assert_eq!(
            adjusted.line_leading_whitespace,
            case["leading"].as_str().unwrap()
        );
        assert_eq!(
            Value::Array(adjusted.markers.iter().map(marker).collect()),
            number_values(case["expected"].clone()),
            "{input:?} {context}",
        );
        let expanded = expand(
            &adjusted.markers,
            limits(),
            |_| Ok(None),
            |_, _| panic!("unexpected variable transform"),
        )
        .unwrap();
        assert_eq!(
            expanded.text,
            case["text"].as_str().unwrap(),
            "{input:?} {context}"
        );
        let spans = expanded
            .placeholders
            .iter()
            .zip(case["spans"].as_array().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            expanded.placeholders.len(),
            case["spans"].as_array().unwrap().len()
        );
        for (span, expected) in spans {
            assert_eq!(
                json!(span.bytes.start),
                expected["start"],
                "{input:?} {context}"
            );
            assert_eq!(
                json!(span.bytes.end),
                expected["end"],
                "{input:?} {context}"
            );
            assert_eq!(
                json!(span.marker_path),
                expected["markerPath"],
                "{input:?} {context}"
            );
            assert_eq!(
                json!(span.enclosing),
                expected["enclosing"],
                "{input:?} {context}"
            );
            assert_eq!(
                json!(span.index.value()),
                number_values(expected["index"].clone()),
                "{input:?} {context}"
            );
        }
    }
}

#[test]
fn snippet_whitespace는_잘못된_문맥과_확장_예산을_입력변경_없이_거절한다() {
    let markers = vec![Marker::Text("\t한\n\t글".into())];
    let original = markers.clone();
    let context = |line, byte_column, tab_size| WhitespaceContext {
        line,
        byte_column,
        indent: IndentOptions {
            tab_size,
            insert_spaces: true,
        },
        line_ending: LineEnding::CrLf,
        adjust_indentation: true,
    };
    for invalid in [
        context("한", 1, 1),
        context("", 1, 1),
        context("\n", 0, 1),
        context("", 0, 0),
    ] {
        assert!(matches!(
            adjust_whitespace(&markers, invalid, limits()),
            Err(EditorError::InvalidBoundary)
        ));
    }
    let limited = ParseLimits {
        max_bytes: WHITESPACE_BYTE_BUDGET,
        ..limits()
    };
    assert!(matches!(
        adjust_whitespace(&markers, context("", 0, OVERSIZED_TAB_SIZE), limited),
        Err(EditorError::Capacity)
    ));
    let cumulative = vec![Marker::Text("\t".into()), Marker::Text("\n\t".into())];
    assert!(matches!(
        adjust_whitespace(
            &cumulative,
            context("", 0, WHITESPACE_BYTE_BUDGET as u32),
            limited
        ),
        Err(EditorError::Capacity)
    ));
    let eol = vec![Marker::Text("\n".repeat(WHITESPACE_BYTE_BUDGET))];
    assert!(matches!(
        adjust_whitespace(&eol, context("", 0, 1), limited),
        Err(EditorError::Capacity)
    ));
    assert_eq!(markers, original);
}

#[test]
fn snippet_expansion은_원본_variable해석_문맥_ast_text_범위와_부모관계를_보존한다() {
    let bun = std::env::var("TAIDE_M8_ORACLE_BUN").unwrap_or_else(|_| "bun".into());
    let oracle = Command::new(bun)
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snippet-parser-oracle.mjs"
        ))
        .arg("variables")
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let cases: Value = serde_json::from_slice(&oracle.stdout).unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), VARIABLE_CASE_COUNT);
    for case in cases {
        let input = case["input"].as_str().unwrap();
        let attempts = case["attempts"].as_array().unwrap();
        let parsed = parse_complete(
            input,
            limits(),
            FinalTabstopOptions {
                insert: true,
                enforce: false,
            },
            |pattern, options| regex_metadata(attempts, pattern, options),
        )
        .unwrap();
        let resolutions = case["resolutions"].as_array().unwrap();
        let evaluations = case["evaluations"].as_array().unwrap();
        let mut resolution = 0;
        let mut evaluation = 0;
        let expanded = expand(
            &parsed,
            limits(),
            |context| {
                let expected = &resolutions[resolution];
                resolution += 1;
                assert_eq!(expected["name"].as_str(), Some(context.name), "{input}");
                assert_eq!(
                    expected["precedingTextLine"].as_str(),
                    context.preceding_text_line,
                    "{input}"
                );
                Ok(case["variables"]
                    .get(context.name)
                    .and_then(Value::as_str)
                    .map(String::from))
            },
            |modification, value| {
                let expected = &evaluations[evaluation];
                evaluation += 1;
                assert_eq!(expected["value"].as_str(), Some(value), "{input}");
                assert_eq!(
                    number_values(expected["transform"].clone()),
                    number_values(transform(&Some(modification.clone()))),
                    "{input}"
                );
                Ok(expected["result"].as_str().unwrap().into())
            },
        )
        .unwrap_or_else(|error| panic!("{input}: {error:?}"));
        assert_eq!(resolution, resolutions.len(), "{input}");
        assert_eq!(evaluation, evaluations.len(), "{input}");
        assert_eq!(expanded.text, case["text"].as_str().unwrap(), "{input}");
        assert_eq!(
            Value::Array(expanded.markers.iter().map(marker).collect()),
            number_values(case["expected"].clone()),
            "{input}"
        );
        let spans = expanded
            .placeholders
            .iter()
            .map(|span| {
                let mut nodes = expanded.markers.as_slice();
                let mut node = None;
                for index in &span.marker_path {
                    let current = &nodes[*index];
                    node = Some(current);
                    nodes = match current {
                        Marker::Text(_) => &[],
                        Marker::Placeholder { children, .. }
                        | Marker::Variable { children, .. } => children,
                    };
                }
                let Some(Marker::Placeholder { choices, .. }) = node else {
                    panic!("invalid path {input}: {:?}", span.marker_path);
                };
                json!({
                    "index": span.index.value(), "start": span.bytes.start, "end": span.bytes.end,
                    "choices": choices, "markerPath": span.marker_path, "enclosing": span.enclosing,
                })
            })
            .collect();
        assert_eq!(
            number_values(Value::Array(spans)),
            number_values(case["spans"].clone()),
            "{input}"
        );
    }
}

#[test]
fn snippet_expansion은_해석_실패_확장예산과_잘못된_choice를_부분결과_없이_거절한다() {
    let options = FinalTabstopOptions {
        insert: true,
        enforce: false,
    };
    let parsed = parse_complete("$TM_CURRENT_LINE", limits(), options, |_, _| None).unwrap();
    for (limited, value) in [
        (
            ParseLimits {
                max_bytes: NORMALIZED_BYTE_BUDGET,
                ..limits()
            },
            "x".repeat(NORMALIZED_BYTE_BUDGET + 1),
        ),
        (
            ParseLimits {
                max_markers: 1,
                ..limits()
            },
            String::new(),
        ),
        (
            ParseLimits {
                max_nesting: 0,
                ..limits()
            },
            String::new(),
        ),
    ] {
        assert!(matches!(
            expand(
                &parsed,
                limited,
                |_| Ok(Some(value.clone())),
                |_, _| Err(EditorError::Refused)
            ),
            Err(EditorError::Capacity)
        ));
    }
    assert!(matches!(
        expand(
            &parsed,
            limits(),
            |_| Err(EditorError::Refused),
            |_, _| Err(EditorError::Refused)
        ),
        Err(EditorError::Refused)
    ));
    let repeated = parse_complete(
        "$TM_CURRENT_LINE$TM_CURRENT_LINE",
        limits(),
        options,
        |_, _| None,
    )
    .unwrap();
    assert!(matches!(
        expand(
            &repeated,
            ParseLimits {
                max_bytes: EXPANSION_BYTE_BUDGET,
                ..limits()
            },
            |_| Ok(Some("x".repeat(EXPANSION_VALUE_BYTES))),
            |_, _| Err(EditorError::Refused),
        ),
        Err(EditorError::Capacity)
    ));
    let transformed = parse_complete(
        "${TM_CURRENT_LINE/a/b/}",
        limits(),
        options,
        |pattern, _| {
            Some(RegexMetadata {
                source: pattern.into(),
                ignore_case: false,
                global: false,
            })
        },
    )
    .unwrap();
    assert!(matches!(
        expand(
            &transformed,
            limits(),
            |_| Ok(None),
            |_, _| Err(EditorError::Refused)
        ),
        Err(EditorError::Refused)
    ));
    let invalid = [Marker::Placeholder {
        index: taide_native_editor::snippet_syntax::Index::FINAL,
        children: Vec::new(),
        choices: Some(Vec::new()),
        transform: None,
    }];
    assert!(matches!(
        expand(
            &invalid,
            limits(),
            |_| Ok(None),
            |_, _| Err(EditorError::Refused)
        ),
        Err(EditorError::InvalidBoundary)
    ));
    assert_eq!(parsed.len(), 1);
    assert!(matches!(&parsed[0], Marker::Variable { children, .. } if children.is_empty()));
}

fn render(markers: &[Marker], text: &mut String, spans: &mut Vec<Value>) {
    for marker in markers {
        match marker {
            Marker::Text(value) => text.push_str(value),
            Marker::Variable { children, .. } => render(children, text, spans),
            Marker::Placeholder {
                index,
                children,
                choices,
                ..
            } => {
                let start = text.len();
                let span = spans.len();
                spans.push(Value::Null);
                if let Some(choices) = choices {
                    text.push_str(&choices[0]);
                } else {
                    render(children, text, spans);
                }
                spans[span] = json!({
                    "index": index.value(), "start": start, "end": text.len(), "choices": choices,
                });
            }
        }
    }
}

#[test]
fn snippet_normalization은_원본_기본값_참조_순환_choice_transform복제와_final_tabstop을_보존한다() {
    let bun = std::env::var("TAIDE_M8_ORACLE_BUN").unwrap_or_else(|_| "bun".into());
    let oracle = Command::new(bun)
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snippet-parser-oracle.mjs"
        ))
        .arg("complete")
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let cases: Value = serde_json::from_slice(&oracle.stdout).unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), COMPLETE_CASE_COUNT);
    for case in cases {
        let input = case["input"].as_str().unwrap();
        let attempts = case["attempts"].as_array().unwrap();
        let options = FinalTabstopOptions {
            insert: case["options"]["insert"].as_bool().unwrap(),
            enforce: case["options"]["enforce"].as_bool().unwrap(),
        };
        let parsed = parse_complete(input, limits(), options, |pattern, options| {
            attempts.iter().find_map(|attempt| {
                if attempt["pattern"].as_str() != Some(pattern)
                    || attempt["options"].as_str() != Some(options)
                    || attempt["accepted"].as_bool() != Some(true)
                {
                    return None;
                }
                Some(RegexMetadata {
                    source: attempt["source"].as_str().unwrap().into(),
                    ignore_case: attempt["ignoreCase"].as_bool().unwrap(),
                    global: attempt["global"].as_bool().unwrap(),
                })
            })
        })
        .unwrap_or_else(|error| panic!("{input}: {error:?}"));
        assert_eq!(
            Value::Array(parsed.iter().map(marker).collect()),
            number_values(case["correctedExpected"].clone()),
            "{input} {:?}; 원본 복제 옵션 누락은 보존 정책으로 교정",
            case["options"]
        );
        let mut text = String::new();
        let mut spans = Vec::new();
        render(&parsed, &mut text, &mut spans);
        assert_eq!(text, case["text"].as_str().unwrap(), "{input}");
        assert_eq!(
            number_values(Value::Array(spans)),
            number_values(case["spans"].clone()),
            "{input}"
        );
    }
}

#[test]
fn snippet_normalization은_기본값_복제의_byte_marker_깊이와_compiler_출력을_제한한다() {
    let options = FinalTabstopOptions {
        insert: true,
        enforce: false,
    };
    for (input, limits) in [
        (
            "${1:abcdefghij} $1 $1 $1",
            ParseLimits {
                max_bytes: NORMALIZED_BYTE_BUDGET,
                ..limits()
            },
        ),
        (
            "${1:a} $1 $1",
            ParseLimits {
                max_markers: NORMALIZED_MARKER_BUDGET,
                ..limits()
            },
        ),
        (
            "${1:${1:x}} $1",
            ParseLimits {
                max_nesting: REFUSED_DEPTH,
                ..limits()
            },
        ),
    ] {
        assert!(parse(input, limits, |_, _| true).is_ok(), "{input}");
        assert_eq!(
            parse_complete(input, limits, options, |_, _| None),
            Err(EditorError::Capacity),
            "{input}"
        );
    }
    assert_eq!(
        parse_complete(
            "${1/a/b/}",
            ParseLimits {
                max_bytes: NORMALIZED_BYTE_BUDGET,
                ..limits()
            },
            options,
            |_, _| Some(RegexMetadata {
                source: "a".repeat(NORMALIZED_BYTE_BUDGET + 1),
                ignore_case: false,
                global: false,
            }),
        ),
        Err(EditorError::Capacity)
    );
}

#[test]
fn 설치된_monaco의_실제_raw문법_ast와_escape_choice_변수_transform을_대조한다() {
    let bun = std::env::var("TAIDE_M8_ORACLE_BUN").unwrap_or_else(|_| "bun".into());
    let oracle = Command::new(bun)
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/snippet-parser-oracle.mjs"
        ))
        .output()
        .unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let cases: Value = serde_json::from_slice(&oracle.stdout).unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), CASE_COUNT);
    for case in cases {
        let input = case["input"].as_str().unwrap();
        let attempts = case["attempts"].as_array().unwrap();
        let parsed = parse(input, limits(), |pattern, options| {
            attempts.iter().any(|attempt| {
                attempt["pattern"].as_str() == Some(pattern)
                    && attempt["options"].as_str() == Some(options)
                    && attempt["accepted"].as_bool() == Some(true)
            })
        })
        .unwrap();
        assert_eq!(
            Value::Array(parsed.iter().map(marker).collect()),
            number_values(case["expected"].clone()),
            "{input}"
        );
    }
}

#[test]
fn snippet_문법은_utf8_byte_중첩_marker_예산을_초과하기_전에_거절한다() {
    for (input, limits) in [
        (
            "한",
            ParseLimits {
                max_bytes: REFUSED_BYTES,
                ..limits()
            },
        ),
        (
            "${1:${2:${3:${4:nested}}}}",
            ParseLimits {
                max_nesting: REFUSED_DEPTH,
                ..limits()
            },
        ),
        (
            "$1$2$3",
            ParseLimits {
                max_markers: REFUSED_MARKERS,
                ..limits()
            },
        ),
        (
            "",
            ParseLimits {
                max_markers: 0,
                ..limits()
            },
        ),
        (
            "",
            ParseLimits {
                max_nesting: usize::MAX,
                ..limits()
            },
        ),
    ] {
        assert_eq!(
            parse(input, limits, |_, _| true),
            Err(EditorError::Capacity)
        );
    }
    assert_eq!(parse("", limits(), |_, _| true), Ok(Vec::new()));
    assert!(parse("${1:a}", limits(), |_, _| true).is_ok());
}
