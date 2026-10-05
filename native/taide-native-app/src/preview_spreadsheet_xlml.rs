use std::borrow::Cow;

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use quick_xml::events::{BytesStart, Event};
use taide_model::error::AppResult;

use crate::preview::invalid;
use crate::preview_spreadsheet::{
    Cell, MAX_DECODED_BYTES, MAX_EXCEL_COLUMNS, MAX_EXCEL_ROWS, MAX_PREVIEW_ROWS, MAX_SHEETS,
    Sheet, Workbook,
};

const MAX_XML_DEPTH: usize = 128;
const MAX_XML_NAME_BYTES: usize = 256;
const UTF16_UNIT_BYTES: usize = 2;
const MILLISECONDS_PER_DAY: f64 = 86_400_000.0;
const EXCEL_LEAP_BOUNDARY: f64 = 60.0;
const ESCAPED_UNIT_BYTES: usize = 7;

pub(crate) fn is_xml(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        return bytes
            .as_chunks::<UTF16_UNIT_BYTES>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
            .find(|unit| !matches!(*unit, 0x0a | 0x0d | 0x20))
            == Some(u16::from(b'<'));
    }
    bytes
        .iter()
        .copied()
        .find(|byte| !matches!(*byte, b'\n' | b'\r' | b' '))
        == Some(b'<')
}

pub(crate) fn source(bytes: &[u8]) -> AppResult<Cow<'_, str>> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid(
            "encoded spreadsheet exceeds the file preview budget",
        ));
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        if !bytes.len().is_multiple_of(UTF16_UNIT_BYTES) {
            return Err(invalid("SpreadsheetML UTF16 input is truncated"));
        }
        let units = bytes
            .as_chunks::<UTF16_UNIT_BYTES>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes([unit[0], unit[1]]));
        let mut text = String::new();
        for character in char::decode_utf16(units) {
            text.push(character.map_err(|error| invalid(error.to_string()))?);
            if text.capacity() > MAX_DECODED_BYTES {
                return Err(invalid("SpreadsheetML decoded source exceeds the budget"));
            }
        }
        return Ok(Cow::Owned(text));
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    Ok(Cow::Borrowed(
        std::str::from_utf8(bytes).map_err(|error| invalid(error.to_string()))?,
    ))
}

fn escaped_units(text: &str) -> String {
    let mut units = Vec::new();
    let mut remainder = text;
    while !remainder.is_empty() {
        let escaped = remainder
            .get(..ESCAPED_UNIT_BYTES)
            .filter(|value| value.starts_with("_x") || value.starts_with("_X"))
            .filter(|value| value.ends_with('_'))
            .and_then(|value| u16::from_str_radix(&value[2..ESCAPED_UNIT_BYTES - 1], 16).ok());
        if let Some(unit) = escaped {
            units.push(unit);
            remainder = &remainder[ESCAPED_UNIT_BYTES..];
            continue;
        }
        let Some(character) = remainder.chars().next() else {
            break;
        };
        let mut buffer = [0; UTF16_UNIT_BYTES];
        units.extend_from_slice(character.encode_utf16(&mut buffer));
        remainder = &remainder[character.len_utf8()..];
    }
    String::from_utf16_lossy(&units)
}

fn without_tags(text: &str) -> String {
    let mut output = String::new();
    let mut remainder = text;
    while let Some(start) = remainder.find('<') {
        output.push_str(&remainder[..start]);
        let Some(end) = remainder[start..].find('>') else {
            output.push_str(&remainder[start..]);
            return output;
        };
        remainder = &remainder[start + end + 1..];
    }
    output.push_str(remainder);
    output
}

#[derive(Default)]
struct Attributes {
    name: Option<String>,
    index: Option<u32>,
    merge_across: u32,
    kind: String,
}

fn attributes(
    element: &BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> AppResult<Attributes> {
    let mut result = Attributes::default();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| invalid(error.to_string()))?;
        let key = attribute.key.local_name();
        if !matches!(key.as_ref(), b"Name" | b"Index" | b"MergeAcross" | b"Type") {
            continue;
        }
        let text = attribute
            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, decoder)
            .map_err(|error| invalid(error.to_string()))?;
        match key.as_ref() {
            b"Name" => result.name = Some(escaped_units(&text)),
            b"Type" => result.kind = text.into_owned(),
            b"Index" => {
                result.index = Some(
                    text.parse::<u32>()
                        .ok()
                        .and_then(|index| index.checked_sub(1))
                        .ok_or_else(|| invalid("SpreadsheetML index must be a positive integer"))?,
                );
            }
            b"MergeAcross" => {
                result.merge_across = text
                    .parse::<u32>()
                    .map_err(|error| invalid(error.to_string()))?;
            }
            _ => (),
        }
    }
    Ok(result)
}

