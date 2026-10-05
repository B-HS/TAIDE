use base64::Engine;
use serde_json::{Value, json};

use taide_native_app::preview_spreadsheet::{Cell, MAX_PREVIEW_ROWS, Workbook};
use taide_native_app::preview_spreadsheet_biff::decode;
use taide_native_app::preview_spreadsheet_xls::workbook_stream;

const BOF: u16 = 0x0809;
const EOF: u16 = 0x000a;
const BOUND_SHEET: u16 = 0x0085;
const SST: u16 = 0x00fc;
const CONTINUE: u16 = 0x003c;
const DIMENSIONS: u16 = 0x0200;
const NUMBER: u16 = 0x0203;
const LABEL_SST: u16 = 0x00fd;
const RK: u16 = 0x027e;
const MUL_RK: u16 = 0x00bd;
const FORMULA: u16 = 0x0006;
const STRING: u16 = 0x0207;
const MAX_ROWS: u32 = 65_536;
const MAX_COLUMNS: u16 = 256;

fn normalized(workbook: &Workbook) -> Value {
    json!(workbook.sheets.iter().map(|sheet| {
        let rows = sheet.rows.iter().map(|row| row.iter().map(|cell| match cell {
            Cell::Null => Value::Null,
            Cell::Text(value) => json!(value),
            Cell::Boolean(value) => json!(value),
            Cell::Number(value) => json!(value),
        }).collect::<Vec<_>>()).collect::<Vec<_>>();
        json!({"name":sheet.name,"rows":rows,"totalRowCount":sheet.total_row_count,"truncated":sheet.truncated})
    }).collect::<Vec<_>>())
}

