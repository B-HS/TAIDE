use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use crate::document::EditorError;
use crate::json_schema_uri;
use crate::json_value::{self, Arena, Member, Value, sort_members};

const HASH: u16 = 0x23;
const SLASH: u16 = 0x2f;
const TILDE: u16 = 0x7e;
const PERCENT: u16 = 0x25;
const ZERO: u16 = 0x30;
const ONE: u16 = 0x31;
const NINE: u16 = 0x39;
const DECIMAL_RADIX: usize = 10;
const HEX_RADIX: u8 = 16;
const PERCENT_ESCAPE_UNITS: usize = 3;
const SINGLE_FIELDS: &[&str] = &[
    "additionalItems",
    "additionalProperties",
    "not",
    "contains",
    "propertyNames",
    "if",
    "then",
    "else",
    "unevaluatedItems",
    "unevaluatedProperties",
];
const MAP_FIELDS: &[&str] = &[
    "definitions",
    "$defs",
    "properties",
    "patternProperties",
    "dependencies",
    "dependentSchemas",
];
const ARRAY_FIELDS: &[&str] = &["anyOf", "allOf", "oneOf", "prefixItems"];

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub values: json_value::Limits,
    pub max_work: usize,
    pub max_merges: usize,
    pub max_text_units: usize,
    pub max_messages: usize,
    pub max_message_units: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct External {
    pub target: usize,
    pub uri: Vec<u16>,
    pub fragment: Option<Vec<u16>>,
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub schema: Arena,
    pub errors: Vec<Vec<u16>>,
    pub unsupported: Vec<&'static str>,
    pub external: Vec<External>,
    pub schema_draft: Option<Vec<u16>>,
}

pub(crate) struct Resolver {
    pub(crate) result: Resolved,
    pub(crate) limits: Limits,
    pub(crate) work: usize,
    merges: usize,
    text_units: usize,
    message_units: usize,
    pub(crate) anchors: Option<BTreeMap<Vec<u16>, usize>>,
}

fn member(arena: &Arena, index: usize, key: &str) -> Option<usize> {
    let Value::Object(members) = arena.get(index)? else {
        return None;
    };
    members
        .iter()
        .find(|member| member.key.iter().copied().eq(key.encode_utf16()))
        .map(|member| member.value)
}

fn string<'a>(arena: &'a Arena, index: usize, key: &str) -> Option<&'a [u16]> {
    let Value::String(value) = arena.get(member(arena, index, key)?)? else {
        return None;
    };
    Some(value)
}

fn children(arena: &Arena, index: usize, max_items: usize) -> Result<Vec<usize>, EditorError> {
    let is_object = |id| matches!(arena.get(id), Some(Value::Object(_)));
    let mut result = Vec::new();
    let mut push = |id| {
        if result.len() >= max_items {
            return Err(EditorError::Capacity);
        }
        result.push(id);
        Ok(())
    };
    for key in SINGLE_FIELDS {
        if let Some(id) = member(arena, index, key).filter(|&id| is_object(id)) {
            push(id)?;
        }
    }
    for key in MAP_FIELDS {
        if let Some(Value::Object(members)) = member(arena, index, key).and_then(|id| arena.get(id))
        {
            for id in members
                .iter()
                .map(|member| member.value)
                .filter(|&id| is_object(id))
            {
                push(id)?;
            }
        }
    }
    for key in ARRAY_FIELDS {
        if let Some(Value::Array(items)) = member(arena, index, key).and_then(|id| arena.get(id)) {
            for id in items.iter().copied().filter(|&id| is_object(id)) {
                push(id)?;
            }
        }
    }
    if let Some(id) = member(arena, index, "items") {
        if let Some(Value::Array(items)) = arena.get(id) {
            for id in items.iter().copied().filter(|&id| is_object(id)) {
                push(id)?;
            }
        } else if is_object(id) {
            push(id)?;
        }
    }
    Ok(result)
}

fn hex(unit: u16) -> Option<u8> {
    let value = match unit {
        0x30..=0x39 => unit - 0x30,
        0x41..=0x46 => unit - 0x41 + 0x0a,
        0x61..=0x66 => unit - 0x61 + 0x0a,
        _ => return None,
    };
    u8::try_from(value).ok()
}

