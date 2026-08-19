use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tauri::State;

use super::service::{self, TreeState};
use super::types::TreeRowPage;
use crate::error::{AppError, AppResult};
use crate::ids::ProjectId;
use crate::state::AppState;

pub struct TreeStore(pub parking_lot::RwLock<HashMap<ProjectId, TreeState>>);

impl TreeStore {
    pub fn new() -> Self {
        Self(parking_lot::RwLock::new(HashMap::new()))
    }

    /// Forgets `project_id`'s cached tree — every expanded/collapsed row and directory listing
    /// scanned so far. Called by `TreeCacheCapability::detach` during `project_close` so reopening the same folder
    /// starts from a fresh scan instead of resurrecting a stale directory listing that may no
    /// longer match disk (files changed while the project was closed and unwatched), and so the
    /// entry doesn't sit in this map for the rest of the app's lifetime.
    pub fn remove(&self, project_id: &ProjectId) {
        self.0.write().remove(project_id);
    }
}

impl Default for TreeStore {
    fn default() -> Self {
        Self::new()
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

fn ensure_entry<'a>(
    trees: &'a mut HashMap<ProjectId, TreeState>,
    state: &AppState,
    project_id: &ProjectId,
) -> AppResult<&'a mut TreeState> {
    if !trees.contains_key(project_id) {
        let root = project_root(state, project_id)?;
        let mut tree = service::new_tree_state(root);
        service::ensure_root_loaded(&mut tree)?;
        trees.insert(project_id.clone(), tree);
    }

    trees
        .get_mut(project_id)
        .ok_or_else(|| AppError::Internal(format!("tree state missing after insert: {project_id}")))
}

#[tauri::command]
#[specta::specta]
pub async fn tree_rows(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    project_id: ProjectId,
    offset: u32,
    limit: Option<u32>,
) -> AppResult<TreeRowPage> {
    let mut trees = tree_store.0.read().clone();
    let tree = ensure_entry(&mut trees, &state, &project_id)?;
    let page = service::rows_page(tree, offset, limit);
    *tree_store.0.write() = trees;
    Ok(page)
}

#[tauri::command]
#[specta::specta]
pub async fn tree_toggle(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    project_id: ProjectId,
    path: String,
) -> AppResult<TreeRowPage> {
    let _guard = state.begin_mutation().await;
    let mut trees = tree_store.0.read().clone();
    let tree = ensure_entry(&mut trees, &state, &project_id)?;
    service::toggle_expand(tree, Path::new(&path))?;
    let page = service::full_page(tree);
    *tree_store.0.write() = trees;
    Ok(page)
}

#[tauri::command]
#[specta::specta]
pub async fn tree_reveal(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    project_id: ProjectId,
    path: String,
) -> AppResult<TreeRowPage> {
    let _guard = state.begin_mutation().await;
    let mut trees = tree_store.0.read().clone();
    let tree = ensure_entry(&mut trees, &state, &project_id)?;
    service::reveal(tree, Path::new(&path))?;
    let page = service::full_page(tree);
    *tree_store.0.write() = trees;
    Ok(page)
}

#[tauri::command]
#[specta::specta]
pub async fn tree_refresh(
    state: State<'_, AppState>,
    tree_store: State<'_, TreeStore>,
    project_id: ProjectId,
    dir: String,
) -> AppResult<TreeRowPage> {
    let _guard = state.begin_mutation().await;
    let mut trees = tree_store.0.read().clone();
    let tree = ensure_entry(&mut trees, &state, &project_id)?;
    service::invalidate(tree, Path::new(&dir))?;
    let page = service::full_page(tree);
    *tree_store.0.write() = trees;
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove는_해당_프로젝트의_캐시된_트리만_지운다() {
        let store = TreeStore::new();
        let closing = ProjectId::new();
        let staying = ProjectId::new();
        store
            .0
            .write()
            .insert(closing.clone(), service::new_tree_state(PathBuf::from("/tmp/closing-root")));
        store
            .0
            .write()
            .insert(staying.clone(), service::new_tree_state(PathBuf::from("/tmp/staying-root")));

        store.remove(&closing);

        assert!(
            !store.0.read().contains_key(&closing),
            "닫힌 프로젝트의 트리 캐시는 제거되어야 한다"
        );
        assert!(
            store.0.read().contains_key(&staying),
            "다른 프로젝트의 트리 캐시는 남아 있어야 한다"
        );
    }
}
