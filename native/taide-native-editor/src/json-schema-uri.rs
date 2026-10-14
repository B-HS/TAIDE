use crate::document::EditorError;
use crate::json_parser::{Document, Kind, Value};

const COLON: u16 = 0x3a;
const SLASH: u16 = 0x2f;
const QUESTION: u16 = 0x3f;
const HASH: u16 = 0x23;
const PERCENT: u16 = 0x25;
const AT: u16 = 0x40;
const ESCAPE_UNITS: usize = 3;
const HEX_RADIX: u8 = 16;
const HEX_DIGITS: &[u8] = b"0123456789ABCDEF";
const DOT: u16 = 0x2e;
const PROPERTY_VALUE_CHILDREN: usize = 2;
const FILE: &[u16] = &[0x66, 0x69, 0x6c, 0x65];
const HTTP: &[u16] = &[0x68, 0x74, 0x74, 0x70];
const HTTPS: &[u16] = &[0x68, 0x74, 0x74, 0x70, 0x73];
const HASH_ESCAPE: &[u16] = &[PERCENT, 0x32, 0x33];
const QUESTION_ESCAPE: &[u16] = &[PERCENT, 0x33, 0x46];

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_input_units: usize,
    pub max_output_units: usize,
    pub max_work: usize,
}

struct Budget {
    limits: Limits,
    work: usize,
}

impl Budget {
    fn charge(&mut self, units: usize) -> Result<(), EditorError> {
        self.work = self.work.checked_add(units).ok_or(EditorError::Capacity)?;
        if self.work > self.limits.max_work {
            return Err(EditorError::Capacity);
        }
        Ok(())
    }

    fn append(&mut self, target: &mut Vec<u16>, value: &[u16]) -> Result<(), EditorError> {
        self.charge(value.len())?;
        let length = target
            .len()
            .checked_add(value.len())
            .ok_or(EditorError::Capacity)?;
        if length > self.limits.max_output_units {
            return Err(EditorError::Capacity);
        }
        target.extend_from_slice(value);
        Ok(())
    }

    fn escaped(&mut self, target: &mut Vec<u16>, value: &[u16]) -> Result<(), EditorError> {
        for &unit in value {
            match unit {
                HASH => self.append(target, HASH_ESCAPE)?,
                QUESTION => self.append(target, QUESTION_ESCAPE)?,
                _ => self.append(target, &[unit])?,
            }
        }
        Ok(())
    }

    fn encoded(
        &mut self,
        target: &mut Vec<u16>,
        value: &[u16],
        is_path: bool,
        is_authority: bool,
        is_component: bool,
    ) -> Result<(), EditorError> {
        self.charge(value.len())?;
        for character in char::decode_utf16(value.iter().copied()) {
            let character = character.map_err(|_| EditorError::InvalidBoundary)?;
            let is_safe = character.is_ascii_alphanumeric()
                || matches!(character, '-' | '.' | '_' | '~')
                || is_path && character == '/'
                || is_authority && matches!(character, '[' | ']' | ':')
                || is_component && matches!(character, '!' | '\'' | '(' | ')' | '*');
            if is_safe {
                let mut units = [0; char::MAX_LEN_UTF16];
                self.append(target, character.encode_utf16(&mut units))?;
                continue;
            }
            let mut bytes = [0; char::MAX_LEN_UTF8];
            for &byte in character.encode_utf8(&mut bytes).as_bytes() {
                self.append(
                    target,
                    &[
                        PERCENT,
                        u16::from(HEX_DIGITS[usize::from(byte / HEX_RADIX)]),
                        u16::from(HEX_DIGITS[usize::from(byte % HEX_RADIX)]),
                    ],
                )?;
            }
        }
        Ok(())
    }

    fn format_component(
        &mut self,
        target: &mut Vec<u16>,
        value: &[u16],
        should_encode: bool,
        is_path: bool,
        is_authority: bool,
    ) -> Result<(), EditorError> {
        if should_encode {
            return self.encoded(target, value, is_path, is_authority, false);
        }
        self.escaped(target, value)
    }
}

struct Parts<'a> {
    scheme: &'a [u16],
    authority: Vec<u16>,
    path: Vec<u16>,
    query: Vec<u16>,
    fragment: Vec<u16>,
}

fn is_word(unit: u16) -> bool {
    matches!(unit, 0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a | 0x5f)
}

