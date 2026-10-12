use std::fmt::{self, Write};
use std::path::Path;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_native_editor::documentation::RichDocument;

#[path = "editor-markdown-html.rs"]
mod html;

const RESERVE_BYTES: usize = 64 * 1024;

struct Output(String);

impl Write for Output {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let length = self.0.len().checked_add(value.len()).ok_or(fmt::Error)?;
        if length > crate::preview_web_document::MAX_OUTPUT_BYTES {
            return Err(fmt::Error);
        }
        if length > self.0.capacity() {
            let capacity = length
                .saturating_add(RESERVE_BYTES)
                .min(crate::preview_web_document::MAX_OUTPUT_BYTES);
            self.0
                .try_reserve_exact(capacity - self.0.len())
                .map_err(|_| fmt::Error)?;
        }
        self.0.push_str(value);
        Ok(())
    }
}

struct Escaped<'a>(&'a str);

impl fmt::Display for Escaped<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut start = 0;
        for (index, character) in self.0.char_indices() {
            let escaped = match character {
                '&' => "&amp;",
                '<' => "&lt;",
                '>' => "&gt;",
                '"' => "&quot;",
                '\'' => "&#39;",
                _ => continue,
            };
            formatter.write_str(&self.0[start..index])?;
            formatter.write_str(escaped)?;
            start = index + character.len_utf8();
        }
        formatter.write_str(&self.0[start..])
    }
}

struct Image {
    source: String,
    title: String,
    alt: String,
    depth: usize,
}

fn document_html(source: &str) -> Result<String, fmt::Error> {
    let mut output = Output(String::new());
    let mut image: Option<Image> = None;
    let mut header = false;
    let mut alignments = Vec::new();
    let mut column = 0;
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    for event in Parser::new_ext(source, options) {
        if let Some(current) = image.as_mut() {
            match event {
                Event::Text(text) | Event::Code(text) => current.alt.push_str(&text),
                Event::SoftBreak | Event::HardBreak => current.alt.push(' '),
                Event::Start(Tag::Image { .. }) => current.depth += 1,
                Event::End(TagEnd::Image) => {
                    current.depth -= 1;
                    if current.depth == 0 {
                        write!(
                            output,
                            "<img src=\"{}\" title=\"{}\" alt=\"{}\">",
                            Escaped(&current.source),
                            Escaped(&current.title),
                            Escaped(&current.alt)
                        )?;
                        image = None;
                    }
                }
                _ => {}
            }
            continue;
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => output.write_str("<p>")?,
                Tag::Heading { level, .. } => write!(output, "<h{}>", level as u8)?,
                Tag::BlockQuote(_) => output.write_str("<blockquote>")?,
                Tag::CodeBlock(kind) => {
                    let language = match kind {
                        CodeBlockKind::Fenced(language) => language.into_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    write!(
                        output,
                        "<pre><code class=\"language-{}\">",
                        Escaped(language.split_whitespace().next().unwrap_or_default())
                    )?;
                }
                Tag::List(Some(start)) => write!(output, "<ol start=\"{start}\">")?,
                Tag::List(None) => output.write_str("<ul>")?,
                Tag::Item => output.write_str("<li>")?,
                Tag::Emphasis => output.write_str("<em>")?,
                Tag::Strong => output.write_str("<strong>")?,
                Tag::Strikethrough => output.write_str("<del>")?,
                Tag::Link {
                    dest_url, title, ..
                } => write!(
                    output,
                    "<a href=\"{}\" title=\"{}\">",
                    Escaped(&dest_url),
                    Escaped(&title)
                )?,
                Tag::Image {
                    dest_url, title, ..
                } => {
                    image = Some(Image {
                        source: dest_url.into_string(),
                        title: title.into_string(),
                        alt: String::new(),
                        depth: 1,
                    })
                }
                Tag::Table(columns) => {
                    alignments = columns;
                    output.write_str("<table>")?;
                }
                Tag::TableHead => {
                    header = true;
                    column = 0;
                    output.write_str("<thead><tr>")?;
                }
                Tag::TableRow => {
                    column = 0;
                    output.write_str("<tr>")?;
                }
                Tag::TableCell => {
                    let tag = if header { "th" } else { "td" };
                    let alignment = match alignments.get(column) {
                        Some(pulldown_cmark::Alignment::Center) => "center",
                        Some(pulldown_cmark::Alignment::Right) => "right",
                        _ => "left",
                    };
                    write!(output, "<{tag} align=\"{alignment}\">")?;
                    column += 1;
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => output.write_str("</p>")?,
                TagEnd::Heading(level) => write!(output, "</h{}>", level as u8)?,
                TagEnd::BlockQuote(_) => output.write_str("</blockquote>")?,
                TagEnd::CodeBlock => output.write_str("</code></pre>")?,
                TagEnd::List(true) => output.write_str("</ol>")?,
                TagEnd::List(false) => output.write_str("</ul>")?,
                TagEnd::Item => output.write_str("</li>")?,
                TagEnd::Emphasis => output.write_str("</em>")?,
                TagEnd::Strong => output.write_str("</strong>")?,
                TagEnd::Strikethrough => output.write_str("</del>")?,
                TagEnd::Link => output.write_str("</a>")?,
                TagEnd::Table => output.write_str("</table>")?,
                TagEnd::TableHead => {
                    output.write_str("</tr></thead>")?;
                    header = false;
                }
                TagEnd::TableRow => output.write_str("</tr>")?,
                TagEnd::TableCell => output.write_str(if header { "</th>" } else { "</td>" })?,
                _ => {}
            },
            Event::Text(text) => write!(output, "{}", Escaped(&text))?,
            Event::Code(text) => write!(output, "<code>{}</code>", Escaped(&text))?,
            Event::Html(text) | Event::InlineHtml(text) => output.write_str(&text)?,
            Event::SoftBreak => output.write_str("\n")?,
            Event::HardBreak => output.write_str("<br>")?,
            Event::Rule => output.write_str("<hr>")?,
            Event::TaskListMarker(checked) => output.write_str(if checked {
                "<input type=\"checkbox\" checked disabled>"
            } else {
                "<input type=\"checkbox\" disabled>"
            })?,
            _ => {}
        }
    }
    Ok(output.0)
}

pub(crate) fn parse(source: &str, path: Option<&Path>) -> AppResult<RichDocument> {
    let markup = document_html(source).map_err(|_| limit_error())?;
    let base = path.and_then(|path| url::Url::from_file_path(path).ok());
    html::parse(&markup, base.as_ref())
}

fn limit_error() -> AppError {
    AppError::localized(
        AppErrorKind::InvalidArgument,
        "editor.markdownPreviewError",
        "Couldn't display the Markdown preview. Try simplifying the document.",
    )
}

#[cfg(test)]
#[path = "editor-markdown-parse-tests.rs"]
mod tests;