struct Data {
    depth: usize,
    kind: String,
    text: String,
    font_text: String,
    font_depth: Option<usize>,
    rich: bool,
}

impl Data {
    fn append(&mut self, text: &str) -> AppResult<()> {
        if self.text.len().saturating_add(text.len()) > MAX_DECODED_BYTES {
            return Err(invalid("SpreadsheetML cell text exceeds the budget"));
        }
        self.text.push_str(text);
        if self.font_depth.is_some() {
            self.font_text.push_str(text);
        }
        Ok(())
    }

    fn cell(self) -> AppResult<Cell> {
        match self.kind.as_str() {
            "Boolean" => Ok(Cell::Boolean(matches!(
                self.text.as_str(),
                "1" | "true" | "TRUE"
            ))),
            "Error" => Ok(Cell::Null),
            "Number" => {
                let value = match self.text.trim() {
                    "Infinity" | "+Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    _ => crate::preview_spreadsheet_csv::numeric(&self.text).unwrap_or(f64::NAN),
                };
                Ok(Cell::Number(value))
            }
            "DateTime" => {
                let text = if self.text.ends_with('Z') {
                    self.text
                } else {
                    format!("{}Z", self.text)
                };
                let date = DateTime::parse_from_rfc3339(&text)
                    .ok()
                    .map(|value| value.naive_utc())
                    .or_else(|| NaiveDateTime::parse_from_str(&text, "%Y-%m-%dT%H:%M:%S%.fZ").ok())
                    .or_else(|| {
                        NaiveDate::parse_from_str(&text, "%Y-%m-%dZ")
                            .ok()?
                            .and_hms_opt(0, 0, 0)
                    });
                let Some(date) = date else {
                    return Ok(Cell::Text(text));
                };
                let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)
                    .and_then(|date| date.and_hms_opt(0, 0, 0))
                    .ok_or_else(|| invalid("SpreadsheetML date epoch is unavailable"))?;
                let serial = date.signed_duration_since(epoch).num_milliseconds() as f64
                    / MILLISECONDS_PER_DAY;
                let serial = if serial < EXCEL_LEAP_BOUNDARY {
                    serial - 1.0
                } else {
                    serial
                };
                Ok(Cell::Number(serial))
            }
            "String" => {
                let text = if self.rich && !self.font_text.is_empty() {
                    self.font_text
                } else {
                    self.text
                };
                let text = escaped_units(&text);
                if self.rich {
                    return Ok(Cell::Text(without_tags(&text)));
                }
                Ok(Cell::Text(text))
            }
            _ if self.text.is_empty() => Ok(Cell::Null),
            _ => Ok(Cell::Text(escaped_units(&self.text))),
        }
    }
}

enum Visit<'a> {
    Sheet(&'a str),
    Row(u32),
    Column(u32),
    Cell(u32, u32, Cell),
    EndSheet,
}

#[derive(Default)]
struct State {
    root_seen: bool,
    sheet_depth: Option<usize>,
    table_depth: Option<usize>,
    row_depth: Option<usize>,
    cell_depth: Option<usize>,
    row: u32,
    column: u32,
    merge_across: u32,
    data: Option<Data>,
}

