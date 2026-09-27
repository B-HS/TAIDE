use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use taide_runtime::search_actions;
pub use taide_runtime::SearchStore;

use super::types::{SearchFileMatches, SearchQuery, SearchReplaceResult};
use crate::error::AppResult;
use crate::ids::ProjectId;
use crate::infra::perf::{self, SpanSlot};
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub async fn search_run(
    state: State<'_, AppState>,
    store: State<'_, SearchStore>,
    project_id: ProjectId,
    owner: String,
    session_id: String,
    query: SearchQuery,
    on_match: Channel<SearchFileMatches>,
) -> AppResult<u32> {
    let _span = perf::span(SpanSlot::SearchRun);
    search_actions::search_run(&state, &store, project_id, owner, session_id, query, move |batch| {
        let _ = on_match.send(batch);
    })
    .await
}

/// Reacquires `AppState::begin_mutation`'s single global lock **once per file** instead of holding
/// it for the whole multi-file replace — a project-wide "replace all" can touch hundreds of files,
/// and that one lock is shared by every other mutating command (`file_save`, `git_pull`, layout
/// writes, ...), so holding it for the entire walk-and-rewrite would starve all of them for as long
/// as the replace runs. Reacquiring per file keeps each hold short while still serializing every
/// actual write against the rest of the app's mutations. Both the target-file resolution (the tree
/// walk) and each file's guarded read-modify-write run inside `spawn_blocking` — none of it runs on
/// the async worker thread — since `service::replace_one_file` does synchronous filesystem I/O.
///
/// Files the pass could not rewrite (oversized, binary, non-UTF-8, unreadable, write failure) are
/// reported back in `skipped`/`skipped_count` rather than dropped silently: a replace that changed
/// nothing because every target was refused used to be indistinguishable from one whose query
/// simply had no matches (audit §4-B C10). Files that were read fine and had no match are not
/// skips and are not counted.
#[tauri::command]
#[specta::specta]
pub async fn search_replace(
    _app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    query: SearchQuery,
    replacement: String,
    paths: Option<Vec<String>>,
) -> AppResult<SearchReplaceResult> {
    search_actions::search_replace(&state, project_id, query, replacement, paths).await
}

#[tauri::command]
#[specta::specta]
pub async fn search_cancel(state: State<'_, AppState>, store: State<'_, SearchStore>, owner: String, session_id: String) -> AppResult<()> {
    search_actions::search_cancel(&state, &store, owner, session_id).await
}

/// Every file path under `project_id`'s root — backs the command palette's file quick-open, which
/// must find a file regardless of whether the Explorer tree has ever been expanded into its parent
/// folder (`docs/features/command-palette.md` §3; contract
/// `2026-08-25-d42-e2e-defects-contract.md` §3, item d). Returns absolute paths (matching
/// `TreeRow.path`'s convention, which this replaces as the palette's file-mode data source) rather
/// than project-relative ones — `command-palette.tsx`'s `toProjectRelativePath`/`openFile(path)`
/// both already expect that shape. Runs the walk in `spawn_blocking` like `search_run`/
/// `search_replace`'s own scans, since [`taide_search::service::list_project_files`] is synchronous filesystem
/// I/O and must not block the async runtime.
#[tauri::command]
#[specta::specta]
pub async fn search_list_files(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Vec<String>> {
    let _span = perf::span(SpanSlot::SearchListFiles);
    let started = std::time::Instant::now();
    let paths = search_actions::search_list_files(&state, project_id.clone()).await?;
    log::debug!(
        "search_list_files 완료 (projectId={project_id}, 건수={}, 소요={}ms)",
        paths.len(),
        started.elapsed().as_millis()
    );
    Ok(paths)
}
