use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::search::{ReplaceSkippedFile, SearchFileMatches, SearchQuery, SearchReplaceResult, REPLACE_SKIP_REPORT_LIMIT};
use taide_search::service::{self, ReplaceOutcome};
use tokio::sync::oneshot;

use crate::{AppState, SearchStore, TaskSupervisor};

struct SearchSessionOwner {
    store: SearchStore,
    owner: String,
    session_id: String,
    cancelled: Arc<AtomicBool>,
}

impl Drop for SearchSessionOwner {
    fn drop(&mut self) {
        self.store.finish(&self.owner, &self.session_id, &self.cancelled);
    }
}

struct SearchRequestCancellation(Arc<AtomicBool>);

impl Drop for SearchRequestCancellation {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn search_shutdown_error() -> AppError {
    AppError::Forbidden("search runtime is shutting down".to_string())
}

async fn run_search_blocking_result<T: Send + 'static>(
    tasks: &TaskSupervisor,
    name: &'static str,
    join_error_prefix: &'static str,
    work: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let (sender, receiver) = oneshot::channel();
    let worker = tasks
        .spawn_blocking_transient_handle(name, move || {
            drop(sender.send(work()));
        })
        .ok_or_else(search_shutdown_error)?;
    worker
        .await
        .map_err(|error| AppError::Internal(format!("{join_error_prefix}{error}")))?;
    receiver.await.map_err(|_| search_shutdown_error())?
}

async fn run_search_worker(
    tasks: &TaskSupervisor,
    session: SearchSessionOwner,
    work: impl FnOnce(&AtomicBool) -> AppResult<u32> + Send + 'static,
) -> AppResult<u32> {
    let _cancel_on_drop = SearchRequestCancellation(session.cancelled.clone());
    run_search_blocking_result(tasks, "search-run", "search task failed: ", move || work(&session.cancelled)).await
}

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<PathBuf> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

/// Groups the shared state, search sessions, and registered task owner for one search request.
pub struct SearchRunContext<'a> {
    pub state: &'a AppState,
    pub store: &'a SearchStore,
    pub tasks: &'a TaskSupervisor,
}

pub async fn search_run(
    context: SearchRunContext<'_>,
    project_id: ProjectId,
    owner: String,
    session_id: String,
    query: SearchQuery,
    on_match: impl FnMut(SearchFileMatches) + Send + 'static,
) -> AppResult<u32> {
    let SearchRunContext { state, store, tasks } = context;
    let root = project_root(state, &project_id)?;
    let _operation = tasks.begin_operation("search-run").ok_or_else(search_shutdown_error)?;
    let cancelled = {
        let _guard = state.begin_mutation().await;
        store.begin(&owner, &session_id)
    };
    let session = SearchSessionOwner {
        store: store.clone(),
        owner,
        session_id,
        cancelled,
    };
    run_search_worker(tasks, session, move |cancelled| service::search(&root, &query, cancelled, on_match)).await
}

pub async fn search_replace(
    state: &AppState,
    tasks: &TaskSupervisor,
    project_id: ProjectId,
    query: SearchQuery,
    replacement: String,
    paths: Option<Vec<String>>,
) -> AppResult<SearchReplaceResult> {
    let root = project_root(state, &project_id)?;
    let _operation = tasks.begin_operation("search-replace").ok_or_else(search_shutdown_error)?;
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
        run_search_blocking_result(tasks, "search-replace-scan", "replace scan task failed: ", move || {
            Ok(service::resolve_replace_targets(&scan_root, &scan_query, target_paths.as_deref()))
        })
        .await?
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
        let outcome = run_search_blocking_result(tasks, "search-replace-file", "replace task failed: ", move || {
            let _guard = shared_state.begin_mutation_blocking();
            let outcome = service::replace_one_file(&target, &compiled, &replacement);
            if matches!(outcome, ReplaceOutcome::Replaced(_)) {
                shared_state.self_writes.mark(&target);
            }
            Ok(outcome)
        })
        .await?;
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

pub async fn search_list_files(state: &AppState, tasks: &TaskSupervisor, project_id: ProjectId) -> AppResult<Vec<String>> {
    let root = project_root(state, &project_id)?;
    let _operation = tasks.begin_operation("search-list-files").ok_or_else(search_shutdown_error)?;
    let paths = run_search_blocking_result(tasks, "search-list-files", "list project files task failed: ", move || {
        Ok(service::list_project_files(&root))
    })
    .await?;
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
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::oneshot;

    use super::*;
    use crate::TaskSupervisor;

    const OWNER_PROBE_MS: u64 = 20;
    const OWNER_TIMEOUT_MS: u64 = 2_000;

    #[tokio::test]
    async fn 취소된_검색_요청의_worker와_세션_정리를_root가_기다린다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let store = SearchStore::new();
        let cancelled = store.begin("main", "panel");
        let session = SearchSessionOwner {
            store: store.clone(),
            owner: "main".to_string(),
            session_id: "panel".to_string(),
            cancelled: cancelled.clone(),
        };
        let request_tasks = tasks.clone();
        let (started, started_rx) = oneshot::channel();
        let (release, held) = std::sync::mpsc::channel();
        let request = tokio::spawn(async move {
            run_search_worker(&request_tasks, session, move |cancelled| {
                started.send(()).ok();
                held.recv().ok();
                assert!(cancelled.load(Ordering::SeqCst));
                Ok(0)
            })
            .await
        });
        tokio::time::timeout(Duration::from_millis(OWNER_TIMEOUT_MS), started_rx)
            .await
            .unwrap()
            .unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(cancelled.load(Ordering::SeqCst));
        let shutdown = tasks.shutdown();
        tokio::pin!(shutdown);
        assert!(tokio::time::timeout(Duration::from_millis(OWNER_PROBE_MS), &mut shutdown)
            .await
            .is_err());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_millis(OWNER_TIMEOUT_MS), shutdown)
            .await
            .unwrap();
        assert_eq!(Arc::strong_count(&cancelled), 1);
    }

    #[tokio::test]
    async fn 검색_worker_panic은_기존_오류_접두사와_세션_정리를_유지한다() {
        let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
        let store = SearchStore::new();
        let cancelled = store.begin("main", "panel");
        let session = SearchSessionOwner {
            store,
            owner: "main".to_string(),
            session_id: "panel".to_string(),
            cancelled: cancelled.clone(),
        };

        let error = run_search_worker(&tasks, session, |_| panic!("synthetic search worker panic"))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("search task failed:"));
        assert_eq!(Arc::strong_count(&cancelled), 1);
    }

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
