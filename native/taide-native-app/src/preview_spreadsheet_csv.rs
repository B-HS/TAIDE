use chrono::{Datelike, Days, Local, NaiveDate, TimeZone};
use num_bigint::BigUint;
use num_traits::ToPrimitive;
use taide_model::error::AppResult;

use crate::preview::invalid;
use crate::preview_spreadsheet::{
    Cell, MAX_DECODED_BYTES, MAX_EXCEL_COLUMNS, MAX_EXCEL_ROWS, MAX_PREVIEW_ROWS, Sheet, Workbook,
};

const SEPARATOR_SAMPLE_UNITS: usize = 1024;
const SEPARATORS: [char; 4] = ['|', ';', '\t', ','];
const PERCENT_FACTOR: f64 = 100.0;
const YEARS_PER_CENTURY: u32 = 100;
const MILLISECONDS_PER_DAY: f64 = 86_400_000.0;
const SHORT_YEAR_THRESHOLD: u32 = 50;
const CURRENT_CENTURY: u32 = 2000;
const PREVIOUS_CENTURY: u32 = 1900;
const DEFAULT_DATE_YEAR: u32 = 2001;
const MAX_DAY: u32 = 31;
const MIN_DATE_YEAR: i32 = 1900;
const MAX_DATE_YEAR: i32 = 9999;
const UTF16_UNIT_BYTES: usize = 2;
const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn js_whitespace(character: char) -> bool {
    (character != '\u{85}' && character.is_whitespace()) || character == '\u{feff}'
}

pub(crate) fn numeric(text: &str) -> Option<f64> {
    let text = text.trim_matches(js_whitespace);
    if text.is_empty() {
        return Some(0.0);
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0b", 2),
        ("0B", 2),
        ("0o", 8),
        ("0O", 8),
    ] {
        if let Some(digits) = text.strip_prefix(prefix) {
            if digits.is_empty() || digits.bytes().any(|byte| !char::from(byte).is_digit(radix)) {
                return None;
            }
            return BigUint::parse_bytes(digits.as_bytes(), radix)?.to_f64();
        }
    }
    if text
        .bytes()
        .any(|byte| !matches!(byte, b'0'..=b'9' | b'+' | b'-' | b'.' | b'e' | b'E'))
    {
        return None;
    }
    text.parse::<f64>().ok()
}

pub(crate) fn fuzzy_number(text: &str) -> Option<f64> {
    if let Some(value) = numeric(text) {
        return value.is_finite().then_some(value);
    }
    if !text.bytes().any(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut digits = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        digits.push(character);
        if character.is_ascii_digit() && characters.peek() == Some(&',') {
            let mut after = characters.clone();
            after.next();
            if after.peek().is_some_and(char::is_ascii_digit) {
                characters.next();
                digits.push(characters.next()?);
            }
        }
    }
    let mut weight = 1.0;
    let mut transformed = String::with_capacity(digits.len());
    for character in digits.chars() {
        match character {
            '$' => (),
            '%' => weight *= PERCENT_FACTOR,
            _ => transformed.push(character),
        }
    }
    if let Some(value) = numeric(&transformed) {
        return Some(value / weight);
    }
    let start = transformed.find('(')?;
    let remainder = &transformed[start + 1..];
    let end = remainder.rfind(')')?;
    if remainder[..end].contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    let transformed = format!(
        "{}{}{}",
        &transformed[..start],
        &remainder[..end],
        &remainder[end + 1..]
    );
    numeric(&transformed).map(|value| value / -weight)
}

