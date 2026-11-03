use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use taide_lsp::native::protocol::lsp_types;
use taide_model::file::{FileSizeTier, LARGE_FILE_BYTES};
use taide_native_editor::css_folding;
use taide_native_editor::css_format;
use taide_native_editor::css_scanner::Dialect;
use taide_native_editor::document::{DocumentKey, DocumentSnapshot, Edit, EditorError};
use taide_native_editor::indent::IndentOptions;
use taide_native_editor::lsp::{LspRange, TextEdit, byte_to_position, range_to_bytes};
use taide_native_editor::syntax_folding::SyntaxFolds;

const MAX_TOKENS: usize = 1024 * 1024;
const MAX_DEPTH: usize = 8192;
const MAX_FOLD_RANGES: usize = 256 * 1024;
const MAX_FORMAT_RANGES: usize = 1024;
const MAX_INDENT_WIDTH: usize = 1024;
const TEXT_MULTIPLIER: usize = 4;
const WORK_MULTIPLIER: usize = 4;
const FOLD_LIMITS: css_folding::Limits = css_folding::Limits {
    max_bytes: LARGE_FILE_BYTES as usize,
    max_tokens: MAX_TOKENS,
    max_depth: MAX_DEPTH,
    max_ranges: MAX_FOLD_RANGES,
};
const FORMAT_LIMITS: css_format::Limits = css_format::Limits {
    max_bytes: LARGE_FILE_BYTES as usize,
    max_output_bytes: LARGE_FILE_BYTES as usize * TEXT_MULTIPLIER,
    max_indent_width: MAX_INDENT_WIDTH,
};

pub(crate) fn dialect(snapshot: &DocumentSnapshot) -> Option<Dialect> {
    if snapshot.metadata.tier != FileSizeTier::Normal
        || matches!(
            snapshot.key,
            DocumentKey::AppFile(_) | DocumentKey::Virtual(_)
        )
    {
        return None;
    }
    Dialect::from_language_id(&snapshot.metadata.language_id)
}

pub(crate) fn folds(
    snapshot: &DocumentSnapshot,
    dialect: Dialect,
    cancelled: &AtomicBool,
) -> Result<Arc<SyntaxFolds>, EditorError> {
    if snapshot.rope.len_bytes() > FOLD_LIMITS.max_bytes {
        return Err(EditorError::Capacity);
    }
    let ranges = css_folding::folding_ranges(&snapshot.rope.to_string(), dialect, FOLD_LIMITS)?;
    if cancelled.load(Ordering::Acquire) {
        return Err(EditorError::StaleRevision);
    }
    Ok(Arc::new(SyntaxFolds::new(snapshot, vec![ranges])))
}

pub(crate) fn is_format_eligible(snapshot: &DocumentSnapshot) -> bool {
    dialect(snapshot).is_some() && !snapshot.metadata.read_only
}

pub(crate) fn format(
    snapshot: &DocumentSnapshot,
    kind: &crate::editor_formatting::Kind,
    options: &lsp_types::FormattingOptions,
) -> Result<Vec<TextEdit>, EditorError> {
    if !is_format_eligible(snapshot) {
        return Err(EditorError::Refused);
    }
    let options = css_format::Options {
        indentation: IndentOptions {
            tab_size: options.tab_size,
            insert_spaces: options.insert_spaces,
        },
    };
    format_ranges(snapshot, kind, FORMAT_LIMITS.max_bytes, |text, range| {
        css_format::format(text, range, options, FORMAT_LIMITS)
    })
}

pub(crate) fn format_ranges(
    snapshot: &DocumentSnapshot,
    kind: &crate::editor_formatting::Kind,
    max_bytes: usize,
    mut format: impl FnMut(&str, Option<Range<usize>>) -> Result<Vec<Edit>, EditorError>,
) -> Result<Vec<TextEdit>, EditorError> {
    if snapshot.rope.len_bytes() > max_bytes {
        return Err(EditorError::Refused);
    }
    let text = snapshot.rope.to_string();
    let mut work_bytes = 0usize;
    let mut compute = |range: Option<Range<usize>>| {
        let work = range.as_ref().map_or(text.len(), |range| {
            let start = snapshot
                .rope
                .line_to_byte(snapshot.rope.byte_to_line(range.start));
            let line = snapshot.rope.byte_to_line(range.end);
            let end = snapshot
                .rope
                .try_line_to_byte(line + 1)
                .unwrap_or(text.len());
            end - start
        });
        work_bytes = work_bytes.checked_add(work).ok_or(EditorError::Capacity)?;
        if work_bytes > max_bytes * WORK_MULTIPLIER {
            return Err(EditorError::Capacity);
        }
        format(&text, range)
    };
    let edits = match kind {
        crate::editor_formatting::Kind::Document => compute(None)?,
        crate::editor_formatting::Kind::Ranges(ranges) => {
            if ranges.len() > MAX_FORMAT_RANGES {
                return Err(EditorError::Capacity);
            }
            let mut ranges = ranges
                .iter()
                .map(|range| range_to_bytes(snapshot, *range))
                .collect::<Result<Vec<_>, _>>()?;
            ranges.sort_by_key(|range| (range.start, range.end));
            let mut merged = Vec::<Range<usize>>::with_capacity(ranges.len());
            for range in ranges {
                match merged.last_mut() {
                    Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
                    _ => merged.push(range),
                }
            }
            let mut results = merged
                .iter()
                .map(|range| compute(Some(range.clone())))
                .collect::<Result<Vec<_>, _>>()?;
            while let Some((first, second)) = (0..results.len()).find_map(|first| {
                (first + 1..results.len())
                    .find(|&second| {
                        results[first].iter().any(|left| {
                            results[second].iter().any(|right| {
                                left.bytes.start.max(right.bytes.start)
                                    <= left.bytes.end.min(right.bytes.end)
                            })
                        })
                    })
                    .map(|second| (first, second))
            }) {
                let range = merged[first].start.min(merged[second].start)
                    ..merged[first].end.max(merged[second].end);
                let edits = compute(Some(range.clone()))?;
                merged.remove(second);
                merged.remove(first);
                results.remove(second);
                results.remove(first);
                merged.push(range);
                results.push(edits);
            }
            results.into_iter().flatten().collect()
        }
        crate::editor_formatting::Kind::OnType { .. } => return Err(EditorError::Refused),
    };
    edits
        .into_iter()
        .map(|edit| {
            Ok(TextEdit::new(
                LspRange::new(
                    byte_to_position(snapshot, edit.bytes.start)?,
                    byte_to_position(snapshot, edit.bytes.end)?,
                ),
                edit.text,
            ))
        })
        .collect()
}
