use std::path::PathBuf;

use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::search::{ReplaceSkippedFile, SearchFileMatches, SearchQuery, SearchReplaceResult, REPLACE_SKIP_REPORT_LIMIT};
use taide_search::service::{self, ReplaceOutcome};

use crate::{AppState, SearchStore};

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<PathBuf> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

pub async fn search_run(
    state: &AppState,
    store: &SearchStore,
    project_id: ProjectId,
    owner: String,
    session_id: String,
    query: SearchQuery,
    on_match: impl FnMut(SearchFileMatches) + Send + 'static,
) -> AppResult<u32> {
    let root = project_root(state, &project_id)?;
    let cancelled = {
        let _guard = state.begin_mutation().await;
        store.begin(&owner, &session_id)
    };
    let join_result = tokio::task::spawn_blocking({
        let cancelled = cancelled.clone();
        move || service::search(&root, &query, &cancelled, on_match)
    })
    .await
    .map_err(|error| AppError::Internal(format!("search task failed: {error}")))?;
    store.finish(&owner, &session_id, &cancelled);
    join_result
}

pub async fn search_replace(
    state: &AppState,
    project_id: ProjectId,
    query: SearchQuery,
    replacement: String,
    paths: Option<Vec<String>>,
) -> AppResult<SearchReplaceResult> {
    let root = project_root(state, &project_id)?;
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
    let mut skipped = Vec::new();
    let mut skipped_count = 0u32;
    for path in &target_files {
        let shared_state = state.clone();
        let target = path.clone();
        let replacement = replacement.clone();
        let compiled = compiled.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let _guard = shared_state.begin_mutation_blocking();
            let outcome = service::replace_one_file(&target, &compiled, &replacement);
            if matches!(outcome, ReplaceOutcome::Replaced(_)) {
                shared_state.self_writes.mark(&target);
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

pub async fn search_cancel(state: &AppState, store: &SearchStore, owner: String, session_id: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    store.cancel(&owner, &session_id);
    Ok(())
}

pub async fn search_list_files(state: &AppState, project_id: ProjectId) -> AppResult<Vec<String>> {
    let root = project_root(state, &project_id)?;
    let paths = tokio::task::spawn_blocking(move || service::list_project_files(&root))
        .await
        .map_err(|error| AppError::Internal(format!("list project files task failed: {error}")))?;
    Ok(utf8_paths(paths))
}

fn utf8_paths(paths: Vec<PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .filter_map(|path| path.into_os_string().into_string().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
