use dom_query::{Document, NodeRef};
use taide_model::error::AppResult;
use taide_native_editor::documentation::{
    Block, ImageDimensions, Inline, Link, ListItem, ListMarkerStyle, RichDocument, Span, Style,
};
use url::Url;

#[path = "editor-markdown-table.rs"]
mod table;

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

fn append(
    contents: &mut Vec<Inline>,
    text: &str,
    style: Style,
    link: Option<Link>,
    code_element: Option<usize>,
) {
    if let Some(Inline::Text(previous)) = contents.last_mut()
        && previous.style == style
        && previous.link == link
        && previous.code_element == code_element
    {
        previous.text.push_str(text);
        return;
    }
    contents.push(Inline::Text(Span {
        text: text.into(),
        style,
        link,
        code_element,
    }));
}

fn inlines(
    node: NodeRef<'_>,
    base: Option<&Url>,
    mut style: Style,
    mut link: Option<Link>,
    output: &mut Vec<Inline>,
    mut code_element: Option<usize>,
    next_code_element: &mut usize,
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
        append(output, &collapsed, style, link, code_element);
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
        "code" => {
            style.code = true;
            code_element = Some(*next_code_element);
            *next_code_element += 1;
        }
        "kbd" | "samp" => style.code = true,
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
                append(output, &alt, style, link, code_element);
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
        inlines(
            child,
            base,
            style,
            link.clone(),
            output,
            code_element,
            next_code_element,
        );
    }
}

fn contents(node: NodeRef<'_>, base: Option<&Url>, bold: bool) -> Vec<Inline> {
    let mut output = Vec::new();
    let mut next_code_element = 0;
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
            None,
            &mut next_code_element,
        );
    }
    output
}

fn flush(blocks: &mut Vec<Block>, contents: &mut Vec<Inline>) {
    if contents.iter().any(|inline| match inline {
        Inline::Text(span) => span
            .text
            .chars()
            .any(|character| !character.is_ascii_whitespace()),
        _ => true,
    }) {
        blocks.push(Block::Inline(std::mem::take(contents)));
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

fn integer(value: &str) -> Option<i64> {
    let value = value
        .trim_start_matches(|character| matches!(character, '\t' | '\n' | '\u{c}' | '\r' | ' '));
    let digits = value.strip_prefix(['-', '+']).unwrap_or(value);
    let length = digits.bytes().take_while(u8::is_ascii_digit).count();
    if length == 0 {
        return None;
    }
    let end = value.len() - digits.len() + length;
    value.get(..end)?.parse().ok()
}

fn marker_style(value: &str) -> Option<ListMarkerStyle> {
    let value = value.trim_matches(|character| {
        matches!(
            character,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    });
    match value {
        "1" => Some(ListMarkerStyle::Decimal),
        "a" => Some(ListMarkerStyle::LowerAlpha),
        "A" => Some(ListMarkerStyle::UpperAlpha),
        "i" => Some(ListMarkerStyle::LowerRoman),
        "I" => Some(ListMarkerStyle::UpperRoman),
        value if value.eq_ignore_ascii_case("disc") => Some(ListMarkerStyle::Disc),
        value if value.eq_ignore_ascii_case("circle") => Some(ListMarkerStyle::Circle),
        value if value.eq_ignore_ascii_case("square") => Some(ListMarkerStyle::Square),
        value if value.eq_ignore_ascii_case("none") => Some(ListMarkerStyle::None),
        _ => None,
    }
}

fn blocks(node: NodeRef<'_>, base: Option<&Url>, bold: bool) -> AppResult<Vec<Block>> {
    let mut output = Vec::new();
    let mut pending = Vec::new();
    let mut next_code_element = 0;
    for child in node.children_it(false) {
        let tag = name(child);
        if REMOVED_TAGS.contains(&tag.as_str()) || child.has_attr("hidden") {
            continue;
        }
        if !BLOCK_TAGS.contains(&tag.as_str()) {
            inlines(
                child,
                base,
                Style {
                    bold,
                    ..Default::default()
                },
                None,
                &mut pending,
                None,
                &mut next_code_element,
            );
            continue;
        }
        flush(&mut output, &mut pending);
        match tag.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => output.push(Block::Heading {
                level: tag.as_bytes()[1] - b'0',
                contents: contents(
                    child,
                    base,
                    bold && !matches!(tag.as_str(), "h1" | "h2" | "h3"),
                ),
            }),
            "p" => output.push(Block::Paragraph(contents(child, base, bold))),
            "blockquote" => output.push(Block::Quote(blocks(child, base, bold)?)),
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
                    style: Style {
                        bold,
                        ..Default::default()
                    },
                });
            }
            "ul" | "ol" => {
                let items = child
                    .children_it(false)
                    .filter(|node| name(*node) == "li" && !node.has_attr("hidden"))
                    .collect::<Vec<_>>();
                let is_reversed = tag == "ol" && child.has_attr("reversed");
                let start = (tag == "ol").then(|| {
                    child
                        .attr("start")
                        .and_then(|value| integer(&value))
                        .unwrap_or_else(|| {
                            if is_reversed {
                                return i64::try_from(items.len()).unwrap_or(i64::MAX);
                            }
                            1
                        })
                });
                let mut ordinal = start.unwrap_or(1);
                let items = items
                    .into_iter()
                    .map(|item| {
                        let marker_style = item.attr("type").and_then(|value| marker_style(&value));
                        ordinal = item
                            .attr("value")
                            .and_then(|value| integer(&value))
                            .unwrap_or(ordinal);
                        let number = (start.is_some() || marker_style.is_some()).then_some(ordinal);
                        ordinal = if is_reversed {
                            ordinal.saturating_sub(1)
                        } else {
                            ordinal.saturating_add(1)
                        };
                        Ok(ListItem {
                            checked: checkbox(item),
                            ordinal: number,
                            marker_style,
                            blocks: blocks(item, base, bold)?,
                        })
                    })
                    .collect::<AppResult<_>>()?;
                output.push(Block::List {
                    start,
                    items,
                    style: Style {
                        bold,
                        ..Default::default()
                    },
                });
            }
            "table" => output.push(table::parse(child, base, bold)?),
            "hr" => output.push(Block::Rule),
            _ => output.extend(blocks(child, base, bold)?),
        }
    }
    flush(&mut output, &mut pending);
    Ok(output)
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
    let mut blocks = blocks(document.root(), base, false)?;
    crate::editor_markup::autolinks::apply(&mut blocks);
    Ok(RichDocument { blocks })
}
