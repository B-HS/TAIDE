//! Single-pass scanner over raw pty output.
//!
//! Replaces the per-sequence `extract_*` scans that used to live in [`crate::infra::shell_integration`]
//! (one full `windows()` search of the chunk per sequence family) with one left-to-right walk that
//! recognizes every escape family TAIDE cares about, carries an unterminated tail across chunk
//! boundaries, and produces the normalized text the agent-activity signals are read from
//! (`docs/acknowledge/2026-09-06-d54-agent-activity-signals-contract.md` §1.3).

use crate::infra::shell_integration::CommandMarker;

/// The OSC 777 `notify` title that marks a notification as TAIDE's own in-band agent event rather
/// than a user-facing desktop notification. Only a payload carrying exactly this title becomes a
/// [`ScanEvent::AgentEvent`]; everything else a shell writes to OSC 777 stays a plain notification.
///
/// Defined here rather than in `domain::agent::types` (which re-exports it) because the scanner is
/// the one place that must recognize it, and infra may not reference domain (architecture.md §2).
pub const AGENT_OSC_SENTINEL: &str = "taide-agent";

const ESC: u8 = 0x1b;
const OSC_BEL: u8 = 0x07;
const OSC_STRING_TERMINATOR: &[u8] = b"\x1b\\";

/// The real OSC 7 spec's payload form (`file://host/path`, percent-encoded) — as opposed to the
/// bare path TAIDE's own shell hooks emit (see [`crate::infra::shell_integration`]). A payload
/// starting with this is never one of TAIDE's own reports, so it is skipped rather than adopted as
/// a filesystem path.
const OSC7_FILE_URI_SCHEME: &str = "file://";

/// The OSC identifier urxvt's notification protocol uses, and the only subcommand of it this
/// scanner reads (`<subcommand>;<args…>`, where `notify` is the one every terminal that implements
/// it agrees on). Public because the in-band agent events TAIDE's own hooks emit have to be written
/// with exactly the prefix this recognizes — emitter and reader share these two definitions the way
/// they share [`AGENT_OSC_SENTINEL`].
pub const OSC_NOTIFY_IDENT: &str = "777";
pub const OSC_NOTIFY_SUBCOMMAND: &str = "notify";

/// The `<subcommand>;<sentinel>;` prefix every in-band agent event carries, and the shape that
/// identifies a hook entry as one TAIDE installed (`domain::agent::service`'s `TAIDE_HOOK_MARKERS`).
///
/// Ownership is matched on this rather than on [`AGENT_OSC_SENTINEL`] alone because the sentinel is
/// an ordinary word: a user's own hook whose command merely mentions it — a path, a message — would
/// otherwise be read as TAIDE's and deleted by the next reconcile. Kept literal because `const`
/// cannot concatenate, and pinned to its parts by
/// `에이전트_이벤트_마커는_서브커맨드와_센티널의_조합이다`.
pub const AGENT_OSC_MARKER: &str = "notify;taide-agent;";

/// Largest unterminated escape sequence carried across a chunk boundary. A sequence that outgrows
/// this is discarded rather than buffered indefinitely: pty output is untrusted, and a peer that
/// opens an OSC and never closes it would otherwise pin memory per session forever.
pub const MAX_OSC_PAYLOAD_BYTES: usize = 4096;

/// Length ceiling for an adopted title (OSC 0/2) or notification body (OSC 9), checked on the raw
/// payload *before* any allocation.
pub const MAX_TITLE_BYTES: usize = 512;

/// Length ceiling for an adopted in-band agent event body (OSC 777), same pre-allocation check.
pub const MAX_AGENT_EVENT_BYTES: usize = 1024;

/// How much normalized text is carried into the next chunk's [`ScanOutcome::overlap`], so a phrase
/// split across two chunks is still visible as one string to whoever matches on it.
pub const TEXT_OVERLAP_BYTES: usize = 128;

