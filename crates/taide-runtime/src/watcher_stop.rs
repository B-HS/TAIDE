use std::sync::{Arc, Mutex};

use tokio::sync::watch;

#[derive(Clone)]
pub struct WatcherStopTracker(Arc<WatcherStopInner>);

struct WatcherStopInner {
    pending: Mutex<usize>,
    changed: watch::Sender<usize>,
}

struct PendingStop {
    tracker: WatcherStopTracker,
}

impl Drop for PendingStop {
    fn drop(&mut self) {
        let mut pending = self.tracker.0.pending.lock().expect("watcher stop lock poisoned");
        *pending -= 1;
        self.tracker.0.changed.send_replace(*pending);
    }
}

impl Default for WatcherStopTracker {
    fn default() -> Self {
        let (changed, _) = watch::channel(0);
        Self(Arc::new(WatcherStopInner {
            pending: Mutex::new(0),
            changed,
        }))
    }
}

impl WatcherStopTracker {
    /// Starts a blocking watcher stop and retains its completion through request cancellation.
    pub fn schedule(&self, stop: Box<dyn FnOnce() + Send>) {
        let mut pending = self.0.pending.lock().expect("watcher stop lock poisoned");
        *pending = pending.checked_add(1).expect("watcher stop count exhausted");
        self.0.changed.send_replace(*pending);
        drop(pending);

        let ticket = PendingStop { tracker: self.clone() };
        std::thread::spawn(move || {
            let _ticket = ticket;
            stop();
        });
    }

    /// Waits until every scheduled watcher stop has finished its callback and thread join.
    pub async fn wait_for_idle(&self) {
        let mut changed = self.0.changed.subscribe();
        loop {
            let pending = *changed.borrow_and_update();
            if pending == 0 {
                return;
            }
            changed.changed().await.expect("watcher stop tracker sender dropped");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::WatcherStopTracker;

    const WAIT_PROBE: Duration = Duration::from_millis(20);
    const FIXTURE_TIMEOUT: Duration = Duration::from_secs(2);

    #[tokio::test]
    async fn 중지_대기자_drop_뒤에도_실제_작업을_다시_기다린다() {
        let tracker = WatcherStopTracker::default();
        let (release, release_rx) = std::sync::mpsc::channel();
        tracker.schedule(Box::new(move || {
            release_rx.recv().unwrap();
        }));

        let first_tracker = tracker.clone();
        let mut first = tokio::spawn(async move { first_tracker.wait_for_idle().await });
        assert!(tokio::time::timeout(WAIT_PROBE, &mut first).await.is_err());
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());

        let second = tracker.wait_for_idle();
        tokio::pin!(second);
        assert!(tokio::time::timeout(WAIT_PROBE, &mut second).await.is_err());
        release.send(()).unwrap();
        tokio::time::timeout(FIXTURE_TIMEOUT, second).await.unwrap();
    }
}