pub(crate) fn fuzzy_date(text: &str, timezone: &impl TimeZone) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    let letters = lower
        .chars()
        .filter(char::is_ascii_alphabetic)
        .collect::<String>();
    let month = if letters.is_empty() {
        None
    } else {
        let index = MONTHS.iter().position(|month| {
            letters == *month || (letters.len() == 3 && month.starts_with(&letters))
        })?;
        Some(u32::try_from(index + 1).ok()?)
    };
    if text.chars().any(|character| {
        !character.is_ascii_alphabetic()
            && !character.is_ascii_digit()
            && !js_whitespace(character)
            && !matches!(character, '-' | '/' | '\\' | ',' | '.' | ':')
    }) {
        return None;
    }
    let mut values = text
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(str::parse::<u32>);
    let first = values.next()?.ok()?;
    let second = values.next()?.ok()?;
    let third = values.next().transpose().ok()?;
    let (year, month, day, third_is_time) = if let Some(month) = month {
        if first >= PREVIOUS_CENTURY {
            (first, month, second, third)
        } else {
            (second, month, first, third)
        }
    } else if first > MAX_DAY {
        (first, second, third.unwrap_or(1), None)
    } else {
        (third.unwrap_or(DEFAULT_DATE_YEAR), first, second, None)
    };
    let year = if year < SHORT_YEAR_THRESHOLD {
        CURRENT_CENTURY + year
    } else if year < YEARS_PER_CENTURY {
        PREVIOUS_CENTURY + year
    } else {
        year
    };
    let year = i32::try_from(year).ok()?;
    if !(MIN_DATE_YEAR..=MAX_DATE_YEAR).contains(&year) || day == 0 || day > MAX_DAY {
        return None;
    }
    let date =
        NaiveDate::from_ymd_opt(year, month, 1)?.checked_add_days(Days::new(u64::from(day - 1)))?;
    let hour = match third_is_time {
        Some(hour) => hour,
        None => values.next().transpose().ok()?.unwrap_or_default(),
    };
    let minute = values.next().transpose().ok()?.unwrap_or_default();
    let second = values.next().transpose().ok()?.unwrap_or_default();
    if values.next().is_some() {
        return None;
    }
    let date = date.and_hms_opt(hour, minute, second)?;
    let is_iso_date =
        (text.len() == 10 && text.as_bytes()[4] == b'-' && text.as_bytes()[7] == b'-')
            || (text.len() == 7 && text.as_bytes()[4] == b'-');
    let date = if is_iso_date {
        timezone.from_utc_datetime(&date)
    } else {
        timezone.from_local_datetime(&date).earliest()?
    };
    if ((date.month0() == 0 && date.day() == 1) || date.year() == DEFAULT_DATE_YEAR as i32)
        && text.chars().any(|character| {
            !character.is_ascii_digit() && !matches!(character, '-' | ':' | ',' | '/' | '\\')
        })
    {
        return None;
    }
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)?.and_hms_opt(0, 0, 0)?;
    Some(
        date.naive_local()
            .signed_duration_since(epoch)
            .num_milliseconds() as f64
            / MILLISECONDS_PER_DAY,
    )
}

fn cell(text: &str, timezone: &impl TimeZone) -> Cell {
    let text = if text.starts_with('"') && text.ends_with('"') {
        if text.len() < UTF16_UNIT_BYTES {
            String::new()
        } else {
            text[1..text.len() - 1].replace("\"\"", "\"")
        }
    } else {
        text.into()
    };
    if text.is_empty() {
        return Cell::Null;
    }
    if text.trim_matches(js_whitespace).is_empty() {
        return Cell::Text(text);
    }
    if text.starts_with('=') {
        if text.starts_with("=\"") && text.ends_with('"') {
            if text.len() <= 3 {
                return Cell::Text(String::new());
            }
            return Cell::Text(text[2..text.len() - 1].replace("\"\"", "\""));
        }
        return if text.len() > 1 {
            Cell::Null
        } else {
            Cell::Text(text)
        };
    }
    match text.as_str() {
        "TRUE" => Cell::Boolean(true),
        "FALSE" => Cell::Boolean(false),
        _ => fuzzy_number(&text)
            .or_else(|| fuzzy_date(&text, timezone))
            .map_or_else(|| Cell::Text(text), Cell::Number),
    }
}

fn source(bytes: &[u8]) -> String {
    if let Some(bytes) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xff, 0xfe]) {
        let units = bytes
            .as_chunks::<UTF16_UNIT_BYTES>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]));
        return char::decode_utf16(units)
            .map(|character| character.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
    }
    bytes.iter().copied().map(char::from).collect()
}