fn is_alphanumeric(unit: u16) -> bool {
    matches!(unit, 0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a)
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

fn decode(input: &[u16], budget: &mut Budget) -> Result<Vec<u16>, EditorError> {
    budget.charge(input.len())?;
    let mut result = Vec::new();
    let mut cursor = 0;
    while cursor < input.len() {
        let start = cursor;
        let mut bytes = Vec::new();
        let mut suffix_start = 0;
        while cursor + ESCAPE_UNITS <= input.len()
            && input[cursor] == PERCENT
            && is_alphanumeric(input[cursor + 1])
            && is_alphanumeric(input[cursor + 2])
        {
            let byte = hex(input[cursor + 1])
                .zip(hex(input[cursor + 2]))
                .map(|(high, low)| high * HEX_RADIX + low);
            bytes.push(byte.unwrap_or(0));
            if byte.is_none() {
                suffix_start = bytes.len();
            }
            cursor += ESCAPE_UNITS;
        }
        if cursor == start {
            result.push(input[cursor]);
            cursor += 1;
            continue;
        }
        loop {
            match std::str::from_utf8(&bytes[suffix_start..]) {
                Ok(text) => {
                    result.extend_from_slice(&input[start..start + suffix_start * ESCAPE_UNITS]);
                    result.extend(text.encode_utf16());
                    break;
                }
                Err(error) => {
                    suffix_start += error.valid_up_to();
                    suffix_start += error.error_len().unwrap_or(bytes.len() - suffix_start);
                }
            }
        }
    }
    Ok(result)
}

fn lower(input: &[u16], budget: &mut Budget) -> Result<Vec<u16>, EditorError> {
    budget.charge(input.len())?;
    let mut result = Vec::new();
    let mut segment = String::new();
    for unit in char::decode_utf16(input.iter().copied()) {
        match unit {
            Ok(character) => segment.push(character),
            Err(error) => {
                result.extend(segment.to_lowercase().encode_utf16());
                segment.clear();
                result.push(error.unpaired_surrogate());
            }
        }
    }
    result.extend(segment.to_lowercase().encode_utf16());
    Ok(result)
}

fn parse<'a>(input: &'a [u16], budget: &mut Budget) -> Result<Parts<'a>, EditorError> {
    if input.len() > budget.limits.max_input_units {
        return Err(EditorError::Capacity);
    }
    budget.charge(input.len())?;
    let mut cursor = 0;
    let scheme_end = input
        .iter()
        .position(|&unit| matches!(unit, COLON | SLASH | QUESTION | HASH));
    let scheme = match scheme_end {
        Some(end) if end > 0 && input[end] == COLON => {
            let scheme = &input[..end];
            if !is_word(scheme[0])
                || !scheme
                    .iter()
                    .all(|&unit| is_word(unit) || matches!(unit, 0x2b | 0x2e | 0x2d))
            {
                return Err(EditorError::InvalidBoundary);
            }
            cursor = end + 1;
            scheme
        }
        _ => FILE,
    };
    let authority = if input.get(cursor..cursor + 2) == Some(&[SLASH, SLASH]) {
        cursor += 2;
        let end = input[cursor..]
            .iter()
            .position(|&unit| matches!(unit, SLASH | QUESTION | HASH))
            .map_or(input.len(), |end| cursor + end);
        let value = decode(&input[cursor..end], budget)?;
        cursor = end;
        value
    } else {
        Vec::new()
    };
    let end = input[cursor..]
        .iter()
        .position(|&unit| matches!(unit, QUESTION | HASH))
        .map_or(input.len(), |end| cursor + end);
    let mut path = decode(&input[cursor..end], budget)?;
    cursor = end;
    if [FILE, HTTP, HTTPS].contains(&scheme) && path.first() != Some(&SLASH) {
        path.insert(0, SLASH);
    }
    if !path.is_empty()
        && (!authority.is_empty() && path[0] != SLASH
            || authority.is_empty() && path.starts_with(&[SLASH, SLASH]))
    {
        return Err(EditorError::InvalidBoundary);
    }
    let query = if input.get(cursor) == Some(&QUESTION) {
        cursor += 1;
        let end = input[cursor..]
            .iter()
            .position(|&unit| unit == HASH)
            .map_or(input.len(), |end| cursor + end);
        let value = decode(&input[cursor..end], budget)?;
        cursor = end;
        value
    } else {
        Vec::new()
    };
    let fragment = if input.get(cursor) == Some(&HASH) {
        cursor += 1;
        let end = input[cursor..]
            .iter()
            .position(|&unit| matches!(unit, 0x0a | 0x0d | 0x2028 | 0x2029))
            .map_or(input.len(), |end| cursor + end);
        decode(&input[cursor..end], budget)?
    } else {
        Vec::new()
    };
    Ok(Parts {
        scheme,
        authority,
        path,
        query,
        fragment,
    })
}

