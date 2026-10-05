use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Osc52, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

#[path = "../../../native/taide-native-terminal/src/input.rs"]
pub mod input;

#[path = "osc-effects.rs"]
pub mod osc_effects;

#[path = "../../../native/taide-native-terminal/src/stream.rs"]
mod normalized_stream;

pub const COLUMNS: usize = 80;
pub const SCREEN_LINES: usize = 24;
pub const HISTORY_LIMIT: usize = 128;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    pub titles: Vec<String>,
    pub clipboard_requests: usize,
    pub pty_writes: Vec<String>,
}

#[derive(Clone, Default)]
struct Listener(Arc<Mutex<Effects>>, bool);

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let mut effects = self.0.lock().unwrap_or_else(|error| error.into_inner());
        match event {
            Event::Title(title) if self.1 => effects.titles.push(title),
            Event::ClipboardStore(..) | Event::ClipboardLoad(..) => effects.clipboard_requests += 1,
            Event::PtyWrite(bytes) if self.1 => effects.pty_writes.push(bytes),
            _ => {}
        }
    }
}

struct Size;

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        SCREEN_LINES
    }
    fn screen_lines(&self) -> usize {
        SCREEN_LINES
    }
    fn columns(&self) -> usize {
        COLUMNS
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub lines: Vec<String>,
    pub history: Vec<String>,
    pub cursor_column: usize,
    pub cursor_line: i32,
    pub is_alternate: bool,
    pub is_bracketed_paste: bool,
    pub is_application_cursor: bool,
    pub effects: Effects,
}

pub struct AlacrittyProbe {
    term: Term<Listener>,
    parser: Processor,
    listener: Listener,
}

impl Default for AlacrittyProbe {
    fn default() -> Self {
        Self::new_with_payload_capture(true)
    }
}

impl AlacrittyProbe {
    pub fn encode_input(
        &self,
        input: input::NativeInput<'_>,
        capacity: usize,
    ) -> Result<input::InputAction, input::InputError> {
        input::encode_input(*self.term.mode(), input, capacity)
    }

    fn new_with_payload_capture(should_capture_payload: bool) -> Self {
        let listener = Listener(Arc::default(), should_capture_payload);
        let config = Config {
            scrolling_history: HISTORY_LIMIT,
            osc52: Osc52::Disabled,
            ..Default::default()
        };
        Self {
            term: Term::new(config, &Size, listener.clone()),
            parser: Processor::new(),
            listener,
        }
    }
    pub fn advance(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn snapshot(&self) -> Snapshot {
        let grid = self.term.grid();
        let read_line = |line| {
            let mut text = String::new();
            for column in 0..grid.columns() {
                let cell = &grid[Line(line)][Column(column)];
                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    continue;
                }
                text.push(cell.c);
                if let Some(extra) = cell.zerowidth() {
                    text.extend(extra.iter().copied());
                }
            }
            text.trim_end().to_string()
        };
        let effects = self
            .listener
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        Snapshot {
            lines: (0..grid.screen_lines())
                .map(|line| read_line(i32::try_from(line).expect("fixture row fits i32")))
                .collect(),
            history: (grid.topmost_line().0..0).map(read_line).collect(),
            cursor_column: grid.cursor.point.column.0,
            cursor_line: grid.cursor.point.line.0,
            is_alternate: self.term.mode().contains(TermMode::ALT_SCREEN),
            is_bracketed_paste: self.term.mode().contains(TermMode::BRACKETED_PASTE),
            is_application_cursor: self.term.mode().contains(TermMode::APP_CURSOR),
            effects,
        }
    }
}

