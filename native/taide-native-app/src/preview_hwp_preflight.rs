use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{Cursor, Read};

use flate2::read::{DeflateDecoder, ZlibDecoder};
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use rhwp::model::style::{BorderFill, CharShape, Font, ParaShape, Style, TabDef};
use rhwp::{
    DocumentCore,
    parser::{
        FileFormat, ParsedDocument, detect_format, hml,
        hwp3::{
            Hwp3Error, Hwp3Limits, Hwp3Payload, parse_hwp3_with_validation, records::Hwp3DocInfo,
        },
    },
};
use taide_model::error::AppResult;
use zip::{CompressionMethod, ZipArchive};

use crate::preview::invalid;

pub const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_NODES: usize = 262_144;
pub const MAX_DEPTH: usize = 128;
pub const MAX_EMBEDDED_DEPTH: usize = 16;
pub const MAX_TABLE_GRID_SLOTS: usize = 262_144;
const MAX_NAME_BYTES: usize = 4096;
const MAX_XML_NAME_BYTES: usize = 256;
const MAX_ATTRIBUTES: usize = 256;
const MAX_NAME_TOTAL: usize = 1024 * 1024;
const STREAM_BUFFER_BYTES: usize = 64 * 1024;
const HWP_HEADER_BYTES: usize = 256;
const HWP3_SIGNATURE_BYTES: usize = 30;
const HWP_FLAGS_OFFSET: usize = 36;
const HWP_COMPRESSED: u32 = 0x01;
const HWP_ENCRYPTED: u32 = 0x02;
const HWP_DISTRIBUTION: u32 = 0x04;
const DISTRIBUTE_DATA_BYTES: usize = 256;
const RECORD_LEVEL_SHIFT: u32 = 10;
const RECORD_SIZE_SHIFT: u32 = 20;
const RECORD_LEVEL_MASK: u32 = 0x3ff;
const RECORD_EXTENDED_SIZE: u32 = 0xfff;
const TABLE_ROWS_OFFSET: usize = size_of::<u32>();
const TABLE_COLUMNS_OFFSET: usize = TABLE_ROWS_OFFSET + size_of::<u16>();
const CFB_SIGNATURE: &[u8] = &[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
const ZIP_SIGNATURE: &[u8] = b"PK\x03\x04";
const ZIP_END_SIGNATURE: &[u8] = b"PK\x05\x06";
const ZIP_CENTRAL_SIGNATURE: &[u8] = b"PK\x01\x02";
const ZIP_END_BYTES: usize = 22;
const ZIP_CENTRAL_BYTES: usize = 46;
const ZIP_DISK_OFFSET: usize = 4;
const ZIP_CENTRAL_DISK_OFFSET: usize = 6;
const ZIP_DISK_COUNT_OFFSET: usize = 8;
const ZIP_COUNT_OFFSET: usize = 10;
const ZIP_CENTRAL_SIZE_OFFSET: usize = 12;
const ZIP_CENTRAL_START_OFFSET: usize = 16;
const ZIP_COMMENT_LENGTH_OFFSET: usize = 20;
const ZIP_ENTRY_SIZE_OFFSET: usize = 24;
const ZIP_NAME_LENGTH_OFFSET: usize = 28;
const ZIP_EXTRA_LENGTH_OFFSET: usize = 30;
const ZIP_ENTRY_COMMENT_OFFSET: usize = 32;
const ZIP_ENTRY_DISK_OFFSET: usize = 34;
const HML_LANGUAGES: [&str; 7] = [
    "Hangul", "Latin", "Hanja", "Japanese", "Other", "Symbol", "User",
];
const HML_RESOURCE_TABLES: usize = HML_LANGUAGES.len() + 5;
const HML_RESOURCE_SIZES: [usize; HML_RESOURCE_TABLES] = [
    size_of::<Font>(),
    size_of::<Font>(),
    size_of::<Font>(),
    size_of::<Font>(),
    size_of::<Font>(),
    size_of::<Font>(),
    size_of::<Font>(),
    size_of::<BorderFill>(),
    size_of::<CharShape>(),
    size_of::<ParaShape>(),
    size_of::<TabDef>(),
    size_of::<Style>(),
];

#[derive(Default)]
struct HmlAdmission {
    font_language: Option<usize>,
    resource_slots: [usize; HML_RESOURCE_TABLES],
    ancestors: Vec<usize>,
    ancestor_bytes: usize,
}

impl HmlAdmission {
    fn element(
        &mut self,
        name: &[u8],
        id: Option<&str>,
        language: Option<&str>,
        is_start: bool,
        budget: &mut Budget,
    ) -> AppResult<()> {
        budget.charge(self.ancestor_bytes + name.len() + self.ancestors.len() + 1)?;
        if is_start {
            self.ancestors.push(name.len());
            self.ancestor_bytes += name.len();
        }
        if name == b"FONTFACE" {
            self.font_language = language
                .and_then(|language| HML_LANGUAGES.iter().position(|value| *value == language));
        }
        let table = match name {
            b"FONT" => self.font_language,
            b"BORDERFILL" => Some(HML_LANGUAGES.len()),
            b"CHARSHAPE" => Some(HML_LANGUAGES.len() + 1),
            b"PARASHAPE" => Some(HML_LANGUAGES.len() + 2),
            b"TABDEF" => Some(HML_LANGUAGES.len() + 3),
            b"STYLE" => Some(HML_LANGUAGES.len() + 4),
            _ => None,
        };
        if let Some(table) = table {
            let default_id = usize::from(name == b"BORDERFILL");
            let id = id
                .map(str::parse::<usize>)
                .transpose()
                .map_err(|_| invalid("HML resource Id is invalid"))?
                .unwrap_or(default_id);
            let index = id
                .checked_sub(default_id)
                .ok_or_else(|| invalid("HML BORDERFILL Id is invalid"))?;
            if index <= hml::HmlLimits::default().max_resource_id {
                let slots = index + 1;
                if slots > self.resource_slots[table] {
                    budget
                        .charge((slots - self.resource_slots[table]) * HML_RESOURCE_SIZES[table])?;
                    self.resource_slots[table] = slots;
                }
            }
        }
        if !is_start && name == b"FONTFACE" {
            self.font_language = None;
        }
        Ok(())
    }

    fn end(&mut self, name: &[u8]) {
        if let Some(length) = self.ancestors.pop() {
            self.ancestor_bytes -= length;
        }
        if name == b"FONTFACE" {
            self.font_language = None;
        }
    }
}

#[derive(Default)]
struct Budget {
    decoded: usize,
    entries: usize,
    nodes: usize,
    names: usize,
    table_grid_slots: usize,
}

impl Budget {
    fn charge(&mut self, bytes: usize) -> AppResult<()> {
        self.decoded = self
            .decoded
            .checked_add(bytes)
            .filter(|total| *total <= MAX_DECODED_BYTES)
            .ok_or_else(|| invalid("HWP exceeds the aggregate decoded budget"))?;
        Ok(())
    }

    fn entry(&mut self, name: &str) -> AppResult<()> {
        self.entries += 1;
        if self.entries > MAX_ENTRIES {
            return Err(invalid(
                "HWP container entry count exceeds the preview budget",
            ));
        }
        self.name(name)
    }

    fn name(&mut self, name: &str) -> AppResult<()> {
        self.names = self.names.saturating_add(name.len());
        if self.names > MAX_NAME_TOTAL
            || name.len() > MAX_NAME_BYTES
            || name.contains('\0')
            || name.split('/').count() > MAX_DEPTH
        {
            return Err(invalid("HWP container metadata exceeds the preview budget"));
        }
        Ok(())
    }

    fn node(&mut self) -> AppResult<()> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(invalid(
                "HWP record or XML node count exceeds the preview budget",
            ));
        }
        Ok(())
    }

    fn remaining(&self) -> usize {
        MAX_DECODED_BYTES - self.decoded
    }

    fn table(&mut self, rows: u16, columns: u16) -> AppResult<()> {
        let slots = (usize::from(rows.max(1)) + 1)
            .checked_mul(usize::from(columns.max(1)) + 1)
            .ok_or_else(|| invalid("HWP table grid dimensions overflow"))?;
        self.table_grid_slots = self
            .table_grid_slots
            .checked_add(slots)
            .filter(|total| *total <= MAX_TABLE_GRID_SLOTS)
            .ok_or_else(|| invalid("HWP aggregate table grid exceeds the preview budget"))?;
        Ok(())
    }
}

