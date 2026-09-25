use super::{AiRequestStore, AppState, SearchStore, TaskSupervisor};

pub struct AppServices {
    pub state: AppState,
    pub search: SearchStore,
    pub ai_requests: AiRequestStore,
    pub tasks: TaskSupervisor,
}

impl AppServices {
    pub fn new(state: AppState, search: SearchStore, ai_requests: AiRequestStore, tasks: TaskSupervisor) -> Self {
        Self {
            state,
            search,
            ai_requests,
            tasks,
        }
    }
}
