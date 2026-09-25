use super::{AiRequestStore, AppState, SearchStore, TaskSupervisor, TreeStore};

pub struct AppServices {
    pub state: AppState,
    pub search: SearchStore,
    pub ai_requests: AiRequestStore,
    pub tree: TreeStore,
    pub tasks: TaskSupervisor,
}

impl AppServices {
    pub fn new(state: AppState, search: SearchStore, ai_requests: AiRequestStore, tree: TreeStore, tasks: TaskSupervisor) -> Self {
        Self {
            state,
            search,
            ai_requests,
            tree,
            tasks,
        }
    }
}