fn short(bytes: &[u8], offset: usize) -> AppResult<u16> {
    let data = bytes
        .get(offset..offset.saturating_add(size_of::<u16>()))
        .and_then(|data| data.try_into().ok())
        .ok_or_else(|| invalid("HWP container header is truncated"))?;
    Ok(u16::from_le_bytes(data))
}

fn integer(bytes: &[u8], offset: usize) -> AppResult<u32> {
    let data = bytes
        .get(offset..offset.saturating_add(size_of::<u32>()))
        .and_then(|data| data.try_into().ok())
        .ok_or_else(|| invalid("HWP container or record header is truncated"))?;
    Ok(u32::from_le_bytes(data))
}

fn read_limited(reader: impl Read, max: usize) -> AppResult<Vec<u8>> {
    let mut output = Vec::new();
    let result = reader.take(max as u64 + 1).read_to_end(&mut output);
    if output.len() > max {
        return Err(invalid("HWP stream exceeds the aggregate decoded budget"));
    }
    result.map_err(|_| invalid("HWP container stream is invalid"))?;
    Ok(output)
}

fn inflate(bytes: &[u8], max: usize) -> AppResult<Option<Vec<u8>>> {
    for reader in [
        Box::new(DeflateDecoder::new(bytes)) as Box<dyn Read>,
        Box::new(ZlibDecoder::new(bytes)) as Box<dyn Read>,
    ] {
        let mut output = Vec::new();
        let result = reader.take(max as u64 + 1).read_to_end(&mut output);
        if output.len() > max {
            return Err(invalid("HWP compressed stream exceeds the decoded budget"));
        }
        if result.is_ok() {
            return Ok(Some(output));
        }
    }
    Ok(None)
}

