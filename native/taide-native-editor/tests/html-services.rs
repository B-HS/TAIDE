use taide_native_editor::document::EditorError;
use taide_native_editor::folding::FoldRegion;
use taide_native_editor::html_folding;
use taide_native_editor::html_format;
use taide_native_editor::html_parser;
use taide_native_editor::html_scanner::{Scanner, ScannerState, TokenType};
use taide_native_editor::html_selection;
use taide_native_editor::html_symbols;
use taide_native_editor::indent::IndentOptions;

#[test]
fn html_selection은_속성값_속성_시작태그_요소_부모_순서의_확장_사슬을_원본처럼_만든다() {
    let limits = html_selection::Limits {
        parser: PARSER,
        max_positions: 8,
        max_ranges: MAX,
    };
    let element = "<div class=\"foo\">bar</div>";
    assert_eq!(
        html_selection::selection_ranges(element, &[13, 18, 23], limits).unwrap(),
        vec![
            vec![12..15, 11..16, 5..16, 1..16, 0..26],
            vec![17..20, 0..26],
            vec![22..25, 0..26],
        ]
    );
    assert_eq!(
        html_selection::selection_ranges("<p><br></p>", &[5], limits).unwrap(),
        vec![vec![4..6, 3..7, 0..11]]
    );
    assert_eq!(
        html_selection::selection_ranges("x", &[0], limits).unwrap(),
        vec![vec![0..0]]
    );
    assert_eq!(
        html_selection::selection_ranges(element, &[0; 9], limits),
        Err(EditorError::Capacity)
    );
}

const MAX: usize = 1 << 20;
const PARSER: html_parser::Limits = html_parser::Limits {
    max_bytes: MAX,
    max_tokens: MAX,
    max_nodes: MAX,
};
const SYMBOLS: html_symbols::Limits = html_symbols::Limits {
    parser: PARSER,
    max_text_bytes: MAX,
};
const FOLDING: html_folding::Limits = html_folding::Limits {
    max_bytes: MAX,
    max_tokens: MAX,
    max_depth: MAX,
    max_ranges: MAX,
};
const FORMAT: html_format::Limits = html_format::Limits {
    max_bytes: MAX,
    max_output_bytes: MAX,
    max_indent_width: 1024,
    max_tokens: MAX,
    max_depth: 64,
};
const SPACES: html_format::Options = html_format::Options {
    indentation: IndentOptions {
        tab_size: 4,
        insert_spaces: true,
    },
};
const WRAPPED_WORDS: usize = 30;
const FIRST_LINE_WORDS: usize = 23;

fn tokens(text: &str) -> Vec<(TokenType, String)> {
    let mut scanner = Scanner::new(text, 0, ScannerState::WithinContent, false);
    let mut tokens = Vec::new();
    loop {
        let token = scanner.scan();
        if token == TokenType::Eos {
            return tokens;
        }
        tokens.push((token, scanner.token_text().to_owned()));
    }
}

fn format(
    text: &str,
    range: Option<std::ops::Range<usize>>,
    options: html_format::Options,
) -> String {
    let edits = html_format::format(text, range, options, FORMAT).unwrap();
    let mut output = text.to_owned();
    for edit in edits.into_iter().rev() {
        output.replace_range(edit.bytes, &edit.text);
    }
    output
}

fn folds(text: &str) -> Vec<(usize, usize, Option<String>)> {
    html_folding::folding_ranges(text, FOLDING)
        .unwrap()
        .into_iter()
        .map(|range| (range.region.start_line, range.region.end_line, range.kind))
        .collect()
}

