use std::borrow::Cow;

use encoding_rs::{Encoding, WINDOWS_1252};
use taide_model::error::AppResult;

use crate::preview::invalid;
use crate::preview_spreadsheet::{
    Cell, MAX_DECODED_BYTES, MAX_PREVIEW_ROWS, MAX_SHEETS, Sheet, Workbook,
};

const BOF: u16 = 0x0809;
const EOF: u16 = 0x000a;
const BOUND_SHEET: u16 = 0x0085;
const SST: u16 = 0x00fc;
const CONTINUE: u16 = 0x003c;
const FILE_PASS: u16 = 0x002f;
const DIMENSIONS: u16 = 0x0200;
const NUMBER: u16 = 0x0203;
const RK: u16 = 0x027e;
const MUL_RK: u16 = 0x00bd;
const LABEL_SST: u16 = 0x00fd;
const LABEL: u16 = 0x0204;
const R_STRING: u16 = 0x00d6;
const BOOL_ERR: u16 = 0x0205;
const FORMULA: u16 = 0x0006;
const STRING: u16 = 0x0207;
const BIFF8_VERSION: u16 = 0x0600;
const BIFF5_VERSION: u16 = 0x0500;
const BOF_2: u16 = 0x0009;
const BOF_3: u16 = 0x0209;
const BOF_4: u16 = 0x0409;
const CODE_PAGE: u16 = 0x0042;
const LEGACY_DIMENSIONS: u16 = 0x0000;
const LEGACY_INTEGER: u16 = 0x0002;
const LEGACY_NUMBER: u16 = 0x0003;
const LEGACY_LABEL: u16 = 0x0004;
const LEGACY_BOOL_ERR: u16 = 0x0005;
const LEGACY_STRING: u16 = 0x0007;
const FORMULA_3: u16 = 0x0206;
const FORMULA_4: u16 = 0x0406;
const WORKBOOK_GLOBALS: u16 = 0x0005;
const WORKSHEET: u16 = 0x0010;
const CHART: u16 = 0x0020;
const MACRO_SHEET: u16 = 0x0040;
const MAX_ROWS: usize = 65_536;
const MAX_LEGACY_ROWS: usize = 16_384;
const MAX_COLUMNS: usize = 256;
const MAX_CHARACTERS: usize = 32_767;
const MAX_SHARED_STRINGS: usize = MAX_DECODED_BYTES / (size_of::<String>() * 2);
const CELL_HEADER_BYTES: usize = 6;
const FORMULA_MINIMUM_BYTES: usize = 20;
const WIDE_FLAG: u8 = 1;
const RICH_FLAG: u8 = 8;
const EXTENDED_FLAG: u8 = 4;
const FORMAT_RUN_BYTES: usize = 4;
const RK_PERCENT_FACTOR: f64 = 100.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Version {
    Biff2,
    Biff3,
    Biff4,
    Biff5,
    Biff8,
}

impl Version {
    fn max_rows(self) -> usize {
        if self == Self::Biff8 {
            return MAX_ROWS;
        }
        MAX_LEGACY_ROWS
    }

    fn is_standalone(self) -> bool {
        matches!(self, Self::Biff2 | Self::Biff3 | Self::Biff4)
    }
}

fn encoding(mut data: &[u8]) -> AppResult<&'static Encoding> {
    let code = match short(&mut data)? {
        0x5212 => 1200,
        0x8000 => 10000,
        0x8001 => 1252,
        code => code,
    };
    codepage::to_encoding_no_replacement(code)
        .ok_or_else(|| invalid("XLS codepage is not supported"))
}

fn fixed<const N: usize>(bytes: &mut &[u8]) -> AppResult<[u8; N]> {
    let value = bytes
        .get(..N)
        .and_then(|value| value.try_into().ok())
        .ok_or_else(|| invalid("XLS BIFF record is truncated"))?;
    *bytes = &bytes[N..];
    Ok(value)
}

fn short(bytes: &mut &[u8]) -> AppResult<u16> {
    Ok(u16::from_le_bytes(fixed(bytes)?))
}

fn integer(bytes: &mut &[u8]) -> AppResult<u32> {
    Ok(u32::from_le_bytes(fixed(bytes)?))
}

struct Record<'a> {
    kind: u16,
    data: &'a [u8],
}

fn record<'a>(bytes: &mut &'a [u8]) -> AppResult<Record<'a>> {
    let kind = short(bytes)?;
    let length = usize::from(short(bytes)?);
    let data = bytes
        .get(..length)
        .ok_or_else(|| invalid("XLS BIFF record payload is truncated"))?;
    *bytes = &bytes[length..];
    Ok(Record { kind, data })
}

