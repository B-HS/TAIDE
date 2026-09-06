use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Deserialize;

use super::types::{
    AgentActivity, CliInstallStatus, DetectedAgent, ExternalOpenRequest, HookInstallScope, ACTIVITY_IDLE_QUIET_MS,
    ACTIVITY_WORKING_HOLD_MS, AGENT_NAME_CLAUDE, AGENT_NAME_CODEX, AGENT_NAME_GEMINI, AGENT_OSC_MARKER, AGENT_OSC_SENTINEL,
    AGENT_PROTOCOL_VERSION, AGENT_PROTOCOL_VERSION_ENV_NAME, CLAUDE_MANAGED_HOOK_EVENTS, CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION,
    CODEX_HOOK_COMMAND_TIMEOUT_SECONDS, CODEX_MANAGED_HOOK_EVENTS, ECHO_SUPPRESS_MS, GEMINI_HOOK_COMMAND_TIMEOUT_MS,
    GEMINI_MANAGED_HOOK_EVENTS, HOOKS_HTTP_TIMEOUT_SECONDS, HOOKS_URL_MARKER, HOOK_EVENT_AFTER_AGENT, HOOK_EVENT_BEFORE_AGENT,
    HOOK_EVENT_NOTIFICATION, HOOK_EVENT_PERMISSION_REQUEST, HOOK_EVENT_POST_TOOL_USE, HOOK_EVENT_STOP, HOOK_EVENT_STOP_FAILURE,
    HOOK_EVENT_USER_PROMPT_SUBMIT, HOOK_HANDLER_TYPE_COMMAND, KNOWN_AGENT_NAMES, NOTIFICATION_MATCHER_ELICITATION_DIALOG,
    NOTIFICATION_MATCHER_IDLE_PROMPT, NOTIFICATION_MATCHER_PERMISSION_PROMPT, OSC_NOTIFY_IDENT, OSC_NOTIFY_SUBCOMMAND,
    SUBSTANTIVE_OUTPUT_MIN_CHARS, TITLE_WORKING_FRESH_MS, WAIT_MARKER_PREFIX,
};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::infra::terminal_scan::{ScanEvent, ScanOutcome};

const LINUX_COMM_MAX_LEN: usize = 15;
const NODE_RUNTIME_NAMES: &[&str] = &["node", "bun", "deno"];
const EDITOR_ENV_HINT: &str = "export EDITOR=\"taide --wait\"";
const CLI_WAIT_MARKER_FLAG: &str = "--wait-marker";
const CLI_WAIT_FLAG: &str = "--wait";

/// Name of the environment variable Claude Code's ctrl+g falls back to when `VISUAL` is unset.
pub const EDITOR_ENV_NAME: &str = "EDITOR";

/// Name of the environment variable Claude Code's ctrl+g resolves first, before `EDITOR`.
pub const VISUAL_ENV_NAME: &str = "VISUAL";

/// Filename of the bundled CLI sidecar binary (also the symlink ownership marker).
pub const CLI_SIDECAR_BIN_NAME: &str = "taide-cli";
const MACOS_BUNDLE_MACOS_DIR_NAME: &str = "MacOS";
const MACOS_BUNDLE_CONTENTS_DIR_NAME: &str = "Contents";
const MACOS_BUNDLE_EXTENSION: &str = "app";
const OSASCRIPT_ADMIN_PRIVILEGES_SUFFIX: &str = "with administrator privileges";
const OSASCRIPT_ARG_FLAG: &str = "-e";
const OSASCRIPT_CANCELLED_EXIT_CODE: i32 = 1;
const OSASCRIPT_CANCELLED_STDERR_MARKER: &str = "-128";

fn process_basename(raw: &str) -> &str {
    raw.rsplit(['/', '\\']).next().unwrap_or(raw)
}

fn match_agent_name_in(base: &str, known: &[&'static str]) -> Option<&'static str> {
    known
        .iter()
        .copied()
        .find(|&name| base == name || (base.len() >= LINUX_COMM_MAX_LEN && name.starts_with(base)))
}

pub fn detect_agent_name(comm: &str, cmdline: &str) -> Option<&'static str> {
    let comm_base = process_basename(comm.trim());

    if let Some(name) = match_agent_name_in(comm_base, KNOWN_AGENT_NAMES) {
        return Some(name);
    }

    if !NODE_RUNTIME_NAMES.contains(&comm_base) {
        return None;
    }

    cmdline
        .split_whitespace()
        .map(process_basename)
        .find_map(|arg| match_agent_name_in(arg, KNOWN_AGENT_NAMES))
}

pub fn validate_wait_marker_path(marker: &str, temp_dir: &Path) -> AppResult<PathBuf> {
    let candidate = PathBuf::from(marker);

    if !candidate.is_absolute() {
        return Err(AppError::InvalidArgument(format!("wait marker path must be absolute: {marker}")));
    }

    let file_name = candidate
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::InvalidArgument(format!("invalid wait marker file name: {marker}")))?;

    if !file_name.starts_with(WAIT_MARKER_PREFIX) {
        return Err(AppError::InvalidArgument(format!(
            "wait marker file name must start with {WAIT_MARKER_PREFIX}: {marker}"
        )));
    }

    let parent = candidate
        .parent()
        .ok_or_else(|| AppError::InvalidArgument(format!("invalid wait marker path: {marker}")))?;

    if parent != temp_dir {
        return Err(AppError::InvalidArgument(format!(
            "wait marker path must be under temp dir: {marker}"
        )));
    }

    Ok(candidate)
}

pub fn build_cli_install_status(target_path: &str, installed: bool, resolved_path: Option<String>, dangling: bool) -> CliInstallStatus {
    CliInstallStatus {
        installed,
        resolved_path,
        dangling,
        target_path: target_path.to_string(),
        editor_env_hint: EDITOR_ENV_HINT.to_string(),
    }
}

/// The `EDITOR`/`VISUAL` entries a newly spawned terminal should inherit so Claude Code's ctrl+g
/// (which resolves `$VISUAL` before `$EDITOR`, opens `<editor> <tmpfile>` and treats the editor's
/// exit as the completion signal) round-trips through the running TAIDE: the `taide` CLI's
/// absolute path, unquoted, followed by `--wait`, assigned to both names.
///
/// Both variables carry the same value because Claude Code prefers `VISUAL` — injecting only
/// `EDITOR` left ctrl+g on vim for anyone whose environment already exported a `VISUAL`.
/// Parent-process values are deliberately overridden (user decision 2026-08-30,
/// `docs/acknowledge/2026-08-30-usability-batch-decisions.md`); a shell rc that exports its own
/// `EDITOR`/`VISUAL` still wins, since rc files run after the spawn — see
/// `docs/features/agent-integration.md` §2.3.
///
/// The value is **not** shell syntax — Claude Code splits it on spaces and spawns the first token
/// directly (`spawnSync(argv0, [...rest, tmpfile])`, no shell), so a quoted path would be looked
/// up verbatim (quotes included) and fail with `ENOENT`.
///
/// Empty — i.e. inject nothing — in two cases:
/// - `cli_path` could not be resolved (dev build without a CLI sidecar), where a bogus editor
///   would be worse than leaving ctrl+g on its `vi` default;
/// - `cli_path` contains whitespace, which that space-splitting consumer cannot express at all.
pub fn build_editor_env_entries(cli_path: Option<&str>) -> Vec<(String, String)> {
    let Some(path) = cli_path.filter(|path| !path.contains(char::is_whitespace)) else {
        return Vec::new();
    };
    let value = format!("{path} {CLI_WAIT_FLAG}");
    vec![(EDITOR_ENV_NAME.to_string(), value.clone()), (VISUAL_ENV_NAME.to_string(), value)]
}

/// True when `exe_path` sits under a macOS `.app` bundle's `Contents/MacOS/` directory
/// (e.g. `/Applications/TAIDE.app/Contents/MacOS/TAIDE`). Used to reject CLI shell-command
/// install/uninstall from unbundled dev builds, since the CLI sidecar only exists in the bundle.
pub fn is_running_from_macos_app_bundle(exe_path: &Path) -> bool {
    let Some(macos_dir) = exe_path.parent() else { return false };
    if macos_dir.file_name().and_then(|name| name.to_str()) != Some(MACOS_BUNDLE_MACOS_DIR_NAME) {
        return false;
    }
    let Some(contents_dir) = macos_dir.parent() else { return false };
    if contents_dir.file_name().and_then(|name| name.to_str()) != Some(MACOS_BUNDLE_CONTENTS_DIR_NAME) {
        return false;
    }
    let Some(app_dir) = contents_dir.parent() else { return false };
    app_dir.extension().and_then(|extension| extension.to_str()) == Some(MACOS_BUNDLE_EXTENSION)
}

/// Resolves the CLI sidecar path (`.../Contents/MacOS/taide-cli`) to symlink to, derived from the
/// currently running app's own executable path. Returns `None` for unbundled dev builds.
pub fn resolve_cli_install_target(exe_path: &Path) -> Option<PathBuf> {
    if !is_running_from_macos_app_bundle(exe_path) {
        return None;
    }
    exe_path.parent().map(|macos_dir| macos_dir.join(CLI_SIDECAR_BIN_NAME))
}

/// Escapes a value so it can be embedded inside an AppleScript double-quoted string literal.
pub fn escape_applescript_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Wraps a value in single quotes for safe embedding as one POSIX shell argument.
pub fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Builds the `do shell script ... with administrator privileges` AppleScript source that creates
/// (or replaces) the `taide` symlink. Passed to `osascript -e` as a single argv element (no shell
/// involved), so only the AppleScript string escaping and the inner shell single-quoting matter.
pub fn build_cli_install_apple_script(target: &Path, source: &Path) -> String {
    let shell_command = format!(
        "mkdir -p /usr/local/bin && ln -sf {} {}",
        shell_single_quote(&target.to_string_lossy()),
        shell_single_quote(&source.to_string_lossy()),
    );
    format!(
        "do shell script \"{}\" {OSASCRIPT_ADMIN_PRIVILEGES_SUFFIX}",
        escape_applescript_string(&shell_command)
    )
}

/// Builds the `do shell script ... with administrator privileges` AppleScript source that removes
/// the `taide` symlink (used only when a plain `remove_file` fails with `PermissionDenied`).
pub fn build_cli_uninstall_apple_script(source: &Path) -> String {
    let shell_command = format!("rm {}", shell_single_quote(&source.to_string_lossy()));
    format!(
        "do shell script \"{}\" {OSASCRIPT_ADMIN_PRIVILEGES_SUFFIX}",
        escape_applescript_string(&shell_command)
    )
}

/// Builds the `osascript` argv (excluding the program name itself) for a given AppleScript source.
/// `-e` is passed as its own argv element so the script never goes through a shell.
pub fn build_osascript_args(script: &str) -> Vec<String> {
    vec![OSASCRIPT_ARG_FLAG.to_string(), script.to_string()]
}

/// True when an `osascript` failure represents the user dismissing the administrator-privileges
/// prompt (exit code 1, stderr containing the `-128` "User canceled" marker) rather than a real
/// error. Callers should treat this as a quiet no-op, not a failure.
pub fn is_osascript_user_cancelled(exit_code: Option<i32>, stderr: &str) -> bool {
    exit_code == Some(OSASCRIPT_CANCELLED_EXIT_CODE) && stderr.contains(OSASCRIPT_CANCELLED_STDERR_MARKER)
}

/// True when a symlink's target file name is the CLI sidecar binary, i.e. the symlink was created
/// by TAIDE's own install flow. Used to refuse uninstalling a symlink/file we don't own.
pub fn is_cli_symlink_owned(link_target: &Path) -> bool {
    link_target.file_name().and_then(|name| name.to_str()) == Some(CLI_SIDECAR_BIN_NAME)
}

pub fn parse_cli_payload(argv: &[String]) -> Option<ExternalOpenRequest> {
    let mut wait_marker: Option<String> = None;
    let mut path: Option<String> = None;

    let mut iter = argv.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == CLI_WAIT_MARKER_FLAG {
            wait_marker = iter.next().cloned();
        } else if path.is_none() {
            path = Some(arg.clone());
        }
    }

    path.map(|path| ExternalOpenRequest { path, wait_marker })
}

pub fn agents_changed(previous: &[DetectedAgent], current: &[DetectedAgent]) -> bool {
    if previous.len() != current.len() {
        return true;
    }

    let mut previous_sorted = previous.to_vec();
    let mut current_sorted = current.to_vec();
    previous_sorted.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    current_sorted.sort_by(|a, b| a.session_id.cmp(&b.session_id));

    previous_sorted != current_sorted
}

/// One row of the unix probe's `ps -o pid=,comm=,args=` output.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub comm: String,
    pub cmdline: String,
}

/// Splits a whole `ps -o pid=,comm=,args=` listing into one [`ProcessInfo`] per pid. The pid
/// column exists only so a **single** `ps` call can serve every pty session at once (one fork per
/// poll tick instead of one per session); rows for pids that have already exited are simply absent,
/// which is why the caller looks its pids up in the returned map rather than zipping by position.
/// Unparsable rows — including the error line `ps` prints for a pid it rejects outright — are
/// skipped rather than aborting the batch.
///
/// The scheduler state column this used to carry is gone: it only ever distinguished `R` from `S`,
/// and every way an agent waits — for the model, for a permission answer, for the next prompt — is
/// `S`, so it could not tell working from blocked from idle apart (contract 2026-09-06 §0.1). Agent
/// activity now comes from the session's own output ([`classify_session`]) and `ps` resolves names
/// only.
pub fn parse_ps_process_infos(stdout: &str) -> HashMap<u32, ProcessInfo> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse::<u32>().ok()?;
            let comm = parts.next()?.to_string();
            let cmdline = parts.collect::<Vec<_>>().join(" ");
            Some((pid, ProcessInfo { comm, cmdline }))
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub cmdline: String,
}

pub fn find_descendant_agent(processes: &[ProcessSnapshot], root_pid: u32) -> Option<(u32, &'static str)> {
    let mut frontier = vec![root_pid];
    let mut visited: HashSet<u32> = HashSet::new();

    while let Some(pid) = frontier.pop() {
        if !visited.insert(pid) {
            continue;
        }

        for process in processes.iter().filter(|process| process.parent_pid == Some(pid)) {
            if let Some(name) = detect_agent_name(&process.name, &process.cmdline) {
                return Some((process.pid, name));
            }
            frontier.push(process.pid);
        }
    }

    None
}