impl State {
    fn start(
        &mut self,
        element: &BytesStart<'_>,
        depth: usize,
        empty: bool,
        decoder: quick_xml::encoding::Decoder,
        visitor: &mut impl FnMut(Visit<'_>) -> AppResult<()>,
    ) -> AppResult<()> {
        let name = element.local_name();
        let name = name.as_ref();
        if let Some(data) = &mut self.data {
            data.rich = true;
            if name.eq_ignore_ascii_case(b"Font") {
                data.font_depth = Some(depth);
            }
            return Ok(());
        }
        if depth == 1 {
            if self.root_seen || !name.eq_ignore_ascii_case(b"Workbook") {
                return Err(invalid("XML spreadsheet root is not a Workbook"));
            }
            self.root_seen = true;
            return Ok(());
        }
        let attrs = attributes(element, decoder)?;
        if name.eq_ignore_ascii_case(b"Worksheet") {
            if depth != 2 || self.sheet_depth.is_some() {
                return Err(invalid("SpreadsheetML worksheet nesting is invalid"));
            }
            let name = attrs
                .name
                .ok_or_else(|| invalid("SpreadsheetML sheet name is missing"))?;
            visitor(Visit::Sheet(&name))?;
            self.sheet_depth = Some(depth);
            self.row = 0;
            self.column = 0;
            return Ok(());
        }
        if name.eq_ignore_ascii_case(b"Table") && self.sheet_depth == Some(depth - 1) {
            self.table_depth = Some(depth);
            return Ok(());
        }
        if name.eq_ignore_ascii_case(b"Row") && self.table_depth == Some(depth - 1) {
            if empty {
                if self.row >= MAX_EXCEL_ROWS {
                    return Err(invalid("SpreadsheetML row exceeds Excel bounds"));
                }
                visitor(Visit::Row(self.row))?;
            }
            if let Some(index) = attrs.index {
                self.row = index;
            }
            if self.row >= MAX_EXCEL_ROWS {
                return Err(invalid("SpreadsheetML row exceeds Excel bounds"));
            }
            self.row_depth = Some(depth);
            return Ok(());
        }
        if name.eq_ignore_ascii_case(b"Cell") && self.row_depth == Some(depth - 1) {
            if let Some(index) = attrs.index {
                self.column = index;
            }
            if self.column >= MAX_EXCEL_COLUMNS {
                return Err(invalid("SpreadsheetML column exceeds Excel bounds"));
            }
            self.merge_across = if empty { 0 } else { attrs.merge_across };
            if self.merge_across >= MAX_EXCEL_COLUMNS - self.column {
                return Err(invalid("SpreadsheetML merge exceeds Excel bounds"));
            }
            self.cell_depth = Some(depth);
            visitor(Visit::Column(self.column))?;
            return Ok(());
        }
        if name == b"Data" && self.cell_depth == Some(depth - 1) {
            self.data = Some(Data {
                depth,
                kind: attrs.kind,
                text: String::new(),
                font_text: String::new(),
                font_depth: None,
                rich: false,
            });
        }
        Ok(())
    }

    fn end(
        &mut self,
        depth: usize,
        empty: bool,
        visitor: &mut impl FnMut(Visit<'_>) -> AppResult<()>,
    ) -> AppResult<()> {
        if let Some(data) = &mut self.data {
            if data.depth != depth {
                if data.font_depth == Some(depth) {
                    data.font_depth = None;
                }
                return Ok(());
            }
            let data = self
                .data
                .take()
                .ok_or_else(|| invalid("SpreadsheetML data state is missing"))?;
            visitor(Visit::Cell(self.row, self.column, data.cell()?))?;
            return Ok(());
        }
        if self.cell_depth == Some(depth) {
            self.column += self.merge_across + 1;
            self.cell_depth = None;
        } else if self.row_depth == Some(depth) {
            if !empty {
                visitor(Visit::Row(self.row))?;
            }
            self.row += 1;
            self.column = 0;
            self.row_depth = None;
        } else if self.table_depth == Some(depth) {
            self.table_depth = None;
        } else if self.sheet_depth == Some(depth) {
            visitor(Visit::EndSheet)?;
            self.sheet_depth = None;
        }
        Ok(())
    }
}

fn scan(source: &str, mut visitor: impl FnMut(Visit<'_>) -> AppResult<()>) -> AppResult<()> {
    let mut reader = quick_xml::Reader::from_str(source);
    let mut state = State::default();
    let mut stack = Vec::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|error| invalid(error.to_string()))?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let depth = stack.len() + 1;
                if depth > MAX_XML_DEPTH || element.name().as_ref().len() > MAX_XML_NAME_BYTES {
                    return Err(invalid(
                        "SpreadsheetML XML nesting or name exceeds the budget",
                    ));
                }
                state.start(&element, depth, empty, reader.decoder(), &mut visitor)?;
                if empty {
                    state.end(depth, true, &mut visitor)?;
                } else {
                    stack.push(element.name().as_ref().to_vec());
                }
            }
            Event::End(element) => {
                let depth = stack.len();
                if stack.pop().as_deref() != Some(element.name().as_ref()) {
                    return Err(invalid("SpreadsheetML XML close tag is invalid"));
                }
                state.end(depth, false, &mut visitor)?;
            }
            Event::Text(text) => {
                let text = text.decode().map_err(|error| invalid(error.to_string()))?;
                if let Some(data) = &mut state.data {
                    data.append(&text)?;
                } else if stack.is_empty() && !text.trim().is_empty() {
                    return Err(invalid("SpreadsheetML XML has text outside the workbook"));
                }
            }
            Event::GeneralRef(reference) => {
                let text = reference
                    .decode()
                    .map_err(|error| invalid(error.to_string()))?;
                let resolved = reference
                    .resolve_char_ref()
                    .map_err(|error| invalid(error.to_string()))?;
                let mut buffer = [0; size_of::<char>()];
                let text = if let Some(character) = resolved {
                    character.encode_utf8(&mut buffer)
                } else {
                    quick_xml::escape::resolve_predefined_entity(&text)
                        .ok_or_else(|| invalid("SpreadsheetML general entities are not allowed"))?
                };
                if let Some(data) = &mut state.data {
                    data.append(text)?;
                }
            }
            Event::DocType(_) => {
                return Err(invalid("SpreadsheetML document types are not allowed"));
            }
            Event::CData(_) => return Err(invalid("SpreadsheetML CDATA is not connected")),
            Event::Eof => {
                if !state.root_seen || !stack.is_empty() {
                    return Err(invalid("SpreadsheetML XML workbook is incomplete"));
                }
                return Ok(());
            }
            _ => (),
        }
    }
}