fn records(bytes: &[u8], budget: &mut Budget, allow_short_tail: bool) -> AppResult<()> {
    let mut offset = 0usize;
    while offset < bytes.len() {
        if allow_short_tail && bytes.len() - offset < size_of::<u32>() {
            break;
        }
        budget.node()?;
        let header = integer(bytes, offset)?;
        offset += size_of::<u32>();
        if ((header >> RECORD_LEVEL_SHIFT) & RECORD_LEVEL_MASK) as usize > MAX_DEPTH {
            return Err(invalid("HWP record nesting exceeds the preview budget"));
        }
        let mut length = header >> RECORD_SIZE_SHIFT;
        if length == RECORD_EXTENDED_SIZE {
            length = integer(bytes, offset)?;
            offset += size_of::<u32>();
        }
        let end = offset
            .checked_add(length as usize)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| invalid("HWP record payload is truncated"))?;
        if header & RECORD_LEVEL_MASK == u32::from(rhwp::parser::tags::HWPTAG_TABLE) {
            let data = &bytes[offset..end];
            budget.table(
                short(data, TABLE_ROWS_OFFSET)?,
                short(data, TABLE_COLUMNS_OFFSET)?,
            )?;
        }
        offset = end;
    }
    Ok(())
}

fn viewtext(bytes: &[u8], compressed: bool, max_decoded: usize) -> AppResult<Vec<u8>> {
    let header = integer(bytes, 0)?;
    let tag = header & RECORD_LEVEL_MASK;
    let mut length = header >> RECORD_SIZE_SHIFT;
    let header_bytes = if length == RECORD_EXTENDED_SIZE {
        length = integer(bytes, size_of::<u32>())?;
        size_of::<u32>() * 2
    } else {
        size_of::<u32>()
    };
    if tag != u32::from(rhwp::parser::tags::HWPTAG_DISTRIBUTE_DOC_DATA)
        || length as usize != DISTRIBUTE_DATA_BYTES
        || bytes.len() <= header_bytes + DISTRIBUTE_DATA_BYTES
    {
        return Err(invalid(
            "HWP distribution header or encrypted body is invalid",
        ));
    }
    let decrypted = rhwp::parser::crypto::decrypt_viewtext_section(bytes, false)
        .map_err(|_| invalid("HWP distribution body cannot be decrypted"))?;
    if compressed {
        return inflate(&decrypted, max_decoded)?
            .ok_or_else(|| invalid("HWP distribution body compression is invalid"));
    }
    if decrypted.len() > max_decoded {
        return Err(invalid("HWP distribution body exceeds the decoded budget"));
    }
    Ok(decrypted)
}

