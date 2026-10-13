use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::document::{Edit, EditorError};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::json_format::{Limits, Options, format};
use taide_native_editor::json_scanner;
use taide_native_editor::lsp::{LspRange, TextEdit, byte_to_position};
use taide_native_editor::store::{EditorLimits, EditorStore};
use taide_native_editor::view::{Selection, SelectionSet, ViewKey};

const LIMITS: Limits = Limits {
    scanner: json_scanner::Limits {
        max_bytes: 64 * 1024,
        max_tokens: 128 * 1024,
        max_value_units: 64 * 1024,
    },
    max_edits: 32 * 1024,
    max_output_bytes: 2 * 1024 * 1024,
    max_indent_width: 64,
};
const DEFAULT_OPTIONS: Options = Options {
    indentation: IndentOptions {
        tab_size: 4,
        insert_spaces: true,
    },
    eol: "\n",
    keep_lines: false,
    insert_final_newline: false,
};
const SAMPLE_COUNT: usize = 8;
const DEEP_NESTING: usize = 1024;
const VIEW_LIMIT: usize = 2;
const HISTORY_LIMIT: usize = 8;
const NON_BMP_NAME: &str = "𐐀";
const MIRROR_WORD: &str = "value";

include!("fixtures/json-format-oracle.rs");

fn describe(input: &str, edits: &[Edit]) -> String {
    edits
        .iter()
        .map(|edit| {
            let start = input[..edit.bytes.start].encode_utf16().count();
            let end = input[..edit.bytes.end].encode_utf16().count();
            let text = edit
                .text
                .encode_utf16()
                .map(|unit| format!("{unit:x}"))
                .collect::<Vec<_>>()
                .join(",");
            format!("{start}..{end}:{text};")
        })
        .collect()
}

