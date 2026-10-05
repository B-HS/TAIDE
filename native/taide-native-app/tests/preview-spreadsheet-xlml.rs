use serde_json::{Value, json};

use taide_native_app::preview_spreadsheet::{Cell, Workbook, decode};

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
fn xlml은_원본_sheetjs의_타입_희소범위_빈행_병합_날짜와_503행을_보존한다() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/spreadsheet-xlml-reference.json")).unwrap();
    for fixture in fixtures.as_array().unwrap() {
        let source = fixture["source"].as_str().unwrap();
        let bytes = match fixture["encoding"].as_str().unwrap() {
            "utf8" => source.as_bytes().to_vec(),
            "utf8-bom" => [vec![0xef, 0xbb, 0xbf], source.as_bytes().to_vec()].concat(),
            _ => panic!("unknown reference encoding"),
        };
        let parsed = decode(&bytes).unwrap();
        assert_eq!(
            normalized(&parsed),
            fixture["expected"],
            "fixture: {}",
            fixture["name"]
        );
    }
}

fn workbook(body: &str) -> String {
    format!("<Workbook xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\">{body}</Workbook>")
}

#[test]
fn xlml은_원본_ascii_utf16_bom과_인코딩선언을_보존하고_잘린_입력을_거절한다() {
    let fixture: Value = serde_json::from_str(include_str!(
        "fixtures/spreadsheet-xlml-utf16-reference.json"
    ))
    .unwrap();
    let source = fixture["source"].as_str().unwrap();
    let mut bytes = [
        vec![0xff, 0xfe],
        source.encode_utf16().flat_map(u16::to_le_bytes).collect(),
    ]
    .concat();
    let parsed = decode(&bytes).unwrap();
    assert_eq!(normalized(&parsed), fixture["expected"]);
    bytes.pop();
    assert!(decode(&bytes).is_err());
    let surrogate = [
        vec![0xff, 0xfe],
        "<Workbook>"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect(),
        0xd800u16.to_le_bytes().to_vec(),
    ]
    .concat();
    assert!(decode(&surrogate).is_err());
}

fn sheet(name: &str, body: &str) -> String {
    format!("<Worksheet ss:Name=\"{name}\"><Table>{body}</Table></Worksheet>")
}

#[test]
fn xlml은_할당전_grid_합산_인덱스와_xml_보안_경계를_거절한다() {
    const MAX_ROWS: u32 = 1_048_576;
    const MAX_COLUMNS: u32 = 16_384;
    const MAX_DEPTH: usize = 128;
    const MAX_SHEETS: usize = 1024;
    const AGGREGATE_COLUMNS: u32 = 2048;
    const PREVIEW_ROWS: u32 = 500;
    for body in [
        "<Row ss:Index=\"0\"><Cell/></Row>".to_owned(),
        "<Row ss:Index=\"1.5\"><Cell/></Row>".to_owned(),
        format!("<Row ss:Index=\"{}\"><Cell/></Row>", MAX_ROWS + 1),
        format!("<Row><Cell ss:Index=\"{}\"/></Row>", MAX_COLUMNS + 1),
        format!("<Row><Cell ss:MergeAcross=\"{MAX_COLUMNS}\"></Cell></Row>"),
        "<Row><Cell><Data ss:Type=\"String\">&custom;</Data></Cell></Row>".to_owned(),
        "<Row><Cell><Data ss:Type=\"String\"><![CDATA[unknown]]></Data></Cell></Row>".to_owned(),
    ] {
        assert!(
            decode(workbook(&sheet("Invalid", &body)).as_bytes()).is_err(),
            "{body}"
        );
    }
    let grid = format!(
        "<Row><Cell/></Row><Row ss:Index=\"{MAX_ROWS}\"><Cell ss:Index=\"{MAX_COLUMNS}\"/></Row>"
    );
    let error = decode(workbook(&sheet("Huge", &grid)).as_bytes()).unwrap_err();
    assert!(error.to_string().contains("projected grid exceeds"));
    let grid = format!(
        "<Row><Cell/></Row><Row ss:Index=\"{PREVIEW_ROWS}\"><Cell ss:Index=\"{AGGREGATE_COLUMNS}\"/></Row>"
    );
    let single = decode(workbook(&sheet("Fits", &grid)).as_bytes()).unwrap();
    assert!(single.retained_bytes() < taide_native_app::preview_spreadsheet::MAX_DECODED_BYTES);
    let aggregate = (0..3)
        .map(|index| sheet(&index.to_string(), &grid))
        .collect::<String>();
    assert!(
        decode(workbook(&aggregate).as_bytes())
            .unwrap_err()
            .to_string()
            .contains("projected grid exceeds")
    );
    let sheets = (0..MAX_SHEETS + 1)
        .map(|index| sheet(&index.to_string(), ""))
        .collect::<String>();
    assert!(decode(workbook(&sheets).as_bytes()).is_err());
    assert!(decode(workbook(&[sheet("Same", ""), sheet("Same", "")].concat()).as_bytes()).is_err());
    let nested = format!(
        "{}{}",
        "<nested>".repeat(MAX_DEPTH),
        "</nested>".repeat(MAX_DEPTH)
    );
    assert!(decode(workbook(&nested).as_bytes()).is_err());
    for source in [
        "<!DOCTYPE Workbook SYSTEM \"file:///synthetic-never-opened\"><Workbook/>",
        "<!DOCTYPE Workbook [<!ENTITY custom SYSTEM \"https://invalid.example/never\">]><Workbook/>",
        "<Workbook><Worksheet ss:Name=\"Truncated\">",
        "<Workbook></Workbook><Workbook/>",
        "<Workbook><Worksheet></Worksheet></Workbook>",
        "<Workbook><Worksheet ss:Name=\"One\" ss:Name=\"Two\"/></Workbook>",
    ] {
        assert!(decode(source.as_bytes()).is_err(), "{source}");
    }
    let mut malformed = b"<Workbook>".to_vec();
    malformed.push(0xff);
    assert!(decode(&malformed).is_err());
    let bytes = vec![b'<'; usize::try_from(taide_model::file::READ_ONLY_FILE_BYTES).unwrap() + 1];
    assert!(decode(&bytes).is_err());
}
