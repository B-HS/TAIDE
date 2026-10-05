use alacritty_terminal::{
    grid::{Dimensions, Grid},
    term::cell::Cell,
};
use taide_model::error::{AppError, AppResult};

use crate::{CellFlags, Column, Line, Point, Selection, SelectionRange, SelectionType, Side};

const WIDE_COLUMNS: usize = 2;
const GROWTH_FACTOR: usize = 2;
const INITIAL_BYTES: usize = 1024;
#[cfg(target_os = "windows")]
const LINE_ENDING: &str = "\r\n";
#[cfg(not(target_os = "windows"))]
const LINE_ENDING: &str = "\n";

struct Output {
    text: String,
    capacity: usize,
    visits: usize,
}

pub(crate) struct Slice {
    start: Point,
    end: Point,
    is_block: bool,
}

pub(crate) fn slice(grid: &Grid<Cell>, selection: &Selection) -> AppResult<Option<Slice>> {
    let is_block = match selection.ty {
        SelectionType::Simple => false,
        SelectionType::Block => true,
        _ => return Err(invalid("terminal selection type is not implemented")),
    };
    let boundary = |(point, side): (Point, Side)| {
        point
            .column
            .0
            .checked_add(usize::from(side == Side::Right))
            .map(|column| Point::new(point.line, Column(column)))
            .ok_or_else(|| invalid("terminal selection column overflow"))
    };
    let [start, end] = selection.native_anchors();
    let mut start = boundary(start)?;
    let mut end = boundary(end)?;
    if start > end {
        std::mem::swap(&mut start, &mut end);
    }
    if is_block && start.column > end.column {
        std::mem::swap(&mut start.column, &mut end.column);
    }
    if start == end || (is_block && start.column == end.column) || end.line < grid.topmost_line() {
        return Ok(None);
    }
    start.line = start.line.max(grid.topmost_line());
    Ok(Some(Slice {
        start,
        end,
        is_block,
    }))
}

pub(crate) fn range(grid: &Grid<Cell>, selection: &Selection) -> AppResult<Option<SelectionRange>> {
    let Some(slice) = slice(grid, selection)? else {
        return Ok(None);
    };
    let mut start = slice.start;
    let mut end = slice.end;
    if slice.is_block {
        start.column.0 = start.column.0.min(grid.columns());
        end.column.0 = end.column.0.saturating_sub(1).min(grid.columns() - 1);
        if start.column > end.column {
            return Ok(None);
        }
    } else {
        if start.column.0 >= grid.columns() {
            start.line = Line(
                start
                    .line
                    .0
                    .checked_add(1)
                    .ok_or_else(|| invalid("terminal selection row overflow"))?,
            );
            start.column = Column(0);
        }
        if end.column == Column(0) {
            end.line = Line(
                end.line
                    .0
                    .checked_sub(1)
                    .ok_or_else(|| invalid("terminal selection row overflow"))?,
            );
            end.column = grid.last_column();
        } else {
            end.column.0 = (end.column.0 - 1).min(grid.columns() - 1);
        }
    }
    if end.line > grid.bottommost_line() {
        end.line = grid.bottommost_line();
        if !slice.is_block {
            end.column = grid.last_column();
        }
    }
    if start > end {
        return Ok(None);
    }
    Ok(Some(SelectionRange::new(start, end, slice.is_block)))
}

impl Output {
    fn visit(&mut self) -> AppResult<()> {
        self.visits = self
            .visits
            .checked_sub(1)
            .ok_or_else(|| invalid("terminal selection exceeds its visit budget"))?;
        Ok(())
    }

    fn push(&mut self, character: char) -> AppResult<()> {
        let character = match character {
            '\t' | '\u{a0}' => ' ',
            character => character,
        };
        let required = self
            .text
            .len()
            .checked_add(character.len_utf8())
            .filter(|length| *length <= self.capacity)
            .ok_or_else(|| invalid("terminal selection exceeds its byte budget"))?;
        if required > self.text.capacity() {
            let target = self
                .text
                .capacity()
                .max(1)
                .saturating_mul(GROWTH_FACTOR)
                .max(required)
                .min(self.capacity);
            self.text
                .try_reserve_exact(target - self.text.len())
                .map_err(|_| invalid("terminal selection allocation failed"))?;
            if self.text.capacity() > self.capacity {
                return Err(invalid(
                    "terminal selection allocation exceeds its byte budget",
                ));
            }
        }
        self.text.push(character);
        Ok(())
    }
}

fn invalid(message: &str) -> AppError {
    AppError::InvalidArgument(message.into())
}

