use std::sync::Arc;

use tokio::sync::{Semaphore, SemaphorePermit};

#[derive(Clone)]
pub struct RemoteDispatchLimiter {
    semaphore: Arc<Semaphore>,
}

impl RemoteDispatchLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn acquire(&self) -> Option<SemaphorePermit<'_>> {
        self.semaphore.acquire().await.ok()
    }
}
