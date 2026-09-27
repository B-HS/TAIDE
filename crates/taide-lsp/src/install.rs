use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use parking_lot::Mutex;
use taide_model::error::{AppError, AppErrorKind, AppResult};
use taide_model::lsp::LspServerId;
use tokio::sync::Notify;

#[derive(Default)]
struct InstallState {
    active: HashMap<LspServerId, Arc<InstallControl>>,
    is_stopped: bool,
}

#[derive(Default)]
struct InstallGate {
    is_committed: bool,
    resources: Vec<Weak<dyn InstallCancellationResource>>,
}

/// Receives synchronous cancellation after admission and resource creation have been serialized.
pub trait InstallCancellationResource: Send + Sync {
    fn cancel(&self);
}

#[derive(Default)]
struct InstallControl {
    cancel: Arc<AtomicBool>,
    changed: Notify,
    gate: Mutex<InstallGate>,
}

impl InstallControl {
    fn cancel(&self) {
        let resources: Vec<_> = {
            let gate = self.gate.lock();
            if gate.is_committed || self.cancel.swap(true, Ordering::SeqCst) {
                return;
            }
            gate.resources.iter().filter_map(Weak::upgrade).collect()
        };
        self.changed.notify_waiters();
        for resource in resources {
            resource.cancel();
        }
    }
}

/// Tracks admission and cancellation until all workers release their installation leases.
#[derive(Clone, Default)]
pub struct LspInstallStore(Arc<Mutex<InstallState>>);

impl LspInstallStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&self, server_id: &LspServerId) -> Option<LspInstallGuard> {
        let mut state = self.0.lock();
        if state.is_stopped || state.active.contains_key(server_id) {
            return None;
        }
        let control = Arc::new(InstallControl::default());
        state.active.insert(server_id.clone(), control.clone());
        Some(LspInstallGuard {
            lease: LspInstallLease(Arc::new(InstallLeaseInner {
                store: self.clone(),
                server_id: server_id.clone(),
                control,
            })),
        })
    }

    pub fn cancel(&self, server_id: &LspServerId) {
        let control = self.0.lock().active.get(server_id).cloned();
        if let Some(control) = control {
            control.cancel();
        }
    }

    /// Closes admission and cancels active requests without releasing running workers' slots.
    pub fn shutdown(&self) {
        let controls: Vec<_> = {
            let mut state = self.0.lock();
            state.is_stopped = true;
            state.active.values().cloned().collect()
        };
        for control in controls {
            control.cancel();
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.0.lock().is_stopped
    }

    fn finish(&self, server_id: &LspServerId, control: &Arc<InstallControl>) {
        let mut state = self.0.lock();
        if state
            .active
            .get(server_id)
            .is_some_and(|existing| Arc::ptr_eq(existing, control))
        {
            state.active.remove(server_id);
        }
    }
}

struct InstallLeaseInner {
    store: LspInstallStore,
    server_id: LspServerId,
    control: Arc<InstallControl>,
}

impl Drop for InstallLeaseInner {
    fn drop(&mut self) {
        self.store.finish(&self.server_id, &self.control);
    }
}

/// Keeps an installation slot alive for a worker independently of its request future.
#[derive(Clone)]
pub struct LspInstallLease(Arc<InstallLeaseInner>);

impl LspInstallLease {
    pub fn cancellation_token(&self) -> Arc<AtomicBool> {
        self.0.control.cancel.clone()
    }

    pub fn ensure_active(&self) -> AppResult<()> {
        if self.0.control.cancel.load(Ordering::SeqCst) {
            return Err(install_cancelled_error());
        }
        Ok(())
    }

    /// Waits for store cancellation, shutdown, or request-owner drop.
    pub async fn cancelled(&self) {
        let notified = self.0.control.changed.notified();
        if self.0.control.cancel.load(Ordering::SeqCst) {
            return;
        }
        notified.await;
    }

    /// Creates and weakly registers a resource under the cancellation gate. The factory must not reenter this store.
    pub fn register_resource<R: InstallCancellationResource + 'static>(
        &self,
        create: impl FnOnce() -> AppResult<Arc<R>>,
    ) -> AppResult<Arc<R>> {
        let mut gate = self.0.control.gate.lock();
        self.ensure_active()?;
        if gate.is_committed {
            return Err(AppError::Internal(
                "installation has already been committed".to_string(),
            ));
        }
        let resource = create()?;
        let cancellation: Arc<dyn InstallCancellationResource> = resource.clone();
        gate.resources
            .retain(|resource| resource.strong_count() > 0);
        gate.resources.push(Arc::downgrade(&cancellation));
        Ok(resource)
    }

    /// Serializes final application with cancellation. The callback must not reenter this store.
    pub fn commit<T>(&self, apply: impl FnOnce() -> AppResult<T>) -> AppResult<T> {
        let mut gate = self.0.control.gate.lock();
        self.ensure_active()?;
        if gate.is_committed {
            return Err(AppError::Internal(
                "installation has already been committed".to_string(),
            ));
        }
        let result = apply()?;
        gate.is_committed = true;
        Ok(result)
    }
}

/// Cancels its request on drop; the slot is released only after every worker lease is dropped.
pub struct LspInstallGuard {
    lease: LspInstallLease,
}

