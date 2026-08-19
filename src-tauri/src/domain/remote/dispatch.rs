use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody, IpcResponse, Response};
use tauri::{AppHandle, Manager};

use super::types::{REMOTE_CHANNEL_PREFIX, REMOTE_OWNER_LABEL};
use crate::domain;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub type ChannelSink = Box<dyn Fn(InvokeResponseBody) -> tauri::Result<()> + Send + Sync>;
pub type ChannelFactory = Arc<dyn Fn(String) -> ChannelSink + Send + Sync>;

pub const IMPLEMENTED_JSON_COMMANDS: &[&str] = &[
    "app_get_info",
    "project_list",
    "project_get",
    "project_get_active",
    "project_open",
    "project_close",
    "project_activate",
    "project_reorder",
    "layout_get",
    "layout_open_tab",
    "layout_close_tab",
    "layout_activate_tab",
    "layout_move_tab",
    "layout_split",
    "layout_resize",
    "layout_focus_pane",
    "layout_pin_tab",
    "layout_set_preview",
    "layout_reopen_closed",
    "layout_set_view_state",
    "layout_set_dirty",
    "layout_set_terminal_session",
    "layout_open_untitled",
    "layout_convert_untitled",
    "layout_move_tab_to_window",
    "layout_set_shell_view",
    "file_open",
    "file_save",
    "file_create",
    "file_rename",
    "file_delete",
    "file_copy",
    "file_mirror_dirty",
    "file_list_mirrors",
    "file_clear_mirror",
    "file_prune_mirrors",
    "file_mirror_untitled",
    "file_list_untitled_mirrors",
    "file_clear_untitled_mirror",
    "file_prune_untitled_mirrors",
    "file_flush_complete",
    "tree_rows",
    "tree_toggle",
    "tree_reveal",
    "tree_refresh",
    "search_run",
    "search_replace",
    "search_cancel",
    "plugin_list",
    "plugin_reload",
    "plugin_read_grammar",
    "plugin_install",
    "plugin_uninstall",
    "agent_list",
    "agent_release_marker",
    "agent_cli_status",
    "agent_hooks_status",
    "agent_hooks_install",
    "agent_hooks_uninstall",
    "agent_cli_install",
    "agent_cli_uninstall",
    "agent_pending_external_opens",
    "lsp_spawn",
    "lsp_send",
    "lsp_stop",
    "lsp_restart",
    "lsp_confirm_reinitialize",
    "lsp_report_reinitialize_failure",
    "lsp_sessions",
    "lsp_detect_servers",
    "lsp_resolve_root",
    "lsp_install",
    "lsp_install_cancel",
    "git_init",
    "git_status",
    "git_diff_file",
    "git_diff_staged_text",
    "git_show_file",
    "git_log",
    "git_ahead_behind",
    "git_remotes",
    "git_gutter",
    "git_blame_range",
    "git_stage",
    "git_unstage",
    "git_discard",
    "git_commit",
    "git_push",
    "git_pull",
    "git_fetch",
    "git_current_user",
    "git_branches",
    "git_branch_create",
    "git_branch_checkout",
    "git_branch_delete",
    "git_stash_list",
    "git_stash_push",
    "git_stash_apply",
    "git_stash_drop",
    "git_discard_hunk",
    "git_undo_last_commit",
    "git_conflict_sides",
    "git_resolve_conflict",
    "git_stage_hunk",
    "git_unstage_hunk",
    "git_stage_lines",
    "git_unstage_lines",
    "git_commit_files",
    "git_file_log",
    "git_revert_commit",
    "git_tags",
    "git_tag_create",
    "git_tag_delete",
    "git_checkout_remote_branch",
    "pty_write",
    "pty_resize",
    "pty_kill",
    "pty_set_paused",
    "pty_detach",
    "terminal_sessions",
    "shell_profiles",
    "resolve_terminal_path",
    "pty_default_options",
    "detect_tasks",
    "font_list",
    "locale_list",
    "locale_get",
    "locale_get_current",
    "theme_list",
    "theme_get",
    "theme_get_current",
    "theme_save",
    "theme_delete",
    "snippet_list",
    "snippet_save",
    "snippet_delete",
    "settings_get",
    "settings_update",
    "settings_set_theme",
    "system_usage_get",
    "system_usage_breakdown",
    "system_open_path",
    "system_reveal_path",
    "system_open_in_browser",
    "system_open_app_data_path",
    "system_open_external_url",
    "ide_get_status",
    "ide_set_selection",
    "ide_clear_selection",
    "ide_publish_diagnostics",
    "ide_resolve_diff",
    "ide_resolve_save",
    "ide_notify_at_mention",
    "ai_token_status",
    "ai_set_token",
    "ai_clear_token",
    "ai_list_models",
    "ai_inline_complete",
    "ai_inline_edit",
    "ai_commit_message",
    "ai_request_cancel",
    "sync_status",
    "sync_connect",
    "sync_disconnect",
    "sync_upload",
    "sync_download",
    "vsix_extract_themes",
    "vsix_import_plugin",
    "remote_status",
    "remote_issue_link",
    "remote_revoke_sessions",
    "remote_set_password",
    "remote_clear_password",
    "window_set_fullscreen",
    "app_file_read",
    "app_file_write",
];

fn err(error: AppError) -> Value {
    serde_json::to_value(&error).unwrap_or_else(|_| serde_json::json!({"code": "Internal", "message": "직렬화 실패"}))
}

fn json_ok<T: serde::Serialize>(value: T) -> Result<String, Value> {
    serde_json::to_string(&value).map_err(|error| err(AppError::Internal(error.to_string())))
}

fn respond<T: serde::Serialize>(result: AppResult<T>) -> Result<String, Value> {
    match result {
        Ok(value) => json_ok(value),
        Err(error) => Err(err(error)),
    }
}

fn from_arg<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, Value> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|error| err(AppError::InvalidArgument(format!("{key}: {error}"))))
}

fn make_channel<T>(args: &Value, key: &str, factory: &ChannelFactory) -> Result<Channel<T>, Value> {
    let raw = args
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| err(AppError::InvalidArgument(format!("{key}: 채널 인자가 필요합니다"))))?;
    let id = raw.strip_prefix(REMOTE_CHANNEL_PREFIX).unwrap_or(raw).to_string();
    Ok(Channel::new(factory(id)))
}

/// Categorizes *why* [`dispatch`]/[`dispatch_raw`] refuse a command to every remote session. This
/// replaces the previous per-command free-text `deny_remote_*` helper functions (one Korean message
/// literal apiece, unchecked at compile time) with a closed, exhaustively-matched enum: every unconditional
/// denial in [`REMOTE_DENIED_COMMANDS`] now carries exactly one variant instead of a function pointer, and
/// [`RemoteDenialPolicy::message`] is the single place that derives the user-facing Korean text — commands
/// that share a rationale now share one message wording instead of each hand-writing a near-duplicate
/// string. [`RemoteDenialPolicy::Unclassified`] additionally backs the *default-deny* gate itself: see
/// [`REMOTE_ALLOWED_COMMANDS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteDenialPolicy {
    /// `remote_set_password`/`remote_clear_password` (a remote session must never flip its own access
    /// gate) and `remote_issue_link` (with no password configured, a freshly issued link alone
    /// establishes a new session — see `server.rs`'s `password_configured == false` branch — so letting
    /// an already-authenticated remote session mint its own would let it self-provision access the
    /// desktop user never saw or approved). See
    /// `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §6.
    SelfAccessExpansion,
    /// Every command whose real handler would pop a window, dialog, or app on the *desktop's own*
    /// display: `window_set_fullscreen`/`layout_move_tab_to_window` (native OS windows with no
    /// remote-renderable counterpart — remote access only ever mirrors the main window's layout;
    /// `window_open_auxiliary` used to be a third member of this group before it was removed as a
    /// duplicate IPC surface over `layout_move_tab_to_window`'s own `newAuxiliary` path, X1#13),
    /// `system_open_external_url` (the desktop's OS-default browser via
    /// `tauri_plugin_opener::open_url`), and `system_open_path`/`system_reveal_path`/
    /// `system_open_in_browser`/`system_open_app_data_path` (same `tauri_plugin_opener` family — default
    /// app opener, Finder/Explorer reveal, `file://` browser open). A remote browser session has no way
    /// to see or use a window the desktop pops up on its own screen, so honoring any of these would only
    /// ever open an unwanted, unreachable window on the desktop user's machine.
    UnreachableDesktopWindow,
    /// `plugin_install`/`plugin_uninstall`/`vsix_import_plugin` (an arbitrary local plugin-directory
    /// path, read from or written to) and `vsix_extract_themes` (an arbitrary local `.vsix` path read,
    /// with no root guard) — none of these are scoped to the currently open project root the way
    /// `file_*`/`tree_*` already are, so a remote session must not be handed them. Note this is
    /// surface reduction, not a confidentiality boundary: an authenticated remote session already
    /// has terminal RCE via the allowed `pty_spawn` (see `strip_remote_gated_settings_patch`'s
    /// doc), so what this variant forecloses is the *command surface* accepting arbitrary local
    /// paths directly, not the session's ultimate capability ceiling.
    LocalFilesystemEscape,
    /// `lsp_install` — downloads a language-server archive (often hundreds of megabytes) onto the
    /// desktop's local disk and spawns an installer process there, the same arbitrary
    /// download-and-execute shape [`LocalFilesystemEscape`] denies for plugins, just with a spawned
    /// process on top. The same capability-ceiling caveat as [`LocalFilesystemEscape`] applies —
    /// `pty_spawn` already grants an authenticated session a shell; this closes the dedicated
    /// install surface, not process execution as such.
    InstallOrProcessExecution,
    /// `agent_cli_install`/`agent_cli_uninstall` (symlinks `/usr/local/bin/taide` directly, or — when
    /// that needs elevation — shells out to `osascript` and pops a native administrator-privilege prompt
    /// on the desktop's own display) and `agent_hooks_install` for a `HookInstallScope::User` agent
    /// (codex/gemini's `~/.codex/hooks.json`/`~/.gemini/settings.json`, outside any project root guard —
    /// injects a `command` hook TAIDE's own CLI executes on every hook event). Both plant or manage a
    /// persistent CLI-executed backdoor that outlives the remote session itself; `HookInstallScope::
    /// Project` agents (`claude`, root-guarded by `project_root`) stay allowed remotely and never reach
    /// this variant.
    DesktopCliInterception,
    /// `file_flush_complete` — resumes the desktop app's own `CloseRequested` hot-exit flush
    /// (`AppState::complete_hot_exit_flush` racing `AppHandle::exit`). A remote browser session has no OS
    /// window to close and must never be able to race ahead of the desktop's own flush to end the
    /// process early.
    DesktopExitControl,
    /// `agent_pending_external_opens` — drains `AgentStore`'s single session-agnostic queue of
    /// CLI-triggered file-open requests (`taide open <path> --wait`) via `mem::take`; whichever session
    /// calls first empties it for everyone, desktop included, and a remote session draining it leaves the
    /// external CLI process blocked (its `waitMarker` release registers in a realm the desktop's own
    /// tab-close handling never observes).
    SharedSingletonStateRace,
    /// The default-deny fallback: a command name that is not filed into either
    /// [`REMOTE_ALLOWED_COMMANDS`] or [`REMOTE_DENIED_COMMANDS`]. Every command [`dispatch`]/
    /// [`dispatch_raw`] can actually reach must be classified into exactly one of those two tables — see
    /// the completeness test `허용_테이블과_거부_테이블은_전체_커맨드를_교집합_없이_정확히_분할한다` — so
    /// this variant answers both a genuinely unrecognized command name and, as a second line of defense,
    /// the `match` arms' own unreachable-by-construction `_` fallback in [`dispatch`] and
    /// [`dispatch_raw`].
    Unclassified,
}