fn separator(text: &str) -> (char, &str) {
    if let Some(rest) = text.strip_prefix("sep=") {
        let mut characters = rest.chars();
        if let Some(separator) = characters.next() {
            let rest = characters.as_str();
            if let Some(rest) = rest
                .strip_prefix("\r\n")
                .or_else(|| rest.strip_prefix(['\r', '\n']))
            {
                return (separator, rest);
            }
        }
    }
    let mut counts = [0usize; SEPARATORS.len()];
    let mut quoted = false;
    for unit in text.encode_utf16().take(SEPARATOR_SAMPLE_UNITS) {
        let character = char::from_u32(u32::from(unit));
        if character == Some('"') {
            quoted = !quoted;
            continue;
        }
        if !quoted
            && let Some(index) = SEPARATORS
                .iter()
                .position(|separator| Some(*separator) == character)
        {
            counts[index] += 1;
        }
    }
    let index = (0..SEPARATORS.len())
        .max_by_key(|index| (counts[*index], *index))
        .unwrap_or_default();
    (SEPARATORS[index], text)
}

fn scan(
    text: &str,
    separator: char,
    mut handle_cell: impl FnMut(usize, usize, &str) -> AppResult<()>,
) -> AppResult<(usize, usize)> {
    let mut row = 0usize;
    let mut column = 0usize;
    let mut total_row_count = 1usize;
    let mut width = 1usize;
    let mut start = 0usize;
    let mut quoted = false;
    let mut finish = |end: usize, is_separator: bool, next: usize| -> AppResult<()> {
        if next == end && end <= start {
            return Ok(());
        }
        if row >= MAX_EXCEL_ROWS as usize || column >= MAX_EXCEL_COLUMNS as usize {
            return Err(invalid("CSV dimensions exceed Excel bounds"));
        }
        total_row_count = total_row_count.max(row + 1);
        width = width.max(column + 1);
        handle_cell(row, column, &text[start..end])?;
        start = next;
        if is_separator {
            column += 1;
        } else {
            column = 0;
            row += 1;
        }
        Ok(())
    };
    let mut starts_quoted = text.starts_with('"');
    for (position, character) in text.char_indices() {
        if character == '"' && starts_quoted {
            quoted = !quoted;
            continue;
        }
        if !quoted && (character == separator || character == '\n' || character == '\r') {
            let next = position + character.len_utf8();
            finish(position, character == separator, next)?;
            starts_quoted = text[next..].starts_with('"');
        }
    }
    if let Some((position, character)) = text.char_indices().next_back() {
        let end = position + character.len_utf8();
        finish(end, false, end)?;
    }
    Ok((total_row_count, width))
}

pub fn decode(bytes: &[u8]) -> AppResult<Workbook> {
    decode_in_timezone(bytes, &Local)
}

pub fn decode_in_timezone(bytes: &[u8], timezone: &impl TimeZone) -> AppResult<Workbook> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded CSV exceeds the file preview budget"));
    }
    let text = source(bytes);
    let (separator, text) = separator(&text);
    let text = text.replace("\r\n", "\n");
    let (total_row_count, width) = scan(&text, separator, |_, _, _| Ok(()))?;
    let height = total_row_count.min(MAX_PREVIEW_ROWS);
    let grid_bytes = height
        .checked_mul(width)
        .and_then(|cells| cells.checked_mul(size_of::<Cell>()))
        .and_then(|bytes| bytes.checked_add(height * size_of::<Vec<Cell>>()))
        .and_then(|bytes| bytes.checked_add(size_of::<Sheet>()))
        .filter(|bytes| *bytes <= MAX_DECODED_BYTES)
        .ok_or_else(|| invalid("CSV grid exceeds the retained budget"))?;
    let mut rows = vec![vec![Cell::Null; width]; height];
    let mut text_bytes = 0usize;
    scan(&text, separator, |row, column, text| {
        if row >= height {
            return Ok(());
        }
        let value = cell(text, timezone);
        text_bytes = text_bytes.saturating_add(value.text_bytes());
        if grid_bytes
            .checked_add(text_bytes)
            .is_none_or(|bytes| bytes > MAX_DECODED_BYTES)
        {
            return Err(invalid("CSV cells exceed the retained budget"));
        }
        rows[row][column] = value;
        Ok(())
    })?;
    let workbook = Workbook {
        sheets: vec![Sheet {
            name: "Sheet1".into(),
            rows,
            total_row_count,
            truncated: total_row_count > MAX_PREVIEW_ROWS,
        }],
    };
    if workbook.retained_bytes() > MAX_DECODED_BYTES {
        return Err(invalid("CSV workbook exceeds the retained budget"));
    }
    Ok(workbook)
}
