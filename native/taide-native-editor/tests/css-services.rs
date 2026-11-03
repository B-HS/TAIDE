use taide_native_editor::css_folding::{self, folding_ranges};
use taide_native_editor::css_format::{self, format};
use taide_native_editor::css_scanner::{Dialect, Kind, Scanner};
use taide_native_editor::document::EditorError;
use taide_native_editor::indent::IndentOptions;

const FOLD_LIMITS: css_folding::Limits = css_folding::Limits {
    max_bytes: 1 << 20,
    max_tokens: 1 << 16,
    max_depth: 64,
    max_ranges: 1024,
};
const FORMAT_LIMITS: css_format::Limits = css_format::Limits {
    max_bytes: 1 << 20,
    max_output_bytes: 1 << 20,
    max_indent_width: 64,
};

fn tokens(input: &str, dialect: Dialect) -> Vec<(Kind, String)> {
    let mut scanner = Scanner::new(input, dialect);
    scanner.ignore_comment = false;
    let mut tokens = Vec::new();
    loop {
        let token = scanner.scan();
        if token.kind == Kind::Eof {
            assert_eq!(token.bytes, input.len()..input.len());
            return tokens;
        }
        tokens.push((token.kind, token.text));
    }
}

fn spaces(tab_size: u32) -> css_format::Options {
    css_format::Options {
        indentation: IndentOptions {
            tab_size,
            insert_spaces: true,
        },
    }
}

fn formatted(input: &str, options: css_format::Options) -> String {
    let edits = format(input, None, options, FORMAT_LIMITS).unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].bytes, 0..input.len());
    edits[0].text.clone()
}

#[test]
fn css_scanner는_원본_token_종류와_escape_단위_문자열을_만든다() {
    let kinds = tokens(
        "@media screen{.a>b{color:#fff;width:10px!important}}",
        Dialect::Css,
    )
    .into_iter()
    .map(|(kind, _)| kind)
    .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [
            Kind::AtKeyword,
            Kind::Ident,
            Kind::CurlyL,
            Kind::Delim,
            Kind::Ident,
            Kind::Delim,
            Kind::Ident,
            Kind::CurlyL,
            Kind::Ident,
            Kind::Colon,
            Kind::Hash,
            Kind::SemiColon,
            Kind::Ident,
            Kind::Colon,
            Kind::Length,
            Kind::Exclamation,
            Kind::Ident,
            Kind::CurlyR,
            Kind::CurlyR,
        ]
    );
    assert_eq!(
        tokens(
            "10% 1.5em 2\u{212a}Hz 3foo 4 <!-- --> ~= |= *= ^= $= @charset /* c */ \\41 x \"a\\\"b\" 'bad",
            Dialect::Css
        ),
        [
            (Kind::Percentage, "10%".into()),
            (Kind::Ems, "1.5em".into()),
            (Kind::Freq, "2\u{212a}Hz".into()),
            (Kind::Dimension, "3foo".into()),
            (Kind::Num, "4".into()),
            (Kind::Cdo, "<!--".into()),
            (Kind::Cdc, "-->".into()),
            (Kind::Includes, "~=".into()),
            (Kind::Dashmatch, "|=".into()),
            (Kind::SubstringOperator, "*=".into()),
            (Kind::PrefixOperator, "^=".into()),
            (Kind::SuffixOperator, "$=".into()),
            (Kind::Charset, "@charset".into()),
            (Kind::Comment, "/* c */".into()),
            (Kind::Ident, "Ax".into()),
            (Kind::String, "\"a\"b\"".into()),
            (Kind::BadString, "'bad".into()),
        ]
    );
    assert_eq!(
        tokens("a//b", Dialect::Css)
            .into_iter()
            .map(|(kind, _)| kind)
            .collect::<Vec<_>>(),
        [Kind::Ident, Kind::Delim, Kind::Delim, Kind::Ident]
    );
    let mut scanner = Scanner::new("+0-7F", Dialect::Css);
    let range = scanner.try_scan_unicode().unwrap();
    assert_eq!((range.kind, range.bytes), (Kind::UnicodeRange, 0..5));
    let mut scanner = Scanner::new("+1234567", Dialect::Css);
    assert!(scanner.try_scan_unicode().is_none());
    assert_eq!(scanner.pos(), 0);
}

#[test]
fn scss_scanner는_변수_보간_비교연산자와_한줄주석을_구분한다() {
    assert_eq!(
        tokens("$var #{ == != <= >= < > ... $ // line\n.é", Dialect::Scss),
        [
            (Kind::VariableName, "$var".into()),
            (Kind::InterpolationFunction, "#{".into()),
            (Kind::EqualsOperator, "==".into()),
            (Kind::NotEqualsOperator, "!=".into()),
            (Kind::SmallerEqualsOperator, "<=".into()),
            (Kind::GreaterEqualsOperator, ">=".into()),
            (Kind::Delim, "<".into()),
            (Kind::Delim, ">".into()),
            (Kind::Ellipsis, "...".into()),
            (Kind::Delim, "$".into()),
            (Kind::Comment, "// line".into()),
            (Kind::Delim, ".".into()),
            (Kind::Ident, "é".into()),
        ]
    );
}

