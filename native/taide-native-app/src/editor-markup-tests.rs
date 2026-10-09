use super::*;

fn markdown(text: &str) -> RichDocument {
    parse(&Content::Markdown(text.into()))
}

fn paragraph(block: &Block) -> &[Inline] {
    let Block::Paragraph(contents) = block else {
        panic!("expected paragraph")
    };
    contents
}

#[test]
fn 강조_중첩_inline코드_줄바꿈과_안전한_링크를_구조로_보존한다() {
    let document = markdown(
        "**bold *nested*** ~~strike~~ `code` [docs](https://example.com/a \"title\")  \nnext",
    );
    let contents = paragraph(&document.blocks[0]);
    let Inline::Text(bold) = &contents[0] else {
        panic!("expected text")
    };
    assert!(bold.style.bold);
    let Inline::Text(nested) = &contents[1] else {
        panic!("expected text")
    };
    assert!(nested.style.bold && nested.style.italic);
    assert!(contents.iter().any(
        |inline| matches!(inline, Inline::Text(span) if span.style.strike && span.text == "strike")
    ));
    assert!(contents.iter().any(
        |inline| matches!(inline, Inline::Text(span) if span.style.code && span.text == "code")
    ));
    assert!(contents.iter().any(|inline| matches!(inline, Inline::Text(span) if span.link.as_ref().is_some_and(|link| link.target == "https://example.com/a" && link.title == "title"))));
    assert!(contents.contains(&Inline::Break));
}

#[test]
fn 제목_인용_순서목록_중첩목록_체크박스와_코드언어를_보존한다() {
    let document = markdown(
        "# heading\n\n> quote\n\n3. first\n   - [x] nested\n4. second\n\n```rust extra\nfn x() {}\n```\n\n---",
    );
    assert!(matches!(
        &document.blocks[0],
        Block::Heading { level: 1, .. }
    ));
    assert!(matches!(&document.blocks[1], Block::Quote(blocks) if blocks.len() == 1));
    let Block::List { start, items } = &document.blocks[2] else {
        panic!("expected list")
    };
    assert_eq!(*start, Some(3));
    assert_eq!(items.len(), 2);
    assert!(
        matches!(&items[0].blocks[1], Block::List { items, .. } if items[0].checked == Some(true))
    );
    assert_eq!(
        document.blocks[3],
        Block::Code {
            language: "rust".into(),
            text: "fn x() {}\n".into()
        }
    );
    assert_eq!(document.blocks[4], Block::Rule);
}

#[test]
fn 표는_열정렬_머리글과_각_행의_inline_강조를_보존한다() {
    let document = markdown("| left | right |\n| :--- | ---: |\n| **value** | `42` |\n");
    let Block::Table {
        columns,
        header,
        rows,
    } = &document.blocks[0]
    else {
        panic!("expected table")
    };
    assert_eq!(columns, &[Alignment::Left, Alignment::Right]);
    assert_eq!(header.len(), 2);
    assert_eq!(rows.len(), 1);
    assert!(
        matches!(&rows[0][0][0], Inline::Text(span) if span.style.bold && span.text == "value")
    );
    assert!(matches!(&rows[0][1][0], Inline::Text(span) if span.style.code && span.text == "42"));
}

#[test]
fn plaintext와_언어코드는_markdown으로_재해석하지않는다() {
    let plain = parse(&Content::Plain(
        "**literal**\n<script>literal</script>".into(),
    ));
    assert!(
        matches!(&paragraph(&plain.blocks[0])[0], Inline::Text(span) if span.text == "**literal**\n<script>literal</script>" && span.style == Style::default())
    );
    let code = parse(&Content::Code {
        language: "rust".into(),
        text: "**literal**".into(),
    });
    assert_eq!(
        code.blocks,
        vec![Block::Code {
            language: "rust".into(),
            text: "**literal**".into()
        }]
    );
}

#[test]
fn 비신뢰_html과_명령_스크립트_상대경로_링크는_실행대상에_들어가지않는다() {
    let document = markdown(
        "safe <b>text</b> [cmd](command:danger) [script](javascript:alert) [relative](../../outside) [mail](mailto:person@example.com) ![alt](javascript:alert)\n\n<script>danger</script>",
    );
    let contents = paragraph(&document.blocks[0]);
    assert!(
        contents
            .iter()
            .all(|inline| matches!(inline, Inline::Text(span) if !span.text.contains('<')))
    );
    let links = contents
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text(span) => span.link.as_ref(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "mailto:person@example.com");
    assert_eq!(document.blocks.len(), 1);
}

#[test]
fn 이미지의_대체문구와_제목은_링크와_별도로_보존한다() {
    let document = markdown(
        "before [![**diagram**](https://example.com/p.png \"image title\")](https://example.com/docs) after",
    );
    assert!(paragraph(&document.blocks[0]).iter().any(|inline| matches!(inline, Inline::Image { source, title, alt, link, .. } if source == "https://example.com/p.png" && title == "image title" && alt == "diagram" && link.as_ref().is_some_and(|link| link.target == "https://example.com/docs"))));
}

#[test]
fn 느슨한_목록의_체크박스는_항목에_속하고_두_문단을_보존한다() {
    let document = markdown("- [x] first\n\n  second paragraph\n\n- [ ] next");
    let Block::List { items, .. } = &document.blocks[0] else {
        panic!("expected list")
    };
    assert_eq!(items[0].checked, Some(true));
    assert_eq!(items[0].blocks.len(), 2);
    assert_eq!(items[1].checked, Some(false));
}

#[test]
fn gfm의_주소와_email은_링크가_되고_코드와_기존링크_안은_재해석하지않는다() {
    let document = markdown(
        "Visit https://example.com/a_(b). mail a.b+tag@example.com and www.example.com/test, `https://code.example.com` [https://label.example.com](https://target.example.com)",
    );
    let links = paragraph(&document.blocks[0])
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text(span) => span
                .link
                .as_ref()
                .map(|link| (span.text.as_str(), link.target.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        links,
        vec![
            ("https://example.com/a_(b)", "https://example.com/a_(b)"),
            ("a.b+tag@example.com", "mailto:a.b+tag@example.com"),
            ("www.example.com/test", "http://www.example.com/test"),
            ("https://label.example.com", "https://target.example.com/"),
        ]
    );
    assert!(paragraph(&document.blocks[0]).iter().any(
        |inline| matches!(inline, Inline::Text(span) if span.style.code && span.link.is_none())
    ));
    let plain = parse(&Content::Plain("https://example.com".into()));
    assert!(matches!(&paragraph(&plain.blocks[0])[0], Inline::Text(span) if span.link.is_none()));
}