#[test]
fn json_format은_실제원본의_문서_범위_옵션과_편집순서에_일치한다() {
    let mut mismatches = Vec::new();
    for (input, range, width, spaces, eol, keep, final_line, raw, expected, _) in CASES {
        let options = Options {
            indentation: IndentOptions {
                tab_size: *width,
                insert_spaces: *spaces,
            },
            eol,
            keep_lines: *keep,
            insert_final_newline: *final_line,
        };
        let actual =
            format(input, range.clone(), options, LIMITS).map(|edits| describe(input, &edits));
        if actual.as_deref() != Ok(*expected) {
            mismatches.push(format!("{input:?}/{range:?}/{width}/{spaces}/{eol:?}/{keep}/{final_line}: actual={actual:?}, expected={expected:?}, source={raw:?}"));
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
fn json_format은_원문byte경계와_실제적용된_텍스트를_보존한다() {
    for (input, range, width, spaces, eol, keep, final_line, _, _, expected) in CASES {
        let options = Options {
            indentation: IndentOptions {
                tab_size: *width,
                insert_spaces: *spaces,
            },
            eol,
            keep_lines: *keep,
            insert_final_newline: *final_line,
        };
        let edits = format(input, range.clone(), options, LIMITS).unwrap();
        let mut previous_end = 0;
        for edit in &edits {
            assert!(
                edit.bytes.start >= previous_end
                    && edit.bytes.start <= edit.bytes.end
                    && edit.bytes.end <= input.len()
            );
            assert!(
                input.is_char_boundary(edit.bytes.start) && input.is_char_boundary(edit.bytes.end)
            );
            previous_end = edit.bytes.end;
        }
        let mut text = input.to_string();
        for edit in edits.into_iter().rev() {
            text.replace_range(edit.bytes, &edit.text);
        }
        assert_eq!(
            text, *expected,
            "{input:?}/{range:?}/{width}/{spaces}/{eol:?}/{keep}/{final_line}"
        );
    }
}

#[test]
fn json_format은_전체출력_편집_폭_토큰예산과_문자중간범위를_거절한다() {
    let input = format!("{}0{}", "[".repeat(DEEP_NESTING), "]".repeat(DEEP_NESTING));
    assert!(matches!(
        format(&input, None, DEFAULT_OPTIONS, LIMITS),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        format(
            "{\"a\":1}",
            None,
            DEFAULT_OPTIONS,
            Limits {
                max_output_bytes: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        format(
            "{\"a\":1}",
            None,
            DEFAULT_OPTIONS,
            Limits {
                max_edits: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        format(
            "{}",
            None,
            Options {
                indentation: IndentOptions {
                    tab_size: 65,
                    insert_spaces: true
                },
                ..DEFAULT_OPTIONS
            },
            LIMITS
        ),
        Err(EditorError::Capacity)
    ));
    assert!(matches!(
        format("{\"𐐀\":1}", Some(3..4), DEFAULT_OPTIONS, LIMITS),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        format("{}", Some(0..3), DEFAULT_OPTIONS, LIMITS),
        Err(EditorError::InvalidBoundary)
    ));
    assert!(matches!(
        format(
            "{}",
            None,
            DEFAULT_OPTIONS,
            Limits {
                scanner: json_scanner::Limits {
                    max_tokens: 1,
                    ..LIMITS.scanner
                },
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    ));
    assert!(
        format(
            "{}",
            None,
            DEFAULT_OPTIONS,
            Limits {
                max_edits: 0,
                max_output_bytes: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .is_empty()
    );
    assert!(
        format(
            "  ",
            None,
            DEFAULT_OPTIONS,
            Limits {
                max_edits: 0,
                max_output_bytes: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .is_empty()
    );
    assert!(
        format(
            "",
            None,
            DEFAULT_OPTIONS,
            Limits {
                max_edits: 0,
                max_output_bytes: 0,
                ..LIMITS
            }
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn json_format의_실제편집은_mirror_선택_단일undo와_readonly_revision을_보존한다() {
    let input = "{\"𐐀\":\"value\",\"b\":[1,2]}";
    for is_read_only in [false, true] {
        let mut store = EditorStore::new(EditorLimits {
            max_documents: 1,
            max_views: VIEW_LIMIT,
            max_undo_groups: HISTORY_LIMIT,
            max_document_bytes: LIMITS.scanner.max_bytes,
        })
        .unwrap();
        let path = "/synthetic/builtin-format.json";
        let document = store
            .open_file(
                path.into(),
                OpenedFile {
                    path: path.into(),
                    content: input.into(),
                    language_id: "json".into(),
                    byte_size: u32::try_from(input.len()).unwrap(),
                    line_count: 1,
                    tier: FileSizeTier::Normal,
                    read_only: is_read_only,
                    encoding_lossy: false,
                    modified_ms: 1.0,
                    editor_config: EditorConfigOptions::default(),
                },
            )
            .unwrap();
        let views = [NON_BMP_NAME, MIRROR_WORD].map(|word| {
            let view = store
                .attach_view(
                    ViewKey {
                        window: "json-format".into(),
                        pane: PaneId::new(),
                        tab: TabId::new(),
                    },
                    document,
                )
                .unwrap();
            let start = input.find(word).unwrap();
            store
                .set_view_state(
                    view,
                    SelectionSet {
                        primary: 0,
                        selections: vec![Selection {
                            anchor: start + word.len(),
                            head: start,
                        }],
                    },
                    Default::default(),
                    Vec::new(),
                )
                .unwrap();
            view
        });
        let before = store.documents().snapshot(document).unwrap();
        let edits = format(input, None, DEFAULT_OPTIONS, LIMITS)
            .unwrap()
            .into_iter()
            .map(|edit| TextEdit {
                range: LspRange {
                    start: byte_to_position(&before, edit.bytes.start).unwrap(),
                    end: byte_to_position(&before, edit.bytes.end).unwrap(),
                },
                new_text: edit.text,
            })
            .collect::<Vec<_>>();
        let result = taide_native_editor::formatting::apply_edits(
            &mut store,
            &before,
            Some(views[0]),
            edits.clone(),
        );
        if is_read_only {
            assert_eq!(result, Err(EditorError::ReadOnly));
            let after = store.documents().snapshot(document).unwrap();
            assert_eq!(after.rope.to_string(), input);
            assert_eq!(after.revision, before.revision);
            assert!(!after.dirty);
            continue;
        }
        assert_eq!(result, Ok(true));
        let after = store.documents().snapshot(document).unwrap();
        assert_eq!(
            after.rope.to_string(),
            "{\n    \"𐐀\": \"value\",\n    \"b\": [\n        1,\n        2\n    ]\n}"
        );
        assert!(after.dirty && after.revision > before.revision);
        for (view, word) in views.into_iter().zip([NON_BMP_NAME, MIRROR_WORD]) {
            let selection = &store.views().get(view).unwrap().selection.selections[0];
            assert!(selection.anchor > selection.head);
            assert_eq!(
                after
                    .rope
                    .byte_slice(selection.head..selection.anchor)
                    .to_string(),
                word
            );
        }
        assert_eq!(
            taide_native_editor::formatting::apply_edits(
                &mut store,
                &before,
                Some(views[0]),
                edits
            ),
            Err(EditorError::StaleRevision)
        );
        assert!(store.undo(document).unwrap());
        let restored = store.documents().snapshot(document).unwrap();
        assert_eq!(restored.rope.to_string(), input);
        assert!(restored.dirty);
        assert!(!store.undo(document).unwrap());
    }
}
