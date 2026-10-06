use std::sync::{Arc, Mutex};
use std::time::Instant;

use taide_native_retained::measure;
use taide_native_terminal::session::Frame;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc};

pub const HIGH_WATER_BYTES: usize = 512 * 1024;
pub const LOW_WATER_BYTES: usize = 64 * 1024;
const LOW_WATER_SHARE: usize = HIGH_WATER_BYTES / LOW_WATER_BYTES;
const HIGH_WATER_LIMIT_SHARE: usize = 2;

pub type FlowPort = Arc<dyn Fn(bool) + Send + Sync>;

#[derive(Clone, Copy)]
pub struct Limits {
    pub bytes: usize,
    pub count: usize,
    pub visits: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Closed,
    Capacity,
    Retained(taide_native_retained::Error),
    Sequence,
    Poisoned,
}

pub struct Delivery {
    frame: Frame,
    observed_at: Instant,
    _bytes: OwnedSemaphorePermit,
    _count: OwnedSemaphorePermit,
    _backlog: BacklogLease,
}

impl Delivery {
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn observed_at(&self) -> Instant {
        self.observed_at
    }
}

struct BacklogLease {
    shared: Arc<Shared>,
    weight: usize,
}

impl Drop for BacklogLease {
    fn drop(&mut self) {
        self.shared.release(self.weight);
    }
}

#[derive(Default)]
struct Backlog {
    bytes: usize,
    count: usize,
    is_paused: bool,
}

#[derive(Clone, Copy)]
struct Watermarks {
    high_bytes: usize,
    low_bytes: usize,
    high_count: usize,
    low_count: usize,
}

impl Watermarks {
    fn new(limits: Limits) -> Self {
        let high_bytes = HIGH_WATER_BYTES.min(limits.bytes / HIGH_WATER_LIMIT_SHARE);
        let high_count = (limits.count / HIGH_WATER_LIMIT_SHARE).max(1);
        Self {
            high_bytes,
            low_bytes: LOW_WATER_BYTES.min(high_bytes / LOW_WATER_SHARE).max(1),
            high_count,
            low_count: (high_count / LOW_WATER_SHARE).max(1),
        }
    }

    fn is_high(self, backlog: &Backlog) -> bool {
        backlog.bytes > self.high_bytes || backlog.count >= self.high_count
    }

    fn is_low(self, backlog: &Backlog) -> bool {
        backlog.bytes < self.low_bytes && backlog.count < self.low_count
    }
}

struct Order {
    revision: u64,
    failure: Option<Error>,
    backlog: Backlog,
}

struct Shared {
    bytes: Arc<Semaphore>,
    count: Arc<Semaphore>,
    order: Mutex<Order>,
    changed: Notify,
    limits: Limits,
    watermarks: Watermarks,
    flow: Option<FlowPort>,
}

impl Shared {
    fn fail(&self, error: Error) -> Error {
        let failure = match self.order.lock() {
            Ok(mut order) => {
                self.resume(&mut order.backlog);
                *order.failure.get_or_insert(error)
            }
            Err(_) => {
                self.signal(false);
                Error::Poisoned
            }
        };
        self.bytes.close();
        self.count.close();
        self.changed.notify_waiters();
        failure
    }

    fn failure(&self) -> Option<Error> {
        match self.order.lock() {
            Ok(order) => order.failure,
            Err(_) => Some(Error::Poisoned),
        }
    }

    fn pause_above_high_water(&self, backlog: &mut Backlog) {
        if backlog.is_paused || !self.watermarks.is_high(backlog) {
            return;
        }
        backlog.is_paused = true;
        self.signal(true);
    }

    fn resume(&self, backlog: &mut Backlog) {
        if !backlog.is_paused {
            return;
        }
        backlog.is_paused = false;
        self.signal(false);
    }

    fn signal(&self, paused: bool) {
        if let Some(flow) = &self.flow {
            flow(paused);
        }
    }

    fn release(&self, weight: usize) {
        let mut order = self.order.lock().unwrap_or_else(|error| error.into_inner());
        order.backlog.bytes = order.backlog.bytes.saturating_sub(weight);
        order.backlog.count = order.backlog.count.saturating_sub(1);
        if self.watermarks.is_low(&order.backlog) {
            self.resume(&mut order.backlog);
        }
    }
}

#[derive(Clone)]
pub struct Sender {
    sender: mpsc::Sender<Delivery>,
    shared: Arc<Shared>,
}

pub struct Receiver {
    receiver: mpsc::Receiver<Delivery>,
    shared: Arc<Shared>,
}

