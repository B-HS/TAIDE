use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use alacritty_terminal::{
    event::{Event, VoidListener, WindowSize},
    grid::Dimensions,
    index::{Column, Line},
    term::{
        Config, Term,
        cell::{Cell, Hyperlink},
    },
    vte::ansi::{Handler, KeyboardModes, Processor, Rgb, StdSyncHandler, StreamObserver},
};
use taide_native_retained::{Error, Limits as RetainedLimits, RetainedBytes, measure};
use taide_native_terminal::{Effect, Limits, Size, TerminalCore};

const COLUMNS: u16 = 20;
const ROWS: u16 = 4;
const CACHE_ROWS: u16 = 32;
const HISTORY: usize = 8;
const STRING_CAPACITY: usize = 8192;
const TITLE_BYTES: usize = 2048;
const STACK_DEPTH: usize = 4096;
const HANDOFF_MARGIN: usize = 1024;
const SYNC_BUFFER_BYTES: usize = 2 * 1024 * 1024;
const PRIVATE_MARKER: &str = "synthetic-private-marker";

static LEAKED: AtomicBool = AtomicBool::new(false);
static LOGGER: LeakCheck = LeakCheck;

struct LeakCheck;

impl log::Log for LeakCheck {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        if record.args().to_string().contains(PRIVATE_MARKER) {
            LEAKED.store(true, Ordering::Relaxed);
        }
    }

    fn flush(&self) {}
}

fn report<T: RetainedBytes>(value: &T) -> taide_native_retained::Report {
    measure(
        value,
        RetainedLimits {
            bytes: Limits::default().retained_bytes,
            visits: Limits::default().retained_visits,
        },
    )
    .unwrap()
}

#[derive(Debug)]
struct UnknownObserver;

impl StreamObserver for UnknownObserver {
    fn observe(&mut self, _: &[&[u8]], _: bool) {}
}

#[test]
fn native_trace는_title_link_dcs와_cursor_payload를_노출하지_않는다() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    let mut core = TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap();
    let fixture = format!(
        "\x1b]2;{PRIVATE_MARKER}\x07\x1b[22;0t\x1b[23;0t\x1b]8;id={PRIVATE_MARKER};https://example.invalid/{PRIVATE_MARKER}\x07x\x1b]22;{PRIVATE_MARKER}\x07\x1bPq{PRIVATE_MARKER}\x1b\\"
    );
    core.advance(fixture.as_bytes()).unwrap();
    assert!(!LEAKED.load(Ordering::Relaxed));
    log::set_max_level(log::LevelFilter::Off);
}

#[test]
fn keyboard_stack의_상한은_title_stack을_건드리지_않는다() {
    let mut term = Term::new(
        Config {
            kitty_keyboard: true,
            ..Config::default()
        },
        &Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        VoidListener,
    );
    let before = report(&term).bytes;
    for _ in 0..=STACK_DEPTH {
        term.push_keyboard_mode(KeyboardModes::DISAMBIGUATE_ESC_CODES);
    }
    assert_eq!(
        report(&term).bytes,
        before + STACK_DEPTH * size_of::<KeyboardModes>()
    );
    term.pop_keyboard_modes(u16::try_from(STACK_DEPTH).unwrap());
    term.push_keyboard_mode(KeyboardModes::NO_MODE);
}

