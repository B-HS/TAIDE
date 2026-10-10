use lsp_types::{CompletionItem, InsertTextFormat};
use taide_model::ids::TabId;
use taide_native_editor::completion::Candidate;
use taide_native_editor::completion_preview::{GhostText, Mode, Options, Part, compute};
use taide_native_editor::completion_preview_view::ViewData;
use taide_native_editor::display_map::{DisplayMap, WrapSettings, WrappingIndent};
use taide_native_editor::document::{DocumentSnapshot, EditorError, LineEnding};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::lsp::{LspRange, Position, byte_to_position};
use taide_native_editor::snippet_syntax::{ParseLimits, RegexMetadata};
use taide_native_editor::store::{EditorLimits, EditorStore};

const MAX_DOCUMENT_BYTES: usize = 100_000;
const HEX_RADIX: u32 = 16;
const HEX_PAIR_SIZE: usize = 2;
const FIELD_COUNT: usize = 10;
const EXPECTED_CASE_COUNT: usize = 9711;
const EXPECTED_UTF16_CORRECTIONS: usize = 9;
const SNIPPET_FIELD_COUNT: usize = 8;
const EXPECTED_SNIPPET_CASE_COUNT: usize = 1344;
const SNIPPET_BYTES: usize = 4096;
const SNIPPET_NESTING: usize = 32;
const SNIPPET_MARKERS: usize = 128;
const TAB_SIZE: u32 = 4;
const VIEW_FIELD_COUNT: usize = 6;
const VIEW_CASE_COUNT: usize = 12;
const WRAP_COLUMN: u32 = 5;
const FULL_WIDTH_COLUMNS: f64 = 2.0;

fn parts(snapshot: &DocumentSnapshot, value: &str) -> Vec<Part> {
    if value == "_" {
        return Vec::new();
    }
    value
        .split(',')
        .map(|value| {
            let mut fields = value.split(':');
            Part {
                byte: byte(snapshot, fields.next().unwrap().parse().unwrap()),
                text: text(fields.next().unwrap()),
                is_preview: fields.next().unwrap() == "1",
            }
        })
        .collect()
}

fn limits() -> ParseLimits {
    ParseLimits {
        max_bytes: SNIPPET_BYTES,
        max_nesting: SNIPPET_NESTING,
        max_markers: SNIPPET_MARKERS,
    }
}

fn candidate(snapshot: &DocumentSnapshot, cursor: usize, body: &str, snippet: bool) -> Candidate {
    let position = byte_to_position(snapshot, cursor).unwrap();
    Candidate::new(
        snapshot,
        position,
        LspRange {
            start: Position {
                line: position.line,
                character: 0,
            },
            end: position,
        },
        CompletionItem {
            label: "preview".into(),
            insert_text: Some(body.into()),
            insert_text_format: Some(if snippet {
                InsertTextFormat::SNIPPET
            } else {
                InsertTextFormat::PLAIN_TEXT
            }),
            ..Default::default()
        },
    )
    .unwrap()
}

fn text(hex: &str) -> String {
    if hex == "_" {
        return String::new();
    }
    assert_eq!(hex.len() % HEX_PAIR_SIZE, 0);
    let bytes = hex
        .as_bytes()
        .chunks_exact(HEX_PAIR_SIZE)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), HEX_RADIX).unwrap())
        .collect();
    String::from_utf8(bytes).unwrap()
}

fn document(source: &str) -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: MAX_DOCUMENT_BYTES,
    })
    .unwrap();
    let document = store
        .open_untitled(TabId::new(), source, "plaintext".into())
        .unwrap();
    store.documents().snapshot(document).unwrap()
}

fn byte(document: &DocumentSnapshot, units: usize) -> usize {
    let character = document.rope.try_utf16_cu_to_char(units).unwrap();
    assert_eq!(document.rope.char_to_utf16_cu(character), units);
    document.rope.char_to_byte(character)
}

