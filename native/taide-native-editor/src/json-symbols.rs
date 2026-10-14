use std::collections::VecDeque;
use std::ops::Range;

pub use lsp_types::SymbolKind;

use crate::document::EditorError;
use crate::json_parser::{self, Document, Kind, Value};

const JS_SCIENTIFIC_LOWER: f64 = 1e-6;
const JS_SCIENTIFIC_UPPER: f64 = 1e21;
const KEY_BINDING_NAME: &[u16] = &[0x6b, 0x65, 0x79];
const LINE_FEED_UNIT: u16 = 0x0a;
const NEWLINE_LABEL_UNIT: u16 = 0x21b5;
const QUOTE_UNIT: u16 = 0x22;
const PATH_SEPARATOR_UNIT: u16 = 0x2e;
const COMMA_UNIT: u16 = 0x2c;
const QUOTED_LABEL_UNITS: usize = 2;
const PROPERTY_VALUE_CHILDREN: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: Vec<u16>,
    pub detail: Option<Vec<u16>>,
    pub kind: SymbolKind,
    pub parent: Option<usize>,
    pub bytes: Range<usize>,
    pub utf16: Range<usize>,
    pub selection_bytes: Range<usize>,
    pub selection_utf16: Range<usize>,
    pub container_name: Option<Vec<u16>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbols {
    pub items: Vec<Symbol>,
    pub limit_exceeded: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub parser: json_parser::Limits,
    pub max_symbols: usize,
    pub max_text_units: usize,
    pub result_limit: Option<usize>,
}

struct Collector {
    result: Symbols,
    limits: Limits,
    text_units: usize,
}

impl Collector {
    fn account(&mut self, count: usize) -> Result<(), EditorError> {
        self.text_units = self
            .text_units
            .checked_add(count)
            .ok_or(EditorError::Capacity)?;
        if self.text_units > self.limits.max_text_units {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn units(&mut self, units: &[u16]) -> Result<Vec<u16>, EditorError> {
        self.account(units.len())?;
        Ok(units.to_vec())
    }

    fn text(&mut self, text: &str) -> Result<Vec<u16>, EditorError> {
        self.account(text.encode_utf16().count())?;
        Ok(text.encode_utf16().collect())
    }

    fn push(&mut self, symbol: Symbol) -> Result<usize, EditorError> {
        if self.result.items.len() >= self.limits.max_symbols {
            return Err(EditorError::Capacity);
        }
        let index = self.result.items.len();
        self.result.items.push(symbol);
        Ok(index)
    }

    fn can_emit(&mut self) -> bool {
        if self
            .limits
            .result_limit
            .filter(|&limit| limit > 0)
            .is_some_and(|limit| self.result.items.len() >= limit)
        {
            self.result.limit_exceeded = true;
            return false;
        }
        true
    }

    fn key_label(&mut self, value: &[u16]) -> Result<Vec<u16>, EditorError> {
        let mut label = self.units(value)?;
        for unit in &mut label {
            if *unit == LINE_FEED_UNIT {
                *unit = NEWLINE_LABEL_UNIT;
            }
        }
        if label.iter().all(|unit| matches!(unit, 0x09..=0x0d | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff)) {
            self.account(QUOTED_LABEL_UNITS)?;
            label.insert(0, QUOTE_UNIT);
            label.push(QUOTE_UNIT);
        }
        Ok(label)
    }

    fn detail(
        &mut self,
        document: &Document,
        index: usize,
    ) -> Result<Option<Vec<u16>>, EditorError> {
        let node = &document.nodes[index];
        let value = match &node.value {
            Value::String(value) => self.units(value)?,
            Value::Number(value) => self.text(&number_string(*value))?,
            Value::Boolean(value) => self.text(if *value { "true" } else { "false" })?,
            Value::Null => self.text("null")?,
            Value::None => match node.kind {
                Kind::Array if node.children.is_empty() => self.text("[]")?,
                Kind::Object if node.children.is_empty() => self.text("{}")?,
                _ => return Ok(None),
            },
        };
        Ok(Some(value))
    }

    fn binding_name(&mut self, document: &Document, index: usize) -> Result<Vec<u16>, EditorError> {
        let node = &document.nodes[index];
        let is_empty = match &node.value {
            Value::String(value) => value.is_empty(),
            Value::Number(value) => *value == 0.0 || value.is_nan(),
            Value::Boolean(value) => !value,
            Value::Null => true,
            Value::None => false,
        };
        if is_empty {
            return self.text("<empty>");
        }
        enum Part {
            Node(usize),
            Comma,
        }
        let mut parts = vec![Part::Node(index)];
        let mut value = Vec::new();
        while let Some(part) = parts.pop() {
            let index = match part {
                Part::Node(index) => index,
                Part::Comma => {
                    self.account(1)?;
                    value.push(COMMA_UNIT);
                    continue;
                }
            };
            let node = &document.nodes[index];
            if node.kind == Kind::Array {
                for (position, &child) in node.children.iter().enumerate().rev() {
                    parts.push(Part::Node(child));
                    if position > 0 {
                        parts.push(Part::Comma);
                    }
                }
                continue;
            }
            let text = match &node.value {
                Value::String(text) => self.units(text)?,
                Value::Number(number) => self.text(&number_string(*number))?,
                Value::Boolean(boolean) => self.text(if *boolean { "true" } else { "false" })?,
                Value::Null => Vec::new(),
                Value::None => self.text("[object Object]")?,
            };
            value.extend(text);
        }
        Ok(value)
    }
}

pub(crate) fn number_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity"
        } else {
            "Infinity"
        }
        .to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if !(JS_SCIENTIFIC_LOWER..JS_SCIENTIFIC_UPPER).contains(&value.abs()) {
        let scientific = format!("{value:e}");
        let (mantissa, exponent) = scientific.split_once('e').unwrap();
        let sign = if exponent.starts_with('-') { "" } else { "+" };
        return format!("{mantissa}e{sign}{exponent}");
    }
    value.to_string()
}

