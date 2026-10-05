use std::sync::OnceLock;

use chrono::{Local, TimeZone};
use regex::Regex;
use taide_model::error::AppResult;

use crate::preview::invalid;
use crate::preview_spreadsheet::{
    Cell, MAX_DECODED_BYTES, MAX_EXCEL_COLUMNS, MAX_EXCEL_ROWS, MAX_PREVIEW_ROWS, MAX_SHEETS,
    Sheet, Workbook,
};

const OPENING_UNITS: usize = 1024;
const HTML_TAGS: [&str; 7] = ["html", "table", "head", "meta", "script", "style", "div"];
const MAX_MERGES: usize = MAX_DECODED_BYTES / (size_of::<Merge>() * 2);
const TABLE_PATTERN: &str = r"(?i)<table[\s\S]*?>[\s\S]*?</table>";
const ROW_PATTERN: &str = r"(?i)<tr[^>]*>";
const CLOSE_CELL_PATTERN: &str = r"(?i)</t[dh]>";
const OPEN_CELL_PATTERN: &str = r"(?i)<t[dh]";
const ATTRIBUTE_PATTERN: &str = r#"([^"\s?>/]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^'">\s]+))"#;
const COMMENT_PATTERN: &str = r"<!--[^\r\n\u{2028}\u{2029}]*?-->";
const SPACE_PATTERN: &str = r"[\t\n\r ]+";
const AFTER_TAG_PATTERN: &str = r">\s+";
const BEFORE_TAG_PATTERN: &str = r"\s+<";
const BREAK_PATTERN: &str = r"<\s*[bB][rR]\s*/?>";
const STRIP_TAG_PATTERN: &str = r"<[^>]*>";
const QUOTED_PATTERN: &str = r#""[^\r\n\u{2028}\u{2029}]*?""#;
const ENTITY_PATTERNS: [(&str, &str); 7] = [
    (r"(?i)&nbsp;", " "),
    (r"(?i)&middot;", "·"),
    (r"(?i)&quot;", "\""),
    (r"(?i)&apos;", "'"),
    (r"(?i)&gt;", ">"),
    (r"(?i)&lt;", "<"),
    (r"(?i)&amp;", "&"),
];

struct Patterns {
    table: Regex,
    row: Regex,
    close_cell: Regex,
    open_cell: Regex,
    attributes: Regex,
    comments: Regex,
    space: Regex,
    after_tag: Regex,
    before_tag: Regex,
    line_break: Regex,
    strip_tag: Regex,
    quoted: Regex,
    entities: Vec<(Regex, &'static str)>,
}

impl Patterns {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            table: Regex::new(TABLE_PATTERN)?,
            row: Regex::new(ROW_PATTERN)?,
            close_cell: Regex::new(CLOSE_CELL_PATTERN)?,
            open_cell: Regex::new(OPEN_CELL_PATTERN)?,
            attributes: Regex::new(ATTRIBUTE_PATTERN)?,
            comments: Regex::new(COMMENT_PATTERN)?,
            space: Regex::new(SPACE_PATTERN)?,
            after_tag: Regex::new(AFTER_TAG_PATTERN)?,
            before_tag: Regex::new(BEFORE_TAG_PATTERN)?,
            line_break: Regex::new(BREAK_PATTERN)?,
            strip_tag: Regex::new(STRIP_TAG_PATTERN)?,
            quoted: Regex::new(QUOTED_PATTERN)?,
            entities: ENTITY_PATTERNS
                .iter()
                .map(|(pattern, value)| Ok((Regex::new(pattern)?, *value)))
                .collect::<Result<_, regex::Error>>()?,
        })
    }

    fn text(&self, raw: &str) -> String {
        let trimmed = raw.trim_matches(['\t', '\n', '\r', ' ']);
        let text = self.after_tag.replace_all(trimmed, ">");
        let text = self.before_tag.replace_all(&text, "<");
        let text = self.space.replace_all(&text, " ");
        let text = self.line_break.replace_all(&text, "\n");
        let text = self.strip_tag.replace_all(&text, "");
        let mut text = text.into_owned();
        for (pattern, value) in &self.entities {
            text = pattern.replace_all(&text, *value).into_owned();
        }
        text
    }
}

fn patterns() -> AppResult<&'static Patterns> {
    static PATTERNS: OnceLock<Result<Patterns, String>> = OnceLock::new();
    PATTERNS
        .get_or_init(|| Patterns::new().map_err(|error| error.to_string()))
        .as_ref()
        .map_err(|error| invalid(error.clone()))
}

