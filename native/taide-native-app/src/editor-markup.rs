use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use taide_native_editor::documentation::{
    Alignment, Block, Content, Inline, Link, ListItem, RichDocument, Span, Style,
};

#[path = "editor-markup-autolinks.rs"]
mod autolinks;

enum Kind {
    Root,
    Paragraph,
    Heading(u8),
    Quote,
    List(Option<u64>),
    Item,
    Code(String),
    Table(Vec<Alignment>),
    TableHead,
    TableRow,
    TableCell,
    Image {
        source: String,
        title: String,
        link: Option<Link>,
        dimensions: taide_native_editor::documentation::ImageDimensions,
    },
    Ignored,
}

struct Frame {
    kind: Kind,
    blocks: Vec<Block>,
    inlines: Vec<Inline>,
    text: String,
    items: Vec<ListItem>,
    cells: Vec<Vec<Inline>>,
    header: Vec<Vec<Inline>>,
    rows: Vec<Vec<Vec<Inline>>>,
    checked: Option<bool>,
}

impl Frame {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            blocks: Vec::new(),
            inlines: Vec::new(),
            text: String::new(),
            items: Vec::new(),
            cells: Vec::new(),
            header: Vec::new(),
            rows: Vec::new(),
            checked: None,
        }
    }

    fn flush(&mut self) {
        if !self.inlines.is_empty() {
            self.blocks
                .push(Block::Paragraph(std::mem::take(&mut self.inlines)));
        }
    }
}

pub(crate) fn parse(content: &Content) -> RichDocument {
    let markdown = match content {
        Content::Plain(text) => {
            return RichDocument {
                blocks: vec![Block::Paragraph(vec![Inline::Text(Span {
                    text: text.clone(),
                    ..Default::default()
                })])],
            };
        }
        Content::Code { language, text } => {
            return RichDocument {
                blocks: vec![Block::Code {
                    language: language.clone(),
                    text: text.clone(),
                }],
            };
        }
        Content::Markdown(text) => text,
    };
    let mut frames = vec![Frame::new(Kind::Root)];
    let mut styles = Vec::new();
    let mut style = Style::default();
    let mut link = None;
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(tag) => {
                let kind = match tag {
                    Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } => {
                        styles.push((style, link.clone()));
                        match tag {
                            Tag::Emphasis => style.italic = true,
                            Tag::Strong => style.bold = true,
                            Tag::Strikethrough => style.strike = true,
                            Tag::Link {
                                dest_url, title, ..
                            } => link = safe_link(&dest_url, &title),
                            _ => unreachable!(),
                        }
                        continue;
                    }
                    Tag::Paragraph => Kind::Paragraph,
                    Tag::Heading { level, .. } => Kind::Heading(level as u8),
                    Tag::BlockQuote(_) => Kind::Quote,
                    Tag::List(start) => Kind::List(start),
                    Tag::Item => Kind::Item,
                    Tag::CodeBlock(kind) => Kind::Code(match kind {
                        CodeBlockKind::Fenced(language) => language
                            .split_whitespace()
                            .next()
                            .unwrap_or_default()
                            .into(),
                        CodeBlockKind::Indented => String::new(),
                    }),
                    Tag::Table(columns) => Kind::Table(
                        columns
                            .into_iter()
                            .map(|column| match column {
                                pulldown_cmark::Alignment::None => Alignment::Default,
                                pulldown_cmark::Alignment::Left => Alignment::Left,
                                pulldown_cmark::Alignment::Center => Alignment::Center,
                                pulldown_cmark::Alignment::Right => Alignment::Right,
                            })
                            .collect(),
                    ),
                    Tag::TableHead => Kind::TableHead,
                    Tag::TableRow => Kind::TableRow,
                    Tag::TableCell => Kind::TableCell,
                    Tag::Image {
                        dest_url, title, ..
                    } => {
                        let (source, dimensions) = safe_image(&dest_url).unwrap_or_default();
                        Kind::Image {
                            source,
                            dimensions,
                            title: title.into_string(),
                            link: link.clone(),
                        }
                    }
                    _ => Kind::Ignored,
                };
                if !matches!(kind, Kind::Image { .. }) {
                    frames.last_mut().unwrap().flush();
                }
                frames.push(Frame::new(kind));
            }
            Event::End(
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link,
            ) => {
                if let Some(previous) = styles.pop() {
                    (style, link) = previous;
                }
            }
            Event::End(_) => finish(&mut frames),
            Event::Text(text) => {
                let frame = frames.last_mut().unwrap();
                if matches!(frame.kind, Kind::Code(_)) {
                    frame.text.push_str(&text);
                } else if !matches!(frame.kind, Kind::Ignored) {
                    append_text(&mut frame.inlines, &text, style, link.clone());
                }
            }
            Event::Code(text) => {
                append_text(
                    &mut frames.last_mut().unwrap().inlines,
                    &text,
                    Style {
                        code: true,
                        ..style
                    },
                    link.clone(),
                );
            }
            Event::SoftBreak => append_text(
                &mut frames.last_mut().unwrap().inlines,
                " ",
                style,
                link.clone(),
            ),
            Event::HardBreak => frames.last_mut().unwrap().inlines.push(Inline::Break),
            Event::Rule => {
                let frame = frames.last_mut().unwrap();
                frame.flush();
                frame.blocks.push(Block::Rule);
            }
            Event::TaskListMarker(checked) => {
                if let Some(item) = frames
                    .iter_mut()
                    .rev()
                    .find(|frame| matches!(frame.kind, Kind::Item))
                {
                    item.checked = Some(checked);
                }
            }
            Event::Html(_) | Event::InlineHtml(_) => {}
            _ => {}
        }
    }
    let mut root = frames.pop().unwrap();
    root.flush();
    autolinks::apply(&mut root.blocks);
    RichDocument {
        blocks: root.blocks,
    }
}

