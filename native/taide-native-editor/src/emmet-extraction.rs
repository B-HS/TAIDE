use super::{Extraction, Syntax};

#[derive(Clone)]
struct Scanner<'a> {
    line: &'a str,
    position: usize,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<char> {
        self.line[..self.position].chars().next_back()
    }

    fn previous(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.position -= character.len_utf8();
        Some(character)
    }

    fn consume(&mut self, character: char) -> bool {
        if self.peek() != Some(character) {
            return false;
        }
        self.previous();
        true
    }

    fn consume_while(&mut self, predicate: impl Fn(char) -> bool) -> bool {
        let start = self.position;
        while self.peek().is_some_and(&predicate) {
            self.previous();
        }
        self.position != start
    }

    fn quoted_attribute(&mut self) -> bool {
        let start = self.position;
        let quote = self.previous();
        if let Some(quote @ ('\'' | '"')) = quote {
            while let Some(character) = self.previous() {
                if character == quote && self.peek() != Some('\\') {
                    if self.consume('=') && self.consume_while(is_identifier) {
                        return true;
                    }
                    break;
                }
            }
        }
        self.position = start;
        false
    }

    fn unquoted_attribute(&mut self) -> bool {
        let start = self.position;
        let mut brackets = Vec::new();
        while let Some(character) = self.peek() {
            if matches!(character, '}' | ')' | ']') {
                brackets.push(character);
            } else if let Some(close) = closing_bracket(character, Syntax::Html) {
                if brackets.pop() != Some(close) {
                    break;
                }
            } else if matches!(character, '=' | ' ' | '\t' | '\'' | '"') {
                break;
            }
            self.previous();
        }
        if start != self.position && self.consume('=') && self.consume_while(is_identifier) {
            return true;
        }
        self.position = start;
        false
    }

    fn is_html(&self) -> bool {
        let mut scanner = self.clone();
        if !scanner.consume('>') {
            return false;
        }
        scanner.consume('/');
        while scanner.position > 0 {
            scanner.consume_while(|character| matches!(character, ' ' | '\t'));
            if scanner.consume_while(is_identifier) {
                if scanner.consume('/') {
                    return scanner.consume('<');
                }
                if scanner.consume('<') {
                    return true;
                }
                if scanner.consume(' ') || scanner.consume('\t') {
                    continue;
                }
                if scanner.consume('=') {
                    if scanner.consume_while(is_identifier) {
                        continue;
                    }
                    return false;
                }
                return scanner.unquoted_attribute();
            }
            if scanner.quoted_attribute() || scanner.unquoted_attribute() {
                continue;
            }
            return false;
        }
        false
    }
}

fn is_identifier(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, ':' | '-')
}

fn closing_bracket(character: char, syntax: Syntax) -> Option<char> {
    match character {
        '(' => Some(')'),
        '[' if syntax != Syntax::Css => Some(']'),
        '{' if syntax != Syntax::Css => Some('}'),
        _ => None,
    }
}

fn is_close(character: char, syntax: Syntax) -> bool {
    character == ')' || syntax != Syntax::Css && matches!(character, ']' | '}')
}

pub fn extract(
    line: &str,
    cursor: usize,
    syntax: Syntax,
    max_bytes: usize,
) -> Option<Extraction<'_>> {
    if line.len() > max_bytes || cursor > line.len() || !line.is_char_boundary(cursor) {
        return None;
    }
    let mut end = cursor;
    if syntax != Syntax::Css {
        if line[end..].starts_with(['\'', '"']) {
            end += 1;
        }
        while let Some(character) = line[end..].chars().next() {
            if !is_close(character, syntax) {
                break;
            }
            end += character.len_utf8();
        }
    }
    let mut scanner = Scanner {
        line,
        position: end,
    };
    let mut brackets = Vec::new();
    while let Some(character) = scanner.peek() {
        if brackets.contains(&'}') {
            if character == '}' {
                brackets.push(character);
                scanner.previous();
                continue;
            }
            if character != '{' {
                scanner.previous();
                continue;
            }
        }
        if is_close(character, syntax) {
            brackets.push(character);
        } else if let Some(close) = closing_bracket(character, syntax) {
            if brackets.pop() != Some(close) {
                break;
            }
        } else if brackets.contains(&']') || brackets.contains(&'}') {
            scanner.previous();
            continue;
        } else if scanner.is_html()
            || !(character.is_ascii_alphanumeric() || "#.*:$-_!@%^+>/".contains(character))
        {
            break;
        }
        scanner.previous();
    }
    if !brackets.is_empty() || scanner.position == end {
        return None;
    }
    let abbreviation = line[scanner.position..end].trim_start_matches(['*', '+', '>', '^']);
    Some(Extraction {
        abbreviation,
        bytes: end - abbreviation.len()..end,
    })
}
