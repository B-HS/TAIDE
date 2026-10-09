use taide_native_editor::documentation::{Block, Inline};
use taide_native_editor::find::{FindPattern, FindPatternCompiler, FindPatternOptions};

use super::{append_text, safe_link};

const URL_SOURCE: &str = r#"((?:ftp|https?)://|www\.)(?:[a-zA-Z0-9\-]+\.?)+[^\s<]*|[A-Za-z0-9._+-]+(@)[a-zA-Z0-9-_]+(?:\.[a-zA-Z0-9-_]*[a-zA-Z0-9])+(?![-_])"#;
const TRAILING_SOURCE: &str =
    r#"(?:[^?!.,:;*_'"~()&]+|\([^)]*\)|&(?![a-zA-Z0-9]+;$)|[?!.,:;*_'"~)]+(?!$))+"#;
const EMAIL_GROUP: usize = 2;

struct Autolinks {
    url: Box<dyn FindPattern>,
    trailing: Box<dyn FindPattern>,
}

impl Autolinks {
    fn new() -> Self {
        let compiler = taide_native_syntax::MonacoFindPatternCompiler;
        let options = FindPatternOptions {
            is_regex: true,
            match_case: false,
            multiline: false,
        };
        Self {
            url: compiler
                .compile(URL_SOURCE, options)
                .expect("bundled GFM URL expression is valid"),
            trailing: compiler
                .compile(TRAILING_SOURCE, options)
                .expect("bundled GFM punctuation expression is valid"),
        }
    }

    fn inlines(&self, inlines: &mut Vec<Inline>) {
        let mut result = Vec::new();
        for inline in std::mem::take(inlines) {
            let Inline::Text(span) = &inline else {
                result.push(inline);
                continue;
            };
            if span.style.code || span.link.is_some() {
                result.push(inline);
                continue;
            }
            let mut search = 0;
            let mut copied = 0;
            while let Ok(Some(found)) = self.url.captures_at(&span.text, search) {
                let Some(mut range) = found.groups.first().cloned().flatten() else {
                    break;
                };
                if range.is_empty() {
                    break;
                }
                search = range.end;
                let email = found.groups.get(EMAIL_GROUP).is_some_and(Option::is_some);
                if !email {
                    loop {
                        let text = &span.text[range.clone()];
                        let trimmed = self
                            .trailing
                            .captures_at(text, 0)
                            .ok()
                            .flatten()
                            .and_then(|found| found.groups.into_iter().next().flatten());
                        let Some(trimmed) = trimmed.filter(|trimmed| trimmed.start == 0) else {
                            break;
                        };
                        let end = range.start + trimmed.end;
                        if range.end == end {
                            break;
                        }
                        range.end = end;
                    }
                }
                let text = &span.text[range.clone()];
                let target = if email {
                    format!("mailto:{text}")
                } else if text.to_ascii_lowercase().starts_with("www.") {
                    format!("http://{text}")
                } else {
                    text.into()
                };
                let Some(link) = safe_link(&target, "") else {
                    continue;
                };
                if copied < range.start {
                    append_text(
                        &mut result,
                        &span.text[copied..range.start],
                        span.style,
                        None,
                    );
                }
                append_text(&mut result, text, span.style, Some(link));
                copied = range.end;
            }
            if copied < span.text.len() {
                append_text(&mut result, &span.text[copied..], span.style, None);
            }
        }
        *inlines = result;
    }

    fn blocks(&self, blocks: &mut [Block]) {
        for block in blocks {
            match block {
                Block::Paragraph(inlines)
                | Block::Heading {
                    contents: inlines, ..
                } => self.inlines(inlines),
                Block::Quote(blocks) => self.blocks(blocks),
                Block::List { items, .. } => {
                    for item in items {
                        self.blocks(&mut item.blocks);
                    }
                }
                Block::Table { header, rows, .. } => {
                    for cell in header.iter_mut().chain(rows.iter_mut().flatten()) {
                        self.inlines(cell);
                    }
                }
                Block::Code { .. } | Block::Rule => {}
            }
        }
    }
}

thread_local! { static AUTOLINKS: Autolinks = Autolinks::new(); }

pub(super) fn apply(blocks: &mut [Block]) {
    AUTOLINKS.with(|autolinks| autolinks.blocks(blocks));
}
