use taide_native_terminal::{
    Column, GridDimensions, Line, Point, Selection, SelectionGranularity, SelectionType, Side,
    Size, TerminalCore,
};

const COLUMNS: u16 = 12;
const ROWS: u16 = 8;
const HISTORY: usize = 8;
const COPY_BYTES: usize = 1024;
const WORD_ROW: i32 = 1;
const SPACES_ROW: i32 = 2;
const UNICODE_ROW: i32 = 3;
const WRAPPED_ROW: i32 = 5;
const LAST_ROW: i32 = 6;
const SEPARATOR_COLUMN: usize = 3;
const INNER_WORD_COLUMN: usize = 4;
const BLANK_COLUMN: usize = 10;

#[test]
fn selection_bounds는_xterm의_separator_공백_wide_nfd와_wrapped_word_line을_보존한다() {
    let mut core = TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    core.advance(
        "alpha-beta x\r\nfoo(bar) z\r\nA   B\r\n한e\u{301}𐐀 Q\r\nabcdefghijklmnop\r\nlast"
            .as_bytes(),
    )
    .unwrap();
    let copy = |line, column, kind| {
        let range = core
            .selection_at(Point::new(Line(line), Column(column)), kind)
            .unwrap();
        let mut selection = Selection::new(SelectionType::Simple, range.start, Side::Left);
        selection.update(range.end, Side::Right);
        core.selection_text(&selection, COPY_BYTES)
            .unwrap()
            .unwrap()
    };
    assert_eq!(copy(0, 1, SelectionGranularity::Word), "alpha-beta");
    assert_eq!(
        copy(WORD_ROW, SEPARATOR_COLUMN, SelectionGranularity::Word),
        "foo(bar"
    );
    assert_eq!(
        copy(WORD_ROW, INNER_WORD_COLUMN, SelectionGranularity::Word),
        "bar"
    );
    assert_eq!(copy(SPACES_ROW, 1, SelectionGranularity::Word), "   ");
    assert_eq!(
        copy(UNICODE_ROW, 1, SelectionGranularity::Word),
        "한e\u{301}𐐀"
    );
    assert_eq!(
        copy(WRAPPED_ROW, 1, SelectionGranularity::Word),
        "abcdefghijklmnop"
    );
    assert_eq!(
        copy(WRAPPED_ROW, 1, SelectionGranularity::Line),
        "abcdefghijklmnop"
    );
    assert_eq!(copy(WORD_ROW, 0, SelectionGranularity::Line), "foo(bar) z");
    assert_eq!(copy(LAST_ROW, BLANK_COLUMN, SelectionGranularity::Word), "");
    assert!(
        core.selection_at(Point::new(Line(-1), Column(0)), SelectionGranularity::Word)
            .is_err()
    );
    assert!(
        core.selection_at(
            Point::new(Line(0), Column(usize::from(COLUMNS))),
            SelectionGranularity::Line
        )
        .is_err()
    );
    assert_eq!(core.grid().unwrap().display_offset(), 0);
    assert_eq!(core.grid().unwrap().total_lines(), usize::from(ROWS));
}

