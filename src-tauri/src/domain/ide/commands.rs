use std::collections::HashSet;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::lockfile;
use super::server;
use super::service;
use super::store::{IdeSelectionSnapshot, IdeStore};
use super::types::{IdeDiagnostic, IdeDiffOutcome, IdeSelectionInput, IdeStatus, IDE_PORT_BIND_MAX_ATTEMPTS};
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::events::IdeStatusChanged;
use crate::ids::ProjectId;
use crate::state::AppState;

/// Reconciles a flipped `ide_integration_enabled` settings value against the live server — starts
/// it when the toggle turns on, stops it when it turns off, no-op when the value didn't change.
/// Registered into `settings::commands::SettingsToggleObservers` by `lib.rs`'s assembly so the
/// settings domain never calls into this one directly (audit R5#6, T1-I §1.4).
pub async fn apply_ide_integration_toggle(app: &AppHandle, was_enabled: bool, enabled: bool) {
    if was_enabled == enabled {
        return;
    }
    let ide = app.state::<IdeStore>();
    if enabled {
        if let Err(error) = ide_start(app.clone(), app.state::<AppState>(), ide).await {
            log::warn!("IDE 연동 시작 실패: {error}");
        }
    } else {
        stop_server(app, &ide);
    }
}

pub fn stop_server(app: &AppHandle, ide: &IdeStore) {
    let Some(shutdown) = ide.take_shutdown_state() else {
        return;
    };

    if let Some(handle) = &shutdown.server_handle {
        handle.abort();
    }
    for handle in &shutdown.connection_handles {
        handle.abort();
    }
    if let Some(dir) = &shutdown.dir {
        if let Err(error) = lockfile::remove_lockfile(dir, shutdown.port) {
            log::warn!("IDE lockfile 삭제 실패: {error}");
        }
    }
    for pending in shutdown.pending_diffs {
        let _ = pending.responder.send((IdeDiffOutcome::Rejected, None));
    }
    for pending in shutdown.pending_saves {
        let _ = pending.responder.send(false);
    }

    let _ = IdeStatusChanged {
        status: IdeStatus::default(),
    }
    .emit(app);
}

/// lockfile 의 `workspaceFolders` 는 Claude Code 가 후보 IDE 를 판정하는 근거다.
/// 프로젝트가 열리거나 닫히면 즉시 다시 써서 기동 시점 스냅샷으로 굳지 않게 한다.
pub fn refresh_lockfile(app: &AppHandle) {
    let ide = app.state::<IdeStore>();
    let Some((port, token, dir)) = ide.lockfile_context() else {
        return;
    };
    let workspace_folders = service::workspace_folders(&app.state::<AppState>().projects.read());
    let content = lockfile::build_lockfile_content(std::process::id(), workspace_folders, token);
    if let Err(error) = lockfile::write_lockfile_atomic(&dir, port, &content) {
        log::warn!("IDE lockfile 갱신 실패: {error}");
    }
}

pub fn reconcile_stale_pending(app: &AppHandle) {
    let ide = app.state::<IdeStore>();
    if !ide.is_running() {
        return;
    }
    let state = app.state::<AppState>();
    let open_projects: HashSet<ProjectId> = state.projects.read().keys().cloned().collect();
    ide.resolve_pending_for_missing_projects(&open_projects);
}

async fn bind_and_start(app: &AppHandle) -> AppResult<IdeStatus> {
    let state = app.state::<AppState>();
    let workspace_folders = service::workspace_folders(&state.projects.read());
    let token = service::generate_auth_token();
    let dir = lockfile::lockfile_dir()?;
    let current_pid = std::process::id();

    match lockfile::cleanup_stale_lockfiles(&dir, current_pid) {
        Ok(0) => {}
        Ok(removed) => log::info!("정지된 IDE lockfile {removed}개 정리"),
        Err(error) => log::warn!("정지된 IDE lockfile 정리 실패: {error}"),
    }

    let mut last_error: Option<std::io::Error> = None;
    for _ in 0..IDE_PORT_BIND_MAX_ATTEMPTS {
        let candidate_port = service::random_port();
        match tokio::net::TcpListener::bind(("127.0.0.1", candidate_port as u16)).await {
            Ok(listener) => {
                let content = lockfile::build_lockfile_content(current_pid, workspace_folders, token.clone());
                lockfile::write_lockfile_atomic(&dir, candidate_port, &content)?;

                let app_for_loop = app.clone();
                let token_for_loop = token.clone();
                let server_handle = tauri::async_runtime::spawn(async move {
                    server::accept_loop(app_for_loop, listener, token_for_loop).await;
                });

                let ide = app.state::<IdeStore>();
                let status = ide.mark_started(candidate_port, token, dir.clone(), server_handle);
                log::info!("IDE 서버 기동: port={candidate_port}, lockfile={}", dir.display());
                let _ = IdeStatusChanged { status }.emit(app);
                return Ok(status);
            }
            Err(error) => last_error = Some(error),
        }
    }

    let detail = last_error.map(|error| error.to_string()).unwrap_or_default();
    Err(AppError::localized(
        AppErrorKind::Internal,
        "error.ide.serverPortUnavailable",
        format!("could not find an IDE server port: {detail}"),
    )
    .with_arg("detail", &detail))
}

