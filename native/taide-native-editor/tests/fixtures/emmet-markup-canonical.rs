use taide_native_editor::emmet::{ExpandedMarkupNode, MarkupValue, MarkupValueType};

fn part(value: Option<&str>) -> String {
    value.map_or_else(|| "-".into(), |value| format!("{}:{value}", value.len()))
}

fn values(value: Option<&[MarkupValue]>) -> String {
    let Some(value) = value else {
        return "-".into();
    };
    format!(
        "[{}]",
        value
            .iter()
            .map(|token| match token {
                MarkupValue::Text(text) => format!("s{}", part(Some(text))),
                MarkupValue::Field { index, name } => format!("f{index}:{}", part(Some(name))),
            })
            .collect::<String>()
    )
}

pub fn canonical(nodes: &[ExpandedMarkupNode]) -> String {
    nodes
        .iter()
        .map(|node| {
            let attributes = node.attributes.as_ref().map_or_else(
                || "-".into(),
                |attributes| {
                    format!(
                        "[{}]",
                        attributes
                            .iter()
                            .map(|attribute| {
                                let kind = match attribute.value_type {
                                    MarkupValueType::Raw => "raw",
                                    MarkupValueType::SingleQuote => "singleQuote",
                                    MarkupValueType::DoubleQuote => "doubleQuote",
                                    MarkupValueType::Expression => "expression",
                                };
                                format!(
                                    "{}{}t{kind}b{}i{}m{}",
                                    part(attribute.name.as_deref()),
                                    values(attribute.value.as_deref()),
                                    usize::from(attribute.is_boolean),
                                    usize::from(attribute.is_implied),
                                    usize::from(attribute.is_multiple)
                                )
                            })
                            .collect::<String>()
                    )
                },
            );
            let repeat = node.repeat.map_or_else(
                || "-".into(),
                |repeat| {
                    format!(
                        "{}:{}:{}",
                        repeat.count,
                        repeat.index,
                        usize::from(repeat.is_implicit)
                    )
                },
            );
            format!(
                "{}{}r{repeat}s{}a{attributes}c[{}]",
                part(node.name.as_deref()),
                values(node.value.as_deref()),
                usize::from(node.self_closing),
                canonical(&node.children)
            )
        })
        .collect()
}
