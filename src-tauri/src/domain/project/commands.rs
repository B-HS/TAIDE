use std::collections::{HashMap, HashSet};
use std::path::Path;

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;

use super::capability::ProjectCapabilities;
use super::service;
use super::types::{Project, ProjectRef, SessionState};
use crate::domain::file::types::{FsChange, FsChangeKind};
use crate::error::AppResult;
use crate::events::{FsChanged, GitStatusChanged, ProjectActivated, ProjectClosed, ProjectListChanged, ProjectOpened};
use crate::ids::ProjectId;
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

#[tauri::command]
#[specta::specta]
pub async fn project_open(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<service::ProjectOpenResult> {
    let _guard = state.begin_mutation().await;
    let mut session = state.session.read().clone();
    let mut projects = state.projects.read().clone();

    let result = service::open_project(&state.paths, &mut session, &mut projects, Path::new(&path), |canonical| {
        app.state::<ProjectCapabilities>().detected_kinds(canonical)
    })?;

    *state.session.write() = session;
    *state.projects.write() = projects;

    if !result.already_open {
        app.state::<ProjectCapabilities>().attach_all(&app, &state, &result.project);

        let _ = ProjectOpened {
            project: result.project.clone(),
        }
        .emit(&app);
        emit_list_changed(&app, &state);
    }

    Ok(result)
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
/// `project_open`'s own synchronous attach (via `ProjectCapabilities::attach_all`) is unchanged;
/// this function only covers boot restore.
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
/// (`domain::project::service::find_existing_project_id`), and its own synchronous attach already
/// ran. `FileWatcherCapability::attach` always attaches the file watcher unconditionally on that
/// path, so `state.watchers.read().contains_key(&project_id)` being true is a reliable "already
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
    use crate::domain::project::types::SESSION_SCHEMA_VERSION;

    fn stub_project(id: &str, root: &str, root_missing: bool) -> Project {
        Project {
            id: ProjectId::from(id.to_string()),
            root: root.to_string(),
            name: id.to_string(),
            capabilities: Vec::new(),
            root_missing,
            last_opened_at: 0.0,
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