struct Bounds {
    name: String,
    rows: Option<(u32, u32)>,
    columns: Option<(u32, u32)>,
}

fn include(range: &mut Option<(u32, u32)>, index: u32) {
    *range = Some(match *range {
        Some((first, last)) => (first.min(index), last.max(index)),
        None => (index, index),
    });
}

pub fn decode(bytes: &[u8]) -> AppResult<Workbook> {
    let source = source(bytes)?;
    if crate::preview_spreadsheet_html::is_html(&source)? {
        return crate::preview_spreadsheet_html::decode(bytes);
    }
    let mut bounds: Vec<Bounds> = Vec::new();
    let mut name_bytes = 0usize;
    scan(&source, |visit| {
        match visit {
            Visit::Sheet(name) => {
                name_bytes = name_bytes.saturating_add(name.len());
                if bounds.len() >= MAX_SHEETS
                    || name_bytes > MAX_DECODED_BYTES
                    || bounds.iter().any(|sheet| sheet.name == name)
                {
                    return Err(invalid(
                        "SpreadsheetML sheets exceed the budget or repeat a name",
                    ));
                }
                bounds.push(Bounds {
                    name: name.into(),
                    rows: None,
                    columns: None,
                });
            }
            Visit::Row(row) => {
                if let Some(sheet) = bounds.last_mut() {
                    include(&mut sheet.rows, row);
                }
            }
            Visit::Column(column) => {
                if let Some(sheet) = bounds.last_mut() {
                    include(&mut sheet.columns, column);
                }
            }
            _ => (),
        }
        Ok(())
    })?;
    let mut workbook = Workbook {
        sheets: Vec::with_capacity(bounds.len()),
    };
    let mut retained = name_bytes + workbook.sheets.capacity() * size_of::<Sheet>();
    for sheet in &bounds {
        let (total, columns) = match (sheet.rows, sheet.columns) {
            (Some((first_row, last_row)), Some((first_column, last_column))) => (
                (last_row - first_row + 1) as usize,
                (last_column - first_column + 1) as usize,
            ),
            _ => (0, 0),
        };
        let rows = total.min(MAX_PREVIEW_ROWS);
        let grid_bytes = rows
            .checked_mul(columns)
            .and_then(|cells| cells.checked_mul(size_of::<Cell>()))
            .and_then(|bytes| bytes.checked_add(rows * size_of::<Vec<Cell>>()))
            .ok_or_else(|| invalid("SpreadsheetML grid size overflows"))?;
        retained = retained
            .checked_add(grid_bytes)
            .filter(|bytes| *bytes <= MAX_DECODED_BYTES)
            .ok_or_else(|| invalid("SpreadsheetML projected grid exceeds the budget"))?;
        workbook.sheets.push(Sheet {
            name: sheet.name.clone(),
            rows: vec![vec![Cell::Null; columns]; rows],
            total_row_count: total,
            truncated: total > MAX_PREVIEW_ROWS,
        });
    }
    let mut current = None;
    scan(&source, |visit| {
        match visit {
            Visit::Sheet(_) => current = Some(current.map_or(0, |index: usize| index + 1)),
            Visit::Cell(row, column, value) => {
                let index = current.ok_or_else(|| invalid("SpreadsheetML cell has no sheet"))?;
                let sheet = &bounds[index];
                let (Some((first_row, _)), Some((first_column, _))) = (sheet.rows, sheet.columns)
                else {
                    return Ok(());
                };
                let Some(row) = row.checked_sub(first_row).map(|row| row as usize) else {
                    return Ok(());
                };
                let Some(column) = column
                    .checked_sub(first_column)
                    .map(|column| column as usize)
                else {
                    return Ok(());
                };
                let Some(cell) = workbook.sheets[index]
                    .rows
                    .get_mut(row)
                    .and_then(|row| row.get_mut(column))
                else {
                    return Ok(());
                };
                retained = retained
                    .saturating_sub(cell.text_bytes())
                    .checked_add(value.text_bytes())
                    .filter(|bytes| *bytes <= MAX_DECODED_BYTES)
                    .ok_or_else(|| {
                        invalid("SpreadsheetML retained cell text exceeds the budget")
                    })?;
                *cell = value;
            }
            _ => (),
        }
        Ok(())
    })?;
    if workbook.retained_bytes() > MAX_DECODED_BYTES {
        return Err(invalid(
            "SpreadsheetML retained workbook exceeds the budget",
        ));
    }
    Ok(workbook)
}
