use taide_model::app_event::AppEvent;

mod ai_request_store;
mod app_services;
mod search_store;
mod state;
mod task_supervisor;
mod tree_store;

pub use ai_request_store::{AiRequestStore, AiRequestToken};
pub use app_services::AppServices;
pub use search_store::SearchStore;
pub use state::{AppState, AppStateInner, FlushScope, FlushTicket};
pub use task_supervisor::TaskSupervisor;
pub use tree_store::TreeStore;

pub trait EventSink: Send + Sync {
    fn publish(&self, event: AppEvent);
}
