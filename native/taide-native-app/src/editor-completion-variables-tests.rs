use super::*;
use chrono::TimeZone;
use serde::Deserialize;
use taide_model::ids::TabId;
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::Selection;

const BYTE_LIMIT: usize = 4096;
const OWNER_LIMIT: usize = 2;
const UNDO_LIMIT: usize = 4;
const MODEL_PATH: &str = "/work/example.tar.rs";
const REFERENCE: &str = include_str!("fixtures/snippet-variables-reference.json");
const RANDOM_VALUE: u128 = 0x1234_1234_1234_4234_8234_1234_1234_1234;

#[derive(Deserialize)]
struct Reference {
    models: Vec<ModelCase>,
    times: Vec<TimeCase>,
    workspace: Vec<Value>,
}

#[derive(Deserialize)]
struct ModelCase {
    path: String,
    values: Vec<Value>,
}

#[derive(Deserialize)]
struct TimeCase {
    date: String,
    offset: i32,
    values: Vec<Value>,
}

#[derive(Deserialize)]
struct Value {
    name: String,
    expected: Option<String>,
}

fn clock() -> Clock {
    Clock {
        date: FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 10, 12, 0, 0)
            .unwrap(),
        timezone_name: Some("Etc/UTC".into()),
    }
}

fn snapshot() -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: OWNER_LIMIT,
        max_views: OWNER_LIMIT,
        max_undo_groups: UNDO_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let id = store
        .open_untitled(TabId::new(), "alpha βeta\nomega", "rust".into())
        .unwrap();
    store.documents().snapshot(id).unwrap()
}

fn selections() -> SelectionSet {
    SelectionSet {
        primary: 0,
        selections: vec![
            Selection { anchor: 0, head: 5 },
            Selection {
                anchor: 12,
                head: 17,
            },
        ],
    }
}

#[test]
fn completion_model_variables는_실제_monaco_파일_경로와_일치한다() {
    let reference: Reference = serde_json::from_str(REFERENCE).unwrap();
    for case in reference.models {
        for value in case.values {
            assert_eq!(
                model_variable(&case.path, &value.name),
                value.expected,
                "{} {}",
                case.path,
                value.name
            );
        }
    }
}

#[test]
fn completion_time_variables는_실제_monaco_윤일_경계와_밀리초를_보존한다() {
    let reference: Reference = serde_json::from_str(REFERENCE).unwrap();
    for case in reference.times {
        let timezone_name = case
            .values
            .iter()
            .find(|value| value.name == "CURRENT_TIMEZONE_NAME")
            .unwrap()
            .expected
            .clone();
        let clock = Clock {
            date: DateTime::parse_from_rfc3339(&case.date)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(case.offset).unwrap()),
            timezone_name,
        };
        for value in case.values {
            assert_eq!(
                clock.resolve(&value.name),
                value.expected,
                "{} {}",
                case.date,
                value.name
            );
        }
    }
}

