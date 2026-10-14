use std::collections::BTreeSet;

use crate::document::EditorError;
use crate::json_value::{Arena, Value};

const QUOTE: u16 = 0x22;
const BACKSLASH: u16 = 0x5c;
const CONTROL_END: u16 = 0x1f;
const BACKSPACE: u16 = 0x08;
const TAB: u16 = 0x09;
const LINE_FEED: u16 = 0x0a;
const FORM_FEED: u16 = 0x0c;
const CARRIAGE_RETURN: u16 = 0x0d;
const ARRAY_ACTIONS: usize = 2;
const OBJECT_ACTIONS: usize = 4;
const HIGH_START: u16 = 0xd800;
const HIGH_END: u16 = 0xdbff;
const LOW_START: u16 = 0xdc00;
const LOW_END: u16 = 0xdfff;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_units: usize,
    pub max_work: usize,
    pub max_depth: usize,
}

enum Action<'a> {
    Value(usize, usize),
    String(&'a [u16]),
    Unit(u16),
    Leave(usize),
}

fn quoted(value: &[u16], limit: usize) -> Result<Vec<u16>, EditorError> {
    if value.len() > limit {
        return Err(EditorError::Capacity);
    }
    let mut result = vec![QUOTE];
    for (index, &unit) in value.iter().enumerate() {
        let paired = (HIGH_START..=HIGH_END).contains(&unit)
            && value
                .get(index + 1)
                .is_some_and(|unit| (LOW_START..=LOW_END).contains(unit))
            || (LOW_START..=LOW_END).contains(&unit)
                && index > 0
                && (HIGH_START..=HIGH_END).contains(&value[index - 1]);
        let short = match unit {
            QUOTE => Some('"'),
            BACKSLASH => Some('\\'),
            BACKSPACE => Some('b'),
            TAB => Some('t'),
            LINE_FEED => Some('n'),
            FORM_FEED => Some('f'),
            CARRIAGE_RETURN => Some('r'),
            _ => None,
        };
        if let Some(character) = short {
            result.push(BACKSLASH);
            result.extend(character.to_string().encode_utf16());
        } else if unit <= CONTROL_END || (HIGH_START..=LOW_END).contains(&unit) && !paired {
            result.extend(format!("\\u{unit:04x}").encode_utf16());
        } else {
            result.push(unit);
        }
        if result.len() >= limit {
            return Err(EditorError::Capacity);
        }
    }
    result.push(QUOTE);
    if result.len() > limit {
        return Err(EditorError::Capacity);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{Limits, stringify};
    use crate::document::EditorError;
    use crate::json_value::{Arena, Value};

    const LIMITS: Limits = Limits {
        max_units: 1024,
        max_work: 1024,
        max_depth: 1024,
    };

    #[test]
    fn stringify는_순환을_거절하고_공유참조를_반복해서_직렬화한다() {
        let cyclic = Arena {
            root: Some(0),
            nodes: vec![Value::Array(vec![0])],
        };
        assert!(matches!(
            stringify(&cyclic, 0, LIMITS),
            Err(EditorError::InvalidIdentity)
        ));
        let shared = Arena {
            root: Some(0),
            nodes: vec![Value::Array(vec![1, 1]), Value::Array(vec![2]), Value::Null],
        };
        assert_eq!(
            String::from_utf16(&stringify(&shared, 0, LIMITS).unwrap()).unwrap(),
            "[[null],[null]]"
        );
        assert!(matches!(
            stringify(&shared, shared.nodes.len(), LIMITS),
            Err(EditorError::InvalidIdentity)
        ));
    }
}

pub fn stringify(arena: &Arena, node: usize, limits: Limits) -> Result<Vec<u16>, EditorError> {
    let mut remaining = limits.max_work;
    stringify_budgeted(arena, node, limits, &mut remaining)
}

pub(crate) fn stringify_budgeted(
    arena: &Arena,
    node: usize,
    limits: Limits,
    remaining: &mut usize,
) -> Result<Vec<u16>, EditorError> {
    let mut pending = vec![Action::Value(node, 1)];
    let mut active = BTreeSet::new();
    let mut result = Vec::new();
    while let Some(action) = pending.pop() {
        *remaining = remaining.checked_sub(1).ok_or(EditorError::Capacity)?;
        if pending.len() > *remaining {
            return Err(EditorError::Capacity);
        }
        let text = match action {
            Action::String(value) => {
                *remaining = remaining
                    .checked_sub(value.len())
                    .ok_or(EditorError::Capacity)?;
                quoted(value, limits.max_units.saturating_sub(result.len()))?
            }
            Action::Unit(unit) => vec![unit],
            Action::Leave(node) => {
                active.remove(&node);
                continue;
            }
            Action::Value(node, depth) => {
                if depth > limits.max_depth {
                    return Err(EditorError::Capacity);
                }
                match arena.get(node).ok_or(EditorError::InvalidIdentity)? {
                    Value::Null => "null".encode_utf16().collect(),
                    Value::Boolean(value) => value.to_string().encode_utf16().collect(),
                    Value::Number(value) if !value.is_finite() => "null".encode_utf16().collect(),
                    Value::Number(value) => crate::json_symbols::number_string(*value)
                        .encode_utf16()
                        .collect(),
                    Value::String(value) => {
                        *remaining = remaining
                            .checked_sub(value.len())
                            .ok_or(EditorError::Capacity)?;
                        quoted(value, limits.max_units.saturating_sub(result.len()))?
                    }
                    Value::Array(items) => {
                        if !active.insert(node) {
                            return Err(EditorError::InvalidIdentity);
                        }
                        if items
                            .len()
                            .saturating_mul(ARRAY_ACTIONS)
                            .saturating_add(ARRAY_ACTIONS)
                            .saturating_add(pending.len())
                            > *remaining
                        {
                            return Err(EditorError::Capacity);
                        }
                        pending.push(Action::Leave(node));
                        pending.push(Action::Unit(u16::from(b']')));
                        for (index, &item) in items.iter().enumerate().rev() {
                            pending.push(Action::Value(item, depth + 1));
                            if index > 0 {
                                pending.push(Action::Unit(u16::from(b',')));
                            }
                        }
                        "[".encode_utf16().collect()
                    }
                    Value::Object(members) => {
                        if !active.insert(node) {
                            return Err(EditorError::InvalidIdentity);
                        }
                        if members
                            .len()
                            .saturating_mul(OBJECT_ACTIONS)
                            .saturating_add(ARRAY_ACTIONS)
                            .saturating_add(pending.len())
                            > *remaining
                        {
                            return Err(EditorError::Capacity);
                        }
                        pending.push(Action::Leave(node));
                        pending.push(Action::Unit(u16::from(b'}')));
                        for (index, member) in members.iter().enumerate().rev() {
                            pending.push(Action::Value(member.value, depth + 1));
                            pending.push(Action::Unit(u16::from(b':')));
                            pending.push(Action::String(&member.key));
                            if index > 0 {
                                pending.push(Action::Unit(u16::from(b',')));
                            }
                        }
                        "{".encode_utf16().collect()
                    }
                }
            }
        };
        if result
            .len()
            .checked_add(text.len())
            .is_none_or(|size| size > limits.max_units)
        {
            return Err(EditorError::Capacity);
        }
        result.extend(text);
    }
    Ok(result)
}