fn xml(bytes: &[u8], budget: &mut Budget, is_hml: bool) -> AppResult<HashSet<String>> {
    let text = String::from_utf8_lossy(bytes);
    let mut reader = Reader::from_str(&text);
    reader.config_mut().enable_all_checks(true);
    let mut depth = 0usize;
    let mut roots = 0usize;
    let mut references = HashSet::new();
    let mut hml_admission = HmlAdmission::default();
    loop {
        let event = reader
            .read_event()
            .map_err(|_| invalid("HWPX XML structure is invalid"))?;
        if matches!(event, Event::Eof) {
            if depth != 0 || roots != 1 {
                return Err(invalid("HWPX XML has an incomplete document root"));
            }
            return Ok(references);
        }
        budget.node()?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if depth == 0 {
                    roots += 1;
                }
                if depth >= MAX_DEPTH || element.name().as_ref().len() > MAX_XML_NAME_BYTES {
                    return Err(invalid(
                        "HWPX XML nesting or name exceeds the preview budget",
                    ));
                }
                let mut href = None;
                let mut is_xml = false;
                let is_table = if is_hml {
                    element.name().as_ref() == b"TABLE"
                } else {
                    element.local_name().as_ref() == b"tbl"
                };
                let mut table_rows = 0;
                let mut table_columns = 0;
                let mut resource_id = None;
                let mut language = None;
                for (index, attribute) in element.attributes().enumerate() {
                    if index >= MAX_ATTRIBUTES {
                        return Err(invalid("HWPX XML has too many attributes"));
                    }
                    let attribute =
                        attribute.map_err(|_| invalid("HWPX XML attribute is invalid"))?;
                    if attribute.key.as_ref().len() > MAX_XML_NAME_BYTES {
                        return Err(invalid(
                            "HWPX XML attribute name exceeds the preview budget",
                        ));
                    }
                    let value = if is_hml {
                        let raw = std::str::from_utf8(&attribute.value)
                            .map_err(|_| invalid("HML XML attribute encoding is invalid"))?;
                        quick_xml::escape::unescape(raw)
                            .map_err(|_| invalid("HML XML attribute entity is invalid"))?
                    } else {
                        attribute
                            .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                            .map_err(|_| invalid("HWPX XML attribute entity is invalid"))?
                    };
                    if is_table {
                        match (is_hml, attribute.key.as_ref()) {
                            (false, b"rowCnt") | (true, b"RowCount") => {
                                table_rows = value
                                    .parse::<u16>()
                                    .map_err(|_| invalid("HWPX table row count is invalid"))?
                            }
                            (false, b"colCnt") | (true, b"ColCount") => {
                                table_columns = value
                                    .parse::<u16>()
                                    .map_err(|_| invalid("HWPX table column count is invalid"))?
                            }
                            _ => {}
                        }
                    }
                    if is_hml {
                        match attribute.key.as_ref() {
                            b"Id" => resource_id = Some(value.into_owned()),
                            b"Lang" => language = Some(value.into_owned()),
                            _ => {}
                        }
                    }
                    if !is_hml && element.local_name().as_ref() == b"item" {
                        match attribute.key.as_ref() {
                            b"href" => {
                                href = Some(String::from_utf8_lossy(&attribute.value).into_owned())
                            }
                            b"media-type" => {
                                is_xml = attribute.value.as_ref() == b"application/xml"
                            }
                            _ => {}
                        }
                    }
                }
                if is_table {
                    budget.table(table_rows, table_columns)?;
                }
                if is_hml {
                    hml_admission.element(
                        element.name().as_ref(),
                        resource_id.as_deref(),
                        language.as_deref(),
                        matches!(event, Event::Start(_)),
                        budget,
                    )?;
                }
                if is_xml && let Some(href) = href {
                    budget.name(&href)?;
                    references.insert(href);
                    if references.len() > MAX_ENTRIES {
                        return Err(invalid(
                            "HWPX XML reference count exceeds the preview budget",
                        ));
                    }
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(element) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("HWPX XML has an unmatched end tag"))?;
                if is_hml {
                    hml_admission.end(element.name().as_ref());
                }
            }
            Event::DocType(_) => return Err(invalid("HWPX DTD declarations are not accepted")),
            Event::GeneralRef(reference) => {
                if depth == 0 {
                    return Err(invalid("HWPX XML has text outside its root"));
                }
                let reference = reference
                    .decode()
                    .map_err(|_| invalid("HWPX XML entity is invalid"))?;
                if reference.starts_with('#') {
                    quick_xml::escape::unescape(&format!("&{reference};"))
                        .map_err(|_| invalid("HWPX XML numeric entity is invalid"))?;
                } else if !matches!(reference.as_ref(), "amp" | "lt" | "gt" | "apos" | "quot") {
                    return Err(invalid(
                        "HWPX XML external or unknown entity is not accepted",
                    ));
                }
            }
            Event::Text(text) if depth == 0 => {
                if text.iter().any(|byte| !byte.is_ascii_whitespace()) {
                    return Err(invalid("HWPX XML has text outside its root"));
                }
            }
            Event::CData(_) if depth == 0 => {
                return Err(invalid("HWPX XML has text outside its root"));
            }
            Event::Text(text)
                if is_hml && text.len() > hml::HmlLimits::default().max_text_node_bytes =>
            {
                return Err(invalid("HML text node exceeds the preview budget"));
            }
            Event::CData(text)
                if is_hml && text.len() > hml::HmlLimits::default().max_text_node_bytes =>
            {
                return Err(invalid("HML text node exceeds the preview budget"));
            }
            _ => {}
        }
    }
}