#[test]
fn selection_motion은_trim_erase_partial_scroll과_resize_buffer_aba를_구분한다() {
    const VISIBLE_ROWS: u16 = 4;
    const RETAINED_ROWS: usize = 2;
    const INNER_COLUMN: usize = 2;
    const NARROW_COLUMNS: u16 = 6;
    let size = Size {
        columns: COLUMNS,
        rows: VISIBLE_ROWS,
    };
    let mut core = TerminalCore::new(size, RETAINED_ROWS, Default::default()).unwrap();
    core.advance(b"first\r\nsecond\r\nthird\r\nfourth\r\nfifth\r\nsixth")
        .unwrap();
    let previous = core.selection_stamp().unwrap();
    assert_eq!(previous.history, RETAINED_ROWS);
    let mut selection = Selection::new(
        SelectionType::Simple,
        Point::new(Line(-(RETAINED_ROWS as i32)), Column(INNER_COLUMN)),
        Side::Left,
    );
    selection.update(Point::new(Line(1), Column(INNER_COLUMN)), Side::Right);
    let mut reversed = Selection::new(
        SelectionType::Simple,
        Point::new(Line(1), Column(INNER_COLUMN)),
        Side::Right,
    );
    reversed.update(
        Point::new(Line(-(RETAINED_ROWS as i32)), Column(INNER_COLUMN)),
        Side::Left,
    );
    core.advance(b"\r\nseventh").unwrap();
    let rebased = core
        .rebase_selection(&selection, previous)
        .unwrap()
        .unwrap();
    let range = core.selection_range(&rebased).unwrap().unwrap();
    assert_eq!(
        range.start,
        Point::new(Line(-(RETAINED_ROWS as i32)), Column(INNER_COLUMN))
    );
    assert_eq!(
        core.selection_text(&rebased, COPY_BYTES).unwrap().unwrap(),
        "cond\nthird\nfou"
    );
    assert!(
        core.rebase_selection(&reversed, previous)
            .unwrap()
            .is_none()
    );
    let before_erase = core.selection_stamp().unwrap();
    core.advance(b"\x1b[2J").unwrap();
    assert_eq!(core.selection_stamp().unwrap(), before_erase);
    assert_eq!(
        core.grid().unwrap().total_lines(),
        usize::from(VISIBLE_ROWS) + RETAINED_ROWS
    );
    assert_eq!(
        core.selection_text(&rebased, COPY_BYTES).unwrap().unwrap(),
        "cond\nthird\n"
    );
    core.advance(b"\x1b[3J").unwrap();
    let after_erase = core.selection_stamp().unwrap();
    assert_eq!(after_erase.origin, before_erase.origin);
    assert_eq!(
        after_erase.trimmed,
        before_erase.trimmed + RETAINED_ROWS as u64
    );
    assert_eq!(after_erase.history, 0);
    let rebased = core
        .rebase_selection(&rebased, before_erase)
        .unwrap()
        .unwrap();
    assert_eq!(
        core.selection_range(&rebased).unwrap().unwrap().start,
        Point::new(Line(0), Column(INNER_COLUMN))
    );

    core.advance(b"\x1b[2;4r\x1b[4;1H\n\x1b[r").unwrap();
    assert_eq!(core.selection_stamp().unwrap(), after_erase);
    core.advance(b"\x1b[?1049h\x1b[?1049l").unwrap();
    assert!(
        core.rebase_selection(&rebased, after_erase)
            .unwrap()
            .is_none()
    );
    let before_resize = core.selection_stamp().unwrap();
    core.resize(Size {
        columns: COLUMNS,
        rows: VISIBLE_ROWS + 1,
    })
    .unwrap();
    core.resize(size).unwrap();
    assert_eq!(core.selection_stamp().unwrap().rows, before_resize.rows);
    assert_ne!(
        core.selection_stamp().unwrap().rows_epoch,
        before_resize.rows_epoch
    );
    assert!(
        core.rebase_selection(&rebased, before_resize)
            .unwrap()
            .is_none()
    );
    let before_reset = core.selection_stamp().unwrap();
    core.advance(b"\x1bc").unwrap();
    assert!(
        core.rebase_selection(&rebased, before_reset)
            .unwrap()
            .is_none()
    );

    core.advance(b"abcdefghijklmnopqrstuvwx\r\nline2\r\nline3\r\nline4")
        .unwrap();
    let before_columns = core.selection_stamp().unwrap();
    core.resize(Size {
        columns: NARROW_COLUMNS,
        rows: VISIBLE_ROWS,
    })
    .unwrap();
    let after_columns = core.selection_stamp().unwrap();
    assert_eq!(after_columns.rows_epoch, before_columns.rows_epoch);
    assert_eq!(
        after_columns.origin - before_columns.origin,
        after_columns.history as i64 - before_columns.history as i64
            + (after_columns.trimmed - before_columns.trimmed) as i64
    );
    assert_eq!(after_columns.columns, usize::from(NARROW_COLUMNS));
}

#[test]
fn selection_columns는_reflow_trim_후에도_xterm의_원본_절대_좌표를_유지한다() {
    const VISIBLE_ROWS: u16 = 4;
    const RETAINED_ROWS: usize = 2;
    const NARROW_COLUMNS: u16 = 6;
    let mut core = TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: VISIBLE_ROWS,
        },
        RETAINED_ROWS,
        Default::default(),
    )
    .unwrap();
    core.advance(b"abcdefghijklmnopqrstuvwx\r\nline2\r\nline3\r\nline4")
        .unwrap();
    let previous = core.selection_stamp().unwrap();
    assert_eq!(previous.history, 1);
    let mut selection = Selection::new(
        SelectionType::Simple,
        Point::new(Line(-1), Column(0)),
        Side::Left,
    );
    selection.update(
        Point::new(Line(0), Column(usize::from(COLUMNS) - 1)),
        Side::Right,
    );
    assert_eq!(
        core.selection_text(&selection, COPY_BYTES)
            .unwrap()
            .unwrap(),
        "abcdefghijklmnopqrstuvwx"
    );
    core.resize(Size {
        columns: NARROW_COLUMNS,
        rows: VISIBLE_ROWS,
    })
    .unwrap();
    let current = core.selection_stamp().unwrap();
    assert_eq!(current.history, RETAINED_ROWS);
    assert_eq!(current.trimmed, previous.trimmed + 1);
    let selection = core
        .rebase_selection(&selection, previous)
        .unwrap()
        .unwrap();
    let range = core.selection_range(&selection).unwrap().unwrap();
    assert_eq!(range.start.line, Line(-(RETAINED_ROWS as i32)));
    assert_eq!(range.end.line, range.start.line);
    assert_eq!(
        core.selection_text(&selection, COPY_BYTES)
            .unwrap()
            .unwrap(),
        "ghijkl"
    );
}

