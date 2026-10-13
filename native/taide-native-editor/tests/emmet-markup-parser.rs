use taide_native_editor::document::EditorError;
use taide_native_editor::emmet::{MarkupLimits, MarkupNode, Repeat, Syntax, parse_markup};

const LIMITS: MarkupLimits = MarkupLimits {
    max_bytes: 4096,
    max_nodes: 256,
    max_nesting: 32,
};

include!("fixtures/emmet-markup-parser-oracle.rs");

fn part(value: Option<&str>) -> String {
    value.map_or_else(|| "-".into(), |value| format!("{}:{value}", value.len()))
}

fn canonical(nodes: &[MarkupNode]) -> String {
    nodes
        .iter()
        .map(|node| {
            let attributes = node
                .attributes
                .iter()
                .map(|attribute| {
                    format!(
                        "{}{}x{}m{}",
                        part(attribute.name.as_deref()),
                        part(attribute.value.as_deref()),
                        usize::from(attribute.is_expression),
                        usize::from(attribute.is_multiple)
                    )
                })
                .collect::<String>();
            let repeat = match node.repeat {
                Some(Repeat::Implicit) => "i".into(),
                Some(Repeat::Count(count)) => count.to_string(),
                None => "-".into(),
            };
            format!(
                "{}{}{}r{repeat}s{}a[{attributes}]c[{}]",
                if node.is_group { "g" } else { "e" },
                part(node.name.as_deref()),
                part(node.text.as_deref()),
                usize::from(node.self_closing),
                canonical(&node.children)
            )
        })
        .collect()
}

#[test]
fn emmet_markup의_구조는_원본_token_tree의_두syntax와_일치한다() {
    let mismatches = CASES
        .iter()
        .filter_map(|&(syntax, input, expected)| {
            let syntax = if syntax == "jsx" {
                Syntax::Jsx
            } else {
                Syntax::Html
            };
            let actual = parse_markup(input, syntax, LIMITS)
                .ok()
                .map(|nodes| canonical(&nodes));
            (actual.as_deref() != expected)
                .then(|| format!("{syntax:?} {input:?}: actual={actual:?}, expected={expected:?}"))
        })
        .collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn emmet_markup의_입력_노드_깊이_예산과_css거절은_고정된다() {
    assert_eq!(
        parse_markup("div", Syntax::Css, LIMITS),
        Err(EditorError::Refused)
    );
    assert_eq!(
        parse_markup(
            "div",
            Syntax::Html,
            MarkupLimits {
                max_bytes: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        parse_markup(
            "div+span",
            Syntax::Html,
            MarkupLimits {
                max_nodes: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        parse_markup(
            "div>span",
            Syntax::Html,
            MarkupLimits {
                max_nesting: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        parse_markup(
            "((div))",
            Syntax::Html,
            MarkupLimits {
                max_nesting: 1,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
    assert_eq!(
        parse_markup(
            "div{a{b}c}",
            Syntax::Html,
            MarkupLimits {
                max_nesting: 0,
                ..LIMITS
            }
        ),
        Err(EditorError::Capacity)
    );
}