struct Segments<'a> {
    data: &'a [u8],
    remaining: &'a [u8],
}

impl Segments<'_> {
    fn advance(&mut self) -> AppResult<()> {
        let next = record(&mut self.remaining)?;
        if next.kind != CONTINUE {
            return Err(invalid("XLS string continuation is missing"));
        }
        self.data = next.data;
        Ok(())
    }

    fn array<const N: usize>(&mut self) -> AppResult<[u8; N]> {
        let mut values = [0; N];
        for value in &mut values {
            while self.data.is_empty() {
                self.advance()?;
            }
            *value = self.data[0];
            self.data = &self.data[1..];
        }
        Ok(values)
    }

    fn skip(&mut self, mut bytes: usize) -> AppResult<()> {
        while bytes > 0 {
            while self.data.is_empty() {
                self.advance()?;
            }
            let length = bytes.min(self.data.len());
            self.data = &self.data[length..];
            bytes -= length;
        }
        Ok(())
    }

    fn text(&mut self, count: usize, flags: u8) -> AppResult<String> {
        if count > MAX_CHARACTERS {
            return Err(invalid("XLS text exceeds Excel character bounds"));
        }
        let runs = if flags & RICH_FLAG != 0 {
            usize::from(u16::from_le_bytes(self.array()?))
        } else {
            0
        };
        let extended = if flags & EXTENDED_FLAG != 0 {
            u32::from_le_bytes(self.array()?) as usize
        } else {
            0
        };
        let mut wide = flags & WIDE_FLAG != 0;
        let mut units = Vec::with_capacity(count);
        for _ in 0..count {
            if self.data.is_empty() {
                self.advance()?;
                wide = self.array::<1>()?[0] & WIDE_FLAG != 0;
            }
            let unit = if wide {
                u16::from_le_bytes(fixed::<2>(&mut self.data)?)
            } else {
                u16::from(fixed::<1>(&mut self.data)?[0])
            };
            units.push(unit);
        }
        self.skip(runs * FORMAT_RUN_BYTES)?;
        self.skip(extended)?;
        Ok(String::from_utf16_lossy(&units))
    }

    fn long_text(&mut self) -> AppResult<String> {
        let count = usize::from(u16::from_le_bytes(self.array()?));
        let flags = self.array::<1>()?[0];
        self.text(count, flags)
    }

    fn legacy_text(&mut self, count: usize, encoding: &'static Encoding) -> AppResult<String> {
        if count > MAX_CHARACTERS {
            return Err(invalid("XLS text exceeds Excel character bounds"));
        }
        let mut bytes = Vec::with_capacity(count);
        while bytes.len() < count {
            while self.data.is_empty() {
                self.advance()?;
            }
            let length = (count - bytes.len()).min(self.data.len());
            bytes.extend_from_slice(&self.data[..length]);
            self.data = &self.data[length..];
        }
        Ok(encoding.decode_without_bom_handling(&bytes).0.into_owned())
    }

    fn cell_text(&mut self, version: Version, encoding: &'static Encoding) -> AppResult<String> {
        if version == Version::Biff8 {
            return self.long_text();
        }
        let count = if version == Version::Biff2 {
            usize::from(self.array::<1>()?[0])
        } else {
            usize::from(u16::from_le_bytes(self.array()?))
        };
        self.legacy_text(count, encoding)
    }
}

fn shared_strings<'a>(data: &'a [u8], remaining: &mut &'a [u8]) -> AppResult<Vec<String>> {
    let mut cursor = Segments { data, remaining };
    cursor.array::<4>()?;
    let count = u32::from_le_bytes(cursor.array()?) as usize;
    if count > MAX_SHARED_STRINGS {
        return Err(invalid("XLS shared string count exceeds the budget"));
    }
    let mut strings = Vec::with_capacity(count);
    let mut bytes = strings.capacity() * size_of::<String>();
    for _ in 0..count {
        let text = cursor.long_text()?;
        bytes = bytes.saturating_add(text.capacity());
        if bytes > MAX_DECODED_BYTES {
            return Err(invalid("XLS shared strings exceed the decoded budget"));
        }
        strings.push(text);
    }
    *remaining = cursor.remaining;
    Ok(strings)
}

struct BoundSheet {
    offset: usize,
    name: String,
}

