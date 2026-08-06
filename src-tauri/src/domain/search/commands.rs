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

#[derive(Default)]
pub struct SearchStore(Mutex<HashMap<ProjectId, Arc<AtomicBool>>>);

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

fn begin_search(store: &SearchStore, project_id: &ProjectId) -> Arc<AtomicBool> {
    let mut sessions = store.0.lock();
    if let Some(previous) = sessions.get(project_id) {
        previous.store(true, Ordering::SeqCst);
    }

    let cancelled = Arc::new(AtomicBool::new(false));
    sessions.insert(project_id.clone(), cancelled.clone());
    cancelled
}

#[tauri::command]
#[specta::specta]
pub async fn search_run(
    state: State<'_, AppState>,
    store: State<'_, SearchStore>,
    project_id: ProjectId,
    query: SearchQuery,
    on_match: Channel<SearchMatch>,
) -> AppResult<u32> {
    let root = project_root(&state, &project_id)?;

    let cancelled = {
        let _guard = state.begin_mutation().await;
        begin_search(&store, &project_id)
    };

    let join_result = tokio::task::spawn_blocking(move || {
        service::search(&root, &query, &cancelled, move |item| {
            let _ = on_match.send(item);
        })
    })
    .await
    .map_err(|error| AppError::Internal(format!("search task failed: {error}")))?;

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
pub async fn search_cancel(state: State<'_, AppState>, store: State<'_, SearchStore>, project_id: ProjectId) -> AppResult<()> {
    let _guard = state.begin_mutation().await;
    if let Some(cancelled) = store.0.lock().get(&project_id) {
        cancelled.store(true, Ordering::SeqCst);
    }
    Ok(())
}
