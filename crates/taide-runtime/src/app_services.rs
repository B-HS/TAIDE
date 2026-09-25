use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_plugin::service::PluginStore;
use taide_terminal::store::TerminalStore;

use super::{AiRequestStore, AppState, SearchStore, TaskSupervisor, TreeStore};

pub struct AppServices {
    pub state: AppState,
    pub search: SearchStore,
    pub ai_requests: AiRequestStore,
    pub tree: TreeStore,
    pub terminal: TerminalStore,
    pub plugin: PluginStore,
    pub lsp: LspStore,
    pub lsp_install: LspInstallStore,
    pub tasks: TaskSupervisor,
}

impl AppServices {
    pub fn new(state: AppState, tasks: TaskSupervisor) -> Self {
        Self {
            state,
            search: SearchStore::default(),
            ai_requests: AiRequestStore::default(),
            tree: TreeStore::default(),
            terminal: TerminalStore::default(),
            plugin: PluginStore::default(),
            lsp: LspStore::default(),
            lsp_install: LspInstallStore::default(),
            tasks,
        }
    }
}
