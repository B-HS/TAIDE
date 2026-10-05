use chrono::{FixedOffset, Local, NaiveDate, TimeZone};
use serde_json::{Value, json};

use taide_native_app::preview_spreadsheet::{Cell, MAX_DECODED_BYTES, MAX_PREVIEW_ROWS};
use taide_native_app::preview_spreadsheet_csv::{decode, decode_in_timezone};

const JST_SECONDS: i32 = 9 * 60 * 60;
const LARGE_COLUMN_COUNT: usize = 16_384;
const MILLISECONDS_PER_DAY: f64 = 86_400_000.0;

fn normalized(cell: &Cell) -> Value {
    let kind = match cell {
        Cell::Null => "null",
        Cell::Text(_) => "text",
        Cell::Number(_) => "number",
        Cell::Boolean(_) => "boolean",
    };
    json!({"kind": kind, "value": cell.display()})
}

#[test]
fn csv는_원본_sheetjs_27입력의_타입_표시_인코딩_빈행과_날짜를_보존한다() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/spreadsheet-csv-reference.json")).unwrap();
    let timezone = FixedOffset::east_opt(JST_SECONDS).unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let source = fixture["source"].as_str().unwrap();
        let bytes = match fixture["encoding"].as_str().unwrap() {
            "utf8" => source.as_bytes().to_vec(),
            "utf8-bom" => [&[0xef, 0xbb, 0xbf], source.as_bytes()].concat(),
            "utf16le" => [
                vec![0xff, 0xfe],
                source.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            ]
            .concat(),
            _ => panic!("unknown reference encoding"),
        };
        let workbook = decode_in_timezone(&bytes, &timezone).unwrap();
        let actual = workbook.sheets.iter().map(|sheet| {
            let rows = sheet.rows.iter().map(|row| row.iter().map(normalized).collect::<Vec<_>>()).collect::<Vec<_>>();
            json!({"name":sheet.name, "totalRowCount":sheet.total_row_count, "truncated":sheet.truncated, "rows":rows})
        }).collect::<Vec<_>>();
        assert_eq!(
            json!(actual),
            fixture["expected"],
            "fixture: {}",
            fixture["name"]
        );
    }
    let parsed = decode(b"10/2/2026\n").unwrap();
    assert_eq!(parsed.sheets[0].rows[0][0], Cell::Number(46297.0));
    let date = NaiveDate::from_ymd_opt(2026, 10, 2)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let expected = Local
        .from_utc_datetime(&date)
        .naive_local()
        .signed_duration_since(epoch)
        .num_milliseconds() as f64
        / MILLISECONDS_PER_DAY;
    let parsed = decode(b"2026-10-02\n").unwrap();
    assert_eq!(parsed.sheets[0].rows[0][0], Cell::Number(expected));
    let large = (0..MAX_PREVIEW_ROWS + 10)
        .map(|index| format!("{index},item\n"))
        .collect::<String>();
    let parsed = decode(large.as_bytes()).unwrap();
    assert_eq!(parsed.sheets[0].total_row_count, MAX_PREVIEW_ROWS + 10);
    assert_eq!(parsed.sheets[0].rows.len(), MAX_PREVIEW_ROWS);
    assert!(parsed.sheets[0].truncated);
    assert!(parsed.retained_bytes() < MAX_DECODED_BYTES);
    assert!(
        decode(&vec![
            b'a';
            usize::try_from(
                taide_model::file::READ_ONLY_FILE_BYTES
            )
            .unwrap()
                + 1
        ])
        .is_err()
    );
    for (value, expected) in [
        (f64::NAN, "NaN"),
        (f64::INFINITY, "Infinity"),
        (f64::NEG_INFINITY, "-Infinity"),
        (-0.0, "0"),
        (1e-7, "1e-7"),
        (1e-6, "0.000001"),
        (1e20, "100000000000000000000"),
        (1e21, "1e+21"),
    ] {
        assert_eq!(Cell::Number(value).display(), expected);
    }
}

#[test]
fn csv는_허용된_열_수의_큰_grid도_할당_전에_거절한다() {
    let wide = format!("{}\n", ",".repeat(LARGE_COLUMN_COUNT - 1));
    let error = decode(wide.repeat(MAX_PREVIEW_ROWS).as_bytes()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("CSV grid exceeds the retained budget")
    );
    let error = decode(format!("{}\n", ",".repeat(LARGE_COLUMN_COUNT)).as_bytes()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("CSV dimensions exceed Excel bounds")
    );
}
