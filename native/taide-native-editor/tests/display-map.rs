use std::time::Instant;

use taide_model::file::{EditorConfigOptions, FileSizeTier, OpenedFile};
use taide_native_editor::display_map::{DisplayMap, RowSegment, WrapSettings, WrappingIndent};
use taide_native_editor::document::{DocumentId, DocumentSnapshot, Edit, UndoGroup};
use taide_native_editor::editing::line_content_range;
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};

const DOCUMENT_LIMIT: usize = 1;
const VIEW_LIMIT: usize = 1;
const HISTORY_LIMIT: usize = 8;
const BYTE_LIMIT: usize = 1024;
const LARGE_BYTE_LIMIT: usize = 64 * 1024 * 1024;
const CONTENT: &str = "ab\r\n한글\n\nend";
const ROW_BYTES: [std::ops::Range<usize>; 4] = [0..2, 4..10, 11..11, 12..15];
const TAB_SIZE: u32 = 4;
const FULL_WIDTH_COLUMNS: f64 = 2.0;
const WRAP_COLUMN: u32 = 10;
const WRAP_CONTENT: &str = "    aaaa aaaa\r\nab\n\naaaaaaaaaaaa";
const WRAP_ROW_COUNT: usize = 6;
const WRAP_COLUMNS: [u32; 7] = [1, 2, 3, 5, 8, 20, 80];
const MIXED_CONTENT: &str = concat!(
    "fn main() {\n",
    "\tlet 값 = \"漢字漢字漢字漢字漢字漢字\"; // 주석 주석 주석\r\n",
    "\n",
    "    // e\u{301} \u{10400} aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
    "\t\t\tdeeply.indented(call, with, many, arguments, that, wrap, around)\r\n",
    "한글한글한글한글한글한글한글한글한글한글\r",
    "}"
);
const EDIT_ROUNDS: usize = 400;
const EDIT_KINDS: usize = 8;
const UNDO_KIND: usize = 0;
const REDO_KIND: usize = 1;
const LONGEST_DELETION: usize = 12;
const INSERTIONS: [&str; 12] = [
    "",
    "a",
    " ",
    "\n",
    "\r\n",
    "\r",
    "\t",
    "한",
    "\u{D55D}",
    "漢字漢字",
    "long words that wrap around",
    "\u{10400}",
];
const RANDOM_SEED: u64 = 0x5DEECE66D;
const RANDOM_MULTIPLIER: u64 = 6_364_136_223_846_793_005;
const RANDOM_INCREMENT: u64 = 1_442_695_040_888_963_407;
const RANDOM_SHIFT: u32 = 33;
const LARGE_LINES: usize = 50_000;
const HUGE_LINES: usize = 300_000;
const LARGE_WRAP_COLUMN: u32 = 100;
const LARGE_LONG_LINE_INTERVAL: usize = 10;
const LARGE_LONG_LINE_ARGUMENTS: usize = 20;

struct Random(u64);

impl Random {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(RANDOM_MULTIPLIER)
            .wrapping_add(RANDOM_INCREMENT);
        (self.0 >> RANDOM_SHIFT) as usize % bound
    }
}

fn wrap(wrap_column: u32) -> WrapSettings {
    WrapSettings {
        wrap_column,
        tab_size: TAB_SIZE,
        full_width_columns: FULL_WIDTH_COLUMNS,
        wrapping_indent: WrappingIndent::Same,
    }
}

