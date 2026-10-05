use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use calamine::{DataRef, Dimensions, Reader, Xlsx};
use quick_xml::events::Event;
use taide_model::error::AppResult;
use zip::{CompressionMethod, ZipArchive};

use crate::preview::{Failure, invalid};

pub const MAX_PREVIEW_ROWS: usize = 500;
pub const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_SHEETS: usize = 1024;
const MAX_ZIP_ENTRIES: usize = u16::MAX as usize;
pub(crate) const MAX_EXCEL_ROWS: u32 = 1_048_576;
pub(crate) const MAX_EXCEL_COLUMNS: u32 = 16_384;
const MAX_SHARED_STRINGS: usize = MAX_DECODED_BYTES / (size_of::<String>() * 2);
const SPARSE_ENTRY_BYTES: usize = size_of::<Cell>() * 2 + size_of::<(u32, u32)>();
const COMPOUND_SIGNATURE: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub path: String,
    pub token: u64,
}

pub async fn read(
    services: &taide_runtime::AppServices,
    request: &Request,
    on_source_ready: impl FnOnce() + Send + 'static,
) -> Result<Workbook, Failure> {
    let result = crate::preview::read_approved(
        services,
        request.path.clone(),
        "native-spreadsheet-preview",
        move |source| {
            let bytes = taide_file::service::read_raw(source)?;
            on_source_ready();
            Ok(decode(&bytes))
        },
    )
    .await;
    match result {
        Ok(result) => result.map_err(Failure::Decode),
        Err(error) => Err(Failure::Read(error)),
    }
}

pub fn decode(bytes: &[u8]) -> AppResult<Workbook> {
    if bytes.starts_with(&COMPOUND_SIGNATURE) {
        let stream = crate::preview_spreadsheet_xls::workbook_stream(bytes)?;
        return crate::preview_spreadsheet_biff::decode(&stream);
    }
    if bytes.first() == Some(&0x09) && bytes.get(1).copied().unwrap_or_default() <= 0x08 {
        return crate::preview_spreadsheet_biff::decode(bytes);
    }
    if bytes.starts_with(b"PK")
        && bytes.get(2).copied().unwrap_or_default() < 0x09
        && bytes.get(3).copied().unwrap_or_default() < 0x09
    {
        return decode_xlsx(bytes);
    }
    if crate::preview_spreadsheet_xlml::is_xml(bytes) {
        return crate::preview_spreadsheet_xlml::decode(bytes);
    }
    crate::preview_spreadsheet_csv::decode(bytes)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Null,
    Text(String),
    Number(f64),
    Boolean(bool),
}

impl Cell {
    pub(crate) fn text_bytes(&self) -> usize {
        match self {
            Self::Text(text) => text.capacity(),
            _ => 0,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Text(text) => text.clone(),
            Self::Boolean(value) => value.to_string(),
            Self::Number(value) => ryu_js::Buffer::new().format(*value).into(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Vec<Cell>>,
    pub total_row_count: usize,
    pub truncated: bool,
}

impl Sheet {
    fn retained_bytes(&self) -> usize {
        self.rows.iter().fold(
            self.name.capacity() + self.rows.capacity() * size_of::<Vec<Cell>>(),
            |bytes, row| {
                row.iter().fold(
                    bytes.saturating_add(row.capacity().saturating_mul(size_of::<Cell>())),
                    |bytes, cell| bytes.saturating_add(cell.text_bytes()),
                )
            },
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct Workbook {
    pub sheets: Vec<Sheet>,
}

impl Workbook {
    pub fn retained_bytes(&self) -> usize {
        self.sheets.iter().fold(
            self.sheets.capacity().saturating_mul(size_of::<Sheet>()),
            |bytes, sheet| bytes.saturating_add(sheet.retained_bytes()),
        )
    }
}

fn preflight_xml(bytes: &[u8]) -> AppResult<()> {
    let mut reader = quick_xml::Reader::from_reader(bytes);
    loop {
        match reader
            .read_event()
            .map_err(|error| invalid(error.to_string()))?
        {
            Event::Start(element) | Event::Empty(element) => {
                for attribute in element.attributes() {
                    let attribute = attribute.map_err(|error| invalid(error.to_string()))?;
                    if attribute.key.local_name().as_ref() != b"uniqueCount" {
                        continue;
                    }
                    std::str::from_utf8(&attribute.value)
                        .ok()
                        .and_then(|value| value.parse::<usize>().ok())
                        .filter(|count| *count <= MAX_SHARED_STRINGS)
                        .ok_or_else(|| {
                            invalid("XLSX shared string reservation exceeds the budget")
                        })?;
                }
            }
            Event::DocType(_) => return Err(invalid("XLSX document types are not allowed")),
            Event::Eof => return Ok(()),
            _ => (),
        }
    }
}

fn preflight(bytes: &[u8]) -> AppResult<()> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid(
            "encoded spreadsheet exceeds the file preview budget",
        ));
    }
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|error| invalid(error.to_string()))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(invalid("XLSX archive has too many entries"));
    }
    let mut remaining = MAX_DECODED_BYTES;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| invalid(error.to_string()))?;
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) || entry.size() > remaining as u64
        {
            return Err(invalid("XLSX archive exceeds the decoded budget"));
        }
        let is_xml = entry.name().to_ascii_lowercase().ends_with(".xml");
        let mut data = Vec::new();
        entry.take(remaining as u64 + 1).read_to_end(&mut data)?;
        remaining = remaining
            .checked_sub(data.len())
            .ok_or_else(|| invalid("XLSX archive exceeds the decoded budget"))?;
        if is_xml {
            preflight_xml(&data)?;
        }
    }
    Ok(())
}