#[tauri::command]
#[specta::specta]
pub async fn ide_get_status(ide: State<'_, IdeStore>) -> AppResult<IdeStatus> {
    Ok(ide.status())
}

/// Starts the embedded IDE MCP server if it isn't already running — no longer a `#[tauri::command]`
/// (X1#13, `docs/acknowledge/2026-08-19-xa-wiring-cleanup-contract.md` §1.2): the only reachable path
/// to it was already `settings_update`'s `ideIntegrationEnabled` toggle
/// ([`apply_ide_integration_toggle`], registered as a settings toggle observer) and this module's
/// own boot-time auto-start (`lib.rs`), so the separate `ide_start` IPC surface duplicated that
/// without adding a distinct capability. Kept as a plain internal function since both of those
/// call sites still need it.
pub async fn ide_start(app: AppHandle, state: State<'_, AppState>, ide: State<'_, IdeStore>) -> AppResult<IdeStatus> {
    if ide.is_running() {
        return Ok(ide.status());
    }
    if !state.settings.read().ide_integration_enabled {
        return Err(AppError::InvalidArgument("IDE integration is disabled in settings".to_string()));
    }
    bind_and_start(&app).await
}

/// A remote session's `owner` (`IdeSelectionInput.owner == REMOTE_OWNER_LABEL`) is a deliberate
/// no-op: neither [`IdeStore::set_selection`] nor the `selection_changed` broadcast run, so a
/// browser-side editor selection can never overwrite what the local desktop's IDE MCP protocol
/// (`getCurrentSelection`/`getLatestSelection` in [`server`]) reports for the actual desktop editor
/// (R6#12).
#[tauri::command]
#[specta::specta]
pub async fn ide_set_selection(ide: State<'_, IdeStore>, input: IdeSelectionInput) -> AppResult<()> {
    if !IdeStore::is_desktop_owner(&input.owner) {
        return Ok(());
    }

    let selection = IdeSelectionSnapshot {
        project_id: input.project_id,
        path: input.path,
        text: input.text,
        start_line: input.start_line,
        start_character: input.start_character,
        end_line: input.end_line,
        end_character: input.end_character,
        is_empty: input.is_empty,
    };
    let notification = server::selection_changed_notification(&selection);
    ide.set_selection(selection);
    ide.broadcast(notification);
    Ok(())
}

/// See [`ide_set_selection`]'s doc comment for why a remote `owner` is a no-op here too.
#[tauri::command]
#[specta::specta]
pub async fn ide_clear_selection(ide: State<'_, IdeStore>, owner: String) -> AppResult<()> {
    if !IdeStore::is_desktop_owner(&owner) {
        return Ok(());
    }
    ide.clear_selection();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ide_publish_diagnostics(ide: State<'_, IdeStore>, project_id: ProjectId, items: Vec<IdeDiagnostic>) -> AppResult<()> {
    ide.publish_diagnostics(project_id, items);
    Ok(())
}

/// A `Saved` outcome persists through [`file::service::save_file_within_open_projects`](
/// crate::domain::file::service::save_file_within_open_projects) — the exact guarded sequence the
/// `file_save` command runs (root-guard resolution, atomic write, self-write mark, mirror clear),
/// under the same mutation guard, so this path can no longer bypass any of `file_save`'s
/// validation or bookkeeping (R6#2). A `Forbidden` error from it is the root-guard rejection and
/// is deliberately swallowed with a warning instead of failing the command: the diff target was
/// inside a project root when `openDiff` registered it (`server::tool_open_diff` resolves it up
/// front), so `Forbidden` here only means that project has since been closed — the MCP client
/// still receives its `Saved` resolution, matching the pre-guard behavior for that race.
#[tauri::command]
#[specta::specta]
pub async fn ide_resolve_diff(
    state: State<'_, AppState>,
    ide: State<'_, IdeStore>,
    request_id: String,
    outcome: IdeDiffOutcome,
    content: Option<String>,
) -> AppResult<()> {
    let pending = ide
        .take_pending_diff(&request_id)
        .ok_or_else(|| AppError::NotFound(format!("pending diff not found: {request_id}")))?;

    let resolved_content = match outcome {
        IdeDiffOutcome::Saved => {
            let content = content.ok_or_else(|| AppError::InvalidArgument("saved outcome requires content".to_string()))?;
            let _guard = state.begin_mutation().await;
            match crate::domain::file::service::save_file_within_open_projects(&state, &pending.new_path, &content) {
                Ok(()) => {}
                Err(AppError::Forbidden(message)) => {
                    log::warn!("IDE diff 저장 대상이 더 이상 프로젝트 루트 안에 있지 않습니다: {message}");
                }
                Err(error) => return Err(error),
            }
            Some(content)
        }
        IdeDiffOutcome::Rejected | IdeDiffOutcome::TabClosed => None,
    };

    let _ = pending.responder.send((outcome, resolved_content));
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ide_resolve_save(ide: State<'_, IdeStore>, request_id: String, saved: bool) -> AppResult<()> {
    let pending = ide
        .take_pending_save(&request_id)
        .ok_or_else(|| AppError::NotFound(format!("pending save not found: {request_id}")))?;
    let _ = pending.responder.send(saved);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ide_notify_at_mention(ide: State<'_, IdeStore>, path: String, line_start: u32, line_end: u32) -> AppResult<()> {
    ide.broadcast(server::at_mentioned_notification(&path, line_start, line_end));
    Ok(())
}