const WINDOWS_EXE_SUFFIX: &str = ".exe";

pub fn strip_windows_exe_suffix(name: &str) -> &str {
    if name.len() > WINDOWS_EXE_SUFFIX.len() && name[name.len() - WINDOWS_EXE_SUFFIX.len()..].eq_ignore_ascii_case(WINDOWS_EXE_SUFFIX) {
        &name[..name.len() - WINDOWS_EXE_SUFFIX.len()]
    } else {
        name
    }
}

#[derive(Debug, Clone)]
pub struct DetectedAgentProbe {
    pub session_id: String,
    /// Always one of [`KNOWN_AGENT_NAMES`], so it is the `&'static str` [`detect_agent_name`]
    /// returned rather than a copy — session signals key their agent-specific tables off it.
    pub name: &'static str,
    pub pid: u32,
}

#[derive(Debug, Deserialize)]
pub struct HookPayload {
    pub hook_event_name: String,
    #[serde(default)]
    pub cwd: String,
}

/// One in-band agent event, as carried by the OSC 777 payload TAIDE's own hook commands write into
/// the session's pty (contract 2026-09-06 §1.1). [`AgentEvent::wire_name`] is the single definition
/// of each spelling: the hook command that emits it is built from the same table that parses it
/// back, so a rename cannot leave installed hooks speaking a vocabulary the reader lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentEvent {
    PermissionRequest,
    QuestionAsked,
    ToolComplete,
    Stop,
    StopFailure,
    IdlePrompt,
}

const ALL_AGENT_EVENTS: [AgentEvent; 6] = [
    AgentEvent::PermissionRequest,
    AgentEvent::QuestionAsked,
    AgentEvent::ToolComplete,
    AgentEvent::Stop,
    AgentEvent::StopFailure,
    AgentEvent::IdlePrompt,
];

impl AgentEvent {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::PermissionRequest => "permission_request",
            Self::QuestionAsked => "question_asked",
            Self::ToolComplete => "tool_complete",
            Self::Stop => "stop",
            Self::StopFailure => "stop_failure",
            Self::IdlePrompt => "idle_prompt",
        }
    }

    fn from_wire(raw: &str) -> Option<Self> {
        ALL_AGENT_EVENTS.into_iter().find(|event| event.wire_name() == raw)
    }
}

/// Why a session counts as blocked — an event the agent itself sent, or a dialog phrase read off
/// its own output. Kept apart so the two can be told apart later even though both latch the same
/// way today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockedSource {
    Event,
    Dialog,
}

/// The leading glyph of Claude Code's window title: the two half-circles it alternates while a turn
/// runs, and the asterisk it settles on when the turn ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleGlyph {
    Working,
    Idle,
}

const TITLE_WORKING_GLYPHS: [char; 2] = ['◐', '◑'];
const TITLE_IDLE_GLYPH: char = '✳';

/// Glyphs that repaint on their own while nothing is actually happening — the `⏺` that keeps
/// blinking every ~600ms *through* a permission dialog, and the spinner frames — so a chunk made of
/// only these is not the agent printing anything.
const NON_SUBSTANTIVE_GLYPHS: [char; 9] = ['⏺', '✶', '✻', '✽', '✢', '✳', '◐', '◑', '·'];

/// Phrases that only appear while Claude Code is holding a turn open for an answer. Matched
/// case-sensitively on purpose: the running-turn footer is the lowercase `esc to interrupt`, and a
/// case-insensitive match on `Esc to cancel` would read every working session as blocked.
///
/// Claude only, this batch — codex/gemini dialog wording is unverified and guessing it would invent
/// signals rather than read them (contract §1.1.3).
pub const CLAUDE_DIALOG_SIGNATURES: &[&str] = &["Do you want to proceed?", "Would you like to proceed?", "Esc to cancel"];

fn dialog_signatures_for(agent_name: &str) -> &'static [&'static str] {
    match agent_name {
        AGENT_NAME_CLAUDE => CLAUDE_DIALOG_SIGNATURES,
        _ => &[],
    }
}

pub fn find_dialog_signature(agent_name: &str, text: &str) -> bool {
    dialog_signatures_for(agent_name).iter().any(|signature| text.contains(signature))
}

/// Whether a signature straddles the boundary between the previous chunk's tail and this one — a
/// phrase that is in neither string on its own.
///
/// Each side is clipped to one byte short of the signature's own length, so a phrase lying wholly
/// inside the carried tail cannot match. That exclusion is the point: the tail still holds the
/// dialog the user has just answered, and matching it again would re-latch a session that the
/// resumed output released — the badge would fall back to "awaiting input" the moment the answer
/// was given, which is the very failure this layer exists to fix.
fn find_dialog_signature_across_boundary(agent_name: &str, overlap: &str, text: &str) -> bool {
    if overlap.is_empty() {
        return false;
    }
    dialog_signatures_for(agent_name).iter().any(|signature| {
        let span = signature.len().saturating_sub(1);
        format!("{}{}", str_tail(overlap, span), str_head(text, span)).contains(signature)
    })
}

fn str_head(text: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn str_tail(text: &str, max_bytes: usize) -> &str {
    let mut start = text.len().saturating_sub(max_bytes);
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

pub fn is_substantive_output(text: &str) -> bool {
    text.chars()
        .filter(|character| !character.is_whitespace() && !NON_SUBSTANTIVE_GLYPHS.contains(character))
        .nth(SUBSTANTIVE_OUTPUT_MIN_CHARS - 1)
        .is_some()
}

pub fn parse_title_glyph(title: &str) -> Option<TitleGlyph> {
    let first = title.trim_start().chars().next()?;
    if TITLE_WORKING_GLYPHS.contains(&first) {
        return Some(TitleGlyph::Working);
    }
    (first == TITLE_IDLE_GLYPH).then_some(TitleGlyph::Idle)
}

#[derive(Debug, Deserialize)]
struct AgentEventBody {
    v: u32,
    agent: String,
    event: String,
}

/// Reads one OSC 777 body back into `(emitting agent, event)`. A body of another protocol version,
/// an event this build does not know, or anything that is not the expected JSON object is `None` —
/// pty output is untrusted, so an unrecognized payload changes no state at all.
pub fn parse_agent_event_body(body: &str) -> Option<(String, AgentEvent)> {
    let parsed: AgentEventBody = serde_json::from_str(body).ok()?;
    if parsed.v != AGENT_PROTOCOL_VERSION {
        return None;
    }
    Some((parsed.agent, AgentEvent::from_wire(&parsed.event)?))
}

/// Everything one pty session has told us about its agent, in the form [`classify_session`] reads.
/// Instants rather than durations so the classifier can be handed an explicit `now` and tested
/// without waiting.
#[derive(Debug, Clone)]
pub struct AgentSessionSignals {
    /// Which agent this session runs, so the signature table and event filter are picked without a
    /// second lookup on the hot per-chunk path.
    pub agent_name: &'static str,
    pub last_substantive_output_at: Option<Instant>,
    pub last_input_at: Option<Instant>,
    pub title_glyph: Option<TitleGlyph>,
    pub title_seen_at: Option<Instant>,
    /// The last "the turn is over" hint — a `stop`/`stop_failure` event, an idle title glyph, or a
    /// terminal notification. A hint alone never means idle (under tmux the title is a static `✳`
    /// while output keeps flowing); it only shortens the quiet window the session must then hold.
    pub idle_hint_at: Option<Instant>,
    pub blocked: Option<BlockedSource>,
    pub last_event: Option<(AgentEvent, Instant)>,
}

impl AgentSessionSignals {
    pub fn new(agent_name: &'static str) -> Self {
        Self {
            agent_name,
            last_substantive_output_at: None,
            last_input_at: None,
            title_glyph: None,
            title_seen_at: None,
            idle_hint_at: None,
            blocked: None,
            last_event: None,
        }
    }
}

fn elapsed_ms(since: Instant, now: Instant) -> u64 {
    now.saturating_duration_since(since).as_millis() as u64
}

fn is_fresh(at: Option<Instant>, window_ms: u64, now: Instant) -> bool {
    at.is_some_and(|instant| elapsed_ms(instant, now) < window_ms)
}

fn is_echo(signals: &AgentSessionSignals, now: Instant) -> bool {
    is_fresh(signals.last_input_at, ECHO_SUPPRESS_MS, now)
}

fn apply_title(signals: &mut AgentSessionSignals, title: &str, now: Instant) {
    let Some(glyph) = parse_title_glyph(title) else {
        return;
    };
    signals.title_glyph = Some(glyph);
    signals.title_seen_at = Some(now);
    if glyph == TitleGlyph::Idle {
        signals.idle_hint_at = Some(now);
    }
}

/// Applies one in-band event, returning whether it left the session blocked. `None` means the
/// payload was not applied at all — unparsable, addressed to another agent, or `idle_prompt`, which
/// carries no state change.
fn apply_agent_event(signals: &mut AgentSessionSignals, body: &str, agent_name: &str, now: Instant) -> Option<bool> {
    let (emitter, event) = parse_agent_event_body(body)?;
    if emitter != agent_name || event == AgentEvent::IdlePrompt {
        return None;
    }

    let blocks = match event {
        AgentEvent::PermissionRequest | AgentEvent::QuestionAsked => {
            signals.blocked = Some(BlockedSource::Event);
            true
        }
        AgentEvent::ToolComplete => {
            signals.blocked = None;
            false
        }
        AgentEvent::Stop | AgentEvent::StopFailure => {
            signals.blocked = None;
            signals.idle_hint_at = Some(now);
            false
        }
        AgentEvent::IdlePrompt => false,
    };
    signals.last_event = Some((event, now));
    Some(blocks)
}

/// Folds one scanned output chunk into a session's signals.
///
/// Within one chunk, evidence that the session is blocked wins over evidence that it is not, and
/// the ordering here is what enforces that. Substantive output releases the latch, but not one an
/// event in this very chunk just set (the `PermissionRequest` hook fires the moment the dialog is
/// drawn, so its OSC and the frame's first bytes arrive together), and the dialog scan runs last
/// because the dialog frame *is* substantive output — clearing afterwards would undo the very latch
/// that frame is evidence for, while clearing first lets one chunk both end a previous block and
/// open a new one.
///
/// The echo window gates only the substantive-output half: what comes back within
/// [`ECHO_SUPPRESS_MS`] of a keystroke is the terminal drawing that keystroke, and counting it would
/// make typing look like the agent working. It does not gate the dialog scan. Answering one
/// approval makes Claude render the next one milliseconds later — inside that window — and nothing
/// redraws that frame afterwards (only the `⏺` blink flows, which is not substantive), so a frame
/// skipped here is a block missed for good. The cost of reading it is that a user who types a
/// signature phrase at the prompt latches the session until the next substantive output; that is
/// self-healing, a missed dialog is not.
pub fn apply_scan_to_signals(signals: &mut AgentSessionSignals, outcome: &ScanOutcome, agent_name: &str, now: Instant) {
    let mut blocked_by_event = false;
    for event in &outcome.events {
        match event {
            ScanEvent::Title(title) => apply_title(signals, title, now),
            ScanEvent::AgentEvent(body) => {
                if let Some(blocks) = apply_agent_event(signals, body, agent_name, now) {
                    blocked_by_event = blocks;
                }
            }
            ScanEvent::Notification9(_) => signals.idle_hint_at = Some(now),
            ScanEvent::Cwd(_) | ScanEvent::CommandMarker(_) => {}
        }
    }

    if !is_echo(signals, now) && is_substantive_output(&outcome.text) {
        signals.last_substantive_output_at = Some(now);
        if !blocked_by_event {
            signals.blocked = None;
        }
    }

    if find_dialog_signature(agent_name, &outcome.text)
        || find_dialog_signature_across_boundary(agent_name, &outcome.overlap, &outcome.text)
    {
        signals.blocked = Some(BlockedSource::Dialog);
    }
}

/// Records that the user just wrote to this session's pty. Whoever is typing is looking at the
/// terminal, so whatever it was waiting on has been seen and the latch is released; the timestamp
/// also opens the echo window [`apply_scan_to_signals`] reads.
pub fn note_input(signals: &mut AgentSessionSignals, now: Instant) {
    signals.last_input_at = Some(now);
    signals.blocked = None;
}

fn last_signal_at(signals: &AgentSessionSignals) -> Option<Instant> {
    [
        signals.last_substantive_output_at,
        signals.last_input_at,
        signals.title_seen_at,
        signals.idle_hint_at,
        signals.last_event.map(|(_, at)| at),
    ]
    .into_iter()
    .flatten()
    .max()
}

/// Turns a session's accumulated signals into the badge state, in the priority order of contract
/// §1.2. `previous` is returned for the window between "no longer freshly working" and "quiet long
/// enough to be idle" — the hysteresis that keeps a badge from flickering on every 500ms tick.
pub fn classify_session(signals: &AgentSessionSignals, previous: AgentActivity, now: Instant) -> AgentActivity {
    if signals.blocked.is_some() {
        return AgentActivity::AwaitingInput;
    }
    if signals.title_glyph == Some(TitleGlyph::Working) && is_fresh(signals.title_seen_at, TITLE_WORKING_FRESH_MS, now) {
        return AgentActivity::Working;
    }
    if is_fresh(signals.last_substantive_output_at, ACTIVITY_WORKING_HOLD_MS, now) {
        return AgentActivity::Working;
    }
    if matches!(signals.last_event, Some((AgentEvent::ToolComplete, at)) if elapsed_ms(at, now) < ACTIVITY_WORKING_HOLD_MS) {
        return AgentActivity::Working;
    }

    let Some(latest) = last_signal_at(signals) else {
        return AgentActivity::Unknown;
    };
    let quiet_ms = elapsed_ms(latest, now);

    if signals.idle_hint_at.is_some() && quiet_ms >= ACTIVITY_WORKING_HOLD_MS {
        return AgentActivity::Idle;
    }
    if quiet_ms >= ACTIVITY_IDLE_QUIET_MS {
        return AgentActivity::Idle;
    }
    previous
}

pub use crate::infra::crypto::constant_time_eq;

fn is_cwd_within_root(cwd: &str, root: &str) -> bool {
    let cwd = cwd.trim_end_matches('/');
    let root = root.trim_end_matches('/');
    cwd == root || cwd.starts_with(&format!("{root}/"))
}

pub fn match_project_by_cwd<'a>(cwd: &str, projects: &'a [(ProjectId, String)]) -> Option<&'a ProjectId> {
    if cwd.is_empty() {
        return None;
    }
    projects
        .iter()
        .filter(|(_, root)| is_cwd_within_root(cwd, root))
        .max_by_key(|(_, root)| root.len())
        .map(|(id, _)| id)
}

