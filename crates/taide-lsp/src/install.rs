use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use taide_model::lsp::LspServerId;

/// Tracks one active installer per language server and its cancellation token.
#[derive(Clone, Default)]
pub struct LspInstallStore(Arc<Mutex<HashMap<LspServerId, Arc<AtomicBool>>>>);

impl LspInstallStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&self, server_id: &LspServerId) -> Option<LspInstallGuard<'_>> {
        let mut active = self.0.lock();
        if active.contains_key(server_id) {
            return None;
        }
        let cancel = Arc::new(AtomicBool::new(false));
        active.insert(server_id.clone(), cancel.clone());
        Some(LspInstallGuard {
            store: self,
            server_id: server_id.clone(),
            cancel,
        })
    }

    pub fn cancel(&self, server_id: &LspServerId) {
        if let Some(cancel) = self.0.lock().get(server_id) {
            cancel.store(true, Ordering::SeqCst);
        }
    }

    fn finish(&self, server_id: &LspServerId, cancel: &Arc<AtomicBool>) {
        let mut active = self.0.lock();
        if active
            .get(server_id)
            .is_some_and(|existing| Arc::ptr_eq(existing, cancel))
        {
            active.remove(server_id);
        }
    }
}

/// Releases an installation slot when an installer finishes or unwinds.
pub struct LspInstallGuard<'a> {
    store: &'a LspInstallStore,
    server_id: LspServerId,
    cancel: Arc<AtomicBool>,
}

impl LspInstallGuard<'_> {
    pub fn cancellation_token(&self) -> Arc<AtomicBool> {
        self.cancel.clone()
    }
}

impl Drop for LspInstallGuard<'_> {
    fn drop(&mut self) {
        self.store.finish(&self.server_id, &self.cancel);
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use super::*;

    #[test]
    fn 설치_작업_패닉에도_슬롯을_해제한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = store.begin(&server_id).expect("첫 설치는 슬롯을 얻는다");
            panic!("설치 도중 패닉");
        }));

        assert!(panicked.is_err());
        assert!(store.begin(&server_id).is_some());
    }

    #[test]
    fn 설치_작업_정상_종료에도_슬롯을_해제한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");

        {
            let _guard = store.begin(&server_id).expect("첫 설치는 슬롯을 얻는다");
        }

        assert!(store.begin(&server_id).is_some());
    }

    #[test]
    fn 대기_중인_설치_작업이_취소되면_슬롯을_해제한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let mut installer = Box::pin(async {
            let _guard = store.begin(&server_id).expect("첫 설치는 슬롯을 얻는다");
            std::future::pending::<()>().await;
        });
        let mut context = Context::from_waker(Waker::noop());

        assert_eq!(installer.as_mut().poll(&mut context), Poll::Pending);
        assert!(store.begin(&server_id).is_none());

        drop(installer);
        assert!(store.begin(&server_id).is_some());
    }

    #[test]
    fn 복제한_저장소는_설치_취소와_슬롯_해제를_공유한다() {
        let store = LspInstallStore::new();
        let legacy_store = store.clone();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).expect("첫 설치 슬롯");
        let cancellation_token = guard.cancellation_token();

        assert!(legacy_store.begin(&server_id).is_none());
        legacy_store.cancel(&server_id);
        assert!(cancellation_token.load(Ordering::SeqCst));

        drop(guard);
        assert!(legacy_store.begin(&server_id).is_some());
    }
}
