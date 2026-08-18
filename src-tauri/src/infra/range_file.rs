//! Shared file-range serving primitives for the two handlers that serve raw project-file bytes
//! gated by [`super::root_guard`]: `domain::remote::serving::file_range` (the remote
//! `/__taide/file` HTTP route) and [`super::asset_protocol::respond`] (the local `asset://` scheme
//! handler, see `docs/acknowledge/2026-08-18-audit-t1-batch2-contract.md` X1#7). Both need
//! identical `Range`-header parsing, extension-to-MIME mapping, and byte-slice reads; extracted
//! here after review found the two handlers' copy-pasted implementations had already begun to
//! drift — the duplicated `parse_range` silently accepted an inverted range (`end < start`) and
//! underflowed the caller's `end - start + 1` length computation, one copy at a time waiting to be
//! fixed twice.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Single-`Range`-request chunk cap: a client may ask for a wider span, but any one response never
/// buffers more than this many bytes (mirrors Tauri's own vendored `asset.rs`'s private `MAX_LEN`).
pub const RANGE_CHUNK_LIMIT: u64 = 1000 * 1024;

/// `Content-Security-Policy` for a raw project-file response. The file being served can be
/// anything a project happens to contain, including an SVG with an embedded `<script>`; without
/// this header a direct navigation (or an `<object>`/`<iframe>` embed) to such a file would let
/// that script run same-origin. `sandbox` (no `allow-*` tokens) plus `script-src 'none'`
/// forecloses that regardless of the file's own content — mirrors the treatment GitHub gives
/// `raw.githubusercontent.com`.
pub const RANGE_RESPONSE_CSP: &str = "default-src 'none'; script-src 'none'; style-src 'none'; sandbox";

pub fn extension_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("m4v") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// Parses a single-range `Range: bytes=<start>-<end>` header against `total`, capping the returned
/// span at [`RANGE_CHUNK_LIMIT`] bytes regardless of what the client asked for. Returns `None` (the
/// caller responds `416 Range Not Satisfiable`) for a missing/malformed spec, a `start` at or past
/// `total`, or an inverted range (`end < start`) — the last of these previously reached the caller
/// un-rejected and underflowed the `end - start + 1` length computation downstream, which either
/// panics (debug `overflow-checks`) or wraps to a near-`u64::MAX` value that then fails the
/// destination `Vec` allocation (release).
pub fn parse_range(range_header: &str, total: u64) -> Option<(u64, u64)> {
    let spec = range_header.strip_prefix("bytes=")?;
    let (start_raw, end_raw) = spec.split_once('-')?;

    let start = if start_raw.is_empty() { 0 } else { start_raw.parse::<u64>().ok()? };
    if start >= total {
        return None;
    }

    let requested_end = if end_raw.is_empty() {
        total - 1
    } else {
        end_raw.parse::<u64>().ok()?
    };
    if requested_end < start {
        return None;
    }
    let capped_end = requested_end.min(total - 1).min(start + RANGE_CHUNK_LIMIT - 1);
    Some((start, capped_end))
}

pub fn read_slice(path: &Path, start: u64, length: u64) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buffer = vec![0u8; length as usize];
    file.read_exact(&mut buffer)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 범위_없는_요청은_파일_전체_바이트_범위를_반환한다() {
        assert_eq!(parse_range("bytes=0-", 1000), Some((0, 999)));
    }

    #[test]
    fn 요청된_길이가_상한을_넘으면_한_청크만큼만_잘린다() {
        let total = RANGE_CHUNK_LIMIT * 3;
        let (start, end) = parse_range("bytes=0-", total).unwrap();
        assert_eq!(start, 0);
        assert_eq!(end - start + 1, RANGE_CHUNK_LIMIT);
    }

    #[test]
    fn 시작점이_파일_크기_이상이면_범위를_거부한다() {
        assert_eq!(parse_range("bytes=1000-", 1000), None);
    }

    #[test]
    fn 끝점이_시작점보다_앞선_뒤집힌_범위는_길이_언더플로_없이_거부된다() {
        assert_eq!(parse_range("bytes=500-100", 1000), None);
        assert_eq!(parse_range("bytes=999-0", 1000), None);
    }

    #[test]
    fn 확장자별_mime_타입을_비디오_오디오_기준으로_판정한다() {
        assert_eq!(extension_mime(Path::new("clip.mp4")), "video/mp4");
        assert_eq!(extension_mime(Path::new("clip.MP3")), "audio/mpeg");
        assert_eq!(extension_mime(Path::new("clip.m4v")), "video/mp4");
        assert_eq!(extension_mime(Path::new("unknown.xyz")), "application/octet-stream");
    }

    #[test]
    fn range_response_csp는_스크립트와_스타일을_모두_차단하고_sandbox를_적용한다() {
        assert!(RANGE_RESPONSE_CSP.contains("script-src 'none'"));
        assert!(RANGE_RESPONSE_CSP.contains("style-src 'none'"));
        assert!(RANGE_RESPONSE_CSP.contains("sandbox"));
    }
}
