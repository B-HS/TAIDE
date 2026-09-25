use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::domain::project::types::{Project, ProjectGroup, ProjectRef, ShellSlotTree, WindowChrome};
use crate::domain::settings::types::Settings;
use crate::ids::{ProjectId, ShellSlotId};
use crate::state::FlushScope;

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:opened")]
pub struct ProjectOpened {
    pub project: Project,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:closed")]
pub struct ProjectClosed {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:activated")]
pub struct ProjectActivated {
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:list-changed")]
pub struct ProjectListChanged {
    pub projects: Vec<ProjectRef>,
}

/// The sidebar's project groups changed — created, renamed, recolored, collapsed, reordered,
/// deleted, or had their membership rewritten (d-62 2c). Carries the whole list for the same reason
/// [`ProjectListChanged`] does: it is small, every window and remote session has to converge on the
/// identical order, and a delta would need ordering guarantees of its own.
///
/// `project_group_open` does **not** emit this — opening a group changes which projects are open,
/// not the groups themselves, so its progress shows up as [`ProjectListChanged`] per member.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:groups-changed")]
pub struct ProjectGroupsChanged {
    pub groups: Vec<ProjectGroup>,
}

/// `File > Clear Recent` ran: how many persisted project records it deleted, and how many it
/// deliberately kept because their hot-exit mirrors still hold unsaved drafts (audit wave 2 #9).
///
/// Those counts already come back to whoever invoked `project_forget_recent`, but the action's real
/// trigger is the native `File > Clear Recent` item, which `lib.rs`'s `dispatch_menu_action` runs in
/// Rust so the menu keeps working with **zero windows open** — leaving no IPC caller to hand the
/// outcome to. Without this event the "kept N projects that still have unsaved drafts" notice was
/// unreachable from the only path that actually clears the list, and a recent list that refused to
/// empty read as a command that had simply failed.
///
/// Emitted unconditionally, alongside the [`ProjectListChanged`] the same call emits: the event
/// means "clear recent ran", and it is the frontend that decides a zero skip count is worth no
/// notice.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "project:recent-cleared")]
pub struct ProjectRecentCleared {
    pub removed: u32,
    pub skipped_with_drafts: u32,
}

/// The main window's shell-slot arrangement changed — a slot was split, replaced, closed, or simply
/// focused (d-62). Carries the whole tree rather than a delta for the same reason
/// [`ProjectListChanged`] carries the whole list: the tree is small, every window and remote session
/// has to converge on the identical shape, and a delta would need its own ordering guarantees.
/// `tree` is `None` only when no project is open at all.
///
/// Emitted from the same mutation that changed the tree, alongside whatever else that mutation
/// emits (`ProjectClosed`, `ProjectActivated`) — contract §0.1 S-1's atomicity requirement.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "session:shell-slots-changed")]
pub struct SessionShellSlotsChanged {
    pub tree: Option<ShellSlotTree>,
    pub focused: Option<ShellSlotId>,
}

/// Window-level chrome (Zen, sidebar icon rail) changed — the axes contract §0.1 S-6 moved off the
/// per-project `ProjectLayout::shell_view`, which is why this is its own event and not a
/// `LayoutChanged`: there is no project id to scope it to.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "session:window-chrome-changed")]
pub struct WindowChromeChanged {
    pub chrome: WindowChrome,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "layout:changed")]
pub struct LayoutChanged {
    pub project_id: ProjectId,
    pub revision: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "theme:changed")]
pub struct ThemeChanged {
    pub theme_id: String,
}

/// Emitted after every settings write reaches the shared reapply path
/// (`settings::commands::apply_and_broadcast`) — a `settings_update` patch, a `sync_download`, or an
/// `app_file_write` on the `Settings` target. Carries the full sanitized `Settings` so a listener
/// can update immediately without a round-trip, though the frontend's own convention is to
/// invalidate its cached `SETTINGS.CURRENT` query and let TanStack Query refetch
/// (`docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.3).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "settings:changed")]
pub struct SettingsChanged {
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "fs:changed")]
pub struct FsChanged {
    pub project_id: ProjectId,
    pub change: crate::domain::file::types::FsChange,
}

/// The file watcher lost events instead of delivering them — `notify` raised its rescan flag after
/// a backend queue overflow (`infra::watcher::WatchNotification::RescanRequired`). Nothing can be
/// said about *which* paths changed, so every consumer that mirrors the project tree
/// (tree, quick-open index, open files) must reload rather than patch. Throttled per watch by
/// `infra::watcher::RESCAN_MIN_INTERVAL_MS`, and never emitted for an ordinary change — a
/// [`FsChanged`] consumer needs no special case for it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "fs:rescan-required")]
pub struct FsRescanRequired {
    pub project_id: ProjectId,
}

