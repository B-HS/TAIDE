use taide_native_terminal::session::SharedTerminal;
use taide_native_terminal::{
    CursorShape, CursorStyle, Effect, Limits, Size, TerminalCore, TerminalEvent,
};

const COLUMNS: u16 = 12;
const ROWS: u16 = 2;
const HISTORY: usize = 8;

#[test]
fn cursor_defaults는_실제_core_override_history_mode_query와_live_설정을_보존한다() {
    let size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    let terminal = SharedTerminal::new(size, HISTORY, Limits::default()).unwrap();
    let bar = CursorStyle {
        shape: CursorShape::Beam,
        blinking: true,
    };
    let underline = CursorStyle {
        shape: CursorShape::Underline,
        blinking: true,
    };
    assert!(terminal.configure_cursor(bar).unwrap());
    terminal
        .advance(b"\x1b[?1hfirst\r\nsecond\r\nkeep")
        .unwrap();
    let state = terminal
        .snapshot(|snapshot| {
            (snapshot.revision, snapshot.core.mode().unwrap(), {
                use taide_native_terminal::{Column, GridDimensions, Line, Point};
                let grid = snapshot.core.grid().unwrap();
                (grid.topmost_line().0..grid.screen_lines() as i32)
                    .flat_map(|line| {
                        (0..grid.columns())
                            .map(move |column| grid[Point::new(Line(line), Column(column))].clone())
                    })
                    .collect::<Vec<_>>()
            })
        })
        .unwrap();
    assert!(terminal.configure_cursor(underline).unwrap());
    terminal
        .snapshot(|snapshot| {
            assert_eq!(snapshot.revision, state.0);
            assert_eq!(snapshot.core.mode().unwrap(), state.1);
            use taide_native_terminal::{Column, GridDimensions, Line, Point};
            let grid = snapshot.core.grid().unwrap();
            let cells = (grid.topmost_line().0..grid.screen_lines() as i32)
                .flat_map(|line| {
                    (0..grid.columns())
                        .map(move |column| grid[Point::new(Line(line), Column(column))].clone())
                })
                .collect::<Vec<_>>();
            assert_eq!(cells, state.2);
            assert_eq!(snapshot.core.cursor_style().unwrap(), underline);
        })
        .unwrap();
    terminal.advance(b"\x1b[2 q").unwrap();
    assert!(terminal.configure_cursor(bar).unwrap());
    terminal.advance(b"\x1b[?12h").unwrap();
    let report = terminal.advance(b"\x1b[?12$p").unwrap();
    assert!(report.outcome.effects.iter().any(|effect| matches!(effect,
        Effect::Terminal(TerminalEvent::PtyWrite(reply)) if reply == "\x1b[?12;1$y")));
    assert_eq!(
        terminal
            .snapshot(|snapshot| snapshot.core.cursor_style().unwrap())
            .unwrap(),
        CursorStyle {
            shape: CursorShape::Block,
            blinking: false
        }
    );
    terminal.advance(b"\x1b[0 q\x1b[?12l").unwrap();
    assert!(!terminal.configure_cursor(bar).unwrap());
    assert_eq!(
        terminal
            .snapshot(|snapshot| snapshot.core.cursor_style().unwrap())
            .unwrap(),
        CursorStyle {
            shape: CursorShape::Beam,
            blinking: false
        }
    );
    assert!(terminal.configure_cursor(underline).unwrap());
    let frame = terminal.advance(b"\x1b[?12$p").unwrap();
    assert!(frame.outcome.effects.iter().any(|effect| matches!(effect,
        Effect::Terminal(TerminalEvent::PtyWrite(reply)) if reply == "\x1b[?12;1$y")));
    assert!(terminal.configure_cursor(bar).unwrap());
    assert_eq!(
        terminal
            .snapshot(|snapshot| snapshot.core.cursor_style().unwrap())
            .unwrap(),
        bar
    );
    terminal.advance(b"\x1b[?25l").unwrap();
    assert_eq!(
        terminal
            .snapshot(|snapshot| snapshot.core.content().unwrap().cursor.shape)
            .unwrap(),
        CursorShape::Hidden
    );
    terminal.advance(b"\x1bc").unwrap();
    assert_eq!(
        terminal
            .snapshot(|snapshot| snapshot.core.cursor_style().unwrap())
            .unwrap(),
        bar
    );
    let mut core = TerminalCore::new(size, HISTORY, Limits::default()).unwrap();
    assert!(
        core.advance(&vec![7; Limits::default().effect_count + 1])
            .is_err()
    );
    assert!(core.set_default_cursor_style(bar).is_err());
    assert!(core.cursor_style().is_err());
}
