pub const UPSTREAM_REVISION: &str = "cab25161054c50fd6c705db4ceefef0f1e5a9575";

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use taide_terminal_core_spike::{COLUMNS, Effects, HISTORY_LIMIT, SCREEN_LINES, Snapshot};
use wezterm_term::color::ColorPalette;
use wezterm_term::{Alert, AlertHandler, Terminal, TerminalConfiguration, TerminalSize};

#[derive(Debug)]
struct Config;

impl TerminalConfiguration for Config {
    fn scrollback_size(&self) -> usize {
        HISTORY_LIMIT
    }
    fn color_palette(&self) -> ColorPalette {
        ColorPalette::default()
    }
}

#[derive(Clone, Default)]
struct ProbeEffects {
    events: Arc<Mutex<Effects>>,
    pty_bytes: Arc<Mutex<Vec<u8>>>,
}

impl AlertHandler for ProbeEffects {
    fn alert(&mut self, alert: Alert) {
        if let Alert::WindowTitleChanged(title) = alert {
            self.events
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .titles
                .push(title);
        }
    }
}

impl Write for ProbeEffects {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.pty_bytes
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct WeztermProbe {
    term: Terminal,
    effects: ProbeEffects,
}

impl Default for WeztermProbe {
    fn default() -> Self {
        let effects = ProbeEffects::default();
        let mut term = Terminal::new(
            TerminalSize {
                rows: SCREEN_LINES,
                cols: COLUMNS,
                ..Default::default()
            },
            Arc::new(Config),
            "TAIDE isolated spike",
            "0.1.0",
            Box::new(effects.clone()),
        );
        term.set_notification_handler(Box::new(effects.clone()));
        Self { term, effects }
    }
}

impl WeztermProbe {
    pub fn advance(&mut self, bytes: &[u8]) {
        self.term.advance_bytes(bytes);
    }

    pub fn snapshot(&self) -> Snapshot {
        let screen = self.term.screen();
        let visible_range =
            screen.phys_range(&(0..i64::try_from(SCREEN_LINES).expect("fixture height fits i64")));
        let read_lines = |range| {
            screen
                .lines_in_phys_range(range)
                .iter()
                .map(|line| line.as_str().trim_end().to_string())
                .collect()
        };
        let mut effects = self
            .effects
            .events
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let bytes = self
            .effects
            .pty_bytes
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !bytes.is_empty() {
            effects
                .pty_writes
                .push(String::from_utf8_lossy(&bytes).into_owned());
        }
        let cursor = self.term.cursor_pos();
        Snapshot {
            lines: read_lines(visible_range.clone()),
            history: read_lines(0..visible_range.start),
            cursor_column: cursor.x,
            cursor_line: i32::try_from(cursor.y).expect("fixture cursor fits i32"),
            is_alternate: self.term.is_alt_screen_active(),
            is_bracketed_paste: self.term.bracketed_paste_enabled(),
            is_application_cursor: self.term.application_cursor_keys_enabled(),
            effects,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taide_terminal_core_spike::{AlacrittyProbe, FIXTURES};

    #[test]
    fn 동일_fixture의_정규화된_화면_cursor_history_mode_effect를_비교한다() {
        for (name, fixture) in FIXTURES {
            let mut alacritty = AlacrittyProbe::default();
            let mut wezterm = WeztermProbe::default();
            alacritty.advance(fixture.as_bytes());
            wezterm.advance(fixture.as_bytes());
            assert_eq!(wezterm.snapshot(), alacritty.snapshot(), "fixture {name}");
        }
    }

    #[test]
    fn wezterm도_모든_byte_분할_경계에서_같은_결과를_낸다() {
        for (name, fixture) in FIXTURES {
            let bytes = fixture.as_bytes();
            let mut whole = WeztermProbe::default();
            whole.advance(bytes);
            let expected = whole.snapshot();
            for split in 0..=bytes.len() {
                let mut fragmented = WeztermProbe::default();
                fragmented.advance(&bytes[..split]);
                fragmented.advance(&bytes[split..]);
                assert_eq!(fragmented.snapshot(), expected, "{name}, split {split}");
            }
        }
    }

    #[test]
    fn burst_history는_두_후보에서_같고_상한이_유지된다() {
        const BURST_LINES: usize = 1_000;
        let mut alacritty = AlacrittyProbe::default();
        let mut wezterm = WeztermProbe::default();
        for line in 0..BURST_LINES {
            let bytes = format!("line-{line}\r\n");
            alacritty.advance(bytes.as_bytes());
            wezterm.advance(bytes.as_bytes());
        }
        assert_eq!(wezterm.snapshot(), alacritty.snapshot());
        assert_eq!(wezterm.snapshot().history.len(), HISTORY_LIMIT);
    }
}