#[test]
fn html_scanner는_태그_속성_script_style_내용을_원본_token으로_나눈다() {
    use TokenType::*;
    let expected = [
        (StartTagOpen, "<"),
        (StartTag, "div"),
        (Whitespace, " "),
        (AttributeName, "class"),
        (DelimiterAssign, "="),
        (AttributeValue, "\"a\""),
        (Whitespace, " "),
        (AttributeName, "hidden"),
        (StartTagClose, ">"),
        (Content, "x"),
        (EndTagOpen, "</"),
        (EndTag, "div"),
        (EndTagClose, ">"),
        (StartCommentTag, "<!--"),
        (Comment, " c "),
        (EndCommentTag, "-->"),
        (StartDoctypeTag, "<!DOCTYPE"),
        (Doctype, " html"),
        (EndDoctypeTag, ">"),
    ];
    let actual = tokens("<div class=\"a\" hidden>x</div><!-- c --><!DOCTYPE html>");
    assert_eq!(
        actual,
        expected
            .iter()
            .map(|(token, text)| (*token, (*text).to_owned()))
            .collect::<Vec<_>>()
    );
    let script = tokens("<script><!--<script></script>--></script>x");
    assert!(script.contains(&(Script, "<!--<script></script>-->".to_owned())));
    assert_eq!(script.last(), Some(&(Content, "x".to_owned())));
    let template = tokens("<script type=\"text/html\"><b></b></script>");
    assert!(template.contains(&(StartTag, "b".to_owned())));
    let style = tokens("<style>a{b:c}</STYLE>");
    assert!(style.contains(&(Styles, "a{b:c}".to_owned())));
    assert!(style.contains(&(EndTag, "STYLE".to_owned())));
    let unquoted = tokens("<a href=http://x/>");
    assert!(unquoted.contains(&(AttributeValue, "http://x".to_owned())));
    assert_eq!(unquoted.last(), Some(&(StartTagSelfClose, "/>".to_owned())));
    let missing_name = tokens("< >");
    assert_eq!(
        missing_name,
        vec![
            (StartTagOpen, "<".to_owned()),
            (Whitespace, " ".to_owned()),
            (StartTagClose, ">".to_owned())
        ]
    );
}