#[test]
fn 실제_graph는_cache_shared_parser_query와_retained_실패를_계산한다() {
    let mut uri = String::with_capacity(STRING_CAPACITY);
    uri.push_str("https://example.invalid/synthetic");
    let mut cell = Cell::default();
    cell.set_hyperlink(Some(Hyperlink::new(Some("synthetic"), uri)));
    cell.push_zerowidth('\u{301}');
    let single = report(&cell);
    assert_eq!(single.shared_allocations, 2);
    assert!(single.bytes > STRING_CAPACITY);
    let cells = vec![cell.clone(), cell.clone()];
    let duplicate = report(&cells);
    assert_eq!(duplicate.shared_allocations, single.shared_allocations);
    assert_eq!(
        duplicate.bytes,
        single.bytes - size_of::<Cell>()
            + size_of::<Vec<Cell>>()
            + cells.capacity() * size_of::<Cell>()
    );

    let mut config = Config {
        scrolling_history: HISTORY,
        ..Config::default()
    };
    config.semantic_escape_chars.reserve(STRING_CAPACITY);
    let mut term = Term::new(
        config,
        &Size {
            columns: COLUMNS,
            rows: CACHE_ROWS,
        },
        VoidListener,
    );
    let allocated = report(&term).bytes;
    let visible_cells = usize::from(COLUMNS) * usize::from(CACHE_ROWS) * size_of::<Cell>();
    term.resize(Size {
        columns: COLUMNS,
        rows: ROWS,
    });
    assert_eq!(term.grid().total_lines(), usize::from(ROWS));
    assert!(report(&term).bytes >= visible_cells * 2);
    assert!(report(&term).bytes >= allocated);

    term.grid_mut()[Line(0)][Column(0)] = cell.clone();
    let primary = report(&term);
    term.swap_alt();
    assert!(term.grid()[Line(0)][Column(0)].extra.is_none());
    assert_eq!(report(&term).shared_allocations, primary.shared_allocations);
    term.set_title(Some("s".repeat(TITLE_BYTES)));
    let titled = report(&term).bytes;
    term.push_title();
    assert!(report(&term).bytes >= titled + TITLE_BYTES);

    let mut parser = Processor::<StdSyncHandler>::new();
    let initial = report(&parser).bytes;
    assert!(initial >= SYNC_BUFFER_BYTES);
    parser.advance(&mut term, b"\x1b]2;partial");
    assert!(report(&parser).bytes > initial);
    parser.advance(&mut term, b"\x07\x1b[?2026hsynchronized");
    assert!(report(&parser).bytes > initial);
    parser.set_stream_observer(Some(Box::new(UnknownObserver)));
    assert_eq!(
        measure(
            &parser,
            RetainedLimits {
                bytes: usize::MAX,
                visits: usize::MAX
            }
        ),
        Err(Error::Opaque)
    );
    let opaque = Event::ColorRequest(0, Arc::new(|_| String::new()));
    assert_eq!(
        measure(
            &opaque,
            RetainedLimits {
                bytes: usize::MAX,
                visits: usize::MAX
            }
        ),
        Err(Error::Opaque)
    );

    let size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    let mut core = TerminalCore::new(size, HISTORY, Limits::default()).unwrap();
    let baseline = core.retained().unwrap();
    assert_eq!(baseline.shared_allocations, 1);
    let outcome = core
        .advance_outcome(b"\x1b]4;1;?\x07\x1b]10;?\x1b\\\x1b[14t")
        .unwrap();
    assert_eq!(outcome.effects.len(), 3);
    let color = Rgb { r: 1, g: 2, b: 3 };
    match &outcome.effects[0] {
        Effect::Terminal(Event::NativeColorRequest(index, query)) => {
            assert_eq!(*index, 1);
            assert_eq!(query.reply(color), "\x1b]4;1;rgb:0101/0202/0303\x07");
        }
        _ => panic!("missing typed indexed color query"),
    }
    match &outcome.effects[1] {
        Effect::Terminal(Event::NativeColorRequest(_, query)) => {
            assert_eq!(query.reply(color), "\x1b]10;rgb:0101/0202/0303\x1b\\")
        }
        _ => panic!("missing typed dynamic color query"),
    }
    match &outcome.effects[2] {
        Effect::Terminal(Event::NativeTextAreaSizeRequest(query)) => assert_eq!(
            query.reply(WindowSize {
                num_lines: u16::MAX,
                num_cols: u16::MAX,
                cell_width: u16::MAX,
                cell_height: u16::MAX
            }),
            "\x1b[4;4294836225;4294836225t"
        ),
        _ => panic!("missing typed pixel query"),
    }
    assert!(report(&outcome).bytes > size_of_val(&outcome));
    assert!(
        !format!(
            "{:?} {:?}",
            Event::Title("synthetic-private".into()),
            Event::PtyWrite("synthetic-private".into())
        )
        .contains("synthetic-private")
    );

    assert!(
        TerminalCore::new(
            size,
            HISTORY,
            Limits {
                retained_bytes: baseline.bytes - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        TerminalCore::new(
            size,
            HISTORY,
            Limits {
                retained_visits: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let limited = Limits {
        retained_bytes: baseline.bytes + HANDOFF_MARGIN,
        ..Limits::default()
    };
    let mut growing = TerminalCore::new(size, HISTORY, limited).unwrap();
    let large_link = format!(
        "\x1b]8;id=synthetic;https://example.invalid/{}\x07x",
        "s".repeat(TITLE_BYTES)
    );
    assert!(growing.advance_outcome(large_link.as_bytes()).is_err());
    assert!(growing.grid().is_err());
    assert!(growing.advance(b"later").is_err());
    assert!(growing.retained().unwrap().bytes < baseline.bytes);
    let mut resized = TerminalCore::new(size, HISTORY, limited).unwrap();
    assert!(
        resized
            .resize(Size {
                columns: COLUMNS,
                rows: CACHE_ROWS
            })
            .is_err()
    );
    assert!(resized.content().is_err());
    let mut query_limit = TerminalCore::new(
        size,
        HISTORY,
        Limits {
            effect_bytes: size_of::<Effect>(),
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(query_limit.advance(b"\x1b]4;1;?\x07").is_err());
    assert!(query_limit.grid().is_err());
}
