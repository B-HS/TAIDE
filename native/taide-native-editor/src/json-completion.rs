use std::collections::{HashMap, HashSet};
use std::ops::Range;

use lsp_types::{Command, CompletionItemKind, Documentation, MarkupContent, MarkupKind};

use crate::document::EditorError;
use crate::json_parser::{self, Document, Kind, Value as NodeValue, is_ecma_whitespace};
use crate::json_scanner::{self, Kind as TokenKind};
use crate::json_schema_associations::PatternCompiler as AssociationCompiler;
use crate::json_schema_document::{self, Contribution};
use crate::json_schema_evaluation::{self, SchemaView, Selection};
use crate::json_schema_string::{PatternCompiler, compile_extended};
use crate::json_value::{Arena, Value};
use crate::json_value_text;

const MAX_LABEL_UNITS: usize = 60;
const SHORTENED_LABEL_UNITS: usize = 57;
const MAX_DEFAULT_ARRAY_DEPTH: usize = 5;
const WORD_SEPARATORS: &[u8] = b" \t\n\r\x0b\":{[,]}";
const NEW_LINE_SYMBOL: &str = "\u{21b5}";
const TRIGGER_SUGGEST: &str = "editor.action.triggerSuggest";
const TRIGGER_SUGGEST_TITLE: &str = "Suggest";
const NEW_OBJECT: &str = "New object";
const NEW_ARRAY: &str = "New array";
const DRAFT_PREFIX: &str = "http://json-schema.org/draft-";

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub document: json_schema_document::Limits,
    pub max_items: usize,
    pub max_text_bytes: usize,
    pub max_work: usize,
    pub max_depth: usize,
}

/// The configured schema state and pattern engines one JSON language-service request uses.
#[derive(Clone, Copy)]
pub struct Environment<'a> {
    pub resource: &'a str,
    pub contributions: &'a [Contribution],
    pub associations: &'a dyn AssociationCompiler,
    pub patterns: &'a dyn PatternCompiler,
    pub cancelled: &'a dyn Fn() -> bool,
}

/// One `doComplete` proposal; `range` is the byte range its insert text replaces.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub label: String,
    pub kind: CompletionItemKind,
    pub insert_text: String,
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
    pub documentation: Option<Documentation>,
    pub detail: Option<String>,
    pub command: Option<Command>,
    pub range: Range<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Completion {
    pub items: Vec<Item>,
    pub is_incomplete: bool,
}

pub(crate) struct Budget {
    work: usize,
    text: usize,
    depth: usize,
}

impl Budget {
    pub(crate) fn new(max_work: usize, max_text_bytes: usize, max_depth: usize) -> Self {
        Self {
            work: max_work,
            text: max_text_bytes,
            depth: max_depth,
        }
    }

    pub(crate) fn spend(&mut self, count: usize) -> Result<(), EditorError> {
        self.work = self.work.checked_sub(count).ok_or(EditorError::Capacity)?;
        Ok(())
    }

    fn keep(&mut self, value: String) -> Result<String, EditorError> {
        self.text = self
            .text
            .checked_sub(value.len())
            .ok_or(EditorError::Capacity)?;
        Ok(value)
    }

    pub(crate) fn compact(&mut self, arena: &Arena, node: usize) -> Result<String, EditorError> {
        let limits = json_value_text::Limits {
            max_units: self.text,
            max_work: self.work,
            max_depth: self.depth,
        };
        let units = json_value_text::stringify_budgeted(arena, node, limits, &mut self.work)?;
        self.keep(String::from_utf16_lossy(&units))
    }

    pub(crate) fn quote(&mut self, value: &[u16]) -> Result<String, EditorError> {
        self.spend(value.len())?;
        let units = json_value_text::quoted(value, self.text)?;
        self.keep(String::from_utf16_lossy(&units))
    }

    fn pretty(
        &mut self,
        arena: &Arena,
        node: usize,
        level: usize,
        active: &mut Vec<usize>,
        output: &mut String,
    ) -> Result<(), EditorError> {
        self.spend(1)?;
        if level > self.depth || output.len() > self.text {
            return Err(EditorError::Capacity);
        }
        let value = arena.get(node).ok_or(EditorError::InvalidIdentity)?;
        let (open, close, count) = match value {
            Value::Array(items) if !items.is_empty() => ('[', ']', items.len()),
            Value::Object(members) if !members.is_empty() => ('{', '}', members.len()),
            Value::Array(_) => {
                output.push_str("[]");
                return Ok(());
            }
            Value::Object(_) => {
                output.push_str("{}");
                return Ok(());
            }
            _ => {
                let text = self.compact(arena, node)?;
                output.push_str(&text);
                return Ok(());
            }
        };
        if active.contains(&node) {
            return Err(EditorError::InvalidIdentity);
        }
        active.push(node);
        output.push(open);
        for index in 0..count {
            if index > 0 {
                output.push(',');
            }
            output.push('\n');
            output.push_str(&"\t".repeat(level + 1));
            let child = match value {
                Value::Object(members) => {
                    let key = self.quote(&members[index].key)?;
                    output.push_str(&key);
                    output.push_str(": ");
                    members[index].value
                }
                Value::Array(items) => items[index],
                _ => unreachable!(),
            };
            self.pretty(arena, child, level + 1, active, output)?;
        }
        output.push('\n');
        output.push_str(&"\t".repeat(level));
        output.push(close);
        active.pop();
        Ok(())
    }

