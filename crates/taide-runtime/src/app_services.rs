use super::{AppState, SearchStore, TaskSupervisor};

pub struct AppServices {
    pub state: AppState,
    pub search: SearchStore,
    pub tasks: TaskSupervisor,
}

impl AppServices {
    pub fn new(state: AppState, search: SearchStore, tasks: TaskSupervisor) -> Self {
        Self { state, search, tasks }
    }
}
