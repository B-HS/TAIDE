use super::*;
use taide_native_editor::documentation::{Block, Inline};

#[test]
fn 표_셀의_flow_구조는_문단_코드_목록_중첩_표와_상대_이미지를_보존한다() {
    let document = parse("<table><tr><th><p>first</p><p>second</p><h3>heading</h3><ol start='-1'><li>item</li></ol><pre><code class='language-rust'>let cell = 1;\nnext line</code></pre><blockquote><p><a href='https://example.com'>linked</a><img src='./nested.png'></p></blockquote><table><tr><td><p>https://example.com/nested</p></td></tr></table></th><td>plain <em>inline</em></td></tr></table>", Some(std::path::Path::new("/workspace/cell.md"))).unwrap();
    let Block::Table {
        rows,
        layout: Some(layout),
        ..
    } = &document.blocks[0]
    else {
        panic!("expected table")
    };
    assert!(rows[0][0].is_empty());
    assert!(layout.cells[0][1].blocks.is_none());
    assert!(
        matches!(&rows[0][1][1], Inline::Text(span) if span.text == "inline" && span.style.italic)
    );
    let blocks = layout.cells[0][0].blocks.as_ref().unwrap();
    assert!(
        matches!(&blocks[0], Block::Paragraph(values) if matches!(&values[0], Inline::Text(span) if span.text == "first" && span.style.bold))
    );
    assert!(
        matches!(&blocks[1], Block::Paragraph(values) if matches!(&values[0], Inline::Text(span) if span.text == "second" && span.style.bold))
    );
    assert!(matches!(&blocks[2], Block::Heading { level: 3, .. }));
    assert!(matches!(
        &blocks[3],
        Block::List {
            start: Some(-1),
            ..
        }
    ));
    assert!(
        matches!(&blocks[4], Block::Code { language, text } if language == "rust" && text == "let cell = 1;\nnext line")
    );
    let Block::Quote(quote) = &blocks[5] else {
        panic!("expected quote")
    };
    let Block::Paragraph(values) = &quote[0] else {
        panic!("expected paragraph")
    };
    assert!(
        matches!(&values[0], Inline::Text(span) if span.link.as_ref().is_some_and(|link| link.target == "https://example.com/"))
    );
    assert!(
        matches!(&values[1], Inline::Image { source, .. } if source == "file:///workspace/nested.png")
    );
    let Block::Table {
        layout: Some(nested),
        ..
    } = &blocks[6]
    else {
        panic!("expected nested table")
    };
    let Block::Paragraph(values) = &nested.cells[0][0].blocks.as_ref().unwrap()[0] else {
        panic!("expected nested paragraph")
    };
    assert!(
        matches!(&values[0], Inline::Text(span) if span.link.as_ref().is_some_and(|link| link.target == "https://example.com/nested"))
    );
}

