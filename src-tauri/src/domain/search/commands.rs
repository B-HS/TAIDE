use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::ipc::Channel;
use tauri::State;

use super::service;
use super::types::{SearchMatch, SearchQuery, SearchReplaceResult};
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::state::AppState;

/// Keyed by a caller-supplied session id (one per search surface — the quick
/// search panel or a single Search Editor tab), not by [`ProjectId`]. Two
/// surfaces searching the same project run independently; only a second run
/// from the *same* surface supersedes its own prior run. See
/// `docs/acknowledge/2026-08-15-wave-d-search-nav-contract.md` §3.4.
#[derive(Default)]
pub struct SearchStore(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl SearchStore {
    pub fn new() -> Self {
        Self::default()
    }
}

fn project_root(state: &AppState, project_id: &ProjectId) -> AppResult<PathBuf> {
    state
        .projects
        .read()
        .get(project_id)
        .map(|project| PathBuf::from(&project.root))
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

fn begin_search(store: &SearchStore, session_id: &str) -> Arc<AtomicBool> {
    let mut sessions = store.0.lock();
    if let Some(previous) = sessions.get(session_id) {
        previous.store(true, Ordering::SeqCst);
    }

    let cancelled = Arc::new(AtomicBool::new(false));
    sessions.insert(session_id.to_string(), cancelled.clone());
    cancelled
}

/// Removes `session_id`'s entry once its search finishes — but only if no
/// newer run for that same session has already replaced it (compared by
/// `Arc` identity), so a slow-finishing cancelled run can never clobber a
/// fresh one. Keeps the store bounded to in-flight/most-recent searches
/// instead of growing forever as Search Editor tabs open and close.
fn end_search(store: &SearchStore, session_id: &str, own_flag: &Arc<AtomicBool>) {
    let mut sessions = store.0.lock();
    if sessions.get(session_id).is_some_and(|current| Arc::ptr_eq(current, own_flag)) {
        sessions.remove(session_id);
    }
}

#[tauri::command]
#[specta::specta]
pub async fn search_run(
    state: State<'_, AppState>,
    store: State<'_, SearchStore>,
    project_id: ProjectId,
    session_id: String,
    query: SearchQuery,
    on_match: Channel<SearchMatch>,
) -> AppResult<u32> {
    let root = project_root(&state, &project_id)?;

    let cancelled = {
        let _guard = state.begin_mutation().await;
        begin_search(&store, &session_id)
    };

    let join_result = tokio::task::spawn_blocking({
        let cancelled = cancelled.clone();
        move || {
            service::search(&root, &query, &cancelled, move |item| {
                let _ = on_match.send(item);
            })
        }
    })
    .await
    .map_err(|error| AppError::Internal(format!("search task failed: {error}")))?;

    end_search(&store, &session_id, &cancelled);

    join_result
}

#[tauri::command]
#[specta::specta]
pub async fn search_replace(
    state: State<'_, AppState>,
    project_id: ProjectId,
    query: SearchQuery,
    replacement: String,
    paths: Option<Vec<String>>,
) -> AppResult<SearchReplaceResult> {
    let root = project_root(&state, &project_id)?;
    let target_paths = paths.map(|list| list.into_iter().map(PathBuf::from).collect::<Vec<_>>());

    tokio::task::spawn_blocking(move || service::replace(&root, &query, &replacement, target_paths.as_deref()))
        .await
        .map_err(|error| AppError::Internal(format!("replace task failed: {error}")))?
}

#[tauri::command]
#[specta::specta]
pub async fn search_cancel(state: State<'_, AppState>, store: State<'_, SearchStore>, session_id: String) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    if let Some(cancelled) = store.0.lock().get(&session_id) {
        cancelled.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 서로_다른_세션은_서로의_검색을_취소하지_않는다() {
        let store = SearchStore::new();

        let panel_flag = begin_search(&store, "panel");
        let editor_flag = begin_search(&store, "search-editor:tab-1");

        assert!(
            !panel_flag.load(Ordering::SeqCst),
            "다른 세션(search-editor)이 panel 세션을 취소하면 안 된다"
        );
        assert!(!editor_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn 같은_세션의_재검색은_이전_실행을_취소한다() {
        let store = SearchStore::new();

        let first = begin_search(&store, "panel");
        let second = begin_search(&store, "panel");

        assert!(first.load(Ordering::SeqCst), "같은 세션의 재검색은 이전 실행을 취소해야 한다");
        assert!(!second.load(Ordering::SeqCst));
    }

    #[test]
    fn 검색이_끝나면_스토어에서_세션_항목이_제거된다() {
        let store = SearchStore::new();
        let flag = begin_search(&store, "panel");

        end_search(&store, "panel", &flag);

        assert!(store.0.lock().get("panel").is_none());
    }

    #[test]
    fn 이미_새_실행으로_대체된_세션_항목은_종료_시_제거되지_않는다() {
        let store = SearchStore::new();
        let stale = begin_search(&store, "panel");
        let fresh = begin_search(&store, "panel");

        end_search(&store, "panel", &stale);

        let current = store.0.lock().get("panel").cloned();
        assert!(
            current.is_some_and(|entry| Arc::ptr_eq(&entry, &fresh)),
            "새 실행의 항목이 유지되어야 한다"
        );
    }

    #[test]
    fn 검색_취소는_해당_세션의_플래그만_설정한다() {
        let store = SearchStore::new();
        let panel_flag = begin_search(&store, "panel");
        let editor_flag = begin_search(&store, "search-editor:tab-1");

        if let Some(cancelled) = store.0.lock().get("panel") {
            cancelled.store(true, Ordering::SeqCst);
        }

        assert!(panel_flag.load(Ordering::SeqCst));
        assert!(!editor_flag.load(Ordering::SeqCst));
    }
}
