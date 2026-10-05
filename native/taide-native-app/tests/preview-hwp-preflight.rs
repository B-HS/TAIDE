use std::io::{Cursor, Write};

use flate2::Compression;
use flate2::write::{DeflateEncoder, ZlibEncoder};
use taide_native_app::preview_hwp_preflight::{
    MAX_DECODED_BYTES, MAX_DEPTH, MAX_EMBEDDED_DEPTH, MAX_ENTRIES, MAX_NODES, admit,
};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const HEADER_BYTES: usize = 256;
const FLAGS_OFFSET: usize = 36;
const RECORD_SIZE_SHIFT: u32 = 20;
const RECORD_LEVEL_SHIFT: u32 = 10;
const EXTENDED_RECORD_SIZE: u32 = 0xfff;
const HEADER_FAT_OFFSET: usize = 76;
const FIRST_DIRECTORY_OFFSET: usize = 48;
const DIRECTORY_STREAM_LENGTH_OFFSET: usize = 120;
const DIRECTORY_ENTRY_BYTES: usize = 128;
const SECTOR_BYTES: usize = 512;
const ZIP_CRC_OFFSET: usize = 16;
const ZIP_SIZE_OFFSET: usize = 24;
const ZIP_ENTRY_NAME_OFFSET: usize = 46;
const ZIP_END_COUNT_OFFSET: usize = 10;
const ZIP_END_DISK_COUNT_OFFSET: usize = 8;
const CHUNK_BYTES: usize = 8192;

fn record(level: u32, payload: &[u8]) -> Vec<u8> {
    let mut data = (16 | level << RECORD_LEVEL_SHIFT | (payload.len() as u32) << RECORD_SIZE_SHIFT)
        .to_le_bytes()
        .to_vec();
    data.extend_from_slice(payload);
    data
}

fn header(flags: u32) -> Vec<u8> {
    let mut data = vec![0; HEADER_BYTES];
    data[..b"HWP Document File".len()].copy_from_slice(b"HWP Document File");
    data[FLAGS_OFFSET..FLAGS_OFFSET + size_of::<u32>()].copy_from_slice(&flags.to_le_bytes());
    data
}

fn compound(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut file =
        cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new())).unwrap();
    for (name, data) in entries {
        if let Some((parent, _)) = name.rsplit_once('/')
            && !parent.is_empty()
        {
            file.create_storage_all(parent).unwrap();
        }
        let mut stream = file.create_stream(name).unwrap();
        stream.write_all(data).unwrap();
        stream.flush().unwrap();
    }
    file.flush().unwrap();
    file.into_inner().into_inner()
}