/// One recognized sequence, in the order it appeared in the chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanEvent {
    /// OSC 7 — the shell's current directory, as the bare path TAIDE's own hooks report.
    Cwd(String),
    /// OSC 133 `C`/`D` — the bounds of a command's runtime.
    CommandMarker(CommandMarker),
    /// OSC 0/2 — the window title the foreground program set.
    Title(String),
    /// OSC 777 `notify` addressed to [`AGENT_OSC_SENTINEL`]; the body is the raw remainder of the
    /// payload (its `;` separators rejoined), left unparsed for the agent domain to interpret.
    AgentEvent(String),
    /// OSC 9 — a terminal notification that is not a progress subcommand.
    Notification9(String),
}

/// What one [`OutputScanner::scan`] call produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanOutcome {
    pub events: Vec<ScanEvent>,
    /// This chunk's normalized text — escape sequences resolved to spaces/newlines or dropped.
    pub text: String,
    /// The tail of the text produced *before* this chunk (at most [`TEXT_OVERLAP_BYTES`]). Callers
    /// that match multi-word phrases prepend it to [`ScanOutcome::text`]; callers that measure how
    /// much this chunk actually printed use `text` alone, which is why the two are kept apart
    /// instead of being handed back pre-joined.
    pub overlap: String,
}

impl ScanOutcome {
    /// The last cwd report in the chunk. A batched chunk can carry several prompt renders and only
    /// the most recent reflects where the shell now is, so this collapses them the way the scan it
    /// replaced (`extract_latest_cwd`) did.
    pub fn latest_cwd(&self) -> Option<&str> {
        self.events.iter().rev().find_map(|event| match event {
            ScanEvent::Cwd(path) => Some(path.as_str()),
            _ => None,
        })
    }
}

/// Per-session scanner state: the unterminated escape tail and the normalized-text overlap.
#[derive(Debug, Default)]
pub struct OutputScanner {
    carry: Vec<u8>,
    text_tail: String,
}

enum EscapeStep {
    Consumed(usize),
    Incomplete,
}

impl OutputScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scan(&mut self, chunk: &[u8]) -> ScanOutcome {
        let carried;
        let bytes = if self.carry.is_empty() {
            chunk
        } else {
            carried = {
                let mut buffer = std::mem::take(&mut self.carry);
                buffer.extend_from_slice(chunk);
                buffer
            };
            carried.as_slice()
        };

        let mut events = Vec::new();
        let mut text = Vec::with_capacity(bytes.len());
        let mut index = 0;

        while index < bytes.len() {
            let byte = bytes[index];
            if byte != ESC {
                push_plain_byte(&mut text, byte);
                index += 1;
                continue;
            }

            match step_escape(bytes, index, &mut events, &mut text) {
                EscapeStep::Consumed(next) => index = next,
                EscapeStep::Incomplete => {
                    self.carry.extend_from_slice(&bytes[index..]);
                    break;
                }
            }
        }

        if self.carry.len() > MAX_OSC_PAYLOAD_BYTES {
            log::warn!("pty 이스케이프 시퀀스가 {MAX_OSC_PAYLOAD_BYTES}바이트를 넘겨 이월 버퍼를 폐기합니다");
            self.carry.clear();
        }

        let text = String::from_utf8_lossy(&text).into_owned();
        let overlap = self.text_tail.clone();
        self.remember_tail(&text);

        ScanOutcome { events, text, overlap }
    }

    fn remember_tail(&mut self, text: &str) {
        let mut joined = std::mem::take(&mut self.text_tail);
        joined.push_str(text);

        let start = char_boundary_at_or_after(&joined, joined.len().saturating_sub(TEXT_OVERLAP_BYTES));
        joined.drain(..start);
        self.text_tail = joined;
    }
}

