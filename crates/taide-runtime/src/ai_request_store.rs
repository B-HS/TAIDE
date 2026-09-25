use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::oneshot;

type AiRequests = HashMap<(String, String), (AiRequestToken, oneshot::Sender<()>)>;

pub struct AiRequestToken(Arc<()>);

#[derive(Clone, Default)]
pub struct AiRequestStore(Arc<Mutex<AiRequests>>);

impl AiRequestStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&self, owner: &str, request_id: &str) -> Option<(AiRequestToken, oneshot::Receiver<()>)> {
        let mut store = self.0.lock();
        let key = (owner.to_string(), request_id.to_string());
        if store.contains_key(&key) {
            return None;
        }

        let token = AiRequestToken(Arc::new(()));
        let (sender, receiver) = oneshot::channel();
        store.insert(key, (AiRequestToken(token.0.clone()), sender));
        Some((token, receiver))
    }

    pub fn finish(&self, owner: &str, request_id: &str, token: &AiRequestToken) {
        let key = (owner.to_string(), request_id.to_string());
        let mut store = self.0.lock();
        if store.get(&key).is_some_and(|(current, _)| Arc::ptr_eq(&current.0, &token.0)) {
            store.remove(&key);
        }
    }

    pub fn cancel(&self, owner: &str, request_id: &str) {
        if let Some((_, sender)) = self.0.lock().remove(&(owner.to_string(), request_id.to_string())) {
            let _ = sender.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
