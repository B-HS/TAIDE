const SHORT_HEX_NO_ALPHA_LENGTH: usize = 3;
const SHORT_HEX_WITH_ALPHA_LENGTH: usize = 4;

fn strip_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    let mut is_in_string = false;
    let mut is_escaped = false;
    while let Some(character) = characters.next() {
        if is_in_string {
            result.push(character);
            if is_escaped {
                is_escaped = false;
            } else if character == '\\' {
                is_escaped = true;
            } else if character == '"' {
                is_in_string = false;
            }
            continue;
        }
        match (character, characters.peek()) {
            ('"', _) => {
                is_in_string = true;
                result.push(character);
            }
            ('/', Some('/')) => {
                characters.next();
                for skipped in characters.by_ref() {
                    if skipped == '\n' {
                        result.push(skipped);
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                characters.next();
                let mut previous = None;
                for skipped in characters.by_ref() {
                    if previous == Some('*') && skipped == '/' {
                        break;
                    }
                    previous = Some(skipped);
                }
            }
            _ => result.push(character),
        }
    }
    result
}

fn strip_trailing_commas(source: &str) -> String {
    let characters = source.chars().collect::<Vec<_>>();
    let mut result = String::with_capacity(source.len());
    let mut is_in_string = false;
    let mut is_escaped = false;
    for (index, character) in characters.iter().copied().enumerate() {
        if is_in_string {
            if is_escaped {
                is_escaped = false;
            } else if character == '\\' {
                is_escaped = true;
            } else if character == '"' {
                is_in_string = false;
            }
            result.push(character);
            continue;
        }
        if character == '"' {
            is_in_string = true;
        }
        let is_trailing_comma = character == ','
            && characters[index + 1..]
                .iter()
                .find(|next| !next.is_whitespace())
                .is_some_and(|next| matches!(next, '}' | ']'));
        if !is_trailing_comma {
            result.push(character);
        }
    }
    result
}

/// Removes `//` and `/* */` comments and trailing commas outside JSON strings.
pub fn strip_json_comments(source: &str) -> String {
    strip_trailing_commas(&strip_comments(source))
}

/// Parses VS Code's JSON-with-comments theme format.
pub fn parse_jsonc(source: &str) -> serde_json::Result<serde_json::Value> {
    serde_json::from_str(&strip_json_comments(source))
}

/// Expands VS Code's `#rgb` and `#rgba` shorthand to the 6- and 8-digit forms.
pub fn expand_vscode_hex(value: &str) -> String {
    let Some(hex) = value.strip_prefix('#') else {
        return value.into();
    };
    let length = super::color::js_length(hex);
    if length != SHORT_HEX_NO_ALPHA_LENGTH && length != SHORT_HEX_WITH_ALPHA_LENGTH {
        return value.into();
    }
    hex.chars().fold(String::from("#"), |mut expanded, digit| {
        expanded.push(digit);
        expanded.push(digit);
        expanded
    })
}
