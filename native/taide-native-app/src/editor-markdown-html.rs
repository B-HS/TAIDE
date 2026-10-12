use dom_query::{Document, NodeRef};
use taide_model::error::AppResult;
use taide_native_editor::documentation::{
    Alignment, Block, ImageDimensions, Inline, Link, ListItem, RichDocument, Span, Style,
};
use url::Url;

const MAX_DEPTH: usize = 128;
const BLOCK_TAGS: &[&str] = &[
    "html",
    "body",
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "pre",
    "ul",
    "ol",
    "table",
    "hr",
    "div",
    "section",
    "article",
    "main",
    "header",
    "footer",
    "aside",
    "figure",
    "figcaption",
];
const REMOVED_TAGS: &[&str] = &[
    "script", "style", "iframe", "object", "embed", "template", "base", "link", "meta", "noscript",
];

fn name(node: NodeRef<'_>) -> String {
    node.node_name()
        .map(|name| name.to_string())
        .unwrap_or_default()
}

fn target(value: &str, base: Option<&Url>, image: bool) -> Option<String> {
    let uri = Url::parse(value)
        .or_else(|_| {
            base.ok_or(url::ParseError::RelativeUrlWithoutBase)?
                .join(value)
        })
        .ok()?;
    let allowed = matches!(uri.scheme(), "http" | "https" | "file")
        || (!image && uri.scheme() == "mailto")
        || (image && value.starts_with("data:image/"));
    allowed.then(|| uri.to_string())
}

fn append(contents: &mut Vec<Inline>, text: &str, style: Style, link: Option<Link>) {
    if let Some(Inline::Text(previous)) = contents.last_mut()
        && previous.style == style
        && previous.link == link
    {
        previous.text.push_str(text);
        return;
    }
    contents.push(Inline::Text(Span {
        text: text.into(),
        style,
        link,
    }));
}

fn inlines(
    node: NodeRef<'_>,
    base: Option<&Url>,
    mut style: Style,
    mut link: Option<Link>,
    output: &mut Vec<Inline>,
) {
    if node.is_text() {
        let text = node.text().to_string();
        let mut collapsed = String::new();
        let mut space = false;
        for character in text.chars() {
            if character.is_ascii_whitespace() {
                if !space {
                    collapsed.push(' ');
                }
                space = true;
            } else {
                collapsed.push(character);
                space = false;
            }
        }
        append(output, &collapsed, style, link);
        return;
    }
    let tag = name(node);
    if REMOVED_TAGS.contains(&tag.as_str()) || node.has_attr("hidden") {
        return;
    }
    match tag.as_str() {
        "strong" | "b" => style.bold = true,
        "em" | "i" => style.italic = true,
        "del" | "s" | "strike" => style.strike = true,
        "code" | "kbd" | "samp" => style.code = true,
        "a" => {
            link = node
                .attr("href")
                .and_then(|href| target(&href, base, false))
                .map(|target| Link {
                    target,
                    title: node
                        .attr("title")
                        .map(|title| title.to_string())
                        .unwrap_or_default(),
                })
        }
        "br" => {
            output.push(Inline::Break);
            return;
        }
        "input" => return,
        "img" => {
            let alt = node
                .attr("alt")
                .map(|alt| alt.to_string())
                .unwrap_or_default();
            let Some(source) = node
                .attr("src")
                .and_then(|source| target(&source, base, true))
            else {
                append(output, &alt, style, link);
                return;
            };
            output.push(Inline::Image {
                source,
                title: node
                    .attr("title")
                    .map(|title| title.to_string())
                    .unwrap_or_default(),
                alt,
                link,
                dimensions: ImageDimensions {
                    width: node.attr("width").and_then(|width| width.parse().ok()),
                    height: node.attr("height").and_then(|height| height.parse().ok()),
                },
            });
            return;
        }
        _ => {}
    }
    for child in node.children_it(false) {
        inlines(child, base, style, link.clone(), output);
    }
}

fn contents(node: NodeRef<'_>, base: Option<&Url>, bold: bool) -> Vec<Inline> {
    let mut output = Vec::new();
    for child in node.children_it(false) {
        inlines(
            child,
            base,
            Style {
                bold,
                ..Default::default()
            },
            None,
            &mut output,
        );
    }
    output
}

fn flush(blocks: &mut Vec<Block>, contents: &mut Vec<Inline>) {
    if contents.iter().any(|inline| match inline {
        Inline::Text(span) => !span.text.trim().is_empty(),
        _ => true,
    }) {
        blocks.push(Block::Paragraph(std::mem::take(contents)));
    } else {
        contents.clear();
    }
}