fn bound_sheet(
    mut data: &[u8],
    version: Version,
    encoding: &'static Encoding,
) -> AppResult<BoundSheet> {
    let offset = integer(&mut data)? as usize;
    fixed::<2>(&mut data)?;
    let count = usize::from(fixed::<1>(&mut data)?[0]);
    let mut cursor = Segments {
        data,
        remaining: &[],
    };
    let name = if version == Version::Biff8 {
        let flags = cursor.array::<1>()?[0];
        cursor.text(count, flags & WIDE_FLAG)?
    } else {
        cursor.legacy_text(count, encoding)?
    };
    Ok(BoundSheet {
        offset,
        name: if name.is_empty() {
            "Sheet1".into()
        } else {
            name
        },
    })
}

enum Value<'a> {
    Null,
    Text(Cow<'a, str>),
    Number(f64),
    Boolean(bool),
}

impl Value<'_> {
    fn cell(self) -> Cell {
        match self {
            Self::Null => Cell::Null,
            Self::Text(text) => Cell::Text(text.into_owned()),
            Self::Number(number) => Cell::Number(number),
            Self::Boolean(value) => Cell::Boolean(value),
        }
    }
}

#[derive(Default, Clone, Copy)]
struct Range {
    first_row: usize,
    first_column: usize,
    end_row: usize,
    end_column: usize,
}

impl Range {
    fn valid(&self, version: Version) -> AppResult<()> {
        if self.first_row > self.end_row
            || self.first_column > self.end_column
            || self.end_row > version.max_rows()
            || self.end_column > MAX_COLUMNS
        {
            return Err(invalid("XLS dimensions exceed Excel bounds"));
        }
        Ok(())
    }

    fn include(&mut self, row: usize, column: usize, version: Version) -> AppResult<()> {
        if row >= version.max_rows() || column >= MAX_COLUMNS {
            return Err(invalid("XLS cell exceeds Excel bounds"));
        }
        self.first_row = self.first_row.min(row);
        self.first_column = self.first_column.min(column);
        self.end_row = self.end_row.max(row + 1);
        self.end_column = self.end_column.max(column + 1);
        Ok(())
    }
}

fn rk(bytes: [u8; 4]) -> f64 {
    let raw = u32::from_le_bytes(bytes);
    let value = if raw & 2 != 0 {
        (i32::from_le_bytes(bytes) >> 2) as f64
    } else {
        f64::from_bits(u64::from(raw & !3) << 32)
    };
    if raw & 1 != 0 {
        value / RK_PERCENT_FACTOR
    } else {
        value
    }
}

fn bof(bytes: &mut &[u8]) -> AppResult<(Version, u16)> {
    let first = record(bytes)?;
    let mut data = first.data;
    let declared = short(&mut data)?;
    let version = match first.kind {
        BOF_2 => Version::Biff2,
        BOF_3 => Version::Biff3,
        BOF_4 => Version::Biff4,
        BOF => match declared {
            0x0200 | 0x0002 | 0x0007 => Version::Biff2,
            0x0300 => Version::Biff3,
            0x0400 => Version::Biff4,
            BIFF5_VERSION => Version::Biff5,
            BIFF8_VERSION => Version::Biff8,
            _ => return Err(invalid("native XLS BIFF version is not connected yet")),
        },
        _ => return Err(invalid("XLS BIFF substream has no BOF record")),
    };
    Ok((version, short(&mut data)?))
}

