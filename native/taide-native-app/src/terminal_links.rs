use std::ops::Range;
use std::sync::OnceLock;

use eframe::egui::Modifiers;
use regex::Regex;
use taide_model::error::{AppError, AppResult};
use taide_native_terminal::{CellFlags, Column, GridDimensions, Line, Point, TerminalCore};

pub const MAX_LINK_BYTES: usize = 64 * 1024;
const ROW_BYTES: usize = MAX_LINK_BYTES;
const WINDOW_BYTES: usize = ROW_BYTES * 4;
const URL_EXPANSION_UNITS: usize = 2048;
const WIDE_COLUMNS: usize = 2;
const PATH_PATTERN: &str = r#"(?:^|[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}'"`(\[<])((?:~|\.{1,2})?/?[A-Za-z0-9_\.@+-]+(?:/[A-Za-z0-9_\.@+-]+)*\.[A-Za-z0-9]{1,10})"#;
const SUFFIX_PATTERNS: [&str; 6] = [
    r"^:([0-9]+):([0-9]+)",
    r"^:([0-9]+)(?:-[0-9]+)?",
    r"^\(([0-9]+),([0-9]+)\)",
    r"^\(([0-9]+)\)",
    r"^#L([0-9]+)(?:-L?[0-9]+)?",
    r#"^["'], line ([0-9]+)"#,
];
const URL_PATTERN: &str = r#"(https?|HTTPS?):[/]{2}[^\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}"'!*(){}|\\\^<>`]*[^\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}"':,.!?{}|\\\^~\[\]`()<>]"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRange {
    pub start: Point,
    pub end: Point,
}

impl CellRange {
    pub fn contains(&self, point: Point) -> bool {
        self.start <= point && point <= self.end
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileLink {
    pub path: String,
    pub line: Option<f64>,
    pub column: Option<f64>,
    pub text: String,
    pub range: CellRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalLink {
    pub uri: String,
    pub range: CellRange,
}

struct Span {
    bytes: Range<usize>,
    range: CellRange,
}

pub struct Row {
    pub text: String,
    spans: Vec<Span>,
}

impl Row {
    fn range(&self, bytes: Range<usize>) -> Option<CellRange> {
        if bytes.is_empty() {
            return None;
        }
        let start = self
            .spans
            .iter()
            .find(|span| span.bytes.contains(&bytes.start))?;
        let end = self
            .spans
            .iter()
            .find(|span| span.bytes.contains(&(bytes.end - 1)))?;
        Some(CellRange {
            start: start.range.start,
            end: end.range.end,
        })
    }

    fn append(&mut self, row: Row) -> AppResult<()> {
        let offset = self.text.len();
        if row.text.len() > WINDOW_BYTES.saturating_sub(offset) {
            return Err(invalid(
                "native terminal link window exceeds its byte limit",
            ));
        }
        self.text.push_str(&row.text);
        self.spans.extend(row.spans.into_iter().map(|span| Span {
            bytes: (span.bytes.start + offset)..(span.bytes.end + offset),
            range: span.range,
        }));
        Ok(())
    }
}

struct Patterns {
    path: Regex,
    suffixes: Vec<Regex>,
    url: Regex,
}

impl Patterns {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            path: Regex::new(PATH_PATTERN)?,
            suffixes: SUFFIX_PATTERNS
                .iter()
                .map(|pattern| Regex::new(pattern))
                .collect::<Result<_, _>>()?,
            url: Regex::new(URL_PATTERN)?,
        })
    }
}

fn patterns() -> AppResult<&'static Patterns> {
    static PATTERNS: OnceLock<Result<Patterns, String>> = OnceLock::new();
    PATTERNS
        .get_or_init(|| Patterns::new().map_err(|error| error.to_string()))
        .as_ref()
        .map_err(|error| invalid(error.clone()))
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}

