use std::ops::Range;

use crate::css_format;
use crate::document::{Edit, EditorError};
use crate::indent::IndentOptions;
use crate::json_parser::is_ecma_whitespace as is_js_whitespace;

const DEFAULT_TAB_SIZE: usize = 4;
const TAB_INDENT_SIZE_FOR_ONE: usize = 4;
const WRAP_LINE_LENGTH: isize = 120;
const MAX_PRESERVED_NEWLINES: usize = 32786;
const INLINE: &[&str] = &[
    "a", "abbr", "area", "audio", "b", "bdi", "bdo", "br", "button", "canvas", "cite", "code",
    "data", "datalist", "del", "dfn", "em", "embed", "i", "iframe", "img", "input", "ins", "kbd",
    "keygen", "label", "map", "mark", "math", "meter", "noscript", "object", "output", "progress",
    "q", "ruby", "s", "samp", "select", "small", "span", "strong", "sub", "sup", "svg", "template",
    "textarea", "time", "u", "var", "video", "wbr", "text", "acronym", "big", "strike", "tt",
];
const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "keygen", "link", "menuitem",
    "meta", "param", "source", "track", "wbr", "!doctype", "?xml", "basefont", "isindex",
];
const UNFORMATTED: &[&str] = &[
    "abbr", "acronym", "b", "bdo", "big", "br", "button", "cite", "code", "dfn", "em", "i", "img",
    "input", "kbd", "label", "map", "object", "q", "samp", "select", "small", "span", "strong",
    "sub", "sup", "textarea", "tt", "var",
];
const CONTENT_UNFORMATTED: &[&str] = &["pre"];
const EXTRA_LINERS: &[&str] = &["head", "body", "/html"];
const P_CLOSERS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "details",
    "div",
    "dl",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "ul",
];
const P_PARENT_EXCLUDES: &[&str] = &["a", "audio", "del", "ins", "map", "noscript", "video"];
const TYPE_PREFIXES: [&str; 3] = ["text/", "application/", "dojo/"];
const JAVASCRIPT_TYPES: [&str; 8] = [
    "javascript",
    "ecmascript",
    "jscript",
    "livescript",
    "ld+json",
    "json",
    "method",
    "aspect",
];
const EXTENDED_TYPE_PREFIX: &str = "x-";
const COMMENT_START: &str = "<!--";
const COMMENT_END: &str = "-->";
const CDATA_START: &str = "<![CDATA[";
const CDATA_END: &str = "]]>";
const CONDITIONAL_START: &str = "<![";
const CONDITIONAL_END: &str = "]>";
const PROCESSING_START: &str = "<?";
const PROCESSING_END: &str = "?>";
const DIRECTIVE_BLOCK_START: &str = "<!-- beautify";
const DIRECTIVE_BLOCK_END: &str = " -->";
const IGNORE_END_PARTS: [&str; 3] = ["<!--", "beautify", "ignore:end"];

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub indentation: IndentOptions,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_output_bytes: usize,
    pub max_indent_width: usize,
    pub max_tokens: usize,
    pub max_depth: usize,
}

#[derive(Clone, Copy)]
struct Settings {
    indent_size: usize,
    insert_spaces: bool,
    limits: Limits,
}

fn utf16_len(text: &str) -> isize {
    text.encode_utf16().count() as isize
}

fn is_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    TagOpen,
    TagClose,
    Attribute,
    Equals,
    Value,
    Comment,
    Text,
    Unknown,
    Eof,
}

#[derive(Clone)]
struct RawToken {
    kind: Kind,
    text: String,
    newlines: usize,
    whitespace_before: String,
    opened: Option<usize>,
    closed: Option<usize>,
}

struct Tokenizer {
    input: Vec<char>,
    position: usize,
    newlines: usize,
    whitespace_before: String,
}

impl Tokenizer {
    fn peek(&self, offset: usize) -> Option<char> {
        self.input.get(self.position + offset).copied()
    }

    fn matches_at(&self, at: usize, pattern: &str) -> bool {
        let mut index = at;
        for expected in pattern.chars() {
            if self.input.get(index) != Some(&expected) {
                return false;
            }
            index += 1;
        }
        true
    }

    fn find(&self, pattern: &str) -> Option<usize> {
        (self.position..self.input.len()).find(|&index| self.matches_at(index, pattern))
    }

    fn take_to(&mut self, end: usize) -> String {
        let text = self.input[self.position..end].iter().collect();
        self.position = end;
        text
    }

    fn read_until(&mut self, stop: impl Fn(&[char], usize) -> bool) -> String {
        let end = (self.position..self.input.len())
            .find(|&index| stop(&self.input, index))
            .unwrap_or(self.input.len());
        self.take_to(end)
    }

    fn read_until_after(&mut self, pattern: &str) -> String {
        let end = self
            .find(pattern)
            .map_or(self.input.len(), |index| index + pattern.chars().count());
        self.take_to(end)
    }

    fn read_delimited(&mut self, start: &str, end: &str) -> String {
        if !self.matches_at(self.position, start) {
            return String::new();
        }
        let mut text = self.take_to(self.position + start.chars().count());
        text.push_str(&self.read_until_after(end));
        text
    }

    fn read_whitespace(&mut self) {
        let text =
            self.read_until(|input, index| !matches!(input[index], '\t' | ' ' | '\n' | '\r'));
        self.newlines = text.matches('\n').count();
        self.whitespace_before = text.rsplit('\n').next().unwrap_or_default().to_owned();
    }

    fn token(&self, kind: Kind, text: String) -> RawToken {
        RawToken {
            kind,
            text,
            newlines: self.newlines,
            whitespace_before: self.whitespace_before.clone(),
            opened: None,
            closed: None,
        }
    }

    fn ignored_until_end(&mut self) -> String {
        let mut index = self.position;
        while index < self.input.len() {
            if let Some(end) = self.ignore_end_at(index) {
                return self.take_to(end);
            }
            index += 1;
        }
        self.take_to(self.input.len())
    }

