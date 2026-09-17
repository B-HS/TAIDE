use std::collections::{HashMap, HashSet};
use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::capability::ProjectCapabilities;
use super::groups;
use super::service;
use super::types::{
    ForgetRecentOutcome, OpenProjectInSlotRequest, Project, ProjectDisplayPatch, ProjectGroup, ProjectGroupOpenResult, ProjectRef,
    SessionShellState, SessionState, WindowChrome, WindowChromePatch,
};
use crate::constants;
use crate::domain::file::types::{FsChange, FsChangeKind};
use crate::error::{AppError, AppResult};
use crate::events::{
    FsChanged, GitStatusChanged, HotExitFlushRequested, ProjectActivated, ProjectClosed, ProjectGroupsChanged, ProjectListChanged,
    ProjectOpened, ProjectRecentCleared, SessionShellSlotsChanged, WindowChromeChanged,
};
use crate::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use crate::infra::perf::{self, SpanSlot};
use crate::state::{AppState, FlushScope};

fn emit_list_changed(app: &AppHandle, state: &AppState) {
    let projects = service::list_projects(&state.session.read());
    let _ = ProjectListChanged { projects }.emit(app);
}

/// Publishes the current slot arrangement to every window and remote session. The session lock is
/// released before the emit (the payload is built in its own scope) because a `listen_any` handler
/// runs inline on the emitting thread — `lib.rs`'s menu refresh is only safe for the same reason.
fn emit_shell_slots_changed(app: &AppHandle, state: &AppState) {
    let payload = {
        let session = state.session.read();
        SessionShellSlotsChanged {
            tree: session.shell_slots.clone(),
            focused: session.focused_shell_slot.clone(),
        }
    };
    let _ = payload.emit(app);
}

/// Publishes the sidebar's project groups to every window and remote session. Releases the session
/// lock before the emit for the same reason [`emit_shell_slots_changed`] does.
fn emit_groups_changed(app: &AppHandle, state: &AppState) {
    let groups = service::list_groups(&state.session.read());
    let _ = ProjectGroupsChanged { groups }.emit(app);
}

#[tauri::command]
#[specta::specta]
pub async fn project_list(state: State<'_, AppState>) -> AppResult<Vec<ProjectRef>> {
    Ok(service::list_projects(&state.session.read()))
}

/// Every persisted project record on this desktop, most-recently-opened first — unlike
/// `project_list` (the currently-open session only), this walks the full on-disk history so the
/// Welcome screen can offer projects the user closed earlier. Read-only: it never writes to disk
/// (a corrupted `project.json` is skipped, not backed up to `.bak` — see
/// `service::try_load_project_readonly`), so no `begin_mutation` guard is needed. This is *not*
/// the same shape as `project_get`'s read-only-ness — `project_get` never touches the filesystem
/// at all (it reads `state.projects`, an in-memory map), while this command does a full disk scan
/// through a dedicated read-only path. Deliberately **not** remote-reachable — see
/// `RemoteDenialPolicy::LocalProjectHistoryExposure` in `domain/remote/dispatch.rs`.
#[tauri::command]
#[specta::specta]
pub async fn project_list_recent(state: State<'_, AppState>) -> AppResult<Vec<Project>> {
    service::list_recent_projects(&state.paths)
}

/// Forgets every project the user is not currently working in: the persisted `projects/<id>/`
/// record of each closed project is deleted, so `project_list_recent` stops offering it. The
/// backing action for `File > Clear Recent` and the sidebar's equivalent.
///
/// Open projects are deliberately kept — their record is live state (layout, display, id reuse),
/// not history; see `service::forget_recent_projects`. Emits `ProjectListChanged` so every window
/// re-reads its project queries — and so `lib.rs`'s listener rebuilds the native `File > Open
/// Recent` menu that listed them.
///
/// A closed project whose hot-exit mirrors still hold unsaved work is kept as well and reported in
/// `ForgetRecentOutcome::skipped_with_drafts`, which is why this returns the whole outcome instead
/// of the removed count it used to: the caller turns a non-zero count into the notice that explains
/// why the recent list did not empty (`service::forget_recent_projects`).
///
/// `ProjectGroupsChanged` follows **only when a forgotten project was actually listed under a
/// group** (`ForgetRecentOutcome::groups_changed`, contract §3 B-1): the groups are untouched in the
/// ordinary clear-recent call, and telling every window to re-read a group list that did not change
/// is both noise and a contradiction of `docs/ipc-contract.md`, which documents this event as the
/// membership-cleanup half of the call.
///
/// Deliberately **not** remote-reachable, for the same reason as `project_list_recent`: the recent
/// list is local-desktop history (`RemoteDenialPolicy::LocalProjectHistoryExposure`), and a remote
/// session that cannot read it has no business destroying it either.
#[tauri::command]
#[specta::specta]
pub async fn project_forget_recent(app: AppHandle, state: State<'_, AppState>) -> AppResult<ForgetRecentOutcome> {
    let outcome = {
        let _guard = state.begin_mutation().await;
        let open_ids: HashSet<ProjectId> = state.projects.read().keys().cloned().collect();
        let mut session = state.session.read().clone();
        let outcome = service::forget_recent_projects(&state.paths, &mut session, &open_ids)?;
        *state.session.write() = session;
        outcome
    };

    emit_list_changed(&app, &state);
    if outcome.groups_changed {
        emit_groups_changed(&app, &state);
    }
    let _ = ProjectRecentCleared {
        removed: outcome.removed,
        skipped_with_drafts: outcome.skipped_with_drafts,
    }
    .emit(&app);

    Ok(outcome)
}

