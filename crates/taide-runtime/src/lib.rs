use taide_model::app_event::AppEvent;

mod ai_request_store;
mod app_services;
mod file_actions;
mod platform_services;
mod remote_dispatch_limiter;
mod search_store;
mod state;
mod task_supervisor;
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