#[test]
fn completion_preview의_삽입위치_괄호_unicode_들여쓰기_다중줄_상한은_실제_monaco와_승인한_utf16수정과_함께_일치한다()
 {
    let mut count = 0;
    let mut corrections = 0;
    for (number, row) in include_str!("fixtures/completion-preview-reference.tsv")
        .lines()
        .enumerate()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), FIELD_COUNT);
        match fields[9] {
            "original" => {}
            "utf16-prefix-correction" => corrections += 1,
            _ => panic!("unexpected reference policy"),
        }
        let snapshot = document(&text(fields[0]));
        let range = byte(&snapshot, fields[1].parse().unwrap())
            ..byte(&snapshot, fields[2].parse().unwrap());
        let cursor = byte(&snapshot, fields[3].parse().unwrap());
        let replacement = text(fields[4]);
        let options = Options {
            mode: match fields[5] {
                "prefix" => Mode::Prefix,
                "subword" => Mode::Subword,
                "subwordSmart" => Mode::SubwordSmart,
                _ => panic!("unexpected mode"),
            },
            preview_suffix_utf16: fields[6].parse().unwrap(),
        };
        let expected = match fields[8] {
            "-" | "invalid-utf16" => None,
            parts => Some(GhostText {
                line: fields[7].parse::<usize>().unwrap() - 1,
                parts: if parts == "_" {
                    Vec::new()
                } else {
                    parts
                        .split(',')
                        .map(|part| {
                            let values = part.split(':').collect::<Vec<_>>();
                            Part {
                                byte: byte(&snapshot, values[0].parse().unwrap()),
                                text: text(values[1]),
                                is_preview: values[2] == "1",
                            }
                        })
                        .collect()
                },
            }),
        };
        let actual = compute(&snapshot, range, &replacement, cursor, options).unwrap();
        assert_eq!(
            actual,
            expected,
            "row {} mode {} source {:?} text {:?}",
            number + 1,
            fields[5],
            snapshot.rope.to_string(),
            replacement
        );
        count += 1;
    }
    assert_eq!(count, EXPECTED_CASE_COUNT);
    assert_eq!(corrections, EXPECTED_UTF16_CORRECTIONS);
}

#[test]
fn completion_preview는_잘못된_utf8범위_커서와_eol중간경계를_거절한다() {
    let snapshot = document("한\u{1f600}\r\nline");
    let options = Options {
        mode: Mode::SubwordSmart,
        preview_suffix_utf16: 0,
    };
    for (range, cursor) in [
        (1..3, 3),
        (0..4, 3),
        (0..3, 1),
        (0..8, 3),
        (8..13, 13),
        (0..14, 3),
    ] {
        assert_eq!(
            compute(&snapshot, range, "replacement", cursor, options),
            Err(EditorError::InvalidBoundary)
        );
    }
}

#[test]
fn completion_preview의_snippet_기본값_mirror_choice_변수_변환_들여쓰기_eol은_실제_monaco와_일치한다()
 {
    let mut count = 0;
    for (number, row) in include_str!("fixtures/completion-preview-snippets-reference.tsv")
        .lines()
        .enumerate()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), SNIPPET_FIELD_COUNT);
        let mut snapshot = document(&text(fields[0]));
        snapshot.metadata.line_ending = match text(fields[6]).as_str() {
            "\n" => LineEnding::Lf,
            "\r\n" => LineEnding::CrLf,
            _ => panic!("unexpected eol"),
        };
        let cursor = byte(&snapshot, fields[1].parse().unwrap());
        let body = text(fields[2]);
        let candidate = candidate(&snapshot, cursor, &body, fields[3] == "1");
        let actual = candidate
            .preview_text(
                &snapshot,
                cursor,
                IndentOptions {
                    tab_size: fields[4].parse().unwrap(),
                    insert_spaces: fields[5] == "1",
                },
                limits(),
                |pattern, options| {
                    Some(RegexMetadata {
                        source: pattern.into(),
                        ignore_case: options.contains('i'),
                        global: options.contains('g'),
                    })
                },
            )
            .unwrap();
        assert_eq!(
            actual,
            text(fields[7]),
            "row {} source {:?} snippet {:?}",
            number + 1,
            snapshot.rope.to_string(),
            body
        );
        count += 1;
    }
    assert_eq!(count, EXPECTED_SNIPPET_CASE_COUNT);
}

