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

/// Read path of the tree store (audit R4#1, C11 axis B). The old body cloned the **entire** map,
/// ran the read-only `rows_page` on the clone, and wrote the whole map back without any guard —
/// so a `tree_toggle`/`tree_reveal`/`tree_refresh` that committed between the clone and the
/// write-back was silently rolled back (lost update), and every scroll paid a full deep copy.
/// The write-back existed only to persist `ensure_entry`'s first-load side effect, so this keeps
/// exactly that: a hit serves the page under the store's read lock and writes nothing; a miss
/// builds the fresh root-loaded `TreeState` outside every lock (the only disk I/O on this path)
/// and inserts **only that entry** via `entry().or_insert` — if a guarded mutation created the
/// entry meanwhile, the existing (newer) entry wins and the freshly built one is dropped, never
/// the other way around.
fn rows_page_from_store(
    tree_store: &TreeStore,
    state: &AppState,
    project_id: &ProjectId,
    offset: u32,
    limit: Option<u32>,
) -> AppResult<TreeRowPage> {
    if let Some(tree) = tree_store.0.read().get(project_id) {
        return Ok(service::rows_page(tree, offset, limit));
    }

    let root = project_root(state, project_id)?;
    let mut tree = service::new_tree_state(root);
    service::ensure_root_loaded(&mut tree)?;

    let mut trees = tree_store.0.write();
    let entry = trees.entry(project_id.clone()).or_insert(tree);
    Ok(service::rows_page(entry, offset, limit))
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
    rows_page_from_store(&tree_store, &state, &project_id, offset, limit)
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

    use crate::domain::project::types::Project;
    use crate::paths::AppPaths;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("taide-tree-cmd-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn state_with_project(project_id: &ProjectId, root: &Path) -> AppState {
        let state = AppState::new(AppPaths::new(std::env::temp_dir()));
        state.projects.write().insert(
            project_id.clone(),
            Project {
                id: project_id.clone(),
                root: root.to_string_lossy().to_string(),
                name: "tree-test".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
            },
        );
        state
    }

    #[test]
    fn tree_rows_미스는_해당_엔트리만_추가하고_기존_엔트리의_상태를_보존한다() {
        let root_a = temp_root("miss-a");
        let sub_a = root_a.join("sub");
        std::fs::create_dir_all(&sub_a).unwrap();
        let root_b = temp_root("miss-b");
        std::fs::write(root_b.join("b.txt"), "b").unwrap();

        let a = ProjectId::new();
        let b = ProjectId::new();
        let state = state_with_project(&b, &root_b);
        let store = TreeStore::new();
        {
            let mut tree_a = service::new_tree_state(root_a.clone());
            service::ensure_root_loaded(&mut tree_a).unwrap();
            service::expand(&mut tree_a, &sub_a).unwrap();
            store.0.write().insert(a.clone(), tree_a);
        }

        let page = rows_page_from_store(&store, &state, &b, 0, None).expect("rows");

        assert_eq!(page.total, 1, "B 루트의 파일 1개가 보여야 한다");
        let trees = store.0.read();
        assert!(trees.contains_key(&b), "미스 경로는 B 엔트리를 캐시에 추가해야 한다");
        let entry_a = trees.get(&a).expect("조회 경로가 A 엔트리를 지우면 안 된다");
        assert!(
            service::expanded_paths(entry_a).contains(&sub_a.to_string_lossy().to_string()),
            "A 의 확장 상태가 보존되어야 한다"
        );
        drop(trees);

        std::fs::remove_dir_all(&root_a).ok();
        std::fs::remove_dir_all(&root_b).ok();
    }

    #[test]
    fn tree_rows_히트는_기존_엔트리의_확장_상태로_페이지를_만든다() {
        let root = temp_root("hit");
        let sub = root.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("child.txt"), "c").unwrap();

        let project_id = ProjectId::new();
        let state = state_with_project(&project_id, &root);
        let store = TreeStore::new();
        {
            let mut tree = service::new_tree_state(root.clone());
            service::ensure_root_loaded(&mut tree).unwrap();
            service::expand(&mut tree, &sub).unwrap();
            store.0.write().insert(project_id.clone(), tree);
        }

        let page = rows_page_from_store(&store, &state, &project_id, 0, None).expect("rows");

        assert_eq!(page.total, 2, "확장된 sub 아래의 child 까지 평탄화되어야 한다");
        assert_eq!(page.rows[1].depth, 1, "히트 경로는 사전 확장 상태를 그대로 반영해야 한다");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn tree_rows_조회는_다른_프로젝트의_동시_뮤테이션을_잃지_않는다() {
        const TOGGLE_ITERATIONS: usize = 200;

        let root_a = temp_root("concurrent-a");
        let sub_a = root_a.join("sub");
        std::fs::create_dir_all(&sub_a).unwrap();
        let root_b = temp_root("concurrent-b");

        let a = ProjectId::new();
        let b = ProjectId::new();
        let state = state_with_project(&b, &root_b);
        let store = TreeStore::new();
        {
            let mut tree_a = service::new_tree_state(root_a.clone());
            service::ensure_root_loaded(&mut tree_a).unwrap();
            store.0.write().insert(a.clone(), tree_a);
        }

        std::thread::scope(|scope| {
            let mutator = scope.spawn(|| {
                for _ in 0..TOGGLE_ITERATIONS {
                    let mut trees = store.0.write();
                    let entry = trees.get_mut(&a).expect("A 엔트리는 유지되어야 한다");
                    service::toggle_expand(entry, &sub_a).expect("toggle");
                }
            });
            let reader = scope.spawn(|| {
                for _ in 0..TOGGLE_ITERATIONS {
                    store.remove(&b);
                    rows_page_from_store(&store, &state, &b, 0, None).expect("rows");
                }
            });
            mutator.join().unwrap();
            reader.join().unwrap();
        });

        let trees = store.0.read();
        let entry_a = trees.get(&a).expect("조회 경로가 A 엔트리를 지우면 안 된다");
        assert!(
            !service::expanded_paths(entry_a).contains(&sub_a.to_string_lossy().to_string()),
            "짝수 번 토글의 최종 상태(collapsed)가 유실 없이 보존되어야 한다 — 조회 경로에 전체 되쓰기가 남아 있으면 실패할 수 있다"
        );
        drop(trees);

        std::fs::remove_dir_all(&root_a).ok();
        std::fs::remove_dir_all(&root_b).ok();
    }

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
