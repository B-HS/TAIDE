use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::diagnostics::Severity;
use crate::document::EditorError;
use crate::json_parser::Document;
use crate::json_schema_scalar::{self, Diagnostic, Phase, Target};
use crate::json_schema_string::PatternCompiler;
use crate::json_value::{self, Arena, Value};
use crate::json_value_text;

const ENUM_MISMATCH: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Draft {
    Draft3,
    Draft4,
    Draft6,
    Draft7,
    Draft2019,
    Draft2020,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub values: json_value::Limits,
    pub scalar: json_schema_scalar::Limits,
    pub max_work: usize,
    pub max_frames: usize,
    pub max_diagnostics: usize,
    pub max_message_units: usize,
    pub max_matches: usize,
    pub max_processed: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchingSchema {
    pub node: usize,
    pub schema: usize,
    pub inverted: bool,
    pub view: SchemaView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaView {
    Source,
    Empty,
    Reject,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Selection {
    pub focus_byte: Option<usize>,
    pub exclude_node: Option<usize>,
}

#[derive(Clone, Copy)]
pub(crate) struct Collection {
    pub enabled: bool,
    selection: Selection,
}

impl Collection {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            selection: Selection::default(),
        }
    }
    pub fn sub(self) -> Self {
        Self {
            selection: Selection {
                focus_byte: None,
                ..self.selection
            },
            ..self
        }
    }
}

#[derive(Debug, Default)]
pub struct Validation {
    pub diagnostics: Vec<Diagnostic>,
    pub schemas: Vec<MatchingSchema>,
    pub processed_properties: BTreeSet<Vec<u16>>,
    pub properties_matches: usize,
    pub properties_value_matches: usize,
    pub primary_value_matches: usize,
    pub enum_value_match: bool,
    pub enum_values: Option<Vec<usize>>,
}

impl Validation {
    fn compare(&self, other: &Self) -> Ordering {
        self.diagnostics
            .is_empty()
            .cmp(&other.diagnostics.is_empty())
            .then(self.enum_value_match.cmp(&other.enum_value_match))
            .then(self.primary_value_matches.cmp(&other.primary_value_matches))
            .then(
                self.properties_value_matches
                    .cmp(&other.properties_value_matches),
            )
            .then(self.properties_matches.cmp(&other.properties_matches))
    }
}

pub(crate) struct Alternatives {
    pub schemas: usize,
    pub index: usize,
    pub best: Option<Box<Validation>>,
    pub matches: usize,
    pub one: bool,
}
pub(crate) struct Contains {
    pub schema: usize,
    pub index: usize,
    pub matches: usize,
}

pub(crate) enum Merge {
    All,
    Property(Option<Vec<u16>>),
    Direct,
    RawProperty,
    Not,
    Condition {
        then_schema: Option<usize>,
        else_schema: Option<usize>,
    },
    Alternative(Alternatives),
    Contains(Contains),
}

pub(crate) enum Operation {
    Finish,
    Scalar(Phase),
    AllOf,
    Not,
    AnyOf,
    OneOf,
    Condition,
    Structure,
    Child {
        node: usize,
        schema: usize,
        collect: Collection,
        merge: Merge,
    },
    Merge(Merge),
    Alternatives(Alternatives),
    Contains(Contains),
    UnevaluatedArray,
    ArrayBounds,
    Mark {
        name: Vec<u16>,
        value_match: bool,
    },
    PatternProperties(usize),
    AdditionalProperties(bool),
    ObjectBounds,
    Dependencies(&'static str),
    PropertyNames,
    Problem(Diagnostic),
}

pub(crate) struct Property {
    pub name: Vec<u16>,
    pub value: Option<usize>,
    pub key_node: usize,
    pub processed: bool,
}

pub(crate) struct Frame {
    pub node: usize,
    pub schema: usize,
    pub collect: Collection,
    view: SchemaView,
    is_included: bool,
    pub result: Validation,
    pub pending: Vec<Operation>,
    pub properties: Vec<Property>,
}

pub(crate) struct Evaluator<'a> {
    pub document: &'a Document,
    pub values: &'a Arena,
    pub mapped: &'a [Option<usize>],
    pub schema: &'a Arena,
    pub compiler: &'a dyn PatternCompiler,
    pub severity: Severity,
    pub draft: Draft,
    pub limits: Limits,
    pub remaining: usize,
    active: BTreeSet<(usize, usize)>,
    diagnostic_count: usize,
    message_units: usize,
    match_count: usize,
    processed_count: usize,
}

impl Evaluator<'_> {
    pub fn spend(&mut self, count: usize) -> Result<(), EditorError> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or(EditorError::Capacity)?;
        Ok(())
    }

    pub fn field(&mut self, schema: usize, name: &str) -> Result<Option<usize>, EditorError> {
        self.spend(1)?;
        let Some(Value::Object(members)) = self.schema.get(schema) else {
            return Ok(None);
        };
        for member in members {
            self.spend(member.key.len() + 1)?;
            if member.key.iter().copied().eq(name.encode_utf16()) {
                return Ok(Some(member.value));
            }
        }
        Ok(None)
    }

    pub fn number(&mut self, schema: usize, name: &str) -> Result<Option<f64>, EditorError> {
        Ok(self
            .field(schema, name)?
            .and_then(|index| match self.schema.get(index) {
                Some(Value::Number(value)) => Some(*value),
                _ => None,
            }))
    }

    pub fn schema_ref(&self, index: Option<usize>) -> Result<Option<usize>, EditorError> {
        let Some(index) = index else {
            return Ok(None);
        };
        match self.schema.get(index).ok_or(EditorError::InvalidIdentity)? {
            Value::Null => Ok(None),
            Value::Boolean(_) | Value::Object(_) => Ok(Some(index)),
            value if !value.is_truthy() => Ok(None),
            _ => Err(EditorError::InvalidBoundary),
        }
    }

    pub fn queue(
        &mut self,
        frame: &mut Frame,
        operations: Vec<Operation>,
    ) -> Result<(), EditorError> {
        self.spend(operations.len())?;
        if frame
            .pending
            .len()
            .checked_add(operations.len())
            .is_none_or(|size| size > self.remaining)
        {
            return Err(EditorError::Capacity);
        }
        frame.pending.extend(operations.into_iter().rev());
        Ok(())
    }

    fn frame(
        &mut self,
        node: usize,
        schema: usize,
        collect: Collection,
        normalize_boolean: bool,
    ) -> Result<Frame, EditorError> {
        self.spend(1)?;
        let ast = self
            .document
            .nodes
            .get(node)
            .ok_or(EditorError::InvalidIdentity)?;
        let is_included = !collect.enabled
            || collect
                .selection
                .focus_byte
                .is_none_or(|byte| ast.bytes.contains(&byte))
                && collect.selection.exclude_node != Some(node);
        if is_included && !self.active.insert((node, schema)) {
            return Err(EditorError::Capacity);
        }
        match self.schema.get(schema) {
            Some(Value::Boolean(_) | Value::Object(_)) => {}
            _ => return Err(EditorError::InvalidBoundary),
        }
        let view = match (normalize_boolean, self.schema.get(schema)) {
            (true, Some(Value::Boolean(true))) => SchemaView::Empty,
            (true, Some(Value::Boolean(false))) => SchemaView::Reject,
            _ => SchemaView::Source,
        };
        Ok(Frame {
            node,
            schema,
            collect,
            view,
            is_included,
            result: Validation::default(),
            properties: Vec::new(),
            pending: if !is_included {
                vec![Operation::Finish]
            } else {
                vec![
                    Operation::Finish,
                    Operation::Structure,
                    Operation::Scalar(Phase::Values),
                    Operation::Condition,
                    Operation::OneOf,
                    Operation::AnyOf,
                    Operation::Not,
                    Operation::AllOf,
                    Operation::Scalar(Phase::Type),
                ]
            },
        })
    }

    pub fn text(
        &mut self,
        prefix: &str,
        value: &[u16],
        suffix: &str,
    ) -> Result<Vec<u16>, EditorError> {
        let size = prefix
            .encode_utf16()
            .count()
            .checked_add(value.len())
            .and_then(|size| size.checked_add(suffix.encode_utf16().count()))
            .ok_or(EditorError::Capacity)?;
        if size
            > self
                .limits
                .max_message_units
                .saturating_sub(self.message_units)
        {
            return Err(EditorError::Capacity);
        }
        let mut message = prefix.encode_utf16().collect::<Vec<_>>();
        message.extend_from_slice(value);
        message.extend(suffix.encode_utf16());
        if message.len()
            > self
                .limits
                .max_message_units
                .saturating_sub(self.message_units)
        {
            return Err(EditorError::Capacity);
        }
        self.spend(message.len())?;
        Ok(message)
    }

    pub fn custom(&mut self, schema: usize) -> Result<Option<Vec<u16>>, EditorError> {
        let Some(index) = self.field(schema, "errorMessage")? else {
            return Ok(None);
        };
        match self.schema.get(index).ok_or(EditorError::InvalidIdentity)? {
            Value::String(value) if !value.is_empty() => {
                self.spend(value.len())?;
                if value.len()
                    > self
                        .limits
                        .max_message_units
                        .saturating_sub(self.message_units)
                {
                    return Err(EditorError::Capacity);
                }
                Ok(Some(value.clone()))
            }
            value if !value.is_truthy() => Ok(None),
            _ => Err(EditorError::InvalidBoundary),
        }
    }

    fn charge(&mut self, diagnostics: &[Diagnostic]) -> Result<(), EditorError> {
        self.diagnostic_count = self
            .diagnostic_count
            .checked_add(diagnostics.len())
            .ok_or(EditorError::Capacity)?;
        for diagnostic in diagnostics {
            self.message_units = self
                .message_units
                .checked_add(diagnostic.message.len())
                .ok_or(EditorError::Capacity)?;
        }
        if self.diagnostic_count > self.limits.max_diagnostics
            || self.message_units > self.limits.max_message_units
        {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    pub fn problem(
        &mut self,
        frame: &mut Frame,
        node: usize,
        message: Vec<u16>,
        short: bool,
    ) -> Result<(), EditorError> {
        let diagnostic = self.diagnostic(node, message, short)?;
        frame.result.diagnostics.push(diagnostic);
        Ok(())
    }

    pub fn diagnostic(
        &mut self,
        node: usize,
        message: Vec<u16>,
        short: bool,
    ) -> Result<Diagnostic, EditorError> {
        let node = self
            .document
            .nodes
            .get(node)
            .ok_or(EditorError::InvalidIdentity)?;
        let diagnostic = Diagnostic {
            bytes: node.bytes.start..if short {
                (node.bytes.start + 1).min(node.bytes.end)
            } else {
                node.bytes.end
            },
            utf16: node.utf16.start..if short {
                (node.utf16.start + 1).min(node.utf16.end)
            } else {
                node.utf16.end
            },
            message,
            severity: self.severity,
            code: None,
        };
        self.charge(std::slice::from_ref(&diagnostic))?;
        Ok(diagnostic)
    }

    pub fn mark(&mut self, result: &mut Validation, name: Vec<u16>) -> Result<(), EditorError> {
        self.spend(name.len() + 1)?;
        if !result.processed_properties.contains(&name) {
            self.processed_count = self
                .processed_count
                .checked_add(1)
                .ok_or(EditorError::Capacity)?;
            if self.processed_count > self.limits.max_processed {
                return Err(EditorError::Capacity);
            }
            result.processed_properties.insert(name);
        }
        Ok(())
    }

    fn merge_processed(
        &mut self,
        result: &mut Validation,
        other: BTreeSet<Vec<u16>>,
    ) -> Result<(), EditorError> {
        for name in other {
            self.mark(result, name)?;
        }
        Ok(())
    }

    fn merge_all(&mut self, result: &mut Validation, other: Validation) -> Result<(), EditorError> {
        self.append_diagnostics(&mut result.diagnostics, other.diagnostics)?;
        self.append_schemas(&mut result.schemas, other.schemas)?;
        result.properties_matches += other.properties_matches;
        result.properties_value_matches += other.properties_value_matches;
        self.merge_processed(result, other.processed_properties)
    }

    fn append_diagnostics(
        &mut self,
        result: &mut Vec<Diagnostic>,
        other: Vec<Diagnostic>,
    ) -> Result<(), EditorError> {
        if result.is_empty() {
            *result = other;
            return Ok(());
        }
        self.spend(other.len() + 1)?;
        result.extend(other);
        Ok(())
    }

    fn append_schemas(
        &mut self,
        result: &mut Vec<MatchingSchema>,
        other: Vec<MatchingSchema>,
    ) -> Result<(), EditorError> {
        if result.is_empty() {
            *result = other;
            return Ok(());
        }
        self.spend(other.len() + 1)?;
        result.extend(other);
        Ok(())
    }

    fn merge_enum(&mut self, best: &mut Validation, other: &Validation) -> Result<(), EditorError> {
        if best.enum_value_match || other.enum_value_match {
            return Ok(());
        }
        let (Some(values), Some(other)) = (&mut best.enum_values, &other.enum_values) else {
            return Ok(());
        };
        if values
            .len()
            .checked_add(other.len())
            .is_none_or(|size| size > self.limits.scalar.max_enum_values)
        {
            return Err(EditorError::Capacity);
        }
        self.spend(other.len())?;
        values.extend_from_slice(other);
        let mut message = self.text("Value is not accepted. Valid values: ", &[], "")?;
        for (index, &value) in values.iter().enumerate() {
            if index > 0 {
                message.extend(", ".encode_utf16());
            }
            let text = json_value_text::stringify_budgeted(
                self.schema,
                value,
                json_value_text::Limits {
                    max_units: self
                        .limits
                        .max_message_units
                        .saturating_sub(self.message_units)
                        .saturating_sub(message.len()),
                    max_work: self.remaining,
                    max_depth: self.limits.scalar.max_depth,
                },
                &mut self.remaining,
            )?;
            message.extend(text);
        }
        message.extend(".".encode_utf16());
        for diagnostic in &mut best.diagnostics {
            if diagnostic.code == Some(ENUM_MISMATCH) {
                self.message_units = self
                    .message_units
                    .checked_add(message.len())
                    .ok_or(EditorError::Capacity)?;
                if self.message_units > self.limits.max_message_units {
                    return Err(EditorError::Capacity);
                }
                self.spend(message.len())?;
                diagnostic.message.clone_from(&message);
            }
        }
        Ok(())
    }

    fn scalar(&mut self, frame: &mut Frame, phase: Phase) -> Result<(), EditorError> {
        let node = &self.document.nodes[frame.node];
        if let Some(Value::Boolean(value)) = self.schema.get(frame.schema) {
            if phase == Phase::Type && !value {
                let message = self.text("Matches a schema that is not allowed.", &[], "")?;
                self.problem(frame, frame.node, message, false)?;
            }
            return Ok(());
        }
        let value = self
            .mapped
            .get(frame.node)
            .copied()
            .flatten()
            .ok_or(EditorError::InvalidIdentity)?;
        let limits = json_schema_scalar::Limits {
            max_diagnostics: self.limits.scalar.max_diagnostics.min(
                self.limits
                    .max_diagnostics
                    .saturating_sub(self.diagnostic_count),
            ),
            max_message_units: self.limits.scalar.max_message_units.min(
                self.limits
                    .max_message_units
                    .saturating_sub(self.message_units),
            ),
            ..self.limits.scalar
        };
        let result = json_schema_scalar::validate_part(
            Target {
                node,
                parent: node
                    .parent
                    .and_then(|parent| self.document.nodes.get(parent)),
                values: self.values,
                value,
            },
            self.schema,
            frame.schema,
            self.severity,
            limits,
            Some(self.compiler),
            phase,
            &mut self.remaining,
        )?;
        self.charge(&result.diagnostics)?;
        self.append_diagnostics(&mut frame.result.diagnostics, result.diagnostics)?;
        if result.enum_values.is_some() {
            frame.result.enum_value_match = result.enum_value_match;
            frame.result.enum_values = result.enum_values;
        }
        Ok(())
    }

    fn merge(
        &mut self,
        frame: &mut Frame,
        mode: Merge,
        mut child: Validation,
    ) -> Result<(), EditorError> {
        let mode = if matches!(mode, Merge::RawProperty) {
            Merge::Property(None)
        } else {
            mode
        };
        match mode {
            Merge::RawProperty => return Err(EditorError::InvalidBoundary),
            Merge::All => self.merge_all(&mut frame.result, child)?,
            Merge::Direct => {
                if child.enum_values.is_some() {
                    frame.result.enum_value_match = child.enum_value_match;
                    frame.result.enum_values = child.enum_values.take();
                }
                frame.result.primary_value_matches += child.primary_value_matches;
                self.merge_all(&mut frame.result, child)?;
            }
            Merge::Property(mark) => {
                frame.result.properties_matches += 1;
                if child.enum_value_match
                    || child.diagnostics.is_empty() && child.properties_matches > 0
                {
                    frame.result.properties_value_matches += 1;
                }
                if child.enum_value_match
                    && child
                        .enum_values
                        .as_ref()
                        .is_some_and(|values| values.len() == 1)
                {
                    frame.result.primary_value_matches += 1;
                }
                self.append_diagnostics(&mut frame.result.diagnostics, child.diagnostics)?;
                self.append_schemas(&mut frame.result.schemas, child.schemas)?;
                if let Some(mark) = mark {
                    self.mark(&mut frame.result, mark)?;
                }
            }
            Merge::Not => {
                if child.diagnostics.is_empty() {
                    let message = if let Some(message) = self.custom(frame.schema)? {
                        message
                    } else {
                        self.text("Matches a schema that is not allowed.", &[], "")?
                    };
                    self.problem(frame, frame.node, message, false)?;
                }
                self.spend(child.schemas.len() + 1)?;
                frame
                    .result
                    .schemas
                    .extend(child.schemas.into_iter().map(|mut schema| {
                        schema.inverted = !schema.inverted;
                        schema
                    }));
            }
            Merge::Condition {
                then_schema,
                else_schema,
            } => {
                let branch = if child.diagnostics.is_empty() {
                    then_schema
                } else {
                    else_schema
                };
                self.append_schemas(&mut frame.result.schemas, child.schemas)?;
                self.merge_processed(&mut frame.result, child.processed_properties)?;
                if let Some(schema) = branch {
                    self.queue(
                        frame,
                        vec![Operation::Child {
                            node: frame.node,
                            schema,
                            collect: frame.collect.sub(),
                            merge: Merge::All,
                        }],
                    )?;
                }
            }
            Merge::Alternative(mut alternatives) => {
                if child.diagnostics.is_empty() {
                    alternatives.matches += 1;
                }
                if let Some(best) = &mut alternatives.best {
                    if !alternatives.one
                        && child.diagnostics.is_empty()
                        && best.diagnostics.is_empty()
                    {
                        self.append_schemas(&mut best.schemas, child.schemas)?;
                        best.properties_matches += child.properties_matches;
                        best.properties_value_matches += child.properties_value_matches;
                        self.merge_processed(best, child.processed_properties)?;
                    } else {
                        match child.compare(best) {
                            Ordering::Greater => alternatives.best = Some(Box::new(child)),
                            Ordering::Equal => {
                                self.merge_enum(best, &child)?;
                                self.append_schemas(&mut best.schemas, child.schemas)?;
                            }
                            Ordering::Less => {}
                        }
                    }
                } else {
                    alternatives.best = Some(Box::new(child));
                }
                self.queue(frame, vec![Operation::Alternatives(alternatives)])?;
            }
            Merge::Contains(mut contains) => {
                if child.diagnostics.is_empty() {
                    contains.matches += 1;
                    if self.draft >= Draft::Draft2020 {
                        self.mark(
                            &mut frame.result,
                            (contains.index - 1).to_string().encode_utf16().collect(),
                        )?;
                    }
                }
                self.queue(frame, vec![Operation::Contains(contains)])?;
            }
        }
        Ok(())
    }

    fn operation(
        &mut self,
        frame: &mut Frame,
        operation: Operation,
        last: &mut Option<Validation>,
    ) -> Result<(), EditorError> {
        match operation {
            Operation::Problem(diagnostic) => frame.result.diagnostics.push(diagnostic),
            Operation::Scalar(phase) => self.scalar(frame, phase)?,
            Operation::Merge(mode) => self.merge(
                frame,
                mode,
                last.take().ok_or(EditorError::InvalidIdentity)?,
            )?,
            Operation::AllOf => {
                if let Some(index) = self.field(frame.schema, "allOf")? {
                    if let Some(Value::Array(items)) = self.schema.get(index) {
                        if items.len() > self.remaining {
                            return Err(EditorError::Capacity);
                        }
                        let mut operations = Vec::new();
                        for &schema in items {
                            operations.push(Operation::Child {
                                node: frame.node,
                                schema,
                                collect: frame.collect.sub(),
                                merge: Merge::All,
                            });
                        }
                        self.queue(frame, operations)?;
                    }
                }
            }
            Operation::Not => {
                let schema = self.field(frame.schema, "not")?;
                if let Some(schema) = self.schema_ref(schema)? {
                    self.queue(
                        frame,
                        vec![Operation::Child {
                            node: frame.node,
                            schema,
                            collect: frame.collect.sub(),
                            merge: Merge::Not,
                        }],
                    )?;
                }
            }
            Operation::AnyOf | Operation::OneOf => {
                let one = matches!(operation, Operation::OneOf);
                if let Some(schemas) =
                    self.field(frame.schema, if one { "oneOf" } else { "anyOf" })?
                {
                    if matches!(self.schema.get(schemas), Some(Value::Array(_))) {
                        self.queue(
                            frame,
                            vec![Operation::Alternatives(Alternatives {
                                schemas,
                                index: 0,
                                best: None,
                                matches: 0,
                                one,
                            })],
                        )?;
                    }
                }
            }
            Operation::Alternatives(mut alternatives) => {
                let Some(Value::Array(items)) = self.schema.get(alternatives.schemas) else {
                    return Err(EditorError::InvalidBoundary);
                };
                if let Some(&schema) = items.get(alternatives.index) {
                    alternatives.index += 1;
                    self.queue(
                        frame,
                        vec![Operation::Child {
                            node: frame.node,
                            schema,
                            collect: frame.collect.sub(),
                            merge: Merge::Alternative(alternatives),
                        }],
                    )?;
                } else {
                    if alternatives.one && alternatives.matches > 1 {
                        let message = self.text(
                            "Matches multiple schemas when only one must validate.",
                            &[],
                            "",
                        )?;
                        self.problem(frame, frame.node, message, true)?;
                    }
                    if let Some(best) = alternatives.best {
                        self.merge_all(&mut frame.result, *best)?;
                    }
                }
            }
            Operation::Condition => {
                let condition = self.field(frame.schema, "if")?;
                if let Some(schema) = self.schema_ref(condition)? {
                    let then_schema = self.field(frame.schema, "then")?;
                    let then_schema = self.schema_ref(then_schema)?;
                    let else_schema = self.field(frame.schema, "else")?;
                    let else_schema = self.schema_ref(else_schema)?;
                    self.queue(
                        frame,
                        vec![Operation::Child {
                            node: frame.node,
                            schema,
                            collect: frame.collect.sub(),
                            merge: Merge::Condition {
                                then_schema,
                                else_schema,
                            },
                        }],
                    )?;
                }
            }
            operation => self.structural_operation(frame, operation)?,
        }
        Ok(())
    }

    fn add_matching(
        &mut self,
        result: &mut Validation,
        node: usize,
        schema: usize,
        view: SchemaView,
        inverted: bool,
    ) -> Result<(), EditorError> {
        self.match_count = self
            .match_count
            .checked_add(1)
            .ok_or(EditorError::Capacity)?;
        if self.match_count > self.limits.max_matches {
            return Err(EditorError::Capacity);
        }
        result.schemas.push(MatchingSchema {
            node,
            schema,
            view,
            inverted,
        });
        Ok(())
    }

    fn run(
        &mut self,
        root: usize,
        schema: usize,
        selection: Selection,
    ) -> Result<Validation, EditorError> {
        let mut frames = vec![self.frame(
            root,
            schema,
            Collection {
                enabled: true,
                selection,
            },
            false,
        )?];
        let mut last = None;
        while let Some(mut frame) = frames.pop() {
            self.spend(1)?;
            match frame.pending.pop().ok_or(EditorError::InvalidIdentity)? {
                Operation::Finish => {
                    if frame.is_included && frame.collect.enabled {
                        if frame.view == SchemaView::Reject {
                            self.add_matching(
                                &mut frame.result,
                                frame.node,
                                frame.schema,
                                SchemaView::Empty,
                                true,
                            )?;
                        }
                        self.add_matching(
                            &mut frame.result,
                            frame.node,
                            frame.schema,
                            frame.view,
                            false,
                        )?;
                    }
                    if frame.is_included {
                        self.active.remove(&(frame.node, frame.schema));
                    }
                    last = Some(frame.result);
                }
                Operation::Child {
                    node,
                    schema,
                    collect,
                    merge,
                } => {
                    if frames.len() + 1 >= self.limits.max_frames {
                        return Err(EditorError::Capacity);
                    }
                    let normalize_boolean = !matches!(merge, Merge::RawProperty);
                    frame.pending.push(Operation::Merge(merge));
                    let child = self.frame(node, schema, collect, normalize_boolean)?;
                    frames.push(frame);
                    frames.push(child);
                }
                operation => {
                    self.operation(&mut frame, operation, &mut last)?;
                    frames.push(frame);
                }
            }
        }
        last.ok_or(EditorError::InvalidIdentity)
    }
}

pub fn validate(
    document: &Document,
    schema: &Arena,
    schema_node: usize,
    compiler: &dyn PatternCompiler,
    severity: Severity,
    draft: Option<Draft>,
    limits: Limits,
) -> Result<Validation, EditorError> {
    if !schema
        .get(schema_node)
        .ok_or(EditorError::InvalidIdentity)?
        .is_truthy()
    {
        return Ok(Validation::default());
    }
    evaluate(
        document,
        schema,
        schema_node,
        compiler,
        severity,
        draft,
        limits,
        Selection::default(),
    )
}

pub fn matching_schemas(
    document: &Document,
    schema: &Arena,
    schema_node: usize,
    compiler: &dyn PatternCompiler,
    selection: Selection,
    limits: Limits,
) -> Result<Vec<MatchingSchema>, EditorError> {
    if !schema
        .get(schema_node)
        .ok_or(EditorError::InvalidIdentity)?
        .is_truthy()
    {
        return Ok(Vec::new());
    }
    if selection
        .exclude_node
        .is_some_and(|node| node >= document.nodes.len())
    {
        return Err(EditorError::InvalidIdentity);
    }
    Ok(evaluate(
        document,
        schema,
        schema_node,
        compiler,
        Severity::Warning,
        None,
        limits,
        selection,
    )?
    .schemas)
}

fn evaluate(
    document: &Document,
    schema: &Arena,
    schema_node: usize,
    compiler: &dyn PatternCompiler,
    severity: Severity,
    draft: Option<Draft>,
    limits: Limits,
    selection: Selection,
) -> Result<Validation, EditorError> {
    let Some(root) = document.root else {
        return Ok(Validation::default());
    };
    if limits.max_frames == 0 {
        return Err(EditorError::Capacity);
    }
    let (values, mapped) = Arena::from_document_mapped(document, limits.values, true)?;
    let mut evaluator = Evaluator {
        document,
        values: &values,
        mapped: &mapped,
        schema,
        compiler,
        severity,
        draft: draft.unwrap_or(Draft::Draft2020),
        limits,
        remaining: limits.max_work,
        active: BTreeSet::new(),
        diagnostic_count: 0,
        message_units: 0,
        match_count: 0,
        processed_count: 0,
    };
    if draft.is_none() {
        if let Some(index) = evaluator.field(schema_node, "$schema")? {
            if let Some(Value::String(id)) = schema.get(index) {
                for (uri, draft) in [
                    ("http://json-schema.org/draft-03/schema#", Draft::Draft3),
                    ("http://json-schema.org/draft-04/schema#", Draft::Draft4),
                    ("http://json-schema.org/draft-06/schema#", Draft::Draft6),
                    ("http://json-schema.org/draft-07/schema#", Draft::Draft7),
                    (
                        "https://json-schema.org/draft/2019-09/schema",
                        Draft::Draft2019,
                    ),
                    (
                        "https://json-schema.org/draft/2020-12/schema",
                        Draft::Draft2020,
                    ),
                ] {
                    evaluator.spend(id.len() + 1)?;
                    if id.iter().copied().eq(uri.encode_utf16()) {
                        evaluator.draft = draft;
                        break;
                    }
                }
            }
        }
    }
    evaluator.run(root, schema_node, selection)
}
