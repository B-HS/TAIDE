use taide_native_terminal::{
    CellFlags, Column, Line, Mode, Point, Selection, SelectionType, Side, Size, TerminalCore,
    session::{Failure, Phase, SharedTerminal},
};

const COLUMNS: u16 = 12;
const ROWS: u16 = 4;
const HISTORY: usize = 4;
const COPY_BYTES: usize = 1024;
const PROMPT_COLUMNS: usize = 5;
const PROMPT: &str = "한e\u{301}> ";
const ASTRAL_COLUMNS: usize = 1;

fn copy_prompt(core: &TerminalCore, columns: usize) -> String {
    let mut selection = Selection::new(
        SelectionType::Simple,
        Point::new(Line(0), Column(0)),
        Side::Left,
    );
    selection.update(Point::new(Line(0), Column(columns - 1)), Side::Right);
    core.selection_text(&selection, COPY_BYTES)
        .unwrap()
        .unwrap()
}

#[test]
fn clear_current_row는_history만_버리고_cursor행_mode_속성_partial_utf8을_보존한다() {
    for alternate in [false, true] {
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Default::default(),
        )
        .unwrap();
        core.advance(b"main\r\n").unwrap();
        if alternate {
            core.advance(b"\x1b[?1049h").unwrap();
        }
        core.advance(
            b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\n\x1b[?2004h\x1b[?1h\x1b[1;31m",
        )
        .unwrap();
        core.advance(PROMPT.as_bytes()).unwrap();
        core.advance(&[0xf0, 0x90]).unwrap();
        let grid = core.grid().unwrap();
        let cursor = grid.cursor.clone();
        let current = grid[cursor.point.line].clone();
        let mode = core.mode().unwrap();
        let stamp = core.selection_stamp().unwrap();
        assert!(cursor.point.line > Line(0));
        assert_eq!(cursor.point.column, Column(PROMPT_COLUMNS));
        assert!(core.clear_current_row().unwrap());
        let cleared = core.selection_stamp().unwrap();
        assert_ne!(cleared.buffer_epoch, stamp.buffer_epoch);
        assert_eq!(cleared.input_epoch, stamp.input_epoch);
        assert_eq!(cleared.history, 0);
        assert_eq!(core.mode().unwrap(), mode);
        let grid = core.grid().unwrap();
        assert_eq!(grid.cursor.point, Point::new(Line(0), cursor.point.column));
        assert_eq!(grid.cursor.template, cursor.template);
        assert_eq!(grid.cursor.input_needs_wrap, cursor.input_needs_wrap);
        assert_eq!(grid.display_offset(), 0);
        for column in 0..usize::from(COLUMNS) {
            assert_eq!(grid[Line(0)][Column(column)], current[Column(column)]);
            for row in 1..ROWS {
                let cell = &grid[Line(i32::from(row))][Column(column)];
                assert_eq!(cell.c, ' ');
                assert!(cell.flags.is_empty());
                assert_ne!(cell.fg, cursor.template.fg);
            }
        }
        assert_eq!(copy_prompt(&core, PROMPT_COLUMNS), PROMPT);
        assert!(!core.clear_current_row().unwrap());
        assert_eq!(core.selection_stamp().unwrap(), cleared);
        core.advance(&[0x90, 0x80]).unwrap();
        assert_eq!(
            copy_prompt(&core, PROMPT_COLUMNS + ASTRAL_COLUMNS),
            format!("{PROMPT}𐐀")
        );
        assert!(
            core.grid().unwrap()[Line(0)][Column(0)]
                .flags
                .contains(CellFlags::WIDE_CHAR)
        );
        assert!(
            core.mode()
                .unwrap()
                .contains(Mode::BRACKETED_PASTE | Mode::APP_CURSOR)
        );
        if alternate {
            core.advance(b"\x1b[?1049l").unwrap();
            assert_eq!(copy_prompt(&core, "main".len()), "main");
        }
    }
}

#[test]
fn clear_current_row의_shared_owner는_revision과_pending_sync를_보존하고_failure를_거절한다() {
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Default::default(),
    )
    .unwrap();
    let frame = terminal.advance(b"old\r\n\x1b[?2026hheld").unwrap();
    assert!(frame.outcome.text.contains("old"));
    let previous = terminal
        .snapshot(|state| {
            (
                state.revision,
                state.core.selection_stamp().unwrap().input_epoch,
            )
        })
        .unwrap();
    assert!(terminal.clear_current_row().unwrap());
    terminal
        .snapshot(|state| {
            assert_eq!(state.revision, previous.0);
            assert_eq!(state.phase, Phase::Running);
            assert_eq!(
                state.core.selection_stamp().unwrap().input_epoch,
                previous.1
            );
            assert_eq!(copy_prompt(state.core, "held".len()), "");
        })
        .unwrap();
    let released = terminal.advance(b"\x1b[?2026l").unwrap();
    assert_eq!(released.revision, previous.0 + 1);
    assert!(released.outcome.text.contains("held"));
    assert_eq!(
        terminal
            .snapshot(|state| copy_prompt(state.core, "held".len()))
            .unwrap(),
        "held"
    );
    terminal.fail_and_stop(Failure::Parser).unwrap();
    assert!(terminal.clear_current_row().is_err());
    assert_eq!(
        terminal.snapshot(|state| state.phase).unwrap(),
        Phase::Failed(Failure::Parser)
    );
}
