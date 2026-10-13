use crate::document::EditorError;

use super::Syntax;

#[derive(Debug, Clone, Copy)]
pub struct MarkupLimits {
    pub max_bytes: usize,
    pub max_nodes: usize,
    pub max_nesting: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repeat {
    Implicit,
    Count(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupAttribute {
    pub name: Option<String>,
    pub value: Option<String>,
    pub is_expression: bool,
    pub is_multiple: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkupNode {
    pub name: Option<String>,
    pub text: Option<String>,
    pub attributes: Vec<MarkupAttribute>,
    pub children: Vec<Self>,
    pub repeat: Option<Repeat>,
    pub is_group: bool,
    pub self_closing: bool,
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
    syntax: Syntax,
    limits: MarkupLimits,
    nodes: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.position += character.len_utf8();
        Some(character)
    }

    fn consume(&mut self, character: char) -> bool {
        if self.peek() != Some(character) {
            return false;
        }
        self.next();
        true
    }

    fn balanced(&mut self, open: char, close: char) -> Result<String, EditorError> {
        if !self.consume(open) {
            return Err(EditorError::Refused);
        }
        let start = self.position;
        let mut depth = 0;
        while let Some(character) = self.next() {
            if character == '\\' {
                self.next();
                continue;
            }
            if character == open {
                depth += 1;
                if depth > self.limits.max_nesting {
                    return Err(EditorError::Capacity);
                }
            }
            if character == close {
                if depth == 0 {
                    return Ok(self.input[start..self.position - character.len_utf8()].into());
                }
                depth -= 1;
            }
        }
        Ok(self.input[start..].into())
    }

    fn quoted(&mut self) -> Result<String, EditorError> {
        let start = self.position;
        let quote = self.next().ok_or(EditorError::Refused)?;
        while let Some(character) = self.next() {
            if character == '\\' {
                self.next();
                continue;
            }
            if character == quote {
                return Ok(self.input[start..self.position].into());
            }
        }
        Err(EditorError::Refused)
    }

    fn literal(
        &mut self,
        attributes: bool,
        jsx_member: bool,
    ) -> Result<Option<String>, EditorError> {
        let start = self.position;
        let mut group_depth = 0;
        let mut attribute_depth = 0;
        let mut expression_depth = 0;
        while let Some(character) = self.peek() {
            if character == '\\' {
                self.next();
                self.next();
                continue;
            }
            if character == '$' {
                self.next();
                if attributes && self.peek() == Some('{') {
                    self.balanced('{', '}')?;
                    continue;
                }
                if self.consume('#') {
                    continue;
                }
                while self.consume('$') {}
                if self.consume('@') {
                    while self.consume('^') {}
                    self.consume('-');
                    while self
                        .peek()
                        .is_some_and(|character| character.is_ascii_digit())
                    {
                        self.next();
                    }
                }
                continue;
            }
            if attributes {
                if expression_depth > 0 {
                    match character {
                        '{' => expression_depth += 1,
                        '}' => expression_depth -= 1,
                        _ => (),
                    }
                } else {
                    match character {
                        '(' => group_depth += 1,
                        ')' if group_depth == 0 => break,
                        ')' => group_depth -= 1,
                        '[' => attribute_depth += 1,
                        ']' if attribute_depth == 0 => break,
                        ']' => attribute_depth -= 1,
                        '{' => expression_depth += 1,
                        '}' | '=' | '\'' | '"' => break,
                        character if character.is_whitespace() => break,
                        _ => (),
                    }
                }
                if group_depth + attribute_depth + expression_depth > self.limits.max_nesting {
                    return Err(EditorError::Capacity);
                }
                self.next();
                continue;
            }
            if character == '.'
                && jsx_member
                && self.input[self.position + character.len_utf8()..]
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_uppercase())
            {
                self.next();
                continue;
            }
            if character == '/'
                && self.input[..self.position]
                    .chars()
                    .next_back()
                    .is_some_and(|character| character.is_ascii_digit())
                && self.input[self.position + character.len_utf8()..]
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit())
            {
                self.next();
                continue;
            }
            if !(character.is_ascii_alphanumeric()
                || matches!(
                    character,
                    '_' | '-'
                        | ':'
                        | '!'
                        | '\u{00C4}'
                        | '\u{00D6}'
                        | '\u{00DC}'
                        | '\u{00E4}'
                        | '\u{00F6}'
                        | '\u{00FC}'
                ))
            {
                break;
            }
            self.next();
        }
        Ok((self.position != start).then(|| self.input[start..self.position].into()))
    }

    fn repeat(&mut self) -> Result<Option<Repeat>, EditorError> {
        if !self.consume('*') {
            return Ok(None);
        }
        let start = self.position;
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_digit())
        {
            self.next();
        }
        if self.position == start {
            return Ok(Some(Repeat::Implicit));
        }
        let count = self.input[start..self.position]
            .parse()
            .map_err(|_| EditorError::Capacity)?;
        Ok(Some(Repeat::Count(count)))
    }