fn format(
    parts: Parts<'_>,
    should_encode: bool,
    is_resource: bool,
    budget: &mut Budget,
) -> Result<Vec<u16>, EditorError> {
    let Parts {
        scheme,
        authority,
        mut path,
        query,
        fragment,
    } = parts;
    let mut result = Vec::new();
    budget.append(&mut result, scheme)?;
    budget.append(&mut result, &[COLON])?;
    if !authority.is_empty() || scheme == FILE {
        budget.append(&mut result, &[SLASH, SLASH])?;
    }
    if !authority.is_empty() {
        let host_start = if let Some(at) = authority.iter().position(|&unit| unit == AT) {
            let userinfo = &authority[..at];
            if let Some(colon) = userinfo.iter().rposition(|&unit| unit == COLON) {
                budget.format_component(
                    &mut result,
                    &userinfo[..colon],
                    should_encode,
                    false,
                    false,
                )?;
                budget.append(&mut result, &[COLON])?;
                budget.format_component(
                    &mut result,
                    &userinfo[colon + 1..],
                    should_encode,
                    false,
                    true,
                )?;
            } else {
                budget.format_component(&mut result, userinfo, should_encode, false, false)?;
            }
            budget.append(&mut result, &[AT])?;
            at + 1
        } else {
            0
        };
        let host = lower(&authority[host_start..], budget)?;
        let port_start = host.iter().rposition(|&unit| unit == COLON);
        let host_end = port_start.unwrap_or(host.len());
        budget.format_component(&mut result, &host[..host_end], should_encode, false, true)?;
        budget.append(&mut result, &host[host_end..])?;
    }
    let drive = if path.len() >= ESCAPE_UNITS && path[0] == SLASH && path[2] == COLON {
        Some(1)
    } else if path.len() >= 2 && path[1] == COLON {
        Some(0)
    } else {
        None
    };
    if let Some(position) = drive.filter(|&position| (0x41..=0x5a).contains(&path[position])) {
        path[position] += 0x20;
    }
    budget.format_component(&mut result, &path, should_encode, true, false)?;
    if !is_resource && !query.is_empty() {
        budget.append(&mut result, &[QUESTION])?;
        budget.format_component(&mut result, &query, should_encode, false, false)?;
    }
    if !is_resource && !fragment.is_empty() {
        budget.append(&mut result, &[HASH])?;
        if should_encode {
            budget.encoded(&mut result, &fragment, false, false, false)?;
        } else {
            budget.append(&mut result, &fragment)?;
        }
    }
    Ok(result)
}

fn normalize(input: &[u16], is_resource: bool, limits: Limits) -> Result<Vec<u16>, EditorError> {
    let mut budget = Budget { limits, work: 0 };
    match parse(input, &mut budget) {
        Ok(parts) => format(parts, false, is_resource, &mut budget),
        Err(EditorError::InvalidBoundary) => {
            let mut result = Vec::new();
            budget.append(&mut result, input)?;
            Ok(result)
        }
        Err(error) => Err(error),
    }
}