pub const FIXTURES: &[(&str, &str)] = &[
    ("unicode", "ASCII 한글 日本語 中文 e\u{301}\r\nnext"),
    ("overwrite", "abcdef\rXY\x1b[K"),
    ("cursor", "old\x1b[2J\x1b[Hnew\x1b[3;5Hplaced"),
    ("alternate", "primary\x1b[?1049halt\x1b[?1049l"),
    ("modes", "\x1b[?2004h\x1b[?1h\x1b[?1006h"),
    (
        "osc",
        "\x1b]2;fixture-title\x07\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\",
    ),
    (
        "clipboard-denied",
        "\x1b]52;c;c3ludGhldGlj\x07\x1b]52;c;?\x1b\\safe",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::index::{Direction, Point, Side};
    use alacritty_terminal::selection::{Selection, SelectionType};
    use alacritty_terminal::term::TermDamage;
    use alacritty_terminal::term::search::RegexSearch;
    use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

    struct ResizedSize {
        columns: usize,
        lines: usize,
    }

    impl Dimensions for ResizedSize {
        fn total_lines(&self) -> usize {
            self.lines
        }
        fn screen_lines(&self) -> usize {
            self.lines
        }
        fn columns(&self) -> usize {
            self.columns
        }
    }

    #[test]
    fn 추가_sgr_wide_cell_link는_분할_입력에서도_보존된다() {
        const STYLED_CELLS: usize = 3;
        let fixture = "\x1b[1;4;38;2;1;2;3;48;2;4;5;6mred\x1b[0m한\x1b]8;id=fixture;https://example.com\x1b\\link\x1b]8;;\x1b\\";
        let mut whole = AlacrittyProbe::default();
        whole.advance(fixture.as_bytes());
        let mut fragmented = AlacrittyProbe::default();
        for byte in fixture.as_bytes() {
            fragmented.advance(std::slice::from_ref(byte));
        }
        for column in 0..COLUMNS {
            assert_eq!(
                whole.term.grid()[Line(0)][Column(column)],
                fragmented.term.grid()[Line(0)][Column(column)]
            );
        }
        for column in 0..STYLED_CELLS {
            let cell = &whole.term.grid()[Line(0)][Column(column)];
            assert_eq!(cell.fg, Color::Spec(Rgb { r: 1, g: 2, b: 3 }));
            assert_eq!(cell.bg, Color::Spec(Rgb { r: 4, g: 5, b: 6 }));
            assert!(cell.flags.contains(Flags::BOLD | Flags::UNDERLINE));
        }
        let wide = &whole.term.grid()[Line(0)][Column(STYLED_CELLS)];
        assert_eq!(wide.c, '한');
        assert_eq!(wide.fg, Color::Named(NamedColor::Foreground));
        assert!(wide.flags.contains(Flags::WIDE_CHAR));
        assert!(
            whole.term.grid()[Line(0)][Column(STYLED_CELLS + 1)]
                .flags
                .contains(Flags::WIDE_CHAR_SPACER)
        );
        let linked = &whole.term.grid()[Line(0)][Column(STYLED_CELLS + 2)];
        assert_eq!(linked.hyperlink().unwrap().uri(), "https://example.com");
    }

    #[test]
    fn 추가_resize_reflow는_history와_화면의_전체_출력을_보존한다() {
        const NARROW_COLUMNS: usize = 40;
        const EXTRA_CHARS: usize = 17;
        let text = "x".repeat(COLUMNS * 2 + EXTRA_CHARS);
        let mut probe = AlacrittyProbe::default();
        probe.advance(text.as_bytes());
        let before = probe.snapshot();
        let before_text = [before.history.concat(), before.lines.concat()].concat();
        probe.term.resize(ResizedSize {
            columns: NARROW_COLUMNS,
            lines: SCREEN_LINES,
        });
        let narrow = probe.snapshot();
        assert_eq!(
            [narrow.history.concat(), narrow.lines.concat()].concat(),
            before_text,
            "history {} rows / visible {} rows",
            narrow.history.len(),
            narrow.lines.len()
        );
        assert!(probe.term.grid().cursor.point.column.0 < NARROW_COLUMNS);
        probe.term.resize(ResizedSize {
            columns: COLUMNS,
            lines: SCREEN_LINES,
        });
        let restored = probe.snapshot();
        assert_eq!(
            [restored.history.concat(), restored.lines.concat()].concat(),
            text
        );
        assert!(probe.term.grid().cursor.point.column.0 < COLUMNS);
    }

    #[test]
    fn 추가_unicode_selection_search와_partial_damage를_검사한다() {
        const SELECTED_END_COLUMN: usize = 9;
        let mut probe = AlacrittyProbe::default();
        probe.term.reset_damage();
        probe.advance("hello 한글".as_bytes());
        let mut selection = Selection::new(
            SelectionType::Simple,
            Point::new(Line(0), Column(0)),
            Side::Left,
        );
        selection.update(
            Point::new(Line(0), Column(SELECTED_END_COLUMN)),
            Side::Right,
        );
        probe.term.selection = Some(selection);
        assert_eq!(
            probe.term.selection_to_string().as_deref(),
            Some("hello 한글")
        );
        let mut search = RegexSearch::new("한글").unwrap();
        let found = probe
            .term
            .search_next(
                &mut search,
                Point::new(Line(0), Column(0)),
                Direction::Right,
                Side::Left,
                None,
            )
            .unwrap();
        assert_eq!(found.start().line, Line(0));
        let TermDamage::Partial(damage) = probe.term.damage() else {
            panic!("expected partial damage after initial reset");
        };
        let rows = damage.collect::<Vec<_>>();
        assert!(
            rows.iter()
                .any(|row| row.line == 0 && row.left == 0 && row.right >= SELECTED_END_COLUMN)
        );
    }

    #[test]
    fn 모든_byte_경계에서_분할해도_grid_mode_effect가_동일하다() {
        for (name, fixture) in FIXTURES {
            let bytes = fixture.as_bytes();
            let mut whole = AlacrittyProbe::default();
            whole.advance(bytes);
            let expected = whole.snapshot();
            for split in 0..=bytes.len() {
                let mut fragmented = AlacrittyProbe::default();
                fragmented.advance(&bytes[..split]);
                fragmented.advance(&bytes[split..]);
                assert_eq!(fragmented.snapshot(), expected, "{name}, split {split}");
            }
        }
    }

    #[test]
    fn 확정_화면과_주화면_복구_mode와_osc52_거부를_검사한다() {
        let snapshot = |input: &str| {
            let mut probe = AlacrittyProbe::default();
            probe.advance(input.as_bytes());
            probe.snapshot()
        };
        assert_eq!(
            snapshot(FIXTURES[0].1).lines[0],
            "ASCII 한글 日本語 中文 e\u{301}"
        );
        assert_eq!(snapshot(FIXTURES[1].1).lines[0], "XY");
        let cursor = snapshot(FIXTURES[2].1);
        assert_eq!(cursor.lines[0], "new");
        assert_eq!(cursor.lines[2], "    placed");
        assert_eq!((cursor.cursor_line, cursor.cursor_column), (2, 10));
        let alternate = snapshot(FIXTURES[3].1);
        assert_eq!(alternate.lines[0], "primary");
        assert!(!alternate.is_alternate);
        let modes = snapshot(FIXTURES[4].1);
        assert!(modes.is_bracketed_paste && modes.is_application_cursor);
        assert_eq!(snapshot(FIXTURES[5].1).effects.titles, ["fixture-title"]);
        let clipboard = snapshot(FIXTURES[6].1);
        assert_eq!(clipboard.effects.clipboard_requests, 0);
        assert_eq!(clipboard.lines[0], "safe");
    }

    #[test]
    fn history는_상한을_넘지_않고_최신_출력을_보존한다() {
        const BURST_LINES: usize = 1_000;
        let mut probe = AlacrittyProbe::default();
        for line in 0..BURST_LINES {
            probe.advance(format!("line-{line}\r\n").as_bytes());
        }
        let snapshot = probe.snapshot();
        assert_eq!(snapshot.history.len(), HISTORY_LIMIT);
        assert_eq!(snapshot.lines[SCREEN_LINES - 2], "line-999");
        assert_eq!(snapshot.lines[SCREEN_LINES - 1], "");
    }
}
