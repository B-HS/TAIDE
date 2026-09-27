use taide_model::app_event::AppEvent;

pub mod agent_actions;
pub mod ai_actions;
mod ai_request_store;
pub mod app_actions;
mod app_services;
mod exit_drain;
pub mod file_actions;
pub mod git_actions;
pub mod ide_actions;
pub mod layout_actions;
pub mod locale_actions;
pub mod lsp_install_actions;
pub mod lsp_install_toolchain;
pub mod notification_actions;
mod platform_services;
pub mod plugin_actions;
pub mod project_actions;
pub mod remote_actions;
mod remote_dispatch_limiter;
pub mod search_actions;
mod search_store;
pub mod settings_actions;
mod state;
pub mod system_actions;
mod task_supervisor;
pub mod terminal_actions;
pub mod theme_actions;
pub mod tree_actions;
mod tree_store;
pub mod vsix_actions;
mod window_registry;

pub use ai_request_store::{AiRequestStore, AiRequestToken};
pub use app_services::AppServices;
pub use exit_drain::ExitDrain;
pub use file_actions::{save_file_within_open_projects, IdeSaveFile};
pub use platform_services::{PlatformServices, PlatformServicesState};
pub use remote_dispatch_limiter::RemoteDispatchLimiter;
pub use search_store::SearchStore;
pub use state::{AppState, AppStateInner, FlushScope, FlushTicket};
pub use task_supervisor::TaskSupervisor;
pub use tree_store::TreeStore;
pub use window_registry::WindowRegistry;

pub trait EventSink: Send + Sync {
    fn publish(&self, event: AppEvent);
}