impl LspInstallGuard {
    pub fn cancellation_token(&self) -> Arc<AtomicBool> {
        self.lease.cancellation_token()
    }

    pub fn lease(&self) -> LspInstallLease {
        self.lease.clone()
    }
}

impl Drop for LspInstallGuard {
    fn drop(&mut self) {
        self.lease.0.control.cancel();
    }
}

pub fn install_cancelled_error() -> AppError {
    AppError::localized(
        AppErrorKind::Internal,
        "error.lsp.installCancelled",
        "Installation was cancelled",
    )
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use super::*;

    struct TestResource {
        cancel_count: std::sync::atomic::AtomicUsize,
        lease: LspInstallLease,
    }

    impl InstallCancellationResource for TestResource {
        fn cancel(&self) {
            self.cancel_count.fetch_add(1, Ordering::SeqCst);
            assert!(self.lease.ensure_active().is_err());
        }
    }

    #[test]
    fn 취소_등록은_resource_생성과_직렬화하고_shutdown에서_동기_호출된다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let lease = guard.lease();
        let resource = lease
            .register_resource(|| {
                Ok(Arc::new(TestResource {
                    cancel_count: std::sync::atomic::AtomicUsize::new(0),
                    lease: lease.clone(),
                }))
            })
            .unwrap();
        store.shutdown();
        assert_eq!(resource.cancel_count.load(Ordering::SeqCst), 1);
        drop(guard);
        drop(lease);
        assert_eq!(store.0.lock().active.len(), 1);
        drop(resource);
        assert!(store.0.lock().active.is_empty());
    }

    #[test]
    fn 취소된_작업은_resource_factory를_실행하지_않는다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let lease = guard.lease();
        store.cancel(&server_id);
        let did_create = AtomicBool::new(false);
        let result = lease.register_resource(|| {
            did_create.store(true, Ordering::SeqCst);
            Ok(Arc::new(TestResource {
                cancel_count: std::sync::atomic::AtomicUsize::new(0),
                lease: lease.clone(),
            }))
        });
        assert!(result.is_err());
        assert!(!did_create.load(Ordering::SeqCst));
    }

    #[test]
    fn 요청_drop도_등록된_resource를_동기_취소하고_weak_등록은_수명을_늘리지_않는다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let lease = guard.lease();
        let resource = lease
            .register_resource(|| {
                Ok(Arc::new(TestResource {
                    cancel_count: std::sync::atomic::AtomicUsize::new(0),
                    lease: lease.clone(),
                }))
            })
            .unwrap();
        drop(guard);
        assert_eq!(resource.cancel_count.load(Ordering::SeqCst), 1);
        drop(resource);
        drop(lease);
        assert!(store.begin(&server_id).is_some());
    }

    #[test]
    fn 요청이_사라져도_worker가_종료될_때까지_설치_슬롯을_보유한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let worker = guard.lease();
        let cancellation_token = guard.cancellation_token();

        drop(guard);
        assert!(cancellation_token.load(Ordering::SeqCst));
        assert!(store.begin(&server_id).is_none());
        drop(worker);
        assert!(store.begin(&server_id).is_some());
    }

    #[test]
    fn 종료는_복제한_저장소의_작업을_취소하고_신규_등록을_거절한다() {
        let store = LspInstallStore::new();
        let other = store.clone();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let worker = guard.lease();

        other.shutdown();
        store.shutdown();
        assert!(store.is_stopped());
        assert!(guard.cancellation_token().load(Ordering::SeqCst));
        assert!(worker.ensure_active().is_err());
        drop(guard);
        assert_eq!(store.0.lock().active.len(), 1);
        drop(worker);
        assert!(store.0.lock().active.is_empty());
        assert!(store.begin(&server_id).is_none());
        assert!(store.begin(&LspServerId::from("another-server")).is_none());
    }

    #[test]
    fn 취소가_적용보다_앞서면_적용_callback을_실행하지_않는다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let worker = guard.lease();
        let applied = AtomicBool::new(false);

        store.cancel(&server_id);
        let result = worker.commit(|| {
            applied.store(true, Ordering::SeqCst);
            Ok(())
        });
        assert!(result.is_err());
        assert!(!applied.load(Ordering::SeqCst));
    }

    #[test]
    fn 적용이_완료된_작업은_늦은_취소로_완료_결과를_뒤집지_않는다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let worker = guard.lease();

        worker.commit(|| Ok(())).unwrap();
        store.cancel(&server_id);
        assert!(!guard.cancellation_token().load(Ordering::SeqCst));
        assert!(worker.commit(|| Ok(())).is_err());
    }

    #[tokio::test]
    async fn 취소_알림은_대기_전과_대기_중_취소를_모두_관찰한다() {
        let store = LspInstallStore::new();
        let server_id = LspServerId::from("test-server");
        let guard = store.begin(&server_id).unwrap();
        let worker = guard.lease();
        let mut cancellation = Box::pin(worker.cancelled());
        let mut context = Context::from_waker(Waker::noop());
        assert_eq!(cancellation.as_mut().poll(&mut context), Poll::Pending);
        store.cancel(&server_id);
        cancellation.await;
        worker.cancelled().await;
    }

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
