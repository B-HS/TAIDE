use crate::document::{EditorError, LineEnding};
use crate::indent::IndentOptions;
use crate::snippet_normalization::own_cost;
use crate::snippet_syntax::{MAX_STACK_DEPTH, Marker, ParseLimits};

pub struct WhitespaceContext<'a> {
    pub line: &'a str,
    pub byte_column: usize,
    pub indent: IndentOptions,
    pub line_ending: LineEnding,
    pub adjust_indentation: bool,
}

pub struct AdjustedWhitespace {
    pub markers: Vec<Marker>,
    pub line_leading_whitespace: String,
}

struct Builder {
    limits: ParseLimits,
    allocated_bytes: usize,
    allocated_markers: usize,
    rendered_bytes: usize,
    preceding_byte: Option<u8>,
}

pub fn adjust_whitespace(
    markers: &[Marker],
    context: WhitespaceContext<'_>,
    limits: ParseLimits,
) -> Result<AdjustedWhitespace, EditorError> {
    if limits.max_nesting > MAX_STACK_DEPTH || limits.max_markers == 0 {
        return Err(EditorError::Capacity);
    }
    if context.indent.tab_size == 0 || context.line.contains(['\r', '\n']) {
        return Err(EditorError::InvalidBoundary);
    }
    let before = context
        .line
        .get(..context.byte_column)
        .ok_or(EditorError::InvalidBoundary)?;
    let length = before
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    if length > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    let leading = &before[..length];
    let mut builder = Builder {
        limits,
        allocated_bytes: 0,
        allocated_markers: 0,
        rendered_bytes: 0,
        preceding_byte: None,
    };
    let markers = builder.children(markers, 0, &context, leading)?;
    Ok(AdjustedWhitespace {
        markers,
        line_leading_whitespace: leading.into(),
    })
}

impl Builder {
    fn append_rendered(&mut self, text: &str) -> Result<(), EditorError> {
        self.rendered_bytes = self
            .rendered_bytes
            .checked_add(text.len())
            .ok_or(EditorError::Capacity)?;
        if self.rendered_bytes > self.limits.max_bytes {
            return Err(EditorError::Capacity);
        }
        if let Some(byte) = text.bytes().last() {
            self.preceding_byte = Some(byte);
        }
        Ok(())
    }

