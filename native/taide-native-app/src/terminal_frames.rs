use std::sync::{Arc, Mutex};
use std::time::Instant;

use taide_native_retained::measure;
use taide_native_terminal::session::Frame;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc};

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
}

impl Delivery {
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    pub fn observed_at(&self) -> Instant {
        self.observed_at
    }
}

struct Order {
    revision: u64,
    failure: Option<Error>,
}

struct Shared {
    bytes: Arc<Semaphore>,
    count: Arc<Semaphore>,
    order: Mutex<Order>,
    changed: Notify,
    limits: Limits,
}

impl Shared {
    fn fail(&self, error: Error) -> Error {
        let failure = match self.order.lock() {
            Ok(mut order) => *order.failure.get_or_insert(error),
            Err(_) => Error::Poisoned,
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
        }),
        changed: Notify::new(),
        limits,
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
            .and_then(|weight| u32::try_from(weight).ok())
            .ok_or(Error::Capacity)?;
        let bytes = self
            .shared
            .bytes
            .clone()
            .try_acquire_many_owned(weight)
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
        self.sender
            .try_send(Delivery {
                frame,
                observed_at,
                _bytes: bytes,
                _count: count,
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error::Capacity,
                mpsc::error::TrySendError::Closed(_) => Error::Closed,
            })?;
        order.revision = revision;
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
