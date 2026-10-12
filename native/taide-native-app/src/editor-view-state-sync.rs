use std::sync::{Arc, Mutex};

use taide_model::error::{AppError, AppResult};
use taide_runtime::AppServices;
use tokio::sync::oneshot;

use crate::editor_view_state::Captured;

pub(crate) type Captures = AppResult<Arc<[Captured]>>;

#[derive(Default)]
struct Pending {
    repaint: Option<Arc<dyn Fn() + Send + Sync>>,
    waiters: Vec<oneshot::Sender<Captures>>,
    final_capture: Option<Captures>,
}

#[derive(Default)]
pub(crate) struct Shared {
    pending: Mutex<Pending>,
}

impl Shared {
    pub(crate) fn bind(&self, repaint: Arc<dyn Fn() + Send + Sync>) {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        pending.repaint = Some(repaint);
        pending.final_capture = None;
    }

    pub(crate) fn resume(&self) {
        self.pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .final_capture = None;
    }

    pub(crate) fn has_pending(&self) -> bool {
        !self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .waiters
            .is_empty()
    }

    pub(crate) fn publish(&self, captures: Captures) {
        let waiters = {
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            std::mem::take(&mut pending.waiters)
        };
        for waiter in waiters {
            drop(waiter.send(captures.clone()));
        }
    }

    pub(crate) fn finish(&self, captures: Captures) {
        let waiters = {
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            pending.final_capture = Some(captures.clone());
            std::mem::take(&mut pending.waiters)
        };
        for waiter in waiters {
            drop(waiter.send(captures.clone()));
        }
    }

    pub(crate) async fn request(&self) -> Captures {
        let (sender, receiver) = oneshot::channel();
        let repaint = {
            let mut pending = self
                .pending
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(captures) = &pending.final_capture {
                return captures.clone();
            }
            let Some(repaint) = pending.repaint.clone() else {
                return Ok(Arc::from([]));
            };
            pending.waiters.push(sender);
            repaint
        };
        repaint();
        receiver
            .await
            .map_err(|_| AppError::Internal("native view state capture owner stopped".into()))?
    }

    pub(crate) async fn flush(&self, services: &AppServices) -> AppResult<()> {
        let captures = self.request().await?;
        crate::editor_view_state::apply(services, &captures).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;

    const REQUEST_COUNT: usize = 2;
    const TIMEOUT: Duration = Duration::from_secs(1);

    #[tokio::test]
    async fn 소유자_없는_포트와_동시_요청은_대기와_캡처_소유를_정확히_처리한다() {
        let shared = Shared::default();
        assert!(shared.request().await.unwrap().is_empty());
        assert!(!shared.has_pending());
        let repaint = Arc::new(AtomicUsize::new(0));
        let counter = repaint.clone();
        shared.bind(Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        let captures = Arc::<[Captured]>::from([]);
        let (first, second, ()) = tokio::time::timeout(TIMEOUT, async {
            tokio::join!(shared.request(), shared.request(), async {
                while repaint.load(Ordering::SeqCst) < REQUEST_COUNT {
                    tokio::task::yield_now().await;
                }
                assert!(shared.has_pending());
                shared.publish(Ok(captures.clone()));
            })
        })
        .await
        .unwrap();
        assert!(Arc::ptr_eq(&first.unwrap(), &captures));
        assert!(Arc::ptr_eq(&second.unwrap(), &captures));
        assert!(!shared.has_pending());
    }

    #[tokio::test]
    async fn 취소와_최종_오류는_회수되고_종료_취소_뒤_새_요청은_다시_live_캡처를_받는다() {
        tokio::time::timeout(TIMEOUT, async {
            let shared = Shared::default();
            let repaint = Arc::new(AtomicUsize::new(0));
            let counter = repaint.clone();
            shared.bind(Arc::new(move || { counter.fetch_add(1, Ordering::SeqCst); }));
            tokio::select! {
                _ = shared.request() => panic!("capture must wait for its UI owner"),
                _ = async { while repaint.load(Ordering::SeqCst) == 0 { tokio::task::yield_now().await; } } => {}
            }
            shared.publish(Ok(Arc::from([])));
            assert!(!shared.has_pending());
            let (pending, ()) = tokio::join!(shared.request(), async {
                while repaint.load(Ordering::SeqCst) < REQUEST_COUNT { tokio::task::yield_now().await; }
                shared.finish(Err(AppError::InvalidArgument("synthetic capture failure".into())));
            });
            assert!(matches!(pending, Err(AppError::InvalidArgument(_))));
            assert!(matches!(shared.request().await, Err(AppError::InvalidArgument(_))));
            assert!(!shared.has_pending());
            shared.resume();
            let (resumed, ()) = tokio::join!(shared.request(), async {
                while !shared.has_pending() { tokio::task::yield_now().await; }
                shared.publish(Ok(Arc::from([])));
            });
            assert!(resumed.unwrap().is_empty());
            assert!(!shared.has_pending());
        }).await.unwrap();
    }
}
