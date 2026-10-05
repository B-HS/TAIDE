use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot, Notify, OwnedSemaphorePermit, Semaphore, TryAcquireError};
use tokio::task::JoinHandle;

use crate::lsp_frame::FrameLimits;

const WRITER_DRAIN_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriterFailure {
    InvalidLimits,
    FrameTooLarge,
    AllocationFailed,
    QueueFull,
    ByteBudgetFull,
    Closed,
    WriteFailed,
    UnsupportedTransport,
}

#[derive(Clone, Copy, Debug)]
pub struct WriterLimits {
    queued_frames: usize,
    retained_bytes: usize,
}

impl WriterLimits {
    pub fn new(queued_frames: usize, retained_bytes: usize) -> Result<Self, WriterFailure> {
        if queued_frames == 0
            || queued_frames > Semaphore::MAX_PERMITS
            || retained_bytes == 0
            || retained_bytes > Semaphore::MAX_PERMITS
            || u32::try_from(retained_bytes).is_err()
        {
            return Err(WriterFailure::InvalidLimits);
        }
        Ok(Self {
            queued_frames,
            retained_bytes,
        })
    }
}

struct WriteFrame {
    bytes: Vec<u8>,
    byte_permit: OwnedSemaphorePermit,
    completed: oneshot::Sender<Result<(), WriterFailure>>,
}

pub struct WriteReceipt(oneshot::Receiver<Result<(), WriterFailure>>);

impl WriteReceipt {
    pub async fn wait(mut self) -> Result<(), WriterFailure> {
        self.wait_ref().await
    }

    pub async fn wait_ref(&mut self) -> Result<(), WriterFailure> {
        (&mut self.0).await.unwrap_or(Err(WriterFailure::Closed))
    }
}

pub struct QueuedWriter {
    sender: mpsc::Sender<WriteFrame>,
    bytes: Arc<Semaphore>,
    byte_limit: usize,
    frame_limits: FrameLimits,
}

impl QueuedWriter {
    pub fn start<W, F>(
        output: W,
        frame_limits: FrameLimits,
        limits: WriterLimits,
        on_failure: F,
    ) -> Result<(Self, WriterTask), WriterFailure>
    where
        W: AsyncWrite + Unpin + Send + 'static,
        F: FnOnce() + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(limits.queued_frames);
        let bytes = Arc::new(Semaphore::new(limits.retained_bytes));
        let stop = Arc::new(Notify::new());
        let stop_for_worker = stop.clone();
        let bytes_for_worker = bytes.clone();
        let task = tokio::spawn(async move {
            tokio::select! {
                biased;
                _ = stop_for_worker.notified() => {},
                result = write_frames(output, receiver) => {
                    if result.is_err() {
                        on_failure();
                    }
                },
            }
            bytes_for_worker.close();
        });
        Ok((
            Self {
                sender,
                bytes,
                byte_limit: limits.retained_bytes,
                frame_limits,
            },
            WriterTask { task: Some(task), stop },
        ))
    }

    pub fn retained_frame_bytes(&self) -> usize {
        self.byte_limit - self.bytes.available_permits()
    }

    pub fn submit(&self, payload: &str) -> Result<WriteReceipt, WriterFailure> {
        if payload.len() > self.frame_limits.body_bytes() {
            return Err(WriterFailure::FrameTooLarge);
        }
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());
        if header.len() > self.frame_limits.header_bytes() {
            return Err(WriterFailure::FrameTooLarge);
        }
        let length = header.len().checked_add(payload.len()).ok_or(WriterFailure::FrameTooLarge)?;
        let permits = u32::try_from(length).map_err(|_| WriterFailure::FrameTooLarge)?;
        let slot = self.sender.try_reserve().map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => WriterFailure::QueueFull,
            mpsc::error::TrySendError::Closed(_) => WriterFailure::Closed,
        })?;
        let byte_permit = self.bytes.clone().try_acquire_many_owned(permits).map_err(|error| match error {
            TryAcquireError::Closed => WriterFailure::Closed,
            TryAcquireError::NoPermits => WriterFailure::ByteBudgetFull,
        })?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| WriterFailure::AllocationFailed)?;
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(payload.as_bytes());
        let (completed, receipt) = oneshot::channel();
        slot.send(WriteFrame {
            bytes,
            byte_permit,
            completed,
        });
        Ok(WriteReceipt(receipt))
    }

    pub async fn write_message(&self, payload: &str) -> Result<(), WriterFailure> {
        self.submit(payload)?.wait().await
    }
}

async fn write_frames<W>(mut output: W, mut receiver: mpsc::Receiver<WriteFrame>) -> Result<(), WriterFailure>
where
    W: AsyncWrite + Unpin,
{
    while let Some(frame) = receiver.recv().await {
        let result = async {
            output.write_all(&frame.bytes).await?;
            output.flush().await
        }
        .await;
        let outcome = result.map_err(|_| WriterFailure::WriteFailed);
        drop(frame.bytes);
        drop(frame.byte_permit);
        frame.completed.send(outcome).ok();
        outcome?;
    }
    Ok(())
}

pub struct WriterTask {
    task: Option<JoinHandle<()>>,
    stop: Arc<Notify>,
}

impl WriterTask {
    pub async fn finish(&mut self) {
        self.stop.notify_one();
        if let Some(task) = self.task.as_mut() {
            if tokio::time::timeout(WRITER_DRAIN_TIMEOUT, &mut *task).await.is_err() {
                task.abort();
                task.await.ok();
            }
        }
        self.task.take();
    }
}

impl Drop for WriterTask {
    fn drop(&mut self) {
        self.stop.notify_one();
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
