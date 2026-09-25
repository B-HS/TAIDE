use taide_model::app_event::AppEvent;

pub trait EventSink: Send + Sync {
    fn publish(&self, event: AppEvent);
}