fn cell(value: &DataRef<'_>) -> AppResult<Cell> {
    match value {
        DataRef::Empty | DataRef::Error(_) => Ok(Cell::Null),
        DataRef::String(text) => Ok(Cell::Text(text.clone())),
        DataRef::SharedString(text) => Ok(Cell::Text((*text).into())),
        DataRef::Bool(value) => Ok(Cell::Boolean(*value)),
        DataRef::Int(value) => Ok(Cell::Number(*value as f64)),
        DataRef::Float(value) if value.is_finite() => Ok(Cell::Number(*value)),
        DataRef::DateTime(value) => Ok(Cell::Number(value.as_f64())),
        _ => Err(invalid("native spreadsheet cell type is not connected yet")),
    }
}

fn valid_dimensions(dimensions: Dimensions) -> AppResult<()> {
    if dimensions.start.0 > dimensions.end.0
        || dimensions.start.1 > dimensions.end.1
        || dimensions.end.0 >= MAX_EXCEL_ROWS
        || dimensions.end.1 >= MAX_EXCEL_COLUMNS
    {
        return Err(invalid("spreadsheet dimensions exceed Excel bounds"));
    }
    Ok(())
}

pub fn decode_xlsx(bytes: &[u8]) -> AppResult<Workbook> {
    preflight(bytes)?;
    let mut reader = Xlsx::new(Cursor::new(bytes)).map_err(|error| invalid(error.to_string()))?;
    let names = reader.sheet_names();
    if names.len() > MAX_SHEETS {
        return Err(invalid("spreadsheet has too many sheets"));
    }
    let mut sheets = Vec::new();
    let mut retained = 0usize;
    for name in names {
        let mut stream = reader
            .worksheet_cells_reader(&name)
            .map_err(|error| invalid(error.to_string()))?;
        let mut dimensions = stream.dimensions();
        valid_dimensions(dimensions)?;
        let declared = dimensions != Dimensions::default();
        let mut has_cells = false;
        let mut values = BTreeMap::new();
        let mut text_bytes = 0usize;
        while let Some(value) = stream
            .next_cell()
            .map_err(|error| invalid(error.to_string()))?
        {
            let (row, column) = value.get_position();
            if row >= MAX_EXCEL_ROWS || column >= MAX_EXCEL_COLUMNS {
                return Err(invalid("spreadsheet cell exceeds Excel bounds"));
            }
            let previous_origin = dimensions.start.0;
            if !declared {
                if !has_cells {
                    dimensions = Dimensions::new((row, column), (row, column));
                } else {
                    dimensions.start.0 = dimensions.start.0.min(row);
                    dimensions.start.1 = dimensions.start.1.min(column);
                    dimensions.end.0 = dimensions.end.0.max(row);
                    dimensions.end.1 = dimensions.end.1.max(column);
                }
            }
            has_cells = true;
            if dimensions.start.0 != previous_origin {
                let removed = values.split_off(&(dimensions.start.0 + MAX_PREVIEW_ROWS as u32, 0));
                text_bytes -= removed.values().map(Cell::text_bytes).sum::<usize>();
            }
            if !dimensions.contains(row, column)
                || row - dimensions.start.0 >= MAX_PREVIEW_ROWS as u32
            {
                continue;
            }
            let value = cell(value.get_value())?;
            text_bytes = text_bytes.saturating_add(value.text_bytes());
            if let Some(previous) = values.insert((row, column), value) {
                text_bytes -= previous.text_bytes();
            }
            let cost = values
                .len()
                .saturating_mul(SPARSE_ENTRY_BYTES)
                .saturating_add(text_bytes);
            if cost > MAX_DECODED_BYTES {
                return Err(invalid(
                    "spreadsheet projection exceeds the retained budget",
                ));
            }
        }
        let total_row_count = if declared || has_cells {
            (dimensions.end.0 - dimensions.start.0 + 1) as usize
        } else {
            0
        };
        let height = total_row_count.min(MAX_PREVIEW_ROWS);
        let width = (dimensions.end.1 - dimensions.start.1 + 1) as usize;
        let grid_bytes = height
            .checked_mul(width)
            .and_then(|cells| cells.checked_mul(size_of::<Cell>()))
            .and_then(|bytes| bytes.checked_add(height * size_of::<Vec<Cell>>()))
            .and_then(|bytes| bytes.checked_add(text_bytes))
            .and_then(|bytes| bytes.checked_add(name.capacity()))
            .and_then(|bytes| bytes.checked_add(retained))
            .filter(|bytes| *bytes <= MAX_DECODED_BYTES)
            .ok_or_else(|| invalid("spreadsheet projection exceeds the retained budget"))?;
        let mut rows = vec![vec![Cell::Null; width]; height];
        for ((row, column), value) in values {
            rows[(row - dimensions.start.0) as usize][(column - dimensions.start.1) as usize] =
                value;
        }
        retained = grid_bytes;
        sheets.push(Sheet {
            name,
            rows,
            total_row_count,
            truncated: total_row_count > MAX_PREVIEW_ROWS,
        });
    }
    let workbook = Workbook { sheets };
    if workbook.retained_bytes() > MAX_DECODED_BYTES {
        return Err(invalid(
            "spreadsheet projection exceeds the retained budget",
        ));
    }
    Ok(workbook)
}