#[test]
fn completion_variables는_선택_언어_클립보드_작업공간과_난수를_평가한다() {
    let snapshot = snapshot();
    let selection = selections();
    let clock = clock();
    let mut calls = 0;
    let mut random = || {
        calls += 1;
        Uuid::from_u128(RANDOM_VALUE)
    };
    let mut variables = Variables {
        document: &snapshot,
        selection: &selection,
        model_path: MODEL_PATH,
        language: taide_native_syntax::monaco_language("rust")
            .unwrap()
            .unwrap(),
        clipboard: Some("first\n \nsecond"),
        clipboard_spread: true,
        clock: &clock,
        random: &mut random,
        max_bytes: BYTE_LIMIT,
    };
    for (name, cursor, expected) in [
        ("TM_SELECTED_TEXT", 0, "alpha"),
        ("SELECTION", 1, "omega"),
        ("TM_CURRENT_LINE", 0, "alpha βeta"),
        ("TM_CURRENT_WORD", 0, "alpha"),
        ("TM_LINE_INDEX", 1, "1"),
        ("TM_LINE_NUMBER", 1, "2"),
        ("CURSOR_INDEX", 1, "1"),
        ("CURSOR_NUMBER", 1, "2"),
        ("LINE_COMMENT", 0, "//"),
        ("BLOCK_COMMENT_START", 0, "/*"),
        ("BLOCK_COMMENT_END", 0, "*/"),
        ("CLIPBOARD", 0, "first"),
        ("CLIPBOARD", 1, "second"),
    ] {
        assert_eq!(
            variables
                .resolve(
                    cursor,
                    VariableContext {
                        name,
                        preceding_text_line: None
                    },
                    None
                )
                .unwrap(),
            Some(expected.into()),
            "{name}"
        );
    }
    let reference: Reference = serde_json::from_str(REFERENCE).unwrap();
    for value in reference.workspace {
        assert_eq!(
            variables
                .resolve(
                    0,
                    VariableContext {
                        name: &value.name,
                        preceding_text_line: None
                    },
                    None
                )
                .unwrap(),
            value.expected
        );
    }
    let decimal = variables
        .resolve(
            0,
            VariableContext {
                name: "RANDOM",
                preceding_text_line: None,
            },
            None,
        )
        .unwrap()
        .unwrap();
    assert_eq!(decimal.len(), RANDOM_WIDTH);
    assert!(decimal.bytes().all(|byte| byte.is_ascii_digit()));
    let hex = variables
        .resolve(
            0,
            VariableContext {
                name: "RANDOM_HEX",
                preceding_text_line: None,
            },
            None,
        )
        .unwrap()
        .unwrap();
    assert_eq!(hex.len(), RANDOM_WIDTH);
    assert!(hex.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let uuid = variables
        .resolve(
            0,
            VariableContext {
                name: "UUID",
                preceding_text_line: None,
            },
            None,
        )
        .unwrap()
        .unwrap();
    assert_eq!(
        Uuid::parse_str(&uuid).unwrap(),
        Uuid::from_u128(RANDOM_VALUE)
    );
    assert_eq!(calls, 3);
}

#[test]
fn completion_variables는_실제_데이터를_읽지_않고_크기와_커서_경계를_검증한다() {
    let snapshot = snapshot();
    let selection = selections();
    let clock = clock();
    let mut random = || Uuid::from_u128(RANDOM_VALUE);
    let mut variables = Variables {
        document: &snapshot,
        selection: &selection,
        model_path: MODEL_PATH,
        language: taide_native_syntax::monaco_language("rust")
            .unwrap()
            .unwrap(),
        clipboard: None,
        clipboard_spread: false,
        clock: &clock,
        random: &mut random,
        max_bytes: BYTE_LIMIT,
    };
    assert_eq!(
        variables
            .resolve(
                0,
                VariableContext {
                    name: "CLIPBOARD",
                    preceding_text_line: None
                },
                None
            )
            .unwrap(),
        None
    );
    assert_eq!(
        variables
            .resolve(
                OWNER_LIMIT,
                VariableContext {
                    name: "TM_FILENAME",
                    preceding_text_line: None
                },
                None
            )
            .unwrap_err(),
        EditorError::InvalidBoundary
    );
    variables.max_bytes = 4;
    assert_eq!(
        variables
            .resolve(
                0,
                VariableContext {
                    name: "TM_FILEPATH",
                    preceding_text_line: None
                },
                None
            )
            .unwrap_err(),
        EditorError::Capacity
    );
    variables.max_bytes = BYTE_LIMIT;
    variables.clipboard = Some("first\nsecond");
    assert_eq!(
        variables
            .resolve(
                1,
                VariableContext {
                    name: "CLIPBOARD",
                    preceding_text_line: None
                },
                None
            )
            .unwrap(),
        Some("first\nsecond".into())
    );
}
