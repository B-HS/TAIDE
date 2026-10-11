use taide_model::ids::TabId;
use taide_native_editor::completion_preview::Part;
use taide_native_editor::completion_preview_view::ViewData;
use taide_native_editor::display_map::{DisplayMap, WrapSettings, WrappingIndent};
use taide_native_editor::document::{DocumentSnapshot, Edit, EditorError, UndoGroup};
use taide_native_editor::inlay_hints::{DEFAULT_MAXIMUM_LENGTH, Hint, Kind};
use taide_native_editor::store::{EditorLimits, EditorStore, Transaction};

const BYTE_LIMIT: usize = 4096;
const HISTORY_LIMIT: usize = 8;
const WRAP_COLUMN: u32 = 6;
const TAB_SIZE: u32 = 4;
const FULL_WIDTH_COLUMNS: f64 = 2.0;
const TEXT: &str = "call(\u{1f600}value)\nnext";
const LONG_LABEL_CHARACTERS: usize = 22;

fn fixture(text: &str) -> (EditorStore, DocumentSnapshot) {
    let mut store = EditorStore::new(EditorLimits {
        max_documents: 1,
        max_views: 1,
        max_undo_groups: HISTORY_LIMIT,
        max_document_bytes: BYTE_LIMIT,
    })
    .unwrap();
    let id = store
        .open_untitled(TabId::new(), text, "rust".into())
        .unwrap();
    let document = store.documents().snapshot(id).unwrap();
    (store, document)
}

fn hint(byte: usize, label: &str) -> Hint {
    Hint {
        byte,
        label: label.into(),
        kind: Some(Kind::Type),
        tooltip: None,
        padding_left: false,
        padding_right: false,
    }
}

#[test]
fn 표시_힌트는_원문과_revision을_바꾸지_않고_종류_툴팁_여백과_바이트를_보존한다() {
    let (store, document) = fixture(TEXT);
    let byte = TEXT.find("value").unwrap();
    let mut annotation = hint(byte, "name:\t Type");
    annotation.kind = Some(Kind::Parameter);
    annotation.tooltip = Some("**parameter**".into());
    annotation.padding_left = true;
    annotation.padding_right = true;
    let hints = [annotation];
    let mut display = DisplayMap::build(&document, None);
    assert!(
        display
            .set_inlay_hints(&document, &hints, DEFAULT_MAXIMUM_LENGTH)
            .unwrap()
    );
    assert!(
        !display
            .set_inlay_hints(&document, &hints, DEFAULT_MAXIMUM_LENGTH)
            .unwrap()
    );
    assert_eq!(display.inlay_hints(), hints);
    let row = display.row_projection(0).unwrap();
    assert_eq!(row.units.iter().filter(|unit| unit.hint_padding).count(), 2);
    assert_eq!(
        row.text,
        "call(\u{1f600}\u{200a}name:\u{a0}\u{a0}Type\u{200a}value)"
    );
    assert!(
        row.units
            .iter()
            .filter(|unit| unit.hint.is_some())
            .all(|unit| {
                unit.hint == Some(0)
                    && unit.injected
                    && !unit.is_preview
                    && unit.source_byte == byte
            })
    );
    let unchanged = store.documents().snapshot(document.id).unwrap();
    assert_eq!(unchanged.revision, document.revision);
    assert_eq!(unchanged.rope.to_string(), TEXT);
}

#[test]
fn ghost와_힌트는_동시에_표시되고_한쪽_회수가_다른쪽을_없애지_않는다() {
    let (_, document) = fixture(TEXT);
    let byte = TEXT.find("value").unwrap();
    let hints = [hint(byte, ": Type")];
    let views = [ViewData {
        line: 0,
        inline: vec![Part {
            byte,
            text: "complete".into(),
            is_preview: true,
        }],
        additional: Vec::new(),
        hidden_source: None,
    }];
    for hints_first in [false, true] {
        let mut display = DisplayMap::build(&document, None);
        if hints_first {
            display
                .set_inlay_hints(&document, &hints, DEFAULT_MAXIMUM_LENGTH)
                .unwrap();
        }
        display.set_preview_views(&document, &views).unwrap();
        display
            .set_inlay_hints(&document, &hints, DEFAULT_MAXIMUM_LENGTH)
            .unwrap();
        let row = display.row_projection(0).unwrap();
        assert!(row.text.contains("complete"));
        assert!(row.text.contains(":\u{a0}Type"));
        assert!(
            row.units
                .iter()
                .any(|unit| unit.is_preview && unit.hint.is_none())
        );
        assert!(
            row.units
                .iter()
                .any(|unit| unit.hint == Some(0) && !unit.is_preview)
        );
        display.set_preview_views(&document, &[]).unwrap();
        assert_eq!(display.inlay_hints(), hints);
        assert!(!display.row_projection(0).unwrap().text.contains("complete"));
        display.set_preview_views(&document, &views).unwrap();
        display
            .set_inlay_hints(&document, &[], DEFAULT_MAXIMUM_LENGTH)
            .unwrap();
        assert_eq!(display.preview_views(), views);
        assert!(display.row_projection(0).unwrap().text.contains("complete"));
        assert!(
            !display
                .row_projection(0)
                .unwrap()
                .units
                .iter()
                .any(|unit| unit.hint.is_some())
        );
    }
}