pub(crate) fn decode(input: &[u16]) -> Result<Vec<u16>, EditorError> {
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor < input.len() {
        if input[cursor] != PERCENT {
            result.push(input[cursor]);
            cursor += 1;
            continue;
        }
        let mut bytes = Vec::new();
        while cursor < input.len() && input[cursor] == PERCENT {
            let high = hex(*input.get(cursor + 1).ok_or(EditorError::InvalidBoundary)?)
                .ok_or(EditorError::InvalidBoundary)?;
            let low = hex(*input.get(cursor + 2).ok_or(EditorError::InvalidBoundary)?)
                .ok_or(EditorError::InvalidBoundary)?;
            bytes.push(high * HEX_RADIX + low);
            cursor += PERCENT_ESCAPE_UNITS;
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| EditorError::InvalidBoundary)?;
        result.extend(text.encode_utf16());
    }
    Ok(result)
}

pub(crate) fn pointer(
    arena: &Arena,
    root: usize,
    path: &[u16],
    remaining: &mut usize,
) -> Result<Option<usize>, EditorError> {
    if path.is_empty() {
        return Ok(Some(root));
    }
    if path[0] != SLASH {
        return Err(EditorError::InvalidBoundary);
    }
    let mut current = root;
    for segment in path[1..].split(|&unit| unit == SLASH) {
        let lookup = match arena.get(current) {
            Some(Value::Object(members)) => members.len(),
            _ => 1,
        };
        let cost = segment
            .len()
            .checked_add(lookup)
            .ok_or(EditorError::Capacity)?;
        *remaining = remaining.checked_sub(cost).ok_or(EditorError::Capacity)?;
        let mut key = Vec::new();
        let mut cursor = 0;
        while cursor < segment.len() {
            let unit = segment[cursor];
            if unit != TILDE {
                key.push(unit);
                cursor += 1;
                continue;
            }
            key.push(match segment.get(cursor + 1) {
                Some(&ZERO) => TILDE,
                Some(&ONE) => SLASH,
                _ => return Err(EditorError::InvalidBoundary),
            });
            cursor += 2;
        }
        let next = match arena.get(current) {
            Some(Value::Object(_)) => arena.member(current, &key),
            Some(Value::Array(items)) => {
                if key.is_empty() || key.len() > 1 && key[0] == ZERO {
                    return Ok(None);
                }
                let mut index = 0usize;
                for unit in key {
                    if !(ZERO..=NINE).contains(&unit) {
                        return Ok(None);
                    }
                    let Some(next) = index
                        .checked_mul(DECIMAL_RADIX)
                        .and_then(|value| value.checked_add(usize::from(unit - ZERO)))
                    else {
                        return Ok(None);
                    };
                    index = next;
                }
                items.get(index).copied()
            }
            _ => None,
        };
        let Some(next) = next else { return Ok(None) };
        current = next;
    }
    Ok(Some(current))
}

