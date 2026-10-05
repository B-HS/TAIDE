use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use taide_model::error::{AppError, AppResult};
use taide_runtime::TaskSupervisor;
use taide_terminal::store::TerminalStore;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc, oneshot};
use tokio::task::JoinHandle;

const ORDER_RESERVATIONS: usize = 256;

#[derive(Default)]
struct OrderState {
    next: u64,
    pending: VecDeque<u64>,
}

#[derive(Default)]
struct OrderQueue {
    state: Mutex<OrderState>,
    changed: Notify,
    closed: AtomicBool,
}

pub(crate) struct Order {
    queue: Arc<OrderQueue>,
    id: u64,
}

impl Order {
    pub(crate) fn is_next(&self, next: &Self) -> AppResult<bool> {
        if !Arc::ptr_eq(&self.queue, &next.queue) {
            return Ok(false);
        }
        let state = self.queue.state.lock().map_err(|_| order_error())?;
        Ok(state
            .pending
            .iter()
            .position(|id| *id == self.id)
            .is_some_and(|index| state.pending.get(index + 1) == Some(&next.id)))
    }

    fn is_first(&self) -> AppResult<bool> {
        if self.queue.closed.load(Ordering::Acquire) {
            return Err(stopped());
        }
        let state = self.queue.state.lock().map_err(|_| order_error())?;
        Ok(state.pending.front() == Some(&self.id))
    }

    async fn wait(&self) -> AppResult<()> {
        loop {
            let changed = self.queue.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.is_first()? {
                return Ok(());
            }
            changed.await;
        }
    }
}

impl Drop for Order {
    fn drop(&mut self) {
        let mut state = self
            .queue
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(index) = state.pending.iter().position(|id| *id == self.id) {
            state.pending.remove(index);
        }
        drop(state);
        self.queue.changed.notify_waiters();
    }
}

fn order_error() -> AppError {
    AppError::Internal("native terminal input order unavailable".into())
}

#[derive(Clone, Copy)]
pub struct Limits {
    pub bytes: usize,
    pub count: usize,
}

impl Limits {
    pub fn validate(self) -> AppResult<Self> {
        if self.bytes == 0
            || self.count == 0
            || self.bytes > Semaphore::MAX_PERMITS
            || u32::try_from(self.bytes).is_err()
            || self.count > Semaphore::MAX_PERMITS
        {
            return Err(AppError::InvalidArgument(
                "native terminal writer limits are invalid".into(),
            ));
        }
        Ok(self)
    }
}

struct Envelope {
    data: Vec<u8>,
    bytes: OwnedSemaphorePermit,
    count: OwnedSemaphorePermit,
    receipt: oneshot::Sender<AppResult<()>>,
}

pub struct Receipt(oneshot::Receiver<AppResult<()>>);

pub enum Submission {
    Accepted(Receipt),
    Pending(Vec<u8>),
}

impl Receipt {
    pub fn try_wait(&mut self) -> Option<AppResult<()>> {
        match self.0.try_recv() {
            Ok(result) => Some(result),
            Err(oneshot::error::TryRecvError::Empty) => None,
            Err(oneshot::error::TryRecvError::Closed) => Some(Err(stopped())),
        }
    }

    pub async fn wait(self) -> AppResult<()> {
        self.0.await.map_err(|_| stopped())?
    }
}

#[derive(Clone)]
pub struct Writer {
    sender: mpsc::Sender<Envelope>,
    bytes: Arc<Semaphore>,
    count: Arc<Semaphore>,
    stop: mpsc::Sender<()>,
    byte_limit: usize,
    order: Arc<OrderQueue>,
}

fn stopped() -> AppError {
    AppError::Forbidden("native terminal writer is closed".into())
}

fn full() -> AppError {
    AppError::InvalidArgument("native terminal write exceeds its queue budget".into())
}

impl Writer {
    pub(crate) fn reserve_order(&self) -> AppResult<Order> {
        if self.sender.is_closed() || self.count.is_closed() {
            return Err(stopped());
        }
        let mut state = self.order.state.lock().map_err(|_| order_error())?;
        if state.pending.len() >= ORDER_RESERVATIONS {
            return Err(AppError::InvalidArgument(
                "native terminal input order budget exceeded".into(),
            ));
        }
        let id = state.next.checked_add(1).ok_or_else(order_error)?;
        state.next = id;
        state.pending.push_back(id);
        Ok(Order {
            queue: self.order.clone(),
            id,
        })
    }

    pub(crate) fn payload_limit(&self) -> usize {
        self.byte_limit.saturating_sub(size_of::<Envelope>())
    }

