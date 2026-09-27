use std::sync::Arc;

use taide_agent::store::{AgentHooksStore, AgentStore};
use taide_git::store::GitStore;
use taide_ide::store::IdeStore;
use taide_infra::secret::SecretStoreState;
use taide_lsp::install::LspInstallStore;
use taide_lsp::store::LspStore;
use taide_plugin::service::PluginStore;
use taide_remote::store::RemoteStore;
use taide_system::store::SystemUsageStore;
use taide_terminal::store::TerminalStore;

use super::{
    AiRequestStore, AppState, EventSink, IdeSaveFile, PlatformServicesState, RemoteDispatchLimiter, SearchStore, TaskSupervisor, TreeStore,
    WindowRegistry,
};

pub struct AppServices {
    pub state: AppState,
    pub search: SearchStore,
    pub ai_requests: AiRequestStore,
    pub tree: TreeStore,
    pub terminal: TerminalStore,
    pub plugin: PluginStore,
    pub agents: AgentStore,
    pub git: GitStore,
    pub remote: RemoteStore,
    pub ide: IdeStore,
    pub secrets: SecretStoreState,
    pub ide_save_file: IdeSaveFile,
    pub agent_hooks: AgentHooksStore,
    pub lsp: LspStore,
    pub lsp_install: LspInstallStore,
    pub system_usage: SystemUsageStore,
    pub remote_dispatch_limiter: RemoteDispatchLimiter,
    pub platform: PlatformServicesState,
    pub events: Arc<dyn EventSink>,
    pub windows: WindowRegistry,
    pub tasks: TaskSupervisor,
}

impl AppServices {
    pub fn new(
        state: AppState,
        tasks: TaskSupervisor,
        remote_dispatch_limiter: RemoteDispatchLimiter,
        platform: PlatformServicesState,
        secrets: SecretStoreState,
        ide_save_file: IdeSaveFile,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            state,
            search: SearchStore::default(),
            ai_requests: AiRequestStore::default(),
            tree: TreeStore::default(),
            terminal: TerminalStore::default(),
            plugin: PluginStore::default(),
            agents: AgentStore::default(),
            git: GitStore::default(),
            remote: RemoteStore::default(),
            ide: IdeStore::default(),
            secrets,
            ide_save_file,
            agent_hooks: AgentHooksStore::default(),
            lsp: LspStore::default(),
            lsp_install: LspInstallStore::default(),
            system_usage: SystemUsageStore::default(),
            remote_dispatch_limiter,
            platform,
            events,
            windows: WindowRegistry::default(),
            tasks,
        }
    }
}