pub(crate) fn is_html(source: &str) -> AppResult<bool> {
    let opening = source
        .trim_start_matches('\u{feff}')
        .chars()
        .take(OPENING_UNITS)
        .collect::<String>()
        .to_ascii_lowercase();
    let opening = patterns()?.quoted.replace_all(&opening, "");
    if opening.contains("<?xml") {
        return Ok(false);
    }
    Ok(HTML_TAGS
        .iter()
        .any(|tag| opening.contains(&format!("<{tag}"))))
}

#[derive(Default)]
struct Attributes {
    column_span: u32,
    row_span: u32,
    kind: String,
}

fn attributes(patterns: &Patterns, raw: &str) -> AppResult<Attributes> {
    let mut attrs = Attributes {
        column_span: 1,
        row_span: 1,
        ..Default::default()
    };
    let mut kind = String::new();
    let mut data_kind = String::new();
    for capture in patterns.attributes.captures_iter(raw) {
        let name = capture[1].to_ascii_lowercase();
        let value = capture
            .get(2)
            .or_else(|| capture.get(3))
            .or_else(|| capture.get(4))
            .ok_or_else(|| invalid("HTML spreadsheet attribute has no value"))?
            .as_str();
        match name.as_str() {
            "colspan" | "rowspan" => {
                let limit = if name == "colspan" {
                    MAX_EXCEL_COLUMNS
                } else {
                    MAX_EXCEL_ROWS
                };
                let value = value
                    .parse::<u32>()
                    .ok()
                    .filter(|value| *value > 0 && *value <= limit)
                    .ok_or_else(|| invalid("HTML spreadsheet span exceeds Excel bounds"))?;
                if name == "colspan" {
                    attrs.column_span = value;
                } else {
                    attrs.row_span = value;
                }
            }
            "t" => kind = value.into(),
            "data-t" => data_kind = value.into(),
            _ => (),
        }
    }
    attrs.kind = if kind.is_empty() { data_kind } else { kind };
    Ok(attrs)
}

struct Merge {
    first_row: u32,
    last_row: u32,
    first_column: u32,
    last_column: u32,
}

fn cell(text: String, kind: &str, timezone: &impl TimeZone) -> Cell {
    if text.trim().is_empty() || kind == "s" {
        return Cell::Text(text);
    }
    match text.as_str() {
        "TRUE" => Cell::Boolean(true),
        "FALSE" => Cell::Boolean(false),
        _ => crate::preview_spreadsheet_csv::fuzzy_number(&text)
            .or_else(|| crate::preview_spreadsheet_csv::fuzzy_date(&text, timezone))
            .map_or_else(|| Cell::Text(text), Cell::Number),
    }
}

fn scan(
    table: &str,
    patterns: &Patterns,
    timezone: &impl TimeZone,
    mut visitor: impl FnMut(u32, u32, Cell) -> AppResult<()>,
) -> AppResult<()> {
    let table = patterns.comments.replace_all(table, "");
    let mut rows = patterns.row.find_iter(&table).peekable();
    let mut row = 0u32;
    let mut merges: Vec<Merge> = Vec::new();
    while let Some(opening) = rows.next() {
        if row >= MAX_EXCEL_ROWS {
            return Err(invalid("HTML spreadsheet row exceeds Excel bounds"));
        }
        merges.retain(|merge| merge.last_row >= row);
        let end = rows.peek().map_or(table.len(), |next| next.start());
        let content = table[opening.end()..end].trim();
        if !content.get(..3).is_some_and(|prefix| {
            prefix.eq_ignore_ascii_case("<td") || prefix.eq_ignore_ascii_case("<th")
        }) {
            row += 1;
            continue;
        }
        let mut column = 0u32;
        for part in patterns.close_cell.split(content) {
            let part = part.trim();
            if !patterns.open_cell.is_match(part) {
                continue;
            }
            let mut raw = part;
            while raw.starts_with('<') {
                let Some(end) = raw.find('>') else {
                    break;
                };
                raw = &raw[end + 1..];
            }
            let mut changed = true;
            while changed {
                changed = false;
                for merge in &merges {
                    if merge.first_column == column
                        && merge.first_row < row
                        && row <= merge.last_row
                    {
                        column = merge.last_column + 1;
                        changed = true;
                        break;
                    }
                }
            }
            let tag = part.split_once('>').map_or(part, |(tag, _)| tag);
            let attrs = attributes(patterns, tag)?;
            let end_column = column
                .checked_add(attrs.column_span)
                .filter(|end| *end <= MAX_EXCEL_COLUMNS)
                .ok_or_else(|| invalid("HTML spreadsheet column exceeds Excel bounds"))?;
            let end_row = row
                .checked_add(attrs.row_span)
                .filter(|end| *end <= MAX_EXCEL_ROWS)
                .ok_or_else(|| invalid("HTML spreadsheet row span exceeds Excel bounds"))?;
            if attrs.column_span > 1 || attrs.row_span > 1 {
                if merges.len() >= MAX_MERGES {
                    return Err(invalid("HTML spreadsheet merges exceed the budget"));
                }
                merges.push(Merge {
                    first_row: row,
                    last_row: end_row - 1,
                    first_column: column,
                    last_column: end_column - 1,
                });
            }
            if !raw.is_empty() {
                let text = patterns.text(raw);
                let value = if text.is_empty() {
                    Cell::Null
                } else {
                    cell(text, &attrs.kind, timezone)
                };
                visitor(row, column, value)?;
            }
            column = end_column;
        }
        row += 1;
    }
    Ok(())
}