fn zip_directory(bytes: &[u8], budget: &Budget) -> AppResult<usize> {
    let minimum = bytes
        .len()
        .saturating_sub(ZIP_END_BYTES + u16::MAX as usize);
    let end = (minimum..=bytes.len().saturating_sub(ZIP_END_BYTES))
        .rev()
        .find(|offset| {
            bytes.get(*offset..offset.saturating_add(ZIP_END_SIGNATURE.len()))
                == Some(ZIP_END_SIGNATURE)
                && short(bytes, *offset + ZIP_COMMENT_LENGTH_OFFSET)
                    .is_ok_and(|length| *offset + ZIP_END_BYTES + length as usize == bytes.len())
        })
        .ok_or_else(|| invalid("HWPX ZIP end record is missing"))?;
    let count = short(bytes, end + ZIP_COUNT_OFFSET)? as usize;
    if short(bytes, end + ZIP_DISK_OFFSET)? != 0
        || short(bytes, end + ZIP_CENTRAL_DISK_OFFSET)? != 0
        || short(bytes, end + ZIP_DISK_COUNT_OFFSET)? as usize != count
        || count == u16::MAX as usize
        || count > MAX_ENTRIES - budget.entries
    {
        return Err(invalid(
            "HWPX ZIP entry count, ZIP64 or split layout is not accepted",
        ));
    }
    let size = integer(bytes, end + ZIP_CENTRAL_SIZE_OFFSET)? as usize;
    let mut offset = integer(bytes, end + ZIP_CENTRAL_START_OFFSET)? as usize;
    let directory_end = offset
        .checked_add(size)
        .filter(|directory_end| *directory_end == end)
        .ok_or_else(|| invalid("HWPX ZIP central directory boundary is invalid"))?;
    let mut decoded = 0usize;
    for _ in 0..count {
        if bytes.get(offset..offset.saturating_add(ZIP_CENTRAL_SIGNATURE.len()))
            != Some(ZIP_CENTRAL_SIGNATURE)
            || short(bytes, offset + ZIP_ENTRY_DISK_OFFSET)? != 0
        {
            return Err(invalid("HWPX ZIP central entry is invalid"));
        }
        decoded = decoded
            .checked_add(integer(bytes, offset + ZIP_ENTRY_SIZE_OFFSET)? as usize)
            .filter(|total| *total <= budget.remaining())
            .ok_or_else(|| {
                invalid("HWPX ZIP declared data exceeds the aggregate decoded budget")
            })?;
        let name = short(bytes, offset + ZIP_NAME_LENGTH_OFFSET)? as usize;
        if name > MAX_NAME_BYTES {
            return Err(invalid("HWPX ZIP entry name exceeds the preview budget"));
        }
        let extra = short(bytes, offset + ZIP_EXTRA_LENGTH_OFFSET)? as usize;
        let comment = short(bytes, offset + ZIP_ENTRY_COMMENT_OFFSET)? as usize;
        offset = offset
            .checked_add(ZIP_CENTRAL_BYTES)
            .and_then(|offset| offset.checked_add(name))
            .and_then(|offset| offset.checked_add(extra))
            .and_then(|offset| offset.checked_add(comment))
            .filter(|offset| *offset <= directory_end)
            .ok_or_else(|| invalid("HWPX ZIP central entry is truncated"))?;
    }
    if offset != directory_end {
        return Err(invalid("HWPX ZIP central entry count is inconsistent"));
    }
    Ok(count)
}