fn cells<'a>(
    mut bytes: &'a [u8],
    strings: &'a [String],
    version: Version,
    mut codepage: &'static Encoding,
    mut on_cell: impl FnMut(usize, usize, Value<'a>) -> AppResult<()>,
) -> AppResult<Range> {
    if !matches!(bof(&mut bytes)?.1, WORKSHEET | CHART | MACRO_SHEET) {
        return Err(invalid("XLS sheet has an unsupported BIFF substream type"));
    }
    let mut range = Range::default();
    let mut depth = 1usize;
    let mut formula_string = None;
    loop {
        let current = record(&mut bytes)?;
        let mut data = current.data;
        if matches!(current.kind, BOF | BOF_2 | BOF_3 | BOF_4) {
            depth += 1;
            continue;
        }
        if current.kind == EOF {
            depth -= 1;
            if depth == 0 {
                if formula_string.is_some() {
                    return Err(invalid("XLS cached formula string is missing"));
                }
                return Ok(range);
            }
            continue;
        }
        if depth != 1 {
            continue;
        }
        if current.kind == FILE_PASS {
            return Err(invalid("encrypted XLS workbooks are not supported"));
        }
        if current.kind == CODE_PAGE {
            codepage = encoding(data)?;
            continue;
        }
        if current.kind == DIMENSIONS
            || current.kind == LEGACY_DIMENSIONS && version == Version::Biff2
        {
            let (first_row, end_row) = if version == Version::Biff8 {
                (integer(&mut data)? as usize, integer(&mut data)? as usize)
            } else {
                (
                    usize::from(short(&mut data)?),
                    usize::from(short(&mut data)?),
                )
            };
            let first_column = usize::from(short(&mut data)?);
            let end_column = usize::from(short(&mut data)?);
            range = Range {
                first_row,
                first_column,
                end_row,
                end_column,
            };
            range.valid(version)?;
            continue;
        }
        if current.kind == STRING || current.kind == LEGACY_STRING && version == Version::Biff2 {
            let (row, column) = formula_string
                .take()
                .ok_or_else(|| invalid("XLS formula string has no cell"))?;
            let mut cursor = Segments {
                data,
                remaining: bytes,
            };
            let text = cursor.cell_text(version, codepage)?;
            bytes = cursor.remaining;
            range.include(row, column, version)?;
            on_cell(row, column, Value::Text(Cow::Owned(text)))?;
            continue;
        }
        let kind = match (version, current.kind) {
            (_, LEGACY_INTEGER) if version.is_standalone() => LEGACY_INTEGER,
            (_, LEGACY_NUMBER) if version.is_standalone() => NUMBER,
            (_, LEGACY_LABEL) if version.is_standalone() => LABEL,
            (_, LEGACY_BOOL_ERR) if version.is_standalone() => BOOL_ERR,
            (_, FORMULA_3 | FORMULA_4) => FORMULA,
            (_, kind) => kind,
        };
        if !matches!(
            kind,
            NUMBER | RK | MUL_RK | LABEL_SST | LABEL | R_STRING | BOOL_ERR | FORMULA
        ) && !(version.is_standalone() && kind == LEGACY_INTEGER)
        {
            continue;
        }
        let row = usize::from(short(&mut data)?);
        let column = usize::from(short(&mut data)?);
        if kind == MUL_RK {
            let mut last = data
                .get(data.len().saturating_sub(size_of::<u16>())..)
                .ok_or_else(|| invalid("XLS MulRk is truncated"))?;
            let last = usize::from(short(&mut last)?);
            let count = last
                .checked_sub(column)
                .and_then(|count| count.checked_add(1))
                .ok_or_else(|| invalid("XLS MulRk columns are reversed"))?;
            if data.len() != count * CELL_HEADER_BYTES + size_of::<u16>() || last >= MAX_COLUMNS {
                return Err(invalid("XLS MulRk payload is invalid"));
            }
            for column in column..=last {
                short(&mut data)?;
                let value = rk(fixed(&mut data)?);
                range.include(row, column, version)?;
                on_cell(row, column, Value::Number(value))?;
            }
            continue;
        }
        short(&mut data)?;
        if matches!(
            current.kind,
            LEGACY_INTEGER | LEGACY_NUMBER | LEGACY_LABEL | LEGACY_BOOL_ERR
        ) || kind == FORMULA && version == Version::Biff2
            || kind == BOOL_ERR && current.data.len() == 9
        {
            fixed::<1>(&mut data)?;
        }
        let value = match kind {
            LEGACY_INTEGER => Value::Number(f64::from(short(&mut data)?)),
            NUMBER => Value::Number(f64::from_le_bytes(fixed(&mut data)?)),
            RK => Value::Number(rk(fixed(&mut data)?)),
            LABEL_SST => {
                let index = integer(&mut data)? as usize;
                let text = strings
                    .get(index)
                    .ok_or_else(|| invalid("XLS cell has an invalid shared string index"))?;
                Value::Text(Cow::Borrowed(text))
            }
            LABEL | R_STRING => {
                let mut cursor = Segments {
                    data,
                    remaining: bytes,
                };
                let text_version = if current.kind == LEGACY_LABEL {
                    Version::Biff2
                } else {
                    version
                };
                let text = cursor.cell_text(text_version, codepage)?;
                bytes = cursor.remaining;
                Value::Text(Cow::Owned(text))
            }
            BOOL_ERR => {
                let [value, is_error] = fixed(&mut data)?;
                if is_error != 0 {
                    Value::Null
                } else {
                    Value::Boolean(value != 0)
                }
            }
            FORMULA => {
                if formula_string.is_some() {
                    return Err(invalid("XLS cached formula string is missing"));
                }
                let minimum = match version {
                    Version::Biff2 => 15,
                    Version::Biff3 | Version::Biff4 => 16,
                    _ => FORMULA_MINIMUM_BYTES,
                };
                if current.data.len() < minimum {
                    return Err(invalid("XLS formula record is truncated"));
                }
                let cached = fixed::<8>(&mut data)?;
                if cached[6..] != [0xff, 0xff] {
                    Value::Number(f64::from_le_bytes(cached))
                } else {
                    match cached[0] {
                        0 => {
                            formula_string = Some((row, column));
                            continue;
                        }
                        1 => Value::Boolean(cached[2] != 0),
                        2 => Value::Null,
                        3 => Value::Text(Cow::Borrowed("")),
                        _ => return Err(invalid("XLS formula cached type is invalid")),
                    }
                }
            }
            _ => unreachable!(),
        };
        range.include(row, column, version)?;
        on_cell(row, column, value)?;
    }
}

