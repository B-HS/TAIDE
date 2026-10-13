use std::ops::Range;

use crate::document::{Edit, EditorError};
use crate::indent::IndentOptions;
use crate::json_scanner::{self, Kind, Token};

const DEFAULT_TAB_SIZE: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct Options<'a> {
    pub indentation: IndentOptions,
    pub eol: &'a str,
    pub keep_lines: bool,
    pub insert_final_newline: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub scanner: json_scanner::Limits,
    pub max_edits: usize,
    pub max_output_bytes: usize,
    pub max_indent_width: usize,
}

#[derive(Default, Clone, Copy)]
struct Content {
    lines: usize,
    indents: usize,
    spaces: usize,
}

struct Formatter<'a> {
    input: &'a str,
    range: Option<Range<usize>>,
    options: Options<'a>,
    limits: Limits,
    tokens: Vec<Token>,
    cursor: usize,
    origin: usize,
    initial_indent: i64,
    indent: i64,
    breaks: usize,
    has_error: bool,
    eol: &'a str,
    indent_unit: String,
    output_bytes: usize,
    edits: Vec<Edit>,
}

impl Formatter<'_> {
    fn token(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn next(&mut self) -> Kind {
        if self.cursor < self.tokens.len() - 1 {
            self.cursor += 1;
        }
        self.significant()
    }

    fn significant(&mut self) -> Kind {
        self.breaks = 0;
        while matches!(self.token().kind, Kind::Whitespace | Kind::LineBreak) {
            if self.token().kind == Kind::LineBreak {
                self.breaks = if self.options.keep_lines {
                    self.breaks + 1
                } else {
                    1
                };
            }
            self.cursor += 1;
        }
        self.has_error = self.token().kind == Kind::Unknown || self.token().error.is_some();
        self.token().kind
    }

    fn newline(&self) -> Result<Content, EditorError> {
        let indents = self
            .initial_indent
            .checked_add(self.indent)
            .ok_or(EditorError::Capacity)?
            .max(0);
        Ok(Content {
            lines: self.breaks.max(1),
            indents: usize::try_from(indents).map_err(|_| EditorError::Capacity)?,
            spaces: 0,
        })
    }

    fn add(&mut self, content: Content, bytes: Range<usize>) -> Result<(), EditorError> {
        if self.has_error
            || self
                .range
                .as_ref()
                .is_some_and(|range| !(bytes.start < range.end && bytes.end > range.start))
        {
            return Ok(());
        }
        if bytes.start > bytes.end
            || bytes.end > self.input.len()
            || !self.input.is_char_boundary(bytes.start)
            || !self.input.is_char_boundary(bytes.end)
        {
            return Err(EditorError::InvalidBoundary);
        }
        let length = self
            .eol
            .len()
            .checked_mul(content.lines)
            .and_then(|count| {
                self.indent_unit
                    .len()
                    .checked_mul(content.indents)
                    .and_then(|indents| count.checked_add(indents))
            })
            .and_then(|count| count.checked_add(content.spaces))
            .ok_or(EditorError::Capacity)?;
        let parts = || {
            std::iter::repeat_n(self.eol, content.lines)
                .chain(std::iter::repeat_n(
                    self.indent_unit.as_str(),
                    content.indents,
                ))
                .chain(std::iter::repeat_n(" ", content.spaces))
        };
        if bytes.len() == length
            && parts()
                .flat_map(str::bytes)
                .eq(self.input[bytes.clone()].bytes())
        {
            return Ok(());
        }
        if self.edits.len() >= self.limits.max_edits
            || length > self.limits.max_output_bytes - self.output_bytes
        {
            return Err(EditorError::Capacity);
        }
        self.output_bytes += length;
        let mut text = String::with_capacity(length);
        for part in parts() {
            text.push_str(part);
        }
        self.edits.push(Edit { bytes, text });
        Ok(())
    }

    fn run(mut self) -> Result<Vec<Edit>, EditorError> {
        let mut first = self.significant();
        if self.options.keep_lines && self.breaks > 0 {
            self.add(
                Content {
                    lines: self.breaks,
                    ..Content::default()
                },
                0..0,
            )?;
        }
        if first != Kind::End {
            self.add(
                Content {
                    indents: usize::try_from(self.initial_indent)
                        .map_err(|_| EditorError::Capacity)?,
                    ..Content::default()
                },
                self.origin..self.origin + self.token().bytes.start,
            )?;
        }
        while first != Kind::End {
            let mut first_end = self.origin + self.token().bytes.end;
            let mut second = self.next();
            let mut content = Content::default();
            let mut needs_line_break = false;
            while self.breaks == 0 && matches!(second, Kind::LineComment | Kind::BlockComment) {
                self.add(
                    Content {
                        spaces: 1,
                        ..Content::default()
                    },
                    first_end..self.origin + self.token().bytes.start,
                )?;
                first_end = self.origin + self.token().bytes.end;
                needs_line_break = second == Kind::LineComment;
                content = if needs_line_break {
                    self.newline()?
                } else {
                    Content::default()
                };
                second = self.next();
            }
            if matches!(second, Kind::CloseBrace | Kind::CloseBracket) {
                let opening = if second == Kind::CloseBrace {
                    Kind::OpenBrace
                } else {
                    Kind::OpenBracket
                };
                if first != opening {
                    self.indent = self.indent.checked_sub(1).ok_or(EditorError::Capacity)?;
                }
                if self.options.keep_lines && self.breaks > 0
                    || !self.options.keep_lines && first != opening
                {
                    content = self.newline()?;
                } else if self.options.keep_lines {
                    content = Content {
                        spaces: 1,
                        ..Content::default()
                    };
                }
            } else {
                match first {
                    Kind::OpenBrace | Kind::OpenBracket => {
                        self.indent = self.indent.checked_add(1).ok_or(EditorError::Capacity)?;
                        content = if self.options.keep_lines && self.breaks > 0
                            || !self.options.keep_lines
                        {
                            self.newline()?
                        } else {
                            Content {
                                spaces: 1,
                                ..Content::default()
                            }
                        };
                    }
                    Kind::Comma => {
                        content = if self.options.keep_lines && self.breaks > 0
                            || !self.options.keep_lines
                        {
                            self.newline()?
                        } else {
                            Content {
                                spaces: 1,
                                ..Content::default()
                            }
                        };
                    }
                    Kind::LineComment => content = self.newline()?,
                    Kind::BlockComment => {
                        if self.breaks > 0 {
                            content = self.newline()?;
                        } else if !needs_line_break {
                            content = Content {
                                spaces: 1,
                                ..Content::default()
                            };
                        }
                    }
                    Kind::Colon => {
                        if self.options.keep_lines && self.breaks > 0 {
                            content = self.newline()?;
                        } else if !needs_line_break {
                            content = Content {
                                spaces: 1,
                                ..Content::default()
                            };
                        }
                    }
                    Kind::String => {
                        if self.options.keep_lines && self.breaks > 0 {
                            content = self.newline()?;
                        } else if second == Kind::Colon && !needs_line_break {
                            content = Content::default();
                        }
                    }
                    Kind::Null
                    | Kind::True
                    | Kind::False
                    | Kind::Number
                    | Kind::CloseBrace
                    | Kind::CloseBracket => {
                        if self.options.keep_lines && self.breaks > 0 {
                            content = self.newline()?;
                        } else if matches!(second, Kind::LineComment | Kind::BlockComment)
                            && !needs_line_break
                        {
                            content = Content {
                                spaces: 1,
                                ..Content::default()
                            };
                        } else if !matches!(second, Kind::Comma | Kind::End) {
                            self.has_error = true;
                        }
                    }
                    Kind::Unknown => self.has_error = true,
                    _ => {}
                }
                if self.breaks > 0 && matches!(second, Kind::LineComment | Kind::BlockComment) {
                    content = self.newline()?;
                }
            }
            if second == Kind::End {
                content = if self.options.keep_lines && self.breaks > 0 {
                    self.newline()?
                } else {
                    Content {
                        lines: usize::from(self.options.insert_final_newline),
                        ..Content::default()
                    }
                };
            }
            self.add(content, first_end..self.origin + self.token().bytes.start)?;
            first = second;
        }
        Ok(self.edits)
    }
}