fn append_text(inlines: &mut Vec<Inline>, text: &str, style: Style, link: Option<Link>) {
    if let Some(Inline::Text(previous)) = inlines.last_mut()
        && previous.style == style
        && previous.link == link
    {
        previous.text.push_str(text);
        return;
    }
    inlines.push(Inline::Text(Span {
        text: text.into(),
        style,
        link,
    }));
}

fn finish(frames: &mut Vec<Frame>) {
    if frames.len() <= 1 {
        return;
    }
    let mut frame = frames.pop().unwrap();
    let parent = frames.last_mut().unwrap();
    let block = match frame.kind {
        Kind::Paragraph => Some(Block::Paragraph(frame.inlines)),
        Kind::Heading(level) => Some(Block::Heading {
            level,
            contents: frame.inlines,
        }),
        Kind::Quote => {
            frame.flush();
            Some(Block::Quote(frame.blocks))
        }
        Kind::List(start) => Some(Block::List {
            start,
            items: frame.items,
        }),
        Kind::Item => {
            frame.flush();
            parent.items.push(ListItem {
                checked: frame.checked,
                blocks: frame.blocks,
            });
            None
        }
        Kind::Code(language) => Some(Block::Code {
            language,
            text: frame.text,
        }),
        Kind::Table(columns) => Some(Block::Table {
            columns,
            header: frame.header,
            rows: frame.rows,
        }),
        Kind::TableHead => {
            parent.header = frame.cells;
            None
        }
        Kind::TableRow => {
            parent.rows.push(frame.cells);
            None
        }
        Kind::TableCell => {
            parent.cells.push(frame.inlines);
            None
        }
        Kind::Image {
            source,
            title,
            link,
            dimensions,
        } => {
            let alt = frame
                .inlines
                .into_iter()
                .map(|inline| match inline {
                    Inline::Text(span) => span.text,
                    Inline::Break => "\n".into(),
                    Inline::Image { alt, .. } => alt,
                })
                .collect::<String>();
            if source.is_empty() {
                append_text(&mut parent.inlines, &alt, Style::default(), link);
            } else {
                parent.inlines.push(Inline::Image {
                    source,
                    title,
                    alt,
                    link,
                    dimensions,
                });
            }
            None
        }
        Kind::Root | Kind::Ignored => None,
    };
    if let Some(block) = block {
        parent.blocks.push(block);
    }
}

fn safe_link(target: &str, title: &str) -> Option<Link> {
    let url = url::Url::parse(target).ok()?;
    if !matches!(url.scheme(), "https" | "http" | "mailto" | "file") {
        return None;
    }
    Some(Link {
        target: url.to_string(),
        title: title.into(),
    })
}

fn safe_image(
    source: &str,
) -> Option<(String, taide_native_editor::documentation::ImageDimensions)> {
    let mut parts = source.split('|').map(str::trim);
    let source = parts.next()?;
    let url = url::Url::parse(source).ok()?;
    if matches!(url.scheme(), "https" | "http" | "file") || source.starts_with("data:image/") {
        let parameters = parts.next().unwrap_or_default();
        let dimension = |name: &str| {
            let tail = parameters.get(parameters.find(name)? + name.len()..)?;
            let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
            tail.get(..digits)?.parse().ok()
        };
        return Some((
            url.to_string(),
            taide_native_editor::documentation::ImageDimensions {
                width: dimension("width="),
                height: dimension("height="),
            },
        ));
    }
    None
}

#[cfg(test)]
#[path = "editor-markup-tests.rs"]
mod tests;