impl RemoteDenialPolicy {
    /// Derives the Korean, user-facing denial message for `name` under this policy. One message per
    /// variant — commands sharing a variant share the exact wording.
    fn message(self, name: &str) -> String {
        match self {
            RemoteDenialPolicy::SelfAccessExpansion => {
                format!("원격 세션에서는 원격 접속 권한(비밀번호·접속 링크)을 스스로 변경하거나 확장할 수 없습니다: {name}")
            }
            RemoteDenialPolicy::UnreachableDesktopWindow => {
                format!("원격 세션에서는 데스크톱 자신의 화면에 표시되는 창이나 앱을 열거나 제어할 수 없습니다: {name}")
            }
            RemoteDenialPolicy::LocalFilesystemEscape => {
                format!("원격 세션에서는 데스크톱의 로컬 파일시스템에 임의 경로로 접근할 수 없습니다: {name}")
            }
            RemoteDenialPolicy::InstallOrProcessExecution => {
                format!("원격 세션에서는 설치 프로그램을 내려받거나 실행할 수 없습니다: {name}")
            }
            RemoteDenialPolicy::DesktopCliInterception => {
                format!("원격 세션에서는 데스크톱 CLI 연동(설치·훅)을 변경할 수 없습니다: {name}")
            }
            RemoteDenialPolicy::DesktopExitControl => {
                format!("원격 세션에서는 앱 종료를 제어할 수 없습니다: {name}")
            }
            RemoteDenialPolicy::SharedSingletonStateRace => {
                format!("원격 세션에서는 보류 중인 외부 열기 요청을 조회할 수 없습니다: {name}")
            }
            RemoteDenialPolicy::Unclassified => {
                format!("원격 세션에서는 아직 허용 목록에 등재되지 않은 명령을 실행할 수 없습니다: {name}")
            }
        }
    }
}

/// Builds the `AppError::Forbidden` value [`dispatch`]/[`dispatch_raw`] answer a denied remote request
/// with, for `name` denied under `policy`.
fn denial_response(policy: RemoteDenialPolicy, name: &str) -> Value {
    err(AppError::Forbidden(policy.message(name)))
}

/// Strips `remote_password_only_login`, `remote_allowed_hosts`, and
/// `shell_override` from a `settings_update` patch arriving through the
/// remote dispatch table.
///
/// The first two guard self-expansion: a remote session must never be able
/// to flip its own access gate from password-optional to password-only (or
/// back), nor grow the Host allowlist that gates which `Host` headers
/// `auth_middleware` accepts (self-expanding it would let an
/// already-connected remote session register a new tunnel hostname for
/// itself, defeating the allowlist's purpose — see
/// `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §3.1).
///
/// `shell_override` guards persistence instead: an authenticated remote
/// session already has terminal RCE via `pty_spawn` (also reachable through
/// this same dispatch table), so stripping it here doesn't close an
/// *immediate* capability gap — but left unfiltered, that same session could
/// use `settings_update` to plant an arbitrary executable as the shell the
/// desktop user's *next* locally-opened terminal spawns (`terminal/commands.rs`
/// reads `shell_override` fresh at spawn time), a backdoor that outlives the
/// remote session itself (survives a password change, session revocation, or
/// the remote server being stopped entirely). The gist-sync path treats this
/// same field as "[RCE급]" for the identical reason (`sync/service.rs`'s
/// `strip_non_syncable`); this is the live-session analogue of that filter.
///
/// `dispatch()` is exclusively the remote request path (desktop calls invoke
/// `settings_update` directly as a Tauri command, bypassing this table
/// entirely — see `ws.rs`), so every patch reaching this arm is remote by
/// construction; no additional "is this remote" check is needed here.
/// `remote_access_enabled` is intentionally left untouched: a remote session
/// disabling remote access only revokes its own future access, matching the
/// existing self-service philosophy for that field (see
/// `docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md` §4).
fn strip_remote_gated_settings_patch(mut patch: domain::settings::service::SettingsPatch) -> domain::settings::service::SettingsPatch {
    patch.remote_password_only_login = None;
    patch.remote_allowed_hosts = None;
    patch.shell_override = None;
    patch
}

/// Same guarantee as [`strip_remote_gated_settings_patch`], for the `Settings`-target
/// `app_file_write` arm — that command applies a *whole* `settings.json` body (parsed from
/// arbitrary remote-supplied JSON), not a `settings_update` patch, so it can't reuse the
/// patch-shaped strip above. Rather than clearing the three gated fields, this forces them back to
/// `current`'s live values: `Settings` has no `Option` wrapper for `remote_password_only_login`/
/// `remote_allowed_hosts` (only `shell_override` does), so "clear" isn't representable — "leave
/// unchanged" is the equivalent operation. Without this, a remote session could plant a persistent
/// `shell_override` (a backdoor that outlives the session — see the doc comment above) or
/// self-expand its own access gate purely by calling `app_file_write` instead of `settings_update`,
/// silently defeating the strip those two entry points are supposed to share equally
/// (contract §3.3: "app_file_* 는 settings_update 와 동급으로 허용").
fn strip_remote_gated_settings(
    mut next: domain::settings::types::Settings,
    current: &domain::settings::types::Settings,
) -> domain::settings::types::Settings {
    next.remote_password_only_login = current.remote_password_only_login;
    next.remote_allowed_hosts = current.remote_allowed_hosts.clone();
    next.shell_override = current.shell_override.clone();
    next
}

/// Reports whether a remote `agent_hooks_install` call for `agent_name` must be denied under
/// [`RemoteDenialPolicy::DesktopCliInterception`] — true for `HookInstallScope::User` agents, false for
/// `HookInstallScope::Project` agents and for an unrecognized `agent_name`. An unrecognized name is
/// deliberately let through rather than denied here: the real handler re-derives the same scope via
/// `domain::agent::service::hook_scope_for_agent` and returns the identical `InvalidArgument` error,
/// so duplicating that validation here would only create a second source of truth for what counts as
/// a known agent name.
fn is_remote_denied_hook_install_scope(agent_name: &str) -> bool {
    matches!(
        domain::agent::service::hook_scope_for_agent(agent_name),
        Ok(domain::agent::types::HookInstallScope::User)
    )
}