fn embedded(bytes: &[u8], budget: &mut Budget, depth: usize) -> AppResult<()> {
    let bytes = if bytes
        .get(size_of::<u32>()..)
        .is_some_and(|bytes| bytes.starts_with(CFB_SIGNATURE))
    {
        &bytes[size_of::<u32>()..]
    } else {
        bytes
    };
    if !bytes.starts_with(CFB_SIGNATURE) && !bytes.starts_with(ZIP_SIGNATURE) {
        return Ok(());
    }
    if depth >= MAX_EMBEDDED_DEPTH {
        return Err(invalid(
            "HWP embedded container nesting exceeds the preview budget",
        ));
    }
    if bytes.starts_with(CFB_SIGNATURE) {
        compound(bytes, budget, depth + 1, false)?;
    } else {
        archive(bytes, budget, depth + 1)?;
    }
    Ok(())
}

fn archive(bytes: &[u8], budget: &mut Budget, depth: usize) -> AppResult<()> {
    let count = zip_directory(bytes, budget)?;
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid("HWPX ZIP archive is invalid"))?;
    if archive.len() != count {
        return Err(invalid("HWPX ZIP has duplicate or inconsistent entries"));
    }
    let mut names = HashSet::with_capacity(count);
    let manifest =
        (0..count).find(|index| archive.name_for_index(*index) == Some("Contents/content.hpf"));
    let indices = manifest
        .into_iter()
        .chain((0..count).filter(|index| Some(*index) != manifest));
    let mut references = HashSet::new();
    for index in indices {
        let entry = archive
            .by_index(index)
            .map_err(|_| invalid("HWPX ZIP entry is invalid"))?;
        let name = entry.name().to_owned();
        budget.entry(&name)?;
        if !names.insert(name.clone()) {
            return Err(invalid("HWPX ZIP has duplicate entry names"));
        }
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(invalid("HWPX ZIP compression is not supported"));
        }
        if entry.size() > budget.remaining() as u64 {
            return Err(invalid(
                "HWPX ZIP data exceeds the aggregate decoded budget",
            ));
        }
        let length = entry.size();
        let data = read_limited(entry, budget.remaining())?;
        if data.len() as u64 != length {
            return Err(invalid("HWPX ZIP entry length is inconsistent"));
        }
        budget.charge(data.len())?;
        let lower_name = name.to_ascii_lowercase();
        if lower_name.ends_with(".xml")
            || lower_name.ends_with(".hpf")
            || references.contains(&name)
        {
            let found = xml(&data, budget, false)?;
            if Some(index) == manifest {
                references = found;
            }
        }
        embedded(&data, budget, depth)?;
    }
    Ok(())
}

