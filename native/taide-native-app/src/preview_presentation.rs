use std::io::{Cursor, Read};

use taide_model::error::AppResult;
use taide_runtime::AppServices;
use zip::{CompressionMethod, ZipArchive};

use crate::preview::{Failure, invalid};

pub const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SLIDES: usize = 4096;
const MAX_ENTRIES: usize = u16::MAX as usize;
const SLIDE_PREFIX: &str = "ppt/slides/slide";
const SLIDE_SUFFIX: &str = ".xml";
const HEXADECIMAL_RADIX: u32 = 16;
const DECIMAL_RADIX: u32 = 10;
const SURROGATE_START: u32 = 0xD800;
const SURROGATE_END: u32 = 0xDFFF;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub path: String,
    pub token: u64,
}

pub async fn read(
    services: &AppServices,
    request: &Request,
    on_source_ready: impl FnOnce() + Send + 'static,
) -> Result<Outline, Failure> {
    let result = crate::preview::read_approved(
        services,
        request.path.clone(),
        "native-pptx-preview",
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

#[derive(Debug, PartialEq, Eq)]
pub struct Slide {
    pub index: usize,
    pub paragraphs: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Outline {
    pub slides: Vec<Slide>,
}

impl Outline {
    pub fn retained_bytes(&self) -> usize {
        self.slides.iter().fold(
            self.slides.capacity().saturating_mul(size_of::<Slide>()),
            |bytes, slide| {
                slide.paragraphs.iter().fold(
                    bytes.saturating_add(
                        slide
                            .paragraphs
                            .capacity()
                            .saturating_mul(size_of::<String>()),
                    ),
                    |bytes, paragraph| bytes.saturating_add(paragraph.capacity()),
                )
            },
        )
    }
}

fn entities(mut text: &str) -> AppResult<String> {
    let mut output = String::with_capacity(text.len());
    while let Some((before, after)) = text.split_once('&') {
        output.push_str(before);
        let (prefix, digits, radix) = if let Some(digits) = after.strip_prefix("#x") {
            ("#x", digits, HEXADECIMAL_RADIX)
        } else if let Some(digits) = after.strip_prefix('#') {
            ("#", digits, DECIMAL_RADIX)
        } else {
            ("", after, 0)
        };
        let length = digits
            .bytes()
            .take_while(|byte| match radix {
                HEXADECIMAL_RADIX => byte.is_ascii_hexdigit(),
                DECIMAL_RADIX => byte.is_ascii_digit(),
                _ => byte.is_ascii_lowercase(),
            })
            .count();
        if length == 0 || digits.as_bytes().get(length) != Some(&b';') {
            output.push('&');
            text = after;
            continue;
        }
        let entity = &digits[..length];
        if radix != 0 {
            let value = u32::from_str_radix(entity, radix)
                .map_err(|_| invalid("PPTX text has an invalid numeric entity"))?;
            let value = if (SURROGATE_START..=SURROGATE_END).contains(&value) {
                char::REPLACEMENT_CHARACTER
            } else {
                char::from_u32(value)
                    .ok_or_else(|| invalid("PPTX text has an invalid numeric entity"))?
            };
            output.push(value);
        } else {
            match entity {
                "amp" => output.push('&'),
                "lt" => output.push('<'),
                "gt" => output.push('>'),
                "quot" => output.push('"'),
                "apos" => output.push('\''),
                _ => {
                    output.push('&');
                    output.push_str(entity);
                    output.push(';');
                }
            }
        }
        text = &after[prefix.len() + length + 1..];
    }
    output.push_str(text);
    Ok(output)
}

fn paragraphs(mut xml: &str, max_bytes: usize) -> AppResult<(Vec<String>, usize)> {
    let mut paragraphs = Vec::new();
    let mut string_bytes = 0usize;
    while let Some((_, after)) = xml.split_once("<a:p>") {
        let Some((mut paragraph, after)) = after.split_once("</a:p>") else {
            break;
        };
        xml = after;
        let mut text = String::new();
        while let Some((_, after)) = paragraph.split_once("<a:t>") {
            let Some((run, after)) = after.split_once("</a:t>") else {
                break;
            };
            paragraph = after;
            text.push_str(&entities(run)?);
        }
        if !text
            .trim_matches(|character: char| {
                character != '\u{85}' && character.is_whitespace() || character == '\u{feff}'
            })
            .is_empty()
        {
            string_bytes = string_bytes.saturating_add(text.capacity());
            paragraphs.push(text);
            if paragraphs
                .capacity()
                .saturating_mul(size_of::<String>())
                .saturating_add(string_bytes)
                > max_bytes
            {
                return Err(invalid("PPTX outline exceeds the retained preview budget"));
            }
        }
    }
    let bytes = paragraphs
        .capacity()
        .saturating_mul(size_of::<String>())
        .saturating_add(string_bytes);
    Ok((paragraphs, bytes))
}

pub fn decode(bytes: &[u8]) -> AppResult<Outline> {
    if bytes.len() as u64 > taide_model::file::READ_ONLY_FILE_BYTES {
        return Err(invalid("encoded PPTX exceeds the file preview budget"));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| invalid("native PPTX decoder rejected the archive"))?;
    if archive.len() > MAX_ENTRIES {
        return Err(invalid("PPTX has too many archive entries"));
    }
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let Some(name) = archive.name_for_index(index) else {
            return Err(invalid("PPTX has an invalid archive entry"));
        };
        let Some(number) = name
            .strip_prefix(SLIDE_PREFIX)
            .and_then(|name| name.strip_suffix(SLIDE_SUFFIX))
            .filter(|number| {
                !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
            })
        else {
            continue;
        };
        let number = number
            .parse::<f64>()
            .map_err(|_| invalid("PPTX has an invalid slide number"))?;
        entries.push((index, number));
        if entries.len() > MAX_SLIDES {
            return Err(invalid("PPTX has too many slides"));
        }
    }
    if entries.is_empty() {
        return Err(invalid("PPTX has no slides"));
    }
    entries.sort_by(|(_, left), (_, right)| left.total_cmp(right));
    let mut decoded_bytes = 0;
    let mut outline = Outline {
        slides: Vec::with_capacity(entries.len()),
    };
    let mut retained_bytes = outline.slides.capacity().saturating_mul(size_of::<Slide>());
    for (index, _) in entries {
        let entry = archive
            .by_index(index)
            .map_err(|_| invalid("PPTX slide could not be read"))?;
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(invalid("PPTX slide compression is not supported"));
        }
        let remaining = MAX_DECODED_BYTES - decoded_bytes;
        if entry.size() > remaining as u64 {
            return Err(invalid(
                "PPTX slide data exceeds the decoded preview budget",
            ));
        }
        let mut data = Vec::new();
        entry
            .take(remaining as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|_| invalid("PPTX slide data is invalid"))?;
        if data.len() > remaining {
            return Err(invalid(
                "PPTX slide data exceeds the decoded preview budget",
            ));
        }
        decoded_bytes += data.len();
        let xml = String::from_utf8_lossy(&data);
        let (paragraphs, bytes) = paragraphs(&xml, MAX_DECODED_BYTES - retained_bytes)?;
        retained_bytes += bytes;
        outline.slides.push(Slide {
            index: outline.slides.len() + 1,
            paragraphs,
        });
    }
    Ok(outline)
}