pub fn format(
    input: &str,
    range: Option<Range<usize>>,
    options: Options<'_>,
    limits: Limits,
) -> Result<Vec<Edit>, EditorError> {
    if input.len() > limits.scanner.max_bytes {
        return Err(EditorError::Capacity);
    }
    let width = match options.indentation.tab_size {
        0 => DEFAULT_TAB_SIZE,
        width => usize::try_from(width).map_err(|_| EditorError::Capacity)?,
    };
    if width > limits.max_indent_width {
        return Err(EditorError::Capacity);
    }
    let mut origin = 0;
    let mut end = input.len();
    if let Some(range) = &range {
        if range.start > range.end
            || !input.is_char_boundary(range.start)
            || !input.is_char_boundary(range.end)
        {
            return Err(EditorError::InvalidBoundary);
        }
        origin = range.start;
        end = range.end;
        while origin > 0 && !matches!(input.as_bytes()[origin - 1], b'\r' | b'\n') {
            origin -= 1;
        }
        while end < input.len() && !matches!(input.as_bytes()[end], b'\r' | b'\n') {
            end += 1;
        }
    }
    let segment = &input[origin..end];
    let mut indent_columns = 0usize;
    if range.is_some() {
        for character in segment.chars() {
            let amount = match character {
                ' ' => 1,
                '\t' => width,
                _ => break,
            };
            indent_columns = indent_columns
                .checked_add(amount)
                .ok_or(EditorError::Capacity)?;
        }
    }
    let eol = match input
        .as_bytes()
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
    {
        Some(byte) if input[byte..].starts_with("\r\n") => "\r\n",
        Some(byte) if input[byte..].starts_with('\r') => "\r",
        Some(_) => "\n",
        None if options.eol.is_empty() => "\n",
        None => options.eol,
    };
    let tokens = json_scanner::scan(segment, false, limits.scanner)?;
    Formatter {
        input,
        range,
        options,
        limits,
        tokens,
        cursor: 0,
        origin,
        initial_indent: i64::try_from(indent_columns / width).map_err(|_| EditorError::Capacity)?,
        indent: 0,
        breaks: 0,
        has_error: false,
        eol,
        indent_unit: if options.indentation.insert_spaces {
            " ".repeat(width)
        } else {
            "\t".to_owned()
        },
        output_bytes: 0,
        edits: Vec::new(),
    }
    .run()
}
