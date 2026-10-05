use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use taide_native_retained::RetainedBytes;

use crate::grid::{Dimensions, Grid};
use crate::index::Line;
use crate::term::cell::Cell;
use crate::vte::ansi::Rgb;

pub const MAX_COMMAND_BLOCKS: usize = 500;
const SIGNIFICAND_BITS: usize = 53;
const MAX_BINARY_EXPONENT: usize = 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, RetainedBytes)]
pub struct CommandColors {
    pub success: Option<Rgb>,
    pub failure: Option<Rgb>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, RetainedBytes)]
pub struct CommandDecoration {
    pub id: u64,
    pub line: Line,
    pub color: Rgb,
}

#[derive(Clone, Copy, Debug, PartialEq, RetainedBytes)]
pub struct CommandBlock {
    pub id: u64,
    pub start: Line,
    pub output: Option<Line>,
    pub end: Option<Line>,
    pub exit_code: Option<f64>,
    pub alternate: bool,
}

#[derive(RetainedBytes)]
pub(super) struct Marker {
    row: Arc<AtomicUsize>,
    alternate: bool,
}

impl Marker {
    pub(super) fn new(row: Arc<AtomicUsize>, alternate: bool) -> Self {
        Self { row, alternate }
    }

    fn alive(&self) -> bool {
        self.row.load(Ordering::Relaxed) != 0
    }

    fn key(&self) -> (bool, usize) {
        (self.alternate, Arc::as_ptr(&self.row) as usize)
    }
}

#[derive(RetainedBytes)]
struct Block {
    id: u64,
    start: Marker,
    output: Option<Marker>,
    end: Option<Marker>,
    exit_code: Option<f64>,
    color: Option<Rgb>,
}

#[derive(Default, RetainedBytes)]
pub(super) struct Commands {
    blocks: Vec<Block>,
    current: Option<u64>,
    sequence: u64,
    seen_output: bool,
    pub(super) colors: CommandColors,
    pub(super) failed: bool,
}

impl Commands {
    pub(super) fn prune(&mut self) {
        self.blocks.retain(|block| block.start.alive());
        if self
            .current
            .is_some_and(|current| !self.blocks.iter().any(|block| block.id == current))
        {
            self.current = None;
        }
    }

    pub(super) fn apply(&mut self, data: &[u8], marker: impl FnOnce() -> Marker) {
        self.prune();
        let mut params = data.split(|byte| *byte == b';');
        match params.next() {
            Some(b"A") => {
                let Some(id) = self.sequence.checked_add(1) else {
                    self.failed = true;
                    return;
                };
                self.sequence = id;
                if self.blocks.len() == MAX_COMMAND_BLOCKS {
                    self.blocks.remove(0);
                }
                self.blocks.push(Block {
                    id,
                    start: marker(),
                    output: None,
                    end: None,
                    exit_code: None,
                    color: None,
                });
                self.current = Some(id);
            }
            Some(b"C") => {
                let Some(block) = self
                    .blocks
                    .iter_mut()
                    .find(|block| Some(block.id) == self.current)
                else {
                    return;
                };
                block.output = Some(marker());
                self.seen_output = true;
            }
            Some(b"D") => {
                let Some(index) = self
                    .blocks
                    .iter()
                    .position(|block| Some(block.id) == self.current)
                else {
                    return;
                };
                self.current = None;
                if self.seen_output && self.blocks[index].output.is_none() {
                    self.blocks.remove(index);
                    return;
                }
                self.blocks[index].end = Some(marker());
                self.blocks[index].exit_code = parse_exit_code(params.next());
                self.blocks[index].color = self.blocks[index].exit_code.and_then(|code| {
                    if code == 0.0 {
                        self.colors.success
                    } else {
                        self.colors.failure
                    }
                });
            }
            _ => (),
        }
    }

