use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use taide_model::ids::ProjectId;
use taide_tree::service::TreeState;

#[derive(Clone)]
pub struct TreeStore(pub Arc<RwLock<HashMap<ProjectId, TreeState>>>);

impl TreeStore {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }

    pub fn remove(&self, project_id: &ProjectId) {
        self.0.write().remove(project_id);
    }
}

impl Default for TreeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 복제한_트리_캐시는_같은_프로젝트_항목을_공유하고_종료시_제거한다() {
        let store = TreeStore::new();
        let clone = store.clone();
        let project_id = ProjectId::new();
        store
            .0
            .write()
            .insert(project_id.clone(), taide_tree::service::new_tree_state(std::env::temp_dir()));

        clone.remove(&project_id);

        assert!(!store.0.read().contains_key(&project_id));
    }
}