    fn snippet_object(
        &mut self,
        arena: &Arena,
        node: usize,
        wraps: usize,
        level: usize,
        active: &mut Vec<usize>,
        output: &mut String,
    ) -> Result<(), EditorError> {
        self.spend(1)?;
        if level > self.depth || output.len() > self.text {
            return Err(EditorError::Capacity);
        }
        if wraps > 0 {
            output.push_str("[\n");
            output.push_str(&"\t".repeat(level + 1));
            self.snippet_object(arena, node, wraps - 1, level + 1, active, output)?;
            output.push('\n');
            output.push_str(&"\t".repeat(level));
            output.push(']');
            return Ok(());
        }
        let value = arena.get(node).ok_or(EditorError::InvalidIdentity)?;
        let (open, close, count) = match value {
            Value::Array(items) if !items.is_empty() => ('[', ']', items.len()),
            Value::Object(members) if !members.is_empty() => ('{', '}', members.len()),
            Value::Array(_) => {
                output.push_str("[]");
                return Ok(());
            }
            Value::Object(_) => {
                output.push_str("{}");
                return Ok(());
            }
            Value::String(text) if text.first() == Some(&u16::from(b'^')) => {
                self.spend(text.len())?;
                output.push_str(&String::from_utf16_lossy(&text[1..]));
                return Ok(());
            }
            _ => {
                let text = self.compact(arena, node)?;
                output.push_str(&text);
                return Ok(());
            }
        };
        if active.contains(&node) {
            return Err(EditorError::InvalidIdentity);
        }
        active.push(node);
        output.push(open);
        output.push('\n');
        for index in 0..count {
            output.push_str(&"\t".repeat(level + 1));
            let child = match value {
                Value::Object(members) => {
                    let key = self.quote(&members[index].key)?;
                    output.push_str(&key);
                    output.push_str(": ");
                    members[index].value
                }
                Value::Array(items) => items[index],
                _ => unreachable!(),
            };
            self.snippet_object(arena, child, 0, level + 1, active, output)?;
            if index + 1 < count {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str(&"\t".repeat(level));
        output.push(close);
        active.pop();
        Ok(())
    }

    fn wrapped_compact(
        &mut self,
        arena: &Arena,
        node: usize,
        wraps: usize,
    ) -> Result<String, EditorError> {
        let text = self.compact(arena, node)?;
        self.keep(format!("{}{text}{}", "[".repeat(wraps), "]".repeat(wraps)))
    }

    fn wrapped_pretty(
        &mut self,
        arena: &Arena,
        node: usize,
        wraps: usize,
    ) -> Result<String, EditorError> {
        let mut text = String::new();
        self.pretty(arena, node, wraps, &mut Vec::new(), &mut text)?;
        for level in (0..wraps).rev() {
            text = format!(
                "[\n{}{text}\n{}]",
                "\t".repeat(level + 1),
                "\t".repeat(level)
            );
        }
        self.keep(text)
    }

    fn value_insert(
        &mut self,
        arena: &Arena,
        node: usize,
        wraps: usize,
        separator: &str,
    ) -> Result<String, EditorError> {
        let text = self.wrapped_pretty(arena, node, wraps)?;
        Ok(match text.as_str() {
            "{}" => format!("{{$1}}{separator}"),
            "[]" => format!("[$1]{separator}"),
            _ => escape(&format!("{text}{separator}")),
        })
    }

    fn snippet_insert(
        &mut self,
        arena: &Arena,
        node: usize,
        wraps: usize,
    ) -> Result<String, EditorError> {
        let mut text = String::new();
        self.snippet_object(arena, node, wraps, 0, &mut Vec::new(), &mut text)?;
        self.keep(text)
    }

    fn guessed(
        &mut self,
        arena: &Arena,
        node: usize,
        separator: &str,
    ) -> Result<String, EditorError> {
        match arena.get(node).ok_or(EditorError::InvalidIdentity)? {
            Value::Null => Ok(format!("${{1:null}}{separator}")),
            Value::Array(_) | Value::Object(_) => self.value_insert(arena, node, 0, separator),
            Value::String(text) => {
                let quoted = self.quote(text)?;
                Ok(format!(
                    "\"${{1:{}}}\"{separator}",
                    escape(&quoted[1..quoted.len() - 1])
                ))
            }
            Value::Number(_) | Value::Boolean(_) => {
                let text = self.compact(arena, node)?;
                Ok(format!("${{1:{text}}}{separator}"))
            }
        }
    }
}

pub(crate) fn field(arena: &Arena, schema: usize, name: &str) -> Option<usize> {
    let Some(Value::Object(members)) = arena.get(schema) else {
        return None;
    };
    members
        .iter()
        .find(|member| member.key.iter().copied().eq(name.encode_utf16()))
        .map(|member| member.value)
}

pub(crate) fn text<'a>(arena: &'a Arena, schema: usize, name: &str) -> Option<&'a [u16]> {
    match arena.get(field(arena, schema, name)?)? {
        Value::String(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

fn truthy(arena: &Arena, schema: usize, name: &str) -> bool {
    field(arena, schema, name)
        .and_then(|index| arena.get(index))
        .is_some_and(Value::is_truthy)
}

fn items<'a>(arena: &'a Arena, schema: usize, name: &str) -> Option<&'a [usize]> {
    match arena.get(field(arena, schema, name)?)? {
        Value::Array(items) => Some(items),
        _ => None,
    }
}

fn is_object(arena: &Arena, node: usize) -> bool {
    matches!(arena.get(node), Some(Value::Object(_)))
}

fn escape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '$' | '}') {
            result.push('\\');
        }
        result.push(character);
    }
    result
}

fn strip_placeholders(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('$') {
        result.push_str(&rest[..index]);
        let tail = &rest[index + 1..];
        let digits = |value: &str| value.bytes().take_while(u8::is_ascii_digit).count();
        if let Some(body) = tail.strip_prefix('{')
            && let count = digits(body)
            && count > 0
            && let Some(placeholder) = body[count..].strip_prefix(':')
            && let Some(end) = placeholder.find('}')
            && end > 0
        {
            result.push_str(&placeholder[..end]);
            rest = &placeholder[end + 1..];
            continue;
        }
        let count = digits(tail);
        if count > 0 {
            rest = &tail[count..];
            continue;
        }
        result.push('$');
        rest = tail;
    }
    result.push_str(rest);
    result
}

fn markdown(arena: &Arena, schema: usize, name: &str) -> Option<Documentation> {
    text(arena, schema, name).map(|value| {
        Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: String::from_utf16_lossy(value),
        })
    })
}

fn plain(arena: &Arena, schema: usize, name: &str) -> Option<Documentation> {
    text(arena, schema, name).map(|value| Documentation::String(String::from_utf16_lossy(value)))
}

fn empty_documentation() -> Option<Documentation> {
    Some(Documentation::String(String::new()))
}

