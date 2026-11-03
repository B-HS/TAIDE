use std::ops::Range;

use crate::document::{Edit, EditorError};
use crate::indent::IndentOptions;
use crate::json_parser::is_ecma_whitespace as is_js_whitespace;

const DEFAULT_TAB_SIZE: usize = 4;
const MAX_PRESERVED_NEWLINES: usize = 32786;
const NESTED_AT_RULES: [&str; 6] = [
    "page",
    "font-face",
    "keyframes",
    "media",
    "supports",
    "document",
];
const CONDITIONAL_GROUP_RULES: [&str; 3] = ["media", "supports", "document"];
const NON_SEMICOLON_NEWLINE_PROPERTIES: [&str; 2] = ["grid-template-areas", "grid-template"];
const VARIABLE_TERMINATORS: &[char] = &[
    ':', ' ', ',', ';', '{', '}', '(', ')', '[', ']', '/', '=', '\'', '"',
];
const IGNORE_END_PREFIX: &str = "/*";
const IGNORE_END_KEYWORDS: [&str; 2] = ["beautify", "ignore:end"];
const DIRECTIVE_BLOCK_PREFIX: &str = "/* beautify";
const DIRECTIVE_BLOCK_SUFFIX: &str = " */";

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub indentation: IndentOptions,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_output_bytes: usize,
    pub max_indent_width: usize,
}

struct Line {
    items: Vec<String>,
    indent: isize,
}

impl Line {
    fn new(indent: isize) -> Self {
        Self {
            items: Vec::new(),
            indent,
        }
    }

    fn set_indent(&mut self, indent: isize) {
        if self.items.is_empty() {
            self.indent = indent;
        }
    }

    fn trim(&mut self) {
        while self.items.last().is_some_and(|item| item == " ") {
            self.items.pop();
        }
    }
}

struct Output {
    lines: Vec<Line>,
    next_indent: isize,
    base: String,
    unit: String,
    space_before_token: bool,
    bytes: usize,
    max_bytes: usize,
}

impl Output {
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

    fn previous(&self) -> Option<&Line> {
        self.lines
            .len()
            .checked_sub(2)
            .map(|index| &self.lines[index])
    }

    fn add_line(&mut self) -> Result<(), EditorError> {
        self.charge(1)?;
        self.lines.push(Line::new(self.next_indent));
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
                && self.previous().is_some_and(|line| line.items.is_empty())
    }

    fn add_new_line(&mut self, force: bool) -> Result<bool, EditorError> {
        if self.is_empty() || !force && self.just_added_newline() {
            return Ok(false);
        }
        self.add_line()?;
        Ok(true)
    }

    fn set_indent(&mut self, indent: isize) {
        self.next_indent = indent;
        let indent = if self.lines.len() > 1 { indent } else { 0 };
        self.current().set_indent(indent);
    }

    fn add_token(&mut self, token: String) -> Result<(), EditorError> {
        if self.space_before_token && !self.just_added_newline() {
            self.charge(1)?;
            self.current().items.push(" ".to_owned());
        }
        self.charge(token.len())?;
        self.current().items.push(token);
        self.space_before_token = false;
        Ok(())
    }

    fn trim(&mut self, eat_newlines: bool) {
        self.current().trim();
        while eat_newlines && self.lines.len() > 1 && self.just_added_newline() {
            self.lines.pop();
            self.current().trim();
        }
    }

    fn ensure_empty_line_above(&mut self, starts_with: &str, ends_with: &str) {
        let mut index = self.lines.len().checked_sub(2);
        while let Some(current) = index {
            let line = &self.lines[current];
            if line.items.is_empty() {
                break;
            }
            if !line.items[0].starts_with(starts_with)
                && line.items.last().is_none_or(|item| item != ends_with)
            {
                self.lines.insert(current + 1, Line::new(-1));
                break;
            }
            index = current.checked_sub(1);
        }
    }

    fn code(mut self) -> String {
        self.trim(true);
        if let Some(last) = self.current().items.pop()
            && !last.is_empty()
        {
            let trimmed = last.trim_end_matches('\n').to_owned();
            self.current().items.push(trimmed);
        }
        let mut code = String::with_capacity(self.bytes);
        for (index, line) in self.lines.iter().enumerate() {
            if index > 0 {
                code.push('\n');
            }
            if line.items.is_empty() {
                continue;
            }
            if line.indent >= 0 {
                code.push_str(&self.base);
                for _ in 0..line.indent {
                    code.push_str(&self.unit);
                }
            }
            for item in &line.items {
                code.push_str(item);
            }
        }
        code
    }
}