/// A pty session was created and registered — the spawn-side counterpart to [`TerminalExited`].
///
/// Exists because the terminal roster (`TERMINAL.SESSIONS`) is a per-window TanStack Query cache
/// with `staleTime: Infinity`, and the only thing that used to record a fresh session was the
/// spawning window writing into *its own* cache. A window that never saw that write — the other
/// window a terminal tab was dragged to — decided the still-running session was dead and spawned a
/// replacement over it, orphaning the original pty. Broadcasting the same fact app-wide lets every
/// window's roster converge, exactly as `terminal:exited` already does for the other direction
/// (`docs/features/terminal.md` §3.1: every event that changes the roster writes to the cache).
///
/// Emitted after the session is in `TerminalStore`, so a listener that reacts by querying the
/// store always finds it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "terminal:spawned")]
pub struct TerminalSpawned {
    pub session_id: String,
    pub project_id: ProjectId,
    pub cwd: String,
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "terminal:exited")]
pub struct TerminalExited {
    pub session_id: String,
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "terminal:cwd-changed")]
pub struct TerminalCwdChanged {
    pub session_id: String,
    pub cwd: String,
}

/// A shell command that ran in a pty session ended, with how long it ran for.
///
/// Detected on the pty reader thread (`domain::terminal::commands::report_command_marker`) rather
/// than in the frontend's own OSC 133 tracker, because the notification this feeds exists precisely
/// for the case the frontend cannot see: a long command in a terminal tab the user switched away
/// from, whose `TerminalSession` — and with it the xterm instance and its tracker — is unmounted
/// while it runs. `duration_ms` is measured between the shell's `133;C` and `133;D`, so it is real
/// elapsed time no matter who was watching; a command whose start was never seen reports nothing at
/// all rather than a guess.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "terminal:command-finished")]
pub struct TerminalCommandFinished {
    pub session_id: String,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "git:status-changed")]
pub struct GitStatusChanged {
    pub project_id: ProjectId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "git:refs-changed")]
pub struct GitRefsChanged {
    pub project_id: ProjectId,
}

/// `generation` (R7#1) increases only when `crate::domain::lsp::commands::handle_process_exit`'s
/// automatic crash-restart path successfully respawns the process. A `status: Crashed` event whose
/// `generation` is higher than the last one the renderer saw means "the process behind this
/// `session_id` was silently replaced — discard your old LSP client state, re-run `initialize` over
/// `lsp_send`, then call `domain::lsp::commands::lsp_confirm_reinitialize` with this same
/// `generation`" (only that confirmation flips `status` back to `Running`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "lsp:session-status-changed")]
pub struct LspSessionStatusChanged {
    pub session_id: String,
    pub status: crate::domain::lsp::types::LspSessionStatus,
    pub last_error: Option<String>,
    pub generation: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "lsp:install-progress")]
pub struct LspInstallProgress {
    pub server_id: crate::domain::lsp::types::LspServerId,
    pub phase: crate::domain::lsp::types::LspInstallPhase,
    pub received_bytes: f64,
    pub total_bytes: Option<f64>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "agent:state-changed")]
pub struct AgentStateChanged {
    pub project_id: ProjectId,
    pub agents: Vec<crate::domain::agent::types::DetectedAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "agent:external-open")]
pub struct AgentExternalOpen {
    pub request: crate::domain::agent::types::ExternalOpenRequest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:status-changed")]
pub struct IdeStatusChanged {
    pub status: crate::domain::ide::types::IdeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:diff-requested")]
pub struct IdeDiffRequested {
    pub request_id: String,
    pub project_id: ProjectId,
    pub old_path: String,
    pub new_path: String,
    pub new_contents: String,
    pub tab_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:save-requested")]
pub struct IdeSaveRequested {
    pub request_id: String,
    pub project_id: ProjectId,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "ide:close-tab-requested")]
pub struct IdeCloseTabRequested {
    pub tab_name: String,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "sync:state-changed")]
pub struct SyncStateChanged {
    pub status: crate::domain::sync::types::SyncStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "remote:state-changed")]
pub struct RemoteStateChanged {
    pub status: crate::domain::remote::types::RemoteStatus,
}

