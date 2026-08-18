use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody, IpcResponse, Response};
use tauri::{AppHandle, Manager};

use super::types::REMOTE_CHANNEL_PREFIX;
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
    "ide_start",
    "ide_stop",
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
    "remote_start",
    "remote_stop",
    "remote_issue_link",
    "remote_revoke_sessions",
    "remote_set_password",
    "remote_clear_password",
    "window_open_auxiliary",
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

fn unknown_command(name: &str) -> Value {
    err(AppError::InvalidArgument(format!("알 수 없는 커맨드: {name}")))
}

/// Answers `remote_set_password`/`remote_clear_password` with an explicit
/// denial instead of dispatching to the real handler — a remote session must
/// never be able to change its own access gate. Both names stay listed in
/// [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity test
/// (`bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다`) still passes; only the
/// `match` arm in [`dispatch`] refuses to call through.
fn deny_remote_password_change(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 원격 접속 비밀번호를 변경할 수 없습니다: {name}"
    )))
}

/// Answers `remote_issue_link` with an explicit denial instead of dispatching
/// to the real handler — issuing a link is a local-only capability. With no
/// password configured, the returned link alone establishes a new session
/// (see `server.rs`'s `password_configured == false` branch), so letting an
/// already-authenticated remote session mint its own externally-reachable
/// onboarding link would let it self-provision additional access the desktop
/// user never saw or approved — see
/// `docs/acknowledge/2026-08-15-wave-b-hardening-contract.md` §6. Stays
/// listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity
/// test still passes; only the `match` arm in [`dispatch`] refuses to call
/// through.
fn deny_remote_link_issue(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 접속 링크를 발급할 수 없습니다: {name}"
    )))
}

/// Answers `file_flush_complete` with an explicit denial instead of
/// dispatching to the real handler. That command resumes the desktop app's
/// own `CloseRequested` exit sequence (`AppState::complete_hot_exit_flush`
/// racing to `AppHandle::exit`) — a remote browser session has no OS window
/// to close and must never be able to end the desktop process early by
/// racing ahead of the desktop's own hot-exit flush. Stays listed in
/// [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity test still
/// passes; only the `match` arm in [`dispatch`] refuses to call through.
fn deny_remote_flush_complete(name: &str) -> Value {
    err(AppError::Forbidden(format!("원격 세션에서는 앱 종료를 제어할 수 없습니다: {name}")))
}

/// Answers `window_open_auxiliary` with an explicit denial instead of
/// dispatching to the real handler. Auxiliary editor windows are native OS
/// windows (`tauri::WebviewWindowBuilder`) on the desktop's own display — a
/// remote browser session has no local display to place one on and no way
/// to interact with it even if it appeared, so honoring the request would
/// only ever pop up an orphaned, unreachable window on the desktop user's
/// machine. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the
/// bindings/dispatch parity test still passes; only the `match` arm in
/// [`dispatch`] refuses to call through.
fn deny_remote_window_open(name: &str) -> Value {
    err(AppError::Forbidden(format!("원격 세션에서는 새 창을 열 수 없습니다: {name}")))
}

/// Answers `window_set_fullscreen` with an explicit denial. That command's real handler takes a
/// `tauri::Window` extractor auto-injected by the *real* Tauri IPC invoke path per-call — there is
/// no such window to inject for a request arriving through this manually-dispatched remote table
/// (unlike every other entry here, which calls the real handler with explicit arguments), so this
/// command literally cannot be dispatched this way even if it were desirable to. Stays listed in
/// [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity test still passes.
fn deny_remote_window_fullscreen(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 전체화면을 전환할 수 없습니다: {name}"
    )))
}

/// Answers `layout_move_tab_to_window` with an explicit denial — every destination
/// (`TabWindowTarget::Main`/`Existing`/`NewAuxiliary`) ultimately concerns a native OS window a
/// remote browser session has no way to correspondingly render (remote access mirrors the main
/// window's layout only), and `NewAuxiliary` additionally opens a real desktop window the same way
/// `window_open_auxiliary` does — already denied remotely for the identical reason. Stays listed in
/// [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity test still passes.
fn deny_remote_move_tab_to_window(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 탭을 다른 창으로 옮길 수 없습니다: {name}"
    )))
}

/// Answers `plugin_install`/`plugin_uninstall` with an explicit denial — both name an arbitrary
/// filesystem path (or existing plugin directory) on the *desktop* machine; a remote session must
/// never be able to read from or write into the desktop's local filesystem outside the already
/// tightly root-guarded project/file commands. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the
/// bindings/dispatch parity test still passes.
fn deny_remote_plugin_install(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 플러그인을 설치/제거할 수 없습니다: {name}"
    )))
}

