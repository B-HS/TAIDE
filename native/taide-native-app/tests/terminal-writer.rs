use std::future::{Future, poll_fn};
use std::sync::{Arc, Condvar, Mutex};
use std::task::Poll;
use std::time::Duration;

use taide_model::error::AppError;
use taide_native_app::terminal_writer::{Limits, Submission, Writer};
use taide_runtime::TaskSupervisor;
use tokio::sync::oneshot;
use tokio::time::timeout;

const BYTES: usize = 4096;
const COUNT: usize = 2;
const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Default)]
struct Gate(Arc<(Mutex<bool>, Condvar)>);

impl Gate {
    fn wait(&self) {
        let (lock, changed) = self.0.as_ref();
        let mut is_open = lock.lock().unwrap();
        while !*is_open {
            is_open = changed.wait(is_open).unwrap();
        }
    }

    fn open(&self) {
        let (lock, changed) = self.0.as_ref();
        *lock.lock().unwrap() = true;
        changed.notify_all();
    }
}

struct Release(Gate);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.open();
    }
}

#[tokio::test]
async fn waiting_writer는_byte_대기_취소와_close에서_permit을_회수한다() {
    const PAYLOAD_BYTES: usize = BYTES / COUNT;
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let gate = Gate::default();
    let _release = Release(gate.clone());
    let blocked = gate.clone();
    let (started, ready) = oneshot::channel();
    let started = Mutex::new(Some(started));
    let (writer, worker) = Writer::start(
        &tasks,
        Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |_| {
            if let Some(started) = started.lock().unwrap().take() {
                let _ = started.send(());
            }
            blocked.wait();
            Ok(())
        },
    )
    .unwrap();
    let mut oversized = Vec::with_capacity(BYTES);
    oversized.push(b'x');
    assert!(
        timeout(TIMEOUT, writer.submit_wait(oversized))
            .await
            .unwrap()
            .is_err()
    );
    let first = writer.submit(vec![b'a'; PAYLOAD_BYTES]).unwrap();
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    let mut cancelled = Box::pin(writer.submit_wait(vec![b'b'; PAYLOAD_BYTES]));
    assert!(poll_fn(|context| Poll::Ready(cancelled.as_mut().poll(context).is_pending())).await);
    drop(cancelled);
    let second = writer.submit(b"after-cancel".to_vec()).unwrap();
    let mut waiting = Box::pin(writer.submit_wait(b"waiting".to_vec()));
    assert!(poll_fn(|context| Poll::Ready(waiting.as_mut().poll(context).is_pending())).await);
    writer.close();
    assert!(timeout(TIMEOUT, waiting).await.unwrap().is_err());
    gate.open();
    timeout(TIMEOUT, first.wait()).await.unwrap().unwrap();
    assert!(timeout(TIMEOUT, second.wait()).await.unwrap().is_err());
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn writer는_cancelled_receipt의_write와_순서_inflight_count_capacity를_보존한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let gate = Gate::default();
    let _release = Release(gate.clone());
    let blocked = gate.clone();
    let writes = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let observed = writes.clone();
    let (started, ready) = oneshot::channel();
    let started = Mutex::new(Some(started));
    let (writer, worker) = Writer::start(
        &tasks,
        Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |data| {
            if data == b"one" {
                if let Some(started) = started.lock().unwrap().take() {
                    let _ = started.send(());
                }
                blocked.wait();
            }
            observed.lock().unwrap().push(data.to_vec());
            Ok(())
        },
    )
    .unwrap();
    let mut reserved = Vec::with_capacity(BYTES);
    reserved.push(b'x');
    assert!(writer.submit(reserved).is_err());
    let first = writer.submit(b"one".to_vec()).unwrap();
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    drop(first);
    let second = writer.submit(b"two".to_vec()).unwrap();
    let Submission::Pending(retry) = writer.try_submit(b"three".to_vec()).unwrap() else {
        panic!("full writer did not return pending bytes")
    };
    assert_eq!(retry, b"three");
    assert!(tasks.tracked_count() >= COUNT);
    gate.open();
    timeout(TIMEOUT, second.wait()).await.unwrap().unwrap();
    assert_eq!(*writes.lock().unwrap(), [b"one".to_vec(), b"two".to_vec()]);
    let Submission::Accepted(third) = writer.try_submit(retry).unwrap() else {
        panic!("available writer did not accept pending bytes")
    };
    timeout(TIMEOUT, third.wait()).await.unwrap().unwrap();
    assert_eq!(
        *writes.lock().unwrap(),
        [b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]
    );
    writer.close();
    assert!(writer.submit(b"after-close".to_vec()).is_err());
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    assert_eq!(tasks.tracked_count(), 0);
    assert!(
        Writer::start(
            &tasks,
            Limits {
                bytes: 0,
                count: COUNT
            },
            |_| Ok(())
        )
        .is_err()
    );
}

