use chrono::FixedOffset;
use serde_json::{Value, json};

use taide_native_app::preview_spreadsheet::{Cell, Workbook, decode};

const JST_SECONDS: i32 = 9 * 60 * 60;

fn normalized(workbook: &Workbook) -> Value {
    json!(workbook.sheets.iter().map(|sheet| {
        let rows = sheet.rows.iter().map(|row| row.iter().map(|cell| {
            let kind = match cell {
                Cell::Null => "null",
                Cell::Text(_) => "text",
                Cell::Boolean(_) => "boolean",
                Cell::Number(_) => "number",
            };
            json!({"kind":kind,"value":cell.display()})
        }).collect::<Vec<_>>()).collect::<Vec<_>>();
        json!({"name":sheet.name,"totalRowCount":sheet.total_row_count,"truncated":sheet.truncated,"rows":rows})
    }).collect::<Vec<_>>())
}

#[test]
fn html은_원본_sheetjs의_복수표_타입_텍스트_병합_빈범위와_503행을_보존한다() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/spreadsheet-html-reference.json")).unwrap();
    let timezone = FixedOffset::east_opt(JST_SECONDS).unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let bytes = fixture["source"].as_str().unwrap().as_bytes();
        let parsed =
            taide_native_app::preview_spreadsheet_html::decode_in_timezone(bytes, &timezone)
                .unwrap();
        assert_eq!(
            normalized(&parsed),
            fixture["expected"],
            "fixture {} timezone {timezone}",
            fixture["name"]
        );
    }
}

#[test]
fn html은_workbook_문구가_주석이나_셀에_있어도_형식을_유지한다() {
    for source in [
        "<!-- <Workbook/> --><table><tr><td>classified</td></tr></table>",
        "<!DOCTYPE html><html><body><table><tr><td>classified</td></tr></table></body></html>",
        "<table><tr><td><Workbook>classified</Workbook></td></tr></table>",
    ] {
        let parsed = decode(source.as_bytes()).unwrap();
        assert_eq!(
            parsed.sheets[0].rows,
            [vec![Cell::Text("classified".into())]],
            "{source}"
        );
    }
}

#[test]
fn html은_내용분기_비실행_본문과_할당전_표_합산_병합_경계를_검사한다() {
    const ROWS: usize = 500;
    const LARGE_COLUMNS: u32 = 8192;
    const AGGREGATE_COLUMNS: u32 = 2048;
    const MAX_ROWS: u32 = 1_048_576;
    const MAX_COLUMNS: u32 = 16_384;
    const MAX_SHEETS: usize = 1024;
    let html = b"<html><script>neverRun()</script><table><tr><td><a href=\"file:///never-opened\">safe</a></td><td><script>literal()</script><img src=\"https://invalid.example/never\"/></td></tr></table></html>";
    let parsed = decode(html).unwrap();
    assert_eq!(
        parsed.sheets[0].rows[0],
        [Cell::Text("safe".into()), Cell::Text("literal()".into())]
    );
    for span in [
        "colspan=\"0\"".to_owned(),
        "colspan=\"1.5\"".to_owned(),
        format!("colspan=\"{}\"", MAX_COLUMNS + 1),
        format!("rowspan=\"{}\"", MAX_ROWS + 1),
    ] {
        assert!(
            decode(format!("<table><tr><td {span}>value</td></tr></table>").as_bytes()).is_err()
        );
    }
    let table = |columns| {
        format!(
            "<table>{}</table>",
            format!(
                "<tr><td colspan=\"{}\">wide</td><td>end</td></tr>",
                columns - 1
            )
            .repeat(ROWS)
        )
    };
    assert!(
        decode(table(LARGE_COLUMNS).as_bytes())
            .unwrap_err()
            .to_string()
            .contains("projected grid exceeds")
    );
    let single = table(AGGREGATE_COLUMNS);
    assert!(
        decode(single.as_bytes()).unwrap().retained_bytes()
            < taide_native_app::preview_spreadsheet::MAX_DECODED_BYTES
    );
    assert!(
        decode(single.repeat(3).as_bytes())
            .unwrap_err()
            .to_string()
            .contains("projected grid exceeds")
    );
    assert!(decode("<table></table>".repeat(MAX_SHEETS + 1).as_bytes()).is_err());
    for source in [
        "<html><table><tr><td>truncated",
        "<html><body>no table</body></html>",
    ] {
        assert!(decode(source.as_bytes()).is_err());
    }
}