/// Asks the frontend to flush dirty editor models to the hot-exit mirror before a teardown that
/// would otherwise lose the last debounce window of typing, and to confirm with
/// `domain::file::commands::file_flush_complete` once it has.
///
/// `scope` says which models: the app is exiting ([`FlushScope::All`], the original and still the
/// only one that ends in `AppHandle::exit`), one auxiliary window is closing
/// ([`FlushScope::Window`]), or one project is closing ([`FlushScope::Project`]). Broadcast
/// app-wide in every case — a project spans windows, so each window decides for itself what the
/// scope covers of its own models and then confirms with that same scope.
///
/// `timeout_ms` mirrors `HOT_EXIT_FLUSH_TIMEOUT_MS` so the frontend never needs its own copy of
/// that constant. The backend proceeds without a confirmation once it elapses, so a listener that
/// takes longer loses the flush, not the teardown.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "app:hot-exit-flush-requested")]
pub struct HotExitFlushRequested {
    pub timeout_ms: f64,
    pub scope: FlushScope,
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;
    use crate::domain::lsp::types::{LspInstallPhase, LspServerId};

    fn project_id() -> ProjectId {
        ProjectId::from("prj-events".to_string())
    }

    /// `serde_json::Map` is a `BTreeMap` in this build (no `preserve_order` feature), so the keys
    /// come back sorted rather than in declaration order — the assertions below compare sorted
    /// name sets, which is what "the payload carries exactly these camelCase keys" needs anyway.
    fn field_names(value: &serde_json::Value) -> Vec<String> {
        value.as_object().expect("객체여야 합니다").keys().cloned().collect()
    }

