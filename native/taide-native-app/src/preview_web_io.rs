use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    time::{Instant, Sleep},
};

pub(crate) struct TimedIo<T> {
    stream: T,
    deadline: Pin<Box<Sleep>>,
    idle: Duration,
}

impl<T> TimedIo<T> {
    pub fn new(stream: T, idle: Duration) -> Self {
        Self {
            stream,
            deadline: Box::pin(tokio::time::sleep(idle)),
            idle,
        }
    }

    fn progress(&mut self) {
        self.deadline.as_mut().reset(Instant::now() + self.idle);
    }

    fn stalled<Output>(&mut self, context: &mut Context<'_>) -> Poll<io::Result<Output>> {
        match self.deadline.as_mut().poll(context) {
            Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "preview connection idle timeout",
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for TimedIo<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let previous = buffer.filled().len();
        match Pin::new(&mut this.stream).poll_read(context, buffer) {
            Poll::Ready(Ok(())) => {
                if buffer.filled().len() > previous {
                    this.progress();
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => this.stalled(context),
        }
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for TimedIo<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        match Pin::new(&mut this.stream).poll_write(context, bytes) {
            Poll::Ready(Ok(count)) => {
                if count > 0 {
                    this.progress();
                }
                Poll::Ready(Ok(count))
            }
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => this.stalled(context),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.stream).poll_flush(context) {
            Poll::Pending => this.stalled(context),
            result => result,
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.stream).poll_shutdown(context) {
            Poll::Pending => this.stalled(context),
            result => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const TIMEOUT: Duration = Duration::from_secs(20);

    #[tokio::test]
    async fn web_idle_io는_읽기와_포화_쓰기의_pending을_deadline으로_종료한다() {
        let (left, _peer) = tokio::io::duplex(1);
        let mut io = TimedIo::new(left, Duration::ZERO);
        let mut byte = [0; 1];
        let error = tokio::time::timeout(TIMEOUT, io.read(&mut byte))
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);

        let (left, mut peer) = tokio::io::duplex(1);
        let mut io = TimedIo::new(left, Duration::ZERO);
        let error = tokio::time::timeout(TIMEOUT, io.write_all(b"xy"))
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        peer.read_exact(&mut byte).await.unwrap();
        assert_eq!(byte, *b"x");
    }
}