pub fn map_hook_event_to_activity(agent_name: &str, event_name: &str) -> Option<AgentActivity> {
    match agent_name {
        AGENT_NAME_CLAUDE => match event_name {
            HOOK_EVENT_USER_PROMPT_SUBMIT => Some(AgentActivity::Working),
            HOOK_EVENT_NOTIFICATION => Some(AgentActivity::AwaitingInput),
            HOOK_EVENT_STOP => Some(AgentActivity::Idle),
            _ => None,
        },
        AGENT_NAME_CODEX => match event_name {
            HOOK_EVENT_USER_PROMPT_SUBMIT => Some(AgentActivity::Working),
            HOOK_EVENT_PERMISSION_REQUEST => Some(AgentActivity::AwaitingInput),
            HOOK_EVENT_POST_TOOL_USE => Some(AgentActivity::Working),
            HOOK_EVENT_STOP => Some(AgentActivity::Idle),
            _ => None,
        },
        AGENT_NAME_GEMINI => match event_name {
            HOOK_EVENT_BEFORE_AGENT => Some(AgentActivity::Working),
            HOOK_EVENT_NOTIFICATION => Some(AgentActivity::AwaitingInput),
            HOOK_EVENT_AFTER_AGENT => Some(AgentActivity::Idle),
            _ => None,
        },
        _ => None,
    }
}

pub fn is_hook_managed_agent(name: &str) -> bool {
    KNOWN_AGENT_NAMES.contains(&name)
}

pub fn hook_scope_for_agent(agent_name: &str) -> AppResult<HookInstallScope> {
    match agent_name {
        AGENT_NAME_CLAUDE => Ok(HookInstallScope::Project),
        AGENT_NAME_CODEX | AGENT_NAME_GEMINI => Ok(HookInstallScope::User),
        other => Err(AppError::InvalidArgument(format!("unknown agent name: {other}"))),
    }
}

pub fn build_command_hook_shell_command(taide_cli_path: &str, hook_url: &str) -> String {
    format!("\"{taide_cli_path}\" hook --url \"{hook_url}\"")
}

pub fn managed_hook_events_for(agent_name: &str) -> &'static [&'static str] {
    match agent_name {
        AGENT_NAME_CLAUDE => CLAUDE_MANAGED_HOOK_EVENTS,
        AGENT_NAME_CODEX => CODEX_MANAGED_HOOK_EVENTS,
        AGENT_NAME_GEMINI => GEMINI_MANAGED_HOOK_EVENTS,
        _ => &[],
    }
}

/// How an installed Claude hook gets its event into the session's terminal.
///
/// `TerminalSequence` hands Claude the sequence in the hook's JSON output and lets Claude write it
/// to its own terminal (2.1.141+); `DevTty` writes it to `/dev/tty` directly, which is all an older
/// build — or one whose version could not be read — leaves available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEmitter {
    TerminalSequence,
    DevTty,
}

const HOOK_TERMINAL_SEQUENCE_FIELD: &str = "terminalSequence";

/// The seven `(hook event, matcher, agent event)` bindings TAIDE installs for Claude (contract
/// §1.4). `UserPromptSubmit` and `SessionStart` are deliberately absent: both feed a hook's stdout
/// back as prompt context, and working state is already covered by the title and output signals.
const CLAUDE_HOOK_BINDINGS: &[(&str, Option<&str>, AgentEvent)] = &[
    (HOOK_EVENT_PERMISSION_REQUEST, None, AgentEvent::PermissionRequest),
    (
        HOOK_EVENT_NOTIFICATION,
        Some(NOTIFICATION_MATCHER_PERMISSION_PROMPT),
        AgentEvent::PermissionRequest,
    ),
    (
        HOOK_EVENT_NOTIFICATION,
        Some(NOTIFICATION_MATCHER_ELICITATION_DIALOG),
        AgentEvent::QuestionAsked,
    ),
    (
        HOOK_EVENT_NOTIFICATION,
        Some(NOTIFICATION_MATCHER_IDLE_PROMPT),
        AgentEvent::IdlePrompt,
    ),
    (HOOK_EVENT_POST_TOOL_USE, None, AgentEvent::ToolComplete),
    (HOOK_EVENT_STOP, None, AgentEvent::Stop),
    (HOOK_EVENT_STOP_FAILURE, None, AgentEvent::StopFailure),
];

fn claude_agent_event_payload(event: AgentEvent) -> String {
    format!(
        "{{\"v\":{AGENT_PROTOCOL_VERSION},\"agent\":\"{AGENT_NAME_CLAUDE}\",\"event\":\"{}\"}}",
        event.wire_name()
    )
}

fn agent_osc_sequence(payload: &str) -> String {
    format!("\u{1b}]{OSC_NOTIFY_IDENT};{OSC_NOTIFY_SUBCOMMAND};{AGENT_OSC_SENTINEL};{payload}\u{7}")
}

/// Builds the whole shell command one installed Claude hook runs.
///
/// Three properties the shape encodes: it is gated on `TAIDE_AGENT_PROTOCOL_VERSION`, so the same
/// `settings.local.json` emits nothing when that project is opened in iTerm instead of TAIDE; it
/// needs no jq, no CLI and no server — the payload is a constant; and it always exits 0, because a
/// `/dev/tty` that cannot be opened must not surface as a failing hook in the user's transcript.
///
/// The `TerminalSequence` envelope is built with `serde_json` rather than by hand so the control
/// bytes come out as the JSON unicode escapes Claude's own reader will decode, and the payload's
/// inner quotes are escaped by that same encoder. The result carries no single quote, which is what
/// makes wrapping it in shell single quotes safe (`훅_명령은_단일_인용을_깨뜨리는_문자를_담지_않는다`).
pub fn build_claude_agent_hook_command(event: AgentEvent, emitter: HookEmitter) -> String {
    let payload = claude_agent_event_payload(event);
    let emit = match emitter {
        HookEmitter::TerminalSequence => {
            let envelope = serde_json::json!({ HOOK_TERMINAL_SEQUENCE_FIELD: agent_osc_sequence(&payload) });
            format!("printf '%s' '{envelope}'")
        }
        HookEmitter::DevTty => {
            format!("printf '\\033]{OSC_NOTIFY_IDENT};{OSC_NOTIFY_SUBCOMMAND};{AGENT_OSC_SENTINEL};{payload}\\007' > /dev/tty 2>/dev/null")
        }
    };

    format!("if [ -n \"${AGENT_PROTOCOL_VERSION_ENV_NAME}\" ]; then {emit}; fi; exit 0")
}

/// The `(hook event, matcher, command)` rows an install writes for Claude.
pub fn claude_hook_entries(emitter: HookEmitter) -> Vec<(&'static str, Option<&'static str>, String)> {
    CLAUDE_HOOK_BINDINGS
        .iter()
        .map(|(hook_event, matcher, event)| (*hook_event, *matcher, build_claude_agent_hook_command(*event, emitter)))
        .collect()
}

/// Reads the highest `<major>.<minor>.<patch>` triple out of `claude --version` output (`2.1.263
/// (Claude Code)`), ignoring any pre-release suffix on the patch field.
pub fn parse_claude_version(stdout: &str) -> Option<(u32, u32, u32)> {
    stdout.split_whitespace().find_map(parse_version_triple)
}

fn parse_version_triple(token: &str) -> Option<(u32, u32, u32)> {
    let mut fields = token.split('.');
    let major = fields.next()?.parse().ok()?;
    let minor = fields.next()?.parse().ok()?;
    let patch = fields
        .next()?
        .split(|character: char| !character.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

pub fn supports_terminal_sequence(version: (u32, u32, u32)) -> bool {
    version >= CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION
}

pub fn user_level_hook_command_timeout(agent_name: &str) -> u64 {
    match agent_name {
        AGENT_NAME_CODEX => CODEX_HOOK_COMMAND_TIMEOUT_SECONDS,
        AGENT_NAME_GEMINI => GEMINI_HOOK_COMMAND_TIMEOUT_MS,
        _ => HOOKS_HTTP_TIMEOUT_SECONDS,
    }
}

const CODEX_HOME_RELATIVE_PATH: &str = ".codex/hooks.json";
const GEMINI_HOME_RELATIVE_PATH: &str = ".gemini/settings.json";

pub fn user_level_hooks_path(agent_name: &str, home_env: Option<&str>) -> AppResult<PathBuf> {
    let relative = match agent_name {
        AGENT_NAME_CODEX => CODEX_HOME_RELATIVE_PATH,
        AGENT_NAME_GEMINI => GEMINI_HOME_RELATIVE_PATH,
        other => return Err(AppError::InvalidArgument(format!("agent has no user-level hooks path: {other}"))),
    };
    let home = home_env
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Internal("home directory not found".to_string()))?;
    Ok(PathBuf::from(home).join(relative))
}

/// What makes a hook entry TAIDE's own: the query marker the retired HTTP hooks carried in their
/// URL, and the OSC payload prefix the in-band command hooks carry in their command. Both are
/// recognized everywhere so an install from an older build is still found, reconciled and removed.
const TAIDE_HOOK_MARKERS: [&str; 2] = [HOOKS_URL_MARKER, AGENT_OSC_MARKER];

fn json_value_contains_marker(value: &serde_json::Value, marker: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text.contains(marker),
        serde_json::Value::Array(items) => items.iter().any(|item| json_value_contains_marker(item, marker)),
        serde_json::Value::Object(map) => map.values().any(|item| json_value_contains_marker(item, marker)),
        _ => false,
    }
}

pub fn has_taide_marker_anywhere(root: &serde_json::Value) -> bool {
    TAIDE_HOOK_MARKERS.iter().any(|marker| json_value_contains_marker(root, marker))
}

fn hook_handler_value(hook: &serde_json::Value) -> Option<&str> {
    hook.get("command").or_else(|| hook.get("url")).and_then(|value| value.as_str())
}

fn is_taide_hook_handler(hook: &serde_json::Value) -> bool {
    hook_handler_value(hook).is_some_and(|value| TAIDE_HOOK_MARKERS.iter().any(|marker| value.contains(marker)))
}

fn is_taide_managed_entry(entry: &serde_json::Value) -> bool {
    entry
        .get("hooks")
        .and_then(|hooks| hooks.as_array())
        .map(|hooks| hooks.iter().any(is_taide_hook_handler))
        .unwrap_or(false)
}

fn remove_taide_managed_entries(mut root: serde_json::Value, events: &[&str]) -> serde_json::Value {
    if let Some(hooks_obj) = root.get_mut("hooks").and_then(|value| value.as_object_mut()) {
        for event in events {
            if let Some(entries) = hooks_obj.get_mut(*event).and_then(|value| value.as_array_mut()) {
                entries.retain(|entry| !is_taide_managed_entry(entry));
            }
        }
        hooks_obj.retain(|_, value| !value.as_array().is_some_and(|entries| entries.is_empty()));
    }
    if root
        .get("hooks")
        .and_then(|value| value.as_object())
        .is_some_and(|hooks_obj| hooks_obj.is_empty())
    {
        if let Some(obj) = root.as_object_mut() {
            obj.remove("hooks");
        }
    }
    root
}

/// Walks down to `root.hooks.<event>`, replacing a non-object `hooks` or a non-array event slot
/// with an empty one of the right shape rather than failing — a hand-edited settings file must not
/// make the install panic or silently no-op.
fn managed_entries_slot<'a>(root: &'a mut serde_json::Value, event: &str) -> Option<&'a mut Vec<serde_json::Value>> {
    let root_obj = root.as_object_mut()?;
    let hooks_slot = root_obj.entry("hooks".to_string()).or_insert_with(|| serde_json::json!({}));
    if !hooks_slot.is_object() {
        *hooks_slot = serde_json::json!({});
    }

    let event_slot = hooks_slot
        .as_object_mut()?
        .entry(event.to_string())
        .or_insert_with(|| serde_json::json!([]));
    if !event_slot.is_array() {
        *event_slot = serde_json::json!([]);
    }
    event_slot.as_array_mut()
}

fn inject_taide_managed_entries(root: serde_json::Value, events: &[&str], hook_handler: serde_json::Value) -> serde_json::Value {
    let root = remove_taide_managed_entries(root, events);
    let mut root = if root.is_object() { root } else { serde_json::json!({}) };

    for event in events {
        if let Some(entries) = managed_entries_slot(&mut root, event) {
            entries.push(serde_json::json!({ "hooks": [hook_handler.clone()] }));
        }
    }

    root
}

/// Removes every TAIDE-owned Claude entry, across both the in-band command install and the retired
/// HTTP one ([`CLAUDE_MANAGED_HOOK_EVENTS`]).
pub fn remove_taide_hook_entries(root: serde_json::Value) -> serde_json::Value {
    remove_taide_managed_entries(root, CLAUDE_MANAGED_HOOK_EVENTS)
}

/// Writes the current in-band Claude hook set, having first removed whatever TAIDE had there
/// before — so reinstalling, upgrading from the HTTP install, and switching emitter all converge on
/// the same file content rather than stacking entries.
pub fn inject_taide_claude_command_hook_entries(root: serde_json::Value, emitter: HookEmitter) -> serde_json::Value {
    let root = remove_taide_hook_entries(root);
    let mut root = if root.is_object() { root } else { serde_json::json!({}) };

    for (hook_event, matcher, command) in claude_hook_entries(emitter) {
        let handler = serde_json::json!({ "type": HOOK_HANDLER_TYPE_COMMAND, "command": command });
        let entry = match matcher {
            Some(matcher) => serde_json::json!({ "matcher": matcher, "hooks": [handler] }),
            None => serde_json::json!({ "hooks": [handler] }),
        };
        if let Some(entries) = managed_entries_slot(&mut root, hook_event) {
            entries.push(entry);
        }
    }

    root
}

