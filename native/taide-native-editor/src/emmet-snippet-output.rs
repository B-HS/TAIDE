use crate::document::EditorError;

const UTF8_MAX_BYTES: usize = 4;

pub(crate) fn placeholder(
    name: &str,
    max_nesting: usize,
    mut push: impl FnMut(&str) -> Result<(), EditorError>,
) -> Result<(), EditorError> {
    let mut characters = name.chars().peekable();
    let mut braces = Vec::new();
    let mut is_field_open = false;
    while let Some(character) = characters.next() {
        if character == '\\' {
            push("\\")?;
            if let Some(next) = characters.next() {
                push(next.encode_utf8(&mut [0; UTF8_MAX_BYTES]))?;
            }
            continue;
        }
        if character == '$' {
            if characters.peek() == Some(&'{') {
                is_field_open = true;
            } else {
                push("\\")?;
            }
        }
        if character == '{' {
            if braces.len() >= max_nesting {
                return Err(EditorError::Capacity);
            }
            braces.push(is_field_open);
            is_field_open = false;
        }
        if character == '}' && !braces.pop().unwrap_or(false) {
            push("\\")?;
        }
        push(character.encode_utf8(&mut [0; UTF8_MAX_BYTES]))?;
    }
    Ok(())
}