/// Every command name [`dispatch`] refuses unconditionally (the denial depends only on `name`, never
/// on `args` or app state — `agent_hooks_install`'s scope-conditional denial via
/// [`is_remote_denied_hook_install_scope`] is the one exception and stays a `match` arm of its own, since
/// it also needs to let the *allowed* scope through to the real handler rather than only ever denying).
/// [`dispatch`] consults this table *before* its command `match` even runs, so this table's contents —
/// not a second, hand-written copy of the same name-to-handler mapping inside the `match` — are the
/// actual routing decision a remote request goes through. [`remote_denied_response`] below is what
/// both `dispatch` and this module's own tests call, so a test asserting "`remote_issue_link` is
/// denied" is asserting exactly the function `dispatch` runs for that name, not a same-shaped
/// stand-in that could silently drift from it — see `docs/acknowledge/2026-08-18-audit-t1-batch1-
/// contract.md` §1 T1-E.
type RemoteDeniedCommandEntry = (&'static str, RemoteDenialPolicy);

const REMOTE_DENIED_COMMANDS: &[RemoteDeniedCommandEntry] = &[
    ("layout_move_tab_to_window", RemoteDenialPolicy::UnreachableDesktopWindow),
    ("file_flush_complete", RemoteDenialPolicy::DesktopExitControl),
    ("plugin_install", RemoteDenialPolicy::LocalFilesystemEscape),
    ("plugin_uninstall", RemoteDenialPolicy::LocalFilesystemEscape),
    ("agent_cli_install", RemoteDenialPolicy::DesktopCliInterception),
    ("agent_cli_uninstall", RemoteDenialPolicy::DesktopCliInterception),
    ("agent_pending_external_opens", RemoteDenialPolicy::SharedSingletonStateRace),
    ("lsp_install", RemoteDenialPolicy::InstallOrProcessExecution),
    ("system_open_path", RemoteDenialPolicy::UnreachableDesktopWindow),
    ("system_reveal_path", RemoteDenialPolicy::UnreachableDesktopWindow),
    ("system_open_in_browser", RemoteDenialPolicy::UnreachableDesktopWindow),
    ("system_open_app_data_path", RemoteDenialPolicy::UnreachableDesktopWindow),
    ("system_open_external_url", RemoteDenialPolicy::UnreachableDesktopWindow),
    ("vsix_extract_themes", RemoteDenialPolicy::LocalFilesystemEscape),
    ("vsix_import_plugin", RemoteDenialPolicy::LocalFilesystemEscape),
    ("remote_issue_link", RemoteDenialPolicy::SelfAccessExpansion),
    ("remote_set_password", RemoteDenialPolicy::SelfAccessExpansion),
    ("remote_clear_password", RemoteDenialPolicy::SelfAccessExpansion),
    ("window_set_fullscreen", RemoteDenialPolicy::UnreachableDesktopWindow),
];

/// Looks `name` up in [`REMOTE_DENIED_COMMANDS`], returning the denial [`dispatch`] must answer with
/// for an unconditionally-denied remote command, or `None` when `name` isn't one (either it's on
/// [`REMOTE_ALLOWED_COMMANDS`] and a real handler exists for it, or it's unclassified — `dispatch`'s
/// own default-deny gate decides which).
fn remote_denied_response(name: &str) -> Option<Value> {
    REMOTE_DENIED_COMMANDS
        .iter()
        .find(|(denied_name, _)| *denied_name == name)
        .map(|(_, policy)| denial_response(*policy, name))
}

/// Every command name [`dispatch`]/[`dispatch_raw`] will actually route to a real handler for a remote
/// session — audited directly off the `match` arms in both functions (159 entries = the 158 arms in
/// [`dispatch`]'s `match` plus `file_read_raw`, [`dispatch_raw`]'s one arm), not derived from
/// [`IMPLEMENTED_JSON_COMMANDS`] minus [`REMOTE_DENIED_COMMANDS`]: deriving it that way would make any
/// newly-added command silently "allowed by subtraction" the moment it's dropped into
/// [`IMPLEMENTED_JSON_COMMANDS`], the exact "new `match` arm = automatic remote allow" gap this table
/// exists to close (`IMPLEMENTED_JSON_COMMANDS` also excludes `pty_spawn`/`pty_attach` — real-handler
/// arms in [`dispatch`]'s own `match` — since it tracks specta/bindings parity, a different axis; see
/// that constant's own doc comment).
///
/// [`dispatch`]/[`dispatch_raw`] both consult this table *before* their `match` even runs: a name absent
/// from both this table and [`REMOTE_DENIED_COMMANDS`] is answered with
/// [`RemoteDenialPolicy::Unclassified`] rather than falling through to `match`. See the completeness
/// test `허용_테이블과_거부_테이블은_전체_커맨드를_교집합_없이_정확히_분할한다`, which fails the moment a
/// command is filed into neither table — the previous "silent = allowed" default is now "silent =
/// denied, and untested silence doesn't compile clean".
const REMOTE_ALLOWED_COMMANDS: &[&str] = &[
    "app_get_info",
    "project_list",
    "project_get",
    "project_get_active",
    "project_open",
    "project_close",
    "project_activate",
    "project_reorder",
    "layout_get",
    "layout_open_tab",
    "layout_close_tab",
    "layout_activate_tab",
    "layout_move_tab",
    "layout_split",
    "layout_resize",
    "layout_focus_pane",
    "layout_pin_tab",
    "layout_set_preview",
    "layout_reopen_closed",
    "layout_set_view_state",
    "layout_set_dirty",
    "layout_set_terminal_session",
    "layout_open_untitled",
    "layout_convert_untitled",
    "layout_set_shell_view",
    "file_open",
    "file_save",
    "file_create",
    "file_rename",
    "file_delete",
    "file_copy",
    "file_mirror_dirty",
    "file_list_mirrors",
    "file_clear_mirror",
    "file_prune_mirrors",
    "file_mirror_untitled",
    "file_list_untitled_mirrors",
    "file_clear_untitled_mirror",
    "file_prune_untitled_mirrors",
    "tree_rows",
    "tree_toggle",
    "tree_reveal",
    "tree_refresh",
    "search_run",
    "search_replace",
    "search_cancel",
    "plugin_list",
    "plugin_reload",
    "plugin_read_grammar",
    "agent_list",
    "agent_release_marker",
    "agent_cli_status",
    "agent_hooks_status",
    "agent_hooks_install",
    "agent_hooks_uninstall",
    "lsp_spawn",
    "lsp_send",
    "lsp_stop",
    "lsp_restart",
    "lsp_confirm_reinitialize",
    "lsp_report_reinitialize_failure",
    "lsp_sessions",
    "lsp_detect_servers",
    "lsp_resolve_root",
    "lsp_install_cancel",
    "git_init",
    "git_status",
    "git_diff_file",
    "git_diff_staged_text",
    "git_show_file",
    "git_log",
    "git_ahead_behind",
    "git_remotes",
    "git_gutter",
    "git_blame_range",
    "git_stage",
    "git_unstage",
    "git_discard",
    "git_commit",
    "git_push",
    "git_pull",
    "git_fetch",
    "git_current_user",
    "git_branches",
    "git_branch_create",
    "git_branch_checkout",
    "git_branch_delete",
    "git_stash_list",
    "git_stash_push",
    "git_stash_apply",
    "git_stash_drop",
    "git_discard_hunk",
    "git_undo_last_commit",
    "git_conflict_sides",
    "git_resolve_conflict",
    "git_stage_hunk",
    "git_unstage_hunk",
    "git_stage_lines",
    "git_unstage_lines",
    "git_commit_files",
    "git_file_log",
    "git_revert_commit",
    "git_tags",
    "git_tag_create",
    "git_tag_delete",
    "git_checkout_remote_branch",
    "pty_write",
    "pty_resize",
    "pty_kill",
    "pty_set_paused",
    "pty_detach",
    "terminal_sessions",
    "shell_profiles",
    "resolve_terminal_path",
    "pty_default_options",
    "detect_tasks",
    "font_list",
    "locale_list",
    "locale_get",
    "locale_get_current",
    "theme_list",
    "theme_get",
    "theme_get_current",
    "theme_save",
    "theme_delete",
    "snippet_list",
    "snippet_save",
    "snippet_delete",
    "settings_get",
    "settings_update",
    "settings_set_theme",
    "system_usage_get",
    "system_usage_breakdown",
    "ide_get_status",
    "ide_set_selection",
    "ide_clear_selection",
    "ide_publish_diagnostics",
    "ide_resolve_diff",
    "ide_resolve_save",
    "ide_notify_at_mention",
    "ai_token_status",
    "ai_set_token",
    "ai_clear_token",
    "ai_list_models",
    "ai_inline_complete",
    "ai_inline_edit",
    "ai_commit_message",
    "ai_request_cancel",
    "sync_status",
    "sync_connect",
    "sync_disconnect",
    "sync_upload",
    "sync_download",
    "remote_status",
    "remote_revoke_sessions",
    "app_file_read",
    "app_file_write",
    "pty_spawn",
    "pty_attach",
    "file_read_raw",
];

macro_rules! arg {
    ($args:expr, $key:literal) => {
        from_arg(&$args, $key)?
    };
}

/// The trust boundary for every `"owner"` value a remote request can ever carry: `dispatch`/
/// `dispatch_raw` are the only two entry points a request arriving over `ws.rs`'s WebSocket can reach
/// (see `handle_request`), and every `args` value they receive is client-controlled JSON from an
/// authenticated but otherwise adversarial remote session. A handful of commands key window-scoped
/// state by a caller-supplied `owner` string — `IdeStore::is_desktop_owner` (selection no-ops for a
/// remote `owner`), and the owner-keyed maps in `SearchStore`/`AiRequestStore`/`LspStore`'s
/// `SessionEntry.channels` — precisely so a browser-side remote session can never be mistaken for the
/// real desktop window (`main`/`editor-<n>`) it is scoped apart from. Every one of those gates only
/// holds if a remote caller can never make its `owner` say anything other than [`REMOTE_OWNER_LABEL`];
/// left to the client, `owner: "main"` impersonates the desktop window outright and walks straight
/// through every gate above (R6#12).
///
/// Rather than adding that same check at each of today's handful of call sites — top-level
/// (`arg!(args, "owner")`: `search_run`/`search_cancel`/`lsp_stop`/`ide_clear_selection`/
/// `ai_request_cancel`) as well as nested one level inside a request/input struct
/// (`IdeSelectionInput.owner` via `arg!(args, "input")`; `LspSpawnRequest.owner`,
/// `AiInlineCompleteRequest.owner`/`AiInlineEditRequest.owner`/`AiCommitMessageRequest.owner` via
/// `arg!(args, "request")`) — and relying on every *future* command that grows an `owner` field to
/// remember it too, this walks the whole `args` tree exactly once, before any per-command handler runs,
/// and force-overwrites every JSON object key literally named `"owner"` it finds, however deeply
/// nested. A well-behaved remote client already sends [`REMOTE_OWNER_LABEL`] for `owner` (the
/// remote-mirror shim's `getCurrentWindow().label` — `src/shared/lib/remote/tauri-internals-shim.ts`),
/// so this is a no-op for it; only a spoofed `owner` is ever changed. See
/// `docs/acknowledge/2026-08-19-audit-t1-batch3-contract.md`.
fn enforce_remote_owner_label(mut args: Value) -> Value {
    match &mut args {
        Value::Object(fields) => {
            if fields.contains_key("owner") {
                fields.insert("owner".to_string(), Value::String(REMOTE_OWNER_LABEL.to_string()));
            }
            for field in fields.values_mut() {
                *field = enforce_remote_owner_label(std::mem::take(field));
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                *item = enforce_remote_owner_label(std::mem::take(item));
            }
        }
        _ => {}
    }
    args
}

/// Routes one JSON-args remote request to its real handler, gating every request through three steps in
/// order before any handler runs: (1) [`REMOTE_DENIED_COMMANDS`] — an unconditional denial always wins
/// first; (2) default-deny — `name` must also be listed in [`REMOTE_ALLOWED_COMMANDS`], or it is refused
/// under [`RemoteDenialPolicy::Unclassified`] without ever reaching the `match` below; only then (3)
/// [`enforce_remote_owner_label`] normalizes `args` before the `match` dispatches to a real handler. A new
/// command's `match` arm alone no longer grants it remote reachability — it must also be filed into
/// [`REMOTE_ALLOWED_COMMANDS`] (or [`REMOTE_DENIED_COMMANDS`]), the inversion of the previous "absent from
/// both tables = silently allowed" default (T1-K, `docs/acknowledge/2026-08-19-audit-t1k-default-deny-
/// contract.md`).
pub async fn dispatch(app: &AppHandle, name: &str, args: Value, channel_factory: ChannelFactory) -> Result<String, Value> {
    if let Some(denial) = remote_denied_response(name) {
        return Err(denial);
    }
    if !REMOTE_ALLOWED_COMMANDS.contains(&name) {
        return Err(denial_response(RemoteDenialPolicy::Unclassified, name));
    }
    let args = enforce_remote_owner_label(args);

    use domain::agent::commands as agent;
    use domain::ai::commands as ai;
    use domain::file::commands as file;
    use domain::git::commands as git;
    use domain::ide::commands as ide;
    use domain::layout::commands as layout;
    use domain::locale::commands as locale;
    use domain::lsp::commands as lsp;
    use domain::plugin::commands as plugin;
    use domain::project::commands as project;
    use domain::remote::commands as remote;
    use domain::search::commands as search;
    use domain::settings::commands as settings;
    use domain::snippet::commands as snippet;
    use domain::sync::commands as sync;
    use domain::system::commands as system;
    use domain::task::commands as task;
    use domain::terminal::commands as terminal;
    use domain::theme::commands as theme;
    use domain::tree::commands as tree;

    match name {
        "app_get_info" => respond(domain::app::commands::app_get_info().await),
        "app_file_read" => respond(domain::app::commands::app_file_read(app.state(), arg!(args, "target")).await),
        "app_file_write" => {
            let target: domain::app::types::AppFileTarget = arg!(args, "target");
            let content: String = arg!(args, "content");
            match target {
                domain::app::types::AppFileTarget::Settings => match domain::settings::service::parse_settings_json(&content) {
                    Ok(parsed) => {
                        let current = app.state::<AppState>().settings.read().clone();
                        let sanitized = strip_remote_gated_settings(parsed, &current);
                        respond(domain::app::commands::apply_settings_file(app.clone(), app.state(), sanitized).await)
                    }
                    Err(error) => Err(err(error)),
                },
                other => respond(domain::app::commands::app_file_write(app.clone(), app.state(), other, content).await),
            }
        }

        "project_list" => respond(project::project_list(app.state()).await),
        "project_get" => respond(project::project_get(app.state(), arg!(args, "projectId")).await),
        "project_get_active" => respond(project::project_get_active(app.state()).await),
        "project_open" => respond(project::project_open(app.clone(), app.state(), arg!(args, "path")).await),
        "project_close" => {
            respond(project::project_close(app.clone(), app.state(), app.state(), app.state(), arg!(args, "projectId")).await)
        }
        "project_activate" => respond(project::project_activate(app.clone(), app.state(), arg!(args, "projectId")).await),
        "project_reorder" => respond(project::project_reorder(app.clone(), app.state(), arg!(args, "ids")).await),

        "layout_get" => respond(layout::layout_get(app.state(), arg!(args, "projectId")).await),
        "layout_open_tab" => respond(
            layout::layout_open_tab(
                app.clone(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "kind"),
                arg!(args, "title"),
                arg!(args, "target"),
                arg!(args, "preview"),
            )
            .await,
        ),
        "layout_close_tab" => respond(layout::layout_close_tab(app.clone(), app.state(), arg!(args, "tabId")).await),
        "layout_activate_tab" => respond(layout::layout_activate_tab(app.clone(), app.state(), arg!(args, "tabId")).await),
        "layout_move_tab" => respond(
            layout::layout_move_tab(
                app.clone(),
                app.state(),
                arg!(args, "tabId"),
                arg!(args, "paneId"),
                arg!(args, "index"),
            )
            .await,
        ),
        "layout_split" => respond(
            layout::layout_split(
                app.clone(),
                app.state(),
                arg!(args, "paneId"),
                arg!(args, "edge"),
                arg!(args, "tabId"),
            )
            .await,
        ),
        "layout_resize" => respond(layout::layout_resize(app.clone(), app.state(), arg!(args, "paneId"), arg!(args, "sizes")).await),
        "layout_focus_pane" => respond(layout::layout_focus_pane(app.clone(), app.state(), arg!(args, "paneId")).await),
        "layout_pin_tab" => respond(layout::layout_pin_tab(app.clone(), app.state(), arg!(args, "tabId"), arg!(args, "pinned")).await),
        "layout_set_preview" => {
            respond(layout::layout_set_preview(app.clone(), app.state(), arg!(args, "tabId"), arg!(args, "preview")).await)
        }
        "layout_reopen_closed" => respond(layout::layout_reopen_closed(app.clone(), app.state(), arg!(args, "projectId")).await),
        "layout_set_view_state" => {
            respond(layout::layout_set_view_state(app.clone(), app.state(), arg!(args, "tabId"), arg!(args, "viewState")).await)
        }
        "layout_set_dirty" => respond(layout::layout_set_dirty(app.clone(), app.state(), arg!(args, "tabId"), arg!(args, "dirty")).await),
        "layout_set_terminal_session" => {
            respond(layout::layout_set_terminal_session(app.clone(), app.state(), arg!(args, "tabId"), arg!(args, "sessionId")).await)
        }
        "layout_open_untitled" => {
            respond(layout::layout_open_untitled(app.clone(), app.state(), arg!(args, "projectId"), arg!(args, "target")).await)
        }
        "layout_convert_untitled" => {
            respond(layout::layout_convert_untitled(app.clone(), app.state(), arg!(args, "tabId"), arg!(args, "path")).await)
        }
        "layout_set_shell_view" => {
            respond(layout::layout_set_shell_view(app.clone(), app.state(), arg!(args, "projectId"), arg!(args, "patch")).await)
        }

        "file_open" => respond(file::file_open(app.state(), app.state(), arg!(args, "path")).await),
        "file_save" => respond(file::file_save(app.state(), arg!(args, "path"), arg!(args, "content")).await),
        "file_create" => respond(file::file_create(app.state(), arg!(args, "path"), arg!(args, "isDir")).await),
        "file_rename" => respond(file::file_rename(app.state(), arg!(args, "from"), arg!(args, "to")).await),
        "file_delete" => respond(file::file_delete(app.state(), arg!(args, "path")).await),
        "file_copy" => respond(file::file_copy(app.state(), arg!(args, "from"), arg!(args, "to")).await),
        "file_mirror_dirty" => {
            respond(file::file_mirror_dirty(app.state(), arg!(args, "projectId"), arg!(args, "path"), arg!(args, "content")).await)
        }
        "file_list_mirrors" => respond(file::file_list_mirrors(app.state(), arg!(args, "projectId")).await),
        "file_clear_mirror" => respond(file::file_clear_mirror(app.state(), arg!(args, "projectId"), arg!(args, "path")).await),
        "file_prune_mirrors" => respond(file::file_prune_mirrors(app.state(), arg!(args, "projectId"), arg!(args, "keepPaths")).await),
        "file_mirror_untitled" => {
            respond(file::file_mirror_untitled(app.state(), arg!(args, "projectId"), arg!(args, "tabId"), arg!(args, "content")).await)
        }
        "file_list_untitled_mirrors" => respond(file::file_list_untitled_mirrors(app.state(), arg!(args, "projectId")).await),
        "file_clear_untitled_mirror" => {
            respond(file::file_clear_untitled_mirror(app.state(), arg!(args, "projectId"), arg!(args, "tabId")).await)
        }
        "file_prune_untitled_mirrors" => {
            respond(file::file_prune_untitled_mirrors(app.state(), arg!(args, "projectId"), arg!(args, "keepTabIds")).await)
        }
        "tree_rows" => respond(
            tree::tree_rows(
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "offset"),
                arg!(args, "limit"),
            )
            .await,
        ),
        "tree_toggle" => respond(tree::tree_toggle(app.state(), app.state(), arg!(args, "projectId"), arg!(args, "path")).await),
        "tree_reveal" => respond(tree::tree_reveal(app.state(), app.state(), arg!(args, "projectId"), arg!(args, "path")).await),
        "tree_refresh" => respond(tree::tree_refresh(app.state(), app.state(), arg!(args, "projectId"), arg!(args, "dir")).await),

        "search_run" => respond(
            search::search_run(
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "owner"),
                arg!(args, "sessionId"),
                arg!(args, "query"),
                make_channel(&args, "onMatch", &channel_factory)?,
            )
            .await,
        ),
        "search_replace" => respond(
            search::search_replace(
                app.clone(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "query"),
                arg!(args, "replacement"),
                arg!(args, "paths"),
            )
            .await,
        ),
        "search_cancel" => respond(search::search_cancel(app.state(), app.state(), arg!(args, "owner"), arg!(args, "sessionId")).await),

        "plugin_list" => respond(plugin::plugin_list(app.state(), app.state()).await),
        "plugin_reload" => respond(plugin::plugin_reload(app.state(), app.state()).await),
        "plugin_read_grammar" => {
            respond(plugin::plugin_read_grammar(app.state(), app.state(), arg!(args, "pluginId"), arg!(args, "languageId")).await)
        }
        "agent_list" => respond(agent::agent_list(app.state(), app.state(), app.state(), app.state(), arg!(args, "projectId")).await),
        "agent_release_marker" => respond(agent::agent_release_marker(app.state(), app.state(), arg!(args, "marker")).await),
        "agent_cli_status" => respond(agent::agent_cli_status().await),
        "agent_hooks_status" => respond(agent::agent_hooks_status(app.state(), arg!(args, "projectId"), arg!(args, "agentName")).await),
        "agent_hooks_install" => {
            let project_id = arg!(args, "projectId");
            let agent_name: String = arg!(args, "agentName");
            if is_remote_denied_hook_install_scope(&agent_name) {
                Err(denial_response(RemoteDenialPolicy::DesktopCliInterception, name))
            } else {
                respond(agent::agent_hooks_install(app.clone(), app.state(), project_id, agent_name).await)
            }
        }
        "agent_hooks_uninstall" => {
            respond(agent::agent_hooks_uninstall(app.state(), arg!(args, "projectId"), arg!(args, "agentName")).await)
        }
        "lsp_spawn" => respond(
            lsp::lsp_spawn(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "request"),
                make_channel(&args, "onMessage", &channel_factory)?,
            )
            .await,
        ),
        "lsp_send" => respond(lsp::lsp_send(app.state(), arg!(args, "sessionId"), arg!(args, "message")).await),
        "lsp_stop" => respond(
            lsp::lsp_stop(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "sessionId"),
                arg!(args, "root"),
                arg!(args, "owner"),
            )
            .await,
        ),
        "lsp_restart" => respond(lsp::lsp_restart(app.clone(), app.state(), app.state(), arg!(args, "sessionId")).await),
        "lsp_confirm_reinitialize" => {
            respond(lsp::lsp_confirm_reinitialize(app.clone(), app.state(), arg!(args, "sessionId"), arg!(args, "generation")).await)
        }
        "lsp_report_reinitialize_failure" => {
            respond(lsp::lsp_report_reinitialize_failure(app.clone(), app.state(), arg!(args, "sessionId"), arg!(args, "generation")).await)
        }
        "lsp_sessions" => respond(lsp::lsp_sessions(app.state(), arg!(args, "projectId")).await),
        "lsp_detect_servers" => respond(lsp::lsp_detect_servers(app.state()).await),
        "lsp_resolve_root" => respond(lsp::lsp_resolve_root(arg!(args, "serverId"), arg!(args, "filePath")).await),
        "lsp_install_cancel" => respond(lsp::lsp_install_cancel(app.state(), arg!(args, "serverId")).await),

        "git_init" => respond(git::git_init(app.clone(), app.state(), app.state(), arg!(args, "projectId")).await),
        "git_status" => respond(git::git_status(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_diff_file" => respond(
            git::git_diff_file(
                app.state(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "mode"),
            )
            .await,
        ),
        "git_diff_staged_text" => respond(git::git_diff_staged_text(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_show_file" => respond(
            git::git_show_file(
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "rev"),
                arg!(args, "path"),
            )
            .await,
        ),
        "git_log" => respond(
            git::git_log(
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "skip"),
                arg!(args, "take"),
            )
            .await,
        ),
        "git_ahead_behind" => respond(git::git_ahead_behind(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_remotes" => respond(git::git_remotes(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_gutter" => respond(git::git_gutter(app.state(), app.state(), arg!(args, "projectId"), arg!(args, "path")).await),
        "git_blame_range" => respond(
            git::git_blame_range(
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "from"),
                arg!(args, "to"),
            )
            .await,
        ),
        "git_stage" => respond(git::git_stage(app.clone(), app.state(), app.state(), arg!(args, "projectId"), arg!(args, "paths")).await),
        "git_unstage" => {
            respond(git::git_unstage(app.clone(), app.state(), app.state(), arg!(args, "projectId"), arg!(args, "paths")).await)
        }
        "git_discard" => {
            respond(git::git_discard(app.clone(), app.state(), app.state(), arg!(args, "projectId"), arg!(args, "paths")).await)
        }
        "git_commit" => respond(
            git::git_commit(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "message"),
                arg!(args, "opts"),
            )
            .await,
        ),
        "git_push" => respond(git::git_push(app.clone(), app.state(), app.state(), arg!(args, "projectId")).await),
        "git_pull" => respond(git::git_pull(app.clone(), app.state(), app.state(), arg!(args, "projectId")).await),
        "git_fetch" => respond(git::git_fetch(app.clone(), app.state(), app.state(), arg!(args, "projectId")).await),
        "git_current_user" => respond(git::git_current_user(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_branches" => respond(git::git_branches(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_branch_create" => respond(
            git::git_branch_create(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "name"),
                arg!(args, "checkout"),
            )
            .await,
        ),
        "git_branch_checkout" => {
            respond(git::git_branch_checkout(app.clone(), app.state(), app.state(), arg!(args, "projectId"), arg!(args, "name")).await)
        }
        "git_branch_delete" => respond(
            git::git_branch_delete(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "name"),
                arg!(args, "force"),
            )
            .await,
        ),
        "git_stash_list" => respond(git::git_stash_list(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_stash_push" => respond(
            git::git_stash_push(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "message"),
            )
            .await,
        ),
        "git_stash_apply" => {
            respond(git::git_stash_apply(app.clone(), app.state(), app.state(), arg!(args, "projectId"), arg!(args, "index")).await)
        }
        "git_stash_drop" => respond(git::git_stash_drop(app.state(), app.state(), arg!(args, "projectId"), arg!(args, "index")).await),
        "git_discard_hunk" => respond(
            git::git_discard_hunk(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "hunkStart"),
                arg!(args, "hunkEnd"),
            )
            .await,
        ),
        "git_undo_last_commit" => respond(git::git_undo_last_commit(app.clone(), app.state(), app.state(), arg!(args, "projectId")).await),
        "git_conflict_sides" => {
            respond(git::git_conflict_sides(app.state(), app.state(), arg!(args, "projectId"), arg!(args, "path")).await)
        }
        "git_resolve_conflict" => respond(
            git::git_resolve_conflict(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "content"),
            )
            .await,
        ),
        "git_stage_hunk" => respond(
            git::git_stage_hunk(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "hunkStart"),
                arg!(args, "hunkEnd"),
            )
            .await,
        ),
        "git_unstage_hunk" => respond(
            git::git_unstage_hunk(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "hunkStart"),
                arg!(args, "hunkEnd"),
            )
            .await,
        ),
        "git_stage_lines" => respond(
            git::git_stage_lines(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "lineStart"),
                arg!(args, "lineEnd"),
            )
            .await,
        ),
        "git_unstage_lines" => respond(
            git::git_unstage_lines(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "lineStart"),
                arg!(args, "lineEnd"),
            )
            .await,
        ),
        "git_commit_files" => respond(git::git_commit_files(app.state(), app.state(), arg!(args, "projectId"), arg!(args, "rev")).await),
        "git_file_log" => respond(
            git::git_file_log(
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "path"),
                arg!(args, "skip"),
                arg!(args, "take"),
            )
            .await,
        ),
        "git_revert_commit" => {
            respond(git::git_revert_commit(app.clone(), app.state(), app.state(), arg!(args, "projectId"), arg!(args, "rev")).await)
        }
        "git_tags" => respond(git::git_tags(app.state(), app.state(), arg!(args, "projectId")).await),
        "git_tag_create" => respond(
            git::git_tag_create(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "name"),
                arg!(args, "target"),
                arg!(args, "opts"),
            )
            .await,
        ),
        "git_tag_delete" => {
            respond(git::git_tag_delete(app.clone(), app.state(), app.state(), arg!(args, "projectId"), arg!(args, "name")).await)
        }
        "git_checkout_remote_branch" => respond(
            git::git_checkout_remote_branch(
                app.clone(),
                app.state(),
                app.state(),
                arg!(args, "projectId"),
                arg!(args, "remoteRef"),
            )
            .await,
        ),

        "pty_spawn" => respond(
            terminal::pty_spawn(
                app.clone(),
                app.state(),
                app.state(),
                app.state(),
                arg!(args, "opts"),
                make_channel(&args, "onData", &channel_factory)?,
            )
            .await,
        ),
        "pty_attach" => respond(
            terminal::pty_attach(
                app.state(),
                app.state(),
                arg!(args, "sessionId"),
                make_channel(&args, "onData", &channel_factory)?,
            )
            .await,
        ),
        "pty_write" => respond(terminal::pty_write(app.state(), arg!(args, "sessionId"), arg!(args, "data")).await),
        "pty_resize" => respond(terminal::pty_resize(app.state(), arg!(args, "sessionId"), arg!(args, "cols"), arg!(args, "rows")).await),
        "pty_kill" => respond(terminal::pty_kill(app.state(), app.state(), arg!(args, "sessionId")).await),
        "pty_set_paused" => respond(terminal::pty_set_paused(app.state(), arg!(args, "sessionId"), arg!(args, "paused")).await),
        "pty_detach" => {
            respond(terminal::pty_detach(app.state(), app.state(), arg!(args, "sessionId"), arg!(args, "subscriptionId")).await)
        }
        "terminal_sessions" => respond(terminal::terminal_sessions(app.state(), arg!(args, "projectId")).await),
        "shell_profiles" => respond(terminal::shell_profiles().await),
        "resolve_terminal_path" => respond(terminal::resolve_terminal_path(arg!(args, "path"), arg!(args, "cwd")).await),
        "pty_default_options" => respond(terminal::pty_default_options(app.state(), arg!(args, "projectId"), arg!(args, "cwd")).await),

        "detect_tasks" => respond(task::detect_tasks(app.state(), arg!(args, "projectId")).await),

        "font_list" => respond(domain::font::commands::font_list().await),

        "locale_list" => respond(locale::locale_list(app.state()).await),
        "locale_get" => respond(locale::locale_get(app.state(), arg!(args, "localeId")).await),
        "locale_get_current" => respond(locale::locale_get_current(app.state(), arg!(args, "systemLanguage")).await),

        "theme_list" => respond(theme::theme_list(app.state()).await),
        "theme_get" => respond(theme::theme_get(app.state(), arg!(args, "themeId")).await),
        "theme_get_current" => respond(theme::theme_get_current(app.state(), arg!(args, "systemTheme")).await),
        "theme_save" => respond(theme::theme_save(app.state(), arg!(args, "theme")).await),
        "theme_delete" => respond(theme::theme_delete(app.state(), arg!(args, "themeId")).await),

        "snippet_list" => respond(snippet::snippet_list(app.state()).await),
        "snippet_save" => respond(snippet::snippet_save(app.state(), arg!(args, "fileName"), arg!(args, "content")).await),
        "snippet_delete" => respond(snippet::snippet_delete(app.state(), arg!(args, "fileName")).await),

        "settings_get" => respond(settings::settings_get(app.state()).await),
        "settings_update" => {
            let patch = strip_remote_gated_settings_patch(arg!(args, "patch"));
            respond(settings::settings_update(app.clone(), app.state(), patch).await)
        }
        "settings_set_theme" => respond(settings::settings_set_theme(app.clone(), app.state(), arg!(args, "themeId")).await),

        "system_usage_get" => respond(system::system_usage_get(app.state()).await),
        "system_usage_breakdown" => {
            respond(system::system_usage_breakdown(app.state(), app.state(), app.state(), app.state(), app.state()).await)
        }
        "ide_get_status" => respond(ide::ide_get_status(app.state()).await),
        "ide_set_selection" => respond(ide::ide_set_selection(app.state(), arg!(args, "input")).await),
        "ide_clear_selection" => respond(ide::ide_clear_selection(app.state(), arg!(args, "owner")).await),
        "ide_publish_diagnostics" => respond(ide::ide_publish_diagnostics(app.state(), arg!(args, "projectId"), arg!(args, "items")).await),
        "ide_resolve_diff" => respond(
            ide::ide_resolve_diff(
                app.state(),
                app.state(),
                arg!(args, "requestId"),
                arg!(args, "outcome"),
                arg!(args, "content"),
            )
            .await,
        ),
        "ide_resolve_save" => respond(ide::ide_resolve_save(app.state(), arg!(args, "requestId"), arg!(args, "saved")).await),
        "ide_notify_at_mention" => {
            respond(ide::ide_notify_at_mention(app.state(), arg!(args, "path"), arg!(args, "lineStart"), arg!(args, "lineEnd")).await)
        }

        "ai_token_status" => respond(ai::ai_token_status(app.state(), app.state()).await),
        "ai_set_token" => respond(ai::ai_set_token(app.state(), arg!(args, "provider"), arg!(args, "token")).await),
        "ai_clear_token" => respond(ai::ai_clear_token(app.state(), arg!(args, "provider")).await),
        "ai_list_models" => respond(ai::ai_list_models(app.state(), app.state(), arg!(args, "provider")).await),
        "ai_inline_complete" => respond(ai::ai_inline_complete(app.state(), app.state(), app.state(), arg!(args, "request")).await),
        "ai_inline_edit" => respond(ai::ai_inline_edit(app.state(), app.state(), app.state(), arg!(args, "request")).await),
        "ai_commit_message" => respond(ai::ai_commit_message(app.state(), app.state(), app.state(), arg!(args, "request")).await),
        "ai_request_cancel" => respond(ai::ai_request_cancel(app.state(), arg!(args, "owner"), arg!(args, "requestId")).await),

        "sync_status" => respond(sync::sync_status(app.state(), app.state()).await),
        "sync_connect" => respond(sync::sync_connect(app.state(), app.state(), arg!(args, "pat")).await),
        "sync_disconnect" => respond(sync::sync_disconnect(app.clone(), app.state(), app.state()).await),
        "sync_upload" => respond(sync::sync_upload(app.clone(), app.state(), app.state()).await),
        "sync_download" => respond(sync::sync_download(app.clone(), app.state(), app.state(), arg!(args, "force")).await),

        "remote_status" => respond(remote::remote_status(app.state()).await),
        "remote_revoke_sessions" => respond(remote::remote_revoke_sessions(app.state()).await),

        _ => Err(denial_response(RemoteDenialPolicy::Unclassified, name)),
    }
}