fn document(flags: u32, info: &[u8], body: &[u8], binary: &[u8]) -> Vec<u8> {
    compound(&[
        ("/FileHeader", &header(flags)),
        ("/DocInfo", info),
        ("/BodyText/Section0", body),
        ("/BinData/BIN0001.ole", binary),
    ])
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, data) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn hwp_admission은_cfb_deflate_zlib_record_ole와_잘린_입력을_검사한다() {
    let info = record(0, b"synthetic info");
    let body = record(1, "합성 본문".as_bytes());
    let nested = compound(&[("/Contents", b"synthetic embedded payload")]);
    let plain = document(0, &info, &body, &nested);
    assert_eq!(admit(&plain).unwrap().as_ref(), plain);
    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::fast());
    zlib.write_all(&body).unwrap();
    let zlib = zlib.finish().unwrap();
    let compressed = document(1, &deflate(&info), &zlib, &deflate(&nested));
    assert_eq!(admit(&compressed).unwrap().as_ref(), compressed);
    let mut prefixed = u32::try_from(nested.len()).unwrap().to_le_bytes().to_vec();
    prefixed.extend_from_slice(&nested);
    assert!(admit(&document(0, &info, &body, &prefixed)).is_ok());
    assert!(admit(&plain[..plain.len() - 1]).is_err());
    assert!(admit(&document(0, &info, &body[..body.len() - 1], b"raw image")).is_err());
    let mut bad_size = (16 | EXTENDED_RECORD_SIZE << RECORD_SIZE_SHIFT)
        .to_le_bytes()
        .to_vec();
    bad_size.extend_from_slice(&u32::MAX.to_le_bytes());
    assert!(admit(&document(0, &bad_size, &body, b"raw image")).is_err());
    assert!(
        admit(&document(
            0,
            &info,
            &record(MAX_DEPTH as u32 + 1, b""),
            b"raw image"
        ))
        .is_err()
    );
    assert!(
        admit(&document(
            0,
            &record(0, b"").repeat(MAX_NODES + 1),
            &[],
            b"raw image"
        ))
        .is_err()
    );
    assert!(admit(&document(1, b"not deflate", &deflate(&body), b"raw image")).is_err());
    assert!(
        admit(&document(2, &info, &body, b"raw image"))
            .unwrap_err()
            .to_string()
            .contains("encrypted")
    );
    assert!(
        admit(&document(4, &info, &body, b"raw image"))
            .unwrap_err()
            .to_string()
            .contains("distribution")
    );
    assert!(admit(b"HWP Document File V3.00").is_err());
    assert!(admit(b"<HWPML/>").is_err());
    assert!(admit(&[]).is_err());
    let fat = u32::from_le_bytes(
        plain[HEADER_FAT_OFFSET..HEADER_FAT_OFFSET + size_of::<u32>()]
            .try_into()
            .unwrap(),
    );
    let mut repeated = plain.clone();
    repeated[HEADER_FAT_OFFSET + size_of::<u32>()..HEADER_FAT_OFFSET + size_of::<u32>() * 2]
        .copy_from_slice(&fat.to_le_bytes());
    assert!(admit(&repeated).is_err());
    let directory = u32::from_le_bytes(
        plain[FIRST_DIRECTORY_OFFSET..FIRST_DIRECTORY_OFFSET + size_of::<u32>()]
            .try_into()
            .unwrap(),
    );
    let mut huge_stream = plain.clone();
    let start = (directory as usize + 1) * SECTOR_BYTES
        + DIRECTORY_ENTRY_BYTES
        + DIRECTORY_STREAM_LENGTH_OFFSET;
    huge_stream[start..start + size_of::<u64>()].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(admit(&huge_stream).is_err());
    let mut deep = compound(&[("/Contents", b"leaf")]);
    for _ in 0..MAX_EMBEDDED_DEPTH {
        deep = compound(&[("/Contents", &deep)]);
    }
    assert!(
        admit(&document(0, &info, &body, &deep))
            .unwrap_err()
            .to_string()
            .contains("nesting")
    );
}

#[test]
fn hwpx_admission은_zip_xml_crc_duplicate_깊이와_선행_상한을_검사한다() {
    let xml = br#"<?xml version="1.0"?><hp:sec xmlns:hp="urn:synthetic"><hp:p id="1">A &amp; B &#65; <![CDATA[<literal>]]></hp:p></hp:sec>"#;
    let bytes = archive(&[
        ("Contents/header.xml", b"<header/>"),
        ("Contents/section0.xml", xml),
        ("BinData/synthetic.png", b"raw resource"),
    ]);
    assert_eq!(admit(&bytes).unwrap().as_ref(), bytes);
    assert!(admit(&bytes[..bytes.len() / 2]).is_err());
    for bad in [
        b"<root>".as_slice(),
        b"<a><b></a>",
        b"<a/><b/>",
        b"before<a/>",
        b"<a>&unknown;</a>",
        b"<a value='&unknown;'/>",
        b"<!DOCTYPE a [<!ENTITY file SYSTEM 'file:///not-approved'>]><a>&file;</a>",
        b"<a id='1' id='2'/>",
    ] {
        assert!(
            admit(&archive(&[("Contents/section0.xml", bad)])).is_err(),
            "{bad:?}"
        );
    }
    let deep = format!(
        "{}{}",
        "<a>".repeat(MAX_DEPTH + 1),
        "</a>".repeat(MAX_DEPTH + 1)
    );
    assert!(admit(&archive(&[("Contents/section0.xml", deep.as_bytes())])).is_err());
    let nodes = format!("<a>{}</a>", "<b/>".repeat(MAX_NODES + 1));
    assert!(admit(&archive(&[("Contents/section0.xml", nodes.as_bytes())])).is_err());
    let central = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    let mut bad_crc = bytes.clone();
    bad_crc[central + ZIP_CRC_OFFSET] ^= 1;
    assert!(admit(&bad_crc).is_err());
    let mut huge = bytes.clone();
    huge[central + ZIP_SIZE_OFFSET..central + ZIP_SIZE_OFFSET + size_of::<u32>()]
        .copy_from_slice(&(MAX_DECODED_BYTES as u32 + 1).to_le_bytes());
    assert!(admit(&huge).unwrap_err().to_string().contains("declared"));
    let end = bytes
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")
        .unwrap();
    let mut many = bytes.clone();
    many[end + ZIP_END_COUNT_OFFSET..end + ZIP_END_COUNT_OFFSET + size_of::<u16>()]
        .copy_from_slice(&(MAX_ENTRIES as u16 + 1).to_le_bytes());
    assert!(admit(&many).is_err());
    let mut duplicate = archive(&[("same1", b"a"), ("same2", b"b")]);
    let directories = duplicate
        .windows(4)
        .enumerate()
        .filter_map(|(index, window)| (window == b"PK\x01\x02").then_some(index))
        .collect::<Vec<_>>();
    let second = directories[1] + ZIP_ENTRY_NAME_OFFSET;
    duplicate[second..second + b"same1".len()].copy_from_slice(b"same1");
    assert!(admit(&duplicate).is_err());
}

