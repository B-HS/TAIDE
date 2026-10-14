use std::ops::Range;

use crate::diagnostics::Severity;
use crate::document::EditorError;
use crate::json_parser::{Kind, Node};
use crate::json_value::{Arena, Value};
use crate::json_value_text;

const ENUM_MISMATCH: u32 = 1;
const DEPRECATED: u32 = 2;
const DECIMAL_BASE: f64 = 10.0;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_work: usize,
    pub max_diagnostics: usize,
    pub max_message_units: usize,
    pub max_enum_values: usize,
    pub max_depth: usize,
}

pub struct Target<'a> {
    pub node: &'a Node,
    pub parent: Option<&'a Node>,
    pub values: &'a Arena,
    pub value: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub bytes: Range<usize>,
    pub utf16: Range<usize>,
    pub message: Vec<u16>,
    pub severity: Severity,
    pub code: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct Validation {
    pub diagnostics: Vec<Diagnostic>,
    pub enum_values: Option<Vec<usize>>,
    pub enum_value_match: bool,
}

struct Validator<'a> {
    schema: &'a Arena,
    schema_node: usize,
    limits: Limits,
    remaining: usize,
    message_units: usize,
    result: Validation,
}

impl Validator<'_> {
    fn spend(&mut self, work: usize) -> Result<(), EditorError> {
        self.remaining = self
            .remaining
            .checked_sub(work)
            .ok_or(EditorError::Capacity)?;
        Ok(())
    }

    fn field(&mut self, name: &str) -> Result<Option<usize>, EditorError> {
        self.spend(1)?;
        let Some(value) = self.schema.get(self.schema_node) else {
            return Err(EditorError::InvalidIdentity);
        };
        let Value::Object(members) = value else {
            return Ok(None);
        };
        for member in members {
            self.spend(1 + member.key.len())?;
            if member.key.iter().copied().eq(name.encode_utf16()) {
                return Ok(Some(member.value));
            }
        }
        Ok(None)
    }

    fn number(&mut self, name: &str) -> Result<Option<f64>, EditorError> {
        let Some(index) = self.field(name)? else {
            return Ok(None);
        };
        Ok(match self.schema.get(index) {
            Some(Value::Number(value)) => Some(*value),
            _ => None,
        })
    }

    fn truthy(&mut self, name: &str) -> Result<bool, EditorError> {
        let Some(index) = self.field(name)? else {
            return Ok(false);
        };
        self.schema
            .get(index)
            .map(Value::is_truthy)
            .ok_or(EditorError::InvalidIdentity)
    }

    fn custom(&mut self, name: &str) -> Result<Option<Vec<u16>>, EditorError> {
        let Some(index) = self.field(name)? else {
            return Ok(None);
        };
        match self.schema.get(index) {
            Some(Value::String(value)) if !value.is_empty() => {
                if value.len()
                    > self
                        .limits
                        .max_message_units
                        .saturating_sub(self.message_units)
                {
                    return Err(EditorError::Capacity);
                }
                self.spend(value.len())?;
                Ok(Some(value.clone()))
            }
            Some(value) if !value.is_truthy() => Ok(None),
            _ => Err(EditorError::InvalidBoundary),
        }
    }

    fn append(&mut self, output: &mut Vec<u16>, text: &[u16]) -> Result<(), EditorError> {
        if output
            .len()
            .checked_add(text.len())
            .and_then(|size| size.checked_add(self.message_units))
            .is_none_or(|size| size > self.limits.max_message_units)
        {
            return Err(EditorError::Capacity);
        }
        self.spend(text.len())?;
        output.extend_from_slice(text);
        Ok(())
    }

    fn literal(&mut self, output: &mut Vec<u16>, text: &str) -> Result<(), EditorError> {
        self.append(output, &text.encode_utf16().collect::<Vec<_>>())
    }

    fn serialized(&mut self, output: &mut Vec<u16>, value: usize) -> Result<(), EditorError> {
        let max_units = self
            .limits
            .max_message_units
            .saturating_sub(self.message_units)
            .saturating_sub(output.len());
        let text = json_value_text::stringify_budgeted(
            self.schema,
            value,
            json_value_text::Limits {
                max_units,
                max_work: self.remaining,
                max_depth: self.limits.max_depth,
            },
            &mut self.remaining,
        )?;
        self.append(output, &text)
    }

    fn add(
        &mut self,
        node: &Node,
        message: Vec<u16>,
        severity: Severity,
        code: Option<u32>,
    ) -> Result<(), EditorError> {
        self.message_units = self
            .message_units
            .checked_add(message.len())
            .ok_or(EditorError::Capacity)?;
        if self.result.diagnostics.len() >= self.limits.max_diagnostics
            || self.message_units > self.limits.max_message_units
        {
            return Err(EditorError::Capacity);
        }
        self.result.diagnostics.push(Diagnostic {
            bytes: node.bytes.clone(),
            utf16: node.utf16.clone(),
            message,
            severity,
            code,
        });
        Ok(())
    }

    fn numeric(&mut self, node: &Node, value: f64, severity: Severity) -> Result<(), EditorError> {
        if let Some(divisor) = self.number("multipleOf")? {
            let mut remainder = f64::NAN;
            if divisor.fract() == 0.0 {
                remainder = value % divisor;
            } else if let (Some((mut value, value_shift)), Some((mut divisor, divisor_shift))) =
                (normalized(value), normalized(divisor))
            {
                let scale = DECIMAL_BASE.powi((value_shift - divisor_shift).abs());
                if value_shift < divisor_shift {
                    value *= scale;
                } else {
                    divisor *= scale;
                }
                remainder = value % divisor;
            }
            if remainder != 0.0 {
                self.numeric_problem(node, "Value is not divisible by ", divisor, severity)?;
            }
        }
        let minimum = self.number("minimum")?;
        let maximum = self.number("maximum")?;
        let exclusive_minimum = self
            .field("exclusiveMinimum")?
            .and_then(|index| self.schema.get(index));
        let exclusive_maximum = self
            .field("exclusiveMaximum")?
            .and_then(|index| self.schema.get(index));
        let minimum_exclusive = match exclusive_minimum {
            Some(Value::Number(value)) => Some(*value),
            Some(Value::Boolean(true)) => minimum,
            _ => None,
        };
        let maximum_exclusive = match exclusive_maximum {
            Some(Value::Number(value)) => Some(*value),
            Some(Value::Boolean(true)) => maximum,
            _ => None,
        };
        if let Some(limit) = minimum_exclusive.filter(|limit| value <= *limit) {
            self.numeric_problem(
                node,
                "Value is below the exclusive minimum of ",
                limit,
                severity,
            )?;
        }
        if let Some(limit) = maximum_exclusive.filter(|limit| value >= *limit) {
            self.numeric_problem(
                node,
                "Value is above the exclusive maximum of ",
                limit,
                severity,
            )?;
        }
        if !matches!(exclusive_minimum, Some(Value::Boolean(true))) {
            if let Some(limit) = minimum.filter(|limit| value < *limit) {
                self.numeric_problem(node, "Value is below the minimum of ", limit, severity)?;
            }
        }
        if !matches!(exclusive_maximum, Some(Value::Boolean(true))) {
            if let Some(limit) = maximum.filter(|limit| value > *limit) {
                self.numeric_problem(node, "Value is above the maximum of ", limit, severity)?;
            }
        }
        Ok(())
    }

    fn numeric_problem(
        &mut self,
        node: &Node,
        prefix: &str,
        value: f64,
        severity: Severity,
    ) -> Result<(), EditorError> {
        let mut message = Vec::new();
        self.literal(&mut message, prefix)?;
        self.literal(&mut message, &crate::json_symbols::number_string(value))?;
        self.literal(&mut message, ".")?;
        self.add(node, message, severity, None)
    }
}

