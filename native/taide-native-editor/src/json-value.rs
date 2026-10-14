use std::collections::{BTreeMap, BTreeSet};

use crate::document::EditorError;
use crate::json_parser::{self, Document, Kind, Value as AstValue};

const ARRAY_INDEX_MAX: u32 = u32::MAX - 1;
const DECIMAL_RADIX: u32 = 10;
const ZERO_UNIT: u16 = 0x30;
const NINE_UNIT: u16 = 0x39;
const HIGH_SURROGATE_START: u16 = 0xd800;
const HIGH_SURROGATE_END: u16 = 0xdbff;
const LOW_SURROGATE_START: u16 = 0xdc00;
const LOW_SURROGATE_END: u16 = 0xdfff;

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub key: Vec<u16>,
    pub value: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Number(f64),
    String(Vec<u16>),
    Array(Vec<usize>),
    Object(Vec<Member>),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Boolean(value) => *value,
            Self::Number(value) => *value != 0.0 && !value.is_nan(),
            Self::String(value) => !value.is_empty(),
            Self::Array(_) | Self::Object(_) => true,
        }
    }

    pub fn string_length(&self) -> Option<usize> {
        let Self::String(value) = self else {
            return None;
        };
        let mut units = value.iter().peekable();
        let mut count = 0;
        while let Some(unit) = units.next() {
            count += 1;
            if (HIGH_SURROGATE_START..=HIGH_SURROGATE_END).contains(unit)
                && units
                    .peek()
                    .is_some_and(|unit| (LOW_SURROGATE_START..=LOW_SURROGATE_END).contains(unit))
            {
                units.next();
            }
        }
        Some(count)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arena {
    pub(crate) root: Option<usize>,
    pub(crate) nodes: Vec<Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub parser: json_parser::Limits,
    pub max_values: usize,
    pub max_text_units: usize,
}

struct Budget {
    limits: Limits,
    text_units: usize,
}

impl Budget {
    fn copy(&mut self, value: &[u16]) -> Result<Vec<u16>, EditorError> {
        self.text_units = self
            .text_units
            .checked_add(value.len())
            .ok_or(EditorError::Capacity)?;
        if self.text_units > self.limits.max_text_units {
            return Err(EditorError::Capacity);
        }
        Ok(value.to_vec())
    }
}

fn array_index(key: &[u16]) -> Option<u32> {
    if key.is_empty() || key.len() > 1 && key[0] == ZERO_UNIT {
        return None;
    }
    let mut value = 0u32;
    for &unit in key {
        if !(ZERO_UNIT..=NINE_UNIT).contains(&unit) {
            return None;
        }
        value = value
            .checked_mul(DECIMAL_RADIX)?
            .checked_add(u32::from(unit - ZERO_UNIT))?;
    }
    (value <= ARRAY_INDEX_MAX).then_some(value)
}

pub(crate) fn sort_members(members: &mut [Member]) {
    members.sort_by(
        |one, two| match (array_index(&one.key), array_index(&two.key)) {
            (Some(one), Some(two)) => one.cmp(&two),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        },
    );
}

impl Arena {
    pub fn root(&self) -> Option<usize> {
        self.root
    }

    pub fn values(&self) -> &[Value] {
        &self.nodes
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        self.nodes.get(index)
    }

    pub fn member(&self, index: usize, name: &[u16]) -> Option<usize> {
        let Value::Object(members) = self.get(index)? else {
            return None;
        };
        members
            .iter()
            .find(|member| member.key == name)
            .map(|member| member.value)
    }

    pub fn item(&self, index: usize, item: usize) -> Option<usize> {
        let Value::Array(items) = self.get(index)? else {
            return None;
        };
        items.get(item).copied()
    }

    pub fn equals(
        &self,
        left: usize,
        other: &Self,
        right: usize,
        max_work: usize,
    ) -> Result<bool, EditorError> {
        let mut remaining = max_work;
        self.equals_budgeted(left, other, right, &mut remaining)
    }

    pub(crate) fn equals_budgeted(
        &self,
        left: usize,
        other: &Self,
        right: usize,
        remaining: &mut usize,
    ) -> Result<bool, EditorError> {
        let mut pending = vec![(left, right)];
        let mut seen = BTreeSet::new();
        while let Some((left, right)) = pending.pop() {
            *remaining = remaining.checked_sub(1).ok_or(EditorError::Capacity)?;
            let one = self.get(left).ok_or(EditorError::InvalidBoundary)?;
            let two = other.get(right).ok_or(EditorError::InvalidBoundary)?;
            match (one, two) {
                (Value::Null, Value::Null) => {}
                (Value::Boolean(one), Value::Boolean(two)) if one == two => {}
                (Value::Number(one), Value::Number(two)) if one == two => {}
                (Value::String(one), Value::String(two)) => {
                    *remaining = remaining
                        .checked_sub(one.len().min(two.len()))
                        .ok_or(EditorError::Capacity)?;
                    if one != two {
                        return Ok(false);
                    }
                }
                (Value::Array(one), Value::Array(two)) => {
                    if std::ptr::eq(self, other) && left == right || !seen.insert((left, right)) {
                        continue;
                    }
                    if one.len() != two.len() {
                        return Ok(false);
                    }
                    if one
                        .len()
                        .checked_add(pending.len())
                        .is_none_or(|count| count > *remaining)
                    {
                        return Err(EditorError::Capacity);
                    }
                    pending.extend(one.iter().copied().zip(two.iter().copied()));
                }
                (Value::Object(one), Value::Object(two)) => {
                    if std::ptr::eq(self, other) && left == right || !seen.insert((left, right)) {
                        continue;
                    }
                    if one.len() != two.len() {
                        return Ok(false);
                    }
                    if one
                        .len()
                        .checked_add(pending.len())
                        .is_none_or(|count| count > *remaining)
                    {
                        return Err(EditorError::Capacity);
                    }
                    for member in one.iter().chain(two) {
                        *remaining = remaining
                            .checked_sub(member.key.len())
                            .ok_or(EditorError::Capacity)?;
                    }
                    let lookup = two
                        .iter()
                        .map(|member| (member.key.as_slice(), member.value))
                        .collect::<BTreeMap<_, _>>();
                    for member in one {
                        let Some(&right) = lookup.get(member.key.as_slice()) else {
                            return Ok(false);
                        };
                        pending.push((member.value, right));
                    }
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    pub fn from_document(document: &Document, limits: Limits) -> Result<Self, EditorError> {
        Ok(Self::from_document_mapped(document, limits, false)?.0)
    }

    pub(crate) fn from_document_mapped(
        document: &Document,
        limits: Limits,
        include_keys: bool,
    ) -> Result<(Self, Vec<Option<usize>>), EditorError> {
        if document.nodes.len() > limits.parser.max_nodes
            || document.diagnostics.len() > limits.parser.max_diagnostics
            || document.comments.len() > limits.parser.max_comments
        {
            return Err(EditorError::Capacity);
        }
        let mut depths = Vec::<usize>::new();
        for (index, node) in document.nodes.iter().enumerate() {
            let depth = if let Some(parent) = node.parent {
                if parent >= index {
                    return Err(EditorError::InvalidBoundary);
                }
                depths[parent].checked_add(1).ok_or(EditorError::Capacity)?
            } else {
                1
            };
            if depth > limits.parser.max_depth {
                return Err(EditorError::Capacity);
            }
            depths.push(depth);
        }
        let Some(root) = document.root else {
            return Ok((
                Self {
                    root: None,
                    nodes: Vec::new(),
                },
                vec![None; document.nodes.len()],
            ));
        };
        let mut arena = Self {
            root: None,
            nodes: Vec::new(),
        };
        let mut budget = Budget {
            limits,
            text_units: 0,
        };
        let mut mapped = vec![None; document.nodes.len()];
        for (index, node) in document.nodes.iter().enumerate().rev() {
            if node.kind == Kind::Property
                || !include_keys
                    && node.parent.is_some_and(|parent| {
                        document.nodes.get(parent).is_some_and(|parent| {
                            parent.kind == Kind::Property && parent.children.first() == Some(&index)
                        })
                    })
            {
                continue;
            }
            if arena.nodes.len() >= limits.max_values {
                return Err(EditorError::Capacity);
            }
            let value = match (&node.kind, &node.value) {
                (Kind::Null, AstValue::Null) => Value::Null,
                (Kind::Boolean, AstValue::Boolean(value)) => Value::Boolean(*value),
                (Kind::Number, AstValue::Number(value)) => Value::Number(*value),
                (Kind::String, AstValue::String(value)) => Value::String(budget.copy(value)?),
                (Kind::Array, _) => {
                    let items = node
                        .children
                        .iter()
                        .map(|&child| {
                            mapped
                                .get(child)
                                .copied()
                                .flatten()
                                .ok_or(EditorError::InvalidBoundary)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Value::Array(items)
                }
                (Kind::Object, _) => {
                    let mut members = Vec::<Member>::new();
                    let mut positions = BTreeMap::<&[u16], usize>::new();
                    for &property in &node.children {
                        let property = document
                            .nodes
                            .get(property)
                            .ok_or(EditorError::InvalidBoundary)?;
                        let Some(&value) = property.children.get(1) else {
                            continue;
                        };
                        let key = *property
                            .children
                            .first()
                            .ok_or(EditorError::InvalidBoundary)?;
                        let AstValue::String(key) = &document
                            .nodes
                            .get(key)
                            .ok_or(EditorError::InvalidBoundary)?
                            .value
                        else {
                            return Err(EditorError::InvalidBoundary);
                        };
                        let value = mapped
                            .get(value)
                            .copied()
                            .flatten()
                            .ok_or(EditorError::InvalidBoundary)?;
                        if let Some(&position) = positions.get(key.as_slice()) {
                            members[position].value = value;
                        } else {
                            positions.insert(key.as_slice(), members.len());
                            members.push(Member {
                                key: budget.copy(key)?,
                                value,
                            });
                        }
                    }
                    sort_members(&mut members);
                    Value::Object(members)
                }
                _ => return Err(EditorError::InvalidBoundary),
            };
            mapped[index] = Some(arena.nodes.len());
            arena.nodes.push(value);
        }
        arena.root = Some(
            mapped
                .get(root)
                .copied()
                .flatten()
                .ok_or(EditorError::InvalidBoundary)?,
        );
        Ok((arena, mapped))
    }
}

pub fn parse(input: &str, limits: Limits) -> Result<Arena, EditorError> {
    let document = json_parser::parse(input, false, limits.parser)?;
    Arena::from_document(&document, limits)
}