    pub(super) fn decorations(
        &self,
        grid: &Grid<Cell>,
        range: std::ops::Range<Line>,
    ) -> Vec<CommandDecoration> {
        if !self
            .blocks
            .iter()
            .any(|block| block.color.is_some() && block.start.alive() && !block.start.alternate)
        {
            return Vec::new();
        }
        let mut lines = BTreeMap::new();
        for index in range.start.0.max(-(grid.history_size() as i32))
            ..range.end.0.min(grid.screen_lines() as i32)
        {
            if let Some(marker) = grid[Line(index)].native_marker_ref() {
                lines.insert(Arc::as_ptr(marker) as usize, Line(index));
            }
        }
        self.blocks
            .iter()
            .filter_map(|block| {
                if !block.start.alive() || block.start.alternate {
                    return None;
                }
                Some(CommandDecoration {
                    id: block.id,
                    line: *lines.get(&block.start.key().1)?,
                    color: block.color?,
                })
            })
            .collect()
    }

    pub(super) fn snapshot(
        &self,
        primary: &Grid<Cell>,
        alternate: &Grid<Cell>,
    ) -> Vec<CommandBlock> {
        if self.blocks.is_empty() {
            return Vec::new();
        }
        let mut positions = BTreeMap::new();
        for (is_alternate, grid) in [(false, primary), (true, alternate)] {
            for index in -(grid.history_size() as i32)..grid.screen_lines() as i32 {
                if let Some(marker) = grid[Line(index)].native_marker_ref() {
                    positions.insert((is_alternate, Arc::as_ptr(marker) as usize), Line(index));
                }
            }
        }
        let position = |marker: &Marker| {
            marker
                .alive()
                .then(|| positions.get(&marker.key()).copied())
                .flatten()
        };
        self.blocks
            .iter()
            .filter_map(|block| {
                Some(CommandBlock {
                    id: block.id,
                    start: position(&block.start)?,
                    output: block.output.as_ref().and_then(position),
                    end: block.end.as_ref().and_then(position),
                    exit_code: block.exit_code,
                    alternate: block.start.alternate,
                })
            })
            .collect()
    }
}

fn is_js_whitespace(value: char) -> bool {
    matches!(value, '\u{9}'..='\u{d}' | '\u{20}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn parse_exit_code(raw: Option<&[u8]>) -> Option<f64> {
    let raw = std::str::from_utf8(raw?)
        .ok()?
        .trim_matches(is_js_whitespace);
    if raw.is_empty() {
        return Some(0.0);
    }
    let value = match raw.as_bytes().get(..2) {
        Some(b"0x" | b"0X") => parse_binary_radix(&raw[2..], 16, 4)?,
        Some(b"0o" | b"0O") => parse_binary_radix(&raw[2..], 8, 3)?,
        Some(b"0b" | b"0B") => parse_binary_radix(&raw[2..], 2, 1)?,
        _ => {
            if !raw.bytes().all(|byte| {
                byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-')
            }) {
                return None;
            }
            raw.parse::<f64>().ok()?
        }
    };
    (value.is_finite() && value.fract() == 0.0).then_some(value)
}

fn parse_binary_radix(raw: &str, radix: u32, width: usize) -> Option<f64> {
    if raw.is_empty() {
        return None;
    }
    let mut significand = 0_u64;
    let mut bits = 0_usize;
    let mut round = false;
    let mut sticky = false;
    for digit in raw.chars() {
        let digit = digit.to_digit(radix)?;
        for bit in (0..width).rev() {
            let bit = u64::from((digit >> bit) & 1);
            if bits == 0 && bit == 0 {
                continue;
            }
            if bits < SIGNIFICAND_BITS {
                significand = (significand << 1) | bit;
            } else if bits == SIGNIFICAND_BITS {
                round = bit != 0;
            } else {
                sticky |= bit != 0;
            }
            bits += 1;
        }
    }
    if bits > MAX_BINARY_EXPONENT {
        return None;
    }
    if round && (sticky || significand & 1 != 0) {
        significand += 1;
    }
    let exponent = i32::try_from(bits.saturating_sub(SIGNIFICAND_BITS)).ok()?;
    Some(significand as f64 * 2_f64.powi(exponent))
}
