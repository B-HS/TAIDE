use std::ops::Range;

use crate::document::EditorError;

pub fn map_utf16_range(
    range: Range<usize>,
    edit: Range<usize>,
    inserted_units: usize,
    grow_at_edges: bool,
    force_move_markers: bool,
) -> Result<Range<usize>, EditorError> {
    if range.start > range.end || edit.start > edit.end {
        return Err(EditorError::InvalidBoundary);
    }
    let deleted = edit.len();
    let common = deleted.min(inserted_units);
    let common_end = edit
        .start
        .checked_add(common)
        .ok_or(EditorError::Capacity)?;
    let inserted_end = edit
        .start
        .checked_add(inserted_units)
        .ok_or(EditorError::Capacity)?;
    let before = |offset: usize, stay_at_equal: bool, boundary: usize, force_stay: bool| {
        offset < boundary
            || (offset == boundary && !force_move_markers && (force_stay || stay_at_equal))
    };
    let map = |offset: usize, stay_at_equal: bool| {
        if before(offset, stay_at_equal, edit.start, deleted > 0) {
            return Ok(offset);
        }
        if common > 0
            && !force_move_markers
            && before(offset, stay_at_equal, common_end, deleted > inserted_units)
        {
            return Ok(offset);
        }
        if before(offset, stay_at_equal, edit.end, false) {
            return Ok(inserted_end);
        }
        offset
            .checked_sub(deleted)
            .and_then(|offset| offset.checked_add(inserted_units))
            .ok_or(EditorError::Capacity)
    };
    let start = map(range.start, grow_at_edges)?;
    let end = map(range.end, !grow_at_edges)?;
    Ok(start..end.max(start))
}