#[test]
fn css_folding은_중괄호_region_여러줄주석과_교차범위_제거를_원본대로_계산한다() {
    let css = ".a {\n  color: red;\n}\n/* #region x */\n.b { color: blue;\n  margin: 0; }\n/* #endregion */\n/*\n multi\n*/\n/* #region x\n y */\n// #region\n/* #region\n z */\n";
    let ranges = folding_ranges(css, Dialect::Css, FOLD_LIMITS)
        .unwrap()
        .into_iter()
        .map(|range| (range.region.start_line, range.region.end_line, range.kind))
        .collect::<Vec<_>>();
    assert_eq!(
        ranges,
        [
            (0, 1, None),
            (3, 6, Some("region".into())),
            (4, 5, None),
            (7, 9, Some("comment".into())),
            (10, 11, Some("comment".into())),
        ]
    );
    let scss = "// #region\n.a {\n// #endregion\n  b: c;\n\n}\n.c-#{$d} {\n}\n";
    let ranges = folding_ranges(scss, Dialect::Scss, FOLD_LIMITS)
        .unwrap()
        .into_iter()
        .map(|range| (range.region.start_line, range.region.end_line, range.kind))
        .collect::<Vec<_>>();
    assert_eq!(ranges, [(0, 2, Some("region".into()))]);
    assert_eq!(
        folding_ranges(
            css,
            Dialect::Css,
            css_folding::Limits {
                max_ranges: 1,
                ..FOLD_LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        folding_ranges(
            "{{{",
            Dialect::Css,
            css_folding::Limits {
                max_depth: 2,
                ..FOLD_LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
}

#[test]
fn css_format은_beautify_css의_규칙_줄바꿈_url_at_rule_중첩과_탭을_원본대로_만든다() {
    assert_eq!(
        formatted(
            ".a,.b{color:red;background:url( x.png )}@media screen{.c{margin:0}}",
            spaces(4)
        ),
        ".a,\n.b {\n    color: red;\n    background: url(x.png )\n}\n\n@media screen {\n    .c {\n        margin: 0\n    }\n}"
    );
    assert_eq!(
        formatted(
            "$c: red;\n\n\n.a {\n  // note\n  &:hover { color: $c; }\n}\n",
            spaces(4)
        ),
        "$c: red;\n\n\n.a {\n\n    // note\n    &:hover {\n        color: $c;\n    }\n}"
    );
    assert_eq!(
        formatted(
            "a{b:c}",
            css_format::Options {
                indentation: IndentOptions {
                    tab_size: 4,
                    insert_spaces: false,
                },
            }
        ),
        "a {\n\tb: c\n}"
    );
    assert_eq!(formatted("  a{}", spaces(4)), "  a {}");
    assert_eq!(
        formatted(
            "a{/* beautify ignore:start */b:c;  d:e/* beautify ignore:end */}",
            spaces(4)
        ),
        "a {\n    /* beautify ignore:start */b:c;  d:e/* beautify ignore:end */\n}"
    );
}

#[test]
fn css_format_범위는_줄_공백을_넓히고_규칙_안쪽과_시작_들여쓰기를_원본대로_적용한다() {
    let input = ".a {\n  color:red;\n  margin:0;\n}\n";
    let start = input.find("color").unwrap();
    let end = start + "color:red;".len();
    let edits = format(input, Some(start..end), spaces(2), FORMAT_LIMITS).unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].bytes, start - 2..end);
    assert_eq!(edits[0].text, "  color: red;");
    let edits = format(input, Some(start..end), spaces(4), FORMAT_LIMITS).unwrap();
    assert_eq!(edits[0].text, "color: red;");
    let input = "@media x {\n  .a{b:c}\n}";
    let start = input.find(".a").unwrap();
    let end = start + ".a{b:c}".len();
    let edits = format(input, Some(start..end), spaces(2), FORMAT_LIMITS).unwrap();
    assert_eq!(edits[0].bytes, start - 2..end);
    assert_eq!(edits[0].text, "  .a {\n      b: c\n    }");
    assert_eq!(
        format(input, Some(end..start), spaces(2), FORMAT_LIMITS).err(),
        Some(EditorError::InvalidBoundary)
    );
    assert_eq!(
        format(
            input,
            None,
            spaces(2),
            css_format::Limits {
                max_output_bytes: 4,
                ..FORMAT_LIMITS
            }
        )
        .err(),
        Some(EditorError::Capacity)
    );
}