    pub fn start(
        tasks: &TaskSupervisor,
        limits: Limits,
        sink: impl Fn(&[u8]) -> AppResult<()> + Send + Sync + 'static,
    ) -> AppResult<(Self, JoinHandle<()>)> {
        let limits = limits.validate()?;
        let (sender, mut receiver) = mpsc::channel::<Envelope>(limits.count);
        let (stop, mut stopping) = mpsc::channel(1);
        let bytes = Arc::new(Semaphore::new(limits.bytes));
        let count = Arc::new(Semaphore::new(limits.count));
        let actor_bytes = bytes.clone();
        let actor_count = count.clone();
        let tasks = tasks.clone();
        let actor_tasks = tasks.clone();
        let sink = Arc::new(sink);
        let worker = tasks.spawn_transient_handle("native-terminal-writer", async move {
            loop {
                let envelope = tokio::select! {
                    biased;
                    _ = stopping.recv() => break,
                    envelope = receiver.recv() => match envelope { Some(envelope) => envelope, None => break },
                };
                let (finished, finish) = oneshot::channel();
                let sink = sink.clone();
                let worker = actor_tasks.spawn_blocking_transient_handle("native-terminal-write", move || {
                    let Envelope { data, bytes, count, receipt } = envelope;
                    let result = sink(&data);
                    let is_successful = result.is_ok();
                    drop(data);
                    drop(bytes);
                    drop(count);
                    drop(receipt.send(result));
                    let _ = finished.send(is_successful);
                });
                let is_successful = match worker {
                    Some(worker) => worker.await.is_ok() && finish.await.unwrap_or(false),
                    None => false,
                };
                if !is_successful { break; }
            }
            actor_bytes.close();
            actor_count.close();
            receiver.close();
            while let Some(envelope) = receiver.recv().await {
                drop(envelope.receipt.send(Err(stopped())));
            }
        }).ok_or_else(stopped)?;
        Ok((
            Self {
                sender,
                bytes,
                count,
                stop,
                byte_limit: limits.bytes,
                order: Arc::default(),
            },
            worker,
        ))
    }

    pub fn for_session(
        tasks: &TaskSupervisor,
        store: &TerminalStore,
        session: &str,
        limits: Limits,
    ) -> AppResult<(Self, JoinHandle<()>)> {
        let writer = store.writer_handle(session)?;
        Self::start(tasks, limits, move |data| {
            let mut writer = writer.lock();
            writer.write_all(data)?;
            writer.flush()?;
            Ok(())
        })
    }

    pub fn submit(&self, data: Vec<u8>) -> AppResult<Receipt> {
        match self.try_submit(data)? {
            Submission::Accepted(receipt) => Ok(receipt),
            Submission::Pending(_) => Err(full()),
        }
    }

    fn payload_weight(&self, capacity: usize) -> AppResult<u32> {
        let weight = capacity
            .checked_add(size_of::<Envelope>())
            .and_then(|weight| u32::try_from(weight).ok())
            .ok_or_else(full)?;
        if weight as usize > self.byte_limit {
            return Err(full());
        }
        Ok(weight)
    }

    pub async fn submit_wait(&self, data: Vec<u8>) -> AppResult<Receipt> {
        if self.sender.is_closed() {
            return Err(stopped());
        }
        self.payload_weight(data.capacity())?;
        let order = self.reserve_order()?;
        self.submit_wait_ordered(data, &order).await
    }

    pub(crate) async fn submit_wait_ordered(
        &self,
        data: Vec<u8>,
        order: &Order,
    ) -> AppResult<Receipt> {
        if self.sender.is_closed() {
            return Err(stopped());
        }
        let weight = self.payload_weight(data.capacity())?;
        if !Arc::ptr_eq(&self.order, &order.queue) {
            return Err(AppError::InvalidArgument(
                "native terminal write order belongs to another writer".into(),
            ));
        }
        let admission = async {
            order.wait().await?;
            let bytes = self
                .bytes
                .clone()
                .acquire_many_owned(weight)
                .await
                .map_err(|_| stopped())?;
            let count = self
                .count
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| stopped())?;
            Ok::<_, AppError>((bytes, count))
        };
        let (bytes, count) = tokio::select! {
            biased;
            () = self.sender.closed() => return Err(stopped()),
            result = admission => result?,
        };
        let (receipt, receiver) = oneshot::channel();
        self.sender
            .send(Envelope {
                data,
                bytes,
                count,
                receipt,
            })
            .await
            .map_err(|_| stopped())?;
        Ok(Receipt(receiver))
    }

    pub fn try_submit(&self, data: Vec<u8>) -> AppResult<Submission> {
        let order = self.reserve_order()?;
        self.try_submit_ordered(data, &order)
    }

    pub(crate) fn try_submit_ordered(&self, data: Vec<u8>, order: &Order) -> AppResult<Submission> {
        if self.sender.is_closed() {
            return Err(stopped());
        }
        let weight = self.payload_weight(data.capacity())?;
        if !Arc::ptr_eq(&self.order, &order.queue) {
            return Err(AppError::InvalidArgument(
                "native terminal write order belongs to another writer".into(),
            ));
        }
        if !order.is_first()? {
            return Ok(Submission::Pending(data));
        }
        let bytes = match self.bytes.clone().try_acquire_many_owned(weight) {
            Ok(permit) => permit,
            Err(tokio::sync::TryAcquireError::Closed) => return Err(stopped()),
            Err(tokio::sync::TryAcquireError::NoPermits) => return Ok(Submission::Pending(data)),
        };
        let count = match self.count.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(tokio::sync::TryAcquireError::Closed) => return Err(stopped()),
            Err(tokio::sync::TryAcquireError::NoPermits) => return Ok(Submission::Pending(data)),
        };
        let (receipt, receiver) = oneshot::channel();
        match self.sender.try_send(Envelope {
            data,
            bytes,
            count,
            receipt,
        }) {
            Ok(()) => Ok(Submission::Accepted(Receipt(receiver))),
            Err(mpsc::error::TrySendError::Full(envelope)) => {
                Ok(Submission::Pending(envelope.data))
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(stopped()),
        }
    }

    pub fn close(&self) {
        self.order.closed.store(true, Ordering::Release);
        self.order.changed.notify_waiters();
        self.bytes.close();
        self.count.close();
        let _ = self.stop.try_send(());
    }
}