fn compound<'a>(
    bytes: &'a [u8],
    budget: &mut Budget,
    depth: usize,
    is_document: bool,
) -> AppResult<Cow<'a, [u8]>> {
    let normalized = crate::preview_spreadsheet_xls::preflight(bytes)?;
    let mut file = cfb::OpenOptions::new()
        .max_buffer_size(STREAM_BUFFER_BYTES)
        .open_with(Cursor::new(normalized.as_ref()))
        .map_err(|_| invalid("HWP CFB container is invalid"))?;
    let mut compressed = false;
    let mut distribution = false;
    if is_document {
        let header = file
            .entry("/FileHeader")
            .map_err(|_| invalid("HWP FileHeader is missing"))?;
        if !header.is_stream() || header.len() > bytes.len() as u64 {
            return Err(invalid("HWP FileHeader length is invalid"));
        }
        let data = read_limited(
            file.open_stream("/FileHeader")
                .map_err(|_| invalid("HWP FileHeader cannot be read"))?,
            bytes.len(),
        )?;
        if data.len() as u64 != header.len()
            || data.len() < HWP_HEADER_BYTES
            || !data.starts_with(b"HWP Document File")
        {
            return Err(invalid("HWP FileHeader signature or length is invalid"));
        }
        let flags = integer(&data, HWP_FLAGS_OFFSET)?;
        if flags & HWP_ENCRYPTED != 0 {
            return Err(invalid("HWP encrypted document is not supported"));
        }
        distribution = flags & HWP_DISTRIBUTION != 0;
        compressed = flags & HWP_COMPRESSED != 0;
        if !file.is_stream("/DocInfo") {
            return Err(invalid("HWP DocInfo stream is missing"));
        }
        if distribution {
            let mut index = 0usize;
            while file.is_stream(format!("/BodyText/Section{index}"))
                || file.is_stream(format!("/ViewText/Section{index}"))
                || file.is_stream(format!("/Section{index}"))
            {
                if index >= MAX_ENTRIES || !file.is_stream(format!("/ViewText/Section{index}")) {
                    return Err(invalid(
                        "HWP distribution section is missing or exceeds the entry budget",
                    ));
                }
                index += 1;
            }
        }
    }
    let mut entries = Vec::new();
    let mut raw_total = 0usize;
    for entry in file.walk() {
        let name = entry
            .path()
            .to_str()
            .ok_or_else(|| invalid("HWP CFB path is invalid"))?;
        budget.entry(name)?;
        if !entry.is_stream() {
            continue;
        }
        let length = usize::try_from(entry.len())
            .ok()
            .filter(|length| *length <= bytes.len() && *length <= budget.remaining())
            .ok_or_else(|| invalid("HWP CFB stream length exceeds the preview budget"))?;
        raw_total = raw_total
            .checked_add(length)
            .filter(|total| *total <= MAX_DECODED_BYTES)
            .ok_or_else(|| invalid("HWP CFB raw streams exceed the aggregate budget"))?;
        entries.push((entry.path().to_owned(), length));
    }
    for (path, length) in entries {
        let stream = file
            .open_stream(&path)
            .map_err(|_| invalid("HWP CFB stream cannot be read"))?;
        let data = read_limited(stream, length.min(budget.remaining()))?;
        if data.len() != length {
            return Err(invalid("HWP CFB stream length is inconsistent"));
        }
        let name = path.to_string_lossy().to_ascii_lowercase();
        if is_document && distribution && name.starts_with("/viewtext/section") {
            let data = viewtext(&data, compressed, budget.remaining())?;
            budget.charge(data.len())?;
            records(&data, budget, true)?;
            continue;
        }
        let is_record_stream = is_document
            && (name == "/docinfo"
                || name.starts_with("/bodytext/section")
                || name.starts_with("/section"));
        let is_bin_data = is_document && name.starts_with("/bindata/");
        let expanded = if is_bin_data || is_record_stream && compressed {
            inflate(&data, budget.remaining())?
        } else {
            None
        };
        if is_record_stream && compressed && expanded.is_none() {
            return Err(invalid("HWP record stream compression is invalid"));
        }
        let data = expanded.as_deref().unwrap_or(&data);
        budget.charge(data.len())?;
        if is_record_stream {
            records(data, budget, false)?;
        }
        if !is_record_stream {
            embedded(data, budget, depth)?;
        }
    }
    drop(file);
    Ok(normalized)
}