/// Answers `vsix_import_plugin` with the same denial as `plugin_install` — it's the same
/// arbitrary-local-path plugin-write capability, just sourced from a real VS Code `.vsix` instead
/// of a TAIDE-native bundle. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch
/// parity test still passes.
fn deny_remote_vsix_import(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 vsix로부터 플러그인을 가져올 수 없습니다: {name}"
    )))
}

/// Answers `vsix_extract_themes` with an explicit denial — previously dispatched to the real
/// handler, which reads an arbitrary local `vsix_path` off the desktop's filesystem with no
/// root-guard. That's fine for a *local* file-picker-sourced path (the same trust boundary every
/// native "Open File" dialog already carries) but was never meant to double as a remote arbitrary
/// local-file-read primitive. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch
/// parity test still passes. See
/// `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.4.
fn deny_remote_vsix_extract_themes(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 vsix 테마를 추출할 수 없습니다: {name}"
    )))
}

/// Answers `system_open_external_url` with an explicit denial instead of dispatching to the real
/// handler — that command opens a URL in the desktop's own OS-default browser
/// (`tauri_plugin_opener::open_url`), which only makes sense on the machine actually running that
/// browser. A remote browser session already has its own browser (the one it's connecting
/// through) and has no way to see or use a window `open_url` pops up on the desktop's display, so
/// honoring the request would only ever open an unwanted browser window on the desktop user's
/// machine — the same rationale as [`deny_remote_window_open`], just for the OS browser instead of
/// a new app window. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity
/// test still passes; only the `match` arm in [`dispatch`] refuses to call through.
fn deny_remote_open_external_url(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 호스트 브라우저를 열 수 없습니다: {name}"
    )))
}

/// Answers `system_open_path`, `system_reveal_path`, `system_open_in_browser`, and
/// `system_open_app_data_path` with an explicit denial instead of dispatching to the real handler —
/// all four hand a path (or a fixed app-data directory) to `tauri_plugin_opener` to pop open in an
/// OS-native app window on the *desktop's* own display: the default-app opener, the Finder/Explorer
/// reveal, and the OS browser (via a `file://` URL) respectively. That only makes sense on the
/// machine actually running that app — a remote session has no way to see or use a window the
/// desktop pops up on its own screen, so honoring the request would only ever open an unwanted
/// window on the desktop user's machine. This is the identical rationale
/// [`deny_remote_open_external_url`] already applies to `system_open_external_url` (same family of
/// commands, same `tauri_plugin_opener` sink), and the same allow-to-deny conversion precedent
/// `deny_remote_vsix_extract_themes` set for Wave I. `system_open_path` was previously left
/// dispatching to the real handler even though it shares this exact rationale and the same
/// root-guard (`resolve_within_open_project`) as `system_reveal_path`/`system_open_in_browser`; it is
/// folded into this denial for consistency rather than left as the one command in the family still
/// popping an unreachable window on the desktop. The path/kind argument's *safety* was never the
/// concern here (all four already root-guard it, or point at a fixed app-data directory) — it's the
/// resulting window's *unreachability* from a remote session that makes every one of them useless to
/// honor remotely. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity test
/// still passes; only the `match` arms in [`dispatch`] refuse to call through.
fn deny_remote_system_open(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 데스크톱에서 파일/폴더를 열거나 표시할 수 없습니다: {name}"
    )))
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

/// Answers `agent_cli_install`/`agent_cli_uninstall` with an explicit denial instead of dispatching
/// to the real handler. On macOS the real handlers either symlink `/usr/local/bin/taide` directly or
/// — when that requires elevated permission — shell out to `osascript` and pop a native
/// administrator-privilege prompt on the desktop's own display (`run_cli_osascript`). A remote
/// browser session has no way to see or answer that prompt, and must never be able to trigger a
/// privilege-escalation dialog on the desktop user's machine at all — the same "no local display to
/// answer on" rationale [`deny_remote_system_open`] already applies to opener windows, just for an OS
/// authorization dialog instead. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the
/// bindings/dispatch parity test still passes; only the `match` arm in [`dispatch`] refuses to call
/// through.
fn deny_remote_agent_cli(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 CLI shell 명령을 설치/제거할 수 없습니다: {name}"
    )))
}

/// Answers `agent_hooks_install` with an explicit denial when [`is_remote_denied_hook_install_scope`]
/// determines the target agent's hooks live outside any project root (`HookInstallScope::User`, i.e.
/// codex/gemini's `~/.codex/hooks.json` / `~/.gemini/settings.json`). Those hooks inject a `command`
/// handler (`domain::agent::service::build_command_hook_shell_command`) that TAIDE's own CLI executes
/// on every hook event — a remote session installing that would be planting a persistent
/// command-execution hook on the desktop user's home directory that survives the remote session
/// itself, the same "backdoor that outlives the session" concern
/// [`strip_remote_gated_settings_patch`] already documents for `shell_override`. Project-scope hooks
/// (`claude` → `.claude/settings.local.json`, root-guarded by `project_root`) stay allowed remotely.
/// Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity test still passes;
/// only the `match` arm in [`dispatch`] refuses to call through for the User-scope branch.
fn deny_remote_agent_hooks_user_scope(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 사용자 범위 에이전트 후킹을 설치할 수 없습니다: {name}"
    )))
}

