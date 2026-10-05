use alacritty_terminal::vte::Params;
use taide_infra::terminal_scan::TEXT_OVERLAP_BYTES;

pub const MAX_PENDING_TEXT_BYTES: usize = 65_536;
const DEFAULT_ROW: u32 = 1;
const SCREEN_SWITCH_MODES: &[u16] = &[47, 1047, 1049];

#[derive(Debug, Default, taide_native_retained::RetainedBytes)]
pub(super) struct NormalizedStream {
    text: String,
    tail: String,
    row: Option<u32>,
    is_overflowed: bool,
}

impl NormalizedStream {
    pub(super) fn print(&mut self, character: char) {
        if character < '\u{20}' || character == '\u{7f}' {
            return;
        }
        if character == ' ' {
            self.space();
            return;
        }
        self.push(character);
    }

    pub(super) fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.push('\n');
                self.row = self.row.map(|row| row.saturating_add(1));
            }
            b'\t' => self.space(),
            _ => {}
        }
    }

    pub(super) fn csi(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        is_ignored: bool,
        action: char,
    ) {
        if is_ignored {
            return;
        }
        let first = params
            .iter()
            .next()
            .and_then(|values| values.first())
            .copied();
        let row = u32::from(first.unwrap_or_default()).max(DEFAULT_ROW);
        match (action, intermediates) {
            ('G' | 'C', []) => self.space(),
            ('H' | 'f' | 'd', []) => {
                if self.row.is_none_or(|current| current == row) {
                    self.space();
                } else {
                    self.push('\n');
                }
                self.row = Some(row);
            }
            ('B' | 'E', []) => self.move_rows(i64::from(row)),
            ('A' | 'F', []) => self.move_rows(-i64::from(row)),
            ('h' | 'l', [b'?'])
                if params.iter().any(|values| {
                    values
                        .first()
                        .is_some_and(|value| SCREEN_SWITCH_MODES.contains(value))
                }) =>
            {
                self.row = None;
            }
            _ => {}
        }
    }

    pub(super) fn take(&mut self) -> Option<(String, String)> {
        if self.is_overflowed {
            return None;
        }
        let text = std::mem::take(&mut self.text);
        let overlap = std::mem::take(&mut self.tail);
        let mut joined = overlap.clone();
        joined.push_str(&text);
        let mut start = joined.len().saturating_sub(TEXT_OVERLAP_BYTES);
        while !joined.is_char_boundary(start) {
            start += 1;
        }
        self.tail = joined[start..].to_string();
        Some((text, overlap))
    }

    fn push(&mut self, character: char) {
        if self.is_overflowed {
            return;
        }
        if self.text.len() + character.len_utf8() > MAX_PENDING_TEXT_BYTES {
            self.text.clear();
            self.is_overflowed = true;
            return;
        }
        self.text.push(character);
    }

    fn space(&mut self) {
        if !self.text.ends_with(' ') {
            self.push(' ');
        }
    }

    fn move_rows(&mut self, delta: i64) {
        self.push('\n');
        self.row = self
            .row
            .and_then(|row| u32::try_from(i64::from(row) + delta).ok())
            .filter(|row| *row >= DEFAULT_ROW);
    }
}