pub fn admit(bytes: &[u8]) -> AppResult<Cow<'_, [u8]>> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded HWP exceeds the file preview budget"));
    }
    let mut budget = Budget::default();
    if bytes.starts_with(CFB_SIGNATURE) {
        return compound(bytes, &mut budget, 0, true);
    }
    if bytes.starts_with(ZIP_SIGNATURE) {
        archive(bytes, &mut budget, 0)?;
        return Ok(Cow::Borrowed(bytes));
    }
    if hml::detect_hml_signature(bytes) {
        let decoded =
            hml::encoding::decode(bytes, taide_model::file::READ_ONLY_FILE_BYTES as usize)
                .map_err(|_| invalid("HML XML encoding is invalid"))?;
        budget.charge(decoded.text.len())?;
        xml(decoded.text.as_bytes(), &mut budget, true)?;
        return Ok(Cow::Borrowed(bytes));
    }
    Err(invalid(
        "HWP input requires bounded parsing or has an unknown format",
    ))
}

pub fn load_core(bytes: &[u8]) -> AppResult<DocumentCore> {
    if detect_format(bytes) != FileFormat::Hwp3 {
        let source = admit(bytes)?;
        return DocumentCore::from_bytes(&source)
            .map_err(|_| invalid("native HWP engine rejected the document"));
    }
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded HWP exceeds the file preview budget"));
    }
    let info = Hwp3DocInfo::read(Cursor::new(
        bytes
            .get(HWP3_SIGNATURE_BYTES..)
            .ok_or_else(|| invalid("HWP3 header is truncated"))?,
    ))
    .map_err(|_| invalid("HWP3 document information is truncated"))?;
    if info.encrypted != 0 {
        return Err(invalid("HWP3 encrypted document is not supported"));
    }
    let limits = Hwp3Limits {
        max_decoded_bytes: MAX_DECODED_BYTES,
        max_nodes: MAX_NODES,
        max_table_grid_slots: MAX_TABLE_GRID_SLOTS,
        ..Hwp3Limits::default()
    };
    let mut budget = Budget::default();
    let document = parse_hwp3_with_validation(bytes, &limits, |payload| {
        let result = match payload {
            Hwp3Payload::DecodedBody {
                metadata_bytes,
                body,
            } => budget
                .charge(metadata_bytes)
                .and_then(|()| budget.charge(body.len())),
            Hwp3Payload::Ole(bytes) => compound(bytes, &mut budget, 0, false).map(|_| ()),
            Hwp3Payload::RepackedOle(bytes) => budget.charge(bytes.len()),
        };
        result.map_err(|error| Hwp3Error::LimitExceeded {
            message: error.to_string(),
        })?;
        Ok(budget.remaining())
    })
    .map_err(|_| invalid("HWP3 input is invalid or exceeds the preview budget"))?;
    DocumentCore::from_parsed(
        ParsedDocument {
            document,
            hml_metadata: None,
        },
        FileFormat::Hwp3,
    )
    .map_err(|_| invalid("native HWP engine rejected the document"))
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde_json::Value;

    use super::{DISTRIBUTE_DATA_BYTES, RECORD_SIZE_SHIFT, viewtext};

    const SMALL_DECODED_BUDGET: usize = 16;
    const EXPANSION_BYTES: usize = 4096;

    #[test]
    fn distribution_경계는_aes_뒤의_실제_팽창과_raw_예산을_제한한다() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/hwp-distribution-reference.json"
        ))
        .unwrap();
        let key_header = STANDARD
            .decode(fixture["keyHeader"].as_str().unwrap())
            .unwrap();
        assert_eq!(key_header.len(), DISTRIBUTE_DATA_BYTES);
        let header = u32::from(rhwp::parser::tags::HWPTAG_DISTRIBUTE_DOC_DATA)
            | (DISTRIBUTE_DATA_BYTES as u32) << RECORD_SIZE_SHIFT;
        let mut bytes = header.to_le_bytes().to_vec();
        bytes.extend_from_slice(&key_header);
        bytes.extend_from_slice(
            &STANDARD
                .decode(fixture["expansion"].as_str().unwrap())
                .unwrap(),
        );
        assert_eq!(
            viewtext(&bytes, true, EXPANSION_BYTES).unwrap().len(),
            EXPANSION_BYTES
        );
        assert!(
            viewtext(&bytes, true, SMALL_DECODED_BUDGET)
                .unwrap_err()
                .to_string()
                .contains("budget")
        );
        assert!(
            viewtext(&bytes, false, SMALL_DECODED_BUDGET)
                .unwrap_err()
                .to_string()
                .contains("budget")
        );
    }
}
