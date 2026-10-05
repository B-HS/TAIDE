use alacritty_terminal::{
    grid::{Dimensions, Grid},
    term::cell::Cell,
};
use taide_model::error::{AppError, AppResult};

use crate::{CellFlags, Column, Line, Point, SelectionRange};

const WORD_SEPARATORS: &str = " ()[]{}',\"`";
const WIDE_COLUMNS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionGranularity {
    Word,
    Line,
}

struct Word {
    start: Column,
    end: Column,
    whitespace: bool,
}

struct Scan<'grid> {
    grid: &'grid Grid<Cell>,
    remaining: usize,
}

fn plain_space(cell: &Cell) -> bool {
    matches!(cell.c, ' ' | '\t') && cell.zerowidth().is_none_or(|extra| extra.is_empty())
}

fn whitespace(cell: &Cell) -> bool {
    cell.c.is_whitespace() && cell.zerowidth().is_none_or(|extra| extra.is_empty())
}

fn separator(cell: &Cell) -> bool {
    !cell
        .flags
        .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        && cell.zerowidth().is_none_or(|extra| extra.is_empty())
        && (cell.c == '\t' || WORD_SEPARATORS.contains(cell.c))
}

impl<'grid> Scan<'grid> {
    fn cell(&mut self, point: Point) -> AppResult<&'grid Cell> {
        self.remaining = self.remaining.checked_sub(1).ok_or_else(|| {
            AppError::InvalidArgument("terminal selection bounds exceed their visit budget".into())
        })?;
        Ok(&self.grid[point])
    }

    fn literal_space(&mut self, point: Point) -> AppResult<bool> {
        let cell = self.cell(point)?;
        Ok(cell.c == ' ' && cell.flags.contains(CellFlags::NATIVE_CONTENT))
    }

    fn wrapped(&mut self, previous: Line) -> AppResult<bool> {
        Ok(self
            .cell(Point::new(previous, self.grid.last_column()))?
            .flags
            .contains(CellFlags::WRAPLINE))
    }

    fn word(&mut self, point: Point) -> AppResult<Word> {
        let mut start = point.column;
        if self
            .cell(point)?
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
        {
            start = Column(start.0.checked_sub(1).ok_or_else(|| {
                AppError::InvalidArgument("invalid terminal wide selection point".into())
            })?);
        }
        let first = self.cell(Point::new(point.line, start))?;
        let spaces = plain_space(first);
        let mut is_whitespace = whitespace(first);
        let mut end = start;
        if first.flags.contains(CellFlags::WIDE_CHAR) {
            end = Column((start.0 + WIDE_COLUMNS - 1).min(self.grid.last_column().0));
        }
        while start.0 > 0 {
            let mut previous = Column(start.0 - 1);
            if self
                .cell(Point::new(point.line, previous))?
                .flags
                .contains(CellFlags::WIDE_CHAR_SPACER)
            {
                previous = Column(previous.0.checked_sub(1).ok_or_else(|| {
                    AppError::InvalidArgument("invalid terminal wide selection point".into())
                })?);
            }
            let cell = self.cell(Point::new(point.line, previous))?;
            if (spaces && !plain_space(cell)) || (!spaces && separator(cell)) {
                break;
            }
            is_whitespace &= whitespace(cell);
            start = previous;
        }
        while end < self.grid.last_column() {
            let next = end + 1;
            let cell = self.cell(Point::new(point.line, next))?;
            if (spaces && !plain_space(cell)) || (!spaces && separator(cell)) {
                break;
            }
            is_whitespace &= whitespace(cell);
            end = next;
            if cell.flags.contains(CellFlags::WIDE_CHAR) {
                end = Column((next.0 + WIDE_COLUMNS - 1).min(self.grid.last_column().0));
            }
        }
        Ok(Word {
            start,
            end,
            whitespace: is_whitespace,
        })
    }
}

pub(crate) fn bounds(
    grid: &Grid<Cell>,
    point: Point,
    granularity: SelectionGranularity,
    visits: usize,
) -> AppResult<SelectionRange> {
    if point.line < grid.topmost_line()
        || point.line > grid.bottommost_line()
        || point.column >= Column(grid.columns())
    {
        return Err(AppError::InvalidArgument(
            "terminal selection point is outside its grid".into(),
        ));
    }
    let mut scan = Scan {
        grid,
        remaining: visits,
    };
    if granularity == SelectionGranularity::Line {
        let mut first = point.line;
        let mut last = point.line;
        while first > grid.topmost_line() && scan.wrapped(first - 1)? {
            first -= 1;
        }
        while last < grid.bottommost_line() && scan.wrapped(last)? {
            last += 1;
        }
        return Ok(SelectionRange::new(
            Point::new(first, Column(0)),
            Point::new(last, grid.last_column()),
            false,
        ));
    }
    let word = scan.word(point)?;
    let mut start = Point::new(point.line, word.start);
    let mut end = Point::new(point.line, word.end);
    while start.column == Column(0) && start.line > grid.topmost_line() {
        let previous = start.line - 1;
        let last = Point::new(previous, grid.last_column());
        if scan.literal_space(start)? || !scan.wrapped(previous)? || scan.literal_space(last)? {
            break;
        }
        let word = scan.word(last)?;
        if word.whitespace {
            break;
        }
        start = Point::new(previous, word.start);
    }
    while end.column == grid.last_column() && end.line < grid.bottommost_line() {
        let next = end.line + 1;
        let first = Point::new(next, Column(0));
        if scan.literal_space(end)? || !scan.wrapped(end.line)? || scan.literal_space(first)? {
            break;
        }
        let word = scan.word(first)?;
        if word.whitespace {
            break;
        }
        end = Point::new(next, word.end);
    }
    Ok(SelectionRange::new(start, end, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Size, TerminalCore};

    const COLUMNS: u16 = 3;
    const ROWS: u16 = 2;

    #[test]
    fn word_line_범위_검사는_방문_상한을_넘으면_부분_성공하지_않는다() {
        let mut core = TerminalCore::new(
            Size {
                columns: COLUMNS,
                rows: ROWS,
            },
            0,
            Default::default(),
        )
        .unwrap();
        core.advance(b"abcdef").unwrap();
        for granularity in [SelectionGranularity::Word, SelectionGranularity::Line] {
            assert!(
                bounds(
                    core.grid().unwrap(),
                    Point::new(Line(1), Column(0)),
                    granularity,
                    0
                )
                .is_err()
            );
        }
    }
}