    fn ignore_end_at(&self, start: usize) -> Option<usize> {
        let mut index = start;
        for part in IGNORE_END_PARTS {
            if !self.matches_at(index, part)
                || !self
                    .input
                    .get(index + part.chars().count())
                    .copied()
                    .is_some_and(is_js_whitespace)
            {
                return None;
            }
            index += part.chars().count() + 1;
        }
        self.matches_at(index, COMMENT_END)
            .then_some(index + COMMENT_END.len())
    }

    fn comment_or_cdata(&mut self) -> Option<RawToken> {
        if self.peek(0) != Some('<') || self.peek(1) != Some('!') {
            return None;
        }
        let mut text = self.read_delimited(COMMENT_START, COMMENT_END);
        if !text.is_empty() {
            if directives_ignore_start(&text) {
                text.push_str(&self.ignored_until_end());
            }
        } else {
            text = self.read_delimited(CDATA_START, CDATA_END);
        }
        (!text.is_empty()).then(|| self.token(Kind::Comment, text))
    }

    fn read_until_end_tag(&mut self, tag: &str) -> String {
        let tag = tag.chars().collect::<Vec<_>>();
        let is_end = |input: &[char], index: usize| {
            if input.get(index) != Some(&'<') || input.get(index + 1) != Some(&'/') {
                return false;
            }
            let mut cursor = index + 2;
            for expected in &tag {
                if !input
                    .get(cursor)
                    .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
                {
                    return false;
                }
                cursor += 1;
            }
            while input
                .get(cursor)
                .is_some_and(|character| matches!(character, '\n' | '\r' | '\t' | ' '))
            {
                cursor += 1;
            }
            input.get(cursor) == Some(&'>')
        };
        self.read_until(is_end)
    }

    fn next_token(&mut self, tokens: &[RawToken], open: Option<usize>) -> RawToken {
        self.read_whitespace();
        let Some(character) = self.peek(0) else {
            return self.token(Kind::Eof, String::new());
        };
        let previous = tokens.last();
        let open_angle = open.is_some_and(|open| tokens[open].text.starts_with('<'));
        if open_angle {
            if character == '=' {
                self.position += 1;
                return self.token(Kind::Equals, "=".to_owned());
            }
            if character == '"' || character == '\'' {
                self.position += 1;
                let mut text = character.to_string();
                text.push_str(&self.read_until_after(&character.to_string()));
                return self.token(Kind::Value, text);
            }
            let text = self.read_until(|input, index| {
                matches!(input[index], '\n' | '\r' | '\t' | ' ' | '=' | '>')
                    || input[index] == '/' && input.get(index + 1) == Some(&'>')
            });
            if !text.is_empty() {
                let kind = if previous.is_some_and(|token| token.kind == Kind::Equals) {
                    Kind::Value
                } else {
                    Kind::Attribute
                };
                return self.token(kind, text);
            }
            if character == '>' || character == '/' && self.peek(1) == Some('>') {
                let length = if character == '/' { 2 } else { 1 };
                let text = self.take_to(self.position + length);
                return self.token(Kind::TagClose, text);
            }
        }
        if let Some(previous) = previous
            && previous.kind == Kind::TagClose
            && !previous.text.starts_with('/')
            && let Some(opened) = previous.opened
            && let Some(name) = tokens[opened].text.strip_prefix('<')
        {
            let name = name.to_lowercase();
            let mut text = String::new();
            if name == "script" || name == "style" {
                if let Some(mut token) = self.comment_or_cdata() {
                    token.kind = Kind::Text;
                    return token;
                }
                text = self.read_until_end_tag(&name);
            } else if !VOID.contains(&name.as_str())
                && (CONTENT_UNFORMATTED.contains(&name.as_str())
                    || UNFORMATTED.contains(&name.as_str()))
            {
                text = self.read_until_end_tag(&name);
            }
            if !text.is_empty() {
                return self.token(Kind::Text, text);
            }
        }
        let word =
            self.read_until(|input, index| matches!(input[index], '\n' | '\r' | '\t' | ' ' | '<'));
        if !word.is_empty() {
            return self.token(Kind::Text, word);
        }
        if let Some(token) = self.comment_or_cdata() {
            return token;
        }
        if character == '<' && matches!(self.peek(1), Some('!' | '?')) {
            let mut text = self.read_delimited(CONDITIONAL_START, CONDITIONAL_END);
            if text.is_empty() {
                text = self.read_delimited(PROCESSING_START, PROCESSING_END);
            }
            if !text.is_empty() {
                return self.token(Kind::Comment, text);
            }
        }
        if open.is_none() && character == '<' {
            let mut text = self.take_to(self.position + 1);
            if self.peek(0) == Some('/') {
                text.push_str(&self.take_to(self.position + 1));
            }
            text.push_str(&self.read_until(|input, index| {
                matches!(input[index], '\n' | '\r' | '\t' | ' ' | '>' | '/')
            }));
            return self.token(Kind::TagOpen, text);
        }
        let text = self.take_to(self.position + 1);
        self.token(Kind::Unknown, text)
    }

    fn tokenize(mut self, max_tokens: usize) -> Result<Vec<RawToken>, EditorError> {
        let mut tokens = Vec::<RawToken>::new();
        let mut open = None::<usize>;
        loop {
            if tokens.len() >= max_tokens {
                return Err(EditorError::Capacity);
            }
            let mut current = self.next_token(&tokens, open);
            let index = tokens.len();
            if current.kind == Kind::TagOpen {
                open = Some(index);
            } else if let Some(opened) = open
                && current.kind == Kind::TagClose
                && (current.text == ">" || current.text == "/>")
                && tokens[opened].text.starts_with('<')
            {
                current.opened = Some(opened);
                tokens[opened].closed = Some(index);
                open = None;
            }
            let end = current.kind == Kind::Eof;
            tokens.push(current);
            if end {
                return Ok(tokens);
            }
        }
    }
}