#[test]
fn html_목록은_원본_parent_css와_개별_type_정화_우선순위를_보존한다() {
    for tag in ["ol", "ul"] {
        for (kind, expected) in [
            ("a", Some("aa.")),
            ("A", Some("AA.")),
            ("i", Some("xxvii.")),
            ("I", Some("XXVII.")),
            ("1", Some("27.")),
            ("disc", Some("\u{2022}")),
            ("CiRcLe", Some("\u{25e6}")),
            ("SQUARE", Some("\u{25aa}")),
            ("NONE", None),
            (" a ", Some("aa.")),
            ("\u{feff}\u{a0}a\u{feff}", Some("aa.")),
        ] {
            let document = parse(
                &format!("<{tag} type='A'><li type='{kind}' value='27'>item</li></{tag}>"),
                None,
            )
            .unwrap();
            let Block::List { start, items } = &document.blocks[0] else {
                panic!("expected list")
            };
            assert_eq!(items[0].marker(*start, 0).as_deref(), expected);
        }
        for kind in ["invalid", "aA", "", "\u{85}a\u{85}"] {
            let document = parse(
                &format!("<{tag} type='A'><li type='{kind}'>item</li></{tag}>"),
                None,
            )
            .unwrap();
            let Block::List { start, items } = &document.blocks[0] else {
                panic!("expected list")
            };
            let expected = if tag == "ol" { "1." } else { "\u{2022}" };
            assert_eq!(items[0].marker(*start, 0).as_deref(), Some(expected));
        }
    }
    let document = parse("<ul><li value='27'>plain<li type='A'>alpha<li type='I' value='4'>roman<li type='none'>none<li type='a'>last</ul><ol reversed type='A'><li type='A'>first<li type='none'>hidden marker<li type='a'>last</ol>", None).unwrap();
    for (block, expected) in document.blocks.iter().zip([
        vec![Some("\u{2022}"), Some("AB."), Some("IV."), None, Some("f.")],
        vec![Some("C."), None, Some("a.")],
    ]) {
        let Block::List { start, items } = block else {
            panic!("expected list")
        };
        assert_eq!(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| item.marker(*start, index))
                .collect::<Vec<_>>(),
            expected
                .into_iter()
                .map(|value| value.map(str::to_owned))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn html_표는_제목_머리_본문_꼬리와_행_그룹_병합을_보존한다() {
    const ROW_COUNT: usize = 4;
    const COLUMN_COUNT: usize = 2;
    const BODY_SECOND: usize = 2;
    const FOOT_ROW: usize = 3;
    let document = parse("<table><caption><strong>caption</strong> <a href='../next.md'>next</a></caption><tfoot><tr><td>footer</td></tr></tfoot><tbody><tr><td rowspan='0'>span</td><td>body first</td></tr><tr><td>body second</td></tr></tbody><thead><tr><th colspan='2'>head</th></tr></thead></table>", Some(Path::new("/synthetic/docs/readme.md"))).unwrap();
    let Block::Table {
        rows,
        layout: Some(layout),
        ..
    } = &document.blocks[0]
    else {
        panic!("expected table")
    };
    assert_eq!(
        (layout.row_count, layout.column_count),
        (ROW_COUNT, COLUMN_COUNT)
    );
    assert_eq!(layout.cells[0][0].column_span, COLUMN_COUNT);
    assert_eq!(layout.cells[1][0].row_span, COLUMN_COUNT);
    assert_eq!(layout.cells[BODY_SECOND][0].column, 1);
    assert_eq!(layout.cells[FOOT_ROW][0].column, 0);
    for (row, expected) in [
        (0, "head"),
        (1, "span"),
        (BODY_SECOND, "body second"),
        (FOOT_ROW, "footer"),
    ] {
        assert!(matches!(&rows[row][0][0], Inline::Text(span) if span.text == expected));
    }
    assert!(layout.caption.iter().any(
        |inline| matches!(inline, Inline::Text(span) if span.style.bold && span.text == "caption")
    ));
    assert!(layout.caption.iter().any(|inline| matches!(inline, Inline::Text(span) if span.link.as_ref().is_some_and(|link| link.target == "file:///synthetic/next.md"))));
}

#[test]
fn html_병합_속성은_0_무효값_상한과_암시된_행을_처리한다() {
    const COLUMN_MAX: usize = 1000;
    const ROW_MAX: usize = 65534;
    for (value, expected) in [
        ("0", 1),
        ("bad", 1),
        ("-2", 1),
        (" \t+2tail", 2),
        ("999999999999999999999999999", COLUMN_MAX),
    ] {
        let document = parse(
            &format!("<table><tr><td colspan='{value}'>cell</td></tr></table>"),
            None,
        )
        .unwrap();
        assert!(
            matches!(&document.blocks[0], Block::Table { layout: Some(layout), .. } if layout.cells[0][0].column_span == expected && layout.column_count == expected)
        );
    }
    let document = parse("<table><tbody><tr><td rowspan='999999999999999999999'>cell</td></tr></tbody><tbody><tr><td>next group</td></tr></tbody></table>", None).unwrap();
    let Block::Table {
        rows,
        layout: Some(layout),
        ..
    } = &document.blocks[0]
    else {
        panic!("expected table")
    };
    assert_eq!(layout.cells[0][0].row_span, ROW_MAX);
    assert_eq!(layout.row_count, ROW_MAX + 1);
    assert!(matches!(&rows[ROW_MAX][0][0], Inline::Text(span) if span.text == "next group"));
    assert_eq!(layout.cells[ROW_MAX][0].column, 0);
}

#[test]
fn html_열_병합은_실제_열_수를_보존한다() {
    const COLUMN_COUNT: usize = 3;
    let document = parse("<table><tr><td colspan='2'>wide</td><td rowspan='2'>tall</td></tr><tr><td>a</td><td>b</td></tr></table>", None).unwrap();
    assert!(
        matches!(&document.blocks[0], Block::Table { columns, .. } if columns.len() == COLUMN_COUNT)
    );
}

#[test]
fn html_목록의_역순과_항목_value는_후속_번호와_중첩_범위를_보존한다() {
    for (attributes, items, expected) in [
        (
            "reversed",
            "<li>first<li value='-4'>second<li>third",
            vec![3, -4, -5],
        ),
        (
            "reversed='false' start='+7tail'",
            "<li>first<li value='0'>second<li>third",
            vec![7, 0, -1],
        ),
        (
            "start='-2'",
            "<li>first<li value=' +7suffix'>second<li value='bad'>third",
            vec![-2, 7, 8],
        ),
        (
            "reversed start='bad'",
            "<li>first<li hidden>hidden<li>third",
            vec![2, 1],
        ),
    ] {
        let document = parse(&format!("<ol {attributes}>{items}</ol>"), None).unwrap();
        let Block::List { items, .. } = &document.blocks[0] else {
            panic!("expected list")
        };
        assert_eq!(
            items
                .iter()
                .map(|item| item.ordinal.unwrap())
                .collect::<Vec<_>>(),
            expected
        );
    }
    let document = parse("<ol reversed><li>outer<ol start='6'><li>nested<li value='-1'>reset</ol><li>last</ol><ul reversed><li value='7'>unordered</ul>", None).unwrap();
    let Block::List { items, .. } = &document.blocks[0] else {
        panic!("expected outer list")
    };
    assert_eq!(
        items
            .iter()
            .map(|item| item.ordinal.unwrap())
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert!(
        matches!(&items[0].blocks[1], Block::List { start: Some(6), items } if items.iter().map(|item| item.ordinal.unwrap()).collect::<Vec<_>>() == vec![6, -1])
    );
    assert!(
        matches!(&document.blocks[1], Block::List { start: None, items } if items[0].ordinal.is_none())
    );
}

#[test]
fn html_역순_목록은_항목_수에서_시작한다() {
    let document = parse(
        "<ol reversed><li>first</li><li>second</li><li>third</li></ol>",
        None,
    )
    .unwrap();
    assert!(matches!(
        &document.blocks[0],
        Block::List { start: Some(3), .. }
    ));
}

#[test]
fn html_순서_목록은_음수와_0의_시작_값을_보존한다() {
    for start in ["-3", "0", "7"] {
        let document = parse(
            &format!("<ol start='{start}'><li>first</li><li>second</li></ol>"),
            None,
        )
        .unwrap();
        assert!(
            matches!(&document.blocks[0], Block::List { start: Some(number), items } if number.to_string() == start && items.len() > 1)
        );
    }
    for (value, expected) in [(" \t-3tail", -3), ("+7suffix", 7), ("bad", 1), ("-", 1)] {
        let document = parse(&format!("<ol start='{value}'><li>first</li></ol>"), None).unwrap();
        assert!(
            matches!(&document.blocks[0], Block::List { start: Some(number), .. } if *number == expected)
        );
    }
}

#[test]
fn gfm과_html의_인용_목록_체크박스_표와_코드는_내용과_정렬을_보존한다() {
    let document = parse(
        "> quote\n\n3. first\n   - [x] checked\n\n| left | right |\n| :--- | ---: |\n| cell | value |\n\n```rust\n<&>  code\n```\n\n<table><tr><td>plain</td><th>bold</th></tr></table>\n",
        None,
    ).unwrap();
    assert!(
        matches!(&document.blocks[0], Block::Quote(contents) if matches!(&contents[0], Block::Paragraph(_)))
    );
    assert!(
        matches!(&document.blocks[1], Block::List { start: Some(3), items } if matches!(&items[0].blocks[1], Block::List { items, .. } if items[0].checked == Some(true)))
    );
    assert!(
        matches!(&document.blocks[2], Block::Table { columns, rows, .. } if columns == &vec![taide_native_editor::documentation::Alignment::Left, taide_native_editor::documentation::Alignment::Right] && rows.len() == 2)
    );
    assert!(
        matches!(&document.blocks[3], Block::Code { language, text } if language == "rust" && text == "<&>  code\n")
    );
    assert!(
        matches!(&document.blocks[4], Block::Table { rows, .. } if matches!(&rows[0][0][0], Inline::Text(span) if span.text == "plain" && !span.style.bold) && matches!(&rows[0][1][0], Inline::Text(span) if span.text == "bold" && span.style.bold))
    );
}

#[test]
fn html_이벤트_위험_uri_숨김_요소와_따옴표는_native_실행_경계를_열지_않는다() {
    let document = parse(
        "<p><a href='javascript:alert(1)' onclick='evil()'>safe label</a><img src='data:text/html,evil' alt='blocked image'><span hidden>private text</span><strong>bold &amp; text</strong></p>\n\n`<script>visible code</script>`\n",
        None,
    ).unwrap();
    let rendered = format!("{document:?}");
    assert!(!rendered.contains("javascript:"));
    assert!(!rendered.contains("onclick"));
    assert!(!rendered.contains("data:text"));
    assert!(!rendered.contains("private text"));
    assert!(rendered.contains("safe label"));
    assert!(rendered.contains("blocked image"));
    assert!(rendered.contains("bold & text"));
    assert!(rendered.contains("<script>visible code</script>"));
    let source = "[label](https://example.com \"a & title\")";
    let document = parse(source, None).unwrap();
    assert!(
        matches!(&document.blocks[0], Block::Paragraph(contents) if matches!(&contents[0], Inline::Text(span) if span.link.as_ref().is_some_and(|link| link.title == "a & title")))
    );
}

#[test]
fn html_중첩과_출력_용량_초과는_부분_출력이나_panic_대신_오류를_반환한다() {
    const DEEP_NESTING: usize = 256;
    let source = format!(
        "{}safe{}",
        "<div>".repeat(DEEP_NESTING),
        "</div>".repeat(DEEP_NESTING)
    );
    assert!(parse(&source, None).is_err());
    let mut output = Output(String::new());
    let chunk = "a".repeat(RESERVE_BYTES);
    for _ in 0..crate::preview_web_document::MAX_OUTPUT_BYTES / RESERVE_BYTES {
        output.write_str(&chunk).unwrap();
    }
    assert!(output.write_str("a").is_err());
    assert_eq!(
        output.0.len(),
        crate::preview_web_document::MAX_OUTPUT_BYTES
    );
    assert!(output.0.capacity() <= crate::preview_web_document::MAX_OUTPUT_BYTES);
}

#[test]
fn markdown의_html_제목_강조_상대_링크와_이미지는_구조로_표시하고_스크립트를_제외한다() {
    let path = Path::new("/synthetic/project/docs/readme.md");
    let document = parse(
        "<h2>html heading</h2>\n\nHello <strong>bold</strong> and [next](../next.md). ![diagram](./image.png)\n\n<script>alert('hidden')</script>\n",
        Some(path),
    ).unwrap();
    assert!(
        matches!(&document.blocks[0], Block::Heading {level: 2, contents} if matches!(&contents[0], Inline::Text(span) if span.text == "html heading"))
    );
    let Block::Paragraph(contents) = &document.blocks[1] else {
        panic!("expected paragraph")
    };
    assert!(contents.iter().any(
        |inline| matches!(inline, Inline::Text(span) if span.text == "bold" && span.style.bold)
    ));
    assert!(contents.iter().any(|inline| matches!(inline, Inline::Text(span) if span.link.as_ref().is_some_and(|link| link.target == "file:///synthetic/project/next.md"))));
    assert!(contents.iter().any(|inline| matches!(inline, Inline::Image { source, alt, .. } if source == "file:///synthetic/project/docs/image.png" && alt == "diagram")));
    assert!(!format!("{document:?}").contains("hidden"));
}
