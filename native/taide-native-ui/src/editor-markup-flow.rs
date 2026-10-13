use taide_native_editor::documentation::{Block, Inline};

use super::{HEADING_BOTTOM, HEADING_SIZES, HEADING_TOP, PARAGRAPH_MARGIN, RULE_MARGIN};

#[derive(Clone, Copy, Default)]
pub(super) struct Edges {
    pub top: f32,
    pub bottom: f32,
    pub through: bool,
}

pub(super) fn has_content(contents: &[Inline]) -> bool {
    contents.iter().any(|inline| match inline {
        Inline::Text(span) => span
            .text
            .chars()
            .any(|character| !character.is_ascii_whitespace()),
        Inline::Image { .. } | Inline::Break => true,
    })
}

pub(super) fn edges(block: &Block) -> Edges {
    match block {
        Block::Inline(contents) => Edges {
            through: !has_content(contents),
            ..Default::default()
        },
        Block::Paragraph(contents) => Edges {
            top: PARAGRAPH_MARGIN,
            bottom: PARAGRAPH_MARGIN,
            through: !has_content(contents),
        },
        Block::Heading { level, contents } => {
            let index = usize::from(level.saturating_sub(1));
            Edges {
                top: HEADING_TOP.get(index).copied().unwrap_or_default(),
                bottom: if HEADING_SIZES.get(index).is_some() {
                    HEADING_BOTTOM
                } else {
                    0.0
                },
                through: !has_content(contents),
            }
        }
        Block::Code { .. } => Edges {
            top: PARAGRAPH_MARGIN,
            bottom: PARAGRAPH_MARGIN,
            through: false,
        },
        Block::Rule => Edges {
            top: RULE_MARGIN,
            bottom: RULE_MARGIN,
            through: false,
        },
        Block::Quote(contents) => group(contents),
        Block::List { items, .. } => {
            let top = items
                .first()
                .map(|item| group(&item.blocks).top)
                .unwrap_or_default();
            let bottom = items
                .last()
                .map(|item| group(&item.blocks).bottom)
                .unwrap_or_default();
            Edges {
                top,
                bottom,
                through: items.is_empty(),
            }
        }
        Block::Table { .. } => Edges::default(),
    }
}

pub(super) fn group(contents: &[Block]) -> Edges {
    let mut result = Edges {
        through: true,
        ..Default::default()
    };
    for block in contents {
        let edges = edges(block);
        if result.through {
            result.top = result.top.max(edges.top);
            if edges.through {
                result.top = result.top.max(edges.bottom)
            }
        }
        if edges.through {
            result.bottom = result.bottom.max(edges.top).max(edges.bottom);
        } else {
            result.through = false;
            result.bottom = edges.bottom;
        }
    }
    if result.through {
        result.bottom = result.top.max(result.bottom);
        result.top = result.bottom
    }
    result
}
