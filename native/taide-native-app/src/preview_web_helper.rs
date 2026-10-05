use std::io::{Read, Write};

use taide_model::{error::AppResult, file::READ_ONLY_FILE_BYTES};
use url::Url;

use crate::{
    preview::invalid,
    preview_web_document::{MAX_OUTPUT_BYTES, prepare_html},
};

pub const FLAG: &str = "--html-preview-helper";
pub const REQUEST_MAGIC: [u8; 4] = *b"THP1";
pub const REPLY_MAGIC: [u8; 4] = *b"THT1";
pub const MAX_SOURCE_BYTES: u32 = 4096;

pub fn serve(mut input: impl Read, mut output: impl Write) -> AppResult<()> {
    let mut magic = [0; REQUEST_MAGIC.len()];
    input
        .read_exact(&mut magic)
        .map_err(|_| invalid("HTML helper request is truncated"))?;
    if magic != REQUEST_MAGIC {
        return Err(invalid("HTML helper request version is invalid"));
    }
    let source_length = length(&mut input)?;
    let document_length = length(&mut input)?;
    if source_length > MAX_SOURCE_BYTES || u64::from(document_length) > READ_ONLY_FILE_BYTES {
        return Err(invalid("HTML helper request exceeds its input budget"));
    }
    let source = payload(&mut input, source_length)?;
    let source =
        std::str::from_utf8(&source).map_err(|_| invalid("HTML helper source is not UTF-8"))?;
    let source = Url::parse(source).map_err(|_| invalid("HTML helper source URL is invalid"))?;
    let bytes = payload(&mut input, document_length)?;
    let mut trailing = [0; 1];
    if input
        .read(&mut trailing)
        .map_err(|_| invalid("HTML helper input failed"))?
        != 0
    {
        return Err(invalid("HTML helper request contains trailing data"));
    }
    let document = prepare_html(&bytes, &source)?;
    if document.len() > MAX_OUTPUT_BYTES {
        return Err(invalid("HTML helper output exceeds its budget"));
    }
    let length = u32::try_from(document.len())
        .map_err(|_| invalid("HTML helper output length is invalid"))?;
    output
        .write_all(&REPLY_MAGIC)
        .and_then(|()| output.write_all(&length.to_le_bytes()))
        .and_then(|()| output.write_all(document.as_bytes()))
        .and_then(|()| output.flush())
        .map_err(|_| invalid("HTML helper reply failed"))
}

fn length(input: &mut impl Read) -> AppResult<u32> {
    let mut bytes = [0; size_of::<u32>()];
    input
        .read_exact(&mut bytes)
        .map_err(|_| invalid("HTML helper length is truncated"))?;
    Ok(u32::from_le_bytes(bytes))
}

fn payload(input: &mut impl Read, length: u32) -> AppResult<Vec<u8>> {
    let length = usize::try_from(length).map_err(|_| invalid("HTML helper length is invalid"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| invalid("HTML helper allocation failed"))?;
    bytes.resize(length, 0);
    input
        .read_exact(&mut bytes)
        .map_err(|_| invalid("HTML helper payload is truncated"))?;
    Ok(bytes)
}
