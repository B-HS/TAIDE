use crate::document::EditorError;
use crate::html_scanner::{Scanner, ScannerState, TokenType};

/// Void elements of the default HTML5 data provider (`webCustomData` tags with `void: true`).
pub const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_nodes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub tag: Option<String>,
    pub start: usize,
    pub start_tag_end: Option<usize>,
    pub end_tag_start: Option<usize>,
    pub end: usize,
    pub closed: bool,
    pub attributes: Option<Vec<(String, Option<String>)>>,
    pub children: Vec<usize>,
    pub parent: Option<usize>,
}

impl Node {
    fn new(start: usize, end: usize, parent: Option<usize>) -> Self {
        Self {
            tag: None,
            start,
            start_tag_end: None,
            end_tag_start: None,
            end,
            closed: false,
            attributes: None,
            children: Vec::new(),
            parent,
        }
    }

    /// Returns the last value written for `name`: `Some(None)` for a valueless attribute.
    pub fn attribute(&self, name: &str) -> Option<Option<&str>> {
        self.attributes
            .as_ref()?
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_deref())
    }

    fn is_same_tag(&self, lower_case: Option<&str>) -> bool {
        match (&self.tag, lower_case) {
            (None, None) => true,
            (Some(tag), Some(lower_case)) => {
                tag.len() == lower_case.len() && tag.to_ascii_lowercase() == lower_case
            }
            _ => false,
        }
    }
}

/// Parsed HTML element tree; node 0 is the document root and its children are the top-level elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub nodes: Vec<Node>,
}

impl Document {
    pub fn roots(&self) -> &[usize] {
        &self.nodes[0].children
    }

    /// Returns the innermost node whose range contains `offset` (start exclusive, end inclusive), or the root.
    pub fn find_node_at(&self, offset: usize) -> usize {
        let mut node = 0;
        loop {
            let children = &self.nodes[node].children;
            let index = children.partition_point(|&child| self.nodes[child].start < offset);
            let Some(&child) = index.checked_sub(1).and_then(|index| children.get(index)) else {
                return node;
            };
            if offset > self.nodes[child].start && offset <= self.nodes[child].end {
                node = child;
            } else {
                return node;
            }
        }
    }
}

pub fn is_void_element(tag: &str) -> bool {
    VOID_ELEMENTS.contains(&tag.to_ascii_lowercase().as_str())
}

/// Builds the element tree the same way as the HTML language service parser.
pub fn parse(text: &str, limits: Limits) -> Result<Document, EditorError> {
    if text.len() > limits.max_bytes {
        return Err(EditorError::Capacity);
    }
    let mut scanner = Scanner::new(text, 0, ScannerState::WithinContent, true);
    let mut nodes = vec![Node::new(0, text.len(), None)];
    let mut current = 0usize;
    let mut end_tag_start = None::<usize>;
    let mut end_tag_name = None::<String>;
    let mut pending_attribute = None::<String>;
    let mut tokens = 0usize;
    loop {
        let token = scanner.scan();
        if token == TokenType::Eos {
            break;
        }
        tokens += 1;
        if tokens > limits.max_tokens {
            return Err(EditorError::Capacity);
        }
        match token {
            TokenType::StartTagOpen => {
                if nodes.len() >= limits.max_nodes {
                    return Err(EditorError::Capacity);
                }
                let child = nodes.len();
                nodes.push(Node::new(scanner.token_offset(), text.len(), Some(current)));
                nodes[current].children.push(child);
                current = child;
            }
            TokenType::StartTag => nodes[current].tag = Some(scanner.token_text().to_owned()),
            TokenType::StartTagClose => {
                if let Some(parent) = nodes[current].parent {
                    nodes[current].end = scanner.token_end();
                    if scanner.token_length() > 0 {
                        nodes[current].start_tag_end = Some(scanner.token_end());
                        if nodes[current].tag.as_deref().is_some_and(is_void_element) {
                            nodes[current].closed = true;
                            current = parent;
                        }
                    } else {
                        current = parent;
                    }
                }
            }
            TokenType::StartTagSelfClose => {
                if let Some(parent) = nodes[current].parent {
                    nodes[current].closed = true;
                    nodes[current].end = scanner.token_end();
                    nodes[current].start_tag_end = Some(scanner.token_end());
                    current = parent;
                }
            }
            TokenType::EndTagOpen => {
                end_tag_start = Some(scanner.token_offset());
                end_tag_name = None;
            }
            TokenType::EndTag => end_tag_name = Some(scanner.token_text().to_ascii_lowercase()),
            TokenType::EndTagClose => {
                let mut node = current;
                while !nodes[node].is_same_tag(end_tag_name.as_deref())
                    && let Some(parent) = nodes[node].parent
                {
                    node = parent;
                }
                if nodes[node].parent.is_some() {
                    let end_start = end_tag_start.unwrap_or(scanner.token_offset());
                    while current != node {
                        nodes[current].end = end_start;
                        nodes[current].closed = false;
                        current = nodes[current].parent.unwrap_or(0);
                    }
                    nodes[current].closed = true;
                    nodes[current].end_tag_start = Some(end_start);
                    nodes[current].end = scanner.token_end();
                    current = nodes[current].parent.unwrap_or(0);
                }
            }
            TokenType::AttributeName => {
                let name = scanner.token_text().to_owned();
                let attributes = nodes[current].attributes.get_or_insert_with(Vec::new);
                match attributes.iter_mut().find(|(key, _)| *key == name) {
                    Some((_, value)) => *value = None,
                    None => attributes.push((name.clone(), None)),
                }
                pending_attribute = Some(name);
            }
            TokenType::AttributeValue => {
                let value = scanner.token_text().to_owned();
                if let (Some(attributes), Some(name)) =
                    (&mut nodes[current].attributes, pending_attribute.as_deref())
                {
                    match attributes.iter_mut().find(|(key, _)| key == name) {
                        Some((_, slot)) => *slot = Some(value),
                        None => attributes.push((name.to_owned(), Some(value))),
                    }
                    pending_attribute = None;
                }
            }
            _ => {}
        }
    }
    while let Some(parent) = nodes[current].parent {
        nodes[current].end = text.len();
        nodes[current].closed = false;
        current = parent;
    }
    Ok(Document { nodes })
}
