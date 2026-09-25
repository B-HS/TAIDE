use taide_model::app_event::AppEvent;

mod state;
mod task_supervisor;

pub use state::{AppState, FlushScope, FlushTicket};
pub use task_supervisor::TaskSupervisor;

pub trait EventSink: Send + Sync {
    fn publish(&self, event: AppEvent);
}