fn context_path(
    relative: &[u16],
    resource: &[u16],
    budget: &mut Budget,
) -> Result<Vec<u16>, EditorError> {
    let total = relative
        .len()
        .checked_add(resource.len())
        .ok_or(EditorError::Capacity)?;
    if total > budget.limits.max_input_units {
        return Err(EditorError::Capacity);
    }
    budget.charge(total)?;
    let base_end = resource
        .iter()
        .rposition(|&unit| unit == SLASH)
        .map_or(0, |position| position + 1);
    let mut parts = parse(&resource[..base_end], budget)?;
    let is_absolute = relative.first() == Some(&SLASH);
    let source = if is_absolute {
        relative.split(|&unit| unit == SLASH).collect::<Vec<_>>()
    } else {
        parts
            .path
            .split(|&unit| unit == SLASH)
            .chain(relative.split(|&unit| unit == SLASH))
            .collect::<Vec<_>>()
    };
    let mut normalized = Vec::new();
    for &segment in &source {
        budget.charge(segment.len().checked_add(1).ok_or(EditorError::Capacity)?)?;
        if segment.is_empty() || segment == [DOT] {
            continue;
        }
        if segment == [DOT, DOT] {
            normalized.pop();
            continue;
        }
        normalized.push(segment);
    }
    if source.len() > 1 && source.last().is_some_and(|segment| segment.is_empty()) {
        normalized.push(&[]);
    }
    let mut path = Vec::new();
    if source.first().is_some_and(|segment| segment.is_empty()) {
        budget.append(&mut path, &[SLASH])?;
    }
    for (index, &segment) in normalized.iter().enumerate() {
        if index > 0 {
            budget.append(&mut path, &[SLASH])?;
        }
        budget.append(&mut path, segment)?;
    }
    if [FILE, HTTP, HTTPS].contains(&parts.scheme) && path.first() != Some(&SLASH) {
        path.insert(0, SLASH);
    }
    if !path.is_empty()
        && (!parts.authority.is_empty() && path[0] != SLASH
            || parts.authority.is_empty() && path.starts_with(&[SLASH, SLASH]))
    {
        return Err(EditorError::InvalidBoundary);
    }
    parts.path = path;
    format(parts, true, false, budget)
}

pub fn resolve_relative_path(
    relative: &[u16],
    resource: &[u16],
    limits: Limits,
) -> Result<Vec<u16>, EditorError> {
    context_path(relative, resource, &mut Budget { limits, work: 0 })
}

pub fn schema_property(
    document: &Document,
    resource: &[u16],
    limits: Limits,
) -> Result<Option<Vec<u16>>, EditorError> {
    let Some(index) = document.root else {
        return Ok(None);
    };
    let root = document
        .nodes
        .get(index)
        .ok_or(EditorError::InvalidBoundary)?;
    if root.kind != Kind::Object {
        return Ok(None);
    }
    let mut budget = Budget { limits, work: 0 };
    for &index in &root.children {
        budget.charge(1)?;
        let property = document
            .nodes
            .get(index)
            .ok_or(EditorError::InvalidBoundary)?;
        if property.kind != Kind::Property || property.children.len() != PROPERTY_VALUE_CHILDREN {
            continue;
        }
        let key = document
            .nodes
            .get(property.children[0])
            .ok_or(EditorError::InvalidBoundary)?;
        let Value::String(key) = &key.value else {
            continue;
        };
        budget.charge(key.len())?;
        if !key.iter().copied().eq("$schema".encode_utf16()) {
            continue;
        }
        let value = document
            .nodes
            .get(property.children[1])
            .ok_or(EditorError::InvalidBoundary)?;
        let Value::String(value) = &value.value else {
            continue;
        };
        if value.len() > limits.max_input_units {
            return Err(EditorError::Capacity);
        }
        budget.charge(value.len())?;
        let end = value
            .iter()
            .position(|&unit| !is_word(unit) && !matches!(unit, 0x2b | 0x2e | 0x2d));
        let has_scheme = value.first().is_some_and(|&unit| is_word(unit))
            && end.is_some_and(|end| value[end] == COLON);
        if has_scheme {
            let mut result = Vec::new();
            budget.append(&mut result, value)?;
            return Ok(Some(result));
        }
        return context_path(value, resource, &mut budget).map(Some);
    }
    Ok(None)
}

pub fn normalize_id(input: &[u16], limits: Limits) -> Result<Vec<u16>, EditorError> {
    normalize(input, false, limits)
}

pub fn normalize_resource(input: &[u16], limits: Limits) -> Result<Vec<u16>, EditorError> {
    normalize(input, true, limits)
}

pub fn encode_component(input: &[u16], limits: Limits) -> Result<Vec<u16>, EditorError> {
    if input.len() > limits.max_input_units {
        return Err(EditorError::Capacity);
    }
    let mut result = Vec::new();
    Budget { limits, work: 0 }.encoded(&mut result, input, false, false, true)?;
    Ok(result)
}

pub fn scheme(input: &[u16], limits: Limits) -> Result<Vec<u16>, EditorError> {
    let mut budget = Budget { limits, work: 0 };
    let parts = parse(input, &mut budget)?;
    let mut result = Vec::new();
    budget.append(&mut result, parts.scheme)?;
    Ok(result)
}