pub(crate) fn text(
    grid: &Grid<Cell>,
    range: Slice,
    capacity: usize,
    visits: usize,
) -> AppResult<String> {
    let mut output = Output {
        text: String::with_capacity(capacity.min(INITIAL_BYTES)),
        capacity,
        visits,
    };
    if output.text.capacity() > capacity {
        return Err(invalid(
            "terminal selection allocation exceeds its byte budget",
        ));
    }
    for line in range.start.line.0..=range.end.line.0 {
        output.visit()?;
        if line != range.start.line.0
            && (range.is_block
                || Line(line - 1) < grid.topmost_line()
                || Line(line - 1) > grid.bottommost_line()
                || !grid[Line(line - 1)][grid.last_column()]
                    .flags
                    .contains(CellFlags::WRAPLINE))
        {
            for character in LINE_ENDING.chars() {
                output.push(character)?;
            }
        }
        if Line(line) < grid.topmost_line() || Line(line) > grid.bottommost_line() {
            continue;
        }
        let row = &grid[Line(line)];
        let mut trimmed_end = 0;
        for column in (0..grid.columns()).rev() {
            output.visit()?;
            let cell = &row[Column(column)];
            if cell.flags.contains(CellFlags::NATIVE_CONTENT)
                || (cell.c != ' ' && cell.c != '\t')
                || cell.zerowidth().is_some_and(|extra| !extra.is_empty())
            {
                let width = if cell.flags.contains(CellFlags::WIDE_CHAR) {
                    WIDE_COLUMNS
                } else {
                    1
                };
                trimmed_end = (column + width).min(grid.columns());
                break;
            }
        }
        let mut column = if range.is_block || line == range.start.line.0 {
            range.start.column.0
        } else {
            0
        };
        let end = if range.is_block || line == range.end.line.0 {
            range.end.column.0
        } else {
            grid.columns()
        }
        .min(trimmed_end);
        while column < end {
            output.visit()?;
            let cell = &row[Column(column)];
            output.push(cell.c)?;
            if let Some(extra) = cell.zerowidth() {
                for &character in extra {
                    output.visit()?;
                    output.push(character)?;
                }
            }
            column += if cell.flags.contains(CellFlags::WIDE_CHAR) {
                WIDE_COLUMNS
            } else {
                1
            };
        }
    }
    Ok(output.text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, Point, Selection, SelectionType, Side, Size, TerminalCore};

    const COLUMNS: u16 = 6;
    const ROWS: u16 = 4;
    const HISTORY: usize = 8;
    const CAPACITY: usize = 128;
    const VISITS: usize = 1024;

    fn selection(start: Point, end: Point, kind: SelectionType) -> Selection {
        let mut selected = Selection::new(kind, start, Side::Left);
        selected.update(end, Side::Right);
        selected.include_all();
        selected
    }

    #[test]
    fn selection_text은_actual_content_wrap_reflow_erase_wide_nfd와_byte_visit_상한을_보존한다() {
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            HISTORY,
            Limits::default(),
        )
        .unwrap();
        core.advance(b"ab  \r\n123456wrap\r\nx").unwrap();
        let selected = selection(
            Point::new(Line(0), Column(0)),
            Point::new(Line(2), Column(usize::from(COLUMNS) - 1)),
            SelectionType::Simple,
        );
        assert_eq!(
            core.selection_text(&selected, CAPACITY).unwrap().unwrap(),
            "ab  \n123456wrap"
        );
        let reverse = selection(
            Point::new(Line(2), Column(usize::from(COLUMNS) - 1)),
            Point::new(Line(0), Column(0)),
            SelectionType::Simple,
        );
        assert_eq!(
            core.selection_text(&reverse, CAPACITY).unwrap(),
            core.selection_text(&selected, CAPACITY).unwrap()
        );
        let block = selection(
            Point::new(Line(1), Column(1)),
            Point::new(Line(2), Column(2)),
            SelectionType::Block,
        );
        assert_eq!(
            core.selection_text(&block, CAPACITY).unwrap().unwrap(),
            "23\nra"
        );
        let range = slice(core.grid().unwrap(), &selected).unwrap().unwrap();
        assert!(text(core.grid().unwrap(), range, CAPACITY, 1).is_err());
        let range = slice(core.grid().unwrap(), &selected).unwrap().unwrap();
        assert!(text(core.grid().unwrap(), range, 1, VISITS).is_err());
        let row = selection(
            Point::new(Line(0), Column(0)),
            Point::new(Line(0), Column(usize::from(COLUMNS) - 1)),
            SelectionType::Simple,
        );
        core.resize(Size {
            columns: COLUMNS + 1,
            rows: ROWS,
        })
        .unwrap();
        assert_eq!(
            core.selection_text(&row, CAPACITY).unwrap().unwrap(),
            "ab  "
        );
        core.advance(b"\x1b[1;3H\x1b[K").unwrap();
        assert_eq!(core.selection_text(&row, CAPACITY).unwrap().unwrap(), "ab");
        core.advance("\x1b[2J\x1b[H한e\u{301}\u{a0}𐐀".as_bytes())
            .unwrap();
        let unicode = selection(
            Point::new(Line(0), Column(0)),
            Point::new(Line(0), Column(usize::from(COLUMNS))),
            SelectionType::Simple,
        );
        let expected = "한e\u{301} 𐐀";
        assert_eq!(
            core.selection_text(&unicode, expected.len())
                .unwrap()
                .unwrap(),
            expected
        );
        assert!(core.selection_text(&unicode, expected.len() - 1).is_err());
        let empty = Selection::new(
            SelectionType::Simple,
            Point::new(Line(0), Column(0)),
            Side::Left,
        );
        assert!(core.selection_text(&empty, CAPACITY).unwrap().is_none());
        let unsupported = Selection::new(
            SelectionType::Semantic,
            Point::new(Line(0), Column(0)),
            Side::Left,
        );
        assert!(core.selection_range(&unsupported).is_err());
        core.advance(b"\x1b[1;2HX").unwrap();
        assert_eq!(
            core.selection_text(&unicode, CAPACITY).unwrap().unwrap(),
            " Xe\u{301} 𐐀"
        );
        let report = core.retained().unwrap();
        assert!(report.bytes <= Limits::default().retained_bytes);
    }
}