struct Input {
    characters: Vec<char>,
    position: usize,
}

impl Input {
    fn has_next(&self) -> bool {
        self.position < self.characters.len()
    }

    fn next(&mut self) -> Option<char> {
        let character = self.characters.get(self.position).copied()?;
        self.position += 1;
        Some(character)
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.characters.get(self.position + offset).copied()
    }

    fn back(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    fn text(&self, range: Range<usize>) -> String {
        self.characters[range].iter().collect()
    }

    fn read_while(&mut self, condition: impl Fn(char) -> bool) -> String {
        let start = self.position;
        while self.peek(0).is_some_and(&condition) {
            self.position += 1;
        }
        self.text(start..self.position)
    }

    fn read_block_comment(&mut self) -> String {
        let start = self.position;
        let mut index = start + 2;
        while index < self.characters.len() {
            if self.characters[index] == '*' && self.characters.get(index + 1) == Some(&'/') {
                self.position = index + 2;
                return self.text(start..self.position);
            }
            index += 1;
        }
        self.position = self.characters.len();
        self.text(start..self.position)
    }

    fn peek_until_after(&self, terminators: &[char]) -> String {
        let end = self.characters[self.position..]
            .iter()
            .position(|character| terminators.contains(character))
            .map_or(self.characters.len(), |index| self.position + index + 1);
        self.text(self.position..end)
    }

    fn look_back(&self, value: &str) -> bool {
        let Some(start) = self.position.checked_sub(1) else {
            return false;
        };
        let length = value.chars().count();
        start >= length && self.text(start - length..start).to_lowercase() == value
    }

    fn read_until_ignore_end(&mut self) -> String {
        let start = self.position;
        let characters = &self.characters;
        let matches_at = |index: usize| {
            let mut cursor = index;
            for expected in IGNORE_END_PREFIX.chars() {
                if characters.get(cursor) != Some(&expected) {
                    return None;
                }
                cursor += 1;
            }
            for keyword in IGNORE_END_KEYWORDS {
                if !characters
                    .get(cursor)
                    .is_some_and(|&character| is_js_whitespace(character))
                {
                    return None;
                }
                cursor += 1;
                for expected in keyword.chars() {
                    if characters.get(cursor) != Some(&expected) {
                        return None;
                    }
                    cursor += 1;
                }
            }
            if !characters
                .get(cursor)
                .is_some_and(|&character| is_js_whitespace(character))
            {
                return None;
            }
            cursor += 1;
            (characters.get(cursor) == Some(&'*') && characters.get(cursor + 1) == Some(&'/'))
                .then_some(cursor + 2)
        };
        let end = (start..characters.len())
            .find_map(matches_at)
            .unwrap_or(characters.len());
        self.position = end;
        self.text(start..end)
    }
}

fn is_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn ignores_following(comment: &str) -> bool {
    let has_block = comment
        .match_indices(DIRECTIVE_BLOCK_PREFIX)
        .any(|(index, _)| {
            let mut rest = &comment[index + DIRECTIVE_BLOCK_PREFIX.len()..];
            let mut groups = 0usize;
            loop {
                if groups > 0 && rest.starts_with(DIRECTIVE_BLOCK_SUFFIX) {
                    return true;
                }
                let Some(group) = rest.strip_prefix(' ') else {
                    return false;
                };
                let key = group.trim_start_matches(is_word);
                if key.len() == group.len() {
                    return false;
                }
                let Some(value) = key.strip_prefix(':') else {
                    return false;
                };
                let after = value.trim_start_matches(is_word);
                if after.len() == value.len() {
                    return false;
                }
                rest = after;
                groups += 1;
            }
        });
    if !has_block {
        return false;
    }
    let mut ignore = None;
    let mut rest = comment;
    while let Some(index) = rest.find(' ') {
        let candidate = &rest[index + 1..];
        let key_end = candidate.len() - candidate.trim_start_matches(is_word).len();
        if key_end > 0 && candidate[key_end..].starts_with(':') {
            let value = &candidate[key_end + 1..];
            let value_end = value.len() - value.trim_start_matches(is_word).len();
            if value_end > 0 {
                if &candidate[..key_end] == "ignore" {
                    ignore = Some(&value[..value_end]);
                }
                rest = &value[value_end..];
                continue;
            }
        }
        rest = candidate;
    }
    ignore == Some("start")
}

struct Beautifier {
    input: Input,
    output: Output,
    indent_level: usize,
    nested_level: usize,
}

impl Beautifier {
    fn print(&mut self, text: String) -> Result<(), EditorError> {
        self.output
            .set_indent(isize::try_from(self.indent_level).map_err(|_| EditorError::Capacity)?);
        self.output.add_token(text)
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn outdent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    fn eat_string(&mut self, end: &[char]) -> String {
        let mut result = String::new();
        while let Some(character) = self.input.next() {
            result.push(character);
            if character == '\\' {
                if let Some(escaped) = self.input.next() {
                    result.push(escaped);
                }
            } else if end.contains(&character) || character == '\n' {
                break;
            }
        }
        result
    }

    fn eat_whitespace(&mut self, allow_newline: bool) -> Result<(), EditorError> {
        let mut newlines = 0usize;
        while self.input.peek(0).is_some_and(is_js_whitespace) {
            let character = self.input.next();
            if allow_newline
                && character == Some('\n')
                && (newlines == 0 || newlines < MAX_PRESERVED_NEWLINES)
            {
                newlines += 1;
                self.output.add_new_line(true)?;
            }
        }
        Ok(())
    }

    fn found_nested_pseudo_class(&self) -> bool {
        let mut open = 0usize;
        let mut index = 1;
        while let Some(character) = self.input.peek(index) {
            match character {
                '{' => return true,
                '(' => open += 1,
                ')' => {
                    if open == 0 {
                        return false;
                    }
                    open -= 1;
                }
                ';' | '}' => return false,
                _ => {}
            }
            index += 1;
        }
        false
    }

    fn print_variable(&mut self, value: String) -> Result<String, EditorError> {
        if value.ends_with([' ', ':']) {
            let value = self
                .eat_string(&[':', ' '])
                .trim_end_matches(is_js_whitespace)
                .to_owned();
            self.print(value.clone())?;
            self.output.space_before_token = true;
            return Ok(value);
        }
        Ok(value)
    }

    fn beautify(mut self) -> Result<String, EditorError> {
        let mut paren_level = 0usize;
        let mut inside_rule = false;
        let mut inside_property_value = false;
        let mut entering_conditional_group = false;
        let mut inside_non_nested_at_rule = false;
        let mut inside_scss_map = false;
        let mut inside_non_semicolon_values = false;
        let mut top: Option<String> = None;
        loop {
            let is_after_space = !self.input.read_while(is_js_whitespace).is_empty();
            let previous = top.take();
            let previous = previous.as_deref();
            let Some(first) = self.input.next() else {
                break;
            };
            let mut current = first.to_string();
            if first == '\\'
                && self.input.has_next()
                && let Some(escaped) = self.input.next()
            {
                current.push(escaped);
            }
            top = Some(current.clone());
            let next = self.input.peek(0);
            match current.as_str() {
                "/" if next == Some('*') => {
                    self.output.add_new_line(false)?;
                    self.input.back();
                    let mut comment = self.input.read_block_comment();
                    if ignores_following(&comment) {
                        comment.push_str(&self.input.read_until_ignore_end());
                    }
                    self.print(comment)?;
                    self.eat_whitespace(true)?;
                    self.output.add_new_line(false)?;
                }
                "/" if next == Some('/') => {
                    self.output.space_before_token = true;
                    self.input.back();
                    let comment = self.input.read_while(|character| {
                        !matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
                    });
                    self.print(comment)?;
                    self.eat_whitespace(true)?;
                }
                "$" => {
                    if is_after_space {
                        self.output.space_before_token = true;
                    }
                    self.print(current)?;
                    let variable = self.input.peek_until_after(VARIABLE_TERMINATORS);
                    let variable = self.print_variable(variable)?;
                    if paren_level == 0 && variable.contains(':') {
                        inside_property_value = true;
                        self.indent();
                    }
                }
                "@" => {
                    if is_after_space {
                        self.output.space_before_token = true;
                    }
                    if next == Some('{') {
                        let value = format!("@{}", self.eat_string(&['}']));
                        self.print(value)?;
                    } else {
                        self.print(current)?;
                        let rule = self.input.peek_until_after(VARIABLE_TERMINATORS);
                        let rule = self.print_variable(rule)?;
                        if paren_level == 0 && rule.contains(':') {
                            inside_property_value = true;
                            self.indent();
                        } else if NESTED_AT_RULES.contains(&rule.as_str()) {
                            self.nested_level += 1;
                            if CONDITIONAL_GROUP_RULES.contains(&rule.as_str()) {
                                entering_conditional_group = true;
                            }
                        } else if paren_level == 0 && !inside_property_value {
                            inside_non_nested_at_rule = true;
                        }
                    }
                }
                "#" if next == Some('{') => {
                    if is_after_space {
                        self.output.space_before_token = true;
                    }
                    let value = format!("#{}", self.eat_string(&['}']));
                    self.print(value)?;
                }
                "{" => {
                    if inside_property_value {
                        inside_property_value = false;
                        self.outdent();
                    }
                    inside_non_nested_at_rule = false;
                    inside_rule = if entering_conditional_group {
                        entering_conditional_group = false;
                        self.indent_level >= self.nested_level
                    } else {
                        self.indent_level + 1 >= self.nested_level
                    };
                    if inside_rule
                        && self
                            .output
                            .previous()
                            .is_some_and(|line| line.items.last().is_none_or(|item| item != "{"))
                    {
                        self.output.ensure_empty_line_above("/", ",");
                    }
                    self.output.space_before_token = true;
                    if previous == Some("(") {
                        self.output.space_before_token = false;
                    } else if previous != Some(",") {
                        self.indent();
                    }
                    self.print(current)?;
                    self.eat_whitespace(true)?;
                    self.output.add_new_line(false)?;
                }
                "}" => {
                    self.outdent();
                    self.output.add_new_line(false)?;
                    if previous == Some("{") {
                        self.output.trim(true);
                    }
                    if inside_property_value {
                        self.outdent();
                        inside_property_value = false;
                    }
                    self.print(current)?;
                    inside_rule = false;
                    self.nested_level = self.nested_level.saturating_sub(1);
                    self.eat_whitespace(true)?;
                    self.output.add_new_line(false)?;
                    if !self.output.just_added_blankline() && self.input.peek(0) != Some('}') {
                        self.output.add_new_line(true)?;
                    }
                    if self.input.peek(0) == Some(')') {
                        self.output.trim(true);
                    }
                }
                ":" => {
                    if NON_SEMICOLON_NEWLINE_PROPERTIES
                        .iter()
                        .any(|property| self.input.look_back(property))
                    {
                        inside_non_semicolon_values = true;
                    }
                    if (inside_rule || entering_conditional_group)
                        && !(self.input.look_back("&") || self.found_nested_pseudo_class())
                        && !self.input.look_back("(")
                        && !inside_non_nested_at_rule
                        && paren_level == 0
                    {
                        self.print(current)?;
                        if !inside_property_value {
                            inside_property_value = true;
                            self.output.space_before_token = true;
                            self.eat_whitespace(true)?;
                            self.indent();
                        }
                    } else {
                        if self.input.look_back(" ") {
                            self.output.space_before_token = true;
                        }
                        if next == Some(':') {
                            self.input.next();
                            self.print("::".to_owned())?;
                        } else {
                            self.print(current)?;
                        }
                    }
                }
                "\"" | "'" => {
                    if matches!(previous, Some("\"" | "'")) || is_after_space {
                        self.output.space_before_token = true;
                    }
                    let value = format!("{current}{}", self.eat_string(&[first]));
                    self.print(value)?;
                    self.eat_whitespace(true)?;
                }
                ";" => {
                    inside_non_semicolon_values = false;
                    if paren_level == 0 {
                        if inside_property_value {
                            self.outdent();
                            inside_property_value = false;
                        }
                        inside_non_nested_at_rule = false;
                        self.print(current)?;
                        self.eat_whitespace(true)?;
                        if self.input.peek(0) != Some('/') {
                            self.output.add_new_line(false)?;
                        }
                    } else {
                        self.print(current)?;
                        self.eat_whitespace(true)?;
                        self.output.space_before_token = true;
                    }
                }
                "(" => {
                    if self.input.look_back("url") {
                        self.print(current)?;
                        self.eat_whitespace(false)?;
                        paren_level += 1;
                        self.indent();
                        match self.input.next() {
                            Some(')' | '"' | '\'') => self.input.back(),
                            Some(character) => {
                                let value = format!("{character}{}", self.eat_string(&[')']));
                                self.print(value)?;
                                if paren_level > 0 {
                                    paren_level -= 1;
                                    self.outdent();
                                }
                            }
                            None => {}
                        }
                    } else {
                        if is_after_space || self.input.look_back("with") {
                            self.output.space_before_token = true;
                        }
                        self.print(current)?;
                        if inside_property_value && previous == Some("$") {
                            self.output.add_new_line(false)?;
                            inside_scss_map = true;
                        } else {
                            self.eat_whitespace(false)?;
                            paren_level += 1;
                            self.indent();
                        }
                    }
                }
                ")" => {
                    if paren_level > 0 {
                        paren_level -= 1;
                        self.outdent();
                    }
                    if inside_scss_map && self.input.peek(0) == Some(';') {
                        inside_scss_map = false;
                        self.outdent();
                        self.output.add_new_line(false)?;
                    }
                    self.print(current)?;
                }
                "," => {
                    self.print(current)?;
                    self.eat_whitespace(true)?;
                    if (!inside_property_value || inside_scss_map)
                        && paren_level == 0
                        && !inside_non_nested_at_rule
                    {
                        self.output.add_new_line(false)?;
                    } else {
                        self.output.space_before_token = true;
                    }
                }
                ">" | "+" | "~" if !inside_property_value && paren_level == 0 => {
                    self.print(current)?;
                    self.eat_whitespace(false)?;
                }
                "]" => self.print(current)?,
                "[" => {
                    if is_after_space {
                        self.output.space_before_token = true;
                    }
                    self.print(current)?;
                }
                "=" => {
                    self.eat_whitespace(false)?;
                    self.print(current)?;
                }
                "!" if !self.input.look_back("\\") => {
                    self.output.space_before_token = true;
                    self.print(current)?;
                }
                _ => {
                    if matches!(previous, Some("\"" | "'")) || is_after_space {
                        self.output.space_before_token = true;
                    }
                    self.print(current)?;
                    if !self.output.just_added_newline()
                        && self.input.peek(0) == Some('\n')
                        && inside_non_semicolon_values
                    {
                        self.output.add_new_line(false)?;
                    }
                }
            }
        }
        Ok(self.output.code())
    }
}

pub(crate) fn beautify(
    source: &str,
    indent_size: usize,
    insert_spaces: bool,
    max_output_bytes: usize,
) -> Result<String, EditorError> {
    let source = source.replace("\r\n", "\n").replace('\r', "\n");
    let base = source
        .chars()
        .take_while(|character| matches!(character, '\t' | ' '))
        .collect::<String>();
    let unit = if insert_spaces {
        " ".repeat(indent_size)
    } else {
        "\t".to_owned()
    };
    let output = Output {
        lines: vec![Line::new(-1)],
        next_indent: -1,
        base,
        unit,
        space_before_token: false,
        bytes: 0,
        max_bytes: max_output_bytes,
    };
    Beautifier {
        input: Input {
            characters: source.chars().collect(),
            position: 0,
        },
        output,
        indent_level: 0,
        nested_level: 0,
    }
    .beautify()
}

fn is_line_break(byte: u8) -> bool {
    matches!(byte, b'\r' | b'\n')
}

fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// Formats a CSS or SCSS document (or the lines touched by `range`) like the Monaco built-in CSS formatter.
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
    let bytes = input.as_bytes();
    let Some(range) = range else {
        let text = beautify(
            input,
            tab_size,
            options.indentation.insert_spaces,
            limits.max_output_bytes,
        )?;
        return Ok(vec![Edit {
            bytes: 0..input.len(),
            text,
        }]);
    };
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
    let in_rule = bytes[..(start + 1).min(input.len())]
        .iter()
        .rev()
        .find(|&&byte| byte == b'{' || byte == b'}')
        .is_some_and(|&byte| byte == b'{');
    let line_start = bytes[..start]
        .iter()
        .rposition(|&byte| is_line_break(byte))
        .map_or(0, |index| index + 1);
    let starts_line = start == line_start;
    let mut value = input[start..end].to_owned();
    let mut indent_level = 0;
    if start != 0 {
        let mut columns = 0usize;
        for &byte in &bytes[line_start..] {
            match byte {
                b' ' => columns += 1,
                b'\t' => columns += tab_size,
                _ => break,
            }
        }
        indent_level = columns / tab_size;
    }
    if in_rule {
        value = format!("{{\n{}", value.trim_start_matches(is_js_whitespace));
    }
    let mut text = beautify(
        &value,
        tab_size,
        options.indentation.insert_spaces,
        limits.max_output_bytes,
    )?;
    if in_rule {
        let skipped = text
            .char_indices()
            .nth(2)
            .map_or(text.len(), |(index, _)| index);
        text = text[skipped..]
            .trim_start_matches(is_js_whitespace)
            .to_owned();
    }
    if indent_level > 0 {
        let indent = if options.indentation.insert_spaces {
            " ".repeat(tab_size * indent_level)
        } else {
            "\t".repeat(indent_level)
        };
        let lines = text.matches('\n').count();
        let length = text
            .len()
            .checked_add(indent.len().saturating_mul(lines + 1))
            .ok_or(EditorError::Capacity)?;
        if length > limits.max_output_bytes {
            return Err(EditorError::Capacity);
        }
        text = text.replace('\n', &format!("\n{indent}"));
        if starts_line {
            text.insert_str(0, &indent);
        }
    }
    Ok(vec![Edit {
        bytes: start..end,
        text,
    }])
}