#[test]
fn hwp_admission은_deflate_expansion과_encoded_합산_상한을_검사한다() {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::fast());
    let chunk = [0; CHUNK_BYTES];
    for _ in 0..MAX_DECODED_BYTES / CHUNK_BYTES {
        encoder.write_all(&chunk).unwrap();
    }
    encoder.write_all(&[0]).unwrap();
    let bomb = encoder.finish().unwrap();
    let bytes = document(1, &bomb, &deflate(&[]), b"raw resource");
    assert!(admit(&bytes).unwrap_err().to_string().contains("decoded"));
    let bytes = document(0, &[], &[], &bomb);
    assert!(admit(&bytes).unwrap_err().to_string().contains("decoded"));
    let huge = vec![0; taide_model::file::READ_ONLY_FILE_BYTES as usize + 1];
    assert!(admit(&huge).unwrap_err().to_string().contains("encoded"));
    let mut bytes = archive(&[("one", b"a"), ("two", b"b")]);
    let directories = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(index, window)| (window == b"PK\x01\x02").then_some(index))
        .collect::<Vec<_>>();
    for directory in directories {
        bytes[directory + ZIP_SIZE_OFFSET..directory + ZIP_SIZE_OFFSET + size_of::<u32>()]
            .copy_from_slice(&(MAX_DECODED_BYTES as u32 / 2 + 1).to_le_bytes());
    }
    assert!(admit(&bytes).unwrap_err().to_string().contains("aggregate"));
}

#[test]
fn hwpx는_일치하는_entry_count와_위조된_size의_실제_압축팽창을_제한한다() {
    let mut bytes = archive(&[("resource", b"raw")]);
    let end = bytes
        .windows(size_of::<u32>())
        .rposition(|window| window == b"PK\x05\x06")
        .unwrap();
    for offset in [ZIP_END_DISK_COUNT_OFFSET, ZIP_END_COUNT_OFFSET] {
        bytes[end + offset..end + offset + size_of::<u16>()]
            .copy_from_slice(&(MAX_ENTRIES as u16 + 1).to_le_bytes());
    }
    assert!(
        admit(&bytes)
            .unwrap_err()
            .to_string()
            .contains("entry count")
    );
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "resource",
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .unwrap();
    let chunk = [0; CHUNK_BYTES];
    for _ in 0..MAX_DECODED_BYTES / CHUNK_BYTES {
        writer.write_all(&chunk).unwrap();
    }
    writer.write_all(&[0]).unwrap();
    let mut bytes = writer.finish().unwrap().into_inner();
    let central = bytes
        .windows(size_of::<u32>())
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    bytes[central + ZIP_SIZE_OFFSET..central + ZIP_SIZE_OFFSET + size_of::<u32>()]
        .copy_from_slice(&1u32.to_le_bytes());
    assert!(admit(&bytes).unwrap_err().to_string().contains("decoded"));
}
