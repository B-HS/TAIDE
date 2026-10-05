use std::fmt::Write as FmtWrite;
use std::io::{Cursor, Write};

use taide_native_app::preview_spreadsheet::{
    Cell, MAX_DECODED_BYTES, MAX_PREVIEW_ROWS, decode_xlsx,
};
use zip::write::SimpleFileOptions;

const SHARED: &str = r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" uniqueCount="1"><si><t>&lt;script&gt; 한글 日本語</t></si></sst>"#;
const STYLES: &str = r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><cellXfs count="2"><xf numFmtId="0"/><xf numFmtId="14"/></cellXfs></styleSheet>"#;

fn workbook(sheets: &[(&str, &str)], shared: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut entries = vec![
        ("[Content_Types].xml".to_owned(), r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/></Types>"#.to_owned()),
        ("_rels/.rels".to_owned(), r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_owned()),
        ("xl/sharedStrings.xml".to_owned(), shared.to_owned()),
        ("xl/styles.xml".to_owned(), STYLES.to_owned()),
    ];
    let mut book = String::from(
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets>"#,
    );
    let mut relations = String::from(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    for (position, (name, sheet)) in sheets.iter().enumerate() {
        let index = position + 1;
        write!(
            book,
            r#"<sheet name="{name}" sheetId="{index}" r:id="rId{index}"/>"#
        )
        .unwrap();
        write!(relations, r#"<Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{index}.xml"/>"#).unwrap();
        entries.push((format!("xl/worksheets/sheet{index}.xml"), (*sheet).into()));
    }
    book.push_str("</sheets></workbook>");
    relations.push_str("</Relationships>");
    entries.push(("xl/workbook.xml".into(), book));
    entries.push(("xl/_rels/workbook.xml.rels".into(), relations));
    for (name, data) in entries {
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(data.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn xlsx_stream의_순서_타입_빈행_500행_total과_할당전_비신뢰_경계를_검사한다() {
    let body = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="B3:E505"/><sheetData><row r="3"><c r="B3" t="s"><v>0</v></c><c r="C3"><f>1+0.25</f><v>1.25</v></c><c r="D3" t="b"><v>1</v></c><c r="E3" t="e"><v>#DIV/0!</v></c></row><row r="4"><c r="B4" t="inlineStr"><is><t>inline</t></is></c><c r="C4" s="1"><v>46297</v></c></row><row r="502"><c r="B502"><v>499</v></c></row><row r="505"><c r="B505"><v>502</v></c></row></sheetData><hyperlinks><hyperlink ref="B3" location="file:///not-approved"/></hyperlinks></worksheet>"#;
    let empty = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>"#;
    let inferred = r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="7"><c r="B7"><v>2</v></c></row></sheetData></worksheet>"#;
    let bytes = workbook(
        &[("First", body), ("Empty", empty), ("Offset", inferred)],
        SHARED,
    );
    let parsed = decode_xlsx(&bytes).unwrap();
    assert_eq!(
        parsed
            .sheets
            .iter()
            .map(|sheet| sheet.name.as_str())
            .collect::<Vec<_>>(),
        ["First", "Empty", "Offset"]
    );
    let first = &parsed.sheets[0];
    assert_eq!(first.total_row_count, 503);
    assert!(first.truncated);
    assert_eq!(first.rows.len(), MAX_PREVIEW_ROWS);
    assert_eq!(
        first.rows[0],
        [
            Cell::Text("<script> 한글 日本語".into()),
            Cell::Number(1.25),
            Cell::Boolean(true),
            Cell::Null
        ]
    );
    assert_eq!(
        first.rows[1],
        [
            Cell::Text("inline".into()),
            Cell::Number(46297.0),
            Cell::Null,
            Cell::Null
        ]
    );
    assert_eq!(first.rows[2], vec![Cell::Null; 4]);
    assert_eq!(first.rows[MAX_PREVIEW_ROWS - 1][0], Cell::Number(499.0));
    assert!(parsed.sheets[1].rows.is_empty());
    assert_eq!(parsed.sheets[1].total_row_count, 0);
    assert_eq!(parsed.sheets[2].rows, [vec![Cell::Number(2.0)]]);
    assert!(parsed.retained_bytes() < MAX_DECODED_BYTES);
    assert!(
        decode_xlsx(&workbook(&[], SHARED))
            .unwrap()
            .sheets
            .is_empty()
    );
    assert!(decode_xlsx(b"not a workbook").is_err());
    assert!(decode_xlsx(&bytes[..bytes.len() / 2]).is_err());
    let mut bad_crc = bytes.clone();
    let central = bad_crc
        .windows(4)
        .position(|value| value == b"PK\x01\x02")
        .unwrap();
    bad_crc[central + 16] ^= 1;
    assert!(decode_xlsx(&bad_crc).is_err());
    let mut huge = bytes.clone();
    huge[central + 24..central + 28]
        .copy_from_slice(&u32::try_from(MAX_DECODED_BYTES + 1).unwrap().to_le_bytes());
    assert!(decode_xlsx(&huge).is_err());
    let bomb = SHARED.replace("uniqueCount=\"1\"", "uniqueCount=\"999999999999999999\"");
    assert!(decode_xlsx(&workbook(&[("First", body)], &bomb)).is_err());
    let doctype =
        format!(r#"<!DOCTYPE sst [<!ENTITY outside SYSTEM "file:///not-approved">]>{SHARED}"#);
    assert!(decode_xlsx(&workbook(&[("First", body)], &doctype)).is_err());
    let huge_grid = body.replace("B3:E505", "A1:XFD1000");
    assert!(decode_xlsx(&workbook(&[("First", &huge_grid)], SHARED)).is_err());
    let out_of_bounds = body.replace("B3:E505", "A1:XFE1000");
    assert!(decode_xlsx(&workbook(&[("First", &out_of_bounds)], SHARED)).is_err());
    let mut reversed = String::from(
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
    );
    for row in [1000, 999, 1] {
        write!(
            reversed,
            r#"<row r="{row}"><c r="A{row}"><v>{row}</v></c></row>"#
        )
        .unwrap();
    }
    reversed.push_str("</sheetData></worksheet>");
    let parsed = decode_xlsx(&workbook(&[("Reversed", &reversed)], SHARED)).unwrap();
    assert_eq!(parsed.sheets[0].total_row_count, 1000);
    assert_eq!(parsed.sheets[0].rows[0], [Cell::Number(1.0)]);
    assert_eq!(parsed.sheets[0].rows[499], [Cell::Null]);
}
