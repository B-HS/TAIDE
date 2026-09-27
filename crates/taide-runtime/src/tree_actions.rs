use std::collections::HashMap;
use std::path::{Path, PathBuf};

use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::tree::TreeRowPage;
use taide_tree::service::{self, DirectoryListings, TreeState};

use crate::{AppState, TreeStore};

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
    listings: &mut DirectoryListings,
) -> AppResult<&'a mut TreeState> {
    if !trees.contains_key(project_id) {
        let root = project_root(state, project_id)?;
        let mut tree = service::new_tree_state(root);
        service::ensure_root_loaded(&mut tree, listings)?;
        trees.insert(project_id.clone(), tree);
    }

    trees
        .get_mut(project_id)
        .ok_or_else(|| AppError::Internal(format!("tree state missing after insert: {project_id}")))
}

fn rows_page_from_store(
    tree_store: &TreeStore,
    state: &AppState,
    project_id: &ProjectId,
    offset: u32,
    limit: Option<u32>,
    listings: &mut DirectoryListings,
) -> AppResult<TreeRowPage> {
    if let Some(tree) = tree_store.0.read().get(project_id) {
        return Ok(service::rows_page(tree, offset, limit));
    }

    let root = project_root(state, project_id)?;
    let mut tree = service::new_tree_state(root);
    service::ensure_root_loaded(&mut tree, listings)?;

    let mut trees = tree_store.0.write();
    if !state.projects.read().contains_key(project_id) {
        return Ok(service::rows_page(&tree, offset, limit));
    }
    let entry = trees.entry(project_id.clone()).or_insert(tree);
    Ok(service::rows_page(entry, offset, limit))
}

fn plan_reads(
    tree_store: &TreeStore,
    state: &AppState,
    project_id: &ProjectId,
    plan: impl FnOnce(&TreeState) -> Vec<PathBuf>,
) -> AppResult<Vec<PathBuf>> {
    if let Some(tree) = tree_store.0.read().get(project_id) {
        return Ok(plan(tree));
    }

    let root = project_root(state, project_id)?;
    Ok(plan(&service::new_tree_state(root)))
}

async fn prefetch_listings(dirs: Vec<PathBuf>) -> AppResult<DirectoryListings> {
    if dirs.is_empty() {
        return Ok(DirectoryListings::default());
    }

    tokio::task::spawn_blocking(move || service::read_directories(dirs))
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

pub async fn tree_rows(
    state: &AppState,
    tree_store: &TreeStore,
    project_id: ProjectId,
    offset: u32,
    limit: Option<u32>,
) -> AppResult<TreeRowPage> {
    let dirs = plan_reads(tree_store, state, &project_id, service::plan_root_read)?;
    let mut listings = prefetch_listings(dirs).await?;

    rows_page_from_store(tree_store, state, &project_id, offset, limit, &mut listings)
}

pub async fn tree_toggle(state: &AppState, tree_store: &TreeStore, project_id: ProjectId, path: String) -> AppResult<TreeRowPage> {
    let dirs = plan_reads(tree_store, state, &project_id, |tree| {
        service::plan_toggle_reads(tree, Path::new(&path))
    })?;
    let mut listings = prefetch_listings(dirs).await?;

    let _guard = state.begin_mutation().await;
    let mut trees = tree_store.0.write();
    let tree = ensure_entry(&mut trees, state, &project_id, &mut listings)?;
    service::toggle_expand(tree, Path::new(&path), &mut listings)?;
    Ok(service::full_page(tree))
}

pub async fn tree_collapse_all(state: &AppState, tree_store: &TreeStore, project_id: ProjectId) -> AppResult<TreeRowPage> {
    let dirs = plan_reads(tree_store, state, &project_id, service::plan_root_read)?;
    let mut listings = prefetch_listings(dirs).await?;

    let _guard = state.begin_mutation().await;
    let mut trees = tree_store.0.write();
    let tree = ensure_entry(&mut trees, state, &project_id, &mut listings)?;
    service::collapse_all(tree);
    Ok(service::full_page(tree))
}

pub async fn tree_reveal(state: &AppState, tree_store: &TreeStore, project_id: ProjectId, path: String) -> AppResult<TreeRowPage> {
    let dirs = plan_reads(tree_store, state, &project_id, |tree| {
        service::plan_reveal_reads(tree, Path::new(&path))
    })?;
    let mut listings = prefetch_listings(dirs).await?;

    let _guard = state.begin_mutation().await;
    let mut trees = tree_store.0.write();
    let tree = ensure_entry(&mut trees, state, &project_id, &mut listings)?;
    service::reveal(tree, Path::new(&path), &mut listings)?;
    Ok(service::full_page(tree))
}

pub async fn tree_refresh(state: &AppState, tree_store: &TreeStore, project_id: ProjectId, dir: String) -> AppResult<TreeRowPage> {
    let dirs = plan_reads(tree_store, state, &project_id, |tree| {
        service::plan_refresh_reads(tree, Path::new(&dir))
    })?;
    let mut listings = prefetch_listings(dirs).await?;

    let _guard = state.begin_mutation().await;
    let mut trees = tree_store.0.write();
    let tree = ensure_entry(&mut trees, state, &project_id, &mut listings)?;
    service::invalidate(tree, Path::new(&dir), &mut listings)?;
    Ok(service::full_page(tree))
}

#[cfg(test)]
mod tests {
    use super::*;

    use taide_model::paths::AppPaths;
    use taide_model::project::Project;

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
                last_opened_at: 0.0,
                display: Default::default(),
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
            service::ensure_root_loaded(&mut tree_a, &mut DirectoryListings::default()).unwrap();
            service::expand(&mut tree_a, &sub_a, &mut DirectoryListings::default()).unwrap();
            store.0.write().insert(a.clone(), tree_a);
        }

        let page = rows_page_from_store(&store, &state, &b, 0, None, &mut DirectoryListings::default()).expect("rows");

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
            service::ensure_root_loaded(&mut tree, &mut DirectoryListings::default()).unwrap();
            service::expand(&mut tree, &sub, &mut DirectoryListings::default()).unwrap();
            store.0.write().insert(project_id.clone(), tree);
        }

        let page = rows_page_from_store(&store, &state, &project_id, 0, None, &mut DirectoryListings::default()).expect("rows");

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
            service::ensure_root_loaded(&mut tree_a, &mut DirectoryListings::default()).unwrap();
            store.0.write().insert(a.clone(), tree_a);
        }

        std::thread::scope(|scope| {
            let mutator = scope.spawn(|| {
                for _ in 0..TOGGLE_ITERATIONS {
                    let mut trees = store.0.write();
                    let entry = trees.get_mut(&a).expect("A 엔트리는 유지되어야 한다");
                    service::toggle_expand(entry, &sub_a, &mut DirectoryListings::default()).expect("toggle");
                }
            });
            let reader = scope.spawn(|| {
                for _ in 0..TOGGLE_ITERATIONS {
                    store.remove(&b);
                    rows_page_from_store(&store, &state, &b, 0, None, &mut DirectoryListings::default()).expect("rows");
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
        assert!(
            trees.contains_key(&b),
            "조회 미스가 삽입한 B 엔트리가 뮤테이션에 의해 지워지면 안 된다 — 뮤테이션 경로에 전체 되쓰기가 남아 있으면 실패할 수 있다"
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