fn opened(text: &str, byte_limit: usize) -> (EditorStore, DocumentId) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: byte_limit,
    })
    .unwrap();
    let id = store
        .open_file(
            "/synthetic/wrapped.txt".into(),
            OpenedFile {
                path: "/synthetic/wrapped.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    (store, id)
}

fn replace(store: &mut EditorStore, id: DocumentId, bytes: std::ops::Range<usize>, text: &str) {
    let revision = store.documents().snapshot(id).unwrap().revision;
    store
        .apply(
            id,
            Transaction {
                revision,
                group: UndoGroup(revision),
                origin: None,
                selection_after: None,
                edits: vec![Edit {
                    bytes,
                    text: text.into(),
                }],
            },
        )
        .unwrap();
}

fn snapshot(text: &str) -> DocumentSnapshot {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: DOCUMENT_LIMIT,
        max_views: VIEW_LIMIT,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let id = store
        .open_file(
            "/synthetic/display.txt".into(),
            OpenedFile {
                path: "/synthetic/display.txt".into(),
                content: text.into(),
                language_id: "plaintext".into(),
                byte_size: text.len().try_into().unwrap(),
                line_count: text.lines().count().try_into().unwrap(),
                tier: FileSizeTier::Normal,
                read_only: false,
                encoding_lossy: false,
                modified_ms: 1.0,
                editor_config: EditorConfigOptions::default(),
            },
        )
        .unwrap();
    store.documents().snapshot(id).unwrap()
}

#[test]
fn identity_표시_줄은_문서_줄과_같고_줄_끝_문자를_뺀_바이트_범위를_가진다() {
    let document = snapshot(CONTENT);
    let map = DisplayMap::identity(document.rope.len_lines(), document.revision);
    assert_eq!(map.row_count(), ROW_BYTES.len());
    assert_eq!(map.revision(), document.revision);
    for (row, bytes) in ROW_BYTES.into_iter().enumerate() {
        assert_eq!(
            map.segment(&document, row),
            RowSegment {
                line: row,
                bytes,
                is_continuation: false,
                indent_columns: 0,
                ends_folded: false,
            }
        );
    }
    assert_eq!(
        map.segment(&document, ROW_BYTES.len()),
        map.segment(&document, ROW_BYTES.len() - 1)
    );
}

#[test]
fn identity_바이트의_표시_줄은_그_바이트가_놓인_문서_줄이다() {
    let document = snapshot(CONTENT);
    let map = DisplayMap::identity(document.rope.len_lines(), document.revision);
    for (byte, row) in [
        (0, 0),
        (2, 0),
        (3, 0),
        (4, 1),
        (10, 1),
        (11, 2),
        (12, 3),
        (CONTENT.len(), 3),
        (CONTENT.len() + 1, 3),
    ] {
        assert_eq!(map.row_of_byte(&document, byte), row);
    }
    let empty = snapshot("");
    let map = DisplayMap::identity(empty.rope.len_lines(), empty.revision);
    assert_eq!(map.row_count(), 1);
    assert_eq!(map.row_of_byte(&empty, 0), 0);
    assert_eq!(map.segment(&empty, 0).bytes, 0..0);
}

#[test]
fn wrap_표시_줄은_이어지는_줄의_들여쓰기와_시작_열을_가지고_wrap_경계_바이트는_뒷_줄에_놓인다() {
    let document = snapshot(WRAP_CONTENT);
    let map = DisplayMap::build(&document, Some(wrap(WRAP_COLUMN)));
    assert_eq!(map.wrap_settings(), Some(&wrap(WRAP_COLUMN)));
    assert_eq!(map.revision(), document.revision);
    assert_eq!(map.row_count(), WRAP_ROW_COUNT);
    for (row, (line, bytes, is_continuation, indent_columns, start_column)) in [
        (0, 0..9, false, 0, 0.0),
        (0, 9..13, true, 4, 9.0),
        (1, 15..17, false, 0, 0.0),
        (2, 18..18, false, 0, 0.0),
        (3, 19..29, false, 0, 0.0),
        (3, 29..31, true, 0, 10.0),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            map.segment(&document, row),
            RowSegment {
                line,
                bytes,
                is_continuation,
                indent_columns,
                ends_folded: false,
            }
        );
        assert_eq!(map.row_start_column(row), start_column);
    }
    assert_eq!(
        map.segment(&document, WRAP_ROW_COUNT),
        map.segment(&document, WRAP_ROW_COUNT - 1)
    );
    for (line, rows) in [(0, 0..2), (1, 2..3), (2, 3..4), (3, 4..6)] {
        assert_eq!(map.rows_of_line(line), rows);
    }
    for (byte, row) in [
        (0, 0),
        (8, 0),
        (9, 1),
        (13, 1),
        (14, 1),
        (15, 2),
        (17, 2),
        (18, 3),
        (19, 4),
        (28, 4),
        (29, 5),
        (WRAP_CONTENT.len(), 5),
        (WRAP_CONTENT.len() + 1, 5),
    ] {
        assert_eq!(map.row_of_byte(&document, byte), row, "{byte}");
        assert_eq!(map.row_of_head(&document, byte, false), row, "{byte}");
    }
    for (byte, row) in [(9, 0), (29, 4), (8, 0), (13, 1), (15, 2), (0, 0)] {
        assert_eq!(map.row_of_head(&document, byte, true), row, "{byte}");
    }
    let unwrapped = DisplayMap::build(&document, None);
    assert_eq!(
        unwrapped,
        DisplayMap::identity(document.rope.len_lines(), document.revision)
    );
    assert_eq!(unwrapped.wrap_settings(), None);
    assert_eq!(unwrapped.rows_of_line(1), 1..2);
    assert_eq!(unwrapped.row_start_column(1), 0.0);
}