pub fn channel(limits: Limits) -> Result<(Sender, Receiver), Error> {
    open(limits, None)
}

pub fn channel_with_flow(limits: Limits, flow: FlowPort) -> Result<(Sender, Receiver), Error> {
    open(limits, Some(flow))
}

fn open(limits: Limits, flow: Option<FlowPort>) -> Result<(Sender, Receiver), Error> {
    if limits.bytes == 0
        || limits.count == 0
        || limits.visits == 0
        || limits.bytes > Semaphore::MAX_PERMITS
        || u32::try_from(limits.bytes).is_err()
        || limits.count > Semaphore::MAX_PERMITS
    {
        return Err(Error::InvalidLimits);
    }
    let (sender, receiver) = mpsc::channel(limits.count);
    let shared = Arc::new(Shared {
        bytes: Arc::new(Semaphore::new(limits.bytes)),
        count: Arc::new(Semaphore::new(limits.count)),
        order: Mutex::new(Order {
            revision: 0,
            failure: None,
            backlog: Backlog::default(),
        }),
        changed: Notify::new(),
        limits,
        watermarks: Watermarks::new(limits),
        flow,
    });
    Ok((
        Sender {
            sender,
            shared: shared.clone(),
        },
        Receiver { receiver, shared },
    ))
}

impl Sender {
    pub fn submit(&self, frame: Frame, observed_at: Instant) -> Result<(), Error> {
        if let Some(error) = self.shared.failure() {
            return Err(error);
        }
        let result = self.admit(frame, observed_at);
        match result {
            Ok(()) => Ok(()),
            Err(error) => Err(self.shared.fail(error)),
        }
    }

    fn admit(&self, frame: Frame, observed_at: Instant) -> Result<(), Error> {
        if self.sender.is_closed() {
            return Err(Error::Closed);
        }
        let report = measure(
            &frame,
            taide_native_retained::Limits {
                bytes: self.shared.limits.bytes,
                visits: self.shared.limits.visits,
            },
        )
        .map_err(Error::Retained)?;
        let weight = report
            .bytes
            .checked_add(size_of::<Delivery>() - size_of::<Frame>())
            .ok_or(Error::Capacity)?;
        let bytes = self
            .shared
            .bytes
            .clone()
            .try_acquire_many_owned(u32::try_from(weight).map_err(|_| Error::Capacity)?)
            .map_err(quota_error)?;
        let count = self
            .shared
            .count
            .clone()
            .try_acquire_owned()
            .map_err(quota_error)?;
        let mut order = self.shared.order.lock().map_err(|_| Error::Poisoned)?;
        if let Some(error) = order.failure {
            return Err(error);
        }
        if order.revision.checked_add(1) != Some(frame.revision) {
            return Err(Error::Sequence);
        }
        let revision = frame.revision;
        order.backlog.bytes += weight;
        order.backlog.count += 1;
        let delivery = Delivery {
            frame,
            observed_at,
            _bytes: bytes,
            _count: count,
            _backlog: BacklogLease {
                shared: self.shared.clone(),
                weight,
            },
        };
        if let Err(rejected) = self.sender.try_send(delivery) {
            drop(order);
            return Err(match rejected {
                mpsc::error::TrySendError::Full(_) => Error::Capacity,
                mpsc::error::TrySendError::Closed(_) => Error::Closed,
            });
        }
        order.revision = revision;
        self.shared.pause_above_high_water(&mut order.backlog);
        Ok(())
    }

    pub fn close(&self) {
        self.shared.fail(Error::Closed);
    }
}

fn quota_error(error: tokio::sync::TryAcquireError) -> Error {
    match error {
        tokio::sync::TryAcquireError::Closed => Error::Closed,
        tokio::sync::TryAcquireError::NoPermits => Error::Capacity,
    }
}

impl Receiver {
    pub fn try_recv(&mut self) -> Result<Option<Delivery>, Error> {
        match self.receiver.try_recv() {
            Ok(delivery) => Ok(Some(delivery)),
            Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected) => {
                match self.shared.failure() {
                    Some(error) => Err(error),
                    None => Ok(None),
                }
            }
        }
    }

    pub async fn recv(&mut self) -> Result<Option<Delivery>, Error> {
        loop {
            let changed = self.shared.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.shared.failure().is_some() {
                self.receiver.close();
            }
            tokio::select! {
                delivery = self.receiver.recv() => return match delivery {
                    Some(delivery) => Ok(Some(delivery)),
                    None => match self.shared.failure() {
                        Some(error) => Err(error),
                        None => Ok(None),
                    },
                },
                _ = &mut changed => {}
            }
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.shared.fail(Error::Closed);
    }
}
