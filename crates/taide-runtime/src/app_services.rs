use taide_lsp::store::LspStore;
use taide_plugin::service::PluginStore;

use super::{AiRequestStore, AppState, SearchStore, TaskSupervisor, TreeStore};

pub struct AppServices {
    pub state: AppState,
    pub search: SearchStore,
    pub ai_requests: AiRequestStore,
    pub tree: TreeStore,
    pub plugin: PluginStore,
    pub lsp: LspStore,
    pub tasks: TaskSupervisor,
}

impl AppServices {
    pub fn new(
        state: AppState,
        search: SearchStore,
        ai_requests: AiRequestStore,
        tree: TreeStore,
        plugin: PluginStore,
        lsp: LspStore,
        tasks: TaskSupervisor,
    ) -> Self {
        Self {
            state,
            search,
            ai_requests,
            tree,
            plugin,
            lsp,
            tasks,
        }
    }
}