#[test]
fn completion_preview는_다른문서_지난revision_readonly_utf8경계_용량을_평가전에_거절한다() {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 2,
        max_views: 1,
        max_undo_groups: 1,
        max_document_bytes: MAX_DOCUMENT_BYTES,
    })
    .unwrap();
    let first = store
        .open_untitled(TabId::new(), "한", "plaintext".into())
        .unwrap();
    let second = store
        .open_untitled(TabId::new(), "한", "plaintext".into())
        .unwrap();
    let snapshot = store.documents().snapshot(first).unwrap();
    let candidate = candidate(&snapshot, snapshot.rope.len_bytes(), "${1:value}", true);
    let indent = IndentOptions {
        tab_size: TAB_SIZE,
        insert_spaces: true,
    };
    let evaluate = |document: &DocumentSnapshot, cursor, limits| {
        candidate.preview_text(document, cursor, indent, limits, |_, _| {
            panic!("compile must not run")
        })
    };
    assert_eq!(
        evaluate(&store.documents().snapshot(second).unwrap(), 3, limits()),
        Err(EditorError::InvalidIdentity)
    );
    let mut stale = snapshot.clone();
    stale.revision += 1;
    assert_eq!(
        evaluate(&stale, 3, limits()),
        Err(EditorError::StaleRevision)
    );
    let mut readonly = snapshot.clone();
    readonly.metadata.read_only = true;
    assert_eq!(evaluate(&readonly, 3, limits()), Err(EditorError::ReadOnly));
    assert_eq!(
        evaluate(&snapshot, 1, limits()),
        Err(EditorError::InvalidBoundary)
    );
    assert_eq!(
        evaluate(
            &snapshot,
            3,
            ParseLimits {
                max_bytes: 0,
                ..limits()
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        store.documents().snapshot(first).unwrap().revision,
        snapshot.revision
    );
    assert_eq!(
        store.documents().snapshot(first).unwrap().rope.to_string(),
        "한"
    );
}

#[test]
fn completion_preview의_추가줄_숨긴접미사_복수삽입과_줄끝은_실제_monaco_표시모델과_일치한다() {
    let mut count = 0;
    for (number, row) in include_str!("fixtures/completion-preview-view-reference.tsv")
        .lines()
        .enumerate()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), VIEW_FIELD_COUNT);
        let snapshot = document(&text(fields[0]));
        let ghost = GhostText {
            line: fields[1].parse::<usize>().unwrap() - 1,
            parts: parts(&snapshot, fields[2]),
        };
        let view = ViewData::new(&snapshot, &ghost).unwrap();
        assert_eq!(
            view.inline,
            parts(&snapshot, fields[3]),
            "inline row {}",
            number + 1
        );
        let hidden = if fields[4] == "_" {
            None
        } else {
            let (start, end) = fields[4].split_once(':').unwrap();
            Some(byte(&snapshot, start.parse().unwrap())..byte(&snapshot, end.parse().unwrap()))
        };
        assert_eq!(view.hidden_source, hidden, "hidden row {}", number + 1);
        let additional = if fields[5] == "_" {
            Vec::new()
        } else {
            fields[5].split(';').collect()
        };
        assert_eq!(
            view.additional.len(),
            additional.len(),
            "additional row {}",
            number + 1
        );
        for (line, expected) in view.additional.iter().zip(additional) {
            let (value, decorations) = expected.split_once(':').unwrap();
            assert_eq!(line.text, text(value), "additional text row {}", number + 1);
            let expected_ranges = if decorations == "_" {
                Vec::new()
            } else {
                let reference = document(&line.text);
                decorations
                    .split(',')
                    .filter_map(|range| {
                        let (start, end) = range.split_once(':').unwrap();
                        let range = byte(&reference, start.parse().unwrap())
                            ..byte(&reference, end.parse().unwrap());
                        (!range.is_empty()).then_some(range)
                    })
                    .collect()
            };
            assert_eq!(
                line.runs
                    .iter()
                    .filter(|run| run.source.is_none())
                    .map(|run| run.bytes.clone())
                    .collect::<Vec<_>>(),
                expected_ranges,
                "ghost ranges row {}",
                number + 1
            );
            for run in &line.runs {
                if let Some(source) = &run.source {
                    assert_eq!(
                        &line.text[run.bytes.clone()],
                        snapshot.rope.byte_slice(source.clone()).to_string(),
                        "source ranges row {}",
                        number + 1
                    );
                }
            }
        }
        assert_eq!(snapshot.revision, 0);
        assert_eq!(snapshot.rope.to_string(), text(fields[0]));
        count += 1;
    }
    assert_eq!(count, VIEW_CASE_COUNT);
}

#[test]
fn completion_preview_표시는_다른줄과_잘못된utf8_역순삽입을_거절한다() {
    let snapshot = document("한abc\nnext");
    for (line, positions) in [
        (snapshot.rope.len_lines(), vec![0]),
        (0, vec![1]),
        (0, vec![snapshot.rope.len_bytes()]),
        (0, vec![4, 3]),
    ] {
        let ghost = GhostText {
            line,
            parts: positions
                .into_iter()
                .map(|byte| Part {
                    byte,
                    text: "preview".into(),
                    is_preview: false,
                })
                .collect(),
        };
        assert_eq!(
            ViewData::new(&snapshot, &ghost),
            Err(EditorError::InvalidBoundary)
        );
    }
}

