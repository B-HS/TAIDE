use std::io::{Cursor, Write};

use rhwp::model::table::{Cell, Table};
use taide_native_app::preview_hwp_preflight::admit;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const ROWS: usize = 128;
const COLUMNS: usize = 1024;
const TABLE_BYTES: usize = 8;
const HEADER_BYTES: usize = 256;
const RECORD_SIZE_SHIFT: u32 = 20;

fn hwpx(xml: &str) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("Contents/section0.xml", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(xml.as_bytes()).unwrap();
    writer.finish().unwrap().into_inner()
}

#[test]
fn hwp_table_dimensions는_파서와_레이아웃_진입전에_합산_검사한다() {
    let dangerous =
        "<hp:sec xmlns:hp='urn:synthetic'><hp:tbl rowCnt='65535' colCnt='65535'/></hp:sec>";
    assert!(admit(&hwpx(dangerous)).is_err());
    let aggregate = format!(
        "<sec><tbl rowCnt='{ROWS}' colCnt='{COLUMNS}'/><tbl rowCnt='{ROWS}' colCnt='{COLUMNS}'/></sec>"
    );
    assert!(admit(&hwpx(&aggregate)).is_err());
    assert!(admit(&hwpx("<sec><tbl rowCnt='2' colCnt='3'/></sec>")).is_ok());
    let mut header = vec![0; HEADER_BYTES];
    header[..b"HWP Document File".len()].copy_from_slice(b"HWP Document File");
    let mut table = [0; TABLE_BYTES];
    table[size_of::<u32>()..size_of::<u32>() + size_of::<u16>()]
        .copy_from_slice(&u16::MAX.to_le_bytes());
    table[size_of::<u32>() + size_of::<u16>()..].copy_from_slice(&u16::MAX.to_le_bytes());
    let mut body = (u32::from(rhwp::parser::tags::HWPTAG_TABLE)
        | (TABLE_BYTES as u32) << RECORD_SIZE_SHIFT)
        .to_le_bytes()
        .to_vec();
    body.extend_from_slice(&table);
    let mut file =
        cfb::CompoundFile::create_with_version(cfb::Version::V3, Cursor::new(Vec::new())).unwrap();
    file.create_storage("/BodyText").unwrap();
    for (path, bytes) in [
        ("/FileHeader", header.as_slice()),
        ("/DocInfo", b"".as_slice()),
        ("/BodyText/Section0", &body),
    ] {
        let mut stream = file.create_stream(path).unwrap();
        stream.write_all(bytes).unwrap();
        stream.flush().unwrap();
    }
    file.flush().unwrap();
    assert!(admit(&file.into_inner().into_inner()).is_err());
}

#[test]
fn hwp_engine_cell_grid는_손상된_u16_span을_오버플로와_범위밖_순회없이_처리한다() {
    let mut table = Table {
        row_count: 1,
        col_count: 1,
        cells: vec![Cell {
            row: u16::MAX,
            col: u16::MAX,
            row_span: u16::MAX,
            col_span: u16::MAX,
            ..Default::default()
        }],
        ..Default::default()
    };
    table.rebuild_grid();
    assert_eq!(table.cell_grid, vec![None]);
    table.cells = vec![Cell {
        row_span: 1,
        col_span: 1,
        ..Default::default()
    }];
    table.rebuild_grid();
    assert_eq!(table.cell_grid, vec![Some(0)]);
}