#[tokio::test]
async fn write_실패는_pending을_거절하고_permit을_반납한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let gate = Gate::default();
    let _release = Release(gate.clone());
    let blocked = gate.clone();
    let (started, ready) = oneshot::channel();
    let started = Mutex::new(Some(started));
    let (writer, worker) = Writer::start(
        &tasks,
        Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |_| {
            if let Some(started) = started.lock().unwrap().take() {
                let _ = started.send(());
            }
            blocked.wait();
            Err(AppError::Internal("synthetic write failure".into()))
        },
    )
    .unwrap();
    let first = writer.submit(b"one".to_vec()).unwrap();
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    let pending = writer.submit(b"pending".to_vec()).unwrap();
    gate.open();
    assert!(timeout(TIMEOUT, first.wait()).await.unwrap().is_err());
    assert!(timeout(TIMEOUT, pending.wait()).await.unwrap().is_err());
    timeout(TIMEOUT, worker).await.unwrap().unwrap();
    assert!(writer.submit(b"after-failure".to_vec()).is_err());
    assert_eq!(tasks.tracked_count(), 0);
}

#[tokio::test]
async fn root_stop은_시작된_write를_실제_완료까지_감독한다() {
    let tasks = TaskSupervisor::new(tokio::runtime::Handle::current());
    let gate = Gate::default();
    let _release = Release(gate.clone());
    let blocked = gate.clone();
    let (started, ready) = oneshot::channel();
    let started = Mutex::new(Some(started));
    let (writer, worker) = Writer::start(
        &tasks,
        Limits {
            bytes: BYTES,
            count: COUNT,
        },
        move |_| {
            if let Some(started) = started.lock().unwrap().take() {
                let _ = started.send(());
            }
            blocked.wait();
            Ok(())
        },
    )
    .unwrap();
    let first = writer.submit(b"one".to_vec()).unwrap();
    timeout(TIMEOUT, ready).await.unwrap().unwrap();
    let pending = writer.submit(b"pending".to_vec()).unwrap();
    let mut waiting = Box::pin(writer.submit_wait(b"waiting".to_vec()));
    assert!(poll_fn(|context| Poll::Ready(waiting.as_mut().poll(context).is_pending())).await);
    tasks.stop_all();
    assert!(timeout(TIMEOUT, worker).await.unwrap().is_err());
    assert!(timeout(TIMEOUT, waiting).await.unwrap().is_err());
    assert!(timeout(TIMEOUT, pending.wait()).await.unwrap().is_err());
    assert!(writer.submit(b"after-stop".to_vec()).is_err());
    assert_eq!(tasks.tracked_count(), 1);
    gate.open();
    timeout(TIMEOUT, first.wait()).await.unwrap().unwrap();
    timeout(TIMEOUT, tasks.shutdown()).await.unwrap();
    assert_eq!(tasks.tracked_count(), 0);
}