#[test]
fn wrap_표시_줄은_문서_줄을_빈틈없이_덮고_모든_바이트는_자신이_놓인_표시_줄로_돌아온다() {
    let document = snapshot(MIXED_CONTENT);
    for wrap_column in WRAP_COLUMNS {
        let map = DisplayMap::build(&document, Some(wrap(wrap_column)));
        let mut next_row = 0;
        for line in 0..document.rope.len_lines() {
            let rows = map.rows_of_line(line);
            let content = line_content_range(&document, line);
            assert_eq!(rows.start, next_row);
            assert!(!rows.is_empty());
            next_row = rows.end;
            let mut covered = content.start;
            for row in rows.clone() {
                let segment = map.segment(&document, row);
                let slice = document.rope.byte_slice(segment.bytes.clone());
                let continues = row + 1 < rows.end;
                assert_eq!(segment.line, line);
                assert_eq!(segment.bytes.start, covered);
                assert_eq!(segment.is_continuation, row > rows.start);
                assert!(!segment.bytes.is_empty() || content.is_empty());
                covered = segment.bytes.end;
                for byte in (0..=slice.len_chars())
                    .map(|character| segment.bytes.start + slice.char_to_byte(character))
                {
                    let downstream = if byte == segment.bytes.end && continues {
                        row + 1
                    } else {
                        row
                    };
                    let upstream = if byte == segment.bytes.start && segment.is_continuation {
                        row - 1
                    } else {
                        row
                    };
                    let landed = map.segment(&document, downstream).bytes;
                    assert_eq!(map.row_of_byte(&document, byte), downstream);
                    assert_eq!(map.row_of_head(&document, byte, false), downstream);
                    assert_eq!(map.row_of_head(&document, byte, true), upstream);
                    assert!(landed.start <= byte && byte <= landed.end);
                }
                if continues {
                    let columns = map.row_start_column(row + 1) - map.row_start_column(row);
                    let available = f64::from(wrap_column) - f64::from(segment.indent_columns);
                    assert!(
                        columns <= available || slice.len_chars() == 1,
                        "wrap {wrap_column} row {row}"
                    );
                }
            }
            assert_eq!(covered, content.end);
        }
        assert_eq!(next_row, map.row_count());
    }
}

#[test]
fn wrap_갱신은_문자와_줄_끝_문자의_중간을_가르는_편집_뒤에도_전체_계산과_같다() {
    for (text, bytes, replacement) in [
        ("aaaa 한한한 aaaa\nbbbb", 8..11, "\u{D55D}"),
        ("aaaa 한한한 aaaa\nbbbb", 8..11, "\u{C55C}"),
        ("aaaa bbbb\r\ncccc dddd", 10..10, "x"),
        ("aaaa bbbb\r\ncccc dddd", 10..11, ""),
        ("aaaa bbbb\rcccc dddd", 10..10, "\n"),
        ("aaaa bbbb\ncccc dddd\neeee ffff", 0..29, ""),
        ("aaaa bbbb\ncccc dddd", 19..19, " eeee ffff"),
        ("", 0..0, "aaaa bbbb\ncccc dddd"),
    ] {
        let (mut store, id) = opened(text, BYTE_LIMIT);
        let settings = wrap(5);
        let mut map = DisplayMap::build(&store.documents().snapshot(id).unwrap(), Some(settings));
        replace(&mut store, id, bytes, replacement);
        let edited = store.documents().snapshot(id).unwrap();
        map.refresh(&edited);
        assert_eq!(map, DisplayMap::build(&edited, Some(settings)), "{text:?}");
        assert!(store.undo(id).unwrap());
        let restored = store.documents().snapshot(id).unwrap();
        map.refresh(&restored);
        assert_eq!(
            map,
            DisplayMap::build(&restored, Some(settings)),
            "{text:?}"
        );
    }
}

