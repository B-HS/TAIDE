use std::io::{Cursor, Read, Write};

use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::read::DeflateDecoder;
use serde_json::Value;
use taide_native_app::{preview_hwp::Document, preview_hwp_preflight::admit};

const BLANK: &[u8] = include_bytes!("../vendor/rhwp/saved/blank2010.hwp");
const FLAGS_OFFSET: usize = 36;
const RECORD_SIZE_SHIFT: u32 = 20;
const EXTENDED_SIZE: u32 = 0xfff;
const DISTRIBUTE_BYTES: usize = 256;
const COMPRESSED: u32 = 1;
const DISTRIBUTION: u32 = 4;
const MAX_SIDE: usize = 4096;

fn reference() -> Value {
    serde_json::from_str(include_str!("fixtures/hwp-distribution-reference.json")).unwrap()
}

fn bytes(fixture: &Value, name: &str) -> Vec<u8> {
    STANDARD.decode(fixture[name].as_str().unwrap()).unwrap()
}

fn view(fixture: &Value, name: &str, extended: bool) -> Vec<u8> {
    let size = if extended {
        EXTENDED_SIZE
    } else {
        DISTRIBUTE_BYTES as u32
    };
    let header =
        u32::from(rhwp::parser::tags::HWPTAG_DISTRIBUTE_DOC_DATA) | size << RECORD_SIZE_SHIFT;
    let mut data = header.to_le_bytes().to_vec();
    if extended {
        data.extend_from_slice(&(DISTRIBUTE_BYTES as u32).to_le_bytes());
    }
    let key_header = bytes(fixture, "keyHeader");
    assert_eq!(key_header.len(), DISTRIBUTE_BYTES);
    data.extend_from_slice(&key_header);
    data.extend_from_slice(&bytes(fixture, name));
    data
}

fn document(section: &[u8], flags: u32) -> Vec<u8> {
    let mut original = cfb::OpenOptions::new()
        .open_with(Cursor::new(BLANK))
        .unwrap();
    let mut header = Vec::new();
    original
        .open_stream("/FileHeader")
        .unwrap()
        .read_to_end(&mut header)
        .unwrap();
    header[FLAGS_OFFSET..FLAGS_OFFSET + size_of::<u32>()].copy_from_slice(&flags.to_le_bytes());
    let mut info = Vec::new();
    original
        .open_stream("/DocInfo")
        .unwrap()
        .read_to_end(&mut info)
        .unwrap();
    if flags & COMPRESSED == 0 {
        let mut raw = Vec::new();
        DeflateDecoder::new(info.as_slice())
            .read_to_end(&mut raw)
            .unwrap();
        info = raw;
    }
    let mut cfb =
        cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new())).unwrap();
    cfb.create_storage("/ViewText").unwrap();
    for (name, data) in [
        ("/FileHeader", header.as_slice()),
        ("/DocInfo", &info),
        ("/ViewText/Section0", section),
    ] {
        let mut stream = cfb.create_stream(name).unwrap();
        stream.write_all(data).unwrap();
        stream.flush().unwrap();
    }
    cfb.flush().unwrap();
    cfb.into_inner().into_inner()
}

#[test]
fn distribution_extended_record의_실제_aes_본문_시작을_검사한다() {
    let fixture = reference();
    for (name, compressed) in [("plain", false), ("deflate", true), ("zlib", true)] {
        let standard = rhwp::parser::crypto::decrypt_viewtext_section(
            &view(&fixture, name, false),
            compressed,
        )
        .unwrap();
        let extended =
            rhwp::parser::crypto::decrypt_viewtext_section(&view(&fixture, name, true), compressed)
                .unwrap();
        assert_eq!(standard.len(), extended.len());
        assert!(
            standard == extended,
            "standard and extended distribution headers must decrypt identical bytes"
        );
    }
}

#[test]
fn distribution_native는_원본_blank_본문의_plain_deflate_zlib와_거절을_연결한다() {
    let fixture = reference();
    let plain = Document::load(BLANK).unwrap();
    let baseline = plain.render(0, MAX_SIDE).unwrap().unwrap();
    for (name, compressed) in [("plain", false), ("deflate", true), ("zlib", true)] {
        for extended in [false, true] {
            let flags = if compressed {
                DISTRIBUTION | COMPRESSED
            } else {
                DISTRIBUTION
            };
            let bytes = document(&view(&fixture, name, extended), flags);
            assert!(admit(&bytes).is_ok());
            let decoded = Document::load(&bytes).unwrap();
            assert_eq!(decoded.page_count(), plain.page_count());
            let rendered = decoded.render(0, MAX_SIDE).unwrap().unwrap();
            assert_eq!(rendered.size, baseline.size);
            assert_eq!(rendered.rgba, baseline.rgba);
        }
    }
    let mut malformed = view(&fixture, "deflate", false);
    malformed[..size_of::<u32>()].copy_from_slice(&0u32.to_le_bytes());
    assert!(admit(&document(&malformed, COMPRESSED | DISTRIBUTION)).is_err());
    assert!(admit(&document(&[0; size_of::<u32>()], DISTRIBUTION)).is_err());
    let size = (u32::from(rhwp::parser::tags::HWPTAG_DISTRIBUTE_DOC_DATA)
        | EXTENDED_SIZE << RECORD_SIZE_SHIFT)
        .to_le_bytes();
    let truncated = [size.as_slice(), &u32::MAX.to_le_bytes()].concat();
    assert!(admit(&document(&truncated, DISTRIBUTION)).is_err());
}

#[test]
fn distribution_경계는_복호화한_표의_선언_확대를_엔진_진입전에_거절한다() {
    let fixture = reference();
    let bytes = document(&view(&fixture, "hugeTable", false), DISTRIBUTION);
    let error = admit(&bytes).unwrap_err();
    assert!(error.to_string().contains("grid"));
}
