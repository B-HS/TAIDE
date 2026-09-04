use std::collections::{HashMap, HashSet};
use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::capability::ProjectCapabilities;
use super::service;
use super::types::{Project, ProjectDisplayPatch, ProjectRef, SessionState};
use crate::domain::file::types::{FsChange, FsChangeKind};
use crate::error::{AppError, AppResult};
use crate::events::{FsChanged, GitStatusChanged, ProjectActivated, ProjectClosed, ProjectListChanged, ProjectOpened};
use crate::ids::ProjectId;
use crate::infra::perf::{self, SpanSlot};
use crate::state::AppState;

fn emit_list_changed(app: &AppHandle, state: &AppState) {
    let projects = service::list_projects(&state.session.read());
    let _ = ProjectListChanged { projects }.emit(app);
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

        let result = service::open_project(&state.paths, &mut session, &mut projects, Path::new(&path), |canonical| {
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

    Ok(result)
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

/// Closes `project_id` and reaps every resource that only makes sense while the project is open.
/// See `architecture.md` §6.3 for the authoritative list of what a project close must reclaim.
/// `asset://` read access needs no entry of its own in that list any more: `infra::asset_protocol`
/// decides per-request from `state.projects`, and the `*state.projects.write() = projects;` above
/// (via `service::close_project`, which removes `project_id`) already revokes it before any
/// capability's detach even runs. The reaps themselves are owned by the registered
/// [`ProjectCapabilities`], whose detach walk runs in registration order — an order that is part
/// of the correctness contract (dirty-layout flush before removal, terminal reap during close);
/// see `lib.rs`'s `project_capabilities` and each capability's `detach` doc.
#[tauri::command]
#[specta::specta]
pub async fn project_close(app: AppHandle, state: State<'_, AppState>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
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

pub(crate) fn restore_state(state: &AppState) -> Vec<String> {
    let mut warnings = Vec::new();

    match service::restore_session(&state.paths) {
        Ok((session, projects, session_warnings)) => {
            let mut layouts = state.layouts.write();
            for project in &projects {
                layouts.insert(
                    project.id.clone(),
                    crate::domain::layout::service::load_layout(&state.paths, &project.id),
                );
            }
            drop(layouts);

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
                })
                .collect(),
            active_project: active_project.map(|id| ProjectId::from(id.to_string())),
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