/// Stateless one-shot scan, for callers (and tests) that hold no session — a chunk-boundary carry
/// is meaningless to them, so an unterminated sequence is simply not reported.
pub fn scan_once(bytes: &[u8]) -> ScanOutcome {
    OutputScanner::new().scan(bytes)
}

fn char_boundary_at_or_after(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn push_space(text: &mut Vec<u8>) {
    if text.last() == Some(&b' ') {
        return;
    }
    text.push(b' ');
}

fn push_plain_byte(text: &mut Vec<u8>, byte: u8) {
    match byte {
        b'\n' => text.push(b'\n'),
        b'\t' | b' ' => push_space(text),
        0x00..=0x1f | 0x7f => {}
        other => text.push(other),
    }
}

fn step_escape(bytes: &[u8], start: usize, events: &mut Vec<ScanEvent>, text: &mut Vec<u8>) -> EscapeStep {
    let Some(&introducer) = bytes.get(start + 1) else {
        return EscapeStep::Incomplete;
    };

    match introducer {
        b']' => step_osc(bytes, start, events),
        b'[' => step_csi(bytes, start, text),
        b'P' | b'X' | b'^' | b'_' => step_string_sequence(bytes, start),
        b'(' | b')' => match bytes.get(start + 2) {
            Some(_) => EscapeStep::Consumed(start + 3),
            None => EscapeStep::Incomplete,
        },
        _ => EscapeStep::Consumed(start + 2),
    }
}

/// Finds the terminator that ends the soonest in `rest` — the ST form `\e\\` or a bare BEL,
/// whichever offset is smaller. Taking ST whenever *any* ST exists later in `rest` would extend a
/// BEL-terminated sequence's payload past its own BEL to an unrelated later ST (routinely present
/// here — TAIDE's own OSC 133 markers are ST-terminated), swallowing everything in between.
fn earliest_terminator(rest: &[u8]) -> Option<(usize, usize)> {
    let st = find_subslice(rest, OSC_STRING_TERMINATOR).map(|offset| (offset, OSC_STRING_TERMINATOR.len()));
    let bel = rest.iter().position(|&byte| byte == OSC_BEL).map(|offset| (offset, 1));

    [st, bel].into_iter().flatten().min_by_key(|(offset, _)| *offset)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn step_osc(bytes: &[u8], start: usize, events: &mut Vec<ScanEvent>) -> EscapeStep {
    let payload_start = start + 2;
    let rest = &bytes[payload_start..];
    let Some((payload_len, terminator_len)) = earliest_terminator(rest) else {
        return EscapeStep::Incomplete;
    };

    if let Ok(payload) = std::str::from_utf8(&rest[..payload_len]) {
        if let Some(event) = classify_osc(payload) {
            events.push(event);
        }
    }

    EscapeStep::Consumed(payload_start + payload_len + terminator_len)
}

/// DCS/SOS/PM/APC carry an opaque string the terminal never renders, so the whole sequence is
/// dropped — payload included — rather than leaking into the normalized text.
fn step_string_sequence(bytes: &[u8], start: usize) -> EscapeStep {
    let payload_start = start + 2;
    match earliest_terminator(&bytes[payload_start..]) {
        Some((payload_len, terminator_len)) => EscapeStep::Consumed(payload_start + payload_len + terminator_len),
        None => EscapeStep::Incomplete,
    }
}

/// Resolves a CSI to the whitespace it moves the cursor by: a forward/absolute column move is one
/// space (Claude Code renders dialog prose word-by-word with `CSI n G` between the words, so
/// without this the phrase never appears as text at all), a row move is a newline, and everything
/// else — SGR, erase, private mode toggles — leaves no text behind.
fn step_csi(bytes: &[u8], start: usize, text: &mut Vec<u8>) -> EscapeStep {
    let mut index = start + 2;

    while index < bytes.len() {
        let byte = bytes[index];
        if (0x40..=0x7e).contains(&byte) {
            match byte {
                b'G' | b'C' => push_space(text),
                b'A' | b'B' | b'E' | b'F' | b'H' | b'd' | b'f' => text.push(b'\n'),
                _ => {}
            }
            return EscapeStep::Consumed(index + 1);
        }
        index += 1;
    }

    EscapeStep::Incomplete
}

fn classify_osc(payload: &str) -> Option<ScanEvent> {
    let (ident, rest) = payload.split_once(';')?;

    match ident {
        "7" => (!rest.starts_with(OSC7_FILE_URI_SCHEME)).then(|| ScanEvent::Cwd(strip_control(rest))),
        "133" => command_marker(rest).map(ScanEvent::CommandMarker),
        "0" | "2" => adopt(rest, MAX_TITLE_BYTES).map(ScanEvent::Title),
        OSC_NOTIFY_IDENT => agent_event(rest),
        "9" => notification(rest),
        _ => None,
    }
}

/// Only the two markers that bound a command's runtime are read; `133;A`/`133;B` (prompt start/end)
/// are indistinguishable downstream from "no marker in this chunk".
fn command_marker(rest: &str) -> Option<CommandMarker> {
    let mut fields = rest.split(';');
    match fields.next() {
        Some("C") => Some(CommandMarker::OutputStart),
        Some("D") => Some(CommandMarker::Finished {
            exit_code: fields.next().and_then(|status| status.trim().parse().ok()),
        }),
        _ => None,
    }
}

fn agent_event(rest: &str) -> Option<ScanEvent> {
    let mut fields = rest.splitn(3, ';');
    if fields.next()? != OSC_NOTIFY_SUBCOMMAND || fields.next()? != AGENT_OSC_SENTINEL {
        return None;
    }

    adopt(fields.next()?, MAX_AGENT_EVENT_BYTES).map(ScanEvent::AgentEvent)
}

/// OSC 9 is overloaded: ConEmu's progress protocol (`9;4;<state>;<percent>`) shares the identifier
/// with the plain notification form, and only the latter is a message. A numeric first field means
/// the former, so it is dropped rather than reported as a notification body.
fn notification(rest: &str) -> Option<ScanEvent> {
    let head = rest.split(';').next().unwrap_or_default();
    if !head.is_empty() && head.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    adopt(rest, MAX_TITLE_BYTES).map(ScanEvent::Notification9)
}

/// Rejects (rather than truncates) an over-long payload, measured before allocating.
fn adopt(raw: &str, limit: usize) -> Option<String> {
    (raw.len() <= limit).then(|| strip_control(raw))
}

fn strip_control(raw: &str) -> String {
    raw.chars().filter(|character| !is_c0(*character)).collect()
}

fn is_c0(character: char) -> bool {
    character < '\u{20}' || character == '\u{7f}'
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dialog frame Claude Code 2.1.263 actually wrote to a TAIDE pty when it asked for
    /// permission to run a `Bash command` — the prose is emitted word by word with `CSI n G`
    /// between the words, so this only reads back as a sentence after normalization.
    const DIALOG_FRAME: &[u8] = b"\r\x1b[1C\x1b[1B\x1b[1mBash command\
\r\x1b[3C\x1b[2B\x1b[22mtouch\x1b[10G/tmp/taide-probe-file\
\r\x1b[3C\x1b[1B\x1b[38;2;153;153;153mCreate empty probe file\
\r\x1b[1C\x1b[2B\x1b[39mDo\x1b[5Gyou\x1b[9Gwant\x1b[14Gto\x1b[17Gproceed?\
\r\x1b[1C\x1b[1B\x1b[38;2;177;185;249m\xe2\x9d\xaf\x1b[4G\x1b[38;2;153;153;153m1. \x1b[38;2;177;185;249mYes\
\r\x1b[1C\x1b[2B\x1b[38;2;153;153;153mEsc to cancel \xc2\xb7 Tab to amend\r\x1b[7B\x1b[39m\x1b[K\r";

    /// One 48-byte frame of the `⏺` blink Claude Code keeps writing every ~600ms *while* the
    /// permission dialog is up, and its blanking counterpart.
    const BLINK_GLYPH_CHUNK: &[u8] = b"\x1b[H\r\x1b[10B\x1b[38;2;153;153;153m\xe2\x8f\xba\x1b[39m\x1b[40;1H\x1b[22;2H";
    const BLINK_BLANK_CHUNK: &[u8] = b"\x1b[H\r\x1b[10B\x1b[38;2;153;153;153m \x1b[39m\x1b[40;1H\x1b[22;2H";

    fn latest_cwd(bytes: &[u8]) -> Option<String> {
        scan_once(bytes).latest_cwd().map(str::to_string)
    }

    #[test]
    fn 에이전트_이벤트_마커는_서브커맨드와_센티널의_조합이다() {
        assert_eq!(AGENT_OSC_MARKER, format!("{OSC_NOTIFY_SUBCOMMAND};{AGENT_OSC_SENTINEL};"));
    }

    fn command_markers(bytes: &[u8]) -> Vec<CommandMarker> {
        scan_once(bytes)
            .events
            .into_iter()
            .filter_map(|event| match event {
                ScanEvent::CommandMarker(marker) => Some(marker),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn extract_latest_cwd는_st_종결자로_끝나는_시퀀스를_추출한다() {
        let output = b"$ \x1b]7;/repo/src\x1b\\prompt> ";
        assert_eq!(latest_cwd(output), Some("/repo/src".to_string()));
    }

    #[test]
    fn extract_latest_cwd는_bel_종결자로_끝나는_시퀀스도_추출한다() {
        let output = b"\x1b]7;/repo/src\x07prompt> ";
        assert_eq!(latest_cwd(output), Some("/repo/src".to_string()));
    }

    #[test]
    fn extract_latest_cwd는_한_청크에_여러_개면_마지막_것을_반환한다() {
        let output = b"\x1b]7;/repo/src\x1b\\...\x1b]7;/repo/src/utils\x1b\\";
        assert_eq!(latest_cwd(output), Some("/repo/src/utils".to_string()));
    }

    #[test]
    fn extract_latest_cwd는_시퀀스가_없으면_none을_반환한다() {
        assert_eq!(latest_cwd(b"$ ls\r\na.rs b.rs\r\n"), None);
    }

    #[test]
    fn extract_latest_cwd는_bel_이_뒤쪽_st보다_먼저_오면_bel에서_끝낸다() {
        let output = b"\x1b]7;/repo\x07\x1b]133;A\x1b\\";
        assert_eq!(
            latest_cwd(output),
            Some("/repo".to_string()),
            "BEL 종결자가 더 앞에 있으면 뒤쪽의 무관한 ST 까지 삼키면 안 된다"
        );
    }

    #[test]
    fn extract_latest_cwd는_file_스킴_payload를_경로로_채택하지_않는다() {
        let output = b"\x1b]7;file://host/repo\x07";
        assert_eq!(
            latest_cwd(output),
            None,
            "file:// 형식은 이 파서의 대상이 아니므로 채택하면 안 된다"
        );
    }

    #[test]
    fn extract_latest_cwd는_file_스킴_payload_뒤에도_직전의_순수_경로_보고를_유지한다() {
        let output = b"\x1b]7;/repo\x1b\\...\x1b]7;file://host/other\x07";
        assert_eq!(
            latest_cwd(output),
            Some("/repo".to_string()),
            "사용자 rc 의 표준 OSC 7 이 TAIDE 자신의 보고를 덮어쓰면 안 된다"
        );
    }

    #[test]
    fn extract_latest_cwd는_종결자_없이_끊긴_시퀀스는_무시한다() {
        let output = b"\x1b]7;/repo/sr";
        assert_eq!(
            latest_cwd(output),
            None,
            "청크 경계에서 잘린 시퀀스는 이번 청크에서 감지되면 안 된다"
        );
    }

    #[test]
    fn extract_command_markers는_한_청크의_c와_d를_순서대로_돌려준다() {
        let output = b"\x1b]133;A\x1b\\$ ls\x1b]133;C\x1b\\a.rs\r\n\x1b]133;D;0\x1b\\";
        assert_eq!(
            command_markers(output),
            vec![CommandMarker::OutputStart, CommandMarker::Finished { exit_code: Some(0) }]
        );
    }

    #[test]
    fn extract_command_markers는_상태가_없는_d도_종료로_인식한다() {
        assert_eq!(
            command_markers(b"\x1b]133;D\x1b\\"),
            vec![CommandMarker::Finished { exit_code: None }]
        );
    }

    #[test]
    fn extract_command_markers는_정수가_아닌_상태를_none으로_읽는다() {
        assert_eq!(
            command_markers(b"\x1b]133;D;fail\x1b\\"),
            vec![CommandMarker::Finished { exit_code: None }]
        );
    }

    #[test]
    fn extract_command_markers는_bel_종결자와_프롬프트_마커를_구분한다() {
        let output = b"\x1b]133;B\x07\x1b]133;C\x07\x1b]133;D;130\x07";
        assert_eq!(
            command_markers(output),
            vec![CommandMarker::OutputStart, CommandMarker::Finished { exit_code: Some(130) }]
        );
    }

    #[test]
    fn extract_command_markers는_종결자_없이_끊긴_마커를_무시한다() {
        assert_eq!(command_markers(b"\x1b]133;D;0"), Vec::new());
    }

    #[test]
    fn extract_command_markers는_한_청크에_실린_두_명령의_마커를_전부_순서대로_돌려준다() {
        let output = b"\x1b]133;C\x1b\\out1\x1b]133;D;0\x1b\\\x1b]133;A\x1b\\$ \x1b]133;C\x1b\\out2\x1b]133;D;1\x1b\\";
        assert_eq!(
            command_markers(output),
            vec![
                CommandMarker::OutputStart,
                CommandMarker::Finished { exit_code: Some(0) },
                CommandMarker::OutputStart,
                CommandMarker::Finished { exit_code: Some(1) },
            ],
            "마지막 하나만 남기면 첫 명령의 C→D 전이를 잃는다"
        );
    }

    #[test]
    fn extract_command_markers는_osc7_cwd_보고와_섞여_있어도_133_마커만_읽는다() {
        let output = b"\x1b]7;/repo\x1b\\\x1b]133;C\x1b\\\x1b]7;/repo/src\x07\x1b]133;D;0\x1b\\";
        assert_eq!(
            command_markers(output),
            vec![CommandMarker::OutputStart, CommandMarker::Finished { exit_code: Some(0) }]
        );
        assert_eq!(
            latest_cwd(output).as_deref(),
            Some("/repo/src"),
            "같은 청크의 cwd 보고도 그대로 읽혀야 한다"
        );
    }

    #[test]
    fn extract_command_markers는_a_b_프롬프트_마커만_있으면_비어_있다() {
        assert_eq!(command_markers(b"\x1b]133;A\x1b\\$ \x1b]133;B\x1b\\"), Vec::new());
    }

    #[test]
    fn extract_command_markers는_음수와_공백이_섞인_종료_상태도_정수로_읽는다() {
        assert_eq!(
            command_markers(b"\x1b]133;D;-1\x1b\\\x1b]133;D; 2 \x1b\\"),
            vec![
                CommandMarker::Finished { exit_code: Some(-1) },
                CommandMarker::Finished { exit_code: Some(2) },
            ]
        );
    }

    #[test]
    fn extract_command_markers는_마커가_없는_일반_출력에서_비어_있다() {
        assert_eq!(command_markers(b"plain output\r\n\x1b[31mred\x1b[0m"), Vec::new());
        assert_eq!(command_markers(b""), Vec::new());
    }

    #[test]
    fn 청크_경계에서_잘린_133_마커는_다음_청크에서_방출된다() {
        let mut scanner = OutputScanner::new();

        let first = scanner.scan(b"done\x1b]133;D;");
        assert_eq!(first.events, Vec::new(), "종결자가 오기 전에는 아무것도 방출하지 않는다");

        let second = scanner.scan(b"0\x1b\\next");
        assert_eq!(
            second.events,
            vec![ScanEvent::CommandMarker(CommandMarker::Finished { exit_code: Some(0) })]
        );
        assert_eq!(second.text, "next", "이월된 시퀀스는 텍스트로 새지 않는다");
    }

    #[test]
    fn 이월_버퍼가_상한을_넘으면_폐기하고_다음_청크부터_다시_읽는다() {
        let mut scanner = OutputScanner::new();
        let mut runaway = b"\x1b]0;".to_vec();
        runaway.resize(runaway.len() + MAX_OSC_PAYLOAD_BYTES + 1, b'x');

        assert_eq!(scanner.scan(&runaway).events, Vec::new());
        assert_eq!(
            scanner.scan(b"\x1b]133;C\x1b\\").events,
            vec![ScanEvent::CommandMarker(CommandMarker::OutputStart)],
            "폐기 후에도 다음 청크의 시퀀스는 정상적으로 읽혀야 한다"
        );
    }

    #[test]
    fn 한_청크에_섞인_7_133_0_777_9_시퀀스를_순서대로_돌려준다() {
        let output =
            b"\x1b]7;/repo\x1b\\\x1b]133;C\x1b\\\x1b]0;title\x07\x1b]777;notify;taide-agent;{\"event\":\"stop\"}\x07\x1b]9;done\x07";
        assert_eq!(
            scan_once(output).events,
            vec![
                ScanEvent::Cwd("/repo".to_string()),
                ScanEvent::CommandMarker(CommandMarker::OutputStart),
                ScanEvent::Title("title".to_string()),
                ScanEvent::AgentEvent("{\"event\":\"stop\"}".to_string()),
                ScanEvent::Notification9("done".to_string()),
            ]
        );
    }

    #[test]
    fn 실물_권한_다이얼로그_프레임은_정규화_텍스트로_복원된다() {
        let text = scan_once(DIALOG_FRAME).text;

        assert!(
            text.contains("Do you want to proceed?"),
            "열 이동으로 쪼개진 문장이 복원되어야 한다: {text}"
        );
        assert!(text.contains("Esc to cancel"), "다이얼로그 푸터가 남아야 한다: {text}");
        assert!(text.contains("Bash command"));
        assert!(!text.contains('\x1b'), "정규화 텍스트에 이스케이프가 남으면 안 된다");
    }

    #[test]
    fn 작업_중_푸터의_소문자_문구는_다이얼로그_문구와_구별된다() {
        let text = scan_once(b"\r\x1b[1C\x1b[2B\x1b[39mesc\x1b[5Gto\x1b[8Ginterrupt").text;

        assert!(text.contains("esc to interrupt"));
        assert!(!text.contains("Esc to cancel"));
    }

    #[test]
    fn 점멸_청크의_정규화_텍스트는_점멸_글리프나_공백뿐이다() {
        for chunk in [BLINK_GLYPH_CHUNK, BLINK_BLANK_CHUNK] {
            let text = scan_once(chunk).text;
            assert!(
                text.chars().all(|character| character.is_whitespace() || character == '⏺'),
                "점멸 프레임에는 실질 출력이 없어야 한다: {text:?}"
            );
        }
    }

    #[test]
    fn 타이틀_시퀀스는_title_이벤트로_방출된다() {
        assert_eq!(
            scan_once("\x1b]0;◐ taide-probe-file 생성\x07".as_bytes()).events,
            vec![ScanEvent::Title("◐ taide-probe-file 생성".to_string())]
        );
        assert_eq!(
            scan_once(b"\x1b]2;plain title\x1b\\").events,
            vec![ScanEvent::Title("plain title".to_string())]
        );
    }

    #[test]
    fn osc777_센티널이_다르면_에이전트_이벤트로_읽지_않는다() {
        assert_eq!(scan_once(b"\x1b]777;notify;other-app;{\"event\":\"stop\"}\x07").events, Vec::new());
        assert_eq!(scan_once(b"\x1b]777;fontsize;taide-agent;12\x07").events, Vec::new());
    }

    #[test]
    fn osc777_본문은_세미콜론을_다시_이어_붙인다() {
        assert_eq!(
            scan_once(b"\x1b]777;notify;taide-agent;a;b;c\x07").events,
            vec![ScanEvent::AgentEvent("a;b;c".to_string())]
        );
    }

    #[test]
    fn 길이_상한을_넘는_타이틀과_에이전트_이벤트는_거절한다() {
        let long_title = format!("\x1b]0;{}\x07", "t".repeat(MAX_TITLE_BYTES + 1));
        assert_eq!(scan_once(long_title.as_bytes()).events, Vec::new());

        let long_event = format!("\x1b]777;notify;taide-agent;{}\x07", "e".repeat(MAX_AGENT_EVENT_BYTES + 1));
        assert_eq!(scan_once(long_event.as_bytes()).events, Vec::new());
    }

    #[test]
    fn 채택한_문자열에서_c0_제어문자를_제거한다() {
        assert_eq!(
            scan_once(b"\x1b]0;ti\x01tle\x1b\\").events,
            vec![ScanEvent::Title("title".to_string())]
        );
        assert_eq!(
            scan_once(b"\x1b]7;/re\x0bpo\x1b\\").events,
            vec![ScanEvent::Cwd("/repo".to_string())]
        );
    }

    #[test]
    fn osc9_의_첫_필드가_숫자면_진행률_서브커맨드로_보고_무시한다() {
        assert_eq!(scan_once(b"\x1b]9;4;1;50\x07").events, Vec::new());
        assert_eq!(
            scan_once(b"\x1b]9;build finished\x07").events,
            vec![ScanEvent::Notification9("build finished".to_string())]
        );
    }

    #[test]
    fn 정규화는_열_이동을_공백으로_행_이동을_개행으로_바꾸고_나머지_시퀀스를_지운다() {
        let text = scan_once(b"\x1b(Ba\x1b[3Cb\x1b[2Ec\x1b[?25l\x1b[0m\td\re\x1b[Kf").text;
        assert_eq!(text, "a b\nc def");
    }

    #[test]
    fn 텍스트_꼬리는_다음_청크의_overlap_으로_이어진다() {
        let mut scanner = OutputScanner::new();

        assert_eq!(scanner.scan(b"Do\x1b[5Gyou\x1b[9Gwant\x1b[14Gto").overlap, "");

        let second = scanner.scan(b"\x1b[17Gproceed?");
        assert_eq!(second.text, " proceed?");
        assert!(
            format!("{}{}", second.overlap, second.text).contains("Do you want to proceed?"),
            "꼬리를 이어 붙여야 청크에 걸친 문구가 보인다"
        );
    }

    #[test]
    fn 텍스트_꼬리는_상한_안에서만_유지된다() {
        let mut scanner = OutputScanner::new();
        scanner.scan("가".repeat(TEXT_OVERLAP_BYTES).as_bytes());

        let overlap = scanner.scan(b"x").overlap;
        assert!(overlap.len() <= TEXT_OVERLAP_BYTES, "꼬리 길이가 상한을 넘었다: {}", overlap.len());
        assert!(overlap.chars().all(|character| character == '가'));
    }
}