fn response_bytes(response: Response) -> Result<Vec<u8>, Value> {
    match response.body().map_err(|error| err(AppError::Internal(error.to_string())))? {
        InvokeResponseBody::Raw(bytes) => Ok(bytes),
        InvokeResponseBody::Json(text) => Ok(text.into_bytes()),
    }
}

/// The raw-bytes-response counterpart to [`dispatch`], routing exactly the one command `ws.rs::
/// handle_request` ever forwards here (`file_read_raw` — every other command, `pty_spawn`/`pty_attach`
/// included, goes through [`dispatch`] instead, even though `lib.rs`'s `RAW_CHANNEL_COMMANDS` groups all
/// three for a different reason, see [`REMOTE_ALLOWED_COMMANDS`]'s doc comment). Gated through the exact
/// same two tables and order as [`dispatch`] (deny → default-deny-if-unlisted) before
/// [`enforce_remote_owner_label`] and its own `match` — T1-K's "동일 원칙 적용" for this function turns out
/// to mean *sharing* [`REMOTE_ALLOWED_COMMANDS`]/[`REMOTE_DENIED_COMMANDS`] with [`dispatch`], not growing
/// a second, `RAW_CHANNEL_COMMANDS`-shaped table of its own — this function's `match` has exactly one real
/// arm today, so a second table would only ever re-encode that arm's own existence. Same
/// client-controlled-`args` trust boundary as [`dispatch`] (see [`enforce_remote_owner_label`]'s doc
/// comment) — `file_read_raw` takes no `owner`, so the normalization below is currently a no-op, but every
/// future raw-channel command's `args` gets it for free without anyone having to remember to add it.
pub async fn dispatch_raw(app: &AppHandle, name: &str, args: Value) -> Result<Vec<u8>, Value> {
    if let Some(denial) = remote_denied_response(name) {
        return Err(denial);
    }
    if !REMOTE_ALLOWED_COMMANDS.contains(&name) {
        return Err(denial_response(RemoteDenialPolicy::Unclassified, name));
    }
    let args = enforce_remote_owner_label(args);
    match name {
        "file_read_raw" => {
            let path = from_arg::<String>(&args, "path")?;
            let response = domain::file::commands::file_read_raw(app.state::<AppState>(), path)
                .await
                .map_err(err)?;
            response_bytes(response)
        }
        _ => Err(denial_response(RemoteDenialPolicy::Unclassified, name)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use regex::Regex;

    use super::*;

    fn invoke_command_names() -> BTreeSet<String> {
        let source = include_str!("../../../../src/shared/api/bindings.ts");
        let pattern = Regex::new(r#"__TAURI_INVOKE\(\s*"([a-zA-Z0-9_]+)""#).expect("유효한 정규식");
        let mut names: BTreeSet<String> = pattern.captures_iter(source).map(|capture| capture[1].to_string()).collect();
        names.extend(crate::RAW_CHANNEL_COMMANDS.iter().map(|name| name.to_string()));
        names
    }

    fn implemented_command_names() -> BTreeSet<String> {
        IMPLEMENTED_JSON_COMMANDS
            .iter()
            .chain(crate::RAW_CHANNEL_COMMANDS.iter())
            .map(|name| name.to_string())
            .collect()
    }

    #[test]
    fn bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다() {
        assert_eq!(invoke_command_names(), implemented_command_names());
    }

    #[test]
    fn 에러는_code와_message만_가진_평면_객체로_직렬화된다() {
        let value = err(AppError::NotFound("prj-1".to_string()));
        assert_eq!(value, serde_json::json!({"code": "NotFound", "message": "prj-1"}));
    }

    /// T1-K: a command name that is neither denied nor explicitly allowed must be refused — the default
    /// flipped from "silently allowed" to "denied under `Unclassified`" (`does_not_exist` covers both a
    /// genuine typo and a real command someone forgot to file into either table).
    #[test]
    fn 미분류_커맨드는_forbidden으로_기본_거부된다() {
        let value = denial_response(RemoteDenialPolicy::Unclassified, "does_not_exist");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    #[test]
    fn 성공_값은_json_문자열로_직렬화된다() {
        let encoded = json_ok(serde_json::json!({"ok": true})).expect("직렬화 성공");
        let parsed: Value = serde_json::from_str(&encoded).expect("파싱 성공");
        assert_eq!(parsed["ok"], serde_json::json!(true));
    }

    #[test]
    fn 원격_settings_update_패치는_비밀번호_전용_로그인_필드가_제거된다() {
        let patch = domain::settings::service::SettingsPatch {
            remote_password_only_login: Some(true),
            editor_font_size: Some(18),
            ..Default::default()
        };

        let stripped = strip_remote_gated_settings_patch(patch);

        assert_eq!(stripped.remote_password_only_login, None);
        assert_eq!(stripped.editor_font_size, Some(18), "게이트 필드 외에는 그대로 통과해야 한다");
    }

    #[test]
    fn 원격_settings_update_패치는_허용_호스트_목록_필드가_제거된다() {
        let patch = domain::settings::service::SettingsPatch {
            remote_allowed_hosts: Some(vec!["attacker.example.com".to_string()]),
            editor_font_size: Some(18),
            ..Default::default()
        };

        let stripped = strip_remote_gated_settings_patch(patch);

        assert_eq!(
            stripped.remote_allowed_hosts, None,
            "원격 세션이 스스로 Host 허용목록을 확장하면 안 된다"
        );
        assert_eq!(stripped.editor_font_size, Some(18), "게이트 필드 외에는 그대로 통과해야 한다");
    }

    #[test]
    fn 원격_settings_update_패치는_shell_override_필드가_제거된다() {
        let patch = domain::settings::service::SettingsPatch {
            shell_override: Some("/tmp/evil.sh".to_string()),
            editor_font_size: Some(18),
            ..Default::default()
        };

        let stripped = strip_remote_gated_settings_patch(patch);

        assert_eq!(
            stripped.shell_override, None,
            "원격 세션이 자신의 종료 이후에도 살아남는 셸 백도어를 심으면 안 된다"
        );
        assert_eq!(stripped.editor_font_size, Some(18), "게이트 필드 외에는 그대로 통과해야 한다");
    }

    #[test]
    fn 원격_세션은_링크_발급을_할_수_없다() {
        let value = remote_denied_response("remote_issue_link").expect("거부되어야 한다");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    #[test]
    fn 원격_세션은_호스트_브라우저를_열_수_없다() {
        let value = remote_denied_response("system_open_external_url").expect("거부되어야 한다");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    #[test]
    fn 원격_세션은_데스크톱에서_파일이나_폴더를_열_수_없다() {
        for name in [
            "system_open_path",
            "system_reveal_path",
            "system_open_in_browser",
            "system_open_app_data_path",
        ] {
            let value = remote_denied_response(name).unwrap_or_else(|| panic!("{name} 은 거부되어야 한다"));
            assert_eq!(value["code"], serde_json::json!("Forbidden"), "{name} 은 거부되어야 한다");
        }
    }

    #[test]
    fn 원격_세션은_cli_shell_명령_설치_제거를_할_수_없다() {
        for name in ["agent_cli_install", "agent_cli_uninstall"] {
            let value = remote_denied_response(name).unwrap_or_else(|| panic!("{name} 은 거부되어야 한다"));
            assert_eq!(value["code"], serde_json::json!("Forbidden"), "{name} 은 거부되어야 한다");
        }
    }

    #[test]
    fn 원격_세션은_사용자_범위_에이전트_후킹_설치를_거부한다() {
        assert!(is_remote_denied_hook_install_scope("codex"));
        assert!(is_remote_denied_hook_install_scope("gemini"));
    }

    #[test]
    fn 원격_세션은_프로젝트_범위_에이전트_후킹_설치는_허용한다() {
        assert!(!is_remote_denied_hook_install_scope("claude"));
    }

    #[test]
    fn 알수없는_에이전트명은_후킹_스코프_판정에서_거부되지_않는다() {
        assert!(
            !is_remote_denied_hook_install_scope("bash"),
            "실핸들러가 동일한 InvalidArgument 를 재생성하므로 여기서 중복 검증하지 않는다"
        );
    }

    #[test]
    fn 원격_세션은_보류중인_외부_열기_요청을_조회할_수_없다() {
        let value = remote_denied_response("agent_pending_external_opens").expect("거부되어야 한다");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    #[test]
    fn 원격_세션은_lsp_서버를_설치할_수_없다() {
        let value = remote_denied_response("lsp_install").expect("거부되어야 한다");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    /// `agent_hooks_install` for a User-scope agent is a scope-conditional denial (not in
    /// [`REMOTE_DENIED_COMMANDS`] — see [`is_remote_denied_hook_install_scope`]), so it must be exercised
    /// through [`denial_response`] directly rather than [`remote_denied_response`].
    #[test]
    fn 원격_세션은_사용자_범위_에이전트_후킹_설치_거부_응답이_forbidden이다() {
        let value = denial_response(RemoteDenialPolicy::DesktopCliInterception, "agent_hooks_install");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    /// `dispatch()` never runs its own `match` for these names — [`remote_denied_response`] answers
    /// them first (see its call at the top of [`dispatch`]) — so calling it directly here exercises
    /// exactly the routing decision a real remote request goes through, not a same-shaped mirror of
    /// it. A regression that dropped a name from [`REMOTE_DENIED_COMMANDS`] (accidentally re-opening
    /// that command to remote callers) fails this test with `None` where `Some(Forbidden)` is expected.
    #[test]
    fn dispatch_가_참조하는_거부_테이블의_모든_커맨드는_forbidden_을_반환한다() {
        for (name, _) in REMOTE_DENIED_COMMANDS {
            let denial = remote_denied_response(name).unwrap_or_else(|| panic!("{name} 은 거부 응답을 반환해야 한다"));
            assert_eq!(
                denial["code"],
                serde_json::json!("Forbidden"),
                "{name} 의 거부 응답 코드가 Forbidden 이어야 한다"
            );
        }
    }

    /// A command with a real handler (never in [`REMOTE_DENIED_COMMANDS`]) must fall through
    /// [`remote_denied_response`] untouched so `dispatch`'s `match` can route it normally.
    #[test]
    fn 거부_테이블에_없는_실핸들러_커맨드는_거부되지_않는다() {
        assert_eq!(remote_denied_response("project_list"), None);
    }

    /// Every unconditionally-denied command must still be a recognized command name (listed in
    /// [`IMPLEMENTED_JSON_COMMANDS`]) — otherwise `bindings와_dispatch_테이블은_커맨드_이름_집합이_
    /// 일치한다` would already catch it, but this pins the two tables' relationship explicitly rather
    /// than relying on that other test's failure message to explain why.
    #[test]
    fn 거부_테이블의_모든_커맨드는_구현된_커맨드_목록에도_있다() {
        for (name, _) in REMOTE_DENIED_COMMANDS {
            assert!(
                IMPLEMENTED_JSON_COMMANDS.contains(name),
                "{name} 이 거부 테이블에는 있지만 IMPLEMENTED_JSON_COMMANDS 에는 없다"
            );
        }
    }

    /// T1-K's core enforcement mechanism (`docs/acknowledge/2026-08-19-audit-t1k-default-deny-
    /// contract.md` §1.1): [`REMOTE_ALLOWED_COMMANDS`] and [`REMOTE_DENIED_COMMANDS`] must partition the
    /// full remote-command universe exactly — no command in both (self-contradicting) and no command in
    /// neither (the "new command, unclassified" gap the default-deny gate in [`dispatch`]/
    /// [`dispatch_raw`] exists to close). The universe is [`IMPLEMENTED_JSON_COMMANDS`] plus
    /// `crate::RAW_CHANNEL_COMMANDS` (`lib.rs`) — together exactly what `collect_commands!` registers,
    /// pinned by `collect_commands_매크로_출력과_dispatch_테이블은_커맨드_이름_집합이_일치한다` in
    /// `lib.rs`. Manually verified (T1-K self-check, not left in the tree): deleting one entry from
    /// [`REMOTE_ALLOWED_COMMANDS`] makes this test fail on the union-mismatch assertion below — a new
    /// command left unclassified in either table is caught here, not silently allowed.
    #[test]
    fn 허용_테이블과_거부_테이블은_전체_커맨드를_교집합_없이_정확히_분할한다() {
        let allowed: BTreeSet<&str> = REMOTE_ALLOWED_COMMANDS.iter().copied().collect();
        let denied: BTreeSet<&str> = REMOTE_DENIED_COMMANDS.iter().map(|(name, _)| *name).collect();

        assert_eq!(
            REMOTE_ALLOWED_COMMANDS.len(),
            allowed.len(),
            "REMOTE_ALLOWED_COMMANDS 에 중복된 커맨드 이름이 있다"
        );
        assert_eq!(
            REMOTE_DENIED_COMMANDS.len(),
            denied.len(),
            "REMOTE_DENIED_COMMANDS 에 중복된 커맨드 이름이 있다"
        );

        let intersection: Vec<&&str> = allowed.intersection(&denied).collect();
        assert!(
            intersection.is_empty(),
            "허용과 거부 테이블에 동시에 등재된 커맨드: {intersection:?}"
        );

        let universe: BTreeSet<String> = IMPLEMENTED_JSON_COMMANDS
            .iter()
            .chain(crate::RAW_CHANNEL_COMMANDS.iter())
            .map(|name| name.to_string())
            .collect();
        let union: BTreeSet<String> = allowed.iter().chain(denied.iter()).map(|name| name.to_string()).collect();

        assert_eq!(
            union, universe,
            "REMOTE_ALLOWED_COMMANDS ⊎ REMOTE_DENIED_COMMANDS 가 전체 커맨드 집합과 다르다 — 새 커맨드를 \
             어느 테이블에도 등재하지 않으면 이 assert 가 실패해야 한다"
        );
    }

    /// A command absent from both tables must fall through [`remote_denied_response`] (`None`) *and*
    /// fail the [`REMOTE_ALLOWED_COMMANDS`] membership check — together, exactly what routes `dispatch`/
    /// `dispatch_raw` to their default-deny branch instead of ever reaching `match`.
    #[test]
    fn 어느_테이블에도_없는_커맨드는_거부_테이블에서_none이고_허용_테이블에도_없다() {
        let name = "이것은_등재되지_않은_가상의_커맨드";
        assert_eq!(remote_denied_response(name), None);
        assert!(!REMOTE_ALLOWED_COMMANDS.contains(&name));
    }

    #[test]
    fn 원격_접속_활성화_필드는_스트립되지_않는다() {
        let patch = domain::settings::service::SettingsPatch {
            remote_access_enabled: Some(false),
            ..Default::default()
        };

        let stripped = strip_remote_gated_settings_patch(patch);

        assert_eq!(
            stripped.remote_access_enabled,
            Some(false),
            "원격 접속 자가 차단은 기존 철학대로 허용되어야 한다"
        );
    }

    /// R6#12 위장 회귀: `search_cancel`/`ai_request_cancel`/`lsp_stop`/`ide_clear_selection` 은 모두
    /// `arg!(args, "owner")` 로 최상위 `owner` 키를 그대로 읽는다. 원격 클라이언트가 `owner: "main"`
    /// 을 보내 데스크톱 창 소유의 검색/AI 요청을 교차 취소하려 해도, `dispatch()` 진입 시 이 값이
    /// [`REMOTE_OWNER_LABEL`] 로 강제되어야 한다.
    #[test]
    fn owner_키가_최상위에_있으면_원격_라벨로_강제된다() {
        for spoofed in ["main", "editor-2", ""] {
            let args = serde_json::json!({ "owner": spoofed, "sessionId": "search-panel-1" });

            let sanitized = enforce_remote_owner_label(args);

            assert_eq!(
                sanitized["owner"],
                serde_json::json!(REMOTE_OWNER_LABEL),
                "위장된 owner({spoofed:?})가 강제 치환되지 않았다"
            );
            assert_eq!(
                sanitized["sessionId"],
                serde_json::json!("search-panel-1"),
                "owner 외 필드는 그대로 유지되어야 한다"
            );
        }
    }

    /// R6#12 위장 회귀: `ide_set_selection` 은 `owner` 가 최상위가 아니라 `arg!(args, "input")` 로 받는
    /// `IdeSelectionInput` 구조체 안에 중첩돼 있다(`lsp_spawn` 의 `request.owner` 도 동일 구조). 최상위
    /// 키만 보는 얕은 치환으로는 이 경로가 막히지 않으므로, 중첩된 `owner` 도 강제되어야 한다.
    #[test]
    fn owner_키가_중첩된_객체_안에_있어도_원격_라벨로_강제된다() {
        let args = serde_json::json!({
            "input": {
                "owner": "main",
                "projectId": "prj-1",
                "path": "src/main.ts",
                "isEmpty": true,
            }
        });

        let sanitized = enforce_remote_owner_label(args);

        assert_eq!(sanitized["input"]["owner"], serde_json::json!(REMOTE_OWNER_LABEL));
        assert_eq!(
            sanitized["input"]["projectId"],
            serde_json::json!("prj-1"),
            "owner 외 중첩 필드는 그대로 유지되어야 한다"
        );
    }

    #[test]
    fn owner_키가_배열_안의_객체나_더_깊은_중첩에_있어도_원격_라벨로_강제된다() {
        let args = serde_json::json!({
            "batch": [
                { "owner": "main", "id": 1 },
                { "nested": { "owner": "editor-3" } },
            ]
        });

        let sanitized = enforce_remote_owner_label(args);

        assert_eq!(sanitized["batch"][0]["owner"], serde_json::json!(REMOTE_OWNER_LABEL));
        assert_eq!(sanitized["batch"][1]["nested"]["owner"], serde_json::json!(REMOTE_OWNER_LABEL));
    }

    #[test]
    fn owner_키가_없는_값은_그대로_유지된다() {
        let args = serde_json::json!({ "projectId": "prj-1", "path": "src/main.ts" });

        let sanitized = enforce_remote_owner_label(args.clone());

        assert_eq!(sanitized, args, "owner 가 없는 페이로드는 손대지 않아야 한다");
    }

    /// 정당한 원격 shim(`tauri-internals-shim.ts`)은 이미 [`REMOTE_OWNER_LABEL`] 을 보내므로, 강제
    /// 치환이 그 값을 바꿔서는 안 된다(동작 무변경).
    #[test]
    fn 이미_원격_라벨인_owner_는_그대로_유지된다() {
        let args = serde_json::json!({ "input": { "owner": REMOTE_OWNER_LABEL } });

        let sanitized = enforce_remote_owner_label(args);

        assert_eq!(sanitized["input"]["owner"], serde_json::json!(REMOTE_OWNER_LABEL));
    }

    /// The partition test above proves ALLOWED ⊎ DENIED covers every command *name*, but nothing
    /// there proves each allowed name actually has a `match` arm — a typo'd or removed arm would
    /// leave the name in [`REMOTE_ALLOWED_COMMANDS`] while requests fall through to the
    /// `Unclassified` fallback with a misleading "not registered" message. Same source-scraping
    /// approach as `lib.rs`'s `collect_commands!` parity test; the regex leans on the arms' fixed
    /// 8-space indentation (table entries are 4-space, so they never match) — if the match blocks
    /// are ever re-indented, this test fails loudly rather than silently matching nothing, because
    /// an empty arm set can never equal the non-empty allowed set.
    #[test]
    fn 허용_테이블의_모든_커맨드는_실제_match_arm_을_가진다() {
        let source = include_str!("dispatch.rs");
        let pattern = Regex::new(r#"(?m)^        "([a-zA-Z0-9_]+)" => "#).expect("유효한 정규식");
        let arm_names: BTreeSet<&str> = pattern
            .captures_iter(source)
            .map(|capture| capture.get(1).expect("캡처 그룹").as_str())
            .collect();
        let allowed: BTreeSet<&str> = REMOTE_ALLOWED_COMMANDS.iter().copied().collect();

        assert_eq!(
            arm_names, allowed,
            "REMOTE_ALLOWED_COMMANDS 와 dispatch/dispatch_raw 의 실제 match arm 집합이 어긋났습니다 — 허용 목록과 arm 은 항상 함께 추가/삭제해야 합니다"
        );
    }
}