pub fn decode(bytes: &[u8]) -> AppResult<Workbook> {
    decode_in_timezone(bytes, &Local)
}

pub fn decode_in_timezone(bytes: &[u8], timezone: &impl TimeZone) -> AppResult<Workbook> {
    let source = crate::preview_spreadsheet_xlml::source(bytes)?;
    let patterns = patterns()?;
    let table_count = patterns
        .table
        .find_iter(&source)
        .take(MAX_SHEETS + 1)
        .count();
    if table_count == 0 || table_count > MAX_SHEETS {
        return Err(invalid(
            "HTML spreadsheet table count is outside the bounds",
        ));
    }
    let mut workbook = Workbook {
        sheets: Vec::with_capacity(table_count),
    };
    let mut retained = workbook.sheets.capacity() * size_of::<Sheet>();
    for table in patterns.table.find_iter(&source) {
        if workbook.sheets.len() >= MAX_SHEETS {
            return Err(invalid("HTML spreadsheet has too many tables"));
        }
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        scan(table.as_str(), patterns, timezone, |row, column, _| {
            bounds = Some(match bounds {
                Some((first_row, last_row, first_column, last_column)) => (
                    first_row.min(row),
                    last_row.max(row),
                    first_column.min(column),
                    last_column.max(column),
                ),
                None => (row, row, column, column),
            });
            Ok(())
        })?;
        let (first_row, last_row, first_column, last_column) = bounds.unwrap_or_default();
        let total = if bounds.is_some() {
            (last_row - first_row + 1) as usize
        } else {
            0
        };
        let columns = if bounds.is_some() {
            (last_column - first_column + 1) as usize
        } else {
            0
        };
        let visible = total.min(MAX_PREVIEW_ROWS);
        let name = format!("Sheet{}", workbook.sheets.len() + 1);
        let grid_bytes = visible
            .checked_mul(columns)
            .and_then(|cells| cells.checked_mul(size_of::<Cell>()))
            .and_then(|bytes| bytes.checked_add(visible * size_of::<Vec<Cell>>() + name.capacity()))
            .ok_or_else(|| invalid("HTML spreadsheet projected grid overflows"))?;
        retained = retained
            .checked_add(grid_bytes)
            .filter(|bytes| *bytes <= MAX_DECODED_BYTES)
            .ok_or_else(|| invalid("HTML spreadsheet projected grid exceeds the budget"))?;
        let mut rows = vec![vec![Cell::Null; columns]; visible];
        scan(table.as_str(), patterns, timezone, |row, column, value| {
            let Some(row) = row.checked_sub(first_row).map(|row| row as usize) else {
                return Ok(());
            };
            let Some(column) = column
                .checked_sub(first_column)
                .map(|column| column as usize)
            else {
                return Ok(());
            };
            let Some(cell) = rows.get_mut(row).and_then(|row| row.get_mut(column)) else {
                return Ok(());
            };
            retained = retained
                .checked_add(value.text_bytes())
                .filter(|bytes| *bytes <= MAX_DECODED_BYTES)
                .ok_or_else(|| invalid("HTML spreadsheet retained text exceeds the budget"))?;
            *cell = value;
            Ok(())
        })?;
        workbook.sheets.push(Sheet {
            name,
            rows,
            total_row_count: total,
            truncated: total > MAX_PREVIEW_ROWS,
        });
    }
    if workbook.sheets.is_empty() {
        return Err(invalid("HTML spreadsheet has no complete table"));
    }
    if workbook.retained_bytes() > MAX_DECODED_BYTES {
        return Err(invalid(
            "HTML spreadsheet retained workbook exceeds the budget",
        ));
    }
    Ok(workbook)
}
