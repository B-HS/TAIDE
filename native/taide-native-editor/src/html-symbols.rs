use std::ops::Range;

use crate::document::EditorError;
use crate::html_parser::{self, Node};
use crate::json_parser::is_ecma_whitespace as is_js_whitespace;

const UNNAMED: &str = "?";

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub parser: html_parser::Limits,
    pub max_text_bytes: usize,
}

/// A flat HTML symbol: Monaco receives `SymbolInformation` (no children) with the parent name as container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlSymbol {
    pub name: String,
    pub container: String,
    pub bytes: Range<usize>,
}

fn node_name(node: &Node) -> String {
    let mut name = node.tag.clone().unwrap_or_default();
    if let Some(Some(id)) = node.attribute("id")
        && !id.is_empty()
    {
        name.push('#');
        name.extend(
            id.chars()
                .filter(|character| !matches!(character, '"' | '\'')),
        );
    }
    if let Some(Some(classes)) = node.attribute("class")
        && !classes.is_empty()
    {
        let unquoted = classes
            .chars()
            .filter(|character| !matches!(character, '"' | '\''))
            .collect::<String>();
        let mut segment = String::new();
        let mut in_separator = false;
        for character in unquoted.chars() {
            if is_js_whitespace(character) {
                if !in_separator {
                    name.push('.');
                    name.push_str(&segment);
                    segment.clear();
                }
                in_separator = true;
            } else {
                in_separator = false;
                segment.push(character);
            }
        }
        name.push('.');
        name.push_str(&segment);
    }
    if name.is_empty() {
        return UNNAMED.to_owned();
    }
    name
}

/// Lists the elements of `text` in document order like `findDocumentSymbols` of the HTML language service.
pub fn document_symbols(text: &str, limits: Limits) -> Result<Vec<HtmlSymbol>, EditorError> {
    let document = html_parser::parse(text, limits.parser)?;
    let mut symbols = Vec::with_capacity(document.nodes.len().saturating_sub(1));
    let mut text_bytes = 0usize;
    let mut pending = document
        .roots()
        .iter()
        .rev()
        .map(|&node| (node, None::<usize>))
        .collect::<Vec<_>>();
    while let Some((index, parent)) = pending.pop() {
        let node = &document.nodes[index];
        let name = node_name(node);
        let container = parent.map_or_else(String::new, |parent| {
            symbols
                .get(parent)
                .map_or_else(String::new, |symbol: &HtmlSymbol| symbol.name.clone())
        });
        text_bytes = text_bytes
            .checked_add(name.len())
            .and_then(|bytes| bytes.checked_add(container.len()))
            .ok_or(EditorError::Capacity)?;
        if text_bytes > limits.max_text_bytes {
            return Err(EditorError::Capacity);
        }
        let symbol = symbols.len();
        symbols.push(HtmlSymbol {
            name,
            container,
            bytes: node.start..node.end,
        });
        pending.extend(
            node.children
                .iter()
                .rev()
                .map(|&child| (child, Some(symbol))),
        );
    }
    Ok(symbols)
}
