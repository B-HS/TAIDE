use alacritty_terminal::{
    event::Event,
    grid::{Dimensions, Scroll},
    index::{Column, Line},
    term::{TermDamage, TermMode, cell::Flags},
    vte::ansi::{Color, Rgb},
};
use taide_infra::terminal_scan::ScanEvent;
use taide_native_terminal::{Effect, Limits, Size, TerminalCore};

const COLUMNS: u16 = 20;
const ROWS: u16 = 4;
const HISTORY: usize = 8;
const NARROW_COLUMNS: u16 = 10;

#[test]
fn 제목은_osc와_stack_복원에서_중복없이_정책을_적용한다() {
    let mut core = TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap();
    let sequence = b"\x1b]2;one\x07\x1b[22;0t\x1b]2;two\x07\x1b[23;0t\x1b]7;/synthetic\x07";
    assert_eq!(
        effects(core.advance(sequence).unwrap()),
        ["title:one", "title:two", "title:one", "cwd:/synthetic"]
    );
    let oversized = format!(
        "\x1b]2;{}\x07",
        "x".repeat(taide_infra::terminal_scan::MAX_TITLE_BYTES + 1)
    );
    assert!(core.advance(oversized.as_bytes()).unwrap().is_empty());
    let mut empty = TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap();
    let events = empty
        .advance(b"\x1b[22;0t\x1b]2;temporary\x07\x1b[23;0t")
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0], Effect::Stream(ScanEvent::Title(title)) if title == "temporary"));
    assert!(matches!(&events[1], Effect::Terminal(Event::ResetTitle)));
}

fn effects(effects: Vec<Effect>) -> Vec<String> {
    effects
        .into_iter()
        .map(|effect| match effect {
            Effect::Stream(ScanEvent::Title(title)) => format!("title:{title}"),
            Effect::Stream(ScanEvent::Cwd(cwd)) => format!("cwd:{cwd}"),
            Effect::Terminal(Event::PtyWrite(reply)) => format!("reply:{reply}"),
            Effect::Terminal(Event::Bell) => "bell".into(),
            Effect::Terminal(Event::ClipboardLoad(..) | Event::ClipboardStore(..)) => {
                panic!("clipboard request must not leave the core")
            }
            _ => panic!("unexpected synthetic effect"),
        })
        .collect()
}

