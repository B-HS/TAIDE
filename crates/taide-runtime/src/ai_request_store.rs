use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::{oneshot, Notify};

type AiRequests = HashMap<(String, String), (Arc<()>, oneshot::Sender<()>)>;

pub struct AiRequestToken {
    store: AiRequestStore,
    key: (String, String),
    identity: Arc<()>,
}

impl Drop for AiRequestToken {
    fn drop(&mut self) {
        let removed = {
            let mut state = self.store.0.state.lock();
            let removed = if state
                .requests
                .get(&self.key)
                .is_some_and(|(current, _)| Arc::ptr_eq(current, &self.identity))
            {
                state.requests.remove(&self.key)
            } else {
                None
            };
            removed
        };
        drop(removed);
        self.store
            .0
            .state
            .lock()
            .active
            .retain(|identity| !Arc::ptr_eq(identity, &self.identity));
        self.store.0.changed.notify_waiters();
    }
}

#[derive(Default)]
struct AiRequestState {
    requests: AiRequests,
    active: Vec<Arc<()>>,
    is_stopped: bool,
}

#[derive(Default)]
struct AiRequestStoreInner {
    state: Mutex<AiRequestState>,
    changed: Notify,
}

#[derive(Clone, Default)]
pub struct AiRequestStore(Arc<AiRequestStoreInner>);

impl AiRequestStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&self, owner: &str, request_id: &str) -> Option<(AiRequestToken, oneshot::Receiver<()>)> {
        let mut store = self.0.state.lock();
        let key = (owner.to_string(), request_id.to_string());
        if store.is_stopped || store.requests.contains_key(&key) {
            return None;
        }