/// Every TAIDE-owned Claude row actually present in the file, as `(event, matcher, handler)` — the
/// handler being the entry's `command`, or its `url` for a leftover HTTP install, which is what
/// makes such a leftover compare unequal to any expected command and get replaced.
fn installed_claude_entries(root: &serde_json::Value) -> Vec<(String, Option<String>, String)> {
    let Some(hooks_obj) = root.get("hooks").and_then(|value| value.as_object()) else {
        return Vec::new();
    };

    let mut installed = Vec::new();
    for event in CLAUDE_MANAGED_HOOK_EVENTS {
        let Some(entries) = hooks_obj.get(*event).and_then(|value| value.as_array()) else {
            continue;
        };
        for entry in entries.iter().filter(|entry| is_taide_managed_entry(entry)) {
            let matcher = entry.get("matcher").and_then(|value| value.as_str()).map(str::to_string);
            let hooks = entry.get("hooks").and_then(|value| value.as_array());
            for hook in hooks.into_iter().flatten() {
                installed.push((
                    (*event).to_string(),
                    matcher.clone(),
                    hook_handler_value(hook).unwrap_or_default().to_string(),
                ));
            }
        }
    }
    installed
}

/// Whether the file's TAIDE-owned Claude rows are *exactly* the set this build would install.
/// Compared as a whole rather than "every expected row exists", so a stale extra row (an event we
/// no longer install, a second emitter's copy) also counts as a mismatch and gets reconciled away.
pub fn claude_hook_entries_match(root: &serde_json::Value, emitter: HookEmitter) -> bool {
    let mut expected: Vec<_> = claude_hook_entries(emitter)
        .into_iter()
        .map(|(hook_event, matcher, command)| (hook_event.to_string(), matcher.map(str::to_string), command))
        .collect();
    let mut installed = installed_claude_entries(root);

    expected.sort();
    installed.sort();
    expected == installed
}

pub fn remove_taide_command_hook_entries(root: serde_json::Value, events: &[&str]) -> serde_json::Value {
    remove_taide_managed_entries(root, events)
}

pub fn inject_taide_command_hook_entries(root: serde_json::Value, events: &[&str], command: &str, timeout: u64) -> serde_json::Value {
    inject_taide_managed_entries(
        root,
        events,
        serde_json::json!({ "type": HOOK_HANDLER_TYPE_COMMAND, "command": command, "timeout": timeout }),
    )
}

/// Whether every one of `events` carries a TAIDE-managed entry running exactly `command` — the
/// freshness check the user-level (codex/gemini) reconcile makes before rewriting a file. An empty
/// `events` slice means "this agent has no managed hooks", which is not freshness, hence the
/// explicit guard rather than `[].iter().all(..)` vacuously returning `true`.
pub fn has_command_hook_entries_for_command(root: &serde_json::Value, events: &[&str], command: &str) -> bool {
    if events.is_empty() {
        return false;
    }
    let Some(hooks_obj) = root.get("hooks").and_then(|value| value.as_object()) else {
        return false;
    };
    events.iter().all(|event| {
        hooks_obj.get(*event).and_then(|value| value.as_array()).is_some_and(|entries| {
            entries
                .iter()
                .filter(|entry| is_taide_managed_entry(entry))
                .any(|entry| entry_has_command(entry, command))
        })
    })
}

fn entry_has_command(entry: &serde_json::Value, command: &str) -> bool {
    entry.get("hooks").and_then(|hooks| hooks.as_array()).is_some_and(|hooks| {
        hooks
            .iter()
            .any(|hook| hook.get("command").and_then(|value| value.as_str()) == Some(command))
    })
}

/// Whether this project has a TAIDE Claude install at all — either shape, any event.
pub fn has_taide_hook_entries(root: &serde_json::Value) -> bool {
    !installed_claude_entries(root).is_empty()
}

#[cfg(test)]
mod tests {
    use super::super::types::TEXT_OVERLAP_BYTES;
    use super::*;

    #[test]
    fn 프로세스명이_직접_claude면_감지한다() {
        assert_eq!(detect_agent_name("claude", "claude"), Some("claude"));
    }

    #[test]
    fn node_런타임_위에서_실행된_claude_바이너리를_감지한다() {
        let cmdline = "/usr/local/bin/node /Users/dev/.claude/local/claude";
        assert_eq!(detect_agent_name("node", cmdline), Some("claude"));
    }

    #[test]
    fn 무관한_프로세스는_감지하지_않는다() {
        assert_eq!(detect_agent_name("bash", "bash"), None);
        assert_eq!(detect_agent_name("node", "node server.js"), None);
    }

    #[test]
    fn comm이_리눅스_15자에서_잘려도_고유하게_매칭되면_감지한다() {
        let known: &[&str] = &["gemini-experimental"];
        let truncated = &"gemini-experimental"[..LINUX_COMM_MAX_LEN];
        assert_eq!(match_agent_name_in(truncated, known), Some("gemini-experimental"));
    }

    #[test]
    fn 짧은_접두사는_잘림으로_간주하지_않는다() {
        let known: &[&str] = &["claude"];
        assert_eq!(match_agent_name_in("cla", known), None);
    }

    #[test]
    fn 정상_마커_경로는_통과한다() {
        let temp = Path::new("/tmp");
        let result = validate_wait_marker_path("/tmp/taide-wait-abc123", temp);
        assert_eq!(result.unwrap(), PathBuf::from("/tmp/taide-wait-abc123"));
    }

