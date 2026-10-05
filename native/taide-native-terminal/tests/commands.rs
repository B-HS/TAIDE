use taide_infra::terminal_scan::ScanEvent;
use taide_native_terminal::{
    CommandBlock, Effect, GridDimensions, Limits, Line, MAX_COMMAND_BLOCKS, Size, TerminalCore,
};

const COLUMNS: u16 = 20;
const ROWS: u16 = 4;
const HISTORY: usize = 8;
const SMALL_COLUMNS: u16 = 10;
const SUCCESS: taide_native_terminal::Rgb = taide_native_terminal::Rgb { r: 1, g: 2, b: 3 };
const FAILURE: taide_native_terminal::Rgb = taide_native_terminal::Rgb { r: 4, g: 5, b: 6 };

fn core(history: usize) -> TerminalCore {
    TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        history,
        Limits::default(),
    )
    .unwrap()
}

fn blocks(core: &TerminalCore) -> Vec<CommandBlock> {
    core.command_blocks().unwrap()
}

#[test]
fn osc133은_같은_parser의_정확한_행과_분할_종료_빈_prompt_cap을_보존한다() {
    let mut whole = core(HISTORY);
    let data = b"\x1b]133;C\x07\x1b]133;D;1\x07\x1b]133;A\x07prompt\r\n\x1b]133;B\x07\x1b]133;C\x1b\\output\r\n\x1b]133;D;2\x07tail\r\n\x1b]133;A\x07\x1b]133;D;9\x07";
    let whole_effects = whole.advance(data).unwrap();
    let mut split = core(HISTORY);
    let mut split_effects = Vec::new();
    for byte in data {
        split_effects.extend(split.advance(&[*byte]).unwrap());
    }
    let stream_count = |effects: &[Effect]| {
        effects
            .iter()
            .filter(|effect| matches!(effect, Effect::Stream(ScanEvent::CommandMarker(_))))
            .count()
    };
    assert_eq!(stream_count(&whole_effects), stream_count(&split_effects));
    assert_eq!(blocks(&whole), blocks(&split));
    assert_eq!(
        blocks(&whole),
        [CommandBlock {
            id: 1,
            start: Line(0),
            output: Some(Line(1)),
            end: Some(Line(2)),
            exit_code: Some(2.0),
            alternate: false
        }]
    );
    assert_eq!(whole.grid().unwrap().cursor.point.line, Line(3));
    whole.advance(b"\x1b]133;A\x07\x1b]133;C\x1b").unwrap();
    assert_eq!(blocks(&whole).last().unwrap().output, None);
    whole.advance(b"\\\x1b]133;D;0\x07").unwrap();
    assert_eq!(blocks(&whole).last().unwrap().exit_code, Some(0.0));
    whole.advance(b"\x1b]133;A\x18\x1b]133;A\x1a").unwrap();
    assert_eq!(blocks(&whole).len(), 2);

    let mut legacy = core(HISTORY);
    legacy.advance(b"\x1b]133;A\x07\x1b]133;D;0\x07").unwrap();
    assert_eq!(blocks(&legacy).len(), 1);
    for _ in 0..MAX_COMMAND_BLOCKS + 1 {
        legacy.advance(b"\x1b]133;A\x07").unwrap();
    }
    let capped = blocks(&legacy);
    assert_eq!(capped.len(), MAX_COMMAND_BLOCKS);
    assert_eq!(capped.first().unwrap().id, 3);
    legacy.advance(b"\x1b]133;C\x07\x1b]133;D;7\x07").unwrap();
    assert_eq!(blocks(&legacy).last().unwrap().exit_code, Some(7.0));
    assert!(
        blocks(&legacy)[..MAX_COMMAND_BLOCKS - 1]
            .iter()
            .all(|block| block.exit_code.is_none())
    );
    assert!(legacy.retained().unwrap().bytes < Limits::default().retained_bytes);
}