    fn attributes(&mut self) -> Result<Vec<MarkupAttribute>, EditorError> {
        self.consume('[');
        let mut attributes = Vec::new();
        while let Some(character) = self.peek() {
            if self.consume(']') {
                break;
            }
            if character.is_whitespace() {
                self.next();
                continue;
            }
            if attributes.len() >= self.limits.max_nodes {
                return Err(EditorError::Capacity);
            }
            if matches!(character, '\'' | '"') {
                attributes.push(MarkupAttribute {
                    name: None,
                    value: Some(self.quoted()?),
                    is_expression: false,
                    is_multiple: false,
                });
                continue;
            }
            let name = self.literal(true, false)?.ok_or(EditorError::Refused)?;
            let value = if self.consume('=') {
                if self
                    .peek()
                    .is_some_and(|character| matches!(character, '\'' | '"'))
                {
                    Some(self.quoted()?)
                } else {
                    self.literal(true, false)?
                }
            } else {
                None
            };
            attributes.push(MarkupAttribute {
                name: Some(name),
                value,
                is_expression: false,
                is_multiple: false,
            });
        }
        Ok(attributes)
    }

    fn element(&mut self) -> Result<MarkupNode, EditorError> {
        let jsx_member = self.syntax == Syntax::Jsx
            && self
                .peek()
                .is_some_and(|character| character.is_ascii_uppercase());
        let mut node = MarkupNode {
            name: self.literal(false, jsx_member)?,
            ..Default::default()
        };
        loop {
            match self.peek() {
                Some('*')
                    if node.repeat.is_none()
                        && (node.name.is_some()
                            || node.text.is_some()
                            || !node.attributes.is_empty()) =>
                {
                    node.repeat = self.repeat()?
                }
                Some('{') if node.text.is_none() => node.text = Some(self.balanced('{', '}')?),
                Some(operator @ ('.' | '#')) => {
                    self.next();
                    let is_multiple = self.consume(operator);
                    while self.consume(operator) {}
                    let is_expression = self.syntax == Syntax::Jsx && self.peek() == Some('{');
                    let value = if is_expression {
                        Some(self.balanced('{', '}')?)
                    } else {
                        self.literal(false, false)?
                    };
                    node.attributes.push(MarkupAttribute {
                        name: Some(if operator == '.' { "class" } else { "id" }.into()),
                        value,
                        is_expression,
                        is_multiple,
                    });
                }
                Some('[') => node.attributes.extend(self.attributes()?),
                Some('/')
                    if node.name.is_some()
                        || node.text.is_some()
                        || !node.attributes.is_empty() =>
                {
                    self.next();
                    node.self_closing = true;
                    if node.repeat.is_none() {
                        node.repeat = self.repeat()?;
                    }
                    break;
                }
                _ => break,
            }
            if node.attributes.len() > self.limits.max_nodes {
                return Err(EditorError::Capacity);
            }
        }
        if node.name.is_none() && node.text.is_none() && node.attributes.is_empty() {
            return Err(EditorError::Refused);
        }
        Ok(node)
    }

    fn statements(&mut self, nesting: usize) -> Result<Vec<MarkupNode>, EditorError> {
        if nesting > self.limits.max_nesting {
            return Err(EditorError::Capacity);
        }
        let mut roots = Vec::<MarkupNode>::new();
        let mut path = Vec::<usize>::new();
        while self.peek().is_some_and(|character| character != ')') {
            if self.nodes >= self.limits.max_nodes {
                return Err(EditorError::Capacity);
            }
            self.nodes += 1;
            let node = if self.consume('(') {
                let children = self.statements(nesting + 1)?;
                self.consume(')');
                MarkupNode {
                    is_group: true,
                    children,
                    repeat: self.repeat()?,
                    ..Default::default()
                }
            } else {
                self.element()?
            };
            let mut children = &mut roots;
            for &index in &path {
                children = &mut children[index].children;
            }
            let index = children.len();
            children.push(node);
            if self.consume('>') {
                if nesting + path.len() >= self.limits.max_nesting {
                    return Err(EditorError::Capacity);
                }
                path.push(index);
            } else if self.consume('+') {
                continue;
            } else {
                while self.consume('^') {
                    path.pop();
                }
            }
        }
        Ok(roots)
    }
}

pub fn parse_markup(
    input: &str,
    syntax: Syntax,
    limits: MarkupLimits,
) -> Result<Vec<MarkupNode>, EditorError> {
    if input.len() > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    if syntax == Syntax::Css {
        return Err(EditorError::Refused);
    }
    let mut parser = Parser {
        input,
        position: 0,
        syntax,
        limits,
        nodes: 0,
    };
    let nodes = parser.statements(0)?;
    if parser.position != input.len() {
        return Err(EditorError::Refused);
    }
    Ok(nodes)
}
