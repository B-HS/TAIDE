use taide_model::app_event::AppEvent;

mod ai_request_store;
pub mod app_actions;
mod app_services;
pub mod file_actions;
pub mod layout_actions;
pub mod locale_actions;
pub mod lsp_install_actions;
pub mod notification_actions;
mod platform_services;
mod remote_dispatch_limiter;
pub mod search_actions;
mod search_store;
pub mod settings_actions;
mod state;
pub mod system_actions;
mod task_supervisor;
pub mod theme_actions;
pub mod tree_actions;
mod tree_store;
mod window_registry;

pub use ai_request_store::{AiRequestStore, AiRequestToken};
pub use app_services::AppServices;
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
