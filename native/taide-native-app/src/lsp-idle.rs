use std::time::Duration;

use tokio::time::Instant;

const SESSION_DISPOSE_GRACE: Duration = Duration::from_secs(5);

#[derive(Clone, Default)]
pub(super) struct Idle {
    deadline: Option<Instant>,
}

impl Idle {
    pub(super) fn release(&mut self, now: Instant) {
        self.deadline.get_or_insert(now + SESSION_DISPOSE_GRACE);
    }

    pub(super) fn acquire(&mut self) {
        self.deadline = None;
    }

    pub(super) fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    pub(super) fn ready(&self, now: Instant) -> bool {
        self.deadline.is_some_and(|deadline| deadline <= now)
    }
}

pub(super) async fn wait(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEFORE_DEADLINE: Duration = Duration::from_millis(4999);
    const DEADLINE_STEP: Duration = Duration::from_millis(1);

    #[tokio::test(start_paused = true)]
    async fn 세션_유예는_5초이며_반복_release로_밀리지_않고_acquire가_취소한다() {
        let mut idle = Idle::default();
        assert!(idle.deadline().is_none());
        idle.release(Instant::now());
        let first = idle.deadline();
        tokio::time::advance(BEFORE_DEADLINE).await;
        assert!(!idle.ready(Instant::now()));
        idle.release(Instant::now());
        assert_eq!(idle.deadline(), first);
        idle.acquire();
        tokio::time::advance(DEADLINE_STEP).await;
        assert!(!idle.ready(Instant::now()));
        idle.release(Instant::now());
        let second = idle.deadline().unwrap();
        assert!(Some(second) > first);
        wait(Some(second)).await;
        assert!(idle.ready(Instant::now()));
    }
}