    /// Every payload in this file must carry `#[serde(rename_all = "camelCase")]`: `specta` reads
    /// the same attribute to generate `bindings.ts`, so a struct that loses it starts emitting
    /// `snake_case` keys the frontend's generated type says are `camelCase` — a drift no compiler
    /// on either side can see. `lib.rs` already pins the event *names* and the declared *set*; this
    /// pins the field-name convention those payloads are read with.
    #[test]
    fn 모든_이벤트_페이로드는_camel_case_직렬화를_선언한다() {
        let pattern = Regex::new(
            r#"(?m)^#\[derive\([^\]]*\bEvent\b[^\]]*\)\]\n(?<attrs>(?:#\[[^\n]*\]\n)*)pub struct (?<name>[A-Za-z][A-Za-z0-9]*)"#,
        )
        .expect("유효한 정규식");

        let declared: Vec<(String, bool)> = pattern
            .captures_iter(include_str!("events.rs"))
            .map(|capture| {
                (
                    capture["name"].to_string(),
                    capture["attrs"].contains(r#"#[serde(rename_all = "camelCase")]"#),
                )
            })
            .collect();

        assert!(
            declared.len() >= 24,
            "이벤트 페이로드 스캔이 구조체를 놓쳤습니다 (찾은 수: {}) — events.rs 의 어트리뷰트 배치가 바뀌었는지 확인하십시오",
            declared.len()
        );

        let missing: Vec<&String> = declared.iter().filter(|(_, has)| !has).map(|(name, _)| name).collect();
        assert!(
            missing.is_empty(),
            "camelCase 직렬화를 선언하지 않은 이벤트 페이로드가 있습니다 (bindings.ts 와 런타임 키가 어긋납니다):\n{missing:#?}"
        );
    }

    #[test]
    fn 단일_필드_이벤트는_camel_case_키로_직렬화된다() {
        let value = serde_json::to_value(ProjectClosed { project_id: project_id() }).expect("직렬화");

        assert_eq!(field_names(&value), vec!["projectId"]);
        assert_eq!(value["projectId"], "prj-events");
    }

    #[test]
    fn 레이아웃_변경은_프로젝트와_리비전만_싣는다() {
        let value = serde_json::to_value(LayoutChanged {
            project_id: project_id(),
            revision: 7,
        })
        .expect("직렬화");

        assert_eq!(field_names(&value), vec!["projectId", "revision"]);
        assert_eq!(value["revision"], 7);
    }

    /// The batch 4 F-1 payload (`docs/acknowledge/2026-09-04-usability-batch4-contract.md` §3.6) —
    /// the native task-completion notification reads all four fields, and `cwd`/`exit_code` are
    /// `Option`, so both the present and absent shapes are pinned.
    #[test]
    fn 명령_종료_이벤트는_네_필드를_camel_case_로_싣는다() {
        let value = serde_json::to_value(TerminalCommandFinished {
            session_id: "pty-1".to_string(),
            cwd: Some("/repo".to_string()),
            exit_code: Some(130),
            duration_ms: 12_345,
        })
        .expect("직렬화");

        assert_eq!(field_names(&value), vec!["cwd", "durationMs", "exitCode", "sessionId"]);
        assert_eq!(value["sessionId"], "pty-1");
        assert_eq!(value["cwd"], "/repo");
        assert_eq!(value["exitCode"], 130);
        assert_eq!(value["durationMs"], 12_345);
    }

    #[test]
    fn 명령_종료_이벤트의_옵션_필드는_null_로_직렬화된다() {
        let value = serde_json::to_value(TerminalCommandFinished {
            session_id: "pty-2".to_string(),
            cwd: None,
            exit_code: None,
            duration_ms: 0,
        })
        .expect("직렬화");

        assert!(value["cwd"].is_null(), "cwd 는 생략이 아니라 null 이어야 합니다");
        assert!(value["exitCode"].is_null(), "exitCode 는 생략이 아니라 null 이어야 합니다");
    }

    #[test]
    fn 설치_진행_이벤트의_message_는_없어도_역직렬화된다() {
        let event: LspInstallProgress =
            serde_json::from_str(r#"{"serverId":"rust-analyzer","phase":"downloading","receivedBytes":1024.0,"totalBytes":null}"#)
                .expect("message 없는 페이로드도 읽혀야 합니다");

        assert_eq!(event.server_id, LspServerId("rust-analyzer".to_string()));
        assert_eq!(event.phase, LspInstallPhase::Downloading);
        assert_eq!(event.received_bytes, 1024.0);
        assert_eq!(event.total_bytes, None);
        assert_eq!(event.message, None);
    }

    #[test]
    fn 탭_닫기_요청은_왕복_직렬화에서_필드를_잃지_않는다() {
        let original = IdeCloseTabRequested {
            tab_name: "main.rs".to_string(),
            request_id: Some("req-1".to_string()),
        };
        let value = serde_json::to_value(&original).expect("직렬화");

        assert_eq!(field_names(&value), vec!["requestId", "tabName"]);

        let restored: IdeCloseTabRequested = serde_json::from_value(value).expect("역직렬화");
        assert_eq!(restored.tab_name, original.tab_name);
        assert_eq!(restored.request_id, original.request_id);
    }

    /// The roster upsert on the frontend reads all four fields (`upsertTerminalSession`), so the
    /// exact camelCase key set is the contract — a rename here would silently produce sessions with
    /// `undefined` cwd/shell in every window's roster.
    #[test]
    fn 터미널_스폰_이벤트는_세션_프로젝트_cwd_셸을_camel_case_로_싣는다() {
        let value = serde_json::to_value(TerminalSpawned {
            session_id: "pty-1".to_string(),
            project_id: project_id(),
            cwd: "/repo".to_string(),
            shell: "/bin/zsh".to_string(),
        })
        .expect("직렬화");

        assert_eq!(field_names(&value), vec!["cwd", "projectId", "sessionId", "shell"]);
        assert_eq!(value["sessionId"], "pty-1");
        assert_eq!(value["projectId"], "prj-events");
        assert_eq!(value["cwd"], "/repo");
        assert_eq!(value["shell"], "/bin/zsh");
    }

    #[test]
    fn 핫엑시트_플러시_요청은_밀리초와_스코프를_싣는다() {
        let value = serde_json::to_value(HotExitFlushRequested {
            timeout_ms: 2_000.0,
            scope: FlushScope::All,
        })
        .expect("직렬화");

        assert_eq!(field_names(&value), vec!["scope", "timeoutMs"]);
        assert_eq!(value["timeoutMs"], 2_000.0);
    }

    /// The scope is the only thing that tells a window whether a flush request is about its own
    /// close, a project it may not even render, or the whole app — so its wire shape is the
    /// contract `bindings.ts` hands the frontend: `"all" | { window } | { project }`.
    #[test]
    fn 플러시_스코프는_all_window_project_유니온으로_직렬화된다() {
        assert_eq!(serde_json::to_value(FlushScope::All).expect("직렬화"), serde_json::json!("all"));
        assert_eq!(
            serde_json::to_value(FlushScope::Window("editor-1".to_string())).expect("직렬화"),
            serde_json::json!({ "window": "editor-1" })
        );
        assert_eq!(
            serde_json::to_value(FlushScope::Project(project_id())).expect("직렬화"),
            serde_json::json!({ "project": "prj-events" })
        );
    }

    #[test]
    fn 플러시_스코프는_왕복_직렬화에서_같은_값으로_돌아온다() {
        for scope in [
            FlushScope::All,
            FlushScope::Window("editor-2".to_string()),
            FlushScope::Project(project_id()),
        ] {
            let value = serde_json::to_value(&scope).expect("직렬화");
            let restored: FlushScope = serde_json::from_value(value).expect("역직렬화");
            assert_eq!(restored, scope);
        }
    }
}