#[test]
fn completion_preview_투영은_기존문서를_바꾸지않고_줄바꿈_접기_다음줄과_커서좌표를_보존한다() {
    let snapshot = document("aa bb cc\ntail\nlast");
    let mut display = DisplayMap::build(
        &snapshot,
        Some(WrapSettings {
            wrap_column: WRAP_COLUMN,
            tab_size: TAB_SIZE,
            full_width_columns: FULL_WIDTH_COLUMNS,
            wrapping_indent: WrappingIndent::None,
        }),
    );
    let original = display.clone();
    let views = [(1, 9, "ZZ"), (0, 2, "XX\nY")]
        .into_iter()
        .map(|(line, byte, value)| {
            ViewData::new(
                &snapshot,
                &GhostText {
                    line,
                    parts: vec![Part {
                        byte,
                        text: value.into(),
                        is_preview: false,
                    }],
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(display.set_preview_views(&snapshot, &views).unwrap());
    assert!(!display.set_preview_views(&snapshot, &views).unwrap());
    assert_eq!(display.row_count(), 4);
    assert_eq!(display.rows_of_line(0), 0..1);
    assert_eq!(display.rows_of_line(1), 1..3);
    assert_eq!(display.rows_of_line(2), 3..4);
    assert_eq!(display.row_of_byte(&snapshot, 2), 0);
    assert_eq!(display.row_of_byte(&snapshot, 9), 1);
    assert_eq!(display.row_of_byte(&snapshot, 12), 2);
    assert_eq!(display.row_of_byte(&snapshot, 14), 3);
    assert_eq!(display.row_projection(0).unwrap().text, "aaXX");
    assert_eq!(display.row_projection(1).unwrap().text, "ZZtai");
    assert_eq!(display.row_projection(2).unwrap().text, "l");
    assert_eq!(display.segment(&snapshot, 3).line, 2);
    for index in 0..display.row_count() {
        if let Some(row) = display.row_projection(index) {
            for unit in &row.units {
                assert!(row.text.is_char_boundary(unit.text_byte));
                assert!(snapshot.rope.byte_to_char(unit.source_byte) <= snapshot.rope.len_chars());
                assert_eq!(
                    snapshot
                        .rope
                        .char_to_byte(snapshot.rope.byte_to_char(unit.source_byte)),
                    unit.source_byte
                );
            }
        }
    }
    let before_error = display.clone();
    let mut bad = views[0].clone();
    bad.inline[0].byte = snapshot.rope.len_bytes();
    assert_eq!(
        display.set_preview_views(&snapshot, &[bad]),
        Err(EditorError::InvalidBoundary)
    );
    assert_eq!(display, before_error);
    assert!(display.set_hidden_lines(&[1..2]));
    let folded = display.clone();
    display.set_preview_views(&snapshot, &views).unwrap();
    assert_eq!(display.row_count(), 2);
    assert!(display.segment(&snapshot, 0).ends_folded);
    assert_eq!(display.row_of_byte(&snapshot, 9), 0);
    assert_eq!(display.segment(&snapshot, 1).line, 2);
    display.set_preview_views(&snapshot, &[]).unwrap();
    assert_eq!(display, folded);
    display.set_hidden_lines(&[]);
    assert_eq!(display, original);
    assert_eq!(snapshot.rope.to_string(), "aa bb cc\ntail\nlast");
    assert_eq!(snapshot.revision, 0);
}

#[test]
fn completion_preview_투영은_빈표시와_줄끝삽입의_문서범위를_보존하며_revision변경때_회수된다() {
    let snapshot = document("abcdef\nnext");
    let mut display = DisplayMap::build(&snapshot, None);
    let view = ViewData::new(
        &snapshot,
        &GhostText {
            line: 0,
            parts: vec![Part {
                byte: 0,
                text: "\n".into(),
                is_preview: false,
            }],
        },
    )
    .unwrap();
    display.set_preview_views(&snapshot, &[view]).unwrap();
    assert_eq!(display.row_projection(0).unwrap().text, "");
    assert_eq!(display.segment(&snapshot, 0).bytes, 0..6);
    assert_eq!(display.row_of_byte(&snapshot, 0), 0);
    let mut newer = snapshot.clone();
    newer.revision += 1;
    newer.rope.insert(0, "X");
    display.refresh(&newer);
    assert!(display.preview_views().is_empty());
    assert!(display.row_projection(0).is_none());
    assert_eq!(display.segment(&newer, 0).bytes, 0..7);
}
