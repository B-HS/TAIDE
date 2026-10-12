use std::mem::size_of;

use dom_query::NodeRef;
use taide_model::error::AppResult;
use taide_native_editor::documentation::{Alignment, Block, Inline, TableCell, TableLayout};
use url::Url;

use super::{BLOCK_TAGS, blocks, contents, name};

const MAX_COLUMN_SPAN: usize = 1000;
const MAX_ROW_SPAN: usize = 65534;
const DECIMAL_BASE: usize = 10;
const COLUMN_BYTES: usize = size_of::<usize>() + size_of::<[f32; 4]>() + size_of::<Alignment>();
const ROW_BYTES: usize =
    size_of::<Vec<TableCell>>() + size_of::<Vec<Vec<Inline>>>() + size_of::<[f32; 2]>();

fn span(node: NodeRef<'_>, attribute: &str, maximum: usize) -> Option<usize> {
    let value = node.attr(attribute)?;
    let value = value
        .trim_start_matches(|character| matches!(character, '\t' | '\n' | '\u{c}' | '\r' | ' '));
    let value = value.strip_prefix('+').unwrap_or(value);
    let length = value.bytes().take_while(u8::is_ascii_digit).count();
    if length == 0 {
        return None;
    }
    Some(value[..length].bytes().fold(0usize, |value, digit| {
        value
            .saturating_mul(DECIMAL_BASE)
            .saturating_add(usize::from(digit - b'0'))
            .min(maximum)
    }))
}

fn budget(rows: usize, columns: usize, cells: usize) -> AppResult<()> {
    let bytes = rows
        .checked_mul(ROW_BYTES)
        .and_then(|bytes| {
            columns
                .checked_mul(COLUMN_BYTES)
                .and_then(|columns| bytes.checked_add(columns))
        })
        .and_then(|bytes| {
            cells
                .checked_mul(size_of::<TableCell>())
                .and_then(|cells| bytes.checked_add(cells))
        });
    if bytes.is_none_or(|bytes| bytes > crate::preview_web_document::MAX_OUTPUT_BYTES) {
        return Err(super::super::limit_error());
    }
    Ok(())
}

pub(super) fn parse(node: NodeRef<'_>, base: Option<&Url>, bold: bool) -> AppResult<Block> {
    let caption = node
        .children_it(false)
        .find(|child| name(*child) == "caption" && !child.has_attr("hidden"))
        .map(|caption| contents(caption, base, bold))
        .unwrap_or_default();
    let mut head = Vec::new();
    let mut body = Vec::new();
    let mut foot = Vec::new();
    let mut direct = Vec::new();
    for child in node.children_it(false) {
        if child.has_attr("hidden") {
            continue;
        }
        let rows = || {
            child
                .children_it(false)
                .filter(|row| name(*row) == "tr" && !row.has_attr("hidden"))
                .collect::<Vec<_>>()
        };
        match name(child).as_str() {
            "thead" => head.push(rows()),
            "tbody" => body.push(rows()),
            "tfoot" => foot.push(rows()),
            "tr" => direct.push(child),
            _ => {}
        }
    }
    if !direct.is_empty() {
        body.push(direct)
    }
    let mut rows = Vec::new();
    let mut cells = Vec::new();
    let mut columns = Vec::new();
    let mut cell_count = 0;
    for group in head.into_iter().chain(body).chain(foot) {
        let start = rows.len();
        let mut end = start + group.len();
        let mut occupied = vec![0; columns.len()];
        for (index, row) in group.iter().enumerate() {
            let row_index = start + index;
            let mut column = 0;
            let mut row_contents = Vec::new();
            let mut row_cells = Vec::new();
            for cell in row.children_it(false).filter(|cell| {
                matches!(name(*cell).as_str(), "td" | "th") && !cell.has_attr("hidden")
            }) {
                while occupied.get(column).is_some_and(|end| *end > row_index) {
                    column += 1
                }
                let column_span = span(cell, "colspan", MAX_COLUMN_SPAN)
                    .filter(|span| *span > 0)
                    .unwrap_or(1);
                let row_span = match span(cell, "rowspan", MAX_ROW_SPAN) {
                    Some(0) => group.len() - index,
                    Some(span) => span,
                    None => 1,
                };
                let column_end = column
                    .checked_add(column_span)
                    .ok_or_else(super::super::limit_error)?;
                let row_end = row_index
                    .checked_add(row_span)
                    .ok_or_else(super::super::limit_error)?;
                end = end.max(row_end);
                cell_count += 1;
                budget(end, column_end.max(columns.len()), cell_count)?;
                occupied.resize(occupied.len().max(column_end), 0);
                for entry in &mut occupied[column..column_end] {
                    *entry = (*entry).max(row_end)
                }
                let alignment = match cell.attr("align").as_deref() {
                    Some("center") => Alignment::Center,
                    Some("right") => Alignment::Right,
                    Some("left") => Alignment::Left,
                    _ if name(cell) == "th" => Alignment::Center,
                    _ => Alignment::Default,
                };
                columns.resize(columns.len().max(column_end), Alignment::Default);
                if row_index == 0 {
                    for entry in &mut columns[column..column_end] {
                        *entry = alignment
                    }
                }
                let bold = bold || name(cell) == "th";
                let body = cell
                    .children_it(false)
                    .any(|child| BLOCK_TAGS.contains(&name(child).as_str()))
                    .then(|| blocks(cell, base, bold))
                    .transpose()?;
                let inlines = if body.is_some() {
                    Vec::new()
                } else {
                    contents(cell, base, bold)
                };
                row_contents.push(inlines);
                row_cells.push(TableCell {
                    column,
                    column_span,
                    row_span,
                    alignment,
                    blocks: body,
                });
                column = column_end;
            }
            rows.push(row_contents);
            cells.push(row_cells);
        }
        budget(end, columns.len(), cell_count)?;
        rows.resize_with(end, Vec::new);
        cells.resize_with(end, Vec::new);
    }
    Ok(Block::Table {
        layout: Some(TableLayout {
            caption,
            column_count: columns.len(),
            row_count: rows.len(),
            cells,
        }),
        columns,
        header: Vec::new(),
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 표_모델의_예산과_크기_overflow는_오류로_반환한다() {
        assert!(budget(1, 1, 1).is_ok());
        assert!(budget(usize::MAX, 1, 1).is_err());
        assert!(budget(1, usize::MAX, 1).is_err());
        assert!(budget(1, 1, usize::MAX).is_err());
        assert!(
            budget(
                1,
                crate::preview_web_document::MAX_OUTPUT_BYTES / COLUMN_BYTES + 1,
                1
            )
            .is_err()
        );
    }
}
