use taide_infra::terminal_scan::ScanEvent;
use taide_native_terminal::{
    Column, Effect, GridDimensions, Limits, Line, Size, TerminalCore, TerminalEvent,
    session::{Failure, SharedTerminal},
};

const COLUMNS: u16 = 20;
const ROWS: u16 = 4;
const HISTORY: usize = 64;
const PRINTED_LINES: usize = 50;
const SHRUNK_HISTORY: usize = 10;
const GROWN_HISTORY: usize = 30;
const LATER_LINES: usize = 25;
const OVER_BUDGET_HISTORY: usize = 10_000_000;

fn core() -> TerminalCore {
    TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap()
}

fn print_lines(core: &mut TerminalCore, prefix: &str, count: usize) {
    let output = (0..count)
        .map(|index| format!("{prefix}-{index}\r\n"))
        .collect::<String>();
    core.advance(output.as_bytes()).unwrap();
}

fn row_text(core: &TerminalCore, line: i32) -> String {
    let grid = core.grid().unwrap();
    (0..grid.columns())
        .map(|column| grid[Line(line)][Column(column)].c)
        .collect::<String>()
        .trim_end()
        .into()
}

fn all_text(core: &TerminalCore) -> String {
    let grid = core.grid().unwrap();
    let history = grid.history_size() as i32;
    (-history..grid.screen_lines() as i32)
        .map(|line| row_text(core, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn 실행중_history_축소는_오래된_줄만_자르고_제목_효과를_만들지_않는다() {
    let mut terminal = core();
    print_lines(&mut terminal, "line", PRINTED_LINES);
    assert_eq!(terminal.history(), HISTORY);
    let before = terminal.grid().unwrap().history_size();
    assert!(before > SHRUNK_HISTORY);
    assert!(terminal.set_history(SHRUNK_HISTORY).unwrap());
    assert!(!terminal.set_history(SHRUNK_HISTORY).unwrap());
    assert_eq!(terminal.history(), SHRUNK_HISTORY);
    assert_eq!(terminal.grid().unwrap().history_size(), SHRUNK_HISTORY);
    let oldest_kept = PRINTED_LINES + 1 - usize::from(ROWS) - SHRUNK_HISTORY;
    assert_eq!(
        row_text(&terminal, -(SHRUNK_HISTORY as i32)),
        format!("line-{oldest_kept}")
    );
    let text = all_text(&terminal);
    assert!(!text.contains(&format!("line-{}\n", oldest_kept - 1)));
    assert!(text.contains(&format!("line-{}", PRINTED_LINES - 1)));
    let effects = terminal.flush_sync_outcome().unwrap().effects;
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        Effect::Stream(ScanEvent::Title(_))
            | Effect::Terminal(TerminalEvent::Title(_) | TerminalEvent::ResetTitle)
    )));
}

#[test]
fn 실행중_history_확대는_기존_줄을_유지하고_새_상한까지_쌓는다() {
    let mut terminal = core();
    print_lines(&mut terminal, "line", PRINTED_LINES);
    terminal.set_history(SHRUNK_HISTORY).unwrap();
    let oldest = row_text(&terminal, -(SHRUNK_HISTORY as i32));
    assert!(terminal.set_history(GROWN_HISTORY).unwrap());
    assert_eq!(terminal.grid().unwrap().history_size(), SHRUNK_HISTORY);
    assert_eq!(row_text(&terminal, -(SHRUNK_HISTORY as i32)), oldest);
    print_lines(&mut terminal, "later", LATER_LINES);
    assert_eq!(terminal.grid().unwrap().history_size(), GROWN_HISTORY);
    assert!(all_text(&terminal).contains(&format!("later-{}", LATER_LINES - 1)));
}

#[test]
fn 예산을_넘는_history는_거절하고_core를_그대로_둔다() {
    let mut terminal = core();
    print_lines(&mut terminal, "line", PRINTED_LINES);
    let before = terminal.grid().unwrap().history_size();
    assert!(terminal.set_history(OVER_BUDGET_HISTORY).is_err());
    assert_eq!(terminal.history(), HISTORY);
    assert_eq!(terminal.grid().unwrap().history_size(), before);
    terminal.advance(b"after\r\n").unwrap();
    assert!(all_text(&terminal).contains("after"));
}

#[test]
fn alt_screen_중_history_변경은_복귀한_primary에_적용된다() {
    let mut terminal = core();
    print_lines(&mut terminal, "line", PRINTED_LINES);
    terminal.advance(b"\x1b[?1049halternate").unwrap();
    assert!(terminal.set_history(SHRUNK_HISTORY).unwrap());
    terminal.advance(b"\x1b[?1049l").unwrap();
    assert!(terminal.grid().unwrap().history_size() <= SHRUNK_HISTORY);
    assert!(all_text(&terminal).contains(&format!("line-{}", PRINTED_LINES - 1)));
}

#[test]
fn 잘린_줄의_osc133_블록은_command_blocks에서_빠진다() {
    let mut terminal = core();
    terminal
        .advance(b"\x1b]133;A\x07\x1b]133;C\x07old\r\n\x1b]133;D;0\x07")
        .unwrap();
    print_lines(&mut terminal, "line", PRINTED_LINES);
    terminal
        .advance(b"\x1b]133;A\x07\x1b]133;C\x07new\r\n\x1b]133;D;1\x07")
        .unwrap();
    assert_eq!(terminal.command_blocks().unwrap().len(), 2);
    terminal.set_history(SHRUNK_HISTORY).unwrap();
    let blocks = terminal.command_blocks().unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].exit_code, Some(1.0));
}

#[test]
fn shared_history는_같은값_실패상태_거절값을_다시_적용하지_않는다() {
    let terminal = SharedTerminal::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap();
    let history = || {
        terminal
            .snapshot(|snapshot| snapshot.core.history())
            .unwrap()
    };
    assert!(!terminal.configure_history(HISTORY).unwrap());
    assert!(terminal.configure_history(SHRUNK_HISTORY).unwrap());
    assert_eq!(history(), SHRUNK_HISTORY);
    assert!(terminal.configure_history(OVER_BUDGET_HISTORY).is_err());
    assert!(!terminal.configure_history(OVER_BUDGET_HISTORY).unwrap());
    assert_eq!(history(), SHRUNK_HISTORY);
    assert!(terminal.configure_history(GROWN_HISTORY).unwrap());
    assert_eq!(history(), GROWN_HISTORY);
    terminal.fail_and_stop(Failure::Delivery).unwrap();
    assert!(!terminal.configure_history(SHRUNK_HISTORY).unwrap());
}