        let token = AiRequestToken {
            store: self.clone(),
            key: key.clone(),
            identity: Arc::new(()),
        };
        let (sender, receiver) = oneshot::channel();
        store.requests.insert(key, (token.identity.clone(), sender));
        store.active.push(token.identity.clone());
        Some((token, receiver))
    }

    pub fn finish(&self, owner: &str, request_id: &str, token: &AiRequestToken) {
        let key = (owner.to_string(), request_id.to_string());
        let removed = {
            let mut store = self.0.state.lock();
            if store
                .requests
                .get(&key)
                .is_some_and(|(current, _)| Arc::ptr_eq(current, &token.identity))
            {
                store.requests.remove(&key)
            } else {
                None
            }
        };
        drop(removed);
    }

    pub fn cancel(&self, owner: &str, request_id: &str) {
        let removed = self.0.state.lock().requests.remove(&(owner.to_string(), request_id.to_string()));
        if let Some((_, sender)) = removed {
            let _ = sender.send(());
        }
    }

    /// Closes admission and cancels registered requests without claiming their owners have dropped.
    pub fn shutdown(&self) {
        let requests = {
            let mut state = self.0.state.lock();
            state.is_stopped = true;
            std::mem::take(&mut state.requests)
        };
        for (_, (_, sender)) in requests {
            let _ = sender.send(());
        }
    }

    /// Waits for all request owners, including cancelled and manually finished identities, to drop.
    pub async fn wait_for_idle(&self) {
        loop {
            let notified = self.0.changed.notified();
            if self.0.state.lock().active.is_empty() {
                return;
            }
            notified.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const PENDING_PROBE_MS: u64 = 20;
    const FIXTURE_TIMEOUT_MS: u64 = 1_000;

    #[test]
    fn request_future_drop은_finish_없이도_같은_key의_재시작을_허용한다() {
        let store = AiRequestStore::new();
        let (token, receiver) = store.begin("main", "dropped").unwrap();
        let future = async move {
            let _token = token;
            receiver.await.ok();
        };
        drop(future);
        assert!(store.begin("main", "dropped").is_some());
    }

    #[test]
    fn 취소된_old_token_drop은_새_요청을_지우지_않는다() {
        let store = AiRequestStore::new();
        let (old, _) = store.begin("main", "request").unwrap();
        store.cancel("main", "request");
        let _new = store.begin("main", "request").unwrap();
        drop(old);
        assert!(store.begin("main", "request").is_none());
    }

    #[tokio::test]
    async fn poll된_요청_task의_abort_완료는_finish_없이_owner와_key를_회수한다() {
        let store = AiRequestStore::new();
        let (owner, pending) = store.begin("main", "request").unwrap();
        let (started, started_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            let _owner = owner;
            started.send(()).ok();
            pending.await.ok();
        });
        started_rx.await.unwrap();
        task.abort();
        let error = tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), task)
            .await
            .unwrap()
            .unwrap_err();
        assert!(error.is_cancelled());
        assert!(store.begin("main", "request").is_some());
        store.shutdown();
        store.wait_for_idle().await;
    }

    #[tokio::test]
    async fn shutdown은_입장을_닫고_취소를_전달하지만_실제_owner_drop까지_기다린다() {
        let store = AiRequestStore::new();
        let (token, receiver) = store.begin("main", "request").unwrap();
        store.shutdown();
        assert!(receiver.await.is_ok());
        assert!(store.begin("other", "new").is_none());
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), store.wait_for_idle())
            .await
            .is_err());
        drop(token);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), store.wait_for_idle())
            .await
            .unwrap();
        store.shutdown();
        store.wait_for_idle().await;
    }

    #[tokio::test]
    async fn 취소와_수동_finish로_registry가_비어도_모든_identity_drop을_기다린다() {
        let store = AiRequestStore::new();
        let (old, _) = store.begin("main", "request").unwrap();
        store.cancel("main", "request");
        let (new, _) = store.begin("main", "request").unwrap();
        store.finish("main", "request", &new);
        store.shutdown();
        drop(new);
        assert!(tokio::time::timeout(Duration::from_millis(PENDING_PROBE_MS), store.wait_for_idle())
            .await
            .is_err());
        drop(old);
        store.wait_for_idle().await;
    }

    #[tokio::test]
    async fn 동시_idle_waiter는_마지막_drop_알림을_모두_받는다() {
        let store = AiRequestStore::new();
        let (token, _) = store.begin("main", "request").unwrap();
        store.shutdown();
        let first_store = store.clone();
        let second_store = store.clone();
        let first = tokio::spawn(async move { first_store.wait_for_idle().await });
        let second = tokio::spawn(async move { second_store.wait_for_idle().await });
        tokio::task::yield_now().await;
        drop(token);
        tokio::time::timeout(Duration::from_millis(FIXTURE_TIMEOUT_MS), async {
            first.await.unwrap();
            second.await.unwrap();
        })
        .await
        .unwrap();
    }

    #[test]
    fn 같은_owner의_같은_request_id로_두번_시작하면_두번째는_거부된다() {
        let store = AiRequestStore::new();
        let _first = store.begin("main", "req-1").expect("first begin");
        assert!(store.begin("main", "req-1").is_none());
    }

    #[test]
    fn finish_후에는_같은_owner의_같은_request_id를_다시_시작할_수_있다() {
        let store = AiRequestStore::new();
        let (token, _receiver) = store.begin("main", "req-1").expect("first begin");
        store.finish("main", "req-1", &token);
        assert!(store.begin("main", "req-1").is_some());
    }

    #[tokio::test]
    async fn cancel_은_대기중인_receiver를_깨운다() {
        let store = AiRequestStore::new();
        let (_token, receiver) = store.begin("main", "req-1").expect("begin");
        store.cancel("main", "req-1");
        assert!(receiver.await.is_ok());
    }

    #[test]
    fn 모르는_request_id를_취소해도_안전하다() {
        let store = AiRequestStore::new();
        store.cancel("main", "unknown");
    }

    #[test]
    fn 취소된_요청의_늦은_완료는_새_요청을_제거하지_않는다() {
        let store = AiRequestStore::new();
        let (old_token, _old_receiver) = store.begin("main", "req-1").expect("old request");
        store.cancel("main", "req-1");
        let _new = store.begin("main", "req-1").expect("new request");

        store.finish("main", "req-1", &old_token);

        assert!(store.begin("main", "req-1").is_none());
    }

    #[tokio::test]
    async fn 서로_다른_owner의_같은_request_id는_서로_충돌하지_않는다() {
        let store = AiRequestStore::new();
        let _main = store.begin("main", "req-1").expect("main request");
        let (_editor_token, editor_receiver) = store.begin("editor-2", "req-1").expect("editor request");

        store.cancel("editor-2", "req-1");

        assert!(editor_receiver.await.is_ok());
        assert!(store.begin("main", "req-1").is_none());
    }
}
