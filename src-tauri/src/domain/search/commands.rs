use std::path::PathBuf;

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

pub use taide_runtime::SearchStore;

use super::service::{self, ReplaceOutcome};
use super::types::{ReplaceSkippedFile, SearchFileMatches, SearchQuery, SearchReplaceResult, REPLACE_SKIP_REPORT_LIMIT};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::infra::perf::{self, SpanSlot};
use crate::state::AppState;

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<PathBuf> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

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
    let root = project_root(&state, &project_id)?;

    let cancelled = {
        let _guard = state.begin_mutation().await;
        store.begin(&owner, &session_id)
    };

    let join_result = tokio::task::spawn_blocking({
        let cancelled = cancelled.clone();
        move || {
            service::search(&root, &query, &cancelled, move |batch| {
                let _ = on_match.send(batch);
            })
        }
    })
    .await
    .map_err(|error| AppError::Internal(format!("search task failed: {error}")))?;

    store.finish(&owner, &session_id, &cancelled);

    join_result
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
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    query: SearchQuery,
    replacement: String,
    paths: Option<Vec<String>>,
) -> AppResult<SearchReplaceResult> {
    let root = project_root(&state, &project_id)?;

    if query.text.is_empty() {
        return Ok(SearchReplaceResult {
            changed_files: 0,
            replaced_matches: 0,
            skipped: Vec::new(),
            skipped_count: 0,
        });
    }

    let compiled = service::compile_query(&query)?;
    let target_paths = paths.map(|list| list.into_iter().map(PathBuf::from).collect::<Vec<_>>());

    let target_files = {
        let scan_root = root.clone();
        let scan_query = query.clone();
        tokio::task::spawn_blocking(move || service::resolve_replace_targets(&scan_root, &scan_query, target_paths.as_deref()))
            .await
            .map_err(|error| AppError::Internal(format!("replace scan task failed: {error}")))?
    };

    let mut changed_files = 0u32;
    let mut replaced_matches = 0u32;
    let mut skipped: Vec<ReplaceSkippedFile> = Vec::new();
    let mut skipped_count = 0u32;

    for path in &target_files {
        let app = app.clone();
        let target = path.clone();
        let replacement = replacement.clone();
        let compiled = compiled.clone();

        let outcome = tokio::task::spawn_blocking(move || {
            let app_state = app.state::<AppState>();
            let _guard = app_state.begin_mutation_blocking();
            let outcome = service::replace_one_file(&target, &compiled, &replacement);
            if matches!(outcome, ReplaceOutcome::Replaced(_)) {
                app_state.self_writes.mark(&target);
            }
            outcome
        })
        .await
        .map_err(|error| AppError::Internal(format!("replace task failed: {error}")))?;

        match outcome {
            ReplaceOutcome::Replaced(count) => {
                changed_files += 1;
                replaced_matches += count;
            }
            ReplaceOutcome::NoMatch => {}
            ReplaceOutcome::Skipped(reason) => {
                skipped_count += 1;
                if skipped.len() < REPLACE_SKIP_REPORT_LIMIT {
                    skipped.push(ReplaceSkippedFile {
                        path: path.to_string_lossy().to_string(),
                        reason,
                    });
                }
            }
        }
    }

    Ok(SearchReplaceResult {
        changed_files,
        replaced_matches,
        skipped,
        skipped_count,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn search_cancel(state: State<'_, AppState>, store: State<'_, SearchStore>, owner: String, session_id: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    store.cancel(&owner, &session_id);
    Ok(())
}

/// Every file path under `project_id`'s root — backs the command palette's file quick-open, which
/// must find a file regardless of whether the Explorer tree has ever been expanded into its parent
/// folder (`docs/features/command-palette.md` §3; contract
/// `2026-08-25-d42-e2e-defects-contract.md` §3, item d). Returns absolute paths (matching
/// `TreeRow.path`'s convention, which this replaces as the palette's file-mode data source) rather
/// than project-relative ones — `command-palette.tsx`'s `toProjectRelativePath`/`openFile(path)`
/// both already expect that shape. Runs the walk in `spawn_blocking` like `search_run`/
/// `search_replace`'s own scans, since [`service::list_project_files`] is synchronous filesystem
/// I/O and must not block the async runtime.
#[tauri::command]
#[specta::specta]
pub async fn search_list_files(state: State<'_, AppState>, project_id: ProjectId) -> AppResult<Vec<String>> {
    let _span = perf::span(SpanSlot::SearchListFiles);
    let root = project_root(&state, &project_id)?;

    let started = std::time::Instant::now();
    let paths = tokio::task::spawn_blocking(move || service::list_project_files(&root))
        .await
        .map_err(|error| AppError::Internal(format!("list project files task failed: {error}")))?;

    let paths = utf8_paths(paths);
    log::debug!(
        "search_list_files 완료 (projectId={project_id}, 건수={}, 소요={}ms)",
        paths.len(),
        started.elapsed().as_millis()
    );
    Ok(paths)
}

/// Drops entries whose path is not valid UTF-8 instead of `to_string_lossy`-ing them. A lossy
/// conversion substitutes U+FFFD for the undecodable bytes, which produces a *different* path than
/// the one on disk: the palette would list it, and opening it would fail at the filesystem with a
/// "not found" for a file that plainly exists. Since `TabKind::File`/`OpenedFile` carry paths as
/// `String`, such a file is unopenable through this app either way — omitting it is the honest
/// listing, and it keeps the index's contract ("every path here is openable") intact for the
/// quick-open pre-validation in `layout::commands::layout_open_tab`.
fn utf8_paths(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .filter_map(|path| path.into_os_string().into_string().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The palette index must never advertise a path that cannot be opened. `to_string_lossy` used
    /// to hand it a U+FFFD-substituted spelling of a non-UTF-8 filename — a path that does not
    /// exist on disk, so clicking the row failed with "not found" for a file the walker had just
    /// reported. The entry is synthesized from raw bytes rather than written to a temp directory
    /// because the development platform (APFS) enforces UTF-8 filenames and rejects the `write`
    /// outright (`EILSEQ`), so no fixture can produce one there; the byte sequence below is exactly
    /// what [`service::list_project_files`] hands back on the filesystems that do allow it.
    #[cfg(unix)]
    #[test]
    fn 비_utf8_경로는_퀵오픈_목록에서_제외된다() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let walked = vec![
            PathBuf::from("/repo/readable.rs"),
            PathBuf::from("/repo").join(OsStr::from_bytes(b"broken-\xff.rs")),
        ];

        let listed = utf8_paths(walked);

        assert_eq!(listed, vec!["/repo/readable.rs".to_string()], "비-UTF8 경로는 목록에서 빠져야 한다");
    }
}