/// Reports whether a remote `agent_hooks_install` call for `agent_name` must be denied by
/// [`deny_remote_agent_hooks_user_scope`] — true for `HookInstallScope::User` agents, false for
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

/// Answers `agent_pending_external_opens` with an explicit denial instead of dispatching to the real
/// handler. That command drains `AgentStore`'s single shared queue of CLI-triggered file-open
/// requests (`taide open <path> --wait`) via `mem::take` — whichever session calls it first empties
/// the queue for everyone, desktop included. Each request can carry a `waitMarker` that the external
/// CLI process blocks on until some frontend releases it (`agent_release_marker`); the frontend
/// registers that marker in a per-JS-realm `Map` (`agent-wait-marker-registry.ts`) keyed to whichever
/// window/tab drained the request and is released when that tab closes. A remote browser session has
/// its own separate realm, so a request it drains registers its release in a registry the desktop's
/// own tab-close handling never observes — leaving the external CLI process blocked until the app
/// exits (`cleanup_all_wait_markers` on shutdown) rather than until the relevant tab closes. Removing
/// `AgentExternalOpen` from `fanout_remote_events!` (`lib.rs`) stops a remote session from being
/// notified of *new* requests live; this arm closes the matching gap for requests it could otherwise
/// still discover by polling. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch
/// parity test still passes; only the `match` arm in [`dispatch`] refuses to call through.
fn deny_remote_agent_pending_external_opens(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 보류 중인 외부 열기 요청을 조회할 수 없습니다: {name}"
    )))
}

/// Answers `lsp_install` with an explicit denial instead of dispatching to the real handler — the
/// same rationale already applied to `plugin_install`/`plugin_uninstall`/`vsix_import_plugin`
/// ([`deny_remote_plugin_install`], [`deny_remote_vsix_import`]): the real handler downloads a
/// language server archive (often hundreds of megabytes) onto the desktop's local disk and spawns an
/// installer process there, an arbitrary-download-and-execute capability a remote session must not be
/// handed. Stays listed in [`IMPLEMENTED_JSON_COMMANDS`] so the bindings/dispatch parity test still
/// passes; only the `match` arm in [`dispatch`] refuses to call through.
fn deny_remote_lsp_install(name: &str) -> Value {
    err(AppError::Forbidden(format!(
        "원격 세션에서는 LSP 서버를 설치할 수 없습니다: {name}"
    )))
}

