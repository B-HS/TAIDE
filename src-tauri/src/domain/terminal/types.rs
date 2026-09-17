use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ids::ProjectId;

/// The scrollback budget a pty session gets when the caller asks for nothing in particular, and
/// the floor every explicit request is raised to — a terminal tab's whole visible history after a
/// tab switch is rebuilt from `pty_attach`'s replay of this ring, so shrinking it below what the
/// app always guaranteed would silently lose history the user could previously scroll back to.
pub const DEFAULT_SCROLLBACK_BYTES: usize = 2 * 1024 * 1024;

/// The ceiling an explicit [`PtySpawnOptions::scrollback_bytes`] request is clamped to.
///
/// The budget is held resident per session ([`super::service::ScrollbackRing`]) and replayed in one
/// piece on every attach, so an unbounded request would trade the user's own memory for history
/// they asked for in lines, not bytes. 32 MiB is 16x the default — enough for the long build/test
/// logs the setting exists for at the 100,000-line maximum the settings UI allows — while staying
/// two orders of magnitude below [`PtyAttachResult::replay_bytes`]'s `u32` ceiling.
pub const MAX_SCROLLBACK_BYTES: usize = 32 * 1024 * 1024;

/// Resolves a caller-requested scrollback budget into the byte count a session's ring is built
/// with: absent means [`DEFAULT_SCROLLBACK_BYTES`], present is clamped into
/// `DEFAULT_SCROLLBACK_BYTES..=MAX_SCROLLBACK_BYTES`.
///
/// The frontend asks in lines (`Settings::terminal_scrollback`) and converts with its own
/// per-line byte estimate, so the number arriving here is an estimate of an estimate — clamping
/// rather than rejecting keeps a wildly-off request working instead of failing the spawn.
pub fn resolve_scrollback_bytes(requested: Option<u32>) -> usize {
    match requested {
        Some(bytes) => (bytes as usize).clamp(DEFAULT_SCROLLBACK_BYTES, MAX_SCROLLBACK_BYTES),
        None => DEFAULT_SCROLLBACK_BYTES,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShellProfile {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PtySpawnOptions {
    pub project_id: ProjectId,
    pub cwd: String,
    #[serde(default)]
    pub shell: Option<String>,
    pub cols: u16,
    pub rows: u16,
    /// How many bytes of output this session's scrollback ring keeps, resolved through
    /// [`resolve_scrollback_bytes`]. `None` — every caller that predates this field — keeps the
    /// previous fixed [`DEFAULT_SCROLLBACK_BYTES`] budget. A `u32` because `specta-typescript`
    /// refuses BigInt-style types, the same constraint [`PtyAttachResult::replay_bytes`] carries;
    /// [`MAX_SCROLLBACK_BYTES`] is far below that ceiling. Applies from the next spawn on — there
    /// is no command to resize a running session's ring.
    #[serde(default)]
    pub scrollback_bytes: Option<u32>,
}

/// What one `pty_attach` handed back: the subscription id `pty_detach` consumes, plus how many
/// bytes that attach replayed before any live output could arrive.
///
/// `replay_bytes` exists so the renderer can tell replayed scrollback from live output on a stream
/// that carries both. It counts them against a budget instead of the write backlog that drives flow
/// control, which otherwise saw up to a full scrollback land at once and paused a healthy child
/// process on every terminal tab switch. It is a `u32` because `specta-typescript` refuses to export
/// BigInt-style types; one replay is bounded by [`MAX_SCROLLBACK_BYTES`] plus a four-byte
/// preamble, two orders of magnitude below that ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PtyAttachResult {
    pub subscription_id: u32,
    pub replay_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    pub id: String,
    pub project_id: ProjectId,
    pub cwd: String,
    pub shell: String,
    pub running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 스크롤백_요청이_없으면_기본_예산을_쓴다() {
        assert_eq!(resolve_scrollback_bytes(None), DEFAULT_SCROLLBACK_BYTES);
    }

    #[test]
    fn 기본값보다_작은_요청은_기본값으로_올린다() {
        assert_eq!(resolve_scrollback_bytes(Some(1024)), DEFAULT_SCROLLBACK_BYTES);
        assert_eq!(resolve_scrollback_bytes(Some(0)), DEFAULT_SCROLLBACK_BYTES);
    }

    #[test]
    fn 상한을_넘는_요청은_상한으로_내린다() {
        assert_eq!(resolve_scrollback_bytes(Some(u32::MAX)), MAX_SCROLLBACK_BYTES);
    }

    #[test]
    fn 범위_안의_요청은_그대로_쓴다() {
        let requested = 8 * 1024 * 1024;
        assert_eq!(resolve_scrollback_bytes(Some(requested as u32)), requested);
    }

    /// The field has to stay optional on the wire: `bindings.ts` is regenerated from this struct,
    /// but a remote session's JSON (`domain::remote::dispatch`) is hand-written by its client and
    /// must keep spawning with the pre-field payload shape.
    #[test]
    fn 스폰_옵션은_스크롤백_필드가_없어도_역직렬화된다() {
        let opts: PtySpawnOptions = serde_json::from_str(r#"{"projectId":"prj-1","cwd":"/repo","cols":80,"rows":24}"#)
            .expect("스크롤백 없는 페이로드도 읽혀야 합니다");

        assert_eq!(opts.scrollback_bytes, None);
        assert_eq!(resolve_scrollback_bytes(opts.scrollback_bytes), DEFAULT_SCROLLBACK_BYTES);
    }
}
