use super::*;
use taide_native_editor::documentation::{Block, Inline};

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
