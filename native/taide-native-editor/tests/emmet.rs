use taide_native_editor::emmet::{Syntax, extract};

const MAX_LINE_BYTES: usize = 4096;

include!("fixtures/emmet-extraction-oracle.rs");

#[test]
fn emmet의_축약식_추출은_원본_engine의_모든_문자경계와_일치한다() {
    for &(syntax, line, cursor, expected) in CASES {
        let syntax = match syntax {
            "html" => Syntax::Html,
            "jsx" => Syntax::Jsx,
            "css" => Syntax::Css,
            _ => panic!("invalid oracle syntax"),
        };
        let actual = extract(line, cursor, syntax, MAX_LINE_BYTES);
        assert_eq!(
            actual
                .as_ref()
                .map(|value| (value.abbreviation, value.bytes.start, value.bytes.end)),
            expected,
            "{syntax:?}: {line:?}, byte={cursor}",
        );
    }
}

#[test]
fn emmet의_언어등록은_원본의_여섯언어만_지원한다() {
    for (language, expected) in [
        ("html", Some(Syntax::Html)),
        ("heex", Some(Syntax::Html)),
        ("javascriptreact", Some(Syntax::Jsx)),
        ("typescriptreact", Some(Syntax::Jsx)),
        ("css", Some(Syntax::Css)),
        ("scss", Some(Syntax::Css)),
        ("javascript", None),
        ("typescript", None),
        ("rust", None),
        ("xml", None),
        ("sass", None),
        ("less", None),
        ("", None),
    ] {
        assert_eq!(Syntax::for_language(language), expected);
    }
}

#[test]
fn emmet의_잘못된_byte경계와_입력상한은_추출하지않는다() {
    let line = "😀div";
    for cursor in 1.."😀".len() {
        assert!(extract(line, cursor, Syntax::Html, MAX_LINE_BYTES).is_none());
    }
    assert!(extract(line, line.len() + 1, Syntax::Html, MAX_LINE_BYTES).is_none());
    assert!(extract(line, line.len(), Syntax::Html, line.len() - 1).is_none());
    assert!(extract("div", "div".len(), Syntax::Html, 0).is_none());
    let extracted = extract(line, line.len(), Syntax::Html, line.len()).unwrap();
    assert_eq!(extracted.abbreviation, "div");
    assert_eq!(extracted.bytes, "😀".len()..line.len());
}
