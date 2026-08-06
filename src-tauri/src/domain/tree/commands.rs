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
    limit: u32,
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