fn directives_ignore_start(text: &str) -> bool {
    let characters = text.chars().collect::<Vec<_>>();
    let read_word = |start: usize| {
        let mut end = start;
        while characters.get(end).copied().is_some_and(is_word) {
            end += 1;
        }
        (end > start).then_some(end)
    };
    let read_pair = |start: usize| {
        if characters.get(start) != Some(&' ') {
            return None;
        }
        let name_end = read_word(start + 1)?;
        if characters.get(name_end) != Some(&':') {
            return None;
        }
        let value_end = read_word(name_end + 1)?;
        Some((start + 1..name_end, name_end + 1..value_end))
    };
    let block_start = DIRECTIVE_BLOCK_START.chars().collect::<Vec<_>>();
    let block_end = DIRECTIVE_BLOCK_END.chars().collect::<Vec<_>>();
    let has_block = (0..characters.len()).any(|start| {
        if !characters[start..].starts_with(&block_start) {
            return false;
        }
        let mut cursor = start + block_start.len();
        let mut pairs = 0usize;
        while let Some((_, value)) = read_pair(cursor) {
            cursor = value.end;
            pairs += 1;
        }
        pairs > 0 && characters[cursor..].starts_with(&block_end)
    });
    if !has_block {
        return false;
    }
    let mut ignore = None::<String>;
    let mut index = 0usize;
    while index < characters.len() {
        match read_pair(index) {
            Some((name, value)) => {
                if characters[name].iter().collect::<String>() == "ignore" {
                    ignore = Some(characters[value.clone()].iter().collect());
                }
                index = value.end;
            }
            None => index += 1,
        }
    }
    ignore.as_deref() == Some("start")
}

#[derive(Clone, Copy)]
struct Metrics {
    base_length: isize,
    indent_size: isize,
}

impl Metrics {
    fn size(self, indent: isize, column: isize) -> isize {
        let base = if indent < 0 { 0 } else { self.base_length };
        base + indent * self.indent_size + column
    }
}

#[derive(Clone)]
struct Line {
    items: Vec<String>,
    character_count: isize,
    indent_count: isize,
    alignment_count: isize,
    wrap_point_index: usize,
    wrap_point_character_count: isize,
    wrap_point_indent_count: isize,
    wrap_point_alignment_count: isize,
}

impl Line {
    fn empty() -> Self {
        Self {
            items: Vec::new(),
            character_count: 0,
            indent_count: -1,
            alignment_count: 0,
            wrap_point_index: 0,
            wrap_point_character_count: 0,
            wrap_point_indent_count: -1,
            wrap_point_alignment_count: 0,
        }
    }

    fn set_indent(&mut self, indent: isize, alignment: isize, metrics: Metrics) {
        if self.items.is_empty() {
            self.indent_count = indent;
            self.alignment_count = alignment;
            self.character_count = metrics.size(indent, alignment);
        }
    }

    fn push(&mut self, item: String) {
        let length = utf16_len(&item);
        match item.rfind('\n') {
            Some(index) => self.character_count = length - utf16_len(&item[..index]),
            None => self.character_count += length,
        }
        self.items.push(item);
    }

    fn pop(&mut self) -> Option<String> {
        let item = self.items.pop()?;
        self.character_count -= utf16_len(&item);
        Some(item)
    }

    fn trim(&mut self) {
        while self.items.last().is_some_and(|item| item == " ") {
            self.items.pop();
            self.character_count -= 1;
        }
    }
}

struct Output {
    lines: Vec<Line>,
    next_line: Line,
    metrics: Metrics,
    base: String,
    unit: String,
    space_before_token: bool,
    non_breaking_space: bool,
    previous_token_wrapped: bool,
    bytes: usize,
    max_bytes: usize,
}

impl Output {
    fn new(base: String, settings: Settings) -> Self {
        let unit = if settings.insert_spaces {
            " ".repeat(settings.indent_size)
        } else {
            "\t".to_owned()
        };
        let mut first = Line::empty();
        let metrics = Metrics {
            base_length: utf16_len(&base),
            indent_size: settings.indent_size as isize,
        };
        first.set_indent(-1, 0, metrics);
        Self {
            lines: vec![first],
            next_line: Line::empty(),
            metrics,
            base,
            unit,
            space_before_token: false,
            non_breaking_space: false,
            previous_token_wrapped: false,
            bytes: 0,
            max_bytes: settings.limits.max_output_bytes,
        }
    }