#[test]
fn selection_slice는_half_open_줄바꿈과_폭_밖_원본_열을_보존한다() {
    const VISIBLE_ROWS: u16 = 4;
    const NARROW_COLUMNS: u16 = 6;
    const ORIGINAL_COLUMN: usize = 9;
    let mut core = TerminalCore::new(
        Size {
            columns: NARROW_COLUMNS,
            rows: VISIBLE_ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    core.advance(b"abcdef\r\nsecond\r\nthird").unwrap();
    let mut selection = Selection::new(
        SelectionType::Simple,
        Point::new(Line(0), Column(2)),
        Side::Left,
    );
    selection.update(Point::new(Line(1), Column(0)), Side::Left);
    assert_eq!(
        core.selection_text(&selection, COPY_BYTES)
            .unwrap()
            .unwrap(),
        "cdef\n"
    );
    let mut selection = Selection::new(
        SelectionType::Simple,
        Point::new(Line(0), Column(ORIGINAL_COLUMN)),
        Side::Left,
    );
    selection.update(Point::new(Line(1), Column(2)), Side::Left);
    assert_eq!(
        core.selection_text(&selection, COPY_BYTES)
            .unwrap()
            .unwrap(),
        "\nse"
    );
}

#[test]
fn selection_render는_복사_원본_범위와_실제_셀_highlight를_분리한다() {
    const VISIBLE_ROWS: u16 = 4;
    const NARROW_COLUMNS: u16 = 6;
    const ORIGINAL_COLUMN: usize = 9;
    let mut core = TerminalCore::new(
        Size {
            columns: NARROW_COLUMNS,
            rows: VISIBLE_ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    core.advance(b"abcdef\r\nsecond\r\nthird").unwrap();
    let mut selection = Selection::new(
        SelectionType::Simple,
        Point::new(Line(0), Column(ORIGINAL_COLUMN)),
        Side::Left,
    );
    selection.update(Point::new(Line(1), Column(2)), Side::Left);
    let range = core.selection_range(&selection).unwrap().unwrap();
    assert_eq!(range.start, Point::new(Line(1), Column(0)));
    assert_eq!(range.end, Point::new(Line(1), Column(1)));
    assert!(!range.contains(Point::new(Line(0), Column(usize::from(NARROW_COLUMNS) - 1))));
    let mut block = Selection::new(
        SelectionType::Block,
        Point::new(Line(0), Column(ORIGINAL_COLUMN)),
        Side::Left,
    );
    block.update(
        Point::new(Line(1), Column(ORIGINAL_COLUMN + 1)),
        Side::Right,
    );
    assert!(core.selection_range(&block).unwrap().is_none());
    assert_eq!(
        core.selection_text(&block, COPY_BYTES).unwrap().unwrap(),
        "\n"
    );
    let mut virtual_end = Selection::new(
        SelectionType::Simple,
        Point::new(Line(2), Column(2)),
        Side::Left,
    );
    virtual_end.update(
        Point::new(Line(i32::from(VISIBLE_ROWS) + 1), Column(0)),
        Side::Left,
    );
    let range = core.selection_range(&virtual_end).unwrap().unwrap();
    assert_eq!(
        range.end,
        Point::new(
            Line(i32::from(VISIBLE_ROWS) - 1),
            Column(usize::from(NARROW_COLUMNS) - 1)
        )
    );
    assert_eq!(
        core.selection_text(&virtual_end, COPY_BYTES)
            .unwrap()
            .unwrap(),
        "ird\n\n\n"
    );
    assert!(core.selection_text(&virtual_end, 1).is_err());
}
