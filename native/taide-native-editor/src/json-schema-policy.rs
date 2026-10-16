use std::collections::BTreeSet;

use crate::diagnostics::Marker;
use crate::document::EditorError;
use crate::json_parser::{self, Kind, Value as AstValue};
use crate::json_schema_evaluation::{self, Draft};
use crate::json_schema_refs::Resolved;
use crate::json_schema_string::PatternCompiler;
use crate::json_validation::{self, Collector, Level, Settings};
use crate::json_value::{Arena, Value};

pub const SCHEMA_RESOLVE_ERROR: u32 = 768;
pub const SCHEMA_UNSUPPORTED_FEATURE: u32 = 769;
const SCHEMA_KEY: &str = "$schema";
const UNSUPPORTED_PREFIX: &str = "The schema uses meta-schema features (";
const UNSUPPORTED_SUFFIX: &str = ") that are not yet supported by the validator.";
const UNSUPPORTED_SEPARATOR: &str = ", ";
const ALLOW_COMMENTS: &[&str] = &["allowComments"];
const ALLOW_TRAILING_COMMAS: &[&str] = &["allowTrailingCommas", "allowsTrailingCommas"];

/// Document-level schema diagnostic levels; `None` keeps the language service default.
#[derive(Clone, Copy, Debug, Default)]
pub struct Policy {
    pub schema_validation: Option<Level>,
    pub schema_request: Option<Level>,
    pub schema_draft: Option<Draft>,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub syntax: json_validation::Limits,
    pub evaluation: json_schema_evaluation::Limits,
    pub max_schema_work: usize,
}

/// Returns the first boolean comment permission found in the schema or its `allOf` chain.
pub fn allows_comments(schema: &Arena, max_work: usize) -> Result<Option<bool>, EditorError> {
    allows(schema, ALLOW_COMMENTS, max_work)
}

/// Returns the first boolean trailing comma permission found in the schema or its `allOf` chain.
pub fn allows_trailing_commas(
    schema: &Arena,
    max_work: usize,
) -> Result<Option<bool>, EditorError> {
    allows(schema, ALLOW_TRAILING_COMMAS, max_work)
}

fn allows(schema: &Arena, fields: &[&str], max_work: usize) -> Result<Option<bool>, EditorError> {
    let Some(root) = schema.root() else {
        return Ok(None);
    };
    let fields = fields
        .iter()
        .map(|field| field.encode_utf16().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let all_of = "allOf".encode_utf16().collect::<Vec<_>>();
    let mut pending = vec![root];
    let mut visited = BTreeSet::new();
    let mut work = 0usize;
    while let Some(index) = pending.pop() {
        work = work.checked_add(1).ok_or(EditorError::Capacity)?;
        if work > max_work {
            return Err(EditorError::Capacity);
        }
        if !visited.insert(index) {
            continue;
        }
        if !matches!(schema.get(index), Some(Value::Object(_))) {
            continue;
        }
        for field in &fields {
            if let Some(Value::Boolean(value)) = schema
                .member(index, field)
                .and_then(|value| schema.get(value))
            {
                return Ok(Some(*value));
            }
        }
        if let Some(Value::Array(items)) = schema
            .member(index, &all_of)
            .and_then(|value| schema.get(value))
        {
            work = work.checked_add(items.len()).ok_or(EditorError::Capacity)?;
            if work > max_work {
                return Err(EditorError::Capacity);
            }
            pending.extend(items.iter().rev());
        }
    }
    Ok(None)
}

fn schema_problem_range(document: &json_parser::Document) -> Option<std::ops::Range<usize>> {
    let root = &document.nodes[document.root?];
    let property = (root.kind == Kind::Object)
        .then(|| root.children.first().copied())
        .flatten()
        .map(|property| &document.nodes[property]);
    if let Some(property) = property {
        let is_schema =
            property
                .children
                .first()
                .is_some_and(|&key| match &document.nodes[key].value {
                    AstValue::String(value) => value.iter().copied().eq(SCHEMA_KEY.encode_utf16()),
                    _ => false,
                });
        if is_schema {
            let node = property
                .children
                .get(1)
                .map_or(property, |&value| &document.nodes[value]);
            return Some(node.bytes.clone());
        }
    }
    Some(root.bytes.start..root.bytes.start + 1)
}

/// Produces the language service document diagnostics: schema problems, schema semantic
/// diagnostics, syntax problems and comments in the original order without duplicates.
pub fn diagnostics(
    input: &str,
    language: &str,
    settings: Settings,
    policy: Policy,
    schema: Option<&Resolved>,
    compiler: &dyn PatternCompiler,
    limits: Limits,
) -> Result<Vec<Marker>, EditorError> {
    if !settings.validate {
        return Ok(Vec::new());
    }
    let document = json_parser::parse(input, true, limits.syntax.parser)?;
    let mut collector = Collector::new(
        limits.syntax.max_diagnostics,
        limits.syntax.max_message_bytes,
    );
    let mut settings = settings;
    if let Some(resolved) = schema {
        let request = policy.schema_request.unwrap_or(Level::Warning).severity();
        let validation = policy
            .schema_validation
            .unwrap_or(Level::Warning)
            .severity();
        let add_schema_problem =
            |collector: &mut Collector, message: Vec<u16>, code: u32| -> Result<(), EditorError> {
                let (Some(severity), Some(range)) = (request, schema_problem_range(&document))
                else {
                    return Ok(());
                };
                collector.add(range, message, Some(code), severity, None)
            };
        if let Some(error) = resolved.errors.first() {
            add_schema_problem(&mut collector, error.clone(), SCHEMA_RESOLVE_ERROR)?;
        } else if let Some(severity) = validation {
            if !resolved.unsupported.is_empty() {
                let mut message = UNSUPPORTED_PREFIX.encode_utf16().collect::<Vec<_>>();
                for (index, feature) in resolved.unsupported.iter().enumerate() {
                    if index > 0 {
                        message.extend(UNSUPPORTED_SEPARATOR.encode_utf16());
                    }
                    message.extend(feature.encode_utf16());
                }
                message.extend(UNSUPPORTED_SUFFIX.encode_utf16());
                add_schema_problem(&mut collector, message, SCHEMA_UNSUPPORTED_FEATURE)?;
            }
            if let Some(root) = resolved.schema.root() {
                let validation = json_schema_evaluation::validate(
                    &document,
                    &resolved.schema,
                    root,
                    compiler,
                    severity,
                    policy.schema_draft,
                    limits.evaluation,
                )?;
                for diagnostic in validation.diagnostics {
                    collector.add(
                        diagnostic.bytes,
                        diagnostic.message,
                        diagnostic.code,
                        diagnostic.severity,
                        None,
                    )?;
                }
            }
        }
        if allows_comments(&resolved.schema, limits.max_schema_work)? == Some(true) {
            settings.schema_allows_comments = true;
        }
        if allows_trailing_commas(&resolved.schema, limits.max_schema_work)? == Some(true) {
            settings.schema_allows_trailing_commas = true;
        }
    }
    json_validation::collect_syntax(&document, language, settings, &mut collector)?;
    Ok(collector.markers)
}