fn checkbox(node: NodeRef<'_>) -> Option<bool> {
    for child in node.children_it(false) {
        match name(child).as_str() {
            "input"
                if child
                    .attr("type")
                    .is_some_and(|value| value.eq_ignore_ascii_case("checkbox")) =>
            {
                return Some(child.has_attr("checked"));
            }
            "p" => {
                if let Some(checked) = checkbox(child) {
                    return Some(checked);
                }
            }
            _ => {}
        }
    }
    None
}

fn table_rows<'a>(node: NodeRef<'a>, rows: &mut Vec<NodeRef<'a>>) {
    for child in node.children_it(false) {
        match name(child).as_str() {
            "tr" => rows.push(child),
            "thead" | "tbody" | "tfoot" => table_rows(child, rows),
            _ => {}
        }
    }
}

fn blocks(node: NodeRef<'_>, base: Option<&Url>) -> Vec<Block> {
    let mut output = Vec::new();
    let mut pending = Vec::new();
    for child in node.children_it(false) {
        let tag = name(child);
        if REMOVED_TAGS.contains(&tag.as_str()) || child.has_attr("hidden") {
            continue;
        }
        if !BLOCK_TAGS.contains(&tag.as_str()) {
            inlines(child, base, Style::default(), None, &mut pending);
            continue;
        }
        flush(&mut output, &mut pending);
        match tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => output.push(Block::Heading {
                level: tag.as_bytes()[1] - b'0',
                contents: contents(child, base, false),
            }),
            "p" => output.push(Block::Paragraph(contents(child, base, false))),
            "blockquote" => output.push(Block::Quote(blocks(child, base))),
            "pre" => {
                let code = child
                    .first_element_child()
                    .filter(|node| name(*node) == "code")
                    .unwrap_or(child);
                let language = code
                    .attr("class")
                    .and_then(|class| {
                        class
                            .split_ascii_whitespace()
                            .find_map(|name| name.strip_prefix("language-").map(str::to_owned))
                    })
                    .unwrap_or_default();
                output.push(Block::Code {
                    language,
                    text: code.text().to_string(),
                });
            }
            "ul" | "ol" => output.push(Block::List {
                start: (tag == "ol").then(|| {
                    child
                        .attr("start")
                        .and_then(|start| {
                            let start = start.trim_start_matches(|character| {
                                matches!(character, '\t' | '\n' | '\u{c}' | '\r' | ' ')
                            });
                            let digits = start.strip_prefix(['-', '+']).unwrap_or(start);
                            let length = digits.bytes().take_while(u8::is_ascii_digit).count();
                            if length == 0 {
                                return None;
                            }
                            let end = start.len() - digits.len() + length;
                            start.get(..end)?.parse::<i64>().ok()
                        })
                        .unwrap_or(1)
                }),
                items: child
                    .children_it(false)
                    .filter(|node| name(*node) == "li")
                    .map(|item| ListItem {
                        checked: checkbox(item),
                        blocks: blocks(item, base),
                    })
                    .collect(),
            }),
            "table" => {
                let mut rows = Vec::new();
                table_rows(child, &mut rows);
                let columns = rows
                    .first()
                    .map(|row| {
                        row.children_it(false)
                            .filter(|node| matches!(name(*node).as_str(), "td" | "th"))
                            .map(|cell| match cell.attr("align").as_deref() {
                                Some("center") => Alignment::Center,
                                Some("right") => Alignment::Right,
                                Some("left") => Alignment::Left,
                                _ => Alignment::Default,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                output.push(Block::Table {
                    columns,
                    header: Vec::new(),
                    rows: rows
                        .into_iter()
                        .map(|row| {
                            row.children_it(false)
                                .filter(|node| matches!(name(*node).as_str(), "td" | "th"))
                                .map(|cell| contents(cell, base, name(cell) == "th"))
                                .collect()
                        })
                        .collect(),
                });
            }
            "hr" => output.push(Block::Rule),
            _ => output.extend(blocks(child, base)),
        }
    }
    flush(&mut output, &mut pending);
    output
}

pub(super) fn parse(source: &str, base: Option<&Url>) -> AppResult<RichDocument> {
    let document = Document::fragment(source);
    let mut pending = vec![(document.root(), 0)];
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_DEPTH {
            return Err(super::limit_error());
        }
        pending.extend(node.children_it(false).map(|child| (child, depth + 1)));
    }
    let mut blocks = blocks(document.root(), base);
    crate::editor_markup::autolinks::apply(&mut blocks);
    Ok(RichDocument { blocks })
}