fn normalized_reference(value: &Value) -> Value {
    match value {
        Value::Number(number) if number.is_f64() => value.clone(),
        Value::Number(number) => json!(number.as_f64().unwrap()),
        Value::Array(values) => json!(values.iter().map(normalized_reference).collect::<Vec<_>>()),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| {
                    if key == "rows" {
                        return (key.clone(), normalized_reference(value));
                    }
                    (key.clone(), value.clone())
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn record(kind: u16, payload: &[u8]) -> Vec<u8> {
    [
        kind.to_le_bytes().to_vec(),
        u16::try_from(payload.len()).unwrap().to_le_bytes().to_vec(),
        payload.to_vec(),
    ]
    .concat()
}

fn bof(kind: u16) -> Vec<u8> {
    record(
        BOF,
        &[
            0x0600u16.to_le_bytes().to_vec(),
            kind.to_le_bytes().to_vec(),
            vec![0; 12],
        ]
        .concat(),
    )
}

fn cell(row: u16, column: u16, value: &[u8]) -> Vec<u8> {
    [
        row.to_le_bytes().to_vec(),
        column.to_le_bytes().to_vec(),
        vec![0; 2],
        value.to_vec(),
    ]
    .concat()
}

fn dimensions(first_row: u32, end_row: u32, first_column: u16, end_column: u16) -> Vec<u8> {
    record(
        DIMENSIONS,
        &[
            first_row.to_le_bytes().to_vec(),
            end_row.to_le_bytes().to_vec(),
            first_column.to_le_bytes().to_vec(),
            end_column.to_le_bytes().to_vec(),
            vec![0; 2],
        ]
        .concat(),
    )
}

fn workbook(globals: &[u8], sheets: &[Vec<u8>]) -> Vec<u8> {
    let start = bof(5);
    let mut offset = start.len()
        + globals.len()
        + record(EOF, &[]).len()
        + sheets.len() * record(BOUND_SHEET, &[0; 9]).len();
    let mut bytes = start;
    for sheet in sheets {
        bytes.extend(record(
            BOUND_SHEET,
            &[
                u32::try_from(offset).unwrap().to_le_bytes().to_vec(),
                vec![0, 0, 1, 0, b'S'],
            ]
            .concat(),
        ));
        offset += sheet.len();
    }
    bytes.extend(globals);
    bytes.extend(record(EOF, &[]));
    for sheet in sheets {
        bytes.extend(sheet);
    }
    bytes
}

fn sheet(records: &[u8]) -> Vec<u8> {
    [bof(0x10), records.to_vec(), record(EOF, &[])].concat()
}

#[test]
fn biff8은_실제_sheetjs_xls의_inline_sst_unicode_범위_빈행_503행을_보존한다() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/spreadsheet-xls-reference.json")).unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(fixture["encoded"].as_str().unwrap())
            .unwrap();
        let stream = workbook_stream(&bytes).unwrap();
        let actual = decode(&stream).unwrap();
        assert_eq!(
            normalized(&actual),
            normalized_reference(&fixture["expected"]),
            "reference {}",
            fixture["name"]
        );
    }
}

#[test]
fn biff8은_continue_rich_extension_rk_mulrk_cached_formula와_할당전_경계를_검사한다() {
    let sst = [
        record(
            SST,
            &[
                1u32.to_le_bytes().to_vec(),
                1u32.to_le_bytes().to_vec(),
                3u16.to_le_bytes().to_vec(),
                vec![12],
                1u16.to_le_bytes().to_vec(),
                3u32.to_le_bytes().to_vec(),
                vec![b'A'],
            ]
            .concat(),
        ),
        record(
            CONTINUE,
            &[
                vec![1],
                "漢B".encode_utf16().flat_map(u16::to_le_bytes).collect(),
                vec![0; 4],
                vec![0; 3],
            ]
            .concat(),
        ),
    ]
    .concat();
    let cached_string = [0, 0, 0, 0, 0, 0, 0xff, 0xff];
    let cached_bool = [1, 0, 1, 0, 0, 0, 0xff, 0xff];
    let records = [
        dimensions(2, 4, 2, 6),
        record(LABEL_SST, &cell(2, 2, &0u32.to_le_bytes())),
        record(RK, &cell(2, 3, &((-7i32 << 2) | 2).to_le_bytes())),
        record(
            MUL_RK,
            &[
                3u16.to_le_bytes().to_vec(),
                2u16.to_le_bytes().to_vec(),
                vec![0; 2],
                ((125u32 << 2) | 3).to_le_bytes().to_vec(),
                vec![0; 2],
                ((9u32 << 2) | 2).to_le_bytes().to_vec(),
                3u16.to_le_bytes().to_vec(),
            ]
            .concat(),
        ),
        record(
            FORMULA,
            &cell(2, 4, &[cached_string.to_vec(), vec![0; 8]].concat()),
        ),
        record(STRING, &[3, 0, 0, b'o', b'k', b'!']),
        record(
            FORMULA,
            &cell(3, 4, &[cached_bool.to_vec(), vec![0; 8]].concat()),
        ),
        record(
            FORMULA,
            &cell(3, 5, &[42.5f64.to_le_bytes().to_vec(), vec![0; 8]].concat()),
        ),
    ]
    .concat();
    let parsed = decode(&workbook(&sst, &[sheet(&records)])).unwrap();
    assert_eq!(
        parsed.sheets[0].rows,
        [
            vec![
                Cell::Text("A漢B".into()),
                Cell::Number(-7.0),
                Cell::Text("ok!".into()),
                Cell::Null
            ],
            vec![
                Cell::Number(1.25),
                Cell::Number(9.0),
                Cell::Boolean(true),
                Cell::Number(42.5)
            ],
        ]
    );
    let full = decode(&workbook(
        &[],
        &[sheet(&dimensions(0, MAX_ROWS, 0, MAX_COLUMNS))],
    ))
    .unwrap();
    assert_eq!(full.sheets[0].rows.len(), MAX_PREVIEW_ROWS);
    assert_eq!(full.sheets[0].rows[0].len(), usize::from(MAX_COLUMNS));
    assert_eq!(full.sheets[0].total_row_count, MAX_ROWS as usize);
    assert!(full.sheets[0].truncated);
    for dimensions in [
        dimensions(0, MAX_ROWS + 1, 0, 1),
        dimensions(0, 1, 0, MAX_COLUMNS + 1),
        dimensions(3, 2, 0, 1),
    ] {
        assert!(decode(&workbook(&[], &[sheet(&dimensions)])).is_err());
    }
    let huge_sst = record(SST, &[1u32.to_le_bytes(), u32::MAX.to_le_bytes()].concat());
    assert!(
        decode(&workbook(&huge_sst, &[]))
            .unwrap_err()
            .to_string()
            .contains("shared string count")
    );
    assert!(
        decode(&workbook(
            &[],
            &[sheet(&record(LABEL_SST, &cell(0, 0, &1u32.to_le_bytes())))]
        ))
        .is_err()
    );
    assert!(decode(&workbook(&record(0x002f, &[]), &[])).is_err());
    assert!(
        decode(&workbook(
            &[],
            &[sheet(&record(
                NUMBER,
                &cell(0, MAX_COLUMNS, &1f64.to_le_bytes())
            ))]
        ))
        .is_err()
    );
    let mut truncated = workbook(&[], &[sheet(&records)]);
    truncated.pop();
    assert!(decode(&truncated).is_err());
    let mut offset = workbook(&[], &[sheet(&[])]);
    let position = bof(5).len() + size_of::<u32>();
    offset[position..position + size_of::<u32>()].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode(&offset).is_err());
    assert!(
        decode(&workbook(
            &record(SST, &[vec![1, 0, 0, 0, 1, 0, 0, 0, 4, 0, 0, b'A']].concat()),
            &[]
        ))
        .is_err()
    );
}