impl Resolver {
    pub(crate) fn charge(&mut self, count: usize) -> Result<(), EditorError> {
        self.work = self.work.checked_add(count).ok_or(EditorError::Capacity)?;
        if self.work > self.limits.max_work {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn step(&mut self) -> Result<(), EditorError> {
        if self.work >= self.limits.max_work {
            return Err(EditorError::Capacity);
        }
        self.work += 1;
        Ok(())
    }

    pub(crate) fn account(&mut self, units: usize) -> Result<(), EditorError> {
        self.text_units = self
            .text_units
            .checked_add(units)
            .ok_or(EditorError::Capacity)?;
        if self.text_units > self.limits.max_text_units {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn error(&mut self, prefix: &str, name: &[u16], suffix: &str) -> Result<(), EditorError> {
        let units = prefix
            .encode_utf16()
            .count()
            .checked_add(name.len())
            .and_then(|count| count.checked_add(suffix.encode_utf16().count()))
            .ok_or(EditorError::Capacity)?;
        self.message_units = self
            .message_units
            .checked_add(units)
            .ok_or(EditorError::Capacity)?;
        if self.result.errors.len() >= self.limits.max_messages
            || self.message_units > self.limits.max_message_units
        {
            return Err(EditorError::Capacity);
        }
        self.result.errors.push(
            prefix
                .encode_utf16()
                .chain(name.iter().copied())
                .chain(suffix.encode_utf16())
                .collect(),
        );
        Ok(())
    }

    pub(crate) fn message(&mut self, parts: &[&[u16]]) -> Result<(), EditorError> {
        let units = parts.iter().try_fold(0usize, |count, part| {
            count.checked_add(part.len()).ok_or(EditorError::Capacity)
        })?;
        let total = self
            .message_units
            .checked_add(units)
            .ok_or(EditorError::Capacity)?;
        if self.result.errors.len() >= self.limits.max_messages
            || total > self.limits.max_message_units
        {
            return Err(EditorError::Capacity);
        }
        self.message_units = total;
        self.result
            .errors
            .push(parts.iter().flat_map(|part| part.iter().copied()).collect());
        Ok(())
    }

    fn unsupported(&mut self, index: usize, fields: &[&'static str]) {
        for &field in fields {
            if member(&self.result.schema, index, field)
                .and_then(|id| self.result.schema.get(id))
                .is_some_and(Value::is_truthy)
                && !self.result.unsupported.contains(&field)
            {
                self.result.unsupported.push(field);
            }
        }
    }

    pub(crate) fn unresolved(&mut self, fragment: &[u16], uri: &[u16]) -> Result<(), EditorError> {
        self.message(&[
            &"$ref '".encode_utf16().collect::<Vec<_>>(),
            fragment,
            &"' in '".encode_utf16().collect::<Vec<_>>(),
            uri,
            &"' can not be resolved.".encode_utf16().collect::<Vec<_>>(),
        ])
    }

    pub(crate) fn build_anchors(&mut self, root: usize) -> Result<(), EditorError> {
        let mut anchors = BTreeMap::new();
        let mut pending = vec![root];
        let mut seen = BTreeSet::new();
        while let Some(index) = pending.pop() {
            self.step()?;
            if !seen.insert(index) {
                continue;
            }
            let arena = &self.result.schema;
            let id = member(arena, index, "$id")
                .filter(|&id| arena.get(id).is_some_and(Value::is_truthy))
                .or_else(|| member(arena, index, "id"));
            let anchor = id
                .and_then(|id| arena.get(id))
                .and_then(|value| match value {
                    Value::String(value) if value.first() == Some(&HASH) => Some(&value[1..]),
                    _ => None,
                })
                .or_else(|| string(arena, index, "$anchor"));
            if let Some(anchor) = anchor.filter(|anchor| !anchor.is_empty()) {
                let anchor = anchor.to_vec();
                if anchors.contains_key(&anchor) {
                    self.error("Duplicate anchor declaration: '", &anchor, "'")?;
                } else {
                    self.account(anchor.len())?;
                    anchors.insert(anchor, index);
                }
            }
            self.unsupported(index, &["$recursiveAnchor", "$dynamicAnchor"]);
            let remaining = self.limits.max_work - self.work;
            pending.extend(children(
                &self.result.schema,
                index,
                remaining.saturating_sub(pending.len()),
            )?);
        }
        self.anchors = Some(anchors);
        Ok(())
    }

    pub(crate) fn merge(&mut self, target: usize, section: usize) -> Result<(), EditorError> {
        if self.merges >= self.limits.max_merges {
            return Err(EditorError::Capacity);
        }
        self.merges += 1;
        if matches!(self.result.schema.get(section), Some(Value::Boolean(false))) {
            if self.result.schema.nodes.len() >= self.limits.values.max_values {
                return Err(EditorError::Capacity);
            }
            self.account("not".encode_utf16().count())?;
            let id = self.result.schema.nodes.len();
            self.result.schema.nodes.push(Value::Object(Vec::new()));
            let Value::Object(members) = &mut self.result.schema.nodes[target] else {
                return Err(EditorError::InvalidBoundary);
            };
            let key = "not".encode_utf16().collect::<Vec<_>>();
            if let Some(member) = members.iter_mut().find(|member| member.key == key) {
                member.value = id;
            } else {
                members.push(Member { key, value: id });
            }
            sort_members(members);
            return Ok(());
        }
        let Some(Value::Object(members)) = self.result.schema.get(section) else {
            return Ok(());
        };
        let work = members
            .len()
            .checked_add(match self.result.schema.get(target) {
                Some(Value::Object(target)) => target.len(),
                _ => 0,
            })
            .ok_or(EditorError::Capacity)?;
        let member_count = members.iter().try_fold(0usize, |count, member| {
            count
                .checked_add(member.key.len())
                .ok_or(EditorError::Capacity)
        })?;
        self.charge(work)?;
        self.account(member_count)?;
        let members = match self.result.schema.get(section) {
            Some(Value::Object(members)) => members.clone(),
            _ => unreachable!(),
        };
        let Value::Object(target) = &mut self.result.schema.nodes[target] else {
            return Err(EditorError::InvalidBoundary);
        };
        let mut merged = std::mem::take(target)
            .into_iter()
            .enumerate()
            .map(|(position, member)| (member.key, (position, member.value)))
            .collect::<BTreeMap<_, _>>();
        let mut position = merged.len();
        for member in members {
            if member.key.iter().copied().eq("id".encode_utf16())
                || member.key.iter().copied().eq("$id".encode_utf16())
            {
                continue;
            }
            match merged.entry(member.key) {
                Entry::Occupied(mut entry) => entry.get_mut().1 = member.value,
                Entry::Vacant(entry) => {
                    entry.insert((position, member.value));
                    position += 1;
                }
            }
        }
        let mut ordered = merged
            .into_iter()
            .map(|(key, (position, value))| (position, Member { key, value }))
            .collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|(position, _)| *position);
        *target = ordered.into_iter().map(|(_, member)| member).collect();
        sort_members(target);
        Ok(())
    }

    pub(crate) fn traverse(
        &mut self,
        node: usize,
        root: usize,
        uri: &[u16],
    ) -> Result<(), EditorError> {
        let mut pending = vec![node];
        let mut visited = BTreeSet::new();
        while let Some(index) = pending.pop() {
            self.step()?;
            if !visited.insert(index) {
                continue;
            }
            let mut refs = BTreeSet::new();
            let mut is_external = false;
            loop {
                let Some(id) = member(&self.result.schema, index, "$ref")
                    .filter(|&id| self.result.schema.get(id).is_some_and(Value::is_truthy))
                else {
                    break;
                };
                self.step()?;
                let reference_units = match self.result.schema.get(id) {
                    Some(Value::String(value)) => value.len(),
                    _ => 0,
                };
                self.account(reference_units)?;
                let reference = match self.result.schema.get(id) {
                    Some(Value::String(value)) => value.clone(),
                    _ => {
                        self.error("Invalid $ref: a string is required.", &[], "")?;
                        if let Value::Object(members) = &mut self.result.schema.nodes[index] {
                            members.retain(|member| {
                                !member.key.iter().copied().eq("$ref".encode_utf16())
                            });
                        }
                        break;
                    }
                };
                if let Value::Object(members) = &mut self.result.schema.nodes[index] {
                    members.retain(|member| !member.key.iter().copied().eq("$ref".encode_utf16()));
                }
                let hash = reference.iter().position(|&unit| unit == HASH);
                let uri_end = hash.unwrap_or(reference.len());
                let fragment = hash.map(|hash| {
                    reference[hash + 1..]
                        .iter()
                        .position(|&unit| unit == HASH)
                        .map_or(&reference[hash + 1..], |end| {
                            &reference[hash + 1..hash + 1 + end]
                        })
                });
                if uri_end > 0 {
                    self.result.external.push(External {
                        target: index,
                        uri: reference[..uri_end].to_vec(),
                        fragment: fragment.map(<[u16]>::to_vec),
                    });
                    is_external = true;
                    break;
                }
                if !refs.insert(reference.clone()) {
                    continue;
                }
                let fragment = fragment.unwrap_or(&[]);
                let decoded = decode(fragment);
                let section = match &decoded {
                    Ok(value) if value.is_empty() => Some(root),
                    Ok(value) if value[0] == SLASH => {
                        let mut remaining = self.limits.max_work - self.work;
                        let section = pointer(&self.result.schema, root, value, &mut remaining);
                        self.work = self.limits.max_work - remaining;
                        match section {
                            Ok(section) => section,
                            Err(EditorError::InvalidBoundary) => None,
                            Err(error) => return Err(error),
                        }
                    }
                    Ok(value) => {
                        if self.anchors.is_none() {
                            self.build_anchors(root)?;
                        }
                        self.anchors
                            .as_ref()
                            .and_then(|anchors| anchors.get(value))
                            .copied()
                    }
                    Err(_) => None,
                };
                if let Some(section) = section {
                    self.merge(index, section)?;
                } else {
                    self.unresolved(fragment, uri)?;
                }
            }
            if !is_external {
                self.unsupported(index, &["$recursiveRef", "$dynamicRef"]);
            }
            let remaining = self.limits.max_work - self.work;
            pending.extend(children(
                &self.result.schema,
                index,
                remaining.saturating_sub(pending.len()),
            )?);
        }
        Ok(())
    }
}

pub fn resolve_local(input: &str, uri: &str, limits: Limits) -> Result<Resolved, EditorError> {
    let schema = json_value::parse(input, limits.values)?;
    resolve_arena(schema, uri, limits)
}

pub fn resolve_arena(schema: Arena, uri: &str, limits: Limits) -> Result<Resolved, EditorError> {
    let (mut resolver, is_old_draft) = create_resolver(schema, limits)?;
    if !is_old_draft {
        let root = resolver
            .result
            .schema
            .root()
            .ok_or(EditorError::InvalidBoundary)?;
        resolver.traverse(root, root, &uri.encode_utf16().collect::<Vec<_>>())?;
    }
    Ok(resolver.result)
}

pub(crate) fn create_resolver(
    schema: Arena,
    limits: Limits,
) -> Result<(Resolver, bool), EditorError> {
    if schema.values().len() > limits.values.max_values {
        return Err(EditorError::Capacity);
    }
    let text_units = schema.values().iter().try_fold(0usize, |total, value| {
        let size = match value {
            Value::String(value) => value.len(),
            Value::Object(members) => members.iter().try_fold(0usize, |total, member| {
                total
                    .checked_add(member.key.len())
                    .ok_or(EditorError::Capacity)
            })?,
            _ => 0,
        };
        total.checked_add(size).ok_or(EditorError::Capacity)
    })?;
    if text_units > limits.values.max_text_units {
        return Err(EditorError::Capacity);
    }
    let root = schema.root().ok_or(EditorError::InvalidBoundary)?;
    let draft = string(&schema, root, "$schema").filter(|value| !value.is_empty());
    let draft_units = draft.map_or(0, <[u16]>::len);
    if draft_units > limits.max_text_units {
        return Err(EditorError::Capacity);
    }
    let schema_draft = draft
        .map(|value| {
            json_schema_uri::normalize_id(
                value,
                json_schema_uri::Limits {
                    max_input_units: limits.max_text_units,
                    max_output_units: limits.max_text_units,
                    max_work: limits.max_work,
                },
            )
        })
        .transpose()?;
    let is_old_draft = schema_draft.as_ref().is_some_and(|draft| {
        draft
            .iter()
            .copied()
            .eq("http://json-schema.org/draft-03/schema".encode_utf16())
    });
    let mut resolver = Resolver {
        result: Resolved {
            schema,
            errors: Vec::new(),
            unsupported: Vec::new(),
            external: Vec::new(),
            schema_draft,
        },
        limits,
        work: 0,
        merges: 0,
        text_units: draft_units,
        message_units: 0,
        anchors: None,
    };
    if is_old_draft {
        resolver.result.schema.nodes[root] = Value::Object(Vec::new());
        resolver.error("Draft-03 schemas are not supported.", &[], "")?;
        return Ok((resolver, true));
    }
    Ok((resolver, false))
}