    fn charge(&mut self, marker: &Marker) -> Result<(), EditorError> {
        let (bytes, markers) = own_cost(marker)?;
        self.allocated_bytes = self
            .allocated_bytes
            .checked_add(bytes)
            .ok_or(EditorError::Capacity)?;
        self.allocated_markers = self
            .allocated_markers
            .checked_add(markers)
            .ok_or(EditorError::Capacity)?;
        if self.allocated_bytes > self.limits.max_bytes
            || self.allocated_markers > self.limits.max_markers
        {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn children(
        &mut self,
        markers: &[Marker],
        depth: usize,
        context: &WhitespaceContext<'_>,
        leading: &str,
    ) -> Result<Vec<Marker>, EditorError> {
        if depth > self.limits.max_nesting {
            return Err(EditorError::Capacity);
        }
        let mut result = Vec::new();
        for marker in markers {
            let marker = match marker {
                Marker::Text(text) => {
                    if text.len() > self.limits.max_bytes {
                        return Err(EditorError::Capacity);
                    }
                    let mut adjusted = String::new();
                    let mut start = 0;
                    let mut cursor = 0;
                    let mut first = true;
                    loop {
                        while cursor < text.len()
                            && !matches!(text.as_bytes()[cursor], b'\r' | b'\n')
                        {
                            cursor += 1;
                        }
                        let line = &text[start..cursor];
                        let after_break = matches!(self.preceding_byte, Some(b'\r' | b'\n'));
                        let normalize = context.adjust_indentation
                            && (!first || self.rendered_bytes == 0 || after_break);
                        let prefix = if normalize && (!first || self.rendered_bytes > 0) {
                            leading
                        } else {
                            ""
                        };
                        if normalize {
                            let remaining = self.limits.max_bytes - adjusted.len();
                            adjusted.push_str(&normalize_indentation(
                                line,
                                prefix,
                                context.indent,
                                remaining,
                            )?);
                        } else {
                            if adjusted.len().saturating_add(line.len()) > self.limits.max_bytes {
                                return Err(EditorError::Capacity);
                            }
                            adjusted.push_str(line);
                        }
                        if cursor == text.len() {
                            break;
                        }
                        let eol = context.line_ending.as_str();
                        if adjusted.len().saturating_add(eol.len()) > self.limits.max_bytes {
                            return Err(EditorError::Capacity);
                        }
                        adjusted.push_str(eol);
                        let carriage_return = text.as_bytes()[cursor] == b'\r';
                        cursor += 1;
                        if carriage_return && text.as_bytes().get(cursor) == Some(&b'\n') {
                            cursor += 1;
                        }
                        start = cursor;
                        first = false;
                    }
                    self.append_rendered(&adjusted)?;
                    Marker::Text(adjusted)
                }
                Marker::Variable {
                    name,
                    children,
                    transform,
                } => {
                    self.charge(marker)?;
                    result.push(Marker::Variable {
                        name: name.clone(),
                        children: self.children(children, depth + 1, context, leading)?,
                        transform: transform.clone(),
                    });
                    continue;
                }
                Marker::Placeholder {
                    index,
                    children,
                    choices,
                    transform,
                } => {
                    self.charge(marker)?;
                    let children = if let Some(choices) = choices {
                        if !children.is_empty() {
                            return Err(EditorError::InvalidBoundary);
                        }
                        self.append_rendered(choices.first().ok_or(EditorError::InvalidBoundary)?)?;
                        Vec::new()
                    } else {
                        self.children(children, depth + 1, context, leading)?
                    };
                    result.push(Marker::Placeholder {
                        index: *index,
                        children,
                        choices: choices.clone(),
                        transform: transform.clone(),
                    });
                    continue;
                }
            };
            self.charge(&marker)?;
            result.push(marker);
        }
        Ok(result)
    }
}

fn normalize_indentation(
    line: &str,
    prefix: &str,
    indent: IndentOptions,
    max_bytes: usize,
) -> Result<String, EditorError> {
    let spaces = line
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let width = indent.tab_size as usize;
    if width == 0 {
        return Err(EditorError::InvalidBoundary);
    }
    let mut columns = 0usize;
    for byte in prefix.bytes().chain(line[..spaces].bytes()) {
        let increment = if byte == b'\t' {
            width - columns % width
        } else {
            1
        };
        columns = columns
            .checked_add(increment)
            .ok_or(EditorError::Capacity)?;
    }
    let (tabs, spaces_count) = if indent.insert_spaces {
        (0, columns)
    } else {
        (columns / width, columns % width)
    };
    let length = tabs
        .checked_add(spaces_count)
        .and_then(|length| length.checked_add(line.len() - spaces))
        .ok_or(EditorError::Capacity)?;
    if length > max_bytes {
        return Err(EditorError::Capacity);
    }
    let mut result = String::with_capacity(length);
    result.extend(std::iter::repeat_n('\t', tabs));
    result.extend(std::iter::repeat_n(' ', spaces_count));
    result.push_str(&line[spaces..]);
    Ok(result)
}

pub fn normalize_transform(
    text: &str,
    leading: &str,
    indent: IndentOptions,
    line_ending: LineEnding,
    max_bytes: usize,
) -> Result<String, EditorError> {
    if text.len() > max_bytes || leading.len() > max_bytes {
        return Err(EditorError::Capacity);
    }
    if indent.tab_size == 0 || !leading.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
        return Err(EditorError::InvalidBoundary);
    }
    let mut result = String::new();
    let mut start = 0;
    let mut cursor = 0;
    let mut first = true;
    loop {
        while cursor < text.len() && !matches!(text.as_bytes()[cursor], b'\r' | b'\n') {
            cursor += 1;
        }
        let line = &text[start..cursor];
        if first {
            result.push_str(line);
        } else {
            result.push_str(&normalize_indentation(
                line,
                leading,
                indent,
                max_bytes - result.len(),
            )?);
        }
        if cursor == text.len() {
            break;
        }
        let eol = line_ending.as_str();
        if result.len().saturating_add(eol.len()) > max_bytes {
            return Err(EditorError::Capacity);
        }
        result.push_str(eol);
        let carriage_return = text.as_bytes()[cursor] == b'\r';
        cursor += 1;
        if carriage_return && text.as_bytes().get(cursor) == Some(&b'\n') {
            cursor += 1;
        }
        start = cursor;
        first = false;
    }
    Ok(result)
}