#[test]
fn biff8은_잘못된_substream과_누락된_cached_string을_거절한다() {
    assert!(
        decode(&sheet(&[]))
            .unwrap_err()
            .to_string()
            .contains("standalone")
    );
    let mut invalid_sheet = bof(5);
    invalid_sheet.extend(record(EOF, &[]));
    assert!(
        decode(&workbook(&[], &[invalid_sheet]))
            .unwrap_err()
            .to_string()
            .contains("substream type")
    );
    let cached = [0, 0, 0, 0, 0, 0, 0xff, 0xff];
    let formula = record(
        FORMULA,
        &cell(0, 0, &[cached.to_vec(), vec![0; 8]].concat()),
    );
    assert!(
        decode(&workbook(&[], &[sheet(&formula)]))
            .unwrap_err()
            .to_string()
            .contains("cached formula string is missing")
    );
    let next = record(
        FORMULA,
        &cell(0, 1, &[1f64.to_le_bytes().to_vec(), vec![0; 8]].concat()),
    );
    assert!(
        decode(&workbook(&[], &[sheet(&[formula, next].concat())]))
            .unwrap_err()
            .to_string()
            .contains("cached formula string is missing")
    );
}

#[test]
fn legacy_biff는_실제_sheetjs의_2_3_4_5_직렬화_셀과_범위를_보존한다() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "fixtures/spreadsheet-biff-legacy-reference.json"
    ))
    .unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(fixture["encoded"].as_str().unwrap())
            .unwrap();
        let stream = if bytes.starts_with(&[0xd0, 0xcf]) {
            workbook_stream(&bytes).unwrap()
        } else {
            bytes
        };
        let parsed = decode(&stream).unwrap();
        assert_eq!(
            normalized(&parsed),
            normalized_reference(&fixture["expected"]),
            "reference {}",
            fixture["name"]
        );
    }
}

#[test]
fn legacy_biff는_표준_3_4의_codepage와_빈_수식_캐시를_보존한다() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "fixtures/spreadsheet-biff-codepage-reference.json"
    ))
    .unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(fixture["encoded"].as_str().unwrap())
            .unwrap();
        let parsed = taide_native_app::preview_spreadsheet::decode(&bytes).unwrap();
        assert_eq!(
            normalized(&parsed),
            normalized_reference(&fixture["expected"]),
            "reference {}",
            fixture["name"]
        );
    }
    let empty = record(
        FORMULA,
        &cell(
            0,
            0,
            &[vec![3, 0, 0, 0, 0, 0, 0xff, 0xff], vec![0; 8]].concat(),
        ),
    );
    assert_eq!(
        decode(&workbook(&[], &[sheet(&empty)])).unwrap().sheets[0].rows,
        [vec![Cell::Text(String::new())]]
    );
    let unknown = record(0x0042, &u16::MAX.to_le_bytes());
    assert!(
        decode(&workbook(&unknown, &[]))
            .unwrap_err()
            .to_string()
            .contains("codepage")
    );
    let encrypted = [record(0x002f, &[]), record(EOF, &[])].concat();
    let standalone = [record(0x0009, &[2, 0, 16, 0]), encrypted].concat();
    assert!(
        decode(&standalone)
            .unwrap_err()
            .to_string()
            .contains("encrypted")
    );
    let invalid = [
        record(0x0209, &[3, 0, 16, 0]),
        record(DIMENSIONS, &[0, 0, 1, 0x40, 0, 0, 1, 0]),
        record(EOF, &[]),
    ]
    .concat();
    assert!(
        decode(&invalid)
            .unwrap_err()
            .to_string()
            .contains("dimensions")
    );
}