fn symbol_kind(kind: Kind) -> SymbolKind {
    match kind {
        Kind::Object => SymbolKind::MODULE,
        Kind::Array => SymbolKind::ARRAY,
        Kind::String => SymbolKind::STRING,
        Kind::Number => SymbolKind::NUMBER,
        Kind::Boolean => SymbolKind::BOOLEAN,
        _ => SymbolKind::VARIABLE,
    }
}

pub fn document_symbols(
    input: &str,
    uri: &str,
    hierarchical: bool,
    limits: Limits,
) -> Result<Symbols, EditorError> {
    let document = json_parser::parse(input, false, limits.parser)?;
    let mut collector = Collector {
        result: Symbols {
            items: Vec::new(),
            limit_exceeded: false,
        },
        limits,
        text_units: 0,
    };
    let Some(root) = document.root else {
        return Ok(collector.result);
    };
    let is_keybindings = uri == "vscode://defaultsettings/keybindings.json"
        || uri.to_lowercase().ends_with("/user/keybindings.json");
    if is_keybindings && document.nodes[root].kind == Kind::Array {
        for &item in &document.nodes[root].children {
            if document.nodes[item].kind != Kind::Object {
                continue;
            }
            for &property in &document.nodes[item].children {
                let property = &document.nodes[property];
                let key = &document.nodes[property.children[0]];
                if !matches!(&key.value, Value::String(value) if value == KEY_BINDING_NAME)
                    || property.children.len() < PROPERTY_VALUE_CHILDREN
                {
                    continue;
                }
                let name = collector.binding_name(&document, property.children[1])?;
                let item_node = &document.nodes[item];
                let selection = if hierarchical { key } else { item_node };
                collector.push(Symbol {
                    name,
                    detail: None,
                    kind: SymbolKind::FUNCTION,
                    parent: None,
                    bytes: item_node.bytes.clone(),
                    utf16: item_node.utf16.clone(),
                    selection_bytes: selection.bytes.clone(),
                    selection_utf16: selection.utf16.clone(),
                    container_name: None,
                })?;
                if !collector.can_emit() {
                    return Ok(collector.result);
                }
            }
        }
        return Ok(collector.result);
    }
    let mut pending = VecDeque::from([(root, None, Vec::new())]);
    while let Some((index, parent, container_name)) = pending.pop_front() {
        let node = &document.nodes[index];
        if node.kind == Kind::Array {
            for (item_index, &item) in node.children.iter().enumerate() {
                if !hierarchical {
                    pending.push_back((item, None, collector.units(&container_name)?));
                    continue;
                }
                if !collector.can_emit() {
                    continue;
                }
                let item_node = &document.nodes[item];
                let name = collector.text(&item_index.to_string())?;
                let symbol = collector.push(Symbol {
                    name,
                    detail: None,
                    kind: symbol_kind(item_node.kind),
                    parent,
                    bytes: item_node.bytes.clone(),
                    utf16: item_node.utf16.clone(),
                    selection_bytes: item_node.bytes.clone(),
                    selection_utf16: item_node.utf16.clone(),
                    container_name: None,
                })?;
                pending.push_back((item, Some(symbol), Vec::new()));
            }
            continue;
        }
        if node.kind != Kind::Object {
            continue;
        }
        for &property in &node.children {
            let property = &document.nodes[property];
            let Some(&value) = property.children.get(1) else {
                continue;
            };
            if !collector.can_emit() {
                continue;
            }
            let key = &document.nodes[property.children[0]];
            let Value::String(key_value) = &key.value else {
                unreachable!()
            };
            let name = collector.key_label(key_value)?;
            let value_node = &document.nodes[value];
            let detail = if hierarchical {
                collector.detail(&document, value)?
            } else {
                None
            };
            let selection = if hierarchical { key } else { property };
            let label = if hierarchical {
                None
            } else {
                Some(collector.units(&container_name)?)
            };
            let symbol = collector.push(Symbol {
                name,
                detail,
                kind: symbol_kind(value_node.kind),
                parent,
                bytes: property.bytes.clone(),
                utf16: property.utf16.clone(),
                selection_bytes: selection.bytes.clone(),
                selection_utf16: selection.utf16.clone(),
                container_name: label,
            })?;
            if hierarchical {
                pending.push_back((value, Some(symbol), Vec::new()));
                continue;
            }
            let extra = usize::from(!container_name.is_empty());
            collector.account(
                container_name
                    .len()
                    .checked_add(key_value.len())
                    .and_then(|count| count.checked_add(extra))
                    .ok_or(EditorError::Capacity)?,
            )?;
            let mut child_container = container_name.clone();
            if !child_container.is_empty() {
                child_container.push(PATH_SEPARATOR_UNIT);
            }
            child_container.extend(key_value);
            pending.push_back((value, None, child_container));
        }
    }
    Ok(collector.result)
}
