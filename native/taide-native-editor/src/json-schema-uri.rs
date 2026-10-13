use crate::document::EditorError;

const COLON: u16 = 0x3a;
const SLASH: u16 = 0x2f;
const QUESTION: u16 = 0x3f;
const HASH: u16 = 0x23;
const PERCENT: u16 = 0x25;
const AT: u16 = 0x40;
const ESCAPE_UNITS: usize = 3;
const HEX_RADIX: u8 = 16;
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

fn normalize(input: &[u16], is_resource: bool, limits: Limits) -> Result<Vec<u16>, EditorError> {
    if input.len() > limits.max_input_units {
        return Err(EditorError::Capacity);
    }
    let mut budget = Budget { limits, work: 0 };
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
                let mut result = Vec::new();
                budget.append(&mut result, input)?;
                return Ok(result);
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
        let value = decode(&input[cursor..end], &mut budget)?;
        cursor = end;
        value
    } else {
        Vec::new()
    };
    let end = input[cursor..]
        .iter()
        .position(|&unit| matches!(unit, QUESTION | HASH))
        .map_or(input.len(), |end| cursor + end);
    let mut path = decode(&input[cursor..end], &mut budget)?;
    cursor = end;
    if [FILE, HTTP, HTTPS].contains(&scheme) && path.first() != Some(&SLASH) {
        path.insert(0, SLASH);
    }
    if !path.is_empty()
        && (!authority.is_empty() && path[0] != SLASH
            || authority.is_empty() && path.starts_with(&[SLASH, SLASH]))
    {
        let mut result = Vec::new();
        budget.append(&mut result, input)?;
        return Ok(result);
    }
    let query = if input.get(cursor) == Some(&QUESTION) {
        cursor += 1;
        let end = input[cursor..]
            .iter()
            .position(|&unit| unit == HASH)
            .map_or(input.len(), |end| cursor + end);
        let value = decode(&input[cursor..end], &mut budget)?;
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
        decode(&input[cursor..end], &mut budget)?
    } else {
        Vec::new()
    };
    let mut result = Vec::new();
    budget.append(&mut result, scheme)?;
    budget.append(&mut result, &[COLON])?;
    if !authority.is_empty() || scheme == FILE {
        budget.append(&mut result, &[SLASH, SLASH])?;
    }
    if !authority.is_empty() {
        let host_start = if let Some(at) = authority.iter().position(|&unit| unit == AT) {
            budget.escaped(&mut result, &authority[..at])?;
            budget.append(&mut result, &[AT])?;
            at + 1
        } else {
            0
        };
        let host = lower(&authority[host_start..], &mut budget)?;
        let port_start = host.iter().rposition(|&unit| unit == COLON);
        let host_end = port_start.unwrap_or(host.len());
        budget.escaped(&mut result, &host[..host_end])?;
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
    budget.escaped(&mut result, &path)?;
    if !is_resource && !query.is_empty() {
        budget.append(&mut result, &[QUESTION])?;
        budget.escaped(&mut result, &query)?;
    }
    if !is_resource && !fragment.is_empty() {
        budget.append(&mut result, &[HASH])?;
        budget.append(&mut result, &fragment)?;
    }
    Ok(result)
}

pub fn normalize_id(input: &[u16], limits: Limits) -> Result<Vec<u16>, EditorError> {
    normalize(input, false, limits)
}

pub fn normalize_resource(input: &[u16], limits: Limits) -> Result<Vec<u16>, EditorError> {
    normalize(input, true, limits)
}