fn whitespace(character: char) -> bool {
    matches!(character, '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub fn read_row(core: &TerminalCore, line: Line) -> AppResult<Row> {
    let grid = core.grid()?;
    if line < grid.topmost_line() || line > grid.bottommost_line() {
        return Err(invalid("native terminal link row is outside the grid"));
    }
    let mut row = Row {
        text: String::new(),
        spans: Vec::new(),
    };
    let mut column = 0;
    while column < grid.columns() {
        let point = Point::new(line, Column(column));
        let cell = &grid[point];
        let width = if cell.flags.contains(CellFlags::WIDE_CHAR) {
            WIDE_COLUMNS
        } else {
            1
        };
        let start = row.text.len();
        let chars = std::iter::once(&cell.c).chain(cell.zerowidth().unwrap_or_default());
        for character in chars {
            if character.len_utf8() > ROW_BYTES.saturating_sub(row.text.len()) {
                return Err(invalid("native terminal link row exceeds its byte limit"));
            }
            row.text.push(*character);
        }
        row.spans.push(Span {
            bytes: start..row.text.len(),
            range: CellRange {
                start: point,
                end: Point::new(line, Column((column + width - 1).min(grid.columns() - 1))),
            },
        });
        column += width;
    }
    let length = row.text.trim_end_matches(whitespace).len();
    row.text.truncate(length);
    row.spans.retain(|span| span.bytes.start < length);
    for span in &mut row.spans {
        span.bytes.end = span.bytes.end.min(length);
    }
    Ok(row)
}

pub fn file_links(row: &Row) -> AppResult<Vec<FileLink>> {
    let patterns = patterns()?;
    let mut result = Vec::new();
    let mut offset = 0;
    while let Some(capture) = patterns.path.captures_at(&row.text, offset) {
        let Some(path) = capture.get(1) else { break };
        let after = &row.text[path.end()..];
        let suffix = patterns
            .suffixes
            .iter()
            .filter_map(|pattern| pattern.captures(after))
            .max_by_key(|capture| capture.get(0).map_or(0, |matched| matched.len()));
        let end = path.end()
            + suffix
                .as_ref()
                .and_then(|capture| capture.get(0))
                .map_or(0, |matched| matched.len());
        if let Some(range) = row.range(path.start()..end) {
            let number = |index| {
                suffix
                    .as_ref()
                    .and_then(|capture| capture.get(index))
                    .and_then(|token| token.as_str().parse::<f64>().ok())
            };
            result.push(FileLink {
                path: path.as_str().into(),
                line: number(1),
                column: number(2),
                text: row.text[path.start()..end].into(),
                range,
            });
        }
        offset = end;
        if result.len() == taide_terminal::service::MAX_LINK_CANDIDATES_PER_ROW {
            break;
        }
    }
    Ok(result)
}

fn wrapped(core: &TerminalCore, line: Line) -> AppResult<bool> {
    let grid = core.grid()?;
    Ok(line > grid.topmost_line()
        && grid[Line(line.0 - 1)][grid.last_column()]
            .flags
            .contains(CellFlags::WRAPLINE))
}

fn url_window(core: &TerminalCore, line: Line) -> AppResult<Row> {
    let current = read_row(core, line)?;
    let mut above = Vec::new();
    let mut top = line;
    let mut units = 0;
    if wrapped(core, line)? && !current.text.starts_with(' ') {
        while top > core.grid()?.topmost_line() && units < URL_EXPANSION_UNITS {
            top = Line(top.0 - 1);
            let row = read_row(core, top)?;
            units += row.text.encode_utf16().count();
            let stop = !wrapped(core, top)? || row.text.contains(' ');
            above.push(row);
            if stop {
                break;
            }
        }
    }
    let mut result = Row {
        text: String::new(),
        spans: Vec::new(),
    };
    for row in above.into_iter().rev() {
        result.append(row)?
    }
    result.append(current)?;
    let mut bottom = line;
    units = 0;
    while bottom < core.grid()?.bottommost_line() && units < URL_EXPANSION_UNITS {
        bottom = Line(bottom.0 + 1);
        if !wrapped(core, bottom)? {
            break;
        }
        let row = read_row(core, bottom)?;
        units += row.text.encode_utf16().count();
        let stop = row.text.contains(' ');
        result.append(row)?;
        if stop {
            break;
        }
    }
    Ok(result)
}

fn valid_http(uri: &str) -> bool {
    url::Url::parse(uri).is_ok_and(|parsed| matches!(parsed.scheme(), "http" | "https"))
}

pub fn external_at(core: &TerminalCore, point: Point) -> AppResult<Option<ExternalLink>> {
    let grid = core.grid()?;
    if point.line < grid.topmost_line()
        || point.line > grid.bottommost_line()
        || point.column.0 >= grid.columns()
    {
        return Ok(None);
    }
    if let Some(link) = grid[point]
        .hyperlink()
        .filter(|link| valid_http(link.uri()))
    {
        let mut start = point.column.0;
        let mut end = start;
        let same = |column| {
            grid[Point::new(point.line, Column(column))]
                .hyperlink()
                .is_some_and(|candidate| candidate == link)
        };
        while start > 0 && same(start - 1) {
            start -= 1
        }
        while end + 1 < grid.columns() && same(end + 1) {
            end += 1
        }
        return Ok(Some(ExternalLink {
            uri: link.uri().into(),
            range: CellRange {
                start: Point::new(point.line, Column(start)),
                end: Point::new(point.line, Column(end)),
            },
        }));
    }
    let window = url_window(core, point.line)?;
    for matched in patterns()?.url.find_iter(&window.text) {
        let Ok(parsed) = url::Url::parse(matched.as_str()) else {
            continue;
        };
        let base = format!("{}://{}", parsed.scheme(), parsed.authority());
        if !matched
            .as_str()
            .to_lowercase()
            .starts_with(&base.to_lowercase())
        {
            continue;
        }
        if let Some(range) = window.range(matched.range())
            && range.contains(point)
        {
            let start = if range.start.line == point.line {
                range.start.column.0
            } else {
                0
            };
            let end = if range.end.line == point.line {
                range.end.column.0
            } else {
                grid.columns() - 1
            };
            if (start..=end).any(|column| {
                grid[Point::new(point.line, Column(column))]
                    .hyperlink()
                    .is_some_and(|link| valid_http(link.uri()))
            }) {
                continue;
            }
            return Ok(Some(ExternalLink {
                uri: matched.as_str().into(),
                range,
            }));
        }
    }
    Ok(None)
}

pub fn should_activate(modifiers: Modifiers, is_mac: bool) -> bool {
    modifiers.alt
        || if is_mac {
            modifiers.mac_cmd
        } else {
            modifiers.ctrl
        }
}
