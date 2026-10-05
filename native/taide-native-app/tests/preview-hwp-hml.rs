use std::borrow::Cow;

use rhwp::{DocumentCore, parser::hml};
use taide_native_app::{
    preview_hwp::Document,
    preview_hwp_preflight::{MAX_DEPTH, MAX_NODES, admit},
};

const MAX_SIDE: usize = 4096;
const CHANNELS: usize = 4;
const BLACK_THRESHOLD: u8 = 96;
const TEXT: &str = "Synthetic HML 한글 日本語 中文 &amp; &#x1d11e;";
const TABLE_ROWS: usize = 128;
const TABLE_COLUMNS: usize = 1024;
const LONG_NAME_BYTES: usize = 240;
const PATH_ITEMS: usize = 3000;

fn document(head: &str, body: &str) -> String {
    format!(
        "<HWPML Version='2.91'><HEAD>{head}</HEAD><BODY><SECTION>{body}</SECTION></BODY></HWPML>"
    )
}

#[test]
fn hml_admission은_표_sparse_resource와_xml_할당_경계를_엔진전에_검사한다() {
    let table = "<TABLE RowCount='1' ColCount='2'><SIZE Width='40000' Height='2000'/><POSITION TreatAsChar='1'/><CELL RowAddr='0' ColAddr='0' Width='20000' Height='2000'><P><TEXT><CHAR>Cell 한글</CHAR></TEXT></P></CELL></TABLE>";
    let normal = document("", &format!("<P><TEXT>{table}</TEXT></P>"));
    let loaded = Document::load(normal.as_bytes()).unwrap();
    assert!(loaded.page_count() >= 1);
    assert!(loaded.render(0, MAX_SIDE).unwrap().is_some());
    assert!(admit(normal.replace("2.91", "2.9").as_bytes()).is_ok());
    let huge = document(
        "",
        "<P><TEXT><TABLE RowCount='65535' ColCount='65535'/></TEXT></P>",
    );
    assert!(
        admit(huge.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("grid")
    );
    let aggregate = document(
        "",
        &format!(
            "<P><TEXT><TABLE RowCount='{TABLE_ROWS}' ColCount='{TABLE_COLUMNS}'/><TABLE RowCount='{TABLE_ROWS}' ColCount='{TABLE_COLUMNS}'/></TEXT></P>"
        ),
    );
    assert!(
        admit(aggregate.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("grid")
    );
    let skipped = document(
        "<MAPPINGTABLE><CHARSHAPELIST><CHARSHAPE Id='2147483647'/></CHARSHAPELIST></MAPPINGTABLE>",
        &paragraph("Skipped resource"),
    );
    assert!(admit(skipped.as_bytes()).is_ok());
    let parsed = hml::parse_hml(skipped.as_bytes()).unwrap();
    assert!(parsed.document.doc_info.char_shapes.len() <= 1);
    assert!(
        parsed
            .warnings
            .iter()
            .any(|warning| warning.code == hml::HmlWarningCode::InvalidReference)
    );
    let sparse = document(
        "<MAPPINGTABLE><CHARSHAPELIST><CHARSHAPE Id='65535'/><CHARSHAPE Id='65535'/></CHARSHAPELIST></MAPPINGTABLE>",
        &paragraph("Sparse"),
    );
    assert!(admit(sparse.as_bytes()).is_ok());
    let fonts: String = [
        "Hangul", "Latin", "Hanja", "Japanese", "Other", "Symbol", "User",
    ]
    .iter()
    .map(|language| format!("<FONTFACE Lang='{language}'><FONT Id='65535'/></FONTFACE>"))
    .collect();
    let large = document(
        &format!(
            "<MAPPINGTABLE>{fonts}<BORDERFILL Id='65536'/><CHARSHAPE Id='65535'/><PARASHAPE Id='65535'/><TABDEF Id='65535'/><STYLE Id='65535'/></MAPPINGTABLE>"
        ),
        &paragraph("Budget"),
    );
    assert!(
        admit(large.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
    let deep = document(
        "",
        &format!("{}{}", "<X>".repeat(MAX_DEPTH), "</X>".repeat(MAX_DEPTH)),
    );
    assert!(admit(deep.as_bytes()).is_err());
    let nodes = document("", &"<X/>".repeat(MAX_NODES));
    assert!(admit(nodes.as_bytes()).is_err());
    let ancestor = "X".repeat(LONG_NAME_BYTES);
    let paths = document(
        "",
        &format!(
            "{}{}{}",
            format!("<{ancestor}>").repeat(MAX_DEPTH - 8),
            "<X/>".repeat(PATH_ITEMS),
            format!("</{ancestor}>").repeat(MAX_DEPTH - 8)
        ),
    );
    assert!(
        admit(paths.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("budget")
    );
    for invalid in [
        "<!DOCTYPE HWPML><HWPML Version='2.91'><HEAD/><BODY/></HWPML>",
        "<HWPML Version='2.91'><HEAD/><BODY><SECTION><P><TEXT><CHAR>&unknown;</CHAR></TEXT></P></SECTION></BODY></HWPML>",
        "<HWPML Version='2.91'><HEAD/><BODY></HEAD></HWPML>",
        "<HWPML Version='2.91'><HEAD/><BODY><TABLE RowCount=' 2 ' ColCount='3'/></BODY></HWPML>",
    ] {
        assert!(admit(invalid.as_bytes()).is_err());
    }
    let mut malformed = normal.as_bytes().to_vec();
    malformed.push(0xff);
    assert!(admit(&malformed).is_err());
    let mut odd = encodings(&normal).remove(2);
    odd.pop();
    assert!(admit(&odd).is_err());
    let mut surrogate = encodings(&normal).remove(2);
    surrogate.extend_from_slice(&0xd800u16.to_le_bytes());
    assert!(admit(&surrogate).is_err());
    assert!(Document::load(b"<HWPML Version='2.91'><BODY/></HWPML>").is_err());
}

fn paragraph(text: &str) -> String {
    format!("<P><TEXT><CHAR>{text}</CHAR></TEXT></P>")
}

fn encodings(xml: &str) -> Vec<Vec<u8>> {
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend_from_slice(xml.as_bytes());
    let mut little = vec![0xff, 0xfe];
    let mut big = vec![0xfe, 0xff];
    for unit in xml.encode_utf16() {
        little.extend_from_slice(&unit.to_le_bytes());
        big.extend_from_slice(&unit.to_be_bytes());
    }
    vec![xml.as_bytes().to_vec(), bom, little, big]
}

#[test]
fn hml_native는_원본_utf8_bom_utf16의_본문과_같은_페이지를_렌더한다() {
    let xml = document("", &paragraph(TEXT));
    let core = DocumentCore::from_bytes(xml.as_bytes()).unwrap();
    let pages = core.page_count() as usize;
    assert!(pages >= 1);
    let mut expected = None;
    for bytes in encodings(&xml) {
        assert!(hml::detect_hml_signature(&bytes));
        let accepted = admit(&bytes).unwrap();
        assert!(matches!(accepted, Cow::Borrowed(_)));
        assert_eq!(accepted.as_ref(), bytes);
        let loaded = Document::load(&bytes).unwrap();
        assert_eq!(loaded.page_count(), pages);
        let raster = loaded.render(0, MAX_SIDE).unwrap().unwrap();
        assert_eq!(
            raster.rgba.len(),
            raster.size[0] * raster.size[1] * CHANNELS
        );
        assert!(raster.rgba.as_chunks::<CHANNELS>().0.iter().any(|pixel| {
            pixel[0] < BLACK_THRESHOLD
                && pixel[1] < BLACK_THRESHOLD
                && pixel[2] < BLACK_THRESHOLD
                && pixel[3] > BLACK_THRESHOLD
        }));
        if let Some((size, rgba)) = &expected {
            assert_eq!(&raster.size, size);
            assert_eq!(&raster.rgba, rgba);
        }
        expected = Some((raster.size, raster.rgba));
    }
}