#[test]
fn osc133_행_marker는_history_region_삭제_clear_alternate_resize를_따른다() {
    let mut terminal = core(2);
    terminal.advance(b"\x1b]133;A\x07first\r\n\x1b]133;C\x07out\r\n\x1b]133;D;0\x07\x1b]133;A\x07second\r\n\x1b]133;C\x07out\r\n\x1b]133;D;1\x07").unwrap();
    assert_eq!(
        blocks(&terminal)
            .iter()
            .map(|block| block.start)
            .collect::<Vec<_>>(),
        [Line(-1), Line(1)]
    );
    terminal.advance(b"\r\n\r\n").unwrap();
    assert_eq!(blocks(&terminal).len(), 1);
    assert_eq!(blocks(&terminal)[0].start, Line(-1));
    terminal.advance(b"\x1b[3J").unwrap();
    assert!(blocks(&terminal).is_empty());
    terminal.advance(b"\x1b[H\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07\x1b[2;1H\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;1\x07").unwrap();
    terminal.advance(b"\x1b[H\x1b[M").unwrap();
    assert_eq!(blocks(&terminal).len(), 1);
    assert_eq!(blocks(&terminal)[0].start, Line(0));
    terminal.advance(b"\x1b[L").unwrap();
    assert_eq!(blocks(&terminal)[0].start, Line(1));
    terminal.advance(b"\x1b[2;3r\x1b[2;1H\x1b[M").unwrap();
    assert!(blocks(&terminal).is_empty());
    terminal
        .advance(
            b"\x1b[r\x1b[H\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07\x1b[?1049h\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;1\x07",
        )
        .unwrap();
    assert_eq!(blocks(&terminal).len(), 2);
    assert!(blocks(&terminal)[1].alternate);
    terminal.advance(b"\x1b[?1049l").unwrap();
    assert_eq!(blocks(&terminal).len(), 2);
    terminal.advance(b"\x1b[?1049h").unwrap();
    assert_eq!(blocks(&terminal).len(), 1);
    terminal.advance(b"\x1b[?1049l\x1b[2J").unwrap();
    assert!(blocks(&terminal).is_empty());
    terminal
        .advance(b"\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07\x1bc")
        .unwrap();
    assert!(blocks(&terminal).is_empty());
    terminal.advance(b"\x1b]133;A\x07\x1b]133;D;0\x07").unwrap();
    assert!(blocks(&terminal).is_empty());

    let mut local = core(HISTORY);
    local.advance(b"\x1b]133;A\x07\x1b]133;D;0\x07").unwrap();
    assert!(!local.clear_current_row().unwrap());
    assert_eq!(blocks(&local).len(), 1);
    local
        .advance(b"\r\n\x1b]133;A\x07\x1b]133;D;1\x07")
        .unwrap();
    assert!(local.clear_current_row().unwrap());
    assert!(blocks(&local).is_empty());

    let mut reflow = core(HISTORY);
    reflow
        .advance(b"\x1b]133;A\x07abcdefghijklmno\r\n\x1b]133;A\x07next\r\n\x1b]133;D;0\x07")
        .unwrap();
    let before = blocks(&reflow);
    reflow
        .resize(Size {
            columns: SMALL_COLUMNS,
            rows: ROWS,
        })
        .unwrap();
    assert_eq!(blocks(&reflow).len(), 2);
    assert_eq!(blocks(&reflow)[0].id, before[0].id);
    assert_eq!(
        blocks(&reflow)[1].start.0 + reflow.grid().unwrap().history_size() as i32,
        2
    );
    reflow
        .resize(Size {
            columns: COLUMNS,
            rows: ROWS,
        })
        .unwrap();
    assert_eq!(
        blocks(&reflow)
            .iter()
            .map(|block| block.start.0 + reflow.grid().unwrap().history_size() as i32)
            .collect::<Vec<_>>(),
        [0, 1]
    );
}

#[test]
fn osc133_exit는_js_number의_정수_문법과_raw_공백을_보존한다() {
    let cases = [
        ("", Some(0.0)),
        ("  ", Some(0.0)),
        ("-0", Some(-0.0)),
        ("+2", Some(2.0)),
        ("1e3", Some(1000.0)),
        ("2.0", Some(2.0)),
        ("2.5", None),
        ("Infinity", None),
        ("0x10", Some(16.0)),
        ("0b11", Some(3.0)),
        ("0o10", Some(8.0)),
        ("-0x1", None),
        ("0x20000000000001", Some(9007199254740992.0)),
        ("0x20000000000003", Some(9007199254740996.0)),
        ("\u{feff}7\u{a0}", Some(7.0)),
        ("\u{85}7", None),
        ("1\t2", None),
        ("NaN", None),
        ("1_0", None),
        ("0x", None),
        ("1e309", None),
        ("0b2", None),
    ];
    let mut terminal = core(HISTORY);
    for (raw, expected) in cases {
        terminal
            .advance(format!("\x1b]133;A\x07\x1b]133;D;{raw}\x07").as_bytes())
            .unwrap();
        assert_eq!(
            blocks(&terminal).last().unwrap().exit_code,
            expected,
            "{raw:?}"
        );
    }
    terminal.advance(b"\x1b]133;A\x07\x1b]133;D\x07").unwrap();
    assert_eq!(blocks(&terminal).last().unwrap().exit_code, None);
}

#[test]
fn osc133_decoration은_완료_시점_color와_행_범위_alt_clear를_보존한다() {
    use taide_native_terminal::CommandColors;
    let mut terminal = core(HISTORY);
    terminal
        .configure_command_colors(CommandColors {
            success: Some(SUCCESS),
            failure: Some(FAILURE),
        })
        .unwrap();
    terminal.advance(b"\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07\r\n\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;9\x07\r\n").unwrap();
    terminal
        .configure_command_colors(CommandColors {
            success: Some(FAILURE),
            failure: None,
        })
        .unwrap();
    terminal.advance(b"\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;0\x07\r\n\x1b]133;A\x07\x1b]133;C\x07\x1b]133;D;1\x07").unwrap();
    let decorations = terminal
        .command_decorations(Line(0)..Line(i32::from(ROWS)))
        .unwrap();
    assert_eq!(
        decorations
            .iter()
            .map(|decoration| (decoration.line, decoration.color))
            .collect::<Vec<_>>(),
        [(Line(0), SUCCESS), (Line(1), FAILURE), (Line(2), FAILURE)]
    );
    assert_eq!(
        terminal
            .command_decorations(Line(1)..Line(2))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(terminal.command_start_lines().unwrap(), [0, 1, 2, 3]);
    terminal.advance(b"\x1b[?1049h").unwrap();
    assert!(
        terminal
            .command_decorations(Line(0)..Line(i32::from(ROWS)))
            .unwrap()
            .is_empty()
    );
    terminal.advance(b"\x1b[?1049l").unwrap();
    assert_eq!(
        terminal
            .command_decorations(Line(0)..Line(i32::from(ROWS)))
            .unwrap(),
        decorations
    );
    terminal.advance(b"\x1b[2J").unwrap();
    assert!(
        terminal
            .command_decorations(Line(0)..Line(i32::from(ROWS)))
            .unwrap()
            .is_empty()
    );
}