fn is_falsy(documentation: &Option<Documentation>) -> bool {
    matches!(documentation, None)
        || matches!(documentation, Some(Documentation::String(text)) if text.is_empty())
}

fn type_kind(arena: &Arena, schema: usize) -> CompletionItemKind {
    let name = field(arena, schema, "type").and_then(|index| match arena.get(index)? {
        Value::Array(items) => match arena.get(*items.first()?)? {
            Value::String(name) => Some(name.clone()),
            _ => None,
        },
        Value::String(name) => Some(name.clone()),
        _ => None,
    });
    kind(name.map(|name| String::from_utf16_lossy(&name)).as_deref())
}

fn kind(name: Option<&str>) -> CompletionItemKind {
    match name {
        Some("object") => CompletionItemKind::MODULE,
        Some("property") => CompletionItemKind::PROPERTY,
        _ => CompletionItemKind::VALUE,
    }
}

fn node_kind(kind: Kind) -> CompletionItemKind {
    if kind == Kind::Object {
        CompletionItemKind::MODULE
    } else {
        CompletionItemKind::VALUE
    }
}

fn is_scalar(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::String | Kind::Number | Kind::Boolean | Kind::Null
    )
}

fn key(document: &Document, property: usize) -> &[u16] {
    document.nodes[property]
        .children
        .first()
        .and_then(|key| match &document.nodes[*key].value {
            NodeValue::String(value) => Some(value.as_slice()),
            _ => None,
        })
        .unwrap_or_default()
}

fn property_value(document: &Document, property: usize) -> Option<usize> {
    document.nodes[property].children.get(1).copied()
}

fn contains(document: &Document, node: usize, offset: usize) -> bool {
    let bytes = &document.nodes[node].bytes;
    offset >= bytes.start && offset <= bytes.end
}

fn preorder(document: &Document) -> Vec<usize> {
    let mut order = Vec::with_capacity(document.nodes.len());
    let mut pending = document.root.into_iter().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        order.push(node);
        pending.extend(document.nodes[node].children.iter().rev());
    }
    order
}

pub(crate) fn scanner_limits(limits: &json_schema_document::Limits) -> json_scanner::Limits {
    limits.policy.syntax.parser.scanner
}

fn next_token(
    input: &str,
    offset: usize,
    limits: json_scanner::Limits,
) -> Result<(TokenKind, Range<usize>), EditorError> {
    let token = json_scanner::tokens(&input[offset..], true, limits)?
        .next()
        .ok_or(EditorError::InvalidBoundary)??;
    Ok((
        token.kind,
        offset + token.bytes.start..offset + token.bytes.end,
    ))
}

fn separator_after(
    input: &str,
    offset: usize,
    limits: json_scanner::Limits,
) -> Result<&'static str, EditorError> {
    Ok(match next_token(input, offset, limits)?.0 {
        TokenKind::Comma | TokenKind::CloseBrace | TokenKind::CloseBracket | TokenKind::End => "",
        _ => ",",
    })
}

fn in_comment(
    input: &str,
    start: usize,
    offset: usize,
    limits: json_scanner::Limits,
) -> Result<bool, EditorError> {
    for token in json_scanner::tokens(&input[start..], false, limits)? {
        let token = token?;
        if token.kind != TokenKind::End && start + token.bytes.end < offset {
            continue;
        }
        return Ok(
            matches!(token.kind, TokenKind::LineComment | TokenKind::BlockComment)
                && start + token.bytes.start <= offset,
        );
    }
    Ok(false)
}

fn current_word(input: &str, offset: usize) -> Range<usize> {
    let bytes = input.as_bytes();
    let start = bytes[..offset]
        .iter()
        .rposition(|byte| WORD_SEPARATORS.contains(byte))
        .map_or(0, |index| index + 1);
    start..offset
}

struct Completer<'a> {
    input: &'a str,
    document: &'a Document,
    schema: Option<&'a Arena>,
    registered: &'a [Vec<u16>],
    patterns: &'a dyn PatternCompiler,
    limits: Limits,
    budget: Budget,
    range: Range<usize>,
    items: Vec<Item>,
    proposed: HashMap<String, Option<usize>>,
    unique: Option<HashSet<String>>,
    incomplete: bool,
}