#[tauri::command]
#[specta::specta]
pub async fn project_get(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Project> {
    service::get_project(&state.projects.read(), &project_id)
}

#[tauri::command]
#[specta::specta]
pub async fn project_get_active(state: State<'_, AppState>) -> AppResult<Option<ProjectId>> {
    Ok(state.session.read().active_project.clone())
}

/// `service::open_project` sets `session.active_project = Some(project.id)` on **every** path
/// (a fresh open, id-reuse from history, and the `already_open` re-open of a project already in
/// this session) — this command must fan that activation out to every window exactly like
/// [`project_activate`] does, or a caller other than the FE's own `useOpenProject`/
/// `useOpenFolderDialog` mutation (whose `onSuccess` invalidates `QUERY_KEY.PROJECT.ALL` itself,
/// masking the gap for that one call site) sees `QUERY_KEY.PROJECT.ACTIVE` go stale: the remote
/// dispatch path (`domain::remote::dispatch`) calls this exact function, and any future direct
/// caller would hit the same gap. Emitted unconditionally (not only inside the `!already_open`
/// branch below, which gates the *first-open-only* `ProjectOpened`/`ProjectListChanged`/capability
/// attach) because activation itself is unconditional. See
/// `docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md` §3 (item c) for the fanout-gap
/// diagnosis this closes.
#[tauri::command]
#[specta::specta]
pub async fn project_open(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<service::ProjectOpenResult> {
    let _span = perf::span(SpanSlot::ProjectOpen);

    let result = {
        let _guard = state.begin_mutation().await;
        let mut session = state.session.read().clone();
        let mut projects = state.projects.read().clone();

        let result = service::open_project(&state.paths, &mut session, &mut projects, Path::new(&path), true, |canonical| {
            app.state::<ProjectCapabilities>().detected_kinds(canonical)
        })?;

        *state.session.write() = session;
        *state.projects.write() = projects;
        result
    };

    if !result.already_open {
        if let Err(error) = attach_project_capabilities(&app, &result.project).await {
            if let Err(rollback) = project_close(app.clone(), state.clone(), result.project.id.clone()).await {
                log::warn!(
                    "capability attach 실패 후 프로젝트 되돌리기도 실패했습니다 (projectId={}): {rollback}",
                    result.project.id
                );
            }
            return Err(error);
        }

        let _ = ProjectOpened {
            project: result.project.clone(),
        }
        .emit(&app);
        emit_list_changed(&app, &state);
    }

    let _ = ProjectActivated {
        project_id: Some(result.project.id.clone()),
    }
    .emit(&app);
    emit_shell_slots_changed(&app, &state);

    Ok(result)
}

/// Opens a project **into a named shell slot** — the split half of d-62. Either `path` (a folder
/// that may not be open yet) or `project_id` (a project already in the sidebar) names what to place;
/// `edge` decides whether the target slot is split or its project simply replaced.
///
/// The whole slot mutation — duplicate rejection, the split, the focus move, the session write and
/// the events — happens under one `begin_mutation` acquisition (contract §0.1 S-1), and validation
/// runs *before* anything is opened so a rejected drop cannot strand a half-opened project. The one
/// step deliberately outside the guard is the capability attach for a project this call opened for
/// the first time, for the reason [`attach_project_capabilities`] documents at length; a failure
/// there unwinds through [`project_close`], which removes the project *and* the slot it was just
/// placed in.
#[tauri::command]
#[specta::specta]
pub async fn project_open_in_slot(
    app: AppHandle,
    state: State<'_, AppState>,
    request: OpenProjectInSlotRequest,
) -> AppResult<SessionShellState> {
    let _span = perf::span(SpanSlot::ProjectOpen);

    let opened = {
        let _guard = state.begin_mutation().await;
        let mut session = state.session.read().clone();
        let mut projects = state.projects.read().clone();

        let existing = match (&request.path, &request.project_id) {
            (Some(path), None) => service::find_open_project_by_root(&projects, Path::new(path)),
            (None, Some(project_id)) => {
                service::get_project(&projects, project_id)?;
                Some(project_id.clone())
            }
            _ => {
                return Err(AppError::InvalidArgument(
                    "project_open_in_slot needs exactly one of path or projectId".to_string(),
                ));
            }
        };

        service::ensure_slot_placement_allowed(&session, existing.as_ref(), &request.target_slot, request.edge)?;

        let opened = match (existing, &request.path) {
            (Some(project_id), _) => {
                let project = service::get_project(&projects, &project_id)?;
                service::ProjectOpenResult {
                    project,
                    already_open: true,
                }
            }
            (None, Some(path)) => service::open_project(&state.paths, &mut session, &mut projects, Path::new(path), false, |canonical| {
                app.state::<ProjectCapabilities>().detected_kinds(canonical)
            })?,
            (None, None) => return Err(AppError::Internal("project_open_in_slot resolved no project".to_string())),
        };

        service::place_project_in_slot(
            &state.paths,
            &mut session,
            &mut projects,
            &opened.project.id,
            &request.target_slot,
            request.edge,
        )?;

        *state.session.write() = session;
        *state.projects.write() = projects;
        opened
    };

    if !opened.already_open {
        if let Err(error) = attach_project_capabilities(&app, &opened.project).await {
            if let Err(rollback) = project_close(app.clone(), state.clone(), opened.project.id.clone()).await {
                log::warn!(
                    "capability attach 실패 후 슬롯 프로젝트 되돌리기도 실패했습니다 (projectId={}): {rollback}",
                    opened.project.id
                );
            }
            return Err(error);
        }

        let _ = ProjectOpened {
            project: opened.project.clone(),
        }
        .emit(&app);
        emit_list_changed(&app, &state);
    }

    let _ = ProjectActivated {
        project_id: Some(opened.project.id.clone()),
    }
    .emit(&app);
    emit_shell_slots_changed(&app, &state);

    Ok(service::shell_state(&state.session.read()))
}

/// The current slot arrangement and window chrome, for a window that just mounted. Both halves
/// otherwise only arrive as events (`SessionShellSlotsChanged`/`WindowChromeChanged`), which fire at
/// transitions — the same gap `project_get_active` was added to close for `project:activated`.
#[tauri::command]
#[specta::specta]
pub async fn session_get_shell_state(state: State<'_, AppState>) -> AppResult<SessionShellState> {
    Ok(service::shell_state(&state.session.read()))
}

/// Moves the window's focus to one slot, which also makes that slot's project the active one — see
/// `service::focus_shell_slot`. The frontend tracks focus itself from DOM events (contract §0.1 U-7)
/// and calls this to persist it, so this is the write half of that pair, not its source of truth.
#[tauri::command]
#[specta::specta]
pub async fn session_focus_shell_slot(app: AppHandle, state: State<'_, AppState>, slot_id: ShellSlotId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::focus_shell_slot(&state.paths, &mut session, &mut projects, &slot_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    *state.projects.write() = projects;
    drop(_guard);

    let _ = ProjectActivated {
        project_id: active_project,
    }
    .emit(&app);
    emit_shell_slots_changed(&app, &state);

    Ok(())
}

/// Persists one split node's child percentages after a drag. `path` addresses the node by child
/// index from the root because a slot split has no id of its own — see
/// `shell_slots::set_sizes`. The frontend debounces these the way `pane-resize-commit.ts` already
/// debounces `layout_resize`, so this is not called per pointer move.
#[tauri::command]
#[specta::specta]
pub async fn session_set_shell_slot_sizes(app: AppHandle, state: State<'_, AppState>, path: Vec<u32>, sizes: Vec<f32>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_shell_slot_sizes(&state.paths, &mut session, &path, sizes)?;

    *state.session.write() = session;
    drop(_guard);

    emit_shell_slots_changed(&app, &state);

    Ok(())
}

/// Closes one shell slot while leaving its project open — the slot header's own close button. The
/// last remaining slot is refused: a window with projects open always shows at least one of them.
#[tauri::command]
#[specta::specta]
pub async fn shell_slot_close(app: AppHandle, state: State<'_, AppState>, slot_id: ShellSlotId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::close_shell_slot(&state.paths, &mut session, &slot_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    drop(_guard);

    let _ = ProjectActivated {
        project_id: active_project,
    }
    .emit(&app);
    emit_shell_slots_changed(&app, &state);

    Ok(())
}

/// Sets the window-level chrome axes (Zen, sidebar icon rail) that contract §0.1 S-6 moved off the
/// per-project layout. Patch semantics match `layout_set_shell_view`'s: an omitted axis is left
/// alone.
#[tauri::command]
#[specta::specta]
pub async fn session_set_window_chrome(app: AppHandle, state: State<'_, AppState>, patch: WindowChromePatch) -> AppResult<WindowChrome> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    let chrome = service::set_window_chrome(&state.paths, &mut session, &patch)?;

    *state.session.write() = session;
    drop(_guard);

    let _ = WindowChromeChanged { chrome }.emit(&app);

    Ok(chrome)
}

/// Runs the capability attach walk for a freshly opened project **without holding
/// `AppState::begin_mutation` across it** — the "build outside the guard, register inside it" split
/// `restore_project_watchers` established for boot, promoted to the capability trait itself
/// (`ProjectCapability::build_attachment` → `ProjectAttachment`) and applied to `project_open`.
///
/// **Why.** The guard is a single app-wide `tokio::sync::Mutex` (`AppState::begin_mutation`), so
/// while `project_open` held it across the whole one-shot attach walk it used to run, *every*
/// mutation in the app — `file_save`,
/// every `git_*`, `layout_*`, `tree_toggle`, the periodic dirty-layout flush and dirty-buffer
/// mirror — queued behind the new project's `notify-debouncer-full` `FileIdMap` walk. That walk
/// stats every entry below the root with no ignore list applied (`node_modules`/`target` included),
/// which on a large working tree is seconds, not milliseconds. `architecture.md` §2.1's "no IO
/// under a lock" rule says the same thing in the abstract; this is the one place `project_open`
/// broke it hardest.
///
/// **Ordering is unchanged.** Both phases walk the registration list forward and the commit replays
/// the build's vector position for position, so each capability still attaches after every
/// capability registered before it — the order `lib.rs`'s `project_capabilities` pins and
/// `project_close`'s reap mirrors (`architecture.md` §3·§6.3).
///
/// **No events are lost by the deferral.** A watcher handle is already subscribed when
/// `build_watcher_handle`/`build_git_watcher_handle` returns, so changes landing between the build
/// and its registration are still fanned out; registration is only what gives `project_close` a
/// handle to drop. And this command still awaits the whole attach before it emits `ProjectOpened`/
/// `ProjectActivated` or returns, so no frontend query for this project can run before its watchers
/// are live — the deferral moved the *lock*, not the attach.
///
/// **Re-validation under the re-acquired guard.** `state.projects` only shrinks via
/// `project_close`, which runs its whole body under one guard acquisition, so a close landing
/// during the unguarded build is fully visible the instant this re-acquires. When it is, the built
/// attachments are dropped instead of committed — dropping a watcher handle stops its debouncer, so
/// nothing is left behind for a `detach` that already ran.
///
/// **Attach-completion git refresh (d-25 boot-gap correction, applied to this path).** `GIT.PROJECT`
/// is one of the two frontend caches that only ever refresh on an event
/// (`entities/git/git.query.ts`; see `restore_project_watchers`'s doc for the full pair), so a
/// `.git` change landing before the watcher subscribes has nowhere to go on its own. One
/// `GitStatusChanged` after the watcher registers closes that, and costs nothing in the normal
/// case: `ProjectOpened`/`ProjectActivated` have not been emitted yet at that point, so no window
/// has mounted a query under this project's key and the invalidation matches nothing. Where it
/// does real work is the one window the deferral genuinely widened — a *second* `project_open` for
/// the same path landing while this attach runs takes the `already_open` branch, so it emits
/// `ProjectActivated` and returns without waiting for these watchers; this emit is what brings that
/// caller's git query back in sync once they are live. The
/// `FsChanged` half of the boot correction is deliberately not replicated — on this path the
/// layout (and therefore any open File tab) becomes visible to the frontend only *after* this
/// function returns, so there is no already-mounted `FILE.CONTENT` query to correct.
///
/// **A build that never lands is a failed open, not a silent one.** `spawn_blocking` reports a
/// panicking build phase as a `JoinError`, and the split gives that failure somewhere to be lost
/// that the pre-split synchronous walk did not have: nothing would be committed, yet `project_open`
/// had already published the project into `state.projects`/`state.session`, so the caller would be
/// told the open succeeded while not one of the registered capabilities — `LayoutCapability`, the
/// only writer of `state.layouts`, included — had attached. It is reported as an error instead, and
/// `project_open` unwinds the half-open project through [`project_close`] (the same reap walk a real
/// close runs) before returning it. A project closed *during* the build is not that case: dropping
/// the built attachments is the designed outcome there, so it still returns `Ok`.
async fn attach_project_capabilities(app: &AppHandle, project: &Project) -> AppResult<()> {
    let build_app = app.clone();
    let build_project = project.clone();
    let built = tauri::async_runtime::spawn_blocking(move || {
        let state = build_app.state::<AppState>();
        build_app
            .state::<ProjectCapabilities>()
            .build_attachments(&build_app, &state, &build_project)
    })
    .await;

    let attachments = match built {
        Ok(attachments) => attachments,
        Err(error) => {
            log::warn!(
                "프로젝트 capability attach 태스크가 실패했습니다 (projectId={}): {error}",
                project.id
            );
            return Err(AppError::Internal(format!("project capability attach failed: {}", project.id)));
        }
    };

    let state = app.state::<AppState>();
    let git_attached = {
        let _guard = state.begin_mutation().await;
        if !state.projects.read().contains_key(&project.id) {
            return Ok(());
        }

        app.state::<ProjectCapabilities>().commit_attachments(&state, project, attachments);
        state.git_watchers.read().contains_key(&project.id)
    };

    if git_attached {
        let _ = GitStatusChanged {
            project_id: project.id.clone(),
        }
        .emit(app);
    }

    Ok(())
}

/// Runs [`project_close`]'s pre-close flush handshake: broadcast the request, wait for every window
/// open at this moment to confirm, give up after `constants::HOT_EXIT_FLUSH_TIMEOUT_MS`.
///
/// Returns nothing because the close proceeds either way — the only thing a timeout changes is
/// that the mirror may be missing the last debounce window, which is strictly better than the
/// project staying open. A handshake already in flight for this project (a second `project_close`
/// racing the first) is left alone and not waited on again; the real close is serialized by
/// `AppState::begin_mutation`, and [`project_close`] re-checks `state.projects` once that guard is
/// held so the losing call returns `Ok(())` — the project is already closed, which is exactly the
/// outcome it asked for, so a double-click on "close project" must not surface an error toast —
/// instead of reaping a project the winner already closed. `service::close_project` refuses
/// nothing on its own — removing an absent key is a no-op,
/// and it would still re-normalize the shell slots, re-persist the session and let the caller
/// re-emit `ProjectClosed`/`ProjectActivated`, moving the focus a second time.
async fn await_project_flush(app: &AppHandle, state: &AppState, project_id: &ProjectId) {
    let scope = FlushScope::Project(project_id.clone());
    // `windows()` needs the `unstable` Tauri feature this project doesn't enable;
    // `webview_windows()` is the stable equivalent and every window here is a webview window.
    let expected_windows: HashSet<String> = app.webview_windows().into_keys().collect();
    if expected_windows.is_empty() {
        return;
    }

    let Some(ticket) = state.begin_flush(scope.clone(), expected_windows) else {
        return;
    };

    let _ = HotExitFlushRequested {
        timeout_ms: constants::HOT_EXIT_FLUSH_TIMEOUT_MS as f64,
        scope: scope.clone(),
    }
    .emit(app);

    let confirmed = ticket
        .wait(std::time::Duration::from_millis(constants::HOT_EXIT_FLUSH_TIMEOUT_MS))
        .await;
    if !confirmed {
        log::warn!("프로젝트 닫기 flush 가 시간 내에 확인되지 않아 그대로 닫습니다 (projectId={project_id})");
    }
    state.clear_flush(&scope);
}

/// Asks every window to flush this project's dirty editor models to the hot-exit mirror, waits for
/// them (bounded by `constants::HOT_EXIT_FLUSH_TIMEOUT_MS`), and only then closes `project_id`.
///
/// The wait is not optional politeness. Every write path the mirror uses is gated on the project
/// still being open — `file_mirror_dirty` starts with `root_guard::project_root`, which answers
/// `NotFound` the instant `state.projects` loses the entry — and the frontend's own flush runs
/// from an unmount effect, which cannot fire until *after* this command has already removed the
/// project and `ipc-sync-provider` has torn the layout query down. So closing first meant the last
/// `HOT_EXIT_MIRROR_DEBOUNCE_MS` of typing was rejected by the backend and swallowed by the
/// caller's `.catch(() => false)` — no error, no toast, nothing on disk (audit wave 2 #12). The
/// handshake here is the same one the app exit and an auxiliary window's close use, scoped to this
/// project ([`FlushScope::Project`]), so a window that renders other projects too flushes only
/// what is about to disappear.
///
/// The wait deliberately happens **before** `AppState::begin_mutation`: it is bounded by a
/// frontend round-trip, and holding the one global mutation guard across it would stall every
/// other command for as long as a window takes to answer. Timing out is not an error — the close
/// proceeds regardless, because a window whose listener never answers must not be able to keep a
/// project open forever. That same gap is why the "still open?" check is repeated after the guard
/// is taken: two closes for the same project can both pass the entry check while they wait, and
/// only one of them may run the reap below.
///
/// Then it reaps every resource that only makes sense while the project is open. See
/// `architecture.md` §6.3 for the authoritative list of what a project close must reclaim.
/// `asset://` read access needs no entry of its own in that list any more: `infra::asset_protocol`
/// decides per-request from `state.projects`, and the `*state.projects.write() = projects;` below
/// (via `service::close_project`, which removes `project_id`) already revokes it before any
/// capability's detach even runs. The reaps themselves are owned by the registered
/// [`ProjectCapabilities`], whose detach walk runs in registration order — an order that is part
/// of the correctness contract (dirty-layout flush before removal, terminal reap during close);
/// see `lib.rs`'s `project_capabilities` and each capability's `detach` doc.
#[tauri::command]
#[specta::specta]
pub async fn project_close(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<()> {
    if !state.projects.read().contains_key(&project_id) {
        return Err(AppError::NotFound(format!("project not open: {project_id}")));
    }

    await_project_flush(&app, &state, &project_id).await;

    let _guard = state.begin_mutation().await;
    if !state.projects.read().contains_key(&project_id) {
        return Ok(());
    }

    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::close_project(&state.paths, &mut session, &mut projects, &project_id)?;

    let active_project = session.active_project.clone();
    *state.session.write() = session;
    *state.projects.write() = projects;

    app.state::<ProjectCapabilities>().detach_all(&app, &state, &project_id);

    let _ = ProjectClosed {
        project_id: project_id.clone(),
    }
    .emit(&app);
    let _ = ProjectActivated {
        project_id: active_project,
    }
    .emit(&app);
    emit_shell_slots_changed(&app, &state);
    emit_list_changed(&app, &state);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn project_activate(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<()> {
    let _span = perf::span(SpanSlot::ProjectActivate);
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::activate_project(&state.paths, &mut session, &mut projects, &project_id)?;

    *state.session.write() = session;
    *state.projects.write() = projects;

    let _ = ProjectActivated {
        project_id: Some(project_id),
    }
    .emit(&app);
    emit_shell_slots_changed(&app, &state);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn project_reorder(app: AppHandle, state: State<'_, AppState>, ids: Vec<ProjectId>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::reorder_projects(&state.paths, &mut session, &ids)?;

    *state.session.write() = session;
    emit_list_changed(&app, &state);

    Ok(())
}

/// Sets one project's sidebar presentation (icon / short label / color token), each axis
/// independently settable, clearable, or left alone — see `types::ProjectDisplayPatch` for the
/// three-state convention and `service::set_project_display` for the sanitizing this command
/// deliberately leaves to the service. Reuses [`ProjectListChanged`] rather than adding a
/// `ProjectDisplayChanged` event: the sidebar renders from `project_list`'s `ProjectRef[]`, which
/// now carries `display`, so the existing fanout already delivers this change to every window and
/// to remote sessions (`lib.rs`'s `fanout_remote_events!`). Remote-allowed at the same grade as
/// `project_reorder` — it rewrites the same two local files (`session.json` plus one
/// `project.json`) with values the remote client could already set by reordering, and exposes no
/// path the remote session cannot already see.
#[tauri::command]
#[specta::specta]
pub async fn project_set_display(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    patch: ProjectDisplayPatch,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    service::set_project_display(&state.paths, &mut session, &mut projects, &project_id, &patch)?;

    *state.session.write() = session;
    *state.projects.write() = projects;
    emit_list_changed(&app, &state);

    Ok(())
}

/// Every sidebar project group, for a window that just mounted — [`ProjectGroupsChanged`] only fires
/// at a transition, the same gap `project_get_active`/`session_get_shell_state` exist to close.
#[tauri::command]
#[specta::specta]
pub async fn project_group_list(state: State<'_, AppState>) -> AppResult<Vec<ProjectGroup>> {
    Ok(service::list_groups(&state.session.read()))
}

/// Creates a sidebar group. `members` may name projects that are merely *known* (a persisted
/// `projects/<id>/` record) rather than currently open — a group organizes what the user can open —
/// and any member that belonged to another group is moved out of it, since a project belongs to at
/// most one group (`types::ProjectGroup`).
#[tauri::command]
#[specta::specta]
pub async fn project_group_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
    members: Option<Vec<ProjectId>>,
) -> AppResult<ProjectGroup> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    let group = service::create_group(&state.paths, &mut session, &name, color.as_deref(), members)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(&app, &state);

    Ok(group)
}

#[tauri::command]
#[specta::specta]
pub async fn project_group_rename(app: AppHandle, state: State<'_, AppState>, group_id: ProjectGroupId, name: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::rename_group(&state.paths, &mut session, &group_id, &name)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(&app, &state);

    Ok(())
}

/// Sets (or, with `color: null`, clears) the group header's tint. The token vocabulary is
/// `ProjectDisplay`'s own `lane1..lane12` palette — see `service::sanitize_display_color`.
#[tauri::command]
#[specta::specta]
pub async fn project_group_set_color(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: ProjectGroupId,
    color: Option<String>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_group_color(&state.paths, &mut session, &group_id, color.as_deref())?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(&app, &state);

    Ok(())
}

/// Persists whether the group's members are folded away in the sidebar. Its own command rather than
/// frontend-local state because `ProjectGroup.collapsed` lives in `session.json` — a fold the user
/// makes has to survive a restart and reach every window, like every other group axis.
#[tauri::command]
#[specta::specta]
pub async fn project_group_set_collapsed(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: ProjectGroupId,
    collapsed: bool,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_group_collapsed(&state.paths, &mut session, &group_id, collapsed)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(&app, &state);

    Ok(())
}

/// Replaces a group's membership wholesale — the backing call for both "그룹에 추가" and "그룹에서
/// 제거" (contract §0.1 U-6), which are the same write with a different list. Nothing is opened or
/// closed by this: membership and open-ness are separate axes (`session.projects` owns the latter).
#[tauri::command]
#[specta::specta]
pub async fn project_group_set_members(
    app: AppHandle,
    state: State<'_, AppState>,
    group_id: ProjectGroupId,
    members: Vec<ProjectId>,
) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::set_group_members(&state.paths, &mut session, &group_id, members)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(&app, &state);

    Ok(())
}

/// Deletes the group only — its members stay open and keep their records, exactly as
/// `project_close` leaves a project's record behind.
#[tauri::command]
#[specta::specta]
pub async fn project_group_delete(app: AppHandle, state: State<'_, AppState>, group_id: ProjectGroupId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::delete_group(&state.paths, &mut session, &group_id)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(&app, &state);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn project_group_reorder(app: AppHandle, state: State<'_, AppState>, ids: Vec<ProjectGroupId>) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();

    service::reorder_groups(&state.paths, &mut session, &ids)?;

    *state.session.write() = session;
    drop(_guard);

    emit_groups_changed(&app, &state);

    Ok(())
}

/// Opens one group's members in the user's own member order. The first member this call actually
/// opens takes focus; every later one is opened with `activate: false` (contract §0.1 S-2) so the
/// queue never yanks the user between projects as it progresses — each member's arrival shows up as
/// a `ProjectListChanged`, which is all the sidebar needs. "Actually opens" is enforced by
/// [`run_group_open_plan`], not by the plan alone: a member whose open fails focuses nothing, so the
/// activation passes to the next member that succeeds.
///
/// Three kinds of member are passed over rather than failing the call, and every one of them comes
/// back in `skipped`: a member that is already open (opening it again would only steal focus), a
/// member whose record or root is gone (contract §0.1 S-7 — logged and stepped over), and a member
/// an open failure or a shutdown stopped this call from reaching. `is_shutting_down` is re-checked
/// per member and stays set once tripped, so the first member that sees it ends the real work and
/// every remaining member is reported as skipped rather than silently dropped.
///
/// Sequential rather than concurrent: each member's open takes the app-wide mutation guard and its
/// capability attach walks the whole working tree, the same reason `restore_project_watchers`
/// attaches one project at a time.
#[tauri::command]
#[specta::specta]
pub async fn project_group_open(app: AppHandle, state: State<'_, AppState>, group_id: ProjectGroupId) -> AppResult<ProjectGroupOpenResult> {
    let members = service::group_members(&state.session.read(), &group_id)?;
    let open_ids: HashSet<ProjectId> = state.projects.read().keys().cloned().collect();
    let plan = groups::plan_open(&members, &open_ids, |project_id| {
        service::group_member_root(&state.paths, project_id)
    });

    for (project_id, reason) in &plan.skipped {
        if reason == &groups::GroupSkipReason::Unavailable {
            log::warn!("그룹 멤버의 레코드나 루트가 없어 건너뜁니다 (groupId={group_id}, projectId={project_id})");
        }
    }

    let mut result = ProjectGroupOpenResult {
        opened: Vec::new(),
        skipped: plan.skipped.into_iter().map(|(project_id, _)| project_id).collect(),
    };

    let app_handle = &app;
    let app_state = &state;
    let opening_group = &group_id;
    run_group_open_plan(
        plan.steps,
        &mut result,
        || state.is_shutting_down(),
        |step, activate| async move {
            match open_group_member(app_handle, app_state, &step.root, activate).await {
                Ok(project) => Some(project.id),
                Err(error) => {
                    log::warn!(
                        "그룹 멤버를 열지 못했습니다 (groupId={opening_group}, projectId={}): {error}",
                        step.project_id
                    );
                    None
                }
            }
        },
    )
    .await;

    Ok(result)
}

/// Walks one group's open queue, handing each member to `open_member` — which answers the opened
/// project's id, or `None` when that member could not be opened — and records every member in
/// `result` as opened or skipped.
///
/// **Carries the activation forward.** [`groups::plan_open`] marks the one member it expects to
/// open first, but a member whose open fails emits no `ProjectActivated` at all, so spending the
/// mark on it would leave the whole group open with the focus still on whatever the user was
/// looking at before — while `ProjectGroupOpenResult` and `docs/ipc-contract.md` both say focus
/// lands on the first member the call *actually* opened (contract §3 S-9). The mark is therefore
/// held until a member really joins the session, and inherited by the next member otherwise.
///
/// `stop` is re-checked per member (it stays set once tripped, so the first member that sees a
/// shutdown ends the real work) and every remaining member is still reported as skipped rather
/// than silently dropped.
///
/// Generic over the open itself so this hand-off is unit-testable: opening a member needs a real
/// `AppHandle` and this codebase has no `tauri::test` mock-app harness — the same constraint
/// [`projects_pending_watcher_restore`] works around by staying a pure selection.
async fn run_group_open_plan<Open, Fut>(
    steps: Vec<groups::GroupOpenStep>,
    result: &mut ProjectGroupOpenResult,
    mut stop: impl FnMut() -> bool,
    mut open_member: Open,
) where
    Open: FnMut(groups::GroupOpenStep, bool) -> Fut,
    Fut: std::future::Future<Output = Option<ProjectId>>,
{
    let mut pending_activation = steps.iter().any(|step| step.activate);

    for step in steps {
        if stop() {
            result.skipped.push(step.project_id);
            continue;
        }

        let activate = pending_activation;
        let project_id = step.project_id.clone();
        match open_member(step, activate).await {
            Some(opened_id) => {
                if activate {
                    pending_activation = false;
                }
                result.opened.push(opened_id);
            }
            None => result.skipped.push(project_id),
        }
    }
}

/// One step of [`project_group_open`]'s queue. Same shape as [`project_open`] — mutation guard
/// around the session/project write, capability attach outside it, rollback through
/// [`project_close`] when the attach fails — with two differences: the root is already known (the
/// member's own record named it), and `activate` is the caller's, so a background member joins the
/// session without emitting `ProjectActivated` or touching the slot tree.
async fn open_group_member(app: &AppHandle, state: &State<'_, AppState>, root: &str, activate: bool) -> AppResult<Project> {
    let opened = {
        let _guard = state.begin_mutation().await;
        let mut session = state.session.read().clone();
        let mut projects = state.projects.read().clone();

        let opened = service::open_project(&state.paths, &mut session, &mut projects, Path::new(root), activate, |canonical| {
            app.state::<ProjectCapabilities>().detected_kinds(canonical)
        })?;

        *state.session.write() = session;
        *state.projects.write() = projects;
        opened
    };

    if !opened.already_open {
        if let Err(error) = attach_project_capabilities(app, &opened.project).await {
            if let Err(rollback) = project_close(app.clone(), state.clone(), opened.project.id.clone()).await {
                log::warn!(
                    "capability attach 실패 후 그룹 멤버 되돌리기도 실패했습니다 (projectId={}): {rollback}",
                    opened.project.id
                );
            }
            return Err(error);
        }

        let _ = ProjectOpened {
            project: opened.project.clone(),
        }
        .emit(app);
        emit_list_changed(app, state);
    }

    if activate {
        let _ = ProjectActivated {
            project_id: Some(opened.project.id.clone()),
        }
        .emit(app);
        emit_shell_slots_changed(app, state);
    }

    Ok(opened.project)
}

pub(crate) fn restore_state(state: &AppState) -> Vec<String> {
    let mut warnings = Vec::new();

    match service::restore_session(&state.paths) {
        Ok((mut session, projects, session_warnings)) => {
            let mut layouts = state.layouts.write();
            for project in &projects {
                layouts.insert(
                    project.id.clone(),
                    crate::domain::layout::service::load_layout(&state.paths, &project.id),
                );
            }

            let mut shell_views = layouts
                .iter()
                .map(|(project_id, layout)| (project_id.clone(), layout.shell_view))
                .collect();
            let promoted = service::promote_legacy_window_chrome(&mut session, &mut shell_views);
            for project_id in &promoted {
                if let (Some(layout), Some(view)) = (layouts.get_mut(project_id), shell_views.get(project_id)) {
                    layout.shell_view = *view;
                }
            }
            drop(layouts);

            if !promoted.is_empty() {
                state.dirty_layouts.write().extend(promoted.iter().cloned());
                if let Err(error) = service::save_session(&state.paths, &session) {
                    warnings.push(format!("창 크롬 상태 승격 후 세션 저장 실패: {error}"));
                }
            }

            *state.session.write() = session;
            *state.projects.write() = projects.into_iter().map(|project| (project.id.clone(), project)).collect();
            warnings.extend(session_warnings);
        }
        Err(error) => warnings.push(format!("세션 복원 실패: {error}")),
    }

    *state.settings.write() = crate::domain::settings::service::load_settings(&state.paths);

    warnings
}

/// The exact snapshot [`restore_project_watchers`] attaches from — every restored project whose
/// root is still present on disk (`!root_missing`), paired with the root path its watcher needs,
/// **ordered** so `session.active_project` (the project the user was actually looking at) attaches
/// first, then the rest in `session.projects`' own order (the same order `project_list`/the
/// project switcher shows — `domain::project::service::list_projects` walks `session.projects` and
/// looks each entry up in `projects`). A project `projects` has that `session.projects` somehow
/// doesn't list (bookkeeping drift, not a state this codebase's own writers produce) is still
/// appended at the end rather than silently dropped from the restore set. Ordering matters because
/// [`restore_project_watchers`] attaches strictly sequentially: without it,
/// `HashMap<ProjectId, Project>`'s unspecified iteration order could just as easily attach the
/// active project *last*, stretching the fs/git event gap the user actually notices to the sum of
/// every other restored project's walk instead of bounding it to its own alone.
///
/// Kept as its own pure function (rather than inline in `setup()`) so this selection — including
/// the ordering — is unit-testable without a running `AppHandle`; see `restore_project_watchers`'s
/// doc for why the attach itself can't be (this codebase has no `tauri::test` mock-app harness —
/// the same constraint `domain::terminal::commands`'s own tests document and work around).
pub(crate) fn projects_pending_watcher_restore(projects: &HashMap<ProjectId, Project>, session: &SessionState) -> Vec<(ProjectId, String)> {
    let mut ordered_ids: Vec<ProjectId> = session.active_project.iter().cloned().collect();
    ordered_ids.extend(
        session
            .projects
            .iter()
            .map(|project_ref| project_ref.id.clone())
            .filter(|id| Some(id) != session.active_project.as_ref()),
    );

    let mut seen: HashSet<ProjectId> = ordered_ids.iter().cloned().collect();
    for project_id in projects.keys() {
        if seen.insert(project_id.clone()) {
            ordered_ids.push(project_id.clone());
        }
    }

    ordered_ids
        .into_iter()
        .filter_map(|project_id| {
            let project = projects.get(&project_id)?;
            (!project.root_missing).then(|| (project.id.clone(), project.root.clone()))
        })
        .collect()
}

/// Re-attaches the file watcher and (where the project is a git repo) the git watcher for every
/// project [`projects_pending_watcher_restore`] selected, as a background task that starts only
/// after `app.manage(state)` in `lib.rs`'s `setup()` — so the multi-second `notify-debouncer-full`
/// `FileIdMap` walk each attach performs (the dominant boot-latency cause identified in
/// `docs/acknowledge/2026-08-20-boot-watcher-defer-contract.md`) never delays window creation the
/// way the old fully-synchronous loop in `lib.rs`'s `setup()` did. `restore_state` and every
/// `app.manage` call stay synchronous in `setup()` — the tree and editor need the
/// projects/layouts they populate immediately — only the watcher attach itself moves here.
/// `project_open` attaches through [`attach_project_capabilities`], which now applies this same
/// build/register split via the capability trait; this function stays separate because its unit of
/// work is a *queue* of restored projects with its own ordering, skip, and shutdown rules.
///
/// **Build outside the guard, register inside it.** Each iteration's `spawn_blocking` call — where
/// the `FileIdMap` walk actually happens — runs with no `AppState::begin_mutation` held at all:
/// `domain::file::capability::build_watcher_handle`/`domain::git::watch::build_git_watcher_handle`
/// only build a `WatcherHandle`, touching no `AppState` field, the same "build outside every lock,
/// re-validate and insert inside the store's own lock" split
/// `domain::tree::commands::rows_page_from_store` already established for a miss's own disk walk
/// (Phase E C11-TREE-2). The guard is acquired only *after* the build returns, held across a
/// `state.projects`/`state.watchers` re-check plus the two handle inserts, and dropped before this
/// iteration's synthetic event emits below — microseconds, not the walk's 0.5–3s. The re-check is
/// what keeps this correct without holding the guard across the walk: `state.projects` only shrinks
/// via `project_close`, which (like `project_open`) runs its entire body under one `begin_mutation`
/// acquisition, so a close landing anywhere during this iteration's unguarded build is fully visible
/// the instant this loop re-acquires the guard afterward — there is no window where `contains_key`
/// could observe a stale "still open". Dropping the handles just built (which stops those
/// `Debouncer`s) when the project closed meanwhile, or when `state.watchers` already has an entry
/// for it (see below), is the only correctness work the guard still has to do.
///
/// **Skip a project `project_open` already reattached.** The snapshot
/// [`projects_pending_watcher_restore`] took can include a project the user closed and reopened
/// before this loop reached it — `project_open` reuses the same `ProjectId` for the same root
/// (`domain::project::service::find_existing_project_id`), and its own attach already ran.
/// `FileWatcherCapability::build_attachment` always attaches the file watcher unconditionally on
/// that path, so `state.watchers.read().contains_key(&project_id)` being true is a reliable "already
/// reattached, skip" signal — checked once before spawning the walk at all (the common case, so the
/// walk is never even started) and once more right before registering under the guard (the race
/// window between the two: `project_open` landing *during* this iteration's unguarded build).
/// Either hit drops this iteration's freshly built handles instead of overwriting `project_open`'s
/// live ones, and skips the synthetic emits below too — that project's caches are already live.
///
/// **Boot-gap correction for the caches that don't self-heal.** Two frontend caches only ever
/// refresh on an event, never on a timer or focus, so a change landing in the gap between window
/// show and a project's own attach here has nowhere to go otherwise (no watcher is listening yet,
/// and `notify` never replays history once one starts):
/// - `git_status`/`git_refs`: `entities/git/git.query.ts`'s `GIT.PROJECT`-scoped cache only
///   refreshes on `GitStatusChanged`/`GitRefsChanged` or the global 60s `staleTime`
///   (`app/query-client.ts`), so a gap-window git change with no *further* change afterward would
///   sit stale for up to a minute — or indefinitely, for a query nothing is actively refetching.
/// - `FILE.CONTENT`/`FILE.RAW`: `entities/file/file.query.ts`'s per-path queries are `staleTime:
///   Infinity` with the global `refetchOnWindowFocus: false` (`app/query-client.ts`), so a file
///   already open in a tab before this project's watcher attaches has *no* time- or focus-based
///   path back to fresh content — only `events.fsChanged` (`app/providers/ipc-sync-provider.tsx`)
///   ever invalidates it. Left uncorrected this is worse than the git gap: it never expires on its
///   own, and `file_save` has no mtime/baseline check, so saving from the stale tab would silently
///   overwrite whatever changed on disk during the gap.
///
///   `tree_rows` needs no such correction: `domain::tree::commands::rows_page_from_store`'s cache
///   starts empty every boot (`TreeStore::default()`), so the very first read for any project
///   always rescans disk directly regardless of watcher state. What's left uncorrected there is a
///   narrower window than it might look — a *second* directory change landing after that first
///   read but before this project's own attach below completes, which stays unseen until some later
///   event touches the same directory. That window is genuinely wider here than on `project_open`
///   (which attaches synchronously before returning, so the frontend's first `tree_rows` call for
///   that project is always already past a live watcher — there is no "first read ~ attach" gap on
///   that path to compare against): sequential attach across a multi-project restore bounds this
///   project's window by *its own position in the queue* (its own walk plus every walk ahead of
///   it), not by a single walk alone. No watcher event can narrow that further than a rescan already
///   does, so it stays an accepted residual — see `docs/quality-assurance/2026-08-11-qa6-checklist.md`'s
///   d-25 section for the hand-QA carryover this and the two corrections below still need.
///
/// Both corrections reuse an existing event end to end — no new command, event, or query key.
/// `GitStatusChanged` alone (not paired with `GitRefsChanged`; the frontend maps both to the exact
/// same `QUERY_KEY.GIT.PROJECT(projectId)` invalidation, so emitting both is pure duplication) once
/// this project's git watcher attaches, and `FsChanged { kind: Modified, from_app: false, paths:
/// <this project's currently open File tab paths> }` (`domain::layout::service::open_file_paths`)
/// once its file watcher attaches and it actually has any open file tabs. Gating the git emit on a
/// `.git/index`/`.git/HEAD` mtime comparison (attach-taken snapshot vs. post-attach) to skip it
/// entirely when nothing changed was considered and rejected: it would add IO on every boot,
/// including the common case where nothing changed, and still couldn't detect a change made through
/// a tool that doesn't preserve mtimes — while the cost it would save is already small,
/// `invalidateQueries` against a query nobody is displaying is a no-op, so these emits only ever do
/// real work for the active project's actually-displayed git/file queries, once, at boot.
///
/// A close/quit requested mid-restore (`AppState::is_shutting_down`) stops the loop before starting
/// the next project's walk — see `AppState::begin_shutdown`'s doc for where that flag is set —
/// instead of continuing to attach every remaining project (each briefly taking `begin_mutation`)
/// while the app is already tearing down.
pub(crate) fn restore_project_watchers(app: &tauri::AppHandle, restored: Vec<(ProjectId, String)>) {
    let app_handle = app.clone();
    let started = std::time::Instant::now();
    let project_count = restored.len();

    tauri::async_runtime::spawn(async move {
        let mut attached = 0usize;

        for (project_id, root) in restored {
            let state = app_handle.state::<AppState>();
            if state.is_shutting_down() {
                break;
            }
            if state.watchers.read().contains_key(&project_id) {
                continue;
            }

            let build_handle = app_handle.clone();
            let build_project_id = project_id.clone();
            let build_root = root.clone();
            let build_result = tauri::async_runtime::spawn_blocking(move || {
                let file_handle = crate::domain::file::capability::build_watcher_handle(&build_handle, &build_project_id, &build_root);
                let git_handle = crate::domain::git::watch::build_git_watcher_handle(&build_handle, &build_project_id, &build_root);
                (file_handle, git_handle)
            })
            .await;

            let (file_handle, git_handle) = match build_result {
                Ok(handles) => handles,
                Err(error) => {
                    log::warn!("복원 프로젝트 워처 attach 태스크가 실패했습니다 (projectId={project_id}): {error}");
                    continue;
                }
            };

            let state = app_handle.state::<AppState>();
            let _guard = state.begin_mutation().await;
            if !state.projects.read().contains_key(&project_id) || state.watchers.read().contains_key(&project_id) {
                continue;
            }

            let file_attached = file_handle.is_some();
            let git_attached = git_handle.is_some();
            if let Some(handle) = file_handle {
                crate::domain::file::capability::register_watcher_handle(&state, &project_id, handle);
            }
            if let Some(handle) = git_handle {
                crate::domain::git::watch::register_git_watcher_handle(&state, &project_id, handle);
            }
            drop(_guard);
            attached += 1;

            if file_attached {
                let open_paths = state
                    .layouts
                    .read()
                    .get(&project_id)
                    .map(crate::domain::layout::service::open_file_paths)
                    .unwrap_or_default();
                if !open_paths.is_empty() {
                    let _ = FsChanged {
                        project_id: project_id.clone(),
                        change: FsChange {
                            kind: FsChangeKind::Modified,
                            paths: open_paths,
                            from_app: false,
                        },
                    }
                    .emit(&app_handle);
                }
            }

            if git_attached {
                let _ = GitStatusChanged {
                    project_id: project_id.clone(),
                }
                .emit(&app_handle);
            }
        }

        log::info!(
            "복원 프로젝트 워처 attach 완료: projects={project_count}, attached={attached}, elapsed_ms={}",
            started.elapsed().as_millis()
        );
    });
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::project::types::{ProjectDisplay, SESSION_SCHEMA_VERSION};

    fn source_between<'a>(start_marker: &str, end_marker: &str) -> &'a str {
        let source = include_str!("commands.rs");
        let start = source
            .find(start_marker)
            .unwrap_or_else(|| panic!("시작 마커를 찾을 수 없습니다: {start_marker}"))
            + start_marker.len();
        let end = source[start..]
            .find(end_marker)
            .unwrap_or_else(|| panic!("종료 마커를 찾을 수 없습니다: {end_marker}"));
        &source[start..start + end]
    }

    fn marker_position(body: &str, marker: &str) -> usize {
        body.find(marker)
            .unwrap_or_else(|| panic!("본문에서 마커를 찾을 수 없습니다: {marker}"))
    }

    const ATTACH_SIGNATURE: &str = "async fn attach_project_capabilities(app: &AppHandle, project: &Project) -> AppResult<()> {";

    /// The whole point of the two-phase attach is the *position of the lock*, and nothing but this
    /// file's own control flow enforces it — every other test still passes if the build moves back
    /// inside the guard. So scan the source: the expensive build must run on a blocking thread
    /// before `begin_mutation` is ever awaited, and the commit only after it.
    #[test]
    fn attach_는_가드를_잡기_전에_build_하고_가드_안에서_commit_한다() {
        let body = source_between(ATTACH_SIGNATURE, "\n}\n");

        let build = marker_position(body, "build_attachments(");
        let guard = marker_position(body, "begin_mutation()");
        let commit = marker_position(body, "commit_attachments(");

        assert!(
            marker_position(body, "spawn_blocking(") < build,
            "capability build 는 blocking 스레드에서 실행돼야 합니다 — async 워커에서 돌리면 워처 walk 가 런타임을 막습니다"
        );
        assert!(
            build < guard,
            "capability build 가 begin_mutation 뒤로 가면 후절화가 무효가 됩니다 — 워처 walk 동안 앱 전역 뮤테이션이 다시 정지합니다"
        );
        assert!(
            guard < commit,
            "commit 은 재획득한 가드 안에서만 실행돼야 합니다 — AppState 쓰기가 가드 밖으로 나가면 안 됩니다"
        );
        assert!(
            body.contains("state.projects.read().contains_key(&project.id)"),
            "가드 재획득 후 프로젝트가 아직 열려 있는지 재검증해야 합니다 — build 중 project_close 가 끼어들 수 있습니다"
        );
    }

    /// `File > Clear Recent` runs this command **from Rust** (`lib.rs`'s `dispatch_menu_action`, so
    /// the menu keeps working with zero windows open) and drops the returned outcome on the floor —
    /// there is no IPC caller to hand it to. Unless the command publishes the counts itself, the
    /// "kept N projects that still hold unsaved drafts" notice is unreachable from the only path that
    /// actually clears the list, and a recent list that refuses to empty reads as a failed command.
    #[test]
    fn project_forget_recent_은_초안_때문에_건너뛴_수를_이벤트로_알린다() {
        let body = source_between("pub async fn project_forget_recent(", "\n}\n");

        assert!(
            body.contains("ProjectRecentCleared {"),
            "project_forget_recent 가 ProjectRecentCleared 를 emit 하지 않으면 네이티브 메뉴 경로에서 초안 보존 안내가 사라집니다"
        );
        assert!(
            body.contains("skipped_with_drafts: outcome.skipped_with_drafts"),
            "이벤트가 skipped_with_drafts 를 그대로 실어야 프론트가 안내 여부를 판단할 수 있습니다"
        );
    }

    /// `await_project_flush` deliberately waits *outside* `begin_mutation`, so two closes of the
    /// same project can both clear the entry check and then queue on the guard. Nothing but this
    /// file's own control flow stops the loser from reaping a second time —
    /// `service::close_project` removes an absent key happily, re-normalizes the shell slots and
    /// re-persists the session, after which the caller re-emits `ProjectClosed`/`ProjectActivated`
    /// and moves the focus again. So scan the source: the re-check must sit after the guard.
    #[test]
    fn project_close_는_가드를_잡은_뒤_프로젝트가_아직_열려_있는지_재검사한다() {
        let body = source_between("pub async fn project_close(", "\n}\n");
        let guard = marker_position(body, "let _guard = state.begin_mutation().await;");
        let checks: Vec<usize> = body
            .match_indices("state.projects.read().contains_key(&project_id)")
            .map(|(position, _)| position)
            .collect();

        assert_eq!(
            checks.len(),
            2,
            "열림 검사는 두 번이어야 합니다 — flush 대기 전 진입 검사와 가드 재획득 뒤 재검사"
        );
        assert!(
            checks[0] < guard && guard < checks[1],
            "재검사가 begin_mutation 앞으로 가면 flush 대기 중 끼어든 다른 close 를 걸러내지 못합니다"
        );
        let recheck_block = &body[checks[1]..marker_position(body, "let mut session = state.session.read().clone();")];
        assert!(
            recheck_block.contains("return Ok(())"),
            "재검사에서 이미 닫힌 프로젝트는 조용히 성공해야 합니다 — 이중 닫기는 목표를 이미 달성한 것이라 NotFound 토스트를 띄우면 안 됩니다"
        );
    }

    /// `project_open` must reach the attach through [`attach_project_capabilities`] (which owns the
    /// build/register split) and only *after* its own mutation guard scope has closed.
    #[test]
    fn project_open_은_가드_스코프를_닫은_뒤에_attach_한다() {
        let body = source_between("pub async fn project_open(", "\n}\n");
        let after_guard = source_between("let _guard = state.begin_mutation().await;", "attach_project_capabilities(");

        assert!(
            marker_position(body, "*state.projects.write() = projects;") < marker_position(body, "attach_project_capabilities("),
            "attach 는 세션·프로젝트 맵 반영 뒤에 와야 합니다"
        );
        assert!(
            after_guard.contains("};"),
            "attach 호출 전에 가드 스코프가 닫혀야 합니다 — 가드를 쥔 채 attach 하면 후절화 이전으로 되돌아갑니다"
        );
        assert!(
            !body.contains("build_attachments(") && !body.contains("commit_attachments("),
            "attach 2단계는 attach_project_capabilities 한 곳에서만 조립돼야 합니다"
        );
    }

    /// The build phase runs on a blocking thread, so its failure arrives as a `JoinError` that the
    /// unit type would have swallowed: nothing committed, yet the project already published into
    /// `state.projects`/`state.session` and the caller told the open succeeded. Pin the two halves
    /// of the correction — the failure is reported, and the half-open project is unwound through
    /// `project_close` before any `ProjectOpened` goes out.
    #[test]
    fn attach_실패는_열기_실패로_보고되고_프로젝트를_되돌린다() {
        let attach = source_between(ATTACH_SIGNATURE, "\n}\n");
        let open = source_between("pub async fn project_open(", "\n}\n");

        assert!(
            attach.contains("return Err(AppError::Internal("),
            "spawn_blocking join 실패는 로그만 남기고 끝내면 안 됩니다 — 호출부가 실패를 알 수 있어야 합니다"
        );
        assert!(
            marker_position(open, "attach_project_capabilities(") < marker_position(open, "project_close("),
            "attach 실패 시 이미 등록해 둔 프로젝트를 project_close 로 되돌려야 합니다"
        );
        assert!(
            marker_position(open, "project_close(") < marker_position(open, "ProjectOpened"),
            "attach 실패 경로는 ProjectOpened 방출 전에 에러로 반환돼야 합니다"
        );
    }

    fn open_step(project_id: &str, activate: bool) -> groups::GroupOpenStep {
        groups::GroupOpenStep {
            project_id: ProjectId::from(project_id.to_string()),
            root: format!("/repo/{project_id}"),
            activate,
        }
    }

    /// 계약 §3 S-9 — 활성화는 "열릴 예정인 첫 멤버" 가 아니라 **실제로 열린 첫 멤버** 의 것이어야
    /// 한다. 실패한 열기는 `ProjectActivated` 를 내지 않으므로, 계획의 표식이 실패한 멤버에서
    /// 소모되면 그룹을 다 열고도 포커스가 이전 프로젝트에 남는다. 승계되는지와, 그래도 활성화가
    /// 한 번뿐인지를 함께 고정한다.
    #[tokio::test]
    async fn 그룹_열기는_활성화가_계획된_멤버가_실패하면_다음_성공_멤버에_승계한다() {
        let failing = ProjectId::from("prj-fails".to_string());
        let steps = vec![
            open_step("prj-fails", true),
            open_step("prj-opens", false),
            open_step("prj-rest", false),
        ];
        let attempts: RefCell<Vec<(ProjectId, bool)>> = RefCell::new(Vec::new());
        let attempted = &attempts;
        let failing_member = &failing;
        let mut result = ProjectGroupOpenResult {
            opened: Vec::new(),
            skipped: Vec::new(),
        };

        run_group_open_plan(
            steps,
            &mut result,
            || false,
            |step, activate| async move {
                attempted.borrow_mut().push((step.project_id.clone(), activate));
                (&step.project_id != failing_member).then_some(step.project_id)
            },
        )
        .await;

        assert_eq!(
            attempts.borrow().clone(),
            vec![
                (failing.clone(), true),
                (ProjectId::from("prj-opens".to_string()), true),
                (ProjectId::from("prj-rest".to_string()), false),
            ],
            "첫 멤버가 실패하면 그다음 멤버가 활성화를 이어받고, 그 뒤로는 다시 포커스를 가로채지 않아야 합니다"
        );
        assert_eq!(
            attempts
                .borrow()
                .iter()
                .filter(|(project_id, activate)| *activate && result.opened.contains(project_id))
                .count(),
            1,
            "ProjectActivated 는 실제로 열린 멤버 하나에서만 나가야 합니다"
        );
        assert_eq!(
            result.opened,
            vec![ProjectId::from("prj-opens".to_string()), ProjectId::from("prj-rest".to_string())]
        );
        assert_eq!(result.skipped, vec![failing], "열지 못한 멤버는 skipped 로 보고돼야 합니다");
    }

    /// 계약 §3 B-1 — 멤버십이 바뀌지 않은 clear-recent 는 `ProjectGroupsChanged` 를 내면 안 된다
    /// (`docs/ipc-contract.md` 도 "그룹이 실제로 바뀐 경우" 라고 적는다). 커맨드 자체는 `AppHandle`
    /// 없이 돌릴 수 없고 이 파일의 제어 흐름 말고는 강제하는 것이 없으므로, 소스에서 emit 이
    /// 조건 안에 있는지를 고정한다 — 위 attach 순서 테스트와 같은 수단이다.
    #[test]
    fn forget_recent_는_그룹이_바뀐_경우에만_groups_changed_를_발행한다() {
        let body = source_between("pub async fn project_forget_recent(", "\n}\n");

        assert_eq!(
            body.matches("emit_groups_changed(").count(),
            1,
            "그룹 이벤트 발행 지점은 한 곳이어야 조건 검사가 의미를 갖습니다"
        );
        assert!(
            marker_position(body, "if outcome.groups_changed {") < marker_position(body, "emit_groups_changed("),
            "그룹 이벤트는 groups_changed 조건 안에서만 발행돼야 합니다 — 무조건 발행하면 모든 창이 바뀌지도 않은 그룹 목록을 다시 읽습니다"
        );
        assert!(
            body.contains("emit_list_changed(&app, &state);"),
            "최근 목록 변경은 그룹과 무관하게 항상 발행돼야 합니다"
        );
    }

    fn stub_project(id: &str, root: &str, root_missing: bool) -> Project {
        Project {
            id: ProjectId::from(id.to_string()),
            root: root.to_string(),
            name: id.to_string(),
            capabilities: Vec::new(),
            root_missing,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        }
    }

    fn stub_session(active_project: Option<&str>, ordered_ids: &[&str]) -> SessionState {
        SessionState {
            version: SESSION_SCHEMA_VERSION,
            projects: ordered_ids
                .iter()
                .map(|id| ProjectRef {
                    id: ProjectId::from((*id).to_string()),
                    root: String::new(),
                    name: (*id).to_string(),
                    display: ProjectDisplay::default(),
                    root_missing: false,
                })
                .collect(),
            active_project: active_project.map(|id| ProjectId::from(id.to_string())),
            ..SessionState::default()
        }
    }

    /// Pins [`restore_project_watchers`]'s attach-target selection — the boundary the boot-watcher
    /// defer contract (`docs/acknowledge/2026-08-20-boot-watcher-defer-contract.md`) actually lets
    /// this codebase unit-test, since `build_watcher_handle`/`build_git_watcher_handle` themselves
    /// need a real `AppHandle` this codebase has no mock-app harness for.
    #[test]
    fn 워처_재부착_대상은_루트가_존재하는_복원_프로젝트만_포함한다() {
        let projects: HashMap<ProjectId, Project> = [
            stub_project("prj-present", "/repo/present", false),
            stub_project("prj-missing", "/repo/missing", true),
        ]
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();
        let session = stub_session(None, &["prj-present", "prj-missing"]);

        let pending = projects_pending_watcher_restore(&projects, &session);

        assert_eq!(
            pending,
            vec![(ProjectId::from("prj-present".to_string()), "/repo/present".to_string())],
            "root_missing 프로젝트는 워처 재부착 대상에서 제외되어야 합니다"
        );
    }

    #[test]
    fn 복원된_프로젝트가_없으면_워처_재부착_대상도_비어있다() {
        let projects: HashMap<ProjectId, Project> = HashMap::new();
        let session = stub_session(None, &[]);

        assert!(
            projects_pending_watcher_restore(&projects, &session).is_empty(),
            "복원된 프로젝트가 없으면 재부착 대상도 없어야 합니다"
        );
    }

    /// design-2/concurrency-2 — sequential attach 라 순서가 곧 각 프로젝트의 이벤트 공백 상한이다.
    /// 활성 프로젝트가 `session.projects` 순서상 어디에 있든 항상 첫 원소로 나와야, 그 공백이
    /// "이 프로젝트 자신의 워크" 하나로 유계되고 다른 프로젝트들의 워크 합으로 늘어나지 않는다.
    #[test]
    fn 활성_프로젝트가_session_순서와_무관하게_항상_첫_원소다() {
        let projects: HashMap<ProjectId, Project> = [
            stub_project("prj-a", "/repo/a", false),
            stub_project("prj-b", "/repo/b", false),
            stub_project("prj-c", "/repo/c", false),
        ]
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();
        let session = stub_session(Some("prj-b"), &["prj-c", "prj-a", "prj-b"]);

        let pending = projects_pending_watcher_restore(&projects, &session);

        assert_eq!(
            pending,
            vec![
                (ProjectId::from("prj-b".to_string()), "/repo/b".to_string()),
                (ProjectId::from("prj-c".to_string()), "/repo/c".to_string()),
                (ProjectId::from("prj-a".to_string()), "/repo/a".to_string()),
            ],
            "활성 프로젝트가 선두, 나머지는 session.projects 순서를 그대로 유지해야 합니다"
        );
    }

    #[test]
    fn session_projects에_없는_프로젝트도_재부착_대상에서_누락되지_않는다() {
        let projects: HashMap<ProjectId, Project> = [
            stub_project("prj-tracked", "/repo/tracked", false),
            stub_project("prj-drift", "/repo/drift", false),
        ]
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();
        let session = stub_session(None, &["prj-tracked"]);

        let pending = projects_pending_watcher_restore(&projects, &session);

        assert_eq!(
            pending.len(),
            2,
            "session.projects 가 놓친 프로젝트도 재부착 대상에 포함되어야 합니다"
        );
        assert!(
            pending.iter().any(|(id, _)| id == &ProjectId::from("prj-drift".to_string())),
            "session 기록에서 누락된 프로젝트도 워처 재부착 대상에 포함되어야 합니다"
        );
    }
}