    fn charge(&mut self, bytes: usize) -> Result<(), EditorError> {
        self.bytes = self.bytes.checked_add(bytes).ok_or(EditorError::Capacity)?;
        if self.bytes > self.max_bytes {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn current(&mut self) -> &mut Line {
        self.lines.last_mut().expect("output keeps a current line")
    }

    fn push(&mut self, item: String) -> Result<(), EditorError> {
        self.charge(item.len())?;
        self.current().push(item);
        Ok(())
    }

    fn add_output_line(&mut self) -> Result<(), EditorError> {
        self.charge(1)?;
        let mut line = Line::empty();
        line.set_indent(
            self.next_line.indent_count,
            self.next_line.alignment_count,
            self.metrics,
        );
        self.lines.push(line);
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].items.is_empty()
    }

    fn just_added_newline(&self) -> bool {
        self.lines.last().is_some_and(|line| line.items.is_empty())
    }

    fn just_added_blankline(&self) -> bool {
        self.is_empty()
            || self.just_added_newline()
                && self.lines.len() > 1
                && self.lines[self.lines.len() - 2].items.is_empty()
    }

    fn add_new_line(&mut self, force: bool) -> Result<bool, EditorError> {
        if self.is_empty() || !force && self.just_added_newline() {
            return Ok(false);
        }
        self.add_output_line()?;
        Ok(true)
    }

    fn set_indent(&mut self, indent: isize, alignment: isize) {
        let metrics = self.metrics;
        self.next_line.set_indent(indent, alignment, metrics);
        if self.lines.len() > 1 {
            self.current().set_indent(indent, alignment, metrics);
        } else {
            self.current().set_indent(0, 0, metrics);
        }
    }

    fn set_wrap_point(&mut self) {
        let (indent, alignment) = (self.next_line.indent_count, self.next_line.alignment_count);
        let line = self.current();
        line.wrap_point_index = line.items.len();
        line.wrap_point_character_count = line.character_count;
        line.wrap_point_indent_count = indent;
        line.wrap_point_alignment_count = alignment;
    }

    fn add_raw_token(
        &mut self,
        text: &str,
        whitespace_before: &str,
        newlines: usize,
    ) -> Result<(), EditorError> {
        for _ in 0..newlines {
            self.add_output_line()?;
        }
        let metrics = self.metrics;
        self.current().set_indent(-1, 0, metrics);
        self.push(whitespace_before.to_owned())?;
        self.push(text.to_owned())?;
        self.space_before_token = false;
        self.non_breaking_space = false;
        self.previous_token_wrapped = false;
        Ok(())
    }

    fn add_token(&mut self, text: &str) -> Result<(), EditorError> {
        if self.space_before_token && !self.just_added_newline() {
            if !self.non_breaking_space {
                self.set_wrap_point();
            }
            self.push(" ".to_owned())?;
        }
        self.push(text.to_owned())?;
        self.space_before_token = false;
        self.non_breaking_space = false;
        self.previous_token_wrapped = self.allow_wrap()?;
        Ok(())
    }

    fn allow_wrap(&mut self) -> Result<bool, EditorError> {
        let next_count = self.next_line.character_count;
        let line = self.lines.last().expect("output keeps a current line");
        if line.wrap_point_index == 0
            || line.character_count <= WRAP_LINE_LENGTH
            || line.wrap_point_character_count <= next_count
        {
            return Ok(false);
        }
        let previous = self.lines.len() - 1;
        self.add_output_line()?;
        let metrics = self.metrics;
        let (moved, indent, alignment, count, wrap_count) = {
            let line = &mut self.lines[previous];
            let moved = line.items.split_off(line.wrap_point_index);
            let values = (
                line.wrap_point_indent_count,
                line.wrap_point_alignment_count,
                line.character_count,
                line.wrap_point_character_count,
            );
            line.character_count = line.wrap_point_character_count;
            (moved, values.0, values.1, values.2, values.3)
        };
        let next = self.current();
        next.set_indent(indent, alignment, metrics);
        next.items = moved;
        next.character_count += count - wrap_count;
        if next.items.first().is_some_and(|item| item == " ") {
            next.items.remove(0);
            next.character_count -= 1;
        }
        Ok(true)
    }

    fn indent_string(&self, indent: isize, column: isize) -> String {
        let (mut result, indent) = if indent < 0 {
            (String::new(), 0)
        } else {
            (self.base.clone(), indent)
        };
        let mut column = column + indent * self.metrics.indent_size;
        let size = self.metrics.indent_size;
        if size > 0 && column >= size {
            let count = column / size;
            column -= count * size;
            result.push_str(&self.unit.repeat(count as usize));
        }
        if column > 0 {
            result.push_str(&" ".repeat(column as usize));
        }
        result
    }

    fn trim(&mut self, eat_newlines: bool) {
        self.current().trim();
        while eat_newlines && self.lines.len() > 1 && self.just_added_newline() {
            self.lines.pop();
            self.current().trim();
        }
    }

    fn code(mut self) -> Result<String, EditorError> {
        self.trim(true);
        if let Some(last) = self.current().pop()
            && !last.is_empty()
        {
            let last = last.trim_end_matches('\n').to_owned();
            self.current().push(last);
        }
        let mut code = String::new();
        for (index, line) in self.lines.iter().enumerate() {
            if index > 0 {
                code.push('\n');
            }
            if !line.items.is_empty() {
                code.push_str(&self.indent_string(line.indent_count, line.alignment_count));
                for item in &line.items {
                    code.push_str(item);
                }
            }
            if code.len() > self.max_bytes {
                return Err(EditorError::Capacity);
            }
        }
        Ok(code)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Custom {
    Css,
    Javascript,
    Html,
    Null,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Last {
    Start,
    TagOpen,
    Inside,
    TagClose,
    Content,
}

#[derive(Clone, Default)]
struct TagToken {
    parent: Option<usize>,
    text: String,
    tag_name: String,
    tag_check: String,
    starts_with_angle: bool,
    is_inline_element: bool,
    is_unformatted: bool,
    is_content_unformatted: bool,
    is_empty_element: bool,
    is_start_tag: bool,
    is_end_tag: bool,
    indent_content: bool,
    multiline_content: bool,
    custom: Option<Custom>,
    start_tag_token: Option<usize>,
    has_wrapped_attrs: bool,
    alignment_size: isize,
    tag_complete: bool,
}

struct Frame {
    tag: String,
    indent_level: isize,
    token: usize,
}

struct Beautifier {
    tokens: Vec<RawToken>,
    tags: Vec<TagToken>,
    frames: Vec<Frame>,
    output: Output,
    indent_level: isize,
    alignment_size: isize,
    settings: Settings,
    depth: usize,
}

fn contains_type(value: &str, kinds: &[&str]) -> bool {
    value.match_indices('/').any(|(slash, _)| {
        let subtype = &value[slash + 1..];
        let extended = subtype.strip_prefix(EXTENDED_TYPE_PREFIX);
        TYPE_PREFIXES
            .iter()
            .any(|prefix| value[..=slash].ends_with(prefix))
            && kinds.iter().any(|kind| {
                subtype.starts_with(kind) || extended.is_some_and(|rest| rest.starts_with(kind))
            })
    })
}

fn strip_trailing_blank_line(text: &str) -> &str {
    match text.rfind('\n') {
        Some(index)
            if text[index + 1..]
                .chars()
                .all(|character| matches!(character, ' ' | '\t')) =>
        {
            &text[..index]
        }
        _ => text,
    }
}

struct Wrapped<'a> {
    open: &'a str,
    has_newline: bool,
    space: &'a str,
    body: &'a str,
    close: &'a str,
}

fn wrapped_content(text: &str) -> Option<Wrapped<'_>> {
    let close = [COMMENT_END, CDATA_END]
        .into_iter()
        .find(|close| text.ends_with(close))?;
    let limit = text.len() - close.len();
    let open_end = if text.starts_with(COMMENT_START) {
        let end = text.find('\n').unwrap_or(text.len()).min(limit);
        (end >= COMMENT_START.len()).then_some(end)?
    } else if text.starts_with(CDATA_START) && limit >= CDATA_START.len() {
        CDATA_START.len()
    } else {
        return None;
    };
    let has_newline = text[open_end..limit].starts_with('\n');
    let space_start = open_end + usize::from(has_newline);
    let space_end = text[space_start..limit]
        .find(|character| !matches!(character, ' ' | '\t' | '\n'))
        .map_or(limit, |offset| space_start + offset);
    Some(Wrapped {
        open: &text[..open_end],
        has_newline,
        space: &text[space_start..space_end],
        body: &text[space_end..limit],
        close,
    })
}

impl Beautifier {
    fn print_newline(&mut self, force: bool) -> Result<(), EditorError> {
        self.output.add_new_line(force).map(drop)
    }

    fn set_space_before_token(&mut self, value: bool, non_breaking: bool) {
        self.output.space_before_token = value;
        self.output.non_breaking_space = non_breaking;
    }

    fn set_wrap_point(&mut self) {
        self.output
            .set_indent(self.indent_level, self.alignment_size);
        self.output.set_wrap_point();
    }

    fn print_token(&mut self, text: &str) -> Result<(), EditorError> {
        if text.is_empty() {
            return Ok(());
        }
        self.output
            .set_indent(self.indent_level, self.alignment_size);
        self.output.add_token(text)
    }

    fn add_raw(&mut self, index: usize) -> Result<(), EditorError> {
        let token = &self.tokens[index];
        let (text, whitespace, newlines) = (
            token.text.clone(),
            token.whitespace_before.clone(),
            token.newlines,
        );
        self.output.add_raw_token(&text, &whitespace, newlines)
    }

    fn print_preserved_newlines(&mut self, index: usize) -> Result<bool, EditorError> {
        let newlines = self.tokens[index].newlines.min(MAX_PRESERVED_NEWLINES + 1);
        for count in 0..newlines {
            self.print_newline(count > 0)?;
        }
        Ok(newlines != 0)
    }

    fn traverse_whitespace(&mut self, index: usize) -> Result<(), EditorError> {
        let token = &self.tokens[index];
        if (!token.whitespace_before.is_empty() || token.newlines > 0)
            && !self.print_preserved_newlines(index)?
        {
            self.output.space_before_token = true;
        }
        Ok(())
    }

    fn full_indent(&self, level: isize) -> String {
        let level = self.indent_level + level;
        if level < 1 {
            return String::new();
        }
        self.output.indent_string(level, 0)
    }

    fn try_pop(&mut self, tag: &str, stop: &[&str]) -> Option<usize> {
        for index in (0..self.frames.len()).rev() {
            if self.frames[index].tag == tag {
                self.indent_level = self.frames[index].indent_level;
                let token = self.frames[index].token;
                self.frames.truncate(index);
                return Some(token);
            }
            if stop.contains(&self.frames[index].tag.as_str()) {
                return None;
            }
        }
        None
    }

    fn tag_open_token(&self, index: usize) -> TagToken {
        let raw = &self.tokens[index];
        let starts_with_angle = raw.text.starts_with('<');
        let tag_check = if starts_with_angle {
            raw.text[1..]
                .chars()
                .take_while(|&character| !is_js_whitespace(character) && character != '>')
                .collect::<String>()
                .to_lowercase()
        } else {
            String::new()
        };
        let tag_complete = raw.kind == Kind::Comment;
        let is_start_tag = !tag_check.starts_with('/');
        let tag_name = if is_start_tag {
            tag_check.clone()
        } else {
            tag_check[1..].to_owned()
        };
        let is_end_tag = !is_start_tag
            || raw
                .closed
                .is_some_and(|closed| self.tokens[closed].text == "/>")
            || VOID.contains(&tag_check.as_str());
        let is_empty_element = tag_complete || is_start_tag && is_end_tag;
        TagToken {
            parent: self.frames.last().map(|frame| frame.token),
            text: raw.text.clone(),
            is_inline_element: INLINE.contains(&tag_name.as_str()) || tag_name.contains('-'),
            is_unformatted: !tag_complete && UNFORMATTED.contains(&tag_check.as_str()),
            is_content_unformatted: !is_empty_element
                && CONTENT_UNFORMATTED.contains(&tag_check.as_str()),
            tag_name,
            tag_check,
            starts_with_angle,
            is_empty_element,
            is_start_tag,
            is_end_tag,
            alignment_size: self.settings.indent_size as isize,
            tag_complete,
            ..TagToken::default()
        }
    }

    fn custom_beautifier(&self, index: usize) -> Option<Custom> {
        let raw = &self.tokens[index];
        let closed = raw.closed?;
        let mut kind = if self.tags.last()?.tag_check == "script" {
            "text/javascript".to_owned()
        } else {
            "text/css".to_owned()
        };
        let mut cursor = index + 1;
        while cursor < self.tokens.len()
            && self.tokens[cursor].kind != Kind::Eof
            && cursor != closed
        {
            let token = &self.tokens[cursor];
            if token.kind == Kind::Attribute && token.text == "type" {
                if self
                    .tokens
                    .get(cursor + 1)
                    .is_some_and(|token| token.kind == Kind::Equals)
                    && let Some(value) = self
                        .tokens
                        .get(cursor + 2)
                        .filter(|token| token.kind == Kind::Value)
                {
                    kind = value.text.clone();
                }
                break;
            }
            cursor += 1;
        }
        if kind.contains("text/css") {
            return Some(Custom::Css);
        }
        if kind.contains("module") || contains_type(&kind, &JAVASCRIPT_TYPES) {
            return Some(Custom::Javascript);
        }
        if contains_type(&kind, &["html"]) {
            return Some(Custom::Html);
        }
        kind.contains("test/null").then_some(Custom::Null)
    }

    fn do_optional_end_element(&mut self, token: usize) -> bool {
        let current = &self.tags[token];
        let Some(parent) = current
            .parent
            .filter(|_| !current.is_empty_element && current.is_start_tag)
        else {
            return false;
        };
        let name = current.tag_name.clone();
        let table = ["table"];
        let result = match name.as_str() {
            "body" => self.try_pop("head", &[]),
            "li" => self.try_pop("li", &["ol", "ul", "menu"]),
            "dd" | "dt" => self
                .try_pop("dt", &["dl"])
                .or_else(|| self.try_pop("dd", &["dl"])),
            _ if self.tags[parent].tag_name == "p" && P_CLOSERS.contains(&name.as_str()) => {
                let excluded = self.tags[parent].parent.is_some_and(|ancestor| {
                    P_PARENT_EXCLUDES.contains(&self.tags[ancestor].tag_name.as_str())
                });
                if excluded {
                    None
                } else {
                    self.try_pop("p", &[])
                }
            }
            "rp" | "rt" => self
                .try_pop("rt", &["ruby", "rtc"])
                .or_else(|| self.try_pop("rp", &["ruby", "rtc"])),
            "optgroup" => self.try_pop("optgroup", &["select"]),
            "option" => self.try_pop("option", &["select", "datalist", "optgroup"]),
            "colgroup" => self.try_pop("caption", &table),
            "thead" => self
                .try_pop("caption", &table)
                .or_else(|| self.try_pop("colgroup", &table)),
            "tbody" | "tfoot" => self
                .try_pop("caption", &table)
                .or_else(|| self.try_pop("colgroup", &table))
                .or_else(|| self.try_pop("thead", &table))
                .or_else(|| self.try_pop("tbody", &table)),
            "tr" => self
                .try_pop("caption", &table)
                .or_else(|| self.try_pop("colgroup", &table))
                .or_else(|| self.try_pop("tr", &["table", "thead", "tbody", "tfoot"])),
            "th" | "td" => self
                .try_pop("td", &["table", "thead", "tbody", "tfoot", "tr"])
                .or_else(|| self.try_pop("th", &["table", "thead", "tbody", "tfoot", "tr"])),
            _ => None,
        };
        self.tags[token].parent = self.frames.last().map(|frame| frame.token);
        result.is_some()
    }

    fn calculate_parent_multiline(&mut self, token: usize) {
        let current = &self.tags[token];
        if let Some(parent) = current.parent
            && self.output.just_added_newline()
            && !((current.is_inline_element || current.is_unformatted)
                && self.tags[parent].is_inline_element)
        {
            self.tags[parent].multiline_content = true;
        }
    }

    fn set_tag_position(
        &mut self,
        raw: usize,
        token: usize,
        last_tag: usize,
        last: Last,
    ) -> Result<(), EditorError> {
        if !self.tags[token].is_empty_element {
            if self.tags[token].is_end_tag {
                let name = self.tags[token].tag_name.clone();
                self.tags[token].start_tag_token = self.try_pop(&name, &[]);
            } else {
                if self.do_optional_end_element(token) && !self.tags[token].is_inline_element {
                    self.print_newline(false)?;
                }
                self.frames.push(Frame {
                    tag: self.tags[token].tag_name.clone(),
                    indent_level: self.indent_level,
                    token,
                });
                let current = &self.tags[token];
                if (current.tag_name == "script" || current.tag_name == "style")
                    && !(current.is_unformatted || current.is_content_unformatted)
                {
                    self.tags[token].custom = self.custom_beautifier(raw);
                }
            }
        }
        if EXTRA_LINERS.contains(&self.tags[token].tag_check.as_str()) {
            self.print_newline(false)?;
            if !self.output.just_added_blankline() {
                self.print_newline(true)?;
            }
        }
        let current = self.tags[token].clone();
        if current.is_empty_element {
            let keeps_line = current.tag_name == "!--"
                && last == Last::TagClose
                && self.tags[last_tag].is_end_tag
                && !current.text.contains('\n');
            if !keeps_line {
                if !(current.is_inline_element || current.is_unformatted) {
                    self.print_newline(false)?;
                }
                self.calculate_parent_multiline(token);
            }
        } else if current.is_end_tag {
            let previous = &self.tags[last_tag];
            let expand = current
                .start_tag_token
                .is_some_and(|start| self.tags[start].multiline_content)
                || !current.is_inline_element
                    && !(previous.is_inline_element || previous.is_unformatted)
                    && !(last == Last::TagClose && current.start_tag_token == Some(last_tag))
                    && last != Last::Content;
            if expand && !(current.is_content_unformatted || current.is_unformatted) {
                self.print_newline(false)?;
            }
        } else {
            let mut indent_content = current.custom.is_none();
            if current.starts_with_angle {
                match current.tag_name.as_str() {
                    "html" => indent_content = false,
                    "head" | "body" => indent_content = true,
                    _ => {}
                }
            }
            self.tags[token].indent_content = indent_content;
            if !(current.is_inline_element || current.is_unformatted)
                && (last != Last::Content || current.is_content_unformatted)
            {
                self.print_newline(false)?;
            }
            self.calculate_parent_multiline(token);
        }
        Ok(())
    }

    fn handle_tag_open(
        &mut self,
        raw: usize,
        last_tag: usize,
        last: Last,
    ) -> Result<usize, EditorError> {
        let token = self.tags.len();
        let parsed = self.tag_open_token(raw);
        self.tags.push(parsed);
        let previous = &self.tags[last_tag];
        if (previous.is_unformatted || previous.is_content_unformatted)
            && !previous.is_empty_element
            && self.tokens[raw].kind == Kind::TagOpen
            && !self.tags[token].is_start_tag
        {
            self.add_raw(raw)?;
            let name = self.tags[token].tag_name.clone();
            self.tags[token].start_tag_token = self.try_pop(&name, &[]);
        } else {
            self.traverse_whitespace(raw)?;
            self.set_tag_position(raw, token, last_tag, last)?;
            if !self.tags[token].is_inline_element {
                self.set_wrap_point();
            }
            let text = self.tokens[raw].text.clone();
            self.print_token(&text)?;
        }
        if !self.tags[token].tag_complete && !self.tags[token].is_unformatted {
            self.alignment_size = self.tags[token].alignment_size;
        }
        Ok(token)
    }

    fn handle_inside_tag(&mut self, raw: usize, last_tag: usize) -> Result<(), EditorError> {
        let (spaced, kind) = {
            let token = &self.tokens[raw];
            (
                token.newlines > 0 || !token.whitespace_before.is_empty(),
                token.kind,
            )
        };
        self.set_space_before_token(spaced, true);
        if self.tags[last_tag].is_unformatted {
            return self.add_raw(raw);
        }
        match kind {
            Kind::Attribute => self.set_space_before_token(true, false),
            Kind::Equals => self.set_space_before_token(false, false),
            Kind::Value if raw > 0 && self.tokens[raw - 1].kind == Kind::Equals => {
                self.set_space_before_token(false, false)
            }
            _ => {}
        }
        let text = self.tokens[raw].text.clone();
        self.print_token(&text)?;
        let wrapped = self.tags[last_tag].has_wrapped_attrs || self.output.previous_token_wrapped;
        self.tags[last_tag].has_wrapped_attrs = wrapped;
        Ok(())
    }

    fn handle_tag_close(&mut self, raw: usize, last_tag: usize) -> Result<(), EditorError> {
        self.alignment_size = 0;
        self.tags[last_tag].tag_complete = true;
        let token = &self.tokens[raw];
        self.set_space_before_token(
            token.newlines > 0 || !token.whitespace_before.is_empty(),
            true,
        );
        let current = self.tags[last_tag].clone();
        if current.is_unformatted {
            self.add_raw(raw)?;
        } else {
            if current.starts_with_angle {
                let self_closing = self.tokens[raw].text.starts_with('/');
                self.set_space_before_token(self_closing, true);
            }
            let text = self.tokens[raw].text.clone();
            self.print_token(&text)?;
        }
        let keeps_content = current.is_unformatted || current.is_content_unformatted;
        if current.indent_content && !keeps_content {
            self.indent_level += 1;
            self.tags[last_tag].indent_content = false;
        }
        if !current.is_inline_element && !keeps_content {
            self.set_wrap_point();
        }
        Ok(())
    }

    fn handle_text(&mut self, raw: usize, last_tag: usize) -> Result<(), EditorError> {
        let current = &self.tags[last_tag];
        if let Some(custom) = current.custom {
            return self.print_custom(raw, custom);
        }
        if current.is_unformatted || current.is_content_unformatted {
            return self.add_raw(raw);
        }
        self.traverse_whitespace(raw)?;
        let text = self.tokens[raw].text.clone();
        self.print_token(&text)
    }

    fn print_custom(&mut self, raw: usize, custom: Custom) -> Result<(), EditorError> {
        let token = self.tokens[raw].clone();
        if token.text.is_empty() {
            return Ok(());
        }
        let indentation = self.full_indent(1);
        let mut text = strip_trailing_blank_line(&token.text).to_owned();
        let mut whitespace_before = token.whitespace_before.clone();
        let mut wrapper = None::<(String, String)>;
        if custom != Custom::Html
            && (text.starts_with(COMMENT_START) || text.starts_with(CDATA_START))
        {
            let Some(parts) = wrapped_content(&text) else {
                return self.add_raw(raw);
            };
            wrapper = Some((
                format!("{indentation}{}\n", parts.open),
                format!("{indentation}{}", parts.close),
            ));
            if parts.has_newline || parts.space.contains('\n') {
                let trailing = parts.space.trim_end_matches([' ', '\t']).len();
                if trailing < parts.space.len() {
                    whitespace_before = parts.space[trailing..].to_owned();
                }
            }
            let body = strip_trailing_blank_line(parts.body).to_owned();
            text = body;
        }
        if !text.is_empty() {
            text = match custom {
                Custom::Css => css_format::beautify(
                    &format!("{indentation}{text}"),
                    self.settings.indent_size,
                    self.settings.insert_spaces,
                    self.settings.limits.max_output_bytes,
                )?,
                Custom::Javascript => format!("{indentation}{text}"),
                Custom::Html => beautify(
                    &format!("{indentation}{text}"),
                    self.settings,
                    self.depth + 1,
                )?,
                Custom::Null => {
                    let mut lines = text.split('\n');
                    let mut indented = format!("{indentation}{}", lines.next().unwrap_or_default());
                    for line in lines {
                        indented.push('\n');
                        indented.push_str(&indentation);
                        indented.push_str(
                            line.strip_prefix(whitespace_before.as_str())
                                .filter(|_| !whitespace_before.is_empty())
                                .unwrap_or(line),
                        );
                    }
                    indented
                }
            };
        }
        if let Some((pre, post)) = wrapper {
            text = if text.is_empty() {
                format!("{pre}{post}")
            } else {
                format!("{pre}{text}\n{post}")
            };
        }
        self.print_newline(false)?;
        if !text.is_empty() {
            self.output.add_raw_token(&text, "", 0)?;
            self.print_newline(true)?;
        }
        Ok(())
    }
}

fn beautify(source: &str, settings: Settings, depth: usize) -> Result<String, EditorError> {
    if depth > settings.limits.max_depth {
        return Err(EditorError::Capacity);
    }
    let source = source.replace("\r\n", "\n").replace('\r', "\n");
    let base = source
        .chars()
        .take_while(|character| matches!(character, '\t' | ' '))
        .collect::<String>();
    let tokens = Tokenizer {
        input: source.chars().collect(),
        position: 0,
        newlines: 0,
        whitespace_before: String::new(),
    }
    .tokenize(settings.limits.max_tokens)?;
    let mut beautifier = Beautifier {
        tokens,
        tags: vec![TagToken {
            tag_complete: true,
            ..TagToken::default()
        }],
        frames: Vec::new(),
        output: Output::new(base, settings),
        indent_level: 0,
        alignment_size: 0,
        settings,
        depth,
    };
    let mut last_tag = 0usize;
    let mut last = Last::Start;
    for raw in 0..beautifier.tokens.len() {
        match beautifier.tokens[raw].kind {
            Kind::Eof => break,
            Kind::TagOpen | Kind::Comment => {
                last_tag = beautifier.handle_tag_open(raw, last_tag, last)?;
                last = Last::TagOpen;
            }
            Kind::Attribute | Kind::Equals | Kind::Value => {
                beautifier.handle_inside_tag(raw, last_tag)?;
                last = Last::Inside;
            }
            Kind::Text if !beautifier.tags[last_tag].tag_complete => {
                beautifier.handle_inside_tag(raw, last_tag)?;
                last = Last::Inside;
            }
            Kind::TagClose => {
                beautifier.handle_tag_close(raw, last_tag)?;
                last = Last::TagClose;
            }
            Kind::Text => {
                beautifier.handle_text(raw, last_tag)?;
                last = Last::Content;
            }
            Kind::Unknown => beautifier.add_raw(raw)?,
        }
    }
    beautifier.output.code()
}

fn is_line_break(byte: u8) -> bool {
    matches!(byte, b'\r' | b'\n')
}

fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// Formats an HTML document (or `range`) like the Monaco built-in HTML formatter with its default options.
pub fn format(
    input: &str,
    range: Option<Range<usize>>,
    options: Options,
    limits: Limits,
) -> Result<Vec<Edit>, EditorError> {
    if input.len() > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    let tab_size = match options.indentation.tab_size {
        0 => DEFAULT_TAB_SIZE,
        size => usize::try_from(size).map_err(|_| EditorError::Capacity)?,
    };
    if tab_size > limits.max_indent_width {
        return Err(EditorError::Capacity);
    }
    let insert_spaces = options.indentation.insert_spaces;
    let settings = Settings {
        indent_size: if !insert_spaces && tab_size == 1 {
            TAB_INDENT_SIZE_FOR_ONE
        } else {
            tab_size
        },
        insert_spaces,
        limits,
    };
    let bytes = input.as_bytes();
    let (start, end, initial_indent) = match range {
        None => (0, input.len(), 0),
        Some(range) => {
            if range.start > range.end
                || range.end > input.len()
                || !input.is_char_boundary(range.start)
                || !input.is_char_boundary(range.end)
            {
                return Err(EditorError::InvalidBoundary);
            }
            let mut start = range.start;
            let mut extended_start = start;
            while extended_start > 0 && is_blank(bytes[extended_start - 1]) {
                extended_start -= 1;
            }
            if extended_start == 0 || is_line_break(bytes[extended_start - 1]) {
                start = extended_start;
            } else if extended_start < start {
                start = extended_start + 1;
            }
            let mut end = range.end;
            let mut extended_end = end;
            while extended_end < input.len() && is_blank(bytes[extended_end]) {
                extended_end += 1;
            }
            if extended_end == input.len() || is_line_break(bytes[extended_end]) {
                end = extended_end;
            }
            let inside_tag = bytes[..start]
                .iter()
                .rposition(|&byte| byte == b'<' || byte == b'>')
                .is_some_and(|index| bytes[index] == b'<');
            if inside_tag {
                return Ok(vec![Edit {
                    bytes: start..end,
                    text: input[start..end].to_owned(),
                }]);
            }
            let mut initial_indent = 0;
            if start != 0 {
                let line_start = bytes[..start]
                    .iter()
                    .rposition(|&byte| is_line_break(byte))
                    .map_or(0, |index| index + 1);
                let columns = bytes[line_start..]
                    .iter()
                    .map_while(|&byte| match byte {
                        b' ' => Some(1),
                        b'\t' => Some(tab_size),
                        _ => None,
                    })
                    .sum::<usize>();
                initial_indent = columns / tab_size;
            }
            (start, end, initial_indent)
        }
    };
    let value = input[start..end].trim_start_matches(is_js_whitespace);
    let mut text = beautify(value, settings, 0)?;
    if initial_indent > 0 {
        let indent = if insert_spaces {
            " ".repeat(tab_size * initial_indent)
        } else {
            "\t".repeat(initial_indent)
        };
        text = text.replace('\n', &format!("\n{indent}"));
        let at_line_start = start == 0 || is_line_break(bytes[start - 1]);
        if at_line_start {
            text.insert_str(0, &indent);
        }
        if text.len() > limits.max_output_bytes {
            return Err(EditorError::Capacity);
        }
    }
    Ok(vec![Edit {
        bytes: start..end,
        text,
    }])
}
