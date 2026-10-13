use taide_native_editor::emmet::{StylesheetProperty, StylesheetToken, StylesheetValue};

fn part(value: Option<&str>) -> String {
    value.map_or_else(|| "-".into(), |value| format!("{}:{value}", value.len()))
}

fn number(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    value.to_string()
}

fn values(fragments: &[Vec<StylesheetToken>]) -> String {
    fragments
        .iter()
        .map(|fragment| {
            let text = fragment
                .iter()
                .map(|token| match &token.value {
                    StylesheetValue::Literal(value) => format!("l{}", part(Some(value))),
                    StylesheetValue::CustomProperty(value) => format!("c{}", part(Some(value))),
                    StylesheetValue::Number { value, raw, unit } => format!(
                        "n{}:{}{}",
                        number(*value),
                        part(Some(raw)),
                        part(Some(unit))
                    ),
                    StylesheetValue::Color {
                        red,
                        green,
                        blue,
                        alpha,
                        raw,
                    } => format!(
                        "r{red}:{green}:{blue}:{}:{}",
                        number(*alpha),
                        part(Some(raw))
                    ),
                    StylesheetValue::String { value, quote } => {
                        let quote = if *quote == '\'' { "single" } else { "double" };
                        format!("s{quote}:{}", part(Some(value)))
                    }
                    StylesheetValue::Field {
                        index,
                        name,
                        is_adjacent,
                    } => format!(
                        "f{}:{}:{}",
                        index.map_or_else(|| "-".into(), |index| index.to_string()),
                        usize::from(*is_adjacent),
                        part(Some(name))
                    ),
                    StylesheetValue::Function { name, arguments } => {
                        format!("x{}[{}]", part(Some(name)), values(arguments))
                    }
                })
                .collect::<String>();
            format!("[{text}]")
        })
        .collect()
}

pub fn canonical(properties: &[StylesheetProperty]) -> String {
    properties
        .iter()
        .map(|property| {
            format!(
                "{}i{}[{}]",
                part(property.name.as_deref()),
                usize::from(property.is_important),
                values(&property.values)
            )
        })
        .collect()
}