    #[test]
    fn 접두사가_없는_파일명은_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("/tmp/other-file", temp).is_err());
    }

    #[test]
    fn temp_dir_밖의_경로는_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("/etc/taide-wait-abc123", temp).is_err());
    }

    #[test]
    fn 상대경로는_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("taide-wait-abc123", temp).is_err());
    }

    #[test]
    fn 상위_디렉토리_순회_시도는_거부한다() {
        let temp = Path::new("/tmp");
        assert!(validate_wait_marker_path("/tmp/../etc/taide-wait-abc123", temp).is_err());
    }

    #[test]
    fn cli_설치_상태_문자열을_구성한다() {
        let status = build_cli_install_status("/usr/local/bin/taide", true, Some("/usr/local/bin/taide".to_string()), false);
        assert!(status.installed);
        assert!(!status.dangling);
        assert_eq!(status.editor_env_hint, EDITOR_ENV_HINT);
    }

    #[test]
    fn dangling_상태도_설치_상태_문자열에_반영된다() {
        let status = build_cli_install_status("/usr/local/bin/taide", true, None, true);
        assert!(status.installed);
        assert!(status.dangling);
        assert!(status.resolved_path.is_none());
    }

    #[test]
    fn editor_env_는_editor_와_visual_에_같은_값을_인용없이_주입한다() {
        let entries = build_editor_env_entries(Some("/usr/local/bin/taide"));
        let expected = "/usr/local/bin/taide --wait".to_string();
        assert_eq!(
            entries,
            vec![("EDITOR".to_string(), expected.clone()), ("VISUAL".to_string(), expected)]
        );
    }

    #[test]
    fn 공백이_있는_사이드카_경로는_주입하지_않는다() {
        assert!(build_editor_env_entries(Some("/Applications/My TAIDE.app/Contents/MacOS/taide-cli")).is_empty());
    }

    #[test]
    fn cli_경로를_해석하지_못하면_주입하지_않는다() {
        assert!(build_editor_env_entries(None).is_empty());
    }

    #[test]
    fn 앱_번들_경로는_macos_번들_실행으로_판정한다() {
        assert!(is_running_from_macos_app_bundle(Path::new(
            "/Applications/TAIDE.app/Contents/MacOS/TAIDE"
        )));
    }

    #[test]
    fn 이동된_앱_번들_경로도_macos_번들_실행으로_판정한다() {
        assert!(is_running_from_macos_app_bundle(Path::new(
            "/Users/dev/Applications/TAIDE.app/Contents/MacOS/TAIDE"
        )));
    }

    #[test]
    fn dev_빌드_경로는_macos_번들_실행이_아니다() {
        assert!(!is_running_from_macos_app_bundle(Path::new(
            "/Users/dev/TAIDE/src-tauri/target/debug/taide"
        )));
    }

    #[test]
    fn 번들_구조가_비슷해도_확장자가_app이_아니면_거부한다() {
        assert!(!is_running_from_macos_app_bundle(Path::new(
            "/Applications/TAIDE.notapp/Contents/MacOS/TAIDE"
        )));
    }

    #[test]
    fn 번들_실행_파일_기준으로_사이드카_설치_대상_경로를_만든다() {
        let target = resolve_cli_install_target(Path::new("/Applications/TAIDE.app/Contents/MacOS/TAIDE")).unwrap();
        assert_eq!(target, PathBuf::from("/Applications/TAIDE.app/Contents/MacOS/taide-cli"));
    }

    #[test]
    fn dev_빌드에서는_설치_대상_경로가_없다() {
        assert!(resolve_cli_install_target(Path::new("/Users/dev/TAIDE/src-tauri/target/debug/taide")).is_none());
    }

    #[test]
    fn applescript_문자열은_백슬래시와_큰따옴표를_이스케이프한다() {
        assert_eq!(escape_applescript_string(r#"say "hi" \ bye"#), r#"say \"hi\" \\ bye"#);
    }

    #[test]
    fn 셸_단일_인용은_내부_작은따옴표를_안전하게_이스케이프한다() {
        assert_eq!(shell_single_quote("/usr/local/bin/taide"), "'/usr/local/bin/taide'");
        assert_eq!(shell_single_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn 설치_스크립트는_mkdir와_ln_sf를_관리자_권한으로_실행한다() {
        let script = build_cli_install_apple_script(
            Path::new("/Applications/TAIDE.app/Contents/MacOS/taide-cli"),
            Path::new("/usr/local/bin/taide"),
        );
        assert_eq!(
            script,
            "do shell script \"mkdir -p /usr/local/bin && ln -sf '/Applications/TAIDE.app/Contents/MacOS/taide-cli' '/usr/local/bin/taide'\" with administrator privileges"
        );
    }

    #[test]
    fn 제거_스크립트는_rm을_관리자_권한으로_실행한다() {
        let script = build_cli_uninstall_apple_script(Path::new("/usr/local/bin/taide"));
        assert_eq!(
            script,
            "do shell script \"rm '/usr/local/bin/taide'\" with administrator privileges"
        );
    }

    #[test]
    fn osascript_인자는_e_플래그와_스크립트_두_요소뿐이다() {
        let args = build_osascript_args("do shell script \"echo hi\"");
        assert_eq!(args, vec!["-e".to_string(), "do shell script \"echo hi\"".to_string()]);
    }

    #[test]
    fn 사용자_취소는_exit_1과_128_마커로_판정한다() {
        assert!(is_osascript_user_cancelled(Some(1), "execution error: User canceled. (-128)"));
        assert!(!is_osascript_user_cancelled(Some(0), "execution error: User canceled. (-128)"));
        assert!(!is_osascript_user_cancelled(Some(1), "execution error: something else"));
        assert!(!is_osascript_user_cancelled(None, "execution error: User canceled. (-128)"));
    }

    #[test]
    fn 심링크_소유_판정은_파일명이_taide_cli일_때만_참이다() {
        assert!(is_cli_symlink_owned(Path::new("/Applications/TAIDE.app/Contents/MacOS/taide-cli")));
        assert!(!is_cli_symlink_owned(Path::new("/opt/homebrew/bin/some-other-tool")));
    }

    #[test]
    fn 순서만_다르고_내용이_같으면_변경으로_보지_않는다() {
        let a = DetectedAgent {
            session_id: "term-1".to_string(),
            name: "claude".to_string(),
            pid: 1,
            activity: AgentActivity::Unknown,
        };
        let b = DetectedAgent {
            session_id: "term-2".to_string(),
            name: "codex".to_string(),
            pid: 2,
            activity: AgentActivity::Unknown,
        };
        assert!(!agents_changed(&[a.clone(), b.clone()], &[b, a]));
    }

    #[test]
    fn 내용이_다르면_변경으로_본다() {
        let a = DetectedAgent {
            session_id: "term-1".to_string(),
            name: "claude".to_string(),
            pid: 1,
            activity: AgentActivity::Unknown,
        };
        assert!(agents_changed(&[], &[a]));
    }

    #[test]
    fn wait_마커가_포함된_argv를_파싱한다() {
        let argv = vec![
            "/usr/local/bin/taide".to_string(),
            "/tmp/edit.md".to_string(),
            "--wait-marker".to_string(),
            "/tmp/taide-wait-abc123".to_string(),
        ];
        let request = parse_cli_payload(&argv).unwrap();
        assert_eq!(request.path, "/tmp/edit.md");
        assert_eq!(request.wait_marker.as_deref(), Some("/tmp/taide-wait-abc123"));
    }

    #[test]
    fn wait_마커_없이_argv를_파싱한다() {
        let argv = vec!["/usr/local/bin/taide".to_string(), "/tmp/edit.md".to_string()];
        let request = parse_cli_payload(&argv).unwrap();
        assert_eq!(request.path, "/tmp/edit.md");
        assert!(request.wait_marker.is_none());
    }

    #[test]
    fn 파일이_없으면_none을_반환한다() {
        let argv = vec!["/usr/local/bin/taide".to_string()];
        assert!(parse_cli_payload(&argv).is_none());
    }

    #[test]
    fn ps_배치_출력은_pid별로_분해된다() {
        let stdout = "    1 /sbin/launchd  /sbin/launchd\n 4242 /usr/bin/node  /usr/bin/node /opt/bin/claude --resume\n";

        let infos = parse_ps_process_infos(stdout);

        assert_eq!(infos.len(), 2);
        let agent = infos.get(&4242).unwrap();
        assert_eq!(agent.comm, "/usr/bin/node");
        assert_eq!(agent.cmdline, "/usr/bin/node /opt/bin/claude --resume");
        assert_eq!(detect_agent_name(&agent.comm, &agent.cmdline), Some(AGENT_NAME_CLAUDE));
    }

    #[test]
    fn 해석할_수_없는_행이_섞여도_나머지_pid는_남는다() {
        let infos = parse_ps_process_infos("ps: process id too large: 999999\n 4242 /bin/zsh  /bin/zsh -l\n");

        assert_eq!(infos.len(), 1, "행 하나가 깨져도 배치 전체를 버리면 안 된다");
        assert_eq!(infos.get(&4242).unwrap().comm, "/bin/zsh");
    }

    #[test]
    fn 이미_종료된_pid는_결과에_없다() {
        let infos = parse_ps_process_infos(" 4242 /bin/zsh  /bin/zsh -l\n");

        assert!(
            !infos.contains_key(&4243),
            "ps 는 죽은 pid 를 아예 출력하지 않으므로 맵 조회로 가려낸다"
        );
    }

    fn snapshot(pid: u32, parent_pid: Option<u32>, name: &str, cmdline: &str) -> ProcessSnapshot {
        ProcessSnapshot {
            pid,
            parent_pid,
            name: name.to_string(),
            cmdline: cmdline.to_string(),
        }
    }

    #[test]
    fn 셸의_직계_자식이_에이전트면_감지한다() {
        let processes = vec![snapshot(200, Some(100), "claude", "claude")];
        assert_eq!(find_descendant_agent(&processes, 100), Some((200, "claude")));
    }

    #[test]
    fn 손자_프로세스까지_내려가서_감지한다() {
        let processes = vec![
            snapshot(200, Some(100), "node", "node"),
            snapshot(300, Some(200), "node", "C:\\node.exe C:\\Users\\dev\\.claude\\local\\claude"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), Some((300, "claude")));
    }

    #[test]
    fn 에이전트가_없으면_none을_반환한다() {
        let processes = vec![
            snapshot(200, Some(100), "powershell", "powershell"),
            snapshot(300, Some(200), "node", "node server.js"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), None);
    }

    #[test]
    fn 다른_트리에_속한_프로세스는_무시한다() {
        let processes = vec![
            snapshot(200, Some(999), "claude", "claude"),
            snapshot(300, Some(100), "powershell", "powershell"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), None);
    }

    #[test]
    fn 순환_참조가_있어도_무한루프에_빠지지_않는다() {
        let processes = vec![
            snapshot(200, Some(100), "powershell", "powershell"),
            snapshot(100, Some(200), "powershell", "powershell"),
        ];
        assert_eq!(find_descendant_agent(&processes, 100), None);
    }

    #[test]
    fn windows_exe_확장자를_제거한다() {
        assert_eq!(strip_windows_exe_suffix("claude.exe"), "claude");
        assert_eq!(strip_windows_exe_suffix("claude.EXE"), "claude");
        assert_eq!(strip_windows_exe_suffix("claude"), "claude");
        assert_eq!(strip_windows_exe_suffix(".exe"), ".exe");
    }

    /// The permission dialog Claude Code 2.1.263 actually wrote to a TAIDE pty, already normalized
    /// by the scanner (its raw form — prose split by `CSI n G` column moves — is pinned in
    /// `infra::terminal_scan`'s own tests).
    const DIALOG_TEXT: &str = "Bash command touch /tmp/taide-probe-file Create empty probe file \
Do you want to proceed? ❯ 1. Yes 2. No Esc to cancel · Tab to amend";

    /// One `⏺` blink frame, which is all that keeps flowing while that dialog waits.
    const BLINK_TEXT: &str = " ⏺ ";

    fn at(base: Instant, offset_ms: u64) -> Instant {
        base + std::time::Duration::from_millis(offset_ms)
    }

    fn claude_signals() -> AgentSessionSignals {
        AgentSessionSignals::new(AGENT_NAME_CLAUDE)
    }

    fn output(text: &str) -> ScanOutcome {
        ScanOutcome {
            events: Vec::new(),
            text: text.to_string(),
            overlap: String::new(),
        }
    }

    fn agent_event_outcome(event: AgentEvent) -> ScanOutcome {
        ScanOutcome {
            events: vec![ScanEvent::AgentEvent(claude_agent_event_payload(event))],
            text: String::new(),
            overlap: String::new(),
        }
    }

    fn title_outcome(title: &str) -> ScanOutcome {
        ScanOutcome {
            events: vec![ScanEvent::Title(title.to_string())],
            text: String::new(),
            overlap: String::new(),
        }
    }

    #[test]
    fn 스피너와_함께_실질_출력이_흐르면_working_이다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(&mut signals, &title_outcome("◐ taide-probe-file 생성"), AGENT_NAME_CLAUDE, base);
        apply_scan_to_signals(
            &mut signals,
            &output("✻ Puttering… (3s · esc to interrupt)"),
            AGENT_NAME_CLAUDE,
            base,
        );

        assert_eq!(
            classify_session(&signals, AgentActivity::Idle, at(base, 500)),
            AgentActivity::Working
        );
    }

    #[test]
    fn 다이얼로그_시그니처는_awaiting_input_래치를_건다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(&mut signals, &output(DIALOG_TEXT), AGENT_NAME_CLAUDE, base);

        assert_eq!(signals.blocked, Some(BlockedSource::Dialog));
        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, 100)),
            AgentActivity::AwaitingInput
        );
    }

    #[test]
    fn 점멸_청크만_흐르는_동안_래치가_유지된다() {
        let base = Instant::now();
        let mut signals = claude_signals();
        apply_scan_to_signals(&mut signals, &output(DIALOG_TEXT), AGENT_NAME_CLAUDE, base);

        for tick in 1..=10 {
            apply_scan_to_signals(&mut signals, &output(BLINK_TEXT), AGENT_NAME_CLAUDE, at(base, tick * 600));
        }

        assert_eq!(signals.blocked, Some(BlockedSource::Dialog), "점멸은 실질 출력이 아니다");
        assert_eq!(
            classify_session(&signals, AgentActivity::AwaitingInput, at(base, 6_000)),
            AgentActivity::AwaitingInput,
            "6초가 지나도 유휴로 떨어지면 안 된다"
        );
    }

    #[test]
    fn 실질_출력이_재개되면_래치가_풀리고_working_이_된다() {
        let base = Instant::now();
        let mut signals = claude_signals();
        apply_scan_to_signals(&mut signals, &output(DIALOG_TEXT), AGENT_NAME_CLAUDE, base);

        apply_scan_to_signals(
            &mut signals,
            &output("Running touch /tmp/taide-probe-file"),
            AGENT_NAME_CLAUDE,
            at(base, 5_000),
        );

        assert_eq!(signals.blocked, None);
        assert_eq!(
            classify_session(&signals, AgentActivity::AwaitingInput, at(base, 5_100)),
            AgentActivity::Working
        );
    }

    #[test]
    fn 사용자_입력은_래치를_해제한다() {
        let base = Instant::now();
        let mut signals = claude_signals();
        apply_scan_to_signals(&mut signals, &output(DIALOG_TEXT), AGENT_NAME_CLAUDE, base);

        note_input(&mut signals, at(base, 1_000));

        assert_eq!(signals.blocked, None);
        assert_ne!(
            classify_session(&signals, AgentActivity::AwaitingInput, at(base, 1_100)),
            AgentActivity::AwaitingInput
        );
    }

    #[test]
    fn 입력_직후의_출력은_에코로_보고_실질_출력에_세지_않는다() {
        let base = Instant::now();
        let mut signals = claude_signals();
        note_input(&mut signals, base);

        apply_scan_to_signals(
            &mut signals,
            &output("git status"),
            AGENT_NAME_CLAUDE,
            at(base, ECHO_SUPPRESS_MS - 1),
        );

        assert_eq!(
            signals.last_substantive_output_at, None,
            "사용자가 친 글자의 되그림은 에이전트의 출력이 아니다"
        );

        apply_scan_to_signals(&mut signals, &output("git status"), AGENT_NAME_CLAUDE, at(base, ECHO_SUPPRESS_MS));
        assert!(signals.last_substantive_output_at.is_some(), "에코 창을 벗어난 출력은 읽는다");
    }

    #[test]
    fn 답변_직후_에코_창_안에_그려진_다이얼로그도_래치한다() {
        let base = Instant::now();
        let mut signals = claude_signals();
        apply_scan_to_signals(&mut signals, &output(DIALOG_TEXT), AGENT_NAME_CLAUDE, base);

        note_input(&mut signals, at(base, 1_000));
        apply_scan_to_signals(
            &mut signals,
            &output(DIALOG_TEXT),
            AGENT_NAME_CLAUDE,
            at(base, 1_000 + ECHO_SUPPRESS_MS - 1),
        );

        assert_eq!(
            signals.blocked,
            Some(BlockedSource::Dialog),
            "연쇄 승인에서 다음 다이얼로그는 답변 직후에 그려지고, 이후에는 점멸만 흐르므로 여기서 놓치면 영영 놓친다"
        );

        for tick in 0..10 {
            apply_scan_to_signals(&mut signals, &output(BLINK_TEXT), AGENT_NAME_CLAUDE, at(base, 2_000 + tick * 600));
        }
        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, 9_000)),
            AgentActivity::AwaitingInput
        );
    }

    #[test]
    fn 같은_청크의_permission_request_는_실질_출력에_지워지지_않는다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(
            &mut signals,
            &ScanOutcome {
                events: vec![ScanEvent::AgentEvent(claude_agent_event_payload(AgentEvent::PermissionRequest))],
                text: "Bash command touch /tmp/taide-probe-file".to_string(),
                overlap: String::new(),
            },
            AGENT_NAME_CLAUDE,
            base,
        );

        assert_eq!(
            signals.blocked,
            Some(BlockedSource::Event),
            "훅은 다이얼로그가 그려지는 순간 발화하므로 프레임의 앞부분과 같은 청크에 실린다"
        );
        assert!(signals.last_substantive_output_at.is_some(), "출력이 있었다는 사실 자체는 기록한다");
    }

    #[test]
    fn 같은_청크의_tool_complete_뒤_다이얼로그는_다시_래치한다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(
            &mut signals,
            &ScanOutcome {
                events: vec![ScanEvent::AgentEvent(claude_agent_event_payload(AgentEvent::ToolComplete))],
                text: DIALOG_TEXT.to_string(),
                overlap: String::new(),
            },
            AGENT_NAME_CLAUDE,
            base,
        );

        assert_eq!(signals.blocked, Some(BlockedSource::Dialog));
    }

    #[test]
    fn 타이틀_유휴_글리프_뒤_짧은_정적이면_idle_이다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(&mut signals, &title_outcome("✳ taide-probe-file 생성"), AGENT_NAME_CLAUDE, base);

        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, ACTIVITY_WORKING_HOLD_MS - 1)),
            AgentActivity::Working,
            "힌트만으로 즉시 유휴로 떨어뜨리지 않는다"
        );
        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, ACTIVITY_WORKING_HOLD_MS)),
            AgentActivity::Idle
        );
    }

    #[test]
    fn 타이틀이_유휴_글리프에서_멈춰도_출력이_흐르면_working_이다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(&mut signals, &title_outcome("✳ Claude Code"), AGENT_NAME_CLAUDE, base);
        apply_scan_to_signals(&mut signals, &output("compiling taide v0.1.8"), AGENT_NAME_CLAUDE, at(base, 3_000));

        assert_eq!(
            classify_session(&signals, AgentActivity::Idle, at(base, 3_500)),
            AgentActivity::Working,
            "tmux 아래의 정적 ✳ 타이틀이 출력 흐름을 이기면 안 된다"
        );
    }

    #[test]
    fn 힌트가_없어도_충분히_조용하면_idle_이다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(&mut signals, &output("done"), AGENT_NAME_CLAUDE, base);

        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, ACTIVITY_IDLE_QUIET_MS - 1)),
            AgentActivity::Working,
            "히스테리시스 구간에서는 직전 상태를 유지한다"
        );
        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, ACTIVITY_IDLE_QUIET_MS)),
            AgentActivity::Idle
        );
    }

    #[test]
    fn permission_request_이벤트는_래치를_건다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        apply_scan_to_signals(
            &mut signals,
            &agent_event_outcome(AgentEvent::PermissionRequest),
            AGENT_NAME_CLAUDE,
            base,
        );

        assert_eq!(signals.blocked, Some(BlockedSource::Event));
        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, 10_000)),
            AgentActivity::AwaitingInput
        );
    }

    #[test]
    fn tool_complete_와_stop_이벤트는_래치를_해제한다() {
        let base = Instant::now();

        let mut working = claude_signals();
        apply_scan_to_signals(
            &mut working,
            &agent_event_outcome(AgentEvent::PermissionRequest),
            AGENT_NAME_CLAUDE,
            base,
        );
        apply_scan_to_signals(
            &mut working,
            &agent_event_outcome(AgentEvent::ToolComplete),
            AGENT_NAME_CLAUDE,
            at(base, 1_000),
        );
        assert_eq!(working.blocked, None);
        assert_eq!(
            classify_session(&working, AgentActivity::AwaitingInput, at(base, 1_500)),
            AgentActivity::Working
        );

        let mut stopped = claude_signals();
        apply_scan_to_signals(
            &mut stopped,
            &agent_event_outcome(AgentEvent::PermissionRequest),
            AGENT_NAME_CLAUDE,
            base,
        );
        apply_scan_to_signals(
            &mut stopped,
            &agent_event_outcome(AgentEvent::Stop),
            AGENT_NAME_CLAUDE,
            at(base, 1_000),
        );
        assert_eq!(stopped.blocked, None);
        assert_eq!(
            classify_session(&stopped, AgentActivity::AwaitingInput, at(base, 1_000 + ACTIVITY_WORKING_HOLD_MS)),
            AgentActivity::Idle
        );
    }

    #[test]
    fn idle_prompt_이벤트는_상태를_바꾸지_않는다() {
        let base = Instant::now();
        let mut signals = claude_signals();
        apply_scan_to_signals(
            &mut signals,
            &agent_event_outcome(AgentEvent::PermissionRequest),
            AGENT_NAME_CLAUDE,
            base,
        );

        apply_scan_to_signals(
            &mut signals,
            &agent_event_outcome(AgentEvent::IdlePrompt),
            AGENT_NAME_CLAUDE,
            at(base, 500),
        );

        assert_eq!(signals.blocked, Some(BlockedSource::Event));
        assert_eq!(signals.last_event.map(|(event, _)| event), Some(AgentEvent::PermissionRequest));
    }

    #[test]
    fn 다른_에이전트가_보낸_이벤트는_적용하지_않는다() {
        let base = Instant::now();
        let mut signals = AgentSessionSignals::new(AGENT_NAME_CODEX);

        apply_scan_to_signals(
            &mut signals,
            &agent_event_outcome(AgentEvent::PermissionRequest),
            AGENT_NAME_CODEX,
            base,
        );

        assert_eq!(signals.blocked, None, "본문의 agent 가 세션의 에이전트와 다르면 무시한다");
    }

    #[test]
    fn 작업_중_푸터는_다이얼로그_시그니처가_아니다() {
        assert!(!find_dialog_signature(
            AGENT_NAME_CLAUDE,
            "✻ Puttering… (12s · 41 tokens · esc to interrupt)"
        ));
        assert!(find_dialog_signature(AGENT_NAME_CLAUDE, DIALOG_TEXT));
    }

    #[test]
    fn 시그니처_표가_없는_에이전트는_다이얼로그로_판정하지_않는다() {
        assert!(!find_dialog_signature(AGENT_NAME_CODEX, DIALOG_TEXT));
    }

    #[test]
    fn 청크에_걸쳐_잘린_다이얼로그_문구도_래치를_건다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        let split = ScanOutcome {
            events: Vec::new(),
            text: " proceed? ❯ 1. Yes".to_string(),
            overlap: "Create empty probe file Do you want to".to_string(),
        };
        apply_scan_to_signals(&mut signals, &split, AGENT_NAME_CLAUDE, base);

        assert_eq!(signals.blocked, Some(BlockedSource::Dialog));
    }

    #[test]
    fn 이미_지나간_다이얼로그_꼬리는_래치를_다시_걸지_않는다() {
        let base = Instant::now();
        let mut signals = claude_signals();

        let after_answer = ScanOutcome {
            events: Vec::new(),
            text: BLINK_TEXT.to_string(),
            overlap: DIALOG_TEXT.to_string(),
        };
        apply_scan_to_signals(&mut signals, &after_answer, AGENT_NAME_CLAUDE, base);

        assert_eq!(signals.blocked, None, "꼬리에 통째로 남아 있는 지난 문구는 새 다이얼로그가 아니다");
    }

    #[test]
    fn 모든_다이얼로그_시그니처는_텍스트_꼬리보다_짧다() {
        for agent_name in KNOWN_AGENT_NAMES {
            for signature in dialog_signatures_for(agent_name) {
                assert!(
                    signature.len() < TEXT_OVERLAP_BYTES,
                    "꼬리보다 긴 시그니처는 청크 경계에서 복원될 수 없다: {signature}"
                );
            }
        }
    }

    /// The dialog line and the `⏺` blink frame exactly as Claude Code 2.1.263 wrote them to a TAIDE
    /// pty, so this covers the scanner and the classifier together rather than a hand-normalized
    /// approximation of the scanner's output.
    #[test]
    fn 실물_pty_바이트로_래치가_걸리고_점멸_동안_유지된다() {
        const RAW_DIALOG_QUESTION: &[u8] = b"\r\x1b[1C\x1b[2B\x1b[39mDo\x1b[5Gyou\x1b[9Gwant\x1b[14Gto\x1b[17Gproceed?";
        const RAW_BLINK_FRAME: &[u8] = b"\x1b[H\r\x1b[10B\x1b[38;2;153;153;153m\xe2\x8f\xba\x1b[39m\x1b[40;1H\x1b[22;2H";

        let base = Instant::now();
        let mut scanner = crate::infra::terminal_scan::OutputScanner::new();
        let mut signals = claude_signals();

        apply_scan_to_signals(&mut signals, &scanner.scan(RAW_DIALOG_QUESTION), AGENT_NAME_CLAUDE, base);
        assert_eq!(signals.blocked, Some(BlockedSource::Dialog));

        for tick in 1..=5 {
            apply_scan_to_signals(
                &mut signals,
                &scanner.scan(RAW_BLINK_FRAME),
                AGENT_NAME_CLAUDE,
                at(base, tick * 600),
            );
        }

        assert_eq!(
            classify_session(&signals, AgentActivity::Working, at(base, 6_000)),
            AgentActivity::AwaitingInput
        );
    }

    #[test]
    fn 신호가_없으면_unknown_이다() {
        assert_eq!(
            classify_session(&claude_signals(), AgentActivity::Working, Instant::now()),
            AgentActivity::Unknown
        );
    }

    #[test]
    fn 실질_출력_판정은_공백과_점멸_글리프를_세지_않는다() {
        assert!(!is_substantive_output(BLINK_TEXT));
        assert!(!is_substantive_output("   \n  "));
        assert!(!is_substantive_output("⏺ ✻ ◐ ·"));
        assert!(!is_substantive_output("a"));
        assert!(is_substantive_output("ok"));
    }

    #[test]
    fn 타이틀_글리프를_판정한다() {
        assert_eq!(parse_title_glyph("◐ 작업 중"), Some(TitleGlyph::Working));
        assert_eq!(parse_title_glyph("◑ 작업 중"), Some(TitleGlyph::Working));
        assert_eq!(parse_title_glyph("✳ Claude Code"), Some(TitleGlyph::Idle));
        assert_eq!(parse_title_glyph("zsh"), None);
        assert_eq!(parse_title_glyph(""), None);
    }

    #[test]
    fn 에이전트_이벤트_본문을_해석한다() {
        assert_eq!(
            parse_agent_event_body("{\"v\":1,\"agent\":\"claude\",\"event\":\"permission_request\"}"),
            Some(("claude".to_string(), AgentEvent::PermissionRequest))
        );
        assert_eq!(
            parse_agent_event_body("{\"v\":2,\"agent\":\"claude\",\"event\":\"stop\"}"),
            None,
            "다른 프로토콜 버전은 읽지 않는다"
        );
        assert_eq!(parse_agent_event_body("{\"v\":1,\"agent\":\"claude\",\"event\":\"launch\"}"), None);
        assert_eq!(parse_agent_event_body("not json"), None);
    }

    #[test]
    fn 상수시간_비교는_같은_바이트열에서_참이다() {
        assert!(constant_time_eq(b"token-value", b"token-value"));
    }

    #[test]
    fn 상수시간_비교는_다른_바이트열에서_거짓이다() {
        assert!(!constant_time_eq(b"token-value", b"token-other"));
        assert!(!constant_time_eq(b"short", b"longer-token"));
    }

    #[test]
    fn cwd가_프로젝트_루트와_정확히_같으면_매칭한다() {
        let projects = vec![(ProjectId::from("prj-1".to_string()), "/Users/dev/app".to_string())];
        assert_eq!(match_project_by_cwd("/Users/dev/app", &projects), Some(&projects[0].0));
    }

    #[test]
    fn cwd가_프로젝트_루트의_하위_경로면_매칭한다() {
        let projects = vec![(ProjectId::from("prj-1".to_string()), "/Users/dev/app".to_string())];
        assert_eq!(match_project_by_cwd("/Users/dev/app/src", &projects), Some(&projects[0].0));
    }

    #[test]
    fn 중첩된_프로젝트에서는_가장_긴_루트가_이긴다() {
        let projects = vec![
            (ProjectId::from("outer".to_string()), "/Users/dev".to_string()),
            (ProjectId::from("inner".to_string()), "/Users/dev/app".to_string()),
        ];
        assert_eq!(match_project_by_cwd("/Users/dev/app/src", &projects), Some(&projects[1].0));
    }

    #[test]
    fn 어느_루트에도_속하지_않으면_none이다() {
        let projects = vec![(ProjectId::from("prj-1".to_string()), "/Users/dev/app".to_string())];
        assert_eq!(match_project_by_cwd("/tmp/scratch", &projects), None);
    }

    #[test]
    fn claude_hook_이벤트를_활동으로_매핑한다() {
        assert_eq!(
            map_hook_event_to_activity("claude", "UserPromptSubmit"),
            Some(AgentActivity::Working)
        );
        assert_eq!(
            map_hook_event_to_activity("claude", "Notification"),
            Some(AgentActivity::AwaitingInput)
        );
        assert_eq!(map_hook_event_to_activity("claude", "Stop"), Some(AgentActivity::Idle));
        assert_eq!(map_hook_event_to_activity("claude", "PreToolUse"), None);
    }

    #[test]
    fn codex_hook_이벤트를_활동으로_매핑한다() {
        assert_eq!(
            map_hook_event_to_activity("codex", "UserPromptSubmit"),
            Some(AgentActivity::Working)
        );
        assert_eq!(
            map_hook_event_to_activity("codex", "PermissionRequest"),
            Some(AgentActivity::AwaitingInput)
        );
        assert_eq!(map_hook_event_to_activity("codex", "PostToolUse"), Some(AgentActivity::Working));
        assert_eq!(map_hook_event_to_activity("codex", "Stop"), Some(AgentActivity::Idle));
        assert_eq!(map_hook_event_to_activity("codex", "Notification"), None);
    }

    #[test]
    fn gemini_hook_이벤트를_활동으로_매핑한다() {
        assert_eq!(map_hook_event_to_activity("gemini", "BeforeAgent"), Some(AgentActivity::Working));
        assert_eq!(
            map_hook_event_to_activity("gemini", "Notification"),
            Some(AgentActivity::AwaitingInput)
        );
        assert_eq!(map_hook_event_to_activity("gemini", "AfterAgent"), Some(AgentActivity::Idle));
    }

    #[test]
    fn 알려지지_않은_에이전트는_hook_이벤트를_매핑하지_않는다() {
        assert_eq!(map_hook_event_to_activity("unknown", "Stop"), None);
    }

    /// A `settings.local.json` written by the retired HTTP install, which reconcile has to
    /// recognize as TAIDE's own and replace.
    const LEGACY_HOOK_URL: &str = "http://127.0.0.1:9999/claude/hook?token=abc&taide=1";

    fn legacy_http_settings() -> serde_json::Value {
        let handler = serde_json::json!({ "type": "http", "url": LEGACY_HOOK_URL, "timeout": 5 });
        serde_json::json!({
            "hooks": {
                "UserPromptSubmit": [{ "hooks": [handler.clone()] }],
                "Notification": [{ "hooks": [handler.clone()] }],
                "Stop": [{ "hooks": [handler] }],
            }
        })
    }

    #[test]
    fn 빈_설정에_claude_hook을_주입하면_이벤트별_항목이_생긴다() {
        let injected = inject_taide_claude_command_hook_entries(serde_json::json!({}), HookEmitter::TerminalSequence);

        assert!(has_taide_hook_entries(&injected));
        assert!(claude_hook_entries_match(&injected, HookEmitter::TerminalSequence));
        assert_eq!(injected["hooks"]["Notification"].as_array().unwrap().len(), 3);
        assert_eq!(injected["hooks"][HOOK_EVENT_STOP_FAILURE].as_array().unwrap().len(), 1);
        assert!(
            injected["hooks"].get(HOOK_EVENT_USER_PROMPT_SUBMIT).is_none(),
            "UserPromptSubmit 은 더 이상 설치하지 않는다"
        );
        assert_eq!(
            injected["hooks"]["Notification"][0]["matcher"],
            NOTIFICATION_MATCHER_PERMISSION_PROMPT
        );
    }

    #[test]
    fn 기존_사용자_hook은_보존하고_taide_항목만_주입한다() {
        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        let injected = inject_taide_claude_command_hook_entries(existing, HookEmitter::TerminalSequence);
        let stop_entries = injected["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 2);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo done");
    }

    #[test]
    fn 두_번_주입해도_taide_항목이_중복되지_않는다() {
        let once = inject_taide_claude_command_hook_entries(serde_json::json!({}), HookEmitter::TerminalSequence);
        let twice = inject_taide_claude_command_hook_entries(once.clone(), HookEmitter::TerminalSequence);
        assert_eq!(twice, once);

        let switched = inject_taide_claude_command_hook_entries(twice, HookEmitter::DevTty);
        assert!(claude_hook_entries_match(&switched, HookEmitter::DevTty));
        assert!(
            !claude_hook_entries_match(&switched, HookEmitter::TerminalSequence),
            "방출 방식이 바뀌면 이전 명령은 남지 않는다"
        );
    }

    #[test]
    fn hooks_값이_객체가_아니어도_패닉없이_주입한다() {
        let broken = serde_json::json!({ "hooks": "disabled" });
        let injected = inject_taide_claude_command_hook_entries(broken, HookEmitter::DevTty);
        assert!(claude_hook_entries_match(&injected, HookEmitter::DevTty));
    }

    #[test]
    fn hook_이벤트_값이_배열이_아니어도_패닉없이_주입한다() {
        let broken = serde_json::json!({ "hooks": { "Stop": {}, "Notification": 3 } });
        let injected = inject_taide_claude_command_hook_entries(broken, HookEmitter::DevTty);
        assert!(claude_hook_entries_match(&injected, HookEmitter::DevTty));
    }

    #[test]
    fn 구버전_http_설치는_taide_항목으로_인식되고_기대_집합과_어긋난다() {
        let legacy = legacy_http_settings();
        assert!(has_taide_hook_entries(&legacy), "구버전 설치도 TAIDE 항목으로 인식해야 제거된다");
        assert!(!claude_hook_entries_match(&legacy, HookEmitter::TerminalSequence));

        let reinstalled = inject_taide_claude_command_hook_entries(legacy, HookEmitter::TerminalSequence);
        assert!(claude_hook_entries_match(&reinstalled, HookEmitter::TerminalSequence));
        assert!(reinstalled["hooks"].get(HOOK_EVENT_USER_PROMPT_SUBMIT).is_none());
        assert!(!has_taide_marker_anywhere(&serde_json::json!({ "hooks": { "Stop": [] } })));
    }

    #[test]
    fn 기록된_command_hook이_현재_커맨드와_다르면_재주입_대상이다() {
        let injected = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        assert!(has_command_hook_entries_for_command(
            &injected,
            CODEX_MANAGED_HOOK_EVENTS,
            &codex_command()
        ));
        let stale_command = build_command_hook_shell_command(
            "/usr/local/bin/taide",
            "http://127.0.0.1:10000/claude/hook?token=xyz&agent=codex&taide=1",
        );
        assert!(!has_command_hook_entries_for_command(
            &injected,
            CODEX_MANAGED_HOOK_EVENTS,
            &stale_command
        ));
    }

    #[test]
    fn hook_override는_알려진_에이전트_3종에만_적용된다() {
        assert!(is_hook_managed_agent("claude"));
        assert!(is_hook_managed_agent("codex"));
        assert!(is_hook_managed_agent("gemini"));
        assert!(!is_hook_managed_agent("bash"));
    }

    #[test]
    fn claude는_프로젝트_스코프이고_codex_gemini는_사용자_스코프다() {
        assert_eq!(hook_scope_for_agent("claude").unwrap(), HookInstallScope::Project);
        assert_eq!(hook_scope_for_agent("codex").unwrap(), HookInstallScope::User);
        assert_eq!(hook_scope_for_agent("gemini").unwrap(), HookInstallScope::User);
        assert!(hook_scope_for_agent("bash").is_err());
    }

    #[test]
    fn 사용자_레벨_hooks_경로를_홈_디렉토리_기준으로_구성한다() {
        assert_eq!(
            user_level_hooks_path("codex", Some("/Users/dev")).unwrap(),
            PathBuf::from("/Users/dev/.codex/hooks.json")
        );
        assert_eq!(
            user_level_hooks_path("gemini", Some("/Users/dev")).unwrap(),
            PathBuf::from("/Users/dev/.gemini/settings.json")
        );
        assert!(user_level_hooks_path("claude", Some("/Users/dev")).is_err());
        assert!(user_level_hooks_path("codex", None).is_err());
    }

    #[test]
    fn 문서_어디에_있어도_taide_마커를_찾는다() {
        let nested = serde_json::json!({
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "taide-cli hook --url http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1" }] }] }
        });
        assert!(has_taide_marker_anywhere(&nested));
        assert!(!has_taide_marker_anywhere(&serde_json::json!({ "hooks": {} })));
    }

    #[test]
    fn taide_hook을_제거하면_사용자_hook만_남는다() {
        let existing = serde_json::json!({
            "hooks": {
                "Stop": [
                    { "hooks": [{ "type": "command", "command": "echo done" }] },
                    { "hooks": [{ "type": "http", "url": "http://127.0.0.1:9999/claude/hook?token=abc&taide=1" }] }
                ]
            }
        });
        let removed = remove_taide_hook_entries(existing);
        let stop_entries = removed["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 1);
        assert!(!has_taide_hook_entries(&removed));
    }

    #[test]
    fn 센티널을_문자열로만_담은_사용자_hook은_taide_소유가_아니다() {
        let user_owned = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "~/dev/taide-agent-notes/notify.sh" }] }]
            }
        });

        assert!(!has_taide_marker_anywhere(&user_owned));
        assert!(!has_taide_hook_entries(&user_owned));
        assert_eq!(
            remove_taide_hook_entries(user_owned.clone()),
            user_owned,
            "소유 판정은 OSC 페이로드 접두사여야 한다 — 단어만 겹친 사용자 항목을 지우면 안 된다"
        );
    }

    #[test]
    fn taide_hook만_있었다면_제거_후_hooks_키_자체가_사라진다() {
        let injected = inject_taide_claude_command_hook_entries(serde_json::json!({}), HookEmitter::TerminalSequence);
        let removed = remove_taide_hook_entries(injected);
        assert!(removed.get("hooks").is_none());

        let legacy_removed = remove_taide_hook_entries(legacy_http_settings());
        assert!(legacy_removed.get("hooks").is_none(), "구버전 http 항목도 남기지 않는다");
    }

    #[test]
    fn 에이전트별_관리대상_이벤트_목록이_다르다() {
        assert_eq!(managed_hook_events_for("claude"), CLAUDE_MANAGED_HOOK_EVENTS);
        assert_eq!(managed_hook_events_for("codex"), CODEX_MANAGED_HOOK_EVENTS);
        assert_eq!(managed_hook_events_for("gemini"), GEMINI_MANAGED_HOOK_EVENTS);
        assert!(managed_hook_events_for("bash").is_empty());
    }

    #[test]
    fn 에이전트별_command_hook_timeout_단위가_다르다() {
        assert_eq!(user_level_hook_command_timeout("codex"), CODEX_HOOK_COMMAND_TIMEOUT_SECONDS);
        assert_eq!(user_level_hook_command_timeout("gemini"), GEMINI_HOOK_COMMAND_TIMEOUT_MS);
        assert_eq!(
            user_level_hook_command_timeout("gemini"),
            CODEX_HOOK_COMMAND_TIMEOUT_SECONDS * 1_000
        );
    }

    #[test]
    fn 쉘_커맨드는_바이너리_경로와_url을_각각_큰따옴표로_감싼다() {
        let command = build_command_hook_shell_command(
            "/usr/local/bin/taide",
            "http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1",
        );
        assert_eq!(
            command,
            "\"/usr/local/bin/taide\" hook --url \"http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1\""
        );
    }

    fn codex_hook_url() -> &'static str {
        "http://127.0.0.1:9999/claude/hook?token=abc&agent=codex&taide=1"
    }

    fn codex_command() -> String {
        build_command_hook_shell_command("/usr/local/bin/taide", codex_hook_url())
    }

    #[test]
    fn codex_빈_설정에_command_hook을_주입하면_관리대상_이벤트_4종이_생긴다() {
        let injected = inject_taide_command_hook_entries(
            serde_json::json!({}),
            CODEX_MANAGED_HOOK_EVENTS,
            &codex_command(),
            CODEX_HOOK_COMMAND_TIMEOUT_SECONDS,
        );
        for event in CODEX_MANAGED_HOOK_EVENTS {
            let entries = injected["hooks"][event].as_array().expect("event entries");
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["hooks"][0]["type"], HOOK_HANDLER_TYPE_COMMAND);
            assert_eq!(entries[0]["hooks"][0]["command"], codex_command());
        }
    }

    #[test]
    fn codex_기존_사용자_hook은_보존하고_taide_command_항목만_주입한다() {
        let existing = serde_json::json!({
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "echo done" }] }]
            }
        });
        let injected = inject_taide_command_hook_entries(existing, CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let stop_entries = injected["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 2);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo done");
    }

    #[test]
    fn codex_두_번_주입해도_taide_command_항목이_중복되지_않는다() {
        let once = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let other_command = build_command_hook_shell_command(
            "/usr/local/bin/taide",
            "http://127.0.0.1:10000/claude/hook?token=xyz&agent=codex&taide=1",
        );
        let twice = inject_taide_command_hook_entries(once, CODEX_MANAGED_HOOK_EVENTS, &other_command, 5);
        for event in CODEX_MANAGED_HOOK_EVENTS {
            let entries = twice["hooks"][event].as_array().unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["hooks"][0]["command"], other_command);
        }
    }

    #[test]
    fn codex_taide_command_hook을_제거하면_사용자_hook만_남고_제거_대상이_없으면_hooks_키가_사라진다() {
        let injected = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let removed = remove_taide_command_hook_entries(injected, CODEX_MANAGED_HOOK_EVENTS);
        assert!(removed.get("hooks").is_none());

        let existing = serde_json::json!({
            "hooks": {
                "Stop": [
                    { "hooks": [{ "type": "command", "command": "echo done" }] },
                    { "hooks": [{ "type": "command", "command": codex_command() }] }
                ]
            }
        });
        let injected_with_user_hook = inject_taide_command_hook_entries(existing, CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let removed = remove_taide_command_hook_entries(injected_with_user_hook, CODEX_MANAGED_HOOK_EVENTS);
        let stop_entries = removed["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_entries.len(), 1);
        assert_eq!(stop_entries[0]["hooks"][0]["command"], "echo done");
    }

    fn gemini_hook_url() -> &'static str {
        "http://127.0.0.1:9999/claude/hook?token=abc&agent=gemini&taide=1"
    }

    fn gemini_command() -> String {
        build_command_hook_shell_command("/usr/local/bin/taide", gemini_hook_url())
    }

    #[test]
    fn gemini_빈_설정에_command_hook을_주입하면_관리대상_이벤트_3종이_생긴다() {
        let injected = inject_taide_command_hook_entries(
            serde_json::json!({}),
            GEMINI_MANAGED_HOOK_EVENTS,
            &gemini_command(),
            GEMINI_HOOK_COMMAND_TIMEOUT_MS,
        );
        for event in GEMINI_MANAGED_HOOK_EVENTS {
            let entries = injected["hooks"][event].as_array().expect("event entries");
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0]["hooks"][0]["command"], gemini_command());
            assert_eq!(entries[0]["hooks"][0]["timeout"], GEMINI_HOOK_COMMAND_TIMEOUT_MS);
        }
    }

    #[test]
    fn gemini_두_번_주입해도_taide_command_항목이_중복되지_않고_다른_에이전트_이벤트와_섞이지_않는다() {
        let with_codex = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let with_both = inject_taide_command_hook_entries(with_codex, GEMINI_MANAGED_HOOK_EVENTS, &gemini_command(), 5_000);

        assert_eq!(with_both["hooks"][HOOK_EVENT_STOP].as_array().unwrap().len(), 1);
        assert!(with_both["hooks"][HOOK_EVENT_BEFORE_AGENT].as_array().unwrap()[0]["hooks"][0]["command"] == gemini_command());
        assert!(with_both["hooks"][HOOK_EVENT_PERMISSION_REQUEST].as_array().unwrap()[0]["hooks"][0]["command"] == codex_command());
    }

    #[test]
    fn gemini_taide_command_hook을_제거해도_codex_항목은_그대로다() {
        let with_codex = inject_taide_command_hook_entries(serde_json::json!({}), CODEX_MANAGED_HOOK_EVENTS, &codex_command(), 5);
        let with_both = inject_taide_command_hook_entries(with_codex, GEMINI_MANAGED_HOOK_EVENTS, &gemini_command(), 5_000);

        let gemini_removed = remove_taide_command_hook_entries(with_both, GEMINI_MANAGED_HOOK_EVENTS);
        assert!(gemini_removed["hooks"].get(HOOK_EVENT_BEFORE_AGENT).is_none());
        assert_eq!(
            gemini_removed["hooks"][HOOK_EVENT_STOP].as_array().unwrap()[0]["hooks"][0]["command"],
            codex_command()
        );
    }

    #[test]
    fn 훅_명령은_env_게이트와_정적_payload로_구성된다() {
        let terminal_sequence = build_claude_agent_hook_command(AgentEvent::PermissionRequest, HookEmitter::TerminalSequence);
        assert_eq!(
            terminal_sequence,
            "if [ -n \"$TAIDE_AGENT_PROTOCOL_VERSION\" ]; then printf '%s' \
'{\"terminalSequence\":\"\\u001b]777;notify;taide-agent;{\\\"v\\\":1,\\\"agent\\\":\\\"claude\\\",\\\"event\\\":\\\"permission_request\\\"}\\u0007\"}'; fi; exit 0"
        );

        let dev_tty = build_claude_agent_hook_command(AgentEvent::Stop, HookEmitter::DevTty);
        assert_eq!(
            dev_tty,
            "if [ -n \"$TAIDE_AGENT_PROTOCOL_VERSION\" ]; then \
printf '\\033]777;notify;taide-agent;{\"v\":1,\"agent\":\"claude\",\"event\":\"stop\"}\\007' > /dev/tty 2>/dev/null; fi; exit 0"
        );
    }

    #[test]
    fn 훅_명령은_단일_인용을_깨뜨리는_문자를_담지_않는다() {
        for event in ALL_AGENT_EVENTS {
            let payload = claude_agent_event_payload(event);
            assert!(
                !payload.contains('\''),
                "payload 에 작은따옴표가 있으면 셸 인용이 깨진다: {payload}"
            );
            assert!(!agent_osc_sequence(&payload).contains('\''));

            for emitter in [HookEmitter::TerminalSequence, HookEmitter::DevTty] {
                let command = build_claude_agent_hook_command(event, emitter);
                assert_eq!(command.matches('\'').count() % 2, 0, "인용이 닫히지 않았다: {command}");
            }
        }
    }

    #[test]
    fn claude_훅_항목은_이벤트_matcher_명령_7종이다() {
        let entries = claude_hook_entries(HookEmitter::DevTty);

        assert_eq!(entries.len(), 7);
        let shapes: Vec<_> = entries.iter().map(|(event, matcher, _)| (*event, *matcher)).collect();
        assert_eq!(
            shapes,
            vec![
                (HOOK_EVENT_PERMISSION_REQUEST, None),
                (HOOK_EVENT_NOTIFICATION, Some(NOTIFICATION_MATCHER_PERMISSION_PROMPT)),
                (HOOK_EVENT_NOTIFICATION, Some(NOTIFICATION_MATCHER_ELICITATION_DIALOG)),
                (HOOK_EVENT_NOTIFICATION, Some(NOTIFICATION_MATCHER_IDLE_PROMPT)),
                (HOOK_EVENT_POST_TOOL_USE, None),
                (HOOK_EVENT_STOP, None),
                (HOOK_EVENT_STOP_FAILURE, None),
            ]
        );
        assert!(
            entries.iter().all(|(event, _, _)| *event != HOOK_EVENT_USER_PROMPT_SUBMIT),
            "stdout 이 컨텍스트로 해석되는 훅은 설치하지 않는다"
        );
    }

    /// The emitted sequence has to survive the round trip TAIDE actually performs: Claude parses the
    /// hook's stdout as JSON, writes `terminalSequence` to the pty, and the scanner reads it back.
    #[cfg(unix)]
    #[test]
    fn 훅_명령을_실행하면_스캐너가_읽는_시퀀스가_나온다() {
        let command = build_claude_agent_hook_command(AgentEvent::PermissionRequest, HookEmitter::TerminalSequence);
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .env(AGENT_PROTOCOL_VERSION_ENV_NAME, AGENT_PROTOCOL_VERSION.to_string())
            .output()
            .expect("sh 실행");

        assert!(output.status.success());
        let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).expect("훅 출력은 유효한 JSON 이다");
        let sequence = envelope[HOOK_TERMINAL_SEQUENCE_FIELD].as_str().expect("terminalSequence 필드");

        let scanned = crate::infra::terminal_scan::scan_once(sequence.as_bytes());
        assert_eq!(
            scanned.events,
            vec![ScanEvent::AgentEvent(claude_agent_event_payload(AgentEvent::PermissionRequest))]
        );
        assert_eq!(
            parse_agent_event_body(&claude_agent_event_payload(AgentEvent::PermissionRequest)),
            Some((AGENT_NAME_CLAUDE.to_string(), AgentEvent::PermissionRequest))
        );
    }

    #[cfg(unix)]
    #[test]
    fn taide_밖에서_실행된_훅_명령은_아무것도_내지_않는다() {
        for emitter in [HookEmitter::TerminalSequence, HookEmitter::DevTty] {
            let command = build_claude_agent_hook_command(AgentEvent::Stop, emitter);
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .env_remove(AGENT_PROTOCOL_VERSION_ENV_NAME)
                .output()
                .expect("sh 실행");

            assert!(output.status.success(), "훅은 언제나 0 으로 끝난다");
            assert!(output.stdout.is_empty(), "env 게이트가 닫히면 출력이 없어야 한다");
        }
    }

    #[test]
    fn claude_버전_문자열에서_삼중_버전을_읽는다() {
        assert_eq!(parse_claude_version("2.1.263 (Claude Code)\n"), Some((2, 1, 263)));
        assert_eq!(parse_claude_version("claude 2.1.141-beta.1\n"), Some((2, 1, 141)));
        assert_eq!(parse_claude_version("unknown"), None);
        assert_eq!(parse_claude_version(""), None);
    }

    #[test]
    fn 터미널_시퀀스_지원은_최소_버전부터다() {
        assert!(supports_terminal_sequence(CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION));
        assert!(supports_terminal_sequence((2, 1, 263)));
        assert!(supports_terminal_sequence((3, 0, 0)));
        assert!(!supports_terminal_sequence((2, 1, 140)));
        assert!(!supports_terminal_sequence((2, 0, 999)));
    }

    mod scenario_timeline {
        use super::*;
        use crate::infra::terminal_scan::OutputScanner;

        const RAW_TITLE_START: &[u8] = b"\x1b]0;\xe2\x9c\xb3 Claude Code\x07";
        const RAW_TITLE_WORKING_LEFT: &[u8] = b"\x1b]0;\xe2\x97\x90 taide-probe-file \xec\x83\x9d\xec\x84\xb1\x07";
        const RAW_TITLE_WORKING_RIGHT: &[u8] = b"\x1b]0;\xe2\x97\x91 taide-probe-file \xec\x83\x9d\xec\x84\xb1\x07";
        const RAW_TITLE_IDLE_END: &[u8] = b"\x1b]0;\xe2\x9c\xb3 taide-probe-file \xec\x83\x9d\xec\x84\xb1\x07";
        const RAW_SPINNER_GLYPH_FRAME: &[u8] =
            b"\x1b[?25l\x1b[H\r\x1b[33B\x1b[38;2;215;119;87m\xe2\x9c\xb6\x1b[39m\x1b[40;1H\x1b[37;3H\x1b[?25h";
        const RAW_SPINNER_COUNTER_FRAME: &[u8] = b"\x1b[?25l\x1b[H\r\x1b[16C\x1b[33B\x1b[38;2;153;153;153m(3s \xc2\xb7 thinking with xhigh effort)\x1b[39m\x1b[40;1H\x1b[37;3H\x1b[?25h";
        const RAW_INPUT_ECHO_FRAME: &[u8] =
            b"\x1b[?25l\x1b[H\r\x1b[2C\x1b[34B\x1b[2mls -la /tmp/taide-probe-file\x1b[22m\x1b[40;1H\x1b[35;3H\x1b[?25h";
        const RAW_DIALOG_QUESTION: &[u8] = b"\r\x1b[1C\x1b[2B\x1b[39mDo\x1b[5Gyou\x1b[9Gwant\x1b[14Gto\x1b[17Gproceed?";
        const RAW_BLINK_FRAME: &[u8] = b"\x1b[H\r\x1b[10B\x1b[38;2;153;153;153m\xe2\x8f\xba\x1b[39m\x1b[40;1H\x1b[22;2H";
        const RAW_TURN_DONE_LINE: &[u8] = b"\x1b[?25l\x1b[H\r\x1b[14B\x1b[38;2;153;153;153m\xe2\x9c\xbb\x1b[3GSaut\xc3\xa9ed for 6s \xc2\xb7 done\x1b[39m\x1b[40;1H\x1b[35;3H\x1b[?25h";

        const SPINNER_FRAME_INTERVAL_MS: u64 = 120;
        const SPINNER_FRAME_COUNT: u64 = 10;
        const BLINK_INTERVAL_MS: u64 = 600;
        const BLINK_COUNT: u64 = 10;

        struct Replay {
            base: Instant,
            scanner: OutputScanner,
            signals: AgentSessionSignals,
            activity: AgentActivity,
        }

        impl Replay {
            fn new() -> Self {
                Self {
                    base: Instant::now(),
                    scanner: OutputScanner::new(),
                    signals: claude_signals(),
                    activity: AgentActivity::Unknown,
                }
            }

            fn output(&mut self, offset_ms: u64, chunk: &[u8]) {
                let outcome = self.scanner.scan(chunk);
                apply_scan_to_signals(&mut self.signals, &outcome, AGENT_NAME_CLAUDE, at(self.base, offset_ms));
            }

            fn input(&mut self, offset_ms: u64) {
                note_input(&mut self.signals, at(self.base, offset_ms));
            }

            fn tick(&mut self, offset_ms: u64) -> AgentActivity {
                self.activity = classify_session(&self.signals, self.activity, at(self.base, offset_ms));
                self.activity
            }
        }

        fn osc777(event: AgentEvent) -> Vec<u8> {
            agent_osc_sequence(&claude_agent_event_payload(event)).into_bytes()
        }

        #[test]
        fn 실물_시나리오_타임라인을_재생하면_각_단계의_배지가_계약대로_판정된다() {
            let mut replay = Replay::new();

            replay.output(0, RAW_TITLE_START);
            assert_eq!(
                replay.tick(100),
                AgentActivity::Unknown,
                "(a) 시작 타이틀만으로는 판정을 바꾸지 않는다"
            );
            assert_eq!(
                replay.tick(ACTIVITY_WORKING_HOLD_MS),
                AgentActivity::Idle,
                "(a) 유휴 힌트 뒤 HOLD 만큼 조용하면 유휴다"
            );

            replay.input(3_000);
            replay.output(3_000 + ECHO_SUPPRESS_MS - 1, RAW_INPUT_ECHO_FRAME);
            assert_eq!(
                replay.signals.last_substantive_output_at, None,
                "(b) 에코 창 안의 되그림은 실질 출력이 아니다"
            );
            assert_eq!(
                replay.tick(3_000 + ECHO_SUPPRESS_MS),
                AgentActivity::Idle,
                "(b) 타이핑이 배지를 흔들지 않는다"
            );

            let spinner_start = 4_000;
            replay.output(spinner_start, RAW_TITLE_WORKING_LEFT);
            for frame in 0..SPINNER_FRAME_COUNT {
                let chunk = if frame % 2 == 0 {
                    RAW_SPINNER_COUNTER_FRAME
                } else {
                    RAW_SPINNER_GLYPH_FRAME
                };
                replay.output(spinner_start + frame * SPINNER_FRAME_INTERVAL_MS, chunk);
            }
            let spinner_end = spinner_start + SPINNER_FRAME_COUNT * SPINNER_FRAME_INTERVAL_MS;
            assert!(
                replay.signals.last_substantive_output_at.is_some(),
                "(c) 경과 카운터 프레임은 실질 출력이다"
            );
            assert_eq!(
                replay.tick(spinner_end + 100),
                AgentActivity::Working,
                "(c) 스피너 흐름 + ◐ 타이틀은 작업 중이다"
            );

            let dialog_at = 6_000;
            replay.output(dialog_at, RAW_DIALOG_QUESTION);
            assert_eq!(replay.signals.blocked, Some(BlockedSource::Dialog));
            assert_eq!(
                replay.tick(dialog_at + 50),
                AgentActivity::AwaitingInput,
                "(d) 다이얼로그 프레임은 즉시 래치한다"
            );

            for blink in 1..=BLINK_COUNT {
                let blink_at = dialog_at + blink * BLINK_INTERVAL_MS;
                replay.output(blink_at, RAW_BLINK_FRAME);
                assert_eq!(
                    replay.tick(blink_at + 10),
                    AgentActivity::AwaitingInput,
                    "(e) 점멸 {blink}회째에도 래치가 유지된다"
                );
            }
            let blink_end = dialog_at + BLINK_COUNT * BLINK_INTERVAL_MS;
            assert!(
                blink_end - dialog_at >= ACTIVITY_IDLE_QUIET_MS,
                "(e) 시나리오가 유휴 정적 창보다 길어야 의미가 있다"
            );
            assert_eq!(
                replay.tick(blink_end + 500),
                AgentActivity::AwaitingInput,
                "(e) 6초가 지나도 유휴로 떨어지지 않는다"
            );

            let answer_at = 13_000;
            replay.input(answer_at);
            assert_eq!(replay.signals.blocked, None, "(f) 사용자 키 입력은 래치를 푼다");
            replay.output(answer_at + ECHO_SUPPRESS_MS + 200, RAW_TITLE_WORKING_RIGHT);
            replay.output(answer_at + ECHO_SUPPRESS_MS + 200, RAW_SPINNER_COUNTER_FRAME);
            assert_eq!(
                replay.tick(answer_at + ECHO_SUPPRESS_MS + 300),
                AgentActivity::Working,
                "(f) 이어지는 실질 출력은 작업 중이다"
            );

            let turn_end_at = 15_000;
            replay.output(turn_end_at, RAW_TITLE_IDLE_END);
            replay.output(turn_end_at, RAW_TURN_DONE_LINE);
            assert_eq!(
                replay.tick(turn_end_at + ACTIVITY_WORKING_HOLD_MS - 1),
                AgentActivity::Working,
                "(g) 마지막 실질 출력 뒤 HOLD 안에는 아직 작업 중이다"
            );
            assert_eq!(
                replay.tick(turn_end_at + ACTIVITY_WORKING_HOLD_MS),
                AgentActivity::Idle,
                "(g) ✳ 타이틀 뒤 2초 조용하면 유휴다"
            );

            let hook_at = 20_000;
            let mut hook_chunk = osc777(AgentEvent::PermissionRequest);
            hook_chunk.extend_from_slice(RAW_SPINNER_COUNTER_FRAME);
            replay.output(hook_at, &hook_chunk);
            assert_eq!(
                replay.signals.blocked,
                Some(BlockedSource::Event),
                "(h) 같은 청크의 실질 출력이 이벤트 래치를 지우지 않는다"
            );
            assert_eq!(
                replay.tick(hook_at + 50),
                AgentActivity::AwaitingInput,
                "(h) permission_request 는 즉시 래치한다"
            );
            replay.output(hook_at + BLINK_INTERVAL_MS, RAW_BLINK_FRAME);
            assert_eq!(replay.tick(hook_at + BLINK_INTERVAL_MS + 10), AgentActivity::AwaitingInput);
            replay.output(hook_at + 2_000, &osc777(AgentEvent::ToolComplete));
            assert_eq!(replay.signals.blocked, None, "(h) tool_complete 는 래치를 푼다");
            assert_eq!(
                replay.tick(hook_at + 2_100),
                AgentActivity::Working,
                "(h) tool_complete 직후는 작업 중이다"
            );

            let chained_answer_at = 23_000;
            replay.input(chained_answer_at);
            replay.output(chained_answer_at + ECHO_SUPPRESS_MS - 1, RAW_DIALOG_QUESTION);
            assert_eq!(
                replay.signals.blocked,
                Some(BlockedSource::Dialog),
                "(i) 에코 창 안의 다이얼로그도 읽는다"
            );
            assert_eq!(
                replay.tick(chained_answer_at + ECHO_SUPPRESS_MS),
                AgentActivity::AwaitingInput,
                "(i) 연쇄 승인의 다음 다이얼로그를 놓치지 않는다"
            );
        }

        #[test]
        fn 실물_스피너_글리프_프레임만으로는_실질_출력이_아니다() {
            let mut scanner = OutputScanner::new();

            let glyph_only = scanner.scan(RAW_SPINNER_GLYPH_FRAME);
            assert!(
                !is_substantive_output(&glyph_only.text),
                "글리프 한 글자 프레임: {:?}",
                glyph_only.text
            );

            let counter = scanner.scan(RAW_SPINNER_COUNTER_FRAME);
            assert!(is_substantive_output(&counter.text), "경과 카운터 프레임: {:?}", counter.text);
        }

        #[test]
        fn 실물_타이틀_시퀀스는_시작_작업_종료_글리프로_해석된다() {
            let mut scanner = OutputScanner::new();
            let glyph_of = |scanner: &mut OutputScanner, raw: &[u8]| match scanner.scan(raw).events.as_slice() {
                [ScanEvent::Title(title)] => parse_title_glyph(title),
                other => panic!("타이틀 이벤트 하나를 기대했다: {other:?}"),
            };

            assert_eq!(glyph_of(&mut scanner, RAW_TITLE_START), Some(TitleGlyph::Idle));
            assert_eq!(glyph_of(&mut scanner, RAW_TITLE_WORKING_LEFT), Some(TitleGlyph::Working));
            assert_eq!(glyph_of(&mut scanner, RAW_TITLE_WORKING_RIGHT), Some(TitleGlyph::Working));
            assert_eq!(glyph_of(&mut scanner, RAW_TITLE_IDLE_END), Some(TitleGlyph::Idle));
        }
    }
}
