use std::ops::Range;

use taide_native_editor::document::DocumentSnapshot;

pub(super) fn extend(
    document: &DocumentSnapshot,
    bytes: &Range<usize>,
    candidate: &str,
    cursor: usize,
    inline: &str,
) -> Option<(String, usize)> {
    if bytes.start > cursor || bytes.end != cursor {
        return None;
    }
    let original = document.rope.get_byte_slice(bytes.clone())?.to_string();
    let candidate = candidate.replace("\r\n", "\n");
    let prefix = original
        .chars()
        .zip(candidate.chars())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum::<usize>();
    if bytes.start.checked_add(prefix)? != cursor {
        return None;
    }
    let suffix = &candidate[prefix..];
    let inline = inline.replace("\r\n", "\n");
    if !inline.starts_with(suffix) {
        return None;
    }
    let preview = inline
        .encode_utf16()
        .count()
        .checked_sub(suffix.encode_utf16().count())?;
    Some((format!("{}{inline}", &candidate[..prefix]), preview))
}