/// Every command name [`dispatch`] refuses unconditionally (the denial depends only on `name`, never
/// on `args` or app state — `agent_hooks_install`'s scope-conditional denial via
/// [`is_remote_denied_hook_install_scope`] is the one exception and stays a `match` arm of its own).
/// [`dispatch`] consults this table *before* its command `match` even runs, so this table's contents
/// — not a second, hand-written copy of the same name-to-handler mapping inside the `match` — are the
/// actual routing decision a remote request goes through. [`remote_denied_response`] below is what
/// both `dispatch` and this module's own tests call, so a test asserting "`remote_issue_link` is
/// denied" is asserting exactly the function `dispatch` runs for that name, not a same-shaped
/// stand-in that could silently drift from it — see `docs/acknowledge/2026-08-18-audit-t1-batch1-
/// contract.md` §1 T1-E.
type RemoteDeniedCommandEntry = (&'static str, fn(&str) -> Value);

const REMOTE_DENIED_COMMANDS: &[RemoteDeniedCommandEntry] = &[
    ("layout_move_tab_to_window", deny_remote_move_tab_to_window),
    ("file_flush_complete", deny_remote_flush_complete),
    ("plugin_install", deny_remote_plugin_install),
    ("plugin_uninstall", deny_remote_plugin_install),
    ("agent_cli_install", deny_remote_agent_cli),
    ("agent_cli_uninstall", deny_remote_agent_cli),
    ("agent_pending_external_opens", deny_remote_agent_pending_external_opens),
    ("lsp_install", deny_remote_lsp_install),
    ("system_open_path", deny_remote_system_open),
    ("system_reveal_path", deny_remote_system_open),
    ("system_open_in_browser", deny_remote_system_open),
    ("system_open_app_data_path", deny_remote_system_open),
    ("system_open_external_url", deny_remote_open_external_url),
    ("vsix_extract_themes", deny_remote_vsix_extract_themes),
    ("vsix_import_plugin", deny_remote_vsix_import),
    ("remote_issue_link", deny_remote_link_issue),
    ("remote_set_password", deny_remote_password_change),
    ("remote_clear_password", deny_remote_password_change),
    ("window_open_auxiliary", deny_remote_window_open),
    ("window_set_fullscreen", deny_remote_window_fullscreen),
];

/// Looks `name` up in [`REMOTE_DENIED_COMMANDS`], returning the denial [`dispatch`] must answer with
/// for an unconditionally-denied remote command, or `None` when `name` isn't one (either a real
/// handler exists for it, or it's unknown — `dispatch`'s own `match` decides which).
fn remote_denied_response(name: &str) -> Option<Value> {
    REMOTE_DENIED_COMMANDS
        .iter()
        .find(|(denied_name, _)| *denied_name == name)
        .map(|(_, deny)| deny(name))
}

macro_rules! arg {
    ($args:expr, $key:literal) => {
        from_arg(&$args, $key)?
    };
}

pub async fn dispatch(app: &AppHandle, name: &str, args: Value, channel_factory: ChannelFactory) -> Result<String, Value> {
    if let Some(denial) = remote_denied_response(name) {
        return Err(denial);
    }

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
        "search_cancel" => respond(search::search_cancel(app.state(), app.state(), arg!(args, "sessionId")).await),

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
                Err(deny_remote_agent_hooks_user_scope(name))
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
        "ide_start" => respond(ide::ide_start(app.clone(), app.state(), app.state()).await),
        "ide_stop" => respond(ide::ide_stop(app.clone(), app.state()).await),
        "ide_set_selection" => respond(ide::ide_set_selection(app.state(), arg!(args, "input")).await),
        "ide_clear_selection" => respond(ide::ide_clear_selection(app.state()).await),
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
        "ai_request_cancel" => respond(ai::ai_request_cancel(app.state(), arg!(args, "requestId")).await),

        "sync_status" => respond(sync::sync_status(app.state(), app.state()).await),
        "sync_connect" => respond(sync::sync_connect(app.state(), app.state(), arg!(args, "pat")).await),
        "sync_disconnect" => respond(sync::sync_disconnect(app.clone(), app.state(), app.state()).await),
        "sync_upload" => respond(sync::sync_upload(app.clone(), app.state(), app.state()).await),
        "sync_download" => respond(sync::sync_download(app.clone(), app.state(), app.state(), arg!(args, "force")).await),

        "remote_status" => respond(remote::remote_status(app.state()).await),
        "remote_start" => respond(remote::remote_start(app.clone(), app.state()).await),
        "remote_stop" => respond(remote::remote_stop(app.clone(), app.state()).await),
        "remote_revoke_sessions" => respond(remote::remote_revoke_sessions(app.state()).await),

        _ => Err(unknown_command(name)),
    }
}

fn response_bytes(response: Response) -> Result<Vec<u8>, Value> {
    match response.body().map_err(|error| err(AppError::Internal(error.to_string())))? {
        InvokeResponseBody::Raw(bytes) => Ok(bytes),
        InvokeResponseBody::Json(text) => Ok(text.into_bytes()),
    }
}

pub async fn dispatch_raw(app: &AppHandle, name: &str, args: Value) -> Result<Vec<u8>, Value> {
    match name {
        "file_read_raw" => {
            let path = from_arg::<String>(&args, "path")?;
            let response = domain::file::commands::file_read_raw(app.state::<AppState>(), path)
                .await
                .map_err(err)?;
            response_bytes(response)
        }
        _ => Err(unknown_command(name)),
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

    #[test]
    fn 알수없는_커맨드는_invalidargument로_보고된다() {
        let value = unknown_command("does_not_exist");
        assert_eq!(value["code"], serde_json::json!("InvalidArgument"));
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
        let value = deny_remote_link_issue("remote_issue_link");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    #[test]
    fn 원격_세션은_보조_창을_열_수_없다() {
        let value = deny_remote_window_open("window_open_auxiliary");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    #[test]
    fn 원격_세션은_호스트_브라우저를_열_수_없다() {
        let value = deny_remote_open_external_url("system_open_external_url");
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
            let value = deny_remote_system_open(name);
            assert_eq!(value["code"], serde_json::json!("Forbidden"), "{name} 은 거부되어야 한다");
        }
    }

    #[test]
    fn 원격_세션은_cli_shell_명령_설치_제거를_할_수_없다() {
        for name in ["agent_cli_install", "agent_cli_uninstall"] {
            let value = deny_remote_agent_cli(name);
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
        let value = deny_remote_agent_pending_external_opens("agent_pending_external_opens");
        assert_eq!(value["code"], serde_json::json!("Forbidden"));
    }

    #[test]
    fn 원격_세션은_lsp_서버를_설치할_수_없다() {
        let value = deny_remote_lsp_install("lsp_install");
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
}