#[test]
fn html_parser와_symbols는_암시적_닫힘_void_요소_id_class_이름을_원본처럼_만든다() {
    let text = "<div id=\"m\" class=\"a  b\"><p>x<br><img/></p><span>y</div>";
    let document = html_parser::parse(text, PARSER).unwrap();
    let div = document.roots()[0];
    assert_eq!(document.nodes[div].attribute("id"), Some(Some("\"m\"")));
    assert!(document.nodes[div].closed);
    let span = *document.nodes[div].children.last().unwrap();
    assert!(!document.nodes[span].closed);
    let symbols = html_symbols::document_symbols(text, SYMBOLS).unwrap();
    let actual = symbols
        .iter()
        .map(|symbol| {
            (
                symbol.name.as_str(),
                symbol.container.as_str(),
                symbol.bytes.clone(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            ("div#m.a.b", "", 0..56),
            ("p", "div#m.a.b", 25..43),
            ("br", "p", 29..33),
            ("img", "p", 33..39),
            ("span", "div#m.a.b", 43..50),
        ]
    );
    let spaced = html_symbols::document_symbols("<i class=\" x \" id=''></i>< >", SYMBOLS).unwrap();
    assert_eq!(spaced[0].name, "i#..x.");
    assert_eq!(spaced[1].name, "?");
    assert_eq!(
        html_parser::parse(
            "<a>",
            html_parser::Limits {
                max_nodes: 1,
                ..PARSER
            }
        ),
        Err(EditorError::Capacity)
    );
}

#[test]
fn html_folding은_요소_region_여러줄_주석과_같은_시작줄_중복을_원본처럼_계산한다() {
    let text = "<html>\n<body>\n  <div>\n    <p>a</p>\n  </div>\n  <!-- #region x -->\n  <br\n   >\n  <!-- #endregion -->\n  <!--\n  multi\n  -->\n</body>\n</html>";
    assert_eq!(
        folds(text),
        vec![
            (0, 12, None),
            (1, 11, None),
            (2, 3, None),
            (5, 8, Some("region".to_owned())),
            (9, 11, Some("comment".to_owned())),
        ]
    );
    assert_eq!(folds("<ul><li>\nx\n</li></ul>"), vec![(0, 1, None)]);
    assert_eq!(
        folds("<!-- #regionx -->\n<p>\n</p>\n<!-- myendregion -->"),
        Vec::<(usize, usize, Option<String>)>::new()
    );
    let ranges = html_folding::folding_ranges("<a>\n\n</a>", FOLDING).unwrap();
    assert_eq!(
        ranges[0].region,
        FoldRegion {
            start_line: 0,
            end_line: 1
        }
    );
    assert_eq!(
        html_folding::folding_ranges(
            "<a><b>",
            html_folding::Limits {
                max_depth: 1,
                ..FOLDING
            }
        ),
        Err(EditorError::Capacity)
    );
}

#[test]
fn html_format은_block_inline_unformatted_extra_liner_pre_주석_줄바꿈을_원본처럼_배치한다() {
    assert_eq!(
        format("<div><p>a</p><span>b</span></div>", None, SPACES),
        "<div>\n    <p>a</p><span>b</span>\n</div>"
    );
    assert_eq!(
        format(
            "<html><head><title>T</title></head><body><p>x</p></body></html>",
            None,
            SPACES
        ),
        "<html>\n\n<head>\n    <title>T</title>\n</head>\n\n<body>\n    <p>x</p>\n</body>\n\n</html>"
    );
    assert_eq!(
        format("<div>\n<pre>\n  a\n   b</pre>\n</div>", None, SPACES),
        "<div>\n    <pre>\n  a\n   b</pre>\n</div>"
    );
    assert_eq!(
        format("<div><br><!-- c --></div>", None, SPACES),
        "<div><br><!-- c --></div>"
    );
    let words = vec!["aaaa"; WRAPPED_WORDS];
    let expected = format!(
        "<div>{}\n    {}</div>",
        words[..FIRST_LINE_WORDS].join(" "),
        words[FIRST_LINE_WORDS..].join(" ")
    );
    assert_eq!(
        format(&format!("<div>{}</div>", words.join(" ")), None, SPACES),
        expected
    );
    let tabs = html_format::Options {
        indentation: IndentOptions {
            tab_size: 4,
            insert_spaces: false,
        },
    };
    assert_eq!(
        format("<ul><li>a<li>b</ul>", None, tabs),
        "<ul>\n\t<li>a\n\t<li>b\n</ul>"
    );
}

#[test]
fn html_format은_style을_css_beautifier로_script는_그대로_들여쓰고_주석감싸기를_유지한다() {
    assert_eq!(
        format(
            "<style>\na{color:red}\n</style>\n<script>\n  var x=1;\n</script>",
            None,
            SPACES
        ),
        "<style>\n    a {\n        color: red\n    }\n</style>\n<script>\n    var x=1;\n</script>"
    );
    assert_eq!(
        format("<script><!--\n  a();\n--></script>", None, SPACES),
        "<script>\n    <!--\n    a();\n    -->\n</script>"
    );
    assert_eq!(
        format(
            "<script type=\"text/template\">\n  a\n    b\n</script>",
            None,
            SPACES
        ),
        "<script type=\"text/template\">\n    a\n    b\n</script>"
    );
}

#[test]
fn html_format의_범위는_줄_공백을_확장하고_시작_들여쓰기를_유지하며_태그_안이면_원문을_돌려준다() {
    let text = "<div>\n    <p>a</p><p>b</p>\n</div>";
    let start = text.find("<p>").unwrap();
    let end = text.rfind("</p>").unwrap() + "</p>".len();
    let edits = html_format::format(text, Some(start..end), SPACES, FORMAT).unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].bytes, 6..end);
    assert_eq!(edits[0].text, "    <p>a</p>\n    <p>b</p>");
    let tag = "<div class=\"a\">x</div>";
    let edits = html_format::format(tag, Some(5..10), SPACES, FORMAT).unwrap();
    assert_eq!(edits[0].bytes, 5..10);
    assert_eq!(edits[0].text, "class");
    assert_eq!(
        html_format::format(
            "<p>",
            None,
            SPACES,
            html_format::Limits {
                max_output_bytes: 1,
                ..FORMAT
            }
        )
        .err(),
        Some(EditorError::Capacity)
    );
}
