use taide_native_editor::document::{DocumentKey, DocumentSnapshot, EditorError};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::json_format;
use taide_native_editor::lsp::{TextEdit, byte_to_position, range_to_bytes};

const MAX_EDITS: usize = 256 * 1024;
const MAX_RANGES: usize = 1024;
const TEXT_MULTIPLIER: usize = 4;
const MAX_INDENT_WIDTH: usize = 1024;
const LIMITS: json_format::Limits = json_format::Limits {
    scanner: crate::editor_json::PARSER_LIMITS.scanner,
    max_edits: MAX_EDITS,
    max_output_bytes: crate::editor_json::PARSER_LIMITS.scanner.max_bytes * TEXT_MULTIPLIER,
    max_indent_width: MAX_INDENT_WIDTH,
};

pub(crate) fn is_eligible(snapshot: &DocumentSnapshot) -> bool {
    snapshot.metadata.language_id == "json"
        && snapshot.metadata.tier == taide_model::file::FileSizeTier::Normal
        && !snapshot.metadata.read_only
        && !matches!(snapshot.key, DocumentKey::AppFile(_))
}

pub(crate) fn build(
    snapshot: &DocumentSnapshot,
    kind: &crate::editor_formatting::Kind,
    options: &taide_lsp::native::protocol::lsp_types::FormattingOptions,
) -> Result<Vec<TextEdit>, EditorError> {
    if !is_eligible(snapshot) || snapshot.rope.len_bytes() > LIMITS.scanner.max_bytes {
        return Err(EditorError::Refused);
    }
    let ranges = match kind {
        crate::editor_formatting::Kind::Document => vec![None],
        crate::editor_formatting::Kind::Ranges(ranges) => {
            if ranges.len() > MAX_RANGES {
                return Err(EditorError::Capacity);
            }
            ranges
                .iter()
                .map(|range| range_to_bytes(snapshot, *range).map(Some))
                .collect::<Result<Vec<_>, _>>()?
        }
        crate::editor_formatting::Kind::OnType { .. } => return Err(EditorError::Refused),
    };
    let text = snapshot.rope.to_string();
    let mut result = Vec::new();
    let mut output_bytes = 0usize;
    let mut work_bytes = 0usize;
    for range in ranges {
        let work = range.as_ref().map_or(snapshot.rope.len_bytes(), |range| {
            let start = snapshot
                .rope
                .line_to_byte(snapshot.rope.byte_to_line(range.start));
            let line = snapshot.rope.byte_to_line(range.end);
            let end = snapshot
                .rope
                .try_line_to_byte(line + 1)
                .unwrap_or(snapshot.rope.len_bytes());
            end - start
        });
        work_bytes = work_bytes.checked_add(work).ok_or(EditorError::Capacity)?;
        if work_bytes > LIMITS.max_output_bytes {
            return Err(EditorError::Capacity);
        }
        let edits = json_format::format(
            &text,
            range,
            json_format::Options {
                indentation: IndentOptions {
                    tab_size: options.tab_size,
                    insert_spaces: options.insert_spaces,
                },
                eol: "\n",
                keep_lines: false,
                insert_final_newline: options.insert_final_newline == Some(true),
            },
            LIMITS,
        )?;
        for edit in edits {
            output_bytes = output_bytes
                .checked_add(edit.text.len())
                .ok_or(EditorError::Capacity)?;
            if result.len() >= MAX_EDITS || output_bytes > LIMITS.max_output_bytes {
                return Err(EditorError::Capacity);
            }
            result.push(edit);
        }
    }
    result.sort_by(|left, right| {
        (left.bytes.start, left.bytes.end)
            .cmp(&(right.bytes.start, right.bytes.end))
            .then(left.text.cmp(&right.text))
    });
    result.dedup_by(|left, right| left.bytes == right.bytes && left.text == right.text);
    result
        .into_iter()
        .map(|edit| {
            Ok(TextEdit::new(
                taide_native_editor::lsp::LspRange::new(
                    byte_to_position(snapshot, edit.bytes.start)?,
                    byte_to_position(snapshot, edit.bytes.end)?,
                ),
                edit.text,
            ))
        })
        .collect()
}
