use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtyPair, PtySize};

use crate::domain::terminal::types::{OUTPUT_BATCH_MS, OUTPUT_FLUSH_TICK_MS, READ_BUFFER_BYTES};
use crate::error::{AppError, AppResult};

pub struct PtySpawnConfig {
    pub shell: Option<String>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

struct PauseGate {
    paused: Mutex<bool>,
    condvar: Condvar,
}

impl PauseGate {
    fn new() -> Self {
        Self {
            paused: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    fn set_paused(&self, paused: bool) {
        *self.paused.lock() = paused;
        if !paused {
            self.condvar.notify_all();
        }
    }

    fn wait_while_paused(&self) {
        let mut guard = self.paused.lock();
        while *guard {
            self.condvar.wait(&mut guard);
        }
    }
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    pause: Arc<PauseGate>,
    #[cfg_attr(not(windows), allow(dead_code))]
    shell_pid: Option<u32>,
}

impl PtySession {
    pub fn write(&self, data: &[u8]) -> AppResult<()> {
        let mut writer = self.writer.lock();
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> AppResult<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    pub fn kill(&self) -> AppResult<()> {
        self.killer.lock().kill().map_err(AppError::from)
    }

    pub fn set_paused(&self, paused: bool) {
        self.pause.set_paused(paused);
    }

    #[cfg(unix)]
    pub fn foreground_pid(&self) -> Option<u32> {
        self.master.process_group_leader().map(|pid| pid as u32)
    }

    #[cfg(windows)]
    pub fn foreground_pid(&self) -> Option<u32> {
        self.shell_pid
    }

    #[cfg(not(any(unix, windows)))]
    pub fn foreground_pid(&self) -> Option<u32> {
        None
    }
}

fn should_flush(batch_len: usize, elapsed: Duration) -> bool {
    batch_len >= READ_BUFFER_BYTES || elapsed >= Duration::from_millis(OUTPUT_BATCH_MS)
}

struct OutputBatch<D> {
    buf: Vec<u8>,
    last_flush: Instant,
    on_data: D,
}

impl<D: Fn(&[u8])> OutputBatch<D> {
    fn new(on_data: D) -> Self {
        Self {
            buf: Vec::with_capacity(READ_BUFFER_BYTES),
            last_flush: Instant::now(),
            on_data,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
        if should_flush(self.buf.len(), self.last_flush.elapsed()) {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        (self.on_data)(&self.buf);
        self.buf.clear();
        self.last_flush = Instant::now();
    }
}

fn build_command(config: &PtySpawnConfig) -> CommandBuilder {
    let mut cmd = match config.shell.as_deref() {
        Some(shell) => CommandBuilder::new(shell),
        None => CommandBuilder::new_default_prog(),
    };
    cmd.cwd(&config.cwd);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "TAIDE");
    cmd
}

pub fn spawn<D, X>(config: PtySpawnConfig, on_data: D, on_exit: X) -> AppResult<PtySession>
where
    D: Fn(&[u8]) + Send + 'static,
    X: FnOnce(Option<i32>) + Send + 'static,
{
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| AppError::Internal(error.to_string()))?;

    let cmd = build_command(&config);

    let PtyPair { slave, master } = pair;
    let mut child = slave.spawn_command(cmd).map_err(|error| AppError::Internal(error.to_string()))?;
    drop(slave);

    let shell_pid = child.process_id();
    let killer = child.clone_killer();
    let mut reader = master.try_clone_reader().map_err(|error| AppError::Internal(error.to_string()))?;
    let writer = master.take_writer().map_err(|error| AppError::Internal(error.to_string()))?;

    let pause = Arc::new(PauseGate::new());
    let reader_pause = pause.clone();

    let batch = Arc::new(Mutex::new(OutputBatch::new(on_data)));
    let flusher_batch = batch.clone();
    let draining = Arc::new(AtomicBool::new(true));
    let flusher_draining = draining.clone();

    std::thread::spawn(move || {
        while flusher_draining.load(AtomicOrdering::SeqCst) {
            std::thread::sleep(Duration::from_millis(OUTPUT_FLUSH_TICK_MS));
            flusher_batch.lock().flush();
        }
    });

    std::thread::spawn(move || {
        let mut buf = [0u8; READ_BUFFER_BYTES];

        loop {
            reader_pause.wait_while_paused();

            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => batch.lock().push(&buf[..n]),
            }
        }

        draining.store(false, AtomicOrdering::SeqCst);
        batch.lock().flush();
    });

    std::thread::spawn(move || {
        let code = child.wait().ok().map(|status| status.exit_code() as i32);
        on_exit(code);
    });

    Ok(PtySession {
        master,
        writer: Mutex::new(writer),
        killer: Mutex::new(killer),
        pause,
        shell_pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 배치_크기가_임계값을_넘으면_플러시한다() {
        assert!(should_flush(READ_BUFFER_BYTES, Duration::from_millis(0)));
        assert!(should_flush(READ_BUFFER_BYTES + 1, Duration::from_millis(0)));
        assert!(!should_flush(1, Duration::from_millis(0)));
    }

    #[test]
    fn 배치_경과시간이_임계값을_넘으면_플러시한다() {
        assert!(should_flush(1, Duration::from_millis(OUTPUT_BATCH_MS)));
        assert!(!should_flush(1, Duration::from_millis(OUTPUT_BATCH_MS - 1)));
    }

    type SentChunks = Arc<Mutex<Vec<Vec<u8>>>>;
    type DataSink = Box<dyn Fn(&[u8]) + Send>;
    type TestBatch = OutputBatch<DataSink>;

    fn collecting_batch() -> (TestBatch, SentChunks) {
        let sent: SentChunks = Arc::new(Mutex::new(Vec::new()));
        let recorder = sent.clone();
        let on_data: DataSink = Box::new(move |bytes: &[u8]| recorder.lock().push(bytes.to_vec()));
        (OutputBatch::new(on_data), sent)
    }

    #[test]
    fn 소량_청크는_즉시_전송되지_않고_배치에_남는다() {
        let (mut batch, sent) = collecting_batch();
        batch.push(b"hello");
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn 타이머_플러시가_갇힌_청크를_내보낸다() {
        let (mut batch, sent) = collecting_batch();
        batch.push(b"hello");
        batch.flush();
        assert_eq!(sent.lock().as_slice(), [b"hello".to_vec()]);
    }

    #[test]
    fn 빈_배치_플러시는_전송하지_않는다() {
        let (mut batch, sent) = collecting_batch();
        batch.flush();
        batch.flush();
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn 플러시_후_배치는_비워진다() {
        let (mut batch, sent) = collecting_batch();
        batch.push(b"a");
        batch.flush();
        batch.push(b"b");
        batch.flush();
        assert_eq!(sent.lock().as_slice(), [b"a".to_vec(), b"b".to_vec()]);
    }
}