#[test]
fn 단일_core는_grid_mode_damage_분할_효과_상한과_명시적_폐기를_검사한다() {
    let size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    let limits = Limits::default();
    let fixture = "\x1b]2;synthetic-title\x07\x1b]7;/synthetic/cwd\x1b\\\x1b[38;2;1;2;3mZ\x1b[0m한e\u{301}\x1b]8;id=fixed;https://example.invalid\x1b\\L\x1b]8;;\x1b\\\x1b]52;c;c3ludGhldGlj\x07\x1b]52;c;?\x1b\\\x1b[6n\x07";
    let mut whole = TerminalCore::new(size, HISTORY, limits).unwrap();
    let complete = effects(whole.advance(fixture.as_bytes()).unwrap());
    let mut split = TerminalCore::new(size, HISTORY, limits).unwrap();
    let mut fragmented = Vec::new();
    for byte in fixture.bytes() {
        fragmented.extend(effects(split.advance(&[byte]).unwrap()));
    }
    assert_eq!(complete, fragmented);
    assert_eq!(
        &complete[..2],
        &["title:synthetic-title", "cwd:/synthetic/cwd"]
    );
    assert!(
        complete
            .iter()
            .any(|effect| effect.starts_with("reply:\x1b["))
    );
    assert_eq!(complete.last().unwrap(), "bell");
    let whole_grid = whole.grid().unwrap();
    let split_grid = split.grid().unwrap();
    for row in 0..i32::from(ROWS) {
        for column in 0..usize::from(COLUMNS) {
            assert_eq!(
                whole_grid[Line(row)][Column(column)],
                split_grid[Line(row)][Column(column)]
            );
        }
    }
    assert_eq!(
        whole_grid[Line(0)][Column(0)].fg,
        Color::Spec(Rgb { r: 1, g: 2, b: 3 })
    );
    assert!(
        whole_grid[Line(0)][Column(1)]
            .flags
            .contains(Flags::WIDE_CHAR)
    );
    assert!(
        whole_grid[Line(0)][Column(2)]
            .flags
            .contains(Flags::WIDE_CHAR_SPACER)
    );
    assert_eq!(
        whole_grid[Line(0)][Column(3)].zerowidth().unwrap(),
        &['\u{301}']
    );
    assert_eq!(
        whole_grid[Line(0)][Column(4)].hyperlink().unwrap().uri(),
        "https://example.invalid"
    );
    whole
        .advance(b"\x1b[?2004h\x1b[?1h\x1b[?1049h\x1b[HALT")
        .unwrap();
    assert!(
        whole
            .mode()
            .unwrap()
            .contains(TermMode::ALT_SCREEN | TermMode::BRACKETED_PASTE | TermMode::APP_CURSOR)
    );
    assert_eq!(whole.grid().unwrap()[Line(0)][Column(0)].c, 'A');
    whole.advance(b"\x1b[?1049l").unwrap();
    assert!(!whole.mode().unwrap().contains(TermMode::ALT_SCREEN));
    assert_eq!(whole.grid().unwrap()[Line(0)][Column(0)].c, 'Z');
    whole.advance(b"\r\n1\r\n2\r\n3\r\n4\r\n5").unwrap();
    assert!(whole.grid().unwrap().history_size() > 0);
    assert!(whole.grid().unwrap().history_size() <= HISTORY);
    whole.scroll(Scroll::Top).unwrap();
    assert!(whole.content().unwrap().display_offset > 0);
    whole.scroll(Scroll::Bottom).unwrap();
    assert_eq!(whole.content().unwrap().display_offset, 0);
    whole
        .resize(Size {
            columns: NARROW_COLUMNS,
            ..size
        })
        .unwrap();
    assert_eq!(whole.grid().unwrap().columns(), usize::from(NARROW_COLUMNS));
    assert_eq!(
        whole.content().unwrap().display_iter.count(),
        usize::from(NARROW_COLUMNS) * usize::from(ROWS)
    );
    whole.reset_damage().unwrap();
    whole.advance(b"X").unwrap();
    match whole.damage().unwrap() {
        TermDamage::Full => {}
        TermDamage::Partial(lines) => assert!(lines.count() > 0),
    }
    let column = whole.grid().unwrap().cursor.point.column;
    assert!(whole.advance(&vec![b'Q'; limits.feed_bytes + 1]).is_err());
    assert_eq!(whole.grid().unwrap().cursor.point.column, column);
    assert!(
        whole
            .resize(Size {
                columns: u16::MAX,
                rows: u16::MAX
            })
            .is_err()
    );
    assert_eq!(whole.grid().unwrap().columns(), usize::from(NARROW_COLUMNS));
    assert!(TerminalCore::new(Size { columns: 1, ..size }, HISTORY, limits).is_err());
    assert!(TerminalCore::new(size, usize::MAX, limits).is_err());
    let mut count_limit = TerminalCore::new(
        size,
        HISTORY,
        Limits {
            effect_count: 1,
            ..limits
        },
    )
    .unwrap();
    assert!(count_limit.advance(b"\x07\x07").is_err());
    assert!(count_limit.content().is_err());
    assert!(count_limit.advance(b"later").is_err());
    let mut byte_limit = TerminalCore::new(
        size,
        HISTORY,
        Limits {
            effect_bytes: size_of::<Effect>() + 1,
            ..limits
        },
    )
    .unwrap();
    assert!(byte_limit.advance(b"\x1b]2;ab\x07").is_err());
    assert!(byte_limit.grid().is_err());
    let mut sync = TerminalCore::new(size, HISTORY, limits).unwrap();
    assert!(
        sync.advance(b"\x1b[?2026h\x1b]2;sync\x07synced")
            .unwrap()
            .is_empty()
    );
    assert_eq!(effects(sync.flush_sync().unwrap()), ["title:sync"]);
    assert_eq!(sync.grid().unwrap()[Line(0)][Column(0)].c, 's');
}