fn normalized(value: f64) -> Option<(f64, i32)> {
    if !value.is_finite() {
        return None;
    }
    let text = crate::json_symbols::number_string(value);
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let exponent = exponent.parse::<i32>().ok()?;
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{integer}{fraction}").parse().ok()?;
    let shift = i32::try_from(fraction.len()).ok()?.checked_sub(exponent)?;
    Some((digits, shift))
}

fn matches_type(node: &Node, expected: &[u16]) -> bool {
    let kind = match node.kind {
        Kind::Object => "object",
        Kind::Array => "array",
        Kind::Property => "property",
        Kind::String => "string",
        Kind::Number => "number",
        Kind::Boolean => "boolean",
        Kind::Null => "null",
    };
    expected.iter().copied().eq(kind.encode_utf16())
        || node.kind == Kind::Number
            && node.is_integer
            && expected.iter().copied().eq("integer".encode_utf16())
}

pub fn validate(
    target: Target<'_>,
    schema: &Arena,
    schema_node: usize,
    severity: Severity,
    limits: Limits,
) -> Result<Validation, EditorError> {
    let instance = target
        .values
        .get(target.value)
        .ok_or(EditorError::InvalidIdentity)?;
    if schema.get(schema_node).is_none() {
        return Err(EditorError::InvalidIdentity);
    }
    let mut validator = Validator {
        schema,
        schema_node,
        limits,
        remaining: limits.max_work,
        message_units: 0,
        result: Validation::default(),
    };
    let node = target.node;
    if let Some(index) = validator.field("type")? {
        let mut names = Vec::new();
        let mut multiple = false;
        match schema.get(index).ok_or(EditorError::InvalidIdentity)? {
            Value::Array(items) => {
                multiple = true;
                if items.len() > limits.max_enum_values {
                    return Err(EditorError::Capacity);
                }
                for &item in items {
                    let Some(Value::String(name)) = schema.get(item) else {
                        return Err(EditorError::InvalidBoundary);
                    };
                    validator.spend(1 + name.len())?;
                    names.push(name.as_slice());
                }
            }
            Value::String(name) if !name.is_empty() => {
                validator.spend(name.len())?;
                names.push(name);
            }
            value if !value.is_truthy() => {}
            _ => return Err(EditorError::InvalidBoundary),
        }
        if (multiple || !names.is_empty()) && !names.iter().any(|name| matches_type(node, name)) {
            let message = if let Some(custom) = validator.custom("errorMessage")? {
                custom
            } else {
                let mut message = Vec::new();
                validator.literal(
                    &mut message,
                    if multiple {
                        "Incorrect type. Expected one of "
                    } else {
                        "Incorrect type. Expected \""
                    },
                )?;
                for (index, name) in names.iter().enumerate() {
                    if index > 0 {
                        validator.literal(&mut message, ", ")?;
                    }
                    validator.append(&mut message, name)?;
                }
                validator.literal(&mut message, if multiple { "." } else { "\"." })?;
                message
            };
            validator.add(node, message, severity, None)?;
        }
    }
    if let Some(index) = validator.field("enum")? {
        if let Some(Value::Array(items)) = schema.get(index) {
            if items.len() > limits.max_enum_values {
                return Err(EditorError::Capacity);
            }
            validator.spend(items.len())?;
            let mut matched = false;
            for &value in items {
                if target.values.equals_budgeted(
                    target.value,
                    schema,
                    value,
                    &mut validator.remaining,
                )? {
                    matched = true;
                    break;
                }
            }
            validator.result.enum_values = Some(items.clone());
            validator.result.enum_value_match = matched;
            if !matched {
                let message = if let Some(custom) = validator.custom("errorMessage")? {
                    custom
                } else {
                    let mut message = Vec::new();
                    validator.literal(&mut message, "Value is not accepted. Valid values: ")?;
                    for (index, &value) in items.iter().enumerate() {
                        if index > 0 {
                            validator.literal(&mut message, ", ")?;
                        }
                        validator.serialized(&mut message, value)?;
                    }
                    validator.literal(&mut message, ".")?;
                    message
                };
                validator.add(node, message, severity, Some(ENUM_MISMATCH))?;
            }
        }
    }
    if let Some(value) = validator.field("const")? {
        let matched =
            target
                .values
                .equals_budgeted(target.value, schema, value, &mut validator.remaining)?;
        validator.result.enum_value_match = matched;
        if limits.max_enum_values == 0 {
            return Err(EditorError::Capacity);
        }
        validator.result.enum_values = Some(vec![value]);
        if !matched {
            let message = if let Some(custom) = validator.custom("errorMessage")? {
                custom
            } else {
                let mut message = Vec::new();
                validator.literal(&mut message, "Value must be ")?;
                validator.serialized(&mut message, value)?;
                validator.literal(&mut message, ".")?;
                message
            };
            validator.add(node, message, severity, Some(ENUM_MISMATCH))?;
        }
    }
    let deprecation = validator.custom("deprecationMessage")?;
    if deprecation.is_some() || validator.truthy("deprecated")? {
        let message = if let Some(message) = deprecation {
            message
        } else {
            "Value is deprecated".encode_utf16().collect()
        };
        let location = target
            .parent
            .filter(|parent| parent.kind == Kind::Property)
            .unwrap_or(node);
        validator.add(location, message, Severity::Warning, Some(DEPRECATED))?;
    }
    match instance {
        Value::Number(value) => validator.numeric(node, *value, severity)?,
        Value::String(value) => {
            validator.spend(value.len())?;
            let length = instance
                .string_length()
                .ok_or(EditorError::InvalidBoundary)? as f64;
            if let Some(limit) = validator
                .number("minLength")?
                .filter(|limit| length < *limit)
            {
                validator.numeric_problem(
                    node,
                    "String is shorter than the minimum length of ",
                    limit,
                    severity,
                )?;
            }
            if let Some(limit) = validator
                .number("maxLength")?
                .filter(|limit| length > *limit)
            {
                validator.numeric_problem(
                    node,
                    "String is longer than the maximum length of ",
                    limit,
                    severity,
                )?;
            }
        }
        _ => {}
    }
    Ok(validator.result)
}
