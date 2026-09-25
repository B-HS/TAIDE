use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

#[derive(Default)]
pub struct SearchStore(Mutex<HashMap<(String, String), Arc<AtomicBool>>>);

impl SearchStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&self, owner: &str, session_id: &str) -> Arc<AtomicBool> {
        let key = (owner.to_string(), session_id.to_string());
        let mut sessions = self.0.lock();
        if let Some(previous) = sessions.get(&key) {
            previous.store(true, Ordering::SeqCst);
        }

        let cancelled = Arc::new(AtomicBool::new(false));
        sessions.insert(key, cancelled.clone());
        cancelled
    }

    pub fn finish(&self, owner: &str, session_id: &str, own_flag: &Arc<AtomicBool>) {
        let key = (owner.to_string(), session_id.to_string());
        let mut sessions = self.0.lock();
        if sessions.get(&key).is_some_and(|current| Arc::ptr_eq(current, own_flag)) {
            sessions.remove(&key);
        }
    }

    pub fn cancel(&self, owner: &str, session_id: &str) {
        if let Some(cancelled) = self.0.lock().get(&(owner.to_string(), session_id.to_string())) {
            cancelled.store(true, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 서로_다른_세션은_서로의_검색을_취소하지_않는다() {
        let store = SearchStore::new();
        let panel_flag = store.begin("main", "panel");
        let editor_flag = store.begin("main", "search-editor:tab-1");

        assert!(!panel_flag.load(Ordering::SeqCst));
        assert!(!editor_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn 같은_세션의_재검색은_이전_실행을_취소한다() {
        let store = SearchStore::new();
        let first = store.begin("main", "panel");
        let second = store.begin("main", "panel");

        assert!(first.load(Ordering::SeqCst));
        assert!(!second.load(Ordering::SeqCst));
    }

    #[test]
    fn 검색이_끝나면_스토어에서_세션_항목이_제거된다() {
        let store = SearchStore::new();
        let flag = store.begin("main", "panel");

        store.finish("main", "panel", &flag);

        assert!(!store.0.lock().contains_key(&("main".to_string(), "panel".to_string())));
    }

    #[test]
    fn 이미_새_실행으로_대체된_세션_항목은_종료_시_제거되지_않는다() {
        let store = SearchStore::new();
        let stale = store.begin("main", "panel");
        let fresh = store.begin("main", "panel");

        store.finish("main", "panel", &stale);

        let current = store.0.lock().get(&("main".to_string(), "panel".to_string())).cloned();
        assert!(current.is_some_and(|entry| Arc::ptr_eq(&entry, &fresh)));
    }

    #[test]
    fn 검색_취소는_해당_세션의_플래그만_설정한다() {
        let store = SearchStore::new();
        let panel_flag = store.begin("main", "panel");
        let editor_flag = store.begin("main", "search-editor:tab-1");

        store.cancel("main", "panel");

        assert!(panel_flag.load(Ordering::SeqCst));
        assert!(!editor_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn 서로_다른_창의_같은_session_id는_서로의_검색을_취소하지_않는다() {
        let store = SearchStore::new();
        let main_flag = store.begin("main", "search-panel-:r0:");
        let second_window_flag = store.begin("editor-2", "search-panel-:r0:");

        assert!(!main_flag.load(Ordering::SeqCst));
        assert!(!second_window_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn 서로_다른_창의_같은_session_id_취소는_해당_창의_플래그만_설정한다() {
        let store = SearchStore::new();
        let main_flag = store.begin("main", "search-panel-:r0:");
        let second_window_flag = store.begin("editor-2", "search-panel-:r0:");

        store.cancel("editor-2", "search-panel-:r0:");

        assert!(second_window_flag.load(Ordering::SeqCst));
        assert!(!main_flag.load(Ordering::SeqCst));
    }
}
