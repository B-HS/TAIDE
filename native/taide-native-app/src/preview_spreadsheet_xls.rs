use std::borrow::Cow;
use std::io::{Cursor, Read};

use taide_model::error::AppResult;

use crate::preview::invalid;

const SIGNATURE: [u8; 8] = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
const HEADER_BYTES: usize = 512;
const MAJOR_VERSION_OFFSET: usize = 26;
const BYTE_ORDER_OFFSET: usize = 28;
const SECTOR_SHIFT_OFFSET: usize = 30;
const FIRST_DIFAT_OFFSET: usize = 68;
const HEADER_DIFAT_OFFSET: usize = 76;
const VERSION_3: u16 = 3;
const VERSION_4: u16 = 4;
const VERSION_3_SECTOR_SHIFT: u16 = 9;
const VERSION_4_SECTOR_SHIFT: u16 = 12;
const LITTLE_ENDIAN: u16 = 0xfffe;
const END_OF_CHAIN: u32 = 0xfffffffe;
const FREE_SECTOR: u32 = 0xffffffff;
const STREAM_BUFFER_BYTES: usize = 64 * 1024;
const STREAM_NAMES: [&str; 2] = ["/Workbook", "/Book"];

fn short(bytes: &[u8], offset: usize) -> AppResult<u16> {
    let bytes = bytes
        .get(offset..offset + size_of::<u16>())
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| invalid("XLS CFB header is truncated"))?;
    Ok(u16::from_le_bytes(bytes))
}

fn integer(bytes: &[u8], offset: usize) -> AppResult<u32> {
    let bytes = bytes
        .get(offset..offset + size_of::<u32>())
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| invalid("XLS CFB sector data is truncated"))?;
    Ok(u32::from_le_bytes(bytes))
}

fn sector(bytes: &[u8], index: u32, sector_bytes: usize) -> AppResult<&[u8]> {
    let start = usize::try_from(index)
        .ok()
        .and_then(|index| index.checked_add(1))
        .and_then(|index| index.checked_mul(sector_bytes))
        .ok_or_else(|| invalid("XLS CFB sector index is invalid"))?;
    bytes
        .get(start..start + sector_bytes)
        .ok_or_else(|| invalid("XLS CFB sector is outside the file"))
}

pub(crate) fn preflight(bytes: &[u8]) -> AppResult<Cow<'_, [u8]>> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded XLS exceeds the file preview budget"));
    }
    if bytes.len() < HEADER_BYTES || !bytes.starts_with(&SIGNATURE) {
        return Err(invalid("XLS CFB header is missing"));
    }
    let version = short(bytes, MAJOR_VERSION_OFFSET)?;
    let shift = short(bytes, SECTOR_SHIFT_OFFSET)?;
    if short(bytes, BYTE_ORDER_OFFSET)? != LITTLE_ENDIAN
        || !matches!(
            (version, shift),
            (VERSION_3, VERSION_3_SECTOR_SHIFT) | (VERSION_4, VERSION_4_SECTOR_SHIFT)
        )
    {
        return Err(invalid("XLS CFB header has an unsupported sector layout"));
    }
    let sector_bytes = 1usize << shift;
    if bytes.len() < sector_bytes || !bytes.len().is_multiple_of(sector_bytes) {
        return Err(invalid("XLS CFB file has a truncated sector"));
    }
    let sector_count = bytes.len() / sector_bytes - 1;
    let mut seen_fat = vec![false; sector_count];
    let mut normalized = Cow::Borrowed(bytes);
    let mut fat_entries = 0usize;
    let mut register_fat = |index: u32| -> AppResult<()> {
        if index == FREE_SECTOR {
            return Ok(());
        }
        let slot = usize::try_from(index)
            .ok()
            .and_then(|index| seen_fat.get_mut(index))
            .ok_or_else(|| invalid("XLS CFB FAT sector is outside the file"))?;
        if *slot {
            return Err(invalid("XLS CFB repeats a FAT sector before allocation"));
        }
        *slot = true;
        let data = sector(bytes, index, sector_bytes)?;
        for offset in (0..sector_bytes).step_by(size_of::<u32>()) {
            if fat_entries >= sector_count && integer(data, offset)? == END_OF_CHAIN {
                let position = (index as usize + 1) * sector_bytes + offset;
                normalized.to_mut()[position..position + size_of::<u32>()]
                    .copy_from_slice(&FREE_SECTOR.to_le_bytes());
            }
            fat_entries += 1;
        }
        Ok(())
    };
    for offset in (HEADER_DIFAT_OFFSET..HEADER_BYTES).step_by(size_of::<u32>()) {
        register_fat(integer(bytes, offset)?)?;
    }
    let mut seen_difat = vec![false; sector_count];
    let mut index = integer(bytes, FIRST_DIFAT_OFFSET)?;
    while index != END_OF_CHAIN && index != FREE_SECTOR {
        let slot = usize::try_from(index)
            .ok()
            .and_then(|index| seen_difat.get_mut(index))
            .ok_or_else(|| invalid("XLS CFB DIFAT sector is outside the file"))?;
        if *slot {
            return Err(invalid("XLS CFB DIFAT chain contains a cycle"));
        }
        *slot = true;
        let data = sector(bytes, index, sector_bytes)?;
        let link_offset = sector_bytes - size_of::<u32>();
        for offset in (0..link_offset).step_by(size_of::<u32>()) {
            register_fat(integer(data, offset)?)?;
        }
        index = integer(data, link_offset)?;
    }
    if seen_fat
        .iter()
        .zip(&seen_difat)
        .any(|(fat, difat)| *fat && *difat)
    {
        return Err(invalid("XLS CFB FAT and DIFAT sectors overlap"));
    }
    Ok(normalized)
}

pub fn workbook_stream(bytes: &[u8]) -> AppResult<Vec<u8>> {
    let normalized = preflight(bytes)?;
    let mut file = cfb::OpenOptions::new()
        .max_buffer_size(STREAM_BUFFER_BYTES)
        .open_with(Cursor::new(normalized.as_ref()))
        .map_err(|error| invalid(error.to_string()))?;
    for name in STREAM_NAMES {
        let Ok(entry) = file.entry(name) else {
            continue;
        };
        if !entry.is_stream() {
            return Err(invalid("XLS workbook entry is not a stream"));
        }
        let length = usize::try_from(entry.len())
            .ok()
            .filter(|length| *length <= bytes.len())
            .ok_or_else(|| invalid("XLS workbook stream exceeds the file budget"))?;
        let stream = file
            .open_stream(name)
            .map_err(|error| invalid(error.to_string()))?;
        let mut output = Vec::with_capacity(length);
        stream
            .take(length as u64 + 1)
            .read_to_end(&mut output)
            .map_err(|error| invalid(error.to_string()))?;
        if output.len() != length {
            return Err(invalid("XLS workbook stream length is inconsistent"));
        }
        return Ok(output);
    }
    Err(invalid("XLS workbook stream is missing"))
}