#[test]
fn wrap_갱신은_연속된_편집과_undo_redo_뒤에_전체를_다시_계산한_결과와_같다() {
    for wrap_column in [5, 20] {
        let (mut store, id) = opened(MIXED_CONTENT, LARGE_BYTE_LIMIT);
        let settings = wrap(wrap_column);
        let mut map = DisplayMap::build(&store.documents().snapshot(id).unwrap(), Some(settings));
        let mut identity = DisplayMap::build(&store.documents().snapshot(id).unwrap(), None);
        let mut random = Random(RANDOM_SEED);
        for round in 0..EDIT_ROUNDS {
            let document = store.documents().snapshot(id).unwrap();
            match random.below(EDIT_KINDS) {
                UNDO_KIND => {
                    store.undo(id).unwrap();
                }
                REDO_KIND => {
                    store.redo(id).unwrap();
                }
                _ => {
                    let boundary =
                        |byte: usize| document.rope.char_to_byte(document.rope.byte_to_char(byte));
                    let length = document.rope.len_bytes();
                    let start = boundary(random.below(length + 1));
                    let end = boundary((start + random.below(LONGEST_DELETION + 1)).min(length));
                    let insertion = INSERTIONS[random.below(INSERTIONS.len())];
                    replace(&mut store, id, start..end, insertion);
                }
            }
            let document = store.documents().snapshot(id).unwrap();
            map.refresh(&document);
            identity.refresh(&document);
            assert_eq!(
                map,
                DisplayMap::build(&document, Some(settings)),
                "wrap {wrap_column} round {round}"
            );
            assert_eq!(identity, DisplayMap::build(&document, None));
        }
    }
}

fn record_wrap_timing(lines: usize) {
    let text: String = (0..lines)
        .map(|line| {
            if line % LARGE_LONG_LINE_INTERVAL == 0 {
                format!(
                    "    let value_{line} = compute({});\n",
                    "argument, ".repeat(LARGE_LONG_LINE_ARGUMENTS)
                )
            } else {
                format!("\tstatement_{line}(alpha, beta); // 주석 {line}\n")
            }
        })
        .collect();
    let (mut store, id) = opened(&text, LARGE_BYTE_LIMIT);
    let document = store.documents().snapshot(id).unwrap();
    let settings = wrap(LARGE_WRAP_COLUMN);
    let started = Instant::now();
    let mut map = DisplayMap::build(&document, Some(settings));
    let built = started.elapsed();
    assert!(map.row_count() > document.rope.len_lines());
    let middle = document.rope.line_to_byte(lines / 2);
    replace(&mut store, id, middle..middle, "x");
    let edited = store.documents().snapshot(id).unwrap();
    let started = Instant::now();
    map.refresh(&edited);
    let refreshed = started.elapsed();
    let started = Instant::now();
    let rebuilt = DisplayMap::build(&edited, Some(settings));
    let rebuilding = started.elapsed();
    assert_eq!(map, rebuilt);
    println!(
        "wrap timing: lines {} bytes {} rows {} build {built:?} refresh {refreshed:?} rebuild {rebuilding:?}",
        edited.rope.len_lines(),
        edited.rope.len_bytes(),
        map.row_count(),
    );
}

#[test]
fn 큰_문서의_wrap_전체_계산과_편집_뒤_갱신에_걸린_시간을_기록한다() {
    record_wrap_timing(LARGE_LINES);
}

#[test]
#[ignore]
fn 수십만_줄_문서의_wrap_전체_계산과_편집_뒤_갱신에_걸린_시간을_기록한다() {
    record_wrap_timing(HUGE_LINES);
}