pub fn decode(bytes: &[u8]) -> AppResult<Workbook> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("XLS BIFF stream exceeds the file budget"));
    }
    let mut remaining = bytes;
    let (version, kind) = bof(&mut remaining)?;
    let mut codepage = WINDOWS_1252;
    if kind != WORKBOOK_GLOBALS && !version.is_standalone() {
        return Err(invalid(
            "native XLS standalone BIFF worksheet is not connected yet",
        ));
    }
    let mut names = Vec::new();
    let mut strings = Vec::new();
    if version.is_standalone() {
        if !matches!(kind, WORKSHEET | CHART | MACRO_SHEET) {
            return Err(invalid(
                "XLS standalone sheet has an unsupported substream type",
            ));
        }
        names.push(BoundSheet {
            offset: 0,
            name: "Sheet1".into(),
        });
        remaining = bytes;
    } else {
        loop {
            let current = record(&mut remaining)?;
            match current.kind {
                BOUND_SHEET => {
                    if names.len() >= MAX_SHEETS {
                        return Err(invalid("XLS has too many sheets"));
                    }
                    names.push(bound_sheet(current.data, version, codepage)?);
                }
                SST => strings = shared_strings(current.data, &mut remaining)?,
                CODE_PAGE => codepage = encoding(current.data)?,
                FILE_PASS => return Err(invalid("encrypted XLS workbooks are not supported")),
                EOF => break,
                _ => (),
            }
        }
    }
    let globals_end = bytes.len() - remaining.len();
    names.sort_by_key(|sheet| sheet.offset);
    if names
        .iter()
        .any(|sheet| sheet.offset < globals_end || sheet.offset >= bytes.len())
        || names
            .windows(2)
            .any(|pair| pair[0].offset == pair[1].offset)
    {
        return Err(invalid("XLS sheet offsets are invalid"));
    }
    let mut sheets = Vec::new();
    let mut used = 0usize;
    for (position, bound) in names.iter().enumerate() {
        let end = names
            .get(position + 1)
            .map_or(bytes.len(), |next| next.offset);
        let source = &bytes[bound.offset..end];
        let range = cells(source, &strings, version, codepage, |_, _, _| Ok(()))?;
        let total = if range.end_row == 0 || range.end_column == 0 {
            0
        } else {
            range.end_row - range.first_row
        };
        let height = total.min(MAX_PREVIEW_ROWS);
        let width = range.end_column - range.first_column;
        let grid = height
            .checked_mul(width)
            .and_then(|cells| cells.checked_mul(size_of::<Cell>()))
            .and_then(|bytes| {
                bytes.checked_add(
                    height * size_of::<Vec<Cell>>() + size_of::<Sheet>() + bound.name.capacity(),
                )
            })
            .filter(|bytes| used.saturating_add(*bytes) <= MAX_DECODED_BYTES)
            .ok_or_else(|| invalid("XLS grid exceeds the retained budget"))?;
        let mut rows = vec![vec![Cell::Null; width]; height];
        let mut text_bytes = 0usize;
        cells(source, &strings, version, codepage, |row, column, value| {
            if row < range.first_row
                || row - range.first_row >= height
                || column < range.first_column
                || column >= range.end_column
            {
                return Ok(());
            }
            let value = value.cell();
            let previous = &rows[row - range.first_row][column - range.first_column];
            text_bytes = text_bytes
                .saturating_add(value.text_bytes())
                .saturating_sub(previous.text_bytes());
            if used.saturating_add(grid).saturating_add(text_bytes) > MAX_DECODED_BYTES {
                return Err(invalid("XLS cell text exceeds the retained budget"));
            }
            rows[row - range.first_row][column - range.first_column] = value;
            Ok(())
        })?;
        used = used.saturating_add(grid).saturating_add(text_bytes);
        sheets.push(Sheet {
            name: bound.name.clone(),
            rows,
            total_row_count: total,
            truncated: total > MAX_PREVIEW_ROWS,
        });
    }
    let workbook = Workbook { sheets };
    if workbook.retained_bytes() > MAX_DECODED_BYTES {
        return Err(invalid("XLS workbook exceeds the retained budget"));
    }
    Ok(workbook)
}