impl Completer<'_> {
    fn scanner(&self) -> json_scanner::Limits {
        scanner_limits(&self.limits.document)
    }

    fn add(&mut self, mut item: Item) -> Result<(), EditorError> {
        self.budget.spend(1)?;
        if self
            .unique
            .as_ref()
            .is_some_and(|values| values.contains(&item.label))
        {
            return Ok(());
        }
        if let Some(existing) = self.proposed.get(&item.label) {
            if let Some(index) = *existing {
                let existing = &mut self.items[index];
                if is_falsy(&existing.documentation) {
                    existing.documentation = item.documentation;
                }
                if existing.detail.as_deref().is_none_or(str::is_empty) {
                    existing.detail = item.detail;
                }
            }
            return Ok(());
        }
        let mut label = item.label.replace('\n', NEW_LINE_SYMBOL);
        let units = label.encode_utf16().collect::<Vec<_>>();
        if units.len() > MAX_LABEL_UNITS {
            let shortened = format!(
                "{}...",
                String::from_utf16_lossy(&units[..SHORTENED_LABEL_UNITS])
                    .trim_matches(is_ecma_whitespace)
            );
            if !self.proposed.contains_key(&shortened) {
                label = shortened;
            }
        }
        if self.items.len() >= self.limits.max_items {
            return Err(EditorError::Capacity);
        }
        item.label = self.budget.keep(label)?;
        item.insert_text = self.budget.keep(item.insert_text)?;
        item.range = self.range.clone();
        self.proposed
            .insert(item.label.clone(), Some(self.items.len()));
        self.items.push(item);
        Ok(())
    }

    fn item(
        &self,
        kind: CompletionItemKind,
        label: String,
        insert_text: String,
        documentation: Option<Documentation>,
    ) -> Item {
        Item {
            label,
            kind,
            insert_text,
            filter_text: None,
            sort_text: None,
            documentation,
            detail: None,
            command: None,
            range: self.range.clone(),
        }
    }

    fn property_insert(
        &mut self,
        key: &[u16],
        property: Option<usize>,
        add_value: bool,
        separator: &str,
    ) -> Result<String, EditorError> {
        let quoted = self.budget.quote(key)?;
        let property_text = escape(&quoted);
        if !add_value {
            return Ok(property_text);
        }
        let mut value = None::<String>;
        let mut proposals = 0usize;
        if let (Some(schema), Some(property)) = (self.schema, property) {
            if let Some(snippets) = items(schema, property, "defaultSnippets") {
                if let [snippet] = snippets
                    && let Some(body) = field(schema, *snippet, "body")
                {
                    value = Some(self.budget.snippet_insert(schema, body, 0)?)
                        .filter(|text| !text.is_empty());
                }
                proposals += snippets.len();
            }
            if let Some(values) = items(schema, property, "enum") {
                if value.is_none()
                    && let [only] = values
                {
                    value = Some(self.budget.guessed(schema, *only, "")?);
                }
                proposals += values.len();
            }
            for name in ["const", "default"] {
                if let Some(constant) = field(schema, property, name) {
                    if value.is_none() {
                        value = Some(self.budget.guessed(schema, constant, "")?);
                    }
                    proposals += 1;
                }
            }
            if let Some(examples) = items(schema, property, "examples")
                && let Some(first) = examples.first()
            {
                if value.is_none() {
                    value = Some(self.budget.guessed(schema, *first, "")?);
                }
                proposals += examples.len();
            }
            if proposals == 0 {
                let declared =
                    field(schema, property, "type").and_then(|index| match schema.get(index)? {
                        Value::Array(types) => types.first().and_then(|first| schema.get(*first)),
                        value => Some(value),
                    });
                let name = match declared {
                    Some(Value::String(name)) if !name.is_empty() => {
                        Some(String::from_utf16_lossy(name))
                    }
                    Some(declared) if declared.is_truthy() => None,
                    _ if truthy(schema, property, "properties") => Some("object".into()),
                    _ if truthy(schema, property, "items") => Some("array".into()),
                    _ => None,
                };
                value = Some(
                    match name.as_deref() {
                        Some("boolean") => "$1",
                        Some("string") => "\"$1\"",
                        Some("object") => "{$1}",
                        Some("array") => "[$1]",
                        Some("number" | "integer") => "${1:0}",
                        Some("null") => "${1:null}",
                        _ => return Ok(property_text),
                    }
                    .to_owned(),
                );
            }
        }
        let value = match value {
            Some(value) if proposals <= 1 => value,
            _ => "$1".to_owned(),
        };
        Ok(format!("{property_text}: {value}{separator}"))
    }

    fn property_item(
        &mut self,
        name: &[u16],
        property: Option<usize>,
        add_value: bool,
        separator: &str,
        documentation: Option<Documentation>,
        sort_text: Option<String>,
    ) -> Result<(), EditorError> {
        let insert_text = self.property_insert(name, property, add_value, separator)?;
        let filter_text = self.budget.quote(name)?;
        let command = insert_text
            .ends_with(&format!("$1{separator}"))
            .then(|| Command {
                title: TRIGGER_SUGGEST_TITLE.into(),
                command: TRIGGER_SUGGEST.into(),
                arguments: None,
            });
        let mut item = self.item(
            CompletionItemKind::PROPERTY,
            String::from_utf16_lossy(name),
            insert_text,
            documentation,
        );
        item.filter_text = Some(filter_text);
        item.sort_text = sort_text;
        item.command = command;
        self.add(item)
    }

    fn property_completions(
        &mut self,
        schema: &Arena,
        object: usize,
        add_value: bool,
        separator: &str,
    ) -> Result<(), EditorError> {
        let Some(root) = schema.root() else {
            return Ok(());
        };
        let matching = json_schema_evaluation::matching_schemas(
            self.document,
            schema,
            root,
            self.patterns,
            Selection {
                focus_byte: Some(self.document.nodes[object].bytes.start),
                exclude_node: None,
            },
            self.limits.document.policy.evaluation,
        )?;
        for matched in matching {
            if matched.node != object || matched.inverted || matched.view != SchemaView::Source {
                continue;
            }
            let current = matched.schema;
            if let Some(index) = field(schema, current, "properties")
                && let Some(Value::Object(members)) = schema.get(index)
            {
                for member in members {
                    let property = member.value;
                    if !matches!(
                        schema.get(property),
                        Some(Value::Object(_) | Value::Array(_))
                    ) || truthy(schema, property, "deprecationMessage")
                        || truthy(schema, property, "doNotSuggest")
                    {
                        continue;
                    }
                    let documentation = markdown(schema, property, "markdownDescription")
                        .or_else(|| plain(schema, property, "description"))
                        .or_else(empty_documentation);
                    let sort_text = field(schema, property, "suggestSortText").and_then(|index| {
                        match schema.get(index)? {
                            Value::String(text) => Some(String::from_utf16_lossy(text)),
                            _ => None,
                        }
                    });
                    self.property_item(
                        &member.key,
                        Some(property),
                        add_value,
                        separator,
                        documentation,
                        sort_text,
                    )?;
                }
            }
            let Some(names) = field(schema, current, "propertyNames") else {
                continue;
            };
            if !is_object(schema, names)
                || truthy(schema, names, "deprecationMessage")
                || truthy(schema, names, "doNotSuggest")
            {
                continue;
            }
            let fallback = markdown(schema, names, "markdownDescription")
                .or_else(|| plain(schema, names, "description"))
                .or_else(empty_documentation);
            let sort_text = field(schema, names, "suggestSortText").and_then(|index| match schema
                .get(index)?
            {
                Value::String(text) => Some(String::from_utf16_lossy(text)),
                _ => None,
            });
            if let Some(values) = items(schema, names, "enum") {
                let markdown_descriptions = items(schema, names, "markdownEnumDescriptions");
                let descriptions = items(schema, names, "enumDescriptions");
                for (index, value) in values.iter().enumerate() {
                    let Some(Value::String(name)) = schema.get(*value) else {
                        continue;
                    };
                    let description = match (markdown_descriptions, descriptions) {
                        (Some(markdown), _) if index < markdown.len() => {
                            match schema.get(markdown[index]) {
                                Some(Value::String(text)) if !text.is_empty() => {
                                    Some(Documentation::MarkupContent(MarkupContent {
                                        kind: MarkupKind::Markdown,
                                        value: String::from_utf16_lossy(text),
                                    }))
                                }
                                _ => None,
                            }
                        }
                        (_, Some(plain)) if index < plain.len() => match schema.get(plain[index]) {
                            Some(Value::String(text)) if !text.is_empty() => {
                                Some(Documentation::String(String::from_utf16_lossy(text)))
                            }
                            _ => None,
                        },
                        _ => None,
                    };
                    let name = name.clone();
                    self.property_item(
                        &name,
                        None,
                        add_value,
                        separator,
                        description.or_else(|| fallback.clone()),
                        sort_text.clone(),
                    )?;
                }
            }
            if let Some(index) = field(schema, names, "const")
                && let Some(Value::String(name)) = schema.get(index)
                && !name.is_empty()
            {
                let name = name.clone();
                self.property_item(&name, None, add_value, separator, fallback, sort_text)?;
            }
        }
        Ok(())
    }

    fn schemaless_property_completions(&mut self, object: usize) -> Result<(), EditorError> {
        let document = self.document;
        let mut similar = Vec::new();
        match document.nodes[object].parent {
            Some(parent) if document.nodes[parent].kind == Kind::Property => {
                let parent_key = key(document, parent);
                for node in preorder(document) {
                    self.budget.spend(1)?;
                    if document.nodes[node].kind == Kind::Property
                        && node != parent
                        && key(document, node) == parent_key
                        && let Some(value) = property_value(document, node)
                        && document.nodes[value].kind == Kind::Object
                    {
                        similar.push(value);
                    }
                }
            }
            Some(parent) if document.nodes[parent].kind == Kind::Array => {
                similar.extend(
                    document.nodes[parent]
                        .children
                        .iter()
                        .copied()
                        .filter(|item| {
                            document.nodes[*item].kind == Kind::Object && *item != object
                        }),
                );
            }
            Some(_) => {}
            None => {
                if document.nodes[object].kind == Kind::Object {
                    let name = "$schema".encode_utf16().collect::<Vec<_>>();
                    let insert_text = self.property_insert(&name, None, true, "")?;
                    let mut item = self.item(
                        CompletionItemKind::PROPERTY,
                        "$schema".into(),
                        insert_text,
                        empty_documentation(),
                    );
                    item.filter_text = Some(self.budget.quote(&name)?);
                    self.add(item)?;
                }
            }
        }
        for similar in similar {
            for &property in &document.nodes[similar].children {
                let name = key(document, property);
                let insert_text = escape(&self.budget.quote(name)?);
                let mut item = self.item(
                    CompletionItemKind::PROPERTY,
                    String::from_utf16_lossy(name),
                    insert_text,
                    empty_documentation(),
                );
                item.filter_text = Some(self.budget.quote(name)?);
                self.add(item)?;
            }
        }
        Ok(())
    }

    fn boolean(&mut self, value: bool, separator: &str) -> Result<(), EditorError> {
        let item = self.item(
            CompletionItemKind::VALUE,
            value.to_string(),
            escape(&format!("{value}{separator}")),
            empty_documentation(),
        );
        self.add(item)
    }

    fn null(&mut self, separator: &str) -> Result<(), EditorError> {
        let item = self.item(
            CompletionItemKind::VALUE,
            "null".into(),
            format!("null{separator}"),
            empty_documentation(),
        );
        self.add(item)
    }

    fn dollar_schemas(&mut self, separator: &str) -> Result<(), EditorError> {
        for id in self.registered {
            let mut id = String::from_utf16_lossy(id);
            if id.starts_with(DRAFT_PREFIX) {
                id.push('#');
            }
            let units = id.encode_utf16().collect::<Vec<_>>();
            let label = self.budget.quote(&units)?;
            let insert_text = escape(&format!("{label}{separator}"));
            let mut item = self.item(
                CompletionItemKind::MODULE,
                label.clone(),
                insert_text,
                empty_documentation(),
            );
            item.filter_text = Some(label);
            self.add(item)?;
        }
        Ok(())
    }

    fn schemaless_value_completions(
        &mut self,
        node: Option<usize>,
        offset: usize,
    ) -> Result<(), EditorError> {
        let document = self.document;
        let mut separator_offset = offset;
        let mut node = node;
        if let Some(scalar) = node
            && is_scalar(document.nodes[scalar].kind)
        {
            separator_offset = document.nodes[scalar].bytes.end;
            node = document.nodes[scalar].parent;
        }
        let Some(node) = node else {
            let object = self.item(
                CompletionItemKind::MODULE,
                "Empty object".into(),
                "{$1}".into(),
                empty_documentation(),
            );
            self.add(object)?;
            let array = self.item(
                CompletionItemKind::VALUE,
                "Empty array".into(),
                "[$1]".into(),
                empty_documentation(),
            );
            return self.add(array);
        };
        let separator = separator_after(self.input, separator_offset, self.scanner())?;
        let mut values = Vec::new();
        let current = &document.nodes[node];
        if current.kind == Kind::Property && offset > current.colon_byte.unwrap_or(0) {
            if let Some(value) = property_value(document, node)
                && (offset > document.nodes[value].bytes.end
                    || matches!(document.nodes[value].kind, Kind::Object | Kind::Array))
            {
                return Ok(());
            }
            let parent_key = key(document, node);
            for other in preorder(document) {
                self.budget.spend(1)?;
                if document.nodes[other].kind == Kind::Property
                    && key(document, other) == parent_key
                    && let Some(value) = property_value(document, other)
                {
                    values.push(value);
                }
            }
            self.matching_values(&values, offset, separator)?;
            if parent_key.iter().copied().eq("$schema".encode_utf16())
                && current
                    .parent
                    .is_some_and(|parent| document.nodes[parent].parent.is_none())
            {
                self.dollar_schemas(separator)?;
            }
        }
        if current.kind == Kind::Array {
            match current.parent {
                Some(parent) if document.nodes[parent].kind == Kind::Property => {
                    let parent_key = key(document, parent);
                    for other in preorder(document) {
                        self.budget.spend(1)?;
                        if document.nodes[other].kind == Kind::Property
                            && key(document, other) == parent_key
                            && let Some(value) = property_value(document, other)
                            && document.nodes[value].kind == Kind::Array
                        {
                            values.extend(&document.nodes[value].children);
                        }
                    }
                }
                _ => values.extend(&current.children),
            }
            self.matching_values(&values, offset, separator)?;
        }
        Ok(())
    }

    fn matching_values(
        &mut self,
        values: &[usize],
        offset: usize,
        separator: &str,
    ) -> Result<(), EditorError> {
        let document = self.document;
        for &value in values {
            let node = &document.nodes[value];
            if node
                .parent
                .is_some_and(|parent| !contains(document, parent, offset))
            {
                let (label, insert_text) = match node.kind {
                    Kind::Array => ("[]".to_owned(), format!("[$1]{separator}")),
                    Kind::Object => ("{}".to_owned(), format!("{{$1}}{separator}")),
                    _ => {
                        let content = &self.input[node.bytes.clone()];
                        (content.to_owned(), escape(&format!("{content}{separator}")))
                    }
                };
                let item = self.item(
                    node_kind(node.kind),
                    label,
                    insert_text,
                    empty_documentation(),
                );
                self.add(item)?;
            }
            if let NodeValue::Boolean(value) = node.value
                && node.kind == Kind::Boolean
            {
                self.boolean(!value, separator)?;
            }
        }
        Ok(())
    }

    fn value_completions(
        &mut self,
        schema: &Arena,
        node: Option<usize>,
        offset: usize,
        types: &mut HashSet<String>,
    ) -> Result<(), EditorError> {
        let document = self.document;
        let Some(root) = schema.root() else {
            return Ok(());
        };
        let mut separator_offset = offset;
        let mut excluded = None;
        let mut node = node;
        if let Some(scalar) = node
            && is_scalar(document.nodes[scalar].kind)
        {
            separator_offset = document.nodes[scalar].bytes.end;
            excluded = Some(scalar);
            node = document.nodes[scalar].parent;
        }
        let Some(mut node) = node else {
            return self.schema_values(schema, root, "", types, 0);
        };
        let mut parent_key = None;
        if document.nodes[node].kind == Kind::Property
            && offset > document.nodes[node].colon_byte.unwrap_or(0)
        {
            if property_value(document, node)
                .is_some_and(|value| offset > document.nodes[value].bytes.end)
            {
                return Ok(());
            }
            parent_key = Some(key(document, node).to_vec());
            let Some(parent) = document.nodes[node].parent else {
                return Ok(());
            };
            node = parent;
        }
        if parent_key.is_none() && document.nodes[node].kind != Kind::Array {
            return Ok(());
        }
        let separator = separator_after(self.input, separator_offset, self.scanner())?;
        let matching = json_schema_evaluation::matching_schemas(
            document,
            schema,
            root,
            self.patterns,
            Selection {
                focus_byte: Some(document.nodes[node].bytes.start),
                exclude_node: excluded,
            },
            self.limits.document.policy.evaluation,
        )?;
        for matched in matching {
            if matched.node != node || matched.inverted || matched.view != SchemaView::Source {
                continue;
            }
            let current = matched.schema;
            if document.nodes[node].kind == Kind::Array && truthy(schema, current, "items") {
                if truthy(schema, current, "uniqueItems") {
                    let mut existing = HashSet::new();
                    for &child in &document.nodes[node].children {
                        if let Some(label) = self.node_label(child)? {
                            existing.insert(label);
                        }
                    }
                    self.unique = Some(existing);
                }
                let result = match items(schema, current, "items") {
                    Some(tuple) => {
                        let index = self.item_index(node, offset)?;
                        match tuple.get(index) {
                            Some(item) => self.schema_values(schema, *item, separator, types, 0),
                            None => Ok(()),
                        }
                    }
                    None => match field(schema, current, "items") {
                        Some(item) => self.schema_values(schema, item, separator, types, 0),
                        None => Ok(()),
                    },
                };
                self.unique = None;
                result?;
            }
            let Some(parent_key) = &parent_key else {
                continue;
            };
            let mut matched_property = false;
            if truthy(schema, current, "properties")
                && let Some(properties) = field(schema, current, "properties")
                && let Some(property) = schema.member(properties, parent_key)
                && schema.get(property).is_some_and(Value::is_truthy)
            {
                matched_property = true;
                self.schema_values(schema, property, separator, types, 0)?;
            }
            if truthy(schema, current, "patternProperties")
                && !matched_property
                && let Some(index) = field(schema, current, "patternProperties")
                && let Some(Value::Object(patterns)) = schema.get(index)
            {
                for pattern in patterns {
                    let regex =
                        compile_extended(self.patterns, &pattern.key, &mut self.budget.work)?;
                    self.budget.spend(parent_key.len())?;
                    if regex.is_some_and(|regex| regex.is_match(parent_key)) {
                        matched_property = true;
                        self.schema_values(schema, pattern.value, separator, types, 0)?;
                    }
                }
            }
            if truthy(schema, current, "additionalProperties")
                && !matched_property
                && let Some(additional) = field(schema, current, "additionalProperties")
            {
                self.schema_values(schema, additional, separator, types, 0)?;
            }
        }
        if parent_key
            .as_deref()
            .is_some_and(|parent_key| parent_key.iter().copied().eq("$schema".encode_utf16()))
            && document.nodes[node].parent.is_none()
        {
            self.dollar_schemas(separator)?;
        }
        if types.contains("boolean") {
            self.boolean(true, separator)?;
            self.boolean(false, separator)?;
        }
        if types.contains("null") {
            self.null(separator)?;
        }
        Ok(())
    }

    fn node_label(&mut self, node: usize) -> Result<Option<String>, EditorError> {
        let node = &self.document.nodes[node];
        Ok(match &node.value {
            _ if matches!(node.kind, Kind::Array | Kind::Object | Kind::Property) => None,
            NodeValue::String(value) => Some(self.budget.quote(value)?),
            NodeValue::Number(value) if value.is_finite() => {
                Some(crate::json_symbols::number_string(*value))
            }
            NodeValue::Number(_) | NodeValue::Null => Some("null".into()),
            NodeValue::Boolean(value) => Some(value.to_string()),
            NodeValue::None => None,
        })
    }

    fn item_index(&mut self, array: usize, offset: usize) -> Result<usize, EditorError> {
        let children = &self.document.nodes[array].children;
        for (index, &child) in children.iter().enumerate().rev() {
            self.budget.spend(1)?;
            let bytes = &self.document.nodes[child].bytes;
            if offset > bytes.end {
                let (kind, token) = next_token(self.input, bytes.end, self.scanner())?;
                if kind == TokenKind::Comma && offset >= token.end {
                    return Ok(index + 1);
                }
                return Ok(index);
            }
            if offset >= bytes.start {
                return Ok(index);
            }
        }
        Ok(0)
    }

    fn schema_values(
        &mut self,
        schema: &Arena,
        current: usize,
        separator: &str,
        types: &mut HashSet<String>,
        depth: usize,
    ) -> Result<(), EditorError> {
        self.budget.spend(1)?;
        if depth > self.limits.max_depth {
            return Err(EditorError::Capacity);
        }
        if !is_object(schema, current) {
            return Ok(());
        }
        self.enum_values(schema, current, separator)?;
        self.default_values(schema, current, separator, 0)?;
        if items(schema, current, "enum").is_none() && field(schema, current, "const").is_none() {
            match field(schema, current, "type").and_then(|index| schema.get(index)) {
                Some(Value::Array(declared)) => {
                    for declared in declared {
                        if let Some(Value::String(name)) = schema.get(*declared) {
                            types.insert(String::from_utf16_lossy(name));
                        }
                    }
                }
                Some(Value::String(name)) if !name.is_empty() => {
                    types.insert(String::from_utf16_lossy(name));
                }
                _ => {}
            }
        }
        for combinator in ["allOf", "anyOf", "oneOf"] {
            for &branch in items(schema, current, combinator).unwrap_or_default() {
                self.schema_values(schema, branch, separator, types, depth + 1)?;
            }
        }
        Ok(())
    }

    fn enum_values(
        &mut self,
        schema: &Arena,
        current: usize,
        separator: &str,
    ) -> Result<(), EditorError> {
        let description = markdown(schema, current, "markdownDescription")
            .or_else(|| plain(schema, current, "description"));
        if let Some(constant) = field(schema, current, "const") {
            let label = self.budget.compact(schema, constant)?;
            let insert_text = self.budget.value_insert(schema, constant, 0, separator)?;
            let item = self.item(
                type_kind(schema, current),
                label,
                insert_text,
                description.clone(),
            );
            self.add(item)?;
        }
        let Some(values) = items(schema, current, "enum") else {
            return Ok(());
        };
        let markdown_descriptions = items(schema, current, "markdownEnumDescriptions");
        let descriptions = items(schema, current, "enumDescriptions");
        for (index, &value) in values.iter().enumerate() {
            let documentation = match (markdown_descriptions, descriptions) {
                (Some(markdown), _) if index < markdown.len() => {
                    match schema.get(markdown[index]) {
                        Some(Value::String(text)) if !text.is_empty() => {
                            Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: String::from_utf16_lossy(text),
                            }))
                        }
                        _ => None,
                    }
                }
                (_, Some(plain)) if index < plain.len() => match schema.get(plain[index]) {
                    Some(Value::String(text)) => {
                        Some(Documentation::String(String::from_utf16_lossy(text)))
                    }
                    _ => None,
                },
                _ => description.clone(),
            };
            let label = self.budget.compact(schema, value)?;
            let insert_text = self.budget.value_insert(schema, value, 0, separator)?;
            let item = self.item(
                type_kind(schema, current),
                label,
                insert_text,
                documentation,
            );
            self.add(item)?;
        }
        Ok(())
    }

    fn default_values(
        &mut self,
        schema: &Arena,
        current: usize,
        separator: &str,
        wraps: usize,
    ) -> Result<(), EditorError> {
        self.budget.spend(1)?;
        let mut proposed = false;
        let kind = if wraps > 0 {
            CompletionItemKind::VALUE
        } else {
            type_kind(schema, current)
        };
        let mut values = field(schema, current, "default")
            .into_iter()
            .collect::<Vec<_>>();
        values.extend(items(schema, current, "examples").unwrap_or_default());
        for value in values {
            let label = self.budget.compact(schema, value)?;
            let label =
                self.budget
                    .keep(format!("{}{label}{}", "[".repeat(wraps), "]".repeat(wraps)))?;
            let insert_text = self.budget.value_insert(schema, value, wraps, separator)?;
            let item = self.item(kind, label, insert_text, None);
            self.add(item)?;
            proposed = true;
        }
        for &snippet in items(schema, current, "defaultSnippets").unwrap_or_default() {
            if !is_object(schema, snippet) {
                continue;
            }
            let label = text(schema, snippet, "label").map(String::from_utf16_lossy);
            let (kind, label, insert_text, filter_text) =
                if let Some(body) = field(schema, snippet, "body") {
                    let compact = self.budget.wrapped_compact(schema, body, wraps)?;
                    let insert_text = format!(
                        "{}{separator}",
                        self.budget.snippet_insert(schema, body, wraps)?
                    );
                    let label = label.unwrap_or_else(|| strip_placeholders(&compact));
                    (
                        type_kind(schema, current),
                        label,
                        insert_text,
                        strip_placeholders(&compact),
                    )
                } else if let Some(index) = field(schema, snippet, "bodyText")
                    && let Some(Value::String(body)) = schema.get(index)
                {
                    let mut prefix = String::new();
                    let mut suffix = String::new();
                    let mut indent = String::new();
                    for _ in 0..wraps {
                        prefix = format!("{prefix}{indent}[\n");
                        suffix = format!("{suffix}\n{indent}]");
                        indent.push('\t');
                    }
                    let body = String::from_utf16_lossy(body).replace('\n', &format!("\n{indent}"));
                    let insert_text = self
                        .budget
                        .keep(format!("{prefix}{indent}{body}{suffix}{separator}"))?;
                    let label = label.unwrap_or_else(|| insert_text.clone());
                    (
                        kind,
                        label,
                        insert_text.clone(),
                        insert_text.replace('\n', ""),
                    )
                } else {
                    continue;
                };
            let documentation = markdown(schema, snippet, "markdownDescription")
                .or_else(|| plain(schema, snippet, "description"));
            let mut item = self.item(kind, label, insert_text, documentation);
            item.filter_text = Some(filter_text);
            self.add(item)?;
            proposed = true;
        }
        if !proposed
            && wraps < MAX_DEFAULT_ARRAY_DEPTH
            && let Some(items) = field(schema, current, "items")
            && is_object(schema, items)
        {
            self.default_values(schema, items, separator, wraps + 1)?;
        }
        Ok(())
    }

    fn filler(&mut self, types: &HashSet<String>, separator: &str) -> Result<(), EditorError> {
        for (name, label, insert_text, detail, kind) in [
            (
                "object",
                "{}",
                format!("{{$1}}{separator}"),
                NEW_OBJECT,
                CompletionItemKind::MODULE,
            ),
            (
                "array",
                "[]",
                format!("[$1]{separator}"),
                NEW_ARRAY,
                CompletionItemKind::VALUE,
            ),
        ] {
            if types.contains(name) {
                let mut item = self.item(kind, label.into(), insert_text, empty_documentation());
                item.detail = Some(detail.into());
                self.add(item)?;
            }
        }
        Ok(())
    }
}