#[test]
fn 줄별_길이_상한은_utf16을_세고_이모지를_쪼개지_않으며_0은_해제한다() {
    let (_, document) = fixture("ab\ncd");
    let long = "\u{1f600}".repeat(LONG_LABEL_CHARACTERS);
    let hints = [hint(0, &long), hint(1, "hidden"), hint(3, "visible")];
    let mut display = DisplayMap::build(&document, None);
    display
        .set_inlay_hints(&document, &hints, DEFAULT_MAXIMUM_LENGTH)
        .unwrap();
    let row = display.row_projection(0).unwrap();
    assert_eq!(
        row.text,
        format!(
            "{}\u{2026}ab",
            "\u{1f600}".repeat(LONG_LABEL_CHARACTERS - 1)
        )
    );
    assert!(!row.units.iter().any(|unit| unit.hint == Some(1)));
    assert!(display.row_projection(1).unwrap().text.contains("visible"));
    display.set_inlay_hints(&document, &hints, 0).unwrap();
    assert!(display.row_projection(0).unwrap().text.contains(&long));
    assert!(display.row_projection(0).unwrap().text.contains("hidden"));
}

#[test]
fn 잘못된_위치와_빈_다중줄_라벨은_이전_표시를_손상시키지_않는다() {
    let (_, document) = fixture(TEXT);
    let byte = TEXT.find("value").unwrap();
    let mut display = DisplayMap::build(&document, None);
    display
        .set_inlay_hints(&document, &[hint(byte, "valid")], DEFAULT_MAXIMUM_LENGTH)
        .unwrap();
    let previous = display.clone();
    let emoji = TEXT.find('\u{1f600}').unwrap();
    for invalid in [
        hint(emoji + 1, "invalid"),
        hint(TEXT.len() + 1, "invalid"),
        hint(byte, ""),
        hint(byte, "multi\nline"),
        hint(byte, "multi\rline"),
    ] {
        assert!(
            display
                .set_inlay_hints(&document, &[invalid], DEFAULT_MAXIMUM_LENGTH)
                .is_err()
        );
        assert_eq!(display, previous);
    }
}

#[test]
fn wrap과_접기는_힌트_바이트를_보존하고_새_revision은_이전_표시를_회수한다() {
    let (mut store, document) = fixture(TEXT);
    let byte = TEXT.find("value").unwrap();
    let hints = [hint(byte, "longtypename")];
    let mut display = DisplayMap::build(
        &document,
        Some(WrapSettings {
            wrap_column: WRAP_COLUMN,
            tab_size: TAB_SIZE,
            full_width_columns: FULL_WIDTH_COLUMNS,
            wrapping_indent: WrappingIndent::None,
        }),
    );
    display
        .set_inlay_hints(&document, &hints, DEFAULT_MAXIMUM_LENGTH)
        .unwrap();
    assert!(display.rows_of_line(0).len() > 1);
    for row in display.rows_of_line(0) {
        let projection = display.row_projection(row).unwrap();
        for unit in &projection.units {
            assert!(projection.text.is_char_boundary(unit.text_byte));
            assert!(TEXT.is_char_boundary(unit.source_byte));
            if unit.hint.is_some() {
                assert_eq!(unit.source_byte, byte);
            }
        }
    }
    display.set_hidden_lines(&[1..2]);
    display
        .set_inlay_hints(
            &document,
            &[hint(TEXT.find("next").unwrap(), "hidden")],
            DEFAULT_MAXIMUM_LENGTH,
        )
        .unwrap();
    assert!(
        (0..display.row_count())
            .filter_map(|row| display.row_projection(row))
            .all(|row| !row.text.contains("hidden"))
    );
    store
        .apply(
            document.id,
            Transaction {
                revision: document.revision,
                group: UndoGroup(document.revision),
                origin: None,
                selection_after: None,
                edits: vec![Edit {
                    bytes: 0..0,
                    text: "x".into(),
                }],
            },
        )
        .unwrap();
    let latest = store.documents().snapshot(document.id).unwrap();
    display.refresh(&latest);
    assert!(display.inlay_hints().is_empty());
    assert_eq!(
        display.set_inlay_hints(&document, &hints, DEFAULT_MAXIMUM_LENGTH),
        Err(EditorError::StaleRevision)
    );
}