/// Runs the language service `doComplete` for one document at `byte` without completion
/// contributions or a schema request service: bundled schemas and `$schema` drive schema
/// proposals, and documents without a schema get the original schema-less proposals.
pub fn complete(
    input: &str,
    byte: usize,
    environment: Environment<'_>,
    limits: Limits,
) -> Result<Completion, EditorError> {
    let Environment {
        resource,
        contributions,
        associations,
        patterns,
        cancelled,
    } = environment;
    if byte > input.len() || !input.is_char_boundary(byte) {
        return Err(EditorError::InvalidBoundary);
    }
    let scanner = scanner_limits(&limits.document);
    let document = json_parser::parse(input, true, limits.document.policy.syntax.parser)?;
    let mut node = document.node_at_byte(byte, true);
    let start = node.map_or(0, |node| document.nodes[node].bytes.start);
    if in_comment(input, start, byte, scanner)? {
        return Ok(Completion::default());
    }
    if let Some(current) = node
        && byte == document.nodes[current].bytes.end
        && byte > 0
    {
        let previous = input.as_bytes()[byte - 1];
        let kind = document.nodes[current].kind;
        if kind == Kind::Object && previous == b'}' || kind == Kind::Array && previous == b']' {
            node = document.nodes[current].parent;
        }
    }
    let word = current_word(input, byte);
    let range = match node {
        Some(current) if is_scalar(document.nodes[current].kind) => {
            document.nodes[current].bytes.clone()
        }
        _ => {
            let mut start = word.start;
            if start > 0 && input.as_bytes()[start - 1] == b'"' {
                start -= 1;
            }
            start..byte
        }
    };
    let schema = json_schema_document::document_schema(
        &document,
        resource,
        contributions,
        associations,
        limits.document,
        cancelled,
    )?;
    let arena = schema.resolved.as_deref().map(|resolved| &resolved.schema);
    let mut completer = Completer {
        input,
        document: &document,
        schema: arena,
        registered: &schema.registered,
        patterns,
        limits,
        budget: Budget::new(limits.max_work, limits.max_text_bytes, limits.max_depth),
        range,
        items: Vec::new(),
        proposed: HashMap::new(),
        unique: None,
        incomplete: false,
    };
    let mut add_value = true;
    let mut current_property = None;
    if let Some(current) = node
        && document.nodes[current].kind == Kind::String
        && let Some(parent) = document.nodes[current].parent
        && document.nodes[parent].kind == Kind::Property
        && document.nodes[parent].children.first() == Some(&current)
    {
        add_value = property_value(&document, parent).is_none();
        current_property = Some(parent);
        node = document.nodes[parent].parent;
    }
    if let Some(object) = node
        && document.nodes[object].kind == Kind::Object
    {
        if document.nodes[object].bytes.start == byte {
            return Ok(Completion::default());
        }
        for &property in &document.nodes[object].children {
            if Some(property) != current_property {
                completer
                    .proposed
                    .insert(String::from_utf16_lossy(key(&document, property)), None);
            }
        }
        let separator = if add_value {
            separator_after(input, completer.range.end, scanner)?
        } else {
            ""
        };
        match arena {
            Some(schema) => completer.property_completions(schema, object, add_value, separator)?,
            None => completer.schemaless_property_completions(object)?,
        }
        if arena.is_none()
            && !word.is_empty()
            && (word.start == 0 || input.as_bytes()[word.start - 1] != b'"')
        {
            let current_word = input[word.clone()].encode_utf16().collect::<Vec<_>>();
            let label = completer.budget.quote(&current_word)?;
            let insert_text = completer.property_insert(&current_word, None, false, separator)?;
            let item = completer.item(
                CompletionItemKind::PROPERTY,
                label,
                insert_text,
                empty_documentation(),
            );
            completer.add(item)?;
            completer.incomplete = true;
        }
    }
    let mut types = HashSet::new();
    match arena {
        Some(schema) => completer.value_completions(schema, node, byte, &mut types)?,
        None => completer.schemaless_value_completions(node, byte)?,
    }
    if completer.items.is_empty() {
        let offset = match node {
            Some(current) if is_scalar(document.nodes[current].kind) => {
                document.nodes[current].bytes.end
            }
            _ => byte,
        };
        let separator = separator_after(input, offset, scanner)?;
        completer.filler(&types, separator)?;
    }
    if cancelled() {
        return Err(EditorError::StaleRevision);
    }
    Ok(Completion {
        items: completer.items,
        is_incomplete: completer.incomplete,
    })
}
